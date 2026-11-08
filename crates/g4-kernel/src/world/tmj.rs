//! The exchange of a world map with Tiled (6.5b): the world as a Tiled map (TMJ, the JSON
//! form documented at doc.mapeditor.org) with its tile images, so a map can be painted in
//! Tiled and read back. TMJ is an exchange format, not a world format: the data are the tile
//! indices, the images are only for the eye. Only the documented format is used, no code
//! from Tiled (GPL).
//!
//! The mapping: every world layer is a `tilelayer` (GID = tile + 1 with the turn as Tiled's
//! flip bits, [`gid_flags`]); the ground's heights are the `tilelayer` „Höhe“ on the helper
//! tileset „g4 Höhen“ (79 tiles for −16 to 62), its ramps the `tilelayer` „Rampen“ on „g4
//! Rampen“ (only when the world has `ramps`); the pieces are tile objects in the
//! `objectgroup` „Teilmodelle“, the permission overrides rectangles in „Permissions“ (only
//! when the world has `override`), the map's buildings points in „Gebäude (nur Ansicht)“. The
//! three tilesets are embedded image collections with fixed first GIDs: the map's tiles from
//! 1, the heights from 1 + N, the ramps from 1 + N + 79 (N tiles in the tileset).
//!
//! The serde types are the subset of TMJ this module writes; reading ignores unknown fields
//! (Tiled adds `class`, `parallaxx`, `tintcolor` …) and fills missing ones with defaults.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use g4_formats::map::{Building, MAP_SIZE};
use g4_formats::nsbtx::Rgba;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::doc::{HEIGHT_MAX, HEIGHT_MIN, LayerRole, TILE_UNITS, WorldJson};
use super::edit::not_a_world_map;
use super::grid::{Ramp, WorldGrid};
use super::tileset::{TileJson, TileRole, Tileset, tileset_texture};
use crate::error::{KernelError, io_at};
use crate::store::Store;

/// The version of the TMJ mapping (property `g4.version`).
pub const TMJ_VERSION: i64 = 1;
/// The property `g4.format` of a TMJ the studio wrote.
pub const TMJ_FORMAT: &str = "gen4-studio-world";
/// Tiles of the helper tileset „g4 Höhen“: one per height step from −16 to 62.
pub const HEIGHT_TILES: u32 = 79;
/// Tiles of the helper tileset „g4 Rampen“, in this order.
pub const RAMP_TILES: [&str; 8] = ["n1", "n2", "e1", "e2", "s1", "s2", "w1", "w2"];
/// Tiled's flip bits of a GID: horizontal, vertical, diagonal (applied D, then H, then V)
/// and the hexagonal turn (refused).
pub const FLIP_H: u32 = 0x8000_0000;
pub const FLIP_V: u32 = 0x4000_0000;
pub const FLIP_D: u32 = 0x2000_0000;
pub const FLIP_HEX: u32 = 0x1000_0000;

/// The TMJ `version` the studio writes.
pub const TMJ_FILE_VERSION: &str = "1.10";
/// The TMJ `tiledversion` the studio writes.
pub const TILED_VERSION: &str = "1.10.2";
/// Pixels per tile in the TMJ (one texel per world unit).
pub const TMJ_TILE: u32 = TILE_UNITS as u32;

/// Names of the layers and tilesets the studio writes.
pub const HEIGHT_LAYER: &str = "Höhe";
pub const RAMP_LAYER: &str = "Rampen";
pub const PIECE_LAYER: &str = "Teilmodelle";
pub const PERMISSION_LAYER: &str = "Permissions";
pub const BUILDING_LAYER: &str = "Gebäude (nur Ansicht)";
pub const HEIGHT_TILESET: &str = "g4 Höhen";
pub const RAMP_TILESET: &str = "g4 Rampen";

/// The TMJ subset this module writes and reads (serde; unknown fields ignored on read).
/// Keys in Tiled's order (alphabetical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjMap {
    #[serde(default = "minus_one")]
    pub compressionlevel: i64,
    pub height: u32,
    #[serde(default)]
    pub infinite: bool,
    #[serde(default)]
    pub layers: Vec<TmjLayer>,
    #[serde(default)]
    pub nextlayerid: u32,
    #[serde(default)]
    pub nextobjectid: u32,
    #[serde(default)]
    pub orientation: String,
    #[serde(default)]
    pub properties: Vec<TmjProperty>,
    #[serde(default)]
    pub renderorder: String,
    #[serde(default)]
    pub tiledversion: String,
    pub tileheight: u32,
    #[serde(default)]
    pub tilesets: Vec<TmjTileset>,
    pub tilewidth: u32,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub version: String,
    pub width: u32,
}

/// A layer: a `tilelayer` (`data`, `width`, `height`, optionally `encoding` and
/// `compression`) or an `objectgroup` (`draworder`, `objects`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<TmjData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draworder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objects: Option<Vec<TmjObject>>,
    #[serde(default = "one")]
    pub opacity: f64,
    #[serde(default)]
    pub properties: Vec<TmjProperty>,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
}

/// The cells of a `tilelayer`: GIDs as a list (CSV in Tiled) or Base64 (`encoding`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TmjData {
    Gids(Vec<u32>),
    Encoded(String),
}

/// An embedded tileset; the studio writes image collections (`columns` 0, `grid` 16 × 16,
/// one image per tile). `source` (an external tileset) is only read, to refuse it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjTileset {
    #[serde(default)]
    pub columns: u32,
    #[serde(default)]
    pub firstgid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<TmjGrid>,
    #[serde(default)]
    pub margin: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub properties: Vec<TmjProperty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub spacing: u32,
    #[serde(default)]
    pub tilecount: u32,
    #[serde(default)]
    pub tileheight: u32,
    #[serde(default)]
    pub tiles: Vec<TmjTile>,
    #[serde(default)]
    pub tilewidth: u32,
}

/// The grid of an image collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjGrid {
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub orientation: String,
    #[serde(default)]
    pub width: u32,
}

/// One tile of an image collection: its image (relative to the TMJ) and properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjTile {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub imagewidth: u32,
    #[serde(default)]
    pub imageheight: u32,
    #[serde(default)]
    pub properties: Vec<TmjProperty>,
}

/// An object: a tile object (`gid`), a rectangle or a point (`point: true`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub point: bool,
    #[serde(default)]
    pub properties: Vec<TmjProperty>,
    #[serde(default)]
    pub rotation: f64,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

/// A custom property (`type` `string`, `int`, `bool` …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjProperty {
    pub name: String,
    #[serde(rename = "type", default = "string_type")]
    pub kind: String,
    #[serde(default)]
    pub value: Value,
}

fn minus_one() -> i64 {
    -1
}

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn string_type() -> String {
    "string".to_owned()
}

/// The table „Drehen und Spiegeln“: (rotate, flip, Tiled's bits). Derived from
/// `mesh::uv_of`: the cell shows texel `uv_of(x, z)`, so the image goes to the cell by the
/// inverse of that mapping, which Tiled builds from D, then H, then V.
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

/// The Tiled flip bits of a tile turned `rotate` quarter turns and mirrored with `flip`
/// (table „Drehen und Spiegeln“: the inverse of `mesh::uv_of`, image → cell).
pub fn gid_flags(rotate: u8, flip: bool) -> u32 {
    TURNS
        .iter()
        .find(|(r, f, _)| *r == rotate % 4 && *f == flip)
        .map_or(0, |&(_, _, bits)| bits)
}

/// The turn of the flip bits of `flags` (a GID or its top bits); `None` with [`FLIP_HEX`].
pub fn flags_turn(flags: u32) -> Option<(u8, bool)> {
    if flags & FLIP_HEX != 0 {
        return None;
    }
    let bits = flags & (FLIP_H | FLIP_V | FLIP_D);
    TURNS
        .iter()
        .find(|&&(_, _, b)| b == bits)
        .map(|&(r, f, _)| (r, f))
}

/// The placeholder colour of a tile's role: the design system's scene colours of the terrain
/// classes (`szene.gelaende`).
fn role_colour(role: TileRole) -> u32 {
    match role {
        TileRole::Ground => 0xBDBDBD,
        TileRole::Wall => 0xD65A00,
        TileRole::Ramp => 0xE79C00,
        TileRole::Water => 0x0073B5,
        TileRole::Grass => 0x009C73,
        TileRole::Piece => 0xCE7BA5,
    }
}

/// The design system's seven cividis steps (`szene.hoehe`), as the scene's height layer
/// colours a height: step `h` cycles through them, below 0 backwards.
const CIVIDIS: [u32; 7] = [
    0x00214A, 0x31426B, 0x636B73, 0x948C7B, 0xC6B563, 0xE7CE5A, 0xFFEF42,
];

fn height_colour(h: i8) -> u32 {
    CIVIDIS[usize::from(h.rem_euclid(7) as u8)]
}

/// The colour of a ramp tile ([`RAMP_TILES`] index): one per direction (the scene's event
/// colours), darker for two steps.
fn ramp_colour(index: usize) -> u32 {
    const DIRECTIONS: [u32; 4] = [0x4273AD, 0xAD3173, 0xCEBD42, 0x218C31];
    let colour = DIRECTIONS[(index / 2) % 4];
    if index.is_multiple_of(2) {
        colour
    } else {
        let dark = |shift: u32| (((colour >> shift) & 0xFF) * 3 / 5) << shift;
        dark(16) | dark(8) | dark(0)
    }
}

/// A 16 × 16 image filled with `rgb`.
fn solid(rgb: u32) -> Rgba {
    let [_, r, g, b] = rgb.to_be_bytes();
    let side = TMJ_TILE;
    Rgba {
        width: side,
        height: side,
        pixels: [r, g, b, 0xFF].repeat((side * side) as usize),
    }
}

/// Whether `rgba` holds its width × height pixels.
fn whole(rgba: &Rgba) -> bool {
    rgba.width > 0
        && rgba.height > 0
        && rgba.pixels.len() as u64 == u64::from(rgba.width) * u64::from(rgba.height) * 4
}

/// The image name of height step `h`: `height-p012`, `height-m016`.
fn height_name(h: i8) -> String {
    let sign = if h < 0 { 'm' } else { 'p' };
    format!("height-{sign}{:03}", h.unsigned_abs())
}

fn string_prop(name: &str, value: &str) -> TmjProperty {
    TmjProperty {
        name: name.to_owned(),
        kind: "string".to_owned(),
        value: json!(value),
    }
}

fn int_prop(name: &str, value: i64) -> TmjProperty {
    TmjProperty {
        name: name.to_owned(),
        kind: "int".to_owned(),
        value: json!(value),
    }
}

fn bool_prop(name: &str, value: bool) -> TmjProperty {
    TmjProperty {
        name: name.to_owned(),
        kind: "bool".to_owned(),
        value: json!(value),
    }
}

/// An image to write next to a TMJ: its path relative to the TMJ and its PNG bytes.
pub type TileImage = (String, Vec<u8>);

/// The images of a TMJ: each tile's image goes to `<stem>.tiles/<name>.png`.
struct Images {
    folder: String,
    files: Vec<TileImage>,
}

impl Images {
    /// The tile `id` of an image collection with image `rgba` as `<name>.png` and the
    /// properties `g4.texture` (`texture`) and `g4.role` (`role`).
    fn tile(
        &mut self,
        id: usize,
        name: &str,
        rgba: &Rgba,
        texture: &str,
        role: &str,
    ) -> Result<TmjTile, KernelError> {
        let path = format!("{}/{name}.png", self.folder);
        let png = crate::data::icons::png(&rgba.pixels, rgba.width as usize, rgba.height as usize)?;
        self.files.push((path.clone(), png));
        Ok(TmjTile {
            id: count(id)?,
            image: path,
            imagewidth: rgba.width,
            imageheight: rgba.height,
            properties: vec![
                string_prop("g4.texture", texture),
                string_prop("g4.role", role),
            ],
        })
    }
}

/// `n` as a TMJ number.
fn count(n: usize) -> Result<u32, KernelError> {
    u32::try_from(n).map_err(|_| KernelError::Refused(format!("{n} ist zu groß für TMJ.")))
}

/// An embedded image collection `name` from GID `firstgid` with property `g4.kind` `kind`.
fn image_collection(
    name: &str,
    firstgid: u32,
    kind: &str,
    tiles: Vec<TmjTile>,
) -> Result<TmjTileset, KernelError> {
    let widest = tiles.iter().map(|t| t.imagewidth).max().unwrap_or(TMJ_TILE);
    let highest = tiles
        .iter()
        .map(|t| t.imageheight)
        .max()
        .unwrap_or(TMJ_TILE);
    Ok(TmjTileset {
        columns: 0,
        firstgid,
        grid: Some(TmjGrid {
            height: TMJ_TILE,
            orientation: "orthogonal".to_owned(),
            width: TMJ_TILE,
        }),
        margin: 0,
        name: name.to_owned(),
        properties: vec![string_prop("g4.kind", kind)],
        source: None,
        spacing: 0,
        tilecount: count(tiles.len())?,
        tileheight: highest,
        tiles,
        tilewidth: widest,
    })
}

/// A `tilelayer` of 32 × 32 `gids`.
fn tile_layer(id: u32, name: &str, properties: Vec<TmjProperty>, gids: Vec<u32>) -> TmjLayer {
    TmjLayer {
        compression: None,
        data: Some(TmjData::Gids(gids)),
        draworder: None,
        encoding: None,
        height: Some(MAP_SIZE as u32),
        id,
        name: name.to_owned(),
        objects: None,
        opacity: 1.0,
        properties,
        kind: "tilelayer".to_owned(),
        visible: true,
        width: Some(MAP_SIZE as u32),
        x: 0,
        y: 0,
    }
}

/// An `objectgroup` with property `g4.kind` `kind`.
fn object_group(id: u32, name: &str, kind: &str, objects: Vec<TmjObject>) -> TmjLayer {
    TmjLayer {
        compression: None,
        data: None,
        draworder: Some("topdown".to_owned()),
        encoding: None,
        height: None,
        id,
        name: name.to_owned(),
        objects: Some(objects),
        opacity: 1.0,
        properties: vec![string_prop("g4.kind", kind)],
        kind: "objectgroup".to_owned(),
        visible: true,
        width: None,
        x: 0,
        y: 0,
    }
}

/// An object at (`x`, `y`) of size `side` × `side` (pixels), without a tile.
fn object(id: u32, x: f64, y: f64, side: f64, properties: Vec<TmjProperty>) -> TmjObject {
    TmjObject {
        gid: None,
        height: side,
        id,
        name: String::new(),
        point: false,
        properties,
        rotation: 0.0,
        kind: String::new(),
        visible: true,
        width: side,
        x,
        y,
    }
}

/// The index of `ramp` in [`RAMP_TILES`]; `None` for no ramp.
fn ramp_index(ramp: Ramp) -> Option<usize> {
    let (direction, steps) = match ramp {
        Ramp::None => return None,
        Ramp::North(s) => (0, s),
        Ramp::East(s) => (1, s),
        Ramp::South(s) => (2, s),
        Ramp::West(s) => (3, s),
    };
    Some(direction * 2 + usize::from(steps.clamp(1, 2) - 1))
}

/// The TMJ of `world` (map `map`, `buildings` for the view layer, `tileset` its tiles) and
/// the images to write next to it (path relative to the TMJ, under `<stem>.tiles/`, PNG
/// bytes). `image_of` gives a tile's texture (`None`: a 16 × 16 placeholder in the colour of
/// its role). Refused when `world` does not read into cells.
pub fn to_tmj(
    map: u16,
    world: &WorldJson,
    buildings: &[Building],
    tileset: &Tileset,
    image_of: &dyn Fn(&TileJson) -> Option<Rgba>,
    stem: &str,
) -> Result<(TmjMap, Vec<TileImage>), KernelError> {
    let grid = WorldGrid::parse(world).map_err(|errors| {
        let first = errors.first().map(ToString::to_string).unwrap_or_default();
        KernelError::Refused(format!("Karte {map:04}: {first}"))
    })?;
    let mut images = Images {
        folder: format!("{stem}.tiles"),
        files: Vec::new(),
    };

    // Tilesets: the map's tiles from 1, the heights after them, the ramps after the heights.
    let mut map_tiles = Vec::with_capacity(tileset.tiles.len());
    for (i, tile) in tileset.tiles.iter().enumerate() {
        let rgba = image_of(tile)
            .filter(whole)
            .unwrap_or_else(|| solid(role_colour(tile.role)));
        map_tiles.push(images.tile(
            i,
            &format!("tile-{i:04x}"),
            &rgba,
            &tile.texture,
            tile.role.key(),
        )?);
    }
    let height_first = 1 + count(map_tiles.len())?;
    let ramp_first = height_first + HEIGHT_TILES;
    let mut height_tiles = Vec::with_capacity(HEIGHT_TILES as usize);
    for (i, h) in (HEIGHT_MIN..=HEIGHT_MAX).enumerate() {
        let name = height_name(h);
        height_tiles.push(images.tile(i, &name, &solid(height_colour(h)), &name, "height")?);
    }
    let mut ramp_tiles = Vec::with_capacity(RAMP_TILES.len());
    for (i, token) in RAMP_TILES.iter().enumerate() {
        let name = format!("ramp-{token}");
        ramp_tiles.push(images.tile(i, &name, &solid(ramp_colour(i)), &name, "ramp")?);
    }
    let tilesets = vec![
        image_collection(
            &format!("g4 Paket {:04}", tileset.package),
            1,
            "tiles",
            map_tiles,
        )?,
        image_collection(HEIGHT_TILESET, height_first, "heights", height_tiles)?,
        image_collection(RAMP_TILESET, ramp_first, "ramps", ramp_tiles)?,
    ];

    // Layers: the world's, the heights, the ramps; then the objects.
    let mut layers: Vec<TmjLayer> = Vec::new();
    // Layer IDs count from 1 in file order.
    let next_layer = |layers: &[TmjLayer]| layers.len() as u32 + 1;
    for layer in &grid.layers {
        let gids: Vec<u32> = layer
            .tiles
            .iter()
            .map(|t| t.map_or(0, |t| (u32::from(t.tile) + 1) | gid_flags(t.rotate, t.flip)))
            .collect();
        let role = match layer.role {
            LayerRole::Ground => "ground",
            LayerRole::Overlay => "overlay",
        };
        layers.push(tile_layer(
            next_layer(&layers),
            &layer.name,
            vec![string_prop("g4.role", role)],
            gids,
        ));
    }
    let heights: Vec<u32> = grid
        .heights
        .iter()
        .map(|&h| height_first + u32::from(h.abs_diff(HEIGHT_MIN)))
        .collect();
    layers.push(tile_layer(
        next_layer(&layers),
        HEIGHT_LAYER,
        vec![string_prop("g4.kind", "heights")],
        heights,
    ));
    if world.layers.first().is_some_and(|l| l.ramps.is_some()) {
        let ramps: Vec<u32> = grid
            .ramps
            .iter()
            .map(|&r| ramp_index(r).map_or(0, |i| ramp_first + i as u32))
            .collect();
        layers.push(tile_layer(
            next_layer(&layers),
            RAMP_LAYER,
            vec![string_prop("g4.kind", "ramps")],
            ramps,
        ));
    }

    let tile = f64::from(TMJ_TILE);
    let mut next_object = 1u32;
    let mut new_id = || {
        let id = next_object;
        next_object += 1;
        id
    };
    let pieces: Vec<TmjObject> = grid
        .pieces
        .iter()
        .map(|p| TmjObject {
            gid: Some(u32::from(p.tile) + 1),
            ..object(
                new_id(),
                f64::from(p.at[0]) * tile,
                (f64::from(p.at[1]) + 1.0) * tile,
                tile,
                vec![int_prop("g4.rotate", i64::from(p.rotate))],
            )
        })
        .collect();
    layers.push(object_group(
        next_layer(&layers),
        PIECE_LAYER,
        "pieces",
        pieces,
    ));
    if world.permissions.overrides.is_some() {
        let overrides: Vec<TmjObject> = grid
            .overrides
            .iter()
            .enumerate()
            .filter_map(|(i, cell)| cell.map(|c| (i, c)))
            .map(|(i, c)| {
                object(
                    new_id(),
                    (i % MAP_SIZE) as f64 * tile,
                    (i / MAP_SIZE) as f64 * tile,
                    tile,
                    vec![
                        int_prop("g4.kind", i64::from(c.kind)),
                        int_prop("g4.collision", i64::from(c.collision)),
                    ],
                )
            })
            .collect();
        layers.push(object_group(
            next_layer(&layers),
            PERMISSION_LAYER,
            "overrides",
            overrides,
        ));
    }
    let centre = (MAP_SIZE as f64) * tile / 2.0;
    let fx = |v: i32| f64::from(v) / 4096.0 + centre;
    let points: Vec<TmjObject> = buildings
        .iter()
        .map(|b| TmjObject {
            point: true,
            ..object(
                new_id(),
                fx(b.position[0]),
                fx(b.position[2]),
                0.0,
                vec![int_prop("g4.model_id", i64::from(b.model_id))],
            )
        })
        .collect();
    layers.push(object_group(
        next_layer(&layers),
        BUILDING_LAYER,
        "buildings",
        points,
    ));

    let tmj = TmjMap {
        compressionlevel: -1,
        height: MAP_SIZE as u32,
        infinite: false,
        nextlayerid: next_layer(&layers),
        layers,
        nextobjectid: next_object,
        orientation: "orthogonal".to_owned(),
        properties: vec![
            string_prop("g4.format", TMJ_FORMAT),
            int_prop("g4.version", TMJ_VERSION),
            int_prop("g4.map", i64::from(map)),
            int_prop("g4.tileset", i64::from(world.tileset)),
            bool_prop("g4.derive", world.permissions.derive),
        ],
        renderorder: "right-down".to_owned(),
        tiledversion: TILED_VERSION.to_owned(),
        tileheight: TMJ_TILE,
        tilesets,
        tilewidth: TMJ_TILE,
        kind: "map".to_owned(),
        version: TMJ_FILE_VERSION.to_owned(),
        width: MAP_SIZE as u32,
    };
    Ok((tmj, images.files))
}

/// The bytes of `tmj` as the studio writes them: pretty JSON and a line feed.
pub fn to_json(tmj: &TmjMap) -> Result<Vec<u8>, KernelError> {
    let mut bytes = serde_json::to_vec_pretty(tmj).map_err(std::io::Error::from)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// What [`world_export_tmj`] wrote.
#[derive(Debug, Clone, Serialize)]
pub struct TmjExportReport {
    pub map: u16,
    #[serde(serialize_with = "crate::relpath::ser")]
    pub path: PathBuf,
    pub layers: usize,
    pub images: usize,
    pub pieces: usize,
    pub overrides: usize,
}

/// Writes `path` (and `<stem>.tiles/` next to it) for world map `map`; refused without world.
/// The tiles' images are the package's textures ([`tileset_texture`]), placeholders where they
/// do not decode. Nothing in the project changes.
pub fn world_export_tmj(
    store: &mut Store,
    map: u16,
    path: &Path,
) -> Result<TmjExportReport, KernelError> {
    let world = match store.is_world_map(map)? {
        true => store.world(map)?.cloned(),
        false => None,
    }
    .ok_or_else(|| KernelError::Refused(not_a_world_map(map)))?;
    let tileset = store.tileset(world.tileset)?;
    let buildings = store.map(map)?.buildings.clone();
    let mut textures = BTreeMap::new();
    for tile in &tileset.tiles {
        if let Some(rgba) = tileset_texture(store.project(), &tileset, &tile.texture)? {
            textures.insert(tile.texture.clone(), rgba);
        }
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            KernelError::Refused(format!(
                "{} hat keinen Dateinamen für TMJ.",
                crate::relpath::shown(path)
            ))
        })?;
    let (tmj, images) = to_tmj(
        map,
        &world,
        &buildings,
        &tileset,
        &|tile| textures.get(&tile.texture).cloned(),
        stem,
    )?;
    let bytes = to_json(&tmj)?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let folder = dir.join(format!("{stem}.tiles"));
    std::fs::create_dir_all(&folder).map_err(io_at(&folder))?;
    for (rel, png) in &images {
        let file = dir.join(rel);
        std::fs::write(&file, png).map_err(io_at(&file))?;
    }
    std::fs::write(path, &bytes).map_err(io_at(path))?;
    Ok(TmjExportReport {
        map,
        path: path.to_path_buf(),
        layers: tmj.layers.len(),
        images: images.len(),
        pieces: world.pieces.len(),
        overrides: tmj
            .layers
            .iter()
            .find(|l| l.name == PERMISSION_LAYER)
            .and_then(|l| l.objects.as_ref())
            .map_or(0, Vec::len),
    })
}

#[cfg(test)]
mod tests;
