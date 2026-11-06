//! The baker: a neutral face list → an MDL0 map model and a budget report.
//!
//! `bake` is pure: faces, the textures of the map texture pack and the area style go in, the
//! NSBMD bytes and a report with check codes come out. The subset it writes (one bone, one
//! material and one shape per texture, quad strips over runs of texture, layer and normal,
//! `VTX_10` on the 1/64 raster, `TEXCOORD` only on change, a style per area) is described in
//! `docs/formats/nsbmd.md`, section „Teilmenge des Bäckers“.

mod encode;
mod strips;
mod test_surface;
mod textures;

pub use test_surface::{
    BAKE_TEXTURE_A, BAKE_TEXTURE_B, BakeTestOptions, BakeTestReport, SampleImage, bake_test,
    bake_test_label, bake_textures, sample_images, test_surface,
};
pub use textures::{
    AppendReport, BakeContext, append_label, append_textures, area_vram, areas_of_pack,
    bake_context, bake_for_map, pack_textures, pack_vram,
};

mod decode;
pub use decode::{BakedQuad, decode_quads};
/// The revision of the baker's bytes: raised whenever `bake` writes other bytes for the same
/// faces (the input hash of the build step `world` salts with it).
pub const BAKE_REV: u32 = 1;

use std::collections::HashSet;

use g4_formats::area_data::AreaData;
use g4_formats::game::Family;
use g4_formats::nitro_dict;
use g4_formats::nsbmd::Nsbmd;

use crate::checks::Limits;

/// One quad of the map: the baker's neutral input.
///
/// `corners` in world units (16 per tile, y up), counter-clockwise seen from above; `uv` in
/// texels, world-bound; `texture`: name in the map texture pack (≤ 16 bytes); `layer`: the
/// layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Face {
    pub corners: [[i32; 3]; 4],
    pub uv: [[i32; 2]; 4],
    pub texture: String,
    pub layer: u8,
}

/// How an area is lit: outdoor areas take light 0 with normals (time of day), indoor areas
/// no light and a white vertex colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AreaStyle {
    Outdoor,
    Indoor,
}

impl AreaStyle {
    /// Platinum and Diamond/Pearl: light type 0 or 3 is outdoor (pret); HeartGold/SoulSilver:
    /// light type 1 is outdoor (measured); everything else indoor.
    pub fn of(area: &AreaData) -> AreaStyle {
        let outdoor = match *area {
            AreaData::Dppt { light_type, .. } => light_type == 0 || light_type == 3,
            AreaData::Hgss { light_type, .. } => light_type == 1,
        };
        if outdoor {
            AreaStyle::Outdoor
        } else {
            AreaStyle::Indoor
        }
    }
}

/// A texture of the map texture pack, as the baker binds it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PackTexture {
    pub name: String,
    /// The palette the material binds; empty when the pack has no recognisable palette for it.
    pub palette: String,
    pub width: u16,
    pub height: u16,
}

/// What `bake` needs besides the faces.
pub struct BakeInput<'a> {
    pub style: AreaStyle,
    pub textures: &'a [PackTexture],
    /// VRAM of the area (map pack + buildings pack), only reported and judged.
    pub vram_bytes: usize,
    /// Above this the report warns (`bake.vram-high`).
    pub vram_limit: usize,
}

/// `posScale` of every baked model: a corner of `w` world units is `64 · w` in 1/4096 model
/// units (`up_scale` 262 144, `down_scale` 64).
pub const POS_SCALE: i32 = 64;
/// Largest vanilla VRAM of an area (map + buildings pack), Platinum and Diamond/Pearl.
pub const VANILLA_VRAM_MAX_PT: usize = 110_640;
/// Largest vanilla VRAM of an area (map + buildings pack), HeartGold/SoulSilver.
pub const VANILLA_VRAM_MAX_HGSS: usize = 87_424;

/// The vanilla VRAM maximum of `family` (measured, not an engine limit).
pub fn vanilla_vram_max(family: Family) -> usize {
    match family {
        Family::Dp | Family::Pt => VANILLA_VRAM_MAX_PT,
        Family::Hgss => VANILLA_VRAM_MAX_HGSS,
    }
}

/// The check codes of the baker and of appending textures to a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakeCode {
    /// Error: no face.
    Empty,
    /// Error, args: the model's bytes, the most the game holds.
    ModelTooBig,
    /// Error, args: the texture name (not in the pack, or without a recognisable palette).
    TextureMissing,
    /// Error, args: the face index, the corner (x, y, z).
    FaceOutOfRange,
    /// Error, args: the face index, the axis (`s`, `t`), the span in texels.
    FaceUvRange,
    /// Warning. From `bake`, args: the VRAM bytes of the area (map pack + buildings pack), the
    /// vanilla maximum. From `append_textures`, args: the area, its VRAM bytes, the vanilla
    /// maximum (one warning per area of the shared pack).
    VramHigh,
    /// Error (append): a side not a power of two 8..=1024.
    TextureSide,
    /// Error (append): a name empty, over 16 bytes or with a byte outside 0x21..=0x7E.
    TextureName,
    /// Error (append): a texture or palette name already in the pack or twice in the request.
    TextureTaken,
    /// Error (append): over 255 entries, texels over 262 144 or palettes over 65 536 bytes,
    /// or an offset over `0xFFFF · 8`.
    PackFull,
}

impl BakeCode {
    /// The code as the UI gets it (`bake.model-too-big`).
    pub fn as_str(self) -> &'static str {
        match self {
            BakeCode::Empty => "bake.empty",
            BakeCode::ModelTooBig => "bake.model-too-big",
            BakeCode::TextureMissing => "bake.texture-missing",
            BakeCode::FaceOutOfRange => "bake.face-out-of-range",
            BakeCode::FaceUvRange => "bake.face-uv-range",
            BakeCode::VramHigh => "bake.vram-high",
            BakeCode::TextureSide => "texture.side",
            BakeCode::TextureName => "texture.name",
            BakeCode::TextureTaken => "texture.taken",
            BakeCode::PackFull => "texture.pack-full",
        }
    }
}

impl serde::Serialize for BakeCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// How bad a finding is: an error refuses the bake, a warning only tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
}

/// A finding: its `code` with the `args` the UI words it from, and the English `message`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BakeMessage {
    pub code: BakeCode,
    pub level: Level,
    pub args: Vec<String>,
    pub message: String,
}

impl BakeMessage {
    pub(crate) fn error(
        code: BakeCode,
        args: &[&dyn std::fmt::Display],
        message: String,
    ) -> BakeMessage {
        BakeMessage::new(code, Level::Error, args, message)
    }

    pub(crate) fn warning(
        code: BakeCode,
        args: &[&dyn std::fmt::Display],
        message: String,
    ) -> BakeMessage {
        BakeMessage::new(code, Level::Warning, args, message)
    }

    fn new(
        code: BakeCode,
        level: Level,
        args: &[&dyn std::fmt::Display],
        message: String,
    ) -> BakeMessage {
        BakeMessage {
            code,
            level,
            args: args.iter().map(ToString::to_string).collect(),
            message,
        }
    }
}

impl std::fmt::Display for BakeMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// The result of `bake`: the model bytes and the budget.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BakeReport {
    /// The NSBMD file; empty when a face or texture error refused before encoding.
    #[serde(skip)]
    pub nsbmd: Vec<u8>,
    /// Bytes of `nsbmd`.
    pub size: usize,
    /// `size / model.max_bytes` (0xF000).
    pub share_of_limit: f32,
    /// The textures of the faces, in order of first occurrence (one material each).
    pub textures: Vec<String>,
    /// VRAM of the area (map pack + buildings pack), as `BakeInput::vram_bytes`.
    pub vram_bytes: usize,
    pub verts: u32,
    pub polys: u32,
    /// Quad strips sent (a run split before the `TEXCOORD` range counts once per piece).
    pub strips: usize,
    /// Faces sent as single quads.
    pub single_quads: usize,
    pub messages: Vec<BakeMessage>,
}

impl BakeReport {
    /// The first error, if any: the reason the model must not be written.
    pub fn refusal(&self) -> Option<&BakeMessage> {
        self.messages.iter().find(|m| m.level == Level::Error)
    }
}

/// Most and least world coordinate of a corner (`VTX_10` range at `posScale` 64).
pub(crate) const WORLD_MIN: i32 = -512;
pub(crate) const WORLD_MAX: i32 = 511;

/// Bakes `faces` (empty nsbmd when a face or texture error refuses before encoding; filled but
/// refused with model-too-big over 0xF000).
pub fn bake(faces: &[Face], input: &BakeInput) -> BakeReport {
    let limit = Limits::get().model.max_bytes.value;
    let mut report = BakeReport {
        nsbmd: Vec::new(),
        size: 0,
        share_of_limit: 0.0,
        textures: texture_order(faces)
            .into_iter()
            .map(str::to_string)
            .collect(),
        vram_bytes: input.vram_bytes,
        verts: 0,
        polys: 0,
        strips: 0,
        single_quads: 0,
        messages: check(faces, input),
    };
    if report.messages.is_empty() {
        match encode::model(faces, input) {
            Err(m) => report.messages.push(m),
            Ok((model, strips, singles)) => {
                let g = model.geometry();
                report.verts = g.verts;
                report.polys = g.polys;
                report.strips = strips;
                report.single_quads = singles;
                let file = Nsbmd {
                    version: 2,
                    models: vec![(encode::MODEL_NAME, model)],
                };
                match file.write() {
                    Ok(bytes) => {
                        report.size = bytes.len();
                        report.share_of_limit = bytes.len() as f32 / limit as f32;
                        if bytes.len() > limit {
                            report.messages.push(BakeMessage::error(
                                BakeCode::ModelTooBig,
                                &[&bytes.len(), &limit],
                                format!(
                                    "the baked model is {} bytes; the game holds at most {limit}",
                                    bytes.len()
                                ),
                            ));
                        }
                        report.nsbmd = bytes;
                    }
                    Err(e) => report.messages.push(BakeMessage::error(
                        BakeCode::ModelTooBig,
                        &[&e],
                        format!("the baked model cannot be written: {e}"),
                    )),
                }
            }
        }
    }
    if input.vram_bytes > input.vram_limit {
        report.messages.push(BakeMessage::warning(
            BakeCode::VramHigh,
            &[&input.vram_bytes, &input.vram_limit],
            format!(
                "the area's textures take {} bytes of VRAM; the largest vanilla area takes {}",
                input.vram_bytes, input.vram_limit
            ),
        ));
    }
    report
}

/// The texture names of `faces` in order of first occurrence.
pub(crate) fn texture_order(faces: &[Face]) -> Vec<&str> {
    let mut seen = HashSet::new();
    faces
        .iter()
        .map(|f| f.texture.as_str())
        .filter(|t| seen.insert(*t))
        .collect()
}

/// The pack texture `name` binds, with its palette name, or `bake.texture-missing`.
pub(crate) fn pack_texture<'a>(
    name: &str,
    textures: &'a [PackTexture],
) -> Result<&'a PackTexture, BakeMessage> {
    textures
        .iter()
        .find(|t| t.name == name && !t.palette.is_empty())
        .ok_or_else(|| {
            BakeMessage::error(
                BakeCode::TextureMissing,
                &[&name],
                format!("texture {name:?} is not in the map texture pack or has no palette there"),
            )
        })
}

/// The refusals found before encoding: no face, the first corner out of range, every texture
/// missing from the pack (once each) or with a name the model cannot hold.
fn check(faces: &[Face], input: &BakeInput) -> Vec<BakeMessage> {
    if faces.is_empty() {
        return vec![BakeMessage::error(
            BakeCode::Empty,
            &[],
            "there is no face to bake".to_string(),
        )];
    }
    let mut out = Vec::new();
    let range = WORLD_MIN..=WORLD_MAX;
    let outside = faces.iter().enumerate().find_map(|(i, f)| {
        f.corners
            .iter()
            .find(|c| c.iter().any(|v| !range.contains(v)))
            .map(|c| (i, *c))
    });
    if let Some((i, [x, y, z])) = outside {
        out.push(BakeMessage::error(
            BakeCode::FaceOutOfRange,
            &[&i, &x, &y, &z],
            format!(
                "face {i} has the corner ({x}, {y}, {z}); corners must lie in \
                 {WORLD_MIN}..={WORLD_MAX} world units"
            ),
        ));
    }
    for name in texture_order(faces) {
        match pack_texture(name, input.textures) {
            Err(m) => out.push(m),
            Ok(t) => {
                for text in [&t.name, &t.palette] {
                    if let Err(e) = nitro_dict::name(text) {
                        out.push(BakeMessage::error(
                            BakeCode::TextureName,
                            &[text],
                            format!("{text:?} cannot be a name in the model: {e}"),
                        ));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use g4_formats::nitro_dict::name;
    use g4_formats::nsbmd::gpu::{GpuCmd, op};
    use g4_formats::nsbmd::model::{Model, Pairing, Shape};
    use g4_formats::nsbmd::sbc::SbcOp;

    use super::encode::vertex;
    use super::*;

    fn tex(n: &str, w: u16, h: u16) -> PackTexture {
        PackTexture {
            name: n.to_string(),
            palette: format!("{n}_pl"),
            width: w,
            height: h,
        }
    }

    /// `t0`..`t3`, `blk`, `a`, `b`: 16 × 16, palettes `<name>_pl`.
    fn pack() -> Vec<PackTexture> {
        ["t0", "t1", "t2", "t3", "blk", "a", "b"]
            .iter()
            .map(|n| tex(n, 16, 16))
            .collect()
    }

    fn input(style: AreaStyle, textures: &[PackTexture]) -> BakeInput<'_> {
        BakeInput {
            style,
            textures,
            vram_bytes: 0,
            vram_limit: VANILLA_VRAM_MAX_PT,
        }
    }

    /// Floor tile (tx, tz) at height `y`, uv world-bound `(x + 256, z + 256)`.
    fn tile(tx: i32, tz: i32, y: i32, texture: &str, layer: u8) -> Face {
        let (x0, z0) = ((tx - 16) * 16, (tz - 16) * 16);
        let corners = [
            [x0, y, z0],
            [x0, y, z0 + 16],
            [x0 + 16, y, z0 + 16],
            [x0 + 16, y, z0],
        ];
        Face {
            corners,
            uv: corners.map(|[x, _, z]| [x + 256, z + 256]),
            texture: texture.to_string(),
            layer,
        }
    }

    /// The same tile with its uv new at 0: `(0, 0), (0, 16), (16, 16), (16, 0)`.
    fn tile_uv0(tx: i32, tz: i32, y: i32, texture: &str) -> Face {
        Face {
            uv: [[0, 0], [0, 16], [16, 16], [16, 0]],
            ..tile(tx, tz, y, texture, 0)
        }
    }

    fn model_of(report: &BakeReport) -> Model {
        let file = Nsbmd::read(&report.nsbmd).expect("the baked model reads");
        assert_eq!(file.models.len(), 1);
        file.models[0].1.clone()
    }

    fn signed(v: u32, bits: u32) -> i32 {
        let shift = 32 - bits;
        ((v << shift) as i32) >> shift
    }

    fn fields_10(v: u32) -> [i32; 3] {
        [signed(v, 10), signed(v >> 10, 10), signed(v >> 20, 10)]
    }

    fn fields_16(v: u32) -> [i32; 2] {
        [signed(v, 16), signed(v >> 16, 16)]
    }

    /// A primitive group of a display list: the `BEGIN_VTXS` type and its commands.
    struct Group {
        prim: u32,
        cmds: Vec<GpuCmd>,
    }

    fn groups(shape: &Shape) -> Vec<Group> {
        let mut out: Vec<Group> = Vec::new();
        let mut open = false;
        for c in &shape.commands {
            match c.op {
                op::BEGIN_VTXS => {
                    out.push(Group {
                        prim: c.params[0],
                        cmds: Vec::new(),
                    });
                    open = true;
                }
                op::END_VTXS => open = false,
                _ if open => out.last_mut().expect("open group").cmds.push(c.clone()),
                _ => {}
            }
        }
        out
    }

    /// Every vertex of a command list as (world corner, sent (s, t) in texels): `VTX_10` · 1,
    /// `TEXCOORD` / 16.
    fn corners_of(cmds: &[GpuCmd]) -> Vec<([i32; 3], [i32; 2])> {
        let mut st = [i32::MIN; 2];
        let mut out = Vec::new();
        for c in cmds {
            match c.op {
                op::TEXCOORD => st = fields_16(c.params[0]).map(|v| v / 16),
                op::VTX_10 => out.push((fields_10(c.params[0]), st)),
                op::VTX_16 => panic!("integer corners are always VTX_10"),
                _ => {}
            }
        }
        out
    }

    fn count(cmds: &[GpuCmd], code: u8) -> usize {
        cmds.iter().filter(|c| c.op == code).count()
    }

    fn input_corners(faces: &[Face]) -> BTreeSet<([i32; 3], [i32; 2])> {
        faces
            .iter()
            .flat_map(|f| (0..4).map(move |k| (f.corners[k], f.uv[k])))
            .collect()
    }

    fn surface() -> Vec<Face> {
        test_surface(["t0", "t1", "t2", "t3"], "blk", (15, 15))
    }

    #[test]
    fn the_test_surface_bakes_under_40000_bytes() {
        let pack = pack();
        for style in [AreaStyle::Outdoor, AreaStyle::Indoor] {
            let report = bake(&surface(), &input(style, &pack));
            assert_eq!(report.messages, Vec::new(), "{style:?}");
            assert!(report.size <= 40_000, "{style:?}: {} bytes", report.size);
            assert_eq!(report.size, report.nsbmd.len());
            assert_eq!(report.share_of_limit, report.size as f32 / 61_440.0);
            println!("test surface {style:?}: {} bytes", report.size);
        }
    }

    #[test]
    fn the_own_reader_reads_the_baked_model() {
        let pack = pack();
        let report = bake(&surface(), &input(AreaStyle::Outdoor, &pack));
        assert_eq!(report.refusal(), None);
        let file = Nsbmd::read(&report.nsbmd).expect("reads");
        assert_eq!(file.write().expect("writes"), report.nsbmd);
        assert_eq!(file.models[0].0, name("g4_bake").unwrap());
        let m = &file.models[0].1;
        assert!(m.counters_match());
        let h = &m.head;
        assert_eq!(
            (h.num_verts, h.num_quads, h.num_polys, h.num_tris),
            (3100, 1032, 1032, 0)
        );
        assert_eq!((report.verts, report.polys), (3100, 1032));
        assert_eq!((h.num_objects, h.num_materials, h.num_shapes), (1, 5, 5));
        assert_eq!(h.first_unused_mtx, 1);
        assert_eq!((h.up_scale, h.down_scale), (262_144, 64));
        assert_eq!((m.materials.len(), m.shapes.len()), (5, 5));
        assert_eq!(m.bones.len(), 1);
        assert_eq!(m.bones[0].name, name("g4_root").unwrap());
        assert_eq!(
            (m.bones[0].flags, m.bones[0].rest.clone()),
            (0xF807, vec![0, 0])
        );
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
                params: vec![],
            },
        ];
        for i in 0..5 {
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
            params: vec![],
        });
        sbc.push(SbcOp {
            op: 0x01,
            params: vec![],
        });
        assert_eq!(m.sbc, sbc);
        assert_eq!((report.strips, report.single_quads), (514, 4));
        assert_eq!(report.textures, ["t0", "t1", "t2", "t3", "blk"]);
        let names: Vec<Name16> = m.shapes.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            (0..5)
                .map(|i| name(&format!("polygon{i}")).unwrap())
                .collect::<Vec<_>>()
        );
        let mat = &m.materials[4];
        assert_eq!(mat.name, name("blk").unwrap());
        assert_eq!(
            (
                mat.tag,
                mat.spe_emi,
                mat.poly_attr_mask,
                mat.teximage,
                mat.teximage_mask
            ),
            (0, 0, 0x3F1F_F8FF, 0x0003_0000, 0xFFFF_FFFF)
        );
        assert_eq!(
            (mat.pltt_base, mat.flag, mat.width, mat.height),
            (0, 0x1FCE, 16, 16)
        );
        assert_eq!((mat.mag, mat.extra.clone()), ([4096, 4096], vec![]));
        assert_eq!(
            m.textures[4],
            Pairing {
                name: name("blk").unwrap(),
                materials: vec![4],
                bound: 0
            }
        );
        assert_eq!(
            m.palettes[4],
            Pairing {
                name: name("blk_pl").unwrap(),
                materials: vec![4],
                bound: 0
            }
        );
        assert!(m.shapes.iter().all(|s| (s.tag, s.size) == (0, 16)));
        // The box holds every corner in 1/4096 world units (vanilla convention):
        // x, z −256..=256, y 0..=16 world units, box_scale 128 (· 4096).
        let s = h.box_scale / 4096;
        assert_eq!((s, h.box_scale_inv), (128, 4096 / 128));
        let lo = h.box_pos.map(|v| i32::from(v) * s);
        let hi: Vec<i32> = (0..3)
            .map(|k| lo[k] + i32::from(h.box_size[k]) * s)
            .collect();
        assert_eq!(lo, [-256 * 4096, 0, -256 * 4096]);
        assert_eq!(hi, [256 * 4096, 16 * 4096, 256 * 4096]);
    }

    type Name16 = g4_formats::nitro_dict::Name;

    #[test]
    fn decoded_corners_equal_the_input() {
        let pack = pack();
        let faces = surface();
        for style in [AreaStyle::Outdoor, AreaStyle::Indoor] {
            let m = model_of(&bake(&faces, &input(style, &pack)));
            let got: BTreeSet<_> = m
                .shapes
                .iter()
                .flat_map(|s| corners_of(&s.commands))
                .collect();
            assert_eq!(got, input_corners(&faces), "{style:?}");
        }
    }

    #[test]
    fn strips_follow_runs_of_texture_and_layer() {
        let pack = pack();
        let bake_in = |faces: &[Face]| bake(faces, &input(AreaStyle::Outdoor, &pack));

        // A row of four tiles `a`: one strip with 10 corners.
        let row: Vec<Face> = (0..4).map(|tx| tile(tx, 0, 0, "a", 0)).collect();
        let r = bake_in(&row);
        assert_eq!((r.strips, r.single_quads), (1, 0));
        let g = groups(&model_of(&r).shapes[0]);
        assert_eq!(g.len(), 1);
        assert_eq!((g[0].prim, count(&g[0].cmds, op::VTX_10)), (3, 10));

        // Layers 0, 0, 1, 1: two strips.
        let layered: Vec<Face> = (0..4)
            .map(|tx| tile(tx, 0, 0, "a", u8::from(tx >= 2)))
            .collect();
        let r = bake_in(&layered);
        assert_eq!((r.strips, r.single_quads), (2, 0));
        let g = groups(&model_of(&r).shapes[0]);
        assert_eq!(
            g.iter()
                .map(|g| (g.prim, count(&g.cmds, op::VTX_10)))
                .collect::<Vec<_>>(),
            [(3, 6), (3, 6)]
        );

        // Textures a, a, b, b: two shapes, one strip each.
        let two: Vec<Face> = (0..4)
            .map(|tx| tile(tx, 0, 0, if tx < 2 { "a" } else { "b" }, 0))
            .collect();
        let r = bake_in(&two);
        let m = model_of(&r);
        assert_eq!((m.shapes.len(), r.strips, r.single_quads), (2, 2, 0));
        for s in &m.shapes {
            let g = groups(s);
            assert_eq!(
                g.iter()
                    .map(|g| (g.prim, count(&g.cmds, op::VTX_10)))
                    .collect::<Vec<_>>(),
                [(3, 6)]
            );
        }

        // UV new at 0 on every tile: four single quads in one group.
        let fresh: Vec<Face> = (0..4).map(|tx| tile_uv0(tx, 0, 0, "a")).collect();
        let r = bake_in(&fresh);
        assert_eq!((r.strips, r.single_quads), (0, 4));
        let g = groups(&model_of(&r).shapes[0]);
        assert_eq!(g.len(), 1);
        assert_eq!((g[0].prim, count(&g[0].cmds, op::VTX_10)), (1, 16));

        // A wall standing on the east edge of a floor tile, its first edge equal to the tile's
        // last one (position and uv), but with another normal: no common strip.
        let floor = tile(16, 16, 0, "a", 0);
        let wall = Face {
            corners: [[16, 0, 0], [16, 0, 16], [16, 16, 16], [16, 16, 0]],
            uv: [[272, 256], [272, 272], [288, 272], [288, 256]],
            texture: "a".to_string(),
            layer: 0,
        };
        assert_eq!(
            (wall.corners[0], wall.uv[0]),
            (floor.corners[3], floor.uv[3])
        );
        assert_eq!(
            (wall.corners[1], wall.uv[1]),
            (floor.corners[2], floor.uv[2])
        );
        assert_ne!(strips::normal(&wall), strips::normal(&floor));
        let r = bake_in(&[floor, wall]);
        assert_eq!((r.strips, r.single_quads), (0, 2));
    }

    #[test]
    fn vtx_10_on_the_raster_and_vtx_16_off_it() {
        assert_eq!(vertex([64, 128, -64]).op, op::VTX_10);
        assert_eq!(vertex([64, 128, -64]), GpuCmd::vtx_10([1, 2, -1]));
        assert_eq!(vertex([65, 0, 0]).op, op::VTX_16);
        assert_eq!(vertex([65, 0, 0]), GpuCmd::vtx_16([65, 0, 0]));
        assert_eq!(vertex([-32_768, 0, 32_704]).op, op::VTX_10);
        assert_eq!(vertex([-32_768, 0, 32_704]), GpuCmd::vtx_10([-512, 0, 511]));
        assert_eq!(vertex([32_767, 0, 0]).op, op::VTX_16);
        // On the raster but past 511: VTX_16.
        assert_eq!(vertex([32_768, 0, 0]).op, op::VTX_16);
    }

    #[test]
    fn texcoord_only_on_change() {
        let pack = pack();
        // A row of four tiles with t fixed at 0: every second corner repeats (s, t).
        let row: Vec<Face> = (0..4)
            .map(|tx| {
                let f = tile(tx, 0, 0, "a", 0);
                Face {
                    uv: f.uv.map(|[s, _]| [s, 0]),
                    ..f
                }
            })
            .collect();
        let r = bake(&row, &input(AreaStyle::Outdoor, &pack));
        assert_eq!((r.strips, r.single_quads), (1, 0));
        let cmds = &groups(&model_of(&r).shapes[0])[0].cmds;
        // Corners in strip order: f0.c0, f0.c1, f0.c3, f0.c2, f1.c3, f1.c2, …
        let mut order = vec![row[0].uv[0], row[0].uv[1], row[0].uv[3], row[0].uv[2]];
        for f in &row[1..] {
            order.push(f.uv[3]);
            order.push(f.uv[2]);
        }
        let changes = order
            .iter()
            .enumerate()
            .filter(|(i, uv)| *i == 0 || order[i - 1] != **uv)
            .count();
        assert_eq!(changes, 5);
        assert_eq!(count(cmds, op::TEXCOORD), changes);
        assert_eq!(count(cmds, op::VTX_10), 10);
    }

    #[test]
    fn long_strips_split_before_the_texcoord_range() {
        let textures = [tex("g", 64, 64)];
        let row: Vec<Face> = (0..32)
            .map(|tx| {
                let f = tile(tx, 0, 0, "g", 0);
                Face {
                    uv: [
                        [tx * 64, 0],
                        [tx * 64, 64],
                        [(tx + 1) * 64, 64],
                        [(tx + 1) * 64, 0],
                    ],
                    ..f
                }
            })
            .collect();
        let r = bake(&row, &input(AreaStyle::Indoor, &textures));
        assert_eq!(r.refusal(), None);
        assert!(r.strips >= 2, "{} strips", r.strips);
        assert_eq!(r.single_quads, 0);
        let m = model_of(&r);
        let g = groups(&m.shapes[0]);
        assert_eq!(g.len(), r.strips);
        // The uv of every corner by its position, as given.
        let given: std::collections::BTreeMap<[i32; 3], [i32; 2]> = row
            .iter()
            .flat_map(|f| (0..4).map(move |k| (f.corners[k], f.uv[k])))
            .collect();
        let mut got = BTreeSet::new();
        for group in &g {
            assert_eq!(group.prim, 3);
            // One shift per strip, a whole number of textures: no field wrapped.
            let offsets: BTreeSet<[i32; 2]> = corners_of(&group.cmds)
                .iter()
                .map(|(c, st)| [given[c][0] - st[0], given[c][1] - st[1]])
                .collect();
            assert_eq!(offsets.len(), 1, "{offsets:?}");
            let [ds, dt] = *offsets.iter().next().unwrap();
            assert_eq!((ds % 64, dt % 64), (0, 0));
            for c in &group.cmds {
                if c.op == op::TEXCOORD {
                    let [s, t] = fields_16(c.params[0]);
                    assert!((-32_768..=32_767).contains(&s) && (-32_768..=32_767).contains(&t));
                    assert!((0..=2047 * 16).contains(&s), "s {s}");
                }
            }
            got.extend(
                corners_of(&group.cmds)
                    .into_iter()
                    .map(|(c, [s, t])| (c, [s.rem_euclid(64), t.rem_euclid(64)])),
            );
        }
        let want: BTreeSet<_> = input_corners(&row)
            .into_iter()
            .map(|(c, [s, t])| (c, [s.rem_euclid(64), t.rem_euclid(64)]))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn a_face_far_out_in_uv_is_shifted_by_whole_textures() {
        let textures = [tex("g", 64, 64)];
        let face = Face {
            uv: [[5000, 0], [5000, 16], [5016, 16], [5016, 0]],
            ..tile(0, 0, 0, "g", 0)
        };
        let r = bake(&[face], &input(AreaStyle::Indoor, &textures));
        assert_eq!(r.refusal(), None);
        let m = model_of(&r);
        let s: BTreeSet<i32> = corners_of(&m.shapes[0].commands)
            .iter()
            .map(|(_, st)| st[0])
            .collect();
        assert_eq!(s, BTreeSet::from([8, 24]));
    }

    #[test]
    fn a_face_wider_than_the_texcoord_range_is_refused() {
        let textures = [tex("g", 64, 64)];
        let face = Face {
            uv: [[0, 0], [0, 16], [4096, 16], [4096, 0]],
            ..tile(0, 0, 0, "g", 0)
        };
        let r = bake(&[face], &input(AreaStyle::Indoor, &textures));
        let refusal = r.refusal().expect("refused");
        assert_eq!(refusal.code, BakeCode::FaceUvRange);
        assert_eq!(refusal.code.as_str(), "bake.face-uv-range");
        assert!(r.nsbmd.is_empty());
    }

    #[test]
    fn outdoor_has_light_fog_and_normals_indoor_white_colour() {
        let pack = pack();
        let faces = surface();
        let out = model_of(&bake(&faces, &input(AreaStyle::Outdoor, &pack)));
        for mat in &out.materials {
            assert_eq!((mat.poly_attr, mat.dif_amb), (0x001F_8081, 0x7FFF_E739));
        }
        for s in &out.shapes {
            assert_eq!(s.flag, 0x5);
            assert_eq!(count(&s.commands, op::COLOR), 0);
        }
        let normals = |s: &Shape| -> Vec<[i32; 3]> {
            s.commands
                .iter()
                .filter(|c| c.op == op::NORMAL)
                .map(|c| fields_10(c.params[0]))
                .collect()
        };
        // Floor: one normal, up.
        assert_eq!(normals(&out.shapes[0]), [[0, 511, 0]]);
        // The block: top, then the four sides; the south side (+z) is the first one.
        let blk = normals(&out.shapes[4]);
        assert_eq!(blk[0], [0, 511, 0]);
        assert!(blk.contains(&[0, 0, 511]), "{blk:?}");
        assert_eq!(blk.len(), 5);

        let ind = model_of(&bake(&faces, &input(AreaStyle::Indoor, &pack)));
        for mat in &ind.materials {
            assert_eq!((mat.poly_attr, mat.dif_amb), (0x001F_0080, 0x7FFF_8000));
        }
        for s in &ind.shapes {
            assert_eq!(s.flag, 0x6);
            assert_eq!(s.commands[0], GpuCmd::color(0x7FFF));
            assert_eq!(count(&s.commands, op::COLOR), 1);
            assert_eq!(count(&s.commands, op::NORMAL), 0);
        }
    }

    #[test]
    fn the_south_side_of_the_block_faces_plus_z() {
        let south = surface()
            .into_iter()
            .find(|f| f.texture == "blk" && f.corners.iter().all(|c| c[2] == 16))
            .expect("south side");
        assert_eq!(strips::normal(&south), [0, 0, 511]);
    }

    #[test]
    fn a_shared_palette_is_one_pairing_with_both_materials() {
        let textures = [
            PackTexture {
                palette: "pal".to_string(),
                ..tex("t0", 16, 16)
            },
            PackTexture {
                palette: "pal".to_string(),
                ..tex("t1", 16, 16)
            },
        ];
        let faces = [tile(0, 0, 0, "t0", 0), tile(5, 5, 0, "t1", 0)];
        let m = model_of(&bake(&faces, &input(AreaStyle::Indoor, &textures)));
        assert_eq!(
            m.palettes,
            [Pairing {
                name: name("pal").unwrap(),
                materials: vec![0, 1],
                bound: 0
            }]
        );
        assert_eq!(m.textures.len(), 2);
    }

    #[test]
    fn area_style_follows_the_light_rule() {
        let dppt = |light_type| AreaData::Dppt {
            buildings_tileset: 0,
            map_tileset: 0,
            unknown: 0,
            light_type,
        };
        let hgss = |light_type| AreaData::Hgss {
            buildings_tileset: 0,
            map_tileset: 0,
            dynamic_texture_type: 0,
            area_type: 0,
            light_type,
        };
        assert_eq!(AreaStyle::of(&dppt(0)), AreaStyle::Outdoor);
        assert_eq!(AreaStyle::of(&dppt(3)), AreaStyle::Outdoor);
        assert_eq!(AreaStyle::of(&dppt(1)), AreaStyle::Indoor);
        assert_eq!(AreaStyle::of(&dppt(2)), AreaStyle::Indoor);
        assert_eq!(AreaStyle::of(&hgss(1)), AreaStyle::Outdoor);
        assert_eq!(AreaStyle::of(&hgss(0)), AreaStyle::Indoor);
        assert_eq!(AreaStyle::of(&hgss(2)), AreaStyle::Indoor);
        assert_eq!(vanilla_vram_max(Family::Pt), 110_640);
        assert_eq!(vanilla_vram_max(Family::Dp), 110_640);
        assert_eq!(vanilla_vram_max(Family::Hgss), 87_424);
    }

    #[test]
    fn an_oversize_model_is_refused_with_model_too_big() {
        let pack = pack();
        let mut faces = Vec::new();
        for y in [0, 32] {
            for tz in 0..32 {
                for tx in 0..32 {
                    faces.push(tile_uv0(tx, tz, y, "t0"));
                }
            }
        }
        let r = bake(&faces, &input(AreaStyle::Outdoor, &pack));
        let refusal = r.refusal().expect("refused");
        assert_eq!(refusal.code, BakeCode::ModelTooBig);
        assert!(r.size > 61_440);
        assert_eq!(r.nsbmd.len(), r.size);
        assert_eq!(refusal.args[0], r.size.to_string());
        assert_eq!(
            serde_json::to_value(refusal).unwrap()["code"],
            "bake.model-too-big"
        );
    }

    #[test]
    fn missing_texture_empty_list_and_out_of_range_corner_are_refused() {
        let pack = pack();
        let i = input(AreaStyle::Outdoor, &pack);
        let r = bake(&[tile(0, 0, 0, "nope", 0)], &i);
        let m = r.refusal().expect("refused");
        assert_eq!(
            (m.code, m.args.clone()),
            (BakeCode::TextureMissing, vec!["nope".to_string()])
        );
        assert_eq!(m.code.as_str(), "bake.texture-missing");
        assert!(r.nsbmd.is_empty());

        let r = bake(&[], &i);
        assert_eq!(r.refusal().map(|m| m.code), Some(BakeCode::Empty));
        assert_eq!(BakeCode::Empty.as_str(), "bake.empty");
        assert!(r.nsbmd.is_empty());

        let mut far = tile(0, 0, 0, "t0", 0);
        far.corners[2][0] = 512;
        let r = bake(&[far], &i);
        let m = r.refusal().expect("refused");
        assert_eq!(m.code, BakeCode::FaceOutOfRange);
        assert_eq!(m.code.as_str(), "bake.face-out-of-range");
        assert_eq!(m.args[0], "0");
        assert!(r.nsbmd.is_empty());

        // A texture without a palette counts as missing.
        let no_pal = [PackTexture {
            palette: String::new(),
            ..tex("t0", 16, 16)
        }];
        let r = bake(
            &[tile(0, 0, 0, "t0", 0)],
            &input(AreaStyle::Outdoor, &no_pal),
        );
        assert_eq!(r.refusal().map(|m| m.code), Some(BakeCode::TextureMissing));
    }

    #[test]
    fn vram_over_the_vanilla_maximum_warns() {
        let pack = pack();
        let i = BakeInput {
            vram_bytes: 110_641,
            vram_limit: 110_640,
            ..input(AreaStyle::Outdoor, &pack)
        };
        let r = bake(&surface(), &i);
        assert_eq!(r.refusal(), None);
        assert_eq!(r.messages.len(), 1);
        let m = &r.messages[0];
        assert_eq!((m.code, m.level), (BakeCode::VramHigh, Level::Warning));
        assert_eq!(m.args, ["110641", "110640"]);
        assert!(!r.nsbmd.is_empty());
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["messages"][0]["code"], "bake.vram-high");
        assert_eq!(json["messages"][0]["level"], "warning");
        assert!(json.get("nsbmd").is_none());
    }

    #[test]
    fn every_code_has_its_string() {
        let all = [
            (BakeCode::Empty, "bake.empty"),
            (BakeCode::ModelTooBig, "bake.model-too-big"),
            (BakeCode::TextureMissing, "bake.texture-missing"),
            (BakeCode::FaceOutOfRange, "bake.face-out-of-range"),
            (BakeCode::FaceUvRange, "bake.face-uv-range"),
            (BakeCode::VramHigh, "bake.vram-high"),
            (BakeCode::TextureSide, "texture.side"),
            (BakeCode::TextureName, "texture.name"),
            (BakeCode::TextureTaken, "texture.taken"),
            (BakeCode::PackFull, "texture.pack-full"),
        ];
        for (code, text) in all {
            assert_eq!(code.as_str(), text);
            assert_eq!(serde_json::to_value(code).unwrap(), text);
        }
    }
}
