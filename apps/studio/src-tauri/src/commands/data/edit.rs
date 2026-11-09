//! `data_edit`: one patch as one undo step, with a speaking label ("Staravia: Attack 75 →
//! 85", in German "Staravia: Angriff 75 → 85") unless the caller brings its own.

use g4_formats::data::egg_moves::SPECIES_BASE;
use g4_formats::data::evolution::{EVO_ENTRIES, EvoEntry};
use g4_formats::data::learnset::{LearnEntry, MAX_ENTRIES};
use g4_formats::data::personal::Personal;
use g4_formats::data::record::RawRecord;
use g4_formats::data::{item::Item, moves::Move};
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::{ChangeId, EGG_MOVES, Store};

use super::dto::{DataKind, DataPatch, EGG_MOVES_MAX, LearnEntryDto};
use super::encounter::{EncounterEdit, apply_extra, swap};
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

/// "TM40" or "HM01" ("VM01" in German) for machine `n` (0-based).
pub fn machine_name(n: usize) -> String {
    if n < TMS {
        t!("commands.data.machine_tm", n = n + 1 : {:02}).into_owned()
    } else {
        t!("commands.data.machine_hm", n = n - TMS + 1 : {:02}).into_owned()
    }
}

/// The field `field` of a record of type `R`, checked against the label table (the editable
/// fields) and the value against the field's width. Returns the span and the name in the
/// language of the process.
fn checked<R: RawRecord>(
    table: &[(&str, &str)],
    field: &str,
    value: i64,
) -> Result<(&'static g4_formats::data::record::FieldSpan, String), CmdError> {
    let name = labels::label(table, field).ok_or_else(|| format!("no editable field {field}"))?;
    let span = numeric_span(R::FIELDS, field).ok_or_else(|| format!("no field {field}"))?;
    if !fits(span, value) {
        return Err(format!("{value} does not fit {field}").into());
    }
    Ok((span, name))
}

/// "on" or "off" ("an", "aus" in German), for the label of a switched bit.
pub(super) fn on_off(on: bool) -> String {
    if on {
        t!("commands.data.on").into_owned()
    } else {
        t!("commands.data.off").into_owned()
    }
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

/// Refuses an egg move list the table cannot hold for species `index`: species 0 or one
/// beyond the personal members, more than [`EGG_MOVES_MAX`] moves, move 0 or one the game
/// does not have (a move of [`SPECIES_BASE`] or more would also be written as a species mark).
fn check_egg_moves(store: &Store, index: u16, moves: &[u16]) -> Result<(), CmdError> {
    let project = store.project();
    let species = project.member_count(AssetKind::Personal)?;
    if index == 0 || usize::from(index) >= species {
        return Err(format!(
            "species {index} has no egg moves (species 1 to {})",
            species.saturating_sub(1)
        )
        .into());
    }
    if moves.len() > EGG_MOVES_MAX {
        return Err(format!(
            "{} egg moves, a species takes at most {EGG_MOVES_MAX}",
            moves.len()
        )
        .into());
    }
    let count = project.member_count(AssetKind::Move)?;
    if let Some(bad) = moves
        .iter()
        .find(|&&m| m == 0 || usize::from(m) >= count || m >= SPECIES_BASE)
    {
        return Err(format!(
            "move {bad} is no egg move (moves 1 to {})",
            count.saturating_sub(1)
        )
        .into());
    }
    Ok(())
}

/// Applies `patch` to member `index` of `kind` (a species kind takes the personal, TM,
/// learnset, evolution, egg move and copy patches; `move`, `item` and `encounter` their own) and
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
            let name = display::species(names, store, index);
            let label = label_or(format!(
                "{name}: {} {} → {}",
                machine_name(n),
                on_off(old),
                on_off(on)
            ));
            let change = store.edit_personal(index, &label, |p| p.set_tm(n, on))?;
            Ok(one(asset(AssetKind::Personal, index), change))
        }
        DataPatch::Learnset { entries } if species_kind => {
            if entries.len() > MAX_ENTRIES {
                return Err(format!(
                    "learnset of {} entries, the game reads at most {MAX_ENTRIES}",
                    entries.len()
                )
                .into());
            }
            // Bounding the move by the move members also keeps an entry from encoding as the
            // terminator (move 511 at level 127).
            let moves = store.project().member_count(AssetKind::Move)?;
            if let Some(bad) = entries
                .iter()
                .find(|e| usize::from(e.move_id) >= moves || e.level > 0x7F)
            {
                return Err(format!(
                    "move {} at level {} does not fit a learnset entry (moves 0 to {}, levels up to 127)",
                    bad.move_id,
                    bad.level,
                    moves.saturating_sub(1)
                )
                .into());
            }
            let old = store.learnset(index)?.entries.len();
            let name = display::species(names, store, index);
            let label = label_or(if old == entries.len() {
                t!("commands.data.learnset_changed", name = name).into_owned()
            } else {
                t!(
                    "commands.data.learnset_count",
                    name = name,
                    old = old,
                    new = entries.len()
                )
                .into_owned()
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
                (t!("commands.data.evolution_method"), old.method, method),
                (t!("commands.data.evolution_param"), old.param, param),
                (t!("commands.data.evolution_target"), old.target, target),
            ]
            .into_iter()
            .filter(|(_, a, b)| a != b)
            .map(|(what, a, b)| format!("{what} {a} → {b}"))
            .collect();
            let name = display::species(names, store, index);
            let detail = match diffs.as_slice() {
                [one] => one.clone(),
                _ => t!("commands.data.evolution_changed").into_owned(),
            };
            let label = label_or(
                t!(
                    "commands.data.evolution_label",
                    name = name,
                    slot = slot + 1,
                    detail = detail
                )
                .into_owned(),
            );
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
            let label =
                label_or(t!("commands.data.copy_moves", name = name, source = source).into_owned());
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
        DataPatch::EggMoves { moves } if species_kind => {
            check_egg_moves(store, index, &moves)?;
            let old = store.egg_moves()?.moves_of(index).unwrap_or_default().len();
            let name = display::species(names, store, index);
            let what = labels::egg_moves();
            let label = label_or(if old == moves.len() {
                t!("commands.data.egg_moves_changed", name = name, what = what).into_owned()
            } else {
                format!("{name}: {what} {old} → {}", moves.len())
            });
            // Over Platinum's budget (or losing the signature) the store refuses with its
            // sentence and records nothing.
            let change = store.edit_egg_moves(&label, |t| t.set_moves(index, moves))?;
            Ok(one(EGG_MOVES, change))
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
                default.push_str(&t!("commands.data.item_shared", count = 1 + shared.len()));
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
            let method_name = labels::label(labels::ENCOUNTER, &method).unwrap_or(method.clone());
            let field_name = labels::label(labels::ENCOUNTER, &field).unwrap_or(field.clone());
            let place = match slot {
                Some(slot) if field != "rate" => {
                    t!("commands.data.slot_suffix", n = slot + 1).into_owned()
                }
                _ => String::new(),
            };
            let label = label_or(
                t!(
                    "commands.data.encounter_edit",
                    index = index : {:04},
                    method = method_name,
                    place = place,
                    field = field_name,
                    old = old,
                    value = value
                )
                .into_owned(),
            );
            let change = store.edit_encounter(index, &label, |e| *e = copy)?;
            Ok(one(asset(AssetKind::Encounter, index), change))
        }
        DataPatch::EncounterSwap { method, a, b } if kind == DataKind::Encounter => {
            // Checked and swapped on a copy: a refused swap writes nothing.
            let mut copy = store.encounter(index)?.clone();
            swap(&mut copy, &method, a.into(), b.into())?;
            let method_name = labels::label(labels::ENCOUNTER, &method).unwrap_or(method.clone());
            let label = label_or(
                t!(
                    "commands.data.encounter_swap",
                    index = index : {:04},
                    method = method_name,
                    a = u16::from(a) + 1,
                    b = u16::from(b) + 1
                )
                .into_owned(),
            );
            let change = store.edit_encounter(index, &label, |e| *e = copy)?;
            Ok(one(asset(AssetKind::Encounter, index), change))
        }
        DataPatch::EncounterExtra { field, value } if kind == DataKind::Encounter => {
            let mut copy = store.encounter(index)?.clone();
            let old = apply_extra(&mut copy, &field, value)?;
            let field_name = labels::label(labels::ENCOUNTER, &field).unwrap_or(field.clone());
            let label = label_or(
                t!(
                    "commands.data.encounter_extra",
                    index = index : {:04},
                    field = field_name,
                    old = old,
                    value = value
                )
                .into_owned(),
            );
            let change = store.edit_encounter(index, &label, |e| *e = copy)?;
            Ok(one(asset(AssetKind::Encounter, index), change))
        }
        patch => Err(format!("{patch:?} does not apply to {kind:?}").into()),
    }
}
