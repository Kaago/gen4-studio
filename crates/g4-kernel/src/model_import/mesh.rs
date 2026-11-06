//! The neutral mesh the readers fill, and its faces for the baker.

use std::path::PathBuf;

use g4_formats::nsbtx::Rgba;

use crate::bake::{Face, WORLD_MAX, WORLD_MIN};
use crate::map_import::{PartCode, PartMessage};

/// A model as read: positions in model units (y up, node transforms applied), uvs as fractions
/// of the texture with t downwards, polygons with their material.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub polygons: Vec<Polygon>,
    pub materials: Vec<MeshMaterial>,
}

/// A polygon: its material (index into `Mesh::materials`) and its corners (position index,
/// uv index), at least 3, counter-clockwise seen from the front.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub material: Option<usize>,
    pub corners: Vec<(u32, Option<u32>)>,
}

/// A material with its image, if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshMaterial {
    pub name: String,
    pub image: Option<MeshImage>,
}

/// The image of a material: the file stem its texture is named after, the file (`None` when
/// embedded) and the pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshImage {
    pub stem: String,
    pub file: Option<PathBuf>,
    pub rgba: Rgba,
}

/// The pack texture a material is drawn with, and its size in texels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub texture: String,
    pub width: u16,
    pub height: u16,
}

/// What the conversion made: polygons read, quads and triangles given to the baker, faces
/// dropped because they had no area once rounded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct MeshStats {
    pub polygons: usize,
    pub quads: usize,
    pub triangles: usize,
    pub dropped: usize,
}

/// The faces of a mesh with their counts and the hints of the conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct Converted {
    pub faces: Vec<Face>,
    pub stats: MeshStats,
    pub notes: Vec<PartMessage>,
}

/// How far the fourth corner of a quad may lie off the plane of the first three, in world units
/// (after scaling, before rounding).
const PLANE_TOLERANCE: f64 = 0.5;

/// A corner after scaling: position in world units, uv in texels (both not yet rounded).
type Corner = ([f64; 3], [f64; 2]);

/// Faces of `mesh` (layer 0), material m bound to `bindings[m]`; refuses with the first error.
///
/// Per polygon in order: a binding (`model.material-untextured`), uv at every corner
/// (`model.uv-missing`), every rounded corner in −512..=511 (`model.out-of-range`). Triangles
/// stay, a flat convex quad stays, every other polygon is ear clipped; corners are rounded to
/// whole world units and texels (half away from 0), faces without area once rounded are dropped
/// (hint `model.faces-dropped`), then triangles are paired into quads. A polygon with fewer
/// than 3 corners or an index outside the mesh (a reader's bug, never a panic) is dropped as
/// well. No face left: `model.no-geometry`.
pub fn to_faces(
    mesh: &Mesh,
    unit: f32,
    bindings: &[Option<Binding>],
) -> Result<Converted, PartMessage> {
    let unit = f64::from(unit);
    let mut dropped = 0;
    let mut faces = Vec::new();
    for polygon in &mesh.polygons {
        let name = polygon
            .material
            .and_then(|m| mesh.materials.get(m))
            .map_or("", |m| m.name.as_str());
        let Some(binding) = polygon
            .material
            .and_then(|m| bindings.get(m))
            .and_then(Option::as_ref)
        else {
            return Err(PartMessage::new(
                PartCode::ModelMaterialUntextured,
                &[&name],
                format!(
                    "the material {name:?} has no image and the area's pack no texture of \
                     its name"
                ),
            ));
        };
        let Some(corners) = corners_of(mesh, polygon, unit, binding, name)? else {
            dropped += 1;
            continue;
        };
        check_range(&corners)?;
        let positions: Vec<[f64; 3]> = corners.iter().map(|c| c.0).collect();
        let pieces: Vec<Vec<usize>> = match positions.len() {
            3 => vec![vec![0, 1, 2]],
            4 if flat_convex(&[positions[0], positions[1], positions[2], positions[3]]) => {
                vec![vec![0, 1, 2, 3]]
            }
            _ => ear_clip(&positions)
                .into_iter()
                .map(|t| t.to_vec())
                .collect(),
        };
        for piece in pieces {
            let face = face_of(&piece, &corners, &binding.texture);
            if has_area(&face) {
                faces.push(face);
            } else {
                dropped += 1;
            }
        }
    }
    let faces = pair_triangles(faces);
    if faces.is_empty() {
        return Err(PartMessage::new(
            PartCode::ModelNoGeometry,
            &[],
            "the model has no face (none read, or none left once rounded to whole world units)"
                .to_string(),
        ));
    }
    let triangles = faces.iter().filter(|f| f.is_triangle()).count();
    let stats = MeshStats {
        polygons: mesh.polygons.len(),
        quads: faces.len() - triangles,
        triangles,
        dropped,
    };
    let mut notes = Vec::new();
    if dropped > 0 {
        notes.push(PartMessage::new(
            PartCode::ModelFacesDropped,
            &[&dropped],
            format!(
                "{dropped} faces have no area once rounded to whole world units and are dropped"
            ),
        ));
    }
    Ok(Converted {
        faces,
        stats,
        notes,
    })
}

/// The corners of `polygon` scaled to world units and texels; `None` for fewer than 3 corners
/// or an index outside the mesh; `model.uv-missing` for a corner without uv.
fn corners_of(
    mesh: &Mesh,
    polygon: &Polygon,
    unit: f64,
    binding: &Binding,
    name: &str,
) -> Result<Option<Vec<Corner>>, PartMessage> {
    if polygon.corners.len() < 3 {
        return Ok(None);
    }
    let size = [f64::from(binding.width), f64::from(binding.height)];
    let mut out = Vec::with_capacity(polygon.corners.len());
    for &(p, uv) in &polygon.corners {
        let Some(uv) = uv else {
            return Err(PartMessage::new(
                PartCode::ModelUvMissing,
                &[&name],
                format!("faces with the material {name:?} have no texture coordinates"),
            ));
        };
        let (Some(p), Some(uv)) = (mesh.positions.get(p as usize), mesh.uvs.get(uv as usize))
        else {
            return Ok(None);
        };
        out.push((
            p.map(|v| f64::from(v) * unit),
            [f64::from(uv[0]) * size[0], f64::from(uv[1]) * size[1]],
        ));
    }
    Ok(Some(out))
}

/// `model.out-of-range` for the first corner that rounds outside −512..=511.
fn check_range(corners: &[Corner]) -> Result<(), PartMessage> {
    let range = f64::from(WORLD_MIN)..=f64::from(WORLD_MAX);
    let Some(corner) = corners
        .iter()
        .map(|c| c.0.map(f64::round))
        .find(|c| c.iter().any(|v| !range.contains(v)))
    else {
        return Ok(());
    };
    // Whole numbers without a sign on zero; NaN and infinity as Rust writes them.
    let [x, y, z] = corner.map(|v| {
        if v.is_finite() {
            (v as i64).to_string()
        } else {
            v.to_string()
        }
    });
    Err(PartMessage::new(
        PartCode::ModelOutOfRange,
        &[&x, &y, &z, &WORLD_MIN, &WORLD_MAX],
        format!("a corner lies at ({x}, {y}, {z}) world units, outside {WORLD_MIN}..={WORLD_MAX}"),
    ))
}

/// The face of the corners `piece` (3 or 4 indices into `corners`), rounded; a triangle repeats
/// corner 2 as corner 3.
fn face_of(piece: &[usize], corners: &[Corner], texture: &str) -> Face {
    let corner = |k: usize| corners[piece[k.min(piece.len() - 1)]];
    let at = |k: usize| corner(k).0.map(|v| v.round() as i32);
    let uv = |k: usize| corner(k).1.map(|v| v.round() as i32);
    Face {
        corners: [at(0), at(1), at(2), at(3)],
        uv: [uv(0), uv(1), uv(2), uv(3)],
        texture: texture.to_string(),
        layer: 0,
    }
}

/// Whether `face` has an area: one of its triangles `c0 c1 c2`, `c0 c2 c3` is not degenerate.
fn has_area(face: &Face) -> bool {
    let c = face.corners.map(|p| p.map(i64::from));
    let sub = |a: [i64; 3], b: [i64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [i64; 3], b: [i64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    cross(sub(c[1], c[0]), sub(c[2], c[0])) != [0; 3]
        || cross(sub(c[2], c[0]), sub(c[3], c[0])) != [0; 3]
}

/// Whether the quad `p` stays a quad: corner 3 at most `PLANE_TOLERANCE` off the plane of
/// corners 0, 1, 2, and every corner turning the same way about its normal (strictly convex).
fn flat_convex(p: &[[f64; 3]; 4]) -> bool {
    let plane = cross(sub(p[1], p[0]), sub(p[2], p[0]));
    let len = dot(plane, plane).sqrt();
    if len == 0.0 || dot(sub(p[3], p[0]), plane).abs() / len > PLANE_TOLERANCE {
        return false;
    }
    let n = newell(p);
    (0..4).all(|k| {
        let e1 = sub(p[(k + 1) % 4], p[k]);
        let e2 = sub(p[(k + 2) % 4], p[(k + 1) % 4]);
        dot(cross(e1, e2), n) > 0.0
    })
}

/// Triangles covering the polygon `points` (counter-clockwise about its Newell normal), in the
/// winding of the polygon: `n − 2` of them, a fan of what is left when no ear is found (a
/// self-intersecting polygon) and a fan of all when the polygon has no normal.
pub(crate) fn ear_clip(points: &[[f64; 3]]) -> Vec<[usize; 3]> {
    let fan = |rest: &[usize]| -> Vec<[usize; 3]> {
        (1..rest.len().saturating_sub(1))
            .map(|k| [rest[0], rest[k], rest[k + 1]])
            .collect()
    };
    let mut rest: Vec<usize> = (0..points.len()).collect();
    let n = newell(points);
    let len = dot(n, n).sqrt();
    if len.is_nan() || len == 0.0 {
        return fan(&rest);
    }
    // A right-handed basis (u, v, w) with w along the normal: counter-clockwise stays so in 2D.
    let w = n.map(|c| c / len);
    let axis = if w[0].abs() <= w[1].abs() && w[0].abs() <= w[2].abs() {
        [1.0, 0.0, 0.0]
    } else if w[1].abs() <= w[2].abs() {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let u = cross(axis, w);
    let u = u.map(|c| c / dot(u, u).sqrt());
    let v = cross(w, u);
    let flat: Vec<[f64; 2]> = points.iter().map(|&p| [dot(p, u), dot(p, v)]).collect();
    let mut out = Vec::with_capacity(points.len().saturating_sub(2));
    while rest.len() > 3 {
        let m = rest.len();
        let Some(k) = (0..m).find(|&k| is_ear(&flat, &rest, k)) else {
            out.extend(fan(&rest));
            return out;
        };
        out.push([rest[(k + m - 1) % m], rest[k], rest[(k + 1) % m]]);
        rest.remove(k);
    }
    out.extend(fan(&rest));
    out
}

/// Twice the signed area of the 2D triangle `a b c` (positive counter-clockwise).
fn turn(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Whether corner `k` of `rest` is an ear: convex, and no other corner in or on its triangle.
fn is_ear(flat: &[[f64; 2]], rest: &[usize], k: usize) -> bool {
    let m = rest.len();
    let (a, b, c) = (
        flat[rest[(k + m - 1) % m]],
        flat[rest[k]],
        flat[rest[(k + 1) % m]],
    );
    if turn(a, b, c) <= 0.0 {
        return false;
    }
    rest.iter().map(|&i| flat[i]).all(|p| {
        p == a
            || p == b
            || p == c
            || turn(a, b, p) < 0.0
            || turn(b, c, p) < 0.0
            || turn(c, a, p) < 0.0
    })
}

/// A directed edge of a triangle: from (position, uv) to (position, uv).
type Edge = ([i32; 3], [i32; 2], [i32; 3], [i32; 2]);

/// Edge `k` of the triangle `f`: from corner k to corner k + 1 (mod 3).
fn edge(f: &Face, k: usize) -> Edge {
    let j = (k + 1) % 3;
    (f.corners[k], f.uv[k], f.corners[j], f.uv[j])
}

/// Triangles merged into quads where allowed: greedily in input order, each triangle with the
/// first later free triangle that has the same texture and layer, the same edge the other way
/// round (positions and uv), and that makes a flat convex quad. The quad takes the place of the
/// first triangle: `[A0, A1, B, A2]` with `A` turned so that the shared edge is `A1 A2`.
pub(crate) fn pair_triangles(faces: Vec<Face>) -> Vec<Face> {
    let mut by_edge: std::collections::HashMap<Edge, Vec<usize>> = Default::default();
    for (i, f) in faces.iter().enumerate().filter(|(_, f)| f.is_triangle()) {
        for k in 0..3 {
            by_edge.entry(edge(f, k)).or_default().push(i);
        }
    }
    let mut taken = vec![false; faces.len()];
    let mut quads: Vec<Option<Face>> = vec![None; faces.len()];
    for i in 0..faces.len() {
        if taken[i] || !faces[i].is_triangle() {
            continue;
        }
        let a = &faces[i];
        let partner = (0..3)
            .flat_map(|k| {
                let (p, s, q, t) = edge(a, k);
                by_edge
                    .get(&(q, t, p, s))
                    .into_iter()
                    .flatten()
                    .map(move |&j| (j, k))
            })
            .filter(|&(j, _)| j > i && !taken[j])
            .filter_map(|(j, k)| quad_of(a, k, &faces[j]).map(|q| (j, q)))
            .min_by_key(|(j, _)| *j);
        if let Some((j, quad)) = partner {
            taken[i] = true;
            taken[j] = true;
            quads[i] = Some(quad);
        }
    }
    faces
        .into_iter()
        .zip(quads)
        .zip(taken)
        .filter_map(|((face, quad), taken)| match (quad, taken) {
            (Some(quad), _) => Some(quad),
            (None, true) => None,
            (None, false) => Some(face),
        })
        .collect()
}

/// The quad of triangle `a` and triangle `b` across edge `k` of `a`, if it may be one.
fn quad_of(a: &Face, k: usize, b: &Face) -> Option<Face> {
    if a.texture != b.texture || a.layer != b.layer {
        return None;
    }
    let (p, s, q, t) = edge(a, k);
    let m = (0..3).find(|&m| edge(b, m) == (q, t, p, s))?;
    let far = (m + 2) % 3;
    let order = [(k + 2) % 3, k, (k + 1) % 3];
    let corners = [
        a.corners[order[0]],
        a.corners[order[1]],
        b.corners[far],
        a.corners[order[2]],
    ];
    let uv = [a.uv[order[0]], a.uv[order[1]], b.uv[far], a.uv[order[2]]];
    flat_convex(&corners.map(|c| c.map(f64::from))).then(|| Face {
        corners,
        uv,
        texture: a.texture.clone(),
        layer: a.layer,
    })
}

type Vec3 = [f64; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The Newell normal of the polygon `points`: twice its vector area.
fn newell(points: &[Vec3]) -> Vec3 {
    let mut n = [0.0; 3];
    for (k, &p) in points.iter().enumerate() {
        let q = points[(k + 1) % points.len()];
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bake::strips::normal;
    use proptest::prelude::*;

    fn bind(name: &str, width: u16, height: u16) -> Option<Binding> {
        Some(Binding {
            texture: name.to_string(),
            width,
            height,
        })
    }

    fn material(name: &str) -> MeshMaterial {
        MeshMaterial {
            name: name.to_string(),
            image: None,
        }
    }

    /// A mesh of `positions` with one uv per position and `polygons` of material 0.
    fn mesh(positions: &[[f32; 3]], uvs: &[[f32; 2]], polygons: &[&[u32]]) -> Mesh {
        Mesh {
            positions: positions.to_vec(),
            uvs: uvs.to_vec(),
            polygons: polygons
                .iter()
                .map(|p| Polygon {
                    material: Some(0),
                    corners: p.iter().map(|&i| (i, Some(i))).collect(),
                })
                .collect(),
            materials: vec![material("t")],
        }
    }

    const QUAD: [[f32; 3]; 4] = [[0., 0., 0.], [0., 0., 1.], [1., 0., 1.], [1., 0., 0.]];
    const QUAD_UV: [[f32; 2]; 4] = [[0., 0.], [0., 1.], [1., 1.], [1., 0.]];

    fn convert(m: &Mesh, unit: f32) -> Converted {
        to_faces(m, unit, &[bind("t", 16, 16)]).unwrap()
    }

    /// Twice the area of the triangle `a b c`.
    fn area2(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
        let n = cross(sub(b, a), sub(c, a));
        dot(n, n).sqrt()
    }

    /// The area of `faces` (a quad as two triangles).
    fn face_area(faces: &[Face]) -> f64 {
        let p = |c: [i32; 3]| c.map(f64::from);
        faces
            .iter()
            .map(|f| {
                let c = f.corners.map(p);
                (area2(c[0], c[1], c[2]) + area2(c[0], c[2], c[3])) / 2.0
            })
            .sum()
    }

    fn triangles_area(points: &[[f64; 3]], tris: &[[usize; 3]]) -> f64 {
        tris.iter()
            .map(|t| area2(points[t[0]], points[t[1]], points[t[2]]) / 2.0)
            .sum()
    }

    #[test]
    fn a_unit_quad_becomes_one_face_scaled_by_unit() {
        let c = convert(&mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2, 3]]), 16.0);
        assert_eq!(
            c.faces,
            [Face {
                corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
                uv: [[0, 0], [0, 16], [16, 16], [16, 0]],
                texture: "t".to_string(),
                layer: 0,
            }]
        );
        assert_eq!(
            c.stats,
            MeshStats {
                polygons: 1,
                quads: 1,
                triangles: 0,
                dropped: 0
            }
        );
        assert!(c.notes.is_empty());
    }

    #[test]
    fn uv_is_scaled_by_the_bound_texture() {
        let uvs = [[0.5, 0.25]; 4];
        let m = mesh(&QUAD, &uvs, &[&[0, 1, 2, 3]]);
        let c = to_faces(&m, 16.0, &[bind("t", 32, 16)]).unwrap();
        assert_eq!(c.faces[0].uv, [[16, 4]; 4]);
    }

    #[test]
    fn rounding_goes_to_the_nearest_world_unit() {
        let x = |v: f32| {
            let mut p = QUAD;
            p[0][0] = v;
            convert(&mesh(&p, &QUAD_UV, &[&[0, 1, 2, 3]]), 16.0).faces[0].corners[0][0]
        };
        assert_eq!(x(0.03), 0);
        assert_eq!(x(0.04), 1);
        assert_eq!(x(-0.04), -1);
    }

    #[test]
    fn two_coplanar_triangles_sharing_an_edge_become_one_quad() {
        let c = convert(&mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2], &[0, 2, 3]]), 16.0);
        assert_eq!(c.faces.len(), 1);
        let quad = &c.faces[0];
        let v = [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]];
        assert!(
            (0..4).any(|r| (0..4).all(|k| quad.corners[k] == v[(k + r) % 4])),
            "{quad:?}"
        );
        assert_eq!(normal(quad), [0, 511, 0]);
        assert_eq!((c.stats.quads, c.stats.triangles), (1, 0));
    }

    #[test]
    fn triangles_are_not_paired_across_a_crease_a_uv_seam_a_material_or_into_a_concave_quad() {
        let two_triangles = |c: Converted| {
            assert_eq!(c.faces.len(), 2, "{:?}", c.faces);
            assert!(c.faces.iter().all(Face::is_triangle));
            assert_eq!((c.stats.quads, c.stats.triangles), (0, 2));
        };
        // A crease: corner 3 one unit (16 E) up.
        let mut crease = QUAD;
        crease[3][1] = 1.0;
        two_triangles(convert(
            &mesh(&crease, &QUAD_UV, &[&[0, 1, 2], &[0, 2, 3]]),
            16.0,
        ));
        // A uv seam: the second triangle has its own uv at the shared corners.
        let mut seam = mesh(
            &QUAD,
            &[QUAD_UV.to_vec(), vec![[0.5, 0.5]]].concat(),
            &[&[0, 1, 2]],
        );
        seam.polygons.push(Polygon {
            material: Some(0),
            corners: vec![(0, Some(4)), (2, Some(2)), (3, Some(3))],
        });
        two_triangles(convert(&seam, 16.0));
        // Two materials.
        let mut two = mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2], &[0, 2, 3]]);
        two.materials.push(material("u"));
        two.polygons[1].material = Some(1);
        two_triangles(to_faces(&two, 16.0, &[bind("t", 16, 16), bind("u", 16, 16)]).unwrap());
        // An arrow: the shared edge runs from the tip to the notch.
        let arrow = [[0., 0., 0.], [2., 0., 1.], [0., 0., 2.], [1., 0., 1.]];
        let uv = [[0., 0.]; 4];
        two_triangles(convert(&mesh(&arrow, &uv, &[&[0, 3, 1], &[3, 2, 1]]), 16.0));
    }

    #[test]
    fn a_non_planar_or_concave_quad_is_split() {
        // Corner 3 is 4 E above the plane of the other three.
        let mut bent = QUAD;
        bent[3][1] = 0.25;
        let c = convert(&mesh(&bent, &QUAD_UV, &[&[0, 1, 2, 3]]), 16.0);
        assert_eq!(c.faces.len(), 2);
        assert!(c.faces.iter().all(Face::is_triangle));
        // An arrow (concave at corner 3).
        let arrow = [[0., 0., 0.], [0., 0., 2.], [2., 0., 1.], [1., 0., 1.]];
        let flat = arrow.map(|p| p.map(|v| f64::from(v) * 16.0));
        let tris = ear_clip(&flat);
        assert_eq!(tris.len(), 2);
        let polygon = dot(newell(&flat), newell(&flat)).sqrt() / 2.0;
        assert!((triangles_area(&flat, &tris) - polygon).abs() < 1e-9);
        let c = convert(&mesh(&arrow, &[[0., 0.]; 4], &[&[0, 1, 2, 3]]), 16.0);
        assert_eq!(c.faces.len(), 2);
        assert!(c.faces.iter().all(Face::is_triangle));
        assert!((face_area(&c.faces) - polygon).abs() < 1e-9);
    }

    #[test]
    fn an_ngon_is_ear_clipped() {
        // An L: 2 × 2 with the corner square (1..2, 1..2) cut away, counter-clockwise from above.
        let l: [[f32; 3]; 6] = [
            [0., 0., 0.],
            [0., 0., 2.],
            [2., 0., 2.],
            [2., 0., 1.],
            [1., 0., 1.],
            [1., 0., 0.],
        ];
        let flat = l.map(|p| p.map(|v| f64::from(v) * 16.0));
        let tris = ear_clip(&flat);
        assert_eq!(tris.len(), 4);
        let area = 3.0 * 256.0;
        assert!((triangles_area(&flat, &tris) - area).abs() < 1e-9);
        let c = convert(&mesh(&l, &[[0., 0.]; 6], &[&[0, 1, 2, 3, 4, 5]]), 16.0);
        assert!((face_area(&c.faces) - area).abs() < 1e-9);
        assert!(c.faces.iter().all(|f| normal(f) == [0, 511, 0]));
        assert_eq!(c.stats.polygons, 1);
    }

    proptest! {
        #[test]
        fn ear_clip_covers_every_convex_polygon(
            n in 3usize..=12,
            radius in 1.0f64..=100.0,
            turn in 0.0f64..std::f64::consts::TAU,
            tilt in 0.0f64..std::f64::consts::PI,
        ) {
            // A regular n-gon in the plane z = 0, turned in its plane and tilted about x.
            let points: Vec<[f64; 3]> = (0..n)
                .map(|k| {
                    let a = turn + std::f64::consts::TAU * k as f64 / n as f64;
                    let (x, y) = (radius * a.cos(), radius * a.sin());
                    [x, y * tilt.cos(), y * tilt.sin()]
                })
                .collect();
            let tris = ear_clip(&points);
            prop_assert_eq!(tris.len(), n - 2);
            let polygon = dot(newell(&points), newell(&points)).sqrt() / 2.0;
            let sum = triangles_area(&points, &tris);
            prop_assert!((sum - polygon).abs() <= 1e-6 * polygon, "{} vs {}", sum, polygon);
        }
    }

    #[test]
    fn faces_that_collapse_when_rounded_are_dropped_with_a_note() {
        // At unit 1 the second triangle's corners round to (0,0,0), (0,0,0) and (0,0,1).
        let positions = [
            [0., 0., 0.],
            [0., 0., 5.],
            [5., 0., 5.],
            [0.2, 0., 0.],
            [0., 0., 0.4],
            [0., 0., 1.],
        ];
        let m = mesh(&positions, &[[0., 0.]; 6], &[&[0, 1, 2], &[3, 4, 5]]);
        let c = to_faces(&m, 1.0, &[bind("t", 16, 16)]).unwrap();
        assert_eq!(c.faces.len(), 1);
        assert_eq!(c.stats.dropped, 1);
        assert_eq!(c.notes.len(), 1);
        assert_eq!(c.notes[0].code, PartCode::ModelFacesDropped);
        assert_eq!(c.notes[0].args, ["1"]);
    }

    #[test]
    fn a_model_that_collapses_entirely_has_no_geometry() {
        let positions = [[0.2, 0., 0.], [0., 0., 0.4], [0., 0., 1.]];
        let m = mesh(&positions, &[[0., 0.]; 3], &[&[0, 1, 2]]);
        let e = to_faces(&m, 1.0, &[bind("t", 16, 16)]).unwrap_err();
        assert_eq!(e.code, PartCode::ModelNoGeometry);
        let empty = Mesh::default();
        let e = to_faces(&empty, 16.0, &[]).unwrap_err();
        assert_eq!(e.code, PartCode::ModelNoGeometry);
    }

    #[test]
    fn a_polygon_without_binding_is_refused() {
        let m = mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2, 3]]);
        let e = to_faces(&m, 16.0, &[None]).unwrap_err();
        assert_eq!(
            (e.code, e.args.as_slice()),
            (PartCode::ModelMaterialUntextured, &["t".to_string()][..])
        );
        let mut none = m.clone();
        none.polygons[0].material = None;
        let e = to_faces(&none, 16.0, &[bind("t", 16, 16)]).unwrap_err();
        assert_eq!(
            (e.code, e.args.as_slice()),
            (PartCode::ModelMaterialUntextured, &[String::new()][..])
        );
    }

    #[test]
    fn a_textured_polygon_without_uv_is_refused() {
        let mut m = mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2, 3]]);
        m.polygons[0].corners[2].1 = None;
        let e = to_faces(&m, 16.0, &[bind("t", 16, 16)]).unwrap_err();
        assert_eq!(
            (e.code, e.args.as_slice()),
            (PartCode::ModelUvMissing, &["t".to_string()][..])
        );
    }

    #[test]
    fn corners_outside_the_map_range_are_refused() {
        let mut p = QUAD;
        p[2] = [40., 0., 0.];
        let e = to_faces(
            &mesh(&p, &QUAD_UV, &[&[0, 1, 2, 3]]),
            16.0,
            &[bind("t", 16, 16)],
        )
        .unwrap_err();
        assert_eq!(e.code, PartCode::ModelOutOfRange);
        assert_eq!(e.args, ["640", "0", "0", "-512", "511"]);
    }

    #[test]
    fn winding_is_kept() {
        let up = convert(&mesh(&QUAD, &QUAD_UV, &[&[0, 1, 2, 3]]), 16.0);
        assert_eq!(normal(&up.faces[0]), [0, 511, 0]);
        let down = convert(&mesh(&QUAD, &QUAD_UV, &[&[3, 2, 1, 0]]), 16.0);
        assert_eq!(normal(&down.faces[0]), [0, -511, 0]);
    }

    fn triangle(c: [[i32; 3]; 3]) -> Face {
        Face {
            corners: [c[0], c[1], c[2], c[2]],
            uv: [[0, 0]; 4],
            texture: "t".to_string(),
            layer: 0,
        }
    }

    #[test]
    fn pairing_keeps_the_order_and_leaves_quads_and_lone_triangles() {
        let a = triangle([[0, 0, 0], [0, 0, 16], [16, 0, 16]]);
        let other = triangle([[100, 0, 0], [100, 0, 16], [116, 0, 16]]);
        let b = triangle([[0, 0, 0], [16, 0, 16], [16, 0, 0]]);
        let out = pair_triangles(vec![a.clone(), other.clone(), b]);
        assert_eq!(out.len(), 2);
        // `a` turned so that the shared edge is corner 1 to 2, then the far corner of `b`.
        assert_eq!(
            out[0].corners,
            [[0, 0, 16], [16, 0, 16], [16, 0, 0], [0, 0, 0]]
        );
        assert!(!out[0].is_triangle());
        assert_eq!(out[1], other);
        let quad = Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
            ..a.clone()
        };
        assert_eq!(pair_triangles(vec![quad.clone(), a.clone()]), [quad, a]);
    }
}
