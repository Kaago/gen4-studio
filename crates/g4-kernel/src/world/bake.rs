//! The pure baker of a world (phase 6.3b): a [`WorldGrid`] with its tileset and the bake input of
//! its area → the three derived parts of the map member, model (the mesher's faces through
//! [`crate::bake::bake`]), terrain (6.2's generator [`bdhc::from_grid`]) and permissions (the
//! top tile of each cell, then the edge rule, then the hand values).
//!
//! [`bake_grid`] gives the parts and the messages, [`assemble`] puts them into a map file whose
//! buildings, tail and BGS stay, [`report`] sums the result up. A verbatim part (Form 1 of
//! „Karte als Quelle übernehmen“, [`Verbatim`]) passes through byte-equal; a refused model keeps
//! the map's last model. The faces are the grid's with the geometry of the piece models in place
//! of their footprints, then the free-form models (6.5b), both from the side files the store
//! loaded ([`SideModels`] in [`BakeEnv`]). [`bake_world`] is the store's way in: it takes the
//! tileset, the textures of the world's package, the side files and the style and VRAM of the
//! map's area from the store.

use g4_formats::bdhc::{self, Bdhc};
use g4_formats::game::Family;
use g4_formats::map::{Cell, MAP_SIZE, MapFile};
use g4_formats::nsbmd_header::material_texture_names;
use serde::Serialize;

use super::check::{WORLD_SCHEMA_CODE, thousands};
use super::doc::WorldJson;
use super::freeform::{SideModels, load_side_models};
use super::grid::WorldGrid;
use super::mesh::{MODEL_BUDGET, MeshFace, footprint_of, mesh_with};
use super::tileset::{TileJson, Tileset};
use crate::bake::{
    AreaStyle, BakeCode, BakeInput, BakeMessage, BakeReport, Face, Level, PackTexture, WORLD_MAX,
    WORLD_MIN, area_vram, bake, pack_textures, vanilla_vram_max,
};
use crate::error::KernelError;
use crate::map_import::map_areas;
use crate::store::Store;
use crate::terrain::block_edges;

/// The revision of the world bake: raised whenever the same world bakes to other bytes (the
/// build step `world` salts its input hash with it).
pub const WORLD_BAKE_REV: u32 = 2;
/// The most the model floor may lie off the BDHC at a cell centre, in world units.
pub const FLOOR_TOLERANCE: f64 = 0.1;
/// The permission of a cell without a tile in any layer: blocked.
pub const EMPTY_CELL: Cell = Cell {
    kind: 0,
    collision: 0x80,
};

/// A painted map outside every matrix cell with a header.
pub const NO_AREA_CODE: &str = "world.no-area";
/// A verbatim model together with painted tiles or free-form models.
pub const VERBATIM_MIXED_CODE: &str = "world.verbatim-mixed";

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
    /// the model when `freeform` has an `nsbmd` entry, the BDHC when the world has `terrain`.
    pub fn of(world: &WorldJson, map: &MapFile) -> Verbatim {
        Verbatim {
            model: world
                .freeform
                .iter()
                .any(|f| f.nsbmd.is_some())
                .then(|| map.model.clone()),
            bdhc: world.terrain.is_some().then(|| map.bdhc.clone()),
        }
    }
}

/// What the pure bake needs: the map (for the messages), the tileset, the bake input of the
/// map's area (`None` without area), the family and the side files of the world.
pub struct BakeEnv<'a> {
    pub map: u16,
    pub tileset: &'a Tileset,
    pub input: Option<BakeInput<'a>>,
    pub family: Family,
    pub side: &'a SideModels,
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
    pub(crate) fn error(code: &str, args: Vec<String>, message: String) -> WorldBakeMessage {
        WorldBakeMessage {
            code: code.to_owned(),
            level: Level::Error,
            args,
            message,
        }
    }
}

impl From<&BakeMessage> for WorldBakeMessage {
    /// The baker's code, level and args; the message in German ([`baker_text`]), the baker's
    /// English one for a code the world bake does not give.
    fn from(m: &BakeMessage) -> WorldBakeMessage {
        WorldBakeMessage {
            code: m.code.as_str().to_owned(),
            level: m.level,
            args: m.args.clone(),
            message: baker_text(m.code, &m.args).unwrap_or_else(|| m.message.clone()),
        }
    }
}

/// The German sentence of the baker's finding `code` with `args` (as [`BakeCode`] documents
/// them), numbers with a space between thousands; `None` for a code of appending textures or
/// args of another shape.
pub fn baker_text(code: BakeCode, args: &[String]) -> Option<String> {
    let number = |s: &String| {
        s.parse::<usize>()
            .map(thousands)
            .unwrap_or_else(|_| s.clone())
    };
    Some(match (code, args) {
        (BakeCode::Empty, []) => "Die Welt hat keine Fläche zum Backen.".to_owned(),
        (BakeCode::ModelTooBig, [bytes, limit]) => format!(
            "Das gebackene Modell hat {} Byte, das Spiel fasst höchstens {} Byte; die Karte \
             behält ihr letztes Modell.",
            number(bytes),
            number(limit)
        ),
        (BakeCode::ModelTooBig, [error]) => {
            format!("Das gebackene Modell lässt sich nicht schreiben: {error}.")
        }
        (BakeCode::TextureMissing, [name]) => {
            format!("Die Textur „{name}“ fehlt im Kartentextur-Paket oder hat dort keine Palette.")
        }
        (BakeCode::FaceOutOfRange, [face, x, y, z]) => format!(
            "Fläche {face} hat die Ecke ({x}, {y}, {z}); Ecken liegen zwischen {WORLD_MIN} und \
             {WORLD_MAX} Welteinheiten."
        ),
        (BakeCode::FaceUvRange, [face, axis, span]) => format!(
            "Fläche {face} spannt {} Texel in {axis}; ihre Texturkoordinaten passen weder so \
             noch um ganze Texturen verschoben in den Bereich des Modells.",
            number(span)
        ),
        (BakeCode::VramHigh, [vram, limit]) => format!(
            "Die Texturen des Gebiets belegen {} Byte VRAM, das größte Vanilla-Gebiet {} Byte.",
            number(vram),
            number(limit)
        ),
        _ => return None,
    })
}

/// The parts of a bake: `model` is `None` when it is refused (a message says why) or verbatim.
/// `faces` are the grid's (with piece geometry), then the free-form ones.
#[derive(Debug, Clone)]
pub struct GridBake {
    pub faces: Vec<MeshFace>,
    /// Faces of free-form models among `faces` (the last ones).
    pub freeform_faces: usize,
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
    /// Faces of the bake: the mesher's (with piece geometry) and the free-form ones.
    pub faces: usize,
    /// Bytes of the map's model (the kept one when the bake was refused).
    pub model_bytes: usize,
    /// `model_bytes / 0xF000`.
    pub model_share: f32,
    /// The map kept its last model: the bake has none. False with a verbatim model, also when
    /// painted tiles refuse the bake (`world.verbatim-mixed`): the verbatim model stays, see
    /// [`WorldBakeReport::verbatim_model`] and [`WorldBakeReport::ok`].
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
    /// Faces of free-form models among `faces`.
    pub freeform_faces: usize,
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
/// order: the side files' messages (`env.side.messages`); with a verbatim model, painted or
/// free-form faces (`world.verbatim-mixed`); without one no face (`bake.empty`), no area
/// (`world.no-area`), else the baker's refusal and its VRAM warning. The faces are
/// [`mesh_with`] the piece models, then every free-form model that read, moved by its `at`. The
/// model is `None` on every refusal (a side file that is missing or does not read refuses it
/// too) and when it is verbatim; BDHC and permissions are always there.
pub fn bake_grid(
    grid: &WorldGrid,
    world: &WorldJson,
    verbatim: &Verbatim,
    env: &BakeEnv,
) -> GridBake {
    let mut faces = mesh_with(grid, env.tileset, &env.side.pieces);
    let grid_faces = faces.len();
    faces.extend(env.side.freeform_faces().cloned().map(MeshFace::from));
    let freeform_faces = faces.len() - grid_faces;
    let mut messages = env.side.messages.clone();
    let side_refused = messages.iter().any(|m| m.level == Level::Error);
    let mut model = None;
    if verbatim.model.is_some() {
        if !faces.is_empty() || world.freeform.iter().any(|f| f.glb.is_some()) {
            messages.push(WorldBakeMessage::error(
                VERBATIM_MIXED_CODE,
                Vec::new(),
                "Ein Verbatim-Modell (Form 1) backt nur allein; für Kacheln oder Freiform \
                 daneben die Karte als binäre Karte übernehmen und mit Form 2 (glTF) neu \
                 übernehmen."
                    .to_owned(),
            ));
        }
    } else if faces.is_empty() {
        messages.push(WorldBakeMessage::error(
            BakeCode::Empty.as_str(),
            Vec::new(),
            baker_text(BakeCode::Empty, &[]).unwrap_or_default(),
        ));
    } else if let Some(input) = &env.input {
        if !side_refused {
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
        freeform_faces,
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
        freeform_faces: bake.freeform_faces,
    }
}

/// A bake through the store: the map file (the Map asset of the world map) and its report.
#[derive(Debug, Clone)]
pub struct WorldBake {
    pub file: MapFile,
    pub report: WorldBakeReport,
}

/// The textures of package `pack` as the world bake binds them: a texture a tile of `tileset`
/// shows with a palette the pack has takes that palette (what the scene shows, the game
/// shows: Platinum's `allpeak` binds `apeak`, the palette of its index, in every vanilla
/// model, a palette [`pack_textures`] does not find); every other texture as
/// [`pack_textures`] binds it. In the pack's texture order.
pub fn world_pack_textures(
    pack: &[u8],
    tileset: &Tileset,
) -> Result<Vec<PackTexture>, g4_formats::error::FormatError> {
    let btx = g4_formats::nsbtx::Nsbtx::read(pack)?;
    let own = pack_textures(pack)?;
    Ok(btx
        .textures
        .iter()
        .filter_map(|t| {
            let named = tileset
                .tiles
                .iter()
                .filter(|tile| tile.texture == t.name)
                .find_map(|tile| tile.palette.as_deref())
                .filter(|&p| btx.palettes.iter().any(|q| q.name == p));
            match named {
                Some(palette) => Some(PackTexture {
                    name: t.name.clone(),
                    palette: palette.to_owned(),
                    width: u16::try_from(t.width).ok()?,
                    height: u16::try_from(t.height).ok()?,
                }),
                None => own.iter().find(|o| o.name == t.name).cloned(),
            }
        })
        .collect())
}

/// The bytes of package `package` (`pack` when given, else its member) and the textures the world
/// bake binds from them ([`world_pack_textures`] with `tileset`): the bake and the scene bind the
/// same. A package that is missing or does not read gives no bytes and no textures.
pub(crate) fn bake_pack(
    store: &Store,
    package: u16,
    tileset: &Tileset,
    pack: Option<&[u8]>,
) -> Result<(Option<Vec<u8>>, Vec<PackTexture>), KernelError> {
    let member = match pack {
        Some(bytes) => Some(bytes.to_vec()),
        None => store.project().map_tileset_member(package)?,
    };
    Ok(match member {
        Some(bytes) => match world_pack_textures(&bytes, tileset) {
            Ok(textures) => (Some(bytes), textures),
            Err(_) => (None, Vec::new()),
        },
        None => (None, Vec::new()),
    })
}

/// Bakes `world` for map `map` with the store: `Store::tileset(world.tileset)`,
/// [`world_pack_textures`] of that package's member, the side files bound to those textures
/// ([`load_side_models`]), style and VRAM of the first of `map_areas(store, map)`. `Err`
/// only for a world that does not parse (`world.schema`) or I/O; refusals are in the report.
///
/// `base` gives the buildings, the tail and the BGS, and the model a refused bake keeps;
/// `verbatim` the verbatim parts ([`Verbatim::of`]). A package that does not read gives no
/// textures (the baker refuses with `bake.texture-missing`) and no VRAM; a map outside every
/// matrix cell with a header, or whose area data is not in the archive, has no area.
pub fn bake_world(
    store: &mut Store,
    map: u16,
    world: &WorldJson,
    base: &MapFile,
    verbatim: &Verbatim,
) -> Result<WorldBake, KernelError> {
    bake_world_with(store, map, world, base, verbatim, None, None)
}

/// [`bake_world`] with what it would read held in memory: `pack` (when given) in place of the
/// package member's bytes, `side` (when given) in place of the side files on disk.
pub(crate) fn bake_world_with(
    store: &mut Store,
    map: u16,
    world: &WorldJson,
    base: &MapFile,
    verbatim: &Verbatim,
    pack: Option<&[u8]>,
    side: Option<&SideModels>,
) -> Result<WorldBake, KernelError> {
    let grid = WorldGrid::parse(world).map_err(|errors| {
        KernelError::Refused(match errors.first() {
            Some(e) => format!("{WORLD_SCHEMA_CODE}: {e}"),
            None => format!("{WORLD_SCHEMA_CODE}: world: kaputt."),
        })
    })?;
    let tileset = store.tileset(world.tileset)?;
    let family = store.project().family;
    let (pack, textures) = bake_pack(store, world.tileset, &tileset, pack)?;
    let loaded;
    let side = match side {
        Some(side) => side,
        None => {
            loaded = load_side_models(store.project(), map, world, &tileset, &textures);
            &loaded
        }
    };
    let mut area = None;
    if let Some(&id) = map_areas(store, map)?.first()
        && let Some(data) = store.area_data_checked(id)?
    {
        let vram = match &pack {
            Some(bytes) => match area_vram(store, id, Some(bytes)) {
                Ok(vram) => vram,
                // A building pack that does not read counts nothing, as a missing one.
                Err(KernelError::Format(_)) => 0,
                Err(e) => return Err(e),
            },
            None => 0,
        };
        area = Some((AreaStyle::of(&data), vram));
    }
    let env = BakeEnv {
        map,
        tileset: &tileset,
        input: area.map(|(style, vram_bytes)| BakeInput {
            style,
            textures: &textures,
            vram_bytes,
            vram_limit: vanilla_vram_max(family),
        }),
        family,
        side,
    };
    let bake = bake_grid(&grid, world, verbatim, &env);
    let (file, kept) = assemble(base, &bake, verbatim);
    let report = report(map, &bake, &file, kept, area.map(|(style, _)| style));
    Ok(WorldBake { file, report })
}

/// „Karte 0123 ist eine Welt-Karte: Modell, Terrain und Permissions entstehen beim Backen;
/// Permissions über `world_edit` (`permission`), zurück zur binären Karte über „Als binäre
/// Karte übernehmen“.“: why an edit of a world map's derived parts is refused.
pub fn world_derived_refusal(map: u16) -> String {
    format!(
        "Karte {map:04} ist eine Welt-Karte: Modell, Terrain und Permissions entstehen beim \
         Backen; Permissions über `world_edit` (`permission`), zurück zur binären Karte über \
         „Als binäre Karte übernehmen“."
    )
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

#[cfg(all(test, feature = "test-support"))]
mod store_tests;
#[cfg(test)]
mod tests;
