//! The checks of the world maps in the text-project check ([`hints`](crate::textproject::check)),
//! not in the check register (`checks/*` is phase 6.2's): `world.schema` (error: the grid of
//! [`WorldGrid::parse`], the package out of range, tiles past the tileset, pieces),
//! `world.tileset-mismatch` (warning: a header that shows the map names area data of another
//! package), `world.bake` (error: each refusal of the bake, with its code) and `world.budget`
//! (info, a warning over the vanilla VRAM: the baked sizes).
//!
//! Serde form errors of `world` stay `textproject.schema` (the import's and the export's). The
//! import skips a world map with a `world.schema` error ([`map_schema_problems`]), its problem in
//! the plan; the check shows each problem once.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use g4_formats::map::MAP_SIZE;

use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::store::Store;
use crate::text::{Problem, Severity};
use crate::textproject::arts::{Art, numbered, slash};
use crate::textproject::locate::position;

use super::bake::WorldBakeReport;
use super::doc::WorldJson;
use super::grid::{WorldError, WorldGrid, error_cell};
use super::keep::world_field;
use super::mesh::{MODEL_BUDGET, piece_problems};
use super::places::headers_of_map;
use super::tileset::{
    TileRole, Tileset, missing_package, package_file, read_package_file, read_tileset_file,
    textures_only, tile_outside,
};
use crate::bake::{BakeCode, Level, vanilla_vram_max};

/// A world document that breaks its grammar, its ranges or its tileset (error).
pub const WORLD_SCHEMA_CODE: &str = "world.schema";
/// A header that shows the map names area data of another package than the world (warning).
pub const WORLD_TILESET_MISMATCH_CODE: &str = "world.tileset-mismatch";
/// The sizes of a world's bake (info; a warning with `bake.vram-high`).
pub const WORLD_BUDGET_CODE: &str = "world.budget";
/// A refusal of a world's bake (error), the bake's code first in the sentence.
pub const WORLD_BAKE_CODE: &str = "world.bake";
/// The member of a world map changed outside the Studio (warning; the check part follows with
/// the build step `world`).
pub const WORLD_MEMBER_CHANGED_CODE: &str = "world.member-changed";

/// „Gebacken: Modell 21 740 Byte (35 % von 0xF000), Terrain 334 Byte (1 % von 0x9000), 3
/// Texturen, VRAM 21 656 Byte.“: the model's bytes and share of [`MODEL_BUDGET`], the BDHC's
/// payload and its share of `bdhc.max_bytes`, the textures the model binds and the area's
/// VRAM, thousands split by a space, shares rounded to a whole percent; with a `bake.vram-high`
/// warning „; VRAM über dem Vanilla-Höchstwert 110 640 Byte“ (`vram_max`) before the stop.
pub fn baked_budget_text(report: &WorldBakeReport, vram_max: usize) -> String {
    let share = |bytes: usize, of: usize| (bytes * 100 + of / 2) / of.max(1);
    let terrain_max = Project::limits().bdhc.max_bytes.value;
    let textures = match report.textures.len() {
        1 => "1 Textur".to_owned(),
        n => format!("{} Texturen", thousands(n)),
    };
    let high = report
        .messages
        .iter()
        .any(|m| m.code == BakeCode::VramHigh.as_str());
    let over = if high {
        format!(
            "; VRAM über dem Vanilla-Höchstwert {} Byte",
            thousands(vram_max)
        )
    } else {
        String::new()
    };
    format!(
        "Gebacken: Modell {} Byte ({} % von {MODEL_BUDGET:#X}), Terrain {} Byte ({} % von \
         {terrain_max:#X}), {textures}, VRAM {} Byte{over}.",
        thousands(report.model_bytes),
        share(report.model_bytes, MODEL_BUDGET),
        thousands(report.bdhc_payload),
        share(report.bdhc_payload, terrain_max),
        thousands(report.vram_bytes),
    )
}

/// `n` with a space between each three digits: `1 680`, `61 440`.
pub(crate) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(d);
    }
    out
}

/// Problems of every world map of the project (the `data/maps/*.json` with `source: "world"`, read with `world_field`):
/// schema (grid, tiles against the tileset file or `textures_only` of the package, package range, pieces), tileset
/// mismatch, and for a world without schema error its bake: each refusal (`world.bake`) and, when it bakes, the sizes
/// (`world.budget`). Headers and bake come from a Store on `Project::open(&project.root)`, read only (a world map whose
/// file the store refuses, derived keys say, is the import's error, not baked here). Without a base at hand it reports
/// a package missing on disk even when the import would restore it from the base.
pub fn world_problems(project: &Project) -> Result<Vec<Problem>, KernelError> {
    let dir = Art::Maps.dir();
    let mut problems = Vec::new();
    let mut tilesets = Tilesets::default();
    let mut store: Option<Store> = None;
    for n in numbered(&project.root.join(&dir), "json")? {
        let rel = dir.join(format!("{n:04}.json"));
        let Ok(map) = u16::try_from(n) else { continue };
        let Ok(bytes) = std::fs::read(project.root.join(&rel)) else {
            continue;
        };
        // A file that does not read, or whose `world` does not, is the import's
        // `textproject.schema`.
        let Ok(Some(world)) = world_field(&bytes) else {
            continue;
        };
        let at = Spot {
            rel: &rel,
            bytes: &bytes,
            world: &world,
        };
        let schema = schema_of(project, &at, &mut tilesets, &|_| false)?;
        let clean = schema.is_empty();
        problems.extend(schema);
        if store.is_none() {
            store = Some(Store::new(Arc::new(Project::open(&project.root)?)));
        }
        if let Some(store) = store.as_mut() {
            mismatch(store, map, &at, &mut problems);
            if clean {
                bake_problems(store, map, &at, &mut problems);
            }
        }
    }
    Ok(problems)
}

/// The bake of world map `map` as the store holds it: one `world.bake` error per refusal
/// („bake.model-too-big: Das gebackene Modell …“), and when it bakes the sizes
/// ([`baked_budget_text`], a warning over the vanilla VRAM). A world the store does not bake
/// (its file refused) gives none.
fn bake_problems(store: &mut Store, map: u16, at: &Spot<'_>, problems: &mut Vec<Problem>) {
    let Ok(Some(report)) = store.world_bake_report(map) else {
        return;
    };
    for m in report.messages.iter().filter(|m| m.level == Level::Error) {
        let sentence = format!("{}: {}", m.code, m.message);
        problems.push(at.problem(Severity::Error, WORLD_BAKE_CODE, "world", &sentence, None));
    }
    if report.ok {
        let vram_max = vanilla_vram_max(store.project().family);
        let high = report
            .messages
            .iter()
            .any(|m| m.code == BakeCode::VramHigh.as_str());
        let severity = if high {
            Severity::Warning
        } else {
            Severity::Info
        };
        let text = baked_budget_text(&report, vram_max);
        problems.push(at.problem(severity, WORLD_BUDGET_CODE, "world", &text, None));
    }
}

/// The `world.schema` errors of the map file `rel` (`bytes`) when it is a world map, for the
/// import: a map with one of them is not imported. Empty for a binary map and for a file whose
/// `world` does not read (that is `textproject.schema`, the import's own). A package past the
/// members on disk that `in_base` has is no error: the import restores its member from the
/// base (its tiles stay unchecked until it is back).
pub(crate) fn map_schema_problems(
    project: &Project,
    rel: &Path,
    bytes: &[u8],
    in_base: &dyn Fn(u16) -> bool,
) -> Result<Vec<Problem>, KernelError> {
    let Ok(Some(world)) = world_field(bytes) else {
        return Ok(Vec::new());
    };
    let at = Spot {
        rel,
        bytes,
        world: &world,
    };
    schema_of(project, &at, &mut Tilesets::default(), in_base)
}

/// The tileset of each package, read once per check: the tileset file, else the package's
/// textures; `None` when neither reads (a broken tileset file is the `tilesets` check's, a
/// package that does not read leaves the tiles unchecked).
#[derive(Default)]
struct Tilesets(BTreeMap<u16, Option<Arc<Tileset>>>);

impl Tilesets {
    fn get(
        &mut self,
        project: &Project,
        package: u16,
    ) -> Result<Option<Arc<Tileset>>, KernelError> {
        if let Some(found) = self.0.get(&package) {
            return Ok(found.clone());
        }
        let found = tileset_of(project, package)?.map(Arc::new);
        self.0.insert(package, found.clone());
        Ok(found)
    }
}

/// The tileset of `package` as [`Store::tileset`] answers it (the file, else
/// [`textures_only`] of the package), without a store; `None` instead of a refusal.
fn tileset_of(project: &Project, package: u16) -> Result<Option<Tileset>, KernelError> {
    match read_tileset_file(project, package) {
        Ok(Some(tileset)) => return Ok(Some(tileset)),
        Ok(None) => {}
        Err(KernelError::Refused(_)) => return Ok(None),
        Err(e) => return Err(e),
    }
    let path = match package_file(project, package) {
        Ok(path) => path,
        Err(KernelError::Refused(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    match read_package_file(&path, package) {
        Ok(btx) => Ok(Some(textures_only(package, &btx))),
        Err(KernelError::Refused(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// One world map file: its path, bytes and world.
struct Spot<'a> {
    rel: &'a Path,
    bytes: &'a [u8],
    world: &'a WorldJson,
}

impl Spot<'_> {
    /// `<code>: <path>: <sentence>` at `path` in the file; with `cell` at that token of the
    /// row `path` names.
    fn problem(
        &self,
        severity: Severity,
        code: &str,
        path: &str,
        sentence: &str,
        cell: Option<usize>,
    ) -> Problem {
        let (line, col) = position(self.bytes, path).unwrap_or((1, 1));
        let col = match cell.and_then(|x| token_offset(row_of(self.world, path)?, x)) {
            // After the opening quote of the row.
            Some(offset) => col + 1 + u32::try_from(offset).unwrap_or(0),
            None => col,
        };
        Problem {
            severity,
            file: slash(self.rel),
            line,
            col,
            message: format!("{code}: {path}: {sentence}"),
        }
    }

    fn error(&self, e: &WorldError) -> Problem {
        self.problem(
            Severity::Error,
            WORLD_SCHEMA_CODE,
            &e.path,
            &e.message,
            error_cell(e),
        )
    }
}

/// The `world.schema` errors of `at`.
fn schema_of(
    project: &Project,
    at: &Spot<'_>,
    tilesets: &mut Tilesets,
    in_base: &dyn Fn(u16) -> bool,
) -> Result<Vec<Problem>, KernelError> {
    let grid = match WorldGrid::parse(at.world) {
        Ok(grid) => grid,
        Err(errors) => return Ok(errors.iter().map(|e| at.error(e)).collect()),
    };
    let package = grid.tileset;
    if project.has_archive(AssetKind::MapTexture) {
        let count = project.member_count(AssetKind::MapTexture)?;
        if usize::from(package) >= count && !in_base(package) {
            let text = match missing_package(package, count) {
                KernelError::Refused(text) => text,
                other => other.to_string(),
            };
            let e = WorldError {
                path: "world.tileset".to_owned(),
                message: text,
            };
            return Ok(vec![at.error(&e)]);
        }
    }
    let Some(tileset) = tilesets.get(project, package)? else {
        return Ok(Vec::new());
    };
    Ok(tile_errors(&grid, &tileset)
        .iter()
        .map(|e| at.error(e))
        .collect())
}

/// The tiles of `grid` past `tileset` (the first per row, like a token error), the pieces whose
/// tile is past it, no piece or has a footprint without area, and the pieces over the edge
/// ([`piece_problems`]).
pub(crate) fn tile_errors(grid: &WorldGrid, tileset: &Tileset) -> Vec<WorldError> {
    let package = grid.tileset;
    let count = tileset.tiles.len();
    let outside = |path: &str, tile: u16| {
        let text = tile_outside(path, tile, package, count);
        text.strip_prefix(&format!("{path}: "))
            .unwrap_or(&text)
            .to_owned()
    };
    let mut errors = Vec::new();
    for (i, layer) in grid.layers.iter().enumerate() {
        for (z, row) in layer.tiles.chunks(MAP_SIZE).enumerate() {
            let past = row
                .iter()
                .enumerate()
                .find_map(|(x, t)| t.filter(|t| usize::from(t.tile) >= count).map(|t| (x, t)));
            if let Some((x, t)) = past {
                let path = format!("world.layers[{i}].tiles[{z}]");
                let text = outside(&path, t.tile);
                errors.push(WorldError {
                    path,
                    message: format!("Zelle {x}: {text}"),
                });
            }
        }
    }
    for (i, piece) in grid.pieces.iter().enumerate() {
        let path = format!("world.pieces[{i}].tile");
        let message = match tileset.tile(piece.tile) {
            None => outside(&path, piece.tile),
            Some(t) if t.role != TileRole::Piece => format!(
                "Kachel {:#06x} ist kein Teilmodell (Rolle `{}`).",
                piece.tile,
                t.role.key()
            ),
            Some(t) => match t.footprint {
                Some([w, d]) if w == 0 || d == 0 => format!(
                    "Kachel {:#06x} hat einen Fußabdruck ohne Fläche ({w} × {d}).",
                    piece.tile
                ),
                _ => continue,
            },
        };
        errors.push(WorldError { path, message });
    }
    errors.extend(piece_problems(grid, tileset));
    errors
}

/// One warning per header that shows `map` and names area data of another package than the
/// world's, at `world.tileset`. Headers or area data that do not read give none (their own
/// checks report them).
fn mismatch(store: &mut Store, map: u16, at: &Spot<'_>, problems: &mut Vec<Problem>) {
    let Ok(headers) = headers_of_map(store, map) else {
        return;
    };
    let tileset = at.world.tileset;
    for (header, pack) in headers {
        if pack != tileset {
            problems.push(at.problem(
                Severity::Warning,
                WORLD_TILESET_MISMATCH_CODE,
                "world.tileset",
                &format!(
                    "Karte {map:04} nimmt Tileset {tileset:04}, ihr Header {header:04} nennt \
                     Gebietsdaten mit Paket {pack:04}."
                ),
                None,
            ));
        }
    }
}

/// The row string `path` names in `world` (`world.layers[0].tiles[4]`,
/// `world.permissions.override[3]`), `None` for any other path.
fn row_of<'w>(world: &'w WorldJson, path: &str) -> Option<&'w str> {
    let rest = path.strip_prefix("world.")?;
    let (lines, index) = match rest.strip_prefix("permissions.override") {
        Some(index) => (world.permissions.overrides.as_ref()?, index),
        None => {
            let (i, rest) = rest.strip_prefix("layers[")?.split_once("].")?;
            let layer = world.layers.get(i.parse::<usize>().ok()?)?;
            let (key, _) = rest.split_once('[')?;
            let lines = match key {
                "tiles" => &layer.tiles,
                "heights" => layer.heights.as_ref()?,
                "ramps" => layer.ramps.as_ref()?,
                _ => return None,
            };
            (lines, &rest[key.len()..])
        }
    };
    let z: usize = index.strip_prefix('[')?.strip_suffix(']')?.parse().ok()?;
    lines.get(z).map(String::as_str)
}

/// The byte offset of token `x` (counted from 0, split at whitespace) in `row`.
fn token_offset(row: &str, x: usize) -> Option<usize> {
    let mut seen = 0;
    let mut in_token = false;
    for (i, c) in row.char_indices() {
        if c.is_whitespace() {
            in_token = false;
        } else if !in_token {
            if seen == x {
                return Some(i);
            }
            seen += 1;
            in_token = true;
        }
    }
    None
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::test_support::TextFixture;
    use crate::textproject::canon::to_canonical;
    use crate::world::doc::{PieceJson, fixtures};
    use crate::world::grid::set_token;
    use crate::world::keep::world_parts;
    use crate::world::tileset::tileset_rel;

    const MAP: &str = "data/maps/0000.json";

    /// A text project with the synthetic tileset as the file of package 0.
    fn fixture() -> TextFixture {
        let fx = TextFixture::exported();
        let rel = tileset_rel(0);
        fx.write(rel.to_str().unwrap(), &to_canonical(&Tileset::synthetic()));
        fx
    }

    /// Map 0 as a world map with `world`.
    fn put_world(fx: &TextFixture, world: &WorldJson) {
        let bytes = world_parts(fx.read(MAP), world, &mut Vec::new()).unwrap();
        fx.write(MAP, &bytes);
    }

    fn with_code<'p>(problems: &'p [Problem], code: &str) -> Vec<&'p Problem> {
        problems
            .iter()
            .filter(|p| p.message.starts_with(&format!("{code}: ")))
            .collect()
    }

    fn problems_of(fx: &TextFixture) -> Vec<Problem> {
        world_problems(&fx.project()).unwrap()
    }

    /// Line and column (1-based, ASCII) of the first `needle` in file `rel`.
    fn spot(fx: &TextFixture, rel: &str, needle: &str) -> (u32, u32) {
        let text = fx.text(rel);
        let (z, line) = text
            .lines()
            .enumerate()
            .find(|(_, l)| l.contains(needle))
            .unwrap();
        let col = line.find(needle).unwrap();
        (
            u32::try_from(z + 1).unwrap(),
            u32::try_from(col + 1).unwrap(),
        )
    }

    #[test]
    fn schema_problems_name_path_and_line() {
        let fx = fixture();
        let mut w = fixtures::one_tile_world();
        w.layers[0].tiles[1] = set_token(&w.layers[0].tiles[1], 3, "00g2").unwrap();
        put_world(&fx, &w);
        let problems = problems_of(&fx);
        let schema = with_code(&problems, WORLD_SCHEMA_CODE);
        assert_eq!(schema.len(), 1, "{problems:#?}");
        let p = schema[0];
        assert_eq!(p.severity, Severity::Error);
        assert_eq!(p.file, Path::new(MAP));
        assert_eq!(
            p.message,
            "world.schema: world.layers[0].tiles[1]: Zelle 3: `00g2` ist keine Kachel (`.` oder \
             vier Hex-Ziffern, dahinter `r1` bis `r3` und `f`)."
        );
        let (line, col) = spot(&fx, MAP, "00g2");
        assert_eq!((p.line, p.col), (line, col));
        assert_eq!(line, 12, "{}", fx.text(MAP));
        // A broken world has no budget.
        assert!(with_code(&problems, WORLD_BUDGET_CODE).is_empty());

        // A ramp that ends above 62, at its token.
        let mut w = fixtures::one_tile_world();
        let ground = &mut w.layers[0];
        ground.heights.as_mut().unwrap()[2] = vec!["61"; 32].join(" ");
        ground.ramps = Some(vec![vec!["."; 32].join(" "); 32]);
        ground.ramps.as_mut().unwrap()[2] = set_token(&vec!["."; 32].join(" "), 5, "e2").unwrap();
        put_world(&fx, &w);
        let problems = problems_of(&fx);
        let schema = with_code(&problems, WORLD_SCHEMA_CODE);
        assert_eq!(schema.len(), 1, "{problems:#?}");
        assert_eq!(
            schema[0].message,
            "world.schema: world.layers[0].ramps[2]: Zelle 5: Die Rampe endet über 62 (Höhe 61, \
             2 Stufen)."
        );
        assert_eq!((schema[0].line, schema[0].col), spot(&fx, MAP, "e2"));
    }

    #[test]
    fn tiles_past_the_tileset_are_schema_errors() {
        let fx = fixture();
        let mut w = fixtures::one_tile_world();
        // Six tiles: 0006 is past them, twice in row 7 (the first is named), once in row 9.
        w.layers[0].tiles[7] = set_token(
            &set_token(&w.layers[0].tiles[7], 2, "0006r1").unwrap(),
            9,
            "0007",
        )
        .unwrap();
        w.layers[0].tiles[9] = set_token(&w.layers[0].tiles[9], 0, "0120").unwrap();
        w.pieces = vec![
            PieceJson {
                tile: 9,
                at: [0, 0],
                rotate: 0,
            },
            PieceJson {
                tile: 1,
                at: [4, 4],
                rotate: 0,
            },
            PieceJson {
                tile: 5,
                at: [31, 10],
                rotate: 0,
            },
            PieceJson {
                tile: 5,
                at: [8, 8],
                rotate: 90,
            },
        ];
        put_world(&fx, &w);
        let problems = problems_of(&fx);
        let schema: Vec<&str> = with_code(&problems, WORLD_SCHEMA_CODE)
            .iter()
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(
            schema,
            [
                "world.schema: world.layers[0].tiles[7]: Zelle 2: Kachel 0x0006 gibt es im \
                 Tileset 0000 nicht (6 Kacheln).",
                "world.schema: world.layers[0].tiles[9]: Zelle 0: Kachel 0x0120 gibt es im \
                 Tileset 0000 nicht (6 Kacheln).",
                "world.schema: world.pieces[0].tile: Kachel 0x0009 gibt es im Tileset 0000 nicht \
                 (6 Kacheln).",
                "world.schema: world.pieces[1].tile: Kachel 0x0001 ist kein Teilmodell (Rolle \
                 `grass`).",
                "world.schema: world.pieces[2]: Das Teilmodell ragt über den Kartenrand (x 31, \
                 Breite 2).",
            ],
            "{problems:#?}"
        );
        let first = with_code(&problems, WORLD_SCHEMA_CODE)[0];
        assert_eq!((first.line, first.col), spot(&fx, MAP, "0006r1"));

        // A piece tile whose footprint has no area.
        let mut tileset = Tileset::synthetic();
        tileset.tiles[5].footprint = Some([0, 2]);
        fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&tileset));
        let mut w = fixtures::one_tile_world();
        w.pieces = vec![PieceJson {
            tile: 5,
            at: [1, 1],
            rotate: 0,
        }];
        put_world(&fx, &w);
        let problems = problems_of(&fx);
        let schema: Vec<&str> = with_code(&problems, WORLD_SCHEMA_CODE)
            .iter()
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(
            schema,
            [
                "world.schema: world.pieces[0].tile: Kachel 0x0005 hat einen Fußabdruck ohne Fläche \
              (0 × 2)."
            ]
        );

        // A package past the archive (one package).
        let mut w = fixtures::one_tile_world();
        w.tileset = 7;
        put_world(&fx, &w);
        let problems = problems_of(&fx);
        let schema: Vec<&str> = with_code(&problems, WORLD_SCHEMA_CODE)
            .iter()
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(
            schema,
            ["world.schema: world.tileset: Kartentextur-Paket 0007 gibt es nicht (1 Paket)."]
        );
    }

    #[test]
    fn a_header_with_another_package_warns() {
        let fx = fixture();
        put_world(&fx, &fixtures::one_tile_world());
        let area = Art::AreaData.dir().join("0001.json");
        let area = area.to_str().unwrap();
        // Area data 1 on package 0, like the world: no warning.
        fx.edit(area, "\"map_tileset\": 2", "\"map_tileset\": 0");
        let problems = problems_of(&fx);
        assert!(
            with_code(&problems, WORLD_TILESET_MISMATCH_CODE).is_empty(),
            "{problems:#?}"
        );

        // On package 1: header 1 (on matrix 0, which shows map 0) warns once.
        fx.edit(area, "\"map_tileset\": 0", "\"map_tileset\": 1");
        let problems = problems_of(&fx);
        let warnings = with_code(&problems, WORLD_TILESET_MISMATCH_CODE);
        assert_eq!(warnings.len(), 1, "{problems:#?}");
        let w = warnings[0];
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(w.file, Path::new(MAP));
        assert_eq!(
            w.message,
            "world.tileset-mismatch: world.tileset: Karte 0000 nimmt Tileset 0000, ihr Header \
             0001 nennt Gebietsdaten mit Paket 0001."
        );
        assert_eq!((w.line, w.col), spot(&fx, MAP, "\"tileset\""));
    }

    #[test]
    fn not_baked_is_gone() {
        let fx = crate::test_support::world_ready();
        put_world(&fx, &fixtures::one_tile_world());
        let problems = problems_of(&fx);
        assert!(
            problems
                .iter()
                .all(|p| !p.message.starts_with("world.not-baked")),
            "{problems:#?}"
        );
        assert!(
            with_code(&problems, WORLD_BAKE_CODE).is_empty(),
            "{problems:#?}"
        );
        // The binary map 0001 has none.
        assert!(
            problems
                .iter()
                .all(|p| p.file != Path::new("data/maps/0001.json")),
            "{problems:#?}"
        );
    }

    fn sample_report() -> WorldBakeReport {
        WorldBakeReport {
            map: 0,
            ok: true,
            style: None,
            faces: 3,
            model_bytes: 21_740,
            model_share: 0.35,
            model_kept: false,
            textures: vec!["a".into(), "b".into(), "c".into()],
            vram_bytes: 21_656,
            bdhc_bytes: 350,
            bdhc_payload: 334,
            plates: 1,
            blocked: 0,
            verbatim_model: false,
            verbatim_terrain: false,
            messages: Vec::new(),
        }
    }

    #[test]
    fn budget_is_the_baked_size() {
        let mut r = sample_report();
        assert_eq!(
            baked_budget_text(&r, 110_640),
            "Gebacken: Modell 21 740 Byte (35 % von 0xF000), Terrain 334 Byte (1 % von 0x9000), \
             3 Texturen, VRAM 21 656 Byte."
        );
        r.textures.truncate(1);
        r.vram_bytes = 120_000;
        r.messages.push(crate::world::WorldBakeMessage {
            code: "bake.vram-high".into(),
            level: Level::Warning,
            args: vec!["120000".into(), "110640".into()],
            message: String::new(),
        });
        assert_eq!(
            baked_budget_text(&r, 110_640),
            "Gebacken: Modell 21 740 Byte (35 % von 0xF000), Terrain 334 Byte (1 % von 0x9000), \
             1 Textur, VRAM 120 000 Byte; VRAM über dem Vanilla-Höchstwert 110 640 Byte."
        );

        // In the check: the bake of the store, an info at `world`.
        let fx = crate::test_support::world_ready();
        put_world(&fx, &fixtures::one_tile_world());
        let problems = problems_of(&fx);
        let budget = with_code(&problems, WORLD_BUDGET_CODE);
        assert_eq!(budget.len(), 1, "{problems:#?}");
        assert_eq!(budget[0].severity, Severity::Info);
        let mut store = Store::new(Arc::new(fx.project()));
        let report = store.world_bake_report(0).unwrap().unwrap();
        assert!(report.ok, "{:?}", report.messages);
        assert_eq!(
            budget[0].message,
            format!(
                "world.budget: world: {}",
                baked_budget_text(&report, 110_640)
            )
        );
        assert!(
            budget[0].message.contains(", 1 Textur, "),
            "{}",
            budget[0].message
        );
        assert_eq!(
            (budget[0].line, budget[0].col),
            spot(&fx, MAP, "\"world\":")
        );
    }

    #[test]
    fn bake_problems_name_the_code() {
        let fx = crate::test_support::world_ready();
        put_world(&fx, &fixtures::oversize_world());
        let problems = problems_of(&fx);
        let bake = with_code(&problems, WORLD_BAKE_CODE);
        assert_eq!(bake.len(), 1, "{problems:#?}");
        assert_eq!(bake[0].severity, Severity::Error);
        assert_eq!(bake[0].file, Path::new(MAP));
        let mut store = Store::new(Arc::new(fx.project()));
        let report = store.world_bake_report(0).unwrap().unwrap();
        let bytes = thousands(report.messages[0].args[0].parse().unwrap());
        assert_eq!(
            bake[0].message,
            format!(
                "world.bake: world: bake.model-too-big: Das gebackene Modell hat {bytes} Byte, \
                 das Spiel fasst höchstens 61 440 Byte; die Karte behält ihr letztes Modell."
            )
        );
        // A refused bake has no budget.
        assert!(
            with_code(&problems, WORLD_BUDGET_CODE).is_empty(),
            "{problems:#?}"
        );

        // A package that does not read: the texture is missing, in German.
        let fx = fixture();
        put_world(&fx, &fixtures::one_tile_world());
        let problems = problems_of(&fx);
        let bake = with_code(&problems, WORLD_BAKE_CODE);
        assert_eq!(
            bake.iter().map(|p| p.message.as_str()).collect::<Vec<_>>(),
            [
                "world.bake: world: bake.texture-missing: Die Textur „syn_boden“ fehlt im \
                 Kartentextur-Paket oder hat dort keine Palette."
            ]
        );
    }
}
