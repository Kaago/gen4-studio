//! `data_edit`: one patch as one undo step, with a speaking label ("Staravia: Angriff 75 →
//! 85") unless the caller brings its own.

use g4_formats::data::evolution::{EVO_ENTRIES, EvoEntry};
use g4_formats::data::learnset::LearnEntry;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::{item::Item, moves::Move};
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::{ChangeId, Store};

use super::dto::{DataKind, DataPatch, LearnEntryDto};
use super::encounter::EncounterEdit;
use super::fields::{fits, get, numeric_span, set};
use super::{display, item_member, labels, shared_with};
use crate::errors::CmdError;
use crate::state::Session;

/// TM/HM bits in use: TM01..TM92, HM01..HM08.
const MACHINES: usize = 100;
const TMS: usize = 92;

fn asset(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

/// "TM40" or "VM01" for machine `n` (0-based).
pub fn machine_name(n: usize) -> String {
    if n < TMS {
        format!("TM{:02}", n + 1)
    } else {
        format!("VM{:02}", n - TMS + 1)
    }
}

/// The field `field` of a record of type `R`, checked against the label table (the editable
/// fields) and the value against the field's width. Returns the span and the German name.
fn checked<R: RawRecord>(
    table: &[(&str, &'static str)],
    field: &str,
    value: i64,
) -> Result<(&'static g4_formats::data::record::FieldSpan, &'static str), CmdError> {
    let name = labels::label(table, field).ok_or_else(|| format!("no editable field {field}"))?;
    let span = numeric_span(R::FIELDS, field).ok_or_else(|| format!("no field {field}"))?;
    if !fits(span, value) {
        return Err(format!("{value} does not fit {field}").into());
    }
    Ok((span, name))
}

/// The assets of `ids` whose bytes differ from `before` (taken before the change).
fn changed(store: &Store, ids: &[AssetId], before: &[Option<Vec<u8>>]) -> Vec<AssetId> {
    ids.iter()
        .zip(before)
        .filter(|(id, old)| store.current_bytes(**id).ok().flatten() != **old)
        .map(|(id, _)| *id)
        .collect()
}

/// The asset of a one-part edit when it recorded a change.
fn one(id: AssetId, change: Option<ChangeId>) -> Vec<AssetId> {
    change.map(|_| vec![id]).unwrap_or_default()
}

/// Applies `patch` to member `index` of `kind` (a species kind takes the personal, TM,
/// learnset, evolution and copy patches; `move`, `item` and `encounter` their own) and
/// returns the assets it changed (empty: nothing changed, nothing recorded).
pub fn apply(
    session: &mut Session,
    kind: DataKind,
    index: u16,
    patch: DataPatch,
    label: Option<String>,
) -> Result<Vec<AssetId>, CmdError> {
    let Session { store, names, .. } = session;
    let species_kind = matches!(
        kind,
        DataKind::Species | DataKind::Learnset | DataKind::Evolution
    );
    let label_or = |default: String| label.clone().unwrap_or(default);
    match patch {
        DataPatch::Personal { field, value } if species_kind => {
            let value = i64::from(value);
            let (span, field_name) = checked::<Personal>(labels::SPECIES, &field, value)?;
            let old = get(store.personal(index)?, span);
            let name = display::species(names, store, index);
            let label = label_or(format!("{name}: {field_name} {old} → {value}"));
            let change = store.edit_personal(index, &label, |p| set(p, span, value))?;
            Ok(one(asset(AssetKind::Personal, index), change))
        }
        DataPatch::Tm { n, on } if species_kind => {
            let n = usize::from(n);
            if n >= MACHINES {
                return Err(format!("there are {MACHINES} machines, none at {n}").into());
            }
            let old = store.personal(index)?.tm(n);
            let word = |b: bool| if b { "an" } else { "aus" };
            let name = display::species(names, store, index);
            let label = label_or(format!(
                "{name}: {} {} → {}",
                machine_name(n),
                word(old),
                word(on)
            ));
            let change = store.edit_personal(index, &label, |p| p.set_tm(n, on))?;
            Ok(one(asset(AssetKind::Personal, index), change))
        }
        DataPatch::Learnset { entries } if species_kind => {
            if let Some(bad) = entries.iter().find(|e| e.move_id > 0x1FF || e.level > 0x7F) {
                return Err(format!(
                    "move {} at level {} does not fit a learnset entry (move up to 511, level up to 127)",
                    bad.move_id, bad.level
                )
                .into());
            }
            let old = store.learnset(index)?.entries.len();
            let name = display::species(names, store, index);
            let label = label_or(if old == entries.len() {
                format!("{name}: Lernset geändert")
            } else {
                format!("{name}: Lernset {old} → {} Attacken", entries.len())
            });
            let entries: Vec<LearnEntry> = entries
                .iter()
                .map(|e: &LearnEntryDto| LearnEntry {
                    move_id: e.move_id,
                    level: e.level,
                })
                .collect();
            let change = store.edit_learnset(index, &label, |l| l.entries = entries)?;
            Ok(one(asset(AssetKind::Learnset, index), change))
        }
        DataPatch::Evolution {
            slot,
            method,
            param,
            target,
        } if species_kind => {
            let slot = usize::from(slot);
            if slot >= EVO_ENTRIES {
                return Err(
                    format!("there are {EVO_ENTRIES} evolution entries, none at {slot}").into(),
                );
            }
            let old = store.evolution(index)?.entry(slot);
            let new = EvoEntry {
                method,
                param,
                target,
            };
            let diffs: Vec<String> = [
                ("Methode", old.method, method),
                ("Parameter", old.param, param),
                ("Ziel", old.target, target),
            ]
            .into_iter()
            .filter(|(_, a, b)| a != b)
            .map(|(what, a, b)| format!("{what} {a} → {b}"))
            .collect();
            let name = display::species(names, store, index);
            let detail = match diffs.as_slice() {
                [one] => one.clone(),
                _ => "geändert".to_owned(),
            };
            let label = label_or(format!("{name}: Entwicklung {} {detail}", slot + 1));
            let change = store.edit_evolution(index, &label, |e| e.set_entry(slot, new))?;
            Ok(one(asset(AssetKind::Evolution, index), change))
        }
        DataPatch::CopyMoves { from } if species_kind => {
            let ids = [
                asset(AssetKind::Personal, index),
                asset(AssetKind::Learnset, index),
            ];
            // Loads both parts (and reports a locked one) before the bytes are compared.
            store.personal(index)?;
            store.learnset(index)?;
            let before: Vec<Option<Vec<u8>>> = ids
                .iter()
                .map(|&id| store.current_bytes(id))
                .collect::<Result<_, _>>()?;
            let name = display::species(names, store, index);
            let source = display::species(names, store, from);
            let label = label_or(format!("{name}: TM/VM und Lernset wie {source}"));
            store.transaction(&label, |tx| {
                let p = tx.personal(from)?;
                let tm: [bool; MACHINES] = std::array::from_fn(|n| p.tm(n));
                let entries = tx.learnset(from)?.entries.clone();
                tx.edit_personal(index, |p| {
                    for (n, on) in tm.into_iter().enumerate() {
                        p.set_tm(n, on);
                    }
                })?;
                tx.edit_learnset(index, |l| l.entries = entries)
            })?;
            Ok(changed(store, &ids, &before))
        }
        DataPatch::Move { field, value } if kind == DataKind::Move => {
            let (span, field_name) = checked::<Move>(labels::MOVE, &field, value)?;
            let old = get(store.move_data(index)?, span);
            let name = display::move_name(names, store, index);
            let label = label_or(format!("{name}: {field_name} {old} → {value}"));
            let change = store.edit_move(index, &label, |m| set(m, span, value))?;
            Ok(one(asset(AssetKind::Move, index), change))
        }
        DataPatch::Item { field, value } if kind == DataKind::Item => {
            let (span, field_name) = checked::<Item>(labels::ITEM, &field, value)?;
            let project = store.project();
            let (member, id) = item_member(project, index)?;
            let shared = id
                .map(|id| shared_with(project, id, member))
                .unwrap_or_default();
            let old = get(store.item(member)?, span);
            let name = display::item(names, store, index, id.is_some());
            let mut default = format!("{name}: {field_name} {old} → {value}");
            if !shared.is_empty() {
                default.push_str(&format!(", gilt für {} Gegenstände", 1 + shared.len()));
            }
            let label = label_or(default);
            let change = store.edit_item(member, &label, |i: &mut Item| set(i, span, value))?;
            Ok(one(asset(AssetKind::Item, member), change))
        }
        DataPatch::Encounter {
            method,
            slot,
            field,
            value,
        } if kind == DataKind::Encounter => {
            let edit = EncounterEdit {
                method: &method,
                slot,
                field: &field,
                value,
            };
            // Checked on a copy first: the codec's accessors panic on a wrong index.
            let mut copy = store.encounter(index)?.clone();
            let old = edit.apply(&mut copy)?;
            let method_name = labels::label(labels::ENCOUNTER, &method).unwrap_or(&method);
            let field_name = labels::label(labels::ENCOUNTER, &field).unwrap_or(&field);
            let place = match slot {
                Some(slot) if field != "rate" => format!(" Platz {}", slot + 1),
                _ => String::new(),
            };
            let label = label_or(format!(
                "Begegnungen {index:04}: {method_name}{place} {field_name} {old} → {value}"
            ));
            let change = store.edit_encounter(index, &label, |e| *e = copy)?;
            Ok(one(asset(AssetKind::Encounter, index), change))
        }
        patch => Err(format!("{patch:?} does not apply to {kind:?}").into()),
    }
}
