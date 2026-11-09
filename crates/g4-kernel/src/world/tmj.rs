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
//! Reading it back ([`from_tmj`], with the store [`world_inspect_tmj`] and [`world_import_tmj`],
//! one undo step) is our own reader over the serde types of `tmj/types.rs` (the TMJ subset
//! written and read; unknown fields ignored, missing ones filled with defaults), every error
//! collected with its path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use g4_formats::map::{Building, Cell, MAP_SIZE};
use g4_formats::nsbtx::Rgba;
use serde::Serialize;
use serde_json::{Value, json};

use super::bake::WorldBakeReport;
use super::bake::{Verbatim, bake_world};
use super::check::tile_errors;
use super::doc::{
    HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, PermissionsJson, PieceJson, TILE_UNITS, WorldJson,
};
use super::edit::{not_a_world_map, refuse_locked_map};
use super::grid::{Ramp, TileRef, WorldGrid, override_token, ramp_token, tile_token};
use super::keep::locked_map_refusal;
use super::tileset::{TileJson, TileRole, Tileset, tileset_texture};
use crate::error::{KernelError, io_at};
use crate::store::{ChangeId, Store};

mod types;
pub use types::*;

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
    let points: Vec<TmjObject> = buildings
        .iter()
        .map(|b| (b, building_point(b)))
        .map(|(b, (x, y))| TmjObject {
            point: true,
            ..object(
                new_id(),
                x,
                y,
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

/// The code of a TMJ import's errors.
pub const TMJ_CODE: &str = "world.tmj";
/// The note of an import whose building points or models differ from the map's buildings: the
/// studio edits buildings itself, the import ignores them.
pub const BUILDINGS_IGNORED_CODE: &str = "tmj.buildings-ignored";

/// The flip bits and the hexagonal turn of a GID.
const GID_FLAGS: u32 = FLIP_H | FLIP_V | FLIP_D | FLIP_HEX;
/// Cells of a layer.
const CELLS: usize = MAP_SIZE * MAP_SIZE;

const NOT_A_MAP: &str = "Keine Tiled-Karte (`type` ist nicht „map“).";
const INFINITE: &str = "Unendliche Karten liest das Studio nicht.";
const COMPRESSED: &str = "Komprimierte Ebenen liest das Studio nicht; in Tiled unter Karte › \
                          Eigenschaften „Kachelebenenformat“ CSV wählen.";
const EXTERNAL: &str =
    "Ausgelagerte Tilesets (`source`) liest das Studio nicht; in Tiled das Tileset einbetten.";
const HEX_TURN: &str = "Sechseck-Drehung gibt es nicht.";
const NO_HEIGHTS: &str = "Die Ebene „Höhe“ fehlt.";

/// An error of a TMJ at `path` (`layers[2].data[517]`; the world's own paths `world.…` for
/// what [`WorldGrid::parse`] or the tileset check refuse after the mapping).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TmjError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for TmjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

/// The TMJ of `bytes`; a file that is no JSON object of the TMJ shape is one error with the
/// parser's sentence (path empty).
fn read_tmj(bytes: &[u8]) -> Result<TmjMap, Vec<TmjError>> {
    serde_json::from_slice(bytes).map_err(|e| {
        vec![TmjError {
            path: String::new(),
            message: e.to_string(),
        }]
    })
}

/// The world `bytes` (a TMJ) describes on top of `target` (version, tileset, freeform, terrain
/// kept); every error collected.
///
/// The mapping is [`to_tmj`]'s backwards: the tile layers without `g4.kind` are the world's
/// layers in file order (the first the ground, the others overlays, so a new layer without
/// properties is an overlay); the tile layers `g4.kind` `heights` and `ramps` the ground's
/// heights and ramps (a world without a „Rampen“ layer has none); the object groups `g4.kind`
/// `pieces` and `overrides` the pieces and the permission overrides (rectangles of whole cells;
/// a world without such a group has none), `g4.derive` the permissions' derivation (the
/// target's when the property is missing). The buildings' points are not read. The tokens are
/// written canonically (`tile_token`, `ramp_token`, `override_token`), so for a canonical world
/// — every world a store saved after an edit, every world of the property test —
/// `from_tmj(to_json(to_tmj(w)), w) == w`; a hand-written token in upper case comes back in
/// lower case. Heights or ramps other than the target's drop its verbatim terrain, as a height
/// painted in the studio does: the BDHC comes from the grid then.
///
/// Refused (every error with its path): a file that is no TMJ map, a map of another size or
/// tile size, an infinite map, an external tileset, another package than the world's, a first
/// tileset that is not the package's tiles, compressed layer data, a GID outside the layer's
/// tileset, a hexagonal turn, a missing height, an object off the grid, a piece whose tile is
/// none, a cell overridden twice; then what [`WorldGrid::parse`] and the tileset check refuse.
pub fn from_tmj(
    bytes: &[u8],
    target: &WorldJson,
    tileset: &Tileset,
) -> Result<WorldJson, Vec<TmjError>> {
    from_map(&read_tmj(bytes)?, target, tileset)
}

/// [`from_tmj`] on a TMJ read already.
fn from_map(
    tmj: &TmjMap,
    target: &WorldJson,
    tileset: &Tileset,
) -> Result<WorldJson, Vec<TmjError>> {
    if tmj.kind != "map" {
        return Err(vec![TmjError {
            path: "type".to_owned(),
            message: NOT_A_MAP.to_owned(),
        }]);
    }
    let mut reader = Reader {
        errors: Vec::new(),
        tileset,
        gids: GidRanges::default(),
    };
    reader.header(tmj, target);
    if !reader.errors.is_empty() {
        return Err(reader.errors);
    }
    let read = reader.layers(tmj);
    let derive = prop(&tmj.properties, "g4.derive")
        .and_then(Value::as_bool)
        .unwrap_or(target.permissions.derive);
    let Read {
        layers,
        heights,
        ramps,
        pieces,
        overrides,
        height_layer,
    } = read;
    if !height_layer {
        reader.push("layers", NO_HEIGHTS);
    }
    if !reader.errors.is_empty() {
        return Err(reader.errors);
    }
    let layers = layers
        .into_iter()
        .enumerate()
        .map(|(i, (name, tiles))| {
            let ground = i == 0;
            LayerJson {
                name,
                role: if ground {
                    LayerRole::Ground
                } else {
                    LayerRole::Overlay
                },
                tiles: rows_of(&tiles),
                heights: if ground {
                    heights.as_deref().map(rows_of)
                } else {
                    None
                },
                ramps: if ground {
                    ramps.as_deref().map(rows_of)
                } else {
                    None
                },
            }
        })
        .collect();
    let mut world = WorldJson {
        version: target.version,
        tileset: target.tileset,
        layers,
        pieces,
        freeform: target.freeform.clone(),
        terrain: target.terrain.clone(),
        permissions: PermissionsJson {
            derive,
            overrides: overrides
                .map(|cells| rows_of(&cells.into_iter().map(override_token).collect::<Vec<_>>())),
        },
    };
    let grid = WorldGrid::parse(&world).map_err(|errors| {
        errors
            .into_iter()
            .map(|e| TmjError {
                path: e.path,
                message: e.message,
            })
            .collect::<Vec<_>>()
    })?;
    let problems = tile_errors(&grid, tileset);
    if !problems.is_empty() {
        return Err(problems
            .into_iter()
            .map(|e| TmjError {
                path: e.path,
                message: e.message,
            })
            .collect());
    }
    if let Ok(old) = WorldGrid::parse(target)
        && (old.heights != grid.heights || old.ramps != grid.ramps)
    {
        world.terrain = None;
    }
    Ok(world)
}

/// 32 rows of the 1024 `tokens`, one space between them.
fn rows_of(tokens: &[String]) -> Vec<String> {
    tokens.chunks(MAP_SIZE).map(|row| row.join(" ")).collect()
}

/// Property `name` of `props`.
fn prop<'a>(props: &'a [TmjProperty], name: &str) -> Option<&'a Value> {
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

/// The `g4.kind` of `props`.
fn kind_of(props: &[TmjProperty]) -> Option<&str> {
    prop(props, "g4.kind").and_then(Value::as_str)
}

/// A byte property (`g4.kind`, `g4.collision` of an override).
fn byte_prop(props: &[TmjProperty], name: &str) -> Option<u8> {
    prop(props, name)
        .and_then(Value::as_u64)
        .and_then(|v| u8::try_from(v).ok())
}

/// The GIDs of the three tilesets: the package's tiles from 1, the helpers where the file has
/// them (by `g4.kind`).
#[derive(Default)]
struct GidRanges {
    tiles: u32,
    heights: Option<u32>,
    ramps: Option<u32>,
}

impl GidRanges {
    /// The tile of GID `id` (flags off), `None` outside the package's tiles.
    fn tile(&self, id: u32) -> Option<u16> {
        (1..=self.tiles)
            .contains(&id)
            .then(|| u16::try_from(id - 1).ok())
            .flatten()
    }

    /// The index of GID `id` in the helper tileset from `first` with `count` tiles.
    fn helper(first: Option<u32>, count: u32, id: u32) -> Option<u32> {
        let first = first?;
        (first..first.saturating_add(count))
            .contains(&id)
            .then(|| id - first)
    }
}

/// What the layers of a TMJ hold: the world's layers (name, 1024 tile tokens), the ground's
/// 1024 height and ramp tokens, the pieces, the overrides (`None` without a group).
struct Read {
    layers: Vec<(String, Vec<String>)>,
    heights: Option<Vec<String>>,
    ramps: Option<Vec<String>>,
    pieces: Vec<PieceJson>,
    overrides: Option<Vec<Option<Cell>>>,
    /// Whether the file has a „Höhe“ layer (good or not).
    height_layer: bool,
}

/// Reads a TMJ, collecting its errors.
struct Reader<'a> {
    errors: Vec<TmjError>,
    tileset: &'a Tileset,
    gids: GidRanges,
}

impl Reader<'_> {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.errors.push(TmjError {
            path: path.into(),
            message: message.into(),
        });
    }

    /// The map's size, tile size, `infinite`, its tilesets and package.
    fn header(&mut self, tmj: &TmjMap, target: &WorldJson) {
        let side = MAP_SIZE as u32;
        if tmj.width != side || tmj.height != side {
            let path = if tmj.width != side { "width" } else { "height" };
            self.push(
                path,
                format!(
                    "Die Karte ist {} × {} Kacheln groß; eine Karte hat {side} × {side}.",
                    tmj.width, tmj.height
                ),
            );
        }
        if tmj.tilewidth != TMJ_TILE || tmj.tileheight != TMJ_TILE {
            let path = if tmj.tilewidth != TMJ_TILE {
                "tilewidth"
            } else {
                "tileheight"
            };
            self.push(
                path,
                format!(
                    "Kacheln sind {} × {} Pixel; das Studio schreibt {TMJ_TILE} × {TMJ_TILE}.",
                    tmj.tilewidth, tmj.tileheight
                ),
            );
        }
        if tmj.infinite {
            self.push("infinite", INFINITE);
        }
        if let Some(package) = prop(&tmj.properties, "g4.tileset").and_then(Value::as_i64)
            && package != i64::from(target.tileset)
        {
            self.push(
                "properties",
                format!(
                    "Die Datei nutzt Paket {package:04}, die Welt Paket {:04}.",
                    target.tileset
                ),
            );
        }
        for (i, set) in tmj.tilesets.iter().enumerate() {
            if set.source.is_some() {
                self.push(format!("tilesets[{i}].source"), EXTERNAL);
            }
        }
        let tiles = self.tileset.tiles.len();
        let count_error = |n: u32| {
            format!(
                "Das Tileset hat {n} Kacheln, Paket {:04} hat {tiles}.",
                target.tileset
            )
        };
        match tmj.tilesets.first() {
            None => self.push("tilesets", count_error(0)),
            Some(set) if set.source.is_none() => {
                if set.firstgid != 1 {
                    self.push(
                        "tilesets[0].firstgid",
                        format!(
                            "Das erste Tileset beginnt bei GID {}; das Studio schreibt 1.",
                            set.firstgid
                        ),
                    );
                }
                if usize::try_from(set.tilecount).ok() != Some(tiles) {
                    self.push("tilesets[0].tilecount", count_error(set.tilecount));
                }
                self.gids.tiles = set.tilecount;
            }
            Some(_) => {}
        }
        let first_of = |kind: &str| {
            tmj.tilesets
                .iter()
                .skip(1)
                .find(|set| set.source.is_none() && kind_of(&set.properties) == Some(kind))
                .map(|set| set.firstgid)
        };
        self.gids.heights = first_of("heights");
        self.gids.ramps = first_of("ramps");
    }

    /// The layers of `tmj`: tile layers and object groups by their `g4.kind`; image layers,
    /// groups and object groups of other kinds (the buildings) are not read.
    fn layers(&mut self, tmj: &TmjMap) -> Read {
        let mut read = Read {
            layers: Vec::new(),
            heights: None,
            ramps: None,
            pieces: Vec::new(),
            overrides: None,
            height_layer: false,
        };
        for (i, layer) in tmj.layers.iter().enumerate() {
            match (layer.kind.as_str(), kind_of(&layer.properties)) {
                ("tilelayer", Some("heights")) => {
                    read.height_layer = true;
                    let Some(gids) = self.cells(i, layer) else {
                        continue;
                    };
                    let heights = self.heights(i, &gids);
                    if read.heights.is_none() {
                        read.heights = heights;
                    }
                }
                ("tilelayer", Some("ramps")) => {
                    let Some(gids) = self.cells(i, layer) else {
                        continue;
                    };
                    let ramps = self.ramps(i, &gids);
                    if read.ramps.is_none() {
                        read.ramps = ramps;
                    }
                }
                ("tilelayer", _) => {
                    let Some(gids) = self.cells(i, layer) else {
                        continue;
                    };
                    if let Some(tiles) = self.tiles(i, &gids) {
                        read.layers.push((layer.name.clone(), tiles));
                    }
                }
                ("objectgroup", Some("pieces")) => {
                    for (j, object) in layer.objects.iter().flatten().enumerate() {
                        if let Some(piece) = self.piece(i, j, object) {
                            read.pieces.push(piece);
                        }
                    }
                }
                ("objectgroup", Some("overrides")) => {
                    let cells = read.overrides.get_or_insert_with(|| vec![None; CELLS]);
                    for (j, object) in layer.objects.iter().flatten().enumerate() {
                        self.override_cells(i, j, object, cells);
                    }
                }
                _ => {}
            }
        }
        read
    }

    /// The 1024 GIDs of tile layer `i`: a list, or Base64 (little-endian, no compression).
    fn cells(&mut self, i: usize, layer: &TmjLayer) -> Option<Vec<u32>> {
        let at = format!("layers[{i}]");
        if layer.compression.as_deref().is_some_and(|c| !c.is_empty()) {
            self.push(format!("{at}.compression"), COMPRESSED);
            return None;
        }
        let gids = match &layer.data {
            None => Vec::new(),
            Some(TmjData::Gids(gids)) => gids.clone(),
            Some(TmjData::Encoded(text)) => {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(text.trim()) {
                    Ok(bytes) if bytes.len() % 4 == 0 => bytes
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| u32::from_le_bytes(*c))
                        .collect(),
                    Ok(bytes) => {
                        self.push(
                            format!("{at}.data"),
                            format!(
                                "Die Ebenendaten sind {} Bytes lang, keine Folge von GIDs zu je \
                                 4 Bytes.",
                                bytes.len()
                            ),
                        );
                        return None;
                    }
                    Err(e) => {
                        self.push(
                            format!("{at}.data"),
                            format!("Die Ebenendaten sind kein Base64 ({e})."),
                        );
                        return None;
                    }
                }
            }
        };
        if gids.len() != CELLS {
            self.push(
                format!("{at}.data"),
                format!(
                    "Die Ebene hat {} Zellen; eine Karte hat {MAP_SIZE} × {MAP_SIZE}.",
                    gids.len()
                ),
            );
            return None;
        }
        Some(gids)
    }

    /// The tile tokens of world layer `i`, `None` after noting a bad cell.
    fn tiles(&mut self, i: usize, gids: &[u32]) -> Option<Vec<String>> {
        let mut ok = true;
        let mut tokens = Vec::with_capacity(CELLS);
        for (c, &gid) in gids.iter().enumerate() {
            let id = gid & !GID_FLAGS;
            if id == 0 {
                tokens.push(tile_token(None));
                continue;
            }
            let at = format!("layers[{i}].data[{c}]");
            let Some((rotate, flip)) = flags_turn(gid) else {
                self.push(at, HEX_TURN);
                ok = false;
                continue;
            };
            match self.gids.tile(id) {
                Some(tile) => tokens.push(tile_token(Some(TileRef { tile, rotate, flip }))),
                None => {
                    self.push(at, outside(id));
                    ok = false;
                }
            }
        }
        ok.then_some(tokens)
    }

    /// The height tokens of layer `i` („Höhe“; flip bits mean nothing there).
    fn heights(&mut self, i: usize, gids: &[u32]) -> Option<Vec<String>> {
        let mut ok = true;
        let mut tokens = Vec::with_capacity(CELLS);
        let count = u32::from(HEIGHT_MAX.abs_diff(HEIGHT_MIN)) + 1;
        for (c, &gid) in gids.iter().enumerate() {
            let id = gid & !GID_FLAGS;
            let at = format!("layers[{i}].data[{c}]");
            if id == 0 {
                self.push(
                    at,
                    format!("Zelle ({}, {}) hat keine Höhe.", c % MAP_SIZE, c / MAP_SIZE),
                );
                ok = false;
                continue;
            }
            let step = GidRanges::helper(self.gids.heights, count, id)
                .and_then(|n| i8::try_from(i32::from(HEIGHT_MIN) + n as i32).ok());
            match step {
                Some(h) => tokens.push(h.to_string()),
                None => {
                    self.push(at, outside(id));
                    ok = false;
                }
            }
        }
        ok.then_some(tokens)
    }

    /// The ramp tokens of layer `i` („Rampen“; 0 is no ramp).
    fn ramps(&mut self, i: usize, gids: &[u32]) -> Option<Vec<String>> {
        let mut ok = true;
        let mut tokens = Vec::with_capacity(CELLS);
        for (c, &gid) in gids.iter().enumerate() {
            let id = gid & !GID_FLAGS;
            if id == 0 {
                tokens.push(ramp_token(Ramp::None));
                continue;
            }
            let token = GidRanges::helper(self.gids.ramps, RAMP_TILES.len() as u32, id)
                .and_then(|n| RAMP_TILES.get(n as usize));
            match token {
                Some(token) => tokens.push((*token).to_owned()),
                None => {
                    self.push(format!("layers[{i}].data[{c}]"), outside(id));
                    ok = false;
                }
            }
        }
        ok.then_some(tokens)
    }

    /// The piece of object `j` of layer `i`: a tile object of a piece tile on the grid (its
    /// lower left corner on a cell's, Tiled's anchor of tile objects), turned by `g4.rotate`.
    fn piece(&mut self, i: usize, j: usize, object: &TmjObject) -> Option<PieceJson> {
        let at = format!("layers[{i}].objects[{j}]");
        let id = object.id;
        let cell = on_grid(object.x).zip(on_grid(object.y)).and_then(|(x, y)| {
            let z = y.checked_sub(1)?;
            let side = MAP_SIZE as u32;
            (x < side && z < side).then_some([x as u8, z as u8])
        });
        let piece_tile = object
            .gid
            .and_then(|g| self.gids.tile(g & !GID_FLAGS))
            .filter(|&t| {
                self.tileset
                    .tile(t)
                    .is_some_and(|t| t.role == TileRole::Piece)
            });
        let rotate = match prop(&object.properties, "g4.rotate") {
            None => Some(0),
            Some(v) => v.as_u64().and_then(|v| u16::try_from(v).ok()),
        };
        let Some(at_cell) = cell else {
            self.push(at, format!("Objekt {id} liegt nicht auf dem Raster."));
            return None;
        };
        let Some(tile) = piece_tile else {
            let shown = match object.gid.map(|g| g & !GID_FLAGS) {
                Some(g) if g > 0 => format!("{:#06x}", g - 1),
                _ => "–".to_owned(),
            };
            self.push(
                at,
                format!("Objekt {id}: Kachel {shown} ist kein Teilmodell."),
            );
            return None;
        };
        let Some(rotate) = rotate else {
            self.push(
                at,
                format!("Objekt {id}: `g4.rotate` ist keine Drehung (0, 90, 180 oder 270)."),
            );
            return None;
        };
        Some(PieceJson {
            tile,
            at: at_cell,
            rotate,
        })
    }

    /// The cells of override object `j` of layer `i` (a rectangle of whole cells with
    /// `g4.kind` and `g4.collision`) into `cells`; a cell set twice is an error.
    fn override_cells(
        &mut self,
        i: usize,
        j: usize,
        object: &TmjObject,
        cells: &mut [Option<Cell>],
    ) {
        let at = format!("layers[{i}].objects[{j}]");
        let id = object.id;
        let side = MAP_SIZE as u32;
        let rect = on_grid(object.x)
            .zip(on_grid(object.y))
            .zip(on_grid(object.width).zip(on_grid(object.height)))
            .filter(|&((x, z), (w, d))| w > 0 && d > 0 && x + w <= side && z + d <= side);
        let Some(((x0, z0), (w, d))) = rect else {
            self.push(at, format!("Objekt {id} liegt nicht auf dem Raster."));
            return;
        };
        let props = &object.properties;
        let (Some(kind), Some(collision)) = (
            byte_prop(props, "g4.kind"),
            byte_prop(props, "g4.collision"),
        ) else {
            self.push(
                at,
                format!("Objekt {id}: `g4.kind` und `g4.collision` brauchen Zahlen von 0 bis 255."),
            );
            return;
        };
        for z in z0..z0 + d {
            for x in x0..x0 + w {
                let c = (z * side + x) as usize;
                let Some(slot) = cells.get_mut(c) else {
                    continue;
                };
                if slot.is_some() {
                    self.push(at, format!("Zelle ({x}, {z}) ist zweimal überschrieben."));
                    return;
                }
                *slot = Some(Cell { kind, collision });
            }
        }
    }
}

/// `v` pixels as whole cells (a multiple of 16 from 0), `None` off the grid.
fn on_grid(v: f64) -> Option<u32> {
    let tile = f64::from(TMJ_TILE);
    ((0.0..=4096.0).contains(&v) && (v / tile).fract() == 0.0).then(|| (v / tile) as u32)
}

/// „GID 7 gehört nicht in diese Ebene.“
fn outside(id: u32) -> String {
    format!("GID {id} gehört nicht in diese Ebene.")
}

/// The point of `building` in the layer „Gebäude (nur Ansicht)“: pixels from the map's corner.
fn building_point(building: &Building) -> (f64, f64) {
    let centre = (MAP_SIZE as f64) * f64::from(TMJ_TILE) / 2.0;
    let fx = |v: i32| f64::from(v) / 4096.0 + centre;
    (fx(building.position[0]), fx(building.position[2]))
}

/// Whether the building points of `tmj` (positions and `g4.model_id`) differ from `buildings`.
fn buildings_differ(tmj: &TmjMap, buildings: &[Building]) -> bool {
    let points: Vec<(f64, f64, Option<i64>)> = tmj
        .layers
        .iter()
        .filter(|l| kind_of(&l.properties) == Some("buildings"))
        .flat_map(|l| l.objects.iter().flatten())
        .map(|o| {
            (
                o.x,
                o.y,
                prop(&o.properties, "g4.model_id").and_then(Value::as_i64),
            )
        })
        .collect();
    let expected: Vec<(f64, f64, Option<i64>)> = buildings
        .iter()
        .map(|b| {
            let (x, y) = building_point(b);
            (x, y, Some(i64::from(b.model_id)))
        })
        .collect();
    points != expected
}

/// The cells (of 1024) in which `after` differs from `before` on any layer, height, ramp or
/// override; every cell when `before` does not read.
fn changed_cells(before: &WorldJson, after: &WorldJson) -> usize {
    let (Ok(a), Ok(b)) = (WorldGrid::parse(before), WorldGrid::parse(after)) else {
        return CELLS;
    };
    let layers = a.layers.len().max(b.layers.len());
    let tile = |g: &WorldGrid, l: usize, c: usize| {
        g.layers
            .get(l)
            .and_then(|l| l.tiles.get(c).copied().flatten())
    };
    (0..CELLS)
        .filter(|&c| {
            (0..layers).any(|l| tile(&a, l, c) != tile(&b, l, c))
                || a.heights.get(c) != b.heights.get(c)
                || a.ramps.get(c) != b.ramps.get(c)
                || a.overrides.get(c) != b.overrides.get(c)
        })
        .count()
}

/// „Tiled importiert: Karte 0123“, the label of [`world_import_tmj`].
pub fn import_label(map: u16) -> String {
    format!("Tiled importiert: Karte {map:04}")
}

/// What [`world_inspect_tmj`] read or [`world_import_tmj`] did: the map, the world's layers
/// after the import, the cells that change, the notes (codes: [`BUILDINGS_IGNORED_CODE`]), the
/// file's errors (inspect only; the import refuses), the undo step (`None`: nothing recorded,
/// also when the file equals the world) and the bake (inspect: in memory; `None` without a
/// change).
#[derive(Debug, Clone, Serialize)]
pub struct TmjImportReport {
    pub map: u16,
    pub layers: usize,
    pub cells: usize,
    pub notes: Vec<String>,
    pub errors: Vec<TmjError>,
    pub change: Option<ChangeId>,
    pub bake: Option<WorldBakeReport>,
}

/// The report of `path` read against world map `map` and the world it gives (`None` with
/// errors or when it equals the world).
fn inspect(
    store: &mut Store,
    map: u16,
    path: &Path,
) -> Result<(TmjImportReport, Option<WorldJson>), KernelError> {
    let target = world_of(store, map)?;
    let bytes = std::fs::read(path).map_err(io_at(path))?;
    let tileset = store.tileset(target.tileset)?;
    let buildings = store.map(map)?.buildings.clone();
    let mut report = TmjImportReport {
        map,
        layers: 0,
        cells: 0,
        notes: Vec::new(),
        errors: Vec::new(),
        change: None,
        bake: None,
    };
    let tmj = match read_tmj(&bytes) {
        Ok(tmj) => tmj,
        Err(errors) => {
            report.errors = errors;
            return Ok((report, None));
        }
    };
    if tmj.kind == "map" && buildings_differ(&tmj, &buildings) {
        report.notes.push(BUILDINGS_IGNORED_CODE.to_owned());
    }
    match from_map(&tmj, &target, &tileset) {
        Err(errors) => {
            report.errors = errors;
            Ok((report, None))
        }
        Ok(world) => {
            report.layers = world.layers.len();
            report.cells = changed_cells(&target, &world);
            Ok((report, (world != target).then_some(world)))
        }
    }
}

/// The world of world map `map`; refused ([`not_a_world_map`]) for any other map.
fn world_of(store: &mut Store, map: u16) -> Result<WorldJson, KernelError> {
    match store.is_world_map(map)? {
        true => store.world(map)?.cloned(),
        false => None,
    }
    .ok_or_else(|| KernelError::Refused(not_a_world_map(map)))
}

/// Reads `path` against world map `map` without changing anything: the errors of the file, or
/// the layers and cells it changes with the bake of the world it gives (in memory).
///
/// Refused ([`KernelError::Refused`]): a map without world ([`not_a_world_map`]); a file that
/// does not read is an I/O error.
pub fn world_inspect_tmj(
    store: &mut Store,
    map: u16,
    path: &Path,
) -> Result<TmjImportReport, KernelError> {
    let (mut report, world) = inspect(store, map, path)?;
    if let Some(world) = world {
        let base = store.map(map)?.clone();
        let verbatim = Verbatim::of(&world, &base);
        report.bake = Some(bake_world(store, map, &world, &base, &verbatim)?.report);
    }
    Ok(report)
}

/// The world `path` describes as one undo step [`import_label`]; a file equal to the world is
/// no step (`change: None`). A refused bake is in the report's messages, as with a painted
/// stroke.
///
/// Refused ([`KernelError::Refused`], nothing changed): a map without world
/// ([`not_a_world_map`]), a map whose member is locked, a file with errors (the first with its
/// path and the count: „layers[0].data[0]: GID 500 gehört nicht in diese Ebene. (2 Fehler)“).
pub fn world_import_tmj(
    store: &mut Store,
    map: u16,
    path: &Path,
) -> Result<TmjImportReport, KernelError> {
    world_of(store, map)?;
    refuse_locked_map(store, map, |_| locked_map_refusal(map))?;
    let (mut report, world) = inspect(store, map, path)?;
    if let Some(first) = report.errors.first() {
        return Err(KernelError::Refused(format!(
            "{first} ({} Fehler)",
            report.errors.len()
        )));
    }
    let Some(world) = world else {
        return Ok(report);
    };
    let (bake, change) = store.transaction(&import_label(map), |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    report.change = change;
    report.bake = bake;
    Ok(report)
}

#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod tests;
