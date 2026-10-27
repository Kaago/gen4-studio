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
use super::super::manifest::SCHEMA_CODE;
use super::super::part::{Member, PartKey};
use super::super::record::{record_from_json, record_to_json, resolve, sample_record, sym_ref};
use super::super::symbols::{SymbolKind, fallback_id};
use super::{
    EntitySchema, ExportCtx, ImportCtx, RecordJson, SymRef, check_id, entity_id, lock_member,
    member_bytes,
};

/// One species: its `personalPokeData`, `learnsets` and `evolutions` members. A part is
/// missing only when its archive has no member of that index.
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
}

/// A learnset: the level-up moves, whether `0xFFFF` ends them, and the bytes after it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearnsetJson {
    pub moves: Vec<LearnMoveJson>,
    pub terminated: bool,
    pub tail: String,
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

/// `trainerTextTable/0000`: `[trainer, trigger]` per message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerTextTableEntity {
    pub entries: Vec<[u16; 2]>,
}

/// `trainerTextOffset/0000`: one table offset per trainer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerTextOffsetsEntity {
    pub offsets: Vec<u16>,
}

/// `members[i]` as bytes, if there.
fn member(members: &[Option<Vec<u8>>], i: usize) -> Option<&[u8]> {
    members.get(i).and_then(|m| m.as_deref())
}

/// Member `i` of an art that has exactly one member per entity; missing is an error.
fn required<'m>(
    members: &'m [Option<Vec<u8>>],
    art: &str,
    index: u32,
) -> Result<&'m [u8], KernelError> {
    member(members, 0)
        .ok_or_else(|| KernelError::InvalidProject(format!("{art} {index:04}: der Member fehlt.")))
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

/// The bytes of a fixed record member.
fn record_bytes<R: RawRecord>(
    m: &Member<RecordJson>,
    key: PartKey,
    current: Option<&[u8]>,
    ctx: &ImportCtx<'_>,
) -> Result<Vec<u8>, String> {
    member_bytes(m, key, current, ctx, |j| {
        record_from_json::<R>(j, key.part, ctx.symbols).map(|r| r.write())
    })
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
            return Ok(Member::Value(LearnsetJson {
                moves,
                terminated: ls.terminated,
                tail: hex(&ls.tail),
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
    let tail = unhex(&j.tail).map_err(|e| format!("{SCHEMA_CODE}: learnset.tail: {e}"))?;
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
        Ok(SpeciesEntity {
            id: entity_id(ctx.symbols, SymbolKind::Species, index),
            personal,
            learnset,
            evolution,
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
            .map(|m| record_bytes::<Personal>(m, key("personal"), current(0), ctx))
            .transpose()?;
        let learnset = self
            .learnset
            .as_ref()
            .map(|m| {
                member_bytes(m, key("learnset"), current(1), ctx, |j| {
                    learnset_bytes(j, ctx)
                })
            })
            .transpose()?;
        let evolution = self
            .evolution
            .as_ref()
            .map(|m| record_bytes::<Evolution>(m, key("evolution"), current(2), ctx))
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
                tail: "0000".into(),
            })),
            evolution: Some(Member::Value(sample_record::<Evolution>("evolution"))),
        }
    }
}

impl EntitySchema for MoveEntity {
    const ART: &'static str = "moves";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let bytes = required(members, Self::ART, index)?;
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
        let bytes = record_bytes::<Move>(&self.data, key, member(ctx.current, 0), ctx)?;
        Ok(vec![Some(bytes)])
    }

    fn sample() -> Self {
        MoveEntity {
            id: fallback_id(SymbolKind::Move, 1),
            data: Member::Value(sample_record::<Move>("move")),
        }
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
            .map(|m| record_bytes::<Item>(m, item_key(index), member(ctx.current, 0), ctx))
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
                "{SCHEMA_CODE}: die Item-Tabelle hat {} IDs, es gibt {} Item-Dateien.",
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
                    "{SCHEMA_CODE}: `data` und `same_member_as` schließen sich aus."
                )),
                (None, None) if m < members => Err(format!(
                    "{SCHEMA_CODE}: weder `data` noch `same_member_as` (Member {m:04})."
                )),
                (None, None) => Ok(()),
                (Some(_), None) if m >= members => Err(format!(
                    "{SCHEMA_CODE}: die ID zeigt auf Member {m:04}, das Archiv hat {members}."
                )),
                (Some(data), None) => {
                    if let Some(other) = carrier[m] {
                        return Err(format!(
                            "{SCHEMA_CODE}: Member {m:04} hat schon Daten von `{}`; hier \
                             `same_member_as` schreiben.",
                            name(other)
                        ));
                    }
                    let current = ctx.current.get(m).and_then(|c| c.as_deref());
                    let bytes = record_bytes::<Item>(data, item_key(id), current, ctx)?;
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
        let bytes = required(members, Self::ART, index)?;
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
            Family::Hgss => record_bytes::<HgssEncounters>(&self.data, key, current, ctx)?,
            Family::Pt | Family::Dp => record_bytes::<PtEncounters>(&self.data, key, current, ctx)?,
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
}

impl EntitySchema for TrainerTextTableEntity {
    const ART: &'static str = "trainer_text_table";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        _ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let table = TrainerTextTable::read(required(members, Self::ART, index)?)?;
        Ok(TrainerTextTableEntity {
            entries: table.entries.iter().map(|&(a, b)| [a, b]).collect(),
        })
    }

    fn import(&self, _index: u32, _ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let table = TrainerTextTable {
            entries: self.entries.iter().map(|&[a, b]| (a, b)).collect(),
        };
        Ok(vec![Some(table.write())])
    }

    fn sample() -> Self {
        TrainerTextTableEntity {
            entries: vec![[0, 0]],
        }
    }
}

impl EntitySchema for TrainerTextOffsetsEntity {
    const ART: &'static str = "trainer_text_offsets";

    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        _ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let offsets = TrainerTextOffsets::read(required(members, Self::ART, index)?)?;
        Ok(TrainerTextOffsetsEntity {
            offsets: offsets.offsets,
        })
    }

    fn import(&self, _index: u32, _ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        let offsets = TrainerTextOffsets {
            offsets: self.offsets.clone(),
        };
        Ok(vec![Some(offsets.write())])
    }

    fn sample() -> Self {
        TrainerTextOffsetsEntity { offsets: vec![0] }
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
    use super::super::super::registry::Symbols;
    use super::*;

    fn symbols() -> Symbols {
        Symbols::load(Path::new("/nonexistent/g4-textproject")).unwrap()
    }

    /// Exports entity `index` of `E` from `members` and reads it back through canonical JSON.
    fn export<E: EntitySchema>(
        family: Family,
        index: u32,
        members: &[Option<Vec<u8>>],
    ) -> (E, Vec<(String, Vec<u8>)>) {
        let mut sym = symbols();
        let base = MemBase::default();
        let mut ctx = ExportCtx {
            family,
            symbols: &mut sym,
            base: &base,
            side: Vec::new(),
        };
        let e = E::export(index, members, &mut ctx).unwrap();
        let again: E = serde_json::from_slice(&to_canonical(&e)).unwrap();
        (again, ctx.side)
    }

    fn import<E: EntitySchema>(
        family: Family,
        e: &E,
        index: u32,
    ) -> Result<Vec<Option<Vec<u8>>>, String> {
        let sym = symbols();
        let base = MemBase::default();
        let ctx = ImportCtx {
            family,
            symbols: &sym,
            dir: Path::new("/nonexistent"),
            base: &base,
            current: &[],
        };
        e.import(index, &ctx)
    }

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
        assert_eq!(v["learnset"]["tail"], json!("0000"));
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
        ls.tail = "00".into();
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

    proptest! {
        #[test]
        fn prop_learnset_round_trip(
            moves in proptest::collection::vec((0u16..512, 0u8..128), 0..25),
            terminated in any::<bool>(),
            tail in proptest::collection::vec(any::<u8>(), 0..6),
        ) {
            let moves: Vec<(u16, u8)> =
                moves.into_iter().filter(|&m| m != (511, 127)).collect();
            let tail = if terminated { tail } else { Vec::new() };
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
