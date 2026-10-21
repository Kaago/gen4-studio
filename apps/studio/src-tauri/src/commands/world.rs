//! Matrices, maps, events and the permission layer.

use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::store::EventFile;
use g4_kernel::tiles::TileBehavior;
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct MatrixDto {
    pub index: u16,
    pub name: String,
    pub width: u8,
    pub height: u8,
    /// Row-major map ids; `None` marks an empty cell.
    pub maps: Vec<Option<u16>>,
}

impl MatrixDto {
    pub fn from_matrix(index: u16, matrix: &Matrix) -> MatrixDto {
        MatrixDto {
            index,
            name: matrix.name.clone(),
            width: matrix.width,
            height: matrix.height,
            maps: matrix
                .maps
                .iter()
                .map(|&m| (m != MATRIX_EMPTY).then_some(m))
                .collect(),
        }
    }
}

/// One building, positions in tiles from the map centre (`y` is vertical), scale 1.0 = unscaled.
#[derive(Serialize)]
pub struct BuildingDto {
    pub model_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub sx: f32,
    pub sy: f32,
    pub sz: f32,
}

#[derive(Serialize)]
pub struct MapDto {
    pub index: u16,
    /// `[kind, collision]` per cell, row-major (32 x 32).
    pub permissions: Vec<[u8; 2]>,
    pub buildings: Vec<BuildingDto>,
}

/// One map tile is 16 world units; positions are fx32 (20.12), so 16 * 4096 raw units per tile.
const TILE_RAW: f32 = 65536.0;
/// fx32 value of 1.0.
const FX_ONE: f32 = 4096.0;

impl MapDto {
    pub fn from_map(index: u16, map: &MapFile) -> MapDto {
        MapDto {
            index,
            permissions: map
                .permissions
                .iter()
                .map(|c| [c.kind, c.collision])
                .collect(),
            buildings: map
                .buildings
                .iter()
                .map(|b| BuildingDto {
                    model_id: b.model_id,
                    x: b.position[0] as f32 / TILE_RAW,
                    y: b.position[1] as f32 / TILE_RAW,
                    z: b.position[2] as f32 / TILE_RAW,
                    sx: b.scale[0] as f32 / FX_ONE,
                    sy: b.scale[1] as f32 / FX_ONE,
                    sz: b.scale[2] as f32 / FX_ONE,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct BgDto {
    pub index: u16,
    pub script: u16,
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
}

#[derive(Serialize)]
pub struct ObjectDto {
    pub index: u16,
    pub local_id: u16,
    pub graphics_id: u16,
    pub movement_type: u16,
    pub script: u16,
    pub x: u16,
    pub z: u16,
    pub y: i32,
}

#[derive(Serialize)]
pub struct WarpDto {
    pub index: u16,
    pub x: u16,
    pub z: u16,
    pub dest_header_id: u16,
    pub dest_warp_id: u16,
    pub height: u32,
}

#[derive(Serialize)]
pub struct CoordDto {
    pub index: u16,
    pub script: u16,
    pub x: i16,
    pub z: i16,
    pub width: u16,
    pub length: u16,
}

/// The events of one event file. Coordinates are absolute tiles over the whole matrix
/// (matrix cell = `x / 32`, tile inside the map = `x % 32`), as the game data stores them.
#[derive(Serialize)]
pub struct EventsDto {
    pub event_file_id: u16,
    pub bg: Vec<BgDto>,
    pub objects: Vec<ObjectDto>,
    pub warps: Vec<WarpDto>,
    pub coords: Vec<CoordDto>,
}

/// Position of an event inside its list as the `u16` the DTOs carry. `get_events_impl`
/// rejects files with more than `u16::MAX` events per class first, so this never saturates.
fn event_index(i: usize) -> u16 {
    u16::try_from(i).unwrap_or(u16::MAX)
}

impl EventsDto {
    pub fn from_file(event_file_id: u16, file: &EventFile) -> Result<EventsDto, CmdError> {
        let counts = [
            file.bg_events.len(),
            file.object_events.len(),
            file.warp_events.len(),
            file.coord_events.len(),
        ];
        if counts.iter().any(|&n| n > usize::from(u16::MAX)) {
            return Err(format!("event file {event_file_id:04} has too many events").into());
        }
        Ok(EventsDto {
            event_file_id,
            bg: file
                .bg_events
                .iter()
                .enumerate()
                .map(|(i, e)| BgDto {
                    index: event_index(i),
                    script: e.script,
                    event_type: e.event_type,
                    x: e.x,
                    z: e.z,
                    y: e.y,
                })
                .collect(),
            objects: file
                .object_events
                .iter()
                .enumerate()
                .map(|(i, e)| ObjectDto {
                    index: event_index(i),
                    local_id: e.local_id,
                    graphics_id: e.graphics_id,
                    movement_type: e.movement_type,
                    script: e.script,
                    x: e.x,
                    z: e.z,
                    y: e.y,
                })
                .collect(),
            warps: file
                .warp_events
                .iter()
                .enumerate()
                .map(|(i, e)| WarpDto {
                    index: event_index(i),
                    x: e.x,
                    z: e.z,
                    dest_header_id: e.dest_header_id,
                    dest_warp_id: e.dest_warp_id,
                    height: e.height,
                })
                .collect(),
            coords: file
                .coord_events
                .iter()
                .enumerate()
                .map(|(i, e)| CoordDto {
                    index: event_index(i),
                    script: e.script,
                    x: e.x,
                    z: e.z,
                    width: e.width,
                    length: e.length,
                })
                .collect(),
        })
    }
}

pub fn tile_behaviors_impl() -> Result<Vec<TileBehavior>, CmdError> {
    g4_kernel::tiles::tile_behaviors().map_err(CmdError::from)
}

pub fn get_matrix_impl(state: &AppState, index: u16) -> Result<MatrixDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let matrix = session.store.matrix(index).map_err(CmdError::from)?;
    Ok(MatrixDto::from_matrix(index, matrix))
}

pub fn get_map_impl(state: &AppState, index: u16) -> Result<MapDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let map = session.store.map(index).map_err(CmdError::from)?;
    Ok(MapDto::from_map(index, map))
}

pub fn get_events_impl(state: &AppState, event_file_id: u16) -> Result<EventsDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let file = session.store.event(event_file_id).map_err(CmdError::from)?;
    EventsDto::from_file(event_file_id, file)
}

/// Tiles per map edge; permission coordinates must stay below this.
const MAP_TILES: u8 = 32;

pub fn set_permission_impl(
    state: &AppState,
    index: u16,
    x: u8,
    y: u8,
    kind: u8,
    collision: u8,
) -> Result<(), CmdError> {
    if x >= MAP_TILES || y >= MAP_TILES {
        return Err(format!("cell ({x}, {y}) is outside the 32 x 32 map").into());
    }
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.edit_map(index, "paint", |map| {
        map.set_cell(usize::from(x), usize::from(y), Cell { kind, collision })
    })?;
    Ok(())
}

#[tauri::command]
pub fn tile_behaviors() -> Result<Vec<TileBehavior>, CmdError> {
    tile_behaviors_impl()
}

#[tauri::command]
pub fn get_matrix(state: State<AppState>, index: u16) -> Result<MatrixDto, CmdError> {
    get_matrix_impl(&state, index)
}

#[tauri::command]
pub fn get_events(state: State<AppState>, event_file_id: u16) -> Result<EventsDto, CmdError> {
    get_events_impl(&state, event_file_id)
}

#[tauri::command]
pub fn get_map(state: State<AppState>, index: u16) -> Result<MapDto, CmdError> {
    get_map_impl(&state, index)
}

#[tauri::command]
pub fn set_permission(
    state: State<AppState>,
    index: u16,
    x: u8,
    y: u8,
    kind: u8,
    collision: u8,
) -> Result<(), CmdError> {
    set_permission_impl(&state, index, x, y, kind, collision)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;

    #[test]
    fn tile_behaviors_command_lists_the_embedded_table() {
        let list = tile_behaviors_impl().unwrap();
        assert!(
            list.iter()
                .any(|b| b.name == "WARP_STAIRS_EAST" && b.value == 0x5E)
        );
    }

    #[test]
    fn map_dto_has_1024_permission_pairs_and_signed_building_coords() {
        let mut map = g4_formats::map::MapFile {
            bgs: None,
            permissions: vec![Default::default(); 1024],
            buildings: vec![],
            buildings_tail: vec![],
            model: vec![],
            bdhc: vec![],
        };
        map.buildings.push(g4_formats::map::Building {
            model_id: 1,
            position: [-16 << 16, 0, 2 << 16],
            rotation: [0; 3],
            scale: [0x1000; 3],
            dummy: [0; 8],
        });
        let dto = MapDto::from_map(3, &map);
        assert_eq!(dto.permissions.len(), 1024);
        assert_eq!(dto.buildings[0].x, -16.0);
        assert_eq!(dto.buildings[0].z, 2.0);
        assert_eq!(dto.buildings[0].sx, 1.0);
    }

    #[test]
    fn set_permission_rejects_out_of_range_without_panicking() {
        let (_fixture, state) = open_synthetic();
        assert!(set_permission_impl(&state, 0, 32, 0, 0, 0x80).is_err());
        assert!(set_permission_impl(&state, 0, 0, 32, 0, 0x80).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(set_permission_impl(&state, 0, 3, 3, 0, 0x80).is_ok());
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        // An unknown map is an error, not a panic.
        assert!(set_permission_impl(&state, 99, 0, 0, 0, 0x80).is_err());
        assert_eq!(
            set_permission_impl(&AppState::default(), 0, 0, 0, 0, 0)
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn set_permission_is_visible_through_get_map_and_undoable() {
        let (_fixture, state) = open_synthetic();
        let before = get_map_impl(&state, 0).unwrap().permissions[3 * 32 + 3];
        set_permission_impl(&state, 0, 3, 3, 4, 0x80).unwrap();
        assert_eq!(
            get_map_impl(&state, 0).unwrap().permissions[3 * 32 + 3],
            [4, 0x80]
        );
        assert!(undo_impl(&state).unwrap());
        assert_eq!(
            get_map_impl(&state, 0).unwrap().permissions[3 * 32 + 3],
            before
        );
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(redo_impl(&state).unwrap());
        assert_eq!(
            get_map_impl(&state, 0).unwrap().permissions[3 * 32 + 3],
            [4, 0x80]
        );
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        // Nothing left to redo, and a second undo/redo pair after exhausting reports false.
        assert!(!redo_impl(&state).unwrap());
        assert!(undo_impl(&state).unwrap());
        assert!(!undo_impl(&state).unwrap());
    }

    #[test]
    fn events_dto_maps_every_event_class_with_its_index() {
        use g4_kernel::store::{BgEvent, CoordEvent, ObjectEvent, WarpEvent};
        let (_fixture, state) = open_synthetic();
        let empty = get_events_impl(&state, 0).unwrap();
        assert_eq!(empty.event_file_id, 0);
        assert!(empty.bg.is_empty() && empty.objects.is_empty());
        assert!(empty.warps.is_empty() && empty.coords.is_empty());

        lock(&state)
            .unwrap()
            .as_mut()
            .unwrap()
            .store
            .edit_event(0, "test", |e| {
                e.warp_events.push(WarpEvent {
                    x: 8,
                    z: 4,
                    dest_header_id: 414,
                    dest_warp_id: 2,
                    height: 0,
                });
                e.warp_events.push(WarpEvent {
                    x: 40,
                    z: 5,
                    dest_header_id: 1,
                    dest_warp_id: 0,
                    height: 0,
                });
                e.object_events.push(ObjectEvent {
                    local_id: 7,
                    graphics_id: 9,
                    movement_type: 3,
                    trainer_type: 0,
                    hidden_flag: 0,
                    script: 2001,
                    dir: 0,
                    data: [0; 3],
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: 12,
                    z: 13,
                    y: -1,
                });
                e.coord_events.push(CoordEvent {
                    script: 9000,
                    x: 3,
                    z: -2,
                    width: 2,
                    length: 1,
                    y: 0,
                    value: 0,
                    var: 0,
                });
                e.bg_events.push(BgEvent {
                    script: 100,
                    event_type: 1,
                    x: 20,
                    z: 21,
                    y: 0,
                    player_facing_dir: 0,
                });
                Ok(())
            })
            .unwrap();

        let ev = get_events_impl(&state, 0).unwrap();
        assert_eq!(ev.warps.len(), 2);
        assert_eq!(
            (ev.warps[1].index, ev.warps[1].x, ev.warps[1].dest_header_id),
            (1, 40, 1)
        );
        assert_eq!(ev.warps[0].dest_warp_id, 2);
        let o = &ev.objects[0];
        assert_eq!(
            (o.index, o.local_id, o.graphics_id, o.script),
            (0, 7, 9, 2001)
        );
        assert_eq!((o.movement_type, o.x, o.z, o.y), (3, 12, 13, -1));
        let c = &ev.coords[0];
        assert_eq!((c.script, c.x, c.z, c.width, c.length), (9000, 3, -2, 2, 1));
        let b = &ev.bg[0];
        assert_eq!((b.script, b.event_type, b.x, b.z, b.y), (100, 1, 20, 21, 0));
    }

    #[test]
    fn events_of_an_unknown_file_are_an_error() {
        let (_fixture, state) = open_synthetic();
        assert!(get_events_impl(&state, 99).is_err());
    }

    /// Gated: Platinum DE header 415 is the player's bedroom; its stairs warp leads to header 414.
    #[test]
    fn vanilla_platinum_player_bedroom_has_the_stairs_warp() {
        let Some(root) = g4_kernel::test_support::vanilla_project() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        if info.game_code != "CPUD" {
            eprintln!("skipped: not Platinum DE");
            return;
        }
        let headers = list_headers_impl(&state).unwrap().headers;
        let ev = get_events_impl(&state, headers[415].event_file_id).unwrap();
        assert_eq!(ev.warps.len(), 1);
        assert_eq!(
            (ev.warps[0].x, ev.warps[0].z, ev.warps[0].dest_header_id),
            (8, 4, 414)
        );
        assert_eq!(ev.objects.len(), 1);
        assert_eq!(ev.coords.len(), 4);
    }
}
