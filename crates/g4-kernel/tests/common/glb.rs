//! Readers of the GLB apicula writes, shared by the oracle tests of the baker (`tests/bake.rs`)
//! and of the world bake (`tests/world_bake.rs`): the chunks, the accessors and the set of
//! (corner, uv) pairs compared against the baker's input.
#![allow(dead_code)]

use std::collections::BTreeSet;

use g4_kernel::bake::Face;

/// (corner, uv in 1/16 texel) of every corner.
pub type CornerSet = BTreeSet<([i32; 3], [i32; 2])>;

/// One accessor of a GLB as f32 tuples of `n` components.
pub fn accessor(json: &serde_json::Value, bin: &[u8], index: usize, n: usize) -> Vec<Vec<f32>> {
    let acc = &json["accessors"][index];
    assert_eq!(acc["componentType"], 5126, "accessor {index}: {acc}");
    let count = acc["count"].as_u64().unwrap() as usize;
    let view = &json["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
    let base = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().map_or(4 * n, |s| s as usize);
    (0..count)
        .map(|i| {
            (0..n)
                .map(|k| {
                    let at = base + i * stride + 4 * k;
                    f32::from_le_bytes(bin[at..at + 4].try_into().unwrap())
                })
                .collect()
        })
        .collect()
}

/// The JSON and BIN chunk of a GLB.
pub fn glb_chunks(glb: &[u8]) -> (serde_json::Value, Vec<u8>) {
    assert_eq!(&glb[0..4], b"glTF");
    let u32_at = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(u32_at(8), glb.len());
    let (json_len, json_type) = (u32_at(12), &glb[16..20]);
    assert_eq!(json_type, b"JSON");
    let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let bin_at = 20 + json_len;
    let (bin_len, bin_type) = (u32_at(bin_at), &glb[bin_at + 4..bin_at + 8]);
    assert_eq!(bin_type, b"BIN\0");
    (json, glb[bin_at + 8..bin_at + 8 + bin_len].to_vec())
}

/// The (corner, uv) pairs of the baker's input faces.
pub fn input_set(faces: &[Face]) -> CornerSet {
    faces
        .iter()
        .flat_map(|f| {
            f.corners
                .iter()
                .zip(&f.uv)
                .map(|(c, uv)| (*c, [uv[0] * 16, uv[1] * 16]))
        })
        .collect()
}

/// The (corner, uv) pairs of a GLB whose textures are all 16 × 16 texels.
pub fn glb_set(json: &serde_json::Value, bin: &[u8]) -> CornerSet {
    glb_corners(json, bin)
        .into_iter()
        .map(|(c, _, t)| (c, uv_of(t, (16, 16))))
        .collect()
}

/// (corner, texture, uv in 1/16 texel modulo the texture's size) of every corner: the baker
/// may shift a quad's uv by whole texture sizes to fit the `TEXCOORD` range, which shows the
/// same texels.
pub type WrappedSet = BTreeSet<([i32; 3], String, [i32; 2])>;

/// The [`WrappedSet`] of the baker's input faces; `size` gives a texture's size in texels.
pub fn input_wrapped(faces: &[Face], size: impl Fn(&str) -> (u16, u16)) -> WrappedSet {
    faces
        .iter()
        .flat_map(|f| {
            let (w, h) = size(&f.texture);
            f.corners.iter().zip(&f.uv).map(move |(c, uv)| {
                let st = [uv[0] * 16, uv[1] * 16];
                (*c, f.texture.clone(), wrap(st, (w, h)))
            })
        })
        .collect()
}

/// The [`WrappedSet`] of a GLB, a primitive's texture being its material's name; `size` gives
/// a texture's size in texels (glTF texture coordinates are fractions of it).
pub fn glb_wrapped(
    json: &serde_json::Value,
    bin: &[u8],
    size: impl Fn(&str) -> (u16, u16),
) -> WrappedSet {
    glb_corners(json, bin)
        .into_iter()
        .map(|(c, material, t)| {
            let wh = size(&material);
            (c, material, wrap(uv_of(t, wh), wh))
        })
        .collect()
}

/// Every corner of every primitive: its position rounded to whole units, its material's name
/// and its texture coordinates.
fn glb_corners(json: &serde_json::Value, bin: &[u8]) -> Vec<([i32; 3], String, [f32; 2])> {
    let mut out = Vec::new();
    for mesh in json["meshes"].as_array().unwrap() {
        for prim in mesh["primitives"].as_array().unwrap() {
            let material = prim["material"]
                .as_u64()
                .and_then(|m| json["materials"][m as usize]["name"].as_str())
                .unwrap_or_default()
                .to_owned();
            let attrs = &prim["attributes"];
            let pos = accessor(json, bin, attrs["POSITION"].as_u64().unwrap() as usize, 3);
            let uv = accessor(json, bin, attrs["TEXCOORD_0"].as_u64().unwrap() as usize, 2);
            assert_eq!(pos.len(), uv.len());
            // Primitives may share one vertex buffer: only the vertices a primitive's indices
            // name are its own.
            let used: Vec<usize> = match prim["indices"].as_u64() {
                Some(i) => indices(json, bin, i as usize),
                None => (0..pos.len()).collect(),
            };
            for v in used {
                let (p, t) = (&pos[v], &uv[v]);
                let c = [p[0], p[1], p[2]].map(|v| v.round() as i32);
                out.push((c, material.clone(), [t[0], t[1]]));
            }
        }
    }
    out
}

/// The vertex indices of index accessor `index` (unsigned byte, short or int).
fn indices(json: &serde_json::Value, bin: &[u8], index: usize) -> Vec<usize> {
    let acc = &json["accessors"][index];
    let count = acc["count"].as_u64().unwrap() as usize;
    let view = &json["bufferViews"][acc["bufferView"].as_u64().unwrap() as usize];
    let base = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let width = match acc["componentType"].as_u64() {
        Some(5121) => 1,
        Some(5123) => 2,
        Some(5125) => 4,
        other => panic!("index accessor {index}: component type {other:?}"),
    };
    (0..count)
        .map(|i| {
            let at = base + i * width;
            let mut word = [0u8; 4];
            word[..width].copy_from_slice(&bin[at..at + width]);
            u32::from_le_bytes(word) as usize
        })
        .collect()
}

/// glTF texture coordinates of a `w` × `h` texture in 1/16 texel.
fn uv_of(t: [f32; 2], (w, h): (u16, u16)) -> [i32; 2] {
    [
        (t[0] * f32::from(w) * 16.0).round() as i32,
        (t[1] * f32::from(h) * 16.0).round() as i32,
    ]
}

/// `st` (1/16 texel) modulo the size of a `w` × `h` texture.
fn wrap(st: [i32; 2], (w, h): (u16, u16)) -> [i32; 2] {
    [
        st[0].rem_euclid(i32::from(w) * 16),
        st[1].rem_euclid(i32::from(h) * 16),
    ]
}

/// The material names of a GLB, in its order.
pub fn glb_materials(json: &serde_json::Value) -> Vec<String> {
    json["materials"]
        .as_array()
        .map(|ms| {
            ms.iter()
                .map(|m| m["name"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}
