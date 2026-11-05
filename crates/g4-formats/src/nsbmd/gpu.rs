//! GPU display lists of MDL0 shapes (GBATEK "DS 3D Geometry Commands").
//!
//! A display list is a run of groups: four opcode bytes, then the 32-bit parameters of
//! those four commands in order (all little-endian). `NOP` (0x00) has no parameters and
//! fills a group that has fewer than four real commands. [`decode`] keeps every `NOP`,
//! [`encode`] pads only the last group, so `encode(decode(b)) == b` for every list the
//! decoder accepts.

use crate::error::FormatError;

/// Opcodes the baker emits.
pub mod op {
    pub const NOP: u8 = 0x00;
    pub const COLOR: u8 = 0x20;
    pub const NORMAL: u8 = 0x21;
    pub const TEXCOORD: u8 = 0x22;
    pub const VTX_16: u8 = 0x23;
    pub const VTX_10: u8 = 0x24;
    pub const VTX_XY: u8 = 0x25;
    pub const VTX_XZ: u8 = 0x26;
    pub const VTX_YZ: u8 = 0x27;
    pub const VTX_DIFF: u8 = 0x28;
    pub const BEGIN_VTXS: u8 = 0x40;
    pub const END_VTXS: u8 = 0x41;
}

/// Largest opcode of the table (`VEC_TEST`).
const MAX_OP: u8 = 0x72;
/// Opcodes per group.
const GROUP: usize = 4;

/// Primitive type of `BEGIN_VTXS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    Triangles = 0,
    Quads = 1,
    TriangleStrip = 2,
    QuadStrip = 3,
}

/// One geometry command with its parameter words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCmd {
    pub op: u8,
    pub params: Vec<u32>,
}

/// Number of parameter words of `op` (GBATEK), `None` for an opcode the GPU does not know.
pub fn param_count(op: u8) -> Option<usize> {
    Some(match op {
        0x00 | 0x11 | 0x15 | 0x41 => 0,
        0x10 | 0x12..=0x14 => 1,
        0x16 | 0x18 => 16,
        0x17 | 0x19 => 12,
        0x1A => 9,
        0x1B | 0x1C => 3,
        0x23 => 2,
        0x20..=0x2B => 1,
        0x30..=0x33 => 1,
        0x34 => 32,
        0x40 | 0x50 | 0x60 => 1,
        0x70 => 3,
        0x71 => 2,
        0x72 => 1,
        _ => return None,
    })
}

/// Whether `op` sends a vertex (`VTX_16` to `VTX_DIFF`).
pub fn is_vertex(op: u8) -> bool {
    (op::VTX_16..=op::VTX_DIFF).contains(&op)
}

fn unknown(op: u8) -> FormatError {
    FormatError::OutOfRange {
        what: "GPU command",
        value: op as usize,
        max: MAX_OP as usize,
    }
}

fn known(op: u8) -> Result<usize, FormatError> {
    param_count(op).ok_or_else(|| unknown(op))
}

/// `len` bytes at `off`, or `Truncated`.
fn slice_at(bytes: &[u8], off: usize, len: usize) -> Result<&[u8], FormatError> {
    let end = off.checked_add(len).ok_or(FormatError::Truncated {
        needed: usize::MAX,
        have: bytes.len(),
    })?;
    bytes.get(off..end).ok_or(FormatError::Truncated {
        needed: end,
        have: bytes.len(),
    })
}

/// Reads a display list. Unknown opcode → `OutOfRange { what: "GPU command" }`, a group
/// or parameter past the end → `Truncated`.
pub fn decode(bytes: &[u8]) -> Result<Vec<GpuCmd>, FormatError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let group = slice_at(bytes, at, GROUP)?;
        at += GROUP;
        for &op in group {
            let k = known(op)?;
            let raw = slice_at(bytes, at, 4 * k)?;
            at += 4 * k;
            let params = raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|w| u32::from_le_bytes(*w))
                .collect();
            out.push(GpuCmd { op, params });
        }
    }
    Ok(out)
}

/// Writes a display list; the last group is padded with `NOP`. Unknown opcode →
/// `OutOfRange { what: "GPU command" }`, wrong number of parameters → `BadSize` (bytes).
pub fn encode(cmds: &[GpuCmd]) -> Result<Vec<u8>, FormatError> {
    let mut out = Vec::new();
    for group in cmds.chunks(GROUP) {
        for cmd in group {
            let k = known(cmd.op)?;
            if cmd.params.len() != k {
                return Err(FormatError::BadSize {
                    expected: 4 * k,
                    found: 4 * cmd.params.len(),
                });
            }
            out.push(cmd.op);
        }
        out.resize(out.len() + GROUP - group.len(), op::NOP);
        for cmd in group {
            for p in &cmd.params {
                out.extend_from_slice(&p.to_le_bytes());
            }
        }
    }
    Ok(out)
}

/// The low `bits` bits of `v` (two's complement), as the GPU packs a signed field.
fn field(v: i32, bits: u32) -> u32 {
    (v as u32) & ((1 << bits) - 1)
}

/// Three signed 10-bit fields: `x | y << 10 | z << 20`.
fn pack_10(v: [i32; 3]) -> u32 {
    field(v[0], 10) | field(v[1], 10) << 10 | field(v[2], 10) << 20
}

impl GpuCmd {
    fn one(op: u8, param: u32) -> GpuCmd {
        GpuCmd {
            op,
            params: vec![param],
        }
    }

    /// `BEGIN_VTXS` with the primitive type.
    pub fn begin(prim: Prim) -> GpuCmd {
        GpuCmd::one(op::BEGIN_VTXS, prim as u32)
    }

    /// `END_VTXS`.
    pub fn end() -> GpuCmd {
        GpuCmd {
            op: op::END_VTXS,
            params: Vec::new(),
        }
    }

    /// `COLOR`: vertex colour as BGR555.
    pub fn color(bgr555: u16) -> GpuCmd {
        GpuCmd::one(op::COLOR, u32::from(bgr555))
    }

    /// `NORMAL` in 1/512, each −512..=511: `x | y << 10 | z << 20` (10 bits each).
    /// Values outside the range keep only their low 10 bits.
    pub fn normal(n: [i32; 3]) -> GpuCmd {
        GpuCmd::one(op::NORMAL, pack_10(n))
    }

    /// `TEXCOORD` in 1/16 texel, each −32768..=32767: `s | t << 16`.
    /// Values outside the range keep only their low 16 bits.
    pub fn texcoord(st: [i32; 2]) -> GpuCmd {
        GpuCmd::one(op::TEXCOORD, field(st[0], 16) | field(st[1], 16) << 16)
    }

    /// `VTX_10` in 1/64 model unit, each −512..=511: `x | y << 10 | z << 20`.
    /// Values outside the range keep only their low 10 bits.
    pub fn vtx_10(v: [i32; 3]) -> GpuCmd {
        GpuCmd::one(op::VTX_10, pack_10(v))
    }

    /// `VTX_16` in 1/4096 model unit: `[x | y << 16, z]` (16 bits each).
    /// Values outside −32768..=32767 keep only their low 16 bits.
    pub fn vtx_16(v: [i32; 3]) -> GpuCmd {
        GpuCmd {
            op: op::VTX_16,
            params: vec![field(v[0], 16) | field(v[1], 16) << 16, field(v[2], 16)],
        }
    }
}

/// Vertex and polygon counts as the MDL0 model header holds them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Geometry {
    pub verts: u32,
    pub polys: u32,
    pub tris: u32,
    pub quads: u32,
}

impl Geometry {
    /// Adds the polygons of a run of `n` vertices of `prim`.
    fn close(&mut self, prim: Option<Prim>, n: u32) {
        let (tris, quads) = match prim {
            None => (0, 0),
            Some(Prim::Triangles) => (n / 3, 0),
            Some(Prim::Quads) => (0, n / 4),
            Some(Prim::TriangleStrip) => (n.saturating_sub(2), 0),
            Some(Prim::QuadStrip) => (0, n.saturating_sub(2) / 2),
        };
        self.tris = self.tris.saturating_add(tris);
        self.quads = self.quads.saturating_add(quads);
        self.polys = self.polys.saturating_add(tris).saturating_add(quads);
    }
}

/// Adds the vertices and polygons of one display list (Triangles n/3, Quads n/4,
/// TriangleStrip n−2, QuadStrip (n−2)/2).
///
/// Every `VTX_*` counts as a vertex. A run ends at `END_VTXS`, at the next `BEGIN_VTXS`
/// or at the end of the list; vertices outside a run make no polygon. The primitive is
/// the low two bits of the `BEGIN_VTXS` parameter, as on the hardware.
pub fn count(cmds: &[GpuCmd], into: &mut Geometry) {
    let mut prim = None;
    let mut n = 0u32;
    for cmd in cmds {
        if is_vertex(cmd.op) {
            into.verts = into.verts.saturating_add(1);
            n = n.saturating_add(1);
        } else if cmd.op == op::BEGIN_VTXS || cmd.op == op::END_VTXS {
            into.close(prim, n);
            n = 0;
            prim = match cmd.params.first() {
                Some(p) if cmd.op == op::BEGIN_VTXS => Some(match p & 3 {
                    0 => Prim::Triangles,
                    1 => Prim::Quads,
                    2 => Prim::TriangleStrip,
                    _ => Prim::QuadStrip,
                }),
                _ => None,
            };
        }
    }
    into.close(prim, n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The table of the brief (GBATEK), verbatim.
    const TABLE: &[(u8, usize)] = &[
        (0x00, 0),
        (0x10, 1),
        (0x11, 0),
        (0x12, 1),
        (0x13, 1),
        (0x14, 1),
        (0x15, 0),
        (0x16, 16),
        (0x17, 12),
        (0x18, 16),
        (0x19, 12),
        (0x1A, 9),
        (0x1B, 3),
        (0x1C, 3),
        (0x20, 1),
        (0x21, 1),
        (0x22, 1),
        (0x23, 2),
        (0x24, 1),
        (0x25, 1),
        (0x26, 1),
        (0x27, 1),
        (0x28, 1),
        (0x29, 1),
        (0x2A, 1),
        (0x2B, 1),
        (0x30, 1),
        (0x31, 1),
        (0x32, 1),
        (0x33, 1),
        (0x34, 32),
        (0x40, 1),
        (0x41, 0),
        (0x50, 1),
        (0x60, 1),
        (0x70, 3),
        (0x71, 2),
        (0x72, 1),
    ];

    fn cmd(op: u8, params: &[u32]) -> GpuCmd {
        GpuCmd {
            op,
            params: params.to_vec(),
        }
    }

    fn nop() -> GpuCmd {
        cmd(op::NOP, &[])
    }

    #[test]
    fn param_counts_match_the_table_and_nothing_else() {
        for op in 0..=255u8 {
            let want = TABLE.iter().find(|(o, _)| *o == op).map(|&(_, k)| k);
            assert_eq!(param_count(op), want, "opcode {op:#04x}");
        }
    }

    #[test]
    fn every_opcode_of_the_table_round_trips() {
        let mut cmds: Vec<GpuCmd> = TABLE
            .iter()
            .enumerate()
            .map(|(i, &(op, k))| GpuCmd {
                op,
                params: (0..k as u32)
                    .map(|j| 0x0101_0101u32.wrapping_mul(i as u32 + 1) ^ j << 24)
                    .collect(),
            })
            .collect();
        while !cmds.len().is_multiple_of(4) {
            cmds.push(nop());
        }
        let bytes = encode(&cmds).unwrap();
        let words: usize = TABLE.iter().map(|&(_, k)| k).sum();
        assert_eq!(bytes.len(), cmds.len() + 4 * words);
        assert_eq!(decode(&bytes).unwrap(), cmds);
        assert_eq!(encode(&decode(&bytes).unwrap()).unwrap(), bytes);
    }

    #[test]
    fn groups_pad_with_nop() {
        let cmds = [
            GpuCmd::begin(Prim::Triangles),
            GpuCmd::vtx_16([1, 2, 3]),
            GpuCmd::end(),
        ];
        let bytes = encode(&cmds).unwrap();
        let mut want = vec![op::BEGIN_VTXS, op::VTX_16, op::END_VTXS, op::NOP];
        for w in [0u32, 1 | 2 << 16, 3] {
            want.extend_from_slice(&w.to_le_bytes());
        }
        assert_eq!(bytes, want);
        let mut padded = cmds.to_vec();
        padded.push(nop());
        assert_eq!(decode(&bytes).unwrap(), padded);
        assert_eq!(encode(&[]).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn unknown_opcode_is_out_of_range() {
        let bytes = [0x42, 0, 0, 0];
        assert!(matches!(
            decode(&bytes),
            Err(FormatError::OutOfRange {
                what: "GPU command",
                value: 0x42,
                ..
            })
        ));
        assert!(matches!(
            encode(&[cmd(0x42, &[])]),
            Err(FormatError::OutOfRange {
                what: "GPU command",
                value: 0x42,
                ..
            })
        ));
    }

    #[test]
    fn wrong_param_count_is_bad_size() {
        assert!(matches!(
            encode(&[cmd(op::VTX_16, &[1])]),
            Err(FormatError::BadSize {
                expected: 8,
                found: 4
            })
        ));
        assert!(matches!(
            encode(&[cmd(op::END_VTXS, &[0])]),
            Err(FormatError::BadSize {
                expected: 0,
                found: 4
            })
        ));
    }

    #[test]
    fn cut_lists_are_truncated() {
        // Opcode group shorter than four bytes.
        assert!(matches!(
            decode(&[op::NOP, op::NOP]),
            Err(FormatError::Truncated { .. })
        ));
        // VTX_16 announces two words, only one is there.
        let mut bytes = vec![op::VTX_16, 0, 0, 0];
        bytes.extend_from_slice(&7u32.to_le_bytes());
        assert!(matches!(
            decode(&bytes),
            Err(FormatError::Truncated {
                needed: 12,
                have: 8
            })
        ));
    }

    #[test]
    fn constructors_pack_like_gbatek() {
        assert_eq!(
            GpuCmd::vtx_10([-1, 2, 511]).params,
            [0x3FF | 2 << 10 | 511 << 20]
        );
        assert_eq!(GpuCmd::vtx_10([0, 0, 0]).op, op::VTX_10);
        assert_eq!(GpuCmd::vtx_16([4096, -4096, 1]).params, [0xF000_1000, 1]);
        assert_eq!(GpuCmd::vtx_16([0, 0, -1]).params, [0, 0xFFFF]);
        assert_eq!(GpuCmd::vtx_16([0, 0, 0]).op, op::VTX_16);
        assert_eq!(GpuCmd::normal([0, 511, 0]).params, [511 << 10]);
        assert_eq!(GpuCmd::normal([0, 0, -512]).params, [0x200 << 20]);
        assert_eq!(GpuCmd::normal([0, 0, 0]).op, op::NORMAL);
        assert_eq!(GpuCmd::texcoord([16, -16]).params, [0xFFF0_0010]);
        assert_eq!(GpuCmd::texcoord([0, 0]).op, op::TEXCOORD);
        assert_eq!(GpuCmd::color(0x7FFF).params, [0x7FFF]);
        assert_eq!(GpuCmd::color(0).op, op::COLOR);
        assert_eq!(GpuCmd::begin(Prim::QuadStrip), cmd(op::BEGIN_VTXS, &[3]));
        assert_eq!(GpuCmd::begin(Prim::Triangles), cmd(op::BEGIN_VTXS, &[0]));
        assert_eq!(GpuCmd::end(), cmd(op::END_VTXS, &[]));
    }

    #[test]
    fn is_vertex_covers_vtx_16_to_vtx_diff() {
        for op in 0..=255u8 {
            assert_eq!(is_vertex(op), (0x23..=0x28).contains(&op), "{op:#04x}");
        }
    }

    fn run(prim: Prim, n: usize) -> Vec<GpuCmd> {
        let mut cmds = vec![GpuCmd::begin(prim), GpuCmd::color(0x7FFF)];
        for i in 0..n {
            cmds.push(GpuCmd::texcoord([0, 0]));
            cmds.push(GpuCmd::vtx_16([i as i32, 0, 0]));
        }
        cmds.push(GpuCmd::end());
        cmds
    }

    fn counted(cmds: &[GpuCmd]) -> Geometry {
        let mut g = Geometry::default();
        count(cmds, &mut g);
        g
    }

    #[test]
    fn counts_follow_the_primitive() {
        let quads = Geometry {
            verts: 8,
            polys: 2,
            tris: 0,
            quads: 2,
        };
        assert_eq!(counted(&run(Prim::Quads, 8)), quads);
        assert_eq!(
            counted(&run(Prim::QuadStrip, 6)),
            Geometry {
                verts: 6,
                polys: 2,
                tris: 0,
                quads: 2
            }
        );
        assert_eq!(
            counted(&run(Prim::Triangles, 6)),
            Geometry {
                verts: 6,
                polys: 2,
                tris: 2,
                quads: 0
            }
        );
        assert_eq!(
            counted(&run(Prim::TriangleStrip, 5)),
            Geometry {
                verts: 5,
                polys: 3,
                tris: 3,
                quads: 0
            }
        );
        // Every VTX_* counts, whatever its form.
        let mut mixed = vec![GpuCmd::begin(Prim::Triangles)];
        for op in [op::VTX_10, op::VTX_XY, op::VTX_XZ, op::VTX_YZ, op::VTX_DIFF] {
            mixed.push(cmd(op, &[0]));
        }
        mixed.push(GpuCmd::vtx_16([0, 0, 0]));
        mixed.push(GpuCmd::end());
        assert_eq!(
            counted(&mixed),
            Geometry {
                verts: 6,
                polys: 2,
                tris: 2,
                quads: 0
            }
        );
    }

    #[test]
    fn counts_add_up_over_lists_and_runs() {
        let mut g = Geometry::default();
        count(&run(Prim::Quads, 4), &mut g);
        // A new BEGIN_VTXS ends the run before it, as on the hardware.
        let mut two = run(Prim::Triangles, 3);
        two.pop();
        two.extend(run(Prim::TriangleStrip, 4));
        count(&two, &mut g);
        // A strip too short for a polygon and a run without END_VTXS.
        count(&run(Prim::TriangleStrip, 2), &mut g);
        let mut open = run(Prim::QuadStrip, 4);
        open.pop();
        count(&open, &mut g);
        assert_eq!(
            g,
            Geometry {
                verts: 4 + 3 + 4 + 2 + 4,
                polys: 1 + 1 + 2 + 1,
                tris: 3,
                quads: 2
            }
        );
    }

    proptest! {
        #[test]
        fn random_bytes_never_panic_the_decoders(
            bytes in proptest::collection::vec(any::<u8>(), 0..=512)
        ) {
            if let Ok(cmds) = decode(&bytes) {
                prop_assert_eq!(encode(&cmds).unwrap(), bytes);
            }
        }
    }
}
