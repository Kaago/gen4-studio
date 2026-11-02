//! „Raum anlegen“: a 1 x 1 matrix with a copy of a map, a header pointed at it and, with an
//! entrance, a warp outside and a warp inside that lead to each other, tiles a warp would not
//! fire on made doors.

use g4_formats::map::MAP_SIZE;
use serde::{Deserialize, Serialize};

use super::area::{
    AssignHeader, EventFileChoice, PlannedPlace, assign_header, assign_irreversible, assign_notes,
    check_assign, check_event_file_writable, event_file_irreversible, plan_place,
    provide_event_file, provide_header, provide_patch,
};
use super::doors::{
    WarpWrite, fires, header_event_file, local, outside_map, set_door, warp, warp_at, warp_count,
    write_warps,
};
use super::types::{
    Created, Draft, DraftCell, DraftEvent, DraftEventKind, DraftTarget, DraftTile, Outcome,
    check_header_id, check_member, check_name, check_writable, id, late, next_steps, refused,
};
use crate::error::KernelError;
use crate::places;
use crate::project::AssetKind;
use crate::store::{Store, Tx};

/// The door between the outside and the room: the outside warp stands at (`x`, `z`) of the
/// outside header's matrix, the room warp at (`inside_x`, `inside_z`) of the room.
#[derive(Debug, Clone, Deserialize)]
pub struct Entrance {
    pub outside_header: u16,
    pub x: u16,
    pub z: u16,
    pub inside_x: u16,
    pub inside_z: u16,
    /// Turns each of the two tiles a warp would not fire on into a door (see [`DOOR`]).
    #[serde(default)]
    pub door_tiles: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomSpec {
    pub name: String,
    pub source_map: u16,
    pub clear_buildings: bool,
    pub header: AssignHeader,
    pub entrance: Option<Entrance>,
}

/// A tile of a map that the wizard turned into a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoorTile {
    pub map: u16,
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Serialize)]
pub struct RoomReport {
    pub matrix: u16,
    pub map: u16,
    pub event_file: Option<u16>,
    pub outside_warp: Option<u16>,
    pub inside_warp: Option<u16>,
    /// The room warp is the copied warp that stood on its tile, now leading outside.
    pub inside_warp_reused: bool,
    /// The tiles made doors, the outside one first.
    pub door_tiles: Vec<DoorTile>,
    /// The call applied the Dynamic Headers patch.
    pub patched: bool,
    /// The copy of an older `unpacked/dynamicHeaders` the patch made under `.g4/backup/`.
    pub snapshot: Option<String>,
    /// The place the header got.
    pub place: Option<u8>,
}

/// What the entrance of a room needs from the existing files, found before anything is
/// created: the outside event file, matrix and map, the warp indices both warps get, the warp
/// of the room's event file that becomes the way back, and which of the two tiles must be
/// made doors.
struct EntrancePlan {
    outside_file: u16,
    outside_matrix: u16,
    outside_map: u16,
    /// The index of the new warp outside.
    outside_index: u16,
    /// The index of the room's warp back.
    room_index: u16,
    /// The room keeps the event file outside: both warps go into that one file.
    shared: bool,
    /// The warp a copied event file already has on the inside tile; it is pointed outside
    /// instead of adding a second warp the game would never take.
    reused: Option<u16>,
    outside_door: bool,
    inside_door: bool,
}

fn plan_entrance(
    store: &mut Store,
    spec: &RoomSpec,
    entrance: &Entrance,
) -> Result<EntrancePlan, KernelError> {
    let family = store.project().family;
    if entrance.outside_header == spec.header.id {
        return Err(refused(
            "the room's header cannot also be the header outside",
        ));
    }
    let last = MAP_SIZE as u16 - 1;
    if entrance.inside_x > last || entrance.inside_z > last {
        return Err(refused(format!(
            "tile ({}, {}) lies outside the room's map (0 to {last})",
            entrance.inside_x, entrance.inside_z
        )));
    }
    let outside_file = header_event_file(store, entrance.outside_header)?;
    check_event_file_writable(store, outside_file)?;
    let (outside_matrix, outside_map) =
        outside_map(store, entrance.outside_header, entrance.x, entrance.z)?;
    if let Some(i) = warp_at(store, outside_file, entrance.x, entrance.z)? {
        return Err(refused(format!(
            "event file {outside_file:04} already has warp {i} on tile ({}, {}) outside",
            entrance.x, entrance.z
        )));
    }
    let outside_index = warp_count(store, outside_file)?;
    // The room's file: kept, a copy of `source` (same warps) or new (none).
    let (reused, room_warps, shared) = match spec.header.event_file {
        EventFileChoice::Keep => {
            let own = header_event_file(store, spec.header.source_header())?;
            check_event_file_writable(store, own)?;
            // The file stays shared with whoever has it now: its warp there is not the wizard's
            // to repoint.
            if let Some(i) = warp_at(store, own, entrance.inside_x, entrance.inside_z)? {
                return Err(refused(format!(
                    "event file {own:04} already has warp {i} on tile ({}, {}) of the room",
                    entrance.inside_x, entrance.inside_z
                )));
            }
            (None, warp_count(store, own)?, own == outside_file)
        }
        EventFileChoice::Copy(source) => {
            check_event_file_writable(store, source)?;
            let reused = warp_at(store, source, entrance.inside_x, entrance.inside_z)?;
            (reused, warp_count(store, source)?, false)
        }
        EventFileChoice::New => (None, 0, false),
    };
    // With one shared file the room warp comes right after the outside warp.
    let room_index = match reused {
        Some(i) => i,
        None if shared => outside_index.saturating_add(1),
        None => room_warps,
    };
    let (mut outside_door, mut inside_door) = (false, false);
    if entrance.door_tiles {
        outside_door = !fires(
            store,
            family,
            outside_map,
            local(entrance.x),
            local(entrance.z),
        )?;
        if outside_door {
            check_writable(store, id(AssetKind::Map, outside_map))?;
        }
        inside_door = !fires(
            store,
            family,
            spec.source_map,
            entrance.inside_x,
            entrance.inside_z,
        )?;
    }
    Ok(EntrancePlan {
        outside_file,
        outside_matrix,
        outside_map,
        outside_index,
        room_index,
        shared,
        reused,
        outside_door,
        inside_door,
    })
}

/// What the checks of a room found.
struct Checked {
    entrance: Option<EntrancePlan>,
    place: Option<PlannedPlace>,
    /// The event file the room keeps on `Keep` (a new header starts with its template's).
    kept_file: u16,
}

/// Every check before the first write: the name, the map, the header, the entrance and the
/// place.
fn check(store: &mut Store, spec: &RoomSpec) -> Result<Checked, KernelError> {
    check_name(&spec.name)?;
    check_member(store, AssetKind::Map, spec.source_map, "map")?;
    check_assign(store, &spec.header, None)?;
    if let Some(entrance) = &spec.entrance {
        check_header_id(store, entrance.outside_header)?;
    }
    let entrance = match &spec.entrance {
        Some(entrance) => Some(plan_entrance(store, spec, entrance)?),
        None => None,
    };
    let place = plan_place(store, &spec.header)?;
    // A new header starts with its template's event file.
    let kept_file = header_event_file(store, spec.header.source_header())?;
    Ok(Checked {
        entrance,
        place,
        kept_file,
    })
}

/// Adds the warp outside (to the room's warp) and the room's warp back (or points the reused
/// one outside): edits of the recipe's one transaction, one per event file ([`write_warps`]).
fn connect(
    tx: &mut Tx<'_>,
    entrance: &Entrance,
    plan: &EntrancePlan,
    room_header: u16,
    room_file: u16,
) -> Result<(), KernelError> {
    let outside = warp(entrance.x, entrance.z, room_header, plan.room_index);
    let (header, index) = (entrance.outside_header, plan.outside_index);
    let inside = match plan.reused {
        Some(reused) => WarpWrite::Point {
            index: reused,
            header,
            warp: index,
        },
        None => WarpWrite::Add(warp(entrance.inside_x, entrance.inside_z, header, index)),
    };
    let room_file = if plan.shared {
        plan.outside_file
    } else {
        room_file
    };
    write_warps(
        tx,
        &[
            (plan.outside_file, WarpWrite::Add(outside)),
            (room_file, inside),
        ],
    )
}

/// What [`apply`] would do, found with the same checks and without writing: the one cell of the
/// new matrix, the two warps (the outside one on the outside header's matrix, the room's on the
/// new matrix: `matrix` `None`), the door tiles, the sentences of the wizard and what undo
/// would not take back.
pub fn plan(store: &mut Store, spec: &RoomSpec) -> Result<Draft, KernelError> {
    let checked = check(store, spec)?;
    let assign = &spec.header;
    let mut draft = Draft {
        cells: vec![DraftCell {
            matrix: None,
            x: 0,
            y: 0,
            source_map: spec.source_map,
        }],
        ..Draft::default()
    };
    draft.notes.push(format!(
        "Legt Raum „{}“ als Kopie von Karte {:04} an.",
        spec.name, spec.source_map
    ));
    draft
        .notes
        .push("Der Raum entsteht auf einer neuen Weltkarte.".to_owned());
    draft
        .notes
        .extend(assign_notes(store, assign, &spec.name, None)?);
    if let (Some(entrance), Some(plan)) = (&spec.entrance, &checked.entrance) {
        draft.events.push(DraftEvent {
            kind: DraftEventKind::Warp,
            matrix: Some(plan.outside_matrix),
            x: entrance.x,
            z: entrance.z,
            look: None,
            target: Some(DraftTarget {
                header: assign.id,
                warp: Some(plan.room_index),
            }),
        });
        draft.events.push(DraftEvent {
            kind: DraftEventKind::Warp,
            matrix: None,
            x: entrance.inside_x,
            z: entrance.inside_z,
            look: None,
            target: Some(DraftTarget {
                header: entrance.outside_header,
                warp: Some(plan.outside_index),
            }),
        });
        draft.notes.push(format!(
            "Eine Tür bei x {}, z {} von Header {} führt in den Raum.",
            entrance.x, entrance.z, entrance.outside_header
        ));
        draft.notes.push(match plan.reused {
            Some(i) => format!(
                "Der kopierte Warp {i} bei x {}, z {} im Raum führt künftig nach draußen.",
                entrance.inside_x, entrance.inside_z
            ),
            None => format!(
                "Eine Tür bei x {}, z {} im Raum führt zurück.",
                entrance.inside_x, entrance.inside_z
            ),
        });
        if plan.outside_door {
            draft.tiles.push(DraftTile {
                matrix: Some(plan.outside_matrix),
                x: entrance.x,
                z: entrance.z,
            });
            draft.notes.push(format!(
                "Die Kachel bei x {}, z {} draußen wird zur Tür.",
                entrance.x, entrance.z
            ));
        }
        if plan.inside_door {
            draft.tiles.push(DraftTile {
                matrix: None,
                x: entrance.inside_x,
                z: entrance.inside_z,
            });
            draft.notes.push(format!(
                "Die Kachel bei x {}, z {} im Raum wird zur Tür.",
                entrance.inside_x, entrance.inside_z
            ));
        }
    }
    draft
        .irreversible
        .extend(assign_irreversible(store, assign)?);
    draft
        .irreversible
        .push(format!("Karte {:04} wird kopiert", spec.source_map));
    draft
        .irreversible
        .push("Eine neue Weltkarte wird angelegt".to_owned());
    draft
        .irreversible
        .extend(event_file_irreversible(assign.event_file));
    Ok(draft)
}

/// Creates a 1 x 1 matrix with a copy of a map, points a header at it and, with an entrance,
/// links a warp outside with a warp inside, then makes the door tiles (outside, inside): the
/// edits are one undo step `Raum „…“ angelegt`. When the outside header and the room header
/// share the event file (`Keep`), the room warp is appended right after the outside warp in
/// that one file. A copied event file that has a warp on the inside tile keeps it as the way
/// back, pointed outside: the game takes the first warp of a tile.
pub fn apply(store: &mut Store, spec: &RoomSpec) -> Result<Outcome<RoomReport>, KernelError> {
    let Checked {
        entrance: plan,
        place,
        kept_file,
    } = check(store, spec)?;
    let generation = store.change_generation();

    let mut created = Created::default();
    let snapshot = provide_patch(store, &spec.header, &mut created)?;
    provide_header(store, &spec.header, &spec.name, &mut created)?;
    let map = store
        .duplicate_map_with(spec.source_map, spec.clear_buildings)
        .map_err(|e| late(&created, "map copy", e))?;
    created.maps.push(map);
    let matrix = store
        .create_matrix(&spec.name)
        .map_err(|e| late(&created, "matrix creation", e))?;
    created.matrices.push(matrix);
    let event_file = provide_event_file(store, spec.header.event_file, &mut created)?;
    let room_file = event_file.unwrap_or(kept_file);

    let bank = places::location_bank(store.project());
    let label = format!("Raum „{}“ angelegt", spec.name);
    let entrance = spec.entrance.as_ref().zip(plan.as_ref());
    let (door_tiles, change) = store.transaction(&label, |tx| {
        tx.edit_matrix(matrix, |m| {
            m.maps[0] = map;
            Ok(())
        })
        .map_err(|e| late(&created, "matrix edit", e))?;
        assign_header(tx, bank, &spec.header, matrix, event_file, place)
            .map_err(|e| late(&created, "header edit", e))?;
        let mut door_tiles = Vec::new();
        if let Some((entrance, plan)) = entrance {
            connect(tx, entrance, plan, spec.header.id, room_file)
                .map_err(|e| late(&created, "warp edit", e))?;
            let door = |tx: &mut Tx<'_>, map, tile| {
                set_door(tx, map, tile).map_err(|e| late(&created, "door tile", e))
            };
            if plan.outside_door {
                let tile = (local(entrance.x), local(entrance.z));
                door_tiles.push(door(tx, plan.outside_map, tile)?);
            }
            if plan.inside_door {
                door_tiles.push(door(tx, map, (entrance.inside_x, entrance.inside_z))?);
            }
        }
        Ok(door_tiles)
    })?;
    Ok(Outcome {
        report: RoomReport {
            matrix,
            map,
            event_file,
            outside_warp: plan.as_ref().map(|p| p.outside_index),
            inside_warp: plan.as_ref().map(|p| p.room_index),
            inside_warp_reused: plan.as_ref().is_some_and(|p| p.reused.is_some()),
            door_tiles,
            patched: created.patch,
            snapshot,
            place: place.map(PlannedPlace::index),
        },
        created: created.objects(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[("new_figure", "Neue Figur hier")]),
    })
}
