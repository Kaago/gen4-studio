//! The entity schemas: one JSON file per entity under `data/<art>/NNNN.json`, each type with
//! `deny_unknown_fields` and named parts instead of `#[serde(flatten)]`. [`EntitySchema`] turns
//! the members of one entity into the type and back; `schema.snap` next to this module lists
//! every key path, so a rename cannot slip past the schema version.
//!
//! Every import error has the form `<code>: <path>: <sentence>`: `path` is the JSON path
//! inside the entity's file (`data.power`, `personal.type1`, `learnset.moves[3].move`,
//! `personal.raw.hex`, `data.party[2].species`, `maps[1]`, `warp_events[0].dest_header_id`,
//! `permissions[4]`, `buildings[1].dummy`, `model.file`, `location_name`),
//! [`ROOT`] for the entity's object itself, and for a missing key the path of the object that
//! lacks it. The caller finds line and column from the path.
//!
//! `schema.snap` also carries a digest of the fixed symbol tables (`symbolTables`): their ids are
//! in every file, so a changed table needs a new schema version like a changed key.
//!
//! In `schema.snap` a key the export can leave out ends in `?`, a position that can be
//! `{"locked", "raw"}` is one line `<path> <member>`, and another locked form (trainers) one
//! line `<path> <locked: keys>`.

use std::path::Path;

use g4_formats::data::egg_moves::EggMoves;
use g4_formats::game::Family;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::KernelError;

use super::manifest::SCHEMA_CODE;
use super::part::{BasePart, HEX_MAX, Member, Part, PartKey, part_in_at, part_out};
use super::record::resolve;
use super::registry::Symbols;
use super::symbols::{SymbolKind, fallback_id};

pub(crate) mod data;
pub(crate) mod header;
pub(crate) mod map;
#[cfg(test)]
mod testkit;
pub(crate) mod trainer;
pub(crate) mod world;

pub(crate) use super::record::{RecordJson, SymRef};

/// The path of an entity's own object in an error.
pub(crate) const ROOT: &str = "$";

/// The path of `key` inside the object at `path`.
pub(crate) fn join(path: &str, key: &str) -> String {
    if path == ROOT {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// The code of an `id` that does not name the entity's number.
pub(crate) use super::codes::ID_MISMATCH_CODE;

/// Every schema type exports and imports through this, `members` in the kind's member order.
pub(crate) trait EntitySchema: Sized + Serialize + DeserializeOwned {
    /// The folder under `data/`.
    const ART: &'static str;
    /// Whether a member of the kind can be locked; false only where every member decodes and
    /// writes back byte for byte (headers), and then [`EntitySchema::locked_samples`] is empty.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the schema snapshot test is the caller")
    )]
    const LOCKABLE: bool = true;
    /// The entity `index` from its members (`None`: the archive has no such member).
    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError>;
    /// The members of entity `index`, in the same order as `export` takes them. The error
    /// starts with its code (`textproject.schema`, `textproject.unknown-id`, …).
    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String>;
    /// For the snapshot: every option set, one element per list.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the schema snapshot test is the caller")
    )]
    fn sample() -> Self;
    /// The samples the snapshot lists, each under its label; more than one when the shape
    /// depends on the family.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the schema snapshot test is the caller")
    )]
    fn samples() -> Vec<(String, Self)> {
        vec![(Self::ART.to_owned(), Self::sample())]
    }
    /// For the snapshot: the samples with every option left out, under the labels of
    /// [`EntitySchema::samples`]; a key the full sample has and one of these lacks (where it
    /// has the object around it) is optional. Several per label when options nest, at least
    /// one per label (the snapshot test checks that).
    ///
    /// A new option (a key the export can leave out) must be set in the full sample and left
    /// out here, or the snapshot shows it as required and a later change of its
    /// optionality slips past the schema version.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the schema snapshot test is the caller")
    )]
    fn minimal_samples() -> Vec<(String, Self)>;
    /// For the snapshot: the samples with every member that can be locked locked, under the
    /// labels of [`EntitySchema::samples`].
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "the schema snapshot test is the caller")
    )]
    fn locked_samples() -> Vec<(String, Self)>;
}

/// What an export needs besides the members; side files of [`Part::File`] parts collect in
/// `side` (name in the entity's folder, bytes). `egg_moves` is the egg move table when it is
/// readable (read once per export): a species gets its entry as `egg_moves`.
pub struct ExportCtx<'a> {
    pub family: Family,
    pub symbols: &'a mut Symbols,
    pub base: &'a dyn BasePart,
    pub side: Vec<(String, Vec<u8>)>,
    pub egg_moves: Option<&'a EggMoves>,
}

/// What an import needs besides the JSON: `dir` is the entity's folder (side files),
/// `current` the members on disk in the kind's member order (`base_sha1` parts).
pub struct ImportCtx<'a> {
    pub family: Family,
    pub symbols: &'a Symbols,
    pub dir: &'a Path,
    pub base: &'a dyn BasePart,
    pub current: &'a [Option<Vec<u8>>],
}

/// `members[i]` as bytes, if there.
pub(crate) fn member(members: &[Option<Vec<u8>>], i: usize) -> Option<&[u8]> {
    members.get(i).and_then(|m| m.as_deref())
}

/// Member `i` of entity `index` of `art`, which every entity of the art has; missing is an
/// error.
pub(crate) fn required<'m>(
    members: &'m [Option<Vec<u8>>],
    i: usize,
    art: &str,
    index: u32,
) -> Result<&'m [u8], KernelError> {
    member(members, i)
        .ok_or_else(|| KernelError::InvalidProject(format!("{art} {index:04}: Member {i} fehlt.")))
}

/// The number of reference `r` at JSON `path`: an id of `kind` or a number, at most `max`.
pub(crate) fn resolve_max(
    sym: &Symbols,
    kind: SymbolKind,
    r: &SymRef,
    path: &str,
    max: u32,
) -> Result<u32, String> {
    let n = resolve(sym, kind, r, path)?;
    if n > max {
        return Err(format!(
            "{SCHEMA_CODE}: {path}: {n} liegt nicht in 0..={max}."
        ));
    }
    Ok(n)
}

/// The `id` of entity `index` of `kind`: its table or register id, else the fallback.
pub(crate) fn entity_id(sym: &Symbols, kind: SymbolKind, index: u32) -> String {
    sym.id(kind, index)
        .unwrap_or_else(|| fallback_id(kind, index))
}

/// Whether `id` names number `index` of `kind` (`textproject.id-mismatch` otherwise); the
/// import never renames an id.
pub(crate) fn check_id(
    sym: &Symbols,
    kind: SymbolKind,
    index: u32,
    id: &str,
) -> Result<(), String> {
    if sym.number(kind, id) == Some(index) {
        return Ok(());
    }
    Err(format!(
        "{ID_MISMATCH_CODE}: id: `{id}` ist nicht die ID von Nummer {index} (`{}`); die ID \
         umzubenennen geht hier nicht.",
        entity_id(sym, kind, index)
    ))
}

/// `bytes` as a locked member with `reason`; the raw part compares with the base layer's
/// part `key` only when `use_base` (and the bytes are too long for hex).
pub(crate) fn lock_member<T>(
    key: PartKey,
    use_base: bool,
    bytes: &[u8],
    reason: String,
    ctx: &mut ExportCtx<'_>,
) -> Result<Member<T>, KernelError> {
    Ok(Member::Locked {
        locked: reason,
        raw: raw_part(key, use_base, bytes, ctx)?,
    })
}

/// `bytes` of part `key` as a [`Part`]; a side file is named `NNNN.<part>.bin`, and the base
/// layer is asked only when `use_base` (and the bytes are too long for hex).
pub(crate) fn raw_part(
    key: PartKey,
    use_base: bool,
    bytes: &[u8],
    ctx: &mut ExportCtx<'_>,
) -> Result<Part, KernelError> {
    let ext = format!("{}.bin", key.part);
    if use_base {
        return entity_part(key, &ext, bytes, ctx);
    }
    let stem = format!("{:04}", key.index);
    Ok(part_out(bytes, None, &stem, &ext, &mut ctx.side))
}

/// `bytes` of part `key` as a [`Part`] compared with the base layer's part `key` (asked only
/// when the bytes are too long for hex); a side file is named `NNNN.<ext>`.
pub(crate) fn entity_part(
    key: PartKey,
    ext: &str,
    bytes: &[u8],
    ctx: &mut ExportCtx<'_>,
) -> Result<Part, KernelError> {
    let base = if bytes.len() > HEX_MAX {
        ctx.base.base_bytes(key)?
    } else {
        None
    };
    let stem = format!("{:04}", key.index);
    Ok(part_out(bytes, base.as_deref(), &stem, ext, &mut ctx.side))
}

/// A locked member for the snapshot's locked samples.
pub(crate) fn locked_sample<T>() -> Member<T> {
    Member::Locked {
        locked: "reason".into(),
        raw: Part::Hex { hex: "00".into() },
    }
}

/// The bytes of member `m` at JSON `path`: a locked member's raw part, else `value` of its
/// value.
pub(crate) fn member_bytes<T>(
    m: &Member<T>,
    path: &str,
    key: PartKey,
    current: Option<&[u8]>,
    ctx: &ImportCtx<'_>,
    value: impl FnOnce(&T) -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    match m {
        Member::Value(v) => value(v),
        Member::Locked { raw, .. } => {
            part_in_at(raw, &join(path, "raw"), ctx.dir, &key, current, ctx.base)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;

    use super::super::SCHEMA_VERSION;
    use super::super::symbols::{fixed_tables_digest, table};
    use super::super::tombstone::DELETED_KEY;
    use super::data::{EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity};
    use super::data::{TrainerTextOffsetsEntity, TrainerTextTableEntity};
    use super::header::{HeaderHgssEntity, HeaderPtEntity};
    use super::map::MapEntity;
    use super::trainer::TrainerEntity;
    use super::world::{AreaDataEntity, EventEntity, MatrixEntity};
    use super::*;

    const SNAPSHOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/textproject/schema.snap");

    /// The kind whose ids start like `s` (the longest prefix wins: `TRAINER_CLASS_` before
    /// `TRAINER_`).
    fn id_kind(s: &str) -> Option<SymbolKind> {
        use SymbolKind::*;
        [
            TrainerClass,
            Species,
            Move,
            Item,
            Ability,
            Type,
            Map,
            Trainer,
            Place,
            Flag,
            Var,
        ]
        .into_iter()
        .find(|k| {
            s.strip_prefix(k.prefix())
                .is_some_and(|rest| rest.starts_with('_'))
        })
    }

    /// Collects into `optional` the plain paths of keys `full` has and `min` lacks while it
    /// has the object around them.
    fn mark_optional(
        path: &str,
        full: &Value,
        min: Option<&Value>,
        optional: &mut HashSet<String>,
    ) {
        match full {
            Value::Object(map) => {
                for (k, x) in map {
                    let p = format!("{path}.{k}");
                    let m = match min {
                        Some(Value::Object(m)) => {
                            let child = m.get(k);
                            if child.is_none() {
                                optional.insert(p.clone());
                            }
                            child
                        }
                        _ => None,
                    };
                    mark_optional(&p, x, m, optional);
                }
            }
            Value::Array(items) if !items.is_empty() => {
                let m = match min {
                    Some(Value::Array(m)) => m.first(),
                    _ => None,
                };
                mark_optional(&format!("{path}[]"), &items[0], m, optional);
            }
            _ => {}
        }
    }

    /// The lines of `v` at `path` (`plain`: the path without `?` marks). With `locked`, an
    /// object `{locked, raw}` is one line `<path> <member>` and any other object with a
    /// `locked` key one line naming its keys.
    fn walk(
        path: &str,
        plain: &str,
        v: &Value,
        optional: &HashSet<String>,
        locked: bool,
        out: &mut Vec<String>,
    ) {
        match v {
            Value::Object(map) if locked && map.contains_key("locked") => {
                let keys: Vec<&str> = map.keys().map(String::as_str).collect();
                if keys == ["locked", "raw"] {
                    out.push(format!("{path} <member>"));
                } else {
                    out.push(format!("{path} <locked: {}>", keys.join(", ")));
                }
            }
            Value::Object(map) if !map.is_empty() => {
                for (k, x) in map {
                    let p = format!("{plain}.{k}");
                    let mark = if optional.contains(&p) { "?" } else { "" };
                    walk(&format!("{path}.{k}{mark}"), &p, x, optional, locked, out);
                }
            }
            Value::Array(items) if !items.is_empty() => walk(
                &format!("{path}[]"),
                &format!("{plain}[]"),
                &items[0],
                optional,
                locked,
                out,
            ),
            Value::String(s) => match id_kind(s) {
                Some(kind) => out.push(format!("{path} →{}", kind.prefix())),
                None => out.push(path.to_owned()),
            },
            _ => out.push(path.to_owned()),
        }
    }

    fn json<E: EntitySchema>(samples: Vec<(String, E)>) -> Vec<(String, Value)> {
        samples
            .into_iter()
            .map(|(label, e)| (label, serde_json::to_value(&e).unwrap()))
            .collect()
    }

    /// The lines of `E`: every key path of its full samples, optional keys marked `?`, then
    /// the positions its locked samples lock.
    fn lines_of<E: EntitySchema>(out: &mut Vec<String>) {
        let full = json(E::samples());
        let minimal = json(E::minimal_samples());
        for (label, _) in &full {
            assert!(
                minimal.iter().any(|(l, _)| l == label),
                "{}: sample `{label}` has no minimal sample, its options would lose their `?`",
                E::ART
            );
        }
        let mut optional = HashSet::new();
        for (label, min) in minimal {
            let (_, f) = full
                .iter()
                .find(|(l, _)| *l == label)
                .unwrap_or_else(|| panic!("{}: minimal sample `{label}` has no full one", E::ART));
            mark_optional(&label, f, Some(&min), &mut optional);
        }
        for (label, v) in &full {
            walk(label, label, v, &optional, false, out);
        }
        let locked = json(E::locked_samples());
        assert_eq!(
            !locked.is_empty(),
            E::LOCKABLE,
            "{}: a kind has locked samples exactly when it is lockable",
            E::ART
        );
        for (label, v) in &locked {
            walk(label, label, v, &optional, true, out);
        }
    }

    /// Every key path of every schema type, and the forms of a locked member and a part.
    fn snapshot() -> String {
        let mut lines = vec![
            format!("schemaVersion {SCHEMA_VERSION}"),
            format!("symbolTables {}", fixed_tables_digest()),
            // The tombstone envelope: no field of a type, a top-level key of every entity file.
            format!("*.{DELETED_KEY}?"),
        ];
        lines_of::<SpeciesEntity>(&mut lines);
        lines_of::<MoveEntity>(&mut lines);
        lines_of::<ItemEntity>(&mut lines);
        lines_of::<EncounterEntity>(&mut lines);
        lines_of::<TrainerTextTableEntity>(&mut lines);
        lines_of::<TrainerTextOffsetsEntity>(&mut lines);
        lines_of::<TrainerEntity>(&mut lines);
        lines_of::<MatrixEntity>(&mut lines);
        lines_of::<AreaDataEntity>(&mut lines);
        lines_of::<EventEntity>(&mut lines);
        lines_of::<MapEntity>(&mut lines);
        lines_of::<HeaderPtEntity>(&mut lines);
        lines_of::<HeaderHgssEntity>(&mut lines);
        for raw in [
            Part::Hex { hex: "00".into() },
            Part::Base {
                base_sha1: "0".repeat(40),
            },
            Part::File {
                file: "0000.bin".into(),
            },
        ] {
            let locked = Member::<()>::Locked {
                locked: "reason".into(),
                raw,
            };
            let v = serde_json::to_value(&locked).unwrap();
            walk(
                "<locked>",
                "<locked>",
                &v,
                &HashSet::new(),
                false,
                &mut lines,
            );
        }
        let mut seen = HashSet::new();
        lines.retain(|l| seen.insert(l.clone()));
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    #[test]
    fn schema_fields_match_the_snapshot() {
        let now = snapshot();
        if std::env::var_os("G4_UPDATE_SNAPSHOT").is_some_and(|v| v == "1") {
            std::fs::write(SNAPSHOT, &now).unwrap();
            return;
        }
        let saved = std::fs::read_to_string(SNAPSHOT).unwrap_or_default();
        if saved == now {
            return;
        }
        let saved_version = saved
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("schemaVersion "))
            .and_then(|v| v.parse::<u32>().ok());
        let diff: Vec<String> = now
            .lines()
            .filter(|l| !saved.lines().any(|s| s == *l))
            .map(|l| format!("+ {l}"))
            .chain(
                saved
                    .lines()
                    .filter(|s| !now.lines().any(|l| l == *s))
                    .map(|s| format!("- {s}")),
            )
            .collect();
        if saved_version == Some(SCHEMA_VERSION) {
            panic!(
                "Schema oder Symboltabellen geändert: schemaVersion erhöhen und eine Migration schreiben (oder \
                 G4_UPDATE_SNAPSHOT=1, wenn die Migration schon da ist)\n{}",
                diff.join("\n")
            );
        }
        panic!(
            "schema.snap ist für schemaVersion {saved_version:?}, der Code für \
             {SCHEMA_VERSION}: nach der Migration G4_UPDATE_SNAPSHOT=1 setzen\n{}",
            diff.join("\n")
        );
    }

    #[test]
    fn snapshot_lines_name_codec_fields_and_refs() {
        let text = snapshot();
        assert!(text.starts_with(&format!("schemaVersion {SCHEMA_VERSION}\n")));
        assert_eq!(
            text.lines().nth(2),
            Some("*.deleted?"),
            "the envelope leads"
        );
        for line in [
            "species.id →SPECIES",
            "species.personal?.hp",
            "species.personal?.type1 →TYPE",
            "species.personal?.ev_yield_hp",
            "species.personal?.ev_yield_rest?",
            "species.learnset?.moves[].move →MOVE",
            "species.learnset?.terminated",
            "species.learnset?.tail?",
            "species.evolution?.target0 →SPECIES",
            "species.personal? <member>",
            "species.learnset? <member>",
            "moves.data.move_type →TYPE",
            "moves.data <member>",
            "items.data?.natural_gift_type →TYPE",
            "items.same_member_as? →ITEM",
            "items.data? <member>",
            "encounters[pt].data.grass_00_species →SPECIES",
            "encounters[hgss].data.grass_morning_00 →SPECIES",
            "encounters[hgss].data <member>",
            "trainer_text_table.entries[][]",
            "trainer_text_table <member>",
            "trainer_text_offsets.offsets[]",
            "trainers.id →TRAINER",
            "trainers.data.class →TRAINER_CLASS",
            "trainers.data.items[] →ITEM",
            "trainers.data.party[].species →SPECIES",
            "trainers.data.party[].form",
            "trainers.data.party[].item? →ITEM",
            "trainers.data.party[].moves?[] →MOVE",
            "trainers.data.party[].ball_capsule?",
            "trainers.data <locked: locked, props, party>",
            "matrices.maps[]",
            "matrices.headers?[]",
            "matrices.altitudes?[]",
            "matrices <member>",
            "area_data[pt].unknown",
            "area_data[pt].light_type",
            "area_data[hgss].area_type",
            "area_data[hgss] <member>",
            "events.bg_events[].player_facing_dir",
            "events.object_events[].data[]",
            "events.warp_events[].dest_header_id →MAP",
            "events.object_events[].hidden_flag →FLAG",
            "events.coord_events[].var →VAR",
            "events.coord_events[].value",
            "events <member>",
            "trainers.data.flags_rest?",
            "maps.source",
            "maps.permissions[]",
            "maps.buildings[].model_id",
            "maps.buildings[].dummy?",
            "maps.buildings_tail?",
            "maps.model.hex",
            "maps.bdhc.hex",
            "maps.bgs?.hex",
            "maps <member>",
            "headers[pt].id →MAP",
            "headers[pt].internal_name",
            "headers[pt].internal_name_bytes?",
            "headers[pt].location_name →PLACE",
            "headers[pt].location_specifier",
            "headers[hgss].id →MAP",
            "headers[hgss].kanto_flag",
            "headers[hgss].location_name →PLACE",
            "<locked>.locked",
            "<locked>.raw.base_sha1",
        ] {
            assert!(text.lines().any(|l| l == line), "missing `{line}`:\n{text}");
        }
        // A container with bit fields is only its bit fields and its rest.
        assert!(
            !text
                .lines()
                .any(|l| l.starts_with("species.personal?.color_flip "))
        );
        assert!(!text.lines().any(|l| l == "species.personal?.color_flip"));
        assert!(
            text.lines()
                .any(|l| l == "species.personal?.color_flip_rest?")
        );
        // Required keys carry no mark; headers are never locked.
        assert!(!text.lines().any(|l| l.starts_with("species.id?")));
        assert!(
            !text
                .lines()
                .any(|l| l.starts_with("headers") && l.contains('<'))
        );
        assert!(
            !text
                .lines()
                .any(|l| l.starts_with("trainers.data.party[].species?"))
        );
        assert!(table(SymbolKind::Type).is_some());
    }
}
