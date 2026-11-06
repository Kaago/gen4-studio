//! Generator: a 32 × 32 height grid (one plane per cell) → BDHC, bounded by
//! construction to [`MAX_PAYLOAD_FROM_GRID`] bytes of payload.
//! See `docs/formats/bdhc.md`, „Erzeugen aus dem Höhenraster“.
//!
//! Each cell is one plane, key [`CellPlane`]; equal keys merge into one
//! rectangle (greedy, row by row: right, then down), so a ramp over two cells
//! is one plate. Points, normals and constants are deduplicated in plate
//! order; the strips are the bands between the sorted z edges, each listing
//! only the rectangles that overlap it with an area (half-open bands). The
//! bytes match the prototype `gutachten-b-bdhc-bau.py` (`build`).

use std::collections::{BTreeSet, HashMap};

use super::{Bdhc, FX_ONE, HALF_MAP_UNITS, Normal, Plate, Point, STEP_UNITS, Strip, TILE_UNITS};

/// Cells per row and per column.
const SIDE: usize = 32;
/// Cells of a map.
const CELLS: usize = SIDE * SIDE;

/// A compass direction on the map; north is −z (row `y` grows southwards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

/// The slope of a cell: the direction it rises towards and the steps it
/// rises over the tile. The cell's height is that of its lower edge (opposite
/// the direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ramp {
    None,
    North(u8),
    East(u8),
    South(u8),
    West(u8),
}

/// A map's walk heights in steps of [`STEP_UNITS`], one cell per tile, index
/// `y * 32 + x` like the permissions. A ramp's `height` is its lower edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeightGrid {
    pub height: [i8; 1024],
    pub ramp: [Ramp; 1024],
}

/// The plane of one cell as an integer key: `2 · y = c + kx · x + kz · z` in
/// units from the map centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPlane {
    pub c: i32,
    pub kx: i32,
    pub kz: i32,
}

/// The first cell [`HeightGrid::check`] refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridProblem {
    /// `height` outside [`MIN_STEP`]`..=`[`MAX_STEP`].
    HeightOutOfRange { x: u8, y: u8, height: i8 },
    /// A ramp rises neither one nor two steps.
    RampSteps { x: u8, y: u8, steps: u8 },
    /// A ramp's upper edge `height + steps` lies above [`MAX_STEP`].
    RampTop { x: u8, y: u8, top: i16 },
}

/// Lowest height of a grid, in steps (−128 units).
pub const MIN_STEP: i8 = -16;
/// Highest height (also of a ramp's upper edge), in steps (496 units).
pub const MAX_STEP: i8 = 62;
/// Upper bound of [`Bdhc::payload_len`] for [`from_grid`] and
/// [`from_planes`]: 23 304 + 12 · 1 024 (B §2.6), below the game's 0x9000.
pub const MAX_PAYLOAD_FROM_GRID: usize = 35_592;

impl Ramp {
    /// Direction and steps of the rise, `None` for a level cell.
    pub fn rise(self) -> Option<(Direction, u8)> {
        match self {
            Ramp::None => None,
            Ramp::North(s) => Some((Direction::North, s)),
            Ramp::East(s) => Some((Direction::East, s)),
            Ramp::South(s) => Some((Direction::South, s)),
            Ramp::West(s) => Some((Direction::West, s)),
        }
    }
}

impl Direction {
    /// The direction pointing the other way (North and South, East and West swap).
    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    /// `(dx, dy)` to the neighbour cell: North `(0, −1)`, East `(1, 0)`,
    /// South `(0, 1)`, West `(−1, 0)`.
    pub fn step(self) -> (i8, i8) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }
}

impl HeightGrid {
    /// Every cell level at `height` steps.
    pub fn flat(height: i8) -> HeightGrid {
        HeightGrid {
            height: [height; CELLS],
            ramp: [Ramp::None; CELLS],
        }
    }

    /// The first refused cell in index order: height in
    /// [`MIN_STEP`]`..=`[`MAX_STEP`], ramps of 1 or 2 steps, a ramp's top
    /// `height + steps ≤` [`MAX_STEP`]; per cell checked in that order.
    pub fn check(&self) -> Option<GridProblem> {
        (0..CELLS).find_map(|i| {
            let (x, y) = ((i % SIDE) as u8, (i / SIDE) as u8);
            let height = self.height[i];
            if !(MIN_STEP..=MAX_STEP).contains(&height) {
                return Some(GridProblem::HeightOutOfRange { x, y, height });
            }
            let (_, steps) = self.ramp[i].rise()?;
            if !(1..=2).contains(&steps) {
                return Some(GridProblem::RampSteps { x, y, steps });
            }
            let top = i16::from(height) + i16::from(steps);
            (top > i16::from(MAX_STEP)).then_some(GridProblem::RampTop { x, y, top })
        })
    }

    /// Plane of cell `i`: level `(16h, 0, 0)`; `East(s)` `(16h − s·x_w, s, 0)`,
    /// `West(s)` `(16h + s·x_e, −s, 0)`, `South(s)` `(16h − s·z_n, 0, s)`,
    /// `North(s)` `(16h + s·z_s, 0, −s)` with the cell's west, east, north and
    /// south edges `x_w`, `x_e`, `z_n`, `z_s` in units from the map centre.
    /// Total: unchecked steps give a steeper plane.
    pub fn plane(&self, i: usize) -> CellPlane {
        let x_w = (i % SIDE) as i32 * TILE_UNITS - HALF_MAP_UNITS;
        let z_n = (i / SIDE) as i32 * TILE_UNITS - HALF_MAP_UNITS;
        let (x_e, z_s) = (x_w + TILE_UNITS, z_n + TILE_UNITS);
        let c = 2 * STEP_UNITS * i32::from(self.height[i]);
        let (c, kx, kz) = match self.ramp[i] {
            Ramp::None => (c, 0, 0),
            Ramp::East(s) => (c - i32::from(s) * x_w, i32::from(s), 0),
            Ramp::West(s) => (c + i32::from(s) * x_e, -i32::from(s), 0),
            Ramp::South(s) => (c - i32::from(s) * z_n, 0, i32::from(s)),
            Ramp::North(s) => (c + i32::from(s) * z_s, 0, -i32::from(s)),
        };
        CellPlane { c, kx, kz }
    }

    /// Height in units at the middle of cell `i`'s edge `side`: the lower
    /// edge of a ramp `8h`, the upper `8h + 8s`, the two others `8h + 4s`; a
    /// level cell `8h` everywhere.
    pub fn edge_units(&self, i: usize, side: Direction) -> i32 {
        let low = STEP_UNITS * i32::from(self.height[i]);
        match self.ramp[i].rise() {
            None => low,
            Some((up, s)) if side == up => low + STEP_UNITS * i32::from(s),
            Some((up, _)) if side == up.opposite() => low,
            Some((_, s)) => low + STEP_UNITS / 2 * i32::from(s),
        }
    }

    /// Height in units at the centre of cell `i`: `8h + 4s` for a ramp, else
    /// `8h`.
    pub fn center_units(&self, i: usize) -> f64 {
        let s = self.ramp[i].rise().map_or(0, |(_, s)| i32::from(s));
        f64::from(STEP_UNITS * i32::from(self.height[i]) + STEP_UNITS / 2 * s)
    }
}

impl CellPlane {
    /// Height in units at `(x, z)` units from the map centre:
    /// `(c + kx·x + kz·z) / 2`.
    pub fn units_at(self, x_units: f64, z_units: f64) -> f64 {
        (f64::from(self.c) + f64::from(self.kx) * x_units + f64::from(self.kz) * z_units) / 2.0
    }

    /// Normal and constant in fx32: with `L = √(1 + (kx/2)² + (kz/2)²)`,
    /// `n = round((−kx/2, 1, −kz/2) / L · 4096)`, `d = round(−c/2 / L · 4096)`.
    fn fx(self) -> (Normal, i32) {
        let fx = f64::from(FX_ONE);
        let (a, bx, bz) = (
            f64::from(self.c) / 2.0,
            f64::from(self.kx) / 2.0,
            f64::from(self.kz) / 2.0,
        );
        let len = (1.0 + bx * bx + bz * bz).sqrt();
        let normal = Normal {
            x: (-bx / len * fx).round() as i32,
            y: (1.0 / len * fx).round() as i32,
            z: (-bz / len * fx).round() as i32,
        };
        (normal, (-a / len * fx).round() as i32)
    }
}

/// A rectangle of equal planes in cells: column, row, width, height.
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    plane: CellPlane,
}

/// Greedy decomposition: row by row the first free cell grows right while the
/// plane is equal, then down while the whole width is equal.
fn rectangles(planes: &[CellPlane; CELLS]) -> Vec<Rect> {
    let mut used = [false; CELLS];
    let mut out = Vec::new();
    for y in 0..SIDE {
        for x in 0..SIDE {
            if used[y * SIDE + x] {
                continue;
            }
            let plane = planes[y * SIDE + x];
            let free = |used: &[bool; CELLS], x: usize, y: usize| {
                !used[y * SIDE + x] && planes[y * SIDE + x] == plane
            };
            let mut w = 1;
            while x + w < SIDE && free(&used, x + w, y) {
                w += 1;
            }
            let mut h = 1;
            while y + h < SIDE && (x..x + w).all(|cx| free(&used, cx, y + h)) {
                h += 1;
            }
            for cy in y..y + h {
                used[cy * SIDE + x..cy * SIDE + x + w].fill(true);
            }
            out.push(Rect { x, y, w, h, plane });
        }
    }
    out
}

/// Index of `value` in `table`, appending it on first sight.
fn index_of<K: std::hash::Hash + Eq + Copy, V>(
    table: &mut Vec<V>,
    seen: &mut HashMap<K, u16>,
    key: K,
    value: V,
) -> u16 {
    *seen.entry(key).or_insert_with(|| {
        table.push(value);
        to_u16(table.len() - 1)
    })
}

/// Every table of a generated BDHC has at most 1 089 entries (33 × 33 points).
fn to_u16(n: usize) -> u16 {
    u16::try_from(n).expect("a 32 × 32 grid gives at most 1 089 entries per table")
}

/// BDHC of one plane per cell (index `y * 32 + x`): rectangles of equal
/// planes, deduplicated points, normals and constants, strips between the
/// sorted z edges with the rectangles that overlap each band with an area.
/// The payload is at most [`MAX_PAYLOAD_FROM_GRID`] for any input.
pub fn from_planes(planes: &[CellPlane; 1024]) -> Bdhc {
    let edge = |cells: usize| (cells as i32 * TILE_UNITS - HALF_MAP_UNITS) * FX_ONE;
    let mut b = Bdhc::default();
    let (mut points, mut normals, mut constants) = (HashMap::new(), HashMap::new(), HashMap::new());
    let mut bands = Vec::new();
    for r in rectangles(planes) {
        let (x1, z1, x2, z2) = (edge(r.x), edge(r.y), edge(r.x + r.w), edge(r.y + r.h));
        let first = index_of(&mut b.points, &mut points, (x1, z1), Point { x: x1, z: z1 });
        let second = index_of(&mut b.points, &mut points, (x2, z2), Point { x: x2, z: z2 });
        let (n, d) = r.plane.fx();
        let normal = index_of(&mut b.normals, &mut normals, (n.x, n.y, n.z), n);
        let constant = index_of(&mut b.constants, &mut constants, d, d);
        b.plates.push(Plate {
            first,
            second,
            normal,
            constant,
        });
        bands.push((z1, z2));
    }
    let edges: Vec<i32> = bands
        .iter()
        .flat_map(|&(z1, z2)| [z1, z2])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    for pair in edges.windows(2) {
        let (low, high) = (pair[0], pair[1]);
        let start = to_u16(b.access.len());
        b.access.extend(
            bands
                .iter()
                .enumerate()
                .filter(|&(_, &(z1, z2))| z1 < high && z2 > low)
                .map(|(plate, _)| to_u16(plate)),
        );
        b.strips.push(Strip {
            scanline: high,
            count: to_u16(b.access.len()) - start,
            start,
        });
    }
    b
}

/// [`from_planes`] over [`HeightGrid::plane`] of every cell. Total: an
/// unchecked grid still gives a BDHC (the kernel refuses it before).
pub fn from_grid(grid: &HeightGrid) -> Bdhc {
    from_planes(&std::array::from_fn(|i| grid.plane(i)))
}

#[cfg(test)]
mod tests {
    use super::super::{FX_ONE, Normal, tile_center};
    use super::*;

    const FLAT: CellPlane = CellPlane { c: 0, kx: 0, kz: 0 };

    fn counts(b: &Bdhc) -> (usize, usize, usize, usize, usize, usize) {
        (
            b.points.len(),
            b.normals.len(),
            b.constants.len(),
            b.plates.len(),
            b.strips.len(),
            b.access.len(),
        )
    }

    fn flat_plane(c: i32) -> CellPlane {
        CellPlane { c, kx: 0, kz: 0 }
    }

    /// xorshift64 (13, 7, 17).
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        /// `0..n`.
        fn below(&mut self, n: u64) -> i32 {
            (self.next() % n) as i32
        }
    }

    fn ramp(direction: i32, steps: u8) -> Ramp {
        match direction {
            0 => Ramp::North(steps),
            1 => Ramp::East(steps),
            2 => Ramp::South(steps),
            _ => Ramp::West(steps),
        }
    }

    /// 1 000 random grids from xorshift64 seed 6, then the four extremes.
    fn sample_grids() -> Vec<HeightGrid> {
        let mut rng = Rng(6);
        let mut out = Vec::with_capacity(1004);
        for _ in 0..1000 {
            let mut g = HeightGrid::flat(0);
            for _ in 0..1 + rng.below(40) {
                let x0 = rng.below(32);
                let y0 = rng.below(32);
                let w = 1 + rng.below((32 - x0) as u64);
                let h = 1 + rng.below((32 - y0) as u64);
                let height = (i32::from(MIN_STEP) + rng.below(79)) as i8;
                for y in y0..y0 + h {
                    for x in x0..x0 + w {
                        g.height[(y * 32 + x) as usize] = height;
                    }
                }
            }
            for i in 0..1024 {
                if rng.below(100) < 15 {
                    let direction = rng.below(4);
                    let steps = 1 + rng.below(2) as u8;
                    g.ramp[i] = ramp(direction, steps);
                    g.height[i] = g.height[i].min(MAX_STEP - steps as i8);
                }
            }
            out.push(g);
        }
        let mut checker = HeightGrid::flat(0);
        let mut diagonal = HeightGrid::flat(0);
        let mut ramps = HeightGrid::flat(0);
        for y in 0..32 {
            for x in 0..32 {
                let i = y * 32 + x;
                checker.height[i] = (2 * ((x + y) % 2)) as i8;
                diagonal.height[i] = ((7 * x + 13 * y) % 79) as i8 + MIN_STEP;
                ramps.ramp[i] = ramp(((x + y) % 4) as i32, 1 + ((x * y) % 2) as u8);
                ramps.height[i] = ((x + y) % 60) as i8;
            }
        }
        out.extend([HeightGrid::flat(0), checker, diagonal, ramps]);
        out
    }

    #[test]
    fn the_emulator_probe_is_rebuilt_exactly() {
        let mut planes = [FLAT; 1024];
        for x in 0..32 {
            planes[6 * 32 + x] = match x {
                0..=3 => FLAT,
                4..=5 => CellPlane {
                    c: 192,
                    kx: 1,
                    kz: 0,
                },
                6..=7 => flat_plane(32),
                _ => flat_plane(80),
            };
            planes[8 * 32 + x] = match x {
                0..=3 => FLAT,
                4..=5 => flat_plane(32),
                _ => flat_plane(72),
            };
        }
        let b = from_planes(&planes);
        assert_eq!(b.write().len(), 340);
        assert_eq!(counts(&b), (18, 2, 5, 9, 5, 12));
        assert_eq!(
            b.normals,
            [
                Normal {
                    x: 0,
                    y: 4096,
                    z: 0
                },
                Normal {
                    x: -1832,
                    y: 3664,
                    z: 0
                }
            ]
        );
        assert_eq!(b.constants, [0, -351_703, -65_536, -163_840, -147_456]);
        let at = |x: usize, y: usize| {
            let (cx, cz) = tile_center(x, y);
            b.height_at(cx, cz, 0)
        };
        assert_eq!(at(4, 6), Some(16_338));
        assert_eq!(at(5, 6), Some(49_106));
        assert_eq!(at(7, 6), Some(65_536));
        assert_eq!(at(8, 6), Some(163_840));
        assert_eq!(at(6, 8), Some(147_456));
    }

    #[test]
    fn prototype_sizes_are_reproduced() {
        let flat = from_grid(&HeightGrid::flat(0));
        assert_eq!(flat.payload_len(), 50);
        assert_eq!(flat, Bdhc::flat(0));

        let mut route = HeightGrid::flat(0);
        for y in 0..32 {
            for x in 0..32 {
                route.height[y * 32 + x] = match y {
                    0..=9 => 0,
                    10..=21 => 2,
                    _ => 4,
                };
            }
        }
        for x in [6, 7, 20, 21] {
            route.ramp[9 * 32 + x] = Ramp::South(2);
            route.ramp[21 * 32 + x] = Ramp::South(2);
        }
        let b = from_grid(&route);
        assert_eq!(b.payload_len(), 422);
        assert_eq!(counts(&b), (26, 2, 5, 13, 5, 13));

        let mut checker = HeightGrid::flat(0);
        for i in 0..1024 {
            checker.height[i] = (2 * ((i % 32 + i / 32) % 2)) as i8;
        }
        assert_eq!(from_grid(&checker).payload_len(), 19_212);

        let mut own = [FLAT; 1024];
        for (i, p) in own.iter_mut().enumerate() {
            *p = flat_plane(2 * i as i32);
        }
        assert_eq!(from_planes(&own).payload_len(), 23_300);
    }

    #[test]
    fn generated_bdhc_never_exceeds_the_bound() {
        let mut largest = 0;
        for g in sample_grids() {
            assert_eq!(g.check(), None);
            let b = from_grid(&g);
            let payload = b.payload_len();
            assert!(payload <= MAX_PAYLOAD_FROM_GRID, "payload {payload}");
            assert!(payload <= 0x9000);
            assert!(b.validate().is_empty(), "{:?}", b.validate());
            assert_eq!(Bdhc::read(&b.write()).unwrap(), b);
            largest = largest.max(payload);
        }
        println!("largest payload from a grid: {largest} B");
    }

    #[test]
    fn generated_heights_hit_five_points_per_cell() {
        let fx = f64::from(FX_ONE);
        let mut worst = 0.0f64;
        for g in sample_grids() {
            let b = from_grid(&g);
            for i in 0..1024 {
                let plane = g.plane(i);
                let (cx, cz) = tile_center(i % 32, i / 32);
                let off = 4 * FX_ONE;
                for (dx, dz) in [(0, 0), (-off, 0), (off, 0), (0, -off), (0, off)] {
                    let (x, z) = (cx + dx, cz + dz);
                    let want = plane.units_at(f64::from(x) / fx, f64::from(z) / fx);
                    let h = b
                        .height_at(x, z, (want * fx) as i32)
                        .unwrap_or_else(|| panic!("no plate under cell {i} at ({x}, {z})"));
                    let deviation = (f64::from(h) / fx - want).abs();
                    assert!(deviation <= 0.1, "cell {i}: {h} vs {want}");
                    worst = worst.max(deviation);
                }
            }
        }
        println!("largest deviation from the grid: {worst:.4} units");
    }

    #[test]
    fn from_grid_is_deterministic() {
        let grids = sample_grids();
        assert_eq!(from_grid(&grids[17]).write(), from_grid(&grids[17]).write());

        let mut g = HeightGrid::flat(0);
        g.ramp[6 * 32 + 4] = Ramp::East(1);
        g.ramp[6 * 32 + 5] = Ramp::East(1);
        g.height[6 * 32 + 5] = 1;
        assert_eq!(g.plane(6 * 32 + 4), g.plane(6 * 32 + 5));
        let b = from_grid(&g);
        let sloped: Vec<_> = b
            .plates
            .iter()
            .filter(|p| b.normals[usize::from(p.normal)].x != 0)
            .collect();
        assert_eq!(sloped.len(), 1);
        let (a, c) = (
            b.points[usize::from(sloped[0].first)],
            b.points[usize::from(sloped[0].second)],
        );
        assert_eq!((a.x, a.z), (-192 * FX_ONE, -160 * FX_ONE));
        assert_eq!((c.x, c.z), (-160 * FX_ONE, -144 * FX_ONE));
    }

    #[test]
    fn grid_check_names_the_first_bad_cell() {
        let mut g = HeightGrid::flat(0);
        assert_eq!(g.check(), None);
        g.height[32 + 3] = 63;
        g.height[2 * 32] = 70;
        assert_eq!(
            g.check(),
            Some(GridProblem::HeightOutOfRange {
                x: 3,
                y: 1,
                height: 63
            })
        );

        let mut g = HeightGrid::flat(0);
        g.ramp[5 * 32 + 7] = Ramp::East(3);
        assert_eq!(
            g.check(),
            Some(GridProblem::RampSteps {
                x: 7,
                y: 5,
                steps: 3
            })
        );

        let mut g = HeightGrid::flat(0);
        g.height[2] = 62;
        g.ramp[2] = Ramp::North(1);
        assert_eq!(
            g.check(),
            Some(GridProblem::RampTop {
                x: 2,
                y: 0,
                top: 63
            })
        );
        g.height[2] = 61;
        assert_eq!(g.check(), None);

        let mut g = HeightGrid::flat(-16);
        g.ramp[31 * 32 + 31] = Ramp::East(0);
        assert_eq!(
            g.check(),
            Some(GridProblem::RampSteps {
                x: 31,
                y: 31,
                steps: 0
            })
        );
        assert_eq!(HeightGrid::flat(-17).check().map(|_| ()), Some(()));
    }

    #[test]
    fn edges_of_a_ramp() {
        let mut g = HeightGrid::flat(0);
        let i = 10 * 32 + 10;
        g.height[i] = 1;
        g.ramp[i] = Ramp::East(2);
        assert_eq!(g.edge_units(i, Direction::West), 8);
        assert_eq!(g.edge_units(i, Direction::East), 24);
        assert_eq!(g.edge_units(i, Direction::North), 16);
        assert_eq!(g.edge_units(i, Direction::South), 16);
        assert_eq!(g.center_units(i), 16.0);
        assert_eq!(g.edge_units(0, Direction::North), 0);
        assert_eq!(g.center_units(0), 0.0);

        // The plane meets the edges: lower west 8, upper east 24.
        let p = g.plane(i);
        let x_w = f64::from(16 * 10 - 256);
        assert_eq!(p.units_at(x_w, 0.0), 8.0);
        assert_eq!(p.units_at(x_w + 16.0, 0.0), 24.0);
        assert_eq!(p.units_at(x_w + 8.0, 0.0), 16.0);

        assert_eq!(Ramp::East(2).rise(), Some((Direction::East, 2)));
        assert_eq!(Ramp::None.rise(), None);
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::West.opposite(), Direction::East);
        assert_eq!(Direction::North.step(), (0, -1));
        assert_eq!(Direction::East.step(), (1, 0));
        assert_eq!(Direction::South.step(), (0, 1));
        assert_eq!(Direction::West.step(), (-1, 0));
    }

    #[test]
    fn the_four_ramp_planes_meet_their_edges() {
        for (r, d) in [
            (Ramp::North(2), Direction::North),
            (Ramp::East(1), Direction::East),
            (Ramp::South(2), Direction::South),
            (Ramp::West(1), Direction::West),
        ] {
            let mut g = HeightGrid::flat(0);
            let (x, y) = (3usize, 20usize);
            let i = y * 32 + x;
            g.height[i] = 5;
            g.ramp[i] = r;
            let p = g.plane(i);
            let (cx, cz) = (
                f64::from(16 * x as i32 - 248),
                f64::from(16 * y as i32 - 248),
            );
            assert_eq!(p.units_at(cx, cz), g.center_units(i));
            for side in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                let (sx, sz) = side.step();
                let at = p.units_at(cx + 8.0 * f64::from(sx), cz + 8.0 * f64::from(sz));
                assert_eq!(at, f64::from(g.edge_units(i, side)), "{r:?} {side:?}");
            }
            assert_eq!(g.edge_units(i, d.opposite()), 40);
        }
    }
}
