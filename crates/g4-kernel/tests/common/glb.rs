//! Reading the GLB apicula writes: the chunks, float accessors, and the set of (corner, uv) of
//! every corner, to hold a baked model against the faces it was baked from (6.1, 6.5a, 6.5b).
#![allow(dead_code)]

use std::collections::BTreeSet;

use g4_kernel::bake::Face;

/// (corner in world units, uv in 1/16 texel) of every corner.
pub type CornerSet = BTreeSet<([i32; 3], [i32; 2])>;

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

/// The corners of `faces` with their uv · 16.
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

/// The corners of every primitive of the GLB, positions rounded to world units, uvs read as
/// of a texture 16 texels wide and high, in 1/16 texel.
pub fn glb_set(json: &serde_json::Value, bin: &[u8]) -> CornerSet {
    let mut set = CornerSet::new();
    for mesh in json["meshes"].as_array().unwrap() {
        for prim in mesh["primitives"].as_array().unwrap() {
            let attrs = &prim["attributes"];
            let pos = accessor(json, bin, attrs["POSITION"].as_u64().unwrap() as usize, 3);
            let uv = accessor(json, bin, attrs["TEXCOORD_0"].as_u64().unwrap() as usize, 2);
            assert_eq!(pos.len(), uv.len());
            for (p, t) in pos.iter().zip(&uv) {
                let c = [p[0], p[1], p[2]].map(|v| v.round() as i32);
                // u · 16 texels (width 16), in 1/16 texel.
                let st = [t[0], t[1]].map(|v| (v * 16.0 * 16.0).round() as i32);
                set.insert((c, st));
            }
        }
    }
    set
}
