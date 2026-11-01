//! The door helpers „Raum anlegen“ and „Tür verbinden“ share: warps of an event file, the
//! event file and the matrix of a header, whether the game takes a warp on a tile, door tiles
//! and the tile of a map under a tile of a matrix. „Neue Figur“ uses [`outside_map`] for the
//! matrix of its draft.

use g4_formats::game::Family;
use g4_formats::map::{Cell, MAP_SIZE};
use g4_formats::matrix::MATRIX_EMPTY;

use super::room::DoorTile;
use super::types::refused;
use crate::error::KernelError;
use crate::store::{MapHeader, Store, Tx, WarpEvent};
use crate::tiles::{DOOR, DOOR_COLLISION, warp_fires};

/// Number of warps in event file `index` as a warp index.
pub(crate) fn warp_count(store: &mut Store, index: u16) -> Result<u16, KernelError> {
    let count = store.event(index)?.warp_events.len();
    u16::try_from(count).map_err(|_| refused(format!("event file {index:04} has too many warps")))
}

/// The first warp of event file `file` on tile (`x`, `z`): the one the game takes there.
pub(crate) fn warp_at(
    store: &mut Store,
    file: u16,
    x: u16,
    z: u16,
) -> Result<Option<u16>, KernelError> {
    let found = store
        .event(file)?
        .warp_events
        .iter()
        .position(|w| w.x == x && w.z == z);
    found
        .map(|i| {
            u16::try_from(i)
                .map_err(|_| refused(format!("event file {file:04} has too many warps")))
        })
        .transpose()
}

/// The event file `header` has now.
pub(crate) fn header_event_file(store: &mut Store, header: u16) -> Result<u16, KernelError> {
    store
        .header_table()?
        .get(usize::from(header))
        .map(MapHeader::event_file_id)
        .ok_or_else(|| refused(format!("header {header} does not exist")))
}

/// The matrix `header` uses and the map under tile (`x`, `z`) of it. Refuses a tile outside
/// that matrix, in a cell without a map, and in a cell the matrix gives to another header: the
/// game loads a header's warps only while the player is in one of its cells.
pub(crate) fn outside_map(
    store: &mut Store,
    header: u16,
    x: u16,
    z: u16,
) -> Result<(u16, u16), KernelError> {
    let matrix_id = store
        .header_table()?
        .get(usize::from(header))
        .map(MapHeader::matrix_id)
        .ok_or_else(|| refused(format!("header {header} does not exist")))?;
    let m = store.matrix(matrix_id)?;
    let (cx, cy) = (usize::from(x) / MAP_SIZE, usize::from(z) / MAP_SIZE);
    let (width, height) = (usize::from(m.width), usize::from(m.height));
    if cx >= width || cy >= height {
        return Err(refused(format!(
            "tile ({x}, {z}) lies outside matrix {matrix_id:04} ({} x {} tiles)",
            width * MAP_SIZE,
            height * MAP_SIZE
        )));
    }
    let i = cy * width + cx;
    let map = m.maps[i];
    if map == MATRIX_EMPTY {
        return Err(refused(format!(
            "tile ({x}, {z}) lies in cell ({cx}, {cy}) of matrix {matrix_id:04}, which has no map"
        )));
    }
    if let Some(owner) = m.headers.as_ref().map(|h| h[i])
        && owner != header
    {
        return Err(refused(format!(
            "tile ({x}, {z}) lies in cell ({cx}, {cy}) of header {owner:04}, not of header {header:04}"
        )));
    }
    Ok((matrix_id, map))
}

/// Whether the game would take a warp on tile (`x`, `y`) of `map`.
pub(crate) fn fires(
    store: &mut Store,
    family: Family,
    map: u16,
    x: u16,
    y: u16,
) -> Result<bool, KernelError> {
    let cell = store.map(map)?.cell(usize::from(x), usize::from(y))?;
    Ok(warp_fires(family, cell.kind, cell.collision))
}

/// Makes tile (`x`, `y`) of `map` a door the player bumps into: an edit of the recipe's one
/// transaction.
pub(crate) fn set_door(
    tx: &mut Tx<'_>,
    map: u16,
    (x, y): (u16, u16),
) -> Result<DoorTile, KernelError> {
    let (x, y) = (
        u8::try_from(x).unwrap_or(u8::MAX),
        u8::try_from(y).unwrap_or(u8::MAX),
    );
    let door = Cell {
        kind: DOOR,
        collision: DOOR_COLLISION,
    };
    tx.edit_map(map, |m| m.set_cell(usize::from(x), usize::from(y), door))?;
    Ok(DoorTile { map, x, y })
}

/// The tile of a map that tile `n` of a matrix lies on.
pub(crate) fn local(n: u16) -> u16 {
    n % MAP_SIZE as u16
}

pub(crate) fn warp(x: u16, z: u16, dest_header_id: u16, dest_warp_id: u16) -> WarpEvent {
    WarpEvent {
        x,
        z,
        dest_header_id,
        dest_warp_id,
        height: 0,
    }
}
