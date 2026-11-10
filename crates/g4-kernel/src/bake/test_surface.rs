//! The test surface of Gutachten A §2.7: a full flat map with four floor textures and a block,
//! the synthetic sample images of the prototype, and `bake_test`, the gate tool that bakes the
//! surface into a map with two own textures appended to its map texture pack.

use std::path::PathBuf;

use g4_formats::game::Family;
use g4_formats::nsbmd::Nsbmd;
use g4_formats::nsbmd::model::ModelHead;
use g4_formats::nsbtx::{Nsbtx, Rgba};
use g4_formats::texconv::{Colors, convert};

use super::textures::appended_pack;
use super::{
    AreaStyle, BakeInput, BakeReport, Face, append_label, append_textures, area_vram, bake,
    bake_context, pack_textures,
};
use crate::error::{KernelError, io_at};
use crate::map_import::import_model_bytes;
use crate::project::{AssetId, AssetKind};
use crate::store::Store;

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

/// A synthetic image with the colour count it is converted to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleImage {
    pub name: &'static str,
    pub image: Rgba,
    pub colors: Colors,
}

/// A `width` × `height` image from `pixel(x, y)`.
fn image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Rgba {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&pixel(x, y));
        }
    }
    Rgba {
        width,
        height,
        pixels,
    }
}

/// The value `v` (below 256 by construction) as a channel.
fn ch(v: u32) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// The four synthetic images of the prototype (`synth_images`): syn_gras 32 × 32, syn_zaun 16 × 16 with
/// transparent rows 0–3 (color0), syn_weg 8 × 8 four colours, syn_himmel 64 × 64 gradient.
pub fn sample_images() -> [SampleImage; 4] {
    let gras = image(32, 32, |x, y| {
        let n = ((x * 7 + y * 13) ^ (x * y)) % 23;
        [ch(40 + n * 3), ch(120 + y * 3), ch(30 + x * 2), 255]
    });
    let zaun = image(16, 16, |x, y| {
        if (x + y) % 5 == 0 || x == 0 || x == 15 {
            [150, 90, 40, 255]
        } else if y < 4 {
            [0, 0, 0, 0]
        } else {
            [200, 160, ch(100 + y * 4), 255]
        }
    });
    let weg = image(8, 8, |x, y| {
        let c = [[90, 80, 70], [130, 120, 100], [180, 170, 150], [60, 50, 40]]
            [((x / 2 + y) % 4) as usize];
        [c[0], c[1], c[2], 255]
    });
    let himmel = image(64, 64, |x, y| {
        [ch((x * 4) % 256), ch((y * 4) % 256), 200, 255]
    });
    [
        ("syn_gras", gras, Colors::Sixteen),
        ("syn_zaun", zaun, Colors::Sixteen),
        ("syn_weg", weg, Colors::Four),
        ("syn_himmel", himmel, Colors::Sixteen),
    ]
    .map(|(name, image, colors)| SampleImage {
        name,
        image,
        colors,
    })
}

/// Name of the first own texture of `bake_test` (16 colours; floor and block).
pub const BAKE_TEXTURE_A: &str = "g4_bake_a";
/// Name of the second own texture of `bake_test` (4 colours; floor).
pub const BAKE_TEXTURE_B: &str = "g4_bake_b";

/// g4_bake_a: 16 × 16, 16 colours, 4-pixel checker of (230, 120, 40) and (40, 90, 200) with a 1-pixel
/// border (20, 20, 20); g4_bake_b: 16 × 16, 4 colours, diagonal stripes ((x + y) / 4) % 4 of four greens.
pub fn bake_textures() -> [SampleImage; 2] {
    let a = image(16, 16, |x, y| {
        if x == 0 || y == 0 || x == 15 || y == 15 {
            [20, 20, 20, 255]
        } else if (x / 4 + y / 4) % 2 == 0 {
            [230, 120, 40, 255]
        } else {
            [40, 90, 200, 255]
        }
    });
    const GREENS: [[u8; 3]; 4] = [[30, 90, 30], [60, 130, 50], [100, 170, 70], [150, 210, 110]];
    let b = image(16, 16, |x, y| {
        let [r, g, b] = GREENS[(((x + y) / 4) % 4) as usize];
        [r, g, b, 255]
    });
    [
        SampleImage {
            name: BAKE_TEXTURE_A,
            image: a,
            colors: Colors::Sixteen,
        },
        SampleImage {
            name: BAKE_TEXTURE_B,
            image: b,
            colors: Colors::Four,
        },
    ]
}

/// What `bake_test` bakes: map `map`; the block's tile (x, z) or, `None`, the middle of the old
/// model's frame; with `out_dir` the model and the pack are also written there.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct BakeTestOptions {
    pub map: u16,
    pub block: Option<(u8, u8)>,
    pub out_dir: Option<PathBuf>,
}

/// What `bake_test` did: the map, its pack and area (the first header that shows the map) and
/// style, the textures appended (empty when the pack already had both), the floor textures
/// and the block's texture and tile, the bake report, the files under `out_dir` and the
/// assets changed (map texture pack first when appended, then the map; empty when nothing
/// changed).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BakeTestReport {
    pub map: u16,
    pub pack: u16,
    pub area: u8,
    pub style: AreaStyle,
    pub appended: Vec<String>,
    pub floor: [String; 4],
    pub block_texture: String,
    pub block: (u8, u8),
    pub report: BakeReport,
    pub model_file: Option<PathBuf>,
    pub pack_file: Option<PathBuf>,
    pub changed: Vec<AssetId>,
}

/// Largest block tile: the block is 2 × 2 tiles.
const BLOCK_MAX: u8 = (TILES - 2) as u8;
/// The block when the old model's frame cannot be read.
const BLOCK_FALLBACK: (u8, u8) = (15, 15);

/// The undo label of `bake_test` on map `map`: „Test surface baked: Map 0187“.
pub fn bake_test_label(map: u16) -> String {
    t!("bake.test_label", map = map : {:04}).into_owned()
}

/// Bakes the test surface into map `opts.map` (plan, „Entscheidungen“ `bake_test`).
///
/// Pack and area come from the first header that shows the map. `g4_bake_a` (16 colours) and
/// `g4_bake_b` (4 colours) are appended to the pack unless it already has a texture of that
/// name (then that one is used). Floor `[g4_bake_a, V0, g4_bake_b, V1]`, `V0`, `V1` the first
/// two other textures of the pack with a recognisable palette (missing ones: `g4_bake_b`, then
/// `g4_bake_a`); the block of `g4_bake_a` at `opts.block` or at the middle of the old model's
/// frame (`cx = (tx0 + tx1) / 2`, `cz = (tz0 + tz1) / 2 + 1`, held to 0..=30; unreadable:
/// (15, 15)). Every check and the bake run before anything is written: a refusal of the bake
/// or of appending is `KernelError::Refused("<code>: <message>")`, a block past 30 is refused.
/// With `out_dir`, `bake-NNNN.nsbmd` (map number) and `maptex-NNNN.nsbtx` (pack number) are
/// written there first. Appending and the model import are then one undo step
/// [`bake_test_label`]; when the import fails the append is rolled back (no redo entry). The
/// report's messages are the bake's plus the `bake.vram-high` warnings of appending for the
/// other areas that share the pack (the own area's is the bake's warning, not repeated).
pub fn bake_test(store: &mut Store, opts: &BakeTestOptions) -> Result<BakeTestReport, KernelError> {
    let map = opts.map;
    if store.project().family == Family::Dp {
        return Err(KernelError::Refused(
            "Diamond/Pearl: the test surface is not baked without a test project".into(),
        ));
    }
    if let Some(why) = store.busy() {
        return Err(KernelError::Busy(why.to_owned()));
    }
    store.project().editable()?;
    if let Some((x, z)) = opts.block
        && (x > BLOCK_MAX || z > BLOCK_MAX)
    {
        return Err(KernelError::Refused(format!(
            "the block at ({x}, {z}) leaves the map: its tile must lie in 0..={BLOCK_MAX}"
        )));
    }
    let ctx = bake_context(store, map)?;
    let pack = ctx.pack;
    let before = store.project().map_tileset_member(pack)?.ok_or_else(|| {
        KernelError::Refused(format!("map texture pack {pack:04} is not in the archive"))
    })?;
    let in_pack: Vec<String> = Nsbtx::read(&before)?
        .textures
        .into_iter()
        .map(|t| t.name)
        .collect();
    let mut new = Vec::new();
    for s in bake_textures() {
        if !in_pack.iter().any(|n| n == s.name) {
            new.push(convert(s.name, &s.image, s.colors)?.texture);
        }
    }
    let (after, textures, vram_bytes) = if new.is_empty() {
        (before.clone(), ctx.textures.clone(), ctx.vram_bytes)
    } else {
        let after = appended_pack(&before, &new)?;
        let textures = pack_textures(&after)?;
        let vram = area_vram(store, ctx.area, Some(&after))?;
        (after, textures, vram)
    };

    let mut others = ctx
        .textures
        .iter()
        .map(|t| t.name.as_str())
        .filter(|n| *n != BAKE_TEXTURE_A && *n != BAKE_TEXTURE_B);
    let v0 = others.next().unwrap_or(BAKE_TEXTURE_B);
    let v1 = others.next().unwrap_or(BAKE_TEXTURE_A);
    let floor = [BAKE_TEXTURE_A, v0, BAKE_TEXTURE_B, v1];
    let block = match opts.block {
        Some(b) => b,
        None => frame_block(&store.map(map)?.model).unwrap_or(BLOCK_FALLBACK),
    };
    let faces = test_surface(floor, BAKE_TEXTURE_A, block);
    let mut report = bake(
        &faces,
        &BakeInput {
            style: ctx.style,
            textures: &textures,
            vram_bytes,
            vram_limit: ctx.vram_limit,
        },
    );
    if let Some(m) = report.refusal() {
        return Err(KernelError::Refused(format!(
            "{}: {}",
            m.code.as_str(),
            m.message
        )));
    }

    let (mut model_file, mut pack_file) = (None, None);
    if let Some(dir) = &opts.out_dir {
        std::fs::create_dir_all(dir).map_err(io_at(dir))?;
        let model_path = dir.join(format!("bake-{map:04}.nsbmd"));
        std::fs::write(&model_path, &report.nsbmd).map_err(io_at(&model_path))?;
        let pack_path = dir.join(format!("maptex-{pack:04}.nsbtx"));
        std::fs::write(&pack_path, &after).map_err(io_at(&pack_path))?;
        model_file = Some(model_path);
        pack_file = Some(pack_path);
    }

    let label = bake_test_label(map);
    let mark = store.undo_depth();
    let mut changed = Vec::new();
    if !new.is_empty() {
        let appended = append_textures(store, pack, &new, &append_label(pack, new.len()))?;
        // `bake.vram-high` of the other areas sharing the pack; the own area's warning is
        // the bake's (same VRAM, same limit), so the append's one for it is left out.
        let own = ctx.area.to_string();
        report.messages.extend(
            appended
                .messages
                .into_iter()
                .filter(|m| m.args.first() != Some(&own)),
        );
        changed.push(AssetId {
            kind: AssetKind::MapTexture,
            index: pack,
        });
    }
    match import_model_bytes(store, map, report.nsbmd.clone(), &label) {
        Ok(ids) => changed.extend(ids),
        Err(e) => {
            // Rolled back, not undone: the failed batch leaves no redo entry.
            if let Err(u) = store.rollback_to(mark) {
                return Err(KernelError::Refused(format!(
                    "{e}; undoing the appended textures failed too: {u}"
                )));
            }
            return Err(e);
        }
    }
    store.squash_since(mark, &label);
    Ok(BakeTestReport {
        map,
        pack,
        area: ctx.area,
        style: ctx.style,
        appended: new.into_iter().map(|t| t.name).collect(),
        floor: floor.map(str::to_string),
        block_texture: BAKE_TEXTURE_A.to_string(),
        block,
        report,
        model_file,
        pack_file,
        changed,
    })
}

/// The frame (box) of a model head in world units, per axis `(min, max)`: `box · box_scale /
/// 4096²` (the convention of the vanilla map models, measured with `box_scale == up_scale` on
/// the rooms CPUD 187, IPKD/IPKE 217, and the one `bake` writes).
pub(crate) fn world_frame(h: &ModelHead) -> [(f64, f64); 3] {
    const FX: f64 = 4096.0;
    let k = f64::from(h.box_scale) / FX / FX;
    [0, 1, 2].map(|a| {
        let lo = f64::from(h.box_pos[a]);
        (lo * k, (lo + f64::from(h.box_size[a])) * k)
    })
}

/// The block tile at the middle of the frame (box) of the first model in `model`: the frame
/// in world units ([`world_frame`]), its tiles `tx0 = ⌊x / 16⌋ + 16`,
/// `tx1 = ⌊(x + w) / 16⌋ + 16 − 1` (z alike), then `cx = (tx0 + tx1) / 2`,
/// `cz = (tz0 + tz1) / 2 + 1`, each held to 0..=30 (Gutachten A prototype, `room`). `None`
/// when the model does not read.
pub(crate) fn frame_block(model: &[u8]) -> Option<(u8, u8)> {
    let file = Nsbmd::read(model).ok()?;
    let (_, m) = file.models.first()?;
    let frame = world_frame(&m.head);
    let tile = |v: f64| (v / f64::from(TILE)).floor() as i64 + i64::from(TILES / 2);
    let span = |a: usize| (tile(frame[a].0), tile(frame[a].1) - 1);
    let ((tx0, tx1), (tz0, tz1)) = (span(0), span(2));
    let held = |v: i64| u8::try_from(v.clamp(0, i64::from(BLOCK_MAX))).ok();
    Some((
        held((tx0 + tx1).div_euclid(2))?,
        held((tz0 + tz1).div_euclid(2) + 1)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bake::PackTexture;

    /// A baked model whose head carries the box `pos`, `size` at `scale` (fx 1/4096).
    fn model_with_box(pos: [i16; 3], size: [i16; 3], scale: i32) -> Vec<u8> {
        let textures: Vec<PackTexture> = ["a", "b"]
            .iter()
            .map(|n| PackTexture {
                name: n.to_string(),
                palette: format!("{n}_pl"),
                width: 16,
                height: 16,
            })
            .collect();
        let report = bake(
            &test_surface(["a", "b", "a", "b"], "a", (0, 0)),
            &BakeInput {
                style: AreaStyle::Indoor,
                textures: &textures,
                vram_bytes: 0,
                vram_limit: usize::MAX,
            },
        );
        let mut file = Nsbmd::read(&report.nsbmd).unwrap();
        let h = &mut file.models[0].1.head;
        h.box_pos = pos;
        h.box_size = size;
        h.box_scale = scale;
        file.write().unwrap()
    }

    #[test]
    fn the_block_sits_at_the_middle_of_the_old_frame() {
        // box_scale 32: x −256..−64 (tiles 0..=11), z −224..+80 → −224..−144 (tiles 2..=6).
        let room = model_with_box([-32768, 0, -28672], [24576, 4096, 10240], 32 * 4096);
        assert_eq!(frame_block(&room), Some((5, 5)));
        // A frame past the edges is held to 0..=30.
        let wide = model_with_box([-32768, 0, 30000], [32767, 0, 2000], 64 * 4096);
        assert_eq!(frame_block(&wide), Some((0, 30)));
        assert_eq!(frame_block(b"not a model"), None);
    }

    #[test]
    fn the_baked_frame_is_the_world_frame_of_the_input_corners() {
        let textures: Vec<PackTexture> = ["a", "b"]
            .iter()
            .map(|n| PackTexture {
                name: n.to_string(),
                palette: format!("{n}_pl"),
                width: 16,
                height: 16,
            })
            .collect();
        for block in [(0, 0), (15, 15), (30, 7)] {
            let faces = test_surface(["a", "b", "a", "b"], "a", block);
            let report = bake(
                &faces,
                &BakeInput {
                    style: AreaStyle::Outdoor,
                    textures: &textures,
                    vram_bytes: 0,
                    vram_limit: usize::MAX,
                },
            );
            assert_eq!(report.refusal(), None);
            let file = Nsbmd::read(&report.nsbmd).unwrap();
            let h = &file.models[0].1.head;
            let want: Vec<(f64, f64)> = (0..3)
                .map(|k| {
                    let all = faces.iter().flat_map(|f| f.corners.iter()).map(|c| c[k]);
                    let (lo, hi) = all.fold((i32::MAX, i32::MIN), |(l, h), v| (l.min(v), h.max(v)));
                    (f64::from(lo), f64::from(hi))
                })
                .collect();
            assert_eq!(world_frame(h).to_vec(), want, "block {block:?}");
            // The test surface spans 512 world units: box_scale 128 (· 4096).
            assert_eq!((h.box_scale, h.box_scale_inv), (128 * 4096, 32));
            // The whole map: the block lands at the middle (15, 16).
            assert_eq!(frame_block(&report.nsbmd), Some((15, 16)));
        }
    }
}
