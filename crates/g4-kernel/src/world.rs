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
//! changed into their members ([`build::bake_world_maps`]); `adopt.rs` „Karte als Quelle
//! übernehmen“ (Form 1: model and BDHC verbatim, [`adopt::world_adopt`]; Form 2: the model as a
//! glTF side file through apicula, [`adopt::world_adopt_with`]). `tileset.rs` also edits
//! the tiles of a tileset file as one step ([`tileset::tileset_edit`]) and sets the model of a
//! piece tile ([`tileset::tileset_piece_set`]).
//! `freeform.rs` the canonical side file of a free-form model (6.5b, [`freeform::side_glb`],
//! [`freeform::side_faces`]), the rules for side file names, and adding and removing free-form
//! models ([`freeform::world_freeform_add`], [`freeform::world_freeform_remove`]), each one undo
//! step with the pack and the side file.

pub mod adopt;
pub mod bake;
pub mod build;
pub mod check;
pub mod doc;
pub mod edit;
pub mod freeform;
pub mod grid;
pub mod keep;
pub mod mesh;
pub mod places;
pub mod record;
pub mod tileset;

pub use adopt::{
    ADOPT_NEEDS_APICULA, ADOPT_TEXTURE_CODE, ADOPT_TOLERANCE, AdoptForm, BakedGeometry,
    FORM2_SIDE_NAME, Form2Report, HeightDeviation, VanillaGeometry, WorldAdoptReport,
    WorldAdoptRequest, adopt_label, world_adopt, world_adopt_glb, world_adopt_with,
};
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
pub use mesh::{PieceModels, mesh_with};
pub use places::{maps_of_packages, packs_of_map};
pub use record::{WorldSide, input_hash, member_hash, record_world, world_salt, world_side};
pub use tileset::{
    DEFAULT_RULE, Deriver, HeightBehavior, PieceSetReport, PieceSetRequest, TILESET_VERSION,
    TILESETS_DIR, TileChange, TileJson, TileOrigin, TileRole, Tileset, TilesetEditReport,
    TilesetEditRequest, TilesetSaveReport, UNIQUE_SHARE, VoteRule, derive_tileset, piece_set_label,
    read_tileset_file, save_tileset, textures_only, tile_outside, tileset_edit, tileset_piece_set,
    tileset_problems, tileset_rel, tileset_texture,
};

pub use edit::WORLD_NEEDS_TEXT_PROJECT;
pub use freeform::{
    APICULA_UNIT, FREEFORM_LAYER, FREEFORM_UNIT, FreeformAddRequest, FreeformReport,
    SIDE_FILE_CODE, SIDE_NAME_MAX, SideModels, WorldFaces, freeform_file, freeform_label,
    freeform_rel, freeform_remove_label, load_side_models, piece_rel, side_faces, side_glb,
    side_name, side_name_of, world_faces, world_freeform_add, world_freeform_inspect,
    world_freeform_remove,
};
