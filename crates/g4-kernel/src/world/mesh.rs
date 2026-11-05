//! The mesher of a world (phase 6.3a): [`mesh`] turns a [`WorldGrid`] and its [`Tileset`] into a
//! list of quads ([`MeshFace`]) in world units around the map centre (x and z from −256 to 256,
//! 16 per tile, z grows to the south, y up, `y = step × 8`), the form the scene shows and the
//! baker of 6.3b writes without conversion.
//!
//! Per layer the floors are merged greedily row by row (same tile token, height and ramp; first
//! along x, then whole rows along z; a ramp only across its slope), overlays follow the height
//! and ramp of the ground; between two ground cells whose top edges differ at their shared edge
//! stands a wall from the lower to the upper profile, facing the lower cell, with the `wall`
//! texture of the upper cell's tile (without one, the tile's own); collinear wall pieces of one
//! texture and facing merge. No wall at the map border, next to an empty cell or where the
//! profiles cross. Every piece leaves one footprint face on [`PIECE_LAYER`]. Order: layer
//! ascending; per layer the floors by start cell (row, then column), then the walls (north
//! edges row by row, then west edges column by column); the footprints last, in list order.
//!
//! [`estimate_model_bytes`] is the upper bound of the model size from the face and texture
//! count (quads with `TEXCOORD` and `VTX_16`); the baker of 6.3b gives the real number.

use g4_formats::map::MAP_SIZE;

use super::doc::{STEP_UNITS, TILE_UNITS};
use super::grid::{Ramp, TileRef, WorldError, WorldGrid};
use super::tileset::{TileRole, Tileset};

/// One quad: corners in world units (16 per tile, height = step × 8, y up), counter-clockwise seen from its front
/// (floors from above); UV in texels, world-bound, 16 per tile; `texture` a name of the package (≤ 16 bytes); `layer` the layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshFace {
    pub corners: [[i32; 3]; 4],
    pub uv: [[i32; 2]; 4],
    pub texture: String,
    pub layer: u8,
}

/// The layer of the footprint faces of pieces.
pub const PIECE_LAYER: u8 = 255;
/// The most bytes a map model may have.
pub const MODEL_BUDGET: usize = 0xF000;

/// Cells of one layer.
const CELLS: usize = MAP_SIZE * MAP_SIZE;
/// Half the map in world units: the map spans −256 to 256 in x and z.
const HALF: i32 = MAP_SIZE as i32 * TILE_UNITS / 2;

/// The heights of the four corners of a ground cell in world units, `[nw, ne, sw, se]`.
type Corners = [i32; 4];
const NW: usize = 0;
const NE: usize = 1;
const SW: usize = 2;
const SE: usize = 3;

/// The world coordinate of the cell edge `c` (0 to 32) in x or z.
fn edge(c: usize) -> i32 {
    // `c` ≤ 32.
    c as i32 * TILE_UNITS - HALF
}

/// The corner heights of a ground cell at step `h` with `ramp` (uphill towards the named edge).
fn corners_of(h: i8, ramp: Ramp) -> Corners {
    let low = i32::from(h) * STEP_UNITS;
    let up = |steps: u8| low + i32::from(steps) * STEP_UNITS;
    match ramp {
        Ramp::None => [low; 4],
        Ramp::North(s) => [up(s), up(s), low, low],
        Ramp::South(s) => [low, low, up(s), up(s)],
        Ramp::East(s) => [low, up(s), low, up(s)],
        Ramp::West(s) => [up(s), low, up(s), low],
    }
}

/// The world-bound UV of the floor point (x, z) of a tile turned `rotate` quarter turns and
/// mirrored after the turn with `flip`.
fn uv_of(x: i32, z: i32, rotate: u8, flip: bool) -> [i32; 2] {
    let (u, v) = match rotate % 4 {
        0 => (x, z),
        1 => (z, -x),
        2 => (-x, -z),
        _ => (-z, x),
    };
    [if flip { -u } else { u }, v]
}

/// The horizontal-ish quad over the cells `x0..x1`, `z0..z1` with the corner heights
/// `[nw, ne, sw, se]`, counter-clockwise from above.
fn floor_face(
    (x0, x1): (usize, usize),
    (z0, z1): (usize, usize),
    y: Corners,
    rotate: u8,
    flip: bool,
    texture: &str,
    layer: u8,
) -> MeshFace {
    let (wx0, wx1, wz0, wz1) = (edge(x0), edge(x1), edge(z0), edge(z1));
    let corners = [
        [wx0, y[NW], wz0],
        [wx0, y[SW], wz1],
        [wx1, y[SE], wz1],
        [wx1, y[NE], wz0],
    ];
    MeshFace {
        corners,
        uv: corners.map(|p| uv_of(p[0], p[2], rotate, flip)),
        texture: texture.to_owned(),
        layer,
    }
}

/// The faces of `grid` with the textures of `tileset`; tiles past the tileset are skipped (the check reports them).
pub fn mesh(grid: &WorldGrid, tileset: &Tileset) -> Vec<MeshFace> {
    let corners: Vec<Corners> = grid
        .heights
        .iter()
        .zip(&grid.ramps)
        .map(|(&h, &r)| corners_of(h, r))
        .collect();
    let mut faces = Vec::new();
    // Layer 255 is the footprints'; a world never has that many layers.
    for (index, layer) in grid
        .layers
        .iter()
        .enumerate()
        .take(usize::from(PIECE_LAYER))
    {
        let id = index as u8;
        floors(&layer.tiles, grid, &corners, tileset, id, &mut faces);
        if index == 0 {
            walls(&layer.tiles, &corners, tileset, &mut faces);
        }
    }
    footprints(grid, &corners, tileset, &mut faces);
    faces
}

/// The merged floors of one layer: greedy row by row over (tile token, height, ramp), first
/// along x, then whole rows along z; a ramp only across its slope.
fn floors(
    tiles: &[Option<TileRef>],
    grid: &WorldGrid,
    corners: &[Corners],
    tileset: &Tileset,
    layer: u8,
    faces: &mut Vec<MeshFace>,
) {
    let key = |i: usize| tiles[i].map(|t| (t, grid.heights[i], grid.ramps[i]));
    let mut visited = vec![false; CELLS];
    for z in 0..MAP_SIZE {
        for x in 0..MAP_SIZE {
            let start = z * MAP_SIZE + x;
            if visited[start] {
                continue;
            }
            let Some(t) = tiles[start] else { continue };
            let Some(tile) = tileset.tile(t.tile) else {
                continue;
            };
            let k = key(start);
            let ramp = grid.ramps[start];
            let mut w = 1;
            if matches!(ramp, Ramp::None | Ramp::North(_) | Ramp::South(_)) {
                while x + w < MAP_SIZE && !visited[start + w] && key(start + w) == k {
                    w += 1;
                }
            }
            let mut d = 1;
            if matches!(ramp, Ramp::None | Ramp::East(_) | Ramp::West(_)) {
                while z + d < MAP_SIZE
                    && (start + d * MAP_SIZE..start + d * MAP_SIZE + w)
                        .all(|c| !visited[c] && key(c) == k)
                {
                    d += 1;
                }
            }
            for row in 0..d {
                let from = start + row * MAP_SIZE;
                visited[from..from + w].fill(true);
            }
            let last = start + (d - 1) * MAP_SIZE;
            let y = [
                corners[start][NW],
                corners[start + w - 1][NE],
                corners[last][SW],
                corners[last + w - 1][SE],
            ];
            faces.push(floor_face(
                (x, x + w),
                (z, z + d),
                y,
                t.rotate,
                t.flip,
                &tile.texture,
                layer,
            ));
        }
    }
}

/// Where a wall faces (its front, towards the lower cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Facing {
    North,
    South,
    West,
    East,
}

/// A wall along one edge line from cell `from` to `to` (along x for north edges, along z for
/// west edges): the lower profile `bottom` and the upper `top` at both ends, the tile whose
/// texture it takes.
#[derive(Debug, Clone, Copy)]
struct Run {
    facing: Facing,
    wall: u16,
    from: usize,
    to: usize,
    bottom: [i32; 2],
    top: [i32; 2],
}

impl Run {
    /// Whether the one-cell run `next` continues this one on the same straight bottom and top.
    fn continues(&self, next: &Run) -> bool {
        // At most 32 cells.
        let n = (self.to - self.from) as i32;
        self.facing == next.facing
            && self.wall == next.wall
            && next.from == self.to
            && next.bottom[0] == self.bottom[1]
            && next.top[0] == self.top[1]
            && self.bottom[1] - self.bottom[0] == n * (next.bottom[1] - next.bottom[0])
            && self.top[1] - self.top[0] == n * (next.top[1] - next.top[0])
    }

    /// The vertical quad of the run on the edge line at `line` (z for north edges, x for west
    /// edges), counter-clockwise seen from its front; UV `u` along the edge, `v = −y`.
    fn face(&self, line: i32, tileset: &Tileset) -> Option<MeshFace> {
        let texture = &tileset.tile(self.wall)?.texture;
        let (a, b) = (edge(self.from), edge(self.to));
        let [ba, bb] = self.bottom;
        let [ta, tb] = self.top;
        // (along, y) in front-view order: bottom left, bottom right, top right, top left.
        let (order, along_x) = match self.facing {
            Facing::South => ([(a, ba), (b, bb), (b, tb), (a, ta)], true),
            Facing::North => ([(b, bb), (a, ba), (a, ta), (b, tb)], true),
            Facing::West => ([(a, ba), (b, bb), (b, tb), (a, ta)], false),
            Facing::East => ([(b, bb), (a, ba), (a, ta), (b, tb)], false),
        };
        let corners = order.map(|(s, y)| if along_x { [s, y, line] } else { [line, y, s] });
        Some(MeshFace {
            corners,
            uv: order.map(|(s, y)| [s, -y]),
            texture: texture.clone(),
            layer: 0,
        })
    }
}

/// The wall between ground cells `a` (north or west) and `b` (south or east) on the cells
/// `at..at + 1` of their edge line, with `pa` and `pb` their profiles on the shared edge (in
/// line order): `None` without a height difference, next to an empty cell, where the
/// profiles cross, or when the upper tile or its wall tile is past the tileset.
fn wall_between(
    tiles: &[Option<TileRef>],
    tileset: &Tileset,
    (a, b): (usize, usize),
    (pa, pb): ([i32; 2], [i32; 2]),
    at: usize,
    (towards_a, towards_b): (Facing, Facing),
) -> Option<Run> {
    let (ta, tb) = (tiles[a]?, tiles[b]?);
    let (d0, d1) = (pa[0] - pb[0], pa[1] - pb[1]);
    let (upper, facing, bottom, top) = if d0 == 0 && d1 == 0 {
        return None;
    } else if d0 >= 0 && d1 >= 0 {
        (ta, towards_b, pb, pa)
    } else if d0 <= 0 && d1 <= 0 {
        (tb, towards_a, pa, pb)
    } else {
        return None;
    };
    let wall = tileset.tile(upper.tile)?.wall.unwrap_or(upper.tile);
    tileset.tile(wall)?;
    Some(Run {
        facing,
        wall,
        from: at,
        to: at + 1,
        bottom,
        top,
    })
}

/// The walls of the ground layer: north edges row by row, then west edges column by column;
/// along each line the pieces of one texture, facing and straight profile merge.
fn walls(
    tiles: &[Option<TileRef>],
    corners: &[Corners],
    tileset: &Tileset,
    faces: &mut Vec<MeshFace>,
) {
    let flush = |run: &mut Option<Run>, line: i32, faces: &mut Vec<MeshFace>| {
        if let Some(face) = run.take().and_then(|r| r.face(line, tileset)) {
            faces.push(face);
        }
    };
    for horizontal in [true, false] {
        for line in 1..MAP_SIZE {
            let mut run: Option<Run> = None;
            let at = edge(line);
            for s in 0..MAP_SIZE {
                let piece = if horizontal {
                    // Between (s, line − 1) to the north and (s, line) to the south.
                    let (a, b) = ((line - 1) * MAP_SIZE + s, line * MAP_SIZE + s);
                    let profiles = (
                        [corners[a][SW], corners[a][SE]],
                        [corners[b][NW], corners[b][NE]],
                    );
                    wall_between(
                        tiles,
                        tileset,
                        (a, b),
                        profiles,
                        s,
                        (Facing::North, Facing::South),
                    )
                } else {
                    // Between (line − 1, s) to the west and (line, s) to the east.
                    let (a, b) = (s * MAP_SIZE + line - 1, s * MAP_SIZE + line);
                    let profiles = (
                        [corners[a][NE], corners[a][SE]],
                        [corners[b][NW], corners[b][SW]],
                    );
                    wall_between(
                        tiles,
                        tileset,
                        (a, b),
                        profiles,
                        s,
                        (Facing::West, Facing::East),
                    )
                };
                match (run.as_mut(), piece) {
                    (Some(r), Some(p)) if r.continues(&p) => {
                        r.to = p.to;
                        r.bottom[1] = p.bottom[1];
                        r.top[1] = p.top[1];
                    }
                    (_, piece) => {
                        flush(&mut run, at, faces);
                        run = piece;
                    }
                }
            }
            flush(&mut run, at, faces);
        }
    }
}

/// Width and depth in cells of the footprint `[w, d]` turned `rotate` degrees.
fn turned((w, d): (u8, u8), rotate: u16) -> (usize, usize) {
    let (w, d) = (usize::from(w), usize::from(d));
    if rotate % 180 == 90 { (d, w) } else { (w, d) }
}

/// The footprint of piece tile `tile` turned `rotate` degrees, `None` when the tile is past
/// the tileset, not a piece or has none.
fn footprint_of(tileset: &Tileset, tile: u16, rotate: u16) -> Option<(usize, usize, &str)> {
    let t = tileset.tile(tile).filter(|t| t.role == TileRole::Piece)?;
    let [w, d] = t.footprint?;
    let (w, d) = turned((w, d), rotate);
    Some((w, d, &t.texture))
}

/// One horizontal face per piece over its footprint (clipped to the map) at the highest
/// corner of the ground cells below it; a footprint of width or depth 0 leaves none.
fn footprints(grid: &WorldGrid, corners: &[Corners], tileset: &Tileset, faces: &mut Vec<MeshFace>) {
    for piece in &grid.pieces {
        let Some((w, d, texture)) = footprint_of(tileset, piece.tile, piece.rotate) else {
            continue;
        };
        let [x0, z0] = piece.at.map(usize::from);
        let (x1, z1) = ((x0 + w).min(MAP_SIZE), (z0 + d).min(MAP_SIZE));
        if x1 <= x0 || z1 <= z0 {
            continue;
        }
        let y = (z0..z1)
            .flat_map(|z| (x0..x1).map(move |x| z * MAP_SIZE + x))
            .flat_map(|c| corners[c])
            .max()
            .unwrap_or(0);
        // `rotate` is 0 to 270 (the grid check), so the turn is 0 to 3.
        let turn = (piece.rotate / 90 % 4) as u8;
        faces.push(floor_face(
            (x0, x1),
            (z0, z1),
            [y; 4],
            turn,
            false,
            texture,
            PIECE_LAYER,
        ));
    }
}

/// The distinct texture names of `faces`, in order of first use.
pub fn mesh_textures(faces: &[MeshFace]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for f in faces {
        if !names.contains(&f.texture) {
            names.push(f.texture.clone());
        }
    }
    names
}

/// The estimated size of a map model with `faces` quads and `textures` textures, an upper
/// bound: `1024 + 600 · textures + 56 · faces`.
pub fn estimate_model_bytes(faces: usize, textures: usize) -> usize {
    1024 + 600 * textures + 56 * faces
}

/// Pieces whose footprint leaves the map: `world.pieces[i]` with „Das Teilmodell ragt über den Kartenrand (x 31, Breite 2).“
pub fn piece_problems(grid: &WorldGrid, tileset: &Tileset) -> Vec<WorldError> {
    let mut problems = Vec::new();
    for (i, piece) in grid.pieces.iter().enumerate() {
        let Some((w, d, _)) = footprint_of(tileset, piece.tile, piece.rotate) else {
            continue;
        };
        if w == 0 || d == 0 {
            continue;
        }
        let [x, z] = piece.at.map(usize::from);
        let mut sides = Vec::new();
        if x + w > MAP_SIZE {
            sides.push(format!("x {x}, Breite {w}"));
        }
        if z + d > MAP_SIZE {
            sides.push(format!("z {z}, Tiefe {d}"));
        }
        if !sides.is_empty() {
            problems.push(WorldError {
                path: format!("world.pieces[{i}]"),
                message: format!(
                    "Das Teilmodell ragt über den Kartenrand ({}).",
                    sides.join("; ")
                ),
            });
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::super::doc::{LayerRole, PieceJson, fixtures};
    use super::super::grid::GridLayer;
    use super::*;

    const N: usize = MAP_SIZE;

    fn grid_of(world: super::super::doc::WorldJson) -> WorldGrid {
        WorldGrid::parse(&world).unwrap()
    }

    fn flat() -> WorldGrid {
        grid_of(fixtures::one_tile_world())
    }

    fn at(x: usize, z: usize) -> usize {
        z * N + x
    }

    fn tile(tile: u16, rotate: u8, flip: bool) -> Option<TileRef> {
        Some(TileRef { tile, rotate, flip })
    }

    /// The (not normalised) normal of `face` by Newell's method.
    fn normal(face: &MeshFace) -> [i64; 3] {
        let c = face.corners.map(|p| p.map(i64::from));
        let mut n = [0i64; 3];
        for i in 0..4 {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        n
    }

    fn is_wall(face: &MeshFace) -> bool {
        normal(face)[1] == 0
    }

    fn walls(faces: &[MeshFace]) -> Vec<&MeshFace> {
        faces.iter().filter(|f| is_wall(f)).collect()
    }

    /// Length along the ground and height of a wall face.
    fn wall_size(face: &MeshFace) -> (i32, i32) {
        let c = &face.corners;
        let span = |k: usize| {
            let (lo, hi) = (c.iter().map(|p| p[k]).min(), c.iter().map(|p| p[k]).max());
            hi.unwrap() - lo.unwrap()
        };
        (span(0) + span(2), span(1))
    }

    /// The faces whose corners all lie in the cells `x0..x1`, `z0..z1`.
    fn inside(faces: &[MeshFace], x: (usize, usize), z: (usize, usize)) -> Vec<&MeshFace> {
        let e = |c: usize| c as i32 * 16 - 256;
        faces
            .iter()
            .filter(|f| {
                f.corners
                    .iter()
                    .all(|p| (e(x.0)..=e(x.1)).contains(&p[0]) && (e(z.0)..=e(z.1)).contains(&p[2]))
            })
            .collect()
    }

    #[test]
    fn one_tile_map_meshes_to_one_face() {
        let faces = mesh(&flat(), &Tileset::synthetic());
        assert!(faces.len() <= 4, "gate: one tile ≤ 4 faces");
        assert_eq!(
            faces,
            [MeshFace {
                corners: [
                    [-256, 0, -256],
                    [-256, 0, 256],
                    [256, 0, 256],
                    [256, 0, -256]
                ],
                uv: [[-256, -256], [-256, 256], [256, 256], [256, -256]],
                texture: "syn_boden".to_owned(),
                layer: 0,
            }]
        );
    }

    #[test]
    fn checkerboard_meshes_to_1024_faces() {
        let faces = mesh(
            &grid_of(fixtures::checkerboard_world()),
            &Tileset::synthetic(),
        );
        assert!(faces.len() <= 1024, "gate: checkerboard ≤ 1024 faces");
        assert_eq!(faces.len(), 1024);
    }

    #[test]
    fn floor_corners_are_counter_clockwise_from_above() {
        let faces = mesh(&grid_of(fixtures::busy_world()), &Tileset::synthetic());
        let floors: Vec<_> = faces.iter().filter(|f| !is_wall(f)).collect();
        assert!(floors.len() > 100);
        for f in floors {
            let c = f.corners.map(|p| p.map(i64::from));
            let (a, b) = (
                [c[1][0] - c[0][0], c[1][1] - c[0][1], c[1][2] - c[0][2]],
                [c[2][0] - c[0][0], c[2][1] - c[0][1], c[2][2] - c[0][2]],
            );
            let cross_y = a[2] * b[0] - a[0] * b[2];
            assert!(cross_y > 0, "{f:?}");
        }
    }

    /// The flat map with a 2 × 2 block of `block` at height 2 on cells (10..12, 10..12).
    fn raised_block(block: u16) -> WorldGrid {
        let mut g = flat();
        for (x, z) in [(10, 10), (11, 10), (10, 11), (11, 11)] {
            g.heights[at(x, z)] = 2;
            g.layers[0].tiles[at(x, z)] = tile(block, 0, false);
        }
        g
    }

    #[test]
    fn a_raised_block_gets_four_merged_walls() {
        let faces = mesh(&raised_block(0), &Tileset::synthetic());
        let walls = walls(&faces);
        assert_eq!(walls.len(), 4, "{walls:?}");
        let mut directions = Vec::new();
        for w in &walls {
            assert_eq!(wall_size(w), (32, 16));
            assert_eq!(w.texture, "syn_wand");
            assert_eq!(w.layer, 0);
            let n = normal(w);
            // Away from the block centre (x = z = −80), towards the lower side.
            let mid = w
                .corners
                .iter()
                .fold([0, 0], |m, p| [m[0] + p[0], m[1] + p[2]]);
            let out = [i64::from(mid[0] / 4 + 80), i64::from(mid[1] / 4 + 80)];
            assert!(n[0] * out[0] + n[2] * out[1] > 0, "{w:?}");
            directions.push((n[0].signum(), n[2].signum()));
        }
        directions.sort();
        assert_eq!(directions, [(-1, 0), (0, -1), (0, 1), (1, 0)]);
    }

    #[test]
    fn walls_use_the_upper_tile_without_a_wall_tile() {
        let faces = mesh(&raised_block(1), &Tileset::synthetic());
        let walls = walls(&faces);
        assert_eq!(walls.len(), 4);
        assert!(walls.iter().all(|w| w.texture == "syn_gras"), "{walls:?}");
    }

    #[test]
    fn walls_face_the_lower_cell() {
        let mut g = flat();
        g.heights[at(10, 10)] = 3;
        g.heights[at(20, 20)] = -2;
        // A step running across the map: rows 25.. one step higher.
        for z in 25..N {
            for x in 0..N {
                g.heights[at(x, z)] = 1;
            }
        }
        let faces = mesh(&g, &Tileset::synthetic());
        let walls = walls(&faces);
        // 4 around the hill, 4 around the pit, 1 long step.
        assert_eq!(walls.len(), 9, "{walls:?}");
        for w in walls {
            let n = normal(w);
            let len = ((n[0] * n[0] + n[2] * n[2]) as f64).sqrt();
            let (mx, mz) = w.corners.iter().fold((0.0, 0.0), |m, p| {
                (m.0 + f64::from(p[0]) / 4.0, m.1 + f64::from(p[2]) / 4.0)
            });
            let cell = |x: f64, z: f64| {
                let cx = ((x + 256.0) / 16.0).floor() as usize;
                let cz = ((z + 256.0) / 16.0).floor() as usize;
                g.heights[at(cx, cz)]
            };
            let (fx, fz) = (n[0] as f64 / len * 4.0, n[2] as f64 / len * 4.0);
            let front = cell(mx + fx, mz + fz);
            let back = cell(mx - fx, mz - fz);
            assert!(front < back, "{w:?}: front {front}, back {back}");
            let top = w.corners.iter().map(|p| p[1]).max().unwrap();
            let bottom = w.corners.iter().map(|p| p[1]).min().unwrap();
            assert_eq!((bottom, top), (i32::from(front) * 8, i32::from(back) * 8));
        }
        let step = mesh(&g, &Tileset::synthetic())
            .into_iter()
            .filter(|f| is_wall(f) && wall_size(f).0 == 512)
            .count();
        assert_eq!(step, 1, "the step is one merged wall");
    }

    #[test]
    fn ramp_north_one_step_rises_to_the_north() {
        let mut g = flat();
        g.heights[at(5, 5)] = 1;
        g.ramps[at(5, 5)] = Ramp::North(1);
        let faces = mesh(&g, &Tileset::synthetic());
        let ramp: Vec<_> = inside(&faces, (5, 6), (5, 6))
            .into_iter()
            .filter(|f| !is_wall(f))
            .collect();
        assert_eq!(ramp.len(), 1, "{ramp:?}");
        for p in ramp[0].corners {
            match p[2] {
                -160 => assert_eq!(p[1], 8, "south edge"),
                -176 => assert_eq!(p[1], 16, "north edge"),
                z => panic!("corner z {z}"),
            }
        }
    }

    #[test]
    fn ramps_merge_only_across_the_slope() {
        let sloped =
            |f: &&MeshFace| !is_wall(f) && f.corners.iter().any(|p| p[1] != f.corners[0][1]);

        let mut g = flat();
        for x in 4..8 {
            g.ramps[at(x, 5)] = Ramp::North(1);
        }
        let faces = mesh(&g, &Tileset::synthetic());
        let ramps: Vec<_> = faces.iter().filter(sloped).collect();
        assert_eq!(ramps.len(), 1, "{ramps:?}");
        assert_eq!(ramps[0].corners[0][0], -192);
        assert_eq!(ramps[0].corners[2][0], -128);

        let mut g = flat();
        g.heights[at(4, 4)] = 1;
        g.ramps[at(4, 4)] = Ramp::North(1);
        g.ramps[at(4, 5)] = Ramp::North(1);
        let faces = mesh(&g, &Tileset::synthetic());
        assert_eq!(faces.iter().filter(sloped).count(), 2);

        // The same along the slope with equal heights stays two faces.
        let mut g = flat();
        g.ramps[at(4, 4)] = Ramp::East(1);
        g.ramps[at(5, 4)] = Ramp::East(1);
        g.ramps[at(6, 4)] = Ramp::East(1);
        g.ramps[at(6, 5)] = Ramp::East(1);
        let faces = mesh(&g, &Tileset::synthetic());
        // Column 6 merges across the slope (z 4–5), columns 4 and 5 stay alone.
        assert_eq!(faces.iter().filter(sloped).count(), 3);
    }

    #[test]
    fn rotation_and_flip_turn_the_uv() {
        let mut g = flat();
        g.layers[0].tiles[at(0, 0)] = tile(0, 1, false);
        let faces = mesh(&g, &Tileset::synthetic());
        let f = &faces[0];
        assert_eq!(
            f.corners,
            [
                [-256, 0, -256],
                [-256, 0, -240],
                [-240, 0, -240],
                [-240, 0, -256]
            ]
        );
        for (p, uv) in f.corners.iter().zip(f.uv) {
            assert_eq!(uv, [p[2], -p[0]]);
        }

        let mut g = flat();
        g.layers[0].tiles[at(0, 0)] = tile(0, 2, true);
        let faces = mesh(&g, &Tileset::synthetic());
        let f = &faces[0];
        for (p, uv) in f.corners.iter().zip(f.uv) {
            assert_eq!(uv, [p[0], -p[2]]);
        }

        let mut g = flat();
        g.layers[0].tiles[at(0, 0)] = tile(0, 3, false);
        let f = &mesh(&g, &Tileset::synthetic())[0];
        for (p, uv) in f.corners.iter().zip(f.uv) {
            assert_eq!(uv, [-p[2], p[0]]);
        }
    }

    fn overlay(cells: &[(usize, usize, u16)]) -> GridLayer {
        let mut tiles = vec![None; N * N];
        for &(x, z, t) in cells {
            tiles[at(x, z)] = tile(t, 0, false);
        }
        GridLayer {
            name: "Deko".to_owned(),
            role: LayerRole::Overlay,
            tiles,
        }
    }

    #[test]
    fn overlay_layers_follow_the_ground_height() {
        let mut g = raised_block(0);
        g.layers.push(overlay(&[(10, 10, 2)]));
        let faces = mesh(&g, &Tileset::synthetic());
        let over: Vec<_> = faces.iter().filter(|f| f.layer == 1).collect();
        assert_eq!(over.len(), 1, "{over:?}");
        assert_eq!(over[0].texture, "syn_wasser");
        assert!(over[0].corners.iter().all(|p| p[1] == 16));
        assert!(normal(over[0])[1] > 0);
        // The walls are those of the ground only.
        assert_eq!(walls(&faces).len(), 4);
        assert!(walls(&faces).iter().all(|w| w.layer == 0));
        // Layers ascend.
        assert_eq!(faces.last().unwrap().layer, 1);

        // On a ramp the overlay takes its slope.
        let mut g = flat();
        g.ramps[at(3, 3)] = Ramp::East(2);
        g.layers.push(overlay(&[(3, 3, 1)]));
        let faces = mesh(&g, &Tileset::synthetic());
        let over: Vec<_> = faces.iter().filter(|f| f.layer == 1).collect();
        assert_eq!(over.len(), 1);
        for p in over[0].corners {
            assert_eq!(p[1], if p[0] == -192 { 16 } else { 0 }, "{p:?}");
        }
    }

    #[test]
    fn pieces_leave_a_footprint_face() {
        let mut g = flat();
        g.pieces.push(PieceJson {
            tile: 5,
            at: [3, 4],
            rotate: 0,
        });
        let faces = mesh(&g, &Tileset::synthetic());
        assert_eq!(faces.len(), 2);
        let f = faces.last().unwrap();
        assert_eq!(f.layer, PIECE_LAYER);
        assert_eq!(f.texture, "syn_stueck");
        assert_eq!(
            f.corners,
            [
                [-208, 0, -192],
                [-208, 0, -160],
                [-176, 0, -160],
                [-176, 0, -192]
            ]
        );
        assert!(normal(f)[1] > 0);
        assert!(piece_problems(&g, &Tileset::synthetic()).is_empty());
    }

    #[test]
    fn a_turned_piece_swaps_its_footprint_and_sits_on_the_highest_cell() {
        let mut t = Tileset::synthetic();
        t.tiles[5].footprint = Some([3, 1]);
        let mut g = flat();
        g.heights[at(3, 6)] = 4;
        g.pieces.push(PieceJson {
            tile: 5,
            at: [3, 4],
            rotate: 90,
        });
        // Past the tileset and not a piece: skipped (the check reports them).
        g.pieces.push(PieceJson {
            tile: 0x40,
            at: [0, 0],
            rotate: 0,
        });
        g.pieces.push(PieceJson {
            tile: 1,
            at: [0, 0],
            rotate: 0,
        });
        let faces = mesh(&g, &t);
        let pieces: Vec<_> = faces.iter().filter(|f| f.layer == PIECE_LAYER).collect();
        assert_eq!(pieces.len(), 1);
        assert_eq!(
            pieces[0].corners,
            [
                [-208, 32, -192],
                [-208, 32, -144],
                [-192, 32, -144],
                [-192, 32, -192]
            ]
        );
        assert!(piece_problems(&g, &t).is_empty());
    }

    #[test]
    fn empty_cells_have_no_face_and_no_wall() {
        let mut g = flat();
        g.layers[0].tiles[at(7, 7)] = None;
        g.heights[at(7, 7)] = 5;
        g.heights[at(8, 7)] = 3;
        let faces = mesh(&g, &Tileset::synthetic());
        // Nothing lies on or touches the empty cell from inside it.
        let e = |c: i32| c * 16 - 256;
        let covers = |f: &&MeshFace| {
            let xs = f.corners.iter().map(|p| p[0]);
            let zs = f.corners.iter().map(|p| p[2]);
            let (x0, x1) = (xs.clone().min().unwrap(), xs.max().unwrap());
            let (z0, z1) = (zs.clone().min().unwrap(), zs.max().unwrap());
            !is_wall(f) && x0 < e(8) && x1 > e(7) && z0 < e(8) && z1 > e(7)
        };
        assert_eq!(faces.iter().filter(covers).count(), 0);
        // Cell (8, 7) at 3 has walls on north, south and east; none towards the empty cell.
        let walls = walls(&faces);
        assert_eq!(walls.len(), 3, "{walls:?}");
        assert!(walls.iter().all(|w| w.corners.iter().any(|p| p[0] != e(8))));
    }

    #[test]
    fn edges_of_the_grid_mesh_without_panic() {
        let mut g = flat();
        let ramps = [Ramp::North(1), Ramp::East(2), Ramp::South(1), Ramp::West(2)];
        for i in 0..N {
            for (k, c) in [at(i, 0), at(i, N - 1), at(0, i), at(N - 1, i)]
                .into_iter()
                .enumerate()
            {
                g.ramps[c] = ramps[(i + k) % 4];
                g.heights[c] = 62;
            }
        }
        g.heights[at(10, 10)] = -16;
        g.heights[at(11, 10)] = 62;
        // Opposite ramps side by side: the profiles cross on the edge z = 11.
        g.ramps[at(15, 10)] = Ramp::East(1);
        g.ramps[at(15, 11)] = Ramp::West(1);
        let mut t = Tileset::synthetic();
        // A piece at the corner leaving the map, and a footprint of width 0.
        t.tiles.push(super::super::tileset::TileJson {
            texture: "syn_flach".to_owned(),
            footprint: Some([0, 3]),
            ..t.tiles[5].clone()
        });
        g.pieces.push(PieceJson {
            tile: 5,
            at: [31, 31],
            rotate: 0,
        });
        g.pieces.push(PieceJson {
            tile: 6,
            at: [31, 31],
            rotate: 90,
        });
        let faces = mesh(&g, &t);
        // No wall on the map border.
        assert!(walls(&faces).len() > 100);
        for w in walls(&faces) {
            let plane = |k: usize| w.corners.iter().all(|p| p[k] == w.corners[0][k]);
            let on_border = (plane(0) && w.corners[0][0].abs() == 256)
                || (plane(2) && w.corners[0][2].abs() == 256);
            assert!(!on_border, "{w:?}");
        }
        // −16 next to 62: one wall of 78 steps.
        assert!(walls(&faces).iter().any(|w| wall_size(w) == (16, 78 * 8)));
        // No wall where the profiles cross.
        let z = 11 * 16 - 256;
        let x = 15 * 16 - 256;
        assert!(
            !walls(&faces)
                .iter()
                .any(|w| w.corners.iter().all(|p| p[2] == z)
                    && w.corners.iter().all(|p| (x..=x + 16).contains(&p[0]))),
            "a wall on the crossing edge"
        );
        // The corner piece is clipped to its cell; the flat one leaves no face.
        let pieces: Vec<_> = faces.iter().filter(|f| f.layer == PIECE_LAYER).collect();
        assert_eq!(pieces.len(), 1);
        assert_eq!(wall_size(pieces[0]).0, 32);
        assert_eq!(
            piece_problems(&g, &t),
            [WorldError {
                path: "world.pieces[0]".to_owned(),
                message: "Das Teilmodell ragt über den Kartenrand (x 31, Breite 2; z 31, Tiefe 2)."
                    .to_owned(),
            }]
        );
    }

    #[test]
    fn piece_problems_name_the_side_that_leaves() {
        let mut g = flat();
        g.pieces.push(PieceJson {
            tile: 5,
            at: [31, 4],
            rotate: 0,
        });
        g.pieces.push(PieceJson {
            tile: 5,
            at: [4, 31],
            rotate: 270,
        });
        g.pieces.push(PieceJson {
            tile: 5,
            at: [30, 30],
            rotate: 180,
        });
        let problems = piece_problems(&g, &Tileset::synthetic());
        let texts: Vec<_> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "world.pieces[0]: Das Teilmodell ragt über den Kartenrand (x 31, Breite 2).",
                "world.pieces[1]: Das Teilmodell ragt über den Kartenrand (z 31, Tiefe 2).",
            ]
        );
    }

    #[test]
    fn mesh_is_deterministic() {
        let g = grid_of(fixtures::busy_world());
        let t = Tileset::synthetic();
        let first = mesh(&g, &t);
        assert_eq!(first, mesh(&g, &t));
        assert!(first.len() > 1024);
        // Layers ascend.
        assert!(first.windows(2).all(|w| w[0].layer <= w[1].layer));
    }

    #[test]
    fn mesh_textures_in_first_use_order() {
        let faces = mesh(&grid_of(fixtures::busy_world()), &Tileset::synthetic());
        assert_eq!(
            mesh_textures(&faces),
            ["syn_boden", "syn_gras", "syn_wand", "syn_wasser"]
        );
        let face = |t: &str| MeshFace {
            corners: [[0; 3]; 4],
            uv: [[0; 2]; 4],
            texture: t.to_owned(),
            layer: 0,
        };
        assert_eq!(
            mesh_textures(&[face("b"), face("a"), face("b"), face("c"), face("a")]),
            ["b", "a", "c"]
        );
        assert!(mesh_textures(&[]).is_empty());
    }

    #[test]
    fn estimate_matches_the_flat_field_of_gutachten_a() {
        let estimate = estimate_model_bytes(1024, 1);
        assert_eq!(estimate, 58_968);
        assert!(estimate >= 57_776);
        assert!(estimate as f64 <= 57_776.0 * 1.05);
        assert_eq!(estimate_model_bytes(1, 1), 1_680);
        assert!(estimate < MODEL_BUDGET);
    }

    #[test]
    fn tiles_past_the_tileset_are_skipped() {
        let mut g = flat();
        g.layers[0].tiles[at(2, 2)] = tile(0x40, 0, false);
        g.heights[at(2, 2)] = 2;
        let faces = mesh(&g, &Tileset::synthetic());
        // No floor and no wall of the unknown cell (its texture is unknown).
        assert!(walls(&faces).is_empty(), "{:?}", walls(&faces));
        assert!(faces.iter().all(|f| f.texture == "syn_boden"));
        let area: i32 = faces
            .iter()
            .map(|f| (f.corners[2][0] - f.corners[0][0]) * (f.corners[1][2] - f.corners[0][2]))
            .sum();
        assert_eq!(area, 512 * 512 - 16 * 16);
    }
}
