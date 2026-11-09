//! The grid of a world document: the token grammar of its rows (tile, height, ramp,
//! permission override), the 1024 cells of each layer in memory ([`WorldGrid`], index
//! `z · 32 + x`) and the schema check that reads them ([`WorldGrid::parse`], every error with
//! its path). The grid knows no tileset: tile indices against the package are checked by the
//! text project check, the footprint of a piece against the edge by the mesher.
//!
//! Tokens: a tile is `.` or four hex digits (`0012`), optionally `r1` to `r3` (quarter turns:
//! seen from above with north up, the picture turns clockwise; its texture coordinates turn
//! counter-clockwise, see `mesh::uv_of`), then optionally `f` (mirrored at the z axis after the
//! turn); read in any case, written lowercase. A height is a whole number from −16 to 62
//! (steps of [`STEP_UNITS`](super::doc::STEP_UNITS)). A ramp is `.` or a direction `n`, `e`,
//! `s`, `w` (uphill) with 1 or 2 steps. An override is `.` or `kkcc` like the permissions.
//!
//! The height and ramp cells are lexed by [`lex_height`] and [`lex_ramp`] (`terrain`'s, which
//! 6.2's rows of `set_map_terrain` read too): one grammar, `+5` and `e01` included;
//! written are the canonical forms (`5`, `e1`). The ramp is 6.2's `bdhc::Ramp` in the same sense
//! (uphill, height at the lower edge); [`Ramp`] converts to and from it without loss.

use std::fmt;

use g4_formats::bdhc::{self, HeightGrid};
use g4_formats::map::{Cell, MAP_SIZE};

use super::doc::{HEIGHT_MAX, HEIGHT_MIN, LayerRole, PieceJson, WORLD_VERSION, WorldJson};
use super::freeform::side_parts;
use crate::terrain::EMPTY;
pub use crate::terrain::{lex_height, lex_ramp};

/// Cells of one layer.
const CELLS: usize = MAP_SIZE * MAP_SIZE;
/// The turns a piece may have, in degrees.
const PIECE_TURNS: [u16; 4] = [0, 90, 180, 270];

fn ground_first() -> String {
    t!("world.ground_first").into_owned()
}
fn ground_only() -> String {
    t!("world.ground_only").into_owned()
}
/// The rule for `glb`; the parser knows no map, [`glb_map_errors`] says the map's number.
fn glb_rule(map: &str) -> String {
    t!("world.glb_rule", map = map).into_owned()
}
/// Where a free-form model may lie, per axis in world units.
const FREEFORM_AT: std::ops::RangeInclusive<i32> = -512..=511;

/// A tile of a cell: index `tile` in the tileset, `rotate` quarter turns (0 to 3), `flip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRef {
    pub tile: u16,
    pub rotate: u8,
    pub flip: bool,
}

/// The ramp of a ground cell with its steps (1 or 2). The direction is uphill: the cell lies at
/// `h` on the opposite edge and at `h + steps` on the named one (`East(1)` at `h = 0`: west
/// edge 0, east edge 8 units).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ramp {
    #[default]
    None,
    North(u8),
    East(u8),
    South(u8),
    West(u8),
}

impl From<bdhc::Ramp> for Ramp {
    fn from(r: bdhc::Ramp) -> Ramp {
        match r {
            bdhc::Ramp::None => Ramp::None,
            bdhc::Ramp::North(s) => Ramp::North(s),
            bdhc::Ramp::East(s) => Ramp::East(s),
            bdhc::Ramp::South(s) => Ramp::South(s),
            bdhc::Ramp::West(s) => Ramp::West(s),
        }
    }
}

impl From<Ramp> for bdhc::Ramp {
    fn from(r: Ramp) -> bdhc::Ramp {
        match r {
            Ramp::None => bdhc::Ramp::None,
            Ramp::North(s) => bdhc::Ramp::North(s),
            Ramp::East(s) => bdhc::Ramp::East(s),
            Ramp::South(s) => bdhc::Ramp::South(s),
            Ramp::West(s) => bdhc::Ramp::West(s),
        }
    }
}

/// An error of a world document at `path` (`world.layers[0].tiles[4]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// One layer of the grid: its 1024 tiles (`None` for `.`), index `z · 32 + x`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridLayer {
    pub name: String,
    pub role: LayerRole,
    pub tiles: Vec<Option<TileRef>>,
}

/// A world document read into cells: the layers, the 1024 heights and ramps of the ground
/// layer, the pieces, and the permissions (`derive`, the 1024 overrides, `None` for `.`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldGrid {
    pub tileset: u16,
    pub layers: Vec<GridLayer>,
    pub heights: Vec<i8>,
    pub ramps: Vec<Ramp>,
    pub pieces: Vec<PieceJson>,
    pub derive: bool,
    pub overrides: Vec<Option<Cell>>,
}

/// Collects the errors of one document, each at its path.
struct Errors(Vec<WorldError>);

impl Errors {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(WorldError {
            path: path.into(),
            message: message.into(),
        });
    }

    /// The 1024 cells of `lines` at `path` (32 rows of 32 tokens), or `None` after noting
    /// what is wrong: the row count, or per row its cell count or its first bad token.
    fn rows<T>(
        &mut self,
        path: &str,
        lines: &[String],
        parse: impl Fn(&str) -> Result<T, String>,
    ) -> Option<Vec<T>> {
        let mut ok = true;
        if lines.len() != MAP_SIZE {
            self.push(
                path,
                t!("world.rows_count", count = lines.len(), expected = MAP_SIZE),
            );
            ok = false;
        }
        let mut cells = Vec::with_capacity(CELLS);
        for (z, line) in lines.iter().enumerate() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != MAP_SIZE {
                self.push(
                    format!("{path}[{z}]"),
                    t!(
                        "world.cells_count",
                        count = tokens.len(),
                        expected = MAP_SIZE
                    ),
                );
                ok = false;
                continue;
            }
            for (x, token) in tokens.into_iter().enumerate() {
                match parse(token) {
                    Ok(cell) => cells.push(cell),
                    Err(e) => {
                        self.push(
                            format!("{path}[{z}]"),
                            t!("world.cell_prefixed", x = x, text = e),
                        );
                        ok = false;
                        break;
                    }
                }
            }
        }
        ok.then_some(cells)
    }
}

impl WorldGrid {
    /// The grid of `world`, or every error it has (version, layers, rows and tokens, ranges,
    /// pieces, free-form models, overrides), each with its path, in document order.
    pub fn parse(world: &WorldJson) -> Result<WorldGrid, Vec<WorldError>> {
        let mut errors = Errors(Vec::new());
        if world.version != WORLD_VERSION {
            errors.push(
                "world.version",
                t!(
                    "world.version_unknown",
                    version = world.version,
                    known = WORLD_VERSION
                ),
            );
        }
        if world.layers.is_empty() {
            errors.push("world.layers", ground_first());
        }
        let mut layers = Vec::with_capacity(world.layers.len());
        let mut heights = None;
        let mut ramps = None;
        for (i, layer) in world.layers.iter().enumerate() {
            let at = format!("world.layers[{i}]");
            let ground = layer.role == LayerRole::Ground;
            if ground != (i == 0) {
                errors.push(format!("{at}.role"), ground_first());
            }
            let tiles = errors.rows(&format!("{at}.tiles"), &layer.tiles, parse_tile);
            if let Some(tiles) = tiles {
                layers.push(GridLayer {
                    name: layer.name.clone(),
                    role: layer.role,
                    tiles,
                });
            }
            if i == 0 && ground {
                match &layer.heights {
                    Some(lines) => {
                        heights = errors.rows(&format!("{at}.heights"), lines, parse_height);
                    }
                    None => errors.push(format!("{at}.heights"), t!("world.ground_needs_heights")),
                }
                ramps = match &layer.ramps {
                    Some(lines) => errors.rows(&format!("{at}.ramps"), lines, parse_ramp),
                    None => Some(vec![Ramp::None; CELLS]),
                };
            } else if !ground {
                if layer.heights.is_some() {
                    errors.push(format!("{at}.heights"), ground_only());
                }
                if layer.ramps.is_some() {
                    errors.push(format!("{at}.ramps"), ground_only());
                }
            }
        }
        if let (Some(heights), Some(ramps)) = (&heights, &ramps) {
            ramp_tops(heights, ramps, &mut errors);
        }
        let overrides = permissions(world, &mut errors);
        for (i, piece) in world.pieces.iter().enumerate() {
            if !PIECE_TURNS.contains(&piece.rotate) {
                errors.push(
                    format!("world.pieces[{i}].rotate"),
                    t!("world.piece_turn_invalid", rotate = piece.rotate),
                );
            }
            let [x, z] = piece.at;
            if usize::from(x) >= MAP_SIZE || usize::from(z) >= MAP_SIZE {
                errors.push(
                    format!("world.pieces[{i}].at"),
                    t!("world.piece_off_map", x = x, z = z, max = MAP_SIZE - 1),
                );
            }
        }
        let mut verbatim = 0;
        for (i, model) in world.freeform.iter().enumerate() {
            if model.glb.is_some() == model.nsbmd.is_some() {
                errors.push(
                    format!("world.freeform[{i}]"),
                    t!("world.freeform_glb_or_nsbmd"),
                );
            }
            if let Some(glb) = &model.glb
                && side_parts(glb).is_none()
            {
                errors.push(format!("world.freeform[{i}].glb"), glb_rule("NNNN"));
            }
            if model.nsbmd.is_none() && model.at.iter().any(|v| !FREEFORM_AT.contains(v)) {
                errors.push(
                    format!("world.freeform[{i}].at"),
                    t!("world.freeform_at_range"),
                );
            }
            if model.nsbmd.is_some() {
                verbatim += 1;
                if verbatim > 1 {
                    errors.push(
                        format!("world.freeform[{i}]"),
                        t!("world.verbatim_at_most_one"),
                    );
                }
                if model.at != [0, 0, 0] {
                    errors.push(
                        format!("world.freeform[{i}].at"),
                        t!("world.verbatim_at_origin"),
                    );
                }
            }
        }
        match (errors.0.is_empty(), heights, ramps, overrides) {
            (true, Some(heights), Some(ramps), Some(overrides)) => Ok(WorldGrid {
                tileset: world.tileset,
                layers,
                heights,
                ramps,
                pieces: world.pieces.clone(),
                derive: world.permissions.derive,
                overrides,
            }),
            // Every way to `None` above notes an error.
            _ => Err(errors.0),
        }
    }
}

/// The free-form models of `world` whose `glb` has the form of a side file but names another map
/// than `map` (the form itself is [`WorldGrid::parse`]'s), each with its path.
pub(crate) fn glb_map_errors(world: &WorldJson, map: u16) -> Vec<WorldError> {
    world
        .freeform
        .iter()
        .enumerate()
        .filter(|(_, model)| {
            model
                .glb
                .as_deref()
                .and_then(side_parts)
                .is_some_and(|(number, _)| number != map)
        })
        .map(|(i, _)| WorldError {
            path: format!("world.freeform[{i}].glb"),
            message: glb_rule(&format!("{map:04}")),
        })
        .collect()
}

impl WorldGrid {
    /// The heights and ramps of the ground layer as 6.2's height grid (steps of 8 units, a
    /// ramp's height at its lower edge), e.g. for the walk height at a cell centre
    /// ([`HeightGrid::center_units`]).
    pub fn height_grid(&self) -> HeightGrid {
        let mut grid = HeightGrid::flat(0);
        for (c, (&h, &r)) in self.heights.iter().zip(&self.ramps).enumerate().take(CELLS) {
            grid.height[c] = h;
            grid.ramp[c] = r.into();
        }
        grid
    }
}

/// Notes every row of the ground's ramps with a cell whose ramp ends above [`HEIGHT_MAX`]
/// (height `h` and `s` steps need `h + s ≤ 62`), its first such cell like a token error.
fn ramp_tops(heights: &[i8], ramps: &[Ramp], errors: &mut Errors) {
    for z in 0..MAP_SIZE {
        let top = (0..MAP_SIZE).find_map(|x| {
            let c = z * MAP_SIZE + x;
            let steps = ramp_steps(ramps[c]);
            (steps > 0 && i16::from(heights[c]) + i16::from(steps) > i16::from(HEIGHT_MAX))
                .then_some((x, heights[c], steps))
        });
        if let Some((x, h, steps)) = top {
            let steps = if steps == 1 {
                t!("world.steps_one").into_owned()
            } else {
                t!("world.steps_other", n = steps).into_owned()
            };
            let text = t!(
                "world.ramp_too_high",
                max = HEIGHT_MAX,
                height = h,
                steps = steps
            );
            errors.push(
                format!("world.layers[0].ramps[{z}]"),
                t!("world.cell_prefixed", x = x, text = text),
            );
        }
    }
}

/// The steps of `ramp` (0 without one).
fn ramp_steps(ramp: Ramp) -> u8 {
    match ramp {
        Ramp::None => 0,
        Ramp::North(s) | Ramp::East(s) | Ramp::South(s) | Ramp::West(s) => s,
    }
}

/// The cell of a row error of [`WorldGrid::parse`] (its message is `world.cell_prefixed` in the
/// current locale: „Cell {x}: …“), `None` for an error of the whole row or of no row.
pub fn error_cell(e: &WorldError) -> Option<usize> {
    let template = t!("world.cell_prefixed");
    let (before, after) = template.split_once("%{x}")?;
    let (separator, _) = after.split_once("%{text}")?;
    let rest = e.message.strip_prefix(before)?;
    let (x, _) = rest.split_once(separator)?;
    x.parse().ok().filter(|&x| x < MAP_SIZE)
}

/// The 1024 overrides of `world` (all `None` without `override`); without `derive` every cell
/// must have one.
fn permissions(world: &WorldJson, errors: &mut Errors) -> Option<Vec<Option<Cell>>> {
    const PATH: &str = "world.permissions.override";
    let p = &world.permissions;
    let Some(lines) = &p.overrides else {
        if !p.derive {
            errors.push(PATH, t!("world.override_needed"));
            return None;
        }
        return Some(vec![None; CELLS]);
    };
    // The cells of the good rows count for the check without `derive`, even when another row
    // is broken, so both errors show at once.
    let mut open = 0;
    for line in lines {
        let cells: Vec<_> = line.split_whitespace().map(parse_override).collect();
        if cells.len() == MAP_SIZE && cells.iter().all(Result::is_ok) {
            open += cells.iter().filter(|c| matches!(c, Ok(None))).count();
        }
    }
    let cells = errors.rows(PATH, lines, parse_override);
    if !p.derive && open > 0 {
        let text = if open == 1 {
            t!("world.override_open_one")
        } else {
            t!("world.override_open_other", open = open)
        };
        errors.push(PATH, text);
        return None;
    }
    cells
}

/// `token` as exactly four hex digits.
fn hex4(token: &str) -> Option<u16> {
    (token.len() == 4 && token.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u16::from_str_radix(token, 16).ok())
        .flatten()
}

/// The tile of a token: `.` is `None`, else `hhhh[r1|r2|r3][f]` in any case.
pub fn parse_tile(token: &str) -> Result<Option<TileRef>, String> {
    if token == EMPTY {
        return Ok(None);
    }
    let bad = || t!("world.token_not_tile", token = token).into_owned();
    let lower = token.to_ascii_lowercase();
    let (digits, rest) = lower.split_at_checked(4).ok_or_else(bad)?;
    let tile = hex4(digits).ok_or_else(bad)?;
    let (rest, flip) = match rest.strip_suffix('f') {
        Some(rest) => (rest, true),
        None => (rest, false),
    };
    let rotate = match rest {
        "" => 0,
        "r1" => 1,
        "r2" => 2,
        "r3" => 3,
        _ => return Err(bad()),
    };
    Ok(Some(TileRef { tile, rotate, flip }))
}

/// The token of a tile, lowercase: `.`, `0012`, `0012r3f`.
pub fn tile_token(t: Option<TileRef>) -> String {
    let Some(t) = t else {
        return EMPTY.to_owned();
    };
    let mut token = format!("{:04x}", t.tile);
    if t.rotate != 0 {
        token.push_str(&format!("r{}", t.rotate));
    }
    if t.flip {
        token.push('f');
    }
    token
}

/// The ramp of a token ([`lex_ramp`]): `.` or `n`, `e`, `s`, `w` with 1 or 2 steps.
pub fn parse_ramp(token: &str) -> Result<Ramp, String> {
    let bad = || t!("world.token_not_ramp", token = token).into_owned();
    let ramp = Ramp::from(lex_ramp(token).ok_or_else(bad)?);
    if ramp == Ramp::None || (1..=2).contains(&ramp_steps(ramp)) {
        Ok(ramp)
    } else {
        Err(bad())
    }
}

/// The token of a ramp: `.`, `n1`, `w2` (`terrain::ramp_token`, one grammar for both row
/// formats).
pub fn ramp_token(r: Ramp) -> String {
    crate::terrain::ramp_token(r.into())
}

/// The height step of a token ([`lex_height`]): a whole number from −16 to 62.
pub fn parse_height(token: &str) -> Result<i8, String> {
    let value = lex_height(token)
        .ok_or_else(|| t!("world.token_not_height", token = token).into_owned())?;
    i8::try_from(value)
        .ok()
        .filter(|h| (HEIGHT_MIN..=HEIGHT_MAX).contains(h))
        .ok_or_else(|| t!("world.token_height_range", token = token).into_owned())
}

/// The permission override of a token: `.` is `None`, else `kkcc` (hex of kind and
/// collision) in any case.
pub fn parse_override(token: &str) -> Result<Option<Cell>, String> {
    if token == EMPTY {
        return Ok(None);
    }
    let value =
        hex4(token).ok_or_else(|| t!("world.token_not_permission", token = token).into_owned())?;
    let [kind, collision] = value.to_be_bytes();
    Ok(Some(Cell { kind, collision }))
}

/// The token of a permission override, lowercase: `.` or `kkcc`.
pub fn override_token(c: Option<Cell>) -> String {
    match c {
        None => EMPTY.to_owned(),
        Some(c) => format!("{:02x}{:02x}", c.kind, c.collision),
    }
}

/// Replaces cell `x` of row `row` (32 tokens, one space), keeping the other tokens as they are.
pub fn set_token(row: &str, x: usize, token: &str) -> Result<String, String> {
    let mut tokens: Vec<&str> = row.split_whitespace().collect();
    if tokens.len() != MAP_SIZE {
        return Err(t!(
            "world.cells_count",
            count = tokens.len(),
            expected = MAP_SIZE
        )
        .into_owned());
    }
    let cell = tokens
        .get_mut(x)
        .ok_or_else(|| t!("world.cell_not_in_row", x = x, max = MAP_SIZE - 1).into_owned())?;
    *cell = token;
    Ok(tokens.join(" "))
}

#[cfg(test)]
mod tests {
    use g4_formats::map::Cell;

    use super::super::doc::{
        FreeformJson, LayerJson, LayerRole, PermissionsJson, PieceJson, WorldJson,
    };
    use super::*;

    fn row(token: &str) -> String {
        vec![token; 32].join(" ")
    }

    fn rows(token: &str) -> Vec<String> {
        vec![row(token); 32]
    }

    fn tile(tile: u16, rotate: u8, flip: bool) -> Option<TileRef> {
        Some(TileRef { tile, rotate, flip })
    }

    fn ground() -> LayerJson {
        LayerJson {
            name: "Boden".to_owned(),
            role: LayerRole::Ground,
            tiles: rows("0000"),
            heights: Some(rows("0")),
            ramps: None,
        }
    }

    fn world(layers: Vec<LayerJson>) -> WorldJson {
        WorldJson {
            version: 2,
            tileset: 0,
            layers,
            pieces: Vec::new(),
            freeform: Vec::new(),
            terrain: None,
            permissions: PermissionsJson {
                derive: true,
                overrides: None,
            },
        }
    }

    /// A small verbatim model part.
    fn hex_part() -> crate::textproject::Part {
        crate::textproject::Part::Hex {
            hex: "00".to_owned(),
        }
    }

    /// `(path, message)` of every error of `world`.
    fn errors_of(world: &WorldJson) -> Vec<(String, String)> {
        WorldGrid::parse(world)
            .expect_err("the world has errors")
            .into_iter()
            .map(|e| (e.path, e.message))
            .collect()
    }

    #[test]
    fn tile_tokens_parse_and_write_back() {
        assert_eq!(parse_tile("0012"), Ok(tile(0x12, 0, false)));
        assert_eq!(parse_tile("0012r1"), Ok(tile(0x12, 1, false)));
        assert_eq!(parse_tile("0012r3f"), Ok(tile(0x12, 3, true)));
        assert_eq!(parse_tile("00ABf"), Ok(tile(0xab, 0, true)));
        assert_eq!(parse_tile("."), Ok(None));
        for token in ["0012", "0012r1", "0012r3f", ".", "ffffr2"] {
            assert_eq!(tile_token(parse_tile(token).unwrap()), token);
        }
        assert_eq!(tile_token(parse_tile("00ABf").unwrap()), "00abf");
        for bad in [
            "12", "0012r4", "0012fr1", "g012", "", "0012r0", "00123", "+012",
        ] {
            let err = parse_tile(bad).expect_err(bad);
            assert!(err.contains(&format!("`{bad}`")), "{bad}: {err}");
        }
        assert_eq!(
            parse_tile("g012").unwrap_err(),
            "`g012` is not a tile (`.` or four hex digits, followed by `r1` to `r3` and `f`)."
        );
    }

    #[test]
    fn ramp_tokens_are_direction_and_steps() {
        assert_eq!(parse_ramp("n1"), Ok(Ramp::North(1)));
        assert_eq!(parse_ramp("e2"), Ok(Ramp::East(2)));
        assert_eq!(parse_ramp("s1"), Ok(Ramp::South(1)));
        assert_eq!(parse_ramp("w2"), Ok(Ramp::West(2)));
        assert_eq!(parse_ramp("."), Ok(Ramp::None));
        for token in [".", "n1", "n2", "e1", "e2", "s1", "s2", "w1", "w2"] {
            assert_eq!(ramp_token(parse_ramp(token).unwrap()), token);
        }
        // 6.2's spelling with a leading zero reads the same; written is the canonical form.
        assert_eq!(parse_ramp("e01"), Ok(Ramp::East(1)));
        assert_eq!(ramp_token(parse_ramp("w02").unwrap()), "w2");
        for bad in ["n3", "x1", "n0", "", "n", "N1", "n12", "n+1", "e256"] {
            let err = parse_ramp(bad).expect_err(bad);
            assert!(err.contains(&format!("`{bad}`")), "{bad}: {err}");
        }
    }

    #[test]
    fn height_range_is_minus_16_to_62() {
        assert_eq!(parse_height("-16"), Ok(-16));
        assert_eq!(parse_height("0"), Ok(0));
        assert_eq!(parse_height("62"), Ok(62));
        assert_eq!(
            parse_height("70").unwrap_err(),
            "`70` is not within −16 to 62."
        );
        assert_eq!(
            parse_height("-17").unwrap_err(),
            "`-17` is not within −16 to 62."
        );
        // 6.2's spelling with a sign reads the same.
        assert_eq!(parse_height("+3"), Ok(3));
        assert_eq!(parse_height("+62"), Ok(62));
        assert_eq!(parse_height("007"), Ok(7));
        for bad in ["", "x", "1.5", "+", "-", "+-3", "--1", "99999999999"] {
            let err = parse_height(bad).expect_err(bad);
            assert!(err.contains(&format!("`{bad}`")), "{bad}: {err}");
        }
    }

    #[test]
    fn override_tokens_are_kind_and_collision() {
        assert_eq!(parse_override("."), Ok(None));
        assert_eq!(
            parse_override("0280"),
            Ok(Some(Cell {
                kind: 0x02,
                collision: 0x80
            }))
        );
        assert_eq!(override_token(parse_override("0A8f").unwrap()), "0a8f");
        assert_eq!(override_token(None), ".");
        for bad in ["028", "zz00", "", "02800"] {
            let err = parse_override(bad).expect_err(bad);
            assert!(err.contains(&format!("`{bad}`")), "{bad}: {err}");
        }
    }

    #[test]
    fn set_token_keeps_the_other_cells() {
        let mut tokens: Vec<String> = (0..32).map(|i| format!("{i:04x}")).collect();
        let line = tokens.join(" ");
        let changed = set_token(&line, 5, "00ffr1").unwrap();
        tokens[5] = "00ffr1".to_owned();
        assert_eq!(changed, tokens.join(" "));
        assert_eq!(
            set_token(&changed, 0, "."),
            Ok(changed.replacen("0000", ".", 1))
        );
        assert_eq!(
            set_token(&line, 32, "0000").unwrap_err(),
            "Cell 32 is not in the row (0 to 31)."
        );
        assert_eq!(
            set_token("0000 0000", 0, "0001").unwrap_err(),
            "2 cells instead of 32."
        );
    }

    #[test]
    fn a_good_world_parses_into_the_grid() {
        let mut w = world(vec![ground()]);
        w.layers[0].tiles[1] = set_token(&row("0000"), 3, "0012r2f").unwrap();
        w.layers[0].heights.as_mut().unwrap()[2] = set_token(&row("0"), 4, "-16").unwrap();
        let mut ramps = rows(".");
        ramps[31] = set_token(&row("."), 31, "e1").unwrap();
        w.layers[0].ramps = Some(ramps);
        w.layers.push(LayerJson {
            name: "Deko".to_owned(),
            role: LayerRole::Overlay,
            tiles: rows("."),
            heights: None,
            ramps: None,
        });
        let mut overrides = rows(".");
        overrides[0] = set_token(&row("."), 1, "0280").unwrap();
        w.permissions.overrides = Some(overrides);
        w.pieces.push(PieceJson {
            tile: 5,
            at: [31, 31],
            rotate: 90,
        });
        let g = WorldGrid::parse(&w).unwrap();
        assert_eq!(g.tileset, 0);
        assert_eq!(g.layers.len(), 2);
        assert_eq!(g.layers[0].name, "Boden");
        assert_eq!(g.layers[1].role, LayerRole::Overlay);
        assert_eq!(g.layers[0].tiles.len(), 1024);
        assert_eq!(g.layers[0].tiles[32 + 3], tile(0x12, 2, true));
        assert_eq!(g.layers[0].tiles[0], tile(0, 0, false));
        assert_eq!(g.layers[1].tiles[7], None);
        assert_eq!(g.heights[2 * 32 + 4], -16);
        assert_eq!(g.heights[0], 0);
        assert_eq!(g.ramps[31 * 32 + 31], Ramp::East(1));
        assert_eq!(g.ramps[0], Ramp::None);
        assert!(g.derive);
        assert_eq!(
            g.overrides[1],
            Some(Cell {
                kind: 0x02,
                collision: 0x80
            })
        );
        assert_eq!(g.overrides[0], None);
        assert_eq!(g.pieces, w.pieces);

        // Without `ramps` and `override` every cell has none.
        let g = WorldGrid::parse(&world(vec![ground()])).unwrap();
        assert!(g.ramps.iter().all(|r| *r == Ramp::None));
        assert!(g.overrides.iter().all(Option::is_none));
        assert_eq!(g.ramps.len(), 1024);
        assert_eq!(g.overrides.len(), 1024);
    }

    #[test]
    fn parse_reports_every_error_with_its_path() {
        let mut w = world(vec![ground()]);
        w.version = 3;
        let g = &mut w.layers[0];
        g.tiles[4] = vec!["0000"; 31].join(" ");
        g.tiles[5] = set_token(&row("0000"), 7, "00g2").unwrap();
        g.heights.as_mut().unwrap()[3] = set_token(&row("0"), 0, "70").unwrap();
        let mut ramps = rows(".");
        ramps[2] = set_token(&row("."), 1, "n3").unwrap();
        ramps[6] = set_token(&row("."), 2, "n0").unwrap();
        g.ramps = Some(ramps);
        w.layers.push(LayerJson {
            name: "Deko".to_owned(),
            role: LayerRole::Overlay,
            tiles: vec![row("."); 31],
            heights: Some(rows("0")),
            ramps: Some(rows(".")),
        });
        w.layers.push(LayerJson {
            name: "Zweiter Boden".to_owned(),
            role: LayerRole::Ground,
            tiles: rows("0000"),
            heights: Some(rows("0")),
            ramps: None,
        });
        let mut overrides = rows("0000");
        overrides[9] = set_token(&row("0000"), 3, ".").unwrap();
        overrides[10] = set_token(&row("0000"), 0, "zz00").unwrap();
        w.permissions = PermissionsJson {
            derive: false,
            overrides: Some(overrides),
        };
        w.pieces = vec![
            PieceJson {
                tile: 5,
                at: [0, 0],
                rotate: 90,
            },
            PieceJson {
                tile: 5,
                at: [1, 1],
                rotate: 45,
            },
        ];
        w.freeform = vec![
            FreeformJson {
                glb: Some("0000.a.glb".to_owned()),
                nsbmd: Some(hex_part()),
                at: [0, 0, 0],
            },
            FreeformJson {
                glb: None,
                nsbmd: None,
                at: [0, 0, 0],
            },
            FreeformJson {
                glb: Some("0000.b.glb".to_owned()),
                nsbmd: None,
                at: [0, 0, 0],
            },
        ];
        let tile_rule = "(`.` or four hex digits, followed by `r1` to `r3` and `f`)";
        let ramp_rule = "(`.` or `n`, `e`, `s`, `w` with 1 or 2 steps)";
        let expected: Vec<(String, String)> = [
            (
                "world.version",
                "Version 3 does not know this version of the file (2).".to_owned(),
            ),
            ("world.layers[0].tiles[4]", "31 cells instead of 32.".to_owned()),
            (
                "world.layers[0].tiles[5]",
                format!("Cell 7: `00g2` is not a tile {tile_rule}."),
            ),
            (
                "world.layers[0].heights[3]",
                "Cell 0: `70` is not within −16 to 62.".to_owned(),
            ),
            (
                "world.layers[0].ramps[2]",
                format!("Cell 1: `n3` is not a ramp {ramp_rule}."),
            ),
            (
                "world.layers[0].ramps[6]",
                format!("Cell 2: `n0` is not a ramp {ramp_rule}."),
            ),
            ("world.layers[1].tiles", "31 rows instead of 32.".to_owned()),
            (
                "world.layers[1].heights",
                "only the ground layer carries heights and ramps.".to_owned(),
            ),
            (
                "world.layers[1].ramps",
                "only the ground layer carries heights and ramps.".to_owned(),
            ),
            (
                "world.layers[2].role",
                "the first layer is the only ground layer (`ground`).".to_owned(),
            ),
            (
                "world.permissions.override[10]",
                "Cell 0: `zz00` is not a permission (`.` or `kkcc` of four hex digits, kind and collision)."
                    .to_owned(),
            ),
            (
                "world.permissions.override",
                "without derivation (`derive: false`) every cell needs a permission; 1 cell is `.`."
                    .to_owned(),
            ),
            (
                "world.pieces[1].rotate",
                "`45` is not a turn (0, 90, 180 or 270).".to_owned(),
            ),
            (
                "world.freeform[0]",
                "needs exactly one of `glb` and `nsbmd`.".to_owned(),
            ),
            (
                "world.freeform[1]",
                "needs exactly one of `glb` and `nsbmd`.".to_owned(),
            ),
        ]
        .into_iter()
        .map(|(p, m)| (p.to_owned(), m))
        .collect();
        assert_eq!(errors_of(&w), expected);
        assert_eq!(errors_of(&w).len(), 15);
    }

    #[test]
    fn version_one_is_refused() {
        let mut w = world(vec![ground()]);
        assert!(WorldGrid::parse(&w).is_ok());
        w.version = 1;
        assert_eq!(
            errors_of(&w),
            vec![(
                "world.version".to_owned(),
                "Version 1 does not know this version of the file (2).".to_owned()
            )]
        );
    }

    #[test]
    fn at_most_one_verbatim_model_at_the_origin() {
        let verbatim = |at: [i32; 3]| FreeformJson {
            glb: None,
            nsbmd: Some(hex_part()),
            at,
        };
        let mut w = world(vec![ground()]);
        w.freeform = vec![verbatim([0, 0, 0])];
        assert!(WorldGrid::parse(&w).is_ok());
        w.freeform = vec![verbatim([0, 0, 0]), verbatim([0, 0, 0])];
        assert_eq!(
            errors_of(&w),
            vec![(
                "world.freeform[1]".to_owned(),
                "At most one verbatim model (`nsbmd`) in `freeform`.".to_owned()
            )]
        );
        w.freeform = vec![verbatim([0, 8, 0])];
        assert_eq!(
            errors_of(&w),
            vec![(
                "world.freeform[0].at".to_owned(),
                "A verbatim model lies at [0, 0, 0].".to_owned()
            )]
        );
        // A glTF model may lie anywhere and beside a verbatim one (the bake refuses it).
        w.freeform = vec![
            verbatim([0, 0, 0]),
            FreeformJson {
                glb: Some("0000.a.glb".to_owned()),
                nsbmd: None,
                at: [16, 0, 16],
            },
        ];
        assert!(WorldGrid::parse(&w).is_ok());
    }

    #[test]
    fn a_glb_name_must_be_a_side_file() {
        let glb = |name: &str| FreeformJson {
            glb: Some(name.to_owned()),
            nsbmd: None,
            at: [0, 0, 0],
        };
        let mut w = world(vec![ground()]);
        // The parser knows no map: any number of four digits passes.
        w.freeform = vec![glb("0123.haus.glb"), glb("0007.a-b_9.glb")];
        assert!(WorldGrid::parse(&w).is_ok());
        let rule =
            "`glb` names a side file of this map: `NNNN.<name>.glb` with a–z, 0–9, `_`, `-`.";
        for bad in [
            "haus.glb",
            "0123.Haus.glb",
            "0123..glb",
            "0123.haus.gltf",
            "123.haus.glb",
        ] {
            w.freeform = vec![glb("0123.haus.glb"), glb(bad)];
            assert_eq!(
                errors_of(&w),
                vec![("world.freeform[1].glb".to_owned(), rule.to_owned())],
                "{bad}"
            );
        }
    }

    #[test]
    fn at_is_bounded() {
        let glb = |at: [i32; 3]| FreeformJson {
            glb: Some("0000.haus.glb".to_owned()),
            nsbmd: None,
            at,
        };
        let mut w = world(vec![ground()]);
        w.freeform = vec![glb([-512, 511, 0]), glb([511, -512, -512])];
        assert!(WorldGrid::parse(&w).is_ok());
        let rule = "`at` is not within −512 to 511.";
        for at in [[512, 0, 0], [0, -513, 0], [0, 0, 600]] {
            w.freeform = vec![glb([0, 0, 0]), glb(at)];
            assert_eq!(
                errors_of(&w),
                vec![("world.freeform[1].at".to_owned(), rule.to_owned())],
                "{at:?}"
            );
        }
    }

    #[test]
    fn the_ground_layer_comes_first_and_has_heights() {
        let ground_rule = "the first layer is the only ground layer (`ground`).";
        assert_eq!(
            errors_of(&world(Vec::new())),
            vec![("world.layers".to_owned(), ground_rule.to_owned())]
        );
        let mut overlay = ground();
        overlay.role = LayerRole::Overlay;
        overlay.heights = None;
        assert_eq!(
            errors_of(&world(vec![overlay])),
            vec![("world.layers[0].role".to_owned(), ground_rule.to_owned())]
        );
        let mut bare = ground();
        bare.heights = None;
        assert_eq!(
            errors_of(&world(vec![bare])),
            vec![(
                "world.layers[0].heights".to_owned(),
                "the ground layer needs heights (32 rows).".to_owned()
            )]
        );
        let mut w = world(vec![ground()]);
        w.permissions.derive = false;
        assert_eq!(
            errors_of(&w),
            vec![(
                "world.permissions.override".to_owned(),
                "without derivation (`derive: false`) `override` with 32 rows is needed."
                    .to_owned()
            )]
        );
        w.permissions.overrides = Some(rows("0000"));
        assert!(WorldGrid::parse(&w).is_ok());
        assert_eq!(
            format!("{}", WorldGrid::parse(&world(Vec::new())).unwrap_err()[0]),
            format!("world.layers: {ground_rule}")
        );
    }

    #[test]
    fn a_ramp_must_end_at_62_at_most() {
        let mut w = world(vec![ground()]);
        let mut heights = rows("0");
        heights[3] = row("61");
        heights[5] = row("62");
        w.layers[0].heights = Some(heights.clone());
        let mut ramps = rows(".");
        // Row 3: 61 + 1 is 62, fine; 61 + 2 is not.
        ramps[3] = set_token(&set_token(&row("."), 0, "n1").unwrap(), 4, "e2").unwrap();
        ramps[5] = set_token(&row("."), 7, "s1").unwrap();
        w.layers[0].ramps = Some(ramps);
        let errors = WorldGrid::parse(&w).expect_err("ramps end above 62");
        assert_eq!(
            errors
                .iter()
                .map(|e| (e.path.as_str(), e.message.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "world.layers[0].ramps[3]",
                    "Cell 4: The ramp ends above 62 (height 61, 2 steps)."
                ),
                (
                    "world.layers[0].ramps[5]",
                    "Cell 7: The ramp ends above 62 (height 62, 1 step)."
                ),
            ]
        );
        assert_eq!(
            errors.iter().map(error_cell).collect::<Vec<_>>(),
            [Some(4), Some(7)]
        );
    }

    #[test]
    fn error_cell_reads_the_cell_of_a_token_error() {
        let e = |path: &str, message: &str| WorldError {
            path: path.to_owned(),
            message: message.to_owned(),
        };
        assert_eq!(
            error_cell(&e(
                "world.layers[0].tiles[4]",
                "Cell 12: `00g2` is not a tile."
            )),
            Some(12)
        );
        assert_eq!(
            error_cell(&e("world.layers[0].tiles[4]", "31 cells instead of 32.")),
            None
        );
        assert_eq!(
            error_cell(&e(
                "world.version",
                "Version 3 does not know this version of the file (2)."
            )),
            None
        );
        assert_eq!(error_cell(&e("x", "Cell 40: zu weit.")), None);
        let w = {
            let mut w = world(vec![ground()]);
            w.layers[0].tiles[4] = set_token(&row("0000"), 12, "00g2").unwrap();
            w
        };
        let errors = WorldGrid::parse(&w).unwrap_err();
        assert_eq!(
            errors.iter().map(error_cell).collect::<Vec<_>>(),
            [Some(12)]
        );
    }

    #[test]
    fn a_piece_must_fit_on_the_map() {
        let mut w = world(vec![ground()]);
        for rotate in [0, 90, 180, 270] {
            w.pieces.push(PieceJson {
                tile: 5,
                at: [31, 31],
                rotate,
            });
        }
        assert!(WorldGrid::parse(&w).is_ok(), "the footprint is Task 3's");
        w.pieces = vec![
            PieceJson {
                tile: 5,
                at: [0, 0],
                rotate: 360,
            },
            PieceJson {
                tile: 5,
                at: [32, 0],
                rotate: 0,
            },
            PieceJson {
                tile: 5,
                at: [0, 200],
                rotate: 1,
            },
        ];
        assert_eq!(
            errors_of(&w),
            vec![
                (
                    "world.pieces[0].rotate".to_owned(),
                    "`360` is not a turn (0, 90, 180 or 270).".to_owned()
                ),
                (
                    "world.pieces[1].at".to_owned(),
                    "[32, 0] is not on the map (0 to 31).".to_owned()
                ),
                (
                    "world.pieces[2].rotate".to_owned(),
                    "`1` is not a turn (0, 90, 180 or 270).".to_owned()
                ),
                (
                    "world.pieces[2].at".to_owned(),
                    "[0, 200] is not on the map (0 to 31).".to_owned()
                ),
            ]
        );
    }

    /// A world of only cell (5, 5) on the ground (tile 0000), at `h` with `ramp`.
    fn lone_cell(h: i8, ramp: Ramp) -> WorldGrid {
        let mut grid = WorldGrid::parse(&world(vec![ground()])).unwrap();
        let c = 5 * MAP_SIZE + 5;
        for (i, tile) in grid.layers[0].tiles.iter_mut().enumerate() {
            if i != c {
                *tile = None;
            }
        }
        grid.heights[c] = h;
        grid.ramps[c] = ramp;
        grid
    }

    #[test]
    fn ramp_direction_matches_terrain() {
        use super::super::mesh::mesh;
        use super::super::tileset::Tileset;
        use bdhc::Direction;

        let c = 5 * MAP_SIZE + 5;
        // Cell (5, 5): x from −176 to −160, z from −176 (north) to −160 (south).
        let (west, east, north, south) = (-176, -160, -176, -160);
        // `East(1)` at `h = 0`: west edge 0, east edge 8 units, in both modules.
        let terrain = lone_cell(0, Ramp::East(1)).height_grid();
        assert_eq!(terrain.ramp[c], bdhc::Ramp::East(1));
        assert_eq!(terrain.edge_units(c, Direction::West), 0);
        assert_eq!(terrain.edge_units(c, Direction::East), 8);
        let faces = mesh(&lone_cell(0, Ramp::East(1)), &Tileset::synthetic());
        assert_eq!(faces.len(), 1, "{faces:?}");
        for p in faces[0].corners {
            assert_eq!(p[1], if p[0] == west { 0 } else { 8 }, "{p:?}");
        }
        // Every direction, 1 and 2 steps, two heights: the mesher's edges are the terrain's.
        let sides = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        for h in [0, 3] {
            for steps in [1, 2] {
                for ramp in [
                    Ramp::North(steps),
                    Ramp::East(steps),
                    Ramp::South(steps),
                    Ramp::West(steps),
                ] {
                    let grid = lone_cell(h, ramp);
                    let terrain = grid.height_grid();
                    let faces = mesh(&grid, &Tileset::synthetic());
                    assert_eq!(faces.len(), 1, "{ramp:?}: {faces:?}");
                    let corners = faces[0].corners;
                    let mid = |side: Direction| {
                        let on: Vec<i32> = corners
                            .iter()
                            .filter(|p| match side {
                                Direction::North => p[2] == north,
                                Direction::South => p[2] == south,
                                Direction::West => p[0] == west,
                                Direction::East => p[0] == east,
                            })
                            .map(|p| p[1])
                            .collect();
                        assert_eq!(on.len(), 2, "{ramp:?} {side:?}");
                        (on[0] + on[1]) / 2
                    };
                    for side in sides {
                        assert_eq!(
                            mid(side),
                            terrain.edge_units(c, side),
                            "h {h}, {ramp:?}, {side:?}"
                        );
                    }
                    let centre = corners.iter().map(|p| f64::from(p[1])).sum::<f64>() / 4.0;
                    assert_eq!(centre, terrain.center_units(c), "h {h}, {ramp:?}");
                }
            }
        }
    }

    #[test]
    fn ramps_convert_to_and_from_the_terrain_ramp_without_loss() {
        let pairs = [
            (Ramp::None, bdhc::Ramp::None),
            (Ramp::North(1), bdhc::Ramp::North(1)),
            (Ramp::North(2), bdhc::Ramp::North(2)),
            (Ramp::East(1), bdhc::Ramp::East(1)),
            (Ramp::East(2), bdhc::Ramp::East(2)),
            (Ramp::South(1), bdhc::Ramp::South(1)),
            (Ramp::South(2), bdhc::Ramp::South(2)),
            (Ramp::West(1), bdhc::Ramp::West(1)),
            (Ramp::West(2), bdhc::Ramp::West(2)),
        ];
        for (world, terrain) in pairs {
            assert_eq!(bdhc::Ramp::from(world), terrain);
            assert_eq!(Ramp::from(terrain), world);
            assert_eq!(Ramp::from(bdhc::Ramp::from(world)), world);
        }
    }

    #[test]
    fn ramp_top_rule_matches_the_terrain_check() {
        // For every height and ramp the world check refuses a ramp top exactly where 6.2's
        // `HeightGrid::check` does (top ≤ 62), and the bounds of the height are the same.
        assert_eq!((HEIGHT_MIN, HEIGHT_MAX), (bdhc::MIN_STEP, bdhc::MAX_STEP));
        for h in HEIGHT_MIN..=HEIGHT_MAX {
            for ramp in [Ramp::None, Ramp::North(1), Ramp::West(2)] {
                let mut doc = world(vec![ground()]);
                let mut heights = rows("0");
                let mut ramps = rows(".");
                let mut cells: Vec<String> = vec!["0".to_owned(); MAP_SIZE];
                cells[3] = h.to_string();
                heights[2] = cells.join(" ");
                let mut cells: Vec<String> = vec![".".to_owned(); MAP_SIZE];
                cells[3] = ramp_token(ramp);
                ramps[2] = cells.join(" ");
                doc.layers[0].heights = Some(heights);
                doc.layers[0].ramps = Some(ramps);
                let world_refuses = WorldGrid::parse(&doc).is_err();
                let mut terrain = HeightGrid::flat(0);
                terrain.height[2 * MAP_SIZE + 3] = h;
                terrain.ramp[2 * MAP_SIZE + 3] = ramp.into();
                let terrain_refuses = matches!(
                    terrain.check(),
                    Some(bdhc::GridProblem::RampTop { x: 3, y: 2, .. })
                );
                assert_eq!(world_refuses, terrain_refuses, "h {h}, {ramp:?}");
            }
        }
    }

    #[test]
    fn both_row_formats_read_the_same_cells() {
        use crate::terrain::{GridRows, parse_grid};

        // 6.2's rows with its spellings (`+5`, `e01`) and the world document's canonical
        // ones give the same heights and ramps.
        let mut heights = rows("0");
        let mut ramps = rows(".");
        heights[4] = row("+5");
        ramps[4] = row("e01");
        let terrain = parse_grid(&GridRows {
            heights: heights.clone(),
            ramps: ramps.clone(),
        })
        .unwrap();
        let mut doc = world(vec![ground()]);
        doc.layers[0].heights = Some(heights);
        doc.layers[0].ramps = Some(ramps);
        let grid = WorldGrid::parse(&doc).unwrap();
        assert_eq!(grid.height_grid(), terrain);
        assert_eq!(grid.heights[4 * MAP_SIZE + 7], 5);
        assert_eq!(grid.ramps[4 * MAP_SIZE + 7], Ramp::East(1));
        // Written back, 6.2's rows are canonical.
        let rows = crate::terrain::format_grid(&terrain);
        assert_eq!(rows.heights[4], row("5"));
        assert_eq!(rows.ramps[4], row("e1"));
    }
}
