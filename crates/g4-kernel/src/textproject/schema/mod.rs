//! The entity schemas: one JSON file per entity under `data/<art>/NNNN.json`, each type with
//! `deny_unknown_fields` and named parts instead of `#[serde(flatten)]`. [`EntitySchema`] turns
//! the members of one entity into the type and back; `schema.snap` next to this module lists
//! every key path, so a rename cannot slip past the schema version.
//!
//! Every import error has the form `<code>: <path>: <sentence>`: `path` is the JSON path
//! inside the entity's file (`data.power`, `personal.type1`, `learnset.moves[3].move`,
//! `personal.raw.hex`), [`ROOT`] for the entity's object itself, and for a missing key the
//! path of the object that lacks it. The caller finds line and column from the path.

use std::path::Path;

use g4_formats::game::Family;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::KernelError;

use super::part::{BasePart, HEX_MAX, Member, PartKey, part_in_at, part_out};
use super::registry::Symbols;
use super::symbols::{SymbolKind, fallback_id};

pub(crate) mod data;

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
pub(crate) const ID_MISMATCH_CODE: &str = "textproject.id-mismatch";

/// Every schema type exports and imports through this, `members` in the kind's member order.
pub(crate) trait EntitySchema: Sized + Serialize + DeserializeOwned {
    /// The folder under `data/`.
    const ART: &'static str;
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
    fn sample() -> Self;
    /// The samples the snapshot lists, each under its label; more than one when the shape
    /// depends on the family.
    fn samples() -> Vec<(String, Self)> {
        vec![(Self::ART.to_owned(), Self::sample())]
    }
}

/// What an export needs besides the members; side files of [`Part::File`] parts collect in
/// `side` (name in the entity's folder, bytes).
pub struct ExportCtx<'a> {
    pub family: Family,
    pub symbols: &'a mut Symbols,
    pub base: &'a dyn BasePart,
    pub side: Vec<(String, Vec<u8>)>,
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
    let base = if use_base && bytes.len() > HEX_MAX {
        ctx.base.base_bytes(key)?
    } else {
        None
    };
    let stem = format!("{:04}", key.index);
    let ext = format!("{}.bin", key.part);
    let raw = part_out(bytes, base.as_deref(), &stem, &ext, &mut ctx.side);
    Ok(Member::Locked {
        locked: reason,
        raw,
    })
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
    use serde_json::Value;

    use super::super::SCHEMA_VERSION;
    use super::super::part::Part;
    use super::super::symbols::table;
    use super::data::{EncounterEntity, ItemEntity, MoveEntity, SpeciesEntity};
    use super::data::{TrainerTextOffsetsEntity, TrainerTextTableEntity};
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
        ]
        .into_iter()
        .find(|k| {
            s.strip_prefix(k.prefix())
                .is_some_and(|rest| rest.starts_with('_'))
        })
    }

    fn walk(path: &str, v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(map) if !map.is_empty() => {
                for (k, x) in map {
                    walk(&format!("{path}.{k}"), x, out);
                }
            }
            Value::Array(items) if !items.is_empty() => walk(&format!("{path}[]"), &items[0], out),
            Value::String(s) => match id_kind(s) {
                Some(kind) => out.push(format!("{path} →{}", kind.prefix())),
                None => out.push(path.to_owned()),
            },
            _ => out.push(path.to_owned()),
        }
    }

    fn lines_of<E: EntitySchema>(out: &mut Vec<String>) {
        for (label, sample) in E::samples() {
            walk(&label, &serde_json::to_value(&sample).unwrap(), out);
        }
    }

    /// Every key path of every schema type, and the forms of a locked member and a part.
    fn snapshot() -> String {
        let mut lines = vec![format!("schemaVersion {SCHEMA_VERSION}")];
        lines_of::<SpeciesEntity>(&mut lines);
        lines_of::<MoveEntity>(&mut lines);
        lines_of::<ItemEntity>(&mut lines);
        lines_of::<EncounterEntity>(&mut lines);
        lines_of::<TrainerTextTableEntity>(&mut lines);
        lines_of::<TrainerTextOffsetsEntity>(&mut lines);
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
            walk(
                "<locked>",
                &serde_json::to_value(&locked).unwrap(),
                &mut lines,
            );
        }
        let mut seen = std::collections::HashSet::new();
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
                "Schema geändert: schemaVersion erhöhen und eine Migration schreiben (oder \
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
        for line in [
            "species.id →SPECIES",
            "species.personal.hp",
            "species.personal.type1 →TYPE",
            "species.personal.ev_yield_hp",
            "species.personal.ev_yield_rest",
            "species.learnset.moves[].move →MOVE",
            "species.evolution.target0 →SPECIES",
            "moves.data.move_type →TYPE",
            "items.data.natural_gift_type →TYPE",
            "items.same_member_as →ITEM",
            "encounters[pt].data.grass_00_species →SPECIES",
            "encounters[hgss].data.grass_morning_00 →SPECIES",
            "trainer_text_table.entries[][]",
            "trainer_text_offsets.offsets[]",
            "<locked>.locked",
            "<locked>.raw.base_sha1",
        ] {
            assert!(text.lines().any(|l| l == line), "missing `{line}`:\n{text}");
        }
        // A container with bit fields is only its bit fields and its rest.
        assert!(!text.lines().any(|l| l == "species.personal.color_flip"));
        assert!(
            text.lines()
                .any(|l| l == "species.personal.color_flip_rest")
        );
        assert!(table(SymbolKind::Type).is_some());
    }
}
