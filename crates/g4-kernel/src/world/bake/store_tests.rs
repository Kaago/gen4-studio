//! The store's side of the world bake (phase 6.3b): the Map asset of a world map is its bake.

use std::sync::Arc;

use g4_formats::bdhc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use serde_json::{Value, json};

use super::*;
use crate::bake::{AreaStyle, BakeInput, Face, VANILLA_VRAM_MAX_PT, bake, pack_textures};
use crate::error::KernelError;
use crate::original::{ResetTarget, reset};
use crate::project::{AssetId, AssetKind};
use crate::store::Store;
use crate::test_support::{ProjectOriginal, TextFixture, world_ready};
use crate::textproject::Part;
use crate::textproject::canon::to_canonical;
use crate::world::doc::TerrainJson;
use crate::world::doc::fixtures::one_tile_world;
use crate::world::edit::{
    WorldCreateRequest, WorldEditRequest, WorldReport, world_create, world_edit, world_to_binary,
    world_to_binary_label,
};
use crate::world::grid::WorldGrid;
use crate::world::mesh::mesh;
use crate::world::tileset::{Tileset, tileset_rel};

const MAP: AssetId = AssetId {
    kind: AssetKind::Map,
    index: 0,
};
const JSON: &str = "data/maps/0000.json";

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

fn create(store: &mut Store, map: u16) -> WorldReport {
    let req: WorldCreateRequest =
        serde_json::from_value(json!({"map": map, "tileset": 0})).unwrap();
    world_create(store, &req).unwrap()
}

fn edit(store: &mut Store, value: Value) -> Result<WorldReport, KernelError> {
    let req: WorldEditRequest = serde_json::from_value(value).unwrap();
    world_edit(store, &req)
}

fn map_bytes(store: &mut Store) -> Vec<u8> {
    store.map(0).unwrap().write(Family::Pt)
}

fn member(fx: &TextFixture) -> Vec<u8> {
    std::fs::read(fx.project().member_path(AssetKind::Map, 0)).unwrap()
}

fn file_json(fx: &TextFixture) -> Value {
    serde_json::from_slice(&fx.read(JSON)).unwrap()
}

fn codes(report: &WorldReport) -> Vec<String> {
    report
        .bake
        .as_ref()
        .expect("a bake report")
        .messages
        .iter()
        .map(|m| m.code.clone())
        .collect()
}

/// The pack textures of `world_ready`'s package 0.
fn ready_textures(fx: &TextFixture) -> Vec<PackTexture> {
    let pack = std::fs::read(fx.project().member_path(AssetKind::MapTexture, 0)).unwrap();
    pack_textures(&pack).unwrap()
}

/// The parts `world` bakes to directly: model of the mesher's faces, the generator's BDHC and
/// the derived permissions.
fn expected(fx: &TextFixture, world: &WorldJson) -> (Vec<u8>, Vec<u8>, Vec<g4_formats::map::Cell>) {
    let grid = WorldGrid::parse(world).unwrap();
    let tileset = Tileset::synthetic();
    let textures = ready_textures(fx);
    let faces: Vec<Face> = mesh(&grid, &tileset).into_iter().map(Face::from).collect();
    let input = BakeInput {
        style: AreaStyle::Outdoor,
        textures: &textures,
        vram_bytes: 0,
        vram_limit: VANILLA_VRAM_MAX_PT,
    };
    let model = bake(&faces, &input);
    assert!(model.refusal().is_none(), "{:?}", model.messages);
    (
        model.nsbmd,
        bdhc::from_grid(&grid.height_grid()).write(),
        world_permissions(&grid, &tileset, Family::Pt),
    )
}

#[test]
fn world_create_bakes_the_map_in_the_same_step() {
    let fx = world_ready();
    let mut st = store(&fx);
    let before = member(&fx);
    assert_eq!(map_bytes(&mut st), before);
    let report = create(&mut st, 0);
    let bake = report.bake.as_ref().expect("a bake report");
    assert!(bake.ok, "{:?}", bake.messages);
    assert!(!bake.model_kept);
    assert_eq!(bake.style, Some(AreaStyle::Outdoor));
    assert_eq!(bake.textures, ["syn_boden"]);
    let (model, terrain, permissions) = expected(&fx, &one_tile_world());
    let map = st.map(0).unwrap().clone();
    assert_eq!(map.model, model);
    assert_eq!(map.bdhc, terrain);
    assert_eq!(map.permissions, permissions);
    assert_eq!(bake.model_bytes, model.len());
    // Buildings stay.
    let old = MapFile::read(&before, Family::Pt).unwrap();
    assert_eq!(map.buildings, old.buildings);
    assert_eq!(st.history().0, ["Welt angelegt: Karte 0000"]);
    assert_eq!(
        st.dirty(),
        [
            MAP,
            AssetId {
                kind: AssetKind::WorldMap,
                index: 0
            }
        ]
    );
    let undone = st.undo().unwrap();
    assert!(undone.contains(&MAP), "{undone:?}");
    assert_eq!(map_bytes(&mut st), before);
    assert_eq!(st.world(0).unwrap(), None);
    st.redo().unwrap();
    assert_eq!(st.map(0).unwrap().model, model);
}

#[test]
fn world_edit_rebakes_and_undo_restores_both() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    let created = map_bytes(&mut st);
    let world_before = st.world(0).unwrap().cloned().unwrap();
    let cells: Vec<[u8; 2]> = (0..8).map(|x| [x, 5]).collect();
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": cells, "tile": "0001"}]}),
    )
    .unwrap();
    let bake = report.bake.as_ref().unwrap();
    assert!(bake.ok, "{:?}", bake.messages);
    assert_eq!(bake.textures, ["syn_boden", "syn_gras"]);
    let world_after = st.world(0).unwrap().cloned().unwrap();
    let (model, terrain, permissions) = expected(&fx, &world_after);
    let edited = st.map(0).unwrap().clone();
    assert_eq!(edited.model, model);
    assert_eq!(edited.bdhc, terrain);
    assert_eq!(edited.permissions, permissions);
    // Grass has kind 0x02.
    assert_eq!(edited.permissions[5 * 32 + 3].kind, 0x02);
    assert_eq!(st.history().0.len(), 2);

    st.undo().unwrap();
    assert_eq!(map_bytes(&mut st), created);
    assert_eq!(st.world(0).unwrap(), Some(&world_before));
    st.redo().unwrap();
    assert_eq!(st.map(0).unwrap(), &edited);
    assert_eq!(st.world(0).unwrap(), Some(&world_after));
}

#[test]
fn save_writes_member_and_json_without_derived_keys() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[4, 4], [5, 4]], "height": 1}]}),
    )
    .unwrap();
    let baked = map_bytes(&mut st);
    st.save().unwrap();
    assert!(st.dirty().is_empty());
    assert_eq!(member(&fx), baked);
    let v = file_json(&fx);
    assert_eq!(v["source"], "world");
    for key in ["permissions", "model", "bdhc"] {
        assert!(v.get(key).is_none(), "{key} in {v}");
    }
    assert!(v.get("buildings").is_some());
    // No side file of a derived part.
    assert!(!fx.root().join("data/maps/0000.nsbmd").exists());
    assert!(!fx.root().join("data/maps/0000.bdhc").exists());
    // A second save writes nothing.
    assert!(st.save().unwrap().written.is_empty());
}

#[test]
fn loading_a_world_map_bakes_it() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    st.save().unwrap();
    let saved = member(&fx);
    let mut again = store(&fx);
    assert_eq!(map_bytes(&mut again), saved);
    assert!(again.dirty().is_empty());
    // The member on disk is not what the store reads: a world map is its bake.
    let project = fx.project();
    std::fs::write(project.member_path(AssetKind::Map, 0), b"not a map").unwrap();
    let mut third = store(&fx);
    assert_eq!(map_bytes(&mut third), saved);
}

#[test]
fn get_map_heights_of_a_world_map_follow_the_world() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    let plateau: Vec<[u8; 2]> = (10..15)
        .flat_map(|z| (10..15).map(move |x| [x, z]))
        .collect();
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": plateau, "height": 2}]}),
    )
    .unwrap();
    let heights = crate::terrain::map_heights(&st.map(0).unwrap().bdhc).unwrap();
    // fx32: 16 world units at the plateau's middle, 0 beside it.
    assert_eq!(heights[12 * 32 + 12], Some(16 << 12));
    assert_eq!(heights[0], Some(0));
}

/// The changes that paint `oversize_world` over a blank world: its three tiles and its heights.
fn oversize_changes() -> Value {
    let mut changes = Vec::new();
    for tile in 0..3u16 {
        let cells: Vec<[u8; 2]> = (0..32u8)
            .flat_map(|z| (0..32u8).map(move |x| [x, z]))
            .filter(|&[x, z]| (u16::from(x) + u16::from(z)) % 3 == tile)
            .collect();
        changes.push(json!({"cells": cells, "tile": format!("{tile:04}")}));
    }
    for h in 1..40usize {
        let cells: Vec<[u8; 2]> = (0..32u8)
            .flat_map(|z| (0..32u8).map(move |x| [x, z]))
            .filter(|&[x, z]| (7 * usize::from(x) + 13 * usize::from(z)) % 40 == h)
            .collect();
        if !cells.is_empty() {
            changes.push(json!({"cells": cells, "height": h}));
        }
    }
    json!({"map": 0, "changes": changes})
}

#[test]
fn an_oversize_stroke_keeps_the_last_model_and_reports() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    let created = st.map(0).unwrap().clone();
    let world_before = st.world(0).unwrap().cloned();
    let report = edit(&mut st, oversize_changes()).unwrap();
    assert_eq!(
        st.world(0).unwrap().cloned(),
        Some(crate::world::doc::fixtures::oversize_world())
    );
    let bake = report.bake.as_ref().unwrap();
    assert!(!bake.ok);
    assert!(bake.model_kept);
    assert_eq!(codes(&report), ["bake.model-too-big"]);
    let map = st.map(0).unwrap().clone();
    assert_eq!(map.model, created.model);
    assert_eq!(bake.model_bytes, created.model.len());
    // BDHC and permissions follow the world.
    let grid = WorldGrid::parse(&crate::world::doc::fixtures::oversize_world()).unwrap();
    assert_eq!(map.bdhc, bdhc::from_grid(&grid.height_grid()).write());
    assert_ne!(map.permissions, created.permissions);
    // One undo step puts everything back.
    assert_eq!(st.history().0.len(), 2);
    st.undo().unwrap();
    assert_eq!(st.map(0).unwrap(), &created);
    assert_eq!(st.world(0).unwrap().cloned(), world_before);
}

#[test]
fn editing_derived_parts_of_a_world_map_is_refused() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    let before = map_bytes(&mut st);
    let refusal = world_derived_refusal(0);
    assert_eq!(
        refusal,
        "Karte 0000 ist eine Welt-Karte: Modell, Terrain und Permissions entstehen beim Backen; \
         Permissions über `world_edit` (`permission`), zurück zur binären Karte über „Als \
         binäre Karte übernehmen“."
    );
    let refused = |r: Result<_, KernelError>| match r {
        Err(KernelError::Refused(text)) => text,
        other => panic!("{other:?}"),
    };
    let r = st.edit_map(0, "Permission", |m| {
        m.permissions[0].collision ^= 0x80;
        Ok(())
    });
    assert_eq!(refused(r.map(|_| ())), refusal);
    let r = st.edit_map(0, "Modell", |m| {
        m.model.push(0);
        Ok(())
    });
    assert_eq!(refused(r.map(|_| ())), refusal);
    let r = st.transaction("Terrain", |tx| {
        tx.edit_map(0, |m| {
            m.bdhc.push(0);
            Ok(())
        })
    });
    assert_eq!(refused(r.map(|_| ())), refusal);
    let mut changed = MapFile::read(&before, Family::Pt).unwrap();
    changed.permissions[3].kind ^= 1;
    let r = st.transaction("Ersetzt", |tx| tx.replace(MAP, &changed.write(Family::Pt)));
    assert_eq!(refused(r.map(|_| ())), refusal);
    assert_eq!(map_bytes(&mut st), before);
    assert_eq!(st.history().0.len(), 1);

    // Buildings are the map's own: moving one goes, and a later stroke keeps it.
    st.edit_map(0, "Gebäude verschoben", |m| {
        m.buildings[0].position[0] += 3;
        Ok(())
    })
    .unwrap();
    let moved = st.map(0).unwrap().buildings.clone();
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[1, 1]], "tile": "0002"}]}),
    )
    .unwrap();
    assert_eq!(st.map(0).unwrap().buildings, moved);
    // A binary map's derived parts stay editable.
    st.edit_map(1, "Permission", |m| {
        m.permissions[0].collision ^= 0x80;
        Ok(())
    })
    .unwrap();
}

#[test]
fn height_edit_ends_verbatim_terrain() {
    let fx = world_ready();
    let mut st = store(&fx);
    let original = st.map(0).unwrap().clone();
    // A world with verbatim terrain: the map's BDHC stays the member's.
    let mut world = one_tile_world();
    world.terrain = Some(TerrainJson {
        bdhc: Part::Hex {
            hex: "00".to_owned(),
        },
    });
    st.transaction("Welt", |tx| {
        tx.edit_world(0, |w| {
            *w = Some(world.clone());
            Ok(())
        })
    })
    .unwrap();
    assert_eq!(st.map(0).unwrap().bdhc, original.bdhc);
    let report = edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[6, 6]], "height": 1}]}),
    )
    .unwrap();
    assert!(!report.bake.as_ref().unwrap().verbatim_terrain);
    let after = st.world(0).unwrap().cloned().unwrap();
    assert_eq!(after.terrain, None);
    let grid = WorldGrid::parse(&after).unwrap();
    assert_eq!(
        st.map(0).unwrap().bdhc,
        bdhc::from_grid(&grid.height_grid()).write()
    );
    // A tile change keeps it.
    let mut st = store(&fx);
    st.transaction("Welt", |tx| {
        tx.edit_world(0, |w| {
            *w = Some(world.clone());
            Ok(())
        })
    })
    .unwrap();
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[6, 6]], "tile": "0001"}]}),
    )
    .unwrap();
    assert!(st.world(0).unwrap().unwrap().terrain.is_some());
    assert_eq!(st.map(0).unwrap().bdhc, original.bdhc);
}

#[test]
fn world_to_binary_keeps_the_baked_map() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    st.save().unwrap();
    let baked = map_bytes(&mut st);
    assert_eq!(world_to_binary_label(123), "Welt aufgegeben: Karte 0123");
    let report = world_to_binary(&mut st, 0).unwrap();
    assert_eq!((report.map, report.tileset), (0, 0));
    assert!(report.change.is_some());
    assert!(report.bake.is_none());
    assert_eq!(
        st.history().0.last().map(String::as_str),
        Some("Welt aufgegeben: Karte 0000")
    );
    assert_eq!(st.world(0).unwrap(), None);
    assert_eq!(map_bytes(&mut st), baked);
    st.save().unwrap();
    let v = file_json(&fx);
    assert_eq!(v["source"], "binary");
    assert!(v.get("world").is_none());
    for key in ["permissions", "model", "bdhc"] {
        assert!(v.get(key).is_some(), "{key} missing in {v}");
    }
    assert_eq!(member(&fx), baked);
    // Read anew: the binary map, its derived parts editable again.
    let mut again = store(&fx);
    assert_eq!(map_bytes(&mut again), baked);
    again
        .edit_map(0, "Permission", |m| {
            m.permissions[0].collision ^= 0x80;
            Ok(())
        })
        .unwrap();
    // A map without world is refused.
    match world_to_binary(&mut again, 0) {
        Err(KernelError::Refused(text)) => {
            assert_eq!(text, crate::world::edit::not_a_world_map(0));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn reset_of_a_world_names_world_to_binary() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    st.save().unwrap();
    let original = ProjectOriginal::new(fx.root());
    match reset(&mut st, &original, &ResetTarget::Map { index: 0 }, None) {
        Err(KernelError::Refused(text)) => assert_eq!(
            text,
            "Eine Welt hat kein Original; erst „Als binäre Karte übernehmen“."
        ),
        other => panic!("{:?}", other.map(|_| ())),
    }
    assert_eq!(st.history().0, ["Welt angelegt: Karte 0000"]);
}

#[test]
fn forget_world_bakes_rebakes_on_use() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    st.save().unwrap();
    assert_eq!(st.map(0).unwrap().permissions[0].kind, 0);
    let mut tileset = Tileset::synthetic();
    tileset.tiles[0].kind = 0x33;
    fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&tileset));
    st.forget_world_bakes(0);
    assert_eq!(st.map(0).unwrap().permissions[0].kind, 0x33);
    assert!(st.dirty().is_empty(), "{:?}", st.dirty());
    // The history stays: undo of the world still works.
    st.undo().unwrap();
    assert_eq!(st.world(0).unwrap(), None);
}

#[test]
fn a_painted_map_without_area_reports_no_area() {
    let fx = world_ready();
    let mut st = store(&fx);
    // Map 2 lies in no matrix cell.
    let report = create(&mut st, 2);
    assert_eq!(codes(&report), ["world.no-area"]);
    let bake = report.bake.unwrap();
    assert!(!bake.ok && bake.model_kept);
    assert_eq!(bake.style, None);
    assert_eq!(st.map(2).unwrap().permissions.len(), 1024);
}

#[test]
fn a_package_that_does_not_read_reports_texture_missing() {
    let fx = TextFixture::exported();
    fx.write(
        tileset_rel(0).to_str().unwrap(),
        &to_canonical(&Tileset::synthetic()),
    );
    let mut st = store(&fx);
    let before = st.map(0).unwrap().clone();
    let report = create(&mut st, 0);
    assert_eq!(codes(&report), ["bake.texture-missing"]);
    let bake = report.bake.unwrap();
    assert!(!bake.ok && bake.model_kept);
    assert_eq!(st.map(0).unwrap().model, before.model);
    assert!(report.change.is_some());
}

#[test]
fn world_edit_refuses_a_world_file_with_derived_keys() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    st.save().unwrap();
    // A hand edit (a Git merge, say) puts a derived key back.
    fx.edit(
        JSON,
        "\"buildings\"",
        "\"permissions\": [],\n  \"buildings\"",
    );
    let before = fx.read(JSON);
    let mut again = store(&fx);
    let err = edit(
        &mut again,
        json!({"map": 0, "changes": [{"cells": [[1, 1]], "tile": "0001"}]}),
    )
    .unwrap_err();
    let KernelError::Compile(problems) = err else {
        panic!("{err:?}");
    };
    let [p] = problems.as_slice() else {
        panic!("{problems:?}");
    };
    assert!(
        p.message.contains("permissions") && p.message.contains("sie entstehen beim Backen"),
        "{p:?}"
    );
    assert_eq!(p.file, std::path::Path::new(JSON));
    assert!(p.line > 1, "{p:?}");
    assert!(again.history().0.is_empty());
    assert_eq!(fx.read(JSON), before);
}

/// Task 4 review: the save bakes a dirty world map anew over a tileset changed since the stroke;
/// undo and redo still work.
#[test]
fn save_rebakes_over_a_tileset_changed_since_the_stroke() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[10, 10], [11, 10]], "height": 1}]}),
    )
    .unwrap();
    let stroke = st.map(0).unwrap().clone();
    assert!(stroke.permissions.iter().all(|c| c.kind != 0x02));
    // Tile 0 becomes grass on disk, past the store (no `forget_world_bakes`).
    let mut tileset = Tileset::synthetic();
    tileset.tiles[0].kind = 0x02;
    fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&tileset));
    st.save().unwrap();
    let saved = MapFile::read(&member(&fx), Family::Pt).unwrap();
    // Every cell with tile 0 follows the new tileset.
    let grid = WorldGrid::parse(st.world(0).unwrap().unwrap()).unwrap();
    let tiled: Vec<usize> = (0..32 * 32)
        .filter(|&i| grid.layers[0].tiles[i].is_some_and(|t| t.tile == 0))
        .collect();
    assert!(!tiled.is_empty());
    for i in tiled {
        assert_eq!(saved.permissions[i].kind, 0x02, "cell {i}");
    }
    assert_eq!(st.map(0).unwrap(), &saved);
    // Undo and redo still work.
    let world = st.world(0).unwrap().cloned();
    st.undo().unwrap();
    assert_eq!(st.history().0.len(), 1);
    assert_ne!(st.world(0).unwrap().cloned(), world);
    st.redo().unwrap();
    assert_eq!(st.history().0.len(), 2);
    assert_eq!(st.world(0).unwrap().cloned(), world);
}

/// Task 4 review: a bake that fails at save time (the tileset file does not read) keeps the
/// stroke's bake for that map and does not stop the save of the others.
#[test]
fn a_failing_bake_at_save_keeps_the_stroke_and_saves_the_rest() {
    let fx = world_ready();
    let mut st = store(&fx);
    create(&mut st, 0);
    edit(
        &mut st,
        json!({"map": 0, "changes": [{"cells": [[3, 3]], "tile": "0001"}]}),
    )
    .unwrap();
    let stroke = map_bytes(&mut st);
    st.edit_map(2, "Permission", |m| {
        m.permissions[0].collision ^= 0x80;
        Ok(())
    })
    .unwrap();
    let map_2 = st.map(2).unwrap().write(Family::Pt);
    fx.write(tileset_rel(0).to_str().unwrap(), b"{ not a tileset");
    let report = st.save().unwrap();
    assert!(report.written.contains(&MAP), "{:?}", report.written);
    assert_eq!(member(&fx), stroke);
    assert_eq!(
        std::fs::read(fx.project().member_path(AssetKind::Map, 2)).unwrap(),
        map_2
    );
    // Not recorded: the next build bakes it and reports.
    assert!(
        !crate::build::state::BuildState::load(fx.root())
            .world
            .contains_key(&0)
    );
}
