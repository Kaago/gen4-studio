//! The permissions of a painted map: pattern rule per cell, then the height edges, then the hand
//! values (decision „Kollision zusammensetzen“, Gutachten B §2.7).

use g4_formats::game::Family;
use g4_formats::map::Cell;
use serde::{Deserialize, Serialize};

use super::{BLOCKED, CELLS, HeightGrid, SIDE, TerrainKind, edge_rule, kind_rule, neighbours};
use crate::error::KernelError;

/// A hand value: cell `(x, y)` (column, row from 0, like the permissions) gets exactly
/// `kind` and `collision`, whatever the derivation said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellOverride {
    pub x: u8,
    pub y: u8,
    pub kind: u8,
    pub collision: u8,
}

/// The 1 024 permission cells (index `y * 32 + x`) of `grid` painted with `kinds`: each cell is
/// the rule of its pattern ([`kind_rule`]), then the height edges ([`block_edges`]).
pub fn derive_permissions(
    grid: &HeightGrid,
    kinds: &[TerrainKind; 1024],
    family: Family,
) -> Vec<Cell> {
    let mut cells: Vec<Cell> = kinds.iter().map(|&k| kind_rule(family, k).cell()).collect();
    block_edges(grid, &mut cells);
    cells
}

/// Blocks (`| 0x80`, sound bits kept) the cell with the higher edge of every pair of neighbours
/// inside the map (each pair once) whose edge heights ([`HeightGrid::edge_units`]) differ by at
/// least `edge_rule().free_below` units. The map border is no edge: the neighbour map does not
/// know the derivation. `cells` holds the 1 024 cells, index `y * 32 + x`; a shorter slice is a
/// caller bug (a debug assertion; in release the cells past its end are skipped).
pub fn block_edges(grid: &HeightGrid, cells: &mut [Cell]) {
    debug_assert_eq!(
        cells.len(),
        CELLS,
        "block_edges takes the 1 024 cells of a map"
    );
    let free_below = edge_rule().free_below;
    for i in 0..CELLS {
        for (side, j) in neighbours(i) {
            let own = grid.edge_units(i, side);
            let other = grid.edge_units(j, side.opposite());
            if (own - other).abs() >= free_below {
                let upper = if own > other { i } else { j };
                if let Some(cell) = cells.get_mut(upper) {
                    cell.collision |= BLOCKED;
                }
            }
        }
    }
}

/// Puts the hand values over `cells` (index `y * 32 + x`), later ones over earlier ones. A
/// value outside the map is refused before any cell changes.
pub fn apply_overrides(cells: &mut [Cell], overrides: &[CellOverride]) -> Result<(), KernelError> {
    let side = SIDE as u8;
    if let Some(o) = overrides.iter().find(|o| o.x >= side || o.y >= side) {
        return Err(KernelError::Refused(format!(
            "overrides: Kachel x {}, y {} liegt außerhalb der Karte (0 bis {}).",
            o.x,
            o.y,
            side - 1
        )));
    }
    for o in overrides {
        if let Some(cell) = cells.get_mut(usize::from(o.y) * SIDE + usize::from(o.x)) {
            *cell = Cell {
                kind: o.kind,
                collision: o.collision,
            };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::terrain::Ramp;

    const GROUND: [TerrainKind; 1024] = [TerrainKind::Ground; 1024];

    fn cell(kind: u8, collision: u8) -> Cell {
        Cell { kind, collision }
    }

    fn at(x: usize, y: usize) -> usize {
        y * SIDE + x
    }

    fn blocked(cells: &[Cell]) -> Vec<(usize, usize)> {
        (0..CELLS)
            .filter(|&i| cells[i].collision & BLOCKED != 0)
            .map(|i| (i % SIDE, i / SIDE))
            .collect()
    }

    fn derive(grid: &HeightGrid) -> Vec<Cell> {
        derive_permissions(grid, &GROUND, Family::Pt)
    }

    #[test]
    fn patterns_give_their_cells() {
        let mut kinds = GROUND;
        kinds[at(1, 1)] = TerrainKind::Door;
        kinds[at(2, 1)] = TerrainKind::TallGrass;
        let cells = derive_permissions(&HeightGrid::flat(0), &kinds, Family::Pt);
        assert_eq!(cells.len(), CELLS);
        assert_eq!(cells[at(1, 1)], cell(0x69, 0x80));
        assert_eq!(cells[at(2, 1)], cell(0x02, 0));
        assert_eq!(blocked(&cells), [(1, 1)]);
    }

    #[test]
    fn one_step_stays_free_two_steps_block_the_upper_cell() {
        let mut g = HeightGrid::flat(0);
        g.height[at(5, 5)] = 1;
        assert_eq!(blocked(&derive(&g)), []);
        g.height[at(5, 5)] = 2;
        assert_eq!(blocked(&derive(&g)), [(5, 5)]);
        g.height[at(5, 5)] = 5;
        assert_eq!(blocked(&derive(&g)), [(5, 5)]);
        // A blocked pattern on the upper cell stays blocked once (no other bit).
        let mut kinds = GROUND;
        kinds[at(5, 5)] = TerrainKind::Blocked;
        let cells = derive_permissions(&g, &kinds, Family::Pt);
        assert_eq!(cells[at(5, 5)], cell(0, 0x80));
    }

    #[test]
    fn a_ramp_is_no_edge() {
        // A stair 0 → 16 units over two cells in every row: x 3 level 0, x 4 East(1) from 0,
        // x 5 East(1) from 1, x 6 and further a plateau at 2.
        let mut g = HeightGrid::flat(2);
        for y in 0..SIDE {
            for x in 0..4 {
                g.height[at(x, y)] = 0;
            }
            g.ramp[at(4, y)] = Ramp::East(1);
            g.height[at(4, y)] = 0;
            g.ramp[at(5, y)] = Ramp::East(1);
            g.height[at(5, y)] = 1;
        }
        assert_eq!(blocked(&derive(&g)), []);
        // The same with one ramp of two steps.
        let mut g = HeightGrid::flat(2);
        for y in 0..SIDE {
            for x in 0..4 {
                g.height[at(x, y)] = 0;
            }
            g.ramp[at(4, y)] = Ramp::East(2);
            g.height[at(4, y)] = 0;
        }
        assert_eq!(blocked(&derive(&g)), []);
    }

    #[test]
    fn a_step_beside_a_ramp_uses_the_edge_midpoint() {
        // A plateau at 2 with a ramp East(2) from 0 at (4, 4): its west edge is 16 units below
        // (3, 4), its north and south edges are 8 units in the middle.
        let mut g = HeightGrid::flat(2);
        g.height[at(4, 4)] = 0;
        g.ramp[at(4, 4)] = Ramp::East(2);
        let cells = derive(&g);
        assert_eq!(cells[at(4, 3)].collision, 0, "8 units to the ramp's side");
        assert_eq!(cells[at(4, 5)].collision, 0);
        assert_eq!(cells[at(4, 4)].collision, 0);
        assert_eq!(blocked(&cells), [(3, 4)]);
        g.height[at(4, 3)] = 3;
        let cells = derive(&g);
        assert_eq!(
            cells[at(4, 3)].collision,
            0x80,
            "16 units to the ramp's side"
        );
        assert_eq!(blocked(&cells), [(4, 3), (3, 4)]);
    }

    #[test]
    fn map_borders_are_no_edges() {
        let mut g = HeightGrid::flat(0);
        for y in 0..SIDE {
            g.height[at(31, y)] = 5;
        }
        let want: Vec<_> = (0..SIDE).map(|y| (31, y)).collect();
        assert_eq!(blocked(&derive(&g)), want);

        let mut g = HeightGrid::flat(0);
        for (x, y) in [(0, 10), (1, 10), (0, 9), (0, 11)] {
            g.height[at(x, y)] = 5;
        }
        let cells = derive(&g);
        assert_eq!(
            cells[at(0, 10)].collision,
            0,
            "no neighbour across the border"
        );
        assert_eq!(blocked(&cells), [(0, 9), (1, 10), (0, 11)]);
    }

    #[test]
    fn hgss_keeps_the_sound_bits_free() {
        let cells = derive_permissions(&HeightGrid::flat(0), &GROUND, Family::Hgss);
        assert!(cells.iter().all(|&c| c == cell(0, 0)));
        let mut g = HeightGrid::flat(0);
        g.height[at(5, 5)] = 2;
        let cells = derive_permissions(&g, &GROUND, Family::Hgss);
        assert_eq!(cells[at(5, 5)].collision, 0x80);
    }

    #[test]
    fn block_edges_matches_derive_permissions() {
        let input = crate::terrain::scenario_input();
        let grid = crate::terrain::parse_grid(&input.grid).unwrap();
        let kinds = crate::terrain::parse_kinds(&input.kinds).unwrap();
        for family in [Family::Pt, Family::Hgss] {
            let mut cells: Vec<Cell> = kinds.iter().map(|&k| kind_rule(family, k).cell()).collect();
            let by_kind = blocked(&cells).len();
            block_edges(&grid, &mut cells);
            assert!(
                blocked(&cells).len() > by_kind,
                "the scenario has height edges"
            );
            assert_eq!(
                cells,
                derive_permissions(&grid, &kinds, family),
                "{family:?}"
            );
        }
    }

    #[test]
    fn block_edges_keeps_sound_bits() {
        let mut g = HeightGrid::flat(0);
        g.height[at(5, 5)] = 2;
        let mut cells = vec![cell(0, 0); CELLS];
        cells[at(5, 5)] = cell(0x10, 0x05);
        cells[at(4, 5)] = cell(0, 0x05);
        block_edges(&g, &mut cells);
        assert_eq!(cells[at(5, 5)], cell(0x10, 0x85));
        assert_eq!(cells[at(4, 5)], cell(0, 0x05), "the lower cell stays free");
        assert_eq!(blocked(&cells), [(5, 5)]);
    }

    #[test]
    fn overrides_replace_whole_cells() {
        let mut g = HeightGrid::flat(0);
        g.height[at(18, 25)] = 2;
        let mut kinds = GROUND;
        kinds[at(18, 25)] = TerrainKind::TallGrass;
        let mut cells = derive_permissions(&g, &kinds, Family::Pt);
        assert_eq!(cells[at(18, 25)], cell(0x02, 0x80));
        let hand = CellOverride {
            x: 18,
            y: 25,
            kind: 0,
            collision: 0,
        };
        apply_overrides(&mut cells, &[hand]).unwrap();
        assert_eq!(cells[at(18, 25)], cell(0, 0));

        let before = cells.clone();
        let outside = CellOverride {
            x: 32,
            y: 0,
            kind: 0x69,
            collision: 0x80,
        };
        let first = CellOverride {
            x: 0,
            y: 0,
            kind: 0x69,
            collision: 0x80,
        };
        match apply_overrides(&mut cells, &[first, outside]) {
            Err(KernelError::Refused(text)) => assert_eq!(
                text,
                "overrides: Kachel x 32, y 0 liegt außerhalb der Karte (0 bis 31)."
            ),
            other => panic!("{other:?}"),
        }
        assert_eq!(cells, before, "nothing changes on a refusal");
        let below = CellOverride {
            y: 40,
            x: 3,
            ..first
        };
        assert!(apply_overrides(&mut cells, &[below]).is_err());
    }
}
