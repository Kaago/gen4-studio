#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, a DSPRE folder with its `.nds` beside it; only a copy is written,
//! see `vanilla_copy`): the world bake of 6.3b on the vanilla projects Platin DE (CPUD),
//! HeartGold DE (IPKD) and HeartGold US (IPKE), each set up as text project (base ROM assigned,
//! exported with script adoption).
//!
//! - Form 1 („Karte als Quelle übernehmen“) of every map bakes byte-equal to its member, and the
//!   project with every map adopted builds the base ROM byte for byte after its map members were
//!   deleted and imported again.
//! - A map painted on a vanilla package (`gate_world`: ground, two heights with wall, ramp,
//!   water, grass) bakes within the limits, its model floor meets the BDHC at every cell centre,
//!   and apicula (`G4_APICULA`, else `apicula=skipped`) reads it like the mesher's faces.
//! - The build step `world` bakes one map for one changed JSON; a member changed outside fails
//!   the build with `world.member-changed` and is settled both ways.
//!
//! Each test prints one evidence line for the gate document (`world-adopt`, `world-adopt-build`,
//! `world-gate`, `world-step`, `world-conflict`). Run once per project in release:
//!
//! ```sh
//! G4_TEST_PROJECT=".../6-3b-pt_DSPRE_contents" G4_APICULA=".../apicula" \
//!   cargo test --release -p g4-kernel --all-features --test world_bake -- --nocapture --test-threads 1
//! ```
mod common;

use common::world::exported_copy;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use g4_formats::bdhc::{Bdhc, tile_center};
use g4_formats::game::Family;
use g4_formats::map::{Cell, MapFile};
use g4_formats::nsbtx::Nsbtx;
use g4_kernel::bake::{Face, decode_quads};
use g4_kernel::build::{self, StepStatus};
use g4_kernel::error::KernelError;
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::text::Severity;
use g4_kernel::textproject::sync::{Keep, record_all, resolve};
use g4_kernel::textproject::{ImportOptions, check, import_with};
use g4_kernel::world::doc::fixtures::gate_world;
use g4_kernel::world::edit::{WorldCreateRequest, WorldEditRequest, world_create, world_edit};
use g4_kernel::world::keep::world_field;
use g4_kernel::world::mesh::{mesh, mesh_textures};
use g4_kernel::world::{
    AdoptForm, FLOOR_TOLERANCE, TileChange, TileRole, TilesetEditRequest, WorldAdoptRequest,
    WorldGrid, WorldJson, override_token, packs_of_map, parse_override, save_tileset, set_token,
    tileset_edit, world_adopt, world_parts,
};

use common::glb::{
    glb_chunks, glb_materials, glb_triangles, glb_wrapped, input_wrapped, unmatched_triangles,
};

/// Cells per row of a map.
const N: usize = 32;

fn ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

fn store_of(copy: &Path) -> Store {
    Store::new(Arc::new(Project::open(copy).unwrap()))
}

fn member(copy: &Path, map: u16) -> Vec<u8> {
    let project = Project::open(copy).unwrap();
    std::fs::read(project.member_path(AssetKind::Map, map)).unwrap()
}

fn write_member(copy: &Path, map: u16, bytes: &[u8]) {
    let project = Project::open(copy).unwrap();
    std::fs::write(project.member_path(AssetKind::Map, map), bytes).unwrap();
}

/// The member a fresh store bakes for `map` from the files on disk.
fn bake_of(copy: &Path, map: u16) -> Vec<u8> {
    let mut store = store_of(copy);
    let family = store.project().family;
    store.map(map).unwrap().write(family)
}

fn json_rel(map: u16) -> String {
    format!("data/maps/{map:04}.json")
}

/// The number of maps of the vanilla game `code`.
fn vanilla_maps(code: &str) -> usize {
    match code {
        "CPUD" | "CPUE" => 666,
        "IPKD" | "IPKE" => 676,
        other => panic!("no map count known for {other}"),
    }
}

fn adopt_request(map: u16) -> WorldAdoptRequest {
    WorldAdoptRequest {
        map,
        tileset: None,
        form: AdoptForm::Verbatim,
    }
}

/// Form 1 of every map in `store`, each compared with the member it had before.
#[derive(Debug, Default)]
struct Adopted {
    maps: usize,
    adopted: usize,
    equal: usize,
    deviating_maps: usize,
    deviating_cells: usize,
    refused: Vec<(u16, String)>,
    differing: Vec<u16>,
}

fn adopt_all(store: &mut Store) -> Adopted {
    let project = Project::open(&store.project().root).unwrap();
    project.ensure_unpacked(AssetKind::Map).unwrap();
    let family = project.family;
    let maps = project.member_count(AssetKind::Map).unwrap();
    let mut out = Adopted {
        maps,
        ..Adopted::default()
    };
    for map in 0..u16::try_from(maps).unwrap() {
        let original = std::fs::read(project.member_path(AssetKind::Map, map)).unwrap();
        match world_adopt(store, &adopt_request(map)) {
            Ok(report) => {
                out.adopted += 1;
                if !report.deviations.is_empty() {
                    out.deviating_maps += 1;
                    out.deviating_cells += report.deviations.len();
                }
            }
            Err(e) => {
                out.refused.push((map, e.to_string()));
                continue;
            }
        }
        if store.map(map).unwrap().write(family) == original {
            out.equal += 1;
        } else {
            out.differing.push(map);
        }
    }
    out
}

#[test]
fn vanilla_maps_adopted_form1_bake_byte_equal() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let started = Instant::now();
    let a = adopt_all(&mut store);
    eprintln!(
        "world-adopt {code} maps={} adopted={} equal={} deviating_maps={} deviating_cells={} \
         ms={}",
        a.maps,
        a.adopted,
        a.equal,
        a.deviating_maps,
        a.deviating_cells,
        ms(started)
    );
    assert!(a.refused.is_empty(), "refused: {:?}", a.refused);
    assert!(a.differing.is_empty(), "differing: {:?}", a.differing);
    assert_eq!(a.maps, vanilla_maps(&code));
    assert_eq!((a.adopted, a.equal), (a.maps, a.maps));
}

#[test]
fn vanilla_adopted_project_builds_byte_equal() {
    let Some((tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let started = Instant::now();
    let mut store = store_of(&copy);
    let a = adopt_all(&mut store);
    assert!(a.refused.is_empty(), "refused: {:?}", a.refused);
    assert_eq!(a.adopted, vanilla_maps(&code));
    let adopt_ms = ms(started);
    let saving = Instant::now();
    store.save().unwrap();
    let save_ms = ms(saving);
    drop(store);

    // Every map member goes; the import bakes them from the worlds.
    let project = Project::open(&copy).unwrap();
    for map in 0..u16::try_from(a.maps).unwrap() {
        std::fs::remove_file(project.member_path(AssetKind::Map, map)).unwrap();
    }
    let importing = Instant::now();
    let report = import_with(&project, Some(&base), &ImportOptions { force: false }).unwrap();
    let import_ms = ms(importing);
    let errors: Vec<_> = report
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "import: {errors:#?}");
    for map in 0..u16::try_from(a.maps).unwrap() {
        assert!(
            project.member_path(AssetKind::Map, map).is_file(),
            "map {map:04} has no member after the import"
        );
    }

    let building = Instant::now();
    let out = tmp.path().join("out.nds");
    build::build(&Project::open(&copy).unwrap(), &out).unwrap();
    let sha1 = g4_kernel::start::inspect_rom(&out).unwrap().sha1;
    let build_ms = ms(building);
    assert_eq!(sha1, base.sha1(), "the adopted project builds another ROM");

    let checked = check(&Project::open(&copy).unwrap()).unwrap();
    let errors: Vec<_> = checked
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "check: {errors:#?}");
    eprintln!(
        "world-adopt-build {code} ms={} adopt_ms={adopt_ms} save_ms={save_ms} \
         import_ms={import_ms} build_ms={build_ms} check_problems={}",
        ms(started),
        checked.problems.len()
    );
}

/// The map of the first cell that shows header `header` in its matrix.
fn first_map_of_header(store: &mut Store, header: u16) -> u16 {
    let matrix = store.header_table().unwrap()[usize::from(header)].matrix_id();
    let m = store.matrix(matrix).unwrap();
    m.maps
        .iter()
        .enumerate()
        .find(|&(i, &map)| {
            map != g4_formats::matrix::MATRIX_EMPTY
                && m.headers.as_ref().is_none_or(|h| h.get(i) == Some(&header))
        })
        .map(|(_, &map)| map)
        .unwrap_or_else(|| panic!("no map shows header {header}"))
}

/// The first header of Zweiblattdorf (Platinum) or Neuborkia (HeartGold).
fn start_town(code: &str) -> u16 {
    match code {
        "CPUD" | "CPUE" => 411,
        "IPKD" | "IPKE" => 60,
        other => panic!("no start town known for {other}"),
    }
}

/// The tokens of one row of the world file.
fn tokens(row: &str) -> Vec<&str> {
    row.split(' ').collect()
}

/// The `world_edit` changes that turn the ground of `from` into the ground and the overrides
/// of `to`: per value one change with every cell that differs.
fn changes_between(from: &WorldJson, to: &WorldJson) -> Vec<serde_json::Value> {
    use std::collections::BTreeMap;
    let mut out = Vec::new();
    let rows = |w: &WorldJson, what: &str| -> Vec<String> {
        let ground = &w.layers[0];
        match what {
            "tile" => ground.tiles.clone(),
            "height" => ground
                .heights
                .clone()
                .unwrap_or_else(|| vec![zeros("0"); N]),
            "ramp" => ground.ramps.clone().unwrap_or_else(|| vec![zeros("."); N]),
            _ => w
                .permissions
                .overrides
                .clone()
                .unwrap_or_else(|| vec![zeros("."); N]),
        }
    };
    for what in ["tile", "height", "ramp", "permission"] {
        let (a, b) = (rows(from, what), rows(to, what));
        let mut cells: BTreeMap<String, Vec<[u8; 2]>> = BTreeMap::new();
        for z in 0..N {
            for (x, (old, new)) in tokens(&a[z]).iter().zip(tokens(&b[z])).enumerate() {
                if *old != new {
                    cells
                        .entry(new.to_owned())
                        .or_default()
                        .push([x as u8, z as u8]);
                }
            }
        }
        for (value, cells) in cells {
            let value = match what {
                "height" => serde_json::json!(value.parse::<i8>().unwrap()),
                _ => serde_json::json!(value),
            };
            out.push(serde_json::json!({ "cells": cells, what: value }));
        }
    }
    out
}

fn zeros(token: &str) -> String {
    vec![token; N].join(" ")
}

/// Where the gate map's bake stands: (largest floor deviation, cells checked).
fn floor_deviation(nsbmd: &[u8], grid: &WorldGrid, bdhc: &[u8]) -> (f64, usize) {
    let quads = decode_quads(nsbmd).expect("the baked model decodes");
    let up: Vec<_> = quads.iter().filter(|q| normal(&q.corners)[1] > 0).collect();
    let terrain = Bdhc::read(bdhc).expect("the BDHC reads");
    let (mut worst, mut checked) = (0.0f64, 0);
    for z in 0..N {
        for x in 0..N {
            if grid.layers[0].tiles[z * N + x].is_none() {
                continue;
            }
            let (fx, fz) = tile_center(x, z);
            let (mx, mz) = (f64::from(fx) / 4096.0, f64::from(fz) / 4096.0);
            let floor = up
                .iter()
                .filter_map(|q| {
                    let c = q.corners;
                    on_triangle(c[0], c[1], c[2], mx, mz)
                        .or_else(|| on_triangle(c[0], c[2], c[3], mx, mz))
                })
                .fold(None, |best: Option<f64>, y| {
                    Some(best.map_or(y, |b| b.max(y)))
                })
                .unwrap_or_else(|| panic!("no floor over cell ({x}, {z})"));
            let walk = terrain
                .height_at(fx, fz, i32::MAX)
                .unwrap_or_else(|| panic!("no BDHC height at cell ({x}, {z})"));
            worst = worst.max((floor - f64::from(walk) / 4096.0).abs());
            checked += 1;
        }
    }
    (worst, checked)
}

/// (c1 − c0) × (c2 − c0), falling back to (c2 − c0) × (c3 − c0) for a degenerate first pair.
fn normal(c: &[[i32; 3]; 4]) -> [i64; 3] {
    let sub = |a: [i32; 3], b: [i32; 3]| [0, 1, 2].map(|i| i64::from(a[i]) - i64::from(b[i]));
    let cross = |u: [i64; 3], v: [i64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let n = cross(sub(c[1], c[0]), sub(c[2], c[0]));
    if n != [0; 3] {
        n
    } else {
        cross(sub(c[2], c[0]), sub(c[3], c[0]))
    }
}

/// The height at (x, z) on the triangle a, b, c (seen from above), or `None` outside it.
fn on_triangle(a: [i32; 3], b: [i32; 3], c: [i32; 3], x: f64, z: f64) -> Option<f64> {
    let [ax, ay, az] = a.map(f64::from);
    let [bx, by, bz] = b.map(f64::from);
    let [cx, cy, cz] = c.map(f64::from);
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-9 {
        return None;
    }
    let l1 = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) / det;
    let l2 = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) / det;
    let l3 = 1.0 - l1 - l2;
    let eps = 1e-9;
    (l1 >= -eps && l2 >= -eps && l3 >= -eps).then_some(l1 * ay + l2 * by + l3 * cy)
}

#[test]
fn painted_map_on_a_vanilla_pack_bakes_within_limits() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let map = first_map_of_header(&mut store, start_town(&code));
    let pack = *packs_of_map(&mut store, map)
        .unwrap()
        .first()
        .expect("the start town has a map texture package");
    save_tileset(&mut store, pack).unwrap();
    let tile = |tile: u16, role: TileRole, kind: u8| TileChange {
        tile,
        role: Some(role),
        kind: Some(kind),
        collision: None,
        sound: None,
        wall: None,
    };
    tileset_edit(
        &mut store,
        &TilesetEditRequest {
            package: pack,
            changes: vec![
                tile(1, TileRole::Grass, 0x02),
                tile(2, TileRole::Water, 0x15),
            ],
            sets: Vec::new(),
        },
    )
    .unwrap();
    let create: WorldCreateRequest =
        serde_json::from_value(serde_json::json!({ "map": map, "tileset": pack })).unwrap();
    world_create(&mut store, &create).unwrap();
    let want = gate_world(pack, "0000", "0001", "0002");
    let created = store.world(map).unwrap().unwrap().clone();
    let edit: WorldEditRequest = serde_json::from_value(serde_json::json!({
        "map": map,
        "changes": changes_between(&created, &want),
    }))
    .unwrap();
    let report = world_edit(&mut store, &edit).unwrap();
    let bake = report.bake.expect("the stroke bakes");
    assert!(bake.ok, "{:#?}", bake.messages);
    let world = store.world(map).unwrap().unwrap().clone();
    let grid = WorldGrid::parse(&world).unwrap();
    let want_grid = WorldGrid::parse(&want).unwrap();
    assert_eq!(
        grid.heights, want_grid.heights,
        "the edit paints gate_world"
    );
    assert_eq!(grid.ramps, want_grid.ramps, "the edit paints gate_world");

    assert_eq!(
        grid.layers[0].tiles, want_grid.layers[0].tiles,
        "the edit paints gate_world"
    );

    let file = store.map(map).unwrap().clone();
    let quads = decode_quads(&file.model).expect("the baked model decodes");
    // Grass (20, 5) and water (5, 26) take their tile's permission, the door (12, 21) its hand
    // value.
    let tiles = store.tileset(pack).unwrap();
    let family = store.project().family;
    let derived = |tile: usize| {
        let t = &tiles.tiles[tile];
        let sound = if family == Family::Hgss { t.sound } else { 0 };
        Cell {
            kind: t.kind,
            collision: t.collision | sound,
        }
    };
    let cell = |x: usize, z: usize| file.permissions[z * N + x];
    assert_eq!((cell(20, 5).kind, cell(20, 5).collision & 0x80), (0x02, 0));
    assert_eq!(cell(20, 5), derived(1), "grass");
    assert_eq!((cell(5, 26).kind, cell(5, 26).collision & 0x80), (0x15, 0));
    assert_eq!(cell(5, 26), derived(2), "water");
    assert_eq!(
        cell(12, 21),
        Cell {
            kind: 0x69,
            collision: 0x80
        },
        "door"
    );
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
    let (floor_max, checked) = floor_deviation(&file.model, &grid, &file.bdhc);
    assert_eq!(checked, N * N);
    assert!(floor_max <= FLOOR_TOLERANCE, "floor deviation {floor_max}");

    let tileset = store.tileset(pack).unwrap();
    let faces = mesh(&grid, &tileset);
    assert_eq!(quads.len(), faces.len(), "one baked quad per face");
    let pack_bytes = store
        .project()
        .map_tileset_member(pack)
        .unwrap()
        .expect("the package member");
    let apicula = match std::env::var_os("G4_APICULA") {
        None => {
            eprintln!("skipped: apicula part needs G4_APICULA");
            "skipped"
        }
        Some(apicula) => {
            let dir = tempfile::tempdir().unwrap();
            let glb_path = nsbmd_to_glb(
                Path::new(&apicula),
                &file.model,
                Some(&pack_bytes),
                dir.path(),
            )
            .unwrap();
            let glb = std::fs::read(&glb_path).unwrap();
            let (json, bin) = glb_chunks(&glb);
            let sizes = Nsbtx::read(&pack_bytes).unwrap().textures;
            let size = |name: &str| {
                sizes
                    .iter()
                    .find(|t| t.name == name)
                    .map(|t| (t.width as u16, t.height as u16))
                    .unwrap_or_else(|| panic!("{name} is no texture of the package"))
            };
            let got = glb_wrapped(&json, &bin, size);
            let inputs: Vec<Face> = faces.iter().cloned().map(Face::from).collect();
            let want = input_wrapped(&inputs, size);
            let missing: Vec<_> = want.difference(&got).take(5).collect();
            let extra: Vec<_> = got.difference(&want).take(5).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "missing {missing:?}, extra {extra:?}"
            );
            let triangles = glb_triangles(&json, &bin, size);
            assert_eq!(triangles.len(), 2 * faces.len(), "two triangles per face");
            let unmatched = unmatched_triangles(&inputs, &triangles, size);
            assert!(
                unmatched.is_empty(),
                "{} of {}: {:?}",
                unmatched.len(),
                triangles.len(),
                &unmatched[..unmatched.len().min(5)]
            );
            assert_eq!(glb_materials(&json), mesh_textures(&faces));
            "ok"
        }
    };
    eprintln!(
        "world-gate {code} map={map} pack={pack} nsbmd={} share={:.3} bdhc={} faces={} \
         floor_max_dev={floor_max:.4} apicula={apicula}",
        file.model.len(),
        bake.model_share,
        file.bdhc.len(),
        faces.len()
    );
}

/// Map `map` adopted (Form 1) and saved.
fn adopt_and_save(copy: &Path, maps: &[u16]) {
    let mut store = store_of(copy);
    for &map in maps {
        world_adopt(&mut store, &adopt_request(map)).unwrap();
    }
    store.save().unwrap();
}

/// The hand value of cell (0, 0) of `map`'s world toggled in its JSON, as Git or an editor
/// would (no store, no record).
fn edit_json(copy: &Path, map: u16) {
    let path = copy.join(json_rel(map));
    let bytes = std::fs::read(&path).unwrap();
    let mut world = world_field(&bytes).unwrap().expect("a world map");
    let overrides = world.permissions.overrides.as_mut().expect("hand values");
    let token = tokens(&overrides[0])[0].to_owned();
    let mut cell = parse_override(&token).unwrap().unwrap_or_default();
    cell.collision ^= 0x80;
    overrides[0] = set_token(&overrides[0], 0, &override_token(Some(cell))).unwrap();
    let file = world_parts(bytes, &world, &mut Vec::new()).unwrap();
    std::fs::write(&path, file).unwrap();
}

#[test]
fn world_step_is_incremental_per_map_on_the_copy() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    adopt_and_save(&copy, &[0, 1]);
    build::prepare(&Project::open(&copy).unwrap()).unwrap();
    let member_0 = member(&copy, 0);
    let member_1 = member(&copy, 1);

    edit_json(&copy, 1);
    let started = Instant::now();
    let prepared = build::prepare(&Project::open(&copy).unwrap()).unwrap();
    let step_ms = ms(started);
    let row = prepared
        .steps
        .iter()
        .find(|r| r.id == "world")
        .unwrap_or_else(|| panic!("no row world: {:?}", prepared.steps));
    assert_eq!(row.status, StepStatus::Incremental, "{row:?}");
    assert_eq!((row.done, row.total), (1, 2), "{row:?}");
    assert_eq!(member(&copy, 0), member_0, "map 0 stays");
    assert_ne!(member(&copy, 1), member_1, "map 1 is baked anew");
    assert_eq!(member(&copy, 1), bake_of(&copy, 1));
    eprintln!(
        "world-step {code} baked={} of={} ms={step_ms}",
        row.done, row.total
    );
}

/// Flips bit 0 of the kind of cell 0 in `map`'s member, as DSPRE would (the member still
/// reads); the bytes written.
fn dspre_member(copy: &Path, map: u16) -> Vec<u8> {
    let family = Project::open(copy).unwrap().family;
    let mut file = MapFile::read(&member(copy, map), family).unwrap();
    file.permissions[0].kind ^= 0x01;
    let outside = file.write(family);
    write_member(copy, map, &outside);
    outside
}

#[test]
fn member_changed_both_ways_on_the_copy() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let map = 0u16;
    let key = json_rel(map);
    adopt_and_save(&copy, &[map]);
    build::prepare(&Project::open(&copy).unwrap()).unwrap();
    record_all(&Project::open(&copy).unwrap()).unwrap();
    let baked = member(&copy, map);

    // DSPRE changes the member: the build fails with `world.member-changed`.
    let outside = dspre_member(&copy, map);
    let problems = match build::prepare(&Project::open(&copy).unwrap()) {
        Err(KernelError::Compile(problems)) => problems,
        other => panic!("the build passes over a changed member: {other:?}"),
    };
    assert!(
        problems
            .iter()
            .any(|p| p.message.starts_with("world.member-changed:") && p.file == Path::new(&key)),
        "{problems:#?}"
    );
    assert_eq!(member(&copy, map), outside, "nothing written");

    // „Neu backen“: the member is the bake again.
    let mut store = store_of(&copy);
    resolve(&mut store, &key, Keep::Studio).unwrap();
    assert_eq!(member(&copy, map), bake_of(&copy, map));
    assert_eq!(member(&copy, map), baked);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(copy.join(&key)).unwrap()).unwrap();
    assert_eq!(json["source"], "world");
    build::prepare(&Project::open(&copy).unwrap()).unwrap();

    // Changed again, „Als binäre Karte übernehmen“: the JSON is binary, the member DSPRE's.
    let outside = dspre_member(&copy, map);
    let mut store = store_of(&copy);
    resolve(&mut store, &key, Keep::Dspre).unwrap();
    assert_eq!(member(&copy, map), outside);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(copy.join(&key)).unwrap()).unwrap();
    assert_eq!(json["source"], "binary", "{json}");
    assert!(json.get("world").is_none());
    let family: Family = store.project().family;
    assert_eq!(store.map(map).unwrap().write(family), outside);
    build::prepare(&Project::open(&copy).unwrap()).unwrap();
    assert_eq!(
        member(&copy, map),
        outside,
        "the build keeps DSPRE's member"
    );
    eprintln!("world-conflict {code} rebake=ok binary=ok");
}
