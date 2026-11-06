//! Walk heights of a BDHC per tile and the two-step edges the game lets the player up
//! (decisions „current im Studio“ and „map.step-unblocked auf binären Karten“).

use g4_formats::bdhc::{Bdhc, FX_ONE, TILE_UNITS, tile_center};
use g4_formats::error::FormatError;
use g4_formats::map::MapFile;

use super::{BLOCKED, CELLS, Direction, SIDE, edge_rule, neighbours, xy};

/// The walk height (fx32) at the centre of every tile (index `y * 32 + x`): the highest
/// candidate (`current = i32::MAX`, the upper deck of a bridge as the top view sees it); `None`
/// where no plate lies under the centre.
pub fn map_heights(bdhc: &[u8]) -> Result<[Option<i32>; 1024], FormatError> {
    Ok(centres(&Bdhc::read(bdhc)?))
}

/// The highest candidate at every tile centre.
fn centres(bdhc: &Bdhc) -> [Option<i32>; 1024] {
    std::array::from_fn(|i| {
        let (x, z) = tile_center(i % SIDE, i / SIDE);
        bdhc.height_at(x, z, i32::MAX)
    })
}

/// Height (fx32) of cell `i` at the middle of its edge `side`: the probe 1 unit inside the edge
/// (`current` = the cell's centre `centre`), extrapolated from the centre over the last unit
/// (the probe lies 7 of the 8 units from centre to edge). On a level plate that is the probe;
/// on a ramp the height of its plane at the edge, as [`HeightGrid::edge_units`] of the grid.
///
/// [`HeightGrid::edge_units`]: g4_formats::bdhc::HeightGrid::edge_units
fn edge_height(bdhc: &Bdhc, i: usize, side: Direction, centre: i32) -> Option<f64> {
    let (x, z) = tile_center(i % SIDE, i / SIDE);
    let half = TILE_UNITS / 2;
    let inset = (half - 1) * FX_ONE;
    let (dx, dz) = side.step();
    let probe = bdhc.height_at(x + i32::from(dx) * inset, z + i32::from(dz) * inset, centre)?;
    let (centre, probe) = (f64::from(centre), f64::from(probe));
    Some(centre + (probe - centre) * f64::from(half) / f64::from(half - 1))
}

/// An edge between two neighbour tiles with a step of two steps (16 units) or more that the game
/// lets the player walk, both cells free: `upper` and `lower` are `(x, y)`, `step` the height
/// difference at the edge and `centres` that of the two tile centres, both fx32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeStep {
    pub upper: (u8, u8),
    pub lower: (u8, u8),
    pub step: i32,
    pub centres: i32,
}

/// The free two-step edges of `map`'s BDHC, sorted by `(upper, lower)`. Per pair of neighbours
/// inside the map whose centres differ by `prefilter − tolerance` up to below `engine_limit`
/// units (the game allows the step) and whose cells are both without `0x80`: each cell's height
/// at the middle of the shared edge, from a probe 1 unit inside the edge (with the cell's own
/// centre as `current`) extrapolated along the line from the centre; a difference of at least
/// `free_below − tolerance` units is a free step, upper is the higher side.
pub fn free_steps(map: &MapFile) -> Result<Vec<FreeStep>, FormatError> {
    let bdhc = Bdhc::read(&map.bdhc)?;
    let centre = centres(&bdhc);
    let rule = edge_rule();
    let fx = f64::from(FX_ONE);
    let low = (f64::from(rule.prefilter) - rule.tolerance) * fx;
    let high = f64::from(rule.engine_limit) * fx;
    let least = (f64::from(rule.free_below) - rule.tolerance) * fx;
    // A missing permission cell counts as blocked: the map is broken, nothing to point at.
    let blocked = |i: usize| {
        map.permissions
            .get(i)
            .is_none_or(|c| c.collision & BLOCKED != 0)
    };
    let mut out = Vec::new();
    for i in 0..CELLS {
        for (side, j) in neighbours(i) {
            let (Some(own), Some(other)) = (centre[i], centre[j]) else {
                continue;
            };
            let centres = (i64::from(own) - i64::from(other)).abs();
            if !(low..high).contains(&(centres as f64)) || blocked(i) || blocked(j) {
                continue;
            }
            let (Some(e_own), Some(e_other)) = (
                edge_height(&bdhc, i, side, own),
                edge_height(&bdhc, j, side.opposite(), other),
            ) else {
                continue;
            };
            let step = (e_own - e_other).abs();
            if step < least {
                continue;
            }
            let (upper, lower) = if e_own > e_other { (i, j) } else { (j, i) };
            out.push(FreeStep {
                upper: xy(upper),
                lower: xy(lower),
                step: step.round() as i32,
                centres: centres as i32,
            });
        }
    }
    out.sort_by_key(|s| (s.upper, s.lower));
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use g4_formats::bdhc::{HeightGrid, Normal, Plate, Point, Ramp, Strip, from_grid};
    use g4_formats::map::Cell;

    use super::*;

    fn at(x: usize, y: usize) -> usize {
        y * SIDE + x
    }

    fn map_of(grid: &HeightGrid) -> MapFile {
        MapFile {
            bgs: None,
            permissions: vec![Cell::default(); CELLS],
            buildings: Vec::new(),
            buildings_tail: Vec::new(),
            model: Vec::new(),
            bdhc: from_grid(grid).write(),
        }
    }

    #[test]
    fn heights_of_a_generated_grid_are_its_centres() {
        let mut g = HeightGrid::flat(1);
        for y in 5..11 {
            for x in 20..27 {
                g.height[at(x, y)] = 4;
            }
        }
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
            g.ramp[at(2 * n + 1, 20)] = ramp;
            g.ramp[at(2 * n + 1, 21)] = ramp;
            g.height[at(2 * n + 1, 21)] = -3;
        }
        g.height[at(31, 31)] = -16;
        g.height[at(0, 31)] = 62;
        assert_eq!(g.check(), None);
        let heights = map_heights(&from_grid(&g).write()).unwrap();
        for (i, h) in heights.iter().enumerate() {
            let h = h.unwrap_or_else(|| panic!("tile {:?} has no height", xy(i)));
            let units = f64::from(h) / f64::from(FX_ONE);
            assert!(
                (units - g.center_units(i)).abs() <= 0.1,
                "tile {:?}: {units} vs {}",
                xy(i),
                g.center_units(i)
            );
        }
    }

    #[test]
    fn map_heights_take_the_highest_candidate() {
        // Two plates over the whole map, at 0 and at 32 units.
        let edge = 256 * FX_ONE;
        let bdhc = Bdhc {
            points: vec![Point { x: -edge, z: -edge }, Point { x: edge, z: edge }],
            normals: vec![Normal {
                x: 0,
                y: FX_ONE,
                z: 0,
            }],
            constants: vec![0, -32 * FX_ONE],
            plates: vec![
                Plate {
                    first: 0,
                    second: 1,
                    normal: 0,
                    constant: 0,
                },
                Plate {
                    first: 0,
                    second: 1,
                    normal: 0,
                    constant: 1,
                },
            ],
            strips: vec![Strip {
                scanline: edge,
                count: 2,
                start: 0,
            }],
            access: vec![0, 1],
        };
        let heights = map_heights(&bdhc.write()).unwrap();
        assert!(heights.iter().all(|h| *h == Some(32 * FX_ONE)));
    }

    #[test]
    fn heights_without_a_plate_are_none() {
        let mut bdhc = Bdhc::flat(0);
        bdhc.points[1] = Point { x: 0, z: 0 };
        let heights = map_heights(&bdhc.write()).unwrap();
        assert_eq!(heights[at(0, 0)], Some(0));
        assert_eq!(heights[at(31, 31)], None);
    }

    #[test]
    fn free_steps_find_free_two_step_edges() {
        let mut g = HeightGrid::flat(0);
        g.height[at(5, 5)] = 2;
        let steps = free_steps(&map_of(&g)).unwrap();
        assert_eq!(steps.len(), 4, "{steps:?}");
        assert!(steps.iter().all(|s| s.upper == (5, 5) && s.step == 65536));
        assert!(steps.iter().all(|s| s.centres == 65536));
        let lowers: Vec<_> = steps.iter().map(|s| s.lower).collect();
        assert_eq!(lowers, [(4, 5), (5, 4), (5, 6), (6, 5)], "sorted");

        let mut blocked = map_of(&g);
        blocked.permissions[at(5, 5)].collision = 0x80;
        assert_eq!(free_steps(&blocked).unwrap(), []);
        let mut lower_blocked = map_of(&g);
        lower_blocked.permissions[at(4, 5)].collision = 0x80;
        assert_eq!(free_steps(&lower_blocked).unwrap().len(), 3);
        // HeartGold's sound bits do not block.
        let mut sound = map_of(&g);
        sound.permissions[at(5, 5)].collision = 0x05;
        assert_eq!(free_steps(&sound).unwrap().len(), 4);

        g.height[at(5, 5)] = 3;
        assert_eq!(
            free_steps(&map_of(&g)).unwrap(),
            [],
            "24 units: the game blocks"
        );
        g.height[at(5, 5)] = 1;
        assert_eq!(free_steps(&map_of(&g)).unwrap(), [], "one step is free");

        // A step at the lower edge of a ramp: centres only 8 units apart.
        let mut g = HeightGrid::flat(2);
        g.height[at(4, 4)] = 0;
        g.ramp[at(4, 4)] = Ramp::East(2);
        let steps = free_steps(&map_of(&g)).unwrap();
        assert_eq!(steps.len(), 1, "{steps:?}");
        let s = steps[0];
        assert_eq!((s.upper, s.lower), ((3, 4), (4, 4)));
        // The generated ramp plate is rounded to fx32 (its centre lies 83 fx below 8 units).
        assert!((s.centres - 8 * FX_ONE).abs() <= FX_ONE / 10, "{s:?}");
        assert!((s.step - 16 * FX_ONE).abs() <= FX_ONE / 10, "{s:?}");
        eprintln!("step beside the ramp: {s:?}");

        // A 16-unit stair over two cells is no edge.
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
        assert_eq!(free_steps(&map_of(&g)).unwrap(), []);
        // Nor is one ramp of two steps.
        let mut g = HeightGrid::flat(2);
        for y in 0..SIDE {
            for x in 0..4 {
                g.height[at(x, y)] = 0;
            }
            g.ramp[at(4, y)] = Ramp::East(2);
            g.height[at(4, y)] = 0;
        }
        assert_eq!(free_steps(&map_of(&g)).unwrap(), []);
    }

    #[test]
    fn free_steps_ignore_the_map_border() {
        let mut g = HeightGrid::flat(0);
        for y in 0..SIDE {
            g.height[at(0, y)] = 2;
        }
        let steps = free_steps(&map_of(&g)).unwrap();
        assert_eq!(steps.len(), 32);
        assert!(steps.iter().all(|s| s.upper.0 == 0 && s.lower.0 == 1));
    }

    #[test]
    fn unreadable_bdhc_is_an_error() {
        assert!(map_heights(b"NOPE").is_err());
        let mut map = map_of(&HeightGrid::flat(0));
        map.bdhc = b"NOPE".to_vec();
        assert!(free_steps(&map).is_err());
    }
}
