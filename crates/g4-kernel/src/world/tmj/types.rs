//! The serde types of TMJ (the subset [`super`] writes and reads): unknown fields are ignored on
//! read (Tiled adds `class`, `parallaxx`, `tintcolor` …), missing ones filled with defaults, so
//! a file Tiled wrote or a hand-edited one reaches the reader's collected errors instead of a
//! bare serde error.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The TMJ subset this module writes and reads (serde; unknown fields ignored on read).
/// Keys in Tiled's order (alphabetical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmjMap {
    #[serde(default = "minus_one")]
    pub compressionlevel: i64,
    #[serde(default)]
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
    #[serde(default)]
    pub tileheight: u32,
    #[serde(default)]
    pub tilesets: Vec<TmjTileset>,
    #[serde(default)]
    pub tilewidth: u32,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
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
