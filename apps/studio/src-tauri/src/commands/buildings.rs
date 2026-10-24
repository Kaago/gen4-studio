//! Map buildings: textured GLB models (through apicula) and editing of the building records.

use std::path::Path;
use std::sync::Arc;

use g4_formats::map::Building;
use g4_kernel::export::apicula_timeout;
use g4_kernel::project::Project;
use serde::Deserialize;
use tauri::{Manager, State};

use super::models::{find_apicula, glb_with_fallback};
use super::world::{FX_ONE, TILE_RAW};
use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// fx32 raw value of the scale 1.0.
const SCALE_ONE: i32 = 0x1000;

/// The fields of a building the editor changes; `None` leaves a field as it is. Positions are
/// in tiles from the map centre (`y` vertical), scales are factors (1.0 = unscaled). The
/// rotation is not part of it: the game ignores it for map buildings, so it stays verbatim.
#[derive(Debug, Default, Deserialize)]
pub struct BuildingPatch {
    pub model_id: Option<u32>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub sx: Option<f32>,
    pub sy: Option<f32>,
    pub sz: Option<f32>,
}

/// `value * unit` rounded to the nearest fx32 raw value; an error when it does not fit.
fn to_fx(value: f32, unit: f32, what: &str) -> Result<i32, CmdError> {
    let raw = (f64::from(value) * f64::from(unit)).round();
    if !raw.is_finite() || raw < f64::from(i32::MIN) || raw > f64::from(i32::MAX) {
        return Err(format!("building {what} {value} is out of range").into());
    }
    Ok(raw as i32)
}

impl BuildingPatch {
    /// Applies the patch to `b`, or fails without changing it.
    fn apply(&self, b: &mut Building) -> Result<(), CmdError> {
        let mut next = b.clone();
        if let Some(id) = self.model_id {
            next.model_id = id;
        }
        for (i, (value, what)) in [(self.x, "x"), (self.y, "y"), (self.z, "z")]
            .into_iter()
            .enumerate()
        {
            if let Some(v) = value {
                next.position[i] = to_fx(v, TILE_RAW, what)?;
            }
        }
        for (i, (value, what)) in [(self.sx, "sx"), (self.sy, "sy"), (self.sz, "sz")]
            .into_iter()
            .enumerate()
        {
            if let Some(v) = value {
                next.scale[i] = to_fx(v, FX_ONE, what)?;
            }
        }
        *b = next;
        Ok(())
    }
}

fn missing_building(map: u16, index: u16) -> CmdError {
    format!("building {index} does not exist in map {map}").into()
}

/// Changes the given fields of building `index` of map `map`. One undoable change.
pub fn update_building_impl(
    state: &AppState,
    map: u16,
    index: u16,
    patch: BuildingPatch,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    if usize::from(index) >= session.store.map(map)?.buildings.len() {
        return Err(missing_building(map, index));
    }
    let label = if patch.model_id.is_some() {
        "change building model"
    } else {
        "edit building"
    };
    // Applied to a copy first: a value that does not fit refuses the whole call.
    let mut edited = session.store.map(map)?.buildings[usize::from(index)].clone();
    patch.apply(&mut edited)?;
    session.store.edit_map(map, label, |m| {
        m.buildings[usize::from(index)] = edited;
        Ok(())
    })?;
    Ok(())
}

/// Appends a building (model 0, at the map centre, scale 1.0, rotation 0) and returns its
/// index. One undoable change.
pub fn add_building_impl(state: &AppState, map: u16) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let count = session.store.map(map)?.buildings.len();
    let index = u16::try_from(count)
        .map_err(|_| CmdError::from(format!("map {map} has too many buildings")))?;
    session.store.edit_map(map, "add building", |m| {
        m.buildings.push(Building {
            model_id: 0,
            position: [0; 3],
            rotation: [0; 3],
            scale: [SCALE_ONE; 3],
            dummy: [0; 8],
        });
        Ok(())
    })?;
    Ok(index)
}

/// Removes building `index` of map `map`; the buildings after it move up by one. One undoable change.
pub fn delete_building_impl(state: &AppState, map: u16, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    if usize::from(index) >= session.store.map(map)?.buildings.len() {
        return Err(missing_building(map, index));
    }
    session.store.edit_map(map, "delete building", |m| {
        m.buildings.remove(usize::from(index));
        Ok(())
    })?;
    Ok(())
}

/// Everything the conversion of a building model needs, with the lock already released.
struct BuildingInputs {
    cache_dir: std::path::PathBuf,
    nsbmd: Vec<u8>,
    tileset: Option<Vec<u8>>,
}

/// Collects the model and its tileset. The store (area data with unsaved edits) is read under
/// one short lock; the archives are unpacked and read outside of it. HGSS buildings of an
/// interior area (`area_type` 0) come from the interior archive. Area data or a tileset that
/// cannot be used only costs the textures, never the model.
fn building_inputs(
    state: &AppState,
    model_id: u32,
    area_data_id: u8,
) -> Result<BuildingInputs, CmdError> {
    let (project, area): (Arc<Project>, _) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        let area = session.store.area_data_checked(area_data_id).ok().flatten();
        (Arc::clone(&session.project), area)
    };
    let interior = area.is_some_and(|a| a.area_type() == Some(0));
    let nsbmd = project
        .building_model_bytes(model_id, interior)?
        .ok_or_else(|| CmdError::from(format!("building model {model_id} does not exist")))?;
    let tileset = area
        .and_then(|a| project.building_tileset_member(a.buildings_tileset()).ok())
        .flatten();
    Ok(BuildingInputs {
        cache_dir: project.root.join(".g4").join("models"),
        nsbmd,
        tileset,
    })
}

/// GLB bytes of building model `model_id`, textured with the building tileset of
/// `area_data_id`; same cache, untextured fallback and failure markers as the map models
/// (see `models::glb_with_fallback`). A model outside its archive is an error.
pub fn get_building_glb_with(
    state: &AppState,
    model_id: u32,
    area_data_id: u8,
    apicula: Option<&Path>,
) -> Result<Vec<u8>, CmdError> {
    let inputs = building_inputs(state, model_id, area_data_id)?;
    glb_with_fallback(
        &inputs.cache_dir,
        &inputs.nsbmd,
        inputs.tileset.as_deref(),
        apicula,
        apicula_timeout(),
    )
}

pub fn get_building_glb_impl(
    state: &AppState,
    model_id: u32,
    area_data_id: u8,
) -> Result<Vec<u8>, CmdError> {
    get_building_glb_with(state, model_id, area_data_id, find_apicula().as_deref())
}

/// Raw GLB bytes over IPC. The apicula run happens on a blocking worker like `build_rom`, not
/// on the async pool.
#[tauri::command]
pub async fn get_building_glb(
    app: tauri::AppHandle,
    model_id: u32,
    area_data_id: u8,
) -> Result<tauri::ipc::Response, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_building_glb_impl(&app.state::<AppState>(), model_id, area_data_id)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
    .map(tauri::ipc::Response::new)
}

#[tauri::command]
pub fn update_building(
    state: State<AppState>,
    map: u16,
    index: u16,
    patch: BuildingPatch,
) -> Result<(), CmdError> {
    update_building_impl(&state, map, index, patch)
}

#[tauri::command]
pub fn add_building(state: State<AppState>, map: u16) -> Result<u16, CmdError> {
    add_building_impl(&state, map)
}

#[tauri::command]
pub fn delete_building(state: State<AppState>, map: u16, index: u16) -> Result<(), CmdError> {
    delete_building_impl(&state, map, index)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;
    use g4_kernel::export::{glb_cache_key, glb_fail_get, remember_tool, tool_fingerprint};

    const BMD: &[u8] = b"BMD0\0\0\0\0";
    const BTX: &[u8] = b"BTX0\0\0\0\0";
    /// Stand-in tool hash for seeding the cache by hand (hits without apicula are looked up
    /// under the last remembered hash).
    const TOOL: u64 = 0xabcd;

    #[test]
    fn update_building_converts_tiles_and_scale() {
        let (_s, state) = open_synthetic();
        update_building_impl(
            &state,
            0,
            0,
            BuildingPatch {
                x: Some(-13.5),
                sy: Some(2.0),
                ..Default::default()
            },
        )
        .unwrap();
        let m = get_map_impl(&state, 0).unwrap();
        assert_eq!((m.buildings[0].x, m.buildings[0].sy), (-13.5, 2.0));
        // Fields that were not in the patch keep their values, rotation stays verbatim.
        assert_eq!((m.buildings[0].z, m.buildings[0].sx), (0.0, 1.0));
        assert_eq!(m.buildings[0].model_id, 7);
        assert_eq!(m.buildings[0].rotation[0], "000000AB");
        let idx = add_building_impl(&state, 0).unwrap();
        assert_eq!(idx, 1);
        delete_building_impl(&state, 0, 1).unwrap();
        assert_eq!(get_map_impl(&state, 0).unwrap().buildings.len(), 1);
    }

    #[test]
    fn building_values_are_stored_as_rounded_fx32() {
        let mut b = Building {
            model_id: 0,
            position: [0; 3],
            rotation: [5, 6, 7],
            scale: [0x1000; 3],
            dummy: [9; 8],
        };
        BuildingPatch {
            model_id: Some(12),
            x: Some(0.5),
            y: Some(-2.0),
            z: Some(1.0 / 65536.0 * 1.4),
            sz: Some(0.5),
            ..Default::default()
        }
        .apply(&mut b)
        .unwrap();
        assert_eq!(b.model_id, 12);
        assert_eq!(b.position, [32768, -131072, 1]);
        assert_eq!(b.scale, [0x1000, 0x1000, 0x800]);
        assert_eq!((b.rotation, b.dummy), ([5, 6, 7], [9; 8]));
    }

    #[test]
    fn building_values_that_do_not_fit_are_refused_without_changes() {
        let (_s, state) = open_synthetic();
        for patch in [
            BuildingPatch {
                x: Some(40_000.0),
                ..Default::default()
            },
            BuildingPatch {
                sy: Some(f32::NAN),
                ..Default::default()
            },
            BuildingPatch {
                // Valid on its own, but the same patch has a bad field: nothing is applied.
                x: Some(1.0),
                sz: Some(f32::INFINITY),
                ..Default::default()
            },
        ] {
            assert!(update_building_impl(&state, 0, 0, patch).is_err());
        }
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert_eq!(get_map_impl(&state, 0).unwrap().buildings[0].x, 3.0);
    }

    #[test]
    fn building_commands_reject_unknown_maps_and_indexes() {
        let (_s, state) = open_synthetic();
        let err = update_building_impl(&state, 0, 5, BuildingPatch::default()).unwrap_err();
        assert!(err.message.contains("building 5"), "{}", err.message);
        assert!(delete_building_impl(&state, 0, 1).is_err());
        assert!(update_building_impl(&state, 99, 0, BuildingPatch::default()).is_err());
        assert!(add_building_impl(&state, 99).is_err());
        assert!(delete_building_impl(&state, 99, 0).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let none = AppState::default();
        assert_eq!(
            add_building_impl(&none, 0).unwrap_err().message,
            "no project open"
        );
    }

    #[test]
    fn building_edits_undo_and_redo_one_step_each() {
        let (_s, state) = open_synthetic();
        let idx = add_building_impl(&state, 1).unwrap();
        assert_eq!(idx, 1);
        let added = &get_map_impl(&state, 1).unwrap().buildings[1];
        assert_eq!(
            (added.model_id, added.x, added.y, added.z),
            (0, 0.0, 0.0, 0.0)
        );
        assert_eq!((added.sx, added.sy, added.sz), (1.0, 1.0, 1.0));
        assert_eq!(added.rotation, ["00000000"; 3]);
        update_building_impl(
            &state,
            1,
            1,
            BuildingPatch {
                model_id: Some(3),
                z: Some(4.5),
                ..Default::default()
            },
        )
        .unwrap();
        delete_building_impl(&state, 1, 0).unwrap();
        // The remaining building moved up to index 0.
        let b = &get_map_impl(&state, 1).unwrap().buildings;
        assert_eq!((b.len(), b[0].model_id, b[0].z), (1, 3, 4.5));
        assert_eq!(
            undo_impl(&state).unwrap(),
            [g4_kernel::project::AssetId {
                kind: g4_kernel::project::AssetKind::Map,
                index: 1
            }]
        );
        assert_eq!(get_map_impl(&state, 1).unwrap().buildings.len(), 2);
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(get_map_impl(&state, 1).unwrap().buildings[1].model_id, 0);
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(get_map_impl(&state, 1).unwrap().buildings.len(), 1);
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(!redo_impl(&state).unwrap().is_empty());
        assert_eq!(get_map_impl(&state, 1).unwrap().buildings.len(), 2);
    }

    #[test]
    fn building_glb_for_unknown_model_is_an_error_not_panic() {
        let (_s, state) = open_synthetic();
        let err = get_building_glb_impl(&state, 99_999, 0).unwrap_err();
        assert_eq!(err.code, "error");
        assert_eq!(err.message, "building model 99999 does not exist");
        // Also without apicula, and for ids that do not even fit a member index.
        let err = get_building_glb_with(&state, u32::MAX, 0, None).unwrap_err();
        assert!(err.message.contains("does not exist"), "{}", err.message);
        assert_eq!(
            get_building_glb_impl(&AppState::default(), 0, 0)
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn building_glb_without_apicula_names_the_tool() {
        let (_s, state) = open_synthetic();
        let err = get_building_glb_with(&state, 0, 0, None).unwrap_err();
        assert_eq!(err.code, "tool_missing");
        assert!(err.message.contains("apicula"), "{}", err.message);
    }

    #[test]
    fn building_glb_cache_follows_the_buildings_tileset_including_unsaved_edits() {
        let (fixture, state) = open_synthetic();
        let models = fixture.dir.path().join(".g4/models");
        std::fs::create_dir_all(&models).unwrap();
        let textured = glb_cache_key(TOOL, BMD, Some(BTX));
        let plain = glb_cache_key(TOOL, BMD, None);
        std::fs::write(models.join(format!("{textured}.glb")), b"glTFtextured").unwrap();
        std::fs::write(models.join(format!("{plain}.glb")), b"glTFplain").unwrap();
        remember_tool(&models, TOOL).unwrap();
        // Area data 0 selects the only building tileset; area data 1 and 200 select none.
        let glb = |area| get_building_glb_with(&state, 0, area, None).unwrap();
        assert_eq!(glb(0), b"glTFtextured");
        assert_eq!(glb(1), b"glTFplain");
        assert_eq!(glb(200), b"glTFplain");
        // An unsaved area data edit re-textures the building ...
        let mut a = get_area_data_impl(&state, 1).unwrap();
        if let AreaDataDto::Dppt {
            buildings_tileset, ..
        } = &mut a
        {
            *buildings_tileset = 0;
        }
        set_area_data_impl(&state, 1, a, "buildings tileset").unwrap();
        assert_eq!(glb(1), b"glTFtextured");
        // ... and undo takes it back.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(glb(1), b"glTFplain");
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_building_tileset_falls_back_and_is_not_retried() {
        use std::os::unix::fs::PermissionsExt;
        let (fixture, state) = open_synthetic();
        let tool = fixture.dir.path().join("fake-apicula");
        let calls = fixture.dir.path().join("calls");
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>. A tileset argument is rejected.
        std::fs::write(
            &tool,
            format!(
                "#!/bin/sh\nif [ \"$4\" = \"-o\" ]; then echo plain >> '{c}'; else echo textured >> '{c}'; \
                 echo 'bad tileset' >&2; exit 2; fi\n\
                 while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
                 mkdir -p \"$out\"; printf '{glb}' > \"$out/m.glb\"\n",
                c = calls.display(),
                glb = crate::commands::FAKE_GLB_PRINTF
            ),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let glb = get_building_glb_with(&state, 0, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        let models = fixture.dir.path().join(".g4/models");
        let key = tool_fingerprint(&tool).unwrap();
        let reason = glb_fail_get(&models, key, BMD, Some(BTX)).unwrap();
        assert!(reason.is_some_and(|r| r.contains("bad tileset")));
        // The second load is a cache hit; with the cached model gone the tileset still is not
        // tried again.
        std::fs::remove_file(models.join(format!("{}.glb", glb_cache_key(key, BMD, None))))
            .unwrap();
        get_building_glb_with(&state, 0, 0, Some(&tool)).unwrap();
        let log = std::fs::read_to_string(&calls).unwrap();
        assert_eq!(
            log.lines().collect::<Vec<_>>(),
            ["textured", "plain", "plain"]
        );
    }

    /// Gated: the player's bedroom (Platinum DE header 415) has a building whose model comes
    /// out of apicula with its textures. Works on a copy; needs `G4_APICULA`.
    #[test]
    fn vanilla_building_glb_has_images() {
        let Some(apicula) = std::env::var_os("G4_APICULA") else {
            eprintln!("skipped: needs G4_APICULA");
            return;
        };
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        if info.game_code != "CPUD" {
            eprintln!("skipped: not Platinum DE");
            return;
        }
        let header = list_headers_impl(&state).unwrap().headers[415].clone();
        let matrix = get_matrix_impl(&state, header.matrix_id).unwrap();
        let map = matrix.maps.iter().flatten().next().unwrap();
        let building = &get_map_impl(&state, *map).unwrap().buildings[0];
        let glb = get_building_glb_with(
            &state,
            building.model_id,
            header.area_data_id,
            Some(Path::new(&apicula)),
        )
        .unwrap();
        assert_eq!(&glb[0..4], b"glTF");
        // Every image must be embedded: a bufferView inside the BIN chunk, no external uri.
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        let bin_at = 20 + json_len;
        let bin_len = u32::from_le_bytes(glb[bin_at..bin_at + 4].try_into().unwrap()) as usize;
        assert_eq!(&glb[bin_at + 4..bin_at + 8], b"BIN\0");
        let images = json["images"].as_array().cloned().unwrap_or_default();
        assert!(
            !images.is_empty(),
            "building model {} has no images",
            building.model_id
        );
        for (i, image) in images.iter().enumerate() {
            assert!(image.get("uri").is_none(), "image {i} still has a uri");
            let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
            let start = view["byteOffset"].as_u64().unwrap() as usize;
            let len = view["byteLength"].as_u64().unwrap() as usize;
            assert!(len > 0 && start + len <= bin_len, "image {i} outside BIN");
        }
    }
}
