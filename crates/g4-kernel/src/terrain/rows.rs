//! The row format of `set_map_terrain`, like the permissions and the world format (Gutachten C
//! §5.2): 32 rows (row `y`, from 0) of 32 cells (column `x`, from 0) separated by spaces.
//! `heights` are integers (steps of 8 units), `ramps` `.` or `n1 n2 e1 e2 s1 s2 w1 w2`
//! (direction uphill, steps per tile), `kinds` one [`TerrainKind::code`] per cell.
//!
//! The cells are lexed by [`lex_height`] and [`lex_ramp`], which the world format's grid reads
//! too (one grammar for both row formats); the steps of a ramp are bounded by
//! [`HeightGrid::check`] here.
//!
//! Refusals are sentences in the current language naming the place ([`KernelError::Refused`]); the shown text
//! goes to the user as it is.

use g4_formats::bdhc::{self, GridProblem, MAX_STEP, MIN_STEP, STEP_UNITS};
use serde::{Deserialize, Serialize};

use super::{CELLS, HeightGrid, SIDE, TerrainKind, edge_rule};
use crate::error::KernelError;

/// The height grid as rows: `heights[y]` and `ramps[y]` hold the 32 cells of row `y`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridRows {
    pub heights: Vec<String>,
    pub ramps: Vec<String>,
}

/// The ramp cells a `ramps` row may hold, as the refusal lists them.
const RAMP_CODES: &str = ". n1 n2 e1 e2 s1 s2 w1 w2";
/// The token of an empty cell in both row formats (no ramp; in the world grid also no tile and
/// no override).
pub(crate) const EMPTY: &str = ".";

fn refused(text: String) -> KernelError {
    KernelError::Refused(text)
}

/// The ramp of a row cell in the grammar of both row formats (the world document and 6.2's
/// rows of `set_map_terrain`): `.` or a direction letter `n`, `e`, `s`, `w` followed by the
/// steps as digits (`e1`, `e01`), lowercase only. The steps are not bounded here: each reader
/// checks their range with its own sentence.
pub fn lex_ramp(token: &str) -> Option<bdhc::Ramp> {
    if token == EMPTY {
        return Some(bdhc::Ramp::None);
    }
    let mut chars = token.chars();
    let letter = chars.next()?;
    let digits = chars.as_str();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let steps: u8 = digits.parse().ok()?;
    match letter {
        'n' => Some(bdhc::Ramp::North(steps)),
        'e' => Some(bdhc::Ramp::East(steps)),
        's' => Some(bdhc::Ramp::South(steps)),
        'w' => Some(bdhc::Ramp::West(steps)),
        _ => None,
    }
}

/// The token of a ramp, the inverse of [`lex_ramp`] in canonical form: `.`, `n1`, `w2`.
pub fn ramp_token(r: bdhc::Ramp) -> String {
    let (direction, steps) = match r {
        bdhc::Ramp::None => return EMPTY.to_owned(),
        bdhc::Ramp::North(s) => ('n', s),
        bdhc::Ramp::East(s) => ('e', s),
        bdhc::Ramp::South(s) => ('s', s),
        bdhc::Ramp::West(s) => ('w', s),
    };
    format!("{direction}{steps}")
}

/// The height of a row cell in the grammar of both row formats: a whole number with an
/// optional sign (`-3`, `+5`, `05`). Not bounded here: each reader checks the range.
pub fn lex_height(token: &str) -> Option<i64> {
    token.parse().ok()
}

/// The 32 × 32 cells of `rows`, split at whitespace; refuses another number of rows or cells.
fn cells<'a>(field: &str, rows: &'a [String]) -> Result<Vec<Vec<&'a str>>, KernelError> {
    if rows.len() != SIDE {
        return Err(refused(
            t!(
                "terrain.rows_expected",
                field = field,
                side = SIDE,
                count = rows.len()
            )
            .into_owned(),
        ));
    }
    rows.iter()
        .enumerate()
        .map(|(y, row)| {
            let cells: Vec<&str> = row.split_whitespace().collect();
            if cells.len() != SIDE {
                return Err(refused(
                    t!(
                        "terrain.row_cells",
                        field = field,
                        y = y,
                        count = cells.len(),
                        side = SIDE
                    )
                    .into_owned(),
                ));
            }
            Ok(cells)
        })
        .collect()
}

/// `v` with the typographic minus of the sentences (`−16`).
fn signed(v: i64) -> String {
    if v < 0 {
        format!("\u{2212}{}", v.unsigned_abs())
    } else {
        v.to_string()
    }
}

/// The sentence of a height outside [`MIN_STEP`]`..=`[`MAX_STEP`] (also for values beyond `i8`).
fn height_text(x: usize, y: usize, height: i64) -> String {
    t!(
        "terrain.height_out_of_range",
        x = x,
        y = y,
        height = signed(height),
        min = signed(i64::from(MIN_STEP)),
        max = signed(i64::from(MAX_STEP))
    )
    .into_owned()
}

/// Parses and checks the grid: 32 rows of 32 cells each in `heights` and `ramps`, integers and
/// ramp codes, then [`HeightGrid::check`]. The first problem is refused with its place.
pub fn parse_grid(rows: &GridRows) -> Result<HeightGrid, KernelError> {
    let heights = cells("heights", &rows.heights)?;
    let ramps = cells("ramps", &rows.ramps)?;
    let mut grid = HeightGrid::flat(0);
    for (y, row) in heights.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let height = lex_height(cell).ok_or_else(|| {
                refused(t!("terrain.not_an_integer", y = y, x = x, cell = cell).into_owned())
            })?;
            grid.height[y * SIDE + x] =
                i8::try_from(height).map_err(|_| refused(height_text(x, y, height)))?;
        }
    }
    for (y, row) in ramps.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            grid.ramp[y * SIDE + x] = lex_ramp(cell).ok_or_else(|| {
                refused(
                    t!(
                        "terrain.not_a_ramp",
                        y = y,
                        x = x,
                        cell = cell,
                        allowed = RAMP_CODES
                    )
                    .into_owned(),
                )
            })?;
        }
    }
    match grid.check() {
        Some(problem) => Err(refused(grid_problem_text(problem))),
        None => Ok(grid),
    }
}

/// Parses the `kinds` rows: 32 rows of 32 [`TerrainKind::code`] characters.
pub fn parse_kinds(rows: &[String]) -> Result<[TerrainKind; 1024], KernelError> {
    let cells = cells("kinds", rows)?;
    let mut kinds = [TerrainKind::Ground; CELLS];
    for (y, row) in cells.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let mut chars = cell.chars();
            let kind = match (chars.next(), chars.next()) {
                (Some(c), None) => TerrainKind::from_code(c),
                _ => None,
            };
            kinds[y * SIDE + x] = kind.ok_or_else(|| {
                let allowed: Vec<String> = TerrainKind::ALL
                    .iter()
                    .map(|k| k.code().to_string())
                    .collect();
                refused(
                    t!(
                        "terrain.not_a_kind",
                        y = y,
                        x = x,
                        cell = cell,
                        allowed = allowed.join(" ")
                    )
                    .into_owned(),
                )
            })?;
        }
    }
    Ok(kinds)
}

/// Rows of 32 cells each, separated by single spaces.
fn join_rows(cells: impl Iterator<Item = String>) -> Vec<String> {
    let cells: Vec<String> = cells.collect();
    cells.chunks(SIDE).map(|row| row.join(" ")).collect()
}

/// The rows of `grid` (single spaces); [`parse_grid`] reads them back.
pub fn format_grid(grid: &HeightGrid) -> GridRows {
    GridRows {
        heights: join_rows(grid.height.iter().map(|h| h.to_string())),
        ramps: join_rows(grid.ramp.iter().map(|&r| ramp_token(r))),
    }
}

/// The rows of `kinds` (single spaces); [`parse_kinds`] reads them back.
pub fn format_kinds(kinds: &[TerrainKind; 1024]) -> Vec<String> {
    join_rows(kinds.iter().map(|k| k.code().to_string()))
}

/// The sentence of a refused grid cell in the current language, naming column and row.
pub fn grid_problem_text(p: GridProblem) -> String {
    match p {
        GridProblem::HeightOutOfRange { x, y, height } => {
            height_text(usize::from(x), usize::from(y), i64::from(height))
        }
        GridProblem::RampSteps { x, y, steps } => t!(
            "terrain.ramp_steps",
            x = x,
            y = y,
            steps = steps,
            units = i32::from(steps) * STEP_UNITS,
            one = STEP_UNITS,
            two = 2 * STEP_UNITS,
            limit = edge_rule().engine_limit
        )
        .into_owned(),
        GridProblem::RampTop { x, y, top } => t!(
            "terrain.ramp_top",
            x = x,
            y = y,
            top = signed(i64::from(top)),
            max = MAX_STEP
        )
        .into_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::terrain::{Direction, Ramp};

    #[test]
    fn lexers_are_shared_with_the_world_grid() {
        let east = Some(Ramp::East(1));
        assert_eq!(crate::terrain::lex_ramp("e01"), east);
        assert_eq!(crate::world::grid::lex_ramp("e01"), east);
        assert_eq!(crate::terrain::lex_height("+5"), Some(5));
        assert_eq!(crate::world::grid::lex_height("+5"), Some(5));
        for token in [".", "n1", "e2", "s1", "w2"] {
            let ramp = lex_ramp(token).unwrap();
            assert_eq!(ramp_token(ramp), token);
            assert_eq!(crate::world::grid::ramp_token(ramp.into()), token);
        }
        assert_eq!(ramp_token(lex_ramp("w02").unwrap()), "w2");
    }

    fn rows_of(cell: &str) -> Vec<String> {
        vec![vec![cell; 32].join(" "); 32]
    }

    fn grid_rows() -> GridRows {
        GridRows {
            heights: rows_of("0"),
            ramps: rows_of("."),
        }
    }

    /// `rows[y]` with cell `x` replaced by `cell`.
    fn with_cell(mut rows: Vec<String>, x: usize, y: usize, cell: &str) -> Vec<String> {
        let mut cells: Vec<String> = rows[y].split(' ').map(str::to_string).collect();
        cells[x] = cell.to_string();
        rows[y] = cells.join(" ");
        rows
    }

    fn refusal<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
        match r {
            Err(KernelError::Refused(text)) => text,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn rows_round_trip() {
        let mut g = HeightGrid::flat(3);
        let ramps = [
            Ramp::North(1),
            Ramp::North(2),
            Ramp::East(1),
            Ramp::East(2),
            Ramp::South(1),
            Ramp::South(2),
            Ramp::West(1),
            Ramp::West(2),
        ];
        for (n, ramp) in ramps.into_iter().enumerate() {
            g.ramp[5 * 32 + 2 * n] = ramp;
        }
        g.height[0] = -16;
        g.height[1023] = 62;
        g.height[40] = -1;
        let rows = format_grid(&g);
        assert_eq!(rows.heights.len(), 32);
        assert_eq!(rows.ramps[5].split(' ').count(), 32);
        assert!(rows.heights[0].starts_with("-16 3 3"));
        assert!(rows.ramps[5].starts_with("n1 . n2 . e1 . e2 . s1 . s2 . w1 . w2 . . ."));
        assert_eq!(parse_grid(&rows).unwrap(), g);

        let mut kinds = [TerrainKind::Ground; 1024];
        for (i, k) in TerrainKind::ALL.into_iter().enumerate() {
            kinds[3 * 32 + i] = k;
        }
        let rows = format_kinds(&kinds);
        assert_eq!(
            rows[3],
            format!("# . g G ~ r i ^ > v < D{}", " .".repeat(20))
        );
        assert_eq!(parse_kinds(&rows).unwrap(), kinds);
    }

    #[test]
    fn rows_parse_with_any_whitespace() {
        let mut rows = grid_rows();
        rows.heights[2] = format!("  1\t{}  ", vec!["2"; 31].join("   "));
        let g = parse_grid(&rows).unwrap();
        assert_eq!(g.height[2 * 32], 1);
        assert_eq!(g.height[2 * 32 + 31], 2);
    }

    #[test]
    fn rows_refuse_with_the_place() {
        let mut short = grid_rows();
        short.heights.truncate(30);
        assert_eq!(
            refusal(parse_grid(&short)),
            "heights: 32 rows expected, 30 found."
        );
        let mut narrow = grid_rows();
        narrow.heights[4] = vec!["0"; 31].join(" ");
        assert_eq!(
            refusal(parse_grid(&narrow)),
            "heights: row 4 has 31 cells instead of 32."
        );
        let not_a_number = GridRows {
            heights: with_cell(rows_of("0"), 7, 4, "x"),
            ramps: rows_of("."),
        };
        assert_eq!(
            refusal(parse_grid(&not_a_number)),
            "heights: row 4, column 7: “x” is not a whole number."
        );
        let mut wide = grid_rows();
        wide.ramps[2] = vec!["."; 33].join(" ");
        assert_eq!(
            refusal(parse_grid(&wide)),
            "ramps: row 2 has 33 cells instead of 32."
        );
        let mut few_ramps = grid_rows();
        few_ramps.ramps.truncate(31);
        assert_eq!(
            refusal(parse_grid(&few_ramps)),
            "ramps: 32 rows expected, 31 found."
        );
        let not_a_ramp = GridRows {
            heights: rows_of("0"),
            ramps: with_cell(rows_of("."), 5, 2, "q1"),
        };
        assert_eq!(
            refusal(parse_grid(&not_a_ramp)),
            "ramps: row 2, column 5: “q1” is not a ramp (allowed: . n1 n2 e1 e2 s1 s2 w1 w2)."
        );
        for bad in ["e", "e+1", "x2", "e999", ".."] {
            let rows = GridRows {
                heights: rows_of("0"),
                ramps: with_cell(rows_of("."), 5, 2, bad),
            };
            assert!(
                refusal(parse_grid(&rows)).contains(&format!("“{bad}” is not a ramp")),
                "{bad}"
            );
        }

        // The grid check, through the rows and as text.
        let high = GridRows {
            heights: with_cell(rows_of("0"), 4, 6, "63"),
            ramps: rows_of("."),
        };
        let height_text = "Tile x 4, y 6: height 63 is outside −16 to 62 steps.";
        assert_eq!(refusal(parse_grid(&high)), height_text);
        assert_eq!(
            grid_problem_text(GridProblem::HeightOutOfRange {
                x: 4,
                y: 6,
                height: 63
            }),
            height_text
        );
        let beyond_i8 = GridRows {
            heights: with_cell(rows_of("0"), 4, 6, "-300"),
            ramps: rows_of("."),
        };
        assert_eq!(
            refusal(parse_grid(&beyond_i8)),
            "Tile x 4, y 6: height −300 is outside −16 to 62 steps."
        );
        let steep = GridRows {
            heights: rows_of("0"),
            ramps: with_cell(rows_of("."), 4, 6, "e3"),
        };
        let steep_text = "Tile x 4, y 6: ramp with 3 steps per tile (24 U); 1 or 2 steps (8 or 16 \
                          U) are allowed, from 20 U on the game refuses the step.";
        assert_eq!(refusal(parse_grid(&steep)), steep_text);
        assert_eq!(
            grid_problem_text(GridProblem::RampSteps {
                x: 4,
                y: 6,
                steps: 3
            }),
            steep_text
        );
        let top = GridRows {
            heights: with_cell(rows_of("0"), 4, 6, "62"),
            ramps: with_cell(rows_of("."), 4, 6, "n1"),
        };
        let top_text = "Tile x 4, y 6: the ramp ends at 63 steps, above 62.";
        assert_eq!(refusal(parse_grid(&top)), top_text);
        assert_eq!(
            grid_problem_text(GridProblem::RampTop {
                x: 4,
                y: 6,
                top: 63
            }),
            top_text
        );

        // The same sentences for `kinds`.
        assert_eq!(
            refusal(parse_kinds(&rows_of(".")[..30])),
            "kinds: 32 rows expected, 30 found."
        );
        let mut wide = rows_of(".");
        wide[2] = vec!["."; 33].join(" ");
        assert_eq!(
            refusal(parse_kinds(&wide)),
            "kinds: row 2 has 33 cells instead of 32."
        );
        assert_eq!(
            refusal(parse_kinds(&with_cell(rows_of("."), 5, 2, "Q"))),
            "kinds: row 2, column 5: “Q” is not a terrain (allowed: # . g G ~ r i ^ > v < D)."
        );
        assert!(refusal(parse_kinds(&with_cell(rows_of("."), 5, 2, "gg"))).contains("“gg”"));
        assert_eq!(
            parse_kinds(&with_cell(rows_of("."), 5, 2, ">")).unwrap()[2 * 32 + 5],
            TerrainKind::Ledge(Direction::East)
        );
    }
}
