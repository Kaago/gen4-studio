//! The TMJ import (6.5b Task 7): the round trip world → TMJ → world, the saved file byte-equal
//! after an unchanged import, one step for a change, the files Tiled writes (extra fields,
//! Base64, new layers) and every refusal with its path.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use proptest::prelude::*;
use serde_json::{Value, json};

use super::*;
use crate::store::Store;
use crate::test_support::{TextFixture, world_ready};
use crate::textproject::Part;
use crate::world::doc::fixtures::{checkerboard_world, gate_world, one_tile_world, tmj_world};
use crate::world::doc::{
    FreeformJson, LayerJson, PermissionsJson, PieceJson, TerrainJson, WORLD_VERSION,
};
use crate::world::edit::{WorldCreateRequest, world_create};
use crate::world::grid::{TileRef, override_token, ramp_token, tile_token};

const MAP_FILE: &str = "data/maps/0000.json";

/// The TMJ of `world` (map 0, no buildings, synthetic tiles) as JSON.
fn tmj_of(world: &WorldJson) -> Value {
    let (tmj, _) = to_tmj(0, world, &[], &Tileset::synthetic(), &|_| None, "0000").unwrap();
    serde_json::from_slice(&to_json(&tmj).unwrap()).unwrap()
}

/// `value` read onto `target` with the synthetic tiles.
fn read(value: &Value, target: &WorldJson) -> Result<WorldJson, Vec<TmjError>> {
    from_tmj(
        &serde_json::to_vec(value).unwrap(),
        target,
        &Tileset::synthetic(),
    )
}

/// The errors of `value` read onto `target`, as (path, message).
fn errors(value: &Value, target: &WorldJson) -> Vec<(String, String)> {
    read(value, target)
        .expect_err("the file has errors")
        .into_iter()
        .map(|e| (e.path, e.message))
        .collect()
}

fn has(errors: &[(String, String)], path: &str, message: &str) -> bool {
    errors.iter().any(|(p, m)| p == path && m == message)
}

/// The index of layer `name` in `tmj`.
fn layer_at(tmj: &Value, name: &str) -> usize {
    tmj["layers"]
        .as_array()
        .unwrap()
        .iter()
        .position(|l| l["name"] == name)
        .unwrap_or_else(|| panic!("no layer {name}"))
}

fn cell(x: usize, z: usize) -> usize {
    z * MAP_SIZE + x
}

#[test]
fn round_trip_is_identity() {
    for (name, world) in [
        ("tmj_world(true)", tmj_world(true)),
        ("tmj_world(false)", tmj_world(false)),
        ("one_tile_world", one_tile_world()),
        ("checkerboard_world", checkerboard_world()),
    ] {
        let again = read(&tmj_of(&world), &world);
        assert_eq!(again.as_ref(), Ok(&world), "{name}");
    }
}

fn a_tile() -> impl Strategy<Value = String> {
    prop_oneof![
        1 => Just(".".to_owned()),
        6 => (0u16..6, 0u8..4, any::<bool>())
            .prop_map(|(tile, rotate, flip)| tile_token(Some(TileRef { tile, rotate, flip }))),
    ]
}

fn rows(tokens: Vec<String>) -> Vec<String> {
    tokens.chunks(MAP_SIZE).map(|r| r.join(" ")).collect()
}

fn a_ramp() -> impl Strategy<Value = Ramp> {
    prop_oneof![
        4 => Just(Ramp::None),
        1 => (1u8..3).prop_map(Ramp::North),
        1 => (1u8..3).prop_map(Ramp::East),
        1 => (1u8..3).prop_map(Ramp::South),
        1 => (1u8..3).prop_map(Ramp::West),
    ]
}

fn steps(r: Ramp) -> i16 {
    match r {
        Ramp::None => 0,
        Ramp::North(s) | Ramp::East(s) | Ramp::South(s) | Ramp::West(s) => i16::from(s),
    }
}

fn an_override(open: bool) -> BoxedStrategy<String> {
    let set = (any::<u8>(), any::<u8>()).prop_map(|(kind, collision)| {
        override_token(Some(g4_formats::map::Cell { kind, collision }))
    });
    if open {
        prop_oneof![2 => Just(".".to_owned()), 1 => set].boxed()
    } else {
        set.boxed()
    }
}

/// A canonical world on the synthetic tiles: 1–3 layers, all turns, heights, ramps or none,
/// up to three pieces, overrides or none, `derive` both ways (without it every cell has one).
fn a_world() -> impl Strategy<Value = WorldJson> {
    (
        proptest::collection::vec(proptest::collection::vec(a_tile(), 1024), 1..=3),
        proptest::collection::vec(-16i8..=62, 1024),
        proptest::option::of(proptest::collection::vec(a_ramp(), 1024)),
        proptest::collection::vec((0u8..31, 0u8..31, 0u16..4), 0..=3),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_flat_map(|(layers, heights, ramps, pieces, derive, with_overrides)| {
            let overrides = if derive && !with_overrides {
                Just(None).boxed()
            } else {
                proptest::collection::vec(an_override(derive), 1024)
                    .prop_map(Some)
                    .boxed()
            };
            (Just((layers, heights, ramps, pieces, derive)), overrides)
        })
        .prop_map(|((layers, heights, ramps, pieces, derive), overrides)| {
            let ramps = ramps.map(|ramps| {
                rows(
                    ramps
                        .into_iter()
                        .zip(&heights)
                        .map(|(r, &h)| {
                            ramp_token(if i16::from(h) + steps(r) > 62 {
                                Ramp::None
                            } else {
                                r
                            })
                        })
                        .collect(),
                )
            });
            let heights = rows(heights.iter().map(ToString::to_string).collect());
            let layers = layers
                .into_iter()
                .enumerate()
                .map(|(i, tiles)| LayerJson {
                    name: if i == 0 {
                        "Boden".to_owned()
                    } else {
                        format!("Ebene {i}")
                    },
                    role: if i == 0 {
                        LayerRole::Ground
                    } else {
                        LayerRole::Overlay
                    },
                    tiles: rows(tiles),
                    heights: (i == 0).then(|| heights.clone()),
                    ramps: if i == 0 { ramps.clone() } else { None },
                })
                .collect();
            WorldJson {
                version: WORLD_VERSION,
                tileset: 0,
                layers,
                pieces: pieces
                    .into_iter()
                    .map(|(x, z, r)| PieceJson {
                        tile: 5,
                        at: [x, z],
                        rotate: r * 90,
                    })
                    .collect(),
                freeform: Vec::new(),
                terrain: None,
                permissions: PermissionsJson {
                    derive,
                    overrides: overrides.map(rows),
                },
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn prop_round_trip_is_identity(world in a_world()) {
        prop_assert!(WorldGrid::parse(&world).is_ok());
        let again = read(&tmj_of(&world), &world);
        prop_assert_eq!(again.as_ref(), Ok(&world));
    }
}

/// A text project with the synthetic package and world map 0 painted as the gate map, saved.
fn painted() -> (TextFixture, Store) {
    let fx = world_ready();
    let mut store = Store::new(Arc::new(fx.project()));
    let req: WorldCreateRequest = serde_json::from_value(json!({"map": 0, "tileset": 0})).unwrap();
    world_create(&mut store, &req).unwrap();
    store
        .transaction("Gemalt", |tx| {
            tx.edit_world(0, |w| {
                *w = Some(gate_world(0, "0000", "0001", "0002"));
                Ok(())
            })
        })
        .unwrap();
    store.save().unwrap();
    (fx, store)
}

/// Exports world map 0 to `<dir>/0000.tmj`.
fn export_to(store: &mut Store, dir: &Path) -> PathBuf {
    let path = dir.join("0000.tmj");
    world_export_tmj(store, 0, &path).unwrap();
    path
}

fn edit_file(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    edit(&mut v);
    std::fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
}

#[test]
fn import_of_an_unchanged_export_is_byte_equal_on_disk() {
    let (fx, mut store) = painted();
    let a = fx.read(MAP_FILE);
    let out = tempfile::tempdir().unwrap();
    let path = export_to(&mut store, out.path());
    let undo = store.history().0.len();
    let report = world_import_tmj(&mut store, 0, &path).unwrap();
    assert_eq!(report.change, None);
    assert_eq!((report.map, report.layers, report.cells), (0, 1, 0));
    assert!(report.errors.is_empty());
    assert_eq!(store.history().0.len(), undo);
    store.save().unwrap();
    let b = fx.read(MAP_FILE);
    assert_eq!(a, b);
}

#[test]
fn an_edited_cell_imports_as_one_step() {
    let (fx, mut store) = painted();
    let a = fx.read(MAP_FILE);
    let out = tempfile::tempdir().unwrap();
    let path = export_to(&mut store, out.path());
    // Cell (5, 6) of „Boden“ becomes water (GID 3).
    edit_file(&path, |v| v["layers"][0]["data"][cell(5, 6)] = json!(3));
    let undo = store.history().0.len();
    let report = world_import_tmj(&mut store, 0, &path).unwrap();
    assert!(report.change.is_some());
    assert_eq!(report.cells, 1);
    assert!(report.bake.is_some());
    let (history, _) = store.history();
    assert_eq!(history.len(), undo + 1);
    assert_eq!(history.last().unwrap(), &import_label(0));
    assert_eq!(import_label(123), "Tiled imported: Map 0123");
    let world = store.world(0).unwrap().cloned().unwrap();
    let grid = WorldGrid::parse(&world).unwrap();
    let before = WorldGrid::parse(&gate_world(0, "0000", "0001", "0002")).unwrap();
    let changed: Vec<usize> = (0..1024)
        .filter(|&c| grid.layers[0].tiles[c] != before.layers[0].tiles[c])
        .collect();
    assert_eq!(changed, [cell(5, 6)]);
    assert_eq!(grid.layers[0].tiles[cell(5, 6)].map(|t| t.tile), Some(2));
    store.save().unwrap();
    assert_ne!(fx.read(MAP_FILE), a);
    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(fx.read(MAP_FILE), a);
}

#[test]
fn a_tmj_with_tiled_extras_reads() {
    let world = tmj_world(true);
    let mut v = tmj_of(&world);
    v["class"] = json!("");
    v["backgroundcolor"] = json!("#202020");
    v["parallaxoriginx"] = json!(0);
    v["editorsettings"] = json!({"export": {"format": "json"}});
    for layer in v["layers"].as_array_mut().unwrap() {
        layer["class"] = json!("");
        layer["parallaxx"] = json!(1.0);
        layer["parallaxy"] = json!(1.0);
        layer["tintcolor"] = json!("#ffffff");
        layer["locked"] = json!(true);
        layer["offsetx"] = json!(0);
        if let Some(objects) = layer["objects"].as_array_mut() {
            for object in objects {
                object["class"] = json!("");
                object["ellipse"] = json!(false);
            }
        }
        if let Some(props) = layer["properties"].as_array_mut() {
            for p in props {
                p["propertytype"] = json!("");
            }
        }
    }
    for set in v["tilesets"].as_array_mut().unwrap() {
        set["objectalignment"] = json!("unspecified");
        set["tileoffset"] = json!({"x": 0, "y": 0});
        set["transformations"] = json!({"hflip": true, "vflip": true, "rotate": true});
        set["wangsets"] = json!([]);
    }
    assert_eq!(read(&v, &world), Ok(world));
}

#[test]
fn base64_reads_and_compression_is_refused() {
    let world = tmj_world(true);
    let mut v = tmj_of(&world);
    let gids: Vec<u32> = serde_json::from_value(v["layers"][1]["data"].clone()).unwrap();
    let bytes: Vec<u8> = gids.iter().flat_map(|g| g.to_le_bytes()).collect();
    v["layers"][1]["data"] = json!(base64::engine::general_purpose::STANDARD.encode(&bytes));
    v["layers"][1]["encoding"] = json!("base64");
    assert_eq!(read(&v, &world), Ok(world.clone()));
    v["layers"][1]["compression"] = json!("zlib");
    let e = errors(&v, &world);
    assert!(
        has(
            &e,
            "layers[1].compression",
            "The Studio does not read compressed layers; in Tiled, choose CSV under Map › Properties “Tile Layer Format”."
        ),
        "{e:?}"
    );
    // An empty `compression` is none (Tiled writes "" for CSV and plain Base64).
    v["layers"][1]["compression"] = json!("");
    assert_eq!(read(&v, &world), Ok(world));
}

#[test]
fn an_external_tileset_is_refused() {
    let world = one_tile_world();
    let mut v = tmj_of(&world);
    v["tilesets"][1] = json!({"firstgid": 7, "source": "hoehen.tsj"});
    let e = errors(&v, &world);
    assert!(
        has(
            &e,
            "tilesets[1].source",
            "The Studio does not read external tilesets (`source`); embed the tileset in Tiled."
        ),
        "{e:?}"
    );
}

#[test]
fn a_new_layer_without_properties_is_an_overlay() {
    let world = one_tile_world();
    let mut v = tmj_of(&world);
    let mut data = vec![0u32; 1024];
    data[cell(4, 9)] = 3 | FLIP_H;
    v["layers"].as_array_mut().unwrap().push(json!({
        "data": data, "height": 32, "id": 9, "name": "Neu", "opacity": 1,
        "type": "tilelayer", "visible": true, "width": 32, "x": 0, "y": 0
    }));
    let again = read(&v, &world).unwrap();
    assert_eq!(again.layers.len(), 2);
    let new = &again.layers[1];
    assert_eq!((new.name.as_str(), new.role), ("Neu", LayerRole::Overlay));
    assert_eq!((new.heights.as_ref(), new.ramps.as_ref()), (None, None));
    let grid = WorldGrid::parse(&again).unwrap();
    assert_eq!(
        grid.layers[1].tiles[cell(4, 9)],
        Some(TileRef {
            tile: 2,
            rotate: 0,
            flip: true
        })
    );
    assert_eq!(grid.layers[1].tiles.iter().flatten().count(), 1);
    // The first tile layer is the ground, whatever its properties say.
    let mut v = tmj_of(&world);
    v["layers"][0]["properties"] = json!([]);
    assert_eq!(read(&v, &world), Ok(world));
}

#[test]
fn an_object_off_the_grid_is_named() {
    let world = tmj_world(true);
    let v = tmj_of(&world);
    let pieces = layer_at(&v, PIECE_LAYER);
    let overrides = layer_at(&v, PERMISSION_LAYER);
    let mut off = v.clone();
    off["layers"][pieces]["objects"][1]["x"] = json!(387.5);
    let id = off["layers"][pieces]["objects"][1]["id"].as_u64().unwrap();
    let mut wrong = off.clone();
    wrong["layers"][overrides]["objects"][0]["y"] = json!(3);
    let oid = wrong["layers"][overrides]["objects"][0]["id"]
        .as_u64()
        .unwrap();
    let e = errors(&wrong, &world);
    assert!(
        has(
            &e,
            &format!("layers[{pieces}].objects[1]"),
            &format!("Object {id} is not on the grid.")
        ),
        "{e:?}"
    );
    assert!(
        has(
            &e,
            &format!("layers[{overrides}].objects[0]"),
            &format!("Object {oid} is not on the grid.")
        ),
        "{e:?}"
    );
    // A piece of a tile that is none.
    let mut grass = v.clone();
    grass["layers"][pieces]["objects"][0]["gid"] = json!(2);
    let id = grass["layers"][pieces]["objects"][0]["id"]
        .as_u64()
        .unwrap();
    let e = errors(&grass, &world);
    assert!(
        has(
            &e,
            &format!("layers[{pieces}].objects[0]"),
            &format!("Object {id}: Tile 0x0001 is not a piece.")
        ),
        "{e:?}"
    );
    // Two overrides on one cell.
    let mut twice = v.clone();
    let copy = twice["layers"][overrides]["objects"][0].clone();
    twice["layers"][overrides]["objects"]
        .as_array_mut()
        .unwrap()
        .push(copy);
    let e = errors(&twice, &world);
    assert!(
        has(
            &e,
            &format!("layers[{overrides}].objects[1]"),
            "Cell (12, 21) is overwritten twice."
        ),
        "{e:?}"
    );
}

#[test]
fn foreign_sizes_and_packages_are_refused() {
    let world = one_tile_world();
    let v = tmj_of(&world);
    let mut sizes = v.clone();
    sizes["width"] = json!(30);
    sizes["tilewidth"] = json!(32);
    sizes["infinite"] = json!(true);
    let e = errors(&sizes, &world);
    assert!(
        has(&e, "width", "The map is 30 × 32 tiles; a map has 32 × 32."),
        "{e:?}"
    );
    assert!(
        has(
            &e,
            "tilewidth",
            "Tiles are 32 × 16 pixels; the Studio writes 16 × 16."
        ),
        "{e:?}"
    );
    assert!(
        has(&e, "infinite", "The Studio does not read infinite maps."),
        "{e:?}"
    );
    // Another package than the world's.
    let mut on_six = one_tile_world();
    on_six.tileset = 6;
    let mut target = one_tile_world();
    target.tileset = 7;
    let e = errors(&tmj_of(&on_six), &target);
    assert!(
        has(
            &e,
            "properties",
            "The file uses package 0006, the world package 0007."
        ),
        "{e:?}"
    );
    // A tileset of another size.
    let mut count = v.clone();
    count["tilesets"][0]["tilecount"] = json!(5);
    let e = errors(&count, &world);
    assert!(
        has(
            &e,
            "tilesets[0].tilecount",
            "The tileset has 5 tiles, package 0000 has 6."
        ),
        "{e:?}"
    );
    // Not a map, not JSON.
    let mut set = v.clone();
    set["type"] = json!("tileset");
    assert_eq!(
        errors(&set, &world),
        [(
            "type".to_owned(),
            "No Tiled map (`type` is not “map”).".to_owned()
        )]
    );
    let e = errors(&json!({"name": "x"}), &world);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].1, "No Tiled map (`type` is not “map”).");
    let broken = from_tmj(b"{ nicht json", &world, &Tileset::synthetic()).unwrap_err();
    assert_eq!(broken.len(), 1);
    assert!(!broken[0].message.is_empty());
}

#[test]
fn gids_past_the_tileset_are_refused() {
    let world = busy_target();
    let v = tmj_of(&world);
    let heights = layer_at(&v, HEIGHT_LAYER);
    let mut bad = v.clone();
    // GID 7 is the first height tile: no tile of „Boden“.
    bad["layers"][0]["data"][0] = json!(7);
    bad["layers"][0]["data"][1] = json!(2 | FLIP_HEX);
    // GID 3 is a map tile: no height; 0 is no height at all.
    bad["layers"][heights]["data"][2] = json!(3);
    bad["layers"][heights]["data"][cell(1, 3)] = json!(0);
    // A GID past every tileset.
    bad["layers"][0]["data"][cell(0, 1)] = json!(500);
    let e = errors(&bad, &world);
    assert!(
        has(
            &e,
            "layers[0].data[0]",
            "GID 7 does not belong in this layer."
        ),
        "{e:?}"
    );
    assert!(
        has(&e, "layers[0].data[1]", "There is no hexagonal turn."),
        "{e:?}"
    );
    assert!(
        has(
            &e,
            &format!("layers[{heights}].data[2]"),
            "GID 3 does not belong in this layer."
        ),
        "{e:?}"
    );
    assert!(
        has(
            &e,
            &format!("layers[{heights}].data[{}]", cell(1, 3)),
            "Cell (1, 3) has no height."
        ),
        "{e:?}"
    );
    assert!(
        has(
            &e,
            "layers[0].data[32]",
            "GID 500 does not belong in this layer."
        ),
        "{e:?}"
    );
    // Without „Höhe“.
    let mut flat = v.clone();
    flat["layers"].as_array_mut().unwrap().remove(heights);
    let e = errors(&flat, &world);
    assert!(has(&e, "layers", "The layer “Höhe” is missing."), "{e:?}");
}

/// The gate world (ramps, overrides) for the GID tests.
fn busy_target() -> WorldJson {
    gate_world(0, "0000", "0001", "0002")
}

fn world_store() -> (TextFixture, Store) {
    let fx = world_ready();
    let store = Store::new(Arc::new(fx.project()));
    (fx, store)
}

#[test]
fn buildings_are_ignored_with_a_note() {
    let (_fx, mut store) = painted();
    let buildings = store.map(0).unwrap().buildings.clone();
    assert!(!buildings.is_empty(), "the fixture has buildings");
    let out = tempfile::tempdir().unwrap();
    let path = export_to(&mut store, out.path());
    let unchanged = world_inspect_tmj(&mut store, 0, &path).unwrap();
    assert!(unchanged.notes.is_empty(), "{:?}", unchanged.notes);
    let points = layer_at(
        &serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap(),
        BUILDING_LAYER,
    );
    edit_file(&path, |v| {
        let x = v["layers"][points]["objects"][0]["x"].as_f64().unwrap();
        v["layers"][points]["objects"][0]["x"] = json!(x + 40.0);
    });
    let report = world_import_tmj(&mut store, 0, &path).unwrap();
    assert_eq!(report.notes, [BUILDINGS_IGNORED_CODE]);
    assert_eq!(report.change, None);
    assert_eq!(store.map(0).unwrap().buildings, buildings);
    // Another model counts too.
    let path = export_to(&mut store, out.path());
    edit_file(&path, |v| {
        v["layers"][points]["objects"][0]["properties"][0]["value"] = json!(999);
    });
    let report = world_inspect_tmj(&mut store, 0, &path).unwrap();
    assert_eq!(report.notes, [BUILDINGS_IGNORED_CODE]);
}

#[test]
fn freeform_and_terrain_stay_from_the_target() {
    let mut form1 = one_tile_world();
    form1.freeform = vec![FreeformJson {
        glb: None,
        nsbmd: Some(Part::Hex {
            hex: "00".to_owned(),
        }),
        at: [0, 0, 0],
    }];
    form1.terrain = Some(TerrainJson {
        bdhc: Part::Hex {
            hex: "01".to_owned(),
        },
    });
    // The file knows neither: exported from the plain world, read onto the Form-1 world.
    let v = tmj_of(&one_tile_world());
    assert_eq!(read(&v, &form1), Ok(form1.clone()));
    // A painted tile keeps both.
    let mut tile = v.clone();
    tile["layers"][0]["data"][5] = json!(2);
    let again = read(&tile, &form1).unwrap();
    assert_eq!(again.freeform, form1.freeform);
    assert_eq!(again.terrain, form1.terrain);
    assert_ne!(again.layers, form1.layers);
    // A painted height bakes the BDHC from the grid, as a painted height in the studio does.
    let mut height = v.clone();
    let heights = layer_at(&v, HEIGHT_LAYER);
    let first = height["layers"][heights]["data"][5].as_u64().unwrap();
    height["layers"][heights]["data"][5] = json!(first + 1);
    let again = read(&height, &form1).unwrap();
    assert_eq!(again.freeform, form1.freeform);
    assert_eq!(again.terrain, None);
}

#[test]
fn import_refuses_a_binary_map() {
    let (_fx, mut store) = world_store();
    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("0000.tmj");
    std::fs::write(
        &path,
        serde_json::to_vec(&tmj_of(&one_tile_world())).unwrap(),
    )
    .unwrap();
    for result in [
        world_import_tmj(&mut store, 0, &path),
        world_inspect_tmj(&mut store, 0, &path),
    ] {
        match result {
            Err(KernelError::Refused(m)) => assert_eq!(m, not_a_world_map(0)),
            other => panic!("{other:?}"),
        }
    }
    assert!(store.history().0.is_empty());
}

#[test]
fn import_refuses_a_file_with_errors() {
    let (_fx, mut store) = painted();
    let out = tempfile::tempdir().unwrap();
    let path = export_to(&mut store, out.path());
    edit_file(&path, |v| {
        v["layers"][0]["data"][0] = json!(500);
        v["layers"][0]["data"][1] = json!(501);
    });
    let undo = store.history().0.len();
    match world_import_tmj(&mut store, 0, &path) {
        Err(KernelError::Refused(m)) => assert_eq!(
            m,
            "layers[0].data[0]: GID 500 does not belong in this layer. (2 errors)"
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(store.history().0.len(), undo);
    let report = world_inspect_tmj(&mut store, 0, &path).unwrap();
    assert_eq!(report.errors.len(), 2);
    assert_eq!(report.change, None);
}

#[test]
fn inspect_changes_nothing() {
    let (fx, mut store) = painted();
    let a = fx.read(MAP_FILE);
    let out = tempfile::tempdir().unwrap();
    let path = export_to(&mut store, out.path());
    edit_file(&path, |v| {
        v["layers"][0]["data"][cell(1, 1)] = json!(3);
        v["layers"][0]["data"][cell(2, 1)] = json!(3);
    });
    let world = store.world(0).unwrap().cloned();
    let history = store.history();
    let report = world_inspect_tmj(&mut store, 0, &path).unwrap();
    assert_eq!((report.layers, report.cells), (1, 2));
    assert!(report.errors.is_empty());
    assert_eq!(report.change, None);
    assert!(
        report.bake.as_ref().is_some_and(|b| b.ok),
        "{:?}",
        report.bake
    );
    assert_eq!(store.world(0).unwrap().cloned(), world);
    assert_eq!(store.history(), history);
    assert!(store.dirty().is_empty());
    store.save().unwrap();
    assert_eq!(fx.read(MAP_FILE), a);
}
