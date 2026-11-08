#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, a DSPRE folder with its `.nds` beside it; only a copy is written,
//! see `vanilla_copy`): smart drawing of 6.4b on the vanilla projects Platin DE (CPUD),
//! HeartGold DE (IPKD) and HeartGold US (IPKE), each set up as text project like
//! `tests/world_paint.rs`.
//!
//! The map: „Neue Karte malen“ (`new_painted_map`, name „Kantenweg“, ground tile `0000`) in a
//! cell beside the start town as in `tests/world_paint.rs`; its package gets its tileset file
//! (`tileset_save`) and, in one `tileset_edit`, tiles `0000` to `000d` as plain ground
//! (`role: ground`, `kind: 0`, `collision: 0`, so no derived water or grass tile changes the
//! counts) and the edge set „Gate“: outside `0000`, the 13 places of `Place::GATE13` in order
//! on `0001` to `000d`. A package with fewer than 14 tiles fails with „Paket NNNN hat nur n
//! Kacheln; der Satz braucht 14.“.
//!
//! - The terrain brush over `plus_cells()`: layer 0 of the world is the tile raster of
//!   `PLUS_EXPECTED` over the whole map, no wrong corner, the bake is ok.
//! - The cliff brush to height 2 on x 20–25, z 20–24 (rule „obere Zelle“): heights and rim as
//!   the unit tests of Task 3, the map asset's permissions block the 18 rim cells and none of
//!   the 26 ring cells, the mesh covers the 22 rim edges with vertical faces; then the set
//!   switched to `block: lower` and a cliff to height 2 on x 4–8, z 20–23: the ring blocks
//!   (without its four corner cells), the rim is free. The map stays within its limits, saves
//!   and builds a ROM.
//!
//! Each test prints one evidence line for the gate document (`smart-ring`, `smart-cliff`, and
//! the counts of the lower cliff as `smart-cliff-lower`). Run once per project in release:
//!
//! ```sh
//! G4_TEST_PROJECT=".../6-4b-pt_DSPRE_contents" \
//!   cargo test --release -p g4-kernel --all-features --test world_smart -- --nocapture --test-threads 1
//! ```
mod common;

use common::world::{exported_copy, painted_cell};

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use g4_formats::map::Cell;
use g4_kernel::build;
use g4_kernel::project::Project;
use g4_kernel::recipes::{self, RecipeCall};
use g4_kernel::store::Store;
use g4_kernel::world::edge::Place;
use g4_kernel::world::edit::{WorldEditRequest, WorldReport, world_edit};
use g4_kernel::world::grid::tile_token;
use g4_kernel::world::mesh::mesh;
use g4_kernel::world::smart::fixtures::{PLUS_EXPECTED, plus_cells};
use g4_kernel::world::smart::wrong_corners;
use g4_kernel::world::tileset::{TilesetEditRequest, save_tileset, tileset_edit};
use g4_kernel::world::{WorldGrid, tileset_rel};

/// Cells per row of a map.
const N: usize = 32;

/// The tiles the set needs: the outside and the 13 places.
const SET_TILES: usize = 14;

/// The name of the gate's edge set.
const SET: &str = "Gate";

fn store_of(copy: &Path) -> Store {
    Store::new(Arc::new(Project::open(copy).unwrap()))
}

/// The token of tile `i`.
fn token(i: usize) -> String {
    format!("{i:04x}")
}

/// The token of `place` in the gate set: `0001` to `000d` in the order of `Place::GATE13`.
fn place_token(place: Place) -> String {
    let i = Place::GATE13
        .iter()
        .position(|p| *p == place)
        .expect("a place of the gate");
    token(i + 1)
}

/// The tileset `tileset_edit` request: tiles `0000` to `000d` as plain ground and the set
/// „Gate“ with the edge rule `block` (`None`: the default, „obere Zelle“).
fn set_request(pack: u16, block: Option<&str>) -> TilesetEditRequest {
    let changes: Vec<_> = (0..SET_TILES)
        .map(|t| serde_json::json!({ "tile": t, "role": "ground", "kind": 0, "collision": 0 }))
        .collect();
    let mut places = serde_json::Map::new();
    for place in Place::GATE13 {
        places.insert(
            place.key().to_owned(),
            serde_json::json!([place_token(place)]),
        );
    }
    let mut set = serde_json::json!({ "name": SET, "outside": token(0), "places": places });
    if let Some(block) = block {
        set["block"] = serde_json::json!(block);
    }
    serde_json::from_value(serde_json::json!({
        "package": pack,
        "changes": changes,
        "sets": [{ "put": set }],
    }))
    .unwrap()
}

/// The new map „Kantenweg“ beside the start town with ground `0000`, its package's tileset
/// file and the set „Gate“: the map, its package and the package's tile count. Fails with the
/// sentence of the module documentation when the package has fewer than 14 tiles.
fn kantenweg(store: &mut Store) -> (u16, u16, usize) {
    let town = g4_kernel::preview::standard_map(store.project())
        .expect("the start town of the world-template recipe")
        .header;
    let ((x, y), _) = painted_cell(store.matrix(0).unwrap(), town);
    let call: RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "new_painted_map",
        "input": { "x": x, "y": y, "name": "Kantenweg", "fill_tile": "0000" },
    }))
    .unwrap();
    let dto = recipes::apply(store, &call)
        .unwrap_or_else(|e| panic!("new_painted_map in cell ({x}, {y}): {e}"));
    let report = &dto.report;
    let map = u16::try_from(report["map"].as_u64().unwrap()).unwrap();
    let pack = u16::try_from(report["tileset"].as_u64().unwrap()).unwrap();
    assert_eq!(report["bake"]["ok"], true, "the new map bakes: {report:#}");
    if !store.project().root.join(tileset_rel(pack)).exists() {
        save_tileset(store, pack).unwrap_or_else(|e| panic!("tileset_save {pack:04}: {e}"));
    }
    let tiles = store.tileset(pack).unwrap().tiles.len();
    assert!(
        tiles >= SET_TILES,
        "Paket {pack:04} hat nur {tiles} Kacheln; der Satz braucht 14."
    );
    tileset_edit(store, &set_request(pack, None))
        .unwrap_or_else(|e| panic!("tileset_edit {pack:04}: {e}"));
    (map, pack, tiles)
}

/// One smart stroke on layer 0 of `map`: `cells` with the change `paint` (`terrain` or
/// `cliff`), labelled as the surface labels it; the bake must be ok.
fn stroke(
    store: &mut Store,
    map: u16,
    verb: &str,
    cells: &[[u8; 2]],
    paint: serde_json::Value,
) -> WorldReport {
    let label = format!("{verb}: Karte {map:04} ({} Zellen)", cells.len());
    let mut change = serde_json::json!({ "cells": cells });
    let (key, value) = paint
        .as_object()
        .and_then(|o| o.iter().next())
        .expect("one paint field");
    change[key] = value.clone();
    let req: WorldEditRequest = serde_json::from_value(serde_json::json!({
        "map": map,
        "label": label,
        "changes": [change],
    }))
    .unwrap();
    let report = world_edit(store, &req).unwrap_or_else(|e| panic!("{label}: {e}"));
    let bake = report
        .bake
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: no bake"));
    assert!(bake.ok, "{label}: {:#?}", bake.messages);
    report
}

fn grid_of(store: &mut Store, map: u16) -> WorldGrid {
    let world = store.world(map).unwrap().expect("a world map").clone();
    WorldGrid::parse(&world).unwrap()
}

/// The place of a legend character of `PLUS_EXPECTED` (`.` is the outside tile).
fn legend(c: char) -> Option<Place> {
    Some(match c {
        '.' => return None,
        'C' => Place::Center,
        'N' => Place::N,
        'E' => Place::E,
        'S' => Place::S,
        'W' => Place::W,
        '1' => Place::OuterNw,
        '2' => Place::OuterNe,
        '3' => Place::OuterSw,
        '4' => Place::OuterSe,
        '5' => Place::InnerNw,
        '6' => Place::InnerNe,
        '7' => Place::InnerSw,
        '8' => Place::InnerSe,
        other => panic!("no legend for {other:?}"),
    })
}

/// Every cell of layer 0 is as `rows` say from (x0, z0) on (legend of `PLUS_EXPECTED`), every
/// other cell `0000`.
fn assert_raster(grid: &WorldGrid, x0: usize, z0: usize, rows: &[&str]) {
    for z in 0..N {
        for x in 0..N {
            let inside = x >= x0 && z >= z0 && z - z0 < rows.len() && x - x0 < rows[0].len();
            let want = if inside {
                let c = rows[z - z0].as_bytes()[x - x0] as char;
                legend(c).map_or_else(|| token(0), place_token)
            } else {
                token(0)
            };
            assert_eq!(
                tile_token(grid.layers[0].tiles[z * N + x]),
                want,
                "cell ({x}, {z})"
            );
        }
    }
}

/// The cells of the rectangle x `x0`–`x1`, z `z0`–`z1` on its border, row-major.
fn border(x0: usize, x1: usize, z0: usize, z1: usize) -> Vec<(usize, usize)> {
    (z0..=z1)
        .flat_map(|z| (x0..=x1).map(move |x| (x, z)))
        .filter(|&(x, z)| x == x0 || x == x1 || z == z0 || z == z1)
        .collect()
}

/// `border` without its four corner cells (they share no edge with the rectangle inside).
fn border_without_corners(x0: usize, x1: usize, z0: usize, z1: usize) -> Vec<(usize, usize)> {
    let corners = [(x0, z0), (x1, z0), (x0, z1), (x1, z1)];
    border(x0, x1, z0, z1)
        .into_iter()
        .filter(|c| !corners.contains(c))
        .collect()
}

fn rect_of(xs: std::ops::RangeInclusive<u8>, zs: std::ops::RangeInclusive<u8>) -> Vec<[u8; 2]> {
    zs.flat_map(|z| xs.clone().map(move |x| [x, z])).collect()
}

/// The blocked cells (0x80) of baked permissions as (x, z).
fn blocked(cells: &[Cell]) -> BTreeSet<(usize, usize)> {
    (0..N * N)
        .filter(|&i| cells[i].collision & 0x80 != 0)
        .map(|i| (i % N, i / N))
        .collect()
}

/// The world coordinate of the cell edge `c` (16 units per cell, map centre at 0).
fn edge_at(c: usize) -> i32 {
    c as i32 * 16 - 256
}

/// The edges of the rectangle x `x0`–`x1`, z `z0`–`z1` as world segments.
fn rim_edges(x0: usize, x1: usize, z0: usize, z1: usize) -> Vec<([i32; 2], [i32; 2])> {
    let mut edges = Vec::new();
    for x in x0..=x1 {
        let (a, b) = (edge_at(x), edge_at(x + 1));
        edges.push(([a, edge_at(z0)], [b, edge_at(z0)]));
        edges.push(([a, edge_at(z1 + 1)], [b, edge_at(z1 + 1)]));
    }
    for z in z0..=z1 {
        let (a, b) = (edge_at(z), edge_at(z + 1));
        edges.push(([edge_at(x0), a], [edge_at(x0), b]));
        edges.push(([edge_at(x1 + 1), a], [edge_at(x1 + 1), b]));
    }
    edges
}

/// Whether a vertical face of layer 0 (`walls`: bounding boxes) covers the segment a–b from
/// height `y0` to `y1`.
fn covered(walls: &[([i32; 3], [i32; 3])], a: [i32; 2], b: [i32; 2], y0: i32, y1: i32) -> bool {
    walls.iter().any(|(lo, hi)| {
        lo[0] <= a[0].min(b[0])
            && hi[0] >= a[0].max(b[0])
            && lo[2] <= a[1].min(b[1])
            && hi[2] >= a[1].max(b[1])
            && lo[1] <= y0
            && hi[1] >= y1
    })
}

#[test]
fn edge_set_rings_the_plus_without_a_wrong_corner() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let (map, pack, _) = kantenweg(&mut store);
    let cells = plus_cells();
    stroke(
        &mut store,
        map,
        "Gelände gemalt",
        &cells,
        serde_json::json!({ "terrain": { "set": SET } }),
    );
    let grid = grid_of(&mut store, map);
    assert_raster(&grid, 2, 3, &PLUS_EXPECTED);
    let set = store
        .tileset(pack)
        .unwrap()
        .edge_sets
        .iter()
        .find(|s| s.name == SET)
        .cloned()
        .expect("the set „Gate“");
    assert_eq!(wrong_corners(&grid, 0, &set), Vec::<[u8; 2]>::new());
    let used: BTreeSet<String> = grid.layers[0]
        .tiles
        .iter()
        .map(|t| tile_token(*t))
        .collect();
    for place in Place::GATE13 {
        assert!(used.contains(&place_token(place)), "{place:?}");
    }
    let nsbmd = store.map(map).unwrap().model.len();
    eprintln!(
        "smart-ring {code} map={map} package={pack} cells={} places=13 wrong=0 nsbmd={nsbmd}",
        cells.len()
    );
}

#[test]
fn cliff_sets_walls_and_collision_by_the_edge_rule() {
    let Some((tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let (map, pack, _) = kantenweg(&mut store);

    // Rule „obere Zelle“: x 20–25, z 20–24 to height 2.
    stroke(
        &mut store,
        map,
        "Klippe gesetzt",
        &rect_of(20..=25, 20..=24),
        serde_json::json!({ "cliff": { "set": SET, "height": 2 } }),
    );
    let grid = grid_of(&mut store, map);
    for z in 0..N {
        for x in 0..N {
            let up = (20..=25).contains(&x) && (20..=24).contains(&z);
            assert_eq!(
                grid.heights[z * N + x],
                if up { 2 } else { 0 },
                "height ({x}, {z})"
            );
        }
    }
    let cliff = [
        "........", ".1NNNN2.", ".WCCCCE.", ".WCCCCE.", ".WCCCCE.", ".3SSSS4.", "........",
    ];
    assert_raster(&grid, 19, 19, &cliff);
    let upper_rim = border(20, 25, 20, 24);
    let upper_ring = border(19, 26, 19, 25);
    assert_eq!((upper_rim.len(), upper_ring.len()), (18, 26));
    let cells = store.map(map).unwrap().permissions.clone();
    let now = blocked(&cells);
    let upper_blocked = upper_rim.iter().filter(|c| now.contains(c)).count();
    assert_eq!(upper_blocked, 18, "rim blocked: {now:?}");
    assert!(
        upper_ring.iter().all(|c| !now.contains(c)),
        "ring free: {now:?}"
    );
    assert_eq!(
        now,
        upper_rim.iter().copied().collect(),
        "only the rim blocks"
    );
    let tileset = store.tileset(pack).unwrap();
    let walls: Vec<([i32; 3], [i32; 3])> = mesh(&grid, &tileset)
        .into_iter()
        .filter(|f| f.layer == 0)
        .filter(|f| {
            let same = |k: usize| f.corners.iter().all(|c| c[k] == f.corners[0][k]);
            same(0) || same(2)
        })
        .map(|f| {
            let lo = [0, 1, 2].map(|k| f.corners.iter().map(|c| c[k]).min().unwrap());
            let hi = [0, 1, 2].map(|k| f.corners.iter().map(|c| c[k]).max().unwrap());
            (lo, hi)
        })
        .collect();
    let edges = rim_edges(20, 25, 20, 24);
    assert_eq!(edges.len(), 22);
    let walls_covering = edges
        .iter()
        .filter(|(a, b)| covered(&walls, *a, *b, 0, 16))
        .count();
    assert_eq!(walls_covering, 22, "rim edges covered by walls");

    // Rule „untere Zelle“: the set switched, x 4–8, z 20–23 to height 2.
    tileset_edit(&mut store, &set_request(pack, Some("lower")))
        .unwrap_or_else(|e| panic!("tileset_edit {pack:04} block lower: {e}"));
    let report = stroke(
        &mut store,
        map,
        "Klippe gesetzt",
        &rect_of(4..=8, 20..=23),
        serde_json::json!({ "cliff": { "set": SET, "height": 2 } }),
    );
    let lower_rim = border(4, 8, 20, 23);
    let lower_ring = border(3, 9, 19, 24);
    let lower_ring_edges = border_without_corners(3, 9, 19, 24);
    let lower_rim_edges = rim_edges(4, 8, 20, 23).len();
    let file = store.map(map).unwrap().clone();
    let now = blocked(&file.permissions);
    let lower_blocked = lower_ring_edges.iter().filter(|c| now.contains(c)).count();
    let lower_rim_free = lower_rim.iter().filter(|c| !now.contains(c)).count();
    eprintln!(
        "smart-cliff-lower {code} map={map} rim={} ring={} ring_at_edges={} rim_edges={} \
         ring_blocked={lower_blocked} rim_free={lower_rim_free}",
        lower_rim.len(),
        lower_ring.len(),
        lower_ring_edges.len(),
        lower_rim_edges
    );
    assert_eq!(
        lower_blocked,
        lower_ring_edges.len(),
        "ring blocked: {now:?}"
    );
    assert_eq!(lower_rim_free, lower_rim.len(), "rim free: {now:?}");
    let want: BTreeSet<(usize, usize)> =
        upper_rim.iter().chain(&lower_ring_edges).copied().collect();
    assert_eq!(now, want, "the upper rim and the lower ring block");

    let bake = report.bake.expect("the stroke bakes");
    assert!(
        file.bdhc.len() <= 0x9000 + 16,
        "BDHC {} bytes",
        file.bdhc.len()
    );
    assert!(
        file.model.len() <= 0xF000,
        "model {} bytes",
        file.model.len()
    );
    store.save().unwrap();
    let out = tmp.path().join("out.nds");
    build::build(&Project::open(&copy).unwrap(), &out).unwrap();
    assert!(out.is_file(), "the build writes the ROM");
    g4_kernel::start::inspect_rom(&out).unwrap();
    eprintln!(
        "smart-cliff {code} map={map} walls={walls_covering} upper_blocked={upper_blocked} \
         lower_blocked={lower_blocked} nsbmd={} share={:.3} bdhc={}",
        file.model.len(),
        bake.model_share,
        file.bdhc.len()
    );
}
