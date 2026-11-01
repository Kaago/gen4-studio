//! The data kinds: species (personal, learnset and evolution of one index in one file),
//! moves, items (one file per item id, members through the ARM9 item table), wild
//! encounters, and the two trainer text tables.

use g4_formats::data::encounter::{HgssEncounters, PtEncounters};
use g4_formats::data::evolution::Evolution;
use g4_formats::data::item::Item;
use g4_formats::data::learnset::{LearnEntry, Learnset};
use g4_formats::data::moves::Move;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::trainer_text::{TrainerTextOffsets, TrainerTextTable};
use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use crate::error::KernelError;

use super::super::canon::{hex, unhex};
use super::super::egg_moves::field_of;
use super::super::manifest::SCHEMA_CODE;
use super::super::part::{Member, PartKey};
use super::super::record::{
    minimal_record, record_from_json, record_from_json_at, record_to_json, resolve, sample_record,
    sym_ref,
};
use super::super::symbols::{SymbolKind, fallback_id};
use super::{
    EntitySchema, ExportCtx, ImportCtx, ROOT, RecordJson, SymRef, check_id, entity_id, lock_member,
    locked_sample, member, member_bytes, required,
};

/// One species: its `personalPokeData`, `learnsets` and `evolutions` members. A part is
/// missing only when its archive has no member of that index. `egg_moves` is the species'
/// entry of the egg move table (one table for all species, see
/// [`textproject::egg_moves`](super::super::egg_moves)): the moves of its entry, empty for a
/// mark without moves, left out without an entry or when the table is not readable. The
/// species import leaves it alone; the table is imported from every species file at once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesEntity {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal: Option<Member<RecordJson>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learnset: Option<Member<LearnsetJson>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution: Option<Member<RecordJson>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub egg_moves: Option<Vec<SymRef>>,
}

/// A learnset: the level-up moves, whether `0xFFFF` ends them, and the bytes after it.
/// `tail` is left out when it is exactly the zero padding to four bytes that
/// `Learnset::write` adds for this number of moves (none without terminator); without it
/// the import pads the same way, so moves can be added by hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearnsetJson {
    pub moves: Vec<LearnMoveJson>,
    pub terminated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<String>,
}

/// One level-up move (level 0 to 127, move 0 to 511).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearnMoveJson {
    pub level: u8,
    #[serde(rename = "move")]
    pub move_id: SymRef,
}

/// One `moveData` member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveEntity {
    pub id: String,
    pub data: Member<RecordJson>,
}

/// One item id: the data of its `itemData` member, or the id that carries the data of the
/// member it shares (`same_member_as`). Both are missing only when the item table points
/// past the archive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEntity {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Member<RecordJson>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_member_as: Option<String>,
}

/// One `encounters` member, in the layout of the project's family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterEntity {
    pub data: Member<RecordJson>,
}

/// `trainerTextTable/0000`: the table, or `{locked, raw}` when its length is no multiple
/// of four.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrainerTextTableEntity(pub Member<TrainerTextTableJson>);

/// The trainer text table: `[trainer, trigger]` per message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerTextTableJson {
    pub entries: Vec<[u16; 2]>,
}

/// `trainerTextOffset/0000`: the offsets, or `{locked, raw}` when its length is odd.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrainerTextOffsetsEntity(pub Member<TrainerTextOffsetsJson>);

/// The trainer text offsets: one table offset per trainer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerTextOffsetsJson {
    pub offsets: Vec<u16>,
}

/// A fixed record member as its JSON, or locked when it does not decode or does not write
/// back byte for byte.
fn record_member<R: RawRecord>(
    key: PartKey,
    use_base: bool,
    bytes: &[u8],
    ctx: &mut ExportCtx<'_>,
) -> Result<Member<RecordJson>, KernelError> {
    let name = key.part;
    let index = key.index;
    let reason = match R::read(bytes) {
        Err(_) => format!(
            "{name} {index:04}: {} bytes, expected {}",
            bytes.len(),
            R::LEN
        ),
        Ok(r) => {
            let j = record_to_json(&r, name, ctx.symbols);
            match record_from_json::<R>(&j, name, ctx.symbols) {
                Ok(back) if back.raw() == bytes => return Ok(Member::Value(j)),
                _ => format!("{name} {index:04}: does not write back byte for byte"),
            }
        }
    };
    lock_member(key, use_base, bytes, reason, ctx)
}

/// The bytes of a fixed record member at JSON `path` of its entity.
fn record_bytes<R: RawRecord>(
    m: &Member<RecordJson>,
    path: &str,
    key: PartKey,
    current: Option<&[u8]>,
    ctx: &ImportCtx<'_>,
) -> Result<Vec<u8>, String> {
    member_bytes(m, path, key, current, ctx, |j| {
        record_from_json_at::<R>(j, key.part, path, ctx.symbols).map(|r| r.write())
    })
}

/// The zero padding `Learnset::write` puts after the terminator of `moves` entries.
fn learnset_padding(moves: usize) -> Vec<u8> {
    let len = 2 * moves + 2;
    vec![0; len.next_multiple_of(4) - len]
}

/// The learnset as written: entries, `0xFFFF` when terminated, then the tail.
fn encode_learnset(entries: &[LearnEntry], terminated: bool, tail: &[u8]) -> Vec<u8> {
    let mut ls = Learnset {
        entries: entries.to_vec(),
        tail: tail.to_vec(),
        terminated,
        read_count: entries.len(),
    }
    .write();
    if !terminated {
        // `write` always ends in the terminator (and an empty tail).
        ls.truncate(ls.len() - 2);
    }
    ls
}

fn learnset_member(
    index: u32,
    bytes: &[u8],
    ctx: &mut ExportCtx<'_>,
) -> Result<Member<LearnsetJson>, KernelError> {
    let key = PartKey {
        art: SpeciesEntity::ART,
        index,
        part: "learnset",
    };
    let reason = match Learnset::read(bytes) {
        Err(_) => format!(
            "learnset {index:04}: {} bytes, odd and without terminator",
            bytes.len()
        ),
        Ok(ls) if encode_learnset(&ls.entries, ls.terminated, &ls.tail) == bytes => {
            let moves = ls
                .entries
                .iter()
                .map(|e| LearnMoveJson {
                    level: e.level,
                    move_id: sym_ref(ctx.symbols, SymbolKind::Move, u32::from(e.move_id)),
                })
                .collect();
            let standard = if ls.terminated {
                learnset_padding(ls.entries.len())
            } else {
                Vec::new()
            };
            return Ok(Member::Value(LearnsetJson {
                moves,
                terminated: ls.terminated,
                tail: (ls.tail != standard).then(|| hex(&ls.tail)),
            }));
        }
        Ok(_) => format!("learnset {index:04}: does not write back byte for byte"),
    };
    lock_member(key, true, bytes, reason, ctx)
}

fn learnset_bytes(j: &LearnsetJson, ctx: &ImportCtx<'_>) -> Result<Vec<u8>, String> {
    let mut entries = Vec::with_capacity(j.moves.len());
    for (i, m) in j.moves.iter().enumerate() {
        let field = format!("learnset.moves[{i}]");
        let move_id = resolve(
            ctx.symbols,
            SymbolKind::Move,
            &m.move_id,
            &format!("{field}.move"),
        )?;
        if move_id > 0x1FF {
            return Err(format!(
                "{SCHEMA_CODE}: {field}.move: {move_id} liegt nicht in 0..=511."
            ));
        }
        if m.level > 0x7F {
            return Err(format!(
                "{SCHEMA_CODE}: {field}.level: {} liegt nicht in 0..=127.",
                m.level
            ));
        }
        if move_id == 0x1FF && m.level == 0x7F {
            return Err(format!(
                "{SCHEMA_CODE}: {field}: Attacke 511 auf Level 127 ist das Endezeichen 0xFFFF."
            ));
        }
        entries.push(LearnEntry {
            move_id: move_id as u16,
            level: m.level,
        });
    }
    let tail = match &j.tail {
        Some(text) => unhex(text).map_err(|e| format!("{SCHEMA_CODE}: learnset.tail: {e}"))?,
        None if j.terminated => learnset_padding(entries.len()),
        None => Vec::new(),
    };
    if !j.terminated && !tail.is_empty() {
        return Err(format!(
            "{SCHEMA_CODE}: learnset.tail: ohne Endezeichen (`terminated: false`) gibt es keinen Rest."
        ));
    }
    Ok(encode_learnset(&entries, j.terminated, &tail))
}

impl EntitySchema for SpeciesEntity {
    const ART: &'static str = "species";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let key = |part| PartKey {
            art: Self::ART,
            index,
            part,
        };
        let personal = match member(members, 0) {
            Some(b) => Some(record_member::<Personal>(key("personal"), true, b, ctx)?),
            None => None,
        };
        let learnset = match member(members, 1) {
            Some(b) => Some(learnset_member(index, b, ctx)?),
            None => None,
        };
        let evolution = match member(members, 2) {
            Some(b) => Some(record_member::<Evolution>(key("evolution"), true, b, ctx)?),
            None => None,
        };
        let egg_moves = ctx
            .egg_moves
            .and_then(|table| field_of(table, index, ctx.symbols));
        Ok(SpeciesEntity {
            id: entity_id(ctx.symbols, SymbolKind::Species, index),
            personal,
            learnset,
            evolution,
            egg_moves,
        })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Species, index, &self.id)?;
        let key = |part| PartKey {
            art: Self::ART,
            index,
            part,
        };
        let current = |i| member(ctx.current, i);
        let personal = self
            .personal
            .as_ref()
            .map(|m| record_bytes::<Personal>(m, "personal", key("personal"), current(0), ctx))
            .transpose()?;
        let learnset = self
            .learnset
            .as_ref()
            .map(|m| {
                member_bytes(m, "learnset", key("learnset"), current(1), ctx, |j| {
                    learnset_bytes(j, ctx)
                })
            })
            .transpose()?;
        let evolution = self
            .evolution
            .as_ref()
            .map(|m| record_bytes::<Evolution>(m, "evolution", key("evolution"), current(2), ctx))
            .transpose()?;
        Ok(vec![personal, learnset, evolution])
    }

    fn sample() -> Self {
        SpeciesEntity {
            id: fallback_id(SymbolKind::Species, 1),
            personal: Some(Member::Value(sample_record::<Personal>("personal"))),
            learnset: Some(Member::Value(LearnsetJson {
                moves: vec![LearnMoveJson {
                    level: 1,
                    move_id: SymRef::Id(fallback_id(SymbolKind::Move, 1)),
                }],
                terminated: true,
                tail: Some("0000".into()),
            })),
            evolution: Some(Member::Value(sample_record::<Evolution>("evolution"))),
            egg_moves: Some(vec![
                SymRef::Id(fallback_id(SymbolKind::Move, 1)),
                SymRef::Id(fallback_id(SymbolKind::Move, 2)),
            ]),
        }
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        let id = fallback_id(SymbolKind::Species, 1);
        let none = SpeciesEntity {
            id: id.clone(),
            personal: None,
            learnset: None,
            evolution: None,
            egg_moves: None,
        };
        let parts = SpeciesEntity {
            id,
            personal: Some(Member::Value(minimal_record::<Personal>("personal"))),
            learnset: Some(Member::Value(LearnsetJson {
                moves: Vec::new(),
                terminated: true,
                tail: None,
            })),
            evolution: Some(Member::Value(minimal_record::<Evolution>("evolution"))),
            egg_moves: Some(Vec::new()),
        };
        vec![(Self::ART.to_owned(), none), (Self::ART.to_owned(), parts)]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        let e = SpeciesEntity {
            id: fallback_id(SymbolKind::Species, 1),
            personal: Some(locked_sample()),
            learnset: Some(locked_sample()),
            evolution: Some(locked_sample()),
            egg_moves: None,
        };
        vec![(Self::ART.to_owned(), e)]
    }
}

impl EntitySchema for MoveEntity {
    const ART: &'static str = "moves";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let key = PartKey {
            art: Self::ART,
            index,
            part: "move",
        };
        Ok(MoveEntity {
            id: entity_id(ctx.symbols, SymbolKind::Move, index),
            data: record_member::<Move>(key, true, bytes, ctx)?,
        })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Move, index, &self.id)?;
        let key = PartKey {
            art: Self::ART,
            index,
            part: "move",
        };
        let bytes = record_bytes::<Move>(&self.data, "data", key, member(ctx.current, 0), ctx)?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        MoveEntity {
            id: fallback_id(SymbolKind::Move, 1),
            data: Member::Value(sample_record::<Move>("move")),
        }
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        let e = MoveEntity {
            id: fallback_id(SymbolKind::Move, 1),
            data: Member::Value(minimal_record::<Move>("move")),
        };
        vec![(Self::ART.to_owned(), e)]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        let e = MoveEntity {
            id: fallback_id(SymbolKind::Move, 1),
            data: locked_sample(),
        };
        vec![(Self::ART.to_owned(), e)]
    }
}

/// An item's data part: `index` is the item id (the file number), so the base layer, which
/// knows members by member index, is never asked.
fn item_key(index: u32) -> PartKey {
    PartKey {
        art: ItemEntity::ART,
        index,
        part: "item",
    }
}

impl EntitySchema for ItemEntity {
    const ART: &'static str = "items";

    /// Item id `index` carrying the data of its member `members[0]` (`None`: the item table
    /// points past the archive). [`export_items`] decides which id carries the data.
    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let data = match member(members, 0) {
            Some(b) => Some(record_member::<Item>(item_key(index), false, b, ctx)?),
            None => None,
        };
        Ok(ItemEntity {
            id: entity_id(ctx.symbols, SymbolKind::Item, index),
            data,
            same_member_as: None,
        })
    }

    /// The member bytes this id carries (`[None]` for `same_member_as`); the checks across
    /// ids are [`import_items`]'.
    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Item, index, &self.id)?;
        let bytes = self
            .data
            .as_ref()
            .map(|m| record_bytes::<Item>(m, "data", item_key(index), member(ctx.current, 0), ctx))
            .transpose()?;
        Ok(vec![bytes])
    }

    fn sample() -> Self {
        ItemEntity {
            id: fallback_id(SymbolKind::Item, 1),
            data: Some(Member::Value(sample_record::<Item>("item"))),
            same_member_as: Some(fallback_id(SymbolKind::Item, 0)),
        }
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        let id = fallback_id(SymbolKind::Item, 1);
        let none = ItemEntity {
            id: id.clone(),
            data: None,
            same_member_as: None,
        };
        let data = ItemEntity {
            id,
            data: Some(Member::Value(minimal_record::<Item>("item"))),
            same_member_as: None,
        };
        vec![(Self::ART.to_owned(), none), (Self::ART.to_owned(), data)]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        let e = ItemEntity {
            id: fallback_id(SymbolKind::Item, 1),
            data: Some(locked_sample()),
            same_member_as: None,
        };
        vec![(Self::ART.to_owned(), e)]
    }
}

/// For each member of `itemData` (`members` of them), the first item id whose table entry
/// (`member_of[id]`) names it; `None` for a member no id names.
pub fn item_owners(member_of: &[u16], members: usize) -> Vec<Option<u32>> {
    let mut owners = vec![None; members];
    for (id, &m) in member_of.iter().enumerate() {
        if let Some(owner) = owners.get_mut(usize::from(m))
            && owner.is_none()
        {
            *owner = Some(id as u32);
        }
    }
    owners
}

/// One entity per item id of the table (`member_of[id]` = member index): the first id of a
/// member carries its data, every later one names that id in `same_member_as`. `members`
/// are the `itemData` members by member index.
pub(crate) fn export_items(
    member_of: &[u16],
    members: &[Option<Vec<u8>>],
    ctx: &mut ExportCtx<'_>,
) -> Result<Vec<ItemEntity>, KernelError> {
    let owners = item_owners(member_of, members.len());
    let mut items = Vec::with_capacity(member_of.len());
    for (id, &m) in member_of.iter().enumerate() {
        let id = id as u32;
        let m = usize::from(m);
        match owners.get(m).copied().flatten() {
            Some(owner) if owner != id => items.push(ItemEntity {
                id: entity_id(ctx.symbols, SymbolKind::Item, id),
                data: None,
                same_member_as: Some(entity_id(ctx.symbols, SymbolKind::Item, owner)),
            }),
            _ => {
                let own = members.get(m).cloned().flatten();
                items.push(ItemEntity::export(id, &[own], ctx)?);
            }
        }
    }
    Ok(items)
}

/// An error of one item id: the id and the message (with its code).
pub(crate) type ItemProblem = (u32, String);

/// The `itemData` members (`members` of them, by member index) from one entity per item id
/// (`items[id]`), checked across ids: each id either carries data or names in
/// `same_member_as` an id with data and the same member, no member gets data twice. A member
/// no id names stays `None`. `ctx.current` holds the current members by member index. The
/// errors name the item id.
pub(crate) fn import_items(
    items: &[ItemEntity],
    member_of: &[u16],
    members: usize,
    ctx: &ImportCtx<'_>,
) -> Result<Vec<Option<Vec<u8>>>, Vec<ItemProblem>> {
    let mut out: Vec<Option<Vec<u8>>> = vec![None; members];
    let mut carrier: Vec<Option<u32>> = vec![None; members];
    let mut errors = Vec::new();
    if items.len() != member_of.len() {
        let id = items.len().min(member_of.len()) as u32;
        errors.push((
            id,
            format!(
                "{SCHEMA_CODE}: {ROOT}: die Item-Tabelle hat {} IDs, es gibt {} Item-Dateien.",
                member_of.len(),
                items.len()
            ),
        ));
    }
    let name = |id: u32| entity_id(ctx.symbols, SymbolKind::Item, id);
    for (id, (item, &m)) in items.iter().zip(member_of).enumerate() {
        let id = id as u32;
        let m = usize::from(m);
        let result = (|| -> Result<(), String> {
            check_id(ctx.symbols, SymbolKind::Item, id, &item.id)?;
            match (&item.data, &item.same_member_as) {
                (Some(_), Some(_)) => Err(format!(
                    "{SCHEMA_CODE}: same_member_as: `data` und `same_member_as` schließen sich aus."
                )),
                (None, None) if m < members => Err(format!(
                    "{SCHEMA_CODE}: {ROOT}: weder `data` noch `same_member_as` (Member {m:04})."
                )),
                (None, None) => Ok(()),
                (Some(_), None) if m >= members => Err(format!(
                    "{SCHEMA_CODE}: data: die ID zeigt auf Member {m:04}, das Archiv hat {members}."
                )),
                (Some(data), None) => {
                    if let Some(other) = carrier[m] {
                        return Err(format!(
                            "{SCHEMA_CODE}: data: Member {m:04} hat schon Daten von `{}`; hier \
                             `same_member_as` schreiben.",
                            name(other)
                        ));
                    }
                    let current = ctx.current.get(m).and_then(|c| c.as_deref());
                    let bytes = record_bytes::<Item>(data, "data", item_key(id), current, ctx)?;
                    carrier[m] = Some(id);
                    out[m] = Some(bytes);
                    Ok(())
                }
                (None, Some(target)) => {
                    let Some(t) = ctx.symbols.number(SymbolKind::Item, target) else {
                        return Err(format!(
                            "{}: same_member_as: `{target}` ist keine bekannte ID (ITEM_…).",
                            super::super::record::UNKNOWN_ID_CODE
                        ));
                    };
                    let target_member = member_of.get(t as usize).map(|&x| usize::from(x));
                    if target_member != Some(m) {
                        return Err(format!(
                            "{SCHEMA_CODE}: same_member_as: `{target}` nutzt {}, diese ID \
                             Member {m:04}.",
                            target_member
                                .map_or("keinen Member".to_owned(), |x| format!("Member {x:04}"))
                        ));
                    }
                    if items.get(t as usize).is_none_or(|i| i.data.is_none()) {
                        return Err(format!(
                            "{SCHEMA_CODE}: same_member_as: `{target}` trägt keine Daten."
                        ));
                    }
                    Ok(())
                }
            }
        })();
        if let Err(e) = result {
            errors.push((id, e));
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        errors.sort_by_key(|(id, _)| *id);
        Err(errors)
    }
}

/// The encounter record name and the layout of `family` (Diamond and Pearl share Platinum's).
fn encounter_key(index: u32) -> PartKey {
    PartKey {
        art: EncounterEntity::ART,
        index,
        part: "encounter",
    }
}

impl EntitySchema for EncounterEntity {
    const ART: &'static str = "encounters";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let key = encounter_key(index);
        let data = match ctx.family {
            Family::Hgss => record_member::<HgssEncounters>(key, true, bytes, ctx)?,
            Family::Pt | Family::Dp => record_member::<PtEncounters>(key, true, bytes, ctx)?,
        };
        Ok(EncounterEntity { data })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = encounter_key(index);
        let current = member(ctx.current, 0);
        let bytes = match ctx.family {
            Family::Hgss => record_bytes::<HgssEncounters>(&self.data, "data", key, current, ctx)?,
            Family::Pt | Family::Dp => {
                record_bytes::<PtEncounters>(&self.data, "data", key, current, ctx)?
            }
        };
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        EncounterEntity {
            data: Member::Value(sample_record::<PtEncounters>("encounter")),
        }
    }

    fn samples() -> Vec<(String, Self)> {
        vec![
            (format!("{}[pt]", Self::ART), Self::sample()),
            (
                format!("{}[hgss]", Self::ART),
                EncounterEntity {
                    data: Member::Value(sample_record::<HgssEncounters>("encounter")),
                },
            ),
        ]
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        vec![
            (
                format!("{}[pt]", Self::ART),
                EncounterEntity {
                    data: Member::Value(minimal_record::<PtEncounters>("encounter")),
                },
            ),
            (
                format!("{}[hgss]", Self::ART),
                EncounterEntity {
                    data: Member::Value(minimal_record::<HgssEncounters>("encounter")),
                },
            ),
        ]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        ["pt", "hgss"]
            .into_iter()
            .map(|f| {
                let e = EncounterEntity {
                    data: locked_sample(),
                };
                (format!("{}[{f}]", Self::ART), e)
            })
            .collect()
    }
}

/// The key of a trainer text member: the art is also the member's name.
fn trainer_text_key(art: &'static str, index: u32) -> PartKey {
    PartKey {
        art,
        index,
        part: art,
    }
}

impl EntitySchema for TrainerTextTableEntity {
    const ART: &'static str = "trainer_text_table";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match TrainerTextTable::read(bytes) {
            Ok(table) if table.write() == bytes => {
                return Ok(TrainerTextTableEntity(Member::Value(
                    TrainerTextTableJson {
                        entries: table.entries.iter().map(|&(a, b)| [a, b]).collect(),
                    },
                )));
            }
            Ok(_) => format!(
                "{} {index:04}: does not write back byte for byte",
                Self::ART
            ),
            Err(_) => format!(
                "{} {index:04}: {} bytes, not a multiple of 4",
                Self::ART,
                bytes.len()
            ),
        };
        let key = trainer_text_key(Self::ART, index);
        Ok(TrainerTextTableEntity(lock_member(
            key, true, bytes, reason, ctx,
        )?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = trainer_text_key(Self::ART, index);
        let bytes = member_bytes(&self.0, ROOT, key, member(ctx.current, 0), ctx, |j| {
            let table = TrainerTextTable {
                entries: j.entries.iter().map(|&[a, b]| (a, b)).collect(),
            };
            Ok(table.write())
        })?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        TrainerTextTableEntity(Member::Value(TrainerTextTableJson {
            entries: vec![[0, 0]],
        }))
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        Self::samples()
    }

    fn locked_samples() -> Vec<(String, Self)> {
        vec![(
            Self::ART.to_owned(),
            TrainerTextTableEntity(locked_sample()),
        )]
    }
}

impl EntitySchema for TrainerTextOffsetsEntity {
    const ART: &'static str = "trainer_text_offsets";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, 0, Self::ART, index)?;
        let reason = match TrainerTextOffsets::read(bytes) {
            Ok(offsets) if offsets.write() == bytes => {
                return Ok(TrainerTextOffsetsEntity(Member::Value(
                    TrainerTextOffsetsJson {
                        offsets: offsets.offsets,
                    },
                )));
            }
            Ok(_) => format!(
                "{} {index:04}: does not write back byte for byte",
                Self::ART
            ),
            Err(_) => format!("{} {index:04}: {} bytes, odd", Self::ART, bytes.len()),
        };
        let key = trainer_text_key(Self::ART, index);
        Ok(TrainerTextOffsetsEntity(lock_member(
            key, true, bytes, reason, ctx,
        )?))
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let key = trainer_text_key(Self::ART, index);
        let bytes = member_bytes(&self.0, ROOT, key, member(ctx.current, 0), ctx, |j| {
            let offsets = TrainerTextOffsets {
                offsets: j.offsets.clone(),
            };
            Ok(offsets.write())
        })?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        TrainerTextOffsetsEntity(Member::Value(TrainerTextOffsetsJson { offsets: vec![0] }))
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        Self::samples()
    }

    fn locked_samples() -> Vec<(String, Self)> {
        vec![(
            Self::ART.to_owned(),
            TrainerTextOffsetsEntity(locked_sample()),
        )]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use proptest::prelude::*;
    use serde_json::json;

    use super::super::super::canon::to_canonical;
    use super::super::super::part::Part;
    use super::super::super::part::tests::MemBase;
    use super::super::testkit::{code_and_path, export, import, symbols};
    use super::*;
    use g4_formats::data::egg_moves::EggMoves;

    fn learnset_bytes_of(moves: &[(u16, u8)], terminated: bool, tail: &[u8]) -> Vec<u8> {
        let entries: Vec<LearnEntry> = moves
            .iter()
            .map(|&(move_id, level)| LearnEntry { move_id, level })
            .collect();
        encode_learnset(&entries, terminated, tail)
    }

    #[test]
    fn species_entity_joins_personal_learnset_evolution() {
        let mut personal = vec![0u8; 44];
        personal[0] = 45;
        personal[0x06] = 12; // TYPE_GRASS
        let learnset = learnset_bytes_of(&[(33, 1), (45, 3)], true, &[0, 0]);
        let mut evolution = vec![0u8; 44];
        evolution[0..6].copy_from_slice(&[4, 0, 16, 0, 2, 0]); // level 16 → 2
        let members = vec![
            Some(personal.clone()),
            Some(learnset.clone()),
            Some(evolution.clone()),
        ];
        let (e, side) = export::<SpeciesEntity>(Family::Pt, 1, &members);
        assert!(side.is_empty());
        assert_eq!(e.id, "SPECIES_BULBASAUR");
        let text = String::from_utf8(to_canonical(&e)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["personal"]["hp"], json!(45));
        assert_eq!(v["personal"]["type1"], json!("TYPE_GRASS"));
        assert_eq!(
            v["learnset"]["moves"][0],
            json!({"level": 1, "move": "MOVE_TACKLE"})
        );
        assert_eq!(v["learnset"]["terminated"], json!(true));
        assert_eq!(v["learnset"].get("tail"), None, "the padding of two moves");
        assert_eq!(v["evolution"]["target0"], json!("SPECIES_IVYSAUR"));
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["id", "personal", "learnset", "evolution"]);

        assert_eq!(import(Family::Pt, &e, 1).unwrap(), members);

        // A part the archive does not have stays out of the file and out of the import.
        let (e, _) = export::<SpeciesEntity>(Family::Pt, 1, &[Some(personal.clone()), None]);
        assert_eq!(e.learnset, None);
        assert!(
            !String::from_utf8(to_canonical(&e))
                .unwrap()
                .contains("learnset")
        );
        assert_eq!(
            import(Family::Pt, &e, 1).unwrap(),
            vec![Some(personal), None, None]
        );
    }

    fn egg_table(words: &[u16]) -> EggMoves {
        EggMoves::read(
            &words
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<u8>>(),
        )
        .unwrap()
    }

    /// Species `index` exported from `members` with the egg move table `table`, read back
    /// through canonical JSON.
    fn export_with_eggs(
        index: u32,
        members: &[Option<Vec<u8>>],
        table: Option<&EggMoves>,
    ) -> SpeciesEntity {
        let mut sym = symbols();
        let base = MemBase::default();
        let mut ctx = ExportCtx {
            family: Family::Pt,
            symbols: &mut sym,
            base: &base,
            side: Vec::new(),
            egg_moves: table,
        };
        let e = SpeciesEntity::export(index, members, &mut ctx).unwrap();
        serde_json::from_slice(&to_canonical(&e)).unwrap()
    }

    #[test]
    fn species_export_carries_egg_moves_from_the_table() {
        // Species 1: moves 1 and 2; species 3: a mark without moves; species 2: no entry.
        let table = egg_table(&[20001, 1, 2, 20003, 0xFFFF]);
        let personal = Some(vec![0u8; 44]);
        let e = export_with_eggs(1, std::slice::from_ref(&personal), Some(&table));
        assert_eq!(
            e.egg_moves,
            Some(vec![
                SymRef::Id("MOVE_POUND".into()),
                SymRef::Id("MOVE_KARATE_CHOP".into())
            ])
        );
        let v: serde_json::Value = serde_json::from_slice(&to_canonical(&e)).unwrap();
        assert_eq!(v["egg_moves"], json!(["MOVE_POUND", "MOVE_KARATE_CHOP"]));
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["id", "personal", "egg_moves"]);
        // Without a table, and for a species without an entry: no field.
        assert_eq!(
            export_with_eggs(1, std::slice::from_ref(&personal), None).egg_moves,
            None
        );
        let e = export_with_eggs(2, std::slice::from_ref(&personal), Some(&table));
        assert_eq!(e.egg_moves, None);
        assert!(
            !String::from_utf8(to_canonical(&e))
                .unwrap()
                .contains("egg_moves")
        );
        // A mark without moves is an empty list.
        assert_eq!(
            export_with_eggs(3, std::slice::from_ref(&personal), Some(&table)).egg_moves,
            Some(Vec::new())
        );
    }

    #[test]
    fn species_import_ignores_egg_moves() {
        let mut personal = vec![0u8; 44];
        personal[0] = 45;
        let members = vec![Some(personal), None, None];
        let table = egg_table(&[20001, 1, 2, 0xFFFF]);
        let mut e = export_with_eggs(1, &members, Some(&table));
        assert!(e.egg_moves.is_some());
        assert_eq!(import(Family::Pt, &e, 1).unwrap(), members);
        // Whatever the field holds, even an unknown id: the members stay the same.
        e.egg_moves = Some(vec![SymRef::Id("MOVE_FOO".into()), SymRef::Num(70000)]);
        assert_eq!(import(Family::Pt, &e, 1).unwrap(), members);
        e.egg_moves = None;
        assert_eq!(import(Family::Pt, &e, 1).unwrap(), members);
    }

    #[test]
    fn locked_member_round_trips_as_raw() {
        let personal: Vec<u8> = (0..47).collect();
        let (e, _) = export::<SpeciesEntity>(Family::Pt, 7, &[Some(personal.clone())]);
        match &e.personal {
            Some(Member::Locked { locked, raw }) => {
                assert_eq!(locked, "personal 0007: 47 bytes, expected 44");
                assert_eq!(
                    raw,
                    &Part::Hex {
                        hex: hex(&personal)
                    }
                );
            }
            other => panic!("not locked: {other:?}"),
        }
        assert_eq!(
            import(Family::Pt, &e, 7).unwrap(),
            vec![Some(personal), None, None]
        );

        // A long locked member goes to a side file with the entity's stem.
        let long = vec![0x11u8; 425];
        let (e, side) = export::<EncounterEntity>(Family::Pt, 3, &[Some(long.clone())]);
        assert_eq!(
            e.data,
            Member::Locked {
                locked: "encounter 0003: 425 bytes, expected 424".into(),
                raw: Part::File {
                    file: "0003.encounter.bin".into()
                }
            }
        );
        assert_eq!(side, vec![("0003.encounter.bin".to_owned(), long)]);

        // An odd learnset without terminator does not decode.
        let (e, _) = export::<SpeciesEntity>(Family::Pt, 2, &[None, Some(vec![1, 2, 3])]);
        assert!(
            matches!(&e.learnset, Some(Member::Locked { locked, .. }) if locked.starts_with("learnset 0002: 3 bytes")),
            "{e:?}"
        );
        assert_eq!(
            import(Family::Pt, &e, 2).unwrap(),
            vec![None, Some(vec![1, 2, 3]), None]
        );
    }

    #[test]
    fn locked_member_compares_with_the_base_layer() {
        let long = vec![0x22u8; 200];
        let mut sym = symbols();
        let mut base = MemBase::default();
        let key = encounter_key(4);
        base.0.insert(key, long.clone());
        let mut ctx = ExportCtx {
            family: Family::Pt,
            symbols: &mut sym,
            base: &base,
            side: Vec::new(),
            egg_moves: None,
        };
        let e = EncounterEntity::export(4, &[Some(long.clone())], &mut ctx).unwrap();
        assert!(matches!(
            &e.data,
            Member::Locked {
                raw: Part::Base { .. },
                ..
            }
        ));
        assert!(ctx.side.is_empty());
        let sym = symbols();
        let ctx = ImportCtx {
            family: Family::Pt,
            symbols: &sym,
            dir: Path::new("/nonexistent"),
            base: &base,
            current: &[],
        };
        assert_eq!(e.import(4, &ctx).unwrap(), vec![Some(long)]);
    }

    #[test]
    fn import_checks_the_id() {
        let (mut e, _) = export::<MoveEntity>(Family::Pt, 1, &[Some(vec![0; 16])]);
        assert_eq!(e.id, "MOVE_POUND");
        e.id = "MOVE_0001".into();
        assert!(
            import(Family::Pt, &e, 1).is_ok(),
            "the fallback names the same number"
        );
        e.id = "MOVE_TACKLE".into();
        let err = import(Family::Pt, &e, 1).unwrap_err();
        assert!(err.starts_with("textproject.id-mismatch: "), "{err}");
        assert!(
            err.contains("MOVE_TACKLE") && err.contains("MOVE_POUND"),
            "{err}"
        );
    }

    #[test]
    fn encounters_take_the_layout_of_the_family() {
        let mut hgss = vec![0u8; 196];
        hgss[0x14] = 25; // grass_morning_00
        let (e, _) = export::<EncounterEntity>(Family::Hgss, 0, &[Some(hgss.clone())]);
        let Member::Value(j) = &e.data else { panic!() };
        assert_eq!(
            j.0.iter()
                .find(|(k, _)| k == "grass_morning_00")
                .map(|(_, v)| v),
            Some(&json!("SPECIES_PIKACHU"))
        );
        assert_eq!(import(Family::Hgss, &e, 0).unwrap(), vec![Some(hgss)]);
        // The HGSS JSON is no Platinum record.
        let err = import(Family::Pt, &e, 0).unwrap_err();
        assert!(err.starts_with("textproject.schema: "), "{err}");
    }

    fn item_bytes(price: u8) -> Vec<u8> {
        let mut b = vec![0u8; 34];
        b[0] = price;
        b
    }

    #[test]
    fn items_sharing_a_member_write_same_member_as() {
        // Ids 0 and 5 share member 0; member 4 is named by no id.
        let member_of = [0u16, 1, 2, 3, 5, 0];
        let members: Vec<Option<Vec<u8>>> = (0..6).map(|i| Some(item_bytes(i))).collect();
        assert_eq!(
            item_owners(&member_of, 6),
            vec![Some(0), Some(1), Some(2), Some(3), None, Some(4)]
        );
        let mut sym = symbols();
        let base = MemBase::default();
        let mut ctx = ExportCtx {
            family: Family::Pt,
            symbols: &mut sym,
            base: &base,
            side: Vec::new(),
            egg_moves: None,
        };
        let items = export_items(&member_of, &members, &mut ctx).unwrap();
        assert_eq!(items.len(), 6);
        assert_eq!(items[0].id, "ITEM_NONE");
        assert!(items[0].data.is_some() && items[0].same_member_as.is_none());
        assert_eq!(items[5].data, None);
        assert_eq!(items[5].same_member_as.as_deref(), Some("ITEM_NONE"));
        assert_eq!(
            String::from_utf8(to_canonical(&items[5])).unwrap(),
            format!(
                "{{\n  \"id\": \"{}\",\n  \"same_member_as\": \"ITEM_NONE\"\n}}\n",
                items[5].id
            )
        );
        let items: Vec<ItemEntity> = items
            .iter()
            .map(|i| serde_json::from_slice(&to_canonical(i)).unwrap())
            .collect();

        let sym = symbols();
        let ctx = ImportCtx {
            family: Family::Pt,
            symbols: &sym,
            dir: Path::new("/nonexistent"),
            base: &base,
            current: &[],
        };
        let mut expected = members.clone();
        expected[4] = None;
        assert_eq!(import_items(&items, &member_of, 6, &ctx).unwrap(), expected);

        // `same_member_as` naming an id with another member is refused, naming the id.
        let mut wrong = items.clone();
        wrong[5].same_member_as = Some(items[1].id.clone());
        let errors = import_items(&wrong, &member_of, 6, &ctx).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 5);
        assert!(
            errors[0].1.contains("Member 0001") && errors[0].1.contains("Member 0000"),
            "{errors:?}"
        );

        // Data twice for one member, an unknown id, and neither form.
        let mut twice = items.clone();
        twice[5].same_member_as = None;
        twice[5].data = items[0].data.clone();
        let errors = import_items(&twice, &member_of, 6, &ctx).unwrap_err();
        assert!(errors[0].1.contains("schon Daten"), "{errors:?}");
        let mut unknown = items.clone();
        unknown[5].same_member_as = Some("ITEM_FOO".into());
        let errors = import_items(&unknown, &member_of, 6, &ctx).unwrap_err();
        assert!(
            errors[0].1.starts_with("textproject.unknown-id: "),
            "{errors:?}"
        );
        let mut neither = items.clone();
        neither[5].same_member_as = None;
        let errors = import_items(&neither, &member_of, 6, &ctx).unwrap_err();
        assert!(errors[0].1.contains("weder"), "{errors:?}");
        let errors = import_items(&items[..5], &member_of, 6, &ctx).unwrap_err();
        assert!(errors[0].1.contains("6 IDs"), "{errors:?}");
    }

    #[test]
    fn learnset_import_checks_levels_moves_and_the_tail() {
        let bytes = learnset_bytes_of(&[(1, 5)], true, &[]);
        let (mut e, _) = export::<SpeciesEntity>(Family::Pt, 1, &[None, Some(bytes)]);
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[0].level = 128;
        assert!(import(Family::Pt, &e, 1).unwrap_err().contains("level"));
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[0].level = 127;
        ls.moves[0].move_id = SymRef::Num(511);
        assert!(import(Family::Pt, &e, 1).unwrap_err().contains("0xFFFF"));
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[0].move_id = SymRef::Num(512);
        assert!(import(Family::Pt, &e, 1).unwrap_err().contains("511"));
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[0].move_id = SymRef::Id("MOVE_FOO".into());
        assert!(
            import(Family::Pt, &e, 1)
                .unwrap_err()
                .starts_with("textproject.unknown-id")
        );
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[0].move_id = SymRef::Num(1);
        ls.terminated = false;
        ls.tail = Some("00".into());
        assert!(
            import(Family::Pt, &e, 1)
                .unwrap_err()
                .contains("learnset.tail")
        );
    }

    #[test]
    fn unknown_keys_are_refused_with_their_position() {
        let err =
            serde_json::from_str::<MoveEntity>("{\n  \"id\": \"MOVE_POUND\",\n  \"dta\": {}\n}")
                .unwrap_err();
        assert!(err.to_string().contains("unknown field `dta`"), "{err}");
        assert_eq!(err.line(), 3);
        let err = serde_json::from_str::<SpeciesEntity>(
            "{\"id\": \"SPECIES_NONE\", \"learnset\": {\"moves\": [], \"terminated\": true,\n \"tail\": \"\", \"x\": 1}}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown field `x`"), "{err}");
        assert_eq!(err.line(), 2);
    }

    fn set_key(m: &mut Member<RecordJson>, key: &str, value: serde_json::Value) {
        let Member::Value(j) = m else { panic!() };
        match j.0.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => j.0.push((key.to_owned(), value)),
        }
    }

    #[test]
    fn import_errors_name_the_json_path() {
        let (good, _) = export::<MoveEntity>(Family::Pt, 1, &[Some(vec![0; 16])]);
        let err_of = |e: &MoveEntity| import(Family::Pt, e, 1).unwrap_err();

        let mut e = good.clone();
        set_key(&mut e.data, "powr", json!(1));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.powr")
        );
        let mut e = good.clone();
        let Member::Value(j) = &mut e.data else {
            panic!()
        };
        j.0.retain(|(k, _)| k != "power");
        let err = err_of(&e);
        assert_eq!(code_and_path(&err), ("textproject.schema", "data"), "{err}");
        assert!(err.contains("`power`"), "{err}");
        let mut e = good.clone();
        set_key(&mut e.data, "power", json!(256));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "data.power")
        );
        let mut e = good.clone();
        set_key(&mut e.data, "move_type", json!("TYPE_FOO"));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.unknown-id", "data.move_type")
        );
        let mut e = good.clone();
        e.id = "MOVE_TACKLE".into();
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.id-mismatch", "id")
        );
        let mut e = good;
        e.data = Member::Locked {
            locked: "x".into(),
            raw: Part::File {
                file: "0001.move.bin".into(),
            },
        };
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.side-file", "data.raw.file")
        );

        let learnset = learnset_bytes_of(&[(1, 5), (2, 7)], true, &[0, 0]);
        let members = [Some(vec![0; 44]), Some(learnset), Some(vec![0; 44])];
        let (good, _) = export::<SpeciesEntity>(Family::Pt, 1, &members);
        let err_of = |e: &SpeciesEntity| import(Family::Pt, e, 1).unwrap_err();
        let mut e = good.clone();
        set_key(e.personal.as_mut().unwrap(), "type1", json!("TYPE_FOO"));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.unknown-id", "personal.type1")
        );
        let mut e = good.clone();
        set_key(e.evolution.as_mut().unwrap(), "targt0", json!(1));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "evolution.targt0")
        );
        let mut e = good.clone();
        let Some(Member::Value(j)) = e.personal.as_mut() else {
            panic!()
        };
        j.0.retain(|(k, _)| k != "hp");
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "personal")
        );
        let mut e = good.clone();
        set_key(e.personal.as_mut().unwrap(), "hp", json!(-1));
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.schema", "personal.hp")
        );
        let mut e = good;
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves[1].move_id = SymRef::Id("MOVE_FOO".into());
        assert_eq!(
            code_and_path(&err_of(&e)),
            ("textproject.unknown-id", "learnset.moves[1].move")
        );
    }

    #[test]
    fn trainer_text_that_does_not_decode_is_locked() {
        let table = vec![1u8, 2, 3, 4, 5];
        let (e, _) = export::<TrainerTextTableEntity>(Family::Pt, 0, &[Some(table.clone())]);
        assert_eq!(
            e.0,
            Member::Locked {
                locked: "trainer_text_table 0000: 5 bytes, not a multiple of 4".into(),
                raw: Part::Hex { hex: hex(&table) },
            }
        );
        assert_eq!(import(Family::Pt, &e, 0).unwrap(), vec![Some(table)]);

        let offsets = vec![9u8, 8, 7];
        let (e, _) = export::<TrainerTextOffsetsEntity>(Family::Hgss, 0, &[Some(offsets.clone())]);
        assert_eq!(
            e.0,
            Member::Locked {
                locked: "trainer_text_offsets 0000: 3 bytes, odd".into(),
                raw: Part::Hex { hex: hex(&offsets) },
            }
        );
        assert_eq!(import(Family::Hgss, &e, 0).unwrap(), vec![Some(offsets)]);

        // The value form is the plain object, as before.
        let (e, _) = export::<TrainerTextOffsetsEntity>(Family::Pt, 0, &[Some(vec![1, 0])]);
        assert_eq!(to_canonical(&e), b"{\n  \"offsets\": [\n    1\n  ]\n}\n");
        let err = serde_json::from_str::<TrainerTextOffsetsEntity>("{\"offset\": []}").unwrap_err();
        assert!(err.to_string().contains("unknown field `offset`"), "{err}");
    }

    #[test]
    fn learnset_tail_is_left_out_when_it_is_the_padding() {
        let tail_of = |moves: &[(u16, u8)], terminated: bool, tail: &[u8]| {
            let bytes = learnset_bytes_of(moves, terminated, tail);
            let (e, _) = export::<SpeciesEntity>(Family::Pt, 1, &[None, Some(bytes.clone())]);
            assert_eq!(import(Family::Pt, &e, 1).unwrap()[1], Some(bytes));
            let Some(Member::Value(ls)) = e.learnset else {
                panic!()
            };
            ls.tail
        };
        assert_eq!(tail_of(&[(1, 1), (2, 2)], true, &[0, 0]), None);
        assert_eq!(tail_of(&[(1, 1)], true, &[]), None);
        assert_eq!(tail_of(&[], true, &[0, 0]), None);
        assert_eq!(tail_of(&[(1, 1)], false, &[]), None);
        assert_eq!(tail_of(&[(1, 1)], true, &[0, 0]).as_deref(), Some("0000"));
        assert_eq!(tail_of(&[(1, 1), (2, 2)], true, &[]).as_deref(), Some(""));
        assert_eq!(
            tail_of(&[(1, 1), (2, 2)], true, &[0, 1]).as_deref(),
            Some("0001")
        );

        // Without `tail` an added move is padded like `Learnset::write`.
        let (mut e, _) = export::<SpeciesEntity>(
            Family::Pt,
            1,
            &[None, Some(learnset_bytes_of(&[(1, 1)], true, &[]))],
        );
        let Some(Member::Value(ls)) = e.learnset.as_mut() else {
            panic!()
        };
        ls.moves.push(LearnMoveJson {
            level: 2,
            move_id: SymRef::Num(2),
        });
        assert_eq!(
            import(Family::Pt, &e, 1).unwrap()[1],
            Some(vec![1, 2, 2, 4, 0xFF, 0xFF, 0, 0])
        );
    }

    proptest! {
        #[test]
        fn prop_learnset_round_trip(
            moves in proptest::collection::vec((0u16..512, 0u8..128), 0..25),
            terminated in any::<bool>(),
            tail in proptest::collection::vec(any::<u8>(), 0..6),
            padded in any::<bool>(),
        ) {
            let moves: Vec<(u16, u8)> =
                moves.into_iter().filter(|&m| m != (511, 127)).collect();
            let tail = match (terminated, padded) {
                (false, _) => Vec::new(),
                (true, true) => learnset_padding(moves.len()),
                (true, false) => tail,
            };
            let bytes = learnset_bytes_of(&moves, terminated, &tail);
            let members = vec![None, Some(bytes)];
            let (e, _) = export::<SpeciesEntity>(Family::Pt, 1, &members);
            prop_assert!(matches!(e.learnset, Some(Member::Value(_))));
            prop_assert_eq!(import(Family::Pt, &e, 1).unwrap(), vec![None, members[1].clone(), None]);
        }

        #[test]
        fn prop_trainer_text_round_trip(
            entries in proptest::collection::vec(any::<(u16, u16)>(), 0..40),
            offsets in proptest::collection::vec(any::<u16>(), 0..40),
        ) {
            let table = TrainerTextTable { entries }.write();
            let (e, _) = export::<TrainerTextTableEntity>(Family::Pt, 0, &[Some(table.clone())]);
            prop_assert_eq!(import(Family::Pt, &e, 0).unwrap(), vec![Some(table)]);
            let offs = TrainerTextOffsets { offsets }.write();
            let (e, _) = export::<TrainerTextOffsetsEntity>(Family::Hgss, 0, &[Some(offs.clone())]);
            prop_assert_eq!(import(Family::Hgss, &e, 0).unwrap(), vec![Some(offs)]);
        }
    }
}
