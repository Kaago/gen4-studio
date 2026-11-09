//! The TMJ export (6.5b Task 6): flip bits against the mesher's UV, the layers of the gate
//! world, the tilesets and their images, the file with its tiles folder.

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::store::Store;
use crate::test_support::world_ready;
use crate::world::doc::fixtures::{gate_world, one_tile_world};
use crate::world::doc::{PieceJson, TILE_UNITS};
use crate::world::edit::{WorldCreateRequest, world_create};
use crate::world::mesh::uv_of;

/// The table „Drehen und Spiegeln“: (rotate, flip, bits).
const TURNS: [(u8, bool, u32); 8] = [
    (0, false, 0),
    (1, false, FLIP_D | FLIP_H),
    (2, false, FLIP_H | FLIP_V),
    (3, false, FLIP_D | FLIP_V),
    (0, true, FLIP_H),
    (1, true, FLIP_D | FLIP_H | FLIP_V),
    (2, true, FLIP_V),
    (3, true, FLIP_D),
];

fn export(world: &WorldJson) -> (TmjMap, Vec<(String, Vec<u8>)>) {
    to_tmj(0, world, &[], &Tileset::synthetic(), &|_| None, "0000").unwrap()
}

fn layer<'a>(tmj: &'a TmjMap, name: &str) -> &'a TmjLayer {
    tmj.layers
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no layer {name}"))
}

fn names(tmj: &TmjMap) -> Vec<&str> {
    tmj.layers.iter().map(|l| l.name.as_str()).collect()
}

fn prop<'a>(props: &'a [TmjProperty], name: &str) -> &'a Value {
    &props
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no property {name}"))
        .value
}

/// The GID of cell (x, z) of a tile layer.
fn gid(layer: &TmjLayer, x: usize, z: usize) -> u32 {
    match &layer.data {
        Some(TmjData::Gids(gids)) => gids[z * MAP_SIZE + x],
        other => panic!("{}: data {other:?}", layer.name),
    }
}

fn objects(layer: &TmjLayer) -> &[TmjObject] {
    layer.objects.as_deref().unwrap()
}

fn decode(png: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
}

#[test]
fn flags_cover_the_eight_turns() {
    for (rotate, flip, bits) in TURNS {
        assert_eq!(gid_flags(rotate, flip), bits, "r{rotate} f {flip}");
        assert_eq!(flags_turn(bits), Some((rotate, flip)));
        assert_eq!(flags_turn(bits | 5), Some((rotate, flip)), "with a GID");
    }
    // Tiled shows image pixel (px, py) at the cell pixel it gets after D, then H, then V; the
    // mesher's UV must name the same texel there (pixel centres, half units doubled).
    let last = TILE_UNITS - 1;
    for (rotate, flip, bits) in TURNS {
        for (cx, cz) in [(0, 0), (5, 7), (17, 3), (31, 31)] {
            for px in 0..TILE_UNITS {
                for py in 0..TILE_UNITS {
                    let (mut a, mut b) = (px, py);
                    if bits & FLIP_D != 0 {
                        (a, b) = (b, a);
                    }
                    if bits & FLIP_H != 0 {
                        a = last - a;
                    }
                    if bits & FLIP_V != 0 {
                        b = last - b;
                    }
                    let edge = |c: i32| (c * TILE_UNITS - 256) * 2;
                    let [u, v] = uv_of(edge(cx) + 2 * a + 1, edge(cz) + 2 * b + 1, rotate, flip);
                    let texel = |t: i32| t.rem_euclid(2 * TILE_UNITS) / 2;
                    assert_eq!(
                        (texel(u), texel(v)),
                        (px, py),
                        "r{rotate} f {flip} cell ({cx}, {cz}) pixel ({px}, {py})"
                    );
                }
            }
        }
    }
}

#[test]
fn hex_flag_is_refused() {
    assert_eq!(flags_turn(FLIP_HEX), None);
    assert_eq!(flags_turn(FLIP_HEX | FLIP_H | 7), None);
    assert_eq!(flags_turn(FLIP_HEX | FLIP_D | FLIP_H | FLIP_V), None);
}

#[test]
fn the_gate_world_exports_its_layers() {
    let (tmj, _) = export(&gate_world(0, "0000", "0001", "0002"));
    assert_eq!(
        names(&tmj),
        [
            "Boden",
            "Höhe",
            "Rampen",
            "Teilmodelle",
            "Permissions",
            "Gebäude (nur Ansicht)"
        ]
    );
    let kinds: Vec<&str> = tmj.layers.iter().map(|l| l.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "tilelayer",
            "tilelayer",
            "tilelayer",
            "objectgroup",
            "objectgroup",
            "objectgroup"
        ]
    );
    let ids: Vec<u32> = tmj.layers.iter().map(|l| l.id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
    assert_eq!(tmj.nextlayerid, 7);
    // Tilesets: the 6 synthetic tiles from 1, the heights from 7, the ramps from 86.
    let sets: Vec<(&str, u32, u32)> = tmj
        .tilesets
        .iter()
        .map(|t| (t.name.as_str(), t.firstgid, t.tilecount))
        .collect();
    assert_eq!(
        sets,
        [
            ("g4 Paket 0000", 1, 6),
            ("g4 Höhen", 7, 79),
            ("g4 Rampen", 86, 8)
        ]
    );
    let ground = layer(&tmj, "Boden");
    assert_eq!(prop(&ground.properties, "g4.role"), "ground");
    assert_eq!((ground.width, ground.height), (Some(32), Some(32)));
    assert_eq!(gid(ground, 0, 0), 1);
    assert_eq!(gid(ground, 16, 3), 2, "grass");
    assert_eq!(gid(ground, 2, 24), 3, "water");
    let heights = layer(&tmj, "Höhe");
    assert_eq!(prop(&heights.properties, "g4.kind"), "heights");
    assert_eq!(gid(heights, 0, 0), 7 + 16);
    assert_eq!(gid(heights, 17, 21), 7 + 17);
    assert_eq!(gid(heights, 18, 20), 7 + 18, "height 2");
    let ramps = layer(&tmj, "Rampen");
    assert_eq!(prop(&ramps.properties, "g4.kind"), "ramps");
    for z in 0..32 {
        for x in 0..32 {
            let want = if (16..=17).contains(&x) && (20..=22).contains(&z) {
                86 + 2 // e1
            } else {
                0
            };
            assert_eq!(gid(ramps, x, z), want, "ramp ({x}, {z})");
        }
    }
    let pieces = layer(&tmj, "Teilmodelle");
    assert_eq!(prop(&pieces.properties, "g4.kind"), "pieces");
    assert!(objects(pieces).is_empty());
    let permissions = layer(&tmj, "Permissions");
    assert_eq!(prop(&permissions.properties, "g4.kind"), "overrides");
    let [door] = objects(permissions) else {
        panic!("one override: {:?}", permissions.objects);
    };
    assert_eq!(
        (door.x, door.y, door.width, door.height),
        (192.0, 336.0, 16.0, 16.0)
    );
    assert_eq!(door.gid, None);
    assert_eq!(prop(&door.properties, "g4.kind"), 0x69);
    assert_eq!(prop(&door.properties, "g4.collision"), 0x80);
    let buildings = layer(&tmj, "Gebäude (nur Ansicht)");
    assert_eq!(prop(&buildings.properties, "g4.kind"), "buildings");
    assert_eq!(tmj.nextobjectid, 2);
}

#[test]
fn turned_tiles_carry_their_flip_bits() {
    let mut w = one_tile_world();
    let mut row: Vec<String> = vec!["0000".to_owned(); 32];
    row[1] = "0001r1".to_owned();
    row[2] = "0002r3f".to_owned();
    row[3] = ".".to_owned();
    w.layers[0].tiles[4] = row.join(" ");
    let (tmj, _) = export(&w);
    let ground = layer(&tmj, "Boden");
    assert_eq!(gid(ground, 1, 4), 2 | FLIP_D | FLIP_H);
    assert_eq!(gid(ground, 2, 4), 3 | FLIP_D);
    assert_eq!(gid(ground, 3, 4), 0);
}

#[test]
fn no_ramps_no_ramp_layer() {
    let (tmj, _) = export(&one_tile_world());
    assert_eq!(
        names(&tmj),
        ["Boden", "Höhe", "Teilmodelle", "Gebäude (nur Ansicht)"]
    );
    // The helper tilesets stay, with the same first GIDs.
    let firsts: Vec<u32> = tmj.tilesets.iter().map(|t| t.firstgid).collect();
    assert_eq!(firsts, [1, 7, 86]);
}

#[test]
fn no_override_no_permission_layer() {
    let mut w = gate_world(0, "0000", "0001", "0002");
    w.permissions.overrides = None;
    let (tmj, _) = export(&w);
    assert_eq!(
        names(&tmj),
        [
            "Boden",
            "Höhe",
            "Rampen",
            "Teilmodelle",
            "Gebäude (nur Ansicht)"
        ]
    );
}

#[test]
fn map_properties_name_format_map_tileset_derive() {
    let mut w = one_tile_world();
    w.tileset = 7;
    let mut tiles = Tileset::synthetic();
    tiles.package = 7;
    let (tmj, _) = to_tmj(123, &w, &[], &tiles, &|_| None, "0123").unwrap();
    let props: Vec<(&str, &str, &Value)> = tmj
        .properties
        .iter()
        .map(|p| (p.name.as_str(), p.kind.as_str(), &p.value))
        .collect();
    assert_eq!(
        props,
        [
            ("g4.format", "string", &json!("gen4-studio-world")),
            ("g4.version", "int", &json!(1)),
            ("g4.map", "int", &json!(123)),
            ("g4.tileset", "int", &json!(7)),
            ("g4.derive", "bool", &json!(true)),
        ]
    );
    let bytes = to_json(&tmj).unwrap();
    assert!(bytes.ends_with(b"}\n"));
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "compressionlevel",
            "height",
            "infinite",
            "layers",
            "nextlayerid",
            "nextobjectid",
            "orientation",
            "properties",
            "renderorder",
            "tiledversion",
            "tileheight",
            "tilesets",
            "tilewidth",
            "type",
            "version",
            "width"
        ]
    );
    assert_eq!(v["version"], "1.10");
    assert_eq!(v["tiledversion"], "1.10.2");
    assert_eq!(v["type"], "map");
    assert_eq!(v["orientation"], "orthogonal");
    assert_eq!(v["renderorder"], "right-down");
    assert_eq!(
        (v["width"].clone(), v["height"].clone()),
        (json!(32), json!(32))
    );
    assert_eq!(
        (v["tilewidth"].clone(), v["tileheight"].clone()),
        (json!(16), json!(16))
    );
    assert_eq!(v["infinite"], false);
    assert_eq!(v["compressionlevel"], -1);
    // What the studio writes reads back as the same map.
    let back: TmjMap = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, tmj);
}

#[test]
fn images_are_png_with_placeholders() {
    let (tmj, images) = export(&one_tile_world());
    assert_eq!(images.len(), 6 + 79 + 8);
    for (path, png) in &images {
        assert!(path.starts_with("0000.tiles/"), "{path}");
        let img = decode(png);
        assert_eq!(img.dimensions(), (16, 16), "{path}");
    }
    let paths: Vec<&str> = images.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths[0], "0000.tiles/tile-0000.png");
    assert_eq!(paths[5], "0000.tiles/tile-0005.png");
    assert_eq!(paths[6], "0000.tiles/height-m016.png");
    assert_eq!(paths[6 + 16], "0000.tiles/height-p000.png");
    assert_eq!(paths[6 + 28], "0000.tiles/height-p012.png");
    assert_eq!(paths[6 + 78], "0000.tiles/height-p062.png");
    assert_eq!(paths[85], "0000.tiles/ramp-n1.png");
    assert_eq!(paths[92], "0000.tiles/ramp-w2.png");
    // The placeholder of a tile is filled with the colour of its role (water: 0x0073B5).
    let water = decode(&images[2].1);
    assert!(water.pixels().all(|p| p.0 == [0x00, 0x73, 0xB5, 0xFF]));
    let grass = decode(&images[1].1);
    assert!(grass.pixels().all(|p| p.0 == [0x00, 0x9C, 0x73, 0xFF]));
    // Every tile of every tileset names its image, size and properties.
    let tiles = &tmj.tilesets[0];
    assert_eq!(tiles.columns, 0);
    assert_eq!(
        tiles.grid,
        Some(TmjGrid {
            height: 16,
            orientation: "orthogonal".to_owned(),
            width: 16
        })
    );
    let water_tile = &tiles.tiles[2];
    assert_eq!(water_tile.id, 2);
    assert_eq!(water_tile.image, "0000.tiles/tile-0002.png");
    assert_eq!((water_tile.imagewidth, water_tile.imageheight), (16, 16));
    assert_eq!(prop(&water_tile.properties, "g4.texture"), "syn_wasser");
    assert_eq!(prop(&water_tile.properties, "g4.role"), "water");
    let all: Vec<&str> = tmj
        .tilesets
        .iter()
        .flat_map(|t| t.tiles.iter().map(|tile| tile.image.as_str()))
        .collect();
    assert_eq!(all, paths);
    let h12 = &tmj.tilesets[1].tiles[28];
    assert_eq!(prop(&h12.properties, "g4.texture"), "height-p012");
    assert_eq!(prop(&h12.properties, "g4.role"), "height");
    let e1 = &tmj.tilesets[2].tiles[2];
    assert_eq!(prop(&e1.properties, "g4.texture"), "ramp-e1");
    assert_eq!(prop(&e1.properties, "g4.role"), "ramp");
}

#[test]
fn pieces_are_tile_objects_on_their_cell() {
    let mut w = one_tile_world();
    w.pieces = vec![PieceJson {
        tile: 5,
        at: [3, 4],
        rotate: 90,
    }];
    let (tmj, _) = export(&w);
    let [piece] = objects(layer(&tmj, "Teilmodelle")) else {
        panic!("one piece");
    };
    assert_eq!(piece.gid, Some(6));
    assert_eq!(
        (piece.x, piece.y, piece.width, piece.height),
        (48.0, 80.0, 16.0, 16.0)
    );
    assert_eq!(prop(&piece.properties, "g4.rotate"), 90);
    assert_eq!(piece.id, 1);
}

#[test]
fn buildings_are_points_at_their_position() {
    // The centre of tile (3, 5) is world unit (3 · 16 + 8, 5 · 16 + 8) from the map's corner,
    // (−200, −168) from its centre.
    let building = Building {
        model_id: 7,
        position: [-200 * 4096, 3 * 4096, -168 * 4096],
        rotation: [0; 3],
        scale: [0x1000; 3],
        dummy: [0; 8],
    };
    let w = one_tile_world();
    let (tmj, _) = to_tmj(0, &w, &[building], &Tileset::synthetic(), &|_| None, "0000").unwrap();
    let [point] = objects(layer(&tmj, "Gebäude (nur Ansicht)")) else {
        panic!("one building");
    };
    assert!(point.point);
    assert_eq!((point.x, point.y), (56.0, 88.0));
    assert_eq!(prop(&point.properties, "g4.model_id"), 7);
}

fn world_store() -> (crate::test_support::TextFixture, Store) {
    let fx = world_ready();
    let store = Store::new(Arc::new(fx.project()));
    (fx, store)
}

#[test]
fn export_writes_file_and_tiles_folder() {
    let (fx, mut store) = world_store();
    let req: WorldCreateRequest = serde_json::from_value(json!({"map": 0, "tileset": 0})).unwrap();
    world_create(&mut store, &req).unwrap();
    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("0000.tmj");
    let report = world_export_tmj(&mut store, 0, &path).unwrap();
    assert_eq!(report.map, 0);
    assert_eq!(report.path, path);
    assert_eq!(report.layers, 4);
    assert_eq!(report.images, 6 + 79 + 8);
    assert_eq!((report.pieces, report.overrides), (0, 0));
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.ends_with(b"\n"));
    let tmj: TmjMap = serde_json::from_slice(&bytes).unwrap();
    let buildings = store.map(0).unwrap().buildings.clone();
    assert_eq!(tmj.layers.len(), 4);
    assert_eq!(prop(&tmj.layers[3].properties, "g4.kind"), "buildings");
    assert_eq!(
        objects(&tmj.layers[3]).len(),
        buildings.len(),
        "one point per building"
    );
    let folder = out.path().join("0000.tiles");
    let mut files: Vec<String> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(files.len(), 6 + 79 + 8);
    assert!(files.iter().all(|f| f.ends_with(".png")));
    // The package decodes: the tiles are its textures, the grass 32 × 32.
    let grass = decode(&std::fs::read(folder.join("tile-0001.png")).unwrap());
    assert_eq!(grass.dimensions(), (32, 32));
    assert_eq!(
        (
            tmj.tilesets[0].tiles[1].imagewidth,
            tmj.tilesets[0].tiles[1].imageheight
        ),
        (32, 32)
    );
    // Nothing in the project changed: the export is no step.
    assert!(!fx.root().join("0000.tiles").exists());
}

#[test]
fn export_refuses_without_world() {
    let (_fx, mut store) = world_store();
    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("0000.tmj");
    match world_export_tmj(&mut store, 0, &path) {
        Err(KernelError::Refused(m)) => assert_eq!(m, not_a_world_map(0)),
        other => panic!("{other:?}"),
    }
    assert!(!path.exists());
}
