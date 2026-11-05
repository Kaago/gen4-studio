//! One MDL0 model: head, bones, SBC, materials, texture and palette pairings, shapes, EVP.
//! See `docs/formats/nsbmd.md`.

use std::ops::Range;

use super::gpu::{self, Geometry, GpuCmd};
use super::sbc::{self, SbcOp};
use super::{Reader, put_u16, put_u32, u16_of, u32_of};
use crate::error::FormatError;
use crate::nitro_dict::{Dict, Name};

/// Length of the model head; the bone dictionary follows it.
pub const HEAD_LEN: usize = 0x40;
/// Material fields before `extra`.
const MATERIAL_LEN: usize = 44;
/// Material sizes the codec knows: without, with a 4- or 8-byte texture matrix.
const MATERIAL_SIZES: [usize; 3] = [44, 48, 52];
/// A shape head: `u16 tag, u16 size, u32 flag, u32` list offset from the head, `u32` length.
const SHAPE_HEAD_LEN: usize = 16;
/// Datum size of every model dictionary (an offset, or `u16 off, u8 n, u8 bound`).
const UNIT: u16 = 4;

/// A model of the MDL0 block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub head: ModelHead,
    pub bones: Vec<Bone>,
    pub sbc: Vec<SbcOp>,
    pub materials: Vec<Material>,
    pub textures: Vec<Pairing>,
    pub palettes: Vec<Pairing>,
    pub shapes: Vec<Shape>,
    pub evp: Vec<u8>,
}

/// The fields 0x14 to 0x3F of the model head, as stored (never recomputed).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelHead {
    pub sbc_type: u8,
    pub scaling_rule: u8,
    pub texmtx_mode: u8,
    pub num_objects: u8,
    pub num_materials: u8,
    pub num_shapes: u8,
    pub first_unused_mtx: u8,
    pub pad: u8,
    pub up_scale: i32,
    pub down_scale: i32,
    pub num_verts: u16,
    pub num_polys: u16,
    pub num_tris: u16,
    pub num_quads: u16,
    pub box_pos: [i16; 3],
    pub box_size: [i16; 3],
    pub box_scale: i32,
    pub box_scale_inv: i32,
}

/// A bone (node): its flag word and the bytes after it, `bone_len(flags) - 2` long.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bone {
    pub name: Name,
    pub flags: u16,
    pub rest: Vec<u8>,
}

/// 4 + 12 (bit 0 clear) + 4 (bit 3 set) or 16 (bit 3 and bit 1 clear) + 24 (bit 2 clear: scale and inverse).
pub fn bone_len(flags: u16) -> usize {
    let mut len = 4;
    if flags & 1 == 0 {
        len += 12;
    }
    if flags & 8 != 0 {
        len += 4;
    } else if flags & 2 == 0 {
        len += 16;
    }
    if flags & 4 == 0 {
        len += 24;
    }
    len
}

/// A material; its size field is `44 + extra.len()` (44, 48 or 52).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    pub name: Name,
    pub tag: u16,
    pub dif_amb: u32,
    pub spe_emi: u32,
    pub poly_attr: u32,
    pub poly_attr_mask: u32,
    pub teximage: u32,
    pub teximage_mask: u32,
    pub pltt_base: u16,
    pub flag: u16,
    pub width: u16,
    pub height: u16,
    pub mag: [i32; 2],
    /// Bytes 44 and on (texture matrix), 0, 4 or 8 bytes.
    pub extra: Vec<u8>,
}

/// A texture or palette name and the materials it binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub name: Name,
    pub materials: Vec<u8>,
    pub bound: u8,
}

/// A shape: head fields and its display list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub name: Name,
    pub tag: u16,
    pub size: u16,
    pub flag: u32,
    pub commands: Vec<GpuCmd>,
}

impl Model {
    /// Vertex and polygon counts over every shape.
    pub fn geometry(&self) -> Geometry {
        let mut g = Geometry::default();
        for shape in &self.shapes {
            gpu::count(&shape.commands, &mut g);
        }
        g
    }

    /// Whether the counters of the head equal [`Model::geometry`].
    pub fn counters_match(&self) -> bool {
        let g = self.geometry();
        let h = &self.head;
        g.verts == u32::from(h.num_verts)
            && g.polys == u32::from(h.num_polys)
            && g.tris == u32::from(h.num_tris)
            && g.quads == u32::from(h.num_quads)
    }

    /// The model at the start of `m` (`m` is exactly the model, its size field long);
    /// `base` is the file offset of `m`, the dictionaries go to `spans` as file ranges.
    pub(super) fn read(
        m: &[u8],
        base: usize,
        spans: &mut Vec<Range<usize>>,
    ) -> Result<Model, FormatError> {
        let r = Reader { bytes: m, base };
        let head = r.slice(0, HEAD_LEN)?;
        let at = |i: usize| u32_of(head, 4 * i) as usize;
        let (ofs_sbc, ofs_mat, ofs_shp, ofs_evp) = (at(1), at(2), at(3), at(4));
        let head = read_head(head);

        let bone_dict = r.dict(HEAD_LEN, UNIT, spans)?;
        let bones = (0..bone_dict.names.len())
            .map(|i| {
                let off = r.offset(HEAD_LEN, &bone_dict, i)?;
                let flags = r.u16(off)?;
                let rest = r.slice(off + 2, bone_len(flags) - 2)?.to_vec();
                Ok(Bone {
                    name: bone_dict.names[i],
                    flags,
                    rest,
                })
            })
            .collect::<Result<_, FormatError>>()?;

        let (sbc, _) = sbc::decode(m, ofs_sbc, m.len())?;

        let tex_off = r.add(ofs_mat, r.u16(ofs_mat)? as usize)?;
        let pal_off = r.add(ofs_mat, r.u16(r.add(ofs_mat, 2)?)? as usize)?;
        let mat_dict = r.dict(r.add(ofs_mat, 4)?, UNIT, spans)?;
        let materials = (0..mat_dict.names.len())
            .map(|i| read_material(&r, r.offset(ofs_mat, &mat_dict, i)?, mat_dict.names[i]))
            .collect::<Result<_, FormatError>>()?;
        let textures = read_pairings(&r, ofs_mat, tex_off, spans)?;
        let palettes = read_pairings(&r, ofs_mat, pal_off, spans)?;

        let shape_dict = r.dict(ofs_shp, UNIT, spans)?;
        let shapes = (0..shape_dict.names.len())
            .map(|i| {
                let at = r.offset(ofs_shp, &shape_dict, i)?;
                let h = r.slice(at, SHAPE_HEAD_LEN)?;
                let list = r.add(at, u32_of(h, 8) as usize)?;
                let commands = gpu::decode(r.slice(list, u32_of(h, 12) as usize)?)?;
                Ok(Shape {
                    name: shape_dict.names[i],
                    tag: u16_of(h, 0),
                    size: u16_of(h, 2),
                    flag: u32_of(h, 4),
                    commands,
                })
            })
            .collect::<Result<_, FormatError>>()?;

        let evp = m
            .get(ofs_evp..)
            .ok_or(FormatError::Truncated {
                needed: ofs_evp,
                have: m.len(),
            })?
            .to_vec();
        Ok(Model {
            head,
            bones,
            sbc,
            materials,
            textures,
            palettes,
            shapes,
            evp,
        })
    }

    /// The model in the vanilla layout: head, bone dictionary and bones, SBC (padded to 4),
    /// material section, shape section, EVP. The head fields 0x14 to 0x3F are written as
    /// they are; only the size and the four section offsets are computed.
    pub(super) fn write(&self) -> Result<Vec<u8>, FormatError> {
        let mut out = vec![0; HEAD_LEN];

        let names: Vec<Name> = self.bones.iter().map(|b| b.name).collect();
        let mut at = dict_len(&names)?;
        let mut offsets = Vec::with_capacity(names.len());
        for bone in &self.bones {
            let len = bone_len(bone.flags);
            if bone.rest.len() + 2 != len {
                return Err(FormatError::BadSize {
                    expected: len - 2,
                    found: bone.rest.len(),
                });
            }
            offsets.push(at);
            at += len;
        }
        out.extend_from_slice(&dict(names, &offsets)?.write()?);
        for bone in &self.bones {
            put_u16(&mut out, bone.flags);
            out.extend_from_slice(&bone.rest);
        }

        let ofs_sbc = out.len();
        out.extend_from_slice(&sbc::encode(&self.sbc));
        pad4(&mut out);

        let ofs_mat = out.len();
        out.extend_from_slice(&self.write_materials()?);
        let ofs_shp = out.len();
        out.extend_from_slice(&self.write_shapes()?);
        let ofs_evp = out.len();
        out.extend_from_slice(&self.evp);

        let total = out.len();
        let mut head = Vec::with_capacity(HEAD_LEN);
        for v in [total, ofs_sbc, ofs_mat, ofs_shp, ofs_evp] {
            put_u32(&mut head, word(v, "MDL0 model size")?);
        }
        write_head(&self.head, &mut head);
        out[..HEAD_LEN].copy_from_slice(&head);
        Ok(out)
    }

    /// The material section: `u16` offsets of the two pairing dictionaries, the material,
    /// texture pairing and palette pairing dictionaries, the material indices of every
    /// texture and then every palette pairing (padded to 4), the materials. All offsets
    /// count from the section start.
    fn write_materials(&self) -> Result<Vec<u8>, FormatError> {
        let names: Vec<Name> = self.materials.iter().map(|m| m.name).collect();
        let tex_names: Vec<Name> = self.textures.iter().map(|p| p.name).collect();
        let pal_names: Vec<Name> = self.palettes.iter().map(|p| p.name).collect();
        let tex_off = 4 + dict_len(&names)?;
        let pal_off = tex_off + dict_len(&tex_names)?;
        let ids_at = pal_off + dict_len(&pal_names)?;

        let mut ids = Vec::new();
        let mut pairing_data = |pairings: &[Pairing]| -> Result<Vec<u8>, FormatError> {
            let mut data = Vec::with_capacity(4 * pairings.len());
            for p in pairings {
                put_u16(&mut data, half(ids_at + ids.len())?);
                data.push(u8::try_from(p.materials.len()).map_err(|_| {
                    FormatError::OutOfRange {
                        what: "materials of a pairing",
                        value: p.materials.len(),
                        max: u8::MAX as usize,
                    }
                })?);
                data.push(p.bound);
                ids.extend_from_slice(&p.materials);
            }
            Ok(data)
        };
        let tex_data = pairing_data(&self.textures)?;
        let pal_data = pairing_data(&self.palettes)?;

        let mut at = (ids_at + ids.len()).next_multiple_of(4);
        let mut offsets = Vec::with_capacity(names.len());
        for m in &self.materials {
            let size = MATERIAL_LEN + m.extra.len();
            if !MATERIAL_SIZES.contains(&size) {
                return Err(FormatError::Unsupported {
                    what: "material size",
                });
            }
            offsets.push(at);
            at += size;
        }

        let mut sec = Vec::with_capacity(at);
        put_u16(&mut sec, half(tex_off)?);
        put_u16(&mut sec, half(pal_off)?);
        sec.extend_from_slice(&dict(names, &offsets)?.write()?);
        for (names, data) in [(tex_names, tex_data), (pal_names, pal_data)] {
            sec.extend_from_slice(
                &Dict {
                    names,
                    unit: UNIT,
                    data,
                }
                .write()?,
            );
        }
        sec.extend_from_slice(&ids);
        pad4(&mut sec);
        for m in &self.materials {
            write_material(m, &mut sec);
        }
        Ok(sec)
    }

    /// The shape section: dictionary (datum = offset of the head), the heads, the display
    /// lists in order.
    fn write_shapes(&self) -> Result<Vec<u8>, FormatError> {
        let names: Vec<Name> = self.shapes.iter().map(|s| s.name).collect();
        let heads = dict_len(&names)?;
        let lists = self
            .shapes
            .iter()
            .map(|s| gpu::encode(&s.commands))
            .collect::<Result<Vec<_>, _>>()?;
        let offsets: Vec<usize> = (0..names.len())
            .map(|i| heads + SHAPE_HEAD_LEN * i)
            .collect();
        let mut sec = dict(names, &offsets)?.write()?;
        let mut list_at = heads + SHAPE_HEAD_LEN * self.shapes.len();
        for (i, (shape, list)) in self.shapes.iter().zip(&lists).enumerate() {
            put_u16(&mut sec, shape.tag);
            put_u16(&mut sec, shape.size);
            put_u32(&mut sec, shape.flag);
            put_u32(
                &mut sec,
                word(list_at - offsets[i], "MDL0 display list offset")?,
            );
            put_u32(&mut sec, word(list.len(), "MDL0 display list length")?);
            list_at += list.len();
        }
        for list in &lists {
            sec.extend_from_slice(list);
        }
        Ok(sec)
    }
}

fn read_head(h: &[u8]) -> ModelHead {
    let i16_of = |at: usize| u16_of(h, at) as i16;
    let i32_of = |at: usize| u32_of(h, at) as i32;
    ModelHead {
        sbc_type: h[0x14],
        scaling_rule: h[0x15],
        texmtx_mode: h[0x16],
        num_objects: h[0x17],
        num_materials: h[0x18],
        num_shapes: h[0x19],
        first_unused_mtx: h[0x1A],
        pad: h[0x1B],
        up_scale: i32_of(0x1C),
        down_scale: i32_of(0x20),
        num_verts: u16_of(h, 0x24),
        num_polys: u16_of(h, 0x26),
        num_tris: u16_of(h, 0x28),
        num_quads: u16_of(h, 0x2A),
        box_pos: [i16_of(0x2C), i16_of(0x2E), i16_of(0x30)],
        box_size: [i16_of(0x32), i16_of(0x34), i16_of(0x36)],
        box_scale: i32_of(0x38),
        box_scale_inv: i32_of(0x3C),
    }
}

fn write_head(h: &ModelHead, out: &mut Vec<u8>) {
    out.extend_from_slice(&[
        h.sbc_type,
        h.scaling_rule,
        h.texmtx_mode,
        h.num_objects,
        h.num_materials,
        h.num_shapes,
        h.first_unused_mtx,
        h.pad,
    ]);
    put_u32(out, h.up_scale as u32);
    put_u32(out, h.down_scale as u32);
    for v in [h.num_verts, h.num_polys, h.num_tris, h.num_quads] {
        put_u16(out, v);
    }
    for v in h.box_pos.iter().chain(&h.box_size) {
        put_u16(out, *v as u16);
    }
    put_u32(out, h.box_scale as u32);
    put_u32(out, h.box_scale_inv as u32);
}

fn read_material(r: &Reader, at: usize, name: Name) -> Result<Material, FormatError> {
    let size = r.u16(r.add(at, 2)?)? as usize;
    if !MATERIAL_SIZES.contains(&size) {
        return Err(FormatError::Unsupported {
            what: "material size",
        });
    }
    let b = r.slice(at, size)?;
    Ok(Material {
        name,
        tag: u16_of(b, 0),
        dif_amb: u32_of(b, 4),
        spe_emi: u32_of(b, 8),
        poly_attr: u32_of(b, 12),
        poly_attr_mask: u32_of(b, 16),
        teximage: u32_of(b, 20),
        teximage_mask: u32_of(b, 24),
        pltt_base: u16_of(b, 28),
        flag: u16_of(b, 30),
        width: u16_of(b, 32),
        height: u16_of(b, 34),
        mag: [u32_of(b, 36) as i32, u32_of(b, 40) as i32],
        extra: b[MATERIAL_LEN..].to_vec(),
    })
}

fn write_material(m: &Material, out: &mut Vec<u8>) {
    put_u16(out, m.tag);
    // `write_materials` checked the size against `MATERIAL_SIZES`.
    put_u16(out, (MATERIAL_LEN + m.extra.len()) as u16);
    for v in [
        m.dif_amb,
        m.spe_emi,
        m.poly_attr,
        m.poly_attr_mask,
        m.teximage,
        m.teximage_mask,
    ] {
        put_u32(out, v);
    }
    for v in [m.pltt_base, m.flag, m.width, m.height] {
        put_u16(out, v);
    }
    put_u32(out, m.mag[0] as u32);
    put_u32(out, m.mag[1] as u32);
    out.extend_from_slice(&m.extra);
}

/// The pairing dictionary at `at`; its data `u16 off, u8 n, u8 bound` point at `n`
/// material indices at `section + off`.
fn read_pairings(
    r: &Reader,
    section: usize,
    at: usize,
    spans: &mut Vec<Range<usize>>,
) -> Result<Vec<Pairing>, FormatError> {
    let d = r.dict(at, UNIT, spans)?;
    (0..d.names.len())
        .map(|i| {
            let e = d.entry(i);
            let ids = r.add(section, u16_of(e, 0) as usize)?;
            Ok(Pairing {
                name: d.names[i],
                materials: r.slice(ids, e[2] as usize)?.to_vec(),
                bound: e[3],
            })
        })
        .collect()
}

/// Length of a dictionary of `names` with 4-byte data.
fn dict_len(names: &[Name]) -> Result<usize, FormatError> {
    dict(names.to_vec(), &vec![0; names.len()]).map(|d| d.byte_len())
}

/// A dictionary of `names` whose data are the `u32` `offsets`.
fn dict(names: Vec<Name>, offsets: &[usize]) -> Result<Dict, FormatError> {
    let mut data = Vec::with_capacity(4 * offsets.len());
    for &off in offsets {
        put_u32(&mut data, word(off, "MDL0 offset")?);
    }
    Ok(Dict {
        names,
        unit: UNIT,
        data,
    })
}

fn word(v: usize, what: &'static str) -> Result<u32, FormatError> {
    u32::try_from(v).map_err(|_| FormatError::OutOfRange {
        what,
        value: v,
        max: u32::MAX as usize,
    })
}

fn half(v: usize) -> Result<u16, FormatError> {
    u16::try_from(v).map_err(|_| FormatError::OutOfRange {
        what: "MDL0 material section offset",
        value: v,
        max: u16::MAX as usize,
    })
}

fn pad4(out: &mut Vec<u8>) {
    out.resize(out.len().next_multiple_of(4), 0);
}

#[cfg(test)]
mod tests {
    use super::super::gpu::{self, Prim, op};
    use super::super::{Nsbmd, dict_spans};
    use super::*;
    use crate::nitro_dict::{self, name};
    use crate::nsbmd_header;
    use proptest::prelude::*;

    fn u16_at(b: &[u8], at: usize) -> usize {
        u16::from_le_bytes([b[at], b[at + 1]]) as usize
    }

    fn u32_at(b: &[u8], at: usize) -> usize {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
    }

    fn sbc_op(op: u8, params: &[u8]) -> SbcOp {
        SbcOp {
            op,
            params: params.to_vec(),
        }
    }

    fn material(text: &str, extra: usize) -> Material {
        Material {
            name: name(text).unwrap(),
            tag: 0,
            dif_amb: 0x7FFF_E739,
            spe_emi: 0,
            poly_attr: 0x001F_8081,
            poly_attr_mask: 0x3F1F_F8FF,
            teximage: 0x0003_0000,
            teximage_mask: 0xFFFF_FFFF,
            pltt_base: 0,
            flag: 0x1FCE,
            width: 16,
            height: 32,
            mag: [4096, 4096],
            extra: (0..extra as u8).map(|b| b + 1).collect(),
        }
    }

    fn pairing(text: &str, materials: &[u8]) -> Pairing {
        Pairing {
            name: name(text).unwrap(),
            materials: materials.to_vec(),
            bound: 1,
        }
    }

    fn vtx_diff(d: u32) -> GpuCmd {
        GpuCmd {
            op: op::VTX_DIFF,
            params: vec![d],
        }
    }

    /// `cmds` with the last group filled with `NOP`, as `gpu::decode` gives it back.
    fn padded(mut cmds: Vec<GpuCmd>) -> Vec<GpuCmd> {
        while !cmds.len().is_multiple_of(4) {
            cmds.push(GpuCmd {
                op: op::NOP,
                params: Vec::new(),
            });
        }
        cmds
    }

    /// Two bones, the baker's SBC over two materials (44 and 52 bytes), three pairings and
    /// two shapes: a quad of `VTX_10` and a quad strip of `VTX_16` and `VTX_DIFF`.
    fn fixture() -> Nsbmd {
        let quad = vec![
            GpuCmd::begin(Prim::Quads),
            GpuCmd::normal([0, 511, 0]),
            GpuCmd::texcoord([0, 0]),
            GpuCmd::vtx_10([0, 0, 0]),
            GpuCmd::texcoord([256, 0]),
            GpuCmd::vtx_10([16, 0, 0]),
            GpuCmd::texcoord([256, 256]),
            GpuCmd::vtx_10([16, 0, 16]),
            GpuCmd::texcoord([0, 256]),
            GpuCmd::vtx_10([0, 0, 16]),
            GpuCmd::end(),
        ];
        let strip = vec![
            GpuCmd::begin(Prim::QuadStrip),
            GpuCmd::color(0x7FFF),
            GpuCmd::texcoord([0, 0]),
            GpuCmd::vtx_16([0, 4096, 0]),
            vtx_diff(64),
            vtx_diff(64 << 10),
            vtx_diff(64 << 20),
            vtx_diff(64),
            vtx_diff(64 << 10),
            GpuCmd::end(),
        ];
        let model = Model {
            head: ModelHead {
                sbc_type: 0,
                scaling_rule: 0,
                texmtx_mode: 0,
                num_objects: 2,
                num_materials: 2,
                num_shapes: 2,
                first_unused_mtx: 1,
                pad: 0,
                up_scale: 64 << 12,
                down_scale: 4096 / 64,
                num_verts: 10,
                num_polys: 3,
                num_tris: 0,
                num_quads: 3,
                box_pos: [-8, 0, -8],
                box_size: [16, 1, 16],
                box_scale: 4096,
                box_scale_inv: 4096,
            },
            bones: vec![
                Bone {
                    name: name("g4_root").unwrap(),
                    flags: 0xF807,
                    rest: vec![0, 0],
                },
                Bone {
                    name: name("g4_moved").unwrap(),
                    flags: 0x0002,
                    rest: (0..38).collect(),
                },
            ],
            sbc: vec![
                sbc_op(0x26, &[0, 0, 0, 0]),
                sbc_op(0x02, &[0, 1]),
                sbc_op(0x0B, &[]),
                sbc_op(0x04, &[0]),
                sbc_op(0x05, &[0]),
                sbc_op(0x04, &[1]),
                sbc_op(0x05, &[1]),
                sbc_op(0x2B, &[]),
                sbc_op(0x01, &[]),
            ],
            materials: vec![material("t0", 0), material("t1", 8)],
            textures: vec![pairing("t0", &[0]), pairing("t1", &[1])],
            palettes: vec![pairing("p0", &[0, 1])],
            shapes: vec![
                Shape {
                    name: name("polygon0").unwrap(),
                    tag: 0,
                    size: 16,
                    flag: 0x5,
                    commands: padded(quad),
                },
                Shape {
                    name: name("polygon1").unwrap(),
                    tag: 0,
                    size: 16,
                    flag: 0x6,
                    commands: padded(strip),
                },
            ],
            evp: Vec::new(),
        };
        Nsbmd {
            version: 2,
            models: vec![(name("g4_bake").unwrap(), model)],
        }
    }

    fn written() -> Vec<u8> {
        fixture().write().unwrap()
    }

    /// File offset of model 0.
    fn model_at(bytes: &[u8]) -> usize {
        let (dict, _) = nitro_dict::read(bytes, 0x1C).unwrap();
        0x14 + u32_at(dict.entry(0), 0)
    }

    #[test]
    fn bone_lengths_follow_the_flags() {
        assert_eq!(bone_len(0xF807), 4);
        assert_eq!(bone_len(0x0000), 56);
        assert_eq!(bone_len(0x0008), 44);
        assert_eq!(bone_len(0x0006), 16);
        assert_eq!(bone_len(0x0002), 40);
    }

    #[test]
    fn a_synthetic_model_round_trips_byte_for_byte() {
        let x = fixture();
        let bytes = x.write().unwrap();
        let back = Nsbmd::read(&bytes).unwrap();
        assert_eq!(back, x);
        assert_eq!(back.write().unwrap(), bytes);
    }

    #[test]
    fn the_layout_is_the_vanilla_order() {
        let b = written();
        assert_eq!(&b[0..4], b"BMD0");
        assert_eq!(u32_at(&b, 8), b.len());
        assert_eq!(u16_at(&b, 0x0E), 1);
        assert_eq!(u32_at(&b, 0x10), 0x14);
        assert_eq!(&b[0x14..0x18], b"MDL0");
        assert_eq!(u32_at(&b, 0x18), b.len() - 0x14);
        let mo = model_at(&b);
        let m = &b[mo..];
        assert_eq!(mo + u32_at(m, 0), b.len(), "model size");
        let (ofs_sbc, ofs_mat, ofs_shp, ofs_evp) =
            (u32_at(m, 4), u32_at(m, 8), u32_at(m, 12), u32_at(m, 16));
        // Bones right behind their dictionary at 0x40, without gaps.
        let (bones, bones_end) = nitro_dict::read(m, 0x40).unwrap();
        assert_eq!(u32_at(bones.entry(0), 0), bones_end - 0x40);
        assert_eq!(u32_at(bones.entry(1), 0), bones_end - 0x40 + 4);
        assert_eq!(ofs_sbc, bones_end + 4 + 40);
        // SBC, zero-padded to 4, then the material section.
        let sbc_len = 19;
        assert_eq!(m[ofs_sbc + sbc_len - 1], 0x01);
        assert_eq!(ofs_mat, (ofs_sbc + sbc_len).next_multiple_of(4));
        assert!(m[ofs_sbc + sbc_len..ofs_mat].iter().all(|&x| x == 0));
        let (mats, mats_end) = nitro_dict::read(m, ofs_mat + 4).unwrap();
        assert_eq!(u16_at(m, ofs_mat), mats_end - ofs_mat);
        let (tex, tex_end) = nitro_dict::read(m, mats_end).unwrap();
        assert_eq!(u16_at(m, ofs_mat + 2), tex_end - ofs_mat);
        let (pal, pal_end) = nitro_dict::read(m, tex_end).unwrap();
        // Indices of every texture pairing, then of every palette pairing.
        let ids = pal_end - ofs_mat;
        assert_eq!(u16_at(tex.entry(0), 0), ids);
        assert_eq!(u16_at(tex.entry(1), 0), ids + 1);
        assert_eq!(u16_at(pal.entry(0), 0), ids + 2);
        assert_eq!(&m[pal_end..pal_end + 4], &[0, 1, 0, 1]);
        let first = (ids + 4).next_multiple_of(4);
        assert_eq!(u32_at(mats.entry(0), 0), first);
        assert_eq!(u16_at(m, ofs_mat + first + 2), 44);
        assert_eq!(u32_at(mats.entry(1), 0), first + 44);
        assert_eq!(u16_at(m, ofs_mat + first + 44 + 2), 52);
        assert_eq!(ofs_shp, ofs_mat + first + 44 + 52);
        // Shape heads behind the dictionary, display lists in order, then EVP.
        let (shapes, shapes_end) = nitro_dict::read(m, ofs_shp).unwrap();
        let heads = shapes_end - ofs_shp;
        assert_eq!(u32_at(shapes.entry(0), 0), heads);
        assert_eq!(u32_at(shapes.entry(1), 0), heads + 16);
        let h0 = ofs_shp + heads;
        let h1 = h0 + 16;
        assert_eq!(h0 + u32_at(m, h0 + 8), h1 + 16);
        assert_eq!(
            h0 + u32_at(m, h0 + 8) + u32_at(m, h0 + 12),
            h1 + u32_at(m, h1 + 8)
        );
        assert_eq!(h1 + u32_at(m, h1 + 8) + u32_at(m, h1 + 12), ofs_evp);
        assert_eq!(ofs_evp, b.len() - mo);
    }

    #[test]
    fn nsbmd_header_reads_the_names_of_a_written_model() {
        let b = written();
        assert_eq!(
            nsbmd_header::material_texture_names(&b).unwrap(),
            ["t0", "t1"]
        );
        assert_eq!(nsbmd_header::material_palette_names(&b).unwrap(), ["p0"]);
        assert_eq!(nsbmd_header::model_count(&b).unwrap(), 1);
    }

    #[test]
    fn geometry_counts_every_shape() {
        let x = fixture();
        let m = &x.models[0].1;
        assert_eq!(
            m.geometry(),
            Geometry {
                verts: 10,
                polys: 3,
                tris: 0,
                quads: 3
            }
        );
        let mut one = Geometry::default();
        gpu::count(&m.shapes[1].commands, &mut one);
        assert_eq!(one.quads, 2);
    }

    #[test]
    fn counters_match() {
        let x = fixture();
        let mut m = x.models[0].1.clone();
        assert!(m.counters_match());
        for change in 0..4 {
            let mut stale = m.clone();
            match change {
                0 => stale.head.num_verts += 1,
                1 => stale.head.num_polys += 1,
                2 => stale.head.num_tris += 1,
                _ => stale.head.num_quads += 1,
            }
            assert!(!stale.counters_match(), "change {change}");
        }
        // Write keeps stale counters: it never sets them itself.
        m.head.num_verts = 999;
        let x = Nsbmd {
            version: 2,
            models: vec![(name("m").unwrap(), m)],
        };
        let back = Nsbmd::read(&x.write().unwrap()).unwrap();
        assert_eq!(back.models[0].1.head.num_verts, 999);
    }

    #[test]
    fn material_sizes_other_than_44_48_52_are_refused() {
        let mut x = fixture();
        x.models[0].1.materials[1].extra = vec![0; 12];
        assert!(matches!(
            x.write(),
            Err(FormatError::Unsupported {
                what: "material size"
            })
        ));
        // The same in the bytes: material 1 grows from 52 to 56 bytes.
        let mut b = written();
        let mo = model_at(&b);
        let ofs_mat = u32_at(&b, mo + 8);
        let (mats, _) = nitro_dict::read(&b, mo + ofs_mat + 4).unwrap();
        let at = mo + ofs_mat + u32_at(mats.entry(1), 0) + 2;
        b[at..at + 2].copy_from_slice(&56u16.to_le_bytes());
        assert!(matches!(
            Nsbmd::read(&b),
            Err(FormatError::Unsupported {
                what: "material size"
            })
        ));
        for size in [44u16, 48, 52] {
            let mut x = fixture();
            x.models[0].1.materials[1].extra = vec![7; size as usize - 44];
            let b = x.write().unwrap();
            assert_eq!(Nsbmd::read(&b).unwrap(), x, "size {size}");
        }
    }

    #[test]
    fn a_tex0_block_is_unsupported() {
        use crate::test_support::bmd0;
        let b = written();
        let mdl0 = b[0x14..].to_vec();
        let mut tex0 = b"TEX0".to_vec();
        tex0.extend_from_slice(&16u32.to_le_bytes());
        tex0.resize(16, 0);
        for blocks in [vec![mdl0.clone(), tex0.clone()], vec![tex0.clone()]] {
            assert!(matches!(
                Nsbmd::read(&bmd0(&blocks)),
                Err(FormatError::Unsupported {
                    what: "NSBMD block other than MDL0"
                })
            ));
        }
        assert_eq!(Nsbmd::read(&bmd0(&[mdl0])).unwrap(), fixture());
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let b = written();
        for len in 0..b.len() {
            assert!(Nsbmd::read(&b[..len]).is_err(), "len {len}");
            assert!(dict_spans(&b[..len]).is_err(), "len {len}");
        }
        // With the size fields fixed up as well, the model itself is cut.
        let mo = model_at(&b);
        for len in mo..b.len() {
            let mut cut = b[..len].to_vec();
            cut[8..12].copy_from_slice(&(len as u32).to_le_bytes());
            cut[0x18..0x1C].copy_from_slice(&((len - 0x14) as u32).to_le_bytes());
            assert!(Nsbmd::read(&cut).is_err(), "len {len}");
        }
    }

    #[test]
    fn dict_spans_cover_six_dictionaries() {
        let b = written();
        let spans = dict_spans(&b).unwrap();
        assert_eq!(spans.len(), 6);
        assert_eq!(spans[0].start, 0x1C);
        for s in &spans {
            let (dict, end) = nitro_dict::read(&b, s.start).unwrap();
            assert_eq!(end, s.end);
            assert_eq!(dict.write().unwrap(), &b[s.clone()]);
        }
        let counts: Vec<usize> = spans.iter().map(|s| b[s.start + 1] as usize).collect();
        assert_eq!(counts, [1, 2, 2, 2, 1, 2]);
    }

    #[test]
    fn write_refuses_what_it_cannot_lay_out() {
        let mut x = fixture();
        x.models[0].1.bones[1].rest.pop();
        assert!(matches!(x.write(), Err(FormatError::BadSize { .. })));
        let mut x = fixture();
        x.models[0].1.shapes[1].name = x.models[0].1.shapes[0].name;
        assert!(matches!(x.write(), Err(FormatError::Unsupported { .. })));
        let mut x = fixture();
        x.models[0].1.palettes[0].materials = vec![0; 256];
        assert!(matches!(x.write(), Err(FormatError::OutOfRange { .. })));
        let mut x = fixture();
        x.models[0].1.shapes[0].commands[0].params.clear();
        assert!(matches!(x.write(), Err(FormatError::BadSize { .. })));
    }

    proptest! {
        #[test]
        fn random_byte_flips_never_panic(at in any::<usize>(), value in any::<u8>()) {
            let mut b = written();
            let at = at % b.len();
            b[at] = value;
            if let Ok(x) = Nsbmd::read(&b) {
                let _ = x.write();
            }
            let _ = dict_spans(&b);
        }
    }
}
