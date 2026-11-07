//! Synthetic sample files for the import (OBJ, glTF, PNG), written from code: no game data,
//! nothing of them in the repository. Tests, scenarios and the browser gate write them with
//! [`write_samples`] or `g4k import-samples <dir>`.

use std::path::{Path, PathBuf};

use crate::error::{KernelError, io_at};

/// The unit cube (`+ g4_imp_cube.mtl`, `g4_imp_cube.png`).
pub const CUBE_OBJ: &str = "g4_imp_cube.obj";
/// The 4 × 4 m plane as binary glTF, its PNG in the BIN chunk.
pub const PLANE_GLB: &str = "g4_imp_plane.glb";
/// The same plane as glTF (`+ g4_imp_plane.bin`, `g4_imp_plane.png`).
pub const PLANE_GLTF: &str = "g4_imp_plane.gltf";
/// The 8 × 8 tile field in the style of a PDSMS export (`+ g4_pds_feld.mtl`, `g4_pds_gras.png`,
/// `g4_pds_weg.png`).
pub const PDSMS_OBJ: &str = "g4_pds_feld.obj";
/// 64 × 64 quads with their own uvs: too big for a map (`+ g4_imp_big.mtl`, uses
/// `g4_imp_cube.png`).
pub const BIG_OBJ: &str = "g4_imp_big.obj";
/// A 64 × 64 PNG with exactly 40 colours.
pub const PNG_40: &str = "g4_imp_40.png";

const CUBE_MTL: &str = "g4_imp_cube.mtl";
const CUBE_PNG: &str = "g4_imp_cube.png";
const PLANE_BIN: &str = "g4_imp_plane.bin";
const PLANE_PNG: &str = "g4_imp_plane.png";
const PDSMS_MTL: &str = "g4_pds_feld.mtl";
const PDSMS_GRAS_PNG: &str = "g4_pds_gras.png";
const PDSMS_WEG_PNG: &str = "g4_pds_weg.png";
const BIG_MTL: &str = "g4_imp_big.mtl";

/// Writes every sample (14 files) into `dir` (created); returns the paths in a fixed order.
pub fn write_samples(dir: &Path) -> Result<Vec<PathBuf>, KernelError> {
    std::fs::create_dir_all(dir).map_err(io_at(dir))?;
    let plane_png = plane_png()?;
    let files: Vec<(&str, Vec<u8>)> = vec![
        (CUBE_OBJ, cube_obj().into_bytes()),
        (CUBE_MTL, cube_mtl().into_bytes()),
        (CUBE_PNG, cube_png()?),
        (PLANE_GLB, plane_glb(&plane_png)?),
        (PLANE_GLTF, plane_gltf()?),
        (PLANE_BIN, plane_geometry()),
        (PLANE_PNG, plane_png),
        (PDSMS_OBJ, pdsms_obj().into_bytes()),
        (PDSMS_MTL, pdsms_mtl().into_bytes()),
        (PDSMS_GRAS_PNG, gras_png()?),
        (PDSMS_WEG_PNG, weg_png()?),
        (BIG_OBJ, big_obj().into_bytes()),
        (BIG_MTL, big_mtl().into_bytes()),
        (PNG_40, forty_png()?),
    ];
    let mut paths = Vec::with_capacity(files.len());
    for (name, bytes) in files {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(io_at(&path))?;
        paths.push(path);
    }
    Ok(paths)
}

/// The 40 colours of g4_imp_40.png: i in 0..40 → [40 + 28·(i % 8), 40 + 45·(i / 8), 200 − 20·(i % 8)].
pub fn forty_colours() -> [[u8; 3]; 40] {
    std::array::from_fn(|i| {
        let (column, row) = ((i % 8) as u8, (i / 8) as u8);
        [40 + 28 * column, 40 + 45 * row, 200 - 20 * column]
    })
}

/// An opaque `side` × `side` RGBA PNG with the colour `pixel(x, y)` at `(x, y)`.
fn png_of(side: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Result<Vec<u8>, KernelError> {
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            rgba.extend(pixel(x, y));
            rgba.push(255);
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, side, side);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::from)?;
    writer
        .write_image_data(&rgba)
        .map_err(std::io::Error::from)?;
    writer.finish().map_err(std::io::Error::from)?;
    Ok(out)
}

/// The unit cube: x and z in −0.5..0.5, y in 0..1, six quads counter-clockwise from outside,
/// each with the whole texture.
fn cube_obj() -> String {
    let mut s = String::from("# g4-studio import sample: unit cube (synthetic)\n");
    s.push_str(&format!("mtllib {CUBE_MTL}\no g4_imp_cube\n"));
    for y in [0, 1] {
        for (x, z) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            s.push_str(&format!("v {x} {y} {z}\n"));
        }
    }
    s.push_str("vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n");
    // Bottom, top, back (−z), front (+z), left (−x), right (+x).
    s.push_str("vn 0 -1 0\nvn 0 1 0\nvn 0 0 -1\nvn 0 0 1\nvn -1 0 0\nvn 1 0 0\n");
    s.push_str("usemtl g4_imp_cube_mat\ns off\n");
    let faces: [([u32; 4], u32); 6] = [
        ([1, 2, 3, 4], 1),
        ([8, 7, 6, 5], 2),
        ([2, 1, 5, 6], 3),
        ([4, 3, 7, 8], 4),
        ([1, 4, 8, 5], 5),
        ([3, 2, 6, 7], 6),
    ];
    for (corners, normal) in faces {
        s.push('f');
        for (uv, v) in corners.iter().enumerate() {
            s.push_str(&format!(" {v}/{}/{normal}", uv + 1));
        }
        s.push('\n');
    }
    s
}

fn cube_mtl() -> String {
    format!("newmtl g4_imp_cube_mat\nKd 1 1 1\nmap_Kd {CUBE_PNG}\n")
}

/// 16 × 16: a checkerboard of 4-pixel fields, orange and blue, with a dark border.
fn cube_png() -> Result<Vec<u8>, KernelError> {
    png_of(16, |x, y| {
        if x == 0 || y == 0 || x == 15 || y == 15 {
            [20, 20, 20]
        } else if (x / 4 + y / 4) % 2 == 0 {
            [230, 120, 40]
        } else {
            [40, 90, 200]
        }
    })
}

/// 16 × 16: diagonal stripes `((x + y) / 4) % 4` of four greens.
fn plane_png() -> Result<Vec<u8>, KernelError> {
    const GREENS: [[u8; 3]; 4] = [
        [30, 110, 40],
        [60, 150, 60],
        [100, 190, 80],
        [150, 220, 120],
    ];
    png_of(16, |x, y| GREENS[(((x + y) / 4) % 4) as usize])
}

const PLANE_POSITIONS: [[f32; 3]; 4] = [
    [-2.0, 0.0, -2.0],
    [-2.0, 0.0, 2.0],
    [2.0, 0.0, 2.0],
    [2.0, 0.0, -2.0],
];
const PLANE_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 4.0], [4.0, 4.0], [4.0, 0.0]];
const PLANE_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
/// Byte offsets in the geometry: positions, uvs, indices, end.
const PLANE_UV_AT: usize = 48;
const PLANE_INDEX_AT: usize = 80;
const PLANE_GEOMETRY_LEN: usize = 92;

/// Positions (f32 × 3), uvs (f32 × 2), indices (u16), little-endian: 92 bytes.
fn plane_geometry() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(PLANE_GEOMETRY_LEN);
    for c in PLANE_POSITIONS
        .iter()
        .flatten()
        .chain(PLANE_UVS.iter().flatten())
    {
        bytes.extend(c.to_le_bytes());
    }
    for i in PLANE_INDICES {
        bytes.extend(i.to_le_bytes());
    }
    bytes
}

/// The glTF document of the plane. `image` is the image entry (by buffer view or by uri),
/// `buffer` the buffer entry, `image_view` the buffer view of the embedded image.
fn plane_document(
    buffer: serde_json::Value,
    image: serde_json::Value,
    image_view: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut views = vec![
        serde_json::json!({"buffer": 0, "byteOffset": 0, "byteLength": PLANE_UV_AT, "target": 34962}),
        serde_json::json!({
            "buffer": 0,
            "byteOffset": PLANE_UV_AT,
            "byteLength": PLANE_INDEX_AT - PLANE_UV_AT,
            "target": 34962
        }),
        serde_json::json!({
            "buffer": 0,
            "byteOffset": PLANE_INDEX_AT,
            "byteLength": PLANE_GEOMETRY_LEN - PLANE_INDEX_AT,
            "target": 34963
        }),
    ];
    views.extend(image_view);
    serde_json::json!({
        "asset": {"version": "2.0", "generator": "gen4-studio import samples (synthetic)"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "g4_imp_plane", "mesh": 0}],
        "meshes": [{
            "name": "g4_imp_plane",
            "primitives": [{
                "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                "indices": 2,
                "material": 0,
                "mode": 4
            }]
        }],
        "materials": [{
            "name": "g4_imp_plane_mat",
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": 0},
                "metallicFactor": 0.0
            }
        }],
        "textures": [{"sampler": 0, "source": 0}],
        "samplers": [{"magFilter": 9728, "minFilter": 9728, "wrapS": 10497, "wrapT": 10497}],
        "images": [image],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 4,
                "type": "VEC3",
                "min": [-2.0, 0.0, -2.0],
                "max": [2.0, 0.0, 2.0]
            },
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC2"},
            {"bufferView": 2, "componentType": 5123, "count": 6, "type": "SCALAR"}
        ],
        "bufferViews": views,
        "buffers": [buffer]
    })
}

fn json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, KernelError> {
    serde_json::to_vec(value).map_err(|e| KernelError::Io(std::io::Error::other(e)))
}

/// The plane as a `.gltf` naming `g4_imp_plane.bin` and `g4_imp_plane.png`.
fn plane_gltf() -> Result<Vec<u8>, KernelError> {
    let document = plane_document(
        serde_json::json!({"uri": PLANE_BIN, "byteLength": PLANE_GEOMETRY_LEN}),
        serde_json::json!({"name": "g4_imp_plane", "uri": PLANE_PNG}),
        None,
    );
    json_bytes(&document)
}

/// Pads `bytes` with `fill` to a multiple of 4.
fn pad4(bytes: &mut Vec<u8>, fill: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(fill);
    }
}

fn le32(n: usize) -> Result<[u8; 4], KernelError> {
    u32::try_from(n)
        .map(u32::to_le_bytes)
        .map_err(|_| KernelError::Io(std::io::Error::other("glb too large")))
}

/// The plane as binary glTF: the geometry, then the PNG (at a 4-byte offset) in the BIN chunk;
/// the JSON chunk padded with spaces, the BIN chunk with zeros.
fn plane_glb(png: &[u8]) -> Result<Vec<u8>, KernelError> {
    let mut bin = plane_geometry();
    pad4(&mut bin, 0);
    let image_at = bin.len();
    bin.extend_from_slice(png);
    let buffer_len = bin.len();
    pad4(&mut bin, 0);
    let document = plane_document(
        serde_json::json!({"byteLength": buffer_len}),
        serde_json::json!({"name": "g4_imp_plane", "bufferView": 3, "mimeType": "image/png"}),
        Some(serde_json::json!({"buffer": 0, "byteOffset": image_at, "byteLength": png.len()})),
    );
    let mut json = json_bytes(&document)?;
    pad4(&mut json, b' ');
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend(le32(total)?);
    out.extend(le32(json.len())?);
    out.extend_from_slice(b"JSON");
    out.extend(json);
    out.extend(le32(bin.len())?);
    out.extend_from_slice(b"BIN\0");
    out.extend(bin);
    Ok(out)
}

/// An 8 × 8 tile field (one tile = one unit) around the origin at y 0 as PDSMS describes its
/// export: one quad per tile with its own corners and uvs 0..1, a checkerboard of the
/// materials `gras` and `weg`, grouped by material. Follows the public description only.
fn pdsms_obj() -> String {
    let mut s = String::from("# Pokemon DS Map Studio style export (synthetic)\n");
    s.push_str(&format!("mtllib {PDSMS_MTL}\no feld\n"));
    let tiles: Vec<(i32, i32)> = (0..8)
        .flat_map(|tz| (0..8).map(move |tx| (tx, tz)))
        .collect();
    for &(tx, tz) in &tiles {
        let (x0, x1, z0, z1) = (tx - 4, tx - 3, tz - 4, tz - 3);
        for (x, z) in [(x0, z1), (x1, z1), (x1, z0), (x0, z0)] {
            s.push_str(&format!("v {x} 0 {z}\n"));
        }
    }
    for _ in &tiles {
        s.push_str("vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n");
    }
    s.push_str("vn 0 1 0\ng feld\ns off\n");
    for (material, parity) in [("gras", 0), ("weg", 1)] {
        s.push_str(&format!("usemtl {material}\n"));
        for (n, &(tx, tz)) in tiles.iter().enumerate() {
            if (tx + tz) % 2 != parity {
                continue;
            }
            let first = 4 * n + 1;
            s.push('f');
            for k in first..first + 4 {
                s.push_str(&format!(" {k}/{k}/1"));
            }
            s.push('\n');
        }
    }
    s
}

fn pdsms_mtl() -> String {
    let mut s = String::from("# Pokemon DS Map Studio style export (synthetic)\n");
    for (name, png) in [("gras", PDSMS_GRAS_PNG), ("weg", PDSMS_WEG_PNG)] {
        s.push_str(&format!(
            "newmtl {name}\nKa 1 1 1\nKd 1 1 1\nKs 0 0 0\nd 1\nillum 1\nmap_Kd {png}\n"
        ));
    }
    s
}

/// 16 × 16 grass: a green with lighter blades.
fn gras_png() -> Result<Vec<u8>, KernelError> {
    png_of(16, |x, y| {
        if (x * 7 + y * 3) % 5 == 0 {
            [110, 200, 80]
        } else {
            [70, 160, 60]
        }
    })
}

/// 16 × 16 path: sand-coloured stones in rows of 4 with darker joints.
fn weg_png() -> Result<Vec<u8>, KernelError> {
    png_of(16, |x, y| {
        let offset = (y / 4) % 2 * 4;
        if y % 4 == 0 || (x + offset) % 8 == 0 {
            [150, 120, 80]
        } else {
            [210, 180, 120]
        }
    })
}

/// 64 × 64 quads of 0.5 units around the origin at y 0, each with uvs 0..1: neighbours differ
/// in uv on their shared edge, so no strip is possible.
fn big_obj() -> String {
    let mut s = String::from("# g4-studio import sample: 64 x 64 quads, too big (synthetic)\n");
    s.push_str(&format!("mtllib {BIG_MTL}\no g4_imp_big\n"));
    for row in 0..=64u32 {
        for column in 0..=64u32 {
            let (x, z) = (f64::from(column) * 0.5 - 16.0, f64::from(row) * 0.5 - 16.0);
            s.push_str(&format!("v {x} 0 {z}\n"));
        }
    }
    s.push_str("vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvn 0 1 0\nusemtl g4_imp_big_mat\ns off\n");
    let at = |column: u32, row: u32| row * 65 + column + 1;
    for row in 0..64 {
        for column in 0..64 {
            let corners = [
                at(column, row + 1),
                at(column + 1, row + 1),
                at(column + 1, row),
                at(column, row),
            ];
            s.push('f');
            for (uv, v) in corners.iter().enumerate() {
                s.push_str(&format!(" {v}/{}/1", uv + 1));
            }
            s.push('\n');
        }
    }
    s
}

fn big_mtl() -> String {
    format!("newmtl g4_imp_big_mat\nKd 1 1 1\nmap_Kd {CUBE_PNG}\n")
}

/// 64 × 64: pixel `(x, y)` has `forty_colours()[(x / 8) + 8 · min(4, y / 13)]`.
fn forty_png() -> Result<Vec<u8>, KernelError> {
    let colours = forty_colours();
    png_of(64, |x, y| colours[(x / 8 + 8 * (y / 13).min(4)) as usize])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use g4_formats::texconv::to_bgr555;

    use super::*;

    fn read_png(path: &Path) -> (u32, u32, Vec<[u8; 3]>) {
        let bytes = std::fs::read(path).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        let step = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            other => panic!("colour type {other:?}"),
        };
        buf.truncate(info.buffer_size());
        let pixels = buf
            .chunks_exact(step)
            .map(|p| {
                if step == 4 {
                    assert_eq!(p[3], 255, "opaque");
                }
                [p[0], p[1], p[2]]
            })
            .collect();
        (info.width, info.height, pixels)
    }

    #[test]
    fn samples_are_written_and_deterministic() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        // A missing folder is created.
        let a_dir = a.path().join("samples");
        let first = write_samples(&a_dir).unwrap();
        let second = write_samples(b.path()).unwrap();
        assert_eq!(first.len(), 14);
        assert_eq!(second.len(), 14);
        let names: Vec<_> = first
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        for main in [CUBE_OBJ, PLANE_GLB, PLANE_GLTF, PDSMS_OBJ, BIG_OBJ, PNG_40] {
            assert!(names.iter().any(|n| n == main), "{main} in {names:?}");
        }
        assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), 14, "{names:?}");
        for (p, q) in first.iter().zip(&second) {
            assert_eq!(p.parent().unwrap(), a_dir);
            assert_eq!(p.file_name(), q.file_name());
            let bytes = std::fs::read(p).unwrap();
            assert!(!bytes.is_empty(), "{}", p.display());
            assert_eq!(bytes, std::fs::read(q).unwrap(), "{}", p.display());
        }
        let entries = std::fs::read_dir(&a_dir).unwrap().count();
        assert_eq!(entries, 14);
    }

    #[test]
    fn the_40_colour_png_has_40_colours() {
        let colours = forty_colours();
        assert_eq!(colours[0], [40, 40, 200]);
        assert_eq!(colours[39], [236, 220, 60]);
        let dir = tempfile::tempdir().unwrap();
        write_samples(dir.path()).unwrap();
        let (w, h, pixels) = read_png(&dir.path().join(PNG_40));
        assert_eq!((w, h), (64, 64));
        let distinct: BTreeSet<[u8; 3]> = pixels.iter().copied().collect();
        assert_eq!(distinct.len(), 40);
        assert_eq!(distinct, colours.iter().copied().collect());
        let bgr: BTreeSet<u16> = distinct.iter().map(|&c| to_bgr555(c)).collect();
        assert_eq!(bgr.len(), 40);
        // Pixel (x, y) → colour (x / 8) + 8 · min(4, y / 13).
        for (x, y) in [(0, 0), (63, 0), (9, 13), (63, 51), (63, 63), (17, 52)] {
            let i = x / 8 + 8 * (y / 13).min(4);
            assert_eq!(pixels[y * 64 + x], colours[i], "({x}, {y})");
        }
    }

    #[test]
    fn the_glb_is_a_valid_container() {
        let dir = tempfile::tempdir().unwrap();
        write_samples(dir.path()).unwrap();
        let glb = std::fs::read(dir.path().join(PLANE_GLB)).unwrap();
        let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
        assert_eq!(&glb[0..4], b"glTF");
        assert_eq!(word(4), 2);
        assert_eq!(word(8), glb.len());
        let json_len = word(12);
        assert_eq!(&glb[16..20], b"JSON");
        assert_eq!(json_len % 4, 0);
        let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        let bin_at = 20 + json_len;
        let bin_len = word(bin_at);
        assert_eq!(&glb[bin_at + 4..bin_at + 8], b"BIN\0");
        assert_eq!(bin_len % 4, 0);
        assert_eq!(bin_at + 8 + bin_len, glb.len());
        let bin = &glb[bin_at + 8..];
        assert_eq!(json["asset"]["version"], "2.0");
        let buffer_len = json["buffers"][0]["byteLength"].as_u64().unwrap() as usize;
        assert!(buffer_len <= bin_len);
        assert!(json["buffers"][0].get("uri").is_none());
        for view in json["bufferViews"].as_array().unwrap() {
            let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
            let len = view["byteLength"].as_u64().unwrap() as usize;
            assert_eq!(offset % 4, 0, "{view}");
            assert!(offset + len <= buffer_len, "{view}");
        }
        let image = &json["images"][0];
        assert_eq!(image["mimeType"], "image/png");
        let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
        let at = view["byteOffset"].as_u64().unwrap() as usize;
        assert_eq!(&bin[at + 1..at + 4], b"PNG");
        // The embedded PNG is the one the .gltf names.
        let len = view["byteLength"].as_u64().unwrap() as usize;
        let png = std::fs::read(dir.path().join("g4_imp_plane.png")).unwrap();
        assert_eq!(&bin[at..at + len], png.as_slice());
        let (w, h, _) = read_png(&dir.path().join("g4_imp_plane.png"));
        assert_eq!((w, h), (16, 16));
        // The .gltf names its buffer and image by file.
        let text = std::fs::read(dir.path().join(PLANE_GLTF)).unwrap();
        let gltf: serde_json::Value = serde_json::from_slice(&text).unwrap();
        assert_eq!(gltf["buffers"][0]["uri"], "g4_imp_plane.bin");
        assert_eq!(gltf["images"][0]["uri"], "g4_imp_plane.png");
        let bin_file = std::fs::read(dir.path().join("g4_imp_plane.bin")).unwrap();
        assert_eq!(
            gltf["buffers"][0]["byteLength"].as_u64().unwrap() as usize,
            bin_file.len()
        );
        assert_eq!(gltf["accessors"], json["accessors"]);
        assert_eq!(
            gltf["accessors"][0]["min"],
            serde_json::json!([-2.0, 0.0, -2.0])
        );
        assert_eq!(
            gltf["accessors"][0]["max"],
            serde_json::json!([2.0, 0.0, 2.0])
        );
    }

    #[test]
    fn the_cube_obj_names_its_mtl_and_png() {
        let dir = tempfile::tempdir().unwrap();
        write_samples(dir.path()).unwrap();
        let obj = std::fs::read_to_string(dir.path().join(CUBE_OBJ)).unwrap();
        let lines: Vec<_> = obj.lines().collect();
        assert!(lines.contains(&"mtllib g4_imp_cube.mtl"), "{obj}");
        assert!(lines.contains(&"usemtl g4_imp_cube_mat"), "{obj}");
        let count = |p: &str| lines.iter().filter(|l| l.starts_with(p)).count();
        assert_eq!(
            (count("v "), count("vt "), count("vn "), count("f ")),
            (8, 4, 6, 6)
        );
        let mtl = std::fs::read_to_string(dir.path().join("g4_imp_cube.mtl")).unwrap();
        let lines: Vec<_> = mtl.lines().collect();
        assert!(lines.contains(&"newmtl g4_imp_cube_mat"), "{mtl}");
        assert!(lines.contains(&"Kd 1 1 1"), "{mtl}");
        assert!(lines.contains(&"map_Kd g4_imp_cube.png"), "{mtl}");
        let (w, h, pixels) = read_png(&dir.path().join("g4_imp_cube.png"));
        assert_eq!((w, h), (16, 16));
        assert_eq!(pixels[0], [20, 20, 20]);
        assert_eq!(pixels[16 + 1], [230, 120, 40]);
        assert_eq!(pixels[16 + 5], [40, 90, 200]);
    }
}
