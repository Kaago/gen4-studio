#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, a DSPRE folder with its `.nds` beside it; only a copy is written,
//! see `vanilla_copy`): painting of 6.4a on the vanilla projects Platin DE (CPUD), HeartGold DE
//! (IPKD) and HeartGold US (IPKE), each set up as text project like `tests/world_bake.rs`.
//!
//! - „Neue Karte malen“ (`new_painted_map`, name „Malweg“) in a cell beside the start town
//!   (Zweiblattdorf, Neuborkia), then the strokes of the surface, one `world_edit` each with
//!   the label of `strokeLabel`: fill grass, brush, rectangle water, height twice, ramp,
//!   height. Every stroke bakes within the limits, the project saves and builds a ROM and the
//!   cheap check group finds no error.
//! - Undo of every stroke leaves every file under `data/` and the manifest `g4-project.json` as it was after
//!   the recipe (SHA-1), no file added.
//! - Every map texture package's tileset (as `tileset_get` derives it) and every tile's
//!   texture (`tileset_texture`, a placeholder where it gives none).
//!
//! The cell: the first empty or outside cell north, west, east, south of a start-town cell
//! (row by row); the vanilla matrices have landscape all around the start towns, so the
//! fallback is the nearest empty cell of matrix 0 (Chebyshev distance to the town's cells,
//! then row by row) with a real (non-landscape) map north, west, east or south of it. The
//! evidence line names the rule (`rule=beside|nearest`).
//!
//! Each test prints one evidence line for the gate document (`paint-gate`, `paint-undo`,
//! `palette`). Run once per project in release:
//!
//! ```sh
//! G4_TEST_PROJECT=".../6-4a-pt_DSPRE_contents" \
//!   cargo test --release -p g4-kernel --all-features --test world_paint -- --nocapture --test-threads 1
//! ```
mod common;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::base::BaseRom;
use g4_kernel::build;
use g4_kernel::checks::{
    Group, IgnoreSet, Limits, Runner, Severity, Snapshot, SnapshotCache, Subject,
};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::recipes::{self, RecipeCall};
use g4_kernel::store::Store;
use g4_kernel::textproject::{DATA_DIR, ExportOptions, MANIFEST_FILE, export};
use g4_kernel::world::edit::{WorldEditRequest, WorldReport, world_edit};
use g4_kernel::world::grid::{ramp_token, tile_token};
use g4_kernel::world::tileset::tileset_texture;
use g4_kernel::world::{TileRole, WorldGrid};
use sha1::{Digest, Sha1};

/// Cells per row of a map.
const N: usize = 32;

/// A vanilla copy with its sibling `.nds` assigned as base ROM, exported with script adoption:
/// the tempdir, the copy's root and the base ROM. `None` (after `skipped: …`) without
/// `G4_TEST_PROJECT` or without the `.nds`.
fn exported_copy() -> Option<(tempfile::TempDir, PathBuf, BaseRom)> {
    let orig = common::vanilla_project()?;
    let Some(nds) = g4_kernel::start::suggest_base(&orig) else {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            orig.display()
        );
        return None;
    };
    let (tmp, copy) = common::vanilla_copy()?;
    let (check, _mark) = g4_kernel::start::assign_base(&copy, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    export(
        &Project::open(&copy).unwrap(),
        &base,
        &ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
    Some((tmp, copy, base))
}

fn store_of(copy: &Path) -> Store {
    Store::new(Arc::new(Project::open(copy).unwrap()))
}

/// Header 0 of a filled cell in a header column: landscape (as `new_painted_map` reads it).
const LANDSCAPE: u16 = 0;

/// Which rule picked the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Beside,
    Nearest,
}

impl Rule {
    fn name(self) -> &'static str {
        match self {
            Rule::Beside => "beside",
            Rule::Nearest => "nearest",
        }
    }
}

/// The cell of matrix 0 for the painted map next to the start town `town` (its header), see
/// the module documentation. Panics with a sentence when there is none.
fn painted_cell(m: &Matrix, town: u16) -> ((u8, u8), Rule) {
    let (w, h) = (i32::from(m.width), i32::from(m.height));
    let index = |x: i32, y: i32| (y * w + x) as usize;
    let headers = m.headers.as_ref().expect("matrix 0 has a header column");
    let town_cells: Vec<(i32, i32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let i = index(x, y);
            m.maps[i] != MATRIX_EMPTY && headers[i] == town
        })
        .collect();
    assert!(
        !town_cells.is_empty(),
        "matrix 0 has no cell of header {town}"
    );
    let around = |x: i32, y: i32| [(x, y - 1), (x - 1, y), (x + 1, y), (x, y + 1)];
    for &(x, y) in &town_cells {
        for (nx, ny) in around(x, y) {
            if nx < 0 || ny < 0 || nx > 254 || ny > 254 {
                continue;
            }
            let outside = nx >= w || ny >= h;
            if outside || m.maps[index(nx, ny)] == MATRIX_EMPTY {
                return ((nx as u8, ny as u8), Rule::Beside);
            }
        }
    }
    let real = |x: i32, y: i32| {
        (0..w).contains(&x) && (0..h).contains(&y) && {
            let i = index(x, y);
            m.maps[i] != MATRIX_EMPTY && headers[i] != LANDSCAPE
        }
    };
    let distance = |x: i32, y: i32| {
        town_cells
            .iter()
            .map(|&(tx, ty)| (tx - x).abs().max((ty - y).abs()))
            .min()
            .unwrap()
    };
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| m.maps[index(x, y)] == MATRIX_EMPTY)
        .filter(|&(x, y)| around(x, y).into_iter().any(|(nx, ny)| real(nx, ny)))
        .min_by_key(|&(x, y)| (distance(x, y), y, x))
        .map(|(x, y)| ((x as u8, y as u8), Rule::Nearest))
        .unwrap_or_else(|| {
            panic!(
                "Keine leere Zelle der Weltkarte 0000 liegt neben der Startstadt (Header {town}) \
                 oder neben einer Karte, die keine Landschaft ist."
            )
        })
}

/// The new painted map („Malweg“) beside the start town, applied with `recipes::apply`: the
/// map, its package, the cell and the rule.
fn paint_new_map(store: &mut Store) -> (u16, u16, (u8, u8), Rule) {
    let town = g4_kernel::preview::standard_map(store.project())
        .expect("the start town of the world-template recipe")
        .header;
    let ((x, y), rule) = painted_cell(store.matrix(0).unwrap(), town);
    let call: RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "new_painted_map",
        "input": { "x": x, "y": y, "name": "Malweg" },
    }))
    .unwrap();
    let dto = recipes::apply(store, &call)
        .unwrap_or_else(|e| panic!("new_painted_map in cell ({x}, {y}) [{}]: {e}", rule.name()));
    let report = &dto.report;
    let map = u16::try_from(report["map"].as_u64().unwrap()).unwrap();
    let pack = u16::try_from(report["tileset"].as_u64().unwrap()).unwrap();
    assert_eq!(report["bake"]["ok"], true, "the new map bakes: {report:#}");
    (map, pack, (x, y), rule)
}

/// „Gefüllt: Karte 0123 (3 Zellen)“, the history text of the surface (`strokeLabel`).
fn stroke_label(verb: &str, map: u16, cells: usize) -> String {
    let count = if cells == 1 {
        "1 Zelle".to_owned()
    } else {
        format!("{cells} Zellen")
    };
    format!("{verb}: Karte {map:04} ({count})")
}

/// The ground cells of a 4-neighbour fill from `seed` over the cells with the seed's tile.
fn fill_cells(grid: &WorldGrid, seed: (usize, usize)) -> Vec<(usize, usize)> {
    let tiles = &grid.layers[0].tiles;
    let want = tiles[seed.1 * N + seed.0];
    let mut seen = vec![false; N * N];
    let mut queue = VecDeque::from([seed]);
    seen[seed.1 * N + seed.0] = true;
    let mut out = Vec::new();
    while let Some((x, z)) = queue.pop_front() {
        out.push((x, z));
        let next = [
            (x.wrapping_sub(1), z),
            (x + 1, z),
            (x, z.wrapping_sub(1)),
            (x, z + 1),
        ];
        for (nx, nz) in next {
            if nx < N && nz < N && !seen[nz * N + nx] && tiles[nz * N + nx] == want {
                seen[nz * N + nx] = true;
                queue.push_back((nx, nz));
            }
        }
    }
    out
}

/// The cells of a line from `a` to `b` (Bresenham, both ends included).
fn line_cells(a: (i32, i32), b: (i32, i32)) -> Vec<(usize, usize)> {
    let (mut x, mut z) = a;
    let (dx, dz) = ((b.0 - x).abs(), -(b.1 - z).abs());
    let (sx, sz) = (if x < b.0 { 1 } else { -1 }, if z < b.1 { 1 } else { -1 });
    let mut err = dx + dz;
    let mut out = Vec::new();
    loop {
        out.push((x as usize, z as usize));
        if (x, z) == b {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dz {
            err += dz;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            z += sz;
        }
    }
}

fn rect_cells(
    xs: std::ops::RangeInclusive<usize>,
    zs: std::ops::RangeInclusive<usize>,
) -> Vec<(usize, usize)> {
    zs.flat_map(|z| xs.clone().map(move |x| (x, z))).collect()
}

/// One stroke of the surface.
enum Stroke {
    Tile(&'static str, String, Vec<(usize, usize)>),
    /// One step up.
    Height(Vec<(usize, usize)>),
    Ramp(String, Vec<(usize, usize)>),
}

/// The tokens the strokes paint with: grass, water and a path tile of `pack`.
fn paint_tiles(store: &mut Store, pack: u16) -> (String, String, String) {
    let tileset = store.tileset(pack).unwrap();
    let token = |i: usize| format!("{i:04x}");
    let last = tileset.tiles.len().saturating_sub(1);
    let first = |role: TileRole| tileset.tiles.iter().position(|t| t.role == role);
    let grass = first(TileRole::Grass).unwrap_or(1.min(last));
    let water = first(TileRole::Water).unwrap_or(2.min(last));
    let path = tileset
        .tiles
        .iter()
        .enumerate()
        .position(|(i, t)| t.role == TileRole::Ground && i != grass && i != water)
        .unwrap_or(0);
    (token(grass), token(water), token(path))
}

/// The strokes of the gate on `map`, each sent as the surface sends it (unchanged cells
/// dropped, one `world_edit` with the label of `strokeLabel`): fill grass from (0, 0), a brush
/// line of the path tile, a water rectangle, two height strokes (a plateau of height 2 at
/// x 18–22, z 20–22 with a step at x 17), the ramp `e1` at x 16–17 and one height stroke on
/// one cell. The reports and the labels, in order.
fn paint_strokes(store: &mut Store, map: u16, pack: u16) -> Vec<(String, WorldReport)> {
    let (grass, water, path) = paint_tiles(store, pack);
    let band = 20..=22;
    let strokes = |grid: &WorldGrid, i: usize| -> Stroke {
        match i {
            0 => Stroke::Tile("Gefüllt", grass.clone(), fill_cells(grid, (0, 0))),
            1 => Stroke::Tile("Gemalt", path.clone(), line_cells((3, 14), (28, 17))),
            2 => Stroke::Tile("Rechteck gemalt", water.clone(), rect_cells(2..=8, 24..=29)),
            3 => Stroke::Height(rect_cells(17..=22, band.clone())),
            4 => Stroke::Height(rect_cells(18..=22, band.clone())),
            5 => Stroke::Ramp("e1".to_owned(), rect_cells(16..=17, band.clone())),
            _ => Stroke::Height(vec![(28, 28)]),
        }
    };
    let mut out = Vec::new();
    for i in 0..7 {
        let world = store.world(map).unwrap().expect("a world map").clone();
        let grid = WorldGrid::parse(&world).unwrap();
        let cell = |(x, z): (usize, usize)| serde_json::json!([x, z]);
        let (verb, changes, sent) = match strokes(&grid, i) {
            Stroke::Tile(verb, tile, cells) => {
                let changed: Vec<_> = cells
                    .into_iter()
                    .filter(|&(x, z)| tile_token(grid.layers[0].tiles[z * N + x]) != tile)
                    .collect();
                let n = changed.len();
                let change = serde_json::json!({
                    "layer": 0,
                    "cells": changed.into_iter().map(cell).collect::<Vec<_>>(),
                    "tile": tile,
                });
                (verb, vec![change], n)
            }
            Stroke::Height(cells) => {
                let mut groups: BTreeMap<i8, Vec<serde_json::Value>> = BTreeMap::new();
                for &(x, z) in &cells {
                    groups
                        .entry(grid.heights[z * N + x] + 1)
                        .or_default()
                        .push(cell((x, z)));
                }
                let changes = groups
                    .into_iter()
                    .map(|(height, cells)| serde_json::json!({ "cells": cells, "height": height }))
                    .collect();
                ("Höhe geändert", changes, cells.len())
            }
            Stroke::Ramp(ramp, cells) => {
                let changed: Vec<_> = cells
                    .into_iter()
                    .filter(|&(x, z)| ramp_token(grid.ramps[z * N + x]) != ramp)
                    .collect();
                let n = changed.len();
                let change = serde_json::json!({
                    "cells": changed.into_iter().map(cell).collect::<Vec<_>>(),
                    "ramp": ramp,
                });
                ("Rampe gesetzt", vec![change], n)
            }
        };
        assert!(sent > 0, "stroke {i} ({verb}) changes nothing");
        let label = stroke_label(verb, map, sent);
        let req: WorldEditRequest = serde_json::from_value(serde_json::json!({
            "map": map,
            "label": label,
            "changes": changes,
        }))
        .unwrap();
        let report = world_edit(store, &req).unwrap_or_else(|e| panic!("{label}: {e}"));
        let bake = report
            .bake
            .as_ref()
            .unwrap_or_else(|| panic!("{label}: no bake"));
        assert!(bake.ok, "{label}: {:#?}", bake.messages);
        out.push((label, report));
    }
    out
}

/// SHA-1 of every file under `data/` and of the manifest (`g4-project.json`, the plan's
/// `project.json`), by path relative to `root`.
fn hashes(root: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_owned(), sha1_of(&path));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &root.join(DATA_DIR), &mut out);
    out.insert(
        PathBuf::from(MANIFEST_FILE),
        sha1_of(&root.join(MANIFEST_FILE)),
    );
    out
}

fn sha1_of(path: &Path) -> String {
    let digest = Sha1::digest(std::fs::read(path).unwrap());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn painted_map_beside_the_start_town_bakes_and_builds() {
    let Some((tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let started = Instant::now();
    let mut store = store_of(&copy);
    let (map, pack, (x, y), rule) = paint_new_map(&mut store);
    let strokes = paint_strokes(&mut store, map, pack);
    let (undo, _) = store.history();
    assert_eq!(undo.len(), 1 + strokes.len(), "{undo:#?}");
    let labels: Vec<_> = strokes.iter().map(|(l, _)| l.clone()).collect();
    assert_eq!(&undo[1..], &labels[..]);
    let bake = strokes
        .last()
        .and_then(|(_, r)| r.bake.clone())
        .expect("the last stroke bakes");
    assert!(bake.ok, "{:#?}", bake.messages);
    let file = store.map(map).unwrap().clone();
    assert!(
        file.model.len() <= 0xF000,
        "model {} bytes",
        file.model.len()
    );
    assert!(
        file.bdhc.len() <= 0x9000 + 16,
        "BDHC {} bytes",
        file.bdhc.len()
    );
    store.save().unwrap();

    let out = tmp.path().join("out.nds");
    build::build(&Project::open(&copy).unwrap(), &out).unwrap();
    assert!(out.is_file(), "the build writes the ROM");
    g4_kernel::start::inspect_rom(&out).unwrap();

    let project = Arc::new(Project::open(&copy).unwrap());
    let snap = Snapshot::build(
        Arc::clone(&project),
        store_of(&copy).snapshot_overlay(),
        SnapshotCache::default(),
    );
    let report = Runner::run(
        &snap,
        Limits::get(),
        &IgnoreSet::default(),
        &[Group::Cheap],
        |_, _, _| {},
    );
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    let of_map = errors
        .iter()
        .filter(|f| f.subject == Subject::Map(map))
        .count();
    assert!(
        errors.is_empty(),
        "checks ({of_map} of map {map}): {errors:#?}"
    );
    eprintln!(
        "paint-gate {code} map={map} cell=({x},{y}) rule={} strokes={} nsbmd={} share={:.3} \
         bdhc={} ms={}",
        rule.name(),
        strokes.len(),
        file.model.len(),
        bake.model_share,
        file.bdhc.len(),
        started.elapsed().as_millis()
    );
}

#[test]
fn undo_to_the_start_leaves_the_files_as_they_were() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let (map, pack, _, _) = paint_new_map(&mut store);
    store.save().unwrap();
    let before = hashes(&copy);

    let strokes = paint_strokes(&mut store, map, pack);
    store.save().unwrap();
    assert_ne!(hashes(&copy), before, "the strokes change a file");
    for _ in &strokes {
        store.undo().unwrap();
    }
    assert_eq!(store.history().0.len(), 1, "the recipe's step stays");
    store.save().unwrap();
    let after = hashes(&copy);

    let added: BTreeSet<_> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    let missing: BTreeSet<_> = before.keys().filter(|k| !after.contains_key(*k)).collect();
    let differing: Vec<_> = before
        .iter()
        .filter(|(k, v)| after.get(*k).is_some_and(|a| a != *v))
        .map(|(k, _)| k)
        .collect();
    let equal = before
        .iter()
        .filter(|(k, v)| after.get(*k) == Some(v))
        .count();
    eprintln!(
        "paint-undo {code} strokes={} files={} equal={equal}",
        strokes.len(),
        before.len()
    );
    assert!(added.is_empty(), "files added: {added:?}");
    assert!(missing.is_empty(), "files gone: {missing:?}");
    assert!(differing.is_empty(), "files changed: {differing:?}");
    assert_eq!(equal, before.len());
}

#[test]
fn palette_textures_of_every_map_package() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let code = project.game_code.clone();
    let packages = project.ensure_unpacked(AssetKind::MapTexture).unwrap();
    let mut store = Store::new(Arc::new(project));
    let (mut tiles, mut decoded, mut placeholder) = (0, 0, 0);
    for p in 0..u16::try_from(packages).unwrap() {
        let tileset = store.tileset(p).unwrap();
        for tile in &tileset.tiles {
            tiles += 1;
            match tileset_texture(store.project(), &tileset, &tile.texture).unwrap() {
                Some(rgba) => {
                    assert!(
                        rgba.width > 0 && rgba.height > 0,
                        "package {p} {}",
                        tile.texture
                    );
                    decoded += 1;
                }
                None => placeholder += 1,
            }
        }
    }
    eprintln!(
        "palette {code} packages={packages} tiles={tiles} decoded={decoded} \
         placeholder={placeholder}"
    );
    assert!(tiles > 0);
}
