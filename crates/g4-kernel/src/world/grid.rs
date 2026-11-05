//! The grid of a world document: the token grammar of its rows (tile, height, ramp,
//! permission override), the 1024 cells of each layer in memory ([`WorldGrid`], index
//! `z · 32 + x`) and the schema check that reads them ([`WorldGrid::parse`], every error with
//! its path). The grid knows no tileset: tile indices against the package are checked by the
//! text project check, the footprint of a piece against the edge by the mesher.
//!
//! Tokens: a tile is `.` or four hex digits (`0012`), optionally `r1` to `r3` (quarter turns
//! counter-clockwise seen from above), then optionally `f` (mirrored at the z axis after the
//! turn); read in any case, written lowercase. A height is a whole number from −16 to 62
//! (steps of [`STEP_UNITS`](super::doc::STEP_UNITS)). A ramp is `.` or a direction `n`, `e`,
//! `s`, `w` (uphill) with 1 or 2 steps. An override is `.` or `kkcc` like the permissions.

use std::fmt;

use g4_formats::map::{Cell, MAP_SIZE};

use super::doc::{HEIGHT_MAX, HEIGHT_MIN, LayerRole, PieceJson, WORLD_VERSION, WorldJson};

/// The token of an empty cell (no tile, no ramp, no override).
const EMPTY: &str = ".";
/// Cells of one layer.
const CELLS: usize = MAP_SIZE * MAP_SIZE;
/// The turns a piece may have, in degrees.
const PIECE_TURNS: [u16; 4] = [0, 90, 180, 270];

const GROUND_FIRST: &str = "die erste Ebene ist die einzige Bodenebene (`ground`).";
const GROUND_ONLY: &str = "nur die Bodenebene trägt Höhen und Rampen.";

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
            self.push(path, format!("{} Zeilen statt {MAP_SIZE}.", lines.len()));
            ok = false;
        }
        let mut cells = Vec::with_capacity(CELLS);
        for (z, line) in lines.iter().enumerate() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != MAP_SIZE {
                self.push(
                    format!("{path}[{z}]"),
                    format!("{} Zellen statt {MAP_SIZE}.", tokens.len()),
                );
                ok = false;
                continue;
            }
            for (x, token) in tokens.into_iter().enumerate() {
                match parse(token) {
                    Ok(cell) => cells.push(cell),
                    Err(e) => {
                        self.push(format!("{path}[{z}]"), format!("Zelle {x}: {e}"));
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
                format!(
                    "Version {} kennt diese Fassung nicht ({WORLD_VERSION}).",
                    world.version
                ),
            );
        }
        if world.layers.is_empty() {
            errors.push("world.layers", GROUND_FIRST);
        }
        let mut layers = Vec::with_capacity(world.layers.len());
        let mut heights = None;
        let mut ramps = None;
        for (i, layer) in world.layers.iter().enumerate() {
            let at = format!("world.layers[{i}]");
            let ground = layer.role == LayerRole::Ground;
            if ground != (i == 0) {
                errors.push(format!("{at}.role"), GROUND_FIRST);
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
                    None => errors.push(
                        format!("{at}.heights"),
                        "die Bodenebene braucht Höhen (32 Zeilen).",
                    ),
                }
                ramps = match &layer.ramps {
                    Some(lines) => errors.rows(&format!("{at}.ramps"), lines, parse_ramp),
                    None => Some(vec![Ramp::None; CELLS]),
                };
            } else if !ground {
                if layer.heights.is_some() {
                    errors.push(format!("{at}.heights"), GROUND_ONLY);
                }
                if layer.ramps.is_some() {
                    errors.push(format!("{at}.ramps"), GROUND_ONLY);
                }
            }
        }
        let overrides = permissions(world, &mut errors);
        for (i, piece) in world.pieces.iter().enumerate() {
            if !PIECE_TURNS.contains(&piece.rotate) {
                errors.push(
                    format!("world.pieces[{i}].rotate"),
                    format!(
                        "`{}` ist keine Drehung (0, 90, 180 oder 270).",
                        piece.rotate
                    ),
                );
            }
            let [x, z] = piece.at;
            if usize::from(x) >= MAP_SIZE || usize::from(z) >= MAP_SIZE {
                errors.push(
                    format!("world.pieces[{i}].at"),
                    format!(
                        "[{x}, {z}] liegt nicht auf der Karte (0 bis {}).",
                        MAP_SIZE - 1
                    ),
                );
            }
        }
        for (i, model) in world.freeform.iter().enumerate() {
            if model.glb.is_some() == model.nsbmd.is_some() {
                errors.push(
                    format!("world.freeform[{i}]"),
                    "braucht genau eines von `glb` und `nsbmd`.",
                );
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

/// The 1024 overrides of `world` (all `None` without `override`); without `derive` every cell
/// must have one.
fn permissions(world: &WorldJson, errors: &mut Errors) -> Option<Vec<Option<Cell>>> {
    const PATH: &str = "world.permissions.override";
    let p = &world.permissions;
    let Some(lines) = &p.overrides else {
        if !p.derive {
            errors.push(
                PATH,
                "ohne Ableitung (`derive: false`) braucht es `override` mit 32 Zeilen.",
            );
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
        let (noun, verb) = if open == 1 {
            ("Zelle", "ist")
        } else {
            ("Zellen", "sind")
        };
        errors.push(
            PATH,
            format!(
                "ohne Ableitung (`derive: false`) braucht jede Zelle eine Permission; {open} \
                 {noun} {verb} `.`."
            ),
        );
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
    let bad = || {
        format!(
            "`{token}` ist keine Kachel (`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` und \
             `f`)."
        )
    };
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

/// The ramp of a token: `.` or `n`, `e`, `s`, `w` with 1 or 2 steps.
pub fn parse_ramp(token: &str) -> Result<Ramp, String> {
    let bad =
        || format!("`{token}` ist keine Rampe (`.` oder `n`, `e`, `s`, `w` mit 1 oder 2 Stufen).");
    if token == EMPTY {
        return Ok(Ramp::None);
    }
    let steps = match token.get(1..) {
        Some("1") => 1,
        Some("2") => 2,
        _ => return Err(bad()),
    };
    match token.as_bytes()[0] {
        b'n' => Ok(Ramp::North(steps)),
        b'e' => Ok(Ramp::East(steps)),
        b's' => Ok(Ramp::South(steps)),
        b'w' => Ok(Ramp::West(steps)),
        _ => Err(bad()),
    }
}

/// The token of a ramp: `.`, `n1`, `w2`.
pub fn ramp_token(r: Ramp) -> String {
    let (direction, steps) = match r {
        Ramp::None => return EMPTY.to_owned(),
        Ramp::North(s) => ('n', s),
        Ramp::East(s) => ('e', s),
        Ramp::South(s) => ('s', s),
        Ramp::West(s) => ('w', s),
    };
    format!("{direction}{steps}")
}

/// The height step of a token: a whole number from −16 to 62.
pub fn parse_height(token: &str) -> Result<i8, String> {
    let digits = token.strip_prefix('-').unwrap_or(token);
    let value = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| token.parse::<i64>().ok())
        .flatten()
        .ok_or_else(|| format!("`{token}` ist keine Höhe (ganze Zahl von \u{2212}16 bis 62)."))?;
    i8::try_from(value)
        .ok()
        .filter(|h| (HEIGHT_MIN..=HEIGHT_MAX).contains(h))
        .ok_or_else(|| format!("`{token}` liegt nicht in \u{2212}16 bis 62."))
}

/// The permission override of a token: `.` is `None`, else `kkcc` (hex of kind and
/// collision) in any case.
pub fn parse_override(token: &str) -> Result<Option<Cell>, String> {
    if token == EMPTY {
        return Ok(None);
    }
    let value = hex4(token).ok_or_else(|| {
        format!(
            "`{token}` ist keine Permission (`.` oder `kkcc` aus vier Hex-Ziffern, Art und \
             Kollision)."
        )
    })?;
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
        return Err(format!("{} Zellen statt {MAP_SIZE}.", tokens.len()));
    }
    let cell = tokens.get_mut(x).ok_or_else(|| {
        format!(
            "Zelle {x} liegt nicht in der Zeile (0 bis {}).",
            MAP_SIZE - 1
        )
    })?;
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
            version: 1,
            tileset: 0,
            layers,
            pieces: Vec::new(),
            freeform: Vec::new(),
            permissions: PermissionsJson {
                derive: true,
                overrides: None,
            },
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
            "`g012` ist keine Kachel (`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` und `f`)."
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
        for bad in ["n3", "x1", "n0", "", "n", "N1", "n12"] {
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
            "`70` liegt nicht in \u{2212}16 bis 62."
        );
        assert_eq!(
            parse_height("-17").unwrap_err(),
            "`-17` liegt nicht in \u{2212}16 bis 62."
        );
        for bad in ["", "x", "1.5", "+3", "--1", "99999999999"] {
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
            "Zelle 32 liegt nicht in der Zeile (0 bis 31)."
        );
        assert_eq!(
            set_token("0000 0000", 0, "0001").unwrap_err(),
            "2 Zellen statt 32."
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
        w.version = 2;
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
                glb: Some("a.glb".to_owned()),
                nsbmd: Some("a.nsbmd".to_owned()),
                at: [0, 0, 0],
            },
            FreeformJson {
                glb: None,
                nsbmd: None,
                at: [0, 0, 0],
            },
            FreeformJson {
                glb: Some("b.glb".to_owned()),
                nsbmd: None,
                at: [0, 0, 0],
            },
        ];
        let tile_rule = "(`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` und `f`)";
        let ramp_rule = "(`.` oder `n`, `e`, `s`, `w` mit 1 oder 2 Stufen)";
        let expected: Vec<(String, String)> = [
            (
                "world.version",
                "Version 2 kennt diese Fassung nicht (1).".to_owned(),
            ),
            ("world.layers[0].tiles[4]", "31 Zellen statt 32.".to_owned()),
            (
                "world.layers[0].tiles[5]",
                format!("Zelle 7: `00g2` ist keine Kachel {tile_rule}."),
            ),
            (
                "world.layers[0].heights[3]",
                "Zelle 0: `70` liegt nicht in \u{2212}16 bis 62.".to_owned(),
            ),
            (
                "world.layers[0].ramps[2]",
                format!("Zelle 1: `n3` ist keine Rampe {ramp_rule}."),
            ),
            (
                "world.layers[0].ramps[6]",
                format!("Zelle 2: `n0` ist keine Rampe {ramp_rule}."),
            ),
            ("world.layers[1].tiles", "31 Zeilen statt 32.".to_owned()),
            (
                "world.layers[1].heights",
                "nur die Bodenebene trägt Höhen und Rampen.".to_owned(),
            ),
            (
                "world.layers[1].ramps",
                "nur die Bodenebene trägt Höhen und Rampen.".to_owned(),
            ),
            (
                "world.layers[2].role",
                "die erste Ebene ist die einzige Bodenebene (`ground`).".to_owned(),
            ),
            (
                "world.permissions.override[10]",
                "Zelle 0: `zz00` ist keine Permission (`.` oder `kkcc` aus vier Hex-Ziffern, \
                 Art und Kollision)."
                    .to_owned(),
            ),
            (
                "world.permissions.override",
                "ohne Ableitung (`derive: false`) braucht jede Zelle eine Permission; 1 Zelle \
                 ist `.`."
                    .to_owned(),
            ),
            (
                "world.pieces[1].rotate",
                "`45` ist keine Drehung (0, 90, 180 oder 270).".to_owned(),
            ),
            (
                "world.freeform[0]",
                "braucht genau eines von `glb` und `nsbmd`.".to_owned(),
            ),
            (
                "world.freeform[1]",
                "braucht genau eines von `glb` und `nsbmd`.".to_owned(),
            ),
        ]
        .into_iter()
        .map(|(p, m)| (p.to_owned(), m))
        .collect();
        assert_eq!(errors_of(&w), expected);
        assert_eq!(errors_of(&w).len(), 15);
    }

    #[test]
    fn the_ground_layer_comes_first_and_has_heights() {
        let ground_rule = "die erste Ebene ist die einzige Bodenebene (`ground`).";
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
                "die Bodenebene braucht Höhen (32 Zeilen).".to_owned()
            )]
        );
        let mut w = world(vec![ground()]);
        w.permissions.derive = false;
        assert_eq!(
            errors_of(&w),
            vec![(
                "world.permissions.override".to_owned(),
                "ohne Ableitung (`derive: false`) braucht es `override` mit 32 Zeilen.".to_owned()
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
                    "`360` ist keine Drehung (0, 90, 180 oder 270).".to_owned()
                ),
                (
                    "world.pieces[1].at".to_owned(),
                    "[32, 0] liegt nicht auf der Karte (0 bis 31).".to_owned()
                ),
                (
                    "world.pieces[2].rotate".to_owned(),
                    "`1` ist keine Drehung (0, 90, 180 oder 270).".to_owned()
                ),
                (
                    "world.pieces[2].at".to_owned(),
                    "[0, 200] liegt nicht auf der Karte (0 bis 31).".to_owned()
                ),
            ]
        );
    }
}
