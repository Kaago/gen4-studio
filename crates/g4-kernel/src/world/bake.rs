//! The pure baker of a world (phase 6.3b): a [`WorldGrid`] with its tileset and the bake input of
//! its area → the three derived parts of the map member, model (the mesher's faces through
//! [`crate::bake::bake`]), terrain (6.2's generator [`bdhc::from_grid`]) and permissions (the
//! top tile of each cell, then the edge rule, then the hand values).
//!
//! [`bake_grid`] gives the parts and the messages, [`assemble`] puts them into a map file whose
//! buildings, tail and BGS stay, [`report`] sums the result up. A verbatim part (Form 1 of
//! „Karte als Quelle übernehmen“, [`Verbatim`]) passes through byte-equal; a refused model keeps
//! the map's last model. Nothing here touches the store.

use g4_formats::bdhc::{self, Bdhc};
use g4_formats::game::Family;
use g4_formats::map::{Cell, MAP_SIZE, MapFile};
use g4_formats::nsbmd_header::material_texture_names;
use serde::Serialize;

use super::doc::WorldJson;
use super::grid::WorldGrid;
use super::mesh::{MODEL_BUDGET, MeshFace, footprint_of, mesh};
use super::tileset::{TileJson, Tileset};
use crate::bake::{
    AreaStyle, BakeCode, BakeInput, BakeMessage, BakeReport, Face, Level, PackTexture, bake,
};
use crate::terrain::block_edges;

/// The revision of the world bake: raised whenever the same world bakes to other bytes (the
/// build step `world` salts its input hash with it).
pub const WORLD_BAKE_REV: u32 = 1;
/// The most the model floor may lie off the BDHC at a cell centre, in world units.
pub const FLOOR_TOLERANCE: f64 = 0.1;
/// The permission of a cell without a tile in any layer: blocked.
pub const EMPTY_CELL: Cell = Cell {
    kind: 0,
    collision: 0x80,
};

/// A painted map outside every matrix cell with a header.
pub const NO_AREA_CODE: &str = "world.no-area";
/// A verbatim model together with painted tiles.
pub const VERBATIM_MIXED_CODE: &str = "world.verbatim-mixed";
/// A glTF free-form model.
pub const FREEFORM_LATER_CODE: &str = "world.freeform-later";

/// The blocking bit of the collision byte.
const BLOCKED: u8 = 0x80;
/// Cells of a map.
const CELLS: usize = MAP_SIZE * MAP_SIZE;

/// The verbatim parts of a world (Form 1): while the world names them, the model and the BDHC
/// of the map are these bytes, not the bake's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verbatim {
    pub model: Option<Vec<u8>>,
    pub bdhc: Option<Vec<u8>>,
}

impl Verbatim {
    /// The verbatim parts `world` names, taken from `map` (store side: the Map asset holds them):
    /// the model when `freeform` has an `nsbmd` entry. The terrain follows with `world.terrain`
    /// (schema 2, Task 3); until then a world has no verbatim terrain.
    pub fn of(world: &WorldJson, map: &MapFile) -> Verbatim {
        Verbatim {
            model: world
                .freeform
                .iter()
                .any(|f| f.nsbmd.is_some())
                .then(|| map.model.clone()),
            bdhc: None,
        }
    }
}

/// What the pure bake needs: the map (for the messages), the tileset, the bake input of the
/// map's area (`None` without area) and the family.
pub struct BakeEnv<'a> {
    pub map: u16,
    pub tileset: &'a Tileset,
    pub input: Option<BakeInput<'a>>,
    pub family: Family,
}

/// A finding of the world bake: its code (`world.no-area` or the baker's `bake.*`), level, the
/// args the UI words it from and the message (German for the world's own codes, the baker's
/// text for its codes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorldBakeMessage {
    pub code: String,
    pub level: Level,
    pub args: Vec<String>,
    pub message: String,
}

impl WorldBakeMessage {
    fn error(code: &str, args: Vec<String>, message: String) -> WorldBakeMessage {
        WorldBakeMessage {
            code: code.to_owned(),
            level: Level::Error,
            args,
            message,
        }
    }
}

impl From<&BakeMessage> for WorldBakeMessage {
    fn from(m: &BakeMessage) -> WorldBakeMessage {
        WorldBakeMessage {
            code: m.code.as_str().to_owned(),
            level: m.level,
            args: m.args.clone(),
            message: m.message.clone(),
        }
    }
}

/// The parts of a bake: `model` is `None` when it is refused (a message says why) or verbatim.
#[derive(Debug, Clone)]
pub struct GridBake {
    pub faces: Vec<MeshFace>,
    pub model: Option<BakeReport>,
    pub bdhc: Vec<u8>,
    pub permissions: Vec<Cell>,
    pub messages: Vec<WorldBakeMessage>,
    /// The model is the verbatim one (Form 1).
    pub verbatim_model: bool,
    /// The BDHC is the verbatim one (Form 1).
    pub verbatim_terrain: bool,
}

/// The sum of a bake for the UI and the checks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldBakeReport {
    pub map: u16,
    /// No message with level error.
    pub ok: bool,
    pub style: Option<AreaStyle>,
    /// Faces of the mesher.
    pub faces: usize,
    /// Bytes of the map's model (the kept one when the bake was refused).
    pub model_bytes: usize,
    /// `model_bytes / 0xF000`.
    pub model_share: f32,
    /// The map kept its last model: the bake has none.
    pub model_kept: bool,
    /// The textures the map's model binds, in material order.
    pub textures: Vec<String>,
    /// VRAM of the area as the baker judged it (0 without a baked model).
    pub vram_bytes: usize,
    pub bdhc_bytes: usize,
    pub bdhc_payload: usize,
    pub plates: usize,
    /// Cells whose collision blocks.
    pub blocked: usize,
    pub verbatim_model: bool,
    pub verbatim_terrain: bool,
    pub messages: Vec<WorldBakeMessage>,
}

/// The derived permissions of `grid` (`derive: true`, before the hand values): per cell the
/// tile of the top layer that has one (`kind`, `collision`, HeartGold/SoulSilver `| sound`),
/// [`EMPTY_CELL`] without; the footprint of a piece sets the piece's tile; then the edge rule
/// blocks the upper cell of every high edge ([`block_edges`]).
pub fn world_permissions(grid: &WorldGrid, tileset: &Tileset, family: Family) -> Vec<Cell> {
    let cell_of = |t: &TileJson| Cell {
        kind: t.kind,
        collision: if family == Family::Hgss {
            t.collision | t.sound
        } else {
            t.collision
        },
    };
    let mut cells: Vec<Cell> = (0..CELLS)
        .map(|i| {
            grid.layers
                .iter()
                .rev()
                .find_map(|l| {
                    l.tiles
                        .get(i)
                        .copied()
                        .flatten()
                        .and_then(|t| tileset.tile(t.tile))
                })
                .map_or(EMPTY_CELL, cell_of)
        })
        .collect();
    for piece in &grid.pieces {
        let (Some(tile), Some((w, d, _))) = (
            tileset.tile(piece.tile),
            footprint_of(tileset, piece.tile, piece.rotate),
        ) else {
            continue;
        };
        let [x0, z0] = piece.at.map(usize::from);
        for z in z0..(z0 + d).min(MAP_SIZE) {
            for x in x0..(x0 + w).min(MAP_SIZE) {
                cells[z * MAP_SIZE + x] = cell_of(tile);
            }
        }
    }
    block_edges(&grid.height_grid(), &mut cells);
    cells
}

/// Bakes `grid` (the grid of `world`): model, BDHC and permissions with the messages in a fixed
/// order: a glTF free-form model (`world.freeform-later`); with a verbatim model, painted faces
/// (`world.verbatim-mixed`); without one no face (`bake.empty`), no area (`world.no-area`), else
/// the baker's refusal and its VRAM warning. The model is `None` on every refusal and when it is
/// verbatim; BDHC and permissions are always there.
pub fn bake_grid(
    grid: &WorldGrid,
    world: &WorldJson,
    verbatim: &Verbatim,
    env: &BakeEnv,
) -> GridBake {
    let faces = mesh(grid, env.tileset);
    let mut messages = Vec::new();
    let gltf = world.freeform.iter().any(|f| f.glb.is_some());
    if gltf {
        messages.push(WorldBakeMessage::error(
            FREEFORM_LATER_CODE,
            Vec::new(),
            "glTF-Freiform backt 6.5b.".to_owned(),
        ));
    }
    let mut model = None;
    if verbatim.model.is_some() {
        if !faces.is_empty() {
            messages.push(WorldBakeMessage::error(
                VERBATIM_MIXED_CODE,
                Vec::new(),
                "Ein Verbatim-Modell (Form 1) und gemalte Kacheln zusammen backt erst 6.5b; \
                 Kacheln leeren oder „Als binäre Karte übernehmen“."
                    .to_owned(),
            ));
        }
    } else if faces.is_empty() {
        messages.push(WorldBakeMessage::error(
            BakeCode::Empty.as_str(),
            Vec::new(),
            "Die Welt hat keine Fläche zum Backen.".to_owned(),
        ));
    } else if let Some(input) = &env.input {
        if !gltf {
            model = bake_model(&faces, input, &mut messages);
        }
    } else {
        let map = env.map;
        messages.push(WorldBakeMessage::error(
            NO_AREA_CODE,
            vec![map.to_string()],
            format!(
                "Karte {map:04} liegt in keiner Matrixzelle mit Header; Backen braucht Licht und \
                 VRAM ihres Gebiets."
            ),
        ));
    }
    let bdhc = verbatim
        .bdhc
        .clone()
        .unwrap_or_else(|| bdhc::from_grid(&grid.height_grid()).write());
    let permissions = if grid.derive {
        let mut cells = world_permissions(grid, env.tileset, env.family);
        for (cell, hand) in cells.iter_mut().zip(&grid.overrides) {
            if let Some(hand) = hand {
                *cell = *hand;
            }
        }
        cells
    } else {
        // Without derivation the schema asks for every cell.
        grid.overrides
            .iter()
            .map(|hand| hand.unwrap_or(EMPTY_CELL))
            .collect()
    };
    GridBake {
        faces,
        model,
        bdhc,
        permissions,
        messages,
        verbatim_model: verbatim.model.is_some(),
        verbatim_terrain: verbatim.bdhc.is_some(),
    }
}

/// The baked model of `faces`, `None` when the baker refuses (its first error becomes a
/// message); its VRAM warning is kept either way.
fn bake_model(
    faces: &[MeshFace],
    input: &BakeInput,
    messages: &mut Vec<WorldBakeMessage>,
) -> Option<BakeReport> {
    let faces: Vec<Face> = faces.iter().cloned().map(Face::from).collect();
    let report = bake(&faces, input);
    let refusal = report.refusal().map(WorldBakeMessage::from);
    let refused = refusal.is_some();
    messages.extend(refusal);
    messages.extend(
        report
            .messages
            .iter()
            .filter(|m| m.code == BakeCode::VramHigh)
            .map(WorldBakeMessage::from),
    );
    (!refused).then_some(report)
}

/// `base` (buildings, tail, BGS kept) with the bake's permissions, BDHC and model; the model stays `base.model`
/// when the bake has none (refused): the bool says so.
pub fn assemble(base: &MapFile, bake: &GridBake, verbatim: &Verbatim) -> (MapFile, bool) {
    let (model, kept) = match (&verbatim.model, &bake.model) {
        (Some(bytes), _) => (bytes.clone(), false),
        (None, Some(report)) => (report.nsbmd.clone(), false),
        (None, None) => (base.model.clone(), true),
    };
    let file = MapFile {
        bgs: base.bgs.clone(),
        permissions: bake.permissions.clone(),
        buildings: base.buildings.clone(),
        buildings_tail: base.buildings_tail.clone(),
        model,
        bdhc: bake.bdhc.clone(),
    };
    (file, kept)
}

/// The report of the bake of map `map` that gave `file`.
pub fn report(
    map: u16,
    bake: &GridBake,
    file: &MapFile,
    model_kept: bool,
    style: Option<AreaStyle>,
) -> WorldBakeReport {
    let terrain = Bdhc::read(&file.bdhc).ok();
    WorldBakeReport {
        map,
        ok: !bake.messages.iter().any(|m| m.level == Level::Error),
        style,
        faces: bake.faces.len(),
        model_bytes: file.model.len(),
        model_share: file.model.len() as f32 / MODEL_BUDGET as f32,
        model_kept,
        textures: material_texture_names(&file.model).unwrap_or_default(),
        vram_bytes: bake.model.as_ref().map_or(0, |m| m.vram_bytes),
        bdhc_bytes: file.bdhc.len(),
        bdhc_payload: terrain.as_ref().map_or(0, Bdhc::payload_len),
        plates: terrain.as_ref().map_or(0, |t| t.plates.len()),
        blocked: file
            .permissions
            .iter()
            .filter(|c| c.collision & BLOCKED != 0)
            .count(),
        verbatim_model: bake.verbatim_model,
        verbatim_terrain: bake.verbatim_terrain,
        messages: bake.messages.clone(),
    }
}

#[cfg(any(test, feature = "test-support"))]
/// Pack textures for `Tileset::synthetic()`: each name with palette `<name>_pl`, sizes of the tiles.
pub fn synthetic_pack_textures() -> Vec<PackTexture> {
    Tileset::synthetic()
        .tiles
        .iter()
        .map(|t| PackTexture {
            name: t.texture.clone(),
            palette: format!("{}_pl", t.texture),
            width: t.size[0],
            height: t.size[1],
        })
        .collect()
}

#[cfg(test)]
mod tests;
