//! The world document of a map (phase 6.3a): a map file of the text project with
//! `source: "world"` carries the object `world`, the painted form of the map (layers of tiles,
//! the heights and ramps of the ground, pieces, permissions derived from the tileset or set by
//! hand). Until the baker of 6.3b the build still writes the map's former member; the world is
//! shown in the scene through the mesher.
//!
//! `doc.rs` holds the document as it is on disk (serde, canonical key order, unknown keys
//! refused) and the synthetic fixtures; `grid.rs` the token grammar of its rows and the schema
//! check that reads it into cells ([`WorldGrid::parse`]).

pub mod doc;
pub mod grid;

pub use doc::{
    FreeformJson, HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, MAP_SOURCE_WORLD, PermissionsJson,
    PieceJson, STEP_UNITS, TILE_UNITS, WORLD_VERSION, WorldJson,
};
pub use grid::{
    GridLayer, Ramp, TileRef, WorldError, WorldGrid, override_token, parse_height, parse_override,
    parse_ramp, parse_tile, ramp_token, set_token, tile_token,
};
