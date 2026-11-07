//! „Welt anlegen“ ([`world_create`]), the changes of cells ([`world_edit`]) and „Als binäre
//! Karte übernehmen“ ([`world_to_binary`]): each is one undo step on the store's world map
//! asset ([`AssetKind::WorldMap`]), whose bytes are the world object of the map's JSON, and
//! (create, edit) on the map it bakes into ([`crate::store::Tx::edit_world`]). All are refused
//! outside a text project of the current schema ([`WORLD_NEEDS_TEXT_PROJECT`]) and change
//! nothing when they are refused.
//!
//! A change names cells and exactly one of `tile` (on a layer, the ground by default),
//! `height`, `ramp` (both the ground's only) and `permission` (the map's override, no layer),
//! with the tokens of the file. Only the rows of the touched cells are written anew
//! (normalized); every other row keeps its bytes.

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::project::SourceMode;
use crate::store::{ChangeId, Store};

use super::bake::WorldBakeReport;
use super::check::tile_errors;
use super::doc::WorldJson;
use super::grid::{
    WorldGrid, override_token, parse_height, parse_override, parse_ramp, parse_tile, ramp_token,
    set_token, tile_token,
};
use super::keep::locked_map_refusal;
use super::places::packs_of_map;
use super::tileset::{Tileset, tile_outside};

/// Why painting is refused outside a text project (`world_create`, `world_edit`,
/// `tileset_save` in the DSPRE layout or a text project of another schema).
pub const WORLD_NEEDS_TEXT_PROJECT: &str =
    "Boden malen gibt es im Textprojekt (Technik › Textprojekt).";
/// A change with none or more than one of its fields.
pub const ONE_FIELD: &str = "Eine Änderung setzt genau eines von tile, height, ramp, permission.";
/// A call whose changes leave the world as it is.
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";
/// Why a world has no state to go back to (`reset` „Auf Original zurücksetzen“).
pub const WORLD_HAS_NO_ORIGINAL: &str =
    "Eine Welt hat kein Original; erst „Als binäre Karte übernehmen“.";
/// A height or a ramp on an overlay.
const GROUND_ONLY: &str = "Höhen und Rampen gibt es nur auf der Bodenebene (0).";
/// A permission with a layer.
const PERMISSION_NO_LAYER: &str =
    "permission gilt für die Karte, nicht für eine Ebene; layer weglassen.";
/// The tile a new world is filled with.
const DEFAULT_FILL: &str = "0000";
/// Cells per row and rows per layer.
const MAP_CELLS: usize = g4_formats::map::MAP_SIZE;
/// The token of an empty cell.
const EMPTY: &str = ".";

/// „Karte 0123 ist keine Welt-Karte; erst „Welt anlegen“ (world_create).“
pub fn not_a_world_map(map: u16) -> String {
    format!("Karte {map:04} ist keine Welt-Karte; erst „Welt anlegen“ (world_create).")
}

/// „Karte 0123 ist schon eine Welt-Karte.“
pub fn already_world(map: u16) -> String {
    format!("Karte {map:04} ist schon eine Welt-Karte.")
}

/// „Karte 0123 liegt in keiner Matrixzelle mit Header; Tileset angeben (`tileset`).“
fn no_package(map: u16) -> String {
    format!("Karte {map:04} liegt in keiner Matrixzelle mit Header; Tileset angeben (`tileset`).")
}

/// „Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).“
fn cell_outside(x: u8, z: u8) -> String {
    format!(
        "Zelle ({x}, {z}) liegt außerhalb der Karte (0 bis {}).",
        MAP_CELLS - 1
    )
}

/// „Ebene 2 gibt es nicht (Karte 0123 hat 1).“
fn no_layer(layer: u8, map: u16, layers: usize) -> String {
    format!("Ebene {layer} gibt es nicht (Karte {map:04} hat {layers}).")
}

/// `world_create`: the map, the map texture package (`None`: the first package of the headers
/// that show the map, [`packs_of_map`]), the tile the ground is filled with (`None`: `0000`)
/// and its height (`None`: 0).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCreateRequest {
    pub map: u16,
    #[serde(default)]
    pub tileset: Option<u16>,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub height: Option<i8>,
}

/// `world_edit`: the map, the label of the step (`None`: „Welt geändert: Karte 0123 (37
/// Zellen)“) and the changes, all of them one undo step.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldEditRequest {
    pub map: u16,
    #[serde(default)]
    pub label: Option<String>,
    pub changes: Vec<WorldChange>,
}

/// One change: the cells (`[x, z]`) and exactly one of `tile` (on `layer`, the ground when it
/// is left out), `height`, `ramp` (the ground's) and `permission` (no `layer`), as tokens of
/// the file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldChange {
    #[serde(default)]
    pub layer: Option<u8>,
    pub cells: Vec<[u8; 2]>,
    #[serde(default)]
    pub tile: Option<String>,
    #[serde(default)]
    pub height: Option<i8>,
    #[serde(default)]
    pub ramp: Option<String>,
    #[serde(default)]
    pub permission: Option<String>,
}

/// What `world_create`, `world_edit`, `world_layers` or `world_to_binary` did: the map, its
/// package, the cells it set (a new world: all 1024 of the ground; a layer added or removed or
/// the world given up: 0), the undo step (`None`: nothing recorded) and the bake of the step
/// (`None` when it baked nothing: the world given up). A refused bake is in its messages, never an error of the call: the stroke stays.
#[derive(Debug, Clone, Serialize)]
pub struct WorldReport {
    pub map: u16,
    pub tileset: u16,
    pub cells: usize,
    pub change: Option<ChangeId>,
    pub bake: Option<WorldBakeReport>,
}

/// „Welt aufgegeben: Karte 0123“, the label of [`world_to_binary`].
pub fn world_to_binary_label(map: u16) -> String {
    format!("Welt aufgegeben: Karte {map:04}")
}

/// The rows a change writes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rows {
    Tiles(usize),
    Heights,
    Ramps,
    Overrides,
}

/// A checked change: its rows, the normalized token and the cells.
struct Checked<'a> {
    rows: Rows,
    token: String,
    cells: &'a [[u8; 2]],
}

/// Refused outside a text project of the current schema.
pub(super) fn require_text_project(store: &Store) -> Result<(), KernelError> {
    if *store.project().source_mode() != SourceMode::TextProject {
        return Err(KernelError::Refused(WORLD_NEEDS_TEXT_PROJECT.to_owned()));
    }
    Ok(())
}

/// The first error of a world that does not read into cells, with its path.
fn first_error(world: &WorldJson) -> Result<(), KernelError> {
    WorldGrid::parse(world).map(|_| ()).map_err(|errors| {
        KernelError::Refused(
            errors
                .first()
                .map_or_else(|| "world: kaputt.".to_owned(), ToString::to_string),
        )
    })
}

/// The first error of a world that does not read into cells or does not fit `tileset` (a tile
/// past it, a piece that is none or lies over the edge: [`tile_errors`]), with its path.
fn first_error_with(world: &WorldJson, tileset: &Tileset) -> Result<(), KernelError> {
    let grid = WorldGrid::parse(world).map_err(|errors| {
        KernelError::Refused(
            errors
                .first()
                .map_or_else(|| "world: kaputt.".to_owned(), ToString::to_string),
        )
    })?;
    match tile_errors(&grid, tileset).first() {
        Some(e) => Err(KernelError::Refused(e.to_string())),
        None => Ok(()),
    }
}

/// Refused when map `map` cannot carry a world: its member does not decode (it is written as
/// `{locked, raw}`, without a place for a world) or the store holds it locked. `refusal` says
/// why from the cause (the decoder's error or the lock's reason): [`locked_map_refusal`] for a
/// world map, [`binary_map_refusal`] for a map that is to become one.
pub(super) fn refuse_locked_map(
    store: &mut Store,
    map: u16,
    refusal: impl Fn(&str) -> String,
) -> Result<(), KernelError> {
    match store.map(map) {
        Ok(_) => {}
        Err(e @ (KernelError::Format(_) | KernelError::Uxie(_))) => {
            return Err(refused(refusal(&e.to_string())));
        }
        Err(e) => return Err(e),
    }
    let id = crate::project::AssetId {
        kind: crate::project::AssetKind::Map,
        index: map,
    };
    if let Some(why) = store.locked(id) {
        return Err(refused(refusal(why)));
    }
    Ok(())
}

/// „Karte 0001 lässt sich nicht übernehmen: ihr Member lässt sich nicht lesen (…).“: why a
/// binary map whose member is locked or does not read does not become a world map; `action`
/// is what was asked („übernehmen“, „als Welt anlegen“), `cause` the decoder's error or the
/// lock's reason.
pub(super) fn binary_map_refusal(map: u16, action: &str, cause: &str) -> String {
    format!(
        "Karte {map:04} lässt sich nicht {action}: ihr Member lässt sich nicht lesen ({cause})."
    )
}

/// Refused with `text` (a parser's sentence).
fn refused(text: String) -> KernelError {
    KernelError::Refused(text)
}

/// „Welt anlegen“: the world of map `req.map` as one undo step „Welt angelegt: Karte 0123“, a
/// ground layer „Boden“ all `fill` at `height`, no ramp, permissions derived.
///
/// Refused ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`WORLD_NEEDS_TEXT_PROJECT`]), a map that is a world map already ([`already_world`]), a map
/// whose member does not read (it cannot carry a world, see [`super::keep::carry_world`]), no
/// package given and none from the headers, a package that is not there (the refusal of
/// [`Store::tileset`]), a `fill` that is no tile or lies past the tileset, a height outside
/// −16 to 62.
pub fn world_create(
    store: &mut Store,
    req: &WorldCreateRequest,
) -> Result<WorldReport, KernelError> {
    require_text_project(store)?;
    let map = req.map;
    if store.world(map)?.is_some() {
        return Err(refused(already_world(map)));
    }
    refuse_locked_map(store, map, |cause| {
        binary_map_refusal(map, "als Welt anlegen", cause)
    })?;
    let package = match req.tileset {
        Some(package) => package,
        None => *packs_of_map(store, map)?
            .first()
            .ok_or_else(|| refused(no_package(map)))?,
    };
    let tiles = store.tileset(package)?.tiles.len();
    let token = req.fill.as_deref().unwrap_or(DEFAULT_FILL);
    let fill = parse_tile(token).map_err(refused)?.ok_or_else(|| {
        refused(format!(
            "`{token}` füllt keinen Boden; eine Kachel angeben."
        ))
    })?;
    if usize::from(fill.tile) >= tiles {
        return Err(refused(tile_outside("fill", fill.tile, package, tiles)));
    }
    let height = req.height.unwrap_or(0);
    parse_height(&height.to_string()).map_err(refused)?;
    let world = WorldJson::blank(package, fill, height);
    first_error(&world)?;
    let label = format!("Welt angelegt: Karte {map:04}");
    let (bake, change) = store.transaction(&label, |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    Ok(WorldReport {
        map,
        tileset: package,
        cells: MAP_CELLS * MAP_CELLS,
        change,
        bake,
    })
}

/// The changes `req.changes` of the world of map `req.map` as one undo step (label `req.label`,
/// else „Welt geändert: Karte 0123 (37 Zellen)“). Only the rows of touched cells are written
/// anew; the world must read into cells and fit its tileset before and after.
///
/// Refused ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`WORLD_NEEDS_TEXT_PROJECT`]), a map without world ([`not_a_world_map`]), a world that does
/// not read into cells or does not fit its tileset already (its first error with path, a tile
/// past the tileset: [`tile_outside`]), a change with none or more than
/// one field ([`ONE_FIELD`]), a permission with a layer, a height or ramp off the ground, a
/// layer the world lacks, a cell off the map, a token the parser refuses (its sentence), a tile
/// past the tileset ([`tile_outside`]), a package that is not there, changes that change
/// nothing ([`NOTHING_TO_CHANGE`]).
pub fn world_edit(store: &mut Store, req: &WorldEditRequest) -> Result<WorldReport, KernelError> {
    require_text_project(store)?;
    let map = req.map;
    let before = store
        .world(map)?
        .cloned()
        .ok_or_else(|| refused(not_a_world_map(map)))?;
    refuse_locked_map(store, map, |_| locked_map_refusal(map))?;
    first_error(&before)?;
    let tileset = store.tileset(before.tileset)?;
    first_error_with(&before, &tileset)?;
    let tiles = tileset.tiles.len();
    let mut checked = Vec::with_capacity(req.changes.len());
    for (i, change) in req.changes.iter().enumerate() {
        checked.push(check_change(change, i, map, &before, tiles)?);
    }
    let mut world = before.clone();
    // Heights or ramps painted: the BDHC comes from the grid from now on.
    if checked
        .iter()
        .any(|c| matches!(c.rows, Rows::Heights | Rows::Ramps))
    {
        world.terrain = None;
    }
    let had_ramps = world.layers.first().is_some_and(|l| l.ramps.is_some());
    let had_overrides = world.permissions.overrides.is_some();
    for change in &checked {
        let rows = rows_of(&mut world, change.rows)?;
        for &[x, z] in change.cells {
            let row = rows
                .get_mut(usize::from(z))
                .ok_or_else(|| refused(cell_outside(x, z)))?;
            *row = set_token(row, usize::from(x), &change.token).map_err(refused)?;
        }
    }
    // Rows made for a change that left every cell empty go again: the file stays as it was.
    let all_empty = |rows: &[String]| {
        rows.iter()
            .all(|r| r.split_whitespace().all(|t| t == EMPTY))
    };
    if let Some(ground) = world.layers.first_mut()
        && !had_ramps
        && ground.ramps.as_deref().is_some_and(all_empty)
    {
        ground.ramps = None;
    }
    if !had_overrides
        && world.permissions.derive
        && world
            .permissions
            .overrides
            .as_deref()
            .is_some_and(all_empty)
    {
        world.permissions.overrides = None;
    }
    if world == before {
        return Err(refused(NOTHING_TO_CHANGE.to_owned()));
    }
    first_error_with(&world, &tileset)?;
    let cells: usize = req.changes.iter().map(|c| c.cells.len()).sum();
    let label = match &req.label {
        Some(label) => label.clone(),
        None if cells == 1 => format!("Welt geändert: Karte {map:04} (1 Zelle)"),
        None => format!("Welt geändert: Karte {map:04} ({cells} Zellen)"),
    };
    let package = world.tileset;
    let (bake, change) = store.transaction(&label, |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    Ok(WorldReport {
        map,
        tileset: package,
        cells,
        change,
        bake,
    })
}

/// „Als binäre Karte übernehmen“ without a conflict: the world of map `map` goes as one undo
/// step ([`world_to_binary_label`]); the Map asset (its last bake) stays, so the next save
/// writes the map binary, with its permissions, model and BDHC.
///
/// Refused ([`KernelError::Refused`], nothing changed): outside a text project
/// ([`WORLD_NEEDS_TEXT_PROJECT`]), a map without world ([`not_a_world_map`]).
pub fn world_to_binary(store: &mut Store, map: u16) -> Result<WorldReport, KernelError> {
    require_text_project(store)?;
    let package = store
        .world(map)?
        .map(|w| w.tileset)
        .ok_or_else(|| refused(not_a_world_map(map)))?;
    // The bake the map keeps: loaded before the world goes.
    store.map(map)?;
    let ((), change) = store.transaction(&world_to_binary_label(map), |tx| {
        tx.edit_world(map, |w| {
            *w = None;
            Ok(())
        })
    })?;
    Ok(WorldReport {
        map,
        tileset: package,
        cells: 0,
        change,
        bake: None,
    })
}

/// Change `i` checked against `world` (map `map`, `tiles` tiles in its tileset): its rows and
/// its token, normalized.
fn check_change<'a>(
    change: &'a WorldChange,
    i: usize,
    map: u16,
    world: &WorldJson,
    tiles: usize,
) -> Result<Checked<'a>, KernelError> {
    let fields = [
        change.tile.is_some(),
        change.height.is_some(),
        change.ramp.is_some(),
        change.permission.is_some(),
    ];
    if fields.iter().filter(|&&set| set).count() != 1 {
        return Err(refused(ONE_FIELD.to_owned()));
    }
    let layer = change.layer.unwrap_or(0);
    if change.permission.is_some() && change.layer.is_some() {
        return Err(refused(PERMISSION_NO_LAYER.to_owned()));
    }
    if (change.height.is_some() || change.ramp.is_some()) && layer != 0 {
        return Err(refused(GROUND_ONLY.to_owned()));
    }
    if usize::from(layer) >= world.layers.len() {
        return Err(refused(no_layer(layer, map, world.layers.len())));
    }
    for &[x, z] in &change.cells {
        if usize::from(x) >= MAP_CELLS || usize::from(z) >= MAP_CELLS {
            return Err(refused(cell_outside(x, z)));
        }
    }
    let (rows, token) = if let Some(token) = &change.tile {
        let tile = parse_tile(token).map_err(refused)?;
        if let Some(t) = tile
            && usize::from(t.tile) >= tiles
        {
            let path = format!("changes[{i}].tile");
            return Err(refused(tile_outside(&path, t.tile, world.tileset, tiles)));
        }
        (Rows::Tiles(usize::from(layer)), tile_token(tile))
    } else if let Some(height) = change.height {
        let token = height.to_string();
        parse_height(&token).map_err(refused)?;
        (Rows::Heights, token)
    } else if let Some(token) = &change.ramp {
        (Rows::Ramps, ramp_token(parse_ramp(token).map_err(refused)?))
    } else {
        let token = change.permission.as_deref().unwrap_or_default();
        (
            Rows::Overrides,
            override_token(parse_override(token).map_err(refused)?),
        )
    };
    Ok(Checked {
        rows,
        token,
        cells: &change.cells,
    })
}

/// The rows of `world` a change of `rows` writes; the ramps and the overrides are made all `.`
/// when the world has none yet.
fn rows_of(world: &mut WorldJson, rows: Rows) -> Result<&mut Vec<String>, KernelError> {
    let empty = || vec![vec![EMPTY; MAP_CELLS].join(" "); MAP_CELLS];
    let missing = || refused("world.layers: die Bodenebene fehlt.".to_owned());
    Ok(match rows {
        Rows::Tiles(layer) => &mut world.layers.get_mut(layer).ok_or_else(missing)?.tiles,
        Rows::Heights => world
            .layers
            .first_mut()
            .and_then(|ground| ground.heights.as_mut())
            .ok_or_else(missing)?,
        Rows::Ramps => world
            .layers
            .first_mut()
            .ok_or_else(missing)?
            .ramps
            .get_or_insert_with(empty),
        Rows::Overrides => world.permissions.overrides.get_or_insert_with(empty),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{Value, json};

    use super::*;
    use crate::project::{AssetId, AssetKind};
    use crate::test_support::TextFixture;
    use crate::textproject::canon::to_canonical;
    use crate::textproject::part::sha1_hex;
    use crate::world::grid::TileRef;
    use crate::world::keep::{world_field, world_parts};
    use crate::world::tileset::{Tileset, tileset_rel};

    const MAP: &str = "data/maps/0000.json";

    fn store(fx: &TextFixture) -> Store {
        Store::new(Arc::new(fx.project()))
    }

    /// The tileset file of `package`: the synthetic tileset (six tiles).
    fn put_tileset(fx: &TextFixture, package: u16) {
        let tileset = Tileset {
            package,
            ..Tileset::synthetic()
        };
        let rel = tileset_rel(package);
        fx.write(rel.to_str().unwrap(), &to_canonical(&tileset));
    }

    /// A text project with the synthetic tileset as package 0 and its store.
    fn fixture() -> (TextFixture, Store) {
        let fx = TextFixture::exported();
        put_tileset(&fx, 0);
        let store = store(&fx);
        (fx, store)
    }

    fn create_req(value: Value) -> WorldCreateRequest {
        serde_json::from_value(value).unwrap()
    }

    fn edit_req(value: Value) -> WorldEditRequest {
        serde_json::from_value(value).unwrap()
    }

    /// „Welt anlegen“ on map `map`, package 0.
    fn create(store: &mut Store, map: u16) -> WorldReport {
        world_create(store, &create_req(json!({"map": map, "tileset": 0}))).unwrap()
    }

    fn edit(store: &mut Store, value: Value) -> Result<WorldReport, KernelError> {
        world_edit(store, &edit_req(value))
    }

    fn world(store: &mut Store, map: u16) -> Option<WorldJson> {
        store.world(map).unwrap().cloned()
    }

    fn refused<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
        match r {
            Err(KernelError::Refused(text)) => text,
            other => panic!("{other:?}"),
        }
    }

    fn member_path(fx: &TextFixture) -> std::path::PathBuf {
        fx.project().member_path(AssetKind::Map, 0)
    }

    fn member_sha1(fx: &TextFixture) -> String {
        sha1_hex(&std::fs::read(member_path(fx)).unwrap())
    }

    fn file_json(fx: &TextFixture) -> Value {
        serde_json::from_slice(&fx.read(MAP)).unwrap()
    }

    fn blank() -> WorldJson {
        WorldJson::blank(
            0,
            TileRef {
                tile: 0,
                rotate: 0,
                flip: false,
            },
            0,
        )
    }

    /// Moves the first building of map 0 as one step (a world map's own part: its derived
    /// parts are the bake's).
    fn move_building(store: &mut Store) {
        store
            .edit_map(0, "Gebäude verschoben", |m| {
                m.buildings[0].position[0] += 1;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn world_create_and_edit_are_one_undo_step_each() {
        let (_fx, mut store) = fixture();
        assert_eq!(world(&mut store, 0), None);
        let made = create(&mut store, 0);
        assert_eq!((made.map, made.tileset, made.cells), (0, 0, 1024));
        assert!(made.change.is_some());
        let created = world(&mut store, 0).unwrap();
        assert_eq!(created, blank());

        // 37 cells of tile 0001 (row 3 and five cells of row 4) and 4 heights in one call.
        let tiles: Vec<[u8; 2]> = (0..32)
            .map(|x| [x, 3])
            .chain((0..5).map(|x| [x, 4]))
            .collect();
        let report = edit(
            &mut store,
            json!({"map": 0, "changes": [
                {"cells": tiles, "tile": "0001"},
                {"cells": [[0, 10], [1, 10], [2, 10], [3, 10]], "height": 3}
            ]}),
        )
        .unwrap();
        assert_eq!((report.map, report.tileset, report.cells), (0, 0, 41));
        assert!(report.change.is_some());
        let (undo, redo) = store.history();
        assert_eq!(
            undo,
            [
                "Welt angelegt: Karte 0000",
                "Welt geändert: Karte 0000 (41 Zellen)"
            ]
        );
        assert!(redo.is_empty());
        let edited = world(&mut store, 0).unwrap();
        let grid = WorldGrid::parse(&edited).unwrap();
        let tile = |x: usize, z: usize| grid.layers[0].tiles[z * 32 + x].unwrap().tile;
        assert_eq!((tile(31, 3), tile(4, 4), tile(5, 4)), (1, 1, 0));
        assert_eq!(
            (grid.heights[10 * 32 + 3], grid.heights[10 * 32 + 4]),
            (3, 0)
        );

        // The world and the map it bakes into.
        assert_eq!(
            store.undo().unwrap(),
            [
                AssetId {
                    kind: AssetKind::WorldMap,
                    index: 0
                },
                AssetId {
                    kind: AssetKind::Map,
                    index: 0
                }
            ]
        );
        assert_eq!(world(&mut store, 0), Some(created.clone()));
        store.undo().unwrap();
        assert_eq!(world(&mut store, 0), None);
        assert!(store.history().0.is_empty());
        store.redo().unwrap();
        assert_eq!(world(&mut store, 0), Some(created));
        store.redo().unwrap();
        assert_eq!(world(&mut store, 0), Some(edited));
        assert_eq!(store.history().0.len(), 2);
        // A label of the caller's.
        let report = edit(
            &mut store,
            json!({"map": 0, "label": "Strich", "changes": [{"cells": [[9, 9]], "tile": "0002"}]}),
        )
        .unwrap();
        assert_eq!(report.cells, 1);
        assert_eq!(store.history().0.last().unwrap(), "Strich");
    }

    #[test]
    fn undo_of_create_returns_to_binary() {
        let (fx, mut store) = fixture();
        let original = fx.read(MAP);
        create(&mut store, 0);
        store.save().unwrap();
        assert_eq!(file_json(&fx)["source"], "world");
        store.undo().unwrap();
        store.save().unwrap();
        let v = file_json(&fx);
        assert_eq!(v["source"], "binary");
        assert!(v.get("world").is_none());
        assert_eq!(fx.read(MAP), original);
        // A store opened anew reads no world.
        assert_eq!(world(&mut store_of(&fx), 0), None);
    }

    fn store_of(fx: &TextFixture) -> Store {
        store(fx)
    }

    #[test]
    fn save_writes_world_and_the_baked_member() {
        let (fx, mut store) = fixture();
        let sha1 = member_sha1(&fx);
        create(&mut store, 0);
        let both = [
            AssetId {
                kind: AssetKind::Map,
                index: 0,
            },
            AssetId {
                kind: AssetKind::WorldMap,
                index: 0,
            },
        ];
        assert_eq!(store.dirty(), both);
        let report = store.save().unwrap();
        assert_eq!(report.written, both);
        assert!(store.dirty().is_empty());
        // The member is the bake now (BDHC and permissions of the world; the stub package
        // has no textures, so the model stays).
        assert_ne!(member_sha1(&fx), sha1);
        let family = fx.project().family;
        let member = std::fs::read(member_path(&fx)).unwrap();
        assert_eq!(store.map(0).unwrap().write(family), member);
        let v = file_json(&fx);
        assert_eq!(v["source"], "world");
        assert_eq!(world_field(&fx.read(MAP)).unwrap(), Some(blank()));
        // Read back by a new store, the map as its bake: the member saved.
        let mut again = store_of(&fx);
        assert_eq!(world(&mut again, 0), Some(blank()));
        assert_eq!(again.map(0).unwrap().write(family), member);
        // A second save writes nothing.
        let before = fx.read(MAP);
        assert!(store.save().unwrap().written.is_empty());
        assert_eq!(fx.read(MAP), before);
    }

    #[test]
    fn a_map_edit_keeps_the_world() {
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        move_building(&mut store);
        store.save().unwrap();
        assert_eq!(world_field(&fx.read(MAP)).unwrap(), Some(blank()));
        let family = fx.project().family;
        let map = store.map(0).unwrap().write(family);
        assert_eq!(std::fs::read(member_path(&fx)).unwrap(), map);
        // And the other way round: a saved world, then a map edit alone.
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        store.save().unwrap();
        let with_world = fx.read(MAP);
        move_building(&mut store);
        store.save().unwrap();
        assert_eq!(world_field(&fx.read(MAP)).unwrap(), Some(blank()));
        // The building is the file's: it moves there and in the member.
        assert_ne!(fx.read(MAP), with_world);
        let x = store.map(0).unwrap().buildings[0].position[0];
        assert_eq!(file_json(&fx)["buildings"][0]["position"][0], x);
        let map = store.map(0).unwrap().write(family);
        assert_eq!(std::fs::read(member_path(&fx)).unwrap(), map);
    }

    #[test]
    fn world_and_map_steps_undo_one_by_one() {
        let (fx, mut store) = fixture();
        let member = || std::fs::read(member_path(&fx)).unwrap();
        let mut states = vec![(fx.read(MAP), member())];
        let step = |store: &mut Store, what: usize| {
            match what {
                0 => {
                    create(store, 0);
                }
                1 => move_building(store),
                _ => {
                    edit(
                        store,
                        json!({"map": 0, "changes": [{"cells": [[3, 3]], "tile": "0001"}]}),
                    )
                    .unwrap();
                }
            }
            store.save().unwrap();
        };
        for what in 0..3 {
            step(&mut store, what);
            states.push((fx.read(MAP), member()));
        }
        assert_eq!(
            store.history().0,
            [
                "Welt angelegt: Karte 0000",
                "Gebäude verschoben",
                "Welt geändert: Karte 0000 (1 Zelle)"
            ]
        );
        // Each step writes the member (a world step its bake, the building move its own part)
        // and the file (the world, the building).
        assert_ne!(states[1].1, states[0].1);
        assert_ne!(states[2].1, states[1].1);
        assert_ne!(states[3].1, states[2].1);
        assert_ne!(states[1].0, states[0].0);
        assert_ne!(states[2].0, states[1].0);
        assert_ne!(states[3].0, states[2].0);
        for at in (0..3).rev() {
            store.undo().unwrap();
            store.save().unwrap();
            assert_eq!((fx.read(MAP), member()), states[at], "undo to {at}");
        }
        for (at, state) in states.iter().enumerate().skip(1) {
            store.redo().unwrap();
            store.save().unwrap();
            assert_eq!(&(fx.read(MAP), member()), state, "redo to {at}");
        }
    }

    #[test]
    fn world_edit_writes_only_touched_rows() {
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        store.save().unwrap();
        let before = String::from_utf8(fx.read(MAP)).unwrap();
        // An upper-case token is written as the file writes it.
        edit(
            &mut store,
            json!({"map": 0, "changes": [{"cells": [[4, 7]], "tile": "0002R1"}]}),
        )
        .unwrap();
        let world = world(&mut store, 0).unwrap();
        let blank = blank();
        for (z, (now, was)) in world.layers[0]
            .tiles
            .iter()
            .zip(&blank.layers[0].tiles)
            .enumerate()
        {
            if z == 7 {
                let tokens: Vec<&str> = now.split(' ').collect();
                assert_eq!(tokens[4], "0002r1");
                assert_eq!(tokens.iter().filter(|t| **t == "0000").count(), 31);
            } else {
                assert_eq!(now, was, "row {z}");
            }
        }
        assert_eq!(world.layers[0].heights, blank.layers[0].heights);
        store.save().unwrap();
        let after = String::from_utf8(fx.read(MAP)).unwrap();
        let differ: Vec<(&str, &str)> = before
            .lines()
            .zip(after.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(before.lines().count(), after.lines().count());
        assert_eq!(differ.len(), 1, "{differ:#?}");
        assert!(
            differ[0].1.contains(&world.layers[0].tiles[7]),
            "{differ:?}"
        );
        // Byte for byte the file with that one line (row 7 of the tiles).
        let at = before.lines().position(|l| l == differ[0].0).unwrap() + 7;
        let expected: Vec<&str> = before
            .lines()
            .enumerate()
            .map(|(i, l)| if i == at { differ[0].1 } else { l })
            .collect();
        assert_eq!(after, expected.join("\n") + "\n");
    }

    #[test]
    fn world_edit_refuses_each_rule() {
        let (fx, mut store) = fixture();
        assert_eq!(
            refused(edit(
                &mut store,
                json!({"map": 0, "changes": [{"cells": [[0, 0]], "tile": "0001"}]})
            )),
            "Karte 0000 ist keine Welt-Karte; erst „Welt anlegen“ (world_create)."
        );
        create(&mut store, 0);
        store.save().unwrap();
        let file = fx.read(MAP);
        let world_before = world(&mut store, 0);
        let history = store.history();
        let cases: Vec<(Value, &str)> = vec![
            (json!([{"cells": [[0, 0]]}]), ONE_FIELD),
            (
                json!([{"cells": [[0, 0]], "tile": "0001", "height": 2}]),
                ONE_FIELD,
            ),
            (
                json!([{"cells": [[0, 0]], "ramp": "n1", "permission": "0080"}]),
                ONE_FIELD,
            ),
            (
                json!([{"cells": [[32, 4]], "tile": "0001"}]),
                "Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).",
            ),
            (
                json!([{"layer": 2, "cells": [[0, 0]], "tile": "0001"}]),
                "Ebene 2 gibt es nicht (Karte 0000 hat 1).",
            ),
            (
                json!([{"layer": 1, "cells": [[0, 0]], "height": 2}]),
                "Höhen und Rampen gibt es nur auf der Bodenebene (0).",
            ),
            (
                json!([{"layer": 1, "cells": [[0, 0]], "ramp": "n1"}]),
                "Höhen und Rampen gibt es nur auf der Bodenebene (0).",
            ),
            (
                json!([{"layer": 0, "cells": [[0, 0]], "permission": "0080"}]),
                "permission gilt für die Karte, nicht für eine Ebene; layer weglassen.",
            ),
            (
                json!([{"cells": [[0, 0]], "tile": "00g2"}]),
                "`00g2` ist keine Kachel (`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` \
                 und `f`).",
            ),
            (
                json!([{"cells": [[0, 0]], "height": 70}]),
                "`70` liegt nicht in \u{2212}16 bis 62.",
            ),
            (
                json!([{"cells": [[0, 0]], "ramp": "n3"}]),
                "`n3` ist keine Rampe (`.` oder `n`, `e`, `s`, `w` mit 1 oder 2 Stufen).",
            ),
            (
                json!([{"cells": [[0, 0]], "permission": "80"}]),
                "`80` ist keine Permission (`.` oder `kkcc` aus vier Hex-Ziffern, Art und \
                 Kollision).",
            ),
            (
                json!([{"cells": [[0, 0]], "tile": "0006"}]),
                "changes[0].tile: Kachel 0x0006 gibt es im Tileset 0000 nicht (6 Kacheln).",
            ),
            (
                json!([{"cells": [[0, 0]], "tile": "0001"}, {"cells": [[0, 40]], "tile": "0001"}]),
                "Zelle (0, 40) liegt außerhalb der Karte (0 bis 31).",
            ),
            (
                json!([{"cells": [[0, 0]], "tile": "0000"}]),
                NOTHING_TO_CHANGE,
            ),
            (json!([{"cells": [[0, 0]], "height": 0}]), NOTHING_TO_CHANGE),
            (json!([{"cells": [[0, 0]], "ramp": "."}]), NOTHING_TO_CHANGE),
            (
                json!([{"cells": [[0, 0]], "permission": "."}]),
                NOTHING_TO_CHANGE,
            ),
            (json!([{"cells": [], "tile": "0001"}]), NOTHING_TO_CHANGE),
            (json!([]), NOTHING_TO_CHANGE),
        ];
        for (changes, expected) in cases {
            let text = refused(edit(&mut store, json!({"map": 0, "changes": changes})));
            assert_eq!(text, expected, "{changes}");
            assert_eq!(world(&mut store, 0), world_before, "{changes}");
            assert_eq!(store.history(), history, "{changes}");
            assert!(store.dirty().is_empty(), "{changes}");
            assert_eq!(fx.read(MAP), file, "{changes}");
        }
        // Unknown keys of a request are serde's.
        assert!(
            serde_json::from_value::<WorldEditRequest>(
                json!({"map": 0, "changes": [{"cells": [], "tile": "0001", "paint": 1}]})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<WorldCreateRequest>(json!({"map": 0, "fil": "0001"})).is_err()
        );
    }

    #[test]
    fn world_create_refuses_each_rule() {
        let (fx, mut store) = fixture();
        let file = fx.read(MAP);
        let cases: Vec<(Value, String)> = vec![
            (
                json!({"map": 0, "tileset": 0, "fill": "00g2"}),
                "`00g2` ist keine Kachel (`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` und \
                 `f`)."
                    .to_owned(),
            ),
            (
                json!({"map": 0, "tileset": 0, "fill": "0007"}),
                "fill: Kachel 0x0007 gibt es im Tileset 0000 nicht (6 Kacheln).".to_owned(),
            ),
            (
                json!({"map": 0, "tileset": 0, "fill": "."}),
                "`.` füllt keinen Boden; eine Kachel angeben.".to_owned(),
            ),
            (
                json!({"map": 0, "tileset": 0, "height": 70}),
                "`70` liegt nicht in \u{2212}16 bis 62.".to_owned(),
            ),
            (
                json!({"map": 0, "tileset": 0, "height": -17}),
                "`-17` liegt nicht in \u{2212}16 bis 62.".to_owned(),
            ),
            // Map 2 lies in no matrix cell of the synthetic project.
            (
                json!({"map": 2}),
                "Karte 0002 liegt in keiner Matrixzelle mit Header; Tileset angeben \
                 (`tileset`)."
                    .to_owned(),
            ),
        ];
        for (req, expected) in cases {
            assert_eq!(
                refused(world_create(&mut store, &create_req(req.clone()))),
                expected
            );
            assert_eq!(world(&mut store, 0), None, "{req}");
            assert!(store.history().0.is_empty(), "{req}");
        }
        // A package past the archive: the refusal of `Store::tileset`.
        let text = refused(world_create(
            &mut store,
            &create_req(json!({"map": 0, "tileset": 200})),
        ));
        assert!(
            text.starts_with("Kartentextur-Paket 0200 gibt es nicht ("),
            "{text}"
        );
        create(&mut store, 0);
        assert_eq!(
            refused(world_create(
                &mut store,
                &create_req(json!({"map": 0, "tileset": 0}))
            )),
            "Karte 0000 ist schon eine Welt-Karte."
        );
        assert_eq!(store.history().0.len(), 1);
        assert_eq!(fx.read(MAP), file);
        // Height and fill of the caller's.
        let report = world_create(
            &mut store,
            &create_req(json!({"map": 1, "tileset": 0, "fill": "0001R2F", "height": -16})),
        )
        .unwrap();
        assert_eq!(report.map, 1);
        let one = world(&mut store, 1).unwrap();
        let grid = WorldGrid::parse(&one).unwrap();
        assert!(grid.heights.iter().all(|&h| h == -16));
        assert_eq!(one.layers[0].tiles[0].split(' ').next(), Some("0001r2f"));
    }

    #[test]
    fn create_refuses_a_locked_map() {
        let fx = TextFixture::exported();
        put_tileset(&fx, 0);
        // A member that does not decode: the map file is `{locked, raw}`.
        let locked = to_canonical(&json!({
            "locked": "map 0000: does not decode",
            "raw": {"hex": "00"}
        }));
        fx.write(MAP, &locked);
        let mut store = store(&fx);
        assert_eq!(
            refused(world_create(
                &mut store,
                &create_req(json!({"map": 0, "tileset": 0}))
            )),
            "Karte 0000 lässt sich nicht als Welt anlegen: ihr Member lässt sich nicht lesen \
             (data truncated: needed 4 bytes, have 1)."
        );
        assert!(store.history().0.is_empty());
        assert_eq!(fx.read(MAP), locked);
    }

    #[test]
    fn world_edit_refuses_a_broken_world_unchanged() {
        let (fx, _) = fixture();
        let mut broken = blank();
        broken.layers[0].tiles.pop();
        broken.layers[0].heights.as_mut().unwrap()[4] = vec!["70"; 32].join(" ");
        let file = world_parts(fx.read(MAP), &broken, &mut Vec::new()).unwrap();
        fx.write(MAP, &file);
        let mut store = store(&fx);
        assert_eq!(world(&mut store, 0), Some(broken.clone()));
        let text = refused(edit(
            &mut store,
            json!({"map": 0, "changes": [{"cells": [[0, 0]], "tile": "0001"}]}),
        ));
        assert_eq!(text, "world.layers[0].tiles: 31 Zeilen statt 32.");
        assert_eq!(world(&mut store, 0), Some(broken));
        assert!(store.history().0.is_empty());
        assert!(store.dirty().is_empty());
        store.save().unwrap();
        assert_eq!(fx.read(MAP), file);
        // A world that does not read at all is the JSON's error, with file and path.
        let mut v: Value = serde_json::from_slice(&file).unwrap();
        v["world"]["layers"][0]["tile"] = json!([]);
        fx.write(MAP, &to_canonical(&v));
        let mut store = store_of(&fx);
        match store.world(0) {
            Err(KernelError::Compile(problems)) => {
                assert_eq!(problems[0].file, std::path::PathBuf::from(MAP));
                assert!(
                    problems[0]
                        .message
                        .starts_with("textproject.schema: world.layers[0].tile: unknown field"),
                    "{problems:?}"
                );
            }
            other => panic!("{other:?}"),
        }
        // The map itself does not read then either (`get_map` answers with the error).
        assert!(
            matches!(store.map(0), Err(KernelError::Compile(_))),
            "{:?}",
            store.map(0).err()
        );
    }

    #[test]
    fn world_edit_refuses_a_tile_past_the_tileset_unchanged() {
        let (fx, _) = fixture();
        let mut broken = blank();
        // Hand-edited: cell (3, 2) names tile 0x0120, the synthetic tileset has six.
        let mut row: Vec<&str> = vec!["0000"; 32];
        row[3] = "0120";
        broken.layers[0].tiles[2] = row.join(" ");
        let file = world_parts(fx.read(MAP), &broken, &mut Vec::new()).unwrap();
        fx.write(MAP, &file);
        let mut store = store(&fx);
        assert_eq!(world(&mut store, 0), Some(broken.clone()));
        let text = refused(edit(
            &mut store,
            json!({"map": 0, "changes": [{"cells": [[0, 0]], "tile": "0001"}]}),
        ));
        assert_eq!(
            text,
            "world.layers[0].tiles[2]: Zelle 3: Kachel 0x0120 gibt es im Tileset 0000 nicht \
             (6 Kacheln)."
        );
        assert_eq!(world(&mut store, 0), Some(broken));
        assert!(store.history().0.is_empty());
        assert!(store.dirty().is_empty());
        store.save().unwrap();
        assert_eq!(fx.read(MAP), file);
    }

    #[test]
    fn dspre_refuses_with_the_way() {
        let fx = TextFixture::dspre();
        put_tileset(&fx, 0);
        let mut store = store(&fx);
        let before = fx.read(&format!(
            "unpacked/{}/0000",
            fx.project().archives.unpacked_name(AssetKind::Map)
        ));
        assert_eq!(
            refused(world_create(
                &mut store,
                &create_req(json!({"map": 0, "tileset": 0}))
            )),
            WORLD_NEEDS_TEXT_PROJECT
        );
        assert_eq!(
            refused(edit(
                &mut store,
                json!({"map": 0, "changes": [{"cells": [[0, 0]], "tile": "0001"}]})
            )),
            WORLD_NEEDS_TEXT_PROJECT
        );
        assert_eq!(refused(store.world(0)), WORLD_NEEDS_TEXT_PROJECT);
        assert!(store.history().0.is_empty());
        assert!(!fx.root().join("data/maps").exists());
        assert_eq!(
            fx.read(&format!(
                "unpacked/{}/0000",
                fx.project().archives.unpacked_name(AssetKind::Map)
            )),
            before
        );
        // A text project of a newer schema reads from its members and paints nothing.
        let fx = TextFixture::exported();
        put_tileset(&fx, 0);
        let manifest = crate::textproject::MANIFEST_FILE;
        let mut v: Value = serde_json::from_slice(&fx.read(manifest)).unwrap();
        v["schemaVersion"] = json!(crate::textproject::SCHEMA_VERSION + 1);
        fx.write(manifest, &to_canonical(&v));
        let mut store = store_of(&fx);
        assert_eq!(
            refused(world_create(
                &mut store,
                &create_req(json!({"map": 0, "tileset": 0}))
            )),
            WORLD_NEEDS_TEXT_PROJECT
        );
        assert_eq!(refused(store.world(0)), WORLD_NEEDS_TEXT_PROJECT);
    }

    #[test]
    fn create_takes_the_package_of_the_header() {
        let fx = TextFixture::exported();
        let mut store = store(&fx);
        let packs = packs_of_map(&mut store, 0).unwrap();
        let package = *packs.first().expect("map 0 is shown by a header");
        put_tileset(&fx, package);
        let report = world_create(&mut store, &create_req(json!({"map": 0}))).unwrap();
        assert_eq!(report.tileset, package);
        assert_eq!(world(&mut store, 0).unwrap().tileset, package);
    }

    #[test]
    fn a_world_map_diffs_as_its_map() {
        use crate::diff::{Against, DiffSources, DiffState, DiffSubject, EntityDiff, diff_entity};
        use crate::original::ChangeState;
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        let entries = crate::diff::unsaved_entries(&mut store).unwrap();
        let rows: Vec<(ChangeState, &str, &DiffSubject)> = entries
            .iter()
            .map(|e| (e.state, e.label.as_str(), &e.subject))
            .collect();
        assert_eq!(
            rows,
            [(
                ChangeState::Changed,
                "Karte 0000",
                &DiffSubject::Map { index: 0 }
            )]
        );
        let project = fx.project();
        let sources = DiffSources {
            original: None,
            base_label: None,
            repo: None,
        };
        let subject = DiffSubject::Map { index: 0 };
        let report = diff_entity(
            &project,
            Some(&mut store),
            &sources,
            &subject,
            Against::Saved,
        )
        .unwrap();
        assert_eq!(report.state, DiffState::Changed);
        let EntityDiff::Map { parts, .. } = &report.diff else {
            panic!("{report:?}");
        };
        let world = parts.iter().find(|p| p.part == "world").unwrap();
        assert!(
            world.changed && world.old.is_none() && world.new.is_some(),
            "{world:?}"
        );
        // The derived parts leave the file of a world map (schema 4); the others stay.
        for p in parts.iter().filter(|p| p.part != "world") {
            let derived = ["model", "permissions", "bdhc"].contains(&p.part.as_str());
            assert_eq!(p.changed, derived, "{p:?}");
            assert_eq!(p.new.is_none(), derived, "{p:?}");
        }
        store.save().unwrap();
        assert!(crate::diff::unsaved_entries(&mut store).unwrap().is_empty());
        let report = diff_entity(
            &project,
            Some(&mut store),
            &sources,
            &subject,
            Against::Saved,
        )
        .unwrap();
        assert_eq!(report.state, DiffState::Same);
    }

    #[test]
    fn a_saved_world_diffs_as_added_against_the_original() {
        use crate::diff::{Against, DiffSources, DiffState, DiffSubject, EntityDiff, diff_entity};
        use crate::test_support::ProjectOriginal;
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        store.save().unwrap();
        let project = fx.project();
        let base = ProjectOriginal::new(&fx.root().join(".g4/cache/base-layer").join(fx.sha1()));
        let sources = DiffSources {
            original: Some(&base),
            base_label: None,
            repo: None,
        };
        let subject = DiffSubject::Map { index: 0 };
        let report = diff_entity(&project, None, &sources, &subject, Against::Base).unwrap();
        assert_eq!(report.state, DiffState::Changed, "{report:?}");
        let EntityDiff::Map { parts, .. } = &report.diff else {
            panic!("{report:?}");
        };
        let world = parts.iter().find(|p| p.part == "world").unwrap();
        assert!(
            world.changed && world.old.is_none() && world.new.is_some(),
            "{world:?}"
        );
        // Map 1 has no world: the same as the original.
        let other = DiffSubject::Map { index: 1 };
        let report = diff_entity(&project, None, &sources, &other, Against::Base).unwrap();
        assert_eq!(report.state, DiffState::Same, "{report:?}");
    }

    #[test]
    fn a_freed_map_number_takes_its_world_with_it() {
        use crate::data::refs::RefIndex;
        use crate::delete::{DeleteTarget, delete, free_number};
        use crate::test_support::ProjectOriginal;
        let (fx, mut store) = fixture();
        // The base counts the packed maps: the copy at the end is new.
        let base = ProjectOriginal::new(fx.root());
        let n = store.duplicate_map(0).unwrap();
        create(&mut store, n);
        store.save().unwrap();
        let rel = format!("data/maps/{n:04}.json");
        assert_eq!(
            serde_json::from_slice::<Value>(&fx.read(&rel)).unwrap()["source"],
            "world"
        );
        let mut refs = RefIndex::new();
        let target = DeleteTarget::Map { index: n };
        delete(&mut store, &mut refs, &base, target).unwrap();
        let result = free_number(&mut store, &mut refs, &base, target).unwrap();
        assert!(result.freed.is_some());
        assert!(!fx.root().join(&rel).exists());
        // The next map takes the number, without the old world.
        assert_eq!(store.duplicate_map(0).unwrap(), n);
        assert_eq!(world(&mut store, n), None);
        let world_id = AssetId {
            kind: AssetKind::WorldMap,
            index: n,
        };
        assert!(store.locked(world_id).is_none());
        store
            .edit_map(n, "Permission gemalt", |m| {
                m.permissions[0].collision ^= 0x80;
                Ok(())
            })
            .unwrap();
        store.save().unwrap();
        let v: Value = serde_json::from_slice(&fx.read(&rel)).unwrap();
        assert_eq!(v["source"], "binary", "{v}");
        assert!(v.get("world").is_none_or(Value::is_null), "{v}");
        assert_eq!(world(&mut store, n), None);
    }

    #[test]
    fn world_edit_refuses_a_locked_map() {
        let (fx, mut store) = fixture();
        create(&mut store, 0);
        store.save().unwrap();
        let file = fx.read(MAP);
        store
            .lock(
                AssetId {
                    kind: AssetKind::Map,
                    index: 0,
                },
                "gesperrt",
            )
            .unwrap();
        let before = world(&mut store, 0);
        let history = store.history();
        assert_eq!(
            refused(edit(
                &mut store,
                json!({"map": 0, "changes": [{"cells": [[0, 0]], "tile": "0001"}]})
            )),
            "textproject.map-source: Karte 0000 ist eine Welt-Karte, ihr Member lässt sich \
             nicht lesen; Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder \
             die Welt aus der Datei sichern)."
        );
        assert_eq!(world(&mut store, 0), before);
        assert_eq!(store.history(), history);
        assert!(store.dirty().is_empty());
        assert_eq!(fx.read(MAP), file);
    }

    #[test]
    fn a_hand_written_world_survives_a_map_save() {
        let (fx, _) = fixture();
        let mut v: Value =
            serde_json::from_slice(&world_parts(fx.read(MAP), &blank(), &mut Vec::new()).unwrap())
                .unwrap();
        // Not as the store writes it: a piece without `rotate`, an empty `freeform`.
        v["world"]["pieces"] = json!([{"tile": 5, "at": [3, 3]}]);
        v["world"]["freeform"] = json!([]);
        let hand = to_canonical(&v);
        fx.write(MAP, &hand);
        let mut store = store_of(&fx);
        // Loaded and unchanged in the store.
        assert_eq!(world(&mut store, 0).unwrap().pieces[0].rotate, 0);
        move_building(&mut store);
        store.save().unwrap();
        let now: Value = serde_json::from_slice(&fx.read(MAP)).unwrap();
        assert_eq!(now["world"], v["world"]);
        // The world as written: the piece without `rotate`, the empty `freeform`.
        assert!(now["world"]["pieces"][0].get("rotate").is_none(), "{now}");
        assert_ne!(fx.read(MAP), hand);
    }

    #[test]
    fn a_refused_world_read_unpacks_nothing() {
        let fx = TextFixture::dspre();
        let maps = fx.project().unpacked_dir(AssetKind::Map);
        std::fs::remove_dir_all(&maps).unwrap();
        let mut store = store(&fx);
        assert_eq!(refused(store.world(0)), WORLD_NEEDS_TEXT_PROJECT);
        assert!(!maps.exists());
    }
}

/// `world_layers`: add an overlay named `name` or remove overlay `layer`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum LayerOp {
    Add { name: String },
    Remove { layer: u8 },
}

/// `world_layers`: the map and what to do with its layers.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLayersRequest {
    pub map: u16,
    pub op: LayerOp,
}

/// The ground layer cannot go.
const GROUND_STAYS: &str = "Die Bodenebene (0) bleibt.";
/// A layer name that is empty after trimming.
const LAYER_NEEDS_NAME: &str = "Eine Ebene braucht einen Namen.";

/// „Ebene „Büsche“ gibt es schon.“
fn layer_taken(name: &str) -> String {
    format!("Ebene „{name}“ gibt es schon.")
}

/// One undo step: `Add` appends an overlay (role `overlay`, 32 rows of `.`) named `name`
/// (trimmed), label „Ebene „Büsche“ angelegt: Karte 0123“; `Remove` drops overlay `layer`, label
/// „Ebene 2 „Büsche“ entfernt: Karte 0123“.
///
/// Refused ([`KernelError::Refused`], nothing changed) as [`world_edit`] (outside a text
/// project, a map without world, a locked map, a world that does not read into cells) and with
/// „Die Bodenebene (0) bleibt.“, „Eine Ebene braucht einen Namen.“, „Ebene „Büsche“ gibt es
/// schon.“ (names compared trimmed, case-sensitive) and a layer the world lacks
/// ([`no_layer`]). [`WorldReport::cells`] is 0.
pub fn world_layers(
    store: &mut Store,
    req: &WorldLayersRequest,
) -> Result<WorldReport, KernelError> {
    require_text_project(store)?;
    let map = req.map;
    let mut world = store
        .world(map)?
        .cloned()
        .ok_or_else(|| refused(not_a_world_map(map)))?;
    refuse_locked_map(store, map, |_| locked_map_refusal(map))?;
    first_error(&world)?;
    let label = match &req.op {
        LayerOp::Add { name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err(refused(LAYER_NEEDS_NAME.to_owned()));
            }
            if world.layers.iter().any(|l| l.name.trim() == name) {
                return Err(refused(layer_taken(name)));
            }
            world.layers.push(super::doc::LayerJson {
                name: name.to_owned(),
                role: super::doc::LayerRole::Overlay,
                tiles: vec![vec![EMPTY; MAP_CELLS].join(" "); MAP_CELLS],
                heights: None,
                ramps: None,
            });
            format!("Ebene „{name}“ angelegt: Karte {map:04}")
        }
        LayerOp::Remove { layer } => {
            if *layer == 0 {
                return Err(refused(GROUND_STAYS.to_owned()));
            }
            let at = usize::from(*layer);
            if at >= world.layers.len() {
                return Err(refused(no_layer(*layer, map, world.layers.len())));
            }
            let removed = world.layers.remove(at);
            format!("Ebene {layer} „{}“ entfernt: Karte {map:04}", removed.name)
        }
    };
    first_error(&world)?;
    let package = world.tileset;
    let (bake, change) = store.transaction(&label, |tx| {
        tx.edit_world(map, |w| {
            *w = Some(world);
            Ok(())
        })?;
        Ok(tx.bake_report(map))
    })?;
    Ok(WorldReport {
        map,
        tileset: package,
        cells: 0,
        change,
        bake,
    })
}

#[cfg(test)]
mod layer_tests {
    use std::sync::Arc;

    use serde_json::{Value, json};

    use super::*;
    use crate::test_support::TextFixture;
    use crate::textproject::canon::to_canonical;
    use crate::world::doc::LayerRole;
    use crate::world::grid::WorldGrid;
    use crate::world::tileset::{Tileset, tileset_rel};

    /// A text project with the synthetic tileset as package 0, its store and the world of map 0.
    fn fixture() -> (TextFixture, Store) {
        let fx = TextFixture::exported();
        let tileset = Tileset {
            package: 0,
            ..Tileset::synthetic()
        };
        fx.write(tileset_rel(0).to_str().unwrap(), &to_canonical(&tileset));
        let mut store = Store::new(Arc::new(fx.project()));
        let req = serde_json::from_value(json!({"map": 0, "tileset": 0})).unwrap();
        world_create(&mut store, &req).unwrap();
        (fx, store)
    }

    fn layers(store: &mut Store, value: Value) -> Result<WorldReport, KernelError> {
        world_layers(store, &serde_json::from_value(value).unwrap())
    }

    fn world(store: &mut Store) -> WorldJson {
        store.world(0).unwrap().cloned().unwrap()
    }

    fn refused<T: std::fmt::Debug>(r: Result<T, KernelError>) -> String {
        match r {
            Err(KernelError::Refused(text)) => text,
            other => panic!("{other:?}"),
        }
    }

    /// Refused with `text`, nothing changed.
    fn refuses(store: &mut Store, value: Value, text: &str) {
        let before = world(store);
        let history = store.history();
        assert_eq!(refused(layers(store, value)), text);
        assert_eq!(world(store), before);
        assert_eq!(store.history(), history);
    }

    #[test]
    fn adding_a_layer_is_one_step() {
        let (_fx, mut store) = fixture();
        let before = world(&mut store);
        let report = layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "  Büsche "}}}),
        )
        .unwrap();
        assert_eq!((report.map, report.tileset, report.cells), (0, 0, 0));
        assert!(report.change.is_some());
        let now = world(&mut store);
        assert_eq!(now.layers.len(), 2);
        let added = &now.layers[1];
        assert_eq!(added.name, "Büsche");
        assert_eq!(added.role, LayerRole::Overlay);
        assert_eq!(added.tiles, vec![vec!["."; 32].join(" "); 32]);
        assert_eq!((&added.heights, &added.ramps), (&None, &None));
        assert_eq!(now.layers[0], before.layers[0]);
        assert_eq!(
            store.history().0,
            [
                "Welt angelegt: Karte 0000",
                "Ebene „Büsche“ angelegt: Karte 0000"
            ]
        );
        store.undo().unwrap();
        assert_eq!(world(&mut store).layers.len(), 1);
        assert_eq!(world(&mut store), before);
        store.redo().unwrap();
        assert_eq!(world(&mut store), now);
    }

    #[test]
    fn removing_an_overlay_is_one_step() {
        let (_fx, mut store) = fixture();
        let before = world(&mut store);
        layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "Büsche"}}}),
        )
        .unwrap();
        layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "Zaun"}}}),
        )
        .unwrap();
        let two = world(&mut store);
        let report = layers(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 1}}}),
        )
        .unwrap();
        assert_eq!((report.map, report.tileset, report.cells), (0, 0, 0));
        assert!(report.change.is_some());
        let now = world(&mut store);
        let names: Vec<&str> = now.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Boden", "Zaun"]);
        assert_eq!(
            store.history().0.last().unwrap(),
            "Ebene 1 „Büsche“ entfernt: Karte 0000"
        );
        store.undo().unwrap();
        assert_eq!(world(&mut store), two);
        layers(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 2}}}),
        )
        .unwrap();
        layers(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 1}}}),
        )
        .unwrap();
        assert_eq!(world(&mut store), before);
    }

    #[test]
    fn the_ground_stays() {
        let (_fx, mut store) = fixture();
        refuses(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 0}}}),
            "Die Bodenebene (0) bleibt.",
        );
    }

    #[test]
    fn a_blank_or_taken_name_is_refused() {
        let (_fx, mut store) = fixture();
        refuses(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "   "}}}),
            "Eine Ebene braucht einen Namen.",
        );
        layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "Büsche"}}}),
        )
        .unwrap();
        refuses(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": " Büsche"}}}),
            "Ebene „Büsche“ gibt es schon.",
        );
        refuses(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "Boden"}}}),
            "Ebene „Boden“ gibt es schon.",
        );
        // Names differ by case.
        layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "büsche"}}}),
        )
        .unwrap();
        // Unknown fields are refused by serde.
        assert!(
            serde_json::from_value::<WorldLayersRequest>(
                json!({"map": 0, "op": {"add": {"name": "A", "role": "ground"}}})
            )
            .is_err()
        );
    }

    #[test]
    fn a_missing_layer_is_refused() {
        let (_fx, mut store) = fixture();
        refuses(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 1}}}),
            "Ebene 1 gibt es nicht (Karte 0000 hat 1).",
        );
        refuses(
            &mut store,
            json!({"map": 0, "op": {"remove": {"layer": 7}}}),
            "Ebene 7 gibt es nicht (Karte 0000 hat 1).",
        );
    }

    #[test]
    fn layers_need_the_text_project() {
        let fx = TextFixture::dspre();
        let mut store = Store::new(Arc::new(fx.project()));
        let add = json!({"map": 0, "op": {"add": {"name": "Büsche"}}});
        assert_eq!(
            refused(layers(&mut store, add.clone())),
            WORLD_NEEDS_TEXT_PROJECT
        );
        assert!(store.history().0.is_empty());
        // A map without world.
        let (_fx, mut store) = fixture();
        assert_eq!(
            refused(layers(
                &mut store,
                json!({"map": 1, "op": {"add": {"name": "Büsche"}}})
            )),
            not_a_world_map(1)
        );
        // A locked map.
        store
            .lock(
                crate::project::AssetId {
                    kind: crate::project::AssetKind::Map,
                    index: 0,
                },
                "gesperrt",
            )
            .unwrap();
        refuses(&mut store, add, &super::super::keep::locked_map_refusal(0));
    }

    #[test]
    fn painting_on_an_added_layer() {
        let (_fx, mut store) = fixture();
        layers(
            &mut store,
            json!({"map": 0, "op": {"add": {"name": "Büsche"}}}),
        )
        .unwrap();
        let req = serde_json::from_value(
            json!({"map": 0, "changes": [{"layer": 1, "cells": [[2, 5]], "tile": "0003"}]}),
        )
        .unwrap();
        world_edit(&mut store, &req).unwrap();
        let grid = WorldGrid::parse(&world(&mut store)).unwrap();
        assert_eq!(grid.layers[1].tiles[5 * 32 + 2].unwrap().tile, 3);
        assert_eq!(grid.layers[1].tiles[5 * 32 + 3], None);
        assert_eq!(store.history().0.len(), 3);
        store.undo().unwrap();
        store.undo().unwrap();
        assert_eq!(world(&mut store).layers.len(), 1);
    }
}
