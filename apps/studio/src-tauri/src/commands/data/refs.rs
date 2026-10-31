//! `refs`: the references of the kernel's index with a German place ("Begegnungen 0012, Gras
//! Platz 3"), the name of the source on its own and where a click leads.

use std::collections::HashMap;

use g4_kernel::data::refs::{Ref, Target};
use g4_kernel::project::AssetKind;

use super::dto::{NavTargetDto, RefDto, TargetDto};
use super::labels;
use super::{display, encounter_users};
use crate::errors::CmdError;
use crate::state::Session;

impl From<TargetDto> for Target {
    fn from(t: TargetDto) -> Self {
        match t {
            TargetDto::Species { index } => Target::Species(index),
            TargetDto::Move { index } => Target::Move(index),
            TargetDto::Item { id } => Target::Item(id),
            TargetDto::Trainer { index } => Target::Trainer(index),
            TargetDto::TrainerClass { index } => Target::TrainerClass(index),
            TargetDto::Encounter { index } => Target::Encounter(index),
        }
    }
}

/// "Platz 3" for list position 2; nothing for a field without a list.
fn place(slot: Option<u16>) -> String {
    slot.map(|s| format!(" Platz {}", s + 1))
        .unwrap_or_default()
}

/// The German name of an encounter source field (methods, HeartGold grass by time of day,
/// replacement lists).
fn encounter_field(field: &str) -> &str {
    match field {
        "grass_morning" => "Gras morgens",
        "grass_day" => "Gras tagsüber",
        "grass_night" => "Gras nachts",
        other => labels::label(labels::ENCOUNTER, other).unwrap_or(other),
    }
}

/// The field of a level script's references (its conditions); every other reference of a
/// script file comes from its rotom source, with the command as field and the line as slot.
pub(crate) const LEVEL_SCRIPT_FIELD: &str = "var_conditions";

/// The name of script file `file` as a source, what of it refers and where a click leads: a
/// level script („Level-Script 0005“, „Bedingung 1 · var_conditions“, its header `header` when
/// one uses it, else the file), else a rotom source („Scriptdatei 0101“, „Zeile 12 ·
/// SetFlag“, the file at that line).
pub(crate) fn script_place(
    file: u16,
    field: &str,
    slot: Option<u16>,
    header: Option<u16>,
) -> (String, String, NavTargetDto) {
    if field == LEVEL_SCRIPT_FIELD {
        let nav = match header {
            Some(id) => NavTargetDto::Header { id },
            None => NavTargetDto::Script { file, line: None },
        };
        return (
            format!("Level-Script {file:04}"),
            format!("Bedingung {} · {field}", slot.unwrap_or(0) + 1),
            nav,
        );
    }
    let what = match slot {
        Some(line) => format!("Zeile {line} · {field}"),
        None => field.to_owned(),
    };
    (
        format!("Scriptdatei {file:04}"),
        what,
        NavTargetDto::Script {
            file,
            line: slot.map(u32::from),
        },
    )
}

/// Every reference to `target`, in the index's order, with label, source name and navigation
/// target.
pub fn refs(session: &mut Session, target: TargetDto) -> Result<Vec<RefDto>, CmdError> {
    let Session {
        store, names, refs, ..
    } = session;
    let found = refs.refs(store, target.into())?;
    // Encounter file → the headers that use it (a click on an encounter leads to the first),
    // read in one pass over the header table when the first encounter source comes up.
    let mut users: Option<HashMap<u16, Vec<u16>>> = None;
    let mut out = Vec::with_capacity(found.len());
    for Ref {
        source,
        field,
        slot,
        access,
        ..
    } in found
    {
        let index = source.index;
        // The source's name, what of it refers (empty: the source as a whole), the target.
        let (source_name, what, nav) = match source.kind {
            AssetKind::HeaderTable => {
                let id = slot.unwrap_or(0);
                (
                    format!("Header {id:04}"),
                    String::new(),
                    NavTargetDto::Header { id },
                )
            }
            AssetKind::Encounter => {
                let user = users
                    .get_or_insert_with(|| encounter_users(store))
                    .get(&index)
                    .and_then(|headers| headers.first().copied());
                let nav = match user {
                    Some(id) => NavTargetDto::Header { id },
                    None => NavTargetDto::Encounter { index },
                };
                (
                    format!("Begegnungen {index:04}"),
                    format!("{}{}", encounter_field(field), place(slot)),
                    nav,
                )
            }
            AssetKind::Event => (
                format!("Eventdatei {index:04}"),
                format!("Objekt {}", slot.unwrap_or(0)),
                NavTargetDto::Event {
                    event_file: index,
                    event_kind: "object",
                    index: slot.unwrap_or(0),
                    header: None,
                },
            ),
            AssetKind::Personal => (
                display::species(names, store, index),
                labels::label(labels::SPECIES, field)
                    .unwrap_or(field)
                    .to_owned(),
                NavTargetDto::Species {
                    index,
                    tab: Some("abilities"),
                },
            ),
            AssetKind::Learnset => (
                display::species(names, store, index),
                format!("Lernset{}", place(slot)),
                NavTargetDto::Species {
                    index,
                    tab: Some("learnset"),
                },
            ),
            AssetKind::Evolution => {
                let what = if field == "target" {
                    "Ziel"
                } else {
                    "Parameter"
                };
                (
                    display::species(names, store, index),
                    format!("Entwicklung {} {what}", slot.unwrap_or(0) + 1),
                    NavTargetDto::Species {
                        index,
                        tab: Some("evolution"),
                    },
                )
            }
            AssetKind::TrainerProperties => {
                let what = match field {
                    "class" => "Klasse".to_owned(),
                    _ => format!("Gegenstand{}", place(slot)),
                };
                (
                    display::trainer(names, store, index),
                    what,
                    NavTargetDto::Trainer { index },
                )
            }
            AssetKind::TrainerParty => (
                display::trainer(names, store, index),
                format!("Team{}", place(slot)),
                NavTargetDto::Trainer { index },
            ),
            AssetKind::Script => script_place(index, field, slot, None),
            // The index has no other sources; one added later needs a label and a target here.
            _ => continue,
        };
        let label = if what.is_empty() {
            source_name.clone()
        } else {
            format!("{source_name}, {what}")
        };
        out.push(RefDto {
            source,
            field,
            slot,
            label,
            source_name,
            nav,
            access,
        });
    }
    Ok(out)
}
