//! The reader of the baker's subset: a baked NSBMD back to its quads, for tests (the floor of
//! the model against the BDHC, the mesh against the faces given).
//!
//! It mirrors `encode` and knows nothing past it: the render commands `MAT` (0x04) and `SHP`
//! (0x05) pair a shape with its material, the shape's display list holds `BEGIN_VTXS` groups of
//! quads or quad strips with `TEXCOORD`, `NORMAL`, `COLOR`, `VTX_10`, `VTX_16` and `END_VTXS`
//! (and the `NOP` padding of the last command group). A strip of `2n + 2` corners is `n` quads:
//! quad `k` has the corners `v[2k], v[2k + 1], v[2k + 3], v[2k + 2]`, the order `encode` sends
//! them in (`c0, c1, c3, c2`, then `c3, c2` of every next face). A quad with `c3 == c2` (the
//! triangle-shaped face) reads like any other.

use g4_formats::error::FormatError;
use g4_formats::nitro_dict::name_text;
use g4_formats::nsbmd::Nsbmd;
use g4_formats::nsbmd::gpu::{GpuCmd, Prim, op};
use g4_formats::nsbmd::model::Model;

use super::POS_SCALE;

/// One quad of a baked model: corners in world units, uv in texels as sent (`TEXCOORD` / 16),
/// the name of its material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedQuad {
    pub corners: [[i32; 3]; 4],
    pub uv: [[i32; 2]; 4],
    pub material: String,
}

/// `MAT` in the render commands: its parameter is the material of the next shapes.
const SBC_MAT: u8 = 0x04;
/// `SHP` in the render commands: draws the shape of its parameter.
const SBC_SHP: u8 = 0x05;

/// The quads of a model `bake` wrote, in world units and texels (strips expanded, display-list order): the
/// inverse of `encode` for its subset (BEGIN quads / quad strip, TEXCOORD, NORMAL, COLOR, VTX_10, VTX_16, END);
/// any other command is `FormatError::Unsupported`.
pub fn decode_quads(nsbmd: &[u8]) -> Result<Vec<BakedQuad>, FormatError> {
    let file = Nsbmd::read(nsbmd)?;
    let Some((_, model)) = file.models.first() else {
        return Err(FormatError::Unsupported {
            what: "an NSBMD without a model",
        });
    };
    let mut out = Vec::new();
    let mut material = None;
    for sbc in &model.sbc {
        let Some(&index) = sbc.params.first() else {
            continue;
        };
        match sbc.op {
            SBC_MAT => material = Some(usize::from(index)),
            SBC_SHP => {
                let name = material_name(model, material)?;
                let shape =
                    model
                        .shapes
                        .get(usize::from(index))
                        .ok_or(FormatError::OutOfRange {
                            what: "shape index",
                            value: usize::from(index),
                            max: model.shapes.len(),
                        })?;
                shape_quads(&shape.commands, &name, &mut out)?;
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The name of the material `MAT` chose last.
fn material_name(model: &Model, material: Option<usize>) -> Result<String, FormatError> {
    let index = material.ok_or(FormatError::Unsupported {
        what: "a shape drawn before any material",
    })?;
    let m = model.materials.get(index).ok_or(FormatError::OutOfRange {
        what: "material index",
        value: index,
        max: model.materials.len(),
    })?;
    Ok(name_text(&m.name))
}

/// One corner as a display list sends it.
type Corner = ([i32; 3], [i32; 2]);

/// The primitive of an open `BEGIN_VTXS` group and its corners so far.
struct Group {
    prim: Prim,
    corners: Vec<Corner>,
}

/// Appends the quads of one shape's display list to `out`.
fn shape_quads(
    cmds: &[GpuCmd],
    material: &str,
    out: &mut Vec<BakedQuad>,
) -> Result<(), FormatError> {
    let mut st = [0i32; 2];
    let mut group: Option<Group> = None;
    for c in cmds {
        match c.op {
            op::NOP | op::NORMAL | op::COLOR => {}
            op::TEXCOORD => {
                let v = param(c, 0)?;
                st = [signed(v, 16) / 16, signed(v >> 16, 16) / 16];
            }
            op::VTX_10 | op::VTX_16 => {
                let g = group.as_mut().ok_or(FormatError::Unsupported {
                    what: "a vertex outside BEGIN_VTXS",
                })?;
                g.corners.push((position(c)?, st));
            }
            op::BEGIN_VTXS => {
                if group.is_some() {
                    return Err(FormatError::Unsupported {
                        what: "BEGIN_VTXS inside a group",
                    });
                }
                group = Some(Group {
                    prim: prim(param(c, 0)?)?,
                    corners: Vec::new(),
                });
            }
            op::END_VTXS => {
                let g = group.take().ok_or(FormatError::Unsupported {
                    what: "END_VTXS without BEGIN_VTXS",
                })?;
                close(g, material, out)?;
            }
            _ => {
                return Err(FormatError::Unsupported {
                    what: "a GPU command outside the baker's subset",
                });
            }
        }
    }
    match group {
        Some(g) => close(g, material, out),
        None => Ok(()),
    }
}

/// The primitive types the baker writes.
fn prim(value: u32) -> Result<Prim, FormatError> {
    match value {
        1 => Ok(Prim::Quads),
        3 => Ok(Prim::QuadStrip),
        _ => Err(FormatError::Unsupported {
            what: "a primitive type outside the baker's subset",
        }),
    }
}

/// The quads of a finished group, in corner order.
fn close(g: Group, material: &str, out: &mut Vec<BakedQuad>) -> Result<(), FormatError> {
    let quad = |c: [&Corner; 4]| BakedQuad {
        corners: c.map(|v| v.0),
        uv: c.map(|v| v.1),
        material: material.to_owned(),
    };
    let v = &g.corners;
    match g.prim {
        Prim::Quads => {
            if !v.len().is_multiple_of(4) {
                return Err(FormatError::Unsupported {
                    what: "a quads group with a corner count not divisible by 4",
                });
            }
            let (quads, _) = v.as_chunks::<4>();
            out.extend(quads.iter().map(|[a, b, c, d]| quad([a, b, c, d])));
        }
        Prim::QuadStrip => {
            if v.len() < 4 || !v.len().is_multiple_of(2) {
                return Err(FormatError::Unsupported {
                    what: "a quad strip with an odd or short corner count",
                });
            }
            for k in 0..(v.len() - 2) / 2 {
                let at = 2 * k;
                out.push(quad([&v[at], &v[at + 1], &v[at + 3], &v[at + 2]]));
            }
        }
        // `prim` lets no other type through.
        Prim::Triangles | Prim::TriangleStrip => {
            return Err(FormatError::Unsupported {
                what: "a primitive type outside the baker's subset",
            });
        }
    }
    Ok(())
}

/// The corner of a `VTX_10` (world units) or `VTX_16` (1/4096 model units, `POS_SCALE` per
/// world unit) in world units.
fn position(c: &GpuCmd) -> Result<[i32; 3], FormatError> {
    let a = param(c, 0)?;
    if c.op == op::VTX_10 {
        return Ok([signed(a, 10), signed(a >> 10, 10), signed(a >> 20, 10)]);
    }
    let b = param(c, 1)?;
    Ok([signed(a, 16), signed(a >> 16, 16), signed(b, 16)].map(|v| v / POS_SCALE))
}

fn param(c: &GpuCmd, i: usize) -> Result<u32, FormatError> {
    c.params.get(i).copied().ok_or(FormatError::Truncated {
        needed: 4 * (i + 1),
        have: 4 * c.params.len(),
    })
}

/// The signed value of the low `bits` bits of `v`.
fn signed(v: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((v << shift) as i32) >> shift
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::{AreaStyle, BakeInput, Face, PackTexture, VANILLA_VRAM_MAX_PT, bake};
    use super::*;
    use crate::bake::test_surface;
    use crate::world::bake::synthetic_pack_textures;
    use crate::world::doc::fixtures::checkerboard_world;
    use crate::world::mesh::mesh;
    use crate::world::{Tileset, WorldGrid};

    fn tex(n: &str) -> PackTexture {
        PackTexture {
            name: n.to_string(),
            palette: format!("{n}_pl"),
            width: 16,
            height: 16,
        }
    }

    fn pack() -> Vec<PackTexture> {
        ["t0", "t1", "t2", "t3", "blk"].map(tex).to_vec()
    }

    fn input(textures: &[PackTexture]) -> BakeInput<'_> {
        BakeInput {
            style: AreaStyle::Outdoor,
            textures,
            vram_bytes: 0,
            vram_limit: VANILLA_VRAM_MAX_PT,
        }
    }

    type Key = ([[i32; 3]; 4], [[i32; 2]; 4], String);

    /// The multiset of (corners, uv, material).
    fn multiset(items: impl Iterator<Item = Key>) -> BTreeMap<Key, usize> {
        let mut out = BTreeMap::new();
        for k in items {
            *out.entry(k).or_insert(0) += 1;
        }
        out
    }

    fn of_faces(faces: &[Face]) -> BTreeMap<Key, usize> {
        multiset(faces.iter().map(|f| (f.corners, f.uv, f.texture.clone())))
    }

    fn of_quads(quads: &[BakedQuad]) -> BTreeMap<Key, usize> {
        multiset(quads.iter().map(|q| (q.corners, q.uv, q.material.clone())))
    }

    fn baked(faces: &[Face], textures: &[PackTexture]) -> Vec<u8> {
        let r = bake(faces, &input(textures));
        assert_eq!(r.refusal(), None);
        r.nsbmd
    }

    #[test]
    fn decode_quads_returns_the_input_faces() {
        let pack = pack();
        let surface = test_surface(["t0", "t1", "t2", "t3"], "blk", (15, 15));
        let quads = decode_quads(&baked(&surface, &pack)).unwrap();
        assert_eq!(quads.len(), surface.len());
        assert_eq!(of_quads(&quads), of_faces(&surface));

        let grid = WorldGrid::parse(&checkerboard_world()).unwrap();
        let faces: Vec<Face> = mesh(&grid, &Tileset::synthetic())
            .into_iter()
            .map(Face::from)
            .collect();
        let syn = synthetic_pack_textures();
        let quads = decode_quads(&baked(&faces, &syn)).unwrap();
        assert_eq!(quads.len(), 1024);
        assert_eq!(of_quads(&quads), of_faces(&faces));
    }

    #[test]
    fn decode_reads_strips_in_the_order_encode_sends() {
        // A row of four tiles of one texture: one strip, four quads in row order.
        let pack = pack();
        let row: Vec<Face> = (0..4)
            .map(|tx| {
                let x0 = tx * 16;
                let corners = [[x0, 0, 0], [x0, 0, 16], [x0 + 16, 0, 16], [x0 + 16, 0, 0]];
                Face {
                    corners,
                    uv: corners.map(|[x, _, z]| [x, z]),
                    texture: "t0".to_string(),
                    layer: 0,
                }
            })
            .collect();
        let r = bake(&row, &input(&pack));
        assert_eq!((r.strips, r.single_quads), (1, 0));
        let quads = decode_quads(&r.nsbmd).unwrap();
        let got: Vec<_> = quads.iter().map(|q| (q.corners, q.uv)).collect();
        let want: Vec<_> = row.iter().map(|f| (f.corners, f.uv)).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn decode_tolerates_a_quad_with_c3_equal_c2() {
        let pack = pack();
        let tri = Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 16]],
            uv: [[0, 0], [0, 16], [16, 16], [16, 16]],
            texture: "t0".to_string(),
            layer: 0,
        };
        let next = Face {
            corners: [[16, 0, 0], [16, 0, 16], [32, 0, 16], [32, 0, 0]],
            uv: [[16, 0], [16, 16], [32, 16], [32, 0]],
            texture: "t1".to_string(),
            layer: 0,
        };
        let quads = decode_quads(&baked(&[tri.clone(), next.clone()], &pack)).unwrap();
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].corners[3], quads[0].corners[2]);
        assert_eq!(of_quads(&quads), of_faces(&[tri, next]));
    }

    /// The baked model of one tile with its first shape's commands changed by `edit`.
    fn edited(edit: impl FnOnce(&mut Vec<GpuCmd>)) -> Vec<u8> {
        let pack = pack();
        let corners = [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]];
        let face = Face {
            corners,
            uv: corners.map(|[x, _, z]| [x, z]),
            texture: "t0".to_string(),
            layer: 0,
        };
        let mut file = Nsbmd::read(&baked(&[face], &pack)).unwrap();
        edit(&mut file.models[0].1.shapes[0].commands);
        file.write().unwrap()
    }

    #[test]
    fn decode_reads_vtx_16_in_world_units() {
        let bytes = edited(|cmds| {
            let at = cmds.iter().position(|c| c.op == op::VTX_10).unwrap();
            let p = cmds[at].params[0];
            let world = [signed(p, 10), signed(p >> 10, 10), signed(p >> 20, 10)];
            cmds[at] = GpuCmd::vtx_16(world.map(|v| v * POS_SCALE));
        });
        let quads = decode_quads(&bytes).unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(
            quads[0].corners,
            [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]]
        );
    }

    #[test]
    fn decode_refuses_another_command() {
        let bytes = edited(|cmds| {
            let at = cmds.iter().position(|c| c.op == op::VTX_10).unwrap();
            cmds.insert(
                at,
                GpuCmd {
                    op: op::VTX_XY,
                    params: vec![0],
                },
            );
        });
        assert!(matches!(
            decode_quads(&bytes),
            Err(FormatError::Unsupported { .. })
        ));

        // Triangles are not written by the baker (yet).
        let bytes = edited(|cmds| {
            let at = cmds.iter().position(|c| c.op == op::BEGIN_VTXS).unwrap();
            cmds[at] = GpuCmd::begin(Prim::Triangles);
        });
        assert!(matches!(
            decode_quads(&bytes),
            Err(FormatError::Unsupported { .. })
        ));
    }
}
