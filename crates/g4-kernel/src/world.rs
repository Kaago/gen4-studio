//! The world document of a map (phase 6.3a): a map file of the text project with
//! `source: "world"` carries the object `world`, the painted form of the map (layers of tiles,
//! the heights and ramps of the ground, pieces, permissions derived from the tileset or set by
//! hand). The map's model, BDHC and permissions are baked from it (6.3b); the world is shown in
//! the scene through the mesher.
//!
//! `doc.rs` holds the document as it is on disk (serde, canonical key order, unknown keys
//! refused) and the synthetic fixtures; `grid.rs` the token grammar of its rows and the schema
//! check that reads it into cells ([`WorldGrid::parse`]); `tileset.rs` the tileset file
//! `data/tilesets/NNNN.json` of a map texture package, its derivation from the package and the
//! vanilla permissions, and the synthetic test tileset; `places.rs` the map texture packages a
//! map is shown with and the maps of every package; `mesh.rs` the mesher from layers, heights,
//! ramps and footprints to a face list and the budget estimate; `keep.rs` the world object
//! kept when the map file is written from its member, without the derived parts
//! ([`keep::world_parts`]); `edit.rs` „Welt anlegen“
//! ([`edit::world_create`]), the changes of cells ([`edit::world_edit`]) and „Als binäre Karte
//! übernehmen“ ([`edit::world_to_binary`]), each one undo step on the store's world map asset
//! ([`crate::project::AssetKind::WorldMap`]) and the map it bakes into; `check.rs` the checks
//! `world.schema`, `world.tileset-mismatch`, `world.bake`, `world.budget` and
//! `world.member-changed` of the
//! text-project check ([`check::world_problems`]); `bake.rs` the baker of a world into the map's
//! model, BDHC and permissions ([`bake::bake_grid`], [`bake::assemble`], with the store
//! [`bake::bake_world`]): in a text project a world map's Map asset is its bake; `record.rs` the
//! record of each world map's last bake in `.g4/build/state.json` ([`record::world_side`]);
//! `build.rs` the build step `world` after `textproject`, which bakes the world maps whose world
//! changed into their members ([`build::bake_world_maps`]).

pub mod bake;
pub mod build;
pub mod check;
pub mod doc;
pub mod edit;
pub mod grid;
pub mod keep;
pub mod mesh;
pub mod places;
pub mod record;
pub mod tileset;

pub use bake::{
    BakeEnv, EMPTY_CELL, FLOOR_TOLERANCE, GridBake, Verbatim, WORLD_BAKE_REV, WorldBake,
    WorldBakeMessage, WorldBakeReport, assemble, bake_grid, bake_world, report,
    world_derived_refusal, world_permissions,
};
pub use build::{BakeMode, BakeScope, WORLD_STEP_ID, WorldBuild, bake_world_maps};
pub use doc::{
    FreeformJson, HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, MAP_SOURCE_WORLD, PermissionsJson,
    PieceJson, STEP_UNITS, TILE_UNITS, TerrainJson, WORLD_VERSION, WorldJson,
};
pub use grid::{
    GridLayer, Ramp, TileRef, WorldError, WorldGrid, error_cell, override_token, parse_height,
    parse_override, parse_ramp, parse_tile, ramp_token, set_token, tile_token,
};
pub use keep::world_parts;
pub use places::{maps_of_packages, packs_of_map};
pub use record::{WorldSide, input_hash, member_hash, record_world, world_salt, world_side};
pub use tileset::{
    DEFAULT_RULE, Deriver, HeightBehavior, TILESET_VERSION, TILESETS_DIR, TileJson, TileOrigin,
    TileRole, Tileset, TilesetSaveReport, UNIQUE_SHARE, VoteRule, derive_tileset,
    read_tileset_file, save_tileset, textures_only, tile_outside, tileset_problems, tileset_rel,
    tileset_texture,
};

pub use edit::WORLD_NEEDS_TEXT_PROJECT;
