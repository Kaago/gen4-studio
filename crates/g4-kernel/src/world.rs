//! The world document of a map (phase 6.3a): a map file of the text project with
//! `source: "world"` carries the object `world`, the painted form of the map (layers of tiles,
//! the heights and ramps of the ground, pieces, permissions derived from the tileset or set by
//! hand). Until the baker of 6.3b the build still writes the map's former member; the world is
//! shown in the scene through the mesher.
//!
//! `doc.rs` holds the document as it is on disk (serde, canonical key order, unknown keys
//! refused) and the synthetic fixtures; `grid.rs` the token grammar of its rows and the schema
//! check that reads it into cells ([`WorldGrid::parse`]); `tileset.rs` the tileset file
//! `data/tilesets/NNNN.json` of a map texture package, its derivation from the package and the
//! vanilla permissions, and the synthetic test tileset; `places.rs` the map texture packages a
//! map is shown with and the maps of every package; `mesh.rs` the mesher from layers, heights,
//! ramps and footprints to a face list and the budget estimate; `keep.rs` the world object
//! kept when the map file is written from its member; `edit.rs` „Welt anlegen“
//! ([`edit::world_create`]) and the changes of cells ([`edit::world_edit`]), each one undo step
//! on the store's world map asset ([`crate::project::AssetKind::WorldMap`]).

pub mod doc;
pub mod edit;
pub mod grid;
pub mod keep;
pub mod mesh;
pub mod places;
pub mod tileset;

pub use doc::{
    FreeformJson, HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, MAP_SOURCE_WORLD, PermissionsJson,
    PieceJson, STEP_UNITS, TILE_UNITS, WORLD_VERSION, WorldJson,
};
pub use grid::{
    GridLayer, Ramp, TileRef, WorldError, WorldGrid, override_token, parse_height, parse_override,
    parse_ramp, parse_tile, ramp_token, set_token, tile_token,
};
pub use places::{maps_of_packages, packs_of_map};
pub use tileset::{
    DEFAULT_RULE, Deriver, HeightBehavior, TILESET_VERSION, TILESETS_DIR, TileJson, TileOrigin,
    TileRole, Tileset, TilesetSaveReport, UNIQUE_SHARE, VoteRule, derive_tileset,
    read_tileset_file, save_tileset, textures_only, tile_outside, tileset_problems, tileset_rel,
    tileset_texture,
};

pub use edit::WORLD_NEEDS_TEXT_PROJECT;
