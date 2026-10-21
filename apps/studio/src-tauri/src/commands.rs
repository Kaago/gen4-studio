//! Tauri commands. Each command is a thin wrapper over a testable `*_impl` function.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, MutexGuard};

use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::build::build;
use g4_kernel::error::KernelError;
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::project::{AssetId, AssetKind, HeaderSummary, Project};
use g4_kernel::run::launch_emulator;
use g4_kernel::store::{SaveReport, Store};
use serde::Serialize;
use tauri::{Manager, State};

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

/// What `build_rom` did: how many dirty assets were saved first, the packed asset kinds
/// (`maps`, `matrices`, `areaData`) with their member counts, and the ROM path.
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
) -> Result<(), String> {
    if x >= MAP_TILES || y >= MAP_TILES {
        return Err(format!("cell ({x}, {y}) is outside the 32 x 32 map"));
    }
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session
        .project
        .ensure_unpacked(AssetKind::Map)
        .map_err(|e| e.to_string())?;
    let mut result = Ok(());
    session
        .store
        .edit_map(index, "paint", |map| {
            result = map
                .set_cell(usize::from(x), usize::from(y), Cell { kind, collision })
                .map_err(|e| e.to_string());
        })
        .map_err(|e| e.to_string())?;
    result
}

pub fn undo_impl(state: &AppState) -> Result<bool, String> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.undo().is_some())
}

pub fn redo_impl(state: &AppState) -> Result<bool, String> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.redo().is_some())
}

pub fn dirty_impl(state: &AppState) -> Result<Vec<AssetId>, String> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.dirty())
}

pub fn save_impl(state: &AppState) -> Result<SaveReport, String> {
    let mut guard = lock(state)?;
    session(&mut guard)?.store.save().map_err(|e| e.to_string())
}

/// Saves dirty assets (a stale file aborts with nothing built), then builds the ROM. The
/// state lock is held only for the save, not for the ds-rom build.
pub fn build_rom_impl(state: &AppState, out: &str) -> Result<BuildDto, String> {
    let (project, saved) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        let saved = if session.store.dirty().is_empty() {
            0
        } else {
            session
                .store
                .save()
                .map_err(|e| e.to_string())?
                .written
                .len()
        };
        (Arc::clone(&session.project), saved)
    };
    let report = build(&project, Path::new(out)).map_err(|e| e.to_string())?;
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

pub fn run_rom_impl(emulator: &str, rom: &str) -> Result<u32, String> {
    launch_emulator(Path::new(emulator), Path::new(rom)).map_err(|e| e.to_string())
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
    candidates.into_iter().flatten().find(|p| p.is_file())
}

static MODEL_WORK_COUNTER: AtomicU32 = AtomicU32::new(0);

fn is_fresh(cache: &Path, source: &Path) -> bool {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    match (modified(cache), modified(source)) {
        (Some(c), Some(s)) => c >= s,
        _ => false,
    }
}

/// GLB bytes of the map's terrain model, cached under `<project>/.g4/models/<index:04>.glb`.
/// `apicula` is only needed on a cache miss.
pub fn get_map_model_glb_with(
    state: &AppState,
    index: u16,
    apicula: Option<&Path>,
) -> Result<Vec<u8>, String> {
    // Project, cache path and model bytes come from ONE lock acquisition, so a project switch
    // in between can never write another project's model into this project's cache.
    // `nsbmd` (with the apicula path) is `None` on a fresh cache hit.
    let (models, cache, nsbmd) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        let project = Arc::clone(&session.project);
        project
            .ensure_unpacked(AssetKind::Map)
            .map_err(|e| e.to_string())?;
        let models = project.root.join(".g4").join("models");
        let cache = models.join(format!("{index:04}.glb"));
        if is_fresh(&cache, &project.member_path(AssetKind::Map, index)) {
            (models, cache, None)
        } else {
            let apicula = apicula.ok_or_else(|| {
                KernelError::ToolMissing {
                    name: "apicula",
                    reason:
                        "set G4_APICULA or place apicula next to the app (see docs/dev-setup.md)"
                            .into(),
                }
                .to_string()
            })?;
            let nsbmd = session
                .store
                .map(index)
                .map_err(|e| e.to_string())?
                .model
                .clone();
            (models, cache, Some((nsbmd, apicula)))
        }
    };
    let Some((nsbmd, apicula)) = nsbmd else {
        return std::fs::read(&cache).map_err(|e| e.to_string());
    };
    let unique = MODEL_WORK_COUNTER.fetch_add(1, Ordering::Relaxed);
    let work = models.join(format!(".work-{index:04}-{unique}"));
    let result = (|| -> Result<Vec<u8>, String> {
        let glb = nsbmd_to_glb(apicula, &nsbmd, &work).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(&glb).map_err(|e| e.to_string())?;
        // Publish atomically so a concurrent reader never sees a half-written cache file.
        let tmp = work.join("cache.glb");
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &cache).map_err(|e| e.to_string())?;
        Ok(bytes)
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

pub fn get_map_model_glb_impl(state: &AppState, index: u16) -> Result<Vec<u8>, String> {
    get_map_model_glb_with(state, index, find_apicula().as_deref())
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

/// Raw GLB bytes over IPC (no file paths, no asset protocol). `async` keeps the apicula run
/// off the main thread.
#[tauri::command(async)]
pub fn get_map_model_glb(
    state: State<AppState>,
    index: u16,
) -> Result<tauri::ipc::Response, String> {
    get_map_model_glb_impl(&state, index).map(tauri::ipc::Response::new)
}

#[tauri::command]
pub fn set_permission(
    state: State<AppState>,
    index: u16,
    x: u8,
    y: u8,
    kind: u8,
    collision: u8,
) -> Result<(), String> {
    set_permission_impl(&state, index, x, y, kind, collision)
}

#[tauri::command]
pub fn undo(state: State<AppState>) -> Result<bool, String> {
    undo_impl(&state)
}

#[tauri::command]
pub fn redo(state: State<AppState>) -> Result<bool, String> {
    redo_impl(&state)
}

#[tauri::command]
pub fn dirty(state: State<AppState>) -> Result<Vec<AssetId>, String> {
    dirty_impl(&state)
}

#[tauri::command]
pub fn save(state: State<AppState>) -> Result<SaveReport, String> {
    save_impl(&state)
}

/// The ds-rom build takes seconds, so it runs on a blocking worker, off the main thread and
/// without holding the state lock.
#[tauri::command]
pub async fn build_rom(app: tauri::AppHandle, out: String) -> Result<BuildDto, String> {
    tauri::async_runtime::spawn_blocking(move || build_rom_impl(&app.state::<AppState>(), &out))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn run_rom(emulator: String, rom: String) -> Result<u32, String> {
    run_rom_impl(&emulator, &rom)
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
    fn model_glb_without_apicula_names_the_tool() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let err = get_map_model_glb_with(&state, 0, None).unwrap_err();
        assert!(err.contains("apicula"), "{err}");
        let err =
            get_map_model_glb_with(&state, 0, Some(Path::new("/nonexistent/apicula"))).unwrap_err();
        assert!(err.contains("apicula"), "{err}");
        let err = get_map_model_glb_impl(&AppState::default(), 0).unwrap_err();
        assert_eq!(err, "no project open");
    }

    #[test]
    fn model_glb_is_cached_and_refreshed_when_the_map_changes() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let cache = fixture.dir.path().join(".g4/models/0000.glb");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        // The synthetic map unpacks lazily; unpack first so the cache can be newer than it.
        get_map_impl(&state, 0).unwrap();
        std::fs::write(&cache, b"glTFcached").unwrap();
        // Cache newer than the unpacked map: served without needing apicula.
        assert_eq!(
            get_map_model_glb_with(&state, 0, None).unwrap(),
            b"glTFcached"
        );
        // Map newer than the cache: the cache is stale, so apicula is required again.
        let member = fixture.dir.path().join("unpacked/maps/0000");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&member)
            .unwrap();
        file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
            .unwrap();
        assert!(get_map_model_glb_with(&state, 0, None).is_err());
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
            "#!/bin/sh\nmkdir -p \"$5\"; printf 'glTFfake' > \"$5/m.glb\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let glb = get_map_model_glb_with(&state, 1, Some(&tool)).unwrap();
        assert_eq!(glb, b"glTFfake");
        let models = fixture.dir.path().join(".g4/models");
        let names: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["0001.glb"]);
        // Second call is a cache hit and does not need the tool.
        assert_eq!(
            get_map_model_glb_with(&state, 1, None).unwrap(),
            b"glTFfake"
        );
    }

    fn open_synthetic() -> (g4_kernel::test_support::Synthetic, AppState) {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        (fixture, state)
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
            set_permission_impl(&AppState::default(), 0, 0, 0, 0, 0).unwrap_err(),
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
        assert!(err.contains("stale"), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn run_rom_reports_a_missing_emulator() {
        let err = run_rom_impl("/nonexistent/melonDS", "/tmp/x.nds").unwrap_err();
        assert!(err.contains("emulator"), "{err}");
    }

    /// Gated end-to-end check on a copy of a vanilla project (`G4_TEST_PROJECT`): paint a
    /// permission on a map of header 0's matrix, build, and re-open the copy.
    #[test]
    fn vanilla_paint_save_build_on_copy() {
        let Some((tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let root = root.to_str().unwrap();
        let state = AppState::default();
        open_project_impl(&state, root).unwrap();
        let headers = list_headers_impl(&state).unwrap();
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
    }
}
