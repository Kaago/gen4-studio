//! Runs of faces: quad strips over runs of texture, layer and normal, single quads for the rest.
//!
//! A face `g` continues a face `f` when `g.c0 == f.c3` and `g.c1 == f.c2` (position and uv),
//! both have the same layer and the same normal (the caller passes the faces of one texture).
//! A chain starts at the first unused face in input order without an unused predecessor (else
//! the first unused face); it goes on with the first unused face in input order that continues
//! it. Chains of two faces and more are strips, the other faces single quads.

use std::collections::HashMap;

use super::Face;

/// Faces sent together: a quad strip in chain order (`strip`) or single quads.
pub(crate) struct Run {
    pub faces: Vec<usize>,
    pub strip: bool,
}

/// The unit normal of `face` in 1/512: `(c1 − c0) × (c2 − c0)`, at length 0
/// `(c2 − c0) × (c3 − c0)`, else `(0, 1, 0)`; each component `round(v · 511)` in −512..=511.
pub(crate) fn normal(face: &Face) -> [i32; 3] {
    let c = face.corners.map(|p| p.map(i128::from));
    let sub = |a: [i128; 3], b: [i128; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [i128; 3], b: [i128; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let mut n = cross(sub(c[1], c[0]), sub(c[2], c[0]));
    if n == [0; 3] {
        n = cross(sub(c[2], c[0]), sub(c[3], c[0]));
    }
    if n == [0; 3] {
        return [0, 511, 0];
    }
    let n = n.map(|v| v as f64);
    let len = n.iter().map(|v| v * v).sum::<f64>().sqrt();
    n.map(|v| ((v / len * 511.0).round() as i32).clamp(-512, 511))
}

/// One corner: position and uv.
type Corner = ([i32; 3], [i32; 2]);
/// An edge of a face with what a strip must share: layer, normal, two corners.
type Edge = (u8, [i32; 3], Corner, Corner);

/// The runs of the faces `of_texture` (indices into `faces`, in input order): strips first,
/// then one run of every single quad.
pub(crate) fn runs(faces: &[Face], of_texture: &[usize]) -> Vec<Run> {
    let n = of_texture.len();
    let normals: Vec<[i32; 3]> = of_texture.iter().map(|&i| normal(&faces[i])).collect();
    let edge = |p: usize, a: usize, b: usize| -> Edge {
        let f = &faces[of_texture[p]];
        (
            f.layer,
            normals[p],
            (f.corners[a], f.uv[a]),
            (f.corners[b], f.uv[b]),
        )
    };
    // First edge (c0, c1) and last edge (c3, c2) → the faces with it, in input order.
    let mut firsts: HashMap<Edge, Vec<usize>> = HashMap::new();
    let mut lasts: HashMap<Edge, Vec<usize>> = HashMap::new();
    for p in 0..n {
        firsts.entry(edge(p, 0, 1)).or_default().push(p);
        lasts.entry(edge(p, 3, 2)).or_default().push(p);
    }
    let has_predecessor = |p: usize, used: &[bool]| {
        lasts
            .get(&edge(p, 0, 1))
            .is_some_and(|qs| qs.iter().any(|&q| q != p && !used[q]))
    };
    let successor = |p: usize, used: &[bool]| {
        firsts
            .get(&edge(p, 3, 2))
            .and_then(|qs| qs.iter().copied().find(|&q| !used[q]))
    };

    let mut used = vec![false; n];
    let mut strips = Vec::new();
    let mut singles = Vec::new();
    let mut lo = 0;
    loop {
        while lo < n && used[lo] {
            lo += 1;
        }
        if lo == n {
            break;
        }
        let start = (lo..n)
            .find(|&p| !used[p] && !has_predecessor(p, &used))
            .unwrap_or(lo);
        used[start] = true;
        let mut chain = vec![of_texture[start]];
        let mut at = start;
        while let Some(next) = successor(at, &used) {
            used[next] = true;
            chain.push(of_texture[next]);
            at = next;
        }
        if chain.len() >= 2 {
            strips.push(Run {
                faces: chain,
                strip: true,
            });
        } else {
            singles.extend(chain);
        }
    }
    if !singles.is_empty() {
        strips.push(Run {
            faces: singles,
            strip: false,
        });
    }
    strips
}
