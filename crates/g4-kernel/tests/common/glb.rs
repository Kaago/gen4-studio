//! Readers of the GLB apicula writes, shared by the oracle tests of the baker (`tests/bake.rs`),
//! of the model import (`tests/model_import.rs`) and of the world bake (`tests/world_bake.rs`):
//! the chunks, the accessors and the set of (corner, uv) pairs compared against the baker's
//! input (6.1, 6.3b, 6.5a, 6.5b).
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
    glb_primitives(json, bin)
        .into_iter()
        .flat_map(|(material, corners)| {
            corners
                .into_iter()
                .map(move |(c, t)| (c, material.clone(), t))
        })
        .collect()
}

/// A corner of a primitive: its position rounded to whole units and its texture coordinates.
type GlbCorner = ([i32; 3], [f32; 2]);

/// Per primitive its material's name and its corners in index order (position rounded to
/// whole units, texture coordinates).
fn glb_primitives(json: &serde_json::Value, bin: &[u8]) -> Vec<(String, Vec<GlbCorner>)> {
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
            let corners = used
                .into_iter()
                .map(|v| {
                    let (p, t) = (&pos[v], &uv[v]);
                    ([p[0], p[1], p[2]].map(|v| v.round() as i32), [t[0], t[1]])
                })
                .collect();
            out.push((material, corners));
        }
    }
    out
}

/// One triangle of a GLB: its material's name and its three corners with uv in 1/16 texel.
pub type GlbTriangle = (String, [([i32; 3], [i32; 2]); 3]);

/// The triangles of a GLB whose primitives are triangle lists (glTF mode 4, the default);
/// `size` gives a texture's size in texels.
pub fn glb_triangles(
    json: &serde_json::Value,
    bin: &[u8],
    size: impl Fn(&str) -> (u16, u16),
) -> Vec<GlbTriangle> {
    for mesh in json["meshes"].as_array().unwrap() {
        for prim in mesh["primitives"].as_array().unwrap() {
            assert!(
                prim["mode"].as_u64().is_none_or(|m| m == 4),
                "not a triangle list: {prim}"
            );
        }
    }
    let mut out = Vec::new();
    for (material, corners) in glb_primitives(json, bin) {
        assert_eq!(
            corners.len() % 3,
            0,
            "{material}: {} corners",
            corners.len()
        );
        let wh = size(&material);
        for t in corners.chunks(3) {
            let tri = [0, 1, 2].map(|k| (t[k].0, uv_of(t[k].1, wh)));
            out.push((material.clone(), tri));
        }
    }
    out
}

/// The triangles of `triangles` that match no input face shift-invariantly, as messages: a
/// triangle matches a face with its texture that has all three of its corners when its uv
/// minus the face's uv at those corners is one vector for all three, a multiple of the
/// texture's size (the baker shifts a face's uv by whole textures, one shift per face or
/// strip). Every face must be matched by exactly two triangles.
pub fn unmatched_triangles(
    faces: &[Face],
    triangles: &[GlbTriangle],
    size: impl Fn(&str) -> (u16, u16),
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut hits = vec![0usize; faces.len()];
    for (material, tri) in triangles {
        let (w, h) = size(material);
        let (w, h) = (i32::from(w) * 16, i32::from(h) * 16);
        let found = faces.iter().enumerate().find(|(_, f)| {
            if &f.texture != material {
                return false;
            }
            let mut shift = None;
            tri.iter().all(|(c, st)| {
                let Some(k) = f.corners.iter().position(|fc| fc == c) else {
                    return false;
                };
                let d = [st[0] - f.uv[k][0] * 16, st[1] - f.uv[k][1] * 16];
                *shift.get_or_insert(d) == d
            }) && shift.is_some_and(|d| d[0].rem_euclid(w) == 0 && d[1].rem_euclid(h) == 0)
        });
        match found {
            Some((i, _)) => hits[i] += 1,
            None => problems.push(format!("triangle {material} {tri:?} matches no face")),
        }
    }
    for (i, n) in hits.iter().enumerate().filter(|(_, n)| **n != 2) {
        problems.push(format!(
            "face {i} ({:?}) has {n} triangles",
            faces[i].corners
        ));
    }
    problems
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
