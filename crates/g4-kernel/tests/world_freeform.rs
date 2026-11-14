#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, a DSPRE folder with its `.nds` beside it; only a copy is written,
//! see `vanilla_copy`): free-form models, piece models, Form 2 and the Tiled exchange of 6.5b on
//! the vanilla projects Platin DE (CPUD), HeartGold DE (IPKD) and HeartGold US (IPKE), each set
//! up as text project (base ROM assigned, exported with script adoption).
//!
//! - A cube added as free-form model to a painted map bakes within the limits, its six faces on
//!   layer 254, and apicula (`G4_APICULA`, else `apicula=skipped`) reads every corner of them.
//! - The cube as piece model of a free tile, placed once on the painted map, bakes.
//! - Form 2 („als glTF übernehmen“) of the start town's map bakes apicula-readable with BDHC and
//!   permissions of the original (needs `G4_APICULA`, else `skipped`); with `G4_FORM2_SWEEP=1`
//!   every map once (evidence, no threshold).
//! - TMJ export and import without change leave the painted map's file byte-equal and the world
//!   of every map adopted in Form 1 equal.
//!
//! Each test prints one evidence line for the gate document (`world-freeform`, `world-piece`,
//! `world-form2`, `world-form2-sweep`, `world-tmj`). Run once per project in release:
//!
//! ```sh
//! G4_TEST_PROJECT=".../6-5b-pt_DSPRE_contents" G4_APICULA=".../apicula" \
//!   cargo test --release --workspace --all-features --test world_freeform -- --nocapture \
//!   --test-threads 1
//! ```
mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::map::MapFile;
use g4_formats::nsbtx::Nsbtx;
use g4_kernel::bake::Face;
use g4_kernel::error::KernelError;
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::model_import::ImportOptions;
use g4_kernel::model_import::samples::{CUBE_OBJ, write_samples};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::world::doc::fixtures::gate_world;
use g4_kernel::world::edit::{WorldCreateRequest, WorldEditRequest, world_create, world_edit};
use g4_kernel::world::{
    ADOPT_TEXTURE_CODE, AdoptForm, FREEFORM_LAYER, FreeformAddRequest, PieceJson, PieceSetRequest,
    TileChange, TileRole, TilesetEditRequest, WorldAdoptRequest, WorldJson, packs_of_map,
    save_tileset, tileset_edit, tileset_piece_set, world_adopt, world_adopt_with, world_export_tmj,
    world_faces, world_freeform_add, world_import_tmj,
};

use common::glb::{glb_chunks, glb_set, glb_wrapped, input_set, input_wrapped};
use common::world::exported_copy;

/// Cells per row of a map.
const N: usize = 32;
/// The largest model of a map.
const MODEL_MAX: usize = 0xF000;

/// apicula from `G4_APICULA`, `None` (after `skipped: …`) without it.
fn apicula() -> Option<PathBuf> {
    g4_formats::test_support::gate_var("G4_APICULA").map(PathBuf::from)
}

fn ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

fn store_of(copy: &Path) -> Store {
    Store::new(Arc::new(Project::open(copy).unwrap()))
}

fn json_rel(map: u16) -> String {
    format!("data/maps/{map:04}.json")
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

/// The map of the start town's first header.
fn start_map(store: &mut Store, code: &str) -> u16 {
    first_map_of_header(store, start_town(code))
}

/// The tokens of one row of the world file.
fn tokens(row: &str) -> Vec<&str> {
    row.split(' ').collect()
}

fn zeros(token: &str) -> String {
    vec![token; N].join(" ")
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

/// The start town's map painted as in 6.3b (`gate_world` on its first package, tile 1 grass,
/// tile 2 water) and saved: the map and its package.
fn painted_map(store: &mut Store, code: &str) -> (u16, u16) {
    let map = start_map(store, code);
    let pack = *packs_of_map(store, map)
        .unwrap()
        .first()
        .expect("the start town has a map texture package");
    save_tileset(store, pack).unwrap();
    let tile = |tile: u16, role: TileRole, kind: u8| TileChange {
        tile,
        role: Some(role),
        kind: Some(kind),
        collision: None,
        sound: None,
        wall: None,
        model: None,
    };
    tileset_edit(
        store,
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
    world_create(store, &create).unwrap();
    let want = gate_world(pack, "0000", "0001", "0002");
    let created = store.world(map).unwrap().unwrap().clone();
    let edit: WorldEditRequest = serde_json::from_value(serde_json::json!({
        "map": map,
        "changes": changes_between(&created, &want),
    }))
    .unwrap();
    let bake = world_edit(store, &edit)
        .unwrap()
        .bake
        .expect("the stroke bakes");
    assert!(bake.ok, "{:#?}", bake.messages);
    store.save().unwrap();
    (map, pack)
}

/// The sample models written to `<copy>/import/`; the path of the cube.
fn cube(copy: &Path) -> PathBuf {
    let dir = copy.join("import");
    write_samples(&dir).unwrap();
    dir.join(CUBE_OBJ)
}

fn pack_bytes(store: &Store, pack: u16) -> Vec<u8> {
    store
        .project()
        .map_tileset_member(pack)
        .unwrap()
        .expect("the package member")
}

/// The baked faces of world map `map` as the baker's input.
fn baked_faces(store: &mut Store, map: u16) -> Vec<Face> {
    world_faces(store, map)
        .unwrap()
        .expect("a world map")
        .faces
        .into_iter()
        .map(Face::from)
        .collect()
}

#[test]
fn freeform_on_a_painted_map_bakes_and_apicula_reads_it() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let (map, pack) = painted_map(&mut store, &code);
    let report = world_freeform_add(
        &mut store,
        &FreeformAddRequest {
            map,
            path: cube(&copy),
            at: [0, 0, 0],
            options: ImportOptions {
                unit: 32.0,
                ..ImportOptions::default()
            },
            name: None,
        },
    )
    .unwrap();
    assert!(report.change.is_some());
    let bake = report.bake.expect("the add bakes");
    assert!(bake.ok, "{:#?}", bake.messages);
    let model = store.map(map).unwrap().model.clone();
    assert!(model.len() <= MODEL_MAX, "model {} bytes", model.len());
    let faces = baked_faces(&mut store, map);
    let freeform: Vec<Face> = faces
        .iter()
        .filter(|f| f.layer == FREEFORM_LAYER)
        .cloned()
        .collect();
    assert_eq!(freeform.len(), 6, "the cube's faces on layer 254");
    assert_eq!(bake.freeform_faces, 6);
    assert_eq!(bake.faces, faces.len());
    let apicula = match apicula() {
        None => "skipped",
        Some(apicula) => {
            let dir = tempfile::tempdir().unwrap();
            let pack_bytes = pack_bytes(&store, pack);
            // Exit 0 and exactly one GLB, else an error.
            let glb_path = nsbmd_to_glb(&apicula, &model, Some(&pack_bytes), dir.path()).unwrap();
            let glb = std::fs::read(&glb_path).unwrap();
            let (json, bin) = glb_chunks(&glb);
            let got = glb_set(&json, &bin);
            let want = input_set(&freeform);
            let missing: Vec<_> = want.difference(&got).take(5).collect();
            assert!(missing.is_empty(), "missing in the GLB: {missing:?}");
            "ok"
        }
    };
    eprintln!(
        "world-freeform {code} map={map} faces={} freeform_faces={} nsbmd={} share={:.3} \
         apicula={apicula}",
        bake.faces,
        bake.freeform_faces,
        model.len(),
        bake.model_share
    );
}

#[test]
fn piece_model_on_the_copy() {
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let (map, pack) = painted_map(&mut store, &code);
    // gate_world paints tiles 0 to 2: tile 3 is free.
    let tile = 3u16;
    let count = store.tileset(pack).unwrap().tiles.len();
    assert!(
        count > usize::from(tile),
        "package {pack} has {count} tiles"
    );
    let report = tileset_piece_set(
        &mut store,
        &PieceSetRequest {
            package: pack,
            tile,
            path: cube(&copy),
            footprint: [1, 1],
            options: ImportOptions::default(),
            name: None,
        },
    )
    .unwrap();
    assert_eq!(report.stats.quads, 6);
    let before = store
        .world_bake_report(map)
        .unwrap()
        .expect("the painted map bakes");
    let (bake, change) = store
        .transaction("Stück gesetzt", |tx| {
            tx.edit_world(map, |w| {
                let w = w.as_mut().expect("a world");
                w.pieces.push(PieceJson {
                    tile,
                    at: [24, 14],
                    rotate: 0,
                });
                Ok(())
            })?;
            Ok(tx.bake_report(map))
        })
        .unwrap();
    assert!(change.is_some());
    let bake = bake.expect("placing the piece bakes");
    assert!(bake.ok, "{:#?}", bake.messages);
    assert!(
        bake.textures.iter().any(|t| t == "g4_imp_cube"),
        "{:?}",
        bake.textures
    );
    let model = store.map(map).unwrap().model.clone();
    assert!(model.len() <= MODEL_MAX, "model {} bytes", model.len());
    assert!(bake.faces > before.faces, "the piece adds faces");
    eprintln!(
        "world-piece {code} pack={pack} tile={tile} faces={} nsbmd={}",
        bake.faces,
        model.len()
    );
}

fn gltf_request(map: u16) -> WorldAdoptRequest {
    WorldAdoptRequest {
        map,
        tileset: None,
        form: AdoptForm::Gltf,
    }
}

#[test]
fn form2_of_a_vanilla_map_bakes_apicula_readable() {
    // The project variable first, and only the variable (a test that needs a project is logged as
    // one, it runs per project in the full run of the gate, whichever of the two is missing); then
    // the tool; the copy and the export of the project, which are the work, only when both are there.
    if common::vanilla_project().is_none() {
        return;
    }
    let Some(apicula) = apicula() else {
        return;
    };
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let mut store = store_of(&copy);
    let map = start_map(&mut store, &code);
    let family = store.project().family;
    let original = {
        let project = Project::open(&copy).unwrap();
        let bytes = std::fs::read(project.member_path(AssetKind::Map, map)).unwrap();
        MapFile::read(&bytes, family).unwrap()
    };
    let report = world_adopt_with(&mut store, &gltf_request(map), Some(&apicula)).unwrap();
    assert!(report.bake.ok, "{:#?}", report.bake.messages);
    let form2 = report.form2.expect("Form 2 reports its figures");
    let file = store.map(map).unwrap().clone();
    assert_eq!(file.bdhc, original.bdhc, "BDHC of the original");
    assert_eq!(
        file.permissions, original.permissions,
        "permissions of the original"
    );
    assert!(file.model.len() <= MODEL_MAX);

    let pack = report.tileset;
    let pack_bytes = pack_bytes(&store, pack);
    let dir = tempfile::tempdir().unwrap();
    let glb_path = nsbmd_to_glb(&apicula, &file.model, Some(&pack_bytes), dir.path()).unwrap();
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
    let faces = baked_faces(&mut store, map);
    assert_eq!(faces.len(), form2.baked.faces);
    let got = glb_wrapped(&json, &bin, size);
    let want = input_wrapped(&faces, size);
    let missing: Vec<_> = want.difference(&got).take(5).collect();
    let extra: Vec<_> = got.difference(&want).take(5).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing {missing:?}, extra {extra:?}"
    );
    eprintln!(
        "world-form2 {code} map={map} vanilla_polys={} vanilla_quads={} vanilla_tris={} \
         baked_faces={} baked_quads={} baked_tris={} strips={} dropped={} vanilla_bytes={} \
         baked_bytes={} share={:.3} ignored={}",
        form2.vanilla.polygons,
        form2.vanilla.quads,
        form2.vanilla.triangles,
        form2.baked.faces,
        form2.baked.quads,
        form2.baked.triangles,
        form2.baked.strips,
        form2.baked.dropped,
        form2.vanilla.bytes,
        form2.baked.bytes,
        report.bake.model_share,
        form2.ignored.join(",")
    );
}

/// The code in the parentheses that end a refusal (`… (bake.model-too-big)`).
fn refusal_code(e: &KernelError) -> String {
    let text = match e {
        KernelError::Refused(why) => why.clone(),
        other => other.to_string(),
    };
    text.rsplit_once('(')
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or(&text)
        .to_owned()
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

#[test]
fn form2_sweep_over_every_map() {
    if std::env::var_os("G4_FORM2_SWEEP").is_none_or(|v| v != "1") {
        eprintln!("skipped: needs G4_FORM2_SWEEP=1");
        return;
    }
    // The project variable, then the tool, then the copy: see `form2_of_a_vanilla_map_bakes_apicula_readable`.
    if common::vanilla_project().is_none() {
        return;
    }
    let Some(apicula) = apicula() else {
        return;
    };
    let Some((_tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let maps = Project::open(&copy)
        .unwrap()
        .member_count(AssetKind::Map)
        .unwrap();
    let started = Instant::now();
    let (mut ok, mut too_big, mut texture, mut other) = (0, 0, 0, 0);
    // Texture refusals of maps no header shows (package 0 guessed).
    let mut texture_guessed = 0;
    // Maps that baked with untextured faces left out, and on a package guessed by fit.
    let (mut with_untextured, mut untextured_faces, mut ok_guessed) = (0, 0, 0);
    let mut ratios = Vec::new();
    let mut others: Vec<(u16, String)> = Vec::new();
    for map in 0..u16::try_from(maps).unwrap() {
        let mut store = store_of(&copy);
        match world_adopt_with(&mut store, &gltf_request(map), Some(&apicula)) {
            Ok(report) if report.bake.ok => {
                ok += 1;
                ok_guessed += usize::from(report.tileset_guessed);
                if let Some(form2) = report.form2 {
                    if form2.baked.untextured > 0 {
                        with_untextured += 1;
                        untextured_faces += form2.baked.untextured;
                    }
                    if form2.vanilla.polygons > 0 {
                        ratios.push(form2.baked.faces as f64 / f64::from(form2.vanilla.polygons));
                    }
                }
            }
            Ok(report) => {
                other += 1;
                others.push((map, format!("{:?}", report.bake.messages)));
            }
            Err(e) => match refusal_code(&e).as_str() {
                "bake.model-too-big" => too_big += 1,
                ADOPT_TEXTURE_CODE => {
                    texture += 1;
                    let packs = packs_of_map(&mut store, map).unwrap_or_default();
                    if packs.is_empty() {
                        texture_guessed += 1;
                    }
                    if texture <= 5 {
                        eprintln!("form2 texture: map {map:04} (packages {packs:?}): {e}");
                    }
                }
                _ => {
                    other += 1;
                    others.push((map, e.to_string()));
                }
            },
        }
    }
    for (map, why) in &others {
        eprintln!("form2 other: map {map:04}: {why}");
    }
    eprintln!("form2 texture refusals on a guessed package: {texture_guessed} of {texture}");
    eprintln!(
        "form2 ok with untextured polygons left out: {with_untextured} maps, \
         {untextured_faces} polygons; ok on a guessed package: {ok_guessed}"
    );
    eprintln!(
        "world-form2-sweep {code} maps={maps} ok={ok} too_big={too_big} texture={texture} \
         other={other} polys_ratio_median={:.3} ms={}",
        median(&mut ratios),
        ms(started)
    );
    assert_eq!(ok + too_big + texture + other, maps);
}

#[test]
fn tmj_round_trip_of_the_painted_map_and_every_adopted_map() {
    let Some((tmp, copy, base)) = exported_copy() else {
        return;
    };
    let code = base.game_code().to_owned();
    let started = Instant::now();
    let mut store = store_of(&copy);

    // The painted map: export, import, save → the file is byte-equal.
    let (map, _pack) = painted_map(&mut store, &code);
    let file = copy.join(json_rel(map));
    let before = std::fs::read(&file).unwrap();
    let painted_dir = tmp.path().join("tmj-painted");
    std::fs::create_dir_all(&painted_dir).unwrap();
    let tmj = painted_dir.join(format!("{map:04}.tmj"));
    world_export_tmj(&mut store, map, &tmj).unwrap();
    let imported = world_import_tmj(&mut store, map, &tmj).unwrap();
    assert!(imported.errors.is_empty(), "{:?}", imported.errors);
    assert_eq!(imported.change, None, "the import changes nothing");
    store.save().unwrap();
    assert!(
        std::fs::read(&file).unwrap() == before,
        "the painted map's file changed"
    );

    // Every map in Form 1: export, import → the same world.
    let project = Project::open(&copy).unwrap();
    let maps = project.member_count(AssetKind::Map).unwrap();
    let (mut adopted, mut equal) = (0, 0);
    let mut refused: Vec<(u16, String)> = Vec::new();
    let mut differing: Vec<(u16, String)> = Vec::new();
    for m in 0..u16::try_from(maps).unwrap() {
        if m == map {
            continue;
        }
        let request = WorldAdoptRequest {
            map: m,
            tileset: None,
            form: AdoptForm::Verbatim,
        };
        if let Err(e) = world_adopt(&mut store, &request) {
            refused.push((m, e.to_string()));
            continue;
        }
        adopted += 1;
        let world = store.world(m).unwrap().unwrap().clone();
        let dir = tmp.path().join(format!("tmj-{m:04}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{m:04}.tmj"));
        let outcome = world_export_tmj(&mut store, m, &path)
            .and_then(|_| world_import_tmj(&mut store, m, &path));
        match outcome {
            Ok(report) if report.change.is_none() && report.errors.is_empty() => {
                if store.world(m).unwrap() == Some(&world) {
                    equal += 1;
                } else {
                    differing.push((m, "world differs".to_owned()));
                }
            }
            Ok(report) => differing.push((
                m,
                format!("change {:?}, errors {:?}", report.change, report.errors),
            )),
            Err(e) => differing.push((m, e.to_string())),
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    eprintln!(
        "world-tmj {code} painted=equal maps={adopted} equal={equal} ms={}",
        ms(started)
    );
    assert!(
        refused.is_empty(),
        "refused: {:?}",
        &refused[..refused.len().min(5)]
    );
    assert!(
        differing.is_empty(),
        "{} differ: {:?}",
        differing.len(),
        &differing[..differing.len().min(5)]
    );
    assert_eq!(adopted, maps - 1);
}
