//! „Karte als Quelle übernehmen“: a binary map becomes a world map, in one of two forms.
//!
//! Form 1 (verbatim, [`world_adopt`]) bakes byte-equal: the world keeps the map's model as its
//! verbatim model (`freeform = [{nsbmd}]`) and its BDHC as verbatim terrain (`terrain = {bdhc}`),
//! both parts in the form a save writes them ([`Store::world_as_written`]: `base_sha1`, a side
//! file or hex); every permission is a hand value; the ground layer „Boden“ has no tile, its
//! heights are sampled from the BDHC (a cell centre's height in steps of 8 units) and listed
//! where they do not fit ([`HeightDeviation`]). Buildings and BGS stay in the Map asset.
//!
//! Form 2 (glTF, [`world_adopt_with`], 6.5b) is the same world with the map's model as the
//! free-form side file `NNNN.model.glb` (`freeform = [{glb, at: [0, 0, 0]}]`): apicula converts
//! the model to glTF, its materials bind the package's textures by the rule of
//! [`bind_vanilla_materials`], and the faces become the canonical side file; the model is baked
//! anew, terrain and permissions stay as they were. Either form is one undo step.

use std::path::Path;

use g4_formats::nsbmd::Nsbmd;
use serde::{Deserialize, Serialize};

use crate::bake::Level;
use crate::error::KernelError;
use crate::export::nsbmd_to_glb;
use crate::map_import::PartCode;
use crate::model_import::mesh::{Binding, Mesh, to_faces};
use crate::model_import::read_model;
use crate::project::AssetKind;
use crate::store::{ChangeId, Store};
use crate::textproject::Part;

use super::bake::{WorldBakeReport, bake_pack};
use super::doc::{
    FreeformJson, HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, PermissionsJson, STEP_UNITS,
    TerrainJson, WORLD_VERSION, WorldJson,
};
use super::edit::{already_world, binary_map_refusal, refuse_locked_map, require_text_project};
use super::freeform::{
    APICULA_UNIT, FREEFORM_UNIT, bind_vanilla_materials, file_step_at, freeform_file, freeform_rel,
    in_one_step, side_glb, world_asset,
};
use super::grid::{override_token, tile_token};
use super::places::packs_of_map;

/// The most a sampled cell centre may lie off its step (`8 · step` units), in world units.
pub const ADOPT_TOLERANCE: f64 = 0.1;
/// Why Form 2 is refused without apicula.
pub fn adopt_needs_apicula() -> String {
    t!("world.adopt_needs_apicula").into_owned()
}
/// A material of Form 2 that binds no texture of the package.
pub const ADOPT_TEXTURE_CODE: &str = "world.adopt-texture";
/// The `<name>` of the side file of Form 2: `NNNN.model.glb`.
pub const FORM2_SIDE_NAME: &str = "model";

/// The name of the ground layer of an adopted world: content of the map's JSON (and of a TMJ
/// export), not a message, so it is the same in every locale.
const GROUND_NAME: &str = "Boden";
/// Cells per row and rows per layer.
const MAP_CELLS: usize = g4_formats::map::MAP_SIZE;
/// fx32 units per world unit.
const FX_ONE: f64 = g4_formats::bdhc::FX_ONE as f64;

/// The form of the adopted world: the map's model and BDHC verbatim (Form 1), or the model as
/// a glTF side file (Form 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdoptForm {
    #[default]
    Verbatim,
    Gltf,
}

/// `world_adopt`: the map, the map texture package of the world (`None`: the first package of
/// the headers that show the map, else 0; Form 2 first looks for the package that holds the
/// model's textures, [`adopt_package`]) and the form (`verbatim` by default).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldAdoptRequest {
    pub map: u16,
    #[serde(default)]
    pub tileset: Option<u16>,
    #[serde(default)]
    pub form: AdoptForm,
}

/// A cell whose sampled height is no whole step: its centre's height in world units (`None`:
/// no plate under it) and the step the world took (`round(y / 8)`, ties to even, −16 to 62;
/// 0 without plate).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeightDeviation {
    pub x: u8,
    pub z: u8,
    pub height: Option<f64>,
    pub step: i8,
}

/// The vanilla model of a map adopted in Form 2: its polygons, quads and triangles as its MDL0
/// shapes draw them ([`g4_formats::nsbmd::Model::geometry`] of the first model; 0 when it does
/// not read) and its size in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct VanillaGeometry {
    pub polygons: u32,
    pub quads: u32,
    pub triangles: u32,
    pub bytes: usize,
}

/// The model baked from the side file of Form 2: the faces of the bake, the quads and
/// triangles the conversion gave the baker, the quad strips and single quads the model sends,
/// the polygons the conversion dropped (no area once rounded, or untextured), the model's size
/// in bytes and the polygons of untextured materials left out (vertex-colour materials without
/// a texture, [`bind_vanilla_materials`]; counted in `dropped` too).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BakedGeometry {
    pub faces: usize,
    pub quads: usize,
    pub triangles: usize,
    pub strips: usize,
    pub single_quads: usize,
    pub dropped: usize,
    pub bytes: usize,
    pub untextured: usize,
}

/// What Form 2 made of the map's model: both polygon counts (they differ: the baker draws
/// quad strips where the vanilla model has its own lists), the parts of the glTF the reader
/// left out (`model.parts-ignored`: `skins`, `vertex-colours` …, then `untextured:<material>`
/// per untextured material whose polygons were left out) and the side file's name.
#[derive(Debug, Clone, Serialize)]
pub struct Form2Report {
    pub vanilla: VanillaGeometry,
    pub baked: BakedGeometry,
    pub ignored: Vec<String>,
    pub glb: String,
}

/// What `world_adopt` did: the map, the package of the world and whether it was guessed (0, no
/// header shows the map), the cells whose heights deviate, the undo step, the bake and, for
/// Form 2, the figures of the model.
#[derive(Debug, Clone, Serialize)]
pub struct WorldAdoptReport {
    pub map: u16,
    pub tileset: u16,
    pub tileset_guessed: bool,
    pub deviations: Vec<HeightDeviation>,
    pub change: Option<ChangeId>,
    pub bake: WorldBakeReport,
    pub form2: Option<Form2Report>,
}

/// „Map adopted as source: Map 0123“, the label of [`world_adopt`].
pub fn adopt_label(map: u16) -> String {
    t!("world.adopt_label", map = map : {:04}).into_owned()
}

/// The step of a cell centre at `height` world units: `round(y / 8)` (ties to even), within
/// −16 to 62.
fn step_of(height: f64) -> i8 {
    let step = (height / f64::from(STEP_UNITS)).round_ties_even();
    step.clamp(f64::from(HEIGHT_MIN), f64::from(HEIGHT_MAX)) as i8
}

/// The steps of the 1024 cells sampled from `bdhc` and the cells that deviate. A BDHC that does
/// not read has no plate anywhere.
fn sample_heights(bdhc: &[u8]) -> (Vec<i8>, Vec<HeightDeviation>) {
    let centres = crate::terrain::map_heights(bdhc).unwrap_or([None; MAP_CELLS * MAP_CELLS]);
    let mut steps = Vec::with_capacity(centres.len());
    let mut deviations = Vec::new();
    for (i, centre) in centres.iter().enumerate() {
        let height = centre.map(|fx| f64::from(fx) / FX_ONE);
        let step = height.map_or(0, step_of);
        let off = height
            .is_none_or(|y| (y - f64::from(STEP_UNITS) * f64::from(step)).abs() > ADOPT_TOLERANCE);
        if off {
            deviations.push(HeightDeviation {
                x: (i % MAP_CELLS) as u8,
                z: (i / MAP_CELLS) as u8,
                height,
                step,
            });
        }
        steps.push(step);
    }
    (steps, deviations)
}

/// 32 rows of the 1024 tokens `token(i)`.
fn rows(token: impl Fn(usize) -> String) -> Vec<String> {
    (0..MAP_CELLS)
        .map(|z| {
            (0..MAP_CELLS)
                .map(|x| token(z * MAP_CELLS + x))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// The map to adopt, checked, and the world both forms share.
struct Source {
    map: u16,
    package: u16,
    guessed: bool,
    file: g4_formats::map::MapFile,
    deviations: Vec<HeightDeviation>,
    world: WorldJson,
}

/// The checks both forms make (the map is no world map yet, its member is not locked and
/// reads, the package is there) and the adopted world with `freeform` as its model: the ground
/// layer „Boden“ all `.` with the heights sampled from the map's BDHC, no ramp, the BDHC as
/// `terrain = {bdhc}`, permissions derived with every cell a hand value. The package is
/// `req.tileset`, else [`adopt_package`] (by fit when `by_fit`, Form 2).
fn source(
    store: &mut Store,
    req: &WorldAdoptRequest,
    freeform: FreeformJson,
    by_fit: bool,
) -> Result<Source, KernelError> {
    let map = req.map;
    if store.world(map)?.is_some() {
        return Err(KernelError::Refused(already_world(map)));
    }
    refuse_locked_map(store, map, |cause| binary_map_refusal(map, true, cause))?;
    let file = store.map(map)?.clone();
    let (package, guessed) = match req.tileset {
        Some(package) => (package, false),
        None => adopt_package(store, map, by_fit.then_some(file.model.as_slice()))?,
    };
    store.tileset(package)?;
    let (steps, deviations) = sample_heights(&file.bdhc);
    // The BDHC part (and the model part of Form 1) is a placeholder here; the writer gives it
    // its form.
    let placeholder = || Part::Hex { hex: String::new() };
    let world = WorldJson {
        version: WORLD_VERSION,
        tileset: package,
        layers: vec![LayerJson {
            name: GROUND_NAME.to_owned(),
            role: LayerRole::Ground,
            tiles: rows(|_| tile_token(None)),
            heights: Some(rows(|i| steps[i].to_string())),
            ramps: None,
        }],
        pieces: Vec::new(),
        freeform: vec![freeform],
        terrain: Some(TerrainJson {
            bdhc: placeholder(),
        }),
        permissions: PermissionsJson {
            derive: true,
            overrides: Some(rows(|i| override_token(file.permissions.get(i).copied()))),
        },
    };
    let world = store.world_as_written(map, &file, &world)?;
    Ok(Source {
        map,
        package,
        guessed,
        file,
        deviations,
        world,
    })
}

/// The package of map `map` adopted without one asked for, and whether it was guessed.
///
/// With `model` (Form 2) and textures paired in its MDL0 ([`model_textures`]): the first
/// package of [`packs_of_map`] whose textures as the world bake binds them
/// ([`super::bake::world_pack_textures`]) include every one of them, else the first such
/// package of the archive (ascending, guessed). Otherwise, and without `model` (Form 1, whose
/// choice stays as it was): the first package of [`packs_of_map`], else 0 (guessed).
fn adopt_package(
    store: &mut Store,
    map: u16,
    model: Option<&[u8]>,
) -> Result<(u16, bool), KernelError> {
    let shown = packs_of_map(store, map)?;
    let wanted = model.map(model_textures).unwrap_or_default();
    if !wanted.is_empty() {
        if let Some(&package) = shown.iter().find(|&&p| holds(store, p, &wanted)) {
            return Ok((package, false));
        }
        let count = store.member_count(AssetKind::MapTexture)?;
        let archive = (0..count).filter_map(|p| u16::try_from(p).ok());
        for package in archive {
            if holds(store, package, &wanted) {
                return Ok((package, true));
            }
        }
    }
    Ok(match shown.first() {
        Some(&package) => (package, false),
        None => (0, true),
    })
}

/// The textures the first model of `model` pairs with a material (MDL0), in pairing order;
/// none when it does not read.
fn model_textures(model: &[u8]) -> Vec<String> {
    let Ok(file) = Nsbmd::read(model) else {
        return Vec::new();
    };
    file.models
        .first()
        .map(|(_, m)| {
            m.textures
                .iter()
                .filter(|p| !p.materials.is_empty())
                .map(|p| g4_formats::nitro_dict::name_text(&p.name))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether package `package` holds every texture of `wanted` as the world bake binds them; a
/// package that is missing or does not read holds none.
fn holds(store: &mut Store, package: u16, wanted: &[String]) -> bool {
    let Ok(tileset) = store.tileset(package) else {
        return false;
    };
    match bake_pack(store, package, &tileset, None) {
        Ok((_, textures)) => wanted.iter().all(|w| textures.iter().any(|t| &t.name == w)),
        Err(_) => false,
    }
}

/// The transaction that makes `world` map `map`'s world, labelled `label`: its bake report and
/// the change.
fn set_world(
    store: &mut Store,
    map: u16,
    label: &str,
    world: WorldJson,
) -> Result<(WorldBakeReport, Option<ChangeId>), KernelError> {
    let (bake, change) = store.transaction(label, |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    let bake = bake.ok_or_else(|| {
        KernelError::InvalidProject(t!("world.adopt_baked_nothing", map = map : {:04}).into_owned())
    })?;
    Ok((bake, change))
}

/// „Karte als Quelle übernehmen“ of map `req.map` as one undo step ([`adopt_label`]), in the
/// form `req.form`; Form 2 needs `apicula`.
///
/// Form 1 (verbatim): a world on package `req.tileset` (else the first of [`packs_of_map`],
/// else 0 with `tileset_guessed`; Form 2 by fit first, [`adopt_package`]), a ground layer
/// „Boden“ all `.` with the heights sampled from the map's BDHC, no ramp, the map's model as
/// `freeform = [{nsbmd}]` and its BDHC as `terrain = {bdhc}` (each part as a save writes it),
/// permissions derived with every cell a hand value. The Map asset bakes to the same bytes.
///
/// Form 2 (glTF): apicula converts the map's model with the package's textures into
/// `<root>/.g4/tmp/adopt-NNNN` (failing that, without them), then [`world_adopt_glb`]; the
/// folder is removed afterwards, whatever the outcome.
///
/// Refused ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`super::world_needs_text_project`]), Form 2 without apicula ([`adopt_needs_apicula`]), a
/// map that is already a world map ([`already_world`]), a locked map or one whose member does
/// not read ([`super::edit::binary_map_refusal`]: „Karte 0001 lässt sich nicht übernehmen: ihr
/// Member lässt sich nicht lesen (…).“), a package that is not there (the refusal of
/// [`Store::tileset`]), and what [`world_adopt_glb`] refuses. apicula's own failure is its
/// error ([`KernelError::ToolMissing`], `ToolTimeout` or `Build`).
pub fn world_adopt_with(
    store: &mut Store,
    req: &WorldAdoptRequest,
    apicula: Option<&Path>,
) -> Result<WorldAdoptReport, KernelError> {
    require_text_project(store)?;
    if req.form == AdoptForm::Gltf {
        let Some(apicula) = apicula else {
            return Err(KernelError::Refused(adopt_needs_apicula()));
        };
        return adopt_through_apicula(store, req, apicula);
    }
    let placeholder = || Part::Hex { hex: String::new() };
    let source = source(
        store,
        req,
        FreeformJson {
            glb: None,
            nsbmd: Some(placeholder()),
            at: [0, 0, 0],
        },
        false,
    )?;
    let map = source.map;
    let (bake, change) = set_world(store, map, &adopt_label(map), source.world)?;
    Ok(WorldAdoptReport {
        map,
        tileset: source.package,
        tileset_guessed: source.guessed,
        deviations: source.deviations,
        change,
        bake,
        form2: None,
    })
}

/// [`world_adopt_with`] without apicula: Form 1, or the refusal [`adopt_needs_apicula`].
pub fn world_adopt(
    store: &mut Store,
    req: &WorldAdoptRequest,
) -> Result<WorldAdoptReport, KernelError> {
    world_adopt_with(store, req, None)
}

/// Form 2 through `apicula`: the checks first (so that a refused map runs no tool), the map's
/// model converted into `<root>/.g4/tmp/adopt-NNNN` (with the package, failing that without),
/// [`world_adopt_glb`], the folder removed.
fn adopt_through_apicula(
    store: &mut Store,
    req: &WorldAdoptRequest,
    apicula: &Path,
) -> Result<WorldAdoptReport, KernelError> {
    let checked = source(store, req, form2_entry(req.map), true)?;
    let model = checked.file.model.clone();
    let pack = store.project().map_tileset_member(checked.package)?;
    let dir = store
        .project()
        .root
        .join(".g4")
        .join("tmp")
        .join(format!("adopt-{:04}", req.map));
    let glb = match pack.as_deref() {
        Some(pack) => nsbmd_to_glb(apicula, &model, Some(pack), &dir)
            .or_else(|_| nsbmd_to_glb(apicula, &model, None, &dir)),
        None => nsbmd_to_glb(apicula, &model, None, &dir),
    };
    let result = glb.and_then(|glb| world_adopt_glb(store, req, &glb));
    // A folder left behind holds nothing anyone reads; the outcome stands either way.
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The free-form entry of Form 2: the side file `NNNN.model.glb` at the map's origin.
fn form2_entry(map: u16) -> FreeformJson {
    FreeformJson {
        glb: Some(freeform_file(map, FORM2_SIDE_NAME)),
        nsbmd: None,
        at: [0, 0, 0],
    }
}

/// `sentence (code)`: the text of a Form 2 refusal.
fn form2_refusal(sentence: &str, code: &str) -> KernelError {
    KernelError::Refused(format!("{sentence} ({code})"))
}

/// Form 2 of map `req.map` from `glb_path`, a GLB apicula wrote of the map's model (positions
/// in world units, [`APICULA_UNIT`]), as one undo step ([`adopt_label`]): the world of Form 1
/// with `freeform = [{glb: "NNNN.model.glb", at: [0, 0, 0]}]`; the side file is the canonical GLB
/// ([`side_glb`]) of the faces converted with the bindings of [`bind_vanilla_materials`]
/// against the package's textures as the world bake binds them, the polygons of untextured
/// materials left out ([`BakedGeometry::untextured`]); the step writes the side file, then sets
/// the world ([`in_one_step`]). The report carries [`Form2Report`].
///
/// Refused (`„{sentence} ({code})“`, no step, no side file, the map stays binary): the GLB does
/// not read or converts to nothing (its `model.*` code; also when only untextured polygons were
/// there), a material that names a texture (MDL0 or image) binds no texture of the package
/// ([`ADOPT_TEXTURE_CODE`]: „Material „lake_lm2“ der Karte 0123 bindet keine Textur des
/// Pakets 0006; Form 2 geht für diese Karte nicht, Form 1 schon.“), the bake reports an error
/// (the first one, e.g. `bake.model-too-big`: the step is rolled back). Besides what Form 1
/// refuses.
pub fn world_adopt_glb(
    store: &mut Store,
    req: &WorldAdoptRequest,
    glb_path: &Path,
) -> Result<WorldAdoptReport, KernelError> {
    require_text_project(store)?;
    let map = req.map;
    let source = source(store, req, form2_entry(map), true)?;
    let package = source.package;
    let (mut mesh, notes) =
        read_model(glb_path).map_err(|m| form2_refusal(&m.message, m.code.as_str()))?;
    let tileset = store.tileset(package)?;
    let (_, textures) = bake_pack(store, package, &tileset, None)?;
    let bindings = bind_vanilla_materials(&mesh, &source.file.model, &textures).map_err(|m| {
        form2_refusal(
            &t!(
                "world.adopt_material_unbound",
                material = m,
                map = map : {:04},
                package = package : {:04}
            ),
            ADOPT_TEXTURE_CODE,
        )
    })?;
    let (untextured, untextured_names) = drop_untextured(&mut mesh, &bindings);
    let converted = to_faces(&mesh, APICULA_UNIT, &bindings)
        .map_err(|m| form2_refusal(&m.message, m.code.as_str()))?;
    let glb_name = freeform_file(map, FORM2_SIDE_NAME);
    let glb = side_glb(&converted.faces, &textures, FREEFORM_UNIT)?;
    let label = adopt_label(map);
    let step = file_step_at(store, world_asset(map), &freeform_rel(&glb_name), Some(glb))?;
    let world = source.world;
    let (bake, change) = in_one_step(store, &label, None, step, |store| {
        let (bake, change) = set_world(store, map, &label, world)?;
        match bake.messages.iter().find(|m| m.level == Level::Error) {
            Some(m) => Err(form2_refusal(&m.message, &m.code)),
            None => Ok((bake, change)),
        }
    })?;
    let vanilla = Nsbmd::read(&source.file.model)
        .ok()
        .and_then(|file| file.models.first().map(|(_, m)| m.geometry()))
        .unwrap_or_default();
    let ignored = notes
        .iter()
        .filter(|n| n.code == PartCode::ModelPartsIgnored)
        .flat_map(|n| n.args.first().map(|a| a.split(',').map(str::to_owned)))
        .flatten()
        .chain(untextured_names)
        .collect();
    let form2 = Form2Report {
        vanilla: VanillaGeometry {
            polygons: vanilla.polys,
            quads: vanilla.quads,
            triangles: vanilla.tris,
            bytes: source.file.model.len(),
        },
        baked: BakedGeometry {
            faces: bake.faces,
            quads: converted.stats.quads,
            triangles: converted.stats.triangles,
            strips: bake.strips,
            single_quads: bake.single_quads,
            dropped: converted.stats.dropped + untextured,
            bytes: bake.model_bytes,
            untextured,
        },
        ignored,
        glb: glb_name,
    };
    Ok(WorldAdoptReport {
        map,
        tileset: package,
        tileset_guessed: source.guessed,
        deviations: source.deviations,
        change,
        bake,
        form2: Some(form2),
    })
}

/// Leaves out the polygons of `mesh` whose material is untextured (its binding `None`): how many,
/// and `untextured:<name>` per material that had any, in material order. A polygon without a
/// material, or with one outside `bindings`, stays (`to_faces` refuses it).
fn drop_untextured(mesh: &mut Mesh, bindings: &[Option<Binding>]) -> (usize, Vec<String>) {
    let mut left_out = vec![false; mesh.materials.len()];
    let before = mesh.polygons.len();
    mesh.polygons.retain(|p| {
        let untextured = p
            .material
            .filter(|&m| matches!(bindings.get(m), Some(None)))
            .and_then(|m| left_out.get_mut(m));
        match untextured {
            Some(flag) => {
                *flag = true;
                false
            }
            None => true,
        }
    });
    let names = mesh
        .materials
        .iter()
        .zip(left_out)
        .filter(|(_, out)| *out)
        .map(|(m, _)| format!("untextured:{}", m.name))
        .collect();
    (before - mesh.polygons.len(), names)
}

#[cfg(all(test, feature = "test-support"))]
mod form2_tests;
#[cfg(test)]
mod tests;
