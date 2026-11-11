//! Import of finished binary map parts into an existing map (model, permissions,
//! buildings, terrain, background sound or a whole map file, also across Platinum and
//! HeartGold/SoulSilver) and export of the same parts.
//!
//! OBJ and glTF files become the map's model through the baker (`model_import`), PNG files one
//! part „textures“ appended to the area's map texture pack.
//!
//! Every check runs before anything is written; a refused import changes nothing. An
//! import is one undo step (the map edit and the textures appended to the pack together), so
//! it is undone like any other map edit.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use g4_formats::map::{BGS_SIGNATURE, BLANK_BGS, BUILDING_LEN, MapFile, PERMISSIONS_LEN};
use g4_formats::nsbmd_header::{
    self, NsbmdHeader, material_palette_names, material_texture_names, model_count,
};
use g4_formats::nsbtx::{NewTexture, Nsbtx};

use crate::bake::{AreaStyle, append_textures, appended_pack_or_refusal, area_vram, bake_context};
use crate::error::{KernelError, io_at};
use crate::model_import::mesh::MeshStats;
use crate::model_import::{
    ImportOptions, SourceFormat, TexturePreview, convert_for_map, from_bake, no_area, png_textures,
    vram_findings,
};
use crate::project::{AssetId, AssetKind, Project};
use crate::store::Store;
use crate::terrain::BudgetItem;

/// Largest map model the game's buffer holds (`MAP_MODEL_FILE_SIZE`, both families,
/// `docs/research/2026-10-23-3b4/task0-3b4.md` §3): `model.max_bytes` of `check_limits.yaml`.
fn model_max_bytes() -> usize {
    Project::limits().model.max_bytes.value
}

/// Largest terrain payload after the 16-byte BDHC header (`BDHC_BUFFER_SIZE`, both
/// families): `bdhc.max_bytes` of `check_limits.yaml`.
fn bdhc_max_bytes() -> usize {
    Project::limits().bdhc.max_bytes.value
}
/// Size of the BDHC header: magic and six `u16` counters.
const BDHC_HEADER_LEN: usize = 16;

/// A part of a map file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    Model,
    Permissions,
    Buildings,
    Bdhc,
    Bgs,
    Whole,
    /// Textures from PNG files, appended to the map texture pack of the map's area (not a part
    /// of the map file: it is neither exported nor taken from another map).
    Textures,
}

impl Part {
    /// The part a file stands for, from its extension: `.obj`, `.gltf`, `.glb` are a model as
    /// well (converted), `.png` textures.
    pub fn from_path(path: &Path) -> Option<Part> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "nsbmd" | "bmd" | "obj" | "gltf" | "glb" => Part::Model,
            "per" | "mp" => Part::Permissions,
            "bld" => Part::Buildings,
            "bdhc" | "bdhcam" => Part::Bdhc,
            "bgs" => Part::Bgs,
            "bin" => Part::Whole,
            "png" => Part::Textures,
            _ => return None,
        })
    }

    /// English name for messages and undo labels.
    pub fn label(self) -> &'static str {
        match self {
            Part::Model => "model",
            Part::Permissions => "permissions",
            Part::Buildings => "buildings",
            Part::Bdhc => "terrain",
            Part::Bgs => "background sound",
            Part::Whole => "whole map",
            Part::Textures => "textures",
        }
    }
}

/// The name of `part` in the language of the process, for the messages that say which part is
/// meant ([`Part::label`] is the English name of the undo labels, which the UI translates).
fn part_name(part: Part) -> String {
    match part {
        Part::Model => t!("map_import.part.model"),
        Part::Permissions => t!("map_import.part.permissions"),
        Part::Buildings => t!("map_import.part.buildings"),
        Part::Bdhc => t!("map_import.part.terrain"),
        Part::Bgs => t!("map_import.part.background_sound"),
        Part::Whole => t!("map_import.part.whole_map"),
        Part::Textures => t!("map_import.part.textures"),
    }
    .into_owned()
}

/// Where the parts come from: files (the extension decides the part) or another map of
/// the project as the store holds it (unsaved edits included).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Files(Vec<PathBuf>),
    Map(u16),
}

/// What a part check can say: errors (the part cannot be imported), warnings and hints. The
/// UI words each code in German from the message's `args`
/// (`apps/studio/src/workbench/inspector/importParts.ts`, held against [`PartCode::ALL`] by a
/// test of the studio).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartCode {
    /// Error, args: the reader's reason. Not an NSBMD container, or its model block unreadable.
    ModelInvalid,
    /// Error, args: the number of `MDL0` blocks (not 1).
    ModelBlocks,
    /// Error, args: the number of models in the `MDL0` block (not 1).
    ModelCount,
    /// Error, args: the model's bytes, the most the game holds.
    ModelTooBig,
    /// Warning, args: the model's bytes (not a multiple of 4).
    ModelUnaligned,
    /// Hint, args: the bytes of the embedded texture block that is removed.
    ModelTextureRemoved,
    /// Hint: a model without terrain next to it; the map's terrain stays.
    ModelBdhcUnchanged,
    /// Hint, args: how many names are missing, the map texture pack, the names.
    ModelTexturesMissing,
    /// Hint: the model's material names cannot be read, so they are not checked.
    ModelTexturesUnreadable,
    /// Hint: no map header shows the map, so the texture names are not checked.
    ModelTexturesNoHeader,
    /// Hint, args: why the map texture packs could not be found.
    ModelTexturesUnchecked,
    /// Error, args: the bytes, the bytes expected.
    PermissionsSize,
    /// Error, args: the bytes, the bytes of one building.
    BuildingsSize,
    /// Error: whole building records that the map codec cannot read.
    BuildingsUnreadable,
    /// Error: no `BDHC` magic.
    BdhcMagic,
    /// Error, args: the bytes, the bytes of the header.
    BdhcShort,
    /// Error, args: the bytes of the terrain data, the most the game holds.
    BdhcTooBig,
    /// Error, args: the bytes, the bytes its counters describe.
    BdhcCounters,
    /// Error: background sound on a Platinum project.
    BgsFamily,
    /// Error, args: the bytes (fewer than 4).
    BgsShort,
    /// Error, args: the signature found, the one expected (hex).
    BgsSignature,
    /// Error, args: the bytes, the bytes it declares.
    BgsLength,
    /// Error, args: the project's family (`Pt`, `Hgss`), the reader's reason.
    WholeUnreadable,
    /// Error, args: the reason the read map file is invalid.
    WholeInvalid,
    /// Warning, args: the bytes of the background sound a Platinum map drops.
    WholeBgsDropped,
    /// Hint, args: the family of the map file (`Pt`, `Hgss`); its buildings stay out.
    WholeForeign,
    /// Hint: an empty background sound block is added (a Platinum map on HeartGold/SoulSilver).
    WholeBlankBgs,
    /// Hint, args: the bytes after the buildings that are kept.
    WholeBuildingsTail,
    /// Error: more than one file for the same part.
    PartDuplicate,
    /// Error, args: the first structure error of the terrain (`BdhcProblem::message`).
    BdhcStructure,
    /// Error, args: the format (`OBJ`, `glTF`), the reader's reason. The model file (or its
    /// material library) cannot be read as that format.
    ModelSourceUnreadable,
    /// Error, args: the path as the model file writes it. A file the model refers to (material
    /// library, image, buffer) does not exist.
    ModelFileMissing,
    /// Error, args: the path as the model file writes it. A file the model refers to lies
    /// outside the model's folder (absolute, `..` out, a URI scheme, a symlink out).
    ModelFileOutside,
    /// Error: a converted model without a face (none read, or every face gone when rounded).
    ModelNoGeometry,
    /// Error, args: the material's name (empty for a polygon without material); it has no
    /// image and the area's pack no texture of its name.
    ModelMaterialUntextured,
    /// Error, args: the material's name; a textured polygon without texture coordinates.
    ModelUvMissing,
    /// Error, args: the corner (x, y, z) in rounded world units, the range (−512, 511).
    ModelOutOfRange,
    /// Error, args: the texture name, the axis (`s`, `t`), the span in texels; a face's uv
    /// span does not fit the game's texture coordinates (at most 2047 texels).
    ModelUvRange,
    /// Error, args: the textures the model uses, the most a map model holds (255).
    ModelTextureCount,
    /// Hint, args: the format (`OBJ`, `glTF`), polygons read, quads, triangles, strips, single
    /// quads; the model was converted.
    ModelConverted,
    /// Hint, args: how many faces have no area once rounded to whole world units (dropped).
    ModelFacesDropped,
    /// Hint, args: what of the model is not taken over, as tokens joined by `,` (`lines`,
    /// `points`, `animations`, `skins`, `cameras`, `lights`, `vertex-colours`,
    /// `texture-options`; in that order, each once).
    ModelPartsIgnored,
    /// Error, args: the map; no header shows it, so there is no area (no texture pack, no light)
    /// for a converted model or textures.
    PartNoArea,
    /// Error, args: the texture name, its width, its height; a side is no power of two from 8
    /// to 1024 (images are not scaled).
    TextureSide,
    /// Error, args: the name; not a name of the pack (empty, over 16 bytes, a byte outside
    /// 0x21..=0x7E).
    TextureName,
    /// Error, args: `texture` or `palette`, the name; already in the area's pack (or twice in
    /// one import). Replacing is not possible.
    TextureTaken,
    /// Error, args: what is full (`entries`, `texture bytes`, `palette bytes`, an offset), its
    /// value, the most the pack holds.
    TexturePackFull,
    /// Error, args: the image file as the model writes it, the decoder's reason.
    TextureUnreadable,
    /// Hint, args: the name, width, height, colours, the mean colour error with one decimal
    /// (`7.6`); a texture converted from an image and appended to the pack.
    TextureNew,
    /// Hint, args: the name; the pack already has a texture of the name, it is used as it is.
    TextureReused,
    /// Warning, args: the area, its VRAM bytes (map pack + buildings pack), the largest vanilla
    /// area's VRAM.
    TextureVramHigh,
}

impl PartCode {
    /// Every code, in the order above.
    pub const ALL: [PartCode; 51] = [
        PartCode::ModelInvalid,
        PartCode::ModelBlocks,
        PartCode::ModelCount,
        PartCode::ModelTooBig,
        PartCode::ModelUnaligned,
        PartCode::ModelTextureRemoved,
        PartCode::ModelBdhcUnchanged,
        PartCode::ModelTexturesMissing,
        PartCode::ModelTexturesUnreadable,
        PartCode::ModelTexturesNoHeader,
        PartCode::ModelTexturesUnchecked,
        PartCode::PermissionsSize,
        PartCode::BuildingsSize,
        PartCode::BuildingsUnreadable,
        PartCode::BdhcMagic,
        PartCode::BdhcShort,
        PartCode::BdhcTooBig,
        PartCode::BdhcCounters,
        PartCode::BgsFamily,
        PartCode::BgsShort,
        PartCode::BgsSignature,
        PartCode::BgsLength,
        PartCode::WholeUnreadable,
        PartCode::WholeInvalid,
        PartCode::WholeBgsDropped,
        PartCode::WholeForeign,
        PartCode::WholeBlankBgs,
        PartCode::WholeBuildingsTail,
        PartCode::PartDuplicate,
        PartCode::BdhcStructure,
        PartCode::ModelSourceUnreadable,
        PartCode::ModelFileMissing,
        PartCode::ModelFileOutside,
        PartCode::ModelNoGeometry,
        PartCode::ModelMaterialUntextured,
        PartCode::ModelUvMissing,
        PartCode::ModelOutOfRange,
        PartCode::ModelUvRange,
        PartCode::ModelTextureCount,
        PartCode::ModelConverted,
        PartCode::ModelFacesDropped,
        PartCode::ModelPartsIgnored,
        PartCode::PartNoArea,
        PartCode::TextureSide,
        PartCode::TextureName,
        PartCode::TextureTaken,
        PartCode::TexturePackFull,
        PartCode::TextureUnreadable,
        PartCode::TextureNew,
        PartCode::TextureReused,
        PartCode::TextureVramHigh,
    ];

    /// The code as the UI gets it (`model.too-big`).
    pub fn as_str(self) -> &'static str {
        match self {
            PartCode::ModelInvalid => "model.invalid",
            PartCode::ModelBlocks => "model.blocks",
            PartCode::ModelCount => "model.count",
            PartCode::ModelTooBig => "model.too-big",
            PartCode::ModelUnaligned => "model.unaligned",
            PartCode::ModelTextureRemoved => "model.texture-removed",
            PartCode::ModelBdhcUnchanged => "model.bdhc-unchanged",
            PartCode::ModelTexturesMissing => "model.textures-missing",
            PartCode::ModelTexturesUnreadable => "model.textures-unreadable",
            PartCode::ModelTexturesNoHeader => "model.textures-no-header",
            PartCode::ModelTexturesUnchecked => "model.textures-unchecked",
            PartCode::PermissionsSize => "permissions.size",
            PartCode::BuildingsSize => "buildings.size",
            PartCode::BuildingsUnreadable => "buildings.unreadable",
            PartCode::BdhcMagic => "bdhc.magic",
            PartCode::BdhcShort => "bdhc.short",
            PartCode::BdhcTooBig => "bdhc.too-big",
            PartCode::BdhcCounters => "bdhc.counters",
            PartCode::BgsFamily => "bgs.family",
            PartCode::BgsShort => "bgs.short",
            PartCode::BgsSignature => "bgs.signature",
            PartCode::BgsLength => "bgs.length",
            PartCode::WholeUnreadable => "whole.unreadable",
            PartCode::WholeInvalid => "whole.invalid",
            PartCode::WholeBgsDropped => "whole.bgs-dropped",
            PartCode::WholeForeign => "whole.foreign",
            PartCode::WholeBlankBgs => "whole.blank-bgs",
            PartCode::WholeBuildingsTail => "whole.buildings-tail",
            PartCode::PartDuplicate => "part.duplicate",
            PartCode::BdhcStructure => "bdhc.structure",
            PartCode::ModelSourceUnreadable => "model.source-unreadable",
            PartCode::ModelFileMissing => "model.file-missing",
            PartCode::ModelFileOutside => "model.file-outside",
            PartCode::ModelNoGeometry => "model.no-geometry",
            PartCode::ModelMaterialUntextured => "model.material-untextured",
            PartCode::ModelUvMissing => "model.uv-missing",
            PartCode::ModelOutOfRange => "model.out-of-range",
            PartCode::ModelUvRange => "model.uv-range",
            PartCode::ModelTextureCount => "model.texture-count",
            PartCode::ModelConverted => "model.converted",
            PartCode::ModelFacesDropped => "model.faces-dropped",
            PartCode::ModelPartsIgnored => "model.parts-ignored",
            PartCode::PartNoArea => "part.no-area",
            PartCode::TextureSide => "texture.side",
            PartCode::TextureName => "texture.name",
            PartCode::TextureTaken => "texture.taken",
            PartCode::TexturePackFull => "texture.pack-full",
            PartCode::TextureUnreadable => "texture.unreadable",
            PartCode::TextureNew => "texture.new",
            PartCode::TextureReused => "texture.reused",
            PartCode::TextureVramHigh => "texture.vram-high",
        }
    }
}

impl serde::Serialize for PartCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What a check of a part found: its `code` with the `args` the UI words it from, and the
/// English `message` with the numbers (the UI's fallback for a code it does not know).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PartMessage {
    pub code: PartCode,
    pub args: Vec<String>,
    pub message: String,
}

impl PartMessage {
    pub(crate) fn new(
        code: PartCode,
        args: &[&dyn std::fmt::Display],
        message: String,
    ) -> PartMessage {
        PartMessage {
            code,
            args: args.iter().map(ToString::to_string).collect(),
            message,
        }
    }

    /// The same finding inside a whole map file: the message names the part it is about.
    fn within(self, part: &str) -> PartMessage {
        PartMessage {
            message: format!("{part}: {}", self.message),
            ..self
        }
    }
}

impl std::fmt::Display for PartMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Outcome of the checks of one part (`{"kind": "error", "code", "args", "message"}`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PartState {
    Ok,
    Warning(PartMessage),
    Error(PartMessage),
}

/// One recognized part with its size (as it would be written; for textures the growth of the
/// pack) and the result of its checks.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PartReport {
    pub part: Part,
    /// The file it came from, `None` for a map of the project.
    #[serde(serialize_with = "crate::relpath::ser_opt")]
    pub file: Option<PathBuf>,
    pub size: usize,
    pub state: PartState,
    /// Hints that never block the import (removed texture block, texture names).
    pub notes: Vec<PartMessage>,
    /// The figures of a converted model or of textures (only for those, and only when their
    /// checks got that far).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ImportPreview>,
}

/// What the dialog shows of a converted model or of textures before the import: the area's
/// map texture pack, the area, its style, its VRAM with the new textures and the largest
/// vanilla area's, every texture used (new or of the pack), the model's figures and the files.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ImportPreview {
    pub pack: u16,
    pub area: u8,
    pub style: AreaStyle,
    pub vram_bytes: usize,
    pub vram_limit: usize,
    pub textures: Vec<TexturePreview>,
    pub model: Option<ModelPreview>,
    #[serde(serialize_with = "crate::relpath::ser_vec")]
    pub files: Vec<PathBuf>,
}

/// The figures of a converted model: its format, the conversion's counts (`stats.triangles` =
/// faces sent as triangles), the quad strips and single quads sent, and its size against the
/// game's buffer (0xF000).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModelPreview {
    pub format: SourceFormat,
    pub stats: MeshStats,
    pub strips: usize,
    pub single_quads: usize,
    pub budget: BudgetItem,
}

/// What would be written for a part.
enum Payload {
    Bytes(Vec<u8>),
    Whole(Box<MapFile>, Family),
    /// A baked model and the textures to append to map texture pack `pack` for it.
    Converted {
        nsbmd: Vec<u8>,
        pack: u16,
        new: Vec<NewTexture>,
    },
    /// Textures to append to map texture pack `pack`.
    Textures {
        pack: u16,
        new: Vec<NewTexture>,
    },
    Nothing,
}

struct Prepared {
    report: PartReport,
    payload: Payload,
}

fn refuse_dp(store: &Store) -> Result<Family, KernelError> {
    match store.project().family {
        Family::Dp => Err(KernelError::Import(
            t!("map_import.dp_not_released").into_owned(),
        )),
        family => Ok(family),
    }
}

/// Checks the parts of `source` for map `map` without changing anything (default
/// [`ImportOptions`]).
pub fn inspect(
    store: &mut Store,
    map: u16,
    source: &Source,
) -> Result<Vec<PartReport>, KernelError> {
    inspect_with(store, map, source, &ImportOptions::default())
}

/// Checks the parts of `source` for map `map` without changing anything; OBJ and glTF are
/// baked and PNG files converted with `opts` (refused before any check when out of range).
pub fn inspect_with(
    store: &mut Store,
    map: u16,
    source: &Source,
    opts: &ImportOptions,
) -> Result<Vec<PartReport>, KernelError> {
    opts.check()?;
    Ok(prepare(store, map, source, opts)?
        .into_iter()
        .map(|p| p.report)
        .collect())
}

/// Imports `parts` of `source` into map `map` as one undo step (default [`ImportOptions`]);
/// see [`import_with`].
pub fn import(
    store: &mut Store,
    map: u16,
    source: &Source,
    parts: &[Part],
    keep_buildings: bool,
) -> Result<Vec<AssetId>, KernelError> {
    import_with(
        store,
        map,
        source,
        parts,
        keep_buildings,
        &ImportOptions::default(),
    )
}

/// Imports `parts` of `source` into map `map` as one undo step. Refuses with the reason
/// of the first selected part that failed its checks, or when a part is not in the source;
/// nothing is changed then. `keep_buildings` takes the buildings of a whole map file of the
/// other family over (they are left out by default). The new textures of every chosen part (a
/// converted model, textures) are appended to the area's map texture pack in one go (a name
/// chosen by two parts is `texture.taken` before anything is written), then the map is
/// written; both are one step `map NNNN: <parts> imported`, and a failed map step rolls the
/// pack back. Returns the changed assets: the map, then the pack (empty when the import leaves
/// both as they were).
pub fn import_with(
    store: &mut Store,
    map: u16,
    source: &Source,
    parts: &[Part],
    keep_buildings: bool,
    opts: &ImportOptions,
) -> Result<Vec<AssetId>, KernelError> {
    let family = refuse_dp(store)?;
    opts.check()?;
    if parts.is_empty() {
        return Err(KernelError::Import(t!("map_import.no_part").into_owned()));
    }
    let mut prepared = prepare(store, map, source, opts)?;
    let mut chosen: Vec<(Part, Payload)> = Vec::new();
    for &part in parts {
        if chosen.iter().any(|(p, _)| *p == part) {
            continue;
        }
        let index = prepared
            .iter()
            .position(|p| p.report.part == part)
            .ok_or_else(|| {
                KernelError::Import(
                    t!("map_import.not_in_source", part = part_name(part)).into_owned(),
                )
            })?;
        let p = prepared.swap_remove(index);
        if let PartState::Error(reason) = &p.report.state {
            return Err(KernelError::Import(format!(
                "{}: {}",
                part.label(),
                reason.message
            )));
        }
        chosen.push((part, p.payload));
    }
    // A whole map goes first, so single parts chosen next to it win; textures last.
    chosen.sort_by_key(|(p, _)| (*p != Part::Whole, *p == Part::Textures));
    let names: Vec<&str> = chosen.iter().map(|(p, _)| p.label()).collect();
    let label = format!("map {map:04}: {} imported", names.join(" and "));
    let (pack, new) = textures_to_append(&chosen)?;
    let edits_map = chosen.iter().any(|(p, _)| *p != Part::Textures);

    let mark = store.undo_depth();
    let mut changed = Vec::new();
    let mut appended = None;
    if let Some(pack) = pack
        && !new.is_empty()
    {
        append_textures(store, pack, &new, &label)?;
        appended = Some(pack);
    }
    if edits_map {
        let before = store.undo_depth();
        let edited = store.edit_map(map, &label, |m| {
            for (part, payload) in chosen {
                apply(m, part, payload, family, keep_buildings);
            }
            Ok(())
        });
        if let Err(e) = edited {
            store.rollback_to(mark)?;
            return Err(e);
        }
        if store.undo_depth() > before {
            changed.push(AssetId {
                kind: AssetKind::Map,
                index: map,
            });
        }
    }
    store.squash_since(mark, &label);
    if let Some(pack) = appended {
        changed.push(AssetId {
            kind: AssetKind::MapTexture,
            index: pack,
        });
    }
    Ok(changed)
}

/// The map texture pack and the new textures of the chosen parts, in order; a texture or
/// palette name chosen by two parts is `texture.taken`.
fn textures_to_append(
    chosen: &[(Part, Payload)],
) -> Result<(Option<u16>, Vec<NewTexture>), KernelError> {
    let mut pack = None;
    let mut new: Vec<NewTexture> = Vec::new();
    for (_, payload) in chosen {
        let (Payload::Converted {
            pack: p, new: n, ..
        }
        | Payload::Textures { pack: p, new: n }) = payload
        else {
            continue;
        };
        pack = Some(*p);
        for t in n {
            let taken = new.iter().find_map(|o| {
                if o.name == t.name || o.palette == t.name {
                    Some(("texture", &t.name))
                } else if o.palette == t.palette || o.name == t.palette {
                    Some(("palette", &t.palette))
                } else {
                    None
                }
            });
            if let Some((what, name)) = taken {
                return Err(KernelError::Import(format!(
                    "{}: {}: the {what} name {name} is chosen by the model and by a PNG file; \
                     rename a file",
                    Part::Textures.label(),
                    PartCode::TextureTaken.as_str()
                )));
            }
            new.push(t.clone());
        }
    }
    Ok((pack, new))
}

fn apply(m: &mut MapFile, part: Part, payload: Payload, family: Family, keep_buildings: bool) {
    match payload {
        Payload::Bytes(bytes) => match part {
            Part::Model => m.model = bytes,
            Part::Permissions => {
                m.permissions = decode_permissions(&bytes);
            }
            Part::Buildings => m.buildings = decode_buildings(&bytes),
            Part::Bdhc => m.bdhc = bytes,
            Part::Bgs => m.bgs = Some(bytes),
            Part::Whole | Part::Textures => {}
        },
        Payload::Converted { nsbmd, .. } => {
            if part == Part::Model {
                m.model = nsbmd;
            }
        }
        Payload::Whole(src, src_family) => {
            let src = *src;
            let same = src_family == family;
            m.permissions = src.permissions;
            m.model = src.model;
            m.bdhc = src.bdhc;
            if same || keep_buildings {
                m.buildings = src.buildings;
                if same {
                    m.buildings_tail = src.buildings_tail;
                }
            }
            m.bgs = match (family, src.bgs) {
                (Family::Hgss, Some(bgs)) => Some(bgs),
                (Family::Hgss, None) => Some(BLANK_BGS.to_vec()),
                _ => None,
            };
        }
        Payload::Textures { .. } | Payload::Nothing => {}
    }
}

fn decode_permissions(bytes: &[u8]) -> Vec<g4_formats::map::Cell> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs
        .iter()
        .map(|c| g4_formats::map::Cell {
            kind: c[0],
            collision: c[1],
        })
        .collect()
}

/// Buildings of a `.bld` file (whole records only; the checks require `n × 48` bytes).
fn decode_buildings(bytes: &[u8]) -> Vec<g4_formats::map::Building> {
    let records = bytes.len() / BUILDING_LEN * BUILDING_LEN;
    parse_buildings(&bytes[..records]).unwrap_or_default()
}

/// Parses whole building records through the map codec (a Platinum map holding only them).
fn parse_buildings(bytes: &[u8]) -> Option<Vec<g4_formats::map::Building>> {
    let mut file = Vec::with_capacity(16 + PERMISSIONS_LEN + bytes.len());
    for len in [PERMISSIONS_LEN, bytes.len(), 0, 0] {
        file.extend_from_slice(&(len as u32).to_le_bytes());
    }
    file.resize(16 + PERMISSIONS_LEN, 0);
    file.extend_from_slice(bytes);
    MapFile::read(&file, Family::Pt).ok().map(|m| m.buildings)
}

/// Writes `part` of map `map` (as the store holds it) to `path`: the model as it is in the
/// map (no texture block), `.per`, `.bld` (whole records), `.bdhc`, `.bgs` (HGSS only) or
/// the whole map file. Textures are refused: they live in the area's map texture pack.
pub fn export(store: &mut Store, map: u16, part: Part, path: &Path) -> Result<(), KernelError> {
    let bytes = part_bytes(store, map, part)?;
    std::fs::write(path, bytes).map_err(io_at(path))
}

/// The bytes [`export`] writes for `part` of map `map`.
pub fn part_bytes(store: &mut Store, map: u16, part: Part) -> Result<Vec<u8>, KernelError> {
    let family = store.project().family;
    let m = store.map(map)?;
    Ok(match part {
        Part::Model => m.model.clone(),
        Part::Permissions => m
            .permissions
            .iter()
            .flat_map(|c| [c.kind, c.collision])
            .collect(),
        Part::Buildings => {
            // Platinum layout: the buildings follow the header and the permissions.
            let file = m.write(Family::Pt);
            let start = 16 + PERMISSIONS_LEN;
            file[start..start + m.buildings.len() * BUILDING_LEN].to_vec()
        }
        Part::Bdhc => m.bdhc.clone(),
        Part::Bgs => match (&m.bgs, family) {
            (Some(bgs), Family::Hgss) => bgs.clone(),
            _ => {
                return Err(KernelError::Import(
                    "background sound exists only on HeartGold/SoulSilver".into(),
                ));
            }
        },
        Part::Whole => m.write(family),
        Part::Textures => {
            return Err(KernelError::Import(
                "textures are not a part of the map file: they live in the area's map texture \
                 pack"
                    .into(),
            ));
        }
    })
}

fn prepare(
    store: &mut Store,
    map: u16,
    source: &Source,
    opts: &ImportOptions,
) -> Result<Vec<Prepared>, KernelError> {
    let family = refuse_dp(store)?;
    // The target must exist; its model is the reference for texture names.
    let old_model = store.map(map)?.model.clone();
    let mut prepared = match source {
        Source::Files(paths) => {
            let mut out = Vec::new();
            let mut pngs = Vec::new();
            for path in paths {
                let part = Part::from_path(path).ok_or_else(|| {
                    KernelError::Import(format!(
                        "{}: unknown file type (expected .nsbmd, .obj, .gltf, .glb, .png, .per, \
                         .bld, .bdhc, .bgs or .bin)",
                        path.display()
                    ))
                })?;
                if part == Part::Textures {
                    pngs.push(path.clone());
                    continue;
                }
                let mut p = if part == Part::Model && SourceFormat::of(path).is_some() {
                    prepare_converted(store, map, path, opts)?
                } else {
                    let bytes = std::fs::read(path).map_err(io_at(path))?;
                    check(part, bytes, family)
                };
                p.report.file = Some(path.clone());
                out.push(p);
            }
            if !pngs.is_empty() {
                out.push(prepare_textures(store, map, &pngs, opts)?);
            }
            mark_duplicates(&mut out);
            out
        }
        Source::Map(src) => {
            let m = store.map(*src)?.clone();
            let mut parts = vec![
                (Part::Model, m.model.clone()),
                (
                    Part::Permissions,
                    m.permissions
                        .iter()
                        .flat_map(|c| [c.kind, c.collision])
                        .collect(),
                ),
                (Part::Buildings, part_bytes(store, *src, Part::Buildings)?),
                (Part::Bdhc, m.bdhc.clone()),
            ];
            if let (Family::Hgss, Some(bgs)) = (family, &m.bgs) {
                parts.push((Part::Bgs, bgs.clone()));
            }
            parts
                .into_iter()
                .map(|(part, bytes)| check(part, bytes, family))
                .collect()
        }
    };
    let has_bdhc = prepared
        .iter()
        .any(|p| matches!(p.report.part, Part::Bdhc | Part::Whole));
    for p in &mut prepared {
        if matches!(p.report.state, PartState::Error(_)) {
            continue;
        }
        if p.report.part == Part::Model && !has_bdhc {
            p.report.notes.push(PartMessage::new(
                PartCode::ModelBdhcUnchanged,
                &[],
                "terrain (BDHC) stays unchanged; check walking heights".into(),
            ));
        }
        // A converted model's names are in the pack once its textures are appended.
        let model = match &p.payload {
            Payload::Bytes(b) if p.report.part == Part::Model => b.clone(),
            Payload::Whole(m, _) => m.model.clone(),
            _ => continue,
        };
        if let Some(note) = texture_note(store, map, &model, &old_model) {
            p.report.notes.push(note);
        }
    }
    Ok(prepared)
}

/// The model part of an OBJ or glTF file: its bake for map `map` ([`convert_for_map`]); a
/// refusal keeps the hints found before it.
fn prepare_converted(
    store: &mut Store,
    map: u16,
    path: &Path,
    opts: &ImportOptions,
) -> Result<Prepared, KernelError> {
    Ok(match convert_for_map(store, map, path, opts)? {
        Ok(c) => {
            let mut p = prepared(
                Part::Model,
                c.nsbmd.len(),
                c.state,
                Payload::Converted {
                    nsbmd: c.nsbmd,
                    pack: c.pack,
                    new: c.new,
                },
            );
            p.report.notes = c.notes;
            p.report.preview = Some(c.preview);
            p
        }
        Err(refusal) => {
            let mut p = error(Part::Model, 0, refusal.reason);
            p.report.notes = refusal.notes;
            p
        }
    })
}

/// The one part „textures“ of the PNG files `files`: converted with `opts`, appended in memory
/// to the map texture pack of the first area that shows map `map`. Its size is the growth of
/// the pack; the VRAM of that area is the state (`Warning(texture.vram-high)`), the other areas
/// of the pack above the vanilla maximum are hints.
fn prepare_textures(
    store: &mut Store,
    map: u16,
    files: &[PathBuf],
    opts: &ImportOptions,
) -> Result<Prepared, KernelError> {
    let part = Part::Textures;
    if map_areas(store, map)?.is_empty() {
        return Ok(error(part, 0, no_area(map)));
    }
    let ctx = bake_context(store, map)?;
    let plan = match png_textures(files, &ctx.textures, opts.texconv_colors()) {
        Ok(plan) => plan,
        Err(reason) => return Ok(error(part, 0, reason)),
    };
    let before = store
        .project()
        .map_tileset_member(ctx.pack)?
        .ok_or_else(|| {
            KernelError::Refused(t!("bake.pack_missing", pack = ctx.pack : {:04}).into_owned())
        })?;
    let after = match appended_pack_or_refusal(&before, &plan.new)? {
        Ok(after) => after,
        Err(m) => return Ok(error(part, 0, from_bake(&m, ctx.area, &[]))),
    };
    let vram = area_vram(store, ctx.area, Some(&after))?;
    let (own, others) = vram_findings(
        store,
        ctx.area,
        ctx.pack,
        Some(&after),
        vram,
        ctx.vram_limit,
    )?;
    let mut p = prepared(
        part,
        after.len().saturating_sub(before.len()),
        own.map_or(PartState::Ok, PartState::Warning),
        Payload::Textures {
            pack: ctx.pack,
            new: plan.new,
        },
    );
    p.report.notes = plan.notes;
    p.report.notes.extend(others);
    p.report.preview = Some(ImportPreview {
        pack: ctx.pack,
        area: ctx.area,
        style: ctx.style,
        vram_bytes: vram,
        vram_limit: ctx.vram_limit,
        textures: plan.previews,
        model: None,
        files: files.to_vec(),
    });
    Ok(p)
}

fn mark_duplicates(prepared: &mut [Prepared]) {
    let mut seen = HashSet::new();
    let dup: HashSet<Part> = prepared
        .iter()
        .filter(|p| !seen.insert(p.report.part))
        .map(|p| p.report.part)
        .collect();
    for p in prepared.iter_mut().filter(|p| dup.contains(&p.report.part)) {
        p.report.state = PartState::Error(PartMessage::new(
            PartCode::PartDuplicate,
            &[],
            format!("more than one file for the {}", p.report.part.label()),
        ));
    }
}

fn prepared(part: Part, size: usize, state: PartState, payload: Payload) -> Prepared {
    Prepared {
        report: PartReport {
            part,
            file: None,
            size,
            state,
            notes: Vec::new(),
            preview: None,
        },
        payload,
    }
}

fn error(part: Part, size: usize, reason: PartMessage) -> Prepared {
    prepared(part, size, PartState::Error(reason), Payload::Nothing)
}

fn check(part: Part, bytes: Vec<u8>, family: Family) -> Prepared {
    let size = bytes.len();
    match part {
        Part::Model => match check_model(&bytes) {
            Ok((model, state, notes)) => {
                let mut p = prepared(part, model.len(), state, Payload::Bytes(model));
                p.report.notes = notes;
                p
            }
            Err(reason) => error(part, size, reason),
        },
        Part::Permissions => {
            if size == PERMISSIONS_LEN {
                prepared(part, size, PartState::Ok, Payload::Bytes(bytes))
            } else {
                error(
                    part,
                    size,
                    PartMessage::new(
                        PartCode::PermissionsSize,
                        &[&size, &PERMISSIONS_LEN],
                        format!("permissions are {size} bytes; expected exactly {PERMISSIONS_LEN}"),
                    ),
                )
            }
        }
        Part::Buildings => {
            if !size.is_multiple_of(BUILDING_LEN) {
                return error(
                    part,
                    size,
                    PartMessage::new(
                        PartCode::BuildingsSize,
                        &[&size, &BUILDING_LEN],
                        format!("buildings are {size} bytes, not a multiple of {BUILDING_LEN}"),
                    ),
                );
            }
            match parse_buildings(&bytes) {
                Some(_) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
                None => error(
                    part,
                    size,
                    PartMessage::new(
                        PartCode::BuildingsUnreadable,
                        &[],
                        "buildings cannot be read".into(),
                    ),
                ),
            }
        }
        Part::Bdhc => match check_bdhc(&bytes) {
            Ok(()) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
            Err(reason) => error(part, size, reason),
        },
        Part::Bgs => {
            if family != Family::Hgss {
                return error(
                    part,
                    size,
                    PartMessage::new(
                        PartCode::BgsFamily,
                        &[],
                        "background sound exists only on HeartGold/SoulSilver".into(),
                    ),
                );
            }
            match check_bgs(&bytes) {
                Ok(()) => prepared(part, size, PartState::Ok, Payload::Bytes(bytes)),
                Err(reason) => error(part, size, reason),
            }
        }
        Part::Whole => check_whole(bytes, family),
        // PNG files are prepared together (`prepare_textures`), never as bytes of a map part.
        Part::Textures => error(
            part,
            size,
            PartMessage::new(
                PartCode::TextureUnreadable,
                &[&"", &"textures come from PNG files"],
                "textures come from PNG files, not from a map".into(),
            ),
        ),
    }
}

/// An unreadable NSBMD container or model block.
fn invalid_model(e: impl std::fmt::Display) -> PartMessage {
    PartMessage::new(
        PartCode::ModelInvalid,
        &[&e],
        format!("not a valid NSBMD model: {e}"),
    )
}

/// Model checks: container header, texture block removed, exactly one model, size limit,
/// alignment. Returns the bytes to write, the state and notes.
fn check_model(bytes: &[u8]) -> Result<(Vec<u8>, PartState, Vec<PartMessage>), PartMessage> {
    NsbmdHeader::read(bytes).map_err(invalid_model)?;
    let model = nsbmd_header::strip_tex0(bytes).map_err(invalid_model)?;
    let mut notes = Vec::new();
    if model.len() != bytes.len() {
        let removed = bytes.len() - model.len();
        notes.push(PartMessage::new(
            PartCode::ModelTextureRemoved,
            &[&removed],
            format!("embedded texture block removed ({removed} bytes)"),
        ));
    }
    check_model_bytes(&model)?;
    let state = if !model.len().is_multiple_of(4) {
        PartState::Warning(PartMessage::new(
            PartCode::ModelUnaligned,
            &[&model.len()],
            format!("model length {} is not a multiple of 4", model.len()),
        ))
    } else {
        PartState::Ok
    };
    Ok((model, state, notes))
}

/// The rules a map model must meet in the game, shared by the import (on the model without
/// its texture block) and the check `map.model-invalid` (on the stored model): a valid
/// container header (`BMD0`, byte-order mark `FFFE`, version 2, size field equal to the
/// length, header 16, 1 or 2 blocks inside the file), exactly one `MDL0` block holding
/// exactly one model, and at most `model.max_bytes`. The error is the reason.
///
/// The model dictionary is read strictly (`g4_formats::nitro_dict::read`, the vanilla Nitro
/// layout: revision 0, the vanilla tree word, the entry offset and the size fields exactly as
/// the writer lays them out); a dictionary in a layout the writer would not produce is
/// refused as an invalid model, even where the game might still load it. This holds for the
/// import and equally for the stored-model check `map.model-invalid` (`checks/world.rs`): a
/// model from another tool with such a dictionary is reported there, where the reader before
/// Phase 6.1 only checked the revision.
pub(crate) fn check_model_bytes(model: &[u8]) -> Result<(), PartMessage> {
    let header = NsbmdHeader::read(model).map_err(invalid_model)?;
    let blocks = header.blocks.iter().filter(|b| &b.tag == b"MDL0").count();
    if blocks != 1 {
        return Err(PartMessage::new(
            PartCode::ModelBlocks,
            &[&blocks],
            format!("the container holds {blocks} model blocks (MDL0); the game needs exactly 1"),
        ));
    }
    let count = model_count(model)
        .map_err(|e| invalid_model(format_args!("model block cannot be read: {e}")))?;
    if count != 1 {
        return Err(PartMessage::new(
            PartCode::ModelCount,
            &[&count],
            format!("the model block holds {count} models; the game needs exactly 1"),
        ));
    }
    let max = model_max_bytes();
    if model.len() > max {
        return Err(PartMessage::new(
            PartCode::ModelTooBig,
            &[&model.len(), &max],
            format!(
                "model is {} bytes; the game holds at most {max} ({max:#X})",
                model.len()
            ),
        ));
    }
    Ok(())
}

/// Terrain checks: magic, counters match the length, payload (counted after the 16-byte
/// header) within the game's buffer, then the structure (the first error of
/// `Bdhc::validate`: every index inside its table, strips inside the access list and in
/// order; a vertical plate is only a warning there and passes). Shared with the check
/// `map.bdhc-size`.
pub(crate) fn check_bdhc(bytes: &[u8]) -> Result<(), PartMessage> {
    if bytes.get(0..4) != Some(b"BDHC".as_slice()) {
        return Err(PartMessage::new(
            PartCode::BdhcMagic,
            &[],
            "terrain does not start with BDHC".into(),
        ));
    }
    if bytes.len() < BDHC_HEADER_LEN {
        return Err(PartMessage::new(
            PartCode::BdhcShort,
            &[&bytes.len(), &BDHC_HEADER_LEN],
            format!(
                "terrain is {} bytes, shorter than its {BDHC_HEADER_LEN}-byte header",
                bytes.len()
            ),
        ));
    }
    let count = |i: usize| usize::from(u16::from_le_bytes([bytes[4 + 2 * i], bytes[5 + 2 * i]]));
    // points, normals, constants, plates, strips, access list
    let payload =
        count(0) * 8 + count(1) * 12 + count(2) * 4 + count(3) * 8 + count(4) * 8 + count(5) * 2;
    let max = bdhc_max_bytes();
    if payload > max {
        return Err(PartMessage::new(
            PartCode::BdhcTooBig,
            &[&payload, &max],
            format!("terrain data is {payload} bytes; the game holds at most {max} ({max:#X})"),
        ));
    }
    if bytes.len() != BDHC_HEADER_LEN + payload {
        let described = BDHC_HEADER_LEN + payload;
        return Err(PartMessage::new(
            PartCode::BdhcCounters,
            &[&bytes.len(), &described],
            format!(
                "terrain is {} bytes but its counters describe {described}",
                bytes.len()
            ),
        ));
    }
    // Cannot fail after the checks above; the reader's reason if it ever does.
    let bdhc = g4_formats::bdhc::Bdhc::read(bytes).map_err(|e| {
        PartMessage::new(
            PartCode::BdhcCounters,
            &[&bytes.len(), &(BDHC_HEADER_LEN + payload)],
            format!("terrain cannot be read: {e}"),
        )
    })?;
    if let Some(p) = bdhc
        .validate()
        .into_iter()
        .find(g4_formats::bdhc::BdhcProblem::is_error)
    {
        return Err(PartMessage::new(
            PartCode::BdhcStructure,
            &[&p.message()],
            format!("terrain structure is invalid: {}", p.message()),
        ));
    }
    Ok(())
}

fn check_bgs(bytes: &[u8]) -> Result<(), PartMessage> {
    if bytes.len() < 4 {
        return Err(PartMessage::new(
            PartCode::BgsShort,
            &[&bytes.len()],
            format!("background sound is {} bytes, shorter than 4", bytes.len()),
        ));
    }
    let signature = u16::from_le_bytes([bytes[0], bytes[1]]);
    if signature != BGS_SIGNATURE {
        let (found, expected) = (format!("{signature:#06x}"), format!("{BGS_SIGNATURE:#06x}"));
        return Err(PartMessage::new(
            PartCode::BgsSignature,
            &[&found, &expected],
            format!("background sound signature is {found}; expected {expected}"),
        ));
    }
    let declared = 4 + usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
    if declared != bytes.len() {
        return Err(PartMessage::new(
            PartCode::BgsLength,
            &[&bytes.len(), &declared],
            format!(
                "background sound is {} bytes but declares {declared}",
                bytes.len()
            ),
        ));
    }
    Ok(())
}

fn other_family(family: Family) -> Family {
    match family {
        Family::Hgss => Family::Pt,
        _ => Family::Hgss,
    }
}

fn family_name(family: Family) -> &'static str {
    match family {
        Family::Hgss => "HeartGold/SoulSilver",
        Family::Pt => "Platinum",
        Family::Dp => "Diamond/Pearl",
    }
}

/// The family as an argument of a [`PartMessage`] (`Pt`, `Hgss`, `Dp`).
fn family_arg(family: Family) -> String {
    format!("{family:?}")
}

fn check_whole(bytes: Vec<u8>, family: Family) -> Prepared {
    let size = bytes.len();
    let (map, src_family) = match MapFile::read(&bytes, family) {
        Ok(m) => (m, family),
        Err(own) => match MapFile::read(&bytes, other_family(family)) {
            Ok(m) => (m, other_family(family)),
            Err(_) => {
                return error(
                    Part::Whole,
                    size,
                    PartMessage::new(
                        PartCode::WholeUnreadable,
                        &[&family_arg(family), &own],
                        format!("not a {} map file: {own}", family_name(family)),
                    ),
                );
            }
        },
    };
    let mut notes = Vec::new();
    let mut warning = None;
    let fail = |reason: PartMessage| error(Part::Whole, size, reason);
    if let Err(e) = map.validate() {
        return fail(PartMessage::new(
            PartCode::WholeInvalid,
            &[&e],
            format!("map file is invalid: {e}"),
        ));
    }
    let model = match check_model(&map.model) {
        Ok((model, state, model_notes)) => {
            notes.extend(model_notes);
            if let PartState::Warning(w) = state {
                warning = Some(w);
            }
            model
        }
        Err(reason) => return fail(reason.within("model")),
    };
    if let Err(reason) = check_bdhc(&map.bdhc) {
        return fail(reason.within("terrain"));
    }
    if src_family != family {
        notes.push(PartMessage::new(
            PartCode::WholeForeign,
            &[&family_arg(src_family)],
            format!(
                "{} map file; its buildings are left out unless taken over",
                family_name(src_family)
            ),
        ));
        match src_family {
            Family::Hgss => {
                let bgs_len = map.bgs.as_ref().map_or(0, Vec::len);
                if bgs_len > BLANK_BGS.len() {
                    warning = Some(PartMessage::new(
                        PartCode::WholeBgsDropped,
                        &[&bgs_len],
                        format!("background sound ({bgs_len} bytes) is dropped; Platinum has none"),
                    ));
                }
            }
            _ => notes.push(PartMessage::new(
                PartCode::WholeBlankBgs,
                &[],
                "an empty background sound block is added".into(),
            )),
        }
    }
    if !map.buildings_tail.is_empty() && src_family == family {
        let tail = map.buildings_tail.len();
        notes.push(PartMessage::new(
            PartCode::WholeBuildingsTail,
            &[&tail],
            format!("{tail} trailing bytes after the buildings are kept"),
        ));
    }
    let mut map = map;
    map.model = model;
    let state = warning.map_or(PartState::Ok, PartState::Warning);
    let mut p = prepared(
        Part::Whole,
        size,
        state,
        Payload::Whole(Box::new(map), src_family),
    );
    p.report.notes = notes;
    p
}

/// Hint about texture and palette names of `model` that the map texture pack of map
/// `map` lacks, leaving out names `old_model` already lacked. Never an error: vanilla maps
/// lack names too (task0-3b4 §6).
fn texture_note(
    store: &mut Store,
    map: u16,
    model: &[u8],
    old_model: &[u8],
) -> Option<PartMessage> {
    let names = |m: &[u8]| -> Option<(Vec<String>, Vec<String>)> {
        Some((
            material_texture_names(m).ok()?,
            material_palette_names(m).ok()?,
        ))
    };
    let Some((tex, pal)) = names(model) else {
        return Some(PartMessage::new(
            PartCode::ModelTexturesUnreadable,
            &[],
            "texture names not checked: materials cannot be read".into(),
        ));
    };
    let packs = match map_packs(store, map) {
        Ok(p) => p,
        Err(e) => {
            return Some(PartMessage::new(
                PartCode::ModelTexturesUnchecked,
                &[&e],
                format!("texture names not checked: {e}"),
            ));
        }
    };
    if packs.is_empty() {
        return Some(PartMessage::new(
            PartCode::ModelTexturesNoHeader,
            &[],
            "texture names not checked: no map header uses this map".into(),
        ));
    }
    let old = names(old_model);
    let mut first = None;
    for pack in packs {
        let bytes = match store.project().map_tileset_member(pack) {
            Ok(Some(b)) => b,
            _ => continue,
        };
        let Ok(btx) = Nsbtx::read(&bytes) else {
            continue;
        };
        let have_tex: HashSet<&str> = btx.textures.iter().map(|t| t.name.as_str()).collect();
        let have_pal: HashSet<&str> = btx.palettes.iter().map(|p| p.name.as_str()).collect();
        let lacks = |name: &String, have: &HashSet<&str>, old: Option<&Vec<String>>| {
            !have.contains(name.as_str()) && !old.is_some_and(|o| o.contains(name))
        };
        let mut missing: Vec<String> = tex
            .iter()
            .filter(|n| lacks(n, &have_tex, old.as_ref().map(|o| &o.0)))
            .chain(
                pal.iter()
                    .filter(|n| lacks(n, &have_pal, old.as_ref().map(|o| &o.1))),
            )
            .cloned()
            .collect();
        missing.dedup();
        if missing.is_empty() {
            return None;
        }
        first.get_or_insert((pack, missing));
    }
    first.map(|(pack, missing)| {
        let (count, names) = (missing.len(), missing.join(", "));
        PartMessage::new(
            PartCode::ModelTexturesMissing,
            &[&count, &pack, &names],
            format!("{count} texture or palette names missing in map texture pack {pack}: {names}"),
        )
    })
}

/// Map texture packs of the area data of every header whose matrix shows map `map`.
fn map_packs(store: &mut Store, map: u16) -> Result<Vec<u16>, KernelError> {
    let mut packs = Vec::new();
    for area in map_areas(store, map)? {
        if let Some(pack) = store.map_tileset_index(area)?
            && !packs.contains(&pack)
        {
            packs.push(pack);
        }
    }
    Ok(packs)
}

/// The area data of every header whose matrix shows map `map`, in header order, each once
/// (a matrix with header ids counts only the cells of that header).
pub fn map_areas(store: &mut Store, map: u16) -> Result<Vec<u8>, KernelError> {
    let headers: Vec<(usize, u16, u8)> = store
        .header_table()?
        .iter()
        .enumerate()
        .map(|(i, h)| (i, h.matrix_id(), h.area_data_id()))
        .collect();
    let matrices = store.member_count(AssetKind::Matrix)?;
    let mut areas = Vec::new();
    for (header, matrix, area) in headers {
        if usize::from(matrix) >= matrices || areas.contains(&area) {
            continue;
        }
        let m = store.matrix(matrix)?;
        let shows = m.maps.iter().enumerate().any(|(i, &cell)| {
            cell == map
                && m.headers
                    .as_ref()
                    .is_none_or(|h| h.get(i).is_some_and(|&id| usize::from(id) == header))
        });
        if shows {
            areas.push(area);
        }
    }
    Ok(areas)
}

/// Writes `model` as the model of map `map` (the checks of a model import first) as one map
/// step `label`. Returns the changed assets (empty when the map already holds the model).
pub fn import_model_bytes(
    store: &mut Store,
    map: u16,
    model: Vec<u8>,
    label: &str,
) -> Result<Vec<AssetId>, KernelError> {
    refuse_dp(store)?;
    let (model, _, _) = check_model(&model).map_err(|reason| {
        KernelError::Import(format!("{}: {}", Part::Model.label(), reason.message))
    })?;
    let depth = store.undo_depth();
    store.edit_map(map, label, |m| {
        m.model = model;
        Ok(())
    })?;
    Ok(if store.undo_depth() > depth {
        vec![AssetId {
            kind: AssetKind::Map,
            index: map,
        }]
    } else {
        Vec::new()
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use crate::project::Project;
    use crate::test_support::synthetic_project;
    use g4_formats::test_support::{bmd0, mdl0_block};
    use std::sync::Arc;

    fn map_id(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::Map,
            index,
        }
    }

    fn model(models: &[&str]) -> Vec<u8> {
        bmd0(&[mdl0_block(models, &["t0"], &["p0"])])
    }

    /// A BDHC with `points` points and nothing else.
    fn bdhc(points: u16) -> Vec<u8> {
        let mut b = b"BDHC".to_vec();
        b.extend_from_slice(&points.to_le_bytes());
        b.resize(16 + usize::from(points) * 8, 0);
        b
    }

    struct Fixture {
        _synth: crate::test_support::Synthetic,
        files: tempfile::TempDir,
        store: Store,
    }

    fn fixture() -> Fixture {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        Fixture {
            _synth: synth,
            files: tempfile::tempdir().unwrap(),
            store: Store::new(Arc::new(project)),
        }
    }

    impl Fixture {
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let p = self.files.path().join(name);
            std::fs::write(&p, bytes).unwrap();
            p
        }
    }

    fn state_of(reports: &[PartReport], part: Part) -> &PartState {
        &reports.iter().find(|r| r.part == part).unwrap().state
    }

    #[test]
    fn inspect_rejects_oversize_and_two_models() {
        let mut f = fixture();
        let mut block = mdl0_block(&["m"], &[], &[]);
        block.resize(model_max_bytes(), 0);
        let len = block.len() as u32;
        block[4..8].copy_from_slice(&len.to_le_bytes());
        let big = f.file("big.nsbmd", &bmd0(&[block]));
        let two = f.file("two.nsbmd", &model(&["a", "b"]));
        let one = f.file("one.nsbmd", &model(&["a"]));
        for (path, want) in [(big, "at most 61440"), (two, "holds 2 models")] {
            let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
            match state_of(&r, Part::Model) {
                PartState::Error(e) => assert!(e.message.contains(want), "{e}"),
                s => panic!("{s:?}"),
            }
        }
        let r = inspect(&mut f.store, 0, &Source::Files(vec![one])).unwrap();
        assert_eq!(state_of(&r, Part::Model), &PartState::Ok);
        assert!(f.store.dirty().is_empty());
    }

    #[test]
    fn inspect_accepts_per_of_2048_bytes_only() {
        let mut f = fixture();
        let ok = f.file("a.per", &[0u8; 2048]);
        let short = f.file("b.mp", &[0u8; 2046]);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![ok])).unwrap();
        assert_eq!(state_of(&r, Part::Permissions), &PartState::Ok);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![short])).unwrap();
        assert!(matches!(
            state_of(&r, Part::Permissions),
            PartState::Error(e) if e.message.contains("2046")
        ));
    }

    #[test]
    fn inspect_checks_bdhc_bgs_and_buildings() {
        let mut f = fixture();
        let good = f.file("a.bdhc", &bdhc(3));
        let mut lying = bdhc(3);
        lying.push(0);
        let lying = f.file("b.bdhc", &lying);
        let mut huge = b"BDHC".to_vec();
        huge.extend_from_slice(&0x1201u16.to_le_bytes());
        huge.resize(16 + 0x1201 * 8, 0);
        let huge = f.file("c.bdhcam", &huge);
        let bgs = f.file("a.bgs", &BLANK_BGS);
        let bld = f.file("a.bld", &[0u8; 47]);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![good])).unwrap();
        assert_eq!(state_of(&r, Part::Bdhc), &PartState::Ok);
        for (path, want) in [
            (lying, "counters"),
            (huge, "at most 36864"),
            (bgs, "only on HeartGold"),
            (bld, "multiple of 48"),
        ] {
            let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
            match &r[0].state {
                PartState::Error(e) => assert!(e.message.contains(want), "{e}"),
                s => panic!("{s:?}"),
            }
        }
    }

    #[test]
    fn strips_the_texture_block_on_import() {
        let mut f = fixture();
        let mut tex0 = b"TEX0".to_vec();
        tex0.extend_from_slice(&24u32.to_le_bytes());
        tex0.resize(24, 1);
        let with = bmd0(&[tex0, mdl0_block(&["m"], &[], &[])]);
        let path = f.file("m.nsbmd", &with);
        let source = Source::Files(vec![path]);
        let r = inspect(&mut f.store, 0, &source).unwrap();
        assert!(
            r[0].notes
                .iter()
                .any(|n| n.message.contains("texture block removed"))
        );
        import(&mut f.store, 0, &source, &[Part::Model], false).unwrap();
        assert_eq!(f.store.map(0).unwrap().model, model_plain());
    }

    fn model_plain() -> Vec<u8> {
        bmd0(&[mdl0_block(&["m"], &[], &[])])
    }

    fn hgss_map(bgs: Vec<u8>) -> Vec<u8> {
        let mut m =
            MapFile::read(&crate::test_support::synthetic_map_bytes(9), Family::Pt).unwrap();
        m.bgs = Some(bgs);
        m.model = model(&["m"]);
        m.bdhc = bdhc(2);
        m.permissions[5].collision = 0x80;
        m.write(Family::Hgss)
    }

    #[test]
    fn whole_bin_from_hgss_to_pt_drops_bgs_with_a_warning() {
        let mut f = fixture();
        let src = hgss_map(vec![0x34, 0x12, 0x02, 0x00, 0xAA, 0xBB]);
        let path = f.file("route.bin", &src);
        let source = Source::Files(vec![path]);
        let r = inspect(&mut f.store, 1, &source).unwrap();
        assert!(matches!(
            state_of(&r, Part::Whole),
            PartState::Warning(w) if w.message.contains("background sound")
        ));
        let target_buildings = f.store.map(1).unwrap().buildings.clone();
        let changed = import(&mut f.store, 1, &source, &[Part::Whole], false).unwrap();
        assert_eq!(changed, vec![map_id(1)]);
        let m = f.store.map(1).unwrap();
        assert_eq!(m.bgs, None);
        assert_eq!(m.model, model(&["m"]));
        assert_eq!(m.bdhc, bdhc(2));
        assert_eq!(m.permissions[5].collision, 0x80);
        assert_eq!(m.buildings, target_buildings);
    }

    #[test]
    fn inspect_checks_the_bdhc_structure() {
        use g4_formats::bdhc::{HeightGrid, from_grid};
        let mut f = fixture();
        let mut broken = from_grid(&HeightGrid::flat(0));
        broken.access[0] = 7;
        let path = f.file("broken.bdhc", &broken.write());
        let source = Source::Files(vec![path]);
        let before = f.store.map(0).unwrap().clone();
        let r = inspect(&mut f.store, 0, &source).unwrap();
        let PartState::Error(e) = state_of(&r, Part::Bdhc) else {
            panic!("{r:?}");
        };
        assert_eq!(e.code, PartCode::BdhcStructure);
        assert_eq!(e.code.as_str(), "bdhc.structure");
        assert_eq!(e.args, ["access entry 0 names plate 7, past the 1 plates"]);
        assert_eq!(
            e.message,
            "terrain structure is invalid: access entry 0 names plate 7, past the 1 plates"
        );
        let err = import(&mut f.store, 0, &source, &[Part::Bdhc], false).unwrap_err();
        assert!(err.to_string().contains("structure"), "{err}");
        assert_eq!(f.store.map(0).unwrap(), &before);
        assert!(f.store.dirty().is_empty());
        assert!(f.store.history().0.is_empty());
        // A generated BDHC imports.
        let sound = f.file("sound.bdhc", &from_grid(&HeightGrid::flat(1)).write());
        let r = inspect(&mut f.store, 0, &Source::Files(vec![sound])).unwrap();
        assert_eq!(state_of(&r, Part::Bdhc), &PartState::Ok);
    }

    #[test]
    fn import_refuses_on_error_and_changes_nothing() {
        let mut f = fixture();
        let per = f.file("a.per", &[0x80u8; 2048]);
        let two = f.file("a.nsbmd", &model(&["a", "b"]));
        let before = f.store.map(0).unwrap().clone();
        let source = Source::Files(vec![per, two]);
        let err = import(
            &mut f.store,
            0,
            &source,
            &[Part::Permissions, Part::Model],
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("model"), "{err}");
        let err = import(&mut f.store, 0, &source, &[Part::Bdhc], false).unwrap_err();
        assert!(err.to_string().contains("not in the source"), "{err}");
        assert_eq!(f.store.map(0).unwrap(), &before);
        assert!(f.store.dirty().is_empty());
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn import_is_one_undo_step_per_map() {
        let mut f = fixture();
        // An unsaved edit first: the import builds on it and undo returns to it.
        f.store
            .edit_map(0, "paint", |m| {
                m.permissions[0].kind = 0x42;
                Ok(())
            })
            .unwrap();
        let unsaved = f.store.current_bytes(map_id(0)).unwrap().unwrap();
        let per = f.file("a.per", &[0x80u8; 2048]);
        let mdl = f.file("a.nsbmd", &model(&["m"]));
        let source = Source::Files(vec![per, mdl]);
        import(
            &mut f.store,
            0,
            &source,
            &[Part::Model, Part::Permissions],
            false,
        )
        .unwrap();
        let (undo, _) = f.store.history();
        assert_eq!(undo.len(), 2);
        assert_eq!(undo[1], "map 0000: model and permissions imported");
        f.store.undo().unwrap();
        assert_eq!(f.store.current_bytes(map_id(0)).unwrap().unwrap(), unsaved);
    }

    #[test]
    fn parts_from_another_map_and_export_round_trip() {
        let mut f = fixture();
        let path = f.files.path().join("out.per");
        export(&mut f.store, 2, Part::Permissions, &path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap().len(), 2048);
        let bld = f.files.path().join("out.bld");
        export(&mut f.store, 2, Part::Buildings, &bld).unwrap();
        assert_eq!(std::fs::read(&bld).unwrap().len(), 48);
        let r = inspect(&mut f.store, 0, &Source::Map(2)).unwrap();
        assert_eq!(state_of(&r, Part::Permissions), &PartState::Ok);
        assert!(matches!(state_of(&r, Part::Model), PartState::Error(_)));
        import(
            &mut f.store,
            0,
            &Source::Files(vec![path, bld]),
            &[Part::Permissions, Part::Buildings],
            false,
        )
        .unwrap();
        // Map 2's permissions and buildings equal map 0's in the fixture: nothing changes.
        assert!(f.store.history().0.is_empty());
        assert!(export(&mut f.store, 0, Part::Bgs, &f.files.path().join("x.bgs")).is_err());
    }

    /// A Platinum map file with a valid model and terrain; `tail` bytes after its buildings.
    fn pt_map(tail: &[u8]) -> Vec<u8> {
        let mut m =
            MapFile::read(&crate::test_support::synthetic_map_bytes(9), Family::Pt).unwrap();
        m.model = model(&["m"]);
        m.bdhc = bdhc(2);
        m.buildings_tail = tail.to_vec();
        m.write(Family::Pt)
    }

    /// A model block `extra` bytes longer than it needs to be (its size field says so).
    fn padded_block(len: usize) -> Vec<u8> {
        let mut block = mdl0_block(&["m"], &[], &[]);
        block.resize(len, 0);
        let size = block.len() as u32;
        block[4..8].copy_from_slice(&size.to_le_bytes());
        block
    }

    /// The code of what the checks of `part` say about `bytes`, and the codes of its hints.
    fn codes_of(part: Part, bytes: Vec<u8>, family: Family) -> (Option<PartCode>, Vec<PartCode>) {
        let p = check(part, bytes, family);
        let state = match p.report.state {
            PartState::Ok => None,
            PartState::Warning(m) | PartState::Error(m) => Some(m.code),
        };
        (state, p.report.notes.iter().map(|n| n.code).collect())
    }

    #[test]
    fn every_error_and_warning_of_a_part_carries_its_code() {
        use PartCode::*;
        let block = mdl0_block(&["m"], &[], &[]);
        let unaligned = padded_block(block.len() + 1);
        let mut tex0 = b"TEX0".to_vec();
        tex0.extend_from_slice(&24u32.to_le_bytes());
        tex0.resize(24, 1);
        let mut huge = b"BDHC".to_vec();
        huge.extend_from_slice(&0x1201u16.to_le_bytes());
        huge.resize(16 + 0x1201 * 8, 0);
        let mut lying = bdhc(3);
        lying.push(0);
        let mut plate = g4_formats::bdhc::Bdhc::flat(0);
        plate.plates[0].normal = 4;
        let bad_index = plate.write();
        let mut two_models = MapFile::read(&pt_map(&[]), Family::Pt).unwrap();
        two_models.model = model(&["a", "b"]);
        let mut bad_terrain = MapFile::read(&pt_map(&[]), Family::Pt).unwrap();
        bad_terrain.bdhc = b"NOPE".to_vec();
        let (pt, hgss) = (Family::Pt, Family::Hgss);
        let refusals: Vec<(Part, Vec<u8>, Family, PartCode)> = vec![
            (Part::Model, b"nope".to_vec(), pt, ModelInvalid),
            (Part::Model, model(&["a", "b"]), pt, ModelCount),
            (
                Part::Model,
                bmd0(&[padded_block(model_max_bytes())]),
                pt,
                ModelTooBig,
            ),
            (Part::Model, bmd0(&[unaligned]), pt, ModelUnaligned),
            (Part::Permissions, vec![0; 10], pt, PermissionsSize),
            (Part::Buildings, vec![0; 47], pt, BuildingsSize),
            (Part::Bdhc, b"NOPE".to_vec(), pt, BdhcMagic),
            (Part::Bdhc, b"BDHC".to_vec(), pt, BdhcShort),
            (Part::Bdhc, huge, pt, BdhcTooBig),
            (Part::Bdhc, lying, pt, BdhcCounters),
            (Part::Bdhc, bad_index, pt, BdhcStructure),
            (Part::Bgs, BLANK_BGS.to_vec(), pt, BgsFamily),
            (Part::Bgs, vec![0x34, 0x12], hgss, BgsShort),
            (Part::Bgs, vec![0, 0, 0, 0], hgss, BgsSignature),
            (Part::Bgs, vec![0x34, 0x12, 5, 0], hgss, BgsLength),
            (Part::Whole, vec![0; 8], pt, WholeUnreadable),
            (Part::Whole, two_models.write(Family::Pt), pt, ModelCount),
            (Part::Whole, bad_terrain.write(Family::Pt), pt, BdhcMagic),
            (
                Part::Whole,
                hgss_map(vec![0x34, 0x12, 2, 0, 0xAA, 0xBB]),
                pt,
                WholeBgsDropped,
            ),
        ];
        // The import keeps the first model block of a container; a stored map model with two
        // is refused by the check `map.model-invalid`, which shares these rules.
        let two_blocks = check_model_bytes(&bmd0(&[block.clone(), block.clone()])).unwrap_err();
        assert_eq!(two_blocks.code, ModelBlocks);
        let mut seen = HashSet::from([ModelBlocks]);
        for (part, bytes, family, want) in refusals {
            let (got, notes) = codes_of(part, bytes, family);
            assert_eq!(got, Some(want), "{part:?}");
            seen.insert(want);
            seen.extend(notes);
        }
        // A model or terrain inside a whole map file keeps its code and names its part in the
        // English message (the dialog puts the rest in brackets behind its own sentence).
        let mut f = fixture();
        let path = f.file("terrain.bin", &bad_terrain.write(Family::Pt));
        let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
        let PartState::Error(e) = state_of(&r, Part::Whole) else {
            panic!("{r:?}");
        };
        assert_eq!(e.code, BdhcMagic);
        assert_eq!(e.message, "terrain: terrain does not start with BDHC");
        let path = f.file("two.bin", &two_models.write(Family::Pt));
        let r = inspect(&mut f.store, 0, &Source::Files(vec![path])).unwrap();
        let PartState::Error(e) = state_of(&r, Part::Whole) else {
            panic!("{r:?}");
        };
        assert_eq!(
            (e.code, e.args.as_slice()),
            (ModelCount, ["2".to_owned()].as_slice())
        );
        assert!(
            e.message.starts_with("model: the model block holds 2"),
            "{e}"
        );

        // The hints.
        let hints: Vec<(Part, Vec<u8>, Family)> = vec![
            (Part::Model, bmd0(&[tex0, block.clone()]), pt),
            (Part::Whole, hgss_map(BLANK_BGS.to_vec()), pt),
            (Part::Whole, pt_map(&[]), hgss),
            (Part::Whole, pt_map(&[1, 2, 3]), pt),
        ];
        for (part, bytes, family) in hints {
            let (state, notes) = codes_of(part, bytes, family);
            assert_eq!(state, None, "{part:?}");
            seen.extend(notes);
        }
        // Map 2 of the fixture is on no matrix, so no header shows it.
        let model_file = f.file("m.nsbmd", &model(&["m"]));
        let r = inspect(&mut f.store, 2, &Source::Files(vec![model_file])).unwrap();
        let notes: Vec<PartCode> = r[0].notes.iter().map(|n| n.code).collect();
        assert_eq!(notes, [ModelBdhcUnchanged, ModelTexturesNoHeader]);
        seen.extend(notes);

        // Two files for one part: both are refused.
        let per = f.file("a.per", &[0u8; 2048]);
        let mp = f.file("b.mp", &[0u8; 2048]);
        let r = inspect(&mut f.store, 0, &Source::Files(vec![per, mp])).unwrap();
        for report in &r {
            let PartState::Error(e) = &report.state else {
                panic!("{report:?}");
            };
            assert_eq!(e.code, PartDuplicate);
        }
        seen.insert(PartDuplicate);

        // Not reached here: whole building records always read, a map file that reads is
        // valid, and the texture hints need materials, packs and headers the fixture lacks.
        // The converted-model codes come from `model_import` (its tests and the converted tests
        // below reach them).
        let unreached = [
            BuildingsUnreadable,
            WholeInvalid,
            ModelTexturesMissing,
            ModelTexturesUnreadable,
            ModelTexturesUnchecked,
            ModelSourceUnreadable,
            ModelFileMissing,
            ModelFileOutside,
            ModelNoGeometry,
            ModelMaterialUntextured,
            ModelUvMissing,
            ModelOutOfRange,
            ModelUvRange,
            ModelTextureCount,
            ModelConverted,
            ModelFacesDropped,
            ModelPartsIgnored,
            PartNoArea,
            TextureSide,
            TextureName,
            TextureTaken,
            TexturePackFull,
            TextureUnreadable,
            TextureNew,
            TextureReused,
            TextureVramHigh,
        ];
        let mut all: HashSet<PartCode> = seen.clone();
        all.extend(unreached);
        assert_eq!(all, PartCode::ALL.into_iter().collect::<HashSet<_>>());
        let names: HashSet<&str> = PartCode::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(names.len(), PartCode::ALL.len());
    }

    #[test]
    fn a_part_state_reaches_the_ui_with_its_code_args_and_english_sentence() {
        let (_, notes) = codes_of(Part::Whole, hgss_map(BLANK_BGS.to_vec()), Family::Pt);
        assert_eq!(notes, [PartCode::WholeForeign]);
        let p = check(Part::Permissions, vec![0; 10], Family::Pt);
        assert_eq!(
            serde_json::to_value(&p.report).unwrap(),
            serde_json::json!({
                "part": "permissions",
                "file": null,
                "size": 10,
                "state": {
                    "kind": "error",
                    "code": "permissions.size",
                    "args": ["10", "2048"],
                    "message": "permissions are 10 bytes; expected exactly 2048",
                },
                "notes": [],
            })
        );
        let p = check(Part::Whole, hgss_map(BLANK_BGS.to_vec()), Family::Pt);
        assert_eq!(
            serde_json::to_value(&p.report.notes).unwrap(),
            serde_json::json!([{
                "code": "whole.foreign",
                "args": ["Hgss"],
                "message": "HeartGold/SoulSilver map file; its buildings are left out unless taken over",
            }])
        );
        assert_eq!(
            serde_json::to_value(PartState::Ok).unwrap(),
            serde_json::json!({ "kind": "ok" })
        );
    }

    #[test]
    fn import_model_bytes_is_one_map_step_and_refuses_oversize() {
        use crate::bake::{AreaStyle, BakeInput, Face, PackTexture, VANILLA_VRAM_MAX_PT, bake};
        let mut f = fixture();
        let old = f.store.map(0).unwrap().clone();

        // 0xF004 bytes: one model, four bytes over the buffer.
        let header = model_plain().len() - mdl0_block(&["m"], &[], &[]).len();
        let mut block = mdl0_block(&["m"], &[], &[]);
        block.resize(0xF004 - header, 0);
        let len = block.len() as u32;
        block[4..8].copy_from_slice(&len.to_le_bytes());
        let big = bmd0(&[block]);
        assert_eq!(big.len(), 0xF004);
        let err = import_model_bytes(&mut f.store, 0, big, "zu groß").unwrap_err();
        assert!(err.to_string().contains("61440"), "{err}");
        assert_eq!(f.store.map(0).unwrap(), &old);
        assert!(f.store.history().0.is_empty());

        let textures = [PackTexture {
            name: "gras".into(),
            palette: "gras_pl".into(),
            width: 16,
            height: 16,
        }];
        let face = Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
            uv: [[0, 0], [0, 16], [16, 16], [16, 0]],
            texture: "gras".into(),
            layer: 0,
        };
        let report = bake(
            &[face],
            &BakeInput {
                style: AreaStyle::Outdoor,
                textures: &textures,
                vram_bytes: 0,
                vram_limit: VANILLA_VRAM_MAX_PT,
            },
        );
        assert!(report.refusal().is_none(), "{:?}", report.messages);
        let changed =
            import_model_bytes(&mut f.store, 0, report.nsbmd.clone(), "Modell gebacken").unwrap();
        assert_eq!(changed, vec![map_id(0)]);
        assert_eq!(f.store.map(0).unwrap().model, report.nsbmd);
        assert_eq!(f.store.history().0, ["Modell gebacken"]);
        // The same model again changes nothing.
        let again =
            import_model_bytes(&mut f.store, 0, report.nsbmd.clone(), "Modell gebacken").unwrap();
        assert!(again.is_empty());
        assert_eq!(f.store.history().0.len(), 1);
        f.store.undo().unwrap();
        assert_eq!(f.store.map(0).unwrap(), &old);
    }

    #[test]
    fn map_areas_lists_the_areas_of_every_header_that_shows_the_map() {
        let mut f = fixture();
        // Both headers use matrix 0 (maps 0 and 1) without header ids; map 2 is in no matrix.
        assert_eq!(map_areas(&mut f.store, 0).unwrap(), [0, 1]);
        assert_eq!(map_areas(&mut f.store, 1).unwrap(), [0, 1]);
        assert!(map_areas(&mut f.store, 2).unwrap().is_empty());
    }

    // --- converted models and textures ---------------------------------------------------

    use crate::model_import::samples::{
        BIG_OBJ, CUBE_OBJ, PDSMS_OBJ, PLANE_GLB, PLANE_GLTF, PNG_40, write_samples,
    };
    use crate::test_support::write_texture_packs;
    use g4_formats::nsbtx::Btx0;

    fn map_texture(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::MapTexture,
            index,
        }
    }

    /// The synthetic project with real texture packs (`t0`, `t1`) and the samples in its folder
    /// `import/`.
    struct Converting {
        synth: crate::test_support::Synthetic,
        store: Store,
    }

    fn converting() -> Converting {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        write_texture_packs(&project);
        write_samples(&synth.dir.path().join("import")).unwrap();
        Converting {
            synth,
            store: Store::new(Arc::new(project)),
        }
    }

    impl Converting {
        fn sample(&self, name: &str) -> PathBuf {
            self.synth.dir.path().join("import").join(name)
        }

        fn files(&self, names: &[&str]) -> Source {
            Source::Files(names.iter().map(|n| self.sample(n)).collect())
        }

        fn member(&self, kind: AssetKind, index: u16) -> Vec<u8> {
            std::fs::read(self.store.project().member_path(kind, index)).unwrap()
        }

        fn pack(&self) -> Vec<u8> {
            self.member(AssetKind::MapTexture, 0)
        }

        fn map_bytes(&mut self, map: u16) -> Vec<u8> {
            self.store.map(map).unwrap();
            self.store.current_bytes(map_id(map)).unwrap().unwrap()
        }

        fn inspect(&mut self, map: u16, names: &[&str]) -> Vec<PartReport> {
            let source = self.files(names);
            inspect(&mut self.store, map, &source).unwrap()
        }

        fn import(&mut self, map: u16, names: &[&str], parts: &[Part]) -> Vec<AssetId> {
            let source = self.files(names);
            import(&mut self.store, map, &source, parts, false).unwrap()
        }

        /// The model `convert_for_map` bakes for `name` into `map` now.
        fn baked(&mut self, map: u16, name: &str) -> Vec<u8> {
            let path = self.sample(name);
            convert_for_map(&mut self.store, map, &path, &ImportOptions::default())
                .unwrap()
                .unwrap()
                .nsbmd
        }
    }

    fn note_codes(report: &PartReport) -> Vec<PartCode> {
        report.notes.iter().map(|n| n.code).collect()
    }

    fn texture_names(pack: &[u8]) -> Vec<String> {
        Nsbtx::read(pack)
            .unwrap()
            .textures
            .into_iter()
            .map(|t| t.name)
            .collect()
    }

    /// The pack `after` starts with every entry and byte of `before`.
    fn keeps_the_old_entries(before: &[u8], after: &[u8]) {
        let (b, a) = (Btx0::read(before).unwrap(), Btx0::read(after).unwrap());
        assert_eq!(a.textures[..b.textures.len()], b.textures[..]);
        assert_eq!(a.palettes[..b.palettes.len()], b.palettes[..]);
        assert_eq!(a.texels[..b.texels.len()], b.texels[..]);
        assert_eq!(a.palette_data[..b.palette_data.len()], b.palette_data[..]);
    }

    #[test]
    fn inspect_converts_a_cube_obj_without_writing() {
        let mut f = converting();
        let pack = f.pack();
        let map = f.map_bytes(0);
        let depth = f.store.undo_depth();
        let r = f.inspect(0, &[CUBE_OBJ]);
        assert_eq!(r.len(), 1);
        let report = &r[0];
        assert_eq!(report.part, Part::Model);
        assert_eq!(report.state, PartState::Ok, "{report:?}");
        let preview = report.preview.as_ref().unwrap();
        let model = preview.model.as_ref().unwrap();
        assert_eq!(model.format, SourceFormat::Obj);
        assert_eq!(model.stats.quads, 6);
        assert_eq!(model.stats.triangles, 0);
        assert_eq!(model.budget.used, report.size as u64);
        assert_eq!(model.budget.max, 0xF000);
        assert_eq!(preview.textures.len(), 1);
        let t = &preview.textures[0];
        assert_eq!(
            (t.name.as_str(), t.new, t.colors),
            ("g4_imp_cube", true, 16)
        );
        assert_eq!((preview.pack, preview.area), (0, 0));
        assert_eq!(preview.files, [f.sample(CUBE_OBJ)]);
        assert_eq!(
            note_codes(report),
            [
                PartCode::ModelConverted,
                PartCode::TextureNew,
                PartCode::ModelBdhcUnchanged
            ]
        );
        assert_eq!(report.notes[0].args[..4], ["OBJ", "6", "6", "0"]);
        // Nothing written.
        assert_eq!(f.store.undo_depth(), depth);
        assert_eq!(f.map_bytes(0), map);
        assert_eq!(f.pack(), pack);
        assert!(f.store.dirty().is_empty());
        // The preview reaches the UI with snake_case keys.
        let json = serde_json::to_value(report).unwrap();
        assert!(json["preview"]["textures"][0]["mean_error"].is_number());
        assert_eq!(json["preview"]["model"]["single_quads"], model.single_quads);
        assert_eq!(json["preview"]["model"]["format"], "obj");
        assert!(json["preview"]["vram_bytes"].is_number());
        assert_eq!(
            json["preview"]["files"][0],
            f.sample(CUBE_OBJ).display().to_string()
        );
    }

    #[test]
    fn import_writes_model_and_pack_as_one_undo_step() {
        let mut f = converting();
        let pack = f.pack();
        let map = f.map_bytes(0);
        let nsbmd = f.baked(0, CUBE_OBJ);
        let changed = f.import(0, &[CUBE_OBJ], &[Part::Model]);
        assert_eq!(changed, [map_id(0), map_texture(0)]);
        assert_eq!(
            f.store.history().0.last().map(String::as_str),
            Some("map 0000: model imported")
        );
        assert_eq!(f.store.undo_depth(), 1);
        assert_eq!(f.store.map(0).unwrap().model, nsbmd);
        let after = f.pack();
        assert_eq!(texture_names(&after), ["t0", "t1", "g4_imp_cube"]);
        keeps_the_old_entries(&pack, &after);
        let imported = f.map_bytes(0);

        f.store.undo().unwrap();
        assert_eq!(f.map_bytes(0), map);
        assert_eq!(f.pack(), pack);
        f.store.redo().unwrap();
        assert_eq!(f.map_bytes(0), imported);
        assert_eq!(f.pack(), after);
    }

    #[test]
    fn the_plane_glb_and_gltf_give_the_same_model() {
        let mut models = Vec::new();
        for name in [PLANE_GLB, PLANE_GLTF] {
            let mut f = converting();
            let r = f.inspect(0, &[name]);
            assert_eq!(r[0].state, PartState::Ok, "{name}: {r:?}");
            let model = r[0].preview.as_ref().unwrap().model.as_ref().unwrap();
            assert_eq!(model.format, SourceFormat::Gltf);
            assert_eq!((model.stats.quads, model.stats.triangles), (1, 0));
            f.import(0, &[name], &[Part::Model]);
            models.push(f.store.map(0).unwrap().model.clone());
        }
        assert_eq!(models[0], models[1]);
    }

    #[test]
    fn the_oversize_obj_is_refused_with_the_number() {
        let mut f = converting();
        let pack = f.pack();
        let map = f.map_bytes(0);
        let r = f.inspect(0, &[BIG_OBJ]);
        let PartState::Error(e) = &r[0].state else {
            panic!("{r:?}");
        };
        assert_eq!(e.code, PartCode::ModelTooBig);
        assert_eq!(e.args[1], "61440");
        let size: usize = e.args[0].parse().unwrap();
        assert!(size > 61_440, "{size}");
        assert!(
            e.message.contains(&e.args[0]) && e.message.contains("61440"),
            "{e}"
        );
        let source = f.files(&[BIG_OBJ]);
        let err = import(&mut f.store, 0, &source, &[Part::Model], false).unwrap_err();
        assert!(err.to_string().contains("61440"), "{err}");
        assert_eq!(f.map_bytes(0), map);
        assert_eq!(f.pack(), pack);
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn the_pdsms_style_obj_imports() {
        let mut f = converting();
        let r = f.inspect(0, &[PDSMS_OBJ]);
        assert_eq!(r[0].state, PartState::Ok, "{r:?}");
        let preview = r[0].preview.as_ref().unwrap();
        assert_eq!(preview.model.as_ref().unwrap().stats.quads, 64);
        let new: Vec<&str> = preview
            .textures
            .iter()
            .filter(|t| t.new)
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(new, ["g4_pds_gras", "g4_pds_weg"]);
        let changed = f.import(0, &[PDSMS_OBJ], &[Part::Model]);
        assert_eq!(changed, [map_id(0), map_texture(0)]);
        assert_eq!(f.store.history().0, ["map 0000: model imported"]);
        assert_eq!(
            texture_names(&f.pack()),
            ["t0", "t1", "g4_pds_gras", "g4_pds_weg"]
        );
    }

    #[test]
    fn pngs_are_one_textures_part_appended_in_one_step() {
        let mut f = converting();
        let pack = f.pack();
        let map = f.map_bytes(0);
        let names = [PNG_40, "g4_imp_cube.png"];
        let r = f.inspect(0, &names);
        assert_eq!(r.len(), 1);
        let report = &r[0];
        assert_eq!(report.part, Part::Textures);
        assert_eq!(report.state, PartState::Ok, "{report:?}");
        let preview = report.preview.as_ref().unwrap();
        assert_eq!(preview.files, [f.sample(names[0]), f.sample(names[1])]);
        assert_eq!(preview.model, None);
        assert_eq!(
            note_codes(report),
            [PartCode::TextureNew, PartCode::TextureNew]
        );
        let changed = f.import(0, &names, &[Part::Textures]);
        assert_eq!(changed, [map_texture(0)]);
        assert_eq!(f.store.history().0, ["map 0000: textures imported"]);
        let after = f.pack();
        assert_eq!(report.size, after.len() - pack.len());
        assert_eq!(
            texture_names(&after),
            ["t0", "t1", "g4_imp_40", "g4_imp_cube"]
        );
        keeps_the_old_entries(&pack, &after);
        assert_eq!(f.map_bytes(0), map);
        f.store.undo().unwrap();
        assert_eq!(f.pack(), pack);
    }

    #[test]
    fn model_and_textures_together_are_one_step() {
        let mut f = converting();
        let pack = f.pack();
        let map = f.map_bytes(0);
        let changed = f.import(0, &[CUBE_OBJ, PNG_40], &[Part::Textures, Part::Model]);
        assert_eq!(changed, [map_id(0), map_texture(0)]);
        assert_eq!(
            f.store.history().0,
            ["map 0000: model and textures imported"]
        );
        assert_eq!(
            texture_names(&f.pack()),
            ["t0", "t1", "g4_imp_cube", "g4_imp_40"]
        );
        f.store.undo().unwrap();
        assert_eq!(f.map_bytes(0), map);
        assert_eq!(f.pack(), pack);
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn a_name_chosen_by_the_model_and_a_png_is_taken_before_writing() {
        let mut f = converting();
        let pack = f.pack();
        let source = f.files(&[CUBE_OBJ, "g4_imp_cube.png"]);
        let r = inspect(&mut f.store, 0, &source).unwrap();
        assert!(r.iter().all(|r| r.state == PartState::Ok), "{r:?}");
        let err = import(
            &mut f.store,
            0,
            &source,
            &[Part::Model, Part::Textures],
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("texture.taken"), "{err}");
        assert!(err.to_string().contains("g4_imp_cube"), "{err}");
        assert_eq!(f.pack(), pack);
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn a_png_name_already_in_the_pack_is_taken() {
        let mut f = converting();
        let png = std::fs::read(f.sample("g4_imp_cube.png")).unwrap();
        std::fs::write(f.sample("t0.png"), png).unwrap();
        let r = f.inspect(0, &["t0.png"]);
        let PartState::Error(e) = &r[0].state else {
            panic!("{r:?}");
        };
        assert_eq!(e.code, PartCode::TextureTaken);
        assert_eq!(e.args, ["texture", "t0"]);
        assert_eq!(r[0].preview, None);
    }

    #[test]
    fn reimporting_the_cube_reuses_its_texture() {
        let mut f = converting();
        f.import(0, &[CUBE_OBJ], &[Part::Model]);
        let pack = f.pack();
        let r = f.inspect(0, &[CUBE_OBJ]);
        let preview = r[0].preview.as_ref().unwrap();
        assert!(!preview.textures[0].new);
        assert!(note_codes(&r[0]).contains(&PartCode::TextureReused));
        assert!(!note_codes(&r[0]).contains(&PartCode::TextureNew));
        // The same model again changes nothing.
        assert!(f.import(0, &[CUBE_OBJ], &[Part::Model]).is_empty());
        assert_eq!(f.store.history().0.len(), 1);
        // Twice the size: a new model with the texture of the pack.
        let source = f.files(&[CUBE_OBJ]);
        let opts = ImportOptions {
            unit: 32.0,
            ..ImportOptions::default()
        };
        let changed = import_with(&mut f.store, 0, &source, &[Part::Model], false, &opts).unwrap();
        assert_eq!(changed, [map_id(0)]);
        assert_eq!(f.pack(), pack);
        assert_eq!(texture_names(&pack), ["t0", "t1", "g4_imp_cube"]);
    }

    #[test]
    fn unchanged_maps_stay_byte_identical() {
        let mut f = converting();
        let others: Vec<(u16, Vec<u8>)> =
            [1u16, 2].into_iter().map(|m| (m, f.map_bytes(m))).collect();
        let archive = f
            .synth
            .dir
            .path()
            .join("files/fielddata/land_data/land_data.narc");
        let land_data = std::fs::read(&archive).unwrap();
        let buildings = f.member(AssetKind::BuildingTexture, 0);
        let packs = f
            .store
            .project()
            .member_count(AssetKind::MapTexture)
            .unwrap();
        f.import(0, &[CUBE_OBJ, PNG_40], &[Part::Model, Part::Textures]);
        for (m, current) in others {
            assert_eq!(f.map_bytes(m), current, "map {m}");
        }
        // The map edit lives in the store until it is saved; the archive on disk is untouched.
        assert_eq!(std::fs::read(&archive).unwrap(), land_data);
        assert_eq!(f.member(AssetKind::BuildingTexture, 0), buildings);
        assert_eq!(
            f.store
                .project()
                .member_count(AssetKind::MapTexture)
                .unwrap(),
            packs
        );
        let dirty: Vec<AssetId> = f.store.dirty().into_iter().collect();
        assert_eq!(dirty, [map_id(0)]);
    }

    #[test]
    fn a_map_without_header_is_part_no_area() {
        let mut f = converting();
        // Map 2 is in no matrix, so no header shows it.
        let r = f.inspect(2, &[CUBE_OBJ, PNG_40]);
        assert_eq!(r.len(), 2);
        for report in &r {
            let PartState::Error(e) = &report.state else {
                panic!("{report:?}");
            };
            assert_eq!(e.code, PartCode::PartNoArea);
            assert_eq!(e.args, ["2"]);
        }
    }

    #[test]
    fn options_out_of_range_are_refused() {
        let mut f = converting();
        let source = f.files(&[CUBE_OBJ]);
        for opts in [
            ImportOptions {
                unit: 0.0,
                ..ImportOptions::default()
            },
            ImportOptions {
                colors: 8,
                ..ImportOptions::default()
            },
        ] {
            assert!(matches!(
                inspect_with(&mut f.store, 0, &source, &opts),
                Err(KernelError::Refused(_))
            ));
            assert!(matches!(
                import_with(&mut f.store, 0, &source, &[Part::Model], false, &opts),
                Err(KernelError::Refused(_))
            ));
        }
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn two_model_files_are_part_duplicate() {
        let mut f = converting();
        let nsbmd = f.sample("m.nsbmd");
        std::fs::write(&nsbmd, model(&["m"])).unwrap();
        let source = Source::Files(vec![f.sample(CUBE_OBJ), nsbmd]);
        let r = inspect(&mut f.store, 0, &source).unwrap();
        assert_eq!(r.len(), 2);
        for report in &r {
            let PartState::Error(e) = &report.state else {
                panic!("{report:?}");
            };
            assert_eq!(e.code, PartCode::PartDuplicate);
        }
    }

    #[test]
    fn textures_cannot_be_exported() {
        let mut f = converting();
        let path = f.synth.dir.path().join("out.png");
        assert!(export(&mut f.store, 0, Part::Textures, &path).is_err());
        assert!(!path.exists());
        assert!(part_bytes(&mut f.store, 0, Part::Textures).is_err());
    }

    #[test]
    fn a_gltf_of_lines_only_has_no_geometry_and_keeps_its_hint() {
        let mut f = converting();
        let text = std::fs::read_to_string(f.sample(PLANE_GLTF)).unwrap();
        let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
        for primitive in document["meshes"][0]["primitives"].as_array_mut().unwrap() {
            primitive["mode"] = serde_json::json!(1);
        }
        std::fs::write(f.sample("linien.gltf"), document.to_string()).unwrap();
        let r = f.inspect(0, &["linien.gltf"]);
        let PartState::Error(e) = &r[0].state else {
            panic!("{r:?}");
        };
        assert_eq!(e.code, PartCode::ModelNoGeometry);
        assert_eq!(note_codes(&r[0]), [PartCode::ModelPartsIgnored]);
        assert_eq!(r[0].notes[0].args, ["lines"]);
    }

    #[test]
    fn the_converted_codes_carry_their_args() {
        // model.converted: format, polygons, quads, triangles, strips, single quads.
        let mut f = converting();
        let r = f.inspect(0, &[PLANE_GLB]);
        let converted = &r[0].notes[0];
        assert_eq!(converted.code, PartCode::ModelConverted);
        let model = r[0].preview.as_ref().unwrap().model.as_ref().unwrap();
        assert_eq!(
            converted.args,
            [
                "glTF".to_string(),
                model.stats.polygons.to_string(),
                "1".into(),
                "0".into(),
                model.strips.to_string(),
                model.single_quads.to_string(),
            ]
        );
        assert_eq!(model.stats.polygons, 2);
    }
}
