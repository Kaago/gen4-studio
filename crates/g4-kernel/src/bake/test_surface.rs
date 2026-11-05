//! The test surface of Gutachten A §2.7: a full flat map with four floor textures and a block.

use super::Face;

/// Tiles per map side.
const TILES: i32 = 32;
/// World units per tile.
const TILE: i32 = 16;
/// uv of a corner: its x or z plus this (the map edge −256 is texel 0).
const UV_ORIGIN: i32 = 256;

/// The test surface of Gutachten A §2.7: 32 × 32 floor at y 0, texture floor[((tx / 2) + (tz / 2)) % 4],
/// corners x0 = (tx − 16)·16, z0 = (tz − 16)·16, [(x0,0,z0), (x0,0,z0+16), (x0+16,0,z0+16), (x0+16,0,z0)],
/// uv = (x + 256, z + 256); a 2 × 2 × 1 block at `block` (top at y 16, uv like the floor; four sides as
/// in `wall_quads`, s = x + 256 or z + 256 along the wall, t = 16 − y), texture `block_texture`.
pub fn test_surface(floor: [&str; 4], block_texture: &str, block: (u8, u8)) -> Vec<Face> {
    let mut faces = Vec::with_capacity((TILES * TILES) as usize + 8);
    for tz in 0..TILES {
        for tx in 0..TILES {
            let texture = floor[(((tx / 2) + (tz / 2)) % 4) as usize];
            faces.push(tile(tx, tz, 0, texture));
        }
    }
    let (bx, bz) = (i32::from(block.0), i32::from(block.1));
    for (tx, tz) in [(bx, bz), (bx + 1, bz), (bx, bz + 1), (bx + 1, bz + 1)] {
        faces.push(tile(tx, tz, TILE, block_texture));
    }
    // The four sides of `wall_quads`: south (+z), east (+x), north (−z), west (−x); each from
    // its corner a to its corner b, top edge first.
    let (x0, x1) = ((bx - 16) * TILE, (bx + 2 - 16) * TILE);
    let (z0, z1) = ((bz - 16) * TILE, (bz + 2 - 16) * TILE);
    for (a, b) in [
        ((x0, z1), (x1, z1)),
        ((x1, z1), (x1, z0)),
        ((x1, z0), (x0, z0)),
        ((x0, z0), (x0, z1)),
    ] {
        let corners = [
            [a.0, TILE, a.1],
            [a.0, 0, a.1],
            [b.0, 0, b.1],
            [b.0, TILE, b.1],
        ];
        let along_x = a.1 == b.1;
        faces.push(Face {
            corners,
            uv: corners.map(|[x, y, z]| [if along_x { x } else { z } + UV_ORIGIN, TILE - y]),
            texture: block_texture.to_string(),
            layer: 0,
        });
    }
    faces
}

/// The level tile (tx, tz) at height `y`, uv world-bound.
fn tile(tx: i32, tz: i32, y: i32, texture: &str) -> Face {
    let (x0, z0) = ((tx - 16) * TILE, (tz - 16) * TILE);
    let corners = [
        [x0, y, z0],
        [x0, y, z0 + TILE],
        [x0 + TILE, y, z0 + TILE],
        [x0 + TILE, y, z0],
    ];
    Face {
        corners,
        uv: corners.map(|[x, _, z]| [x + UV_ORIGIN, z + UV_ORIGIN]),
        texture: texture.to_string(),
        layer: 0,
    }
}
