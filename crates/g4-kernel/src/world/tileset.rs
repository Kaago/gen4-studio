//! The tileset of a map texture package: `data/tilesets/NNNN.json` (the stem is the package),
//! one tile per texture of the package (tile `i` is texture `i`), each with its role on the
//! ground, its permission defaults (`kind`, `collision`, HeartGold/SoulSilver `sound`), its
//! height behaviour and optionally the tile of its walls (`wall`) and the footprint of a piece.
//! Keys in the order of the structs, unknown keys refused, numbers as JSON numbers.
//!
//! `edge_sets` (6.4b) and `model` (6.5b, side file `NNNN.<name>.glb`) are reserved: serde
//! reads them so a later file names its problem instead of an unknown key, and
//! [`tileset_problems`] refuses them. The only tileset data in the repository is the
//! synthetic test tileset of [`Tileset::synthetic`], made in code (names `syn_*`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::Cell;
use g4_formats::nsbmd_header::material_texture_names;
use g4_formats::nsbtx::{Nsbtx, Rgba};
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project, SourceMode};
use crate::store::{
    ChangeId, FileChange, FileStep, FileStepKind, HEADER_TABLE, Store, write_file_step,
};
use crate::text::Problem;
use crate::textproject::arts::file_names;
use crate::textproject::canon::{parse_json, problem, to_canonical};
use crate::textproject::codes::SCHEMA_CODE;
use crate::textproject::locate::located;
use crate::tiles::tile_behavior;
use crate::world::WORLD_NEEDS_TEXT_PROJECT;

/// The version of the tileset file this build reads and writes.
pub const TILESET_VERSION: u32 = 1;
/// The folder of the tileset files, relative to the project root.
pub const TILESETS_DIR: &str = "data/tilesets";
/// The package of the synthetic test tileset.
#[cfg(any(test, feature = "test-support"))]
pub const SYNTHETIC_PACKAGE: u16 = 0;

/// The longest texture name of a Nitro texture dictionary, in bytes.
const TEXTURE_NAME_MAX: usize = 16;
/// The highest HeartGold/SoulSilver step sound (bits 0 to 6 of the collision byte).
const SOUND_MAX: u8 = 0x7f;

/// The tileset file `data/tilesets/NNNN.json` of package `package`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tileset {
    pub version: u32,
    pub package: u16,
    pub tiles: Vec<TileJson>,
    /// Reserved for the edge sets of 6.4b; must be empty (left out).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edge_sets: Vec<serde_json::Value>,
}

/// One tile: the texture `texture_index` of the package, named `texture`, with its palette
/// (left out: none) and its size in texels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileJson {
    pub texture: String,
    pub texture_index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    /// Width and height in texels.
    pub size: [u16; 2],
    pub role: TileRole,
    /// The default permission behaviour (`kind`) of a cell with this tile.
    pub kind: u8,
    /// The default collision byte (bit 7 blocks).
    pub collision: u8,
    pub height: HeightBehavior,
    /// HeartGold/SoulSilver only: the step sound (bits 0 to 6 of the collision byte).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub sound: u8,
    /// The tile whose texture the walls below a cell with this tile take (role `wall`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall: Option<u16>,
    /// Width and depth in cells; exactly the tiles of role `piece` have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<[u8; 2]>,
    /// Reserved for the model side files of 6.5b; must be left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub origin: TileOrigin,
}

/// What a tile is on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileRole {
    Ground,
    Wall,
    Ramp,
    Water,
    Grass,
    Piece,
}

/// How a cell with the tile takes its height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeightBehavior {
    Flat,
    Ramp,
    Wall,
}

/// Where the values of a tile come from: derived from the vanilla permissions, not decidable
/// from them, the synthetic test tileset, or set by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileOrigin {
    Vanilla,
    Unclear,
    Synthetic,
    User,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

impl TileRole {
    /// The key of the role in the file (`water`).
    pub fn key(self) -> &'static str {
        match self {
            TileRole::Ground => "ground",
            TileRole::Wall => "wall",
            TileRole::Ramp => "ramp",
            TileRole::Water => "water",
            TileRole::Grass => "grass",
            TileRole::Piece => "piece",
        }
    }
}

impl Tileset {
    /// The synthetic test tileset on [`SYNTHETIC_PACKAGE`], one tile per role: 0 `syn_boden`
    /// (ground, walls of tile 3), 1 `syn_gras` 32 × 32 (grass, kind 0x02), 2 `syn_wasser`
    /// (water, kind 0x15), 3 `syn_wand` (wall, collision 0x80), 4 `syn_rampe` (ramp),
    /// 5 `syn_stueck` (piece 2 × 2); the others 16 × 16, all `origin: synthetic`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn synthetic() -> Tileset {
        let tile = |index: u16, name: &str, size: u16, role: TileRole| TileJson {
            texture: name.to_owned(),
            texture_index: index,
            palette: None,
            size: [size, size],
            role,
            kind: 0,
            collision: 0,
            height: HeightBehavior::Flat,
            sound: 0,
            wall: None,
            footprint: None,
            model: None,
            origin: TileOrigin::Synthetic,
        };
        let tiles = vec![
            TileJson {
                wall: Some(3),
                ..tile(0, "syn_boden", 16, TileRole::Ground)
            },
            TileJson {
                kind: 0x02,
                ..tile(1, "syn_gras", 32, TileRole::Grass)
            },
            TileJson {
                kind: 0x15,
                ..tile(2, "syn_wasser", 16, TileRole::Water)
            },
            TileJson {
                collision: 0x80,
                height: HeightBehavior::Wall,
                ..tile(3, "syn_wand", 16, TileRole::Wall)
            },
            TileJson {
                height: HeightBehavior::Ramp,
                ..tile(4, "syn_rampe", 16, TileRole::Ramp)
            },
            TileJson {
                footprint: Some([2, 2]),
                ..tile(5, "syn_stueck", 16, TileRole::Piece)
            },
        ];
        Tileset {
            version: TILESET_VERSION,
            package: SYNTHETIC_PACKAGE,
            tiles,
            edge_sets: Vec::new(),
        }
    }

    /// Tile `index`, `None` past the tiles.
    pub fn tile(&self, index: u16) -> Option<&TileJson> {
        self.tiles.get(usize::from(index))
    }
}

/// The tileset file of `package` relative to the project root: `data/tilesets/0006.json`.
pub fn tileset_rel(package: u16) -> PathBuf {
    PathBuf::from(TILESETS_DIR).join(format!("{package:04}.json"))
}

/// The tileset file of `package`; `None` when there is none. A file that does not read as
/// a [`Tileset`] is [`KernelError::Refused`] with its path and serde's reason; the rules
/// beyond serde are [`tileset_problems`] (the check reports them, reading does not).
pub fn read_tileset_file(project: &Project, package: u16) -> Result<Option<Tileset>, KernelError> {
    let rel = tileset_rel(package);
    let path = project.root.join(&rel);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(&path)(e)),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| KernelError::Refused(format!("{}: {e}", crate::relpath::slash(&rel))))
}

/// Problems of one tileset file beyond serde, each as `<path>: <text>` with the JSON path
/// in the file (`tiles[3].wall`): `version` ≠ [`TILESET_VERSION`], `package` ≠ the stem,
/// `edge_sets` not empty („Kantensätze kommen mit 6.4b.“), `model` set („Teilmodell-Dateien
/// kommen mit 6.5b.“), `wall` past the tiles ([`tile_outside`]) or not of role `wall`,
/// `footprint` on a tile that is not `piece` or a `piece` without it, a texture name longer
/// than 16 bytes, `sound` ≠ 0 outside HeartGold/SoulSilver or past bit 6 in it.
pub fn tileset_problems(tileset: &Tileset, stem: u16, family: Family) -> Vec<String> {
    let mut problems = Vec::new();
    if tileset.version != TILESET_VERSION {
        problems.push(format!(
            "version: Version {} kennt diese Fassung nicht ({TILESET_VERSION}).",
            tileset.version
        ));
    }
    if tileset.package != stem {
        problems.push(format!(
            "package: Paket {:04} passt nicht zum Dateinamen {stem:04}.json.",
            tileset.package
        ));
    }
    if !tileset.edge_sets.is_empty() {
        problems.push("edge_sets: Kantensätze kommen mit 6.4b.".to_owned());
    }
    let count = tileset.tiles.len();
    for (i, tile) in tileset.tiles.iter().enumerate() {
        let at = |key: &str| format!("tiles[{i}].{key}");
        let len = tile.texture.len();
        if len > TEXTURE_NAME_MAX {
            problems.push(format!(
                "{}: der Texturname `{}` ist {len} Byte lang (höchstens {TEXTURE_NAME_MAX}).",
                at("texture"),
                tile.texture
            ));
        }
        if tile.sound != 0 {
            if family != Family::Hgss {
                problems.push(format!(
                    "{}: Geräusche gibt es nur in HeartGold/SoulSilver.",
                    at("sound")
                ));
            } else if tile.sound > SOUND_MAX {
                problems.push(format!(
                    "{}: {:#04x} hat mehr als 7 Bit (höchstens {SOUND_MAX:#04x}).",
                    at("sound"),
                    tile.sound
                ));
            }
        }
        if let Some(wall) = tile.wall {
            match tileset.tile(wall) {
                None => problems.push(tile_outside(&at("wall"), wall, tileset.package, count)),
                Some(target) if target.role != TileRole::Wall => problems.push(format!(
                    "{}: Kachel {wall:#06x} ist keine Wand (Rolle `{}`).",
                    at("wall"),
                    target.role.key()
                )),
                Some(_) => {}
            }
        }
        match (tile.role == TileRole::Piece, tile.footprint.is_some()) {
            (false, true) => problems.push(format!(
                "{}: einen Fußabdruck haben nur Teilmodelle (`piece`).",
                at("footprint")
            )),
            (true, false) => problems.push(format!(
                "tiles[{i}]: ein Teilmodell (`piece`) braucht einen Fußabdruck (`footprint`)."
            )),
            _ => {}
        }
        if tile.model.is_some() {
            problems.push(format!(
                "{}: Teilmodell-Dateien kommen mit 6.5b.",
                at("model")
            ));
        }
    }
    problems
}

/// „<path>: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln).“ (check and
/// world_edit share it).
pub fn tile_outside(path: &str, tile: u16, package: u16, tiles: usize) -> String {
    let count = if tiles == 1 {
        "1 Kachel".to_owned()
    } else {
        format!("{tiles} Kacheln")
    };
    format!("{path}: Kachel {tile:#06x} gibt es im Tileset {package:04} nicht ({count}).")
}

/// The problems of the folder `data/tilesets/` (the file kind `tilesets` of
/// [`check_files`](crate::textproject::kinds::check_files)), each `textproject.schema` with
/// file and line: a file name that is no package number `NNNN.json`, a file that does not read
/// as a [`Tileset`] (serde's path), and every one of [`tileset_problems`] of a file that does,
/// at its JSON path.
pub fn check_folder(project: &Project, problems: &mut Vec<Problem>) -> Result<(), KernelError> {
    let dir = Path::new(TILESETS_DIR);
    for name in file_names(&project.root.join(dir))? {
        let rel = dir.join(&name);
        let Some(package) = package_of(&name) else {
            problems.push(problem(
                &rel,
                SCHEMA_CODE,
                "Der Dateiname ist keine Paketnummer (`NNNN.json`); die Datei gehört zu keinem \
                 Kartentextur-Paket."
                    .to_owned(),
            ));
            continue;
        };
        let path = project.root.join(&rel);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        let tileset: Tileset = match parse_json(&rel, &bytes, SCHEMA_CODE) {
            Ok(tileset) => tileset,
            Err(found) => {
                problems.push(found);
                continue;
            }
        };
        for text in tileset_problems(&tileset, package, project.family) {
            problems.push(located(&rel, &bytes, &format!("{SCHEMA_CODE}: {text}")));
        }
    }
    Ok(())
}

/// One tile per texture of `btx` (names, indices, sizes, palettes), every tile `ground`,
/// `kind 0`, `origin unclear`: what meshing and range checks need, without reading a map.
pub fn textures_only(package: u16, btx: &Nsbtx) -> Tileset {
    let palettes: Vec<String> = btx.palettes.iter().map(|p| p.name.clone()).collect();
    let tiles = btx
        .textures
        .iter()
        .enumerate()
        .map(|(i, texture)| TileJson {
            texture: texture.name.clone(),
            texture_index: u16::try_from(i).unwrap_or(u16::MAX),
            palette: palette_of(&texture.name, i, texture.format, &palettes),
            size: [texels(texture.width), texels(texture.height)],
            role: TileRole::Ground,
            kind: 0,
            collision: 0,
            height: HeightBehavior::Flat,
            sound: 0,
            wall: None,
            footprint: None,
            model: None,
            origin: TileOrigin::Unclear,
        })
        .collect();
    Tileset {
        version: TILESET_VERSION,
        package,
        tiles,
        edge_sets: Vec::new(),
    }
}

fn texels(n: u32) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The texture format without a palette (direct colour).
const DIRECT_FORMAT: u8 = 7;

/// The palette of a tile (Task 0, §8): `<texture>_pl` (cut to 16 bytes), else the palette of
/// the same name, else the palette with the texture's index, else none; a direct-colour
/// texture (format 7) has none.
fn palette_of(texture: &str, index: usize, format: u8, palettes: &[String]) -> Option<String> {
    if format == DIRECT_FORMAT {
        return None;
    }
    let pl = format!("{texture}_pl");
    let pl = cut(&pl, TEXTURE_NAME_MAX);
    palettes
        .iter()
        .find(|p| p.as_str() == pl)
        .or_else(|| palettes.iter().find(|p| p.as_str() == texture))
        .or_else(|| palettes.get(index))
        .cloned()
}

/// `s` cut to at most `max` bytes, at a character boundary.
fn cut(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// How the permission defaults of a texture are decided from the vanilla permissions of the
/// maps that bind it (plan 6.3a, „Vorgaben je Textur“): K1 the majority, K2 the kind that
/// stands out against the package's maps, `None` every texture unclear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteRule {
    K1,
    K2,
    None,
}

/// The rule the derivation takes (Task 0, §8).
pub const DEFAULT_RULE: VoteRule = VoteRule::K1;
/// The share of the votes the winning kind needs (K1), or kind 0 in K2's fallback (Task 0).
pub const UNIQUE_SHARE: f64 = 0.6;
/// K2: the least share of the votes a kind other than 0 needs to be a candidate.
const K2_MIN_SHARE: f64 = 0.05;
/// K2: how many times its share among all cells of the package's maps a candidate needs.
const K2_RATIO: f64 = 2.0;

/// Derives the tilesets of the packages from the vanilla permissions; it knows the maps of
/// every package from one pass over headers and matrices
/// ([`maps_of_packages`](super::places::maps_of_packages)).
pub struct Deriver {
    maps: BTreeMap<u16, Vec<u16>>,
}

impl Deriver {
    pub fn new(store: &mut Store) -> Result<Deriver, KernelError> {
        Ok(Deriver {
            maps: super::places::maps_of_packages(store)?,
        })
    }

    /// The maps that show `package`, ascending; empty for a package no map shows.
    pub fn maps_of(&self, package: u16) -> &[u16] {
        self.maps.get(&package).map_or(&[], Vec::as_slice)
    }

    /// The tileset of `package`: [`textures_only`] of the package, each texture's defaults
    /// decided by [`DEFAULT_RULE`] from the maps of [`Deriver::maps_of`]: a decided texture
    /// takes `kind` k*, the role of its behaviour (`grass`, `water`, else `ground`), in
    /// HeartGold/SoulSilver the step sound, and `origin: vanilla`; every collision stays 0
    /// (Task 0, §8). A package that is no member or does not read is refused.
    pub fn derive(&self, store: &mut Store, package: u16) -> Result<Tileset, KernelError> {
        let btx = read_package(store.project(), package)?;
        let family = store.project().family;
        let defaults = self.defaults(store, package, &btx)?;
        let mut tileset = textures_only(package, &btx);
        for (tile, d) in tileset.tiles.iter_mut().zip(defaults) {
            if d.decided {
                tile.kind = d.kind;
                tile.sound = d.sound;
                tile.role = role_of(family, d.kind);
                tile.origin = TileOrigin::Vanilla;
            }
        }
        Ok(tileset)
    }

    /// Per texture of `package` the share of the winning kind among its votes
    /// (`votes(k*) / Σ votes`), `None` for a texture no map binds (the numbers of Task 0).
    pub fn vote_shares(
        &self,
        store: &mut Store,
        package: u16,
    ) -> Result<Vec<Option<f64>>, KernelError> {
        let btx = read_package(store.project(), package)?;
        Ok(self
            .defaults(store, package, &btx)?
            .into_iter()
            .map(|d| d.share)
            .collect())
    }

    /// The defaults of every texture of `btx` from the maps of `package`.
    fn defaults(
        &self,
        store: &mut Store,
        package: u16,
        btx: &Nsbtx,
    ) -> Result<Vec<Defaults>, KernelError> {
        let family = store.project().family;
        let maps = self.map_votes(store, package);
        let base = base_of(&maps);
        Ok(btx
            .textures
            .iter()
            .map(|t| defaults(DEFAULT_RULE, family, &t.name, &maps, &base))
            .collect())
    }

    /// The maps of `package` as the vote counts them. A map that does not load does not
    /// count; a map whose model does not read counts only in K2's base (it binds nothing).
    fn map_votes(&self, store: &mut Store, package: u16) -> Vec<MapVotes> {
        let mut maps = Vec::new();
        for &map in self.maps_of(package) {
            let Ok(file) = store.map(map) else {
                continue;
            };
            let names = material_texture_names(&file.model).unwrap_or_default();
            maps.push(MapVotes::new(names, file.permissions.clone()));
        }
        maps
    }
}

/// [`Deriver::derive`] of one package.
pub fn derive_tileset(store: &mut Store, package: u16) -> Result<Tileset, KernelError> {
    Deriver::new(store)?.derive(store, package)
}

/// The role of a decided `kind`: the class of its behaviour, `grass` and `water` as such,
/// every other `ground`.
fn role_of(family: Family, kind: u8) -> TileRole {
    match tile_behavior(family, kind).map(|b| b.class) {
        Some(class) if class == "grass" => TileRole::Grass,
        Some(class) if class == "water" => TileRole::Water,
        _ => TileRole::Ground,
    }
}

/// The permissions of one map of a package as the vote counts them: the texture names its
/// model binds (sorted, each once; none when the model does not read), its cells per kind
/// and its cells.
struct MapVotes {
    names: Vec<String>,
    counts: [u32; 256],
    cells: Vec<Cell>,
}

impl MapVotes {
    fn new(mut names: Vec<String>, cells: Vec<Cell>) -> MapVotes {
        names.sort();
        names.dedup();
        let mut counts = [0u32; 256];
        for cell in &cells {
            counts[usize::from(cell.kind)] += 1;
        }
        MapVotes {
            names,
            counts,
            cells,
        }
    }

    fn binds(&self, texture: &str) -> bool {
        self.names
            .binary_search_by(|n| n.as_str().cmp(texture))
            .is_ok()
    }
}

/// The permission defaults of one texture: decided or not, `kind` k* and the step sound of
/// a decided one, and the share of the top kind among the votes (`None`: no map binds it).
struct Defaults {
    decided: bool,
    kind: u8,
    sound: u8,
    share: Option<f64>,
}

/// K2's base: the share of every kind among all cells of the package's maps.
fn base_of(maps: &[MapVotes]) -> [f64; 256] {
    let mut base = [0f64; 256];
    let mut total = 0usize;
    for map in maps {
        for (b, &n) in base.iter_mut().zip(&map.counts) {
            *b += f64::from(n);
        }
        total += map.cells.len();
    }
    if total > 0 {
        let total = total as f64;
        for b in &mut base {
            *b /= total;
        }
    }
    base
}

/// The defaults of `texture` by `rule` (plan 6.3a, „Vorgaben je Textur“): the maps that bind
/// it vote with each cell `1 / |names|` of the map for the cell's kind; K1 takes the top kind
/// k* (ties: the smaller kind) from [`UNIQUE_SHARE`] of the votes; K2 the kind other than 0
/// with at least 5 % of the votes and the highest quotient of its share by its `base` share
/// (ties: the smaller kind) when that is at least 2, else kind 0 from [`UNIQUE_SHARE`] of the
/// votes; `None` decides nothing. The step sound (HeartGold/SoulSilver) is the most frequent
/// `collision & 0x7f` of the weighted cells with the decided kind (ties: the smaller).
fn defaults(
    rule: VoteRule,
    family: Family,
    texture: &str,
    maps: &[MapVotes],
    base: &[f64; 256],
) -> Defaults {
    let weighted: Vec<(&MapVotes, f64)> = maps
        .iter()
        .filter(|m| m.binds(texture))
        .map(|m| (m, 1.0 / m.names.len() as f64))
        .collect();
    let mut votes = [0f64; 256];
    for (map, weight) in &weighted {
        for (v, &n) in votes.iter_mut().zip(&map.counts) {
            *v += f64::from(n) * weight;
        }
    }
    let total: f64 = votes.iter().sum();
    let mut out = Defaults {
        decided: false,
        kind: 0,
        sound: 0,
        share: None,
    };
    if total <= 0.0 {
        return out;
    }
    let top = argmax(&votes);
    let share = votes[top] / total;
    out.share = Some(share);
    let decided = match rule {
        VoteRule::None => None,
        VoteRule::K1 => (share >= UNIQUE_SHARE).then_some(top),
        VoteRule::K2 => standout(&votes, total, base),
    };
    let Some(kind) = decided else {
        return out;
    };
    out.decided = true;
    out.kind = u8::try_from(kind).unwrap_or(0);
    if family == Family::Hgss {
        let mut sounds = [0f64; SOUND_MAX as usize + 1];
        for (map, weight) in &weighted {
            for cell in map.cells.iter().filter(|c| usize::from(c.kind) == kind) {
                sounds[usize::from(cell.collision & SOUND_MAX)] += weight;
            }
        }
        out.sound = u8::try_from(argmax(&sounds)).unwrap_or(0);
    }
    out
}

/// K2's kind of `votes`: the candidate that stands out, else 0 when it has
/// [`UNIQUE_SHARE`] of the votes, else none.
fn standout(votes: &[f64; 256], total: f64, base: &[f64; 256]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for k in 1..votes.len() {
        let p = votes[k] / total;
        if p < K2_MIN_SHARE || base[k] <= 0.0 {
            continue;
        }
        let ratio = p / base[k];
        if best.is_none_or(|(_, r)| ratio > r) {
            best = Some((k, ratio));
        }
    }
    match best {
        Some((k, ratio)) if ratio >= K2_RATIO => Some(k),
        _ => (votes[0] / total >= UNIQUE_SHARE).then_some(0),
    }
}

/// The index of the largest value, the first of equal ones (0 for all zero).
fn argmax(values: &[f64]) -> usize {
    let mut best = 0;
    for (i, &v) in values.iter().enumerate() {
        if v > values[best] {
            best = i;
        }
    }
    best
}

/// „Kartentextur-Paket 0200 gibt es nicht (74 Pakete).“
pub(crate) fn missing_package(package: u16, count: usize) -> KernelError {
    let count = if count == 1 {
        "1 Paket".to_owned()
    } else {
        format!("{count} Pakete")
    };
    KernelError::Refused(format!(
        "Kartentextur-Paket {package:04} gibt es nicht ({count})."
    ))
}

/// The member file of map texture package `package` (its archive unpacked first); refused
/// when the package is no member ([`missing_package`]).
pub(crate) fn package_file(project: &Project, package: u16) -> Result<PathBuf, KernelError> {
    let count = project.ensure_unpacked(AssetKind::MapTexture)?;
    if usize::from(package) >= count {
        return Err(missing_package(package, count));
    }
    Ok(project.member_path(AssetKind::MapTexture, package))
}

/// The package in `path` read; refused with the reason when it does not read („Kartentextur-
/// Paket 0006 lässt sich nicht lesen: …“).
pub(crate) fn read_package_file(path: &Path, package: u16) -> Result<Nsbtx, KernelError> {
    let bytes = std::fs::read(path).map_err(io_at(path))?;
    Nsbtx::read(&bytes).map_err(|e| {
        KernelError::Refused(format!(
            "Kartentextur-Paket {package:04} lässt sich nicht lesen: {e}"
        ))
    })
}

fn read_package(project: &Project, package: u16) -> Result<Nsbtx, KernelError> {
    read_package_file(&package_file(project, package)?, package)
}

/// RGBA of `texture` with its tile's palette (none: palette 0); `None` when the package or the
/// texture is missing or the package does not read (synthetic tileset), when the tile names a
/// palette the package lacks, when the package has no palette for a paletted texture or when
/// the texture's format has no decoder (4 × 4 compressed, format 5).
pub fn tileset_texture(
    project: &Project,
    tileset: &Tileset,
    texture: &str,
) -> Result<Option<Rgba>, KernelError> {
    let Some(tile) = tileset.tiles.iter().find(|t| t.texture == texture) else {
        return Ok(None);
    };
    let path = match package_file(project, tileset.package) {
        Ok(path) => path,
        Err(KernelError::Refused(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    let btx = match read_package_file(&path, tileset.package) {
        Ok(btx) => btx,
        // A package that does not read has no texture (the synthetic project's stub).
        Err(KernelError::Refused(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    let Some(index) = btx.textures.iter().position(|t| t.name == texture) else {
        return Ok(None);
    };
    let palette = match &tile.palette {
        None => 0,
        Some(name) => match btx.palettes.iter().position(|p| &p.name == name) {
            Some(i) => i,
            None => return Ok(None),
        },
    };
    match btx.decode(index, palette) {
        Ok(rgba) => Ok(Some(rgba)),
        // No decoder for the format (4 × 4 compressed), or a paletted texture in a package
        // without palettes.
        Err(FormatError::Unsupported { .. } | FormatError::OutOfRange { .. }) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// What [`save_tileset`] wrote.
#[derive(Debug, Clone, Serialize)]
pub struct TilesetSaveReport {
    pub package: u16,
    #[serde(serialize_with = "crate::relpath::ser")]
    pub file: PathBuf,
    pub tiles: usize,
    pub vanilla: usize,
    pub unclear: usize,
    pub change: ChangeId,
}

/// Writes the derived tileset of `package` as one file step (FileStepKind::Reset, asset
/// HEADER_TABLE, label „Tileset 0006 abgeleitet“); refused in the DSPRE layout and when the
/// file exists („data/tilesets/0006.json gibt es schon.“).
pub fn save_tileset(store: &mut Store, package: u16) -> Result<TilesetSaveReport, KernelError> {
    let project = store.project_arc();
    project.editable()?;
    if let Some(why) = store.busy() {
        return Err(KernelError::Busy(why.to_owned()));
    }
    if *project.source_mode() != SourceMode::TextProject {
        return Err(KernelError::Refused(WORLD_NEEDS_TEXT_PROJECT.to_owned()));
    }
    let rel = tileset_rel(package);
    if project.root.join(&rel).exists() {
        return Err(KernelError::Refused(format!(
            "{} gibt es schon.",
            crate::relpath::slash(&rel)
        )));
    }
    let tileset = derive_tileset(store, package)?;
    let dir = project.root.join(TILESETS_DIR);
    std::fs::create_dir_all(&dir).map_err(io_at(&dir))?;
    let step = FileStep {
        asset: HEADER_TABLE,
        files: vec![FileChange {
            path: rel.clone(),
            before: None,
            after: Some(to_canonical(&tileset)),
        }],
        rotom_entry: None,
        kind: FileStepKind::Reset,
    };
    write_file_step(&project, &step)?;
    let change = store.record_file_step(&format!("Tileset {package:04} abgeleitet"), step);
    let tiles = tileset.tiles.len();
    let vanilla = tileset
        .tiles
        .iter()
        .filter(|t| t.origin == TileOrigin::Vanilla)
        .count();
    Ok(TilesetSaveReport {
        package,
        file: rel,
        tiles,
        vanilla,
        unclear: tiles - vanilla,
        change,
    })
}

/// The package of a tileset file name `NNNN.json` (four decimal digits).
fn package_of(name: &str) -> Option<u16> {
    let digits = name.strip_suffix(".json")?;
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{Value, json};

    use super::*;
    use crate::textproject::canon::to_canonical;

    fn keys(v: &Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
    }

    #[test]
    fn synthetic_tileset_has_six_roles() {
        let t = Tileset::synthetic();
        assert_eq!(t.version, TILESET_VERSION);
        assert_eq!(t.package, SYNTHETIC_PACKAGE);
        assert!(t.edge_sets.is_empty());
        let names: Vec<&str> = t.tiles.iter().map(|t| t.texture.as_str()).collect();
        assert_eq!(
            names,
            [
                "syn_boden",
                "syn_gras",
                "syn_wasser",
                "syn_wand",
                "syn_rampe",
                "syn_stueck"
            ]
        );
        let roles: Vec<TileRole> = t.tiles.iter().map(|t| t.role).collect();
        assert_eq!(
            roles,
            [
                TileRole::Ground,
                TileRole::Grass,
                TileRole::Water,
                TileRole::Wall,
                TileRole::Ramp,
                TileRole::Piece
            ]
        );
        let sizes: Vec<[u16; 2]> = t.tiles.iter().map(|t| t.size).collect();
        assert_eq!(
            sizes,
            [[16, 16], [32, 32], [16, 16], [16, 16], [16, 16], [16, 16]]
        );
        let heights: Vec<HeightBehavior> = t.tiles.iter().map(|t| t.height).collect();
        assert_eq!(
            heights,
            [
                HeightBehavior::Flat,
                HeightBehavior::Flat,
                HeightBehavior::Flat,
                HeightBehavior::Wall,
                HeightBehavior::Ramp,
                HeightBehavior::Flat
            ]
        );
        let kinds: Vec<(u8, u8)> = t.tiles.iter().map(|t| (t.kind, t.collision)).collect();
        assert_eq!(
            kinds,
            [(0, 0), (0x02, 0), (0x15, 0), (0, 0x80), (0, 0), (0, 0)]
        );
        for (i, tile) in t.tiles.iter().enumerate() {
            assert_eq!(usize::from(tile.texture_index), i);
            assert_eq!(tile.origin, TileOrigin::Synthetic);
            assert_eq!(tile.palette, None);
            assert_eq!(tile.sound, 0);
            assert_eq!(tile.model, None);
            assert!(tile.texture.len() <= TEXTURE_NAME_MAX);
            assert_eq!(tile.wall, if i == 0 { Some(3) } else { None });
            assert_eq!(tile.footprint, if i == 5 { Some([2, 2]) } else { None });
        }
        assert_eq!(t.tile(3).unwrap().texture, "syn_wand");
        assert_eq!(t.tile(6), None);
        // The synthetic tileset is a valid file in every family.
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            assert_eq!(
                tileset_problems(&t, SYNTHETIC_PACKAGE, family),
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn tileset_json_round_trips_and_refuses_unknown_keys() {
        let mut t = Tileset::synthetic();
        t.tiles[1].palette = Some("syn_gras_pl".to_owned());
        t.tiles[1].sound = 3;
        let bytes = to_canonical(&t);
        let back: Tileset = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, t);
        assert_eq!(to_canonical(&back), bytes);

        // Keys in the order of the structs; empty and default fields left out.
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(keys(&v), ["version", "package", "tiles"]);
        assert_eq!(
            keys(&v["tiles"][0]),
            [
                "texture",
                "texture_index",
                "size",
                "role",
                "kind",
                "collision",
                "height",
                "wall",
                "origin"
            ]
        );
        assert_eq!(
            keys(&v["tiles"][1]),
            [
                "texture",
                "texture_index",
                "palette",
                "size",
                "role",
                "kind",
                "collision",
                "height",
                "sound",
                "origin"
            ]
        );
        assert_eq!(v["tiles"][5]["footprint"], json!([2, 2]));
        assert_eq!(v["tiles"][3]["role"], "wall");
        assert_eq!(v["tiles"][3]["height"], "wall");
        assert_eq!(v["tiles"][0]["origin"], "synthetic");

        // The reserved keys read (the check names them), unknown keys do not.
        let mut reserved = v.clone();
        reserved["edge_sets"] = json!([{}]);
        reserved["tiles"][0]["model"] = json!("baum");
        let r: Tileset = serde_json::from_value(reserved).unwrap();
        assert_eq!(r.edge_sets.len(), 1);
        assert_eq!(r.tiles[0].model.as_deref(), Some("baum"));

        let mut top = v.clone();
        top["tile"] = json!([]);
        let e = serde_json::from_value::<Tileset>(top).unwrap_err();
        assert!(e.to_string().contains("unknown field `tile`"), "{e}");
        let mut inner = v.clone();
        inner["tiles"][2]["colision"] = json!(0);
        let e = serde_json::from_value::<Tileset>(inner).unwrap_err();
        assert!(e.to_string().contains("unknown field `colision`"), "{e}");
        let mut role = v;
        role["tiles"][2]["role"] = json!("lava");
        assert!(serde_json::from_value::<Tileset>(role).is_err());
    }

    #[test]
    fn tileset_problems_name_each_rule() {
        let problems = |change: &dyn Fn(&mut Tileset), stem: u16, family: Family| {
            let mut t = Tileset::synthetic();
            change(&mut t);
            tileset_problems(&t, stem, family)
        };
        let pt = Family::Pt;
        let none = |_: &mut Tileset| {};

        assert_eq!(
            problems(&|t| t.version = 2, 0, pt),
            ["version: Version 2 kennt diese Fassung nicht (1)."]
        );
        assert_eq!(
            problems(&none, 6, pt),
            ["package: Paket 0000 passt nicht zum Dateinamen 0006.json."]
        );
        assert_eq!(
            problems(&|t| t.edge_sets.push(json!({})), 0, pt),
            ["edge_sets: Kantensätze kommen mit 6.4b."]
        );
        assert_eq!(
            problems(&|t| t.tiles[2].model = Some("baum".to_owned()), 0, pt),
            ["tiles[2].model: Teilmodell-Dateien kommen mit 6.5b."]
        );
        assert_eq!(
            problems(&|t| t.tiles[0].wall = Some(6), 0, pt),
            ["tiles[0].wall: Kachel 0x0006 gibt es im Tileset 0000 nicht (6 Kacheln)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[0].wall = Some(2), 0, pt),
            ["tiles[0].wall: Kachel 0x0002 ist keine Wand (Rolle `water`)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].footprint = Some([1, 1]), 0, pt),
            ["tiles[1].footprint: einen Fußabdruck haben nur Teilmodelle (`piece`)."]
        );
        assert_eq!(
            problems(&|t| t.tiles[5].footprint = None, 0, pt),
            ["tiles[5]: ein Teilmodell (`piece`) braucht einen Fußabdruck (`footprint`)."]
        );
        assert_eq!(
            problems(
                &|t| t.tiles[4].texture = "syn_rampe_zu_lang".to_owned(),
                0,
                pt
            ),
            [
                "tiles[4].texture: der Texturname `syn_rampe_zu_lang` ist 17 Byte lang (höchstens 16)."
            ]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, pt),
            ["tiles[1].sound: Geräusche gibt es nur in HeartGold/SoulSilver."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, Family::Dp),
            ["tiles[1].sound: Geräusche gibt es nur in HeartGold/SoulSilver."]
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 3, 0, Family::Hgss),
            Vec::<String>::new()
        );
        assert_eq!(
            problems(&|t| t.tiles[1].sound = 0x80, 0, Family::Hgss),
            ["tiles[1].sound: 0x80 hat mehr als 7 Bit (höchstens 0x7f)."]
        );
        // Several problems: all of them, file first, then tile by tile.
        assert_eq!(
            problems(
                &|t| {
                    t.edge_sets.push(json!([]));
                    t.tiles[3].wall = Some(0x0120);
                    t.tiles[2].model = Some("x".to_owned());
                },
                1,
                pt
            ),
            [
                "package: Paket 0000 passt nicht zum Dateinamen 0001.json.",
                "edge_sets: Kantensätze kommen mit 6.4b.",
                "tiles[2].model: Teilmodell-Dateien kommen mit 6.5b.",
                "tiles[3].wall: Kachel 0x0120 gibt es im Tileset 0000 nicht (6 Kacheln).",
            ]
        );
    }

    #[test]
    fn tile_outside_names_tile_package_and_count() {
        assert_eq!(
            tile_outside("world.layers[0].tiles[4]", 0x0120, 6, 94),
            "world.layers[0].tiles[4]: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln)."
        );
        assert_eq!(
            tile_outside("tiles[0].wall", 0x00ab, 12, 1),
            "tiles[0].wall: Kachel 0x00ab gibt es im Tileset 0012 nicht (1 Kachel)."
        );
    }

    #[test]
    fn tileset_rel_is_four_digits() {
        assert_eq!(tileset_rel(6), Path::new("data/tilesets/0006.json"));
        assert_eq!(tileset_rel(0), Path::new("data/tilesets/0000.json"));
        assert_eq!(tileset_rel(1234), Path::new("data/tilesets/1234.json"));
        assert!(tileset_rel(6).starts_with(TILESETS_DIR));
    }

    #[test]
    fn read_tileset_file_is_none_some_or_refused() {
        let synthetic = crate::test_support::synthetic_project();
        let project = Project::open(synthetic.dir.path()).unwrap();
        assert!(read_tileset_file(&project, 0).unwrap().is_none());

        let dir = project.root.join(TILESETS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let t = Tileset::synthetic();
        std::fs::write(project.root.join(tileset_rel(0)), to_canonical(&t)).unwrap();
        assert_eq!(read_tileset_file(&project, 0).unwrap(), Some(t));

        std::fs::write(
            project.root.join(tileset_rel(7)),
            b"{\"version\": 1, \"tile\": []}",
        )
        .unwrap();
        match read_tileset_file(&project, 7) {
            Err(KernelError::Refused(why)) => {
                assert!(why.starts_with("data/tilesets/0007.json: "), "{why}");
                assert!(why.contains("unknown field `tile`"), "{why}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    // ---- Derivation from the package and the vanilla permissions (Task 5) ----

    use std::sync::Arc;

    use g4_formats::map::MapFile;
    use g4_formats::narc::Narc;
    use g4_formats::test_support::{bmd0, btx0, mdl0_block};

    use crate::test_support::{TextFixture, synthetic_map_bytes, synthetic_project};
    use crate::textproject::ExportOptions;

    /// Colour 1 of the palettes `t0_pl` (red) and `t1` (blue), BGR555.
    const RED: u16 = 0x001f;
    const BLUE: u16 = 0x7c00;

    fn name16(name: &str) -> [u8; 16] {
        let mut record = [0u8; 16];
        record[..name.len()].copy_from_slice(name.as_bytes());
        record
    }

    /// `bytes` with the 16-byte name record `from` renamed to `to`.
    fn renamed(mut bytes: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
        let at = bytes.windows(16).position(|w| w == name16(from)).unwrap();
        bytes[at..at + 16].copy_from_slice(&name16(to));
        bytes
    }

    /// The synthetic package 0: textures `t0` 16 × 16, `t1` 32 × 32, `t2` 16 × 16 (16 colours,
    /// every texel colour 1), palettes `t0_pl` (colour 1 red) and `t1` (blue).
    fn package_bytes() -> Vec<u8> {
        let small = [0x11u8; 128];
        let big = [0x11u8; 512];
        let palette = |colour: u16| {
            let mut p = vec![0u16; 16];
            p[1] = colour;
            p
        };
        let (red, blue) = (palette(RED), palette(BLUE));
        let mut bytes = btx0(
            &[
                (16, 16, 3, false, &small),
                (32, 32, 3, false, &big),
                (16, 16, 3, false, &small),
            ],
            &[&red, &blue],
        );
        for (from, to) in [
            ("tex0", "t0"),
            ("tex1", "t1"),
            ("tex2", "t2"),
            ("pal0", "t0_pl"),
            ("pal1", "t1"),
        ] {
            bytes = renamed(bytes, from, to);
        }
        bytes
    }

    fn package() -> Nsbtx {
        Nsbtx::read(&package_bytes()).unwrap()
    }

    /// Map 0 with a model that binds `t0` and permissions of 1 000 cells `kind 0x02` and 24
    /// cells 0.
    fn map0_bytes() -> Vec<u8> {
        let mut map = MapFile::read(&synthetic_map_bytes(0), Family::Pt).unwrap();
        map.model = bmd0(&[mdl0_block(&["m"], &["t0"], &["t0_pl"])]);
        for (i, cell) in map.permissions.iter_mut().enumerate() {
            *cell = Cell {
                kind: if i < 1000 { 0x02 } else { 0 },
                collision: 0,
            };
        }
        map.write(Family::Pt)
    }

    /// The synthetic project with package 0 of [`package_bytes`] and map 0 of [`map0_bytes`]
    /// (maps 0 and 1 show packages 0 and 2; map 1's model does not read), exported to a text
    /// project when `export`.
    fn package_fixture(export: bool) -> TextFixture {
        let fx = TextFixture::with(|project| {
            let put = |rel: &str, members: Vec<Vec<u8>>| {
                std::fs::write(project.root.join(rel), Narc { members }.write()).unwrap();
            };
            put(
                "files/fielddata/areadata/area_map_tex/map_tex_set.narc",
                vec![package_bytes()],
            );
            put(
                "files/fielddata/land_data/land_data.narc",
                vec![map0_bytes(), synthetic_map_bytes(1), synthetic_map_bytes(2)],
            );
        });
        if export {
            crate::textproject::export(
                &fx.project(),
                &fx.base(),
                &ExportOptions {
                    adopt_scripts: true,
                    force: false,
                },
            )
            .unwrap();
        }
        fx
    }

    fn store_of(fx: &TextFixture) -> Store {
        Store::new(Arc::new(fx.project()))
    }

    fn refusal<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
        match result {
            Err(KernelError::Refused(why)) => why,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn derive_gives_one_ground_tile_per_texture() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let t = derive_tileset(&mut store, 0).unwrap();
        assert_eq!(t.version, TILESET_VERSION);
        assert_eq!(t.package, 0);
        assert!(t.edge_sets.is_empty());
        let names: Vec<&str> = t.tiles.iter().map(|t| t.texture.as_str()).collect();
        assert_eq!(names, ["t0", "t1", "t2"]);
        let indices: Vec<u16> = t.tiles.iter().map(|t| t.texture_index).collect();
        assert_eq!(indices, [0, 1, 2]);
        let sizes: Vec<[u16; 2]> = t.tiles.iter().map(|t| t.size).collect();
        assert_eq!(sizes, [[16, 16], [32, 32], [16, 16]]);
        let palettes: Vec<Option<&str>> = t.tiles.iter().map(|t| t.palette.as_deref()).collect();
        assert_eq!(palettes, [Some("t0_pl"), Some("t1"), None]);
        for tile in &t.tiles {
            assert_eq!(tile.height, HeightBehavior::Flat);
            assert_eq!(tile.collision, 0);
            assert_eq!(tile.sound, 0);
            assert_eq!(
                (tile.wall, tile.footprint, tile.model.as_ref()),
                (None, None, None)
            );
        }
        assert_eq!(t.tiles[1].role, TileRole::Ground);
        assert_eq!(t.tiles[2].role, TileRole::Ground);
        assert_eq!(tileset_problems(&t, 0, Family::Pt), Vec::<String>::new());

        // The DSPRE layout derives the same (tileset_get goes there).
        let dspre = package_fixture(false);
        assert_eq!(derive_tileset(&mut store_of(&dspre), 0).unwrap(), t);
    }

    #[test]
    fn derive_takes_the_vote_of_the_maps() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let deriver = Deriver::new(&mut store).unwrap();
        assert_eq!(deriver.maps_of(0), [0, 1]);
        assert_eq!(deriver.maps_of(2), [0, 1]);
        assert_eq!(deriver.maps_of(1), [0u16; 0]);
        let t = deriver.derive(&mut store, 0).unwrap();
        let defaults: Vec<(TileRole, u8, u8, TileOrigin)> = t
            .tiles
            .iter()
            .map(|t| (t.role, t.kind, t.collision, t.origin))
            .collect();
        assert_eq!(
            defaults,
            [
                (TileRole::Grass, 0x02, 0, TileOrigin::Vanilla),
                (TileRole::Ground, 0, 0, TileOrigin::Unclear),
                (TileRole::Ground, 0, 0, TileOrigin::Unclear),
            ]
        );
        // Package 2 (named by area data 1) is no member: refused with the count.
        assert_eq!(
            refusal(deriver.derive(&mut store, 2)),
            "Kartentextur-Paket 0002 gibt es nicht (1 Paket)."
        );
    }

    /// One map of the vote: the names its model binds and its permissions.
    fn vote_map(names: &[&str], cells: &[(u8, u8, usize)]) -> MapVotes {
        let mut permissions = Vec::new();
        for &(kind, collision, n) in cells {
            permissions.extend(std::iter::repeat_n(Cell { kind, collision }, n));
        }
        assert_eq!(permissions.len(), 1024);
        MapVotes::new(names.iter().map(|n| (*n).to_owned()).collect(), permissions)
    }

    #[test]
    fn vote_rules_decide_as_the_plan_says() {
        let decide = |rule: VoteRule, family: Family, texture: &str, maps: &[MapVotes]| {
            let d = defaults(rule, family, texture, maps, &base_of(maps));
            (d.decided.then_some(d.kind), d.sound)
        };
        let pt = Family::Pt;

        // K1: the majority wins from 0.6 of the votes; a map votes 1 / |names| per cell.
        let maps = [
            vote_map(&["a", "b"], &[(0x02, 0, 1024)]),
            vote_map(&["a"], &[(0, 0, 1024)]),
        ];
        assert_eq!(decide(VoteRule::K1, pt, "a", &maps), (Some(0), 0));
        assert_eq!(decide(VoteRule::K1, pt, "b", &maps), (Some(0x02), 0));
        assert_eq!(decide(VoteRule::K1, pt, "c", &maps), (None, 0));
        let close = [vote_map(&["a"], &[(0x02, 0, 600), (0, 0, 424)])];
        assert_eq!(decide(VoteRule::K1, pt, "a", &close), (None, 0));
        assert_eq!(
            defaults(VoteRule::K1, pt, "a", &close, &base_of(&close)).share,
            Some(600.0 / 1024.0)
        );
        assert_eq!(
            defaults(VoteRule::K1, pt, "c", &close, &base_of(&close)).share,
            None
        );

        // K2: a kind that stands out twice against the package's maps, ties to the smaller
        // kind; else kind 0 from 0.6; else unclear. K1 keeps kind 0 there.
        let maps = [
            vote_map(&["x"], &[(0, 0, 1024)]),
            vote_map(&["y"], &[(0, 0, 1024)]),
            vote_map(&["c"], &[(0x15, 0, 200), (0, 0, 824)]),
            vote_map(&["d"], &[(0x15, 0, 200), (0x02, 0, 200), (0, 0, 624)]),
        ];
        assert_eq!(decide(VoteRule::K2, pt, "c", &maps), (Some(0x15), 0));
        assert_eq!(decide(VoteRule::K1, pt, "c", &maps), (Some(0), 0));
        assert_eq!(decide(VoteRule::K2, pt, "d", &maps), (Some(0x02), 0));
        assert_eq!(decide(VoteRule::K2, pt, "x", &maps), (Some(0), 0));
        let even = [vote_map(&["e"], &[(0x02, 0, 512), (0, 0, 512)])];
        assert_eq!(decide(VoteRule::K2, pt, "e", &even), (None, 0));

        // None: every texture unclear.
        assert_eq!(decide(VoteRule::None, pt, "b", &maps), (None, 0));

        // HeartGold/SoulSilver: the most frequent sound (bits 0 to 6) of the cells with k*.
        let hgss = [vote_map(
            &["g"],
            &[(0x02, 0x83, 600), (0x02, 0x04, 400), (0, 0x05, 24)],
        )];
        assert_eq!(
            decide(VoteRule::K1, Family::Hgss, "g", &hgss),
            (Some(0x02), 0x03)
        );
        assert_eq!(decide(VoteRule::K1, pt, "g", &hgss), (Some(0x02), 0));
        let tie = [vote_map(&["g"], &[(0x02, 0x06, 512), (0x02, 0x04, 512)])];
        assert_eq!(
            decide(VoteRule::K1, Family::Hgss, "g", &tie),
            (Some(0x02), 0x04)
        );
    }

    #[test]
    fn palette_rule_takes_pl_same_name_index_or_none() {
        let palettes =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| (*n).to_owned()).collect() };
        let pick = |texture: &str, index: usize, format: u8, names: &[&str]| {
            palette_of(texture, index, format, &palettes(names))
        };
        assert_eq!(pick("t0", 0, 3, &["t0", "t0_pl"]).as_deref(), Some("t0_pl"));
        assert_eq!(pick("t0", 0, 3, &["x", "t0"]).as_deref(), Some("t0"));
        // `_pl` cut to 16 bytes.
        assert_eq!(
            pick("abcdefghijklmn", 0, 3, &["abcdefghijklmn_p"]).as_deref(),
            Some("abcdefghijklmn_p")
        );
        // Else the palette with the texture's index, else none; format 7 never has one.
        assert_eq!(pick("t5", 1, 3, &["p0", "p1"]).as_deref(), Some("p1"));
        assert_eq!(pick("t5", 2, 3, &["p0", "p1"]), None);
        assert_eq!(pick("t0", 0, 7, &["t0_pl"]), None);
    }

    #[test]
    fn textures_only_matches_derive_in_names_and_count() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let only = textures_only(0, &package());
        let derived = derive_tileset(&mut store, 0).unwrap();
        assert_eq!(only.package, 0);
        assert_eq!(only.tiles.len(), derived.tiles.len());
        for (a, b) in only.tiles.iter().zip(&derived.tiles) {
            assert_eq!(
                (&a.texture, a.texture_index, a.size, &a.palette),
                (&b.texture, b.texture_index, b.size, &b.palette)
            );
            assert_eq!(
                (a.role, a.kind, a.collision, a.sound, a.height, a.origin),
                (
                    TileRole::Ground,
                    0,
                    0,
                    0,
                    HeightBehavior::Flat,
                    TileOrigin::Unclear
                )
            );
        }
    }

    #[test]
    fn store_tileset_prefers_the_file_and_rereads_it() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let only = textures_only(0, &package());
        let first = store.tileset(0).unwrap();
        assert_eq!(*first, only);
        assert!(Arc::ptr_eq(&first, &store.tileset(0).unwrap()), "cached");

        // A file wins and is read again once it changes.
        let mut file = Tileset::synthetic();
        fx.write("data/tilesets/0000.json", &to_canonical(&file));
        assert_eq!(*store.tileset(0).unwrap(), file);
        file.tiles.truncate(2);
        fx.write("data/tilesets/0000.json", &to_canonical(&file));
        assert_eq!(*store.tileset(0).unwrap(), file);
        // Gone: the textures again.
        std::fs::remove_file(fx.root().join(tileset_rel(0))).unwrap();
        assert_eq!(*store.tileset(0).unwrap(), only);

        assert_eq!(
            refusal(store.tileset(5)),
            "Kartentextur-Paket 0005 gibt es nicht (1 Paket)."
        );
        // A package that does not read names why.
        let synth = synthetic_project();
        let mut plain = Store::new(Arc::new(Project::open(synth.dir.path()).unwrap()));
        let why = refusal(plain.tileset(0));
        assert!(
            why.starts_with("Kartentextur-Paket 0000 lässt sich nicht lesen: "),
            "{why}"
        );
    }

    #[test]
    fn tileset_texture_decodes_with_the_tile_palette() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let project = fx.project();
        let btx = package();
        let t = derive_tileset(&mut store, 0).unwrap();
        let texture = |t: &Tileset, name: &str| tileset_texture(&project, t, name).unwrap();
        let t0 = texture(&t, "t0").unwrap();
        assert_eq!((t0.width, t0.height), (16, 16));
        assert_eq!(t0, btx.decode(0, 0).unwrap());
        let t1 = texture(&t, "t1").unwrap();
        assert_eq!((t1.width, t1.height), (32, 32));
        assert_eq!(t1, btx.decode(1, 1).unwrap());
        assert_ne!(t0.pixels[..4], t1.pixels[..4], "red against blue");
        // Without a palette: palette 0.
        assert_eq!(texture(&t, "t2").unwrap(), btx.decode(2, 0).unwrap());
        // The palette of the tile, as the file has it.
        let mut changed = t.clone();
        changed.tiles[0].palette = Some("t1".to_owned());
        assert_eq!(texture(&changed, "t0").unwrap(), btx.decode(0, 1).unwrap());
        changed.tiles[0].palette = Some("fehlt".to_owned());
        assert_eq!(texture(&changed, "t0"), None);
        // A texture the tileset or the package lacks, a missing package: none.
        assert_eq!(texture(&t, "t9"), None);
        assert_eq!(texture(&Tileset::synthetic(), "syn_boden"), None);
        let mut far = t.clone();
        far.package = 9;
        assert_eq!(texture(&far, "t0"), None);
    }

    #[test]
    fn the_synthetic_tileset_on_the_synthetic_project_has_no_texture() {
        // Package 0 of the synthetic project is a stub that does not read.
        let fx = TextFixture::exported();
        fx.write(
            "data/tilesets/0000.json",
            &to_canonical(&Tileset::synthetic()),
        );
        let project = fx.project();
        assert_eq!(
            tileset_texture(&project, &Tileset::synthetic(), "syn_boden").unwrap(),
            None
        );
        // The file answers without reading the package (Tasks 6, 7, 9 rely on it).
        let mut store = store_of(&fx);
        assert_eq!(*store.tileset(0).unwrap(), Tileset::synthetic());
    }

    #[test]
    fn a_paletted_texture_without_palettes_has_no_texture() {
        let fx = TextFixture::with(|project| {
            let texels = [0x11u8; 128];
            let bytes = renamed(btx0(&[(16, 16, 3, false, &texels)], &[]), "tex0", "t0");
            std::fs::write(
                project
                    .root
                    .join("files/fielddata/areadata/area_map_tex/map_tex_set.narc"),
                Narc {
                    members: vec![bytes],
                }
                .write(),
            )
            .unwrap();
        });
        let project = fx.project();
        let btx = read_package(&project, 0).unwrap();
        let t = textures_only(0, &btx);
        assert_eq!(t.tiles[0].palette, None);
        assert_eq!(tileset_texture(&project, &t, "t0").unwrap(), None);
    }

    #[test]
    fn save_tileset_is_one_undo_step() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        let derived = derive_tileset(&mut store, 0).unwrap();
        let path = fx.root().join(tileset_rel(0));
        let report = save_tileset(&mut store, 0).unwrap();
        assert_eq!(
            (report.package, report.tiles, report.vanilla, report.unclear),
            (0, 3, 1, 2)
        );
        assert_eq!(report.file, tileset_rel(0));
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["file"], "data/tilesets/0000.json");
        assert_eq!(std::fs::read(&path).unwrap(), to_canonical(&derived));
        assert_eq!(
            read_tileset_file(&fx.project(), 0).unwrap(),
            Some(derived.clone())
        );
        assert_eq!(*store.tileset(0).unwrap(), derived);
        let (undo, _) = store.history();
        assert_eq!(undo, ["Tileset 0000 abgeleitet"]);

        assert_eq!(store.undo().unwrap(), [HEADER_TABLE]);
        assert!(!path.exists());
        assert_eq!(*store.tileset(0).unwrap(), textures_only(0, &package()));
        store.redo().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), to_canonical(&derived));
    }

    #[test]
    fn save_tileset_refuses_existing_and_dspre() {
        let fx = package_fixture(true);
        let mut store = store_of(&fx);
        save_tileset(&mut store, 0).unwrap();
        assert_eq!(
            refusal(save_tileset(&mut store, 0)),
            "data/tilesets/0000.json gibt es schon."
        );
        assert_eq!(store.history().0.len(), 1);

        let dspre = package_fixture(false);
        let mut store = store_of(&dspre);
        assert_eq!(
            refusal(save_tileset(&mut store, 0)),
            WORLD_NEEDS_TEXT_PROJECT
        );
        assert!(!dspre.root().join(tileset_rel(0)).exists());
        assert!(store.history().0.is_empty());
    }
}
