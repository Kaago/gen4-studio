//! Import of OBJ and glTF models as the model of a map: the readers fill a neutral [`mesh::Mesh`],
//! [`mesh::to_faces`] turns it into the faces of the baker (`bake::Face`).

pub mod mesh;
mod read_gltf;
mod read_obj;
pub mod samples;
pub mod textures;

pub use textures::{
    TexturePlan, TexturePreview, load_image, plan_textures, png_textures, texture_name,
};

use std::path::{Component, Path, PathBuf};

use g4_formats::nsbtx::{NewTexture, Rgba};
use g4_formats::texconv::Colors;

use crate::bake::{
    BakeCode, BakeInput, BakeMessage, Face, Level, WORLD_MAX, WORLD_MIN, appended_pack_or_refusal,
    area_vram, areas_of_pack, bake, bake_context, pack_textures,
};
use crate::error::KernelError;
use crate::map_import::{ImportPreview, ModelPreview, PartCode, PartMessage, PartState, map_areas};
use crate::project::Project;
use crate::store::Store;
use crate::terrain::{BudgetItem, budget_state};
use mesh::{Mesh, MeshStats, to_faces};

/// What the user chooses for an import: `unit` world units per model unit (16: one model unit
/// is one tile), `colors` per texture (4 or 16).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ImportOptions {
    pub unit: f32,
    pub colors: u8,
}

impl Default for ImportOptions {
    fn default() -> ImportOptions {
        ImportOptions {
            unit: 16.0,
            colors: 16,
        }
    }
}

/// The smallest `unit` (1/16 world unit per model unit).
pub const UNIT_MIN: f32 = 0.0625;
/// The largest `unit` (16 tiles per model unit).
pub const UNIT_MAX: f32 = 256.0;

impl ImportOptions {
    /// `unit` finite in `UNIT_MIN..=UNIT_MAX`, `colors` 4 or 16; else [`KernelError::Refused`].
    pub fn check(&self) -> Result<(), KernelError> {
        if !(self.unit.is_finite() && (UNIT_MIN..=UNIT_MAX).contains(&self.unit)) {
            return Err(KernelError::Refused(format!(
                "the unit {} is not in {UNIT_MIN}..={UNIT_MAX} world units per model unit",
                self.unit
            )));
        }
        if !matches!(self.colors, 4 | 16) {
            return Err(KernelError::Refused(format!(
                "{} colours per texture: only 4 or 16 are possible",
                self.colors
            )));
        }
        Ok(())
    }

    /// The texture format of `colors` (4 → 2 bits per texel, else 4 bits).
    pub fn texconv_colors(&self) -> Colors {
        if self.colors == 4 {
            Colors::Four
        } else {
            Colors::Sixteen
        }
    }
}

/// The format of a model file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Obj,
    Gltf,
}

impl SourceFormat {
    /// By the extension, ignoring case: `obj` → OBJ; `gltf`, `glb` → glTF; else `None`.
    pub fn of(path: &Path) -> Option<SourceFormat> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "obj" => Some(SourceFormat::Obj),
            "gltf" | "glb" => Some(SourceFormat::Gltf),
            _ => None,
        }
    }

    /// The name the `model.*` messages give the format.
    pub fn label(self) -> &'static str {
        match self {
            SourceFormat::Obj => "OBJ",
            SourceFormat::Gltf => "glTF",
        }
    }
}

/// The mesh of an OBJ or glTF file and its notes (model.parts-ignored); the refusal with code.
///
/// The format follows the extension ([`SourceFormat::of`]); every file the model refers to is
/// read only from inside the model's folder ([`resolve_inside`]). A polygon of more than
/// [`mesh::MAX_CORNERS`] corners is `model.source-unreadable`.
pub fn read_model(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage> {
    let (format, read) = match SourceFormat::of(path) {
        Some(SourceFormat::Obj) => (SourceFormat::Obj, read_obj::read_obj(path)?),
        Some(SourceFormat::Gltf) => (SourceFormat::Gltf, read_gltf::read_gltf(path)?),
        None => {
            return Err(PartMessage::new(
                PartCode::ModelSourceUnreadable,
                &[&"OBJ/glTF", &"unknown file extension"],
                format!("{} is neither an OBJ nor a glTF file", path.display()),
            ));
        }
    };
    check_corners(&read.0, format)?;
    Ok(read)
}

/// Every polygon at most [`mesh::MAX_CORNERS`] corners (ear clipping one huge n-gon would take
/// minutes), else `model.source-unreadable`. glTF brings triangles only; the check holds for
/// both readers alike.
fn check_corners(mesh: &Mesh, format: SourceFormat) -> Result<(), PartMessage> {
    match mesh
        .polygons
        .iter()
        .map(|p| p.corners.len())
        .find(|&n| n > mesh::MAX_CORNERS)
    {
        Some(n) => Err(source_unreadable(
            format,
            &format!("a polygon has {n} corners, more than {}", mesh::MAX_CORNERS),
        )),
        None => Ok(()),
    }
}

/// The tokens of model.parts-ignored in the order the hint lists them.
pub(crate) const IGNORED_PARTS: [&str; 8] = [
    "lines",
    "points",
    "animations",
    "skins",
    "cameras",
    "lights",
    "vertex-colours",
    "texture-options",
];

/// model.parts-ignored with the tokens of `found` (each once, in the order of
/// [`IGNORED_PARTS`], joined by `,`); `None` when nothing was left out. Unknown tokens are
/// dropped (a reader's bug, never a wrong sentence).
pub(crate) fn parts_ignored(found: &[&str]) -> Option<PartMessage> {
    let tokens: Vec<&str> = IGNORED_PARTS
        .iter()
        .copied()
        .filter(|t| found.contains(t))
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let list = tokens.join(",");
    Some(PartMessage::new(
        PartCode::ModelPartsIgnored,
        &[&list],
        format!("not taken over: {}", tokens.join(", ")),
    ))
}

/// `model.source-unreadable` with the format's label and the reason.
pub(crate) fn source_unreadable(format: SourceFormat, reason: &str) -> PartMessage {
    let label = format.label();
    PartMessage::new(
        PartCode::ModelSourceUnreadable,
        &[&label, &reason],
        format!("not a readable {label} model ({reason})"),
    )
}

fn file_outside(reference: &str) -> PartMessage {
    PartMessage::new(
        PartCode::ModelFileOutside,
        &[&reference],
        format!("the model refers to {reference}, outside its folder"),
    )
}

fn file_missing(reference: &str, why: &str) -> PartMessage {
    PartMessage::new(
        PartCode::ModelFileMissing,
        &[&reference],
        format!("the file {reference} the model refers to is missing ({why})"),
    )
}

/// A URI scheme in front of `reference` (`https:`, `file:`, `data:`, a drive letter `C:`).
fn has_scheme(reference: &str) -> bool {
    let Some((scheme, _)) = reference.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `reference` (as written in a file inside `root`, relative to `from`) as a path inside the
/// canonical folder `root`: model.file-outside for absolute paths, schemes and anything
/// leaving `root` after canonicalising; model.file-missing when it does not exist.
///
/// `from` is the folder of the referring file. Backslashes count as separators (exports from
/// Windows). Refused before any file is touched: an absolute path, a URI scheme (also `data:`;
/// the glTF reader decodes those itself) and a `..` that leaves `root` on its own. After that
/// the path is canonicalised (symlinks followed) and must still lie in `root` and be a file.
/// The returned path is the canonical one, so the caller reads exactly what was checked.
pub(crate) fn resolve_inside(
    root: &Path,
    from: &Path,
    reference: &str,
) -> Result<PathBuf, PartMessage> {
    let written = reference.trim();
    if written.is_empty() {
        return Err(file_missing(written, "no file name"));
    }
    let slashed = written.replace('\\', "/");
    if slashed.starts_with('/') || Path::new(&slashed).has_root() || has_scheme(&slashed) {
        return Err(file_outside(written));
    }
    let root = root
        .canonicalize()
        .map_err(|e| file_missing(written, &e.to_string()))?;
    let from = from
        .canonicalize()
        .map_err(|e| file_missing(written, &e.to_string()))?;
    if !from.starts_with(&root) {
        return Err(file_outside(written));
    }
    // Lexically first: `..` must not climb above `root`, whether the target exists or not.
    let mut lexical = from;
    for part in Path::new(&slashed).components() {
        match part {
            Component::Normal(name) => lexical.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if !lexical.pop() || !lexical.starts_with(&root) {
                    return Err(file_outside(written));
                }
            }
            Component::RootDir | Component::Prefix(_) => return Err(file_outside(written)),
        }
    }
    if !lexical.starts_with(&root) {
        return Err(file_outside(written));
    }
    // A dangling symlink is missing as well: nothing is read either way.
    let canonical = lexical
        .canonicalize()
        .map_err(|e| file_missing(written, &e.to_string()))?;
    if !canonical.starts_with(&root) {
        return Err(file_outside(written));
    }
    if !canonical.is_file() {
        return Err(file_missing(written, "not a file"));
    }
    Ok(canonical)
}

/// The largest side of a texture; an image whose header claims more is refused before it is
/// decoded.
const IMAGE_SIDE_MAX: u32 = 1024;

/// The pixels of a PNG or JPEG as RGBA8; texture.unreadable `[file, reason]` (`file` as the
/// model writes it).
///
/// The sides are read from the header first: one over 1024 is texture.side `[file, width,
/// height]` without decoding (a header may claim any size); the decoder runs with a limit of
/// 1024 × 1024 and 64 MiB of memory as well. Smaller sides that are no power of two are judged
/// by the texture plan, which knows the texture's name.
pub(crate) fn decode_image(bytes: &[u8], file: &str) -> Result<Rgba, PartMessage> {
    let unreadable = |e: image::ImageError| {
        let reason = e.to_string();
        PartMessage::new(
            PartCode::TextureUnreadable,
            &[&file, &reason],
            format!("the image {file} cannot be read ({reason})"),
        )
    };
    let reader = || {
        image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| unreadable(image::ImageError::IoError(e)))
    };
    let (width, height) = reader()?.into_dimensions().map_err(unreadable)?;
    if width > IMAGE_SIDE_MAX || height > IMAGE_SIDE_MAX {
        return Err(textures::side_refused(file, width, height));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(IMAGE_SIDE_MAX);
    limits.max_image_height = Some(IMAGE_SIDE_MAX);
    limits.max_alloc = Some(64 << 20);
    let mut decoder = reader()?;
    decoder.limits(limits);
    let image = decoder.decode().map_err(unreadable)?;
    let rgba = image.to_rgba8();
    Ok(Rgba {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

/// The finding of `bake` as a part message; `area` is the first argument of the VRAM warning,
/// `faces` the faces given to `bake` (the uv range names the texture of its face).
///
/// `bake.empty` → `model.no-geometry`; `bake.model-too-big` → `model.too-big` (same args);
/// `bake.face-out-of-range` → `model.out-of-range` (`[x, y, z, -512, 511]`);
/// `bake.face-uv-range` → `model.uv-range` (`[texture, axis, span]`; the baker gives the face
/// index, a face outside `faces` keeps it); `bake.texture-missing` → `model.material-untextured`
/// (the texture name: a bound texture the pack has without a palette); the codes of appending
/// keep their args: `texture.side` `[name, width, height]`, `texture.name` `[name]`,
/// `texture.taken` `["texture" | "palette", name]`, `texture.pack-full` `[what, value, most]`;
/// `bake.vram-high` → `texture.vram-high` `[area, vram, most]` (`bake` gives `[vram, most]`,
/// `append_textures` `[area, vram, most]`). The VRAM finding is a warning: [`bake_state`] gives
/// the state of a part.
pub(crate) fn from_bake(m: &BakeMessage, area: u8, faces: &[Face]) -> PartMessage {
    let message = m.message.clone();
    let keep = |code: PartCode| PartMessage {
        code,
        args: m.args.clone(),
        message: message.clone(),
    };
    match m.code {
        BakeCode::Empty => PartMessage::new(PartCode::ModelNoGeometry, &[], message),
        BakeCode::ModelTooBig => keep(PartCode::ModelTooBig),
        BakeCode::FaceOutOfRange => {
            let corner = m.args.get(1..4).unwrap_or_default();
            let mut args = corner.to_vec();
            args.extend([WORLD_MIN.to_string(), WORLD_MAX.to_string()]);
            PartMessage {
                code: PartCode::ModelOutOfRange,
                args,
                message,
            }
        }
        BakeCode::FaceUvRange => {
            let mut args = m.args.clone();
            if let Some(first) = args.first_mut()
                && let Some(face) = first.parse::<usize>().ok().and_then(|i| faces.get(i))
            {
                *first = face.texture.clone();
            }
            PartMessage {
                code: PartCode::ModelUvRange,
                args,
                message,
            }
        }
        BakeCode::TextureMissing => keep(PartCode::ModelMaterialUntextured),
        BakeCode::VramHigh => {
            let args = match m.args.as_slice() {
                [_, _, _] => m.args.clone(),
                _ => std::iter::once(area.to_string())
                    .chain(m.args.iter().cloned())
                    .collect(),
            };
            let message = if m.args.len() == 3 {
                message
            } else {
                format!("area {area}: {message}")
            };
            PartMessage {
                code: PartCode::TextureVramHigh,
                args,
                message,
            }
        }
        BakeCode::TextureSide => keep(PartCode::TextureSide),
        BakeCode::TextureName => keep(PartCode::TextureName),
        BakeCode::TextureTaken => keep(PartCode::TextureTaken),
        BakeCode::PackFull => keep(PartCode::TexturePackFull),
    }
}

/// The state a finding of `bake` or of appending gives a part: an error refuses it, a warning
/// (`texture.vram-high`) only tells; the message as [`from_bake`] maps it.
pub(crate) fn bake_state(m: &BakeMessage, area: u8, faces: &[Face]) -> PartState {
    let message = from_bake(m, area, faces);
    match m.level {
        Level::Error => PartState::Error(message),
        Level::Warning => PartState::Warning(message),
    }
}

/// The most textures a map model holds (one material each).
pub const MODEL_TEXTURES_MAX: usize = 255;

/// Everything `inspect` needs for a model file of map `map`: the bake (nothing written).
///
/// `faces` as given to the baker, `nsbmd` the baked model, `pack` the area's map texture pack,
/// `new` the textures to append to it (empty when every texture is in the pack), `preview` the
/// figures of the dialog, `state` `Ok` or `Warning(texture.vram-high)` of the own area, `notes`
/// in the order `model.converted`, the reader's hints, `model.faces-dropped`, `texture.new` /
/// `texture.reused` per texture, then `texture.vram-high` of the other areas of the pack.
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertedModel {
    pub faces: Vec<Face>,
    pub nsbmd: Vec<u8>,
    pub pack: u16,
    pub new: Vec<NewTexture>,
    pub preview: ImportPreview,
    pub state: PartState,
    pub notes: Vec<PartMessage>,
}

/// Why a model file is refused, with the hints found before (a glTF of lines only: its
/// `model.parts-ignored` stays next to `model.no-geometry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRefusal {
    pub reason: PartMessage,
    pub notes: Vec<PartMessage>,
}

impl From<PartMessage> for ModelRefusal {
    fn from(reason: PartMessage) -> ModelRefusal {
        ModelRefusal {
            reason,
            notes: Vec::new(),
        }
    }
}

/// `part.no-area` `[map]`: no header shows map `map`.
pub(crate) fn no_area(map: u16) -> PartMessage {
    PartMessage::new(
        PartCode::PartNoArea,
        &[&map],
        format!(
            "no map header shows map {map}; without an area there is no texture pack and no light"
        ),
    )
}

/// The area's own `texture.vram-high` from `vram` against `limit` (a warning, `None` within),
/// and the hints of the other areas of `pack` whose VRAM with `after` (the pack after
/// appending) is above `limit`.
pub(crate) fn vram_findings(
    store: &mut Store,
    area: u8,
    pack: u16,
    after: Option<&[u8]>,
    vram: usize,
    limit: usize,
) -> Result<(Option<PartMessage>, Vec<PartMessage>), KernelError> {
    let high = |a: u8, vram: usize| {
        PartMessage::new(
            PartCode::TextureVramHigh,
            &[&a, &vram, &limit],
            format!(
                "area {a}: the textures take {vram} bytes of VRAM; the largest vanilla area \
                 takes {limit}"
            ),
        )
    };
    let own = (vram > limit).then(|| high(area, vram));
    let mut others = Vec::new();
    if let Some(after) = after {
        for other in areas_of_pack(store, pack)? {
            if other == area {
                continue;
            }
            let v = area_vram(store, other, Some(after))?;
            if v > limit {
                others.push(high(other, v));
            }
        }
    }
    Ok((own, others))
}

/// The bake of model file `path` for map `map` (rules „Prüfen ohne Schreiben“); nothing is
/// written. Outer error: the project cannot be read; inner: the refusal with its code.
///
/// In order, each refusal with its code: the map's area (`part.no-area`), the model file
/// (`read_model`), the texture plan against the area's pack (`plan_textures`), at most
/// [`MODEL_TEXTURES_MAX`] textures (`model.texture-count`), the pack with the new textures
/// appended (in memory, only when there are new ones), the faces (`to_faces`), the bake (its
/// refusal through [`from_bake`]). The VRAM of the own area with the new pack is the state
/// (`Warning(texture.vram-high)`), the other areas of the pack above the vanilla maximum are
/// hints with the same code.
pub fn convert_for_map(
    store: &mut Store,
    map: u16,
    path: &Path,
    opts: &ImportOptions,
) -> Result<Result<ConvertedModel, ModelRefusal>, KernelError> {
    opts.check()?;
    if map_areas(store, map)?.is_empty() {
        return Ok(Err(no_area(map).into()));
    }
    let ctx = bake_context(store, map)?;
    let format = SourceFormat::of(path);
    let (mesh, read_notes) = match read_model(path) {
        Ok(read) => read,
        Err(reason) => return Ok(Err(reason.into())),
    };
    let refuse = |reason: PartMessage| {
        Ok(Err(ModelRefusal {
            reason,
            notes: read_notes.clone(),
        }))
    };
    let plan = match plan_textures(&mesh.materials, &ctx.textures, opts.texconv_colors()) {
        Ok(plan) => plan,
        Err(reason) => return refuse(reason),
    };
    let count = plan.previews.len();
    if count > MODEL_TEXTURES_MAX {
        return refuse(PartMessage::new(
            PartCode::ModelTextureCount,
            &[&count, &MODEL_TEXTURES_MAX],
            format!(
                "the model uses {count} textures; a map model holds at most {MODEL_TEXTURES_MAX}"
            ),
        ));
    }
    let (after, textures, vram) = if plan.new.is_empty() {
        (None, ctx.textures.clone(), ctx.vram_bytes)
    } else {
        let before = store
            .project()
            .map_tileset_member(ctx.pack)?
            .ok_or_else(|| {
                KernelError::Refused(format!(
                    "map texture pack {:04} is not in the archive",
                    ctx.pack
                ))
            })?;
        let after = match appended_pack_or_refusal(&before, &plan.new)? {
            Ok(after) => after,
            Err(m) => return refuse(from_bake(&m, ctx.area, &[])),
        };
        let textures = pack_textures(&after)?;
        let vram = area_vram(store, ctx.area, Some(&after))?;
        (Some(after), textures, vram)
    };
    let converted = match to_faces(&mesh, opts.unit, &plan.bindings) {
        Ok(c) => c,
        Err(reason) => return refuse(reason),
    };
    let report = bake(
        &converted.faces,
        &BakeInput {
            style: ctx.style,
            textures: &textures,
            vram_bytes: vram,
            vram_limit: ctx.vram_limit,
        },
    );
    if let Some(m) = report.refusal() {
        return refuse(from_bake(m, ctx.area, &converted.faces));
    }
    // The own area's VRAM is the bake's warning (same VRAM, same limit).
    let (_, others) = vram_findings(
        store,
        ctx.area,
        ctx.pack,
        after.as_deref(),
        vram,
        ctx.vram_limit,
    )?;
    let state = report
        .messages
        .iter()
        .find(|m| m.code == BakeCode::VramHigh)
        .map_or(PartState::Ok, |m| bake_state(m, ctx.area, &converted.faces));
    let label = format.map_or("OBJ/glTF", SourceFormat::label);
    let stats = MeshStats {
        triangles: report.triangles,
        ..converted.stats
    };
    let mut notes = vec![PartMessage::new(
        PartCode::ModelConverted,
        &[
            &label,
            &stats.polygons,
            &stats.quads,
            &stats.triangles,
            &report.strips,
            &report.single_quads,
        ],
        format!(
            "{label} model converted: {} polygons, {} quads, {} triangles, {} strips, {} \
             single quads",
            stats.polygons, stats.quads, stats.triangles, report.strips, report.single_quads
        ),
    )];
    notes.extend(read_notes);
    notes.extend(converted.notes);
    notes.extend(plan.notes);
    notes.extend(others);
    let limit = Project::limits().model.max_bytes.value;
    let preview = ImportPreview {
        pack: ctx.pack,
        area: ctx.area,
        style: ctx.style,
        vram_bytes: vram,
        vram_limit: ctx.vram_limit,
        textures: plan.previews,
        model: format.map(|format| ModelPreview {
            format,
            stats,
            strips: report.strips,
            single_quads: report.single_quads,
            budget: BudgetItem {
                used: report.size as u64,
                max: limit as u64,
                state: budget_state(report.size as u64, limit as u64),
            },
        }),
        files: vec![path.to_path_buf()],
    };
    Ok(Ok(ConvertedModel {
        faces: converted.faces,
        nsbmd: report.nsbmd,
        pack: ctx.pack,
        new: plan.new,
        preview,
        state,
        notes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bake::Level;

    #[test]
    fn options_default_and_check() {
        let d = ImportOptions::default();
        assert_eq!((d.unit, d.colors), (16.0, 16));
        assert!(d.check().is_ok());
        assert_eq!(d.texconv_colors(), Colors::Sixteen);
        let four = ImportOptions { colors: 4, ..d };
        assert!(four.check().is_ok());
        assert_eq!(four.texconv_colors(), Colors::Four);
        for edge in [UNIT_MIN, UNIT_MAX] {
            assert!(ImportOptions { unit: edge, ..d }.check().is_ok());
        }
        for bad in [
            ImportOptions { unit: 0.0, ..d },
            ImportOptions {
                unit: f32::NAN,
                ..d
            },
            ImportOptions { unit: 300.0, ..d },
            ImportOptions { colors: 8, ..d },
        ] {
            assert!(
                matches!(bad.check(), Err(KernelError::Refused(_))),
                "{bad:?}"
            );
        }
        // Missing keys take the defaults; keys are camelCase.
        let parsed: ImportOptions = serde_json::from_str(r#"{"unit": 1}"#).unwrap();
        assert_eq!(parsed, ImportOptions { unit: 1.0, ..d });
    }

    #[test]
    fn source_format_follows_the_extension() {
        assert_eq!(
            SourceFormat::of(Path::new("a.OBJ")),
            Some(SourceFormat::Obj)
        );
        assert_eq!(
            SourceFormat::of(Path::new("b.glb")),
            Some(SourceFormat::Gltf)
        );
        assert_eq!(
            SourceFormat::of(Path::new("c.gltf")),
            Some(SourceFormat::Gltf)
        );
        assert_eq!(SourceFormat::of(Path::new("d.png")), None);
        assert_eq!(SourceFormat::of(Path::new("obj")), None);
        assert_eq!(SourceFormat::Obj.label(), "OBJ");
        assert_eq!(SourceFormat::Gltf.label(), "glTF");
    }

    #[test]
    fn parts_ignored_lists_each_token_once_in_order() {
        assert_eq!(parts_ignored(&[]), None);
        assert_eq!(parts_ignored(&["unknown"]), None);
        let m = parts_ignored(&["texture-options", "animations", "lines", "animations"]).unwrap();
        assert_eq!(m.code, PartCode::ModelPartsIgnored);
        assert_eq!(m.args, ["lines,animations,texture-options"]);
        assert_eq!(
            m.message,
            "not taken over: lines, animations, texture-options"
        );
    }

    fn bake_message(code: BakeCode, args: &[&str]) -> BakeMessage {
        BakeMessage {
            code,
            level: Level::Error,
            args: args.iter().map(ToString::to_string).collect(),
            message: "why".to_string(),
        }
    }

    #[test]
    fn bake_findings_map_to_part_codes() {
        let m = from_bake(
            &bake_message(BakeCode::ModelTooBig, &["70000", "61440"]),
            3,
            &[],
        );
        assert_eq!(m.code, PartCode::ModelTooBig);
        assert_eq!(m.args, ["70000", "61440"]);
        let m = from_bake(&bake_message(BakeCode::Empty, &[]), 3, &[]);
        assert_eq!((m.code, m.args.len()), (PartCode::ModelNoGeometry, 0));
        let m = from_bake(
            &bake_message(BakeCode::FaceOutOfRange, &["7", "640", "0", "-1"]),
            3,
            &[],
        );
        assert_eq!(m.code, PartCode::ModelOutOfRange);
        assert_eq!(m.args, ["640", "0", "-1", "-512", "511"]);
        let m = from_bake(&bake_message(BakeCode::TextureMissing, &["grass"]), 3, &[]);
        assert_eq!(m.code, PartCode::ModelMaterialUntextured);
        assert_eq!(m.args, ["grass"]);
        // VRAM: `bake` has no area, appending names it; a warning, not an error.
        let mut vram = bake_message(BakeCode::VramHigh, &["120000", "110640"]);
        vram.level = Level::Warning;
        let m = from_bake(&vram, 3, &[]);
        assert_eq!(m.code, PartCode::TextureVramHigh);
        assert_eq!(m.args, ["3", "120000", "110640"]);
        assert_eq!(m.message, "area 3: why");
        assert_eq!(bake_state(&vram, 3, &[]), PartState::Warning(m));
        vram.args = vec!["5".into(), "120000".into(), "110640".into()];
        let m = from_bake(&vram, 3, &[]);
        assert_eq!(
            (m.args.as_slice(), m.message.as_str()),
            (
                &["5".to_string(), "120000".into(), "110640".into()][..],
                "why"
            )
        );
        // The refusals of appending keep their args.
        for (code, args, part) in [
            (
                BakeCode::TextureSide,
                &["kachel", "24", "16"][..],
                PartCode::TextureSide,
            ),
            (BakeCode::TextureName, &["a b"][..], PartCode::TextureName),
            (
                BakeCode::TextureTaken,
                &["palette", "gras_pl"][..],
                PartCode::TextureTaken,
            ),
            (
                BakeCode::PackFull,
                &["entries", "256", "255"][..],
                PartCode::TexturePackFull,
            ),
        ] {
            let found = bake_message(code, args);
            let m = from_bake(&found, 3, &[]);
            assert_eq!((m.code, m.args.as_slice()), (part, &found.args[..]));
            assert_eq!(bake_state(&found, 3, &[]), PartState::Error(m));
        }
        // The uv range names the texture of the face; a face outside the list keeps its index.
        let faces = [Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
            uv: [[0, 0], [0, 16], [4096, 16], [4096, 0]],
            texture: "weit".into(),
            layer: 0,
        }];
        let uv = bake_message(BakeCode::FaceUvRange, &["0", "s", "4096"]);
        let m = from_bake(&uv, 3, &faces);
        assert_eq!(m.code, PartCode::ModelUvRange);
        assert_eq!(m.args, ["weit", "s", "4096"]);
        assert_eq!(m.message, "why");
        assert_eq!(from_bake(&uv, 3, &[]).args, ["0", "s", "4096"]);
    }
}
