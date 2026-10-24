//! `refs`: the references of the kernel's index with a German place ("Begegnungen 0012, Gras
//! Platz 3") and where a click leads.

use std::collections::HashMap;

use g4_kernel::data::refs::{Ref, Target};
use g4_kernel::project::AssetKind;

use super::display;
use super::dto::{NavTargetDto, RefDto, TargetDto};
use super::labels;
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

/// Every reference to `target`, in the index's order, with label and navigation target.
pub fn refs(session: &mut Session, target: TargetDto) -> Result<Vec<RefDto>, CmdError> {
    let Session {
        store, names, refs, ..
    } = session;
    let found = refs.refs(store, target.into())?;
    // Encounter file → the first header that uses it (where a click on an encounter leads).
    let mut users: HashMap<u16, Option<u16>> = HashMap::new();
    let mut out = Vec::with_capacity(found.len());
    for Ref {
        source,
        field,
        slot,
        ..
    } in found
    {
        let index = source.index;
        let (label, nav) = match source.kind {
            AssetKind::HeaderTable => {
                let id = slot.unwrap_or(0);
                (format!("Header {id:04}"), NavTargetDto::Header { id })
            }
            AssetKind::Encounter => {
                let user = match users.get(&index) {
                    Some(user) => *user,
                    None => {
                        let user = refs
                            .refs(store, Target::Encounter(index))?
                            .iter()
                            .find(|r| r.source.kind == AssetKind::HeaderTable)
                            .and_then(|r| r.slot);
                        users.insert(index, user);
                        user
                    }
                };
                let nav = match user {
                    Some(id) => NavTargetDto::Header { id },
                    None => NavTargetDto::Encounter { index },
                };
                let label = format!(
                    "Begegnungen {index:04}, {}{}",
                    encounter_field(field),
                    place(slot)
                );
                (label, nav)
            }
            AssetKind::Event => (
                format!("Eventdatei {index:04}, Objekt {}", slot.unwrap_or(0)),
                NavTargetDto::Event {
                    event_file: index,
                    event_kind: "object",
                    index: slot.unwrap_or(0),
                },
            ),
            AssetKind::Personal => {
                let name = display::species(names, store, index);
                let what = labels::label(labels::SPECIES, field).unwrap_or(field);
                (
                    format!("{name}, {what}"),
                    NavTargetDto::Species {
                        index,
                        tab: Some("abilities"),
                    },
                )
            }
            AssetKind::Learnset => (
                format!(
                    "{}, Lernset{}",
                    display::species(names, store, index),
                    place(slot)
                ),
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
                    format!(
                        "{}, Entwicklung {} {what}",
                        display::species(names, store, index),
                        slot.unwrap_or(0) + 1
                    ),
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
                    format!("{}, {what}", display::trainer(names, store, index)),
                    NavTargetDto::Trainer { index },
                )
            }
            AssetKind::TrainerParty => (
                format!(
                    "{}, Team{}",
                    display::trainer(names, store, index),
                    place(slot)
                ),
                NavTargetDto::Trainer { index },
            ),
            // The index has no other sources; one added later needs a label and a target here.
            _ => continue,
        };
        out.push(RefDto {
            source,
            field,
            slot,
            label,
            nav,
        });
    }
    Ok(out)
}
