//! Tauri commands. Each command is a thin wrapper over a testable `*_impl` function.

use std::path::{Path, PathBuf};
use std::sync::{Arc, MutexGuard};

use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::build::build;
use g4_kernel::error::KernelError;
use g4_kernel::export::{glb_cache_get, glb_cached, glb_fail_get, glb_fail_record};
use g4_kernel::project::{AssetId, AssetKind, HeaderListing, Project};
use g4_kernel::run::launch_emulator;
use g4_kernel::store::{EventFile, SaveReport, Store};
use g4_kernel::tiles::TileBehavior;
use serde::Serialize;
use tauri::{Manager, State};

use crate::errors::CmdError;
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

fn lock(state: &AppState) -> Result<MutexGuard<'_, Option<Session>>, CmdError> {
    state
        .session
        .lock()
        .map_err(|_| "internal state is poisoned".into())
}

fn session(guard: &mut Option<Session>) -> Result<&mut Session, CmdError> {
    guard.as_mut().ok_or_else(|| "no project open".into())
}

pub fn open_project_impl(state: &AppState, path: &str) -> Result<ProjectInfo, CmdError> {
    let project = Project::open(path).map_err(CmdError::from)?;
    // Only the header table: location names are cosmetic and must not gate opening.
    let header_count = project.header_count().map_err(CmdError::from)?;
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

pub fn list_headers_impl(state: &AppState) -> Result<HeaderListing, CmdError> {
    // Decoding the text archive (and unpacking it on first use) happens without the lock.
    let project = {
        let mut guard = lock(state)?;
        Arc::clone(&session(&mut guard)?.project)
    };
    project.header_listing().map_err(CmdError::from)
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

/// What `build_rom` did: how many dirty assets were saved first, the packed asset kinds (by
/// unpacked folder name, e.g. `maps`, `eventFiles`) with their member counts, and the ROM path.
#[derive(Debug, Serialize)]
pub struct BuildDto {
    pub saved: usize,
    pub packed: Vec<(String, usize)>,
    pub rom: String,
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

pub fn undo_impl(state: &AppState) -> Result<bool, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.undo().is_some())
}

pub fn redo_impl(state: &AppState) -> Result<bool, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.redo().is_some())
}

pub fn dirty_impl(state: &AppState) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.dirty())
}

pub fn save_impl(state: &AppState) -> Result<SaveReport, CmdError> {
    let mut guard = lock(state)?;
    session(&mut guard)?.store.save().map_err(CmdError::from)
}

/// Drops the own changes (and their undo/redo entries) of one asset and reloads it from its
/// file on disk.
pub fn reload_asset_impl(state: &AppState, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.reload_from_disk(AssetId { kind, index })?;
    Ok(())
}

/// Keeps the in-memory version of one asset and takes the file on disk as the new baseline,
/// so the next save no longer reports it as stale.
pub fn accept_disk_impl(state: &AppState, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.accept_disk_as_base(AssetId { kind, index })?;
    Ok(())
}

/// Saves dirty assets (a stale file aborts with nothing built), then builds the ROM. The
/// state lock is held only for the save, not for the ds-rom build.
pub fn build_rom_impl(state: &AppState, out: &str) -> Result<BuildDto, CmdError> {
    let (project, saved) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        let saved = if session.store.dirty().is_empty() {
            0
        } else {
            session.store.save()?.written.len()
        };
        (Arc::clone(&session.project), saved)
    };
    let report = build(&project, Path::new(out)).map_err(CmdError::from)?;
    Ok(BuildDto {
        saved,
        packed: report
            .packed
            .iter()
            .map(|(kind, count)| (project.archives.unpacked_name(*kind).to_owned(), *count))
            .collect(),
        rom: report.rom.display().to_string(),
    })
}

pub fn run_rom_impl(emulator: &str, rom: &str) -> Result<u32, CmdError> {
    launch_emulator(Path::new(emulator), Path::new(rom)).map_err(CmdError::from)
}

/// Finds the apicula executable: `G4_APICULA`, then `apicula` next to the running binary, then
/// (debug builds only) the repository's `tools/apicula/bin/apicula`. A set `G4_APICULA` always
/// wins, even when it points nowhere, so a typo is reported instead of silently ignored.
fn find_apicula() -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("G4_APICULA").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(env));
    }
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("apicula")));
    let mut candidates = vec![beside_exe];
    if cfg!(debug_assertions) {
        candidates.push(Some(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/apicula/bin/apicula"
        ))));
    }
    // An empty file is the build-time placeholder for the bundled sidecar (see build.rs), not apicula.
    candidates.into_iter().flatten().find(|p| {
        p.metadata()
            .map(|m| m.is_file() && m.len() > 0)
            .unwrap_or(false)
    })
}

/// Why a map's model has no textures although it should have: the area data or tileset it
/// depends on cannot be used. `tileset` is the tileset index when it is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextureIssue {
    pub tileset: Option<u16>,
    /// First line of the (English) kernel error, for display next to the German note.
    pub reason: String,
}

fn issue(tileset: Option<u16>, reason: &str) -> TextureIssue {
    let line = reason.lines().next().unwrap_or_default();
    TextureIssue {
        tileset,
        reason: line.chars().take(200).collect(),
    }
}

/// What the model conversion needs, read under one short lock acquisition.
struct ModelInputs {
    cache_dir: PathBuf,
    nsbmd: Vec<u8>,
    tileset: Option<Vec<u8>>,
    tileset_id: Option<u16>,
    /// Set when the area data or the tileset could not be read: `tileset` is then `None`.
    problem: Option<TextureIssue>,
}

/// Cache directory, model and tileset come from ONE short lock acquisition, so a project
/// switch in between can never mix two projects. A tileset that cannot be resolved (unreadable
/// area data, unreadable or corrupt texture archive) is not an error: the map is converted
/// without textures and the problem is reported in `problem`.
fn model_inputs(state: &AppState, index: u16, area_data_id: u8) -> Result<ModelInputs, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let nsbmd = session.store.map(index)?.model.clone();
    let mut inputs = ModelInputs {
        cache_dir: project.root.join(".g4").join("models"),
        nsbmd,
        tileset: None,
        tileset_id: None,
        problem: None,
    };
    match project.map_tileset_index(area_data_id) {
        Ok(Some(id)) => {
            inputs.tileset_id = Some(id);
            match project.map_tileset_member(id) {
                Ok(bytes) => inputs.tileset = bytes,
                Err(e) => inputs.problem = Some(issue(Some(id), &e.to_string())),
            }
        }
        Ok(None) => {}
        Err(e) => inputs.problem = Some(issue(None, &e.to_string())),
    }
    Ok(inputs)
}

/// The apicula to run, or a `ToolMissing` error naming the fix. Checked before any
/// conversion so a missing tool is never mistaken for a tileset that cannot be converted.
fn require_apicula(apicula: Option<&Path>) -> Result<&Path, KernelError> {
    let missing = |reason: String| KernelError::ToolMissing {
        name: "apicula",
        reason,
    };
    let apicula = apicula.ok_or_else(|| {
        missing("set G4_APICULA or place apicula next to the app (see docs/dev-setup.md)".into())
    })?;
    if !apicula.exists() {
        return Err(missing(format!("not found: {}", apicula.display())));
    }
    Ok(apicula)
}

/// GLB bytes of the map's terrain model with the map's texture set (from `area_data_id`),
/// cached by content hash under `<project>/.g4/models/`. `apicula` is only needed on a cache
/// miss. A map whose area data or tileset does not exist or cannot be read, or whose textured
/// conversion fails, is converted without textures; a failed textured conversion is recorded
/// next to the cache (`<key>.fail`) so later loads skip it. The reason is available through
/// [`map_model_texture_status_impl`].
pub fn get_map_model_glb_with(
    state: &AppState,
    index: u16,
    area_data_id: u8,
    apicula: Option<&Path>,
) -> Result<Vec<u8>, CmdError> {
    let ModelInputs {
        cache_dir,
        nsbmd,
        tileset,
        ..
    } = model_inputs(state, index, area_data_id)?;
    let mut textured_failure = None;
    if let Some(tileset) = tileset.as_deref() {
        if let Some(bytes) = glb_cache_get(&cache_dir, &nsbmd, Some(tileset))? {
            return Ok(bytes);
        }
        if glb_fail_get(&cache_dir, &nsbmd, Some(tileset))?.is_none() {
            let apicula = require_apicula(apicula)?;
            match glb_cached(&cache_dir, apicula, &nsbmd, Some(tileset)) {
                Ok(bytes) => return Ok(bytes),
                Err(e) => textured_failure = Some(e.to_string()),
            }
        }
    }
    let plain = match glb_cache_get(&cache_dir, &nsbmd, None)? {
        Some(bytes) => bytes,
        None => glb_cached(&cache_dir, require_apicula(apicula)?, &nsbmd, None)?,
    };
    // Recorded only now: if the untextured conversion fails too, the tileset is not to blame.
    if let (Some(reason), Some(tileset)) = (textured_failure, tileset.as_deref()) {
        // Best effort: without a marker the next load simply tries the tileset again.
        let _ = glb_fail_record(&cache_dir, &nsbmd, Some(tileset), &reason);
    }
    Ok(plain)
}

/// Whether the model of map `index` is shown without textures because of its tileset: the area
/// data or tileset cannot be read, or an earlier conversion with this tileset failed (see
/// [`get_map_model_glb_with`]). `None` when nothing is wrong. Call it after the model was
/// requested; it never runs apicula.
pub fn map_model_texture_status_impl(
    state: &AppState,
    index: u16,
    area_data_id: u8,
) -> Result<Option<TextureIssue>, CmdError> {
    let inputs = model_inputs(state, index, area_data_id)?;
    if inputs.problem.is_some() {
        return Ok(inputs.problem);
    }
    let Some(tileset) = inputs.tileset.as_deref() else {
        return Ok(None);
    };
    Ok(
        glb_fail_get(&inputs.cache_dir, &inputs.nsbmd, Some(tileset))?
            .map(|reason| issue(inputs.tileset_id, &reason)),
    )
}

pub fn get_map_model_glb_impl(
    state: &AppState,
    index: u16,
    area_data_id: u8,
) -> Result<Vec<u8>, CmdError> {
    get_map_model_glb_with(state, index, area_data_id, find_apicula().as_deref())
}

#[tauri::command]
pub fn open_project(state: State<AppState>, path: String) -> Result<ProjectInfo, CmdError> {
    open_project_impl(&state, &path)
}

#[tauri::command]
pub fn list_headers(state: State<AppState>) -> Result<HeaderListing, CmdError> {
    list_headers_impl(&state)
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

/// Raw GLB bytes over IPC (no file paths, no asset protocol). `async` keeps the apicula run
/// off the main thread.
#[tauri::command(async)]
pub fn get_map_model_glb(
    state: State<AppState>,
    index: u16,
    area_data_id: u8,
) -> Result<tauri::ipc::Response, CmdError> {
    get_map_model_glb_impl(&state, index, area_data_id).map(tauri::ipc::Response::new)
}

/// Why a map's model is untextured because of its tileset, if it is (see
/// [`map_model_texture_status_impl`]). Cheap: reads a few small files, never runs apicula.
#[tauri::command]
pub fn map_model_texture_status(
    state: State<AppState>,
    index: u16,
    area_data_id: u8,
) -> Result<Option<TextureIssue>, CmdError> {
    map_model_texture_status_impl(&state, index, area_data_id)
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

#[tauri::command]
pub fn undo(state: State<AppState>) -> Result<bool, CmdError> {
    undo_impl(&state)
}

#[tauri::command]
pub fn redo(state: State<AppState>) -> Result<bool, CmdError> {
    redo_impl(&state)
}

#[tauri::command]
pub fn dirty(state: State<AppState>) -> Result<Vec<AssetId>, CmdError> {
    dirty_impl(&state)
}

#[tauri::command]
pub fn save(state: State<AppState>) -> Result<SaveReport, CmdError> {
    save_impl(&state)
}

#[tauri::command]
pub fn reload_asset(state: State<AppState>, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    reload_asset_impl(&state, kind, index)
}

#[tauri::command]
pub fn accept_disk(state: State<AppState>, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    accept_disk_impl(&state, kind, index)
}

/// The ds-rom build takes seconds, so it runs on a blocking worker, off the main thread and
/// without holding the state lock.
#[tauri::command]
pub async fn build_rom(app: tauri::AppHandle, out: String) -> Result<BuildDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || build_rom_impl(&app.state::<AppState>(), &out))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// Quits after the user confirmed discarding unsaved edits: the flag lets the exit request
/// raised by `exit` through the exit guard.
#[tauri::command]
pub fn force_quit(app: tauri::AppHandle) {
    app.state::<AppState>()
        .force_quit
        .store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
pub fn run_rom(emulator: String, rom: String) -> Result<u32, CmdError> {
    run_rom_impl(&emulator, &rom)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use g4_kernel::export::glb_cache_key;

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
    fn commands_work_on_the_synthetic_project() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(info.game_code, "CPUE");
        assert_eq!(info.header_count, 2);
        let headers = list_headers_impl(&state).unwrap().headers;
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].internal_name, "T01");
        let map = get_map_impl(&state, 0).unwrap();
        assert_eq!(map.permissions.len(), 1024);
        let matrix = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((matrix.width, matrix.height), (2, 1));
        assert_eq!(matrix.maps, vec![Some(0), Some(1)]);
    }

    #[test]
    fn model_glb_without_apicula_names_the_tool() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let err = get_map_model_glb_with(&state, 0, 0, None).unwrap_err();
        assert!(err.message.contains("apicula"), "{}", err.message);
        let err = get_map_model_glb_with(&state, 0, 0, Some(Path::new("/nonexistent/apicula")))
            .unwrap_err();
        assert!(err.message.contains("apicula"), "{}", err.message);
        let err = get_map_model_glb_impl(&AppState::default(), 0, 0).unwrap_err();
        assert_eq!(err.message, "no project open");
    }

    #[test]
    fn model_glb_cache_is_keyed_by_model_and_tileset_content() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let models = fixture.dir.path().join(".g4/models");
        std::fs::create_dir_all(&models).unwrap();
        let model = g4_kernel::test_support::synthetic_map_bytes(0);
        let model = MapFile::read(&model, g4_formats::game::Family::Pt)
            .unwrap()
            .model;
        // Synthetic area data 0 selects the only tileset, area data 1 a missing one.
        let tileset = b"BTX0\0\0\0\0";
        let textured = glb_cache_key(&model, Some(tileset));
        let plain = glb_cache_key(&model, None);
        std::fs::write(models.join(format!("{textured}.glb")), b"glTFtextured").unwrap();
        std::fs::write(models.join(format!("{plain}.glb")), b"glTFplain").unwrap();
        // Hits are served without apicula.
        assert_eq!(
            get_map_model_glb_with(&state, 0, 0, None).unwrap(),
            b"glTFtextured"
        );
        assert_eq!(
            get_map_model_glb_with(&state, 0, 1, None).unwrap(),
            b"glTFplain"
        );
        assert_eq!(
            get_map_model_glb_with(&state, 0, 200, None).unwrap(),
            b"glTFplain"
        );
    }

    #[cfg(unix)]
    #[test]
    fn model_glb_is_converted_once_and_leaves_no_work_files() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let tool = fixture.dir.path().join("fake-apicula");
        std::fs::write(
            &tool,
            "#!/bin/sh\nwhile [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
             mkdir -p \"$out\"; printf 'glTFfake' > \"$out/m.glb\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFfake");
        let models = fixture.dir.path().join(".g4/models");
        let names: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names.len(), 1);
        assert!(names[0].ends_with(".glb"), "{names:?}");
        // Second call is a cache hit and does not need the tool.
        assert_eq!(
            get_map_model_glb_with(&state, 1, 0, None).unwrap(),
            b"glTFfake"
        );
        // A map with a missing area data entry still converts, just without textures.
        let glb = get_map_model_glb_with(&state, 1, 200, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFfake");
        assert_eq!(std::fs::read_dir(&models).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn model_glb_falls_back_to_untextured_when_the_textured_conversion_fails() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let tool = fixture.dir.path().join("fake-apicula");
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>; a fourth argument other than -o is
        // the tileset, which this tool rejects (a "bad tileset").
        std::fs::write(
            &tool,
            "#!/bin/sh\n[ \"$4\" = \"-o\" ] || { echo 'bad tileset' >&2; exit 2; }\n\
             while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
             mkdir -p \"$out\"; printf 'glTFplain' > \"$out/m.glb\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Area data 0 selects the synthetic tileset: the textured run fails, the retry
        // without the tileset yields an untextured model instead of an error.
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFplain");
        // The untextured result is cached under the plain key.
        let models = fixture.dir.path().join(".g4/models");
        let names: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            names.iter().filter(|n| n.ends_with(".glb")).count(),
            1,
            "{names:?}"
        );
        // A model that fails without a tileset too is still an error.
        std::fs::write(&tool, "#!/bin/sh\necho boom >&2; exit 3\n").unwrap();
        let err = get_map_model_glb_with(&state, 0, 0, Some(&tool)).unwrap_err();
        assert!(err.message.contains("boom"), "{}", err.message);
    }

    #[cfg(unix)]
    fn counting_tool(dir: &Path, glb: &str, fail_with_tileset: bool) -> (PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let tool = dir.join("fake-apicula");
        let calls = dir.join("apicula-calls");
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>. Logs "plain" or "textured" per run.
        let fail = if fail_with_tileset {
            "[ \"$4\" = \"-o\" ] || { echo 'bad tileset' >&2; exit 2; }\n"
        } else {
            ""
        };
        std::fs::write(
            &tool,
            format!(
                "#!/bin/sh\nif [ \"$4\" = \"-o\" ]; then echo plain >> '{calls}'; \
                 else echo textured >> '{calls}'; fi\n{fail}\
                 while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
                 mkdir -p \"$out\"; printf '{glb}' > \"$out/m.glb\"\n",
                calls = calls.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        (tool, calls)
    }

    #[cfg(unix)]
    fn recorded_calls(calls: &Path) -> Vec<String> {
        std::fs::read_to_string(calls)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn failing_tileset_is_remembered_and_not_retried() {
        let (fixture, state) = open_synthetic();
        let (tool, calls) = counting_tool(fixture.dir.path(), "glTFplain", true);
        let models = fixture.dir.path().join(".g4/models");
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);

        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFplain");
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        let fails: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".fail"))
            .collect();
        assert_eq!(fails.len(), 1, "one marker");
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, Some(0));
        assert!(issue.reason.contains("bad tileset"), "{}", issue.reason);

        // Even with the untextured result gone, the tileset is not tried again.
        for e in std::fs::read_dir(&models).unwrap() {
            let path = e.unwrap().path();
            if path.extension().is_some_and(|x| x == "glb") {
                std::fs::remove_file(path).unwrap();
            }
        }
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFplain");
        assert_eq!(recorded_calls(&calls), ["textured", "plain", "plain"]);
        // A map without a tileset problem reports none.
        assert_eq!(map_model_texture_status_impl(&state, 1, 200).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_that_is_not_the_tilesets_leaves_no_marker() {
        let (fixture, state) = open_synthetic();
        let (tool, _calls) = counting_tool(fixture.dir.path(), "x", true);
        std::fs::write(&tool, "#!/bin/sh\necho boom >&2; exit 3\n").unwrap();
        assert!(get_map_model_glb_with(&state, 1, 0, Some(&tool)).is_err());
        let models = fixture.dir.path().join(".g4/models");
        let fails = std::fs::read_dir(&models)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "fail")
            })
            .count();
        assert_eq!(fails, 0);
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_area_data_or_tileset_member_still_converts_untextured() {
        let (fixture, state) = open_synthetic();
        let (tool, calls) = counting_tool(fixture.dir.path(), "glTFplain", false);
        let project = Project::open(fixture.dir.path()).unwrap();
        project.ensure_unpacked(AssetKind::AreaData).unwrap();
        project.ensure_unpacked(AssetKind::MapTexture).unwrap();

        // A truncated area data member.
        std::fs::write(project.member_path(AssetKind::AreaData, 0), [1, 2, 3]).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFplain");
        assert_eq!(recorded_calls(&calls), ["plain"]);
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, None);
        assert!(!issue.reason.is_empty());

        // A tileset member that cannot be read.
        use std::os::unix::fs::PermissionsExt;
        let member = project.member_path(AssetKind::MapTexture, 0);
        std::fs::set_permissions(&member, std::fs::Permissions::from_mode(0o000)).unwrap();
        std::fs::write(project.member_path(AssetKind::AreaData, 0), [0u8; 8]).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFplain");
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, Some(0));
    }

    fn open_synthetic() -> (g4_kernel::test_support::Synthetic, AppState) {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        (fixture, state)
    }

    #[test]
    fn commands_unpack_on_first_access_without_help_from_the_wrapper() {
        // None of these call ensure_unpacked themselves; the store unpacks what it loads.
        let (fixture, state) = open_synthetic();
        assert!(!fixture.dir.path().join("unpacked").exists());
        reload_asset_impl(&state, AssetKind::Event, 0).unwrap();
        assert!(
            fixture
                .dir
                .path()
                .join("unpacked/eventFiles/0000")
                .is_file()
        );
        get_matrix_impl(&state, 0).unwrap();
        get_map_impl(&state, 0).unwrap();
        get_events_impl(&state, 1).unwrap();
        set_permission_impl(&state, 2, 0, 0, 1, 0).unwrap();
        for dir in ["matrices", "maps", "eventFiles"] {
            assert!(
                fixture.dir.path().join("unpacked").join(dir).is_dir(),
                "{dir}"
            );
        }
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
    fn save_writes_dirty_maps_and_clears_dirty() {
        let (fixture, state) = open_synthetic();
        assert!(save_impl(&state).unwrap().written.is_empty());
        set_permission_impl(&state, 1, 5, 6, 2, 0x80).unwrap();
        let report = save_impl(&state).unwrap();
        assert_eq!(report.written.len(), 1);
        assert_eq!(report.written[0].index, 1);
        assert!(dirty_impl(&state).unwrap().is_empty());
        let bytes = std::fs::read(fixture.dir.path().join("unpacked/maps/0001")).unwrap();
        let map = MapFile::read(&bytes, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(map.permissions[6 * 32 + 5].collision, 0x80);
    }

    #[test]
    fn build_saves_first_and_a_stale_file_aborts_it() {
        let (fixture, state) = open_synthetic();
        let out = fixture.dir.path().join("out/rom.nds");
        set_permission_impl(&state, 0, 1, 1, 2, 0x80).unwrap();
        // The synthetic project has no real ds-rom config, so the ROM build itself fails,
        // but only after the dirty map was saved.
        assert!(build_rom_impl(&state, out.to_str().unwrap()).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let on_disk = std::fs::read(fixture.dir.path().join("unpacked/maps/0000")).unwrap();
        let map = MapFile::read(&on_disk, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(
            map.permissions[32 + 1],
            g4_formats::map::Cell {
                kind: 2,
                collision: 0x80
            }
        );

        // A member changed behind the editor's back: the save refuses, so nothing is built.
        set_permission_impl(&state, 0, 2, 2, 3, 0x80).unwrap();
        let member = fixture.dir.path().join("unpacked/maps/0000");
        let mut bytes = std::fs::read(&member).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&member, &bytes).unwrap();
        let err = build_rom_impl(&state, out.to_str().unwrap()).unwrap_err();
        assert!(err.message.contains("changed on disk"), "{}", err.message);
        assert!(!out.exists());
    }

    fn corrupt_map_member_on_disk(fixture: &g4_kernel::test_support::Synthetic) {
        let path = fixture.dir.path().join("unpacked/maps/0000");
        let mut b = std::fs::read(&path).unwrap();
        b[20] ^= 0xFF;
        std::fs::write(&path, &b).unwrap();
    }

    #[test]
    fn save_stale_returns_structured_error() {
        let (s, state) = open_synthetic();
        set_permission_impl(&state, 0, 1, 1, 0, 0x80).unwrap();
        corrupt_map_member_on_disk(&s);
        let err = save_impl(&state).unwrap_err();
        assert_eq!(err.code, "stale");
        assert_eq!(
            err.asset,
            Some(AssetId {
                kind: AssetKind::Map,
                index: 0
            })
        );
        assert!(
            err.message.starts_with("map 0000 changed on disk"),
            "{}",
            err.message
        );
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn accept_disk_keeps_own_change_and_lets_save_proceed() {
        let (s, state) = open_synthetic();
        set_permission_impl(&state, 0, 1, 1, 2, 0x80).unwrap();
        corrupt_map_member_on_disk(&s);
        assert_eq!(save_impl(&state).unwrap_err().code, "stale");
        accept_disk_impl(&state, AssetKind::Map, 0).unwrap();
        // Own edit is still pending and now saves over the accepted disk state.
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        assert_eq!(save_impl(&state).unwrap().written.len(), 1);
        assert!(dirty_impl(&state).unwrap().is_empty());
        let bytes = std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap();
        let map = MapFile::read(&bytes, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(map.permissions[32 + 1].kind, 2);
    }

    #[test]
    fn missing_project_and_missing_tool_have_their_own_codes() {
        let err = dirty_impl(&AppState::default()).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("error", "no project open")
        );
        let (_s, state) = open_synthetic();
        let err = get_map_model_glb_with(&state, 0, 0, None).unwrap_err();
        assert_eq!(err.code, "tool_missing");
        assert!(reload_asset_impl(&AppState::default(), AssetKind::Map, 0).is_err());
    }

    #[test]
    fn run_rom_reports_a_missing_emulator() {
        let err = run_rom_impl("/nonexistent/melonDS", "/tmp/x.nds").unwrap_err();
        assert!(err.message.contains("emulator"), "{}", err.message);
    }

    /// Gated end-to-end check on a copy of a vanilla project (`G4_TEST_PROJECT`): paint a
    /// permission on a map of header 0's matrix, build, and re-open the copy.
    #[test]
    fn open_and_list_survive_a_broken_text_archive() {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("files/msgdata/pl_msg.narc"),
            g4_formats::narc::Narc {
                members: vec![vec![0xAA; 8]],
            }
            .write(),
        )
        .unwrap();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(info.header_count, 2);
        // Opening never touches the text archive.
        assert!(!fixture.dir.path().join("unpacked/textArchives").exists());
        let listing = list_headers_impl(&state).unwrap();
        assert_eq!(listing.headers.len(), 2);
        assert!(listing.headers.iter().all(|h| h.location_name.is_empty()));
        assert!(listing.names_error.is_some_and(|e| e.contains("433")));
        assert!(get_map_impl(&state, 0).is_ok());
    }

    /// Timing of the backend part of loading a large matrix, as the frontend drives it: one
    /// `get_map_model_glb` per distinct map, four at a time. Gated (`G4_TEST_PROJECT`, a
    /// vanilla copy is used) and `#[ignore]`d; run with
    /// `G4_APICULA=... cargo test -p studio --release -- --ignored --nocapture bench_matrix_models`.
    /// `G4_BENCH_HEADER` picks the header (default 3: Platinum C01 with its 900-cell matrix).
    #[test]
    #[ignore = "timing run, minutes on a cold cache"]
    fn bench_matrix_models() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Instant;
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let header: usize = std::env::var("G4_BENCH_HEADER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let h = list_headers_impl(&state).unwrap().headers[header].clone();
        let matrix = get_matrix_impl(&state, h.matrix_id).unwrap();
        let mut ids: Vec<u16> = matrix.maps.iter().flatten().copied().collect();
        let cells = ids.len();
        ids.sort_unstable();
        ids.dedup();
        let apicula = find_apicula();
        let t = Instant::now();
        for id in &ids {
            get_map_impl(&state, *id).unwrap();
        }
        eprintln!("get_map x{}: {:?}", ids.len(), t.elapsed());
        for pass in ["cold", "warm"] {
            let next = AtomicUsize::new(0);
            let failed = AtomicUsize::new(0);
            let bytes = AtomicUsize::new(0);
            let t = Instant::now();
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(|| {
                        loop {
                            let i = next.fetch_add(1, Ordering::SeqCst);
                            let Some(id) = ids.get(i) else { break };
                            match get_map_model_glb_with(
                                &state,
                                *id,
                                h.area_data_id,
                                apicula.as_deref(),
                            ) {
                                Ok(glb) => bytes.fetch_add(glb.len(), Ordering::SeqCst),
                                Err(_) => failed.fetch_add(1, Ordering::SeqCst),
                            };
                        }
                    });
                }
            });
            eprintln!(
                "{pass}: {} models ({cells} cells, header {} {}) in {:?}, {} failed, {} MB",
                ids.len(),
                h.id,
                h.internal_name,
                t.elapsed(),
                failed.load(Ordering::SeqCst),
                bytes.load(Ordering::SeqCst) / 1_000_000
            );
        }
    }

    #[test]
    fn vanilla_paint_save_build_on_copy() {
        let Some((tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let root = root.to_str().unwrap();
        let state = AppState::default();
        open_project_impl(&state, root).unwrap();
        let headers = list_headers_impl(&state).unwrap().headers;
        let matrix = get_matrix_impl(&state, headers[0].matrix_id).unwrap();
        let map_id = matrix
            .maps
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("matrix has a map");
        let (x, y) = (3u8, 3u8);
        let at = usize::from(y) * 32 + usize::from(x);
        let [kind, collision] = get_map_impl(&state, map_id).unwrap().permissions[at];
        let flipped = if collision == 0x80 { 0x00 } else { 0x80 };
        set_permission_impl(&state, map_id, x, y, kind, flipped).unwrap();
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);

        let out = tmp.path().join("out.nds");
        let report = build_rom_impl(&state, out.to_str().unwrap()).unwrap();
        assert_eq!(report.saved, 1);
        assert!(report.packed.iter().any(|(k, n)| k == "maps" && *n > 0));
        assert!(std::fs::metadata(&out).unwrap().len() > 1_000_000);
        assert_eq!(report.rom, out.display().to_string());
        assert!(dirty_impl(&state).unwrap().is_empty());

        let reopened = AppState::default();
        open_project_impl(&reopened, root).unwrap();
        assert_eq!(
            get_map_impl(&reopened, map_id).unwrap().permissions[at],
            [kind, flipped]
        );
    }

    #[test]
    fn commands_fail_without_an_open_project() {
        let state = AppState::default();
        assert!(list_headers_impl(&state).is_err());
        assert!(get_map_impl(&state, 0).is_err());
        assert!(get_events_impl(&state, 0).is_err());
    }

    #[test]
    fn headers_carry_their_event_file_id() {
        let (_fixture, state) = open_synthetic();
        // The synthetic arm9 is zero-filled, so every header points at event file 0.
        let headers = list_headers_impl(&state).unwrap().headers;
        assert!(headers.iter().all(|h| h.event_file_id == 0));
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
