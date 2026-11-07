//! World maps (phase 6.3a): „Welt anlegen“ and the changes of cells (`world_create`,
//! `world_edit`, one undo step each), the world of a map with its face count and budget
//! (`world_get`), the tileset of a map texture package (`tileset_get`, `tileset_save`), the
//! mesh of a world map as flat arrays for the scene (`get_world_mesh`) and the RGBA of one of
//! its textures (`get_tileset_texture`).
//!
//! Painting is for text projects only: outside one the writers refuse with
//! [`WORLD_NEEDS_TEXT_PROJECT`], while `world_get` answers `world: null` and `get_world_mesh`
//! an empty mesh (the scene shows the map's model then); `tileset_get` derives the tileset in
//! any layout.
//!
//! Phase 6.4a adds `world_layers`: a layer added or removed as one undo step.

use std::collections::HashMap;

use g4_kernel::project::SourceMode;
use g4_kernel::store::Store;
use g4_kernel::world::edit::{
    self, WorldCreateRequest, WorldEditRequest, WorldLayersRequest, WorldReport,
};
use g4_kernel::world::mesh::{MODEL_BUDGET, MeshFace, estimate_model_bytes, mesh, mesh_textures};
use g4_kernel::world::{
    TileRole, Tileset, TilesetSaveReport, WorldGrid, WorldJson, derive_tileset, read_tileset_file,
    save_tileset, tileset_texture,
};
use serde::Serialize;
use tauri::State;

use super::{SpriteDto, lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

#[cfg(doc)]
use g4_kernel::world::WORLD_NEEDS_TEXT_PROJECT;

/// What `world_get` answers: the world of the map (`None`: none, or outside a text project),
/// the faces and textures of its mesh, the estimated model size and its share of
/// [`MODEL_BUDGET`].
#[derive(Debug, Serialize)]
pub struct WorldGetDto {
    pub map: u16,
    pub world: Option<WorldJson>,
    pub faces: usize,
    pub textures: usize,
    pub estimate: usize,
    /// `estimate / 0xF000`.
    pub share: f64,
}

/// The tileset of a package: the file `data/tilesets/NNNN.json` (`stored`) or derived from the
/// package and the vanilla permissions.
#[derive(Debug, Serialize)]
pub struct TilesetDto {
    pub package: u16,
    pub stored: bool,
    pub tileset: Tileset,
}

/// One texture of a mesh: its name in the package, its size in texels, the role of its tile
/// and the tile's palette.
#[derive(Debug, Serialize)]
pub struct MeshTextureDto {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub role: TileRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
}

/// The mesh of a world map as flat arrays: per face 12 corner coordinates (four corners
/// `x, y, z`, world units), 8 UV values (texels), the index of its texture in `textures` and
/// its layer.
#[derive(Debug, Serialize)]
pub struct WorldMeshDto {
    pub map: u16,
    pub tileset: u16,
    pub faces: usize,
    pub textures: Vec<MeshTextureDto>,
    pub corners: Vec<i32>,
    pub uv: Vec<i32>,
    pub texture: Vec<u16>,
    pub layer: Vec<u8>,
}

/// Whether the store's project is a text project of the current schema (worlds exist only
/// there; [`Store::world`] refuses elsewhere).
fn is_text_project(store: &Store) -> bool {
    *store.project().source_mode() == SourceMode::TextProject
}

/// The world of `map` in a text project; `None` elsewhere or without one.
fn world_of(store: &mut Store, map: u16) -> Result<Option<WorldJson>, CmdError> {
    if !is_text_project(store) {
        return Ok(None);
    }
    Ok(store.world(map)?.cloned())
}

/// The faces of `world` with its tileset ([`Store::tileset`]) and the tileset; no faces for a
/// world that does not read into cells (the check names its problems).
fn faces_of(
    store: &mut Store,
    world: &WorldJson,
) -> Result<(Vec<MeshFace>, std::sync::Arc<Tileset>), CmdError> {
    let tileset = store.tileset(world.tileset)?;
    let faces = match WorldGrid::parse(world) {
        Ok(grid) => mesh(&grid, &tileset),
        Err(_) => Vec::new(),
    };
    Ok((faces, tileset))
}

pub fn world_create_impl(
    state: &AppState,
    req: WorldCreateRequest,
) -> Result<WorldReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(edit::world_create(&mut session.store, &req)?)
}

pub fn world_edit_impl(state: &AppState, req: WorldEditRequest) -> Result<WorldReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(edit::world_edit(&mut session.store, &req)?)
}

/// „Ebene hinzufügen“ and „Ebene entfernen“ (6.4a): one undo step each.
pub fn world_layers_impl(
    state: &AppState,
    req: WorldLayersRequest,
) -> Result<WorldReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(edit::world_layers(&mut session.store, &req)?)
}

pub fn world_get_impl(state: &AppState, map: u16) -> Result<WorldGetDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let store = &mut session.store;
    let Some(world) = world_of(store, map)? else {
        return Ok(WorldGetDto {
            map,
            world: None,
            faces: 0,
            textures: 0,
            estimate: 0,
            share: 0.0,
        });
    };
    let (faces, _) = faces_of(store, &world)?;
    let textures = mesh_textures(&faces).len();
    let estimate = estimate_model_bytes(faces.len(), textures);
    Ok(WorldGetDto {
        map,
        world: Some(world),
        faces: faces.len(),
        textures,
        estimate,
        share: estimate as f64 / MODEL_BUDGET as f64,
    })
}

pub fn tileset_get_impl(state: &AppState, package: u16) -> Result<TilesetDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let store = &mut session.store;
    let (stored, tileset) = match read_tileset_file(store.project(), package)? {
        Some(tileset) => (true, tileset),
        None => (false, derive_tileset(store, package)?),
    };
    Ok(TilesetDto {
        package,
        stored,
        tileset,
    })
}

pub fn tileset_save_impl(state: &AppState, package: u16) -> Result<TilesetSaveReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(save_tileset(&mut session.store, package)?)
}

/// The mesh of the world of `map`; without one (or outside a text project) no faces and empty
/// arrays.
pub fn get_world_mesh_impl(state: &AppState, map: u16) -> Result<WorldMeshDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let store = &mut session.store;
    let Some(world) = world_of(store, map)? else {
        return Ok(WorldMeshDto {
            map,
            tileset: 0,
            faces: 0,
            textures: Vec::new(),
            corners: Vec::new(),
            uv: Vec::new(),
            texture: Vec::new(),
            layer: Vec::new(),
        });
    };
    let (faces, tileset) = faces_of(store, &world)?;
    let names = mesh_textures(&faces);
    let index: HashMap<&str, u16> = names
        .iter()
        .enumerate()
        // A mesh never has more textures than a tileset has tiles (u16).
        .map(|(i, name)| (name.as_str(), i as u16))
        .collect();
    let textures = names
        .iter()
        .map(|name| {
            let tile = tileset.tiles.iter().find(|t| &t.texture == name);
            MeshTextureDto {
                name: name.clone(),
                width: tile.map_or(0, |t| t.size[0]),
                height: tile.map_or(0, |t| t.size[1]),
                role: tile.map_or(TileRole::Ground, |t| t.role),
                palette: tile.and_then(|t| t.palette.clone()),
            }
        })
        .collect();
    let mut corners = Vec::with_capacity(faces.len() * 12);
    let mut uv = Vec::with_capacity(faces.len() * 8);
    let mut texture = Vec::with_capacity(faces.len());
    let mut layer = Vec::with_capacity(faces.len());
    for face in &faces {
        corners.extend(face.corners.iter().flatten());
        uv.extend(face.uv.iter().flatten());
        texture.push(index.get(face.texture.as_str()).copied().unwrap_or(0));
        layer.push(face.layer);
    }
    Ok(WorldMeshDto {
        map,
        tileset: world.tileset,
        faces: faces.len(),
        textures,
        corners,
        uv,
        texture,
        layer,
    })
}

/// The RGBA of `texture` in the tileset of `package` with its tile's palette; `None` when the
/// package has none to give (the synthetic tileset, a format without decoder).
pub fn get_tileset_texture_impl(
    state: &AppState,
    package: u16,
    texture: String,
) -> Result<Option<SpriteDto>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let store = &mut session.store;
    let tileset = store.tileset(package)?;
    let rgba = tileset_texture(store.project(), &tileset, &texture)?;
    Ok(rgba.map(|s| SpriteDto {
        width: s.width,
        height: s.height,
        rgba: s.pixels,
    }))
}

#[tauri::command]
pub fn world_create(
    state: State<'_, AppState>,
    req: WorldCreateRequest,
) -> Result<WorldReport, CmdError> {
    world_create_impl(&state, req)
}

#[tauri::command]
pub fn world_edit(
    state: State<'_, AppState>,
    req: WorldEditRequest,
) -> Result<WorldReport, CmdError> {
    world_edit_impl(&state, req)
}

#[tauri::command]
pub fn world_get(state: State<'_, AppState>, map: u16) -> Result<WorldGetDto, CmdError> {
    world_get_impl(&state, map)
}

#[tauri::command]
pub fn tileset_get(state: State<'_, AppState>, package: u16) -> Result<TilesetDto, CmdError> {
    tileset_get_impl(&state, package)
}

#[tauri::command]
pub fn tileset_save(
    state: State<'_, AppState>,
    package: u16,
) -> Result<TilesetSaveReport, CmdError> {
    tileset_save_impl(&state, package)
}

#[tauri::command]
pub fn get_world_mesh(state: State<'_, AppState>, map: u16) -> Result<WorldMeshDto, CmdError> {
    get_world_mesh_impl(&state, map)
}

#[tauri::command]
pub fn get_tileset_texture(
    state: State<'_, AppState>,
    package: u16,
    texture: String,
) -> Result<Option<SpriteDto>, CmdError> {
    get_tileset_texture_impl(&state, package, texture)
}

#[tauri::command]
pub fn world_layers(
    state: State<'_, AppState>,
    req: WorldLayersRequest,
) -> Result<WorldReport, CmdError> {
    world_layers_impl(&state, req)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Instant;

    use g4_kernel::test_support::TextFixture;
    use g4_kernel::world::doc::fixtures::busy_world;
    use g4_kernel::world::{WORLD_NEEDS_TEXT_PROJECT, tileset_rel};
    use serde_json::{Value, json};

    use super::*;
    use crate::commands::dispatch::{DispatchError, Reply, dispatch};
    use crate::commands::open_project_impl;
    use crate::commands::policy::UserPolicy;

    /// `fx` (the synthetic tileset written as the file of package 0, whose member is an
    /// unreadable stub) open in a fresh state, with its policy.
    fn open(fx: &TextFixture) -> (AppState, UserPolicy) {
        let rel = tileset_rel(0);
        fx.write(
            rel.to_str().unwrap(),
            &serde_json::to_vec_pretty(&Tileset::synthetic()).unwrap(),
        );
        let state = AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        (state, policy)
    }

    fn call(state: &AppState, policy: &UserPolicy, name: &str, args: Value) -> Value {
        match dispatch(state, policy, name, args) {
            Ok(Reply::Json(v)) => v,
            Ok(Reply::Bytes(_)) => panic!("{name}: bytes"),
            Err(e) => panic!("{name}: {e:?}"),
        }
    }

    fn refused(state: &AppState, policy: &UserPolicy, name: &str, args: Value) -> String {
        match dispatch(state, policy, name, args) {
            Err(DispatchError::Cmd(e)) => e.message,
            other => panic!("{name}: {:?}", other.map(|_| ())),
        }
    }

    /// The 37 cells of the kernel's test: row 3 and five cells of row 4.
    fn cells_37() -> Vec<[u8; 2]> {
        (0..32)
            .map(|x| [x, 3])
            .chain((0..5).map(|x| [x, 4]))
            .collect()
    }

    #[test]
    fn world_commands_over_dispatch() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        let none = call(&state, &policy, "world_get", json!({ "map": 0 }));
        assert_eq!((&none["world"], &none["faces"]), (&Value::Null, &json!(0)));
        assert_eq!(
            call(&state, &policy, "get_map", json!({ "index": 0 }))["world"],
            false
        );

        let made = call(
            &state,
            &policy,
            "world_create",
            json!({ "req": { "map": 0, "tileset": 0 } }),
        );
        assert_eq!((&made["map"], &made["cells"]), (&json!(0), &json!(1024)));
        let created = call(&state, &policy, "world_get", json!({ "map": 0 }));
        assert_eq!(created["world"]["tileset"], 0, "{created}");
        // One floor of one tile.
        assert_eq!(
            (&created["faces"], &created["textures"]),
            (&json!(1), &json!(1))
        );

        let edited = call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": [{ "cells": cells_37(), "tile": "0001" }] } }),
        );
        assert_eq!(edited["cells"], 37, "{edited}");
        let got = call(&state, &policy, "world_get", json!({ "map": 0 }));
        let faces = got["faces"].as_u64().unwrap() as usize;
        assert!(faces > 1, "{got}");
        assert_eq!(got["textures"], 2);
        let estimate = 1024 + 600 * 2 + 56 * faces;
        assert_eq!(got["estimate"], estimate);
        assert_eq!(got["share"], estimate as f64 / 61440.0);

        let mesh = call(&state, &policy, "get_world_mesh", json!({ "map": 0 }));
        assert_eq!((&mesh["map"], &mesh["tileset"]), (&json!(0), &json!(0)));
        assert_eq!(mesh["faces"], faces);
        assert_eq!(mesh["corners"].as_array().unwrap().len(), 12 * faces);
        assert_eq!(mesh["uv"].as_array().unwrap().len(), 8 * faces);
        assert_eq!(mesh["texture"].as_array().unwrap().len(), faces);
        assert_eq!(mesh["layer"].as_array().unwrap().len(), faces);
        let textures = mesh["textures"].as_array().unwrap();
        let names: Vec<&str> = textures
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["syn_boden", "syn_gras"]);
        assert_eq!(
            textures[1],
            json!({ "name": "syn_gras", "width": 32, "height": 32, "role": "grass" })
        );
        assert!(
            mesh["texture"]
                .as_array()
                .unwrap()
                .iter()
                .all(|t| t.as_u64().unwrap() < 2)
        );

        call(&state, &policy, "undo", json!({}));
        assert_eq!(
            call(&state, &policy, "world_get", json!({ "map": 0 })),
            created
        );
        assert_eq!(
            call(&state, &policy, "get_map", json!({ "index": 0 }))["world"],
            true
        );
    }

    #[test]
    fn a_world_map_has_the_heights_of_its_world() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        let before = call(&state, &policy, "get_map", json!({ "index": 0 }))["heights"].clone();
        assert_eq!(before.as_array().unwrap().len(), 1024);
        call(
            &state,
            &policy,
            "world_create",
            json!({ "req": { "map": 0, "tileset": 0, "height": 2 } }),
        );
        call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": [{ "cells": [[3, 4]], "ramp": "e1" }] } }),
        );
        let map = call(&state, &policy, "get_map", json!({ "index": 0 }));
        let heights = map["heights"].as_array().unwrap();
        assert_eq!(heights.len(), 1024);
        // Level cells at step 2 (16 units); the ramp half way up its step at the centre.
        assert_eq!(heights[0], json!(16.0));
        assert_eq!(heights[4 * 32 + 3], json!(20.0));
        assert_eq!(heights.iter().filter(|h| **h == json!(16.0)).count(), 1023);
        // An empty ground cell has no floor: no height.
        call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": [{ "cells": [[5, 6]], "tile": "." }] } }),
        );
        let heights = call(&state, &policy, "get_map", json!({ "index": 0 }))["heights"].clone();
        let heights = heights.as_array().unwrap();
        assert_eq!(heights[6 * 32 + 5], Value::Null);
        assert_eq!(heights[0], json!(16.0));
        assert_eq!(heights.iter().filter(|h| h.is_null()).count(), 1);
        // Undoing the three steps gives the member's heights back.
        call(&state, &policy, "undo", json!({}));
        call(&state, &policy, "undo", json!({}));
        call(&state, &policy, "undo", json!({}));
        assert_eq!(
            call(&state, &policy, "get_map", json!({ "index": 0 }))["heights"],
            before
        );
    }

    #[test]
    fn get_tileset_texture_is_null_for_the_synthetic_tileset() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        let got = call(
            &state,
            &policy,
            "get_tileset_texture",
            json!({ "package": 0, "texture": "syn_boden" }),
        );
        assert_eq!(got, Value::Null);
        // The file answers `tileset_get` as it is.
        let tileset = call(&state, &policy, "tileset_get", json!({ "package": 0 }));
        assert_eq!(tileset["stored"], true);
        assert_eq!(tileset["package"], 0);
        assert_eq!(
            tileset["tileset"],
            serde_json::to_value(Tileset::synthetic()).unwrap()
        );
        // It is there already.
        assert_eq!(
            refused(&state, &policy, "tileset_save", json!({ "package": 0 })),
            "data/tilesets/0000.json gibt es schon."
        );
    }

    #[test]
    fn world_commands_in_the_dspre_layout() {
        let fx = TextFixture::dspre();
        let (state, policy) = open(&fx);
        for (name, args) in [
            ("world_create", json!({ "req": { "map": 0, "tileset": 0 } })),
            (
                "world_edit",
                json!({ "req": { "map": 0, "changes": [{ "cells": [[0, 0]], "tile": "0001" }] } }),
            ),
            ("tileset_save", json!({ "package": 0 })),
        ] {
            assert_eq!(
                refused(&state, &policy, name, args),
                WORLD_NEEDS_TEXT_PROJECT,
                "{name}"
            );
        }
        assert_eq!(
            call(&state, &policy, "get_map", json!({ "index": 0 }))["world"],
            false
        );
        let got = call(&state, &policy, "world_get", json!({ "map": 0 }));
        assert_eq!((&got["world"], &got["faces"]), (&Value::Null, &json!(0)));
        let mesh = call(&state, &policy, "get_world_mesh", json!({ "map": 0 }));
        assert_eq!(mesh["faces"], 0);
        assert_eq!(mesh["corners"], json!([]));
        assert!(
            crate::commands::history_impl(&state)
                .unwrap()
                .undo
                .is_empty()
        );
    }

    /// The ground of [`busy_world`] (checkerboard, heights, ramps) set over `world_edit`; then
    /// `get_world_mesh` over `dispatch` with its JSON serialization, warmed once, median of 10.
    /// Prints `world-mesh-bridge median_ms=<…> faces=<…>`; with `G4_TIMING_STRICT=1` at most
    /// 50 ms (a debug build).
    #[test]
    fn world_mesh_over_the_command_path_is_fast() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        call(
            &state,
            &policy,
            "world_create",
            json!({ "req": { "map": 0, "tileset": 0 } }),
        );
        let busy = busy_world();
        let ground = &busy.layers[0];
        let mut changes = Vec::new();
        for z in 0..32u8 {
            let row = |rows: &[String]| -> Vec<String> {
                rows[usize::from(z)]
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect()
            };
            let tiles = row(&ground.tiles);
            let heights = row(ground.heights.as_ref().unwrap());
            let ramps = row(ground.ramps.as_ref().unwrap());
            for x in 0..32u8 {
                let at = usize::from(x);
                if tiles[at] != "0000" {
                    changes.push(json!({ "cells": [[x, z]], "tile": tiles[at] }));
                }
                if heights[at] != "0" {
                    changes.push(
                        json!({ "cells": [[x, z]], "height": heights[at].parse::<i8>().unwrap() }),
                    );
                }
                if ramps[at] != "." {
                    changes.push(json!({ "cells": [[x, z]], "ramp": ramps[at] }));
                }
            }
        }
        call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": changes } }),
        );
        let run = || {
            let start = Instant::now();
            let reply = call(&state, &policy, "get_world_mesh", json!({ "map": 0 }));
            let text = serde_json::to_string(&reply).unwrap();
            (start.elapsed().as_secs_f64() * 1000.0, reply, text.len())
        };
        let (_, warm, _) = run();
        let faces = warm["faces"].as_u64().unwrap();
        assert!(faces > 500, "{faces}");
        let mut times: Vec<f64> = (0..10).map(|_| run().0).collect();
        times.sort_by(f64::total_cmp);
        let median = (times[4] + times[5]) / 2.0;
        println!("world-mesh-bridge median_ms={median:.2} faces={faces}");
        if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
            assert!(median <= 50.0, "median {median:.2} ms");
        }
    }

    /// „Ebene hinzufügen“ over `dispatch`: a second layer, a tile on it, both undone again.
    #[test]
    fn world_layers_over_dispatch() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        call(
            &state,
            &policy,
            "world_create",
            json!({ "req": { "map": 0, "tileset": 0 } }),
        );
        let added = call(
            &state,
            &policy,
            "world_layers",
            json!({ "req": { "map": 0, "op": { "add": { "name": "Brücke" } } } }),
        );
        assert_eq!(added["map"], 0, "{added}");
        call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": [{ "layer": 1, "cells": [[4, 5]], "tile": "0001" }] } }),
        );
        let got = call(&state, &policy, "world_get", json!({ "map": 0 }));
        let layers = got["world"]["layers"].as_array().unwrap();
        assert_eq!(layers.len(), 2, "{got}");
        assert_eq!(layers[1]["name"], "Brücke");
        call(&state, &policy, "undo", json!({}));
        call(&state, &policy, "undo", json!({}));
        let back = call(&state, &policy, "world_get", json!({ "map": 0 }));
        assert_eq!(
            back["world"]["layers"].as_array().unwrap().len(),
            1,
            "{back}"
        );
        // The ground stays.
        assert_eq!(
            refused(
                &state,
                &policy,
                "world_layers",
                json!({ "req": { "map": 0, "op": { "remove": { "layer": 0 } } } }),
            ),
            "Die Bodenebene (0) bleibt."
        );
    }

    /// The changes that set the ground of [`busy_world`] (checkerboard, heights, ramps).
    fn busy_ground_changes() -> Vec<Value> {
        let busy = busy_world();
        let ground = &busy.layers[0];
        let mut changes = Vec::new();
        for z in 0..32u8 {
            let row = |rows: &[String]| -> Vec<String> {
                rows[usize::from(z)]
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect()
            };
            let tiles = row(&ground.tiles);
            let heights = row(ground.heights.as_ref().unwrap());
            let ramps = row(ground.ramps.as_ref().unwrap());
            for x in 0..32u8 {
                let at = usize::from(x);
                if tiles[at] != "0000" {
                    changes.push(json!({ "cells": [[x, z]], "tile": tiles[at] }));
                }
                if heights[at] != "0" {
                    changes.push(
                        json!({ "cells": [[x, z]], "height": heights[at].parse::<i8>().unwrap() }),
                    );
                }
                if ramps[at] != "." {
                    changes.push(json!({ "cells": [[x, z]], "ramp": ramps[at] }));
                }
            }
        }
        changes
    }

    /// One stroke of the app over `dispatch` with JSON: `world_edit` of 37 cells (two tiles
    /// in turn) on the ground of [`busy_world`], then `world_get` and `get_world_mesh`; warmed
    /// once, median of 10. Prints `world-stroke-bridge median_ms=<…> faces=<…>`; with
    /// `G4_TIMING_STRICT=1` at most 300 ms (a debug build).
    #[test]
    fn world_stroke_over_the_command_path_is_fast() {
        let fx = TextFixture::exported();
        let (state, policy) = open(&fx);
        call(
            &state,
            &policy,
            "world_create",
            json!({ "req": { "map": 0, "tileset": 0 } }),
        );
        call(
            &state,
            &policy,
            "world_edit",
            json!({ "req": { "map": 0, "changes": busy_ground_changes() } }),
        );
        let stroke = |tile: &str| {
            let start = Instant::now();
            let edited = call(
                &state,
                &policy,
                "world_edit",
                json!({ "req": { "map": 0, "changes": [{ "cells": cells_37(), "tile": tile }] } }),
            );
            let got = call(&state, &policy, "world_get", json!({ "map": 0 }));
            let mesh = call(&state, &policy, "get_world_mesh", json!({ "map": 0 }));
            let bytes = [&edited, &got, &mesh]
                .iter()
                .map(|v| serde_json::to_string(v).unwrap().len())
                .sum::<usize>();
            assert!(bytes > 0);
            (start.elapsed().as_secs_f64() * 1000.0, mesh)
        };
        let (_, warm) = stroke("0001");
        let faces = warm["faces"].as_u64().unwrap();
        assert!(faces > 500, "{faces}");
        let mut times: Vec<f64> = (0..10)
            .map(|i| stroke(if i % 2 == 0 { "0000" } else { "0001" }).0)
            .collect();
        times.sort_by(f64::total_cmp);
        let median = (times[4] + times[5]) / 2.0;
        println!("world-stroke-bridge median_ms={median:.2} faces={faces}");
        if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
            assert!(median <= 300.0, "median {median:.2} ms");
        }
    }
}
