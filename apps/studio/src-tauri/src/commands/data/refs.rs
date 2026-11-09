//! `refs`: the references of the kernel's index with a place in words ("Encounters 0012, Grass
//! slot 3", in German "Begegnungen 0012, Gras Platz 3"), the name of the source on its own and
//! where a click leads.

use std::collections::HashMap;

use g4_kernel::data::refs::{Ref, Target};
use g4_kernel::project::AssetKind;

use super::dto::{NavTargetDto, RefDto, TargetDto};
use super::labels;
use super::{display, encounter_users, header_name, place_names};
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
            TargetDto::Message { archive, index } => Target::Message { archive, index },
        }
    }
}

/// " slot 3" (" Platz 3" in German) for list position 2; nothing for a field without a list.
fn place(slot: Option<u16>) -> String {
    slot.map(|s| t!("commands.data.slot_suffix", n = s + 1).into_owned())
        .unwrap_or_default()
}

/// The name of an encounter source field (methods, HeartGold grass by time of day,
/// replacement lists), in the language of the process.
fn encounter_field(field: &str) -> String {
    match field {
        "grass_morning" => t!("commands.data.grass_morning").into_owned(),
        "grass_day" => t!("commands.data.grass_day").into_owned(),
        "grass_night" => t!("commands.data.grass_night").into_owned(),
        other => labels::label(labels::ENCOUNTER, other).unwrap_or_else(|| other.to_owned()),
    }
}

/// The field of a level script's references (its conditions); every other reference of a
/// script file comes from its rotom source, with the command as field and the line as slot.
pub(crate) const LEVEL_SCRIPT_FIELD: &str = "var_conditions";

/// The name of script file `file` as a source, what of it refers and where a click leads: a
/// level script („Level script 0005“, „Condition 1 · var_conditions“, its header `header` when
/// one uses it, else the file), else a rotom source („Script file 0101“, „Line 12 · SetFlag“,
/// the file at that line); the German texts say „Level-Script“, „Bedingung“, „Scriptdatei“
/// and „Zeile“.
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
            t!("commands.data.level_script", file = file : {:04}).into_owned(),
            t!(
                "commands.data.condition",
                n = slot.unwrap_or(0) + 1,
                field = field
            )
            .into_owned(),
            nav,
        );
    }
    let what = match slot {
        Some(line) => t!("commands.data.script_line", line = line, field = field).into_owned(),
        None => field.to_owned(),
    };
    (
        t!("commands.data.script_file", file = file : {:04}).into_owned(),
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

/// "Event file 0012" ("Eventdatei 0012" in German).
fn event_file_name(index: u16) -> String {
    t!("commands.data.source_event_file", index = index : {:04}).into_owned()
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
            .unwrap_or_else(|| header_name(id));
        out.entry(header.event_file_id()).or_insert((id, name));
    }
    out
}

/// The name of a field of area data (its texture sets), in the language of the process.
fn area_field(field: &str) -> String {
    match field {
        "map_tileset" => t!("commands.data.area_map_tileset").into_owned(),
        "building_tileset" => t!("commands.data.area_building_tileset").into_owned(),
        other => other.to_owned(),
    }
}

/// The references `found` as the app shows them, in their order: label, source name and where a
/// click leads (also the list of a `referenced` error, [`crate::errors::CmdError::in_session`]).
pub(crate) fn ref_dtos(session: &mut Session, found: &[Ref]) -> Vec<RefDto> {
    // Event file → its first header and that header's name, read once when an event is among
    // them (a warp names the place, every event leads to the header that shows it).
    let events = if found.iter().any(|r| r.source.kind == AssetKind::Event) {
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
        ..
    } in found
    {
        let index = source.index;
        // The whole label where the reference says it in one sentence.
        let mut sentence: Option<String> = None;
        // The source's name, what of it refers (empty: the source as a whole), the target.
        let (source_name, what, nav) = match source.kind {
            AssetKind::HeaderTable => {
                let id = slot.unwrap_or(0);
                (header_name(id), String::new(), NavTargetDto::Header { id })
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
                    t!("commands.data.source_encounters", index = index : {:04}).into_owned(),
                    format!("{}{}", encounter_field(field), place(slot)),
                    nav,
                )
            }
            // A warp: „Door 2 of Twinleaf Town leads here“ („Tür 2 von Zweiblattdorf springt
            // hierher“; the place of the first header that shows the event file), to the warp.
            AssetKind::Event if warp_field(field) => {
                let door = slot.unwrap_or(0);
                let user = events.get(&index);
                let place = user.map_or_else(|| event_file_name(index), |(_, name)| name.clone());
                sentence =
                    Some(t!("commands.data.warp_here", door = door, place = place).into_owned());
                (
                    event_file_name(index),
                    String::new(),
                    NavTargetDto::Event {
                        event_file: index,
                        event_kind: "warp",
                        index: door,
                        header: user.map(|&(id, _)| id),
                    },
                )
            }
            // A figure, to it in the first header that shows its event file (the scene shows
            // that place), as a warp.
            AssetKind::Event => (
                event_file_name(index),
                t!("commands.data.event_object", n = slot.unwrap_or(0)).into_owned(),
                NavTargetDto::Event {
                    event_file: index,
                    event_kind: "object",
                    index: slot.unwrap_or(0),
                    header: events.get(&index).map(|&(id, _)| id),
                },
            ),
            AssetKind::Personal => (
                display::species(names, store, index),
                labels::label(labels::SPECIES, field).unwrap_or_else(|| field.to_owned()),
                NavTargetDto::Species {
                    index,
                    tab: Some("abilities"),
                },
            ),
            AssetKind::Learnset => (
                display::species(names, store, index),
                format!("{}{}", t!("commands.data.learnset"), place(slot)),
                NavTargetDto::Species {
                    index,
                    tab: Some("learnset"),
                },
            ),
            AssetKind::Evolution => {
                let what = if field == "target" {
                    t!("commands.data.evolution_target")
                } else {
                    t!("commands.data.evolution_param")
                };
                (
                    display::species(names, store, index),
                    t!(
                        "commands.data.evolution_label_short",
                        slot = slot.unwrap_or(0) + 1,
                        what = what
                    )
                    .into_owned(),
                    NavTargetDto::Species {
                        index,
                        tab: Some("evolution"),
                    },
                )
            }
            AssetKind::TrainerProperties => {
                let what = match field {
                    "class" => t!("commands.data.trainer_class").into_owned(),
                    _ => format!("{}{}", t!("commands.data.trainer_item"), place(slot)),
                };
                (
                    display::trainer(names, store, index),
                    what,
                    NavTargetDto::Trainer { index },
                )
            }
            AssetKind::TrainerParty => (
                display::trainer(names, store, index),
                format!("{}{}", t!("commands.data.trainer_party"), place(slot)),
                NavTargetDto::Trainer { index },
            ),
            AssetKind::Script => {
                if let Target::WarpIndex(door) = target {
                    sentence = Some(
                        t!("commands.data.script_warp_index", index = index : {:04}, door = door)
                            .into_owned(),
                    );
                }
                script_place(index, field, slot, None)
            }
            // A cell of a matrix: its map or its header layer („Grid 0003, Cell 1, 0 · Map“;
            // „Matrix 0003, Zelle 1, 0 · Karte“ in German).
            AssetKind::Matrix => {
                let cell = slot.unwrap_or(0);
                let width = store
                    .matrix(index)
                    .map(|m| u16::from(m.width).max(1))
                    .unwrap_or(u16::MAX);
                let (x, y) = (cell % width, cell / width);
                let layer = if field == "headers" {
                    t!("commands.data.layer_headers")
                } else {
                    t!("commands.data.layer_map")
                };
                (
                    t!("commands.data.source_matrix", index = index : {:04}).into_owned(),
                    t!("commands.data.matrix_cell", x = x, y = y, layer = layer).into_owned(),
                    NavTargetDto::MatrixCell {
                        matrix: index,
                        x: u8::try_from(x).unwrap_or(u8::MAX),
                        y: u8::try_from(y).unwrap_or(u8::MAX),
                        header: None,
                    },
                )
            }
            AssetKind::AreaData => (
                t!("commands.data.source_area_data", index = index : {:04}).into_owned(),
                area_field(field),
                NavTargetDto::Area { id: index },
            ),
            // A building of a map („Map 0012, Building 3“; HeartGold/SoulSilver inside:
            // „(inside)“; in German „Karte 0012, Gebäude 3“ and „(innen)“).
            AssetKind::Map => {
                let building = slot.unwrap_or(0);
                let what = if field.starts_with("interior_") {
                    t!("commands.data.building_inside", n = building)
                } else {
                    t!("commands.data.building", n = building)
                };
                (
                    t!("commands.data.source_map", index = index : {:04}).into_owned(),
                    what.into_owned(),
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
        assert_eq!(dto.label, "Grid 0000, Cell 1, 0 · Map");
        assert_eq!(dto.source_name, "Grid 0000");
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
        assert_eq!(dto.label, "Door 0 of Event file 0001 leads here");
        assert_eq!(dto.source_name, "Event file 0001");
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

    /// Gate 3d.4 B12 (a): a figure's reference (a trainer it starts) leads to the figure in the
    /// first header that shows its event file, like a warp's, so the scene shows its place.
    #[test]
    fn an_object_reference_leads_to_the_header_of_its_event_file() {
        use g4_kernel::data::refs::{Access, Ref};
        use g4_kernel::project::AssetId;
        let (_s, state) = open_synthetic();
        let mut guard = lock(&state).unwrap();
        let session = session(&mut guard).unwrap();
        // Header 0 is the first that shows its event file.
        let file = session.store.header_table().unwrap()[0].event_file_id();
        let figure = Ref {
            source: AssetId {
                kind: AssetKind::Event,
                index: file,
            },
            field: "object_events.script",
            slot: Some(0),
            target: Target::Trainer(1),
            access: Access::Unknown,
            value: None,
        };
        let dto = ref_dtos(session, &[figure]).remove(0);
        assert_eq!(
            dto.nav,
            NavTargetDto::Event {
                event_file: file,
                event_kind: "object",
                index: 0,
                header: Some(0),
            }
        );
    }
}
