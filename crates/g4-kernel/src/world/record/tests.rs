//! The world records on `world_built()`: maps 0 and 1 world maps, saved and built once.

use std::sync::Arc;

use super::*;
use crate::build::state::state_path;
use crate::test_support::{TextFixture, world_built};
use crate::textproject::canon::to_canonical;
use crate::world::grid::set_token;
use crate::world::keep::{world_field, world_parts};
use crate::world::tileset::{Tileset, tileset_rel};

fn hash(fx: &TextFixture, map: u16) -> Option<String> {
    input_hash(&mut Store::new(Arc::new(fx.project())), map).unwrap()
}

fn side(fx: &TextFixture, map: u16) -> Option<WorldSide> {
    world_side(&mut Store::new(Arc::new(fx.project())), map).unwrap()
}

/// Paints tile 1 at (5, 5) of map `map` in its JSON, as Git would.
fn paint_json(fx: &TextFixture, map: u16) {
    let rel = format!("data/maps/{map:04}.json");
    let bytes = fx.read(&rel);
    let mut world = world_field(&bytes).unwrap().unwrap();
    world.layers[0].tiles[5] = set_token(&world.layers[0].tiles[5], 5, "0001").unwrap();
    fx.write(&rel, &world_parts(bytes, &world, &mut Vec::new()).unwrap());
}

#[test]
fn world_salt_names_the_three_revisions() {
    assert_eq!(world_salt(), "world#1+bake#1+bdhc#1");
}

#[test]
fn input_hash_moves_with_json_tileset_and_package() {
    let fx = world_built();
    let before = hash(&fx, 0).expect("a world map");
    assert_eq!(before.len(), 16);
    assert_eq!(hash(&fx, 0).unwrap(), before, "stable");
    // Map 2 is binary.
    assert_eq!(hash(&fx, 2), None);

    // The JSON.
    let json = fx.read("data/maps/0000.json");
    paint_json(&fx, 0);
    assert_ne!(hash(&fx, 0).unwrap(), before);
    fx.write("data/maps/0000.json", &json);
    assert_eq!(hash(&fx, 0).unwrap(), before);

    // The tileset file.
    let rel = tileset_rel(0);
    let rel = rel.to_str().unwrap();
    let tileset = fx.read(rel);
    let mut changed = Tileset::synthetic();
    changed.tiles[0].kind = 0x02;
    fx.write(rel, &to_canonical(&changed));
    assert_ne!(hash(&fx, 0).unwrap(), before);
    fx.write(rel, &tileset);
    assert_eq!(hash(&fx, 0).unwrap(), before);

    // The package member.
    let pack = fx.project().member_path(AssetKind::MapTexture, 0);
    let bytes = std::fs::read(&pack).unwrap();
    let mut other = bytes.clone();
    let last = other.len() - 1;
    other[last] ^= 0xFF;
    std::fs::write(&pack, &other).unwrap();
    assert_ne!(hash(&fx, 0).unwrap(), before);
    std::fs::write(&pack, &bytes).unwrap();
    assert_eq!(hash(&fx, 0).unwrap(), before);
}

#[test]
fn side_is_member_changed_after_an_outside_write() {
    let fx = world_built();
    assert_eq!(side(&fx, 0), Some(WorldSide::InStep));
    assert_eq!(side(&fx, 2), None);
    let path = fx.project().member_path(AssetKind::Map, 0);
    let member = std::fs::read(&path).unwrap();
    let mut outside = member.clone();
    outside[0x10] ^= 0x01;
    std::fs::write(&path, &outside).unwrap();
    assert_eq!(side(&fx, 0), Some(WorldSide::MemberChanged));
    paint_json(&fx, 0);
    assert_eq!(side(&fx, 0), Some(WorldSide::Both));
    // A missing member is baked again, never a conflict.
    std::fs::remove_file(&path).unwrap();
    assert_eq!(side(&fx, 0), Some(WorldSide::SourceChanged));
    std::fs::write(&path, &member).unwrap();
    assert_eq!(side(&fx, 0), Some(WorldSide::SourceChanged));
    assert_eq!(side(&fx, 1), Some(WorldSide::InStep));
    std::fs::remove_file(state_path(fx.root())).unwrap();
    assert_eq!(side(&fx, 1), Some(WorldSide::Unrecorded));
}

#[test]
fn record_world_and_forget_world_keep_the_others() {
    let fx = world_built();
    let project = fx.project();
    let before = BuildState::load(fx.root()).world;
    let record = WorldRecord {
        input: "0".repeat(16),
        member: "1".repeat(16),
    };
    record_world(&project, &[(7, record.clone())]).unwrap();
    let after = BuildState::load(fx.root()).world;
    assert_eq!(after[&7], record);
    assert_eq!(after[&0], before[&0]);
    forget_world(&project, &[7]).unwrap();
    assert_eq!(BuildState::load(fx.root()).world, before);
}
