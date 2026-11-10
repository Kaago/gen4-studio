//! The build step `world` and [`bake_world_maps`] on `world_built()`: maps 0 and 1 world maps
//! on package 0, saved and built once.

use std::sync::Arc;

use g4_formats::game::Family;
use serde_json::json;

use super::*;
use crate::build::state::{BuildState, state_path};
use crate::build::step::{StepReport, StepStatus};
use crate::build::{Prepared, prepare};
use crate::test_support::{TextFixture, world_built};
use crate::textproject::canon::to_canonical;
use crate::world::doc::fixtures::oversize_world;
use crate::world::edit::{WorldEditRequest, world_edit};
use crate::world::grid::set_token;
use crate::world::keep::world_parts;
use crate::world::record::{input_hash, world_side};
use crate::world::tileset::{Tileset, tileset_rel};
use crate::world::{WorldJson, WorldSide};

fn json_rel(map: u16) -> String {
    format!("data/maps/{map:04}.json")
}

fn member(fx: &TextFixture, map: u16) -> Vec<u8> {
    std::fs::read(fx.project().member_path(AssetKind::Map, map)).unwrap()
}

fn write_member(fx: &TextFixture, map: u16, bytes: &[u8]) {
    std::fs::write(fx.project().member_path(AssetKind::Map, map), bytes).unwrap();
}

/// The member a fresh store bakes for `map` from the files on disk.
fn bake_of(fx: &TextFixture, map: u16) -> Vec<u8> {
    let mut store = Store::new(Arc::new(fx.project()));
    store.map(map).unwrap().write(Family::Pt)
}

fn world_of(fx: &TextFixture, map: u16) -> WorldJson {
    world_field(&fx.read(&json_rel(map))).unwrap().unwrap()
}

/// Writes the JSON of `map` with `world`, as Git or an editor would (no store, no record).
fn write_world(fx: &TextFixture, map: u16, world: &WorldJson) {
    let rel = json_rel(map);
    let file = world_parts(fx.read(&rel), world, &mut Vec::new()).unwrap();
    fx.write(&rel, &file);
}

/// `map`'s world with tile 1 (grass) painted at (5, 5).
fn painted(fx: &TextFixture, map: u16) -> WorldJson {
    let mut world = world_of(fx, map);
    world.layers[0].tiles[5] = set_token(&world.layers[0].tiles[5], 5, "0001").unwrap();
    world
}

fn row<'a>(prepared: &'a Prepared, id: &str) -> &'a StepReport {
    prepared
        .steps
        .iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}: {:?}", prepared.steps))
}

fn compile_problems(result: Result<Prepared, KernelError>) -> Vec<Problem> {
    match result {
        Err(KernelError::Compile(problems)) => problems,
        other => panic!("{other:?}"),
    }
}

/// The changes that paint map 0 over the model budget (as `store_tests`: three tiles, 39
/// heights).
fn oversize_changes() -> serde_json::Value {
    let all = || (0..32u8).flat_map(|z| (0..32u8).map(move |x| [x, z]));
    let mut changes = Vec::new();
    for tile in 0..3u16 {
        let cells: Vec<[u8; 2]> = all()
            .filter(|&[x, z]| (u16::from(x) + u16::from(z)) % 3 == tile)
            .collect();
        changes.push(json!({"cells": cells, "tile": format!("{tile:04}")}));
    }
    for h in 1..40usize {
        let cells: Vec<[u8; 2]> = all()
            .filter(|&[x, z]| (7 * usize::from(x) + 13 * usize::from(z)) % 40 == h)
            .collect();
        if !cells.is_empty() {
            changes.push(json!({"cells": cells, "height": h}));
        }
    }
    json!({"map": 0, "changes": changes})
}

#[test]
fn world_step_runs_between_textproject_and_rotom() {
    let fx = world_built();
    let prepared = prepare(&fx.project()).unwrap();
    let ids: Vec<&str> = prepared.steps.iter().map(|r| r.id.as_str()).collect();
    let at = |id: &str| ids.iter().position(|i| *i == id).unwrap();
    assert_eq!(at("world"), at("textproject") + 1, "{ids:?}");
    if let Some(rotom) = ids.iter().position(|i| *i == "rotom") {
        assert!(rotom > at("world"), "{ids:?}");
    }
    // Nothing changed since the fixture's build: skipped.
    assert_eq!(row(&prepared, "world").status, StepStatus::Skipped);
    // The fixture's build baked both maps and recorded them.
    let state = BuildState::load(fx.root());
    assert_eq!(state.world.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));
}

#[test]
fn world_step_bakes_one_map_for_one_changed_json() {
    let fx = world_built();
    let member_0 = member(&fx, 0);
    write_world(&fx, 1, &painted(&fx, 1));
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!(world.status, StepStatus::Incremental, "{world:?}");
    assert_eq!((world.done, world.total), (1, 2));
    assert_eq!(member(&fx, 1), bake_of(&fx, 1));
    // Grass (kind 0x02) at (5, 5): the member follows the JSON.
    let map = g4_formats::map::MapFile::read(&member(&fx, 1), Family::Pt).unwrap();
    assert_eq!(map.permissions[5 * 32 + 5].kind, 0x02);
    assert_eq!(member(&fx, 0), member_0);

    // Directly: one map changed, the other in step.
    write_world(&fx, 1, &world_of(&fx, 0));
    let build = bake_world_maps(&fx.project(), &BakeScope::All, BakeMode::Step).unwrap();
    assert_eq!(build.baked, [1]);
    assert_eq!(build.skipped, 1);
    assert!(build.problems.is_empty(), "{:?}", build.problems);
}

#[test]
fn a_changed_tileset_rebakes_its_maps_only() {
    let fx = world_built();
    let members = (member(&fx, 0), member(&fx, 1));
    // A tileset of package 1, which no world map uses: the step runs, bakes nothing.
    let mut other = Tileset::synthetic();
    other.package = 1;
    fx.write(tileset_rel(1).to_str().unwrap(), &to_canonical(&other));
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!(world.status, StepStatus::Incremental, "{world:?}");
    assert_eq!((world.done, world.total), (0, 2));
    assert_eq!((member(&fx, 0), member(&fx, 1)), members);

    // Tile 0 of package 0 becomes grass: both maps on it bake anew.
    let mut tileset = Tileset::synthetic();
    tileset.tiles[0].kind = 0x02;
    fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&tileset));
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!((world.done, world.total), (2, 2), "{world:?}");
    for map in [0, 1] {
        let file = g4_formats::map::MapFile::read(&member(&fx, map), Family::Pt).unwrap();
        assert!(file.permissions.iter().any(|c| c.kind == 0x02), "map {map}");
        assert_eq!(member(&fx, map), bake_of(&fx, map));
    }
}

#[test]
fn a_changed_member_fails_the_step_and_writes_nothing() {
    let fx = world_built();
    // DSPRE writes member 0; map 1's world changes too.
    let mut outside = member(&fx, 0);
    outside[0x10] ^= 0x01;
    write_member(&fx, 0, &outside);
    write_world(&fx, 1, &painted(&fx, 1));
    let member_1 = member(&fx, 1);
    let record = BuildState::load(fx.root()).world;

    let problems = compile_problems(prepare(&fx.project()));
    let changed: Vec<&Problem> = problems
        .iter()
        .filter(|p| p.message.starts_with("world.member-changed:"))
        .collect();
    assert_eq!(changed.len(), 1, "{problems:#?}");
    assert_eq!(changed[0].file, std::path::Path::new("data/maps/0000.json"));
    assert_eq!(changed[0].severity, Severity::Error);
    assert_eq!(
        changed[0].message,
        "world.member-changed: Map 0000: The member was changed outside the Studio (DSPRE?). In Technical › Text project, “Bake again” or “Adopt as binary map”."
    );
    // Nothing written, nothing recorded.
    assert_eq!(member(&fx, 0), outside);
    assert_eq!(member(&fx, 1), member_1);
    assert_eq!(BuildState::load(fx.root()).world, record);

    // „Neu backen“ bakes over it.
    let build =
        bake_world_maps(&fx.project(), &BakeScope::Maps(vec![0]), BakeMode::Rebake).unwrap();
    assert_eq!(build.baked, [0]);
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(world_side(&mut store, 0).unwrap(), Some(WorldSide::InStep));
}

#[test]
fn without_a_record_the_world_wins() {
    let fx = world_built();
    let mut outside = member(&fx, 0);
    outside[0x10] ^= 0x01;
    write_member(&fx, 0, &outside);
    std::fs::remove_file(state_path(fx.root())).unwrap();
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!(world.status, StepStatus::Full, "{world:?}");
    assert_eq!((world.done, world.total), (2, 2));
    assert!(
        prepared
            .problems
            .iter()
            .all(|p| !p.message.starts_with("world.")),
        "{:#?}",
        prepared.problems
    );
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));
    assert_eq!(BuildState::load(fx.root()).world.len(), 2);
}

#[test]
fn clear_cache_keeps_world_records_and_the_next_build_is_quiet() {
    let fx = world_built();
    let record = BuildState::load(fx.root()).world;
    crate::build::clear_cache(&fx.project()).unwrap();
    assert_eq!(BuildState::load(fx.root()).world, record);
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!(
        (world.status, world.done),
        (StepStatus::Full, 0),
        "{world:?}"
    );
}

#[test]
fn textproject_step_leaves_world_maps_alone() {
    let fx = world_built();
    let quiet = |prepared: &Prepared| {
        prepared.problems.iter().all(|p| {
            !(p.message.starts_with("textproject.member-newer")
                || p.message.starts_with("textproject.unsynced")
                || p.message.starts_with("textproject.conflict"))
        })
    };
    // Full run (no state): the world map's member differs from the base, no unit.
    std::fs::remove_file(state_path(fx.root())).unwrap();
    let prepared = prepare(&fx.project()).unwrap();
    assert!(quiet(&prepared), "{:#?}", prepared.problems);
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));

    // Incremental: the world step's member writes are no member side change for it.
    write_world(&fx, 0, &painted(&fx, 0));
    let prepared = prepare(&fx.project()).unwrap();
    assert!(quiet(&prepared), "{:#?}", prepared.problems);
    assert_eq!(row(&prepared, "world").done, 1);
    let prepared = prepare(&fx.project()).unwrap();
    assert!(quiet(&prepared), "{:#?}", prepared.problems);
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));
}

#[test]
fn the_import_bakes_world_maps() {
    let fx = world_built();
    let baked = member(&fx, 0);
    std::fs::remove_file(fx.project().member_path(AssetKind::Map, 0)).unwrap();
    let report = crate::textproject::import(&fx.project(), Some(&fx.base())).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != Severity::Error),
        "{:#?}",
        report.problems
    );
    assert_eq!(member(&fx, 0), baked);
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(world_side(&mut store, 0).unwrap(), Some(WorldSide::InStep));
}

#[test]
fn a_refused_bake_fails_the_step_with_world_bake() {
    let fx = world_built();
    let member_0 = member(&fx, 0);
    write_world(&fx, 0, &oversize_world());
    let problems = compile_problems(prepare(&fx.project()));
    let bake: Vec<&Problem> = problems
        .iter()
        .filter(|p| p.message.starts_with("world.bake:"))
        .collect();
    assert_eq!(bake.len(), 1, "{problems:#?}");
    assert_eq!(bake[0].file, std::path::Path::new("data/maps/0000.json"));
    assert!(
        bake[0]
            .message
            .starts_with("world.bake: world: bake.model-too-big: The baked model has"),
        "{}",
        bake[0].message
    );
    assert_eq!(member(&fx, 0), member_0);
}

#[test]
fn a_deleted_world_map_is_not_baked() {
    let fx = world_built();
    let member_1 = member(&fx, 1);
    write_world(&fx, 1, &painted(&fx, 1));
    let rel = json_rel(1);
    let mut v: serde_json::Value = serde_json::from_slice(&fx.read(&rel)).unwrap();
    v["deleted"] = true.into();
    fx.write(&rel, &serde_json::to_vec_pretty(&v).unwrap());
    assert_eq!(world_maps(&fx.project()).unwrap(), [0]);
    let prepared = prepare(&fx.project()).unwrap();
    let world = row(&prepared, "world");
    assert_eq!((world.done, world.total), (0, 1), "{world:?}");
    assert_eq!(member(&fx, 1), member_1);
    let build =
        bake_world_maps(&fx.project(), &BakeScope::Maps(vec![1]), BakeMode::Rebake).unwrap();
    assert!(build.baked.is_empty(), "{build:?}");
}

#[test]
fn save_records_the_world() {
    let fx = world_built();
    let mut store = Store::new(Arc::new(fx.project()));
    let req: WorldEditRequest = serde_json::from_value(
        json!({"map": 0, "changes": [{"cells": [[3, 3], [4, 3]], "tile": "0001"}]}),
    )
    .unwrap();
    world_edit(&mut store, &req).unwrap();
    store.save().unwrap();
    let record = BuildState::load(fx.root()).world[&0].clone();
    let mut fresh = Store::new(Arc::new(fx.project()));
    assert_eq!(
        record.input,
        input_hash(&mut fresh, 0).unwrap().expect("a world map")
    );
    assert_eq!(record.member, crate::world::member_hash(&member(&fx, 0)));
    assert_eq!(world_side(&mut fresh, 0).unwrap(), Some(WorldSide::InStep));
    // The next build bakes nothing.
    let prepared = prepare(&fx.project()).unwrap();
    assert_eq!(row(&prepared, "world").done, 0);

    // A save with a refused bake leaves the map unrecorded: the build reports `world.bake`.
    let mut store = Store::new(Arc::new(fx.project()));
    let req: WorldEditRequest = serde_json::from_value(oversize_changes()).unwrap();
    world_edit(&mut store, &req).unwrap();
    store.save().unwrap();
    assert!(!BuildState::load(fx.root()).world.contains_key(&0));
    let problems = compile_problems(prepare(&fx.project()));
    assert!(
        problems.iter().any(|p| p
            .message
            .starts_with("world.bake: world: bake.model-too-big")),
        "{problems:#?}"
    );
}

const MOVE_1_JSON: &str = "data/moves/0001.json";
const MOVE_1_MEMBER: &str = "unpacked/moveData/0001";

/// DSPRE changed member 0, and the JSON of move 1 changed (power 99).
fn dspre_member_and_an_edited_move(fx: &TextFixture) -> Vec<u8> {
    let mut outside = member(fx, 0);
    outside[0x10] ^= 0x01;
    write_member(fx, 0, &outside);
    let mut v: serde_json::Value = serde_json::from_slice(&fx.read(MOVE_1_JSON)).unwrap();
    v["data"]["power"] = 99.into();
    fx.write(MOVE_1_JSON, &to_canonical(&v));
    outside
}

#[test]
fn the_import_refuses_a_world_member_changed_outside_before_writing() {
    let fx = world_built();
    let outside = dspre_member_and_an_edited_move(&fx);
    let move_1 = fx.read(MOVE_1_MEMBER);
    let refused = crate::textproject::import(&fx.project(), Some(&fx.base()));
    let problems = match refused {
        Err(KernelError::Compile(problems)) => problems,
        other => panic!("{other:?}"),
    };
    assert!(
        problems.iter().any(|p| p.severity == Severity::Error
            && p.file == std::path::Path::new("data/maps/0000.json")
            && p.message.starts_with("world.member-changed: Map 0000:")),
        "{problems:#?}"
    );
    // Nothing written.
    assert_eq!(member(&fx, 0), outside);
    assert_eq!(fx.read(MOVE_1_MEMBER), move_1);
}

#[test]
fn the_import_with_force_bakes_over_a_world_member_changed_outside() {
    let fx = world_built();
    dspre_member_and_an_edited_move(&fx);
    let move_1 = fx.read(MOVE_1_MEMBER);
    let opts = crate::textproject::ImportOptions { force: true };
    let report = crate::textproject::import_with(&fx.project(), Some(&fx.base()), &opts).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != Severity::Error),
        "{:#?}",
        report.problems
    );
    assert_ne!(fx.read(MOVE_1_MEMBER), move_1);
    assert_eq!(member(&fx, 0), bake_of(&fx, 0));
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(world_side(&mut store, 0).unwrap(), Some(WorldSide::InStep));
}

#[test]
fn a_failing_textproject_step_leaves_world_members_alone() {
    let fx = world_built();
    write_world(&fx, 1, &painted(&fx, 1));
    let member_1 = member(&fx, 1);
    let record = BuildState::load(fx.root()).world;
    fx.write(MOVE_1_JSON, b"{ not json");
    let problems = compile_problems(prepare(&fx.project()));
    assert!(
        problems
            .iter()
            .any(|p| p.file == std::path::Path::new(MOVE_1_JSON)),
        "{problems:#?}"
    );
    assert_eq!(member(&fx, 1), member_1);
    assert_eq!(BuildState::load(fx.root()).world, record);
}

/// A Git branch switch: map 0 is a world map on branch A (recorded), binary on branch B (the
/// step `textproject` writes its binary member), and a world again, byte for byte, back on A.
/// The binary write ended the record, so the member the step wrote is no outside change: the
/// map bakes without `world.member-changed`.
#[test]
fn a_branch_switch_through_a_binary_map_is_no_member_change() {
    let fx = world_built();
    let p = fx.project();
    let rel = json_rel(0);
    let world_a = fx.read(&rel);
    let state_a = std::fs::read(state_path(fx.root())).unwrap();
    // Branch B: map 0 binary, one permission cell other than the baked member's.
    take_member_as_binary(&p, 0).unwrap();
    let mut binary: serde_json::Value = serde_json::from_slice(&fx.read(&rel)).unwrap();
    let row = binary["permissions"][0].as_str().unwrap().to_owned();
    binary["permissions"][0] = set_token(&row, 0, "0180").unwrap().into();
    // `.g4/` is no part of the branch: the record of branch A stays.
    std::fs::write(state_path(fx.root()), &state_a).unwrap();
    assert!(BuildState::load(fx.root()).world.contains_key(&0));
    fx.write(&rel, &serde_json::to_vec_pretty(&binary).unwrap());
    let member_a = member(&fx, 0);
    prepare(&p).unwrap();
    assert_ne!(member(&fx, 0), member_a, "the step wrote the binary member");

    // Back on branch A.
    fx.write(&rel, &world_a);
    let mut store = Store::new(Arc::new(fx.project()));
    let side = world_side(&mut store, 0).unwrap();
    assert!(
        !matches!(side, Some(WorldSide::MemberChanged | WorldSide::Both)),
        "{side:?}"
    );
    prepare(&p).unwrap();
    assert_eq!(member(&fx, 0), member_a);
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(world_side(&mut store, 0).unwrap(), Some(WorldSide::InStep));
}
