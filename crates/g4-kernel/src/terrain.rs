//! Terrain of a map: the height grid and the terrain patterns a map is painted with, the
//! permissions derived from them, and the walk heights of a BDHC per tile.
//! See `docs/formats/terrain.md`.
//!
//! - [`rules`](kind_rule): one collision rule per pattern and family from
//!   `metadata/terrain_rules.yaml`, plus the edge rule ([`edge_rule`]).
//! - [`parse_grid`] / [`parse_kinds`] and their `format_*` inverses: the row format of
//!   `set_map_terrain` (32 rows of 32 cells, separated by spaces; row `y`, column `x`).
//! - [`derive_permissions`]: pattern rule per cell, then [`block_edges`] (every height edge
//!   inside the map of at least `free_below` units blocks its upper cell); [`apply_overrides`]
//!   puts hand values last.
//! - [`lex_height`] / [`lex_ramp`]: the cell grammar of both row formats (these rows and the
//!   world document's grid).
//! - [`map_heights`]: the walk height at every tile centre (highest candidate);
//!   [`free_steps`]: two-step edges of a BDHC the game lets the player walk up.
//! - [`set_map_terrain`]: BDHC and permissions of a map from rows, one store step;
//!   [`map_budget`]: model, terrain payload, buildings and texture VRAM against the limits.

mod apply;
mod budget;
mod derive;
mod heights;
mod rows;
mod rules;

#[cfg(any(test, feature = "test-support"))]
pub use apply::scenario_input;
pub use apply::{
    BdhcSource, PermissionSource, SET_TERRAIN_LABEL, TerrainInput, TerrainReport, set_map_terrain,
};
pub use budget::{
    BudgetItem, BudgetState, MapBudget, TextureBudget, budget_state, map_budget, vram_bytes,
};
pub use derive::{CellOverride, apply_overrides, block_edges, derive_permissions};
pub use g4_formats::bdhc::{Direction, HeightGrid, Ramp};
pub use heights::{FreeStep, free_steps, map_heights};
pub use rows::{
    GridRows, format_grid, format_kinds, grid_problem_text, lex_height, lex_ramp, parse_grid,
    parse_kinds,
};
pub use rules::{EdgeRule, KindRule, edge_rule, kind_rule};

/// Cells per row and per column of a map.
const SIDE: usize = 32;
/// Cells of a map, index `y * 32 + x`.
const CELLS: usize = SIDE * SIDE;
/// The collision bit of a permission cell.
const BLOCKED: u8 = 0x80;

/// The terrain pattern of a cell: what the player does there. `Ledge(d)` jumps towards `d`.
/// Each pattern maps to one permission cell per family ([`kind_rule`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TerrainKind {
    Blocked,
    #[default]
    Ground,
    TallGrass,
    VeryTallGrass,
    Sea,
    River,
    Ice,
    Ledge(Direction),
    Door,
}

impl TerrainKind {
    /// Every pattern, in the order of the row codes `# . g G ~ r i ^ > v < D`.
    pub const ALL: [TerrainKind; 12] = [
        TerrainKind::Blocked,
        TerrainKind::Ground,
        TerrainKind::TallGrass,
        TerrainKind::VeryTallGrass,
        TerrainKind::Sea,
        TerrainKind::River,
        TerrainKind::Ice,
        TerrainKind::Ledge(Direction::North),
        TerrainKind::Ledge(Direction::East),
        TerrainKind::Ledge(Direction::South),
        TerrainKind::Ledge(Direction::West),
        TerrainKind::Door,
    ];

    /// The character of the pattern in a `kinds` row: `#` Blocked, `.` Ground, `g` TallGrass,
    /// `G` VeryTallGrass, `~` Sea, `r` River, `i` Ice, `^ > v <` Ledge North / East / South /
    /// West, `D` Door.
    pub fn code(self) -> char {
        match self {
            TerrainKind::Blocked => '#',
            TerrainKind::Ground => '.',
            TerrainKind::TallGrass => 'g',
            TerrainKind::VeryTallGrass => 'G',
            TerrainKind::Sea => '~',
            TerrainKind::River => 'r',
            TerrainKind::Ice => 'i',
            TerrainKind::Ledge(Direction::North) => '^',
            TerrainKind::Ledge(Direction::East) => '>',
            TerrainKind::Ledge(Direction::South) => 'v',
            TerrainKind::Ledge(Direction::West) => '<',
            TerrainKind::Door => 'D',
        }
    }

    /// The pattern of a row character, `None` for any other character.
    pub fn from_code(c: char) -> Option<TerrainKind> {
        TerrainKind::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// Column and row of cell `i`.
fn xy(i: usize) -> (u8, u8) {
    ((i % SIDE) as u8, (i / SIDE) as u8)
}

/// The east and south neighbour of cell `i` inside the map: every pair of neighbours once, with
/// the side of `i` that faces the neighbour. The map border is no edge.
fn neighbours(i: usize) -> impl Iterator<Item = (Direction, usize)> {
    let (x, y) = (i % SIDE, i / SIDE);
    [
        (x + 1 < SIDE).then_some((Direction::East, i + 1)),
        (y + 1 < SIDE).then_some((Direction::South, i + SIDE)),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for k in TerrainKind::ALL {
            assert_eq!(TerrainKind::from_code(k.code()), Some(k), "{k:?}");
        }
        let codes: String = TerrainKind::ALL.iter().map(|k| k.code()).collect();
        assert_eq!(codes, "#.gG~ri^>v<D");
        assert_eq!(TerrainKind::from_code('Q'), None);
        assert_eq!(TerrainKind::default(), TerrainKind::Ground);
    }

    #[test]
    fn neighbours_stay_inside_the_map() {
        assert_eq!(
            neighbours(0).collect::<Vec<_>>(),
            [(Direction::East, 1), (Direction::South, 32)]
        );
        assert_eq!(neighbours(31).collect::<Vec<_>>(), [(Direction::South, 63)]);
        assert_eq!(
            neighbours(31 * 32).collect::<Vec<_>>(),
            [(Direction::East, 31 * 32 + 1)]
        );
        assert_eq!(neighbours(CELLS - 1).count(), 0);
        let pairs: usize = (0..CELLS).map(|i| neighbours(i).count()).sum();
        assert_eq!(pairs, 2 * 32 * 31);
        assert_eq!(xy(32 * 6 + 4), (4, 6));
    }
}
