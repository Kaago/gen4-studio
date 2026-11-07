//! „Karte als Quelle übernehmen“ (Form 1, verbatim): a binary map becomes a world map that
//! bakes byte-equal ([`world_adopt`]). The world keeps the map's model as its verbatim model
//! (`freeform = [{nsbmd}]`) and its BDHC as verbatim terrain (`terrain = {bdhc}`), both parts
//! in the form a save writes them ([`Store::world_as_written`]: `base_sha1`, a side file or
//! hex); every permission is a hand value; the ground layer „Boden“ has no tile, its heights
//! are sampled from the BDHC (a cell centre's height in steps of 8 units) and listed where they
//! do not fit ([`HeightDeviation`]). Buildings and BGS stay in the Map asset. One undo step.

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::store::{ChangeId, Store};
use crate::textproject::Part;

use super::bake::WorldBakeReport;
use super::doc::{
    FreeformJson, HEIGHT_MAX, HEIGHT_MIN, LayerJson, LayerRole, PermissionsJson, STEP_UNITS,
    TerrainJson, WORLD_VERSION, WorldJson,
};
use super::edit::{already_world, binary_map_refusal, refuse_locked_map, require_text_project};
use super::grid::{override_token, tile_token};
use super::places::packs_of_map;

/// The most a sampled cell centre may lie off its step (`8 · step` units), in world units.
pub const ADOPT_TOLERANCE: f64 = 0.1;
/// Why Form 2 is refused.
pub const ADOPT_GLTF_LATER: &str = "Form 2 (glTF) kommt mit 6.5b.";

/// The name of the ground layer of an adopted world.
const GROUND_NAME: &str = "Boden";
/// Cells per row and rows per layer.
const MAP_CELLS: usize = g4_formats::map::MAP_SIZE;
/// fx32 units per world unit.
const FX_ONE: f64 = g4_formats::bdhc::FX_ONE as f64;

/// The form of the adopted world: the map's model and BDHC verbatim (Form 1), or the model as
/// glTF (Form 2, 6.5b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdoptForm {
    #[default]
    Verbatim,
    Gltf,
}

/// `world_adopt`: the map, the map texture package of the world (`None`: the first package of
/// the headers that show the map, else 0) and the form (`verbatim` by default).
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

/// What `world_adopt` did: the map, the package of the world and whether it was guessed (0, no
/// header shows the map), the cells whose heights deviate, the undo step and the bake.
#[derive(Debug, Clone, Serialize)]
pub struct WorldAdoptReport {
    pub map: u16,
    pub tileset: u16,
    pub tileset_guessed: bool,
    pub deviations: Vec<HeightDeviation>,
    pub change: Option<ChangeId>,
    pub bake: WorldBakeReport,
}

/// „Karte als Quelle übernommen: Karte 0123“, the label of [`world_adopt`].
pub fn adopt_label(map: u16) -> String {
    format!("Karte als Quelle übernommen: Karte {map:04}")
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

/// „Karte als Quelle übernehmen“ (Form 1) of map `req.map` as one undo step
/// ([`adopt_label`]): a world on package `req.tileset` (else the first of [`packs_of_map`],
/// else 0 with `tileset_guessed`), a ground layer „Boden“ all `.` with the heights sampled
/// from the map's BDHC, no ramp, the map's model as `freeform = [{nsbmd}]` and its BDHC as
/// `terrain = {bdhc}` (each part as a save writes it), permissions derived with every cell a
/// hand value. The Map asset bakes to the same bytes.
///
/// Refused ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`super::WORLD_NEEDS_TEXT_PROJECT`]), Form 2 ([`ADOPT_GLTF_LATER`]), a map that is a world
/// map already ([`already_world`]), a locked map or one whose member does not read
/// ([`super::edit::binary_map_refusal`]: „Karte 0001 lässt sich nicht übernehmen: ihr Member
/// lässt sich nicht lesen (…).“), a package that is not there (the refusal of
/// [`Store::tileset`]).
pub fn world_adopt(
    store: &mut Store,
    req: &WorldAdoptRequest,
) -> Result<WorldAdoptReport, KernelError> {
    require_text_project(store)?;
    if req.form == AdoptForm::Gltf {
        return Err(KernelError::Refused(ADOPT_GLTF_LATER.to_owned()));
    }
    let map = req.map;
    if store.world(map)?.is_some() {
        return Err(KernelError::Refused(already_world(map)));
    }
    refuse_locked_map(store, map, |cause| {
        binary_map_refusal(map, "übernehmen", cause)
    })?;
    let (package, guessed) = match req.tileset {
        Some(package) => (package, false),
        None => match packs_of_map(store, map)?.first() {
            Some(&package) => (package, false),
            None => (0, true),
        },
    };
    store.tileset(package)?;
    let file = store.map(map)?.clone();
    let (steps, deviations) = sample_heights(&file.bdhc);
    // The parts are placeholders here; the writer gives them their form.
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
        freeform: vec![FreeformJson {
            glb: None,
            nsbmd: Some(placeholder()),
            at: [0, 0, 0],
        }],
        terrain: Some(TerrainJson {
            bdhc: placeholder(),
        }),
        permissions: PermissionsJson {
            derive: true,
            overrides: Some(rows(|i| override_token(file.permissions.get(i).copied()))),
        },
    };
    let world = store.world_as_written(map, &file, &world)?;
    let (bake, change) = store.transaction(&adopt_label(map), |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    let bake = bake.ok_or_else(|| {
        KernelError::InvalidProject(format!("Karte {map:04}: das Übernehmen backte nicht."))
    })?;
    Ok(WorldAdoptReport {
        map,
        tileset: package,
        tileset_guessed: guessed,
        deviations,
        change,
        bake,
    })
}

#[cfg(test)]
mod tests;
