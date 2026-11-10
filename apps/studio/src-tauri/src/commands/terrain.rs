//! The terrain commands (6.2): the walk heights of a map, its terrain from a height grid, and
//! its budgets against the game's limits.

use g4_formats::bdhc::FX_ONE;
use g4_kernel::terrain::{
    BdhcSource, CellOverride, GridRows, MapBudget, PermissionSource, TerrainInput, TerrainReport,
    map_budget, map_heights,
};
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The walk height of every tile centre of map `index` in world units (16 per tile, index
/// `y * 32 + x`), the highest candidate; `None` where no plate lies under the centre. An
/// unreadable BDHC gives 1 024 × `None` and the reader's reason in `error`.
#[derive(Debug, Serialize)]
pub struct MapHeightsDto {
    pub index: u16,
    pub heights: Vec<Option<f32>>,
    pub error: Option<String>,
}

/// The tile heights of `bdhc` in world units, and the reader's reason when it is unreadable
/// (then every height is `None`). The same numbers for `get_map_heights` and `MapDto.heights`.
pub(crate) fn tile_heights(bdhc: &[u8]) -> (Vec<Option<f32>>, Option<String>) {
    match map_heights(bdhc) {
        Ok(heights) => (
            heights
                .iter()
                .map(|h| h.map(|fx| fx as f32 / FX_ONE as f32))
                .collect(),
            None,
        ),
        Err(e) => (vec![None; 1024], Some(e.to_string())),
    }
}

pub fn get_map_heights_impl(state: &AppState, index: u16) -> Result<MapHeightsDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let map = session.store.map(index)?;
    let (heights, error) = tile_heights(&map.bdhc);
    Ok(MapHeightsDto {
        index,
        heights,
        error,
    })
}

pub fn set_map_terrain_impl(
    state: &AppState,
    index: u16,
    input: TerrainInput,
) -> Result<TerrainReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(g4_kernel::terrain::set_map_terrain(
        &mut session.store,
        index,
        &input,
    )?)
}

pub fn get_map_budget_impl(state: &AppState, index: u16) -> Result<MapBudget, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_budget(&mut session.store, index)?)
}

/// The arguments of `set_map_terrain` after `index`, as the command takes them.
pub(crate) fn terrain_input(
    grid: GridRows,
    kinds: Vec<String>,
    overrides: Option<Vec<CellOverride>>,
    bdhc: Option<BdhcSource>,
    permissions: Option<PermissionSource>,
) -> TerrainInput {
    TerrainInput {
        grid,
        kinds,
        overrides: overrides.unwrap_or_default(),
        bdhc: bdhc.unwrap_or_default(),
        permissions: permissions.unwrap_or_default(),
    }
}

#[tauri::command]
pub fn get_map_heights(state: State<AppState>, index: u16) -> Result<MapHeightsDto, CmdError> {
    get_map_heights_impl(&state, index)
}

#[tauri::command]
pub fn set_map_terrain(
    state: State<AppState>,
    index: u16,
    grid: GridRows,
    kinds: Vec<String>,
    overrides: Option<Vec<CellOverride>>,
    bdhc: Option<BdhcSource>,
    permissions: Option<PermissionSource>,
) -> Result<TerrainReport, CmdError> {
    let input = terrain_input(grid, kinds, overrides, bdhc, permissions);
    set_map_terrain_impl(&state, index, input)
}

#[tauri::command]
pub fn get_map_budget(state: State<AppState>, index: u16) -> Result<MapBudget, CmdError> {
    get_map_budget_impl(&state, index)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use g4_formats::bdhc::HeightGrid;
    use g4_kernel::terrain::{BudgetState, parse_grid, scenario_input};
    use serde_json::{Value, json};

    use super::super::open_synthetic;
    use super::*;
    use crate::commands::dispatch::{Reply, dispatch};
    use crate::commands::policy::UserPolicy;
    use crate::commands::*;

    fn call(state: &AppState, root: &std::path::Path, name: &str, args: Value) -> Value {
        let policy = UserPolicy::new(root, &[], None).unwrap();
        match dispatch(state, &policy, name, args).unwrap_or_else(|e| panic!("{name}: {e:?}")) {
            Reply::Json(v) => v,
            Reply::Bytes(_) => panic!("{name}: bytes"),
        }
    }

    /// The JSON of `set_map_terrain` for map `index` with the scenario terrain.
    fn scenario_args(index: u16) -> Value {
        let input = scenario_input();
        json!({
            "index": index,
            "grid": { "heights": input.grid.heights, "ramps": input.grid.ramps },
            "kinds": input.kinds,
            "overrides": [{ "x": 18, "y": 25, "kind": 0, "collision": 0 }],
        })
    }

    fn centres(grid: &HeightGrid) -> Vec<f64> {
        (0..1024).map(|i| grid.center_units(i)).collect()
    }

    #[test]
    fn heights_of_an_unreadable_bdhc_are_null_with_the_reason() {
        let (_fixture, state) = open_synthetic();
        {
            let mut guard = lock(&state).unwrap();
            let session = session(&mut guard).unwrap();
            session
                .store
                .edit_map(0, "test", |m| {
                    m.bdhc = b"NOPE".to_vec();
                    Ok(())
                })
                .unwrap();
        }
        let dto = get_map_heights_impl(&state, 0).unwrap();
        assert_eq!(dto.index, 0);
        assert_eq!(dto.heights.len(), 1024);
        assert!(dto.heights.iter().all(Option::is_none));
        let reason = g4_kernel::terrain::map_heights(b"NOPE")
            .unwrap_err()
            .to_string();
        assert_eq!(dto.error.as_deref(), Some(reason.as_str()));
        let map = get_map_impl(&state, 0).unwrap();
        assert_eq!(map.heights.len(), 1024);
        assert!(map.heights.iter().all(Option::is_none));
        let json = serde_json::to_value(&map).unwrap();
        assert_eq!(json["heights"][0], Value::Null);
    }

    #[test]
    fn terrain_commands_answer_in_the_documented_shape() {
        let (fixture, state) = open_synthetic();
        let root = fixture.dir.path();
        let report = call(&state, root, "set_map_terrain", scenario_args(0));
        for key in [
            "map",
            "bdhc_bytes",
            "payload",
            "plates",
            "strips",
            "blocked",
            "overrides",
            "permissions_kept",
        ] {
            assert!(report.get(key).is_some(), "{key}: {report}");
        }
        assert_eq!(report["overrides"], 1);
        assert_eq!(report["permissions_kept"], false);

        let heights = call(&state, root, "get_map_heights", json!({ "index": 0 }));
        assert_eq!(heights["index"], 0);
        assert_eq!(heights["error"], Value::Null);
        let grid = parse_grid(&scenario_input().grid).unwrap();
        let got: Vec<f64> = heights["heights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_f64().unwrap())
            .collect();
        for (i, (h, want)) in got.iter().zip(centres(&grid)).enumerate() {
            assert!((h - want).abs() <= 0.1, "tile {i}: {h} vs {want}");
        }
        // get_map carries the same numbers.
        let map = call(&state, root, "get_map", json!({ "index": 0 }));
        assert_eq!(map["heights"], heights["heights"]);
        assert_eq!(map["permissions"][25 * 32 + 18], json!([0, 0]));

        let budget = call(&state, root, "get_map_budget", json!({ "index": 0 }));
        assert_eq!(budget["map"], 0);
        assert_eq!(budget["bdhc"]["used"], report["payload"]);
        assert_eq!(budget["bdhc"]["max"], 36864);
        assert_eq!(budget["bdhc"]["state"], "calm");
        assert_eq!(budget["model"]["max"], 61440);
        assert_eq!(budget["model"]["state"], "calm");
        assert_eq!(budget["buildings"]["max"], 32);
        assert_eq!(budget["buildings"]["state"], "calm");
        // The synthetic packs are no BTX0 with TEX0.
        assert_eq!(budget["textures"], Value::Null);
        assert!(
            budget["textures_missing"]
                .as_str()
                .unwrap()
                .starts_with("Texture pack 0 unreadable: "),
            "{budget}"
        );

        // A refused row over the command path: the German sentence, nothing written.
        let mut short = scenario_args(0);
        short["kinds"].as_array_mut().unwrap().pop();
        let policy = UserPolicy::new(root, &[], None).unwrap();
        let before = get_map_impl(&state, 0).unwrap().permissions;
        match dispatch(&state, &policy, "set_map_terrain", short) {
            Err(crate::commands::dispatch::DispatchError::Cmd(e)) => {
                assert_eq!(e.message, "kinds: 32 rows expected, 31 found.")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(get_map_impl(&state, 0).unwrap().permissions, before);
        // `flat` and `keep` over the command path.
        let mut flat = scenario_args(0);
        flat["bdhc"] = json!("flat");
        flat["permissions"] = json!("keep");
        let report = call(&state, root, "set_map_terrain", flat);
        assert_eq!(report["payload"], 50);
        assert_eq!(report["permissions_kept"], true);
        let heights = get_map_heights_impl(&state, 0).unwrap();
        assert!(heights.heights.iter().all(|h| *h == Some(0.0)));
        assert_eq!(get_map_impl(&state, 0).unwrap().permissions, before);
    }

    /// The most common value of `heights` (`None` counts as a value).
    fn most_common(heights: &[Option<f32>]) -> Option<f32> {
        let mut counts: HashMap<Option<u32>, usize> = HashMap::new();
        for h in heights {
            *counts.entry(h.map(f32::to_bits)).or_default() += 1;
        }
        let (bits, _) = counts
            .into_iter()
            .max_by_key(|&(bits, n)| (n, std::cmp::Reverse(bits)))
            .unwrap();
        bits.map(f32::from_bits)
    }

    /// Gated (`G4_TEST_PROJECT`): the three commands on a copy of a vanilla project. Prints
    /// `terrain <game_code> …` with the budgets and VRAM estimates of a few maps.
    #[test]
    fn terrain_over_the_command_path_on_the_vanilla_copy() {
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let code = info.game_code.clone();
        let started = std::time::Instant::now();
        let mut samples = Vec::new();
        match code.as_str() {
            "CPUD" => {
                let heights = get_map_heights_impl(&state, 100).unwrap();
                assert_eq!(heights.error, None);
                assert_eq!(most_common(&heights.heights), Some(32.0), "map 100");
                let budget = get_map_budget_impl(&state, 188).unwrap();
                assert_eq!(budget.bdhc.used, 21452, "map 188");
                assert_eq!(budget.bdhc.state, BudgetState::Calm);
                samples.extend([0u16, 100, 188, 3, 411]);
            }
            "IPKD" | "IPKE" => {
                let budget = get_map_budget_impl(&state, 658).unwrap();
                assert_eq!(budget.bdhc.used, 10778, "map 658");
                assert_eq!(budget.bdhc.state, BudgetState::Calm);
                samples.extend([0u16, 1, 658, 100, 200]);
            }
            other => eprintln!("terrain {other}: no expected values, the round trip only"),
        }
        let budget_ms = started.elapsed().as_millis();
        for map in samples {
            let b = get_map_budget_impl(&state, map).unwrap();
            match &b.textures {
                Some(t) => eprintln!(
                    "terrain {code} map {map}: model {} B, bdhc {} B, buildings {}, area {} \
                     (packs {} / {}, {} textures): VRAM {} B ({:?}, vanilla max {})",
                    b.model.used,
                    b.bdhc.used,
                    b.buildings.used,
                    t.area,
                    t.map_pack,
                    t.building_pack,
                    t.textures,
                    t.vram.used,
                    t.vram.state,
                    t.vanilla_max,
                ),
                None => eprintln!(
                    "terrain {code} map {map}: model {} B, bdhc {} B, buildings {}, textures: {}",
                    b.model.used,
                    b.bdhc.used,
                    b.buildings.used,
                    b.textures_missing.as_deref().unwrap_or("?"),
                ),
            }
        }

        // The VRAM estimate of every area against Gutachten A §2.4 (median 37 744 / 27 144 B,
        // maximum 110 640 / 87 424 B per area, measured): printed, not asserted.
        let mut vram: Vec<(u64, u8)> = Vec::new();
        {
            let mut guard = lock(&state).unwrap();
            let s = session(&mut guard).unwrap();
            let areas = s
                .store
                .member_count(g4_kernel::project::AssetKind::AreaData)
                .unwrap();
            let read = |bytes: Option<Vec<u8>>| {
                bytes
                    .and_then(|b| g4_formats::nsbtx::Nsbtx::read(&b).ok())
                    .map(|btx| g4_kernel::terrain::vram_bytes(&btx))
            };
            for area in 0..areas.min(256) as u8 {
                let Some(data) = s.store.area_data_checked(area).unwrap() else {
                    continue;
                };
                let project = s.store.project();
                let map = read(project.map_tileset_member(data.map_tileset()).unwrap());
                let building = read(
                    project
                        .building_tileset_member(data.buildings_tileset())
                        .unwrap(),
                );
                if let (Some(m), Some(b)) = (map, building) {
                    vram.push((m + b, area));
                }
            }
        }
        vram.sort();
        if let (Some(&(max, max_area)), Some(&(median, _))) =
            (vram.last(), vram.get(vram.len() / 2))
        {
            let vanilla_max = g4_kernel::checks::limits::Limits::get()
                .vram
                .vanilla_max
                .of(open_project_arc(&state).unwrap().family) as u64;
            let tight = vram.iter().filter(|(v, _)| *v > vanilla_max).count();
            eprintln!(
                "terrain {code} VRAM estimate over {} areas: median {median} B, max {max} B \
                 (area {max_area}), {tight} above the vanilla maximum {vanilla_max}",
                vram.len()
            );
        }

        // set_map_terrain and undo restore the bytes.
        let before = {
            let mut guard = lock(&state).unwrap();
            let s = session(&mut guard).unwrap();
            let m = s.store.map(0).unwrap();
            (m.bdhc.clone(), m.permissions.clone())
        };
        let input = scenario_input();
        let grid = parse_grid(&input.grid).unwrap();
        let started = std::time::Instant::now();
        let report = set_map_terrain_impl(&state, 0, input).unwrap();
        let set_ms = started.elapsed().as_millis();
        let heights = get_map_heights_impl(&state, 0).unwrap();
        for (i, (h, want)) in heights.heights.iter().zip(centres(&grid)).enumerate() {
            let h = f64::from(h.unwrap_or_else(|| panic!("tile {i}: no height")));
            assert!((h - want).abs() <= 0.1, "tile {i}: {h} vs {want}");
        }
        undo_impl(&state).unwrap();
        let after = {
            let mut guard = lock(&state).unwrap();
            let s = session(&mut guard).unwrap();
            let m = s.store.map(0).unwrap();
            (m.bdhc.clone(), m.permissions.clone())
        };
        assert_eq!(after, before, "undo restores BDHC and permissions");
        eprintln!(
            "terrain {code}: budgets in {budget_ms} ms, set_map_terrain {set_ms} ms, {report:?}"
        );
    }
}
