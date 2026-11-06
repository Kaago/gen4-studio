//! Import of OBJ and glTF models as the model of a map: the readers fill a neutral [`mesh::Mesh`],
//! [`mesh::to_faces`] turns it into the faces of the baker (`bake::Face`).

pub mod mesh;
mod read_gltf;
mod read_obj;
pub mod samples;

use std::path::{Component, Path, PathBuf};

use g4_formats::nsbtx::Rgba;
use g4_formats::texconv::Colors;

use crate::bake::{BakeCode, BakeMessage, WORLD_MAX, WORLD_MIN};
use crate::error::KernelError;
use crate::map_import::{PartCode, PartMessage};
use mesh::Mesh;

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
/// read only from inside the model's folder ([`resolve_inside`]).
pub fn read_model(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage> {
    match SourceFormat::of(path) {
        Some(SourceFormat::Obj) => read_obj::read_obj(path),
        Some(SourceFormat::Gltf) => read_gltf::read_gltf(path),
        None => Err(PartMessage::new(
            PartCode::ModelSourceUnreadable,
            &[&"OBJ/glTF", &"unknown file extension"],
            format!("{} is neither an OBJ nor a glTF file", path.display()),
        )),
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

/// The pixels of a PNG or JPEG as RGBA8; texture.unreadable `[file, reason]` (`file` as the
/// model writes it).
pub(crate) fn decode_image(bytes: &[u8], file: &str) -> Result<Rgba, PartMessage> {
    let image = image::load_from_memory(bytes).map_err(|e| {
        let reason = e.to_string();
        PartMessage::new(
            PartCode::TextureUnreadable,
            &[&file, &reason],
            format!("the image {file} cannot be read ({reason})"),
        )
    })?;
    let rgba = image.to_rgba8();
    Ok(Rgba {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

/// The finding of `bake` as a part message; `area` is the first argument of the VRAM warning.
///
/// `bake.empty` → `model.no-geometry`; `bake.model-too-big` → `model.too-big` (same args);
/// `bake.face-out-of-range` → `model.out-of-range` (`[x, y, z, -512, 511]`);
/// `bake.texture-missing` → `model.material-untextured` (the texture name: a bound texture the
/// pack has without a palette). The remaining codes (uv range, VRAM, the `texture.*` codes of
/// appending) have no part code yet: they keep the baker's English sentence under
/// `model.invalid` (args: that sentence) until their own codes exist.
///
/// Public, not crate-internal, so the library builds without a dead-code exception before
/// `map_import` calls it.
pub fn from_bake(m: &BakeMessage, area: u8) -> PartMessage {
    let message = m.message.clone();
    match m.code {
        BakeCode::Empty => PartMessage::new(PartCode::ModelNoGeometry, &[], message),
        BakeCode::ModelTooBig => PartMessage {
            code: PartCode::ModelTooBig,
            args: m.args.clone(),
            message,
        },
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
        BakeCode::TextureMissing => PartMessage {
            code: PartCode::ModelMaterialUntextured,
            args: m.args.clone(),
            message,
        },
        BakeCode::VramHigh => {
            let message = format!("area {area}: {message}");
            PartMessage::new(PartCode::ModelInvalid, &[&message], message.clone())
        }
        BakeCode::FaceUvRange
        | BakeCode::TextureSide
        | BakeCode::TextureName
        | BakeCode::TextureTaken
        | BakeCode::PackFull => {
            PartMessage::new(PartCode::ModelInvalid, &[&message], message.clone())
        }
    }
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
        let m = from_bake(&bake_message(BakeCode::ModelTooBig, &["70000", "61440"]), 3);
        assert_eq!(m.code, PartCode::ModelTooBig);
        assert_eq!(m.args, ["70000", "61440"]);
        let m = from_bake(&bake_message(BakeCode::Empty, &[]), 3);
        assert_eq!((m.code, m.args.len()), (PartCode::ModelNoGeometry, 0));
        let m = from_bake(
            &bake_message(BakeCode::FaceOutOfRange, &["7", "640", "0", "-1"]),
            3,
        );
        assert_eq!(m.code, PartCode::ModelOutOfRange);
        assert_eq!(m.args, ["640", "0", "-1", "-512", "511"]);
        let m = from_bake(&bake_message(BakeCode::TextureMissing, &["grass"]), 3);
        assert_eq!(m.code, PartCode::ModelMaterialUntextured);
        assert_eq!(m.args, ["grass"]);
        let m = from_bake(&bake_message(BakeCode::VramHigh, &["9000", "8000"]), 3);
        assert_eq!(m.code, PartCode::ModelInvalid);
        assert_eq!(m.args, ["area 3: why"]);
        let m = from_bake(&bake_message(BakeCode::FaceUvRange, &["0", "s", "4000"]), 3);
        assert_eq!(
            (m.code, m.message.as_str()),
            (PartCode::ModelInvalid, "why")
        );
    }
}
