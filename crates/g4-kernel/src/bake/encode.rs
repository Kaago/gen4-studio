//! Encoding: faces → one MDL0 model (one bone, one material and one shape per texture).

use std::collections::HashMap;

use g4_formats::nitro_dict::{self, Name};
use g4_formats::nsbmd::gpu::{GpuCmd, Prim};
use g4_formats::nsbmd::model::{Bone, Material, Model, ModelHead, Pairing, Shape};
use g4_formats::nsbmd::sbc::SbcOp;

use super::strips::{self, RunKind};
use super::{
    AreaStyle, BakeCode, BakeInput, BakeMessage, Face, POS_SCALE, pack_texture, texture_order,
};

/// The model's name in the model dictionary.
pub(crate) const MODEL_NAME: Name = *b"g4_bake\0\0\0\0\0\0\0\0\0";
/// The one bone.
const BONE_NAME: Name = *b"g4_root\0\0\0\0\0\0\0\0\0";
/// Bone flags: no translation, rotation or scale (identity).
const BONE_FLAGS: u16 = 0xF807;
/// 1.0 in fx32.
const FX: i32 = 4096;
/// The `TEXCOORD` range in texels (1.11.4 in 1/16 texel: −32 768..=32 767).
const TC_MIN: i64 = -2048;
const TC_MAX: i64 = 2047;
/// Vertex colour of indoor shapes.
const WHITE: u16 = 0x7FFF;
/// Material fields every baked material shares (A §4, `gutachten-a-minimal.py` `build_nsbmd`).
const POLY_ATTR_MASK: u32 = 0x3F1F_F8FF;
/// Repeat in S and T.
const TEXIMAGE: u32 = 0x0003_0000;
const TEXIMAGE_MASK: u32 = 0xFFFF_FFFF;
const MATERIAL_FLAG: u16 = 0x1FCE;
/// Shape head size.
const SHAPE_SIZE: u16 = 16;
/// Largest number of materials: SBC and pairings hold the index in one byte.
const MAX_TEXTURES: usize = 255;

/// The values of a style.
struct Style {
    dif_amb: u32,
    poly_attr: u32,
    shape_flag: u32,
}

fn style(s: AreaStyle) -> Style {
    match s {
        // Light 0, front face, fog, alpha 31; one normal per plane.
        AreaStyle::Outdoor => Style {
            dif_amb: 0x7FFF_E739,
            poly_attr: 0x001F_8081,
            shape_flag: 0x5,
        },
        // No light, front face, alpha 31; vertex colour white.
        AreaStyle::Indoor => Style {
            dif_amb: 0x7FFF_8000,
            poly_attr: 0x001F_0080,
            shape_flag: 0x6,
        },
    }
}

/// A corner in 1/4096 model units: `VTX_10` when all three are on the 1/64 raster and
/// `fx / 64` in −512..=511, else `VTX_16`.
pub(crate) fn vertex(fx: [i32; 3]) -> GpuCmd {
    let on_raster = fx
        .iter()
        .all(|v| v % 64 == 0 && (-512..=511).contains(&(v / 64)));
    if on_raster {
        GpuCmd::vtx_10(fx.map(|v| v / 64))
    } else {
        GpuCmd::vtx_16(fx)
    }
}

/// The model of `faces`, the quad strips sent, the faces sent as single quads and the faces
/// sent as triangles. The faces are checked (`bake`): corners in range, every texture in the
/// pack.
pub(crate) fn model(
    faces: &[Face],
    input: &BakeInput,
) -> Result<(Model, usize, usize, usize), BakeMessage> {
    let names = texture_order(faces);
    if names.len() > MAX_TEXTURES {
        return Err(BakeMessage::error(
            BakeCode::ModelTooBig,
            &[&names.len(), &MAX_TEXTURES],
            format!(
                "the faces use {} textures; a model holds at most {MAX_TEXTURES}",
                names.len()
            ),
        ));
    }
    let index: HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let mut of_texture = vec![Vec::new(); names.len()];
    for (i, f) in faces.iter().enumerate() {
        if let Some(&m) = index.get(f.texture.as_str()) {
            of_texture[m].push(i);
        }
    }

    let st = style(input.style);
    let mut materials = Vec::new();
    let mut textures = Vec::new();
    let mut palettes: Vec<Pairing> = Vec::new();
    let mut shapes = Vec::new();
    let (mut strips, mut singles, mut triangles) = (0, 0, 0);
    for (m, name) in names.iter().enumerate() {
        let t = pack_texture(name, input.textures)?;
        let mi = m as u8; // m < 256: checked above
        let mut list = List::new(faces, input.style, [t.width, t.height]);
        for run in strips::runs(faces, &of_texture[m]) {
            match run.kind {
                RunKind::Strip => strips += list.strip(&run.faces)?,
                RunKind::Quads => singles += list.quads(&run.faces)?,
                RunKind::Triangles => triangles += list.triangles(&run.faces)?,
            }
        }
        let tex_name = dict_name(&t.name)?;
        materials.push(Material {
            name: tex_name,
            tag: 0,
            dif_amb: st.dif_amb,
            spe_emi: 0,
            poly_attr: st.poly_attr,
            poly_attr_mask: POLY_ATTR_MASK,
            teximage: TEXIMAGE,
            teximage_mask: TEXIMAGE_MASK,
            pltt_base: 0,
            flag: MATERIAL_FLAG,
            width: t.width,
            height: t.height,
            mag: [FX, FX],
            extra: Vec::new(),
        });
        textures.push(Pairing {
            name: tex_name,
            materials: vec![mi],
            bound: 0,
        });
        let pal_name = dict_name(&t.palette)?;
        match palettes.iter_mut().find(|p| p.name == pal_name) {
            Some(p) => p.materials.push(mi),
            None => palettes.push(Pairing {
                name: pal_name,
                materials: vec![mi],
                bound: 0,
            }),
        }
        shapes.push(Shape {
            name: dict_name(&format!("polygon{m}"))?,
            tag: 0,
            size: SHAPE_SIZE,
            flag: st.shape_flag,
            commands: list.cmds,
        });
    }

    let count = materials.len() as u8; // ≤ 255: checked above
    let mut sbc = vec![
        SbcOp {
            op: 0x26,
            params: vec![0, 0, 0, 0],
        },
        SbcOp {
            op: 0x02,
            params: vec![0, 1],
        },
        SbcOp {
            op: 0x0B,
            params: Vec::new(),
        },
    ];
    for i in 0..count {
        sbc.push(SbcOp {
            op: 0x04,
            params: vec![i],
        });
        sbc.push(SbcOp {
            op: 0x05,
            params: vec![i],
        });
    }
    sbc.push(SbcOp {
        op: 0x2B,
        params: Vec::new(),
    });
    sbc.push(SbcOp {
        op: 0x01,
        params: Vec::new(),
    });

    let mut model = Model {
        head: ModelHead {
            num_objects: 1,
            num_materials: count,
            num_shapes: count,
            first_unused_mtx: 1,
            up_scale: POS_SCALE * FX,
            down_scale: FX / POS_SCALE,
            ..ModelHead::default()
        },
        bones: vec![Bone {
            name: BONE_NAME,
            flags: BONE_FLAGS,
            rest: vec![0, 0],
        }],
        sbc,
        materials,
        textures,
        palettes,
        shapes,
        evp: Vec::new(),
    };
    let g = model.geometry();
    let h = &mut model.head;
    h.num_verts = counter(g.verts);
    h.num_polys = counter(g.polys);
    h.num_tris = counter(g.tris);
    h.num_quads = counter(g.quads);
    let (pos, size, scale) = bounding_box(faces);
    h.box_pos = pos;
    h.box_size = size;
    h.box_scale = scale * FX;
    h.box_scale_inv = FX / scale;
    Ok((model, strips, singles, triangles))
}

/// A head counter; past `u16` (a model far over the size limit) it stays at the maximum.
fn counter(v: u32) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

fn dict_name(text: &str) -> Result<Name, BakeMessage> {
    nitro_dict::name(text).map_err(|e| {
        BakeMessage::error(
            BakeCode::TextureName,
            &[&text],
            format!("{text:?} cannot be a name in the model: {e}"),
        )
    })
}

/// The box over every corner in the vanilla map model convention: 1/4096 world units, so that
/// `world = box · box_scale / 4096²` (the reading of `frame_block`; vanilla rooms carry
/// `box_scale == up_scale` and a box in world units). `ext` the largest of |min|, |max| and the
/// extent over all axes (in 1/4096 world units); `box_scale / 4096` the least power of two ≥ 1
/// with `ext / (box_scale / 4096) < 7.99 · 4096`; position `round(min / (box_scale / 4096))`,
/// size `round((max − min) / (box_scale / 4096))`. Corners are whole world units and the scale
/// at most 256 for corners in range, so the box is exact.
fn bounding_box(faces: &[Face]) -> ([i16; 3], [i16; 3], i32) {
    let mut lo = [i64::MAX; 3];
    let mut hi = [i64::MIN; 3];
    for c in faces.iter().flat_map(|f| f.corners.iter()) {
        for k in 0..3 {
            let v = i64::from(c[k]) * i64::from(FX);
            lo[k] = lo[k].min(v);
            hi[k] = hi[k].max(v);
        }
    }
    if faces.is_empty() {
        return ([0; 3], [0; 3], 1);
    }
    let ext = (0..3)
        .map(|k| lo[k].abs().max(hi[k].abs()).max(hi[k] - lo[k]))
        .max()
        .unwrap_or(0);
    let mut scale: i64 = 1;
    // ext / scale < 7.99 · 4096, in integers; corners in range keep scale at most 256.
    while ext * 100 >= 799 * i64::from(FX) * scale && scale < i64::from(FX) {
        scale *= 2;
    }
    let round = |v: i64| (v as f64 / scale as f64).round() as i16;
    let pos = [round(lo[0]), round(lo[1]), round(lo[2])];
    let size = [
        round(hi[0] - lo[0]),
        round(hi[1] - lo[1]),
        round(hi[2] - lo[2]),
    ];
    (pos, size, scale as i32)
}

/// The display list of one shape while it is built: `TEXCOORD` only when (s, t) changes
/// against the last one sent in this shape, outdoor a `NORMAL` before the first corner of a
/// strip, single quad or triangle when it changes, indoor `COLOR` white once at the start.
struct List<'a> {
    faces: &'a [Face],
    outdoor: bool,
    /// Texture width and height in texels: the step of a uv shift.
    side: [i64; 2],
    cmds: Vec<GpuCmd>,
    last_st: Option<[i32; 2]>,
    last_normal: Option<[i32; 3]>,
}

impl<'a> List<'a> {
    fn new(faces: &'a [Face], style: AreaStyle, side: [u16; 2]) -> List<'a> {
        let outdoor = style == AreaStyle::Outdoor;
        let cmds = if outdoor {
            Vec::new()
        } else {
            vec![GpuCmd::color(WHITE)]
        };
        List {
            faces,
            outdoor,
            side: side.map(|v| i64::from(v.max(1))),
            cmds,
            last_st: None,
            last_normal: None,
        }
    }

    /// One quad strip per piece of `run` (in chain order): a piece ends before the face whose
    /// new corners (c3, c2) leave the `TEXCOORD` range under the piece's shift. Returns the
    /// pieces.
    fn strip(&mut self, run: &[usize]) -> Result<usize, BakeMessage> {
        let mut pieces = 0;
        let mut i = 0;
        while let Some(&first) = run.get(i) {
            let shift = self.shift(first)?;
            self.cmds.push(GpuCmd::begin(Prim::QuadStrip));
            self.normal(first);
            for k in [0, 1, 3, 2] {
                self.corner(first, k, shift);
            }
            i += 1;
            while let Some(&f) = run.get(i) {
                if !fits(&self.faces[f], &[3, 2], shift) {
                    break;
                }
                for k in [3, 2] {
                    self.corner(f, k, shift);
                }
                i += 1;
            }
            self.cmds.push(GpuCmd::end());
            pieces += 1;
        }
        Ok(pieces)
    }

    /// One `BEGIN_VTXS` quads group with every face of `run`, corners c0..c3.
    fn quads(&mut self, run: &[usize]) -> Result<usize, BakeMessage> {
        self.cmds.push(GpuCmd::begin(Prim::Quads));
        for &f in run {
            let shift = self.shift(f)?;
            self.normal(f);
            for k in 0..4 {
                self.corner(f, k, shift);
            }
        }
        self.cmds.push(GpuCmd::end());
        Ok(run.len())
    }

    /// One `BEGIN_VTXS` triangles group with every face of `run`, corners c0, c1, c2.
    fn triangles(&mut self, run: &[usize]) -> Result<usize, BakeMessage> {
        self.cmds.push(GpuCmd::begin(Prim::Triangles));
        for &f in run {
            // uv[3] == uv[2]: the shift over the four corners is the one over the three.
            let shift = self.shift(f)?;
            self.normal(f);
            for k in 0..3 {
                self.corner(f, k, shift);
            }
        }
        self.cmds.push(GpuCmd::end());
        Ok(run.len())
    }

    fn normal(&mut self, f: usize) {
        if !self.outdoor {
            return;
        }
        let n = strips::normal(&self.faces[f]);
        if self.last_normal != Some(n) {
            self.cmds.push(GpuCmd::normal(n));
            self.last_normal = Some(n);
        }
    }

    fn corner(&mut self, f: usize, k: usize, shift: [i64; 2]) {
        let face = &self.faces[f];
        // In range: `shift` and `fits` keep uv − shift in −2048..=2047.
        let st = [0, 1].map(|a| ((i64::from(face.uv[k][a]) - shift[a]) * 16) as i32);
        if self.last_st != Some(st) {
            self.cmds.push(GpuCmd::texcoord(st));
            self.last_st = Some(st);
        }
        self.cmds
            .push(vertex(face.corners[k].map(|w| w.saturating_mul(POS_SCALE))));
    }

    /// The uv shift of a strip, single quad or triangle starting with face `f`, per axis: 0
    /// while the face fits, else the largest multiple of the texture side at or below its least
    /// uv (one side more when the face still does not fit, so every face with a span ≤ 2047
    /// texels fits); a face that fits under none of the three is `bake.face-uv-range`. With
    /// `r = min mod side` a shifted face fits exactly when `span ≤ 2047 + side − r`, so a span
    /// over 2047 + side is always refused (unless the uv fit unshifted), and one over 4095
    /// always.
    fn shift(&self, f: usize) -> Result<[i64; 2], BakeMessage> {
        let face = &self.faces[f];
        let mut shift = [0; 2];
        for a in 0..2 {
            let vals = face.uv.map(|u| i64::from(u[a]));
            let fits_at = |s: i64| vals.iter().all(|v| (TC_MIN..=TC_MAX).contains(&(v - s)));
            if fits_at(0) {
                continue;
            }
            let min = vals.iter().copied().min().unwrap_or(0);
            let max = vals.iter().copied().max().unwrap_or(0);
            let below = min.div_euclid(self.side[a]) * self.side[a];
            shift[a] = match [below, below + self.side[a]]
                .into_iter()
                .find(|&s| fits_at(s))
            {
                Some(s) => s,
                None => {
                    let axis = ["s", "t"][a];
                    let span = max - min;
                    let side = self.side[a];
                    return Err(BakeMessage::error(
                        BakeCode::FaceUvRange,
                        &[&f, &axis, &span],
                        format!(
                            "face {f} spans {span} texels in {axis}: its uv fit \
                             {TC_MIN}..={TC_MAX} neither as they are nor shifted by whole \
                             textures ({side} texels); a span up to {TC_MAX} always fits"
                        ),
                    ));
                }
            };
        }
        Ok(shift)
    }
}

/// Whether the corners `ks` of `face` fit the `TEXCOORD` range under `shift`.
fn fits(face: &Face, ks: &[usize], shift: [i64; 2]) -> bool {
    ks.iter().all(|&k| {
        (0..2).all(|a| (TC_MIN..=TC_MAX).contains(&(i64::from(face.uv[k][a]) - shift[a])))
    })
}
