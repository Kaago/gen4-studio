//! Free-form models of a world (phase 6.5b), the pure part: the canonical side file of a model
//! and the rules for the names of side files.
//!
//! A free-form model of a world map is a side file `data/maps/NNNN.<name>.glb` (`NNNN` the map,
//! `<name>` from `[a-z0-9_-]{1,32}`); the geometry of a piece is `data/tilesets/PPPP.<name>.glb`.
//! Every side file is the canonical GLB [`side_glb`] writes from the baker's faces: one primitive
//! per texture, one material per texture without an image, named after the pack texture; positions
//! in tiles (world units / [`FREEFORM_UNIT`]), uvs as fractions of the texture (t downwards as in
//! `model_import`), no node transform. [`side_faces`] reads it back through `read_model` and
//! `to_faces`, bound only to the pack's textures: the faces come back as written (up to corners at
//! the position of the one before, which a quad with no area there loses), and a side file written
//! from what was read is the same bytes.
//!
//! The store side: [`load_side_models`] reads the side files a world names (its free-form models
//! and the models of the piece tiles it places) for the bake and the scene, [`world_faces`] is
//! what the scene shows of a world map.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use g4_formats::nitro_dict::name_text;
use g4_formats::nsbmd::Nsbmd;
use g4_formats::nsbtx::NewTexture;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::bake::{
    Verbatim, WorldBakeMessage, WorldBakeReport, bake_pack, bake_world_with, baker_text,
    world_pack_textures,
};
use super::doc::{FreeformJson, WorldJson};
use super::edit::{WorldReport, not_a_world_map, refuse_locked_map, require_text_project};
use super::grid::WorldGrid;
use super::keep::locked_map_refusal;
use super::mesh::{MeshFace, PIECE_LAYER, PieceModels, mesh_textures, mesh_with_marks};
use super::tileset::{TILESETS_DIR, TileRole, Tileset};
use crate::bake::{BakeCode, Face, Level, PackTexture, append_textures, appended_pack_or_refusal};
use crate::error::{KernelError, io_at};
use crate::export::write_glb;
use crate::map_import::{PartCode, PartMessage, map_areas};
use crate::model_import::mesh::{Binding, Mesh, MeshStats, to_faces};
use crate::model_import::{
    ImportOptions, ModelRefusal, TexturePreview, from_bake, plan_textures, read_model,
};
use crate::project::{AssetId, AssetKind, Project, SourceMode};
use crate::store::{ChangeId, FileChange, FileStep, FileStepKind, Store, write_file_step};
use crate::textproject::arts::Art;

/// World units per glTF unit in a side file (one glTF unit is one tile).
pub const FREEFORM_UNIT: f32 = 16.0;
/// World units per unit of a GLB apicula writes.
///
/// Probe 6.5b Task 5 (2026-11-08, apicula `convert -f=glb` of the model of the first map of
/// Zweiblattdorf's first header, Platin DE, map 0, package 6, with the package): positions are
/// world units — the GLB's box (−260, −16, −272) to (292, 58.3125, 320) is the model head's box
/// (`box_pos`/`box_size` × `box_scale`); two nodes (the bone without transform, the mesh with a
/// skin of that bone), one mesh, one primitive per material; the reader leaves out `skins` and
/// `vertex-colours` (`model.parts-ignored`); every glTF material is named as its MDL0 material
/// (`s_snow03_lm2`, which binds texture `s_snow02`); images are `<texture>.png` beside the GLB,
/// a `.` in the texture name written as `_` (`lakep.1` → `lakep_1.png`), none without the
/// package; 1628 triangles in the GLB for 815 polygons (813 quads, 2 triangles) in the MDL0.
pub const APICULA_UNIT: f32 = 1.0;
/// The layer of the faces of free-form models.
pub const FREEFORM_LAYER: u8 = 254;
/// A side file that is missing or does not read.
pub const SIDE_FILE_CODE: &str = "world.side-file";
/// The most characters of `<name>` in a side file name.
pub const SIDE_NAME_MAX: usize = 32;

/// The name when nothing of the stem is left.
const SIDE_NAME_EMPTY: &str = "freiform";
/// The generator a side file names.
const GENERATOR: &str = "gen4-studio";
/// glTF component types and primitive mode.
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const TRIANGLES: u32 = 4;

/// `stem` cleaned for a side file name (lower case, a–z 0–9 `_` `-` kept, every other char `_`, cut to
/// SIDE_NAME_MAX, empty → "freiform"), `_2`, `_3` … while it is in `taken`.
pub fn side_name(stem: &str, taken: &[String]) -> String {
    let clean: String = stem
        .chars()
        .map(|c| match c.to_ascii_lowercase() {
            c @ ('a'..='z' | '0'..='9' | '_' | '-') => c,
            _ => '_',
        })
        .take(SIDE_NAME_MAX)
        .collect();
    let base = if clean.is_empty() {
        SIDE_NAME_EMPTY.to_owned()
    } else {
        clean
    };
    let free = |name: &String| !taken.contains(name);
    if free(&base) {
        return base;
    }
    (2usize..)
        .map(|n| {
            let suffix = format!("_{n}");
            // ASCII only: every char is one byte.
            let keep = base.len().min(SIDE_NAME_MAX.saturating_sub(suffix.len()));
            format!("{}{suffix}", &base[..keep])
        })
        .find(free)
        .unwrap_or(base)
}

/// The file name of free-form model `name` of map `map`: `0123.haus.glb`.
pub fn freeform_file(map: u16, name: &str) -> String {
    format!("{map:04}.{name}.glb")
}

/// The path of the free-form side file `file` in the text project: `data/maps/<file>`.
pub fn freeform_rel(file: &str) -> PathBuf {
    Art::Maps.dir().join(file)
}

/// The path of the model side file `name` of package `package`: `data/tilesets/0006.<name>.glb`.
pub fn piece_rel(package: u16, name: &str) -> PathBuf {
    Path::new(TILESETS_DIR).join(piece_file(package, name))
}

/// The file name of the model side file `name` of package `package`: `0006.<name>.glb`.
fn piece_file(package: u16, name: &str) -> String {
    format!("{package:04}.{name}.glb")
}

/// The side files `world` names (file names in data/maps/): each `freeform[].glb`, once, in
/// document order. Writer and export keep these when they clear a map's other files.
pub fn named_side_files(world: &WorldJson) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for file in world.freeform.iter().filter_map(|f| f.glb.as_deref()) {
        if !names.iter().any(|n| n == file) {
            names.push(file.to_owned());
        }
    }
    names
}

/// The piece model side files `tileset` names (file names in data/tilesets/): `PPPP.<model>.glb`
/// of each tile of role `piece` with a `model`, once, in tile order.
pub fn named_piece_files(tileset: &Tileset) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for tile in tileset.tiles.iter().filter(|t| t.role == TileRole::Piece) {
        if let Some(model) = tile.model.as_deref() {
            let file = piece_file(tileset.package, model);
            if !names.contains(&file) {
                names.push(file);
            }
        }
    }
    names
}

/// The side files the map file `bytes` names: [`named_side_files`] of its world. A world that
/// does not read (a hand-made typo the check reports) still names each string
/// `world.freeform[].glb` of the JSON, so that writing the map never deletes a side file the
/// user is about to fix the world for; no world (a binary map, no JSON): none.
pub(crate) fn named_side_files_in(bytes: &[u8]) -> Vec<String> {
    match super::keep::world_field(bytes) {
        Ok(Some(world)) => named_side_files(&world),
        Ok(None) => Vec::new(),
        Err(_) => {
            let value: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
            let mut names: Vec<String> = Vec::new();
            let entries = value
                .get("world")
                .and_then(|w| w.get("freeform"))
                .and_then(serde_json::Value::as_array);
            for file in entries
                .into_iter()
                .flatten()
                .filter_map(|f| f.get("glb").and_then(serde_json::Value::as_str))
            {
                if !names.iter().any(|n| n == file) {
                    names.push(file.to_owned());
                }
            }
            names
        }
    }
}

/// Whether `name` is a `<name>` of a side file: 1 to SIDE_NAME_MAX of a–z, 0–9, `_`, `-`.
pub(crate) fn is_side_name(name: &str) -> bool {
    (1..=SIDE_NAME_MAX).contains(&name.len())
        && name
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

/// The `<name>` of `file` when it is a side file of `stem` (`NNNN.<name>.glb` with the rule), else None.
pub fn side_name_of(file: &str, stem: u16) -> Option<&str> {
    side_parts(file)
        .filter(|(number, _)| *number == stem)
        .map(|(_, name)| name)
}

/// The number and `<name>` of a side file name `NNNN.<name>.glb` (the number as `{:04}` writes
/// it, the name by the rule), whatever the number; else None.
pub(crate) fn side_parts(file: &str) -> Option<(u16, &str)> {
    let (number, rest) = file.split_once('.')?;
    let name = rest.strip_suffix(".glb")?;
    let n: u16 = number.parse().ok()?;
    let canonical = number.bytes().all(|b| b.is_ascii_digit()) && format!("{n:04}") == number;
    (canonical && is_side_name(name)).then_some((n, name))
}

/// A corner as written: position in world units, uv in texels.
type Corner = ([i32; 3], [i32; 2]);

/// The vertices and triangles of one texture's primitive: quads first, then lone triangles.
#[derive(Default)]
struct Primitive {
    quads: Vec<[Corner; 4]>,
    triangles: Vec<[Corner; 3]>,
}

/// The side file of `faces` (rules „Form des Beifiles“), positions divided by `unit`; a face whose texture
/// is not in `textures` is KernelError::Refused (the caller binds first).
///
/// A face is its corners without one at the position of the corner before it (a triangle repeats
/// corner 2; a wall beside a ramp has two corners at one place); a face of 4 such corners is a
/// quad, written as the triangles `3 0 1` and `3 1 2` (which `to_faces` pairs back into the quad
/// with its corners in order), a face of 3 one triangle; triangles without area are left out. Per
/// texture (in the order of the faces) one primitive, its quads before its lone triangles so that
/// no lone triangle takes half of a quad when the file is read.
pub fn side_glb(
    faces: &[Face],
    textures: &[PackTexture],
    unit: f32,
) -> Result<Vec<u8>, KernelError> {
    let mut order: Vec<(&PackTexture, Primitive)> = Vec::new();
    for (i, face) in faces.iter().enumerate() {
        let Some(texture) = textures.iter().find(|t| t.name == face.texture) else {
            return Err(KernelError::Refused(
                t!(
                    "world.face_texture_missing",
                    face = i,
                    texture = face.texture
                )
                .into_owned(),
            ));
        };
        let at = match order.iter().position(|(t, _)| t.name == texture.name) {
            Some(at) => at,
            None => {
                order.push((texture, Primitive::default()));
                order.len() - 1
            }
        };
        let primitive = &mut order[at].1;
        let corners = distinct_corners(face);
        match corners[..] {
            [a, b, c, d] => {
                let halves = [[d, a, b], [d, b, c]];
                match (has_area(&halves[0]), has_area(&halves[1])) {
                    (true, true) => primitive.quads.push([a, b, c, d]),
                    (true, false) => primitive.triangles.push(halves[0]),
                    (false, true) => primitive.triangles.push(halves[1]),
                    (false, false) => {}
                }
            }
            [a, b, c] if has_area(&[a, b, c]) => primitive.triangles.push([a, b, c]),
            _ => {}
        }
    }

    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    let mut materials = Vec::new();
    for (texture, primitive) in &order {
        let mut corners: Vec<Corner> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for quad in &primitive.quads {
            let base = corners.len() as u32;
            corners.extend_from_slice(quad);
            indices.extend([3, 0, 1, 3, 1, 2].map(|k| base + k));
        }
        for triangle in &primitive.triangles {
            let base = corners.len() as u32;
            corners.extend_from_slice(triangle);
            indices.extend([0, 1, 2].map(|k| base + k));
        }
        if corners.is_empty() {
            continue;
        }
        let size = [f32::from(texture.width), f32::from(texture.height)];
        let positions: Vec<[f32; 3]> = corners
            .iter()
            .map(|c| c.0.map(|v| v as f32 / unit))
            .collect();
        let uvs: Vec<[f32; 2]> = corners
            .iter()
            .map(|c| [c.1[0] as f32 / size[0], c.1[1] as f32 / size[1]])
            .collect();
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &positions {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        let mut view = |bytes: Vec<u8>, target: u32| {
            views.push(json!({
                "buffer": 0,
                "byteOffset": bin.len(),
                "byteLength": bytes.len(),
                "target": target,
            }));
            bin.extend_from_slice(&bytes);
            views.len() - 1
        };
        let floats = |values: &mut dyn Iterator<Item = f32>| -> Vec<u8> {
            values.flat_map(f32::to_le_bytes).collect()
        };
        let position_view = view(floats(&mut positions.iter().flatten().copied()), 34962);
        let uv_view = view(floats(&mut uvs.iter().flatten().copied()), 34962);
        let index_view = view(
            indices.iter().flat_map(|i| i.to_le_bytes()).collect(),
            34963,
        );
        accessors.push(json!({
            "bufferView": position_view,
            "componentType": FLOAT,
            "count": positions.len(),
            "type": "VEC3",
            "min": min,
            "max": max,
        }));
        accessors.push(json!({
            "bufferView": uv_view,
            "componentType": FLOAT,
            "count": uvs.len(),
            "type": "VEC2",
        }));
        accessors.push(json!({
            "bufferView": index_view,
            "componentType": UNSIGNED_INT,
            "count": indices.len(),
            "type": "SCALAR",
        }));
        let first = accessors.len() - 3;
        primitives.push(json!({
            "attributes": {"POSITION": first, "TEXCOORD_0": first + 1},
            "indices": first + 2,
            "material": materials.len(),
            "mode": TRIANGLES,
        }));
        materials.push(json!({"name": texture.name}));
    }

    let mut document = json!({
        "asset": {"version": "2.0", "generator": GENERATOR},
        "scene": 0,
        "scenes": [{}],
    });
    if !primitives.is_empty() {
        document["scenes"] = json!([{"nodes": [0]}]);
        document["nodes"] = json!([{"mesh": 0}]);
        document["meshes"] = json!([{"primitives": primitives}]);
        document["materials"] = materials.into();
        document["accessors"] = accessors.into();
        document["bufferViews"] = views.into();
        document["buffers"] = json!([{"byteLength": bin.len()}]);
    }
    Ok(write_glb(&document, &bin))
}

/// The corners of `face` without a corner at the position of the one before it (cyclically).
fn distinct_corners(face: &Face) -> Vec<Corner> {
    let mut corners: Vec<Corner> = Vec::with_capacity(4);
    for k in 0..4 {
        if corners.last().is_none_or(|c| c.0 != face.corners[k]) {
            corners.push((face.corners[k], face.uv[k]));
        }
    }
    if corners.len() > 1 && corners.first().map(|c| c.0) == corners.last().map(|c| c.0) {
        corners.pop();
    }
    corners
}

/// Whether the triangle has an area in whole world units.
fn has_area(t: &[Corner; 3]) -> bool {
    let p = t.map(|c| c.0.map(i64::from));
    let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    a[1] * b[2] - a[2] * b[1] != 0
        || a[2] * b[0] - a[0] * b[2] != 0
        || a[0] * b[1] - a[1] * b[0] != 0
}

/// The faces of a side file read from `path`, bound only to `textures` by material name, scaled by
/// FREEFORM_UNIT, layer `layer`; `world.side-file` (missing, unreadable, no geometry) or
/// `bake.texture-missing` (first material without a pack texture) as the message.
///
/// `world.side-file` names the file from its folder `data` on (`data/maps/0123.haus.glb`): args
/// `[path]` when it is missing („Das Beifile … fehlt.“), `[path, reason]` when it does not read
/// (the reader's sentence as the reason).
pub fn side_faces(
    path: &Path,
    textures: &[PackTexture],
    layer: u8,
) -> Result<Vec<Face>, WorldBakeMessage> {
    let shown = shown_path(path);
    if matches!(path.try_exists(), Ok(false)) {
        return Err(WorldBakeMessage::error(
            SIDE_FILE_CODE,
            vec![shown.clone()],
            t!("world.side_file_missing", file = shown).into_owned(),
        ));
    }
    let unreadable = |reason: &str| unreadable(&shown, reason);
    let (mesh, _notes) = read_model(path).map_err(|m| unreadable(&m.message))?;
    let bindings: Vec<Option<Binding>> = mesh
        .materials
        .iter()
        .map(|m| {
            textures.iter().find(|t| t.name == m.name).map(|t| Binding {
                texture: t.name.clone(),
                width: t.width,
                height: t.height,
            })
        })
        .collect();
    let converted = to_faces(&mesh, FREEFORM_UNIT, &bindings).map_err(|m| {
        if m.code == PartCode::ModelMaterialUntextured {
            let args = vec![m.args.first().cloned().unwrap_or_default()];
            let message = baker_text(BakeCode::TextureMissing, &args).unwrap_or(m.message);
            WorldBakeMessage::error(BakeCode::TextureMissing.as_str(), args, message)
        } else {
            unreadable(&m.message)
        }
    })?;
    Ok(converted
        .faces
        .into_iter()
        .map(|face| Face { layer, ..face })
        .collect())
}

/// `world.side-file` for the side file shown as `shown` that does not read for `reason`.
fn unreadable(shown: &str, reason: &str) -> WorldBakeMessage {
    WorldBakeMessage::error(
        SIDE_FILE_CODE,
        vec![shown.to_owned(), reason.to_owned()],
        t!("world.side_file_unreadable", file = shown, reason = reason).into_owned(),
    )
}

/// The side files a world reads: per `freeform` entry with `glb` its faces shifted by `at` (layer
/// FREEFORM_LAYER) or the message; per piece tile with `model` that the grid places its faces or the message.
///
/// `messages` holds every side file's message once, in the order of `freeform`, then of the
/// pieces; an `Err` in `freeform` only marks its entry (the bake reports `messages`).
#[derive(Debug, Clone, Default)]
pub struct SideModels {
    pub freeform: Vec<Result<Vec<Face>, WorldBakeMessage>>,
    pub pieces: PieceModels,
    pub messages: Vec<WorldBakeMessage>,
}

impl SideModels {
    /// The faces of every free-form model that read, in `freeform` order.
    pub fn freeform_faces(&self) -> impl Iterator<Item = &Face> {
        self.freeform
            .iter()
            .filter_map(|f| f.as_ref().ok())
            .flatten()
    }
}

/// [`load_side_models_at`] the project's root: the side files of `world` (map `map`) bound to
/// `textures` (the pack textures the bake binds, [`world_pack_textures`]).
pub fn load_side_models(
    project: &Project,
    map: u16,
    world: &WorldJson,
    tileset: &Tileset,
    textures: &[PackTexture],
) -> SideModels {
    load_side_models_at(&project.root, map, world, tileset, textures)
}

/// What one side file is for: a free-form entry (moved by `at`) or the model of a piece tile.
#[derive(Debug, Clone, Copy)]
enum SideKind {
    Freeform { at: [i32; 3] },
    Piece { tile: u16 },
}

/// One side file a world reads: its path relative to the root, the path its messages show, what
/// it is for, and the reason it is not read when its name is no side file name.
#[derive(Debug, Clone)]
struct SideRead {
    rel: PathBuf,
    shown: String,
    kind: SideKind,
    refused: Option<String>,
}

/// The side files of `world` (map `map`) in the order they are read: each `freeform[i].glb`
/// (`data/maps/`), then once per piece tile of `tileset` that `world.pieces` places and that has
/// a `model` (`data/tilesets/PPPP.<model>.glb`; none without `tileset`). A name that is no side
/// file name of the map (or of the package) is planned with the reason it is not read. The one
/// list both [`load_side_models_at`] and [`side_file_rels`] (the input hash) follow.
fn side_plan(map: u16, world: &WorldJson, tileset: Option<&Tileset>) -> Vec<SideRead> {
    let mut plan: Vec<SideRead> = world
        .freeform
        .iter()
        .filter_map(|entry| {
            let file = entry.glb.as_deref()?;
            Some(SideRead {
                rel: freeform_rel(file),
                shown: format!("data/maps/{file}"),
                kind: SideKind::Freeform { at: entry.at },
                refused: side_name_of(file, map)
                    .is_none()
                    .then(|| t!("world.no_side_name_map", map = map : {:04}).into_owned()),
            })
        })
        .collect();
    let Some(tileset) = tileset else {
        return plan;
    };
    let package = world.tileset;
    let mut seen = BTreeSet::new();
    for piece in &world.pieces {
        if !seen.insert(piece.tile) {
            continue;
        }
        let Some(name) = tileset
            .tile(piece.tile)
            .filter(|t| t.role == TileRole::Piece)
            .and_then(|t| t.model.as_deref())
        else {
            continue;
        };
        let rel = piece_rel(package, name);
        plan.push(SideRead {
            shown: crate::relpath::slash(&rel),
            rel,
            kind: SideKind::Piece { tile: piece.tile },
            refused: side_parts(&piece_file(package, name))
                .is_none()
                .then(|| t!("world.no_side_name_package", package = package : {:04}).into_owned()),
        });
    }
    plan
}

/// The side files of `world` (map `map`) below `root` ([`side_plan`]), read with [`side_faces`]
/// against `textures`: free-form models on layer FREEFORM_LAYER, moved by `at`; piece models on
/// layer PIECE_LAYER in model space. A planned name that is no side file name is not read:
/// `world.side-file` with the reason.
pub(crate) fn load_side_models_at(
    root: &Path,
    map: u16,
    world: &WorldJson,
    tileset: &Tileset,
    textures: &[PackTexture],
) -> SideModels {
    let mut side = SideModels::default();
    for read in side_plan(map, world, Some(tileset)) {
        let layer = match read.kind {
            SideKind::Freeform { .. } => FREEFORM_LAYER,
            SideKind::Piece { .. } => PIECE_LAYER,
        };
        let faces = match &read.refused {
            Some(reason) => Err(unreadable(&read.shown, reason)),
            None => side_faces(&root.join(&read.rel), textures, layer),
        };
        if let Err(m) = &faces {
            side.messages.push(m.clone());
        }
        match read.kind {
            SideKind::Freeform { at } => side
                .freeform
                .push(faces.map(|faces| faces.into_iter().map(|f| moved(f, at)).collect())),
            SideKind::Piece { tile } => {
                if let Ok(faces) = faces {
                    side.pieces.by_tile.insert(tile, faces);
                }
            }
        }
    }
    side
}

/// The side files [`load_side_models_at`] reads for `world` (map `map`), relative to the root, in
/// its order ([`side_plan`], none of a piece without `tileset`); names that are no side file name
/// are left out (they are not read).
pub(crate) fn side_file_rels(
    map: u16,
    world: &WorldJson,
    tileset: Option<&Tileset>,
) -> Vec<PathBuf> {
    side_plan(map, world, tileset)
        .into_iter()
        .filter(|read| read.refused.is_none())
        .map(|read| read.rel)
        .collect()
}

/// `face` moved by `at` world units.
fn moved(face: Face, at: [i32; 3]) -> Face {
    Face {
        corners: face
            .corners
            .map(|p| [0, 1, 2].map(|k| p[k].saturating_add(at[k]))),
        ..face
    }
}

/// What the scene shows of world map `map`: grid, piece geometry and free-form faces, per face
/// whether it is model geometry (`model`: a face of a piece's or a free-form model's side file,
/// not a grid face or a footprint), the pack textures they bind, the messages of the side files.
/// `None` without world.
#[derive(Debug, Clone)]
pub struct WorldFaces {
    pub faces: Vec<MeshFace>,
    pub model: Vec<bool>,
    pub textures: Vec<PackTexture>,
    pub messages: Vec<WorldBakeMessage>,
}

/// The faces of world map `map` as the bake sees them: [`mesh_with`](super::mesh::mesh_with)
/// the piece models (no grid faces for a world that does not read into cells, the check names
/// its problems), then the free-form models that read, from the side files bound to the
/// package's [`world_pack_textures`]; `model` marks the faces of the side files
/// ([`mesh_with_marks`] and every free-form face); `textures` are the pack textures of the
/// faces in order of first use (a texture the pack lacks is left out). `None` outside a text
/// project or without world.
pub fn world_faces(store: &mut Store, map: u16) -> Result<Option<WorldFaces>, KernelError> {
    if *store.project().source_mode() != SourceMode::TextProject {
        return Ok(None);
    }
    let Some(world) = store.world(map)?.cloned() else {
        return Ok(None);
    };
    let tileset = store.tileset(world.tileset)?;
    let (_, pack) = bake_pack(store, world.tileset, &tileset, None)?;
    let side = load_side_models(store.project(), map, &world, &tileset, &pack);
    let (mut faces, mut model) = match WorldGrid::parse(&world) {
        Ok(grid) => mesh_with_marks(&grid, &tileset, &side.pieces),
        Err(_) => (Vec::new(), Vec::new()),
    };
    faces.extend(side.freeform_faces().cloned().map(MeshFace::from));
    model.resize(faces.len(), true);
    let textures = mesh_textures(&faces)
        .iter()
        .filter_map(|name| pack.iter().find(|t| &t.name == name).cloned())
        .collect();
    Ok(Some(WorldFaces {
        faces,
        model,
        textures,
        messages: side.messages,
    }))
}

/// `path` from its last folder `data` on, with `/` (`data/maps/0123.haus.glb`); the whole path
/// without such a folder.
fn shown_path(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    match parts.iter().rposition(|p| p == "data") {
        Some(at) => parts[at..].join("/"),
        None => path.display().to_string(),
    }
}

/// `world_freeform_add` and `world_freeform_inspect`: the world map, the model file (OBJ or
/// glTF, read by `read_model`), where it stands (`at`, world units), the import options and the
/// `<name>` of the side file (`None`: the file's stem, [`side_name`] cleans and numbers it).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FreeformAddRequest {
    pub map: u16,
    pub path: PathBuf,
    #[serde(default)]
    pub at: [i32; 3],
    #[serde(default)]
    pub options: ImportOptions,
    #[serde(default)]
    pub name: Option<String>,
}

/// What inspect or add of a free-form model found or did: the map, the side file's name, the
/// figures of the conversion, the textures it uses (new ones planned or appended), the hints
/// (reader, conversion, textures), the undo step (`None` for inspect) and the world's bake with
/// the model.
#[derive(Debug, Clone, Serialize)]
pub struct FreeformReport {
    pub map: u16,
    pub glb: String,
    pub stats: MeshStats,
    pub textures: Vec<TexturePreview>,
    pub notes: Vec<PartMessage>,
    pub change: Option<ChangeId>,
    pub bake: Option<WorldBakeReport>,
}

/// „Free-form added: Map 0123 (0123.haus.glb)“, the label of [`world_freeform_add`].
pub fn freeform_label(map: u16, glb: &str) -> String {
    t!("world.freeform_added_label", map = map : {:04}, glb = glb).into_owned()
}

/// „Free-form removed: Map 0123 (0123.haus.glb)“, the label of [`world_freeform_remove`].
pub fn freeform_remove_label(map: u16, glb: &str) -> String {
    t!("world.freeform_removed_label", map = map : {:04}, glb = glb).into_owned()
}

/// A model file bound against a package: the faces (world units, layer 0), the figures, the
/// textures used, the textures to append, the hints, the package's bytes after appending
/// (`None`: nothing new), the textures the bake binds from those bytes
/// ([`world_pack_textures`]) and the side file written against them.
pub(crate) struct BoundModel {
    pub faces: Vec<Face>,
    pub stats: MeshStats,
    pub previews: Vec<TexturePreview>,
    pub new: Vec<NewTexture>,
    pub notes: Vec<PartMessage>,
    pub pack_after: Option<Vec<u8>>,
    pub textures: Vec<PackTexture>,
    pub glb: Vec<u8>,
}

/// Model file `path` read (`read_model`: every file it names only from its folder), its
/// textures planned against package `package` as the world bake binds it
/// ([`world_pack_textures`] with `tileset`), new ones appended in memory
/// (`appended_pack_or_refusal`, nothing written), its faces converted with `options.unit`, and
/// the side file written against the pack after appending (D2: the names and sizes the bake
/// binds). Inner error: the refusal with its code (`area` names the area of a VRAM or pack
/// finding); outer: the package is missing or does not read, or the options are wrong.
pub(crate) fn bind_model(
    store: &Store,
    package: u16,
    tileset: &Tileset,
    path: &Path,
    options: &ImportOptions,
    area: u8,
) -> Result<Result<BoundModel, ModelRefusal>, KernelError> {
    options.check()?;
    let before = store
        .project()
        .map_tileset_member(package)?
        .ok_or_else(|| {
            KernelError::Refused(
                t!("world.package_not_in_archive", package = package : {:04}).into_owned(),
            )
        })?;
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
    let pack = world_pack_textures(&before, tileset)?;
    let plan = match plan_textures(&mesh.materials, &pack, options.texconv_colors()) {
        Ok(plan) => plan,
        Err(reason) => return refuse(reason),
    };
    let (pack_after, textures) = if plan.new.is_empty() {
        (None, pack)
    } else {
        let after = match appended_pack_or_refusal(&before, &plan.new)? {
            Ok(after) => after,
            Err(m) => return refuse(from_bake(&m, area, &[])),
        };
        let textures = world_pack_textures(&after, tileset)?;
        (Some(after), textures)
    };
    let converted = match to_faces(&mesh, options.unit, &plan.bindings) {
        Ok(converted) => converted,
        Err(reason) => return refuse(reason),
    };
    let glb = side_glb(&converted.faces, &textures, FREEFORM_UNIT)?;
    let mut notes = read_notes.clone();
    notes.extend(converted.notes);
    notes.extend(plan.notes);
    Ok(Ok(BoundModel {
        faces: converted.faces,
        stats: converted.stats,
        previews: plan.previews,
        new: plan.new,
        notes,
        pack_after,
        textures,
        glb,
    }))
}

/// One binding per material of `mesh` by the texture rule of Form 2 against the vanilla `model` and `pack`;
/// Err(material name) for the first that names a texture but binds nothing in the pack.
///
/// Per material: (1) the MDL0 material of the same name in the first model of `model` gives
/// the texture that binds it (`Model.textures[*].materials`), when the pack has that texture;
/// (2) else the stem of the material's image (apicula writes `<texture>.png`, a `.` in the
/// name as `_`), when the pack has a texture of that name. A `model` that does not read has no
/// MDL0 materials: only (2). The binding takes the pack texture's name and size. A material
/// with neither an MDL0 texture nor an image is untextured (`None`: a vertex-colour material
/// such as Maya's `lambert1`), no error.
pub fn bind_vanilla_materials(
    mesh: &Mesh,
    model: &[u8],
    pack: &[PackTexture],
) -> Result<Vec<Option<Binding>>, String> {
    let vanilla = Nsbmd::read(model).ok();
    let mdl0 = vanilla
        .as_ref()
        .and_then(|file| file.models.first())
        .map(|(_, m)| m);
    let mdl0_texture = |material: &str| -> Option<String> {
        let model = mdl0?;
        let index = model
            .materials
            .iter()
            .position(|m| name_text(&m.name) == material)?;
        let index = u8::try_from(index).ok()?;
        model
            .textures
            .iter()
            .find(|p| p.materials.contains(&index))
            .map(|p| name_text(&p.name))
    };
    let in_pack = |name: &str| pack.iter().find(|t| t.name == name);
    mesh.materials
        .iter()
        .map(|material| {
            let by_mdl0 = mdl0_texture(&material.name);
            let by_stem = material.image.as_ref().map(|i| i.stem.as_str());
            if by_mdl0.is_none() && by_stem.is_none() {
                return Ok(None);
            }
            by_mdl0
                .as_deref()
                .and_then(in_pack)
                .or_else(|| by_stem.and_then(in_pack))
                .map(|t| {
                    Some(Binding {
                        texture: t.name.clone(),
                        width: t.width,
                        height: t.height,
                    })
                })
                .ok_or_else(|| material.name.clone())
        })
        .collect()
}

/// The `<name>` a new side file of `stem` (`PPPP` or `NNNN`) in folder `dir` takes: [`side_name`]
/// of `wanted` (the request's name, else `path`'s file stem), numbered past `taken` and past
/// every side file of `stem` already in the folder but `own` (an orphan is not overwritten; the
/// file the new one replaces may be).
pub(crate) fn new_side_name(
    root: &Path,
    dir: &Path,
    stem: u16,
    wanted: Option<&str>,
    path: &Path,
    mut taken: Vec<String>,
    own: Option<&str>,
) -> String {
    if let Ok(entries) = std::fs::read_dir(root.join(dir)) {
        for entry in entries.flatten() {
            if let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|f| side_name_of(f, stem))
                && own != Some(name)
            {
                taken.push(name.to_owned());
            }
        }
    }
    let stem_of_path = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    side_name(wanted.unwrap_or(&stem_of_path), &taken)
}

/// A file step that writes `bytes` to `rel` (below the project root; `None` removes the file),
/// with the bytes there now as `before`; `None` when the file already is so.
pub(crate) fn file_step_at(
    store: &Store,
    asset: AssetId,
    rel: &Path,
    bytes: Option<Vec<u8>>,
) -> Result<Option<FileStep>, KernelError> {
    let path = store.project().root.join(rel);
    let before = match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_at(&path)(e)),
    };
    if before == bytes {
        return Ok(None);
    }
    Ok(Some(FileStep {
        asset,
        files: vec![FileChange {
            path,
            before,
            after: bytes,
        }],
        rotom_entry: None,
        kind: FileStepKind::Reset,
    }))
}

/// One undo step labelled `label` of several recorded ones (R12): `textures` appended to their
/// package (`append_textures`, when there are any), then the file step `files` written and
/// recorded, then `then` (a transaction, another file step); [`Store::squash_since`] the mark
/// before. Any error rolls every step since the mark back ([`Store::rollback_to`]) and is
/// returned (a rollback that fails too: [`KernelError::Partial`] with both).
pub(crate) fn in_one_step<T>(
    store: &mut Store,
    label: &str,
    textures: Option<(u16, &[NewTexture])>,
    files: Option<FileStep>,
    then: impl FnOnce(&mut Store) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    let mark = store.undo_depth();
    let result = (|| {
        if let Some((package, new)) = textures
            && !new.is_empty()
        {
            append_textures(store, package, new, label)?;
        }
        if let Some(step) = files {
            write_file_step(store.project(), &step)?;
            store.record_file_step(label, step);
        }
        then(store)
    })();
    match result {
        Ok(value) => {
            store.squash_since(mark, label);
            Ok(value)
        }
        Err(e) => match store.rollback_to(mark) {
            Ok(_) => Err(e),
            Err(undo) => Err(KernelError::Partial(
                t!("world.undo_incomplete", error = e, undo = undo).into_owned(),
            )),
        },
    }
}

/// What inspect binds and bakes for an add: the report (no change), the textures to append,
/// the side file, its name and the world with the new entry.
struct Prepared {
    report: FreeformReport,
    package: u16,
    new: Vec<NewTexture>,
    glb: Vec<u8>,
    world: WorldJson,
}

/// Inspect's work for `req` (see [`world_freeform_inspect`]).
fn prepare(
    store: &mut Store,
    req: &FreeformAddRequest,
) -> Result<Result<Prepared, ModelRefusal>, KernelError> {
    require_text_project(store)?;
    let map = req.map;
    let world = store
        .world(map)?
        .cloned()
        .ok_or_else(|| KernelError::Refused(not_a_world_map(map)))?;
    let package = world.tileset;
    let tileset = store.tileset(package)?;
    let area = map_areas(store, map)?.first().copied().unwrap_or(0);
    let bound = match bind_model(store, package, &tileset, &req.path, &req.options, area)? {
        Ok(bound) => bound,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let taken: Vec<String> = named_side_files(&world)
        .iter()
        .filter_map(|f| side_name_of(f, map).map(str::to_owned))
        .collect();
    let name = new_side_name(
        &store.project().root,
        &Art::Maps.dir(),
        map,
        req.name.as_deref(),
        &req.path,
        taken,
        None,
    );
    let glb = freeform_file(map, &name);
    // The bake with the model: the side files there now, and the new one from memory.
    let mut side = load_side_models(store.project(), map, &world, &tileset, &bound.textures);
    side.freeform.push(Ok(bound
        .faces
        .iter()
        .cloned()
        .map(|f| {
            moved(
                Face {
                    layer: FREEFORM_LAYER,
                    ..f
                },
                req.at,
            )
        })
        .collect()));
    let mut after = world;
    after.freeform.push(FreeformJson {
        glb: Some(glb.clone()),
        nsbmd: None,
        at: req.at,
    });
    let base = store.map(map)?.clone();
    let verbatim = Verbatim::of(&after, &base);
    let baked = bake_world_with(
        store,
        map,
        &after,
        &base,
        &verbatim,
        bound.pack_after.as_deref(),
        Some(&side),
    )?;
    Ok(Ok(Prepared {
        report: FreeformReport {
            map,
            glb,
            stats: bound.stats,
            textures: bound.previews,
            notes: bound.notes,
            change: None,
            bake: Some(baked.report),
        },
        package,
        new: bound.new,
        glb: bound.glb,
        world: after,
    }))
}

/// Reads and binds the model against the world's pack (new textures planned and appended in
/// memory with `appended_pack_or_refusal`, nothing written) and bakes the world with it in
/// memory (`report.bake`, also when it is refused). Inner Err: the model's refusal
/// (ModelRefusal).
///
/// Refused ([`KernelError::Refused`]): outside a text project
/// ([`super::edit::world_needs_text_project`]), a map without world ([`not_a_world_map`]),
/// options out of range.
pub fn world_freeform_inspect(
    store: &mut Store,
    req: &FreeformAddRequest,
) -> Result<Result<FreeformReport, ModelRefusal>, KernelError> {
    Ok(prepare(store, req)?.map(|p| p.report))
}

/// „Free-form not added: …“ with the first error of a bake (or the model's refusal).
fn not_added(why: &str) -> KernelError {
    KernelError::Refused(t!("world.freeform_not_added", why = why).into_owned())
}

/// One undo step (squash_since, label freeform_label): new textures appended to `world.tileset`,
/// the side file written as a file step, the entry appended to `world.freeform`; refused
/// (Refused with the German sentence) when inspect refuses or its bake is not `ok`; any failure
/// rolls back to the mark.
///
/// Refused besides, nothing written: a busy store, a map whose member is locked, what
/// [`world_freeform_inspect`] refuses.
pub fn world_freeform_add(
    store: &mut Store,
    req: &FreeformAddRequest,
) -> Result<FreeformReport, KernelError> {
    store.project().editable()?;
    if let Some(why) = store.busy() {
        return Err(KernelError::Busy(why.to_owned()));
    }
    require_text_project(store)?;
    let map = req.map;
    if store.world(map)?.is_none() {
        return Err(KernelError::Refused(not_a_world_map(map)));
    }
    refuse_locked_map(store, map, |_| locked_map_refusal(map))?;
    let prepared = match prepare(store, req)? {
        Ok(prepared) => prepared,
        Err(refusal) => {
            return Err(not_added(&format!(
                "{} ({})",
                refusal.reason.message,
                refusal.reason.code.as_str()
            )));
        }
    };
    let bake = prepared.report.bake.as_ref();
    if let Some(m) = bake.and_then(|b| b.messages.iter().find(|m| m.level == Level::Error)) {
        return Err(not_added(&m.message));
    }
    let label = freeform_label(map, &prepared.report.glb);
    let step = file_step_at(
        store,
        world_asset(map),
        &freeform_rel(&prepared.report.glb),
        Some(prepared.glb),
    )?;
    let world = prepared.world;
    let (bake, change) = in_one_step(
        store,
        &label,
        Some((prepared.package, &prepared.new)),
        step,
        |store| {
            store.transaction(&label, |tx| {
                tx.edit_world(map, |w| {
                    *w = Some(world);
                    Ok(())
                })?;
                Ok(tx.bake_report(map))
            })
        },
    )?;
    Ok(FreeformReport {
        change,
        bake,
        ..prepared.report
    })
}

/// One undo step „Freiform entfernt: Karte 0123 (0123.haus.glb)“: entry `index` removed, its side
/// file removed as a file step when no other entry names it. Refused for a `nsbmd` entry and a
/// past index.
///
/// Refused besides ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`super::edit::world_needs_text_project`]), a map without world ([`not_a_world_map`]), a
/// map whose member is locked ([`locked_map_refusal`]), a busy store.
pub fn world_freeform_remove(
    store: &mut Store,
    map: u16,
    index: usize,
) -> Result<WorldReport, KernelError> {
    store.project().editable()?;
    if let Some(why) = store.busy() {
        return Err(KernelError::Busy(why.to_owned()));
    }
    require_text_project(store)?;
    let mut world = store
        .world(map)?
        .cloned()
        .ok_or_else(|| KernelError::Refused(not_a_world_map(map)))?;
    refuse_locked_map(store, map, |_| locked_map_refusal(map))?;
    let count = world.freeform.len();
    let Some(entry) = world.freeform.get(index) else {
        return Err(KernelError::Refused(
            t!(
                "world.freeform_missing",
                map = map : {:04},
                index = index,
                count = count
            )
            .into_owned(),
        ));
    };
    let Some(glb) = entry.glb.clone() else {
        return Err(KernelError::Refused(
            t!("world.freeform_is_original", index = index, map = map : {:04}).into_owned(),
        ));
    };
    world.freeform.remove(index);
    let shared = world
        .freeform
        .iter()
        .any(|f| f.glb.as_deref() == Some(glb.as_str()));
    let step = if shared || side_name_of(&glb, map).is_none() {
        None
    } else {
        file_step_at(store, world_asset(map), &freeform_rel(&glb), None)?
    };
    let label = freeform_remove_label(map, &glb);
    let package = world.tileset;
    let (bake, change) = in_one_step(store, &label, None, step, |store| {
        store.transaction(&label, |tx| {
            tx.edit_world(map, |w| {
                *w = Some(world);
                Ok(())
            })?;
            Ok(tx.bake_report(map))
        })
    })?;
    Ok(WorldReport {
        map,
        tileset: package,
        cells: 0,
        change,
        bake,
        notes: Vec::new(),
    })
}

/// The world map asset of map `map`, the asset of its side files' steps.
pub(crate) fn world_asset(map: u16) -> AssetId {
    AssetId {
        kind: AssetKind::WorldMap,
        index: map,
    }
}

/// Synthetic side-file geometry for tests (no ROM data).
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    use crate::bake::Face;

    /// The six faces of the box from `lo` to `hi` (world units), counter-clockwise seen from
    /// outside, uv 0 to 16 texels, texture `texture`, layer 0.
    pub fn box_faces(lo: [i32; 3], hi: [i32; 3], texture: &str) -> Vec<Face> {
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        let quads = [
            // Top, bottom.
            [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            // East, west.
            [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
            [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
            // South, north.
            [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
        ];
        quads
            .into_iter()
            .map(|corners| Face {
                corners,
                uv: [[0, 16], [16, 16], [16, 0], [0, 0]],
                texture: texture.to_owned(),
                layer: 0,
            })
            .collect()
    }

    /// The box of 16 world units around the vertical axis through the origin, standing on it:
    /// x and z from −8 to 8, y from 0 to 16.
    pub fn cube_faces(texture: &str) -> Vec<Face> {
        box_faces([-8, 0, -8], [8, 16, 8], texture)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod store_tests;

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use proptest::prelude::*;

    use super::*;
    use crate::bake::Level;
    use crate::world::bake::synthetic_pack_textures;
    use crate::world::doc::WorldJson;
    use crate::world::doc::fixtures::{checkerboard_world, one_tile_world};
    use crate::world::grid::{Ramp, WorldGrid, ramp_token};
    use crate::world::mesh::mesh;
    use crate::world::tileset::Tileset;

    const N: usize = 32;

    /// The mesher's faces of `world` on the synthetic tileset.
    fn faces_of(world: &WorldJson) -> Vec<Face> {
        let grid = WorldGrid::parse(world).expect("the world parses");
        mesh(&grid, &Tileset::synthetic())
            .into_iter()
            .map(Face::from)
            .collect()
    }

    /// `bytes` written as `data/maps/<file>` under `dir`.
    fn put(dir: &Path, file: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(freeform_rel(file));
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
        std::fs::write(&path, bytes).expect("written");
        path
    }

    /// The side file of `faces` read back on the synthetic pack.
    fn round_trip(faces: &[Face]) -> Vec<Face> {
        let pack = synthetic_pack_textures();
        let dir = tempfile::tempdir().expect("temp dir");
        let glb = side_glb(faces, &pack, FREEFORM_UNIT).expect("side file");
        let path = put(dir.path(), "0003.haus.glb", &glb);
        side_faces(&path, &pack, FREEFORM_LAYER).expect("read back")
    }

    /// A face as polygon: texture, layer, corners (position, uv) without a corner at the
    /// position of the one before it (a triangle repeats corner 2), turned to start at the
    /// smallest corner.
    type Polygon = (String, u8, Vec<([i32; 3], [i32; 2])>);

    fn polygon(f: &Face) -> Polygon {
        let mut corners: Vec<([i32; 3], [i32; 2])> = Vec::new();
        for k in 0..4 {
            if corners.last().is_none_or(|c| c.0 != f.corners[k]) {
                corners.push((f.corners[k], f.uv[k]));
            }
        }
        if corners.len() > 1 && corners.first().map(|c| c.0) == corners.last().map(|c| c.0) {
            corners.pop();
        }
        let start = (0..corners.len()).min_by_key(|&k| corners[k]).unwrap_or(0);
        corners.rotate_left(start);
        (f.texture.clone(), f.layer, corners)
    }

    /// The faces as a sorted list of polygons (a multiset), every layer set to `layer`.
    fn polygons(faces: &[Face], layer: u8) -> Vec<Polygon> {
        let mut out: Vec<Polygon> = faces
            .iter()
            .map(|f| {
                let mut p = polygon(f);
                p.1 = layer;
                p
            })
            .collect();
        out.sort();
        out
    }

    /// The JSON chunk of a GLB.
    fn json_of(glb: &[u8]) -> serde_json::Value {
        let len = u32::from_le_bytes(glb[12..16].try_into().expect("4 bytes")) as usize;
        serde_json::from_slice(&glb[20..20 + len]).expect("JSON chunk")
    }

    #[test]
    fn side_glb_round_trips_the_faces() {
        for world in [one_tile_world(), checkerboard_world()] {
            let faces = faces_of(&world);
            assert!(!faces.is_empty());
            let back = round_trip(&faces);
            assert!(back.iter().all(|f| f.layer == FREEFORM_LAYER));
            assert_eq!(back.len(), faces.len());
            assert_eq!(
                polygons(&back, FREEFORM_LAYER),
                polygons(&faces, FREEFORM_LAYER)
            );
        }
    }

    #[test]
    fn a_triangle_stays_one_triangle() {
        let triangle = Face {
            corners: [[0, 0, 0], [16, 0, 0], [16, 8, -16], [16, 8, -16]],
            uv: [[0, 0], [16, 0], [16, 16], [16, 16]],
            texture: "syn_boden".to_owned(),
            layer: 0,
        };
        let back = round_trip(std::slice::from_ref(&triangle));
        assert_eq!(back.len(), 1);
        assert!(back[0].is_triangle());
        assert_eq!(polygons(&back, 0), polygons(&[triangle], 0));
    }

    #[test]
    fn the_side_file_is_a_fixed_point() {
        let pack = synthetic_pack_textures();
        let faces = faces_of(&checkerboard_world());
        let first = side_glb(&faces, &pack, FREEFORM_UNIT).expect("side file");
        let dir = tempfile::tempdir().expect("temp dir");
        let path = put(dir.path(), "0003.haus.glb", &first);
        let back = side_faces(&path, &pack, FREEFORM_LAYER).expect("read back");
        let second = side_glb(&back, &pack, FREEFORM_UNIT).expect("side file");
        assert_eq!(second, first);
    }

    #[test]
    fn side_glb_has_no_images_and_names_materials_after_textures() {
        let pack = synthetic_pack_textures();
        let faces = faces_of(&checkerboard_world());
        let json = json_of(&side_glb(&faces, &pack, FREEFORM_UNIT).expect("side file"));
        assert!(json.get("images").is_none(), "{json}");
        assert!(json.get("textures").is_none(), "{json}");
        assert_eq!(json["asset"]["generator"], "gen4-studio");
        assert_eq!(json["asset"]["version"], "2.0");
        let names: Vec<&str> = json["materials"]
            .as_array()
            .expect("materials")
            .iter()
            .map(|m| m["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, ["syn_boden", "syn_gras"]);
        let primitives = json["meshes"][0]["primitives"]
            .as_array()
            .expect("primitives");
        assert_eq!(primitives.len(), 2);
        for p in primitives {
            let position =
                &json["accessors"][p["attributes"]["POSITION"].as_u64().unwrap() as usize];
            assert!(position.get("min").is_some() && position.get("max").is_some());
            let indices = &json["accessors"][p["indices"].as_u64().unwrap() as usize];
            assert_eq!(indices["componentType"], 5125);
        }
        assert!(json["nodes"][0].get("matrix").is_none());
        assert!(json["nodes"][0].get("translation").is_none());
    }

    #[test]
    fn side_glb_refuses_a_texture_outside_the_pack() {
        let mut faces = faces_of(&one_tile_world());
        faces[0].texture = "fremd".to_owned();
        let e = side_glb(&faces, &synthetic_pack_textures(), FREEFORM_UNIT).unwrap_err();
        assert!(
            matches!(&e, KernelError::Refused(t) if t.contains("“fremd”")),
            "{e}"
        );
    }

    #[test]
    fn a_side_file_with_an_unknown_material_is_texture_missing() {
        let pack = synthetic_pack_textures();
        let glb = side_glb(&faces_of(&one_tile_world()), &pack, FREEFORM_UNIT).expect("side");
        let dir = tempfile::tempdir().expect("temp dir");
        let path = put(dir.path(), "0003.haus.glb", &glb);
        let without: Vec<PackTexture> =
            pack.into_iter().filter(|t| t.name != "syn_boden").collect();
        let m = side_faces(&path, &without, FREEFORM_LAYER).unwrap_err();
        assert_eq!(m.code, "bake.texture-missing");
        assert_eq!(m.level, Level::Error);
        assert_eq!(m.args, ["syn_boden"]);
        assert_eq!(
            m.message,
            "The texture “syn_boden” is missing in the map texture package or has no palette there."
        );
    }

    #[test]
    fn a_missing_side_file_is_side_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(freeform_rel("0003.haus.glb"));
        let m = side_faces(&path, &synthetic_pack_textures(), FREEFORM_LAYER).unwrap_err();
        assert_eq!(m.code, SIDE_FILE_CODE);
        assert_eq!(m.level, Level::Error);
        assert_eq!(m.args, ["data/maps/0003.haus.glb"]);
        assert_eq!(
            m.message,
            "The side file data/maps/0003.haus.glb is missing."
        );
    }

    #[test]
    fn garbage_is_side_file_unreadable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = put(dir.path(), "0003.haus.glb", b"kein glTF");
        let m = side_faces(&path, &synthetic_pack_textures(), FREEFORM_LAYER).unwrap_err();
        assert_eq!(m.code, SIDE_FILE_CODE);
        assert_eq!(m.args.len(), 2);
        assert_eq!(m.args[0], "data/maps/0003.haus.glb");
        assert!(!m.args[1].is_empty());
        assert_eq!(
            m.message,
            format!(
                "The side file data/maps/0003.haus.glb cannot be read ({}).",
                m.args[1]
            )
        );
    }

    #[test]
    fn side_name_cleans_and_numbers() {
        assert_eq!(side_name("Haus Groß", &[]), "haus_gro_");
        assert_eq!(side_name("", &[]), "freiform");
        assert_eq!(side_name("a-b_9", &[]), "a-b_9");
        let taken = ["haus".to_owned(), "haus_2".to_owned()];
        assert_eq!(side_name("Haus", &taken[..1]), "haus_2");
        assert_eq!(side_name("Haus", &taken), "haus_3");
        let long = "x".repeat(40);
        assert_eq!(side_name(&long, &[]), "x".repeat(SIDE_NAME_MAX));
        // The number fits into the limit.
        let numbered = side_name(&long, &["x".repeat(SIDE_NAME_MAX)]);
        assert_eq!(numbered, format!("{}_2", "x".repeat(SIDE_NAME_MAX - 2)));
        assert_eq!(
            side_name_of(&freeform_file(3, &numbered), 3),
            Some(&numbered[..])
        );
    }

    #[test]
    fn side_name_of_accepts_only_the_own_stem() {
        assert_eq!(side_name_of("0003.haus.glb", 3), Some("haus"));
        assert_eq!(side_name_of("0003.haus.glb", 4), None);
        for bad in [
            "0003.Haus.glb",
            "0003..glb",
            "0003.haus.gltf",
            "0003.a.b.glb",
            "3.haus.glb",
            "haus.glb",
        ] {
            assert_eq!(side_name_of(bad, 3), None, "{bad}");
        }
        assert_eq!(
            side_name_of(&format!("0003.{}.glb", "a".repeat(33)), 3),
            None
        );
        assert_eq!(freeform_file(123, "haus"), "0123.haus.glb");
        assert_eq!(
            freeform_rel("0123.haus.glb"),
            Path::new("data/maps/0123.haus.glb")
        );
        assert_eq!(
            piece_rel(6, "turm"),
            Path::new("data/tilesets/0006.turm.glb")
        );
        assert_eq!(side_parts("0123.haus.glb"), Some((123, "haus")));
        assert_eq!(side_parts("123.haus.glb"), None);
    }

    fn at(x: usize, z: usize) -> usize {
        z * N + x
    }

    fn rows(cell: impl Fn(usize, usize) -> String) -> Vec<String> {
        (0..N)
            .map(|z| (0..N).map(|x| cell(x, z)).collect::<Vec<_>>().join(" "))
            .collect()
    }

    fn a_ramp() -> impl Strategy<Value = Ramp> {
        prop_oneof![
            4 => Just(Ramp::None),
            1 => (1u8..3).prop_map(Ramp::North),
            1 => (1u8..3).prop_map(Ramp::East),
            1 => (1u8..3).prop_map(Ramp::South),
            1 => (1u8..3).prop_map(Ramp::West),
        ]
    }

    /// Random grounds without pieces (as the bake's `prop_model_floor_meets_the_bdhc`): tiles
    /// `.`, `0000` to `0002`, heights −16 to 60, ramps of one or two steps.
    fn a_ground() -> impl Strategy<Value = WorldJson> {
        (
            proptest::collection::vec(
                prop_oneof![1 => Just(None), 6 => (0u16..3).prop_map(Some)],
                N * N,
            ),
            proptest::collection::vec(-16i8..=60, N * N),
            proptest::collection::vec(a_ramp(), N * N),
        )
            .prop_map(|(tiles, heights, ramps)| {
                let mut w = one_tile_world();
                let ground = &mut w.layers[0];
                ground.tiles = rows(|x, z| match tiles[at(x, z)] {
                    None => ".".to_owned(),
                    Some(t) => format!("{t:04x}"),
                });
                ground.heights = Some(rows(|x, z| heights[at(x, z)].to_string()));
                ground.ramps = Some(rows(|x, z| ramp_token(ramps[at(x, z)])));
                w
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        #[test]
        fn prop_side_glb_round_trips(w in a_ground()) {
            let faces = faces_of(&w);
            let back = round_trip(&faces);
            prop_assert_eq!(polygons(&back, FREEFORM_LAYER), polygons(&faces, FREEFORM_LAYER));
        }
    }
}
