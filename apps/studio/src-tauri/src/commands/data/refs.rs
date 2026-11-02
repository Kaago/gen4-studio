//! `refs`: the references of the kernel's index with a German place ("Begegnungen 0012, Gras
//! Platz 3"), the name of the source on its own and where a click leads.

use std::collections::HashMap;

use g4_kernel::data::refs::{Ref, Target};
use g4_kernel::project::AssetKind;

use super::dto::{NavTargetDto, RefDto, TargetDto};
use super::labels;
use super::{display, encounter_users, place_names};
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
    let found = session.refs.refs(&session.store, target.into())?;
    Ok(ref_dtos(session, &found))
}

/// Whether `field` of an event file is one of a warp's (its destination header or door).
fn warp_field(field: &str) -> bool {
    field.starts_with("warp_events.")
}

/// Event file → the first header (by id) that shows it, with the header's name
/// ([`place_names`], else „Header NNNN“).
fn event_users(session: &mut Session) -> HashMap<u16, (u16, String)> {
    let names = place_names(session);
    let Ok(headers) = session.store.header_table() else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for (id, header) in headers.iter().enumerate() {
        let Ok(id) = u16::try_from(id) else { break };
        let name = names
            .get(usize::from(id))
            .filter(|n| !n.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Header {id:04}"));
        out.entry(header.event_file_id()).or_insert((id, name));
    }
    out
}

/// The German name of a field of area data (its texture sets).
fn area_field(field: &str) -> &str {
    match field {
        "map_tileset" => "Kartentextur",
        "building_tileset" => "Gebäudetextur",
        other => other,
    }
}

/// The references `found` as the app shows them, in their order: label, source name and where a
/// click leads (also the list of a `referenced` error, [`crate::errors::CmdError::in_session`]).
pub(crate) fn ref_dtos(session: &mut Session, found: &[Ref]) -> Vec<RefDto> {
    // Event file → its first header and that header's name, read once when a warp is among them.
    let events = if found
        .iter()
        .any(|r| r.source.kind == AssetKind::Event && warp_field(r.field))
    {
        event_users(session)
    } else {
        HashMap::new()
    };
    let Session { store, names, .. } = session;
    // Encounter file → the headers that use it (a click on an encounter leads to the first),
    // read in one pass over the header table when the first encounter source comes up.
    let mut users: Option<HashMap<u16, Vec<u16>>> = None;
    let mut out = Vec::with_capacity(found.len());
    for &Ref {
        source,
        field,
        slot,
        target,
        access,
    } in found
    {
        let index = source.index;
        // The whole label where the reference says it in one sentence.
        let mut sentence: Option<String> = None;
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
            // A warp: „Tür 2 von Zweiblattdorf springt hierher“ (the place of the first header
            // that shows the event file), to the warp.
            AssetKind::Event if warp_field(field) => {
                let door = slot.unwrap_or(0);
                let user = events.get(&index);
                let place = user.map_or_else(
                    || format!("Eventdatei {index:04}"),
                    |(_, name)| name.clone(),
                );
                sentence = Some(format!("Tür {door} von {place} springt hierher"));
                (
                    format!("Eventdatei {index:04}"),
                    String::new(),
                    NavTargetDto::Event {
                        event_file: index,
                        event_kind: "warp",
                        index: door,
                        header: user.map(|&(id, _)| id),
                    },
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
            AssetKind::Script => {
                if let Target::WarpIndex(door) = target {
                    sentence = Some(format!("Script {index:04} setzt Tür {door} per Index um"));
                }
                script_place(index, field, slot, None)
            }
            // A cell of a matrix: its map or its header layer („Matrix 0003, Zelle 1, 0 · Karte“).
            AssetKind::Matrix => {
                let cell = slot.unwrap_or(0);
                let width = store
                    .matrix(index)
                    .map(|m| u16::from(m.width).max(1))
                    .unwrap_or(u16::MAX);
                let (x, y) = (cell % width, cell / width);
                let layer = if field == "headers" {
                    "Header"
                } else {
                    "Karte"
                };
                (
                    format!("Matrix {index:04}"),
                    format!("Zelle {x}, {y} · {layer}"),
                    NavTargetDto::MatrixCell {
                        matrix: index,
                        x: u8::try_from(x).unwrap_or(u8::MAX),
                        y: u8::try_from(y).unwrap_or(u8::MAX),
                        header: None,
                    },
                )
            }
            AssetKind::AreaData => (
                format!("Gebietsdaten {index:04}"),
                area_field(field).to_owned(),
                NavTargetDto::Area { id: index },
            ),
            // A building of a map („Karte 0012, Gebäude 3“; HeartGold/SoulSilver inside: „innen“).
            AssetKind::Map => {
                let building = slot.unwrap_or(0);
                let inside = if field.starts_with("interior_") {
                    " (innen)"
                } else {
                    ""
                };
                (
                    format!("Karte {index:04}"),
                    format!("Gebäude {building}{inside}"),
                    NavTargetDto::Building {
                        map: index,
                        index: building,
                        header: None,
                    },
                )
            }
            // The index has no other sources; one added later needs a label and a target here.
            _ => continue,
        };
        let label = match sentence {
            Some(sentence) => sentence,
            None if what.is_empty() => source_name.clone(),
            None => format!("{source_name}, {what}"),
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
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use g4_kernel::data::refs::Target;
    use g4_kernel::project::AssetKind;
    use g4_kernel::store::WarpEvent;

    use super::{NavTargetDto, ref_dtos};
    use crate::commands::{lock, open_synthetic, session};

    #[test]
    fn world_references_have_a_label_and_a_way() {
        let (_s, state) = open_synthetic();
        let mut guard = lock(&state).unwrap();
        let session = session(&mut guard).unwrap();
        session
            .store
            .edit_event(1, "Warp", |e| {
                e.warp_events.push(WarpEvent {
                    x: 3,
                    z: 4,
                    dest_header_id: 0,
                    dest_warp_id: 2,
                    height: 0,
                });
                Ok(())
            })
            .unwrap();

        // Matrix 0 (2 × 1) places map 1 in its cell (1, 0).
        let found = session.refs.refs(&session.store, Target::Map(1)).unwrap();
        let matrix = found
            .iter()
            .find(|r| r.source.kind == AssetKind::Matrix)
            .unwrap();
        let dto = ref_dtos(session, std::slice::from_ref(matrix)).remove(0);
        assert_eq!(dto.label, "Matrix 0000, Zelle 1, 0 · Karte");
        assert_eq!(dto.source_name, "Matrix 0000");
        assert_eq!(
            dto.nav,
            NavTargetDto::MatrixCell {
                matrix: 0,
                x: 1,
                y: 0,
                header: None,
            }
        );

        // Warp 0 of event file 1 leads to door 2 of header 0; no header shows event file 1.
        let found = session
            .refs
            .refs(
                &session.store,
                Target::Warp {
                    header: 0,
                    index: 2,
                },
            )
            .unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        let dto = ref_dtos(session, &found).remove(0);
        assert_eq!(dto.label, "Tür 0 von Eventdatei 0001 springt hierher");
        assert_eq!(dto.source_name, "Eventdatei 0001");
        assert_eq!(
            dto.nav,
            NavTargetDto::Event {
                event_file: 1,
                event_kind: "warp",
                index: 0,
                header: None,
            }
        );
    }
}
