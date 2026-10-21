//! Tauri commands. Each command is a thin wrapper over a testable `*_impl` function.

use std::sync::{Arc, MutexGuard};

use g4_formats::map::MapFile;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::project::{AssetKind, HeaderSummary, Project};
use g4_kernel::store::Store;
use serde::Serialize;
use tauri::State;

use crate::state::{AppState, Session};

#[derive(Serialize)]
pub struct ProjectInfo {
    pub root: String,
    pub game_code: String,
    pub family: String,
    pub language: String,
    pub header_count: usize,
}

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

fn lock(state: &AppState) -> Result<MutexGuard<'_, Option<Session>>, String> {
    state
        .0
        .lock()
        .map_err(|_| "internal state is poisoned".to_string())
}

fn session(guard: &mut Option<Session>) -> Result<&mut Session, String> {
    guard.as_mut().ok_or_else(|| "no project open".to_string())
}

pub fn open_project_impl(state: &AppState, path: &str) -> Result<ProjectInfo, String> {
    let project = Project::open(path).map_err(|e| e.to_string())?;
    let header_count = project.headers().map_err(|e| e.to_string())?.len();
    let info = ProjectInfo {
        root: project.root.display().to_string(),
        game_code: project.game_code.clone(),
        family: format!("{:?}", project.family),
        language: format!("{:?}", project.language),
        header_count,
    };
    let project = Arc::new(project);
    let store = Store::new(Arc::clone(&project));
    *lock(state)? = Some(Session { project, store });
    Ok(info)
}

pub fn list_headers_impl(state: &AppState) -> Result<Vec<HeaderSummary>, String> {
    let mut guard = lock(state)?;
    session(&mut guard)?
        .project
        .headers()
        .map_err(|e| e.to_string())
}

pub fn get_matrix_impl(state: &AppState, index: u16) -> Result<MatrixDto, String> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session
        .project
        .ensure_unpacked(AssetKind::Matrix)
        .map_err(|e| e.to_string())?;
    let matrix = session.store.matrix(index).map_err(|e| e.to_string())?;
    Ok(MatrixDto::from_matrix(index, matrix))
}

pub fn get_map_impl(state: &AppState, index: u16) -> Result<MapDto, String> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session
        .project
        .ensure_unpacked(AssetKind::Map)
        .map_err(|e| e.to_string())?;
    let map = session.store.map(index).map_err(|e| e.to_string())?;
    Ok(MapDto::from_map(index, map))
}

#[tauri::command]
pub fn open_project(state: State<AppState>, path: String) -> Result<ProjectInfo, String> {
    open_project_impl(&state, &path)
}

#[tauri::command]
pub fn list_headers(state: State<AppState>) -> Result<Vec<HeaderSummary>, String> {
    list_headers_impl(&state)
}

#[tauri::command]
pub fn get_matrix(state: State<AppState>, index: u16) -> Result<MatrixDto, String> {
    get_matrix_impl(&state, index)
}

#[tauri::command]
pub fn get_map(state: State<AppState>, index: u16) -> Result<MapDto, String> {
    get_map_impl(&state, index)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::state::AppState;

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
    fn commands_work_on_the_synthetic_project() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(info.game_code, "CPUE");
        assert_eq!(info.header_count, 2);
        let headers = list_headers_impl(&state).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].internal_name, "T01");
        let map = get_map_impl(&state, 0).unwrap();
        assert_eq!(map.permissions.len(), 1024);
        let matrix = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((matrix.width, matrix.height), (2, 1));
        assert_eq!(matrix.maps, vec![Some(0), Some(1)]);
    }

    #[test]
    fn commands_fail_without_an_open_project() {
        let state = AppState::default();
        assert!(list_headers_impl(&state).is_err());
        assert!(get_map_impl(&state, 0).is_err());
    }
}
