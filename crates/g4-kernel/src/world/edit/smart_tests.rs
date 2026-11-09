//! `world_edit` with `terrain` and `cliff` (phase 6.4b): a smart stroke resolved into cell
//! changes before the check, one undo step, the seam read from neighbour maps of the matrix.

use std::sync::Arc;

use serde_json::{Value, json};

use super::*;
use crate::project::{AssetId, AssetKind};
use crate::test_support::{TextFixture, world_ready_edges};
use crate::textproject::Part;
use crate::textproject::canon::to_canonical;
use crate::world::doc::fixtures::one_tile_world;
use crate::world::doc::{LayerJson, LayerRole, TerrainJson};
use crate::world::edge::fixtures::weg13;
use crate::world::edge::{EdgeSetJson, Place};
use crate::world::smart::fixtures::{PLUS_EXPECTED, plus_cells};
use crate::world::smart::{Seams, paint_terrain, seams, wrong_corners};
use crate::world::tileset::{Tileset, tileset_rel};

const N: usize = 32;
const MAP1: &str = "data/maps/0001.json";

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// Puts `world` on map `map` as one step „Welt“.
fn put_world(store: &mut Store, map: u16, world: WorldJson) {
    store
        .transaction("Welt", |tx| {
            tx.edit_world(map, |w| {
                *w = Some(world);
                Ok(())
            })
        })
        .unwrap();
}

/// `world_ready_edges` with `one_tile_world` on map 0 (map 1 east of it in matrix 0 stays
/// binary).
fn setup() -> (TextFixture, Store) {
    let fx = world_ready_edges();
    let mut st = store(&fx);
    put_world(&mut st, 0, one_tile_world());
    (fx, st)
}

fn edit(store: &mut Store, value: Value) -> Result<WorldReport, KernelError> {
    world_edit(store, &serde_json::from_value(value).unwrap())
}

fn refused<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
    match r {
        Err(KernelError::Refused(text)) => text,
        other => panic!("{other:?}"),
    }
}

fn world(store: &mut Store, map: u16) -> WorldJson {
    store.world(map).unwrap().cloned().unwrap()
}

fn grid(store: &mut Store, map: u16) -> WorldGrid {
    WorldGrid::parse(&world(store, map)).unwrap()
}

fn token(grid: &WorldGrid, x: usize, z: usize) -> String {
    tile_token(grid.layers[0].tiles[z * N + x])
}

fn rect(xs: std::ops::RangeInclusive<u8>, zs: std::ops::RangeInclusive<u8>) -> Vec<[u8; 2]> {
    zs.flat_map(|z| xs.clone().map(move |x| [x, z])).collect()
}

/// The token of a legend character of [`PLUS_EXPECTED`] in a set with one entry per place.
fn legend_token(set: &EdgeSetJson, c: char) -> String {
    let place = match c {
        '.' => return set.outside.clone(),
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
    };
    set.places.get(place)[0].clone()
}

/// The plus of [`PLUS_EXPECTED`] (x 2–14, z 3–15) on an otherwise outside map.
fn assert_plus(grid: &WorldGrid) {
    let set = weg13();
    for z in 0..N {
        for x in 0..N {
            let c = if (2..=14).contains(&x) && (3..=15).contains(&z) {
                PLUS_EXPECTED[z - 3].as_bytes()[x - 2] as char
            } else {
                '.'
            };
            assert_eq!(token(grid, x, z), legend_token(&set, c), "cell ({x}, {z})");
        }
    }
}

#[test]
fn a_terrain_stroke_is_one_step() {
    let (_fx, mut st) = setup();
    let steps = st.history().0.len();
    assert_eq!(plus_cells().len(), 45);
    let label = "Gelände gemalt: Karte 0000 (45 Zellen)";
    let report = edit(
        &mut st,
        json!({"map": 0, "label": label, "changes": [{"cells": plus_cells(), "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    assert_eq!((report.map, report.tileset, report.cells), (0, 0, 45));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    let bake = report.bake.as_ref().unwrap();
    assert!(bake.ok, "{:?}", bake.messages);
    let (undo, _) = st.history();
    assert_eq!(undo.len(), steps + 1);
    assert_eq!(undo.last().unwrap(), label);
    let g = grid(&mut st, 0);
    assert_plus(&g);
    assert_eq!(wrong_corners(&g, 0, &weg13()), Vec::<[u8; 2]>::new());
    st.undo().unwrap();
    assert_eq!(world(&mut st, 0), one_tile_world());
    // The default label counts the painted cells.
    st.redo().unwrap();
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[20, 20], [21, 20]], "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    assert_eq!(
        st.history().0.last().unwrap(),
        "Welt geändert: Karte 0000 (2 Zellen)"
    );
}

#[test]
fn a_cliff_stroke_is_one_step_and_drops_the_verbatim_terrain() {
    let (_fx, mut st) = setup();
    let mut verbatim = one_tile_world();
    verbatim.terrain = Some(TerrainJson {
        bdhc: Part::Hex {
            hex: "00".to_owned(),
        },
    });
    put_world(&mut st, 0, verbatim);
    let steps = st.history().0.len();
    let report = edit(
        &mut st,
        json!({"map": 0, "label": "Klippe", "changes": [
            {"cells": rect(10..=15, 10..=15), "cliff": {"set": "Weg", "height": 16}}
        ]}),
    )
    .unwrap();
    assert_eq!(report.cells, 36);
    assert!(!report.bake.as_ref().unwrap().verbatim_terrain);
    assert_eq!(st.history().0.len(), steps + 1);
    let after = world(&mut st, 0);
    assert_eq!(after.terrain, None);
    let g = WorldGrid::parse(&after).unwrap();
    assert_eq!((g.heights[12 * N + 12], g.heights[9 * N + 9]), (16, 0));
    // The rim on top: the corner cell is the outer corner NW, the middle the centre, the ring
    // below stays ground.
    assert_eq!(token(&g, 10, 10), "000b");
    assert_eq!(token(&g, 12, 10), "0007");
    assert_eq!(token(&g, 12, 12), "0006");
    assert_eq!(token(&g, 9, 9), "0000");
    st.undo().unwrap();
    assert!(world(&mut st, 0).terrain.is_some());
}

#[test]
fn a_refused_smart_stroke_writes_nothing() {
    let (fx, mut st) = setup();
    let before = world(&mut st, 0);
    let history = st.history();
    let dirty = st.dirty();
    // Two areas touching at one corner need the diagonal, which weg13 lacks.
    let text = refused(edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[4, 4], [6, 6]], "terrain": {"set": "Weg"}}]}),
    ));
    assert_eq!(
        text,
        "Kantensatz „Weg“: der Platz „Diagonale NW–SO“ hat keine Kachel (Zelle (5, 5))."
    );
    // A plain change before it in the same call is not written either.
    let text = refused(edit(
        &mut st,
        json!({"map": 0, "changes": [
            {"cells": [[20, 20]], "tile": "0001"},
            {"cells": [[4, 4], [6, 6]], "terrain": {"set": "Weg"}}
        ]}),
    ));
    assert!(text.contains("Diagonale NW–SO"), "{text}");
    // A half-filled set from the editor: the stroke needs the empty place.
    let mut half = Tileset::synthetic_edges();
    half.edge_sets[0].places.e.clear();
    fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&half));
    let mut fresh = store(&fx);
    put_world(&mut fresh, 0, one_tile_world());
    let fresh_history = fresh.history();
    let text = refused(edit(
        &mut fresh,
        json!({"map": 0, "changes": [{"cells": [[8, 8]], "terrain": {"set": "Weg"}}]}),
    ));
    assert_eq!(
        text,
        "Kantensatz „Weg“: der Platz „Kante O“ hat keine Kachel (Zelle (9, 8))."
    );
    assert_eq!(fresh.history(), fresh_history);
    assert_eq!(world(&mut st, 0), before);
    assert_eq!(st.history(), history);
    assert_eq!(st.dirty(), dirty);
}

#[test]
fn a_stroke_with_a_missing_set_is_refused() {
    let fx = world_ready_edges();
    let mut without = Tileset::synthetic_edges();
    without.edge_sets.clear();
    fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&without));
    let mut st = store(&fx);
    put_world(&mut st, 0, one_tile_world());
    let history = st.history();
    for change in [
        json!({"cells": [[3, 3]], "terrain": {"set": "Weg"}}),
        json!({"cells": [[3, 3]], "cliff": {"set": "Weg", "height": 16}}),
    ] {
        let text = refused(edit(&mut st, json!({"map": 0, "changes": [change]})));
        assert_eq!(text, "Kantensatz „Weg“ gibt es im Tileset 0000 nicht.");
    }
    assert_eq!(st.history(), history);
    assert_eq!(world(&mut st, 0), one_tile_world());
}

/// `one_tile_world` with an overlay „Deko“ (all empty).
fn with_overlay() -> WorldJson {
    let mut w = one_tile_world();
    w.layers.push(LayerJson {
        name: "Deko".to_owned(),
        role: LayerRole::Overlay,
        tiles: vec![vec!["."; N].join(" "); N],
        heights: None,
        ramps: None,
    });
    w
}

#[test]
fn cliff_off_the_ground_is_refused() {
    let (_fx, mut st) = setup();
    put_world(&mut st, 0, with_overlay());
    let history = st.history();
    for layer in [1, 5] {
        let text = refused(edit(
            &mut st,
            json!({"map": 0, "changes": [
                {"layer": layer, "cells": [[3, 3]], "cliff": {"set": "Weg", "height": 16}}
            ]}),
        ));
        assert_eq!(text, "Klippen gibt es nur auf der Bodenebene.");
    }
    // A height outside −16 to 62: the parser's sentence.
    let text = refused(edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[3, 3]], "cliff": {"set": "Weg", "height": 63}}]}),
    ));
    assert_eq!(text, "`63` liegt nicht in \u{2212}16 bis 62.");
    assert_eq!(st.history(), history);
    // On the ground (`layer: 0` given) it paints.
    edit(
        &mut st,
        json!({"map": 0, "changes": [
            {"layer": 0, "cells": [[3, 3]], "cliff": {"set": "Weg", "height": 16}}
        ]}),
    )
    .unwrap();
    assert_eq!(grid(&mut st, 0).heights[3 * N + 3], 16);
}

#[test]
fn cells_and_layers_use_the_sentences_of_world_edit() {
    let (_fx, mut st) = setup();
    let text = refused(edit(
        &mut st,
        json!({"map": 0, "changes": [{"layer": 3, "cells": [[3, 3]], "terrain": {"set": "Weg"}}]}),
    ));
    assert_eq!(text, "Ebene 3 gibt es nicht (Karte 0000 hat 1).");
    for change in [
        json!({"cells": [[32, 4]], "terrain": {"set": "Weg"}}),
        json!({"cells": [[32, 4]], "cliff": {"set": "Weg", "height": 16}}),
    ] {
        let text = refused(edit(&mut st, json!({"map": 0, "changes": [change]})));
        assert_eq!(text, "Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).");
    }
    // The terrain brush paints an overlay of its own.
    put_world(&mut st, 0, with_overlay());
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"layer": 1, "cells": [[3, 3]], "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    let g = grid(&mut st, 0);
    assert_eq!(tile_token(g.layers[1].tiles[3 * N + 3]), "0006");
    assert_eq!(token(&g, 3, 3), "0000");
}

#[test]
fn one_field_names_terrain_and_cliff() {
    assert_eq!(
        ONE_FIELD,
        "Eine Änderung setzt genau eines von tile, height, ramp, permission, terrain, cliff."
    );
    let (_fx, mut st) = setup();
    for change in [
        json!({"cells": [[3, 3]], "tile": "0001", "terrain": {"set": "Weg"}}),
        json!({"cells": [[3, 3]], "terrain": {"set": "Weg"}, "cliff": {"set": "Weg", "height": 16}}),
        json!({"cells": [[3, 3]], "height": 2, "cliff": {"set": "Weg", "height": 16}}),
        json!({"cells": [[3, 3]]}),
    ] {
        let text = refused(edit(&mut st, json!({"map": 0, "changes": [change]})));
        assert_eq!(text, ONE_FIELD);
    }
    // Unknown fields of the brushes are refused by the request.
    let bad: Result<WorldEditRequest, _> = serde_json::from_value(
        json!({"map": 0, "changes": [{"cells": [[3, 3]], "terrain": {"set": "Weg", "x": 1}}]}),
    );
    assert!(bad.is_err());
}

#[test]
fn smart_and_plain_changes_apply_in_order() {
    let (_fx, mut st) = setup();
    let row: Vec<[u8; 2]> = (0..32).map(|x| [x, 0]).collect();
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [
            {"cells": row, "tile": "0001"},
            {"cells": [[5, 1]], "terrain": {"set": "Weg"}}
        ]}),
    )
    .unwrap();
    assert_eq!(report.cells, 33);
    let g = grid(&mut st, 0);
    // The ring sees the 0001 row as foreign: in the ring it is overwritten, beside it stays.
    let row0: Vec<String> = (3..=7).map(|x| token(&g, x, 0)).collect();
    assert_eq!(row0, ["0001", "000b", "0007", "000c", "0001"]);
    assert_eq!(token(&g, 5, 1), "0006");
    // A centre tile set by the first change is part of the area the second one grows: its
    // corners read inside, so it stays the centre (resolved on the world before the call, the
    // stroke would have made it the edge W, `000a`), and the ring above sees both cells.
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [
            {"cells": [[10, 10]], "tile": "0006"},
            {"cells": [[11, 10]], "terrain": {"set": "Weg"}}
        ]}),
    )
    .unwrap();
    assert_eq!(report.cells, 2);
    let g = grid(&mut st, 0);
    assert_eq!(token(&g, 10, 10), "0006");
    assert_eq!(token(&g, 11, 10), "0006");
    assert_eq!(token(&g, 11, 9), "0007");
    assert_eq!(token(&g, 12, 9), "000c");
    assert_eq!(token(&g, 12, 10), "0008");
    // Resolved on a copy without the first change, the stroke gives the edge W there.
    let alone = paint_terrain(
        &WorldGrid::parse(&one_tile_world()).unwrap(),
        0,
        &weg13(),
        &[[11, 10]],
        true,
        0,
        &Seams::default(),
    )
    .unwrap();
    assert!(alone.tiles.contains(&([10, 10], "000a".to_owned())));
}

#[test]
fn nothing_changed_is_nothing_to_change() {
    let (_fx, mut st) = setup();
    let history = st.history();
    // Taking the area away where there is none.
    let text = refused(edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[3, 3]], "terrain": {"set": "Weg", "inside": false}}]}),
    ));
    assert_eq!(text, NOTHING_TO_CHANGE);
    assert_eq!(st.history(), history);
    // The same stroke twice: the second changes nothing.
    let stroke = json!({"map": 0, "changes": [{"cells": plus_cells(), "terrain": {"set": "Weg"}}]});
    edit(&mut st, stroke.clone()).unwrap();
    let history = st.history();
    assert_eq!(refused(edit(&mut st, stroke)), NOTHING_TO_CHANGE);
    // The same cliff twice.
    let cliff = json!({"map": 0, "changes": [{"cells": rect(20..=24, 20..=24), "cliff": {"set": "Weg", "height": 16}}]});
    edit(&mut st, cliff.clone()).unwrap();
    assert_eq!(st.history().0.len(), history.0.len() + 1);
    assert_eq!(refused(edit(&mut st, cliff)), NOTHING_TO_CHANGE);
}

/// [`setup`] with map 1 (east of map 0) a world map with an area at its x 0–3, z 9–11, painted
/// by `world_edit` on map 1; the report of that stroke.
fn with_east_area() -> (TextFixture, Store, WorldReport) {
    let (fx, mut st) = setup();
    put_world(&mut st, 1, one_tile_world());
    let report = edit(
        &mut st,
        json!({"map": 1, "changes": [{"cells": rect(0..=3, 9..=11), "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    (fx, st, report)
}

#[test]
fn the_seam_continues_a_neighbour_s_area() {
    let (_fx, mut st, _) = with_east_area();
    let east = grid(&mut st, 1);
    assert_eq!(token(&east, 0, 10), "0006");
    assert_eq!(token(&east, 0, 8), "0007");
    // A stroke up to x 30 on map 0: its ring cells at x 31 lie between the stroke and map 1's
    // area, so they are middle, not the edge O; the corners at x 32 are map 1's.
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": rect(27..=30, 9..=11), "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    let g = grid(&mut st, 0);
    let x31: Vec<String> = (8..=12).map(|z| token(&g, 31, z)).collect();
    assert_eq!(x31, ["0007", "0006", "0006", "0006", "0009"]);
    assert_eq!(token(&g, 26, 10), "000a");
    // Without the neighbour the stroke ends in the edge O at x 31.
    let alone = paint_terrain(
        &WorldGrid::parse(&one_tile_world()).unwrap(),
        0,
        &weg13(),
        &rect(27..=30, 9..=11),
        true,
        0,
        &Seams::default(),
    )
    .unwrap();
    assert!(alone.tiles.contains(&([31, 10], "0008".to_owned())));
    assert!(alone.tiles.contains(&([31, 8], "000c".to_owned())));
}

#[test]
fn neighbours_are_never_written() {
    let (fx, mut st, _) = with_east_area();
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": rect(28..=31, 9..=11), "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    st.save().unwrap();
    let east = world(&mut st, 1);
    let file = fx.read(MAP1);
    let member_path = fx.project().member_path(AssetKind::Map, 1);
    let member = std::fs::read(&member_path).unwrap();
    // Taking (31, 10) away: map 1's middle cells at x 0, z 9–11 no longer fit.
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[31, 10]], "terrain": {"set": "Weg", "inside": false}}]}),
    )
    .unwrap();
    assert_eq!(
        report.notes,
        [
            "Naht zu Karte 0001: 3 Zellen dort passen nicht mehr zum Satz „Weg“; die Nachbarkarte \
          bleibt, wie sie ist."
        ]
    );
    assert_eq!(world(&mut st, 1), east);
    let map1 = |kind| AssetId { kind, index: 1 };
    let dirty = st.dirty();
    assert!(!dirty.contains(&map1(AssetKind::Map)), "{dirty:?}");
    assert!(!dirty.contains(&map1(AssetKind::WorldMap)), "{dirty:?}");
    st.save().unwrap();
    assert_eq!(fx.read(MAP1), file);
    assert_eq!(std::fs::read(&member_path).unwrap(), member);
}

#[test]
fn a_binary_or_foreign_neighbour_reads_nothing() {
    let cells = rect(27..=30, 9..=11);
    let alone = |cells: &[[u8; 2]]| {
        let mut g = WorldGrid::parse(&one_tile_world()).unwrap();
        let r = paint_terrain(&g, 0, &weg13(), cells, true, 0, &Seams::default()).unwrap();
        for ([x, z], t) in &r.tiles {
            g.layers[0].tiles[usize::from(*z) * N + usize::from(*x)] = parse_tile(t).unwrap();
        }
        g
    };
    let stroke = |st: &mut Store, cells: &[[u8; 2]]| {
        edit(
            st,
            json!({"map": 0, "changes": [{"cells": cells, "terrain": {"set": "Weg"}}]}),
        )
        .unwrap()
    };
    // A binary neighbour (map 1 as the fixture has it).
    let (_fx, mut st) = setup();
    assert_eq!(
        seams(&mut st, 0, &one_tile_world(), 0).unwrap(),
        Seams::default()
    );
    let report = stroke(&mut st, &cells);
    assert!(report.notes.is_empty());
    assert_eq!(grid(&mut st, 0), alone(&cells));

    // A world neighbour on another package: all middle tiles, read as nothing.
    let (fx, mut st) = setup();
    let other = Tileset {
        package: 2,
        ..Tileset::synthetic_edges()
    };
    fx.write(tileset_rel(2).to_str().unwrap(), &to_canonical(&other));
    let mut foreign = one_tile_world();
    foreign.tileset = 2;
    foreign.layers[0].tiles = vec![vec!["0006"; N].join(" "); N];
    put_world(&mut st, 1, foreign);
    assert_eq!(
        seams(&mut st, 0, &one_tile_world(), 0).unwrap(),
        Seams::default()
    );
    let report = stroke(&mut st, &cells);
    assert!(report.notes.is_empty());
    assert_eq!(grid(&mut st, 0), alone(&cells));

    // The map at the matrix edge (map 0's west): nothing past it.
    let (_fx, mut st) = setup();
    let west = rect(0..=3, 9..=11);
    stroke(&mut st, &west);
    assert_eq!(grid(&mut st, 0), alone(&west));
}

#[test]
fn a_same_package_neighbour_is_read_at_the_seam() {
    let (_fx, mut st, _) = with_east_area();
    let s = seams(&mut st, 0, &one_tile_world(), 0).unwrap();
    // The ring x 32, z 0 to 31 of map 1 (its x 0); nothing from the other sides.
    assert_eq!(s.owner.len(), 32);
    assert!(s.owner.values().all(|&m| m == 1));
    assert_eq!(s.tile(0, 32, 10).map(|t| t.tile), Some(6));
    assert_eq!(s.tile(0, 32, 2).map(|t| t.tile), Some(0));
    assert_eq!(s.tile(0, -1, 10), None);
    assert_eq!(s.heights.get(&(32, 10)), Some(&0));
    // An overlay index map 1 lacks reads no tiles.
    let s = seams(&mut st, 0, &with_overlay(), 1).unwrap();
    assert!(s.tiles.is_empty());
}

#[test]
fn notes_are_in_the_report() {
    let (_fx, _st, report) = with_east_area();
    // Map 1's stroke at its x 0: map 0's outside cells at x 31, z 8–12 no longer fit.
    assert_eq!(
        report.notes,
        [
            "Naht zu Karte 0000: 5 Zellen dort passen nicht mehr zum Satz „Weg“; die Nachbarkarte \
          bleibt, wie sie ist."
        ]
    );
    let v = serde_json::to_value(&report).unwrap();
    assert_eq!(v["notes"][0], json!(report.notes[0]));
    // No notes: no key.
    let (_fx, mut st) = setup();
    let quiet = edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[9, 9]], "terrain": {"set": "Weg"}}]}),
    )
    .unwrap();
    let v = serde_json::to_value(&quiet).unwrap();
    assert!(v.get("notes").is_none(), "{v}");
}
