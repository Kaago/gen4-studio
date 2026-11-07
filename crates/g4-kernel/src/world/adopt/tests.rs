//! „Karte als Quelle übernehmen“ (Form 1): byte-equal bakes, sampled heights, parts as a save
//! writes them, refusals.

use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::narc::Narc;
use serde_json::{Value, json};

use super::*;
use crate::project::{AssetId, AssetKind};
use crate::terrain::{parse_grid, scenario_input, set_map_terrain};
use crate::test_support::{TextFixture, synthetic_map_bytes};
use crate::textproject::canon::to_canonical;
use crate::world::WORLD_NEEDS_TEXT_PROJECT;
use crate::world::edit::{WorldEditRequest, world_edit, world_to_binary};
use crate::world::grid::WorldGrid;
use crate::world::tileset::{Tileset, tileset_rel};

const JSON: &str = "data/maps/0000.json";

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// The synthetic tileset as package 0.
fn put_tileset(fx: &TextFixture) {
    fx.write(
        tileset_rel(0).to_str().unwrap(),
        &to_canonical(&Tileset::synthetic()),
    );
}

/// A text project whose map 0 has its own BDHC and permissions from `scenario_input()`
/// (saved: the BDHC is a side file `0000.bdhc`), with the synthetic tileset as package 0.
fn scenario_fixture() -> TextFixture {
    let fx = TextFixture::exported();
    put_tileset(&fx);
    let mut st = store(&fx);
    set_map_terrain(&mut st, 0, &scenario_input()).unwrap();
    st.save().unwrap();
    fx
}

fn req(value: Value) -> WorldAdoptRequest {
    serde_json::from_value(value).unwrap()
}

fn adopt(st: &mut Store, map: u16) -> WorldAdoptReport {
    world_adopt(st, &req(json!({"map": map}))).unwrap()
}

fn map_bytes(st: &mut Store, map: u16) -> Vec<u8> {
    st.map(map).unwrap().write(Family::Pt)
}

fn member(fx: &TextFixture, map: u16) -> Vec<u8> {
    std::fs::read(fx.project().member_path(AssetKind::Map, map)).unwrap()
}

fn file_json(fx: &TextFixture) -> Value {
    serde_json::from_slice(&fx.read(JSON)).unwrap()
}

fn refused<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
    match r {
        Err(KernelError::Refused(text)) => text,
        other => panic!("{other:?}"),
    }
}

/// The files of `data/maps/` whose stem is `0000` (JSON and side files), sorted.
fn map_files(fx: &TextFixture) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fx.root().join("data/maps"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("0000."))
        .collect();
    names.sort();
    names
}

#[test]
fn adopt_bakes_byte_equal() {
    let fx = scenario_fixture();
    let original = member(&fx, 0);
    let mut st = store(&fx);
    let before = map_bytes(&mut st, 0);
    assert_eq!(before, original);
    let report = adopt(&mut st, 0);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert!(report.bake.verbatim_model && report.bake.verbatim_terrain);
    assert!(!report.bake.model_kept);
    assert_eq!(report.bake.faces, 0);
    assert_eq!(map_bytes(&mut st, 0), before);
    st.save().unwrap();
    assert_eq!(member(&fx, 0), original);
    // A store opened anew bakes the map from its JSON: the original member.
    let mut again = store(&fx);
    assert_eq!(map_bytes(&mut again, 0), original);
    assert_eq!(again.world(0).unwrap(), st.world(0).unwrap());
    assert!(again.dirty().is_empty());
    // The file: a world with the verbatim parts and every permission a hand value.
    let v = file_json(&fx);
    assert_eq!(v["source"], "world");
    for derived in ["permissions", "model", "bdhc"] {
        assert!(v.get(derived).is_none(), "{derived}: {v}");
    }
    let world = &v["world"];
    assert_eq!(world["freeform"].as_array().unwrap().len(), 1);
    assert!(world["freeform"][0]["nsbmd"].is_object(), "{world}");
    assert_eq!(world["freeform"][0]["at"], json!([0, 0, 0]));
    assert_eq!(world["terrain"]["bdhc"], json!({"file": "0000.bdhc"}));
    let overrides = world["permissions"]["override"].as_array().unwrap();
    assert_eq!(overrides.len(), 32);
    for row in overrides {
        let row = row.as_str().unwrap();
        assert_eq!(row.split(' ').count(), 32);
        assert!(!row.split(' ').any(|t| t == "."), "{row}");
    }
    assert_eq!(world["permissions"]["derive"], true);
    let ground = &world["layers"][0];
    assert_eq!(ground["name"], "Boden");
    assert!(
        ground["tiles"].as_array().unwrap().iter().all(|r| r
            .as_str()
            .unwrap()
            .split(' ')
            .all(|t| t == "."))
    );
    assert!(ground.get("ramps").is_none());
    // A second save writes nothing.
    let file = fx.read(JSON);
    assert!(st.save().unwrap().written.is_empty());
    assert_eq!(fx.read(JSON), file);
}

#[test]
fn adopt_samples_heights_and_lists_deviations() {
    let fx = scenario_fixture();
    let mut st = store(&fx);
    let report = adopt(&mut st, 0);
    let world = st.world(0).unwrap().cloned().unwrap();
    let grid = WorldGrid::parse(&world).unwrap();
    let at = |x: usize, z: usize| grid.heights[z * 32 + x];
    // The middle of the ramp `e1` at x 16 (4 units) takes step 0 and deviates.
    assert_eq!(at(16, 21), 0);
    let deviation = |x: u8, z: u8| report.deviations.iter().find(|d| (d.x, d.z) == (x, z));
    assert_eq!(
        deviation(16, 21),
        Some(&HeightDeviation {
            x: 16,
            z: 21,
            height: Some(4.0),
            step: 0
        })
    );
    // The plateau (16 units) is step 2 without deviation; so are the edge to 5 and the ground.
    assert_eq!((at(18, 21), at(22, 20), at(23, 22), at(0, 0)), (2, 2, 5, 0));
    for (x, z) in [(18, 21), (22, 20), (23, 22), (0, 0), (20, 26)] {
        assert_eq!(deviation(x, z), None, "({x}, {z})");
    }
    // Only the ramp cells deviate: x 16 and 17 in rows 20 to 22.
    let cells: Vec<(u8, u8)> = report.deviations.iter().map(|d| (d.x, d.z)).collect();
    assert_eq!(
        cells,
        [(16, 20), (17, 20), (16, 21), (17, 21), (16, 22), (17, 22)]
    );
    // A BDHC without plates: every cell deviates at step 0.
    let (steps, deviations) = sample_heights(&[0xCA, 0xFE, 0x01]);
    assert!(steps.iter().all(|&s| s == 0));
    assert_eq!(deviations.len(), 1024);
    assert_eq!(deviations[33].height, None);
    assert_eq!((deviations[33].x, deviations[33].z), (1, 1));
    // Steps: ties to even, bounded.
    assert_eq!(
        [4.0, 12.0, 15.95, -200.0, 900.0].map(step_of),
        [0, 2, 2, -16, 62]
    );
}

/// Map 0 of `seed` with a 100-byte model and the scenario's BDHC.
fn big_map() -> Vec<u8> {
    let mut m = MapFile::read(&synthetic_map_bytes(0), Family::Pt).unwrap();
    m.model = vec![0x5A; 100];
    m.bdhc = g4_formats::bdhc::from_grid(&parse_grid(&scenario_input().grid).unwrap()).write();
    m.write(Family::Pt)
}

#[test]
fn adopt_uses_base_sha1_when_the_part_is_vanilla() {
    let fx = TextFixture::with(|p| {
        let maps = vec![big_map(), synthetic_map_bytes(1), synthetic_map_bytes(2)];
        std::fs::write(
            p.root.join("files/fielddata/land_data/land_data.narc"),
            Narc { members: maps }.write(),
        )
        .unwrap();
    });
    crate::textproject::export(
        &fx.project(),
        &fx.base(),
        &crate::textproject::ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
    put_tileset(&fx);
    let binary = file_json(&fx);
    assert!(binary["model"].get("base_sha1").is_some(), "{binary}");
    let mut st = store(&fx);
    let original = member(&fx, 0);
    adopt(&mut st, 0);
    let world = st.world(0).unwrap().cloned().unwrap();
    assert_eq!(
        world.freeform[0].nsbmd.as_ref(),
        serde_json::from_value(binary["model"].clone())
            .ok()
            .as_ref()
    );
    assert_eq!(
        Some(&world.terrain.as_ref().unwrap().bdhc),
        serde_json::from_value::<Part>(binary["bdhc"].clone())
            .ok()
            .as_ref()
    );
    assert!(matches!(world.freeform[0].nsbmd, Some(Part::Base { .. })));
    assert!(matches!(
        world.terrain.as_ref().unwrap().bdhc,
        Part::Base { .. }
    ));
    st.save().unwrap();
    // No vanilla bytes as side files; the member stays.
    assert_eq!(map_files(&fx), ["0000.json"]);
    assert_eq!(member(&fx, 0), original);
    let v = file_json(&fx);
    assert_eq!(v["world"]["freeform"][0]["nsbmd"], binary["model"]);
    assert_eq!(v["world"]["terrain"]["bdhc"], binary["bdhc"]);
    assert_eq!(map_bytes(&mut store(&fx), 0), original);
}

#[test]
fn adopt_without_header_guesses_package_zero() {
    let fx = TextFixture::exported();
    put_tileset(&fx);
    let mut st = store(&fx);
    let original = map_bytes(&mut st, 2);
    // Map 2 lies in no matrix cell; its BDHC does not read.
    let report = adopt(&mut st, 2);
    assert_eq!(
        (report.map, report.tileset, report.tileset_guessed),
        (2, 0, true)
    );
    assert_eq!(report.deviations.len(), 1024);
    assert!(report.bake.ok, "{:?}", report.bake.messages);
    assert_eq!(report.bake.style, None);
    assert_eq!(map_bytes(&mut st, 2), original);
    st.save().unwrap();
    assert_eq!(member(&fx, 2), original);
    assert_eq!(map_bytes(&mut store(&fx), 2), original);
    // Map 0 lies in a cell with a header: its package, not guessed; a package asked for wins.
    let shown = adopt(&mut st, 0);
    assert!(!shown.tileset_guessed);
    let asked = world_adopt(&mut st, &req(json!({"map": 1, "tileset": 0}))).unwrap();
    assert_eq!((asked.tileset, asked.tileset_guessed), (0, false));
}

#[test]
fn adopt_is_one_undo_step() {
    let fx = scenario_fixture();
    let mut st = store(&fx);
    let before = map_bytes(&mut st, 0);
    let report = adopt(&mut st, 0);
    assert!(report.change.is_some());
    assert_eq!(st.history().0, ["Karte als Quelle übernommen: Karte 0000"]);
    let adopted = st.world(0).unwrap().cloned();
    assert!(adopted.is_some());
    st.undo().unwrap();
    assert_eq!(st.world(0).unwrap(), None);
    assert_eq!(map_bytes(&mut st, 0), before);
    assert!(st.history().0.is_empty());
    st.redo().unwrap();
    assert_eq!(st.world(0).unwrap().cloned(), adopted);
    assert_eq!(map_bytes(&mut st, 0), before);
    // Undone and saved: the binary map as it was.
    let file = fx.read(JSON);
    st.undo().unwrap();
    st.save().unwrap();
    assert_eq!(fx.read(JSON), file);
}

#[test]
fn adopt_refuses_dspre_world_locked_and_gltf() {
    let fx = scenario_fixture();
    let mut st = store(&fx);
    assert_eq!(
        refused(world_adopt(
            &mut st,
            &req(json!({"map": 0, "form": "gltf"}))
        )),
        "Form 2 (glTF) kommt mit 6.5b."
    );
    adopt(&mut st, 0);
    assert_eq!(
        refused(world_adopt(&mut st, &req(json!({"map": 0})))),
        "Karte 0000 ist schon eine Welt-Karte."
    );
    assert_eq!(st.history().0.len(), 1);
    // A locked map.
    st.lock(
        AssetId {
            kind: AssetKind::Map,
            index: 1,
        },
        "gesperrt",
    )
    .unwrap();
    assert_eq!(
        refused(world_adopt(&mut st, &req(json!({"map": 1})))),
        "textproject.map-source: Karte 0001 ist eine Welt-Karte, ihr Member lässt sich nicht \
         lesen; Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder die Welt aus \
         der Datei sichern)."
    );
    assert_eq!(st.world(1).unwrap(), None);
    assert_eq!(st.history().0.len(), 1);
    // The DSPRE layout.
    let dspre = TextFixture::dspre();
    let mut st = store(&dspre);
    assert_eq!(
        refused(world_adopt(&mut st, &req(json!({"map": 0})))),
        WORLD_NEEDS_TEXT_PROJECT
    );
    assert!(st.history().0.is_empty());
    // Unknown keys and forms are serde's.
    assert!(serde_json::from_value::<WorldAdoptRequest>(json!({"map": 0, "from": "x"})).is_err());
    assert!(serde_json::from_value::<WorldAdoptRequest>(json!({"map": 0, "form": "obj"})).is_err());
}

#[test]
fn painting_on_an_adopted_map_reports_verbatim_mixed() {
    let fx = scenario_fixture();
    let mut st = store(&fx);
    let before = map_bytes(&mut st, 0);
    adopt(&mut st, 0);
    let edit: WorldEditRequest =
        serde_json::from_value(json!({"map": 0, "changes": [{"cells": [[3, 3]], "tile": "0001"}]}))
            .unwrap();
    let report = world_edit(&mut st, &edit).unwrap();
    let bake = report.bake.unwrap();
    let codes: Vec<&str> = bake.messages.iter().map(|m| m.code.as_str()).collect();
    assert_eq!(codes, ["world.verbatim-mixed"]);
    assert!(!bake.ok && bake.verbatim_model);
    // The verbatim model and BDHC stay; the grass tile's derived permission is under the hand
    // value of the cell.
    let now = st.map(0).unwrap().clone();
    let was = MapFile::read(&before, Family::Pt).unwrap();
    assert_eq!((now.model, now.bdhc), (was.model, was.bdhc));
}

/// The carried check of Task 6: an adopted map whose parts are side files, then given up as
/// binary (`world_to_binary` + save, or `take_member_as_binary`), leaves no side file that the
/// map's JSON does not name.
#[test]
fn an_adopted_map_given_up_leaves_no_orphaned_side_files() {
    let referenced = |fx: &TextFixture| {
        let v = file_json(fx);
        let mut named = vec!["0000.json".to_owned()];
        for part in [
            &v["model"],
            &v["bdhc"],
            &v["world"]["freeform"][0]["nsbmd"],
            &v["world"]["terrain"]["bdhc"],
        ] {
            if let Some(file) = part.get("file").and_then(Value::as_str) {
                named.push(file.to_owned());
            }
        }
        named.sort();
        named
    };
    for take in [false, true] {
        let fx = scenario_fixture();
        fx.remove_layer();
        let original = member(&fx, 0);
        let mut st = store(&fx);
        adopt(&mut st, 0);
        st.save().unwrap();
        assert_eq!(map_files(&fx), referenced(&fx), "adopted");
        assert!(map_files(&fx).contains(&"0000.bdhc".to_owned()));
        if take {
            crate::world::build::take_member_as_binary(&fx.project(), 0).unwrap();
        } else {
            world_to_binary(&mut st, 0).unwrap();
            st.save().unwrap();
        }
        let v = file_json(&fx);
        assert_eq!(v["source"], "binary", "take {take}");
        assert_eq!(map_files(&fx), referenced(&fx), "take {take}");
        assert_eq!(member(&fx, 0), original, "take {take}");
        assert_eq!(map_bytes(&mut store(&fx), 0), original, "take {take}");
    }
}
