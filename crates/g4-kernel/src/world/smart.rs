//! Smart drawing (phase 6.4b): the terrain brush resolves the corners it paints into the
//! places of an edge set ([`paint_terrain`]); the cliff brush ([`paint_cliff`]) raises or lowers
//! cells and puts the set's rim on top. Both and `world_edit` build on the same parts: the seam
//! cells read from neighbour maps ([`Seams`]), the result of a smart change ([`Resolved`]), the
//! corner grid ([`Corners`]) and the set read as a table token ⇄ place ([`SetTable`]).
//!
//! Gen 4 maps keep no corner colours; the tiles carry them (rule „Eckfarben lesen“): a cell is
//! a **member** of the set when its token stands in a place (the place's mask), **outside**
//! when it is the set's `outside` (mask 0), else **foreign** (reads nothing). A corner is
//! inside when at least one of the readable cells around it says so. The terrain brush (rule
//! „Terrain-Pinsel“) sets the four corners of each painted cell inside (⇧: outside) and gives
//! every cell that touches a changed corner the tile of its new mask; cells whose mask stays
//! keep their tile (no new roll), foreign cells with the new mask 0 stay. The corner and fill
//! logic is written for gen4-studio; the Wang-corner idea follows Tiled's terrain brush (no
//! code).

use std::collections::{BTreeMap, BTreeSet};

use g4_formats::game::Family;
use g4_formats::map::Cell;

use super::bake::tile_cells;
use super::edge::{EdgeBlock, EdgeSetJson, NE, NW, Place, SE, SW, variant};
use super::grid::{Ramp, TileRef, WorldGrid, override_token, parse_tile, tile_token};
use super::tileset::Tileset;
use crate::terrain::high_edges;

/// Cells and rows of a map.
const CELLS: i32 = 32;
/// The corners of the grid per row: −1 to 33, the map's 33 and the seam's outer ones.
const SPAN: i32 = CELLS + 3;

/// Cells just past the map's edge (x or z −1 or 32) read from neighbour world maps: tiles per layer index and ground
/// heights; a cell that is not there reads nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seams {
    pub tiles: BTreeMap<(usize, i8, i8), TileRef>,
    pub heights: BTreeMap<(i8, i8), i8>,
    /// The neighbour map of each seam cell, for the notes.
    pub owner: BTreeMap<(i8, i8), u16>,
}

impl Seams {
    /// The tile of seam cell (x, z) on `layer`; `None` inside the map, past the seam ring or
    /// where no neighbour is read.
    pub fn tile(&self, layer: usize, x: i32, z: i32) -> Option<TileRef> {
        let (x, z) = seam_key(x, z)?;
        self.tiles.get(&(layer, x, z)).copied()
    }
}

/// The key of seam cell (x, z): both in −1 to 32, one of them −1 or 32.
fn seam_key(x: i32, z: i32) -> Option<(i8, i8)> {
    let ring = |v: i32| v == -1 || v == CELLS;
    let near = |v: i32| (-1..=CELLS).contains(&v);
    (near(x) && near(z) && (ring(x) || ring(z))).then_some((x as i8, z as i8))
}

/// What a smart change resolves to (cells map-local, each once, row-major).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved {
    pub tiles: Vec<([u8; 2], String)>,
    pub heights: Vec<([u8; 2], i8)>,
    pub ramps_cleared: Vec<[u8; 2]>,
    pub overrides: Vec<([u8; 2], String)>,
    pub notes: Vec<String>,
}

/// „Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).“
pub(crate) fn cell_outside(x: u8, z: u8) -> String {
    format!(
        "Zelle ({x}, {z}) liegt außerhalb der Karte (0 bis {}).",
        CELLS - 1
    )
}

/// „Ebene 2 gibt es nicht (die Karte hat 1).“
fn no_layer(layer: usize, layers: usize) -> String {
    format!("Ebene {layer} gibt es nicht (die Karte hat {layers}).")
}

/// „Kantensatz „Weg“: der Platz „Ecke innen NW“ hat keine Kachel (Zelle (6, 7)).“ (the
/// outside tile is named „Außen“).
pub(crate) fn missing_place(set: &str, place: &str, x: i32, z: i32) -> String {
    format!("Kantensatz „{set}“: der Platz „{place}“ hat keine Kachel (Zelle ({x}, {z})).")
}

/// „Naht zu Karte 0124: 3 Zellen dort passen nicht mehr zum Satz „Weg“; die Nachbarkarte
/// bleibt, wie sie ist.“ (one cell: „1 Zelle dort passt …“).
pub(crate) fn seam_note(map: u16, cells: usize, set: &str) -> String {
    let (noun, verb) = if cells == 1 {
        ("Zelle", "passt")
    } else {
        ("Zellen", "passen")
    };
    format!(
        "Naht zu Karte {map:04}: {cells} {noun} dort {verb} nicht mehr zum Satz „{set}“; die \
         Nachbarkarte bleibt, wie sie ist."
    )
}

/// An edge set read for the brushes: each token of the set (parsed, so case and spelling do
/// not matter) with its mask, and the entries of each mask (0 = `outside`). Tokens that do not
/// parse (a hand-broken file, a schema problem of its own) are left out and read as foreign.
#[derive(Debug, Clone)]
pub(crate) struct SetTable<'a> {
    pub set: &'a EdgeSetJson,
    /// The entries per mask, index 0 the outside tile.
    entries: [Vec<TileRef>; 16],
}

impl<'a> SetTable<'a> {
    pub fn new(set: &'a EdgeSetJson) -> SetTable<'a> {
        let mut entries: [Vec<TileRef>; 16] = Default::default();
        let mut push = |mask: u8, token: &str| {
            if let (Ok(Some(t)), Some(list)) =
                (parse_tile(token), entries.get_mut(usize::from(mask)))
            {
                list.push(t);
            }
        };
        push(0, &set.outside);
        for place in Place::ALL {
            for token in set.places.get(place) {
                push(place.mask(), token);
            }
        }
        SetTable { set, entries }
    }

    /// The entries of `mask` (0 = `outside`); none past 15.
    fn entries(&self, mask: u8) -> &[TileRef] {
        self.entries
            .get(usize::from(mask))
            .map_or(&[], Vec::as_slice)
    }

    /// The mask a tile reads as: the mask of its place, 0 for `outside`, `None` for a foreign
    /// tile (and `.`). The first match wins should a broken set name a token twice.
    pub fn read(&self, tile: Option<TileRef>) -> Option<u8> {
        let tile = tile?;
        (0..16u8).find(|&m| self.entries(m).contains(&tile))
    }

    /// The canonical token for `mask` at cell (x, z) with stroke seed `seed` (rule „Zufall“);
    /// Err: the sentence of the empty place.
    pub fn pick(&self, mask: u8, x: i32, z: i32, seed: u32) -> Result<String, String> {
        let entries = self.entries(mask);
        if entries.is_empty() {
            let place = Place::of_mask(mask).map_or("Außen", Place::label);
            return Err(missing_place(&self.set.name, place, x, z));
        }
        let cell = (z * CELLS + x) as usize;
        let index = variant(seed, cell, entries.len());
        Ok(tile_token(entries.get(index).copied()))
    }
}

/// The corner colours of a map and its seam ring: corners −1 to 33 in x and z (cell (x, z)
/// has the corners (x, z) NW, (x + 1, z) NE, (x, z + 1) SW, (x + 1, z + 1) SE), true =
/// inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Corners {
    inside: Vec<bool>,
}

impl Corners {
    /// All corners outside.
    pub fn outside() -> Corners {
        Corners {
            inside: vec![false; (SPAN * SPAN) as usize],
        }
    }

    fn index(cx: i32, cz: i32) -> Option<usize> {
        let range = -1..=CELLS + 1;
        (range.contains(&cx) && range.contains(&cz)).then(|| ((cz + 1) * SPAN + cx + 1) as usize)
    }

    /// Corner (cx, cz); outside past the grid.
    pub fn get(&self, cx: i32, cz: i32) -> bool {
        Corners::index(cx, cz)
            .and_then(|i| self.inside.get(i))
            .is_some_and(|&inside| inside)
    }

    /// Sets corner (cx, cz); true if it changed. Past the grid nothing happens.
    pub fn set(&mut self, cx: i32, cz: i32, inside: bool) -> bool {
        match Corners::index(cx, cz).and_then(|i| self.inside.get_mut(i)) {
            Some(corner) if *corner != inside => {
                *corner = inside;
                true
            }
            _ => false,
        }
    }

    /// The four corners of cell (x, z) as (cx, cz, bit).
    pub fn of_cell(x: i32, z: i32) -> [(i32, i32, u8); 4] {
        [
            (x, z, NW),
            (x + 1, z, NE),
            (x, z + 1, SW),
            (x + 1, z + 1, SE),
        ]
    }

    /// The mask of cell (x, z).
    pub fn mask(&self, x: i32, z: i32) -> u8 {
        Corners::of_cell(x, z)
            .into_iter()
            .filter(|&(cx, cz, _)| self.get(cx, cz))
            .fold(0, |m, (_, _, bit)| m | bit)
    }

    /// The corners `layer` reads with `table` (rule „Eckfarben lesen“): the map's cells and the
    /// seam cells, inside wins.
    pub fn read(grid: &WorldGrid, layer: usize, table: &SetTable, seams: &Seams) -> Corners {
        let mut corners = Corners::outside();
        for z in -1..=CELLS {
            for x in -1..=CELLS {
                let Some(mask) = table.read(cell_tile(grid, layer, seams, x, z)) else {
                    continue;
                };
                for (cx, cz, bit) in Corners::of_cell(x, z) {
                    if mask & bit != 0 {
                        corners.set(cx, cz, true);
                    }
                }
            }
        }
        corners
    }
}

/// The cells (x, z) of the map and the seam ring that touch corner (cx, cz).
pub(crate) fn cells_of_corner(cx: i32, cz: i32) -> [(i32, i32); 4] {
    [(cx - 1, cz - 1), (cx, cz - 1), (cx - 1, cz), (cx, cz)]
}

fn in_map(x: i32, z: i32) -> bool {
    (0..CELLS).contains(&x) && (0..CELLS).contains(&z)
}

/// The tile of cell (x, z) on `layer`: from the map inside it, from `seams` on the ring.
pub(crate) fn cell_tile(
    grid: &WorldGrid,
    layer: usize,
    seams: &Seams,
    x: i32,
    z: i32,
) -> Option<TileRef> {
    if in_map(x, z) {
        grid.layers
            .get(layer)?
            .tiles
            .get((z * CELLS + x) as usize)
            .copied()
            .flatten()
    } else {
        seams.tile(layer, x, z)
    }
}

/// The painted cells checked against the map and `layer`, as (x, z).
pub(crate) fn check_cells(
    grid: &WorldGrid,
    layer: usize,
    cells: &[[u8; 2]],
) -> Result<Vec<(i32, i32)>, String> {
    if layer >= grid.layers.len() {
        return Err(no_layer(layer, grid.layers.len()));
    }
    cells
        .iter()
        .map(|&[x, z]| {
            let (cx, cz) = (i32::from(x), i32::from(z));
            if in_map(cx, cz) {
                Ok((cx, cz))
            } else {
                Err(cell_outside(x, z))
            }
        })
        .collect()
}

/// Resolves the map cells `touched` (as (z, x), so row-major) on `layer` to the masks of
/// `corners` (rule „Terrain-Pinsel“): a readable cell whose mask stays keeps its tile, a
/// foreign cell with the new mask 0 stays, every other cell gets the tile of its new mask;
/// only cells whose token changes are returned. Err: the empty place of the first cell in row
/// order that needs one.
pub(crate) fn resolve_cells(
    grid: &WorldGrid,
    layer: usize,
    table: &SetTable,
    corners: &Corners,
    touched: &BTreeSet<(i32, i32)>,
    seed: u32,
) -> Result<Vec<([u8; 2], String)>, String> {
    let empty = Seams::default();
    let mut tiles = Vec::new();
    for &(z, x) in touched {
        if !in_map(x, z) {
            continue;
        }
        let current = cell_tile(grid, layer, &empty, x, z);
        let old = table.read(current);
        let new = corners.mask(x, z);
        if old == Some(new) || (old.is_none() && new == 0) {
            continue;
        }
        let token = table.pick(new, x, z, seed)?;
        if token != tile_token(current) {
            tiles.push(([x as u8, z as u8], token));
        }
    }
    Ok(tiles)
}

/// The notes of the seam: per neighbour map (ascending), the seam cells that fitted the
/// corners `before` (readable, own mask = the corners' mask) and would have another mask
/// `after`. Seam cells without an owner are not counted.
pub(crate) fn seam_notes(
    table: &SetTable,
    layer: usize,
    seams: &Seams,
    before: &Corners,
    after: &Corners,
) -> Vec<String> {
    let mut broken: BTreeMap<u16, usize> = BTreeMap::new();
    for (&(l, x, z), &tile) in &seams.tiles {
        if l != layer {
            continue;
        }
        let (x32, z32) = (i32::from(x), i32::from(z));
        let Some(own) = table.read(Some(tile)) else {
            continue;
        };
        if before.mask(x32, z32) == own
            && after.mask(x32, z32) != own
            && let Some(&map) = seams.owner.get(&(x, z))
        {
            *broken.entry(map).or_default() += 1;
        }
    }
    broken
        .into_iter()
        .map(|(map, cells)| seam_note(map, cells, &table.set.name))
        .collect()
}

/// The terrain brush on `layer`: the corners of `cells` set inside (`inside`) or outside, the touched cells resolved
/// (rules „Eckfarben lesen“, „Terrain-Pinsel“). Err: the sentence of the first place without tile.
pub fn paint_terrain(
    grid: &WorldGrid,
    layer: usize,
    set: &EdgeSetJson,
    cells: &[[u8; 2]],
    inside: bool,
    seed: u32,
    seams: &Seams,
) -> Result<Resolved, String> {
    let painted = check_cells(grid, layer, cells)?;
    let table = SetTable::new(set);
    let before = Corners::read(grid, layer, &table, seams);
    let mut after = before.clone();
    // The painted cells are resolved too: a foreign cell painted inside an area becomes part
    // of it even when no corner changes.
    let mut touched: BTreeSet<(i32, i32)> = painted.iter().map(|&(x, z)| (z, x)).collect();
    for &(x, z) in &painted {
        for (cx, cz, _) in Corners::of_cell(x, z) {
            if after.set(cx, cz, inside) {
                touched.extend(cells_of_corner(cx, cz).into_iter().map(|(x, z)| (z, x)));
            }
        }
    }
    let tiles = resolve_cells(grid, layer, &table, &after, &touched, seed)?;
    let notes = seam_notes(&table, layer, seams, &before, &after);
    Ok(Resolved {
        tiles,
        notes,
        ..Resolved::default()
    })
}

/// The offset (dx, dz) of a neighbour cell.
type Offset = (i32, i32);
/// A corner two cells share: its bit in the first cell's mask and in the other's.
type SharedCorner = (u8, u8);

/// Cells of `layer` whose corners disagree with a neighbour's (both members of `set` or `outside`), by the top-left
/// cell of each pair, sorted, without repeats.
///
/// The pairs are the neighbours across an edge (two shared corners) and across a corner (one
/// shared corner); the cell named is the pair's first in row order, and the list is sorted
/// row-major like [`Resolved`].
pub fn wrong_corners(grid: &WorldGrid, layer: usize, set: &EdgeSetJson) -> Vec<[u8; 2]> {
    let Some(tiles) = grid.layers.get(layer).map(|l| &l.tiles) else {
        return Vec::new();
    };
    let table = SetTable::new(set);
    let masks: Vec<Option<u8>> = (0..(CELLS * CELLS) as usize)
        .map(|i| table.read(tiles.get(i).copied().flatten()))
        .collect();
    let mask = |x: i32, z: i32| -> Option<u8> {
        in_map(x, z)
            .then(|| masks.get((z * CELLS + x) as usize).copied().flatten())
            .flatten()
    };
    // Each pair: the other cell's offset, then the (bit of the first, bit of the other) of
    // each shared corner.
    let pairs: [(Offset, &[SharedCorner]); 4] = [
        ((1, 0), &[(NE, NW), (SE, SW)]),
        ((0, 1), &[(SW, NW), (SE, NE)]),
        ((1, 1), &[(SE, NW)]),
        ((-1, 1), &[(SW, NE)]),
    ];
    let mut wrong = BTreeSet::new();
    for z in 0..CELLS {
        for x in 0..CELLS {
            let Some(a) = mask(x, z) else { continue };
            for ((dx, dz), shared) in pairs {
                let Some(b) = mask(x + dx, z + dz) else {
                    continue;
                };
                if shared
                    .iter()
                    .any(|&(ba, bb)| (a & ba != 0) != (b & bb != 0))
                {
                    wrong.insert((z, x));
                }
            }
        }
    }
    wrong.into_iter().map(|(z, x)| [x as u8, z as u8]).collect()
}

/// The collision bit the edge rule sets.
const BLOCKED: u8 = 0x80;

/// „Zelle (12, 21) hat eine eigene Permission; die Klippe lässt sie stehen.“
pub(crate) fn own_permission(x: i32, z: i32) -> String {
    format!("Zelle ({x}, {z}) hat eine eigene Permission; die Klippe lässt sie stehen.")
}

/// The ground height of cell (x, z): from `grid` inside the map, from `seams` on the ring;
/// `None` where nothing is read.
fn ground_height(grid: &WorldGrid, seams: &Seams, x: i32, z: i32) -> Option<i8> {
    if in_map(x, z) {
        grid.heights.get((z * CELLS + x) as usize).copied()
    } else {
        seams.heights.get(&seam_key(x, z)?).copied()
    }
}

/// The cliff brush: `cells` to `height` (ramps there cleared), the rim of `set` on top (rule „Klippenpinsel“), and for
/// `set.block == Lower` the overrides of rule „Kollision der Klippe“. Err: a place without tile, or
/// „Klippen gibt es nur auf der Bodenebene.“ is the caller's (layer) check.
///
/// A corner touching a painted cell is inside when every readable cell around it (the map's and
/// the seam's) is at least `height` high, else outside; the painted cells and every cell at least
/// `height` high that touches a changed corner are resolved as the terrain brush does, lower
/// cells keep their tile. Heights and cleared ramps list only cells that change.
#[allow(clippy::too_many_arguments)]
pub fn paint_cliff(
    grid: &WorldGrid,
    tileset: &Tileset,
    family: Family,
    set: &EdgeSetJson,
    cells: &[[u8; 2]],
    height: i8,
    seed: u32,
    seams: &Seams,
) -> Result<Resolved, String> {
    let painted: BTreeSet<(i32, i32)> = check_cells(grid, 0, cells)?
        .into_iter()
        .map(|(x, z)| (z, x))
        .collect();
    let mut raised = grid.clone();
    let mut heights = Vec::new();
    let mut ramps_cleared = Vec::new();
    for &(z, x) in &painted {
        let i = (z * CELLS + x) as usize;
        let at = [x as u8, z as u8];
        if let Some(h) = raised.heights.get_mut(i)
            && *h != height
        {
            *h = height;
            heights.push((at, height));
        }
        if let Some(r) = raised.ramps.get_mut(i)
            && *r != Ramp::None
        {
            *r = Ramp::None;
            ramps_cleared.push(at);
        }
    }
    let high = |x: i32, z: i32| ground_height(&raised, seams, x, z).map(|h| h >= height);
    let table = SetTable::new(set);
    let before = Corners::read(grid, 0, &table, seams);
    let mut after = before.clone();
    let corners: BTreeSet<(i32, i32)> = painted
        .iter()
        .flat_map(|&(z, x)| Corners::of_cell(x, z).map(|(cx, cz, _)| (cz, cx)))
        .collect();
    let mut touched = painted.clone();
    for (cz, cx) in corners {
        let around = cells_of_corner(cx, cz);
        let inside = around.iter().all(|&(x, z)| high(x, z) != Some(false));
        if after.set(cx, cz, inside) {
            touched.extend(
                around
                    .into_iter()
                    .filter(|&(x, z)| high(x, z) == Some(true))
                    .map(|(x, z)| (z, x)),
            );
        }
    }
    let tiles = resolve_cells(grid, 0, &table, &after, &touched, seed)?;
    let mut notes = seam_notes(&table, 0, seams, &before, &after);
    let overrides = if set.block == EdgeBlock::Lower {
        if let Some(ground) = raised.layers.get_mut(0) {
            for ([x, z], token) in &tiles {
                let i = usize::from(*z) * CELLS as usize + usize::from(*x);
                if let (Some(cell), Ok(tile)) = (ground.tiles.get_mut(i), parse_tile(token)) {
                    *cell = tile;
                }
            }
        }
        lower_overrides(grid, &raised, tileset, family, &painted, &mut notes)
    } else {
        Vec::new()
    };
    Ok(Resolved {
        tiles,
        heights,
        ramps_cleared,
        overrides,
        notes,
    })
}

/// The overrides of rule „Kollision der Klippe“ for `block: lower` on the painted grid `raised`:
/// for every high edge with a cell in `painted` (as (z, x)) or the ring of 8 around them, the
/// upper cell gets its tile's value ([`tile_cells`]), the lower one that value `| 0x80` (lower
/// wins for a cell that is both); a cell with an override in `grid` stays, with a note.
fn lower_overrides(
    grid: &WorldGrid,
    raised: &WorldGrid,
    tileset: &Tileset,
    family: Family,
    painted: &BTreeSet<(i32, i32)>,
    notes: &mut Vec<String>,
) -> Vec<([u8; 2], String)> {
    let near: BTreeSet<usize> = painted
        .iter()
        .flat_map(|&(z, x)| (-1..=1).flat_map(move |dz| (-1..=1).map(move |dx| (x + dx, z + dz))))
        .filter(|&(x, z)| in_map(x, z))
        .map(|(x, z)| (z * CELLS + x) as usize)
        .collect();
    let values = tile_cells(raised, tileset, family);
    let pairs: Vec<(usize, usize)> = high_edges(&raised.height_grid())
        .into_iter()
        .filter(|(upper, lower)| near.contains(upper) || near.contains(lower))
        .collect();
    // Row-major by index; uppers first, so a lower cell's bit wins.
    let mut cells: BTreeMap<usize, Cell> = BTreeMap::new();
    for &(upper, _) in &pairs {
        if let Some(&value) = values.get(upper) {
            cells.entry(upper).or_insert(value);
        }
    }
    for &(_, lower) in &pairs {
        if let Some(&value) = values.get(lower) {
            cells.insert(
                lower,
                Cell {
                    collision: value.collision | BLOCKED,
                    ..value
                },
            );
        }
    }
    let side = CELLS as usize;
    let mut overrides = Vec::new();
    for (i, cell) in cells {
        let (x, z) = (i % side, i / side);
        if grid.overrides.get(i).copied().flatten().is_some() {
            notes.push(own_permission(x as i32, z as i32));
        } else {
            overrides.push(([x as u8, z as u8], override_token(Some(cell))));
        }
    }
    overrides
}

/// The fixed gate pattern of the terrain brush (table „Fester gated Musterfall“).
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    /// The plus: x 4–12 z 8–10 and x 7–9 z 5–13.
    pub fn plus_cells() -> Vec<[u8; 2]> {
        let mut cells = Vec::new();
        for z in 5..=13u8 {
            for x in 4..=12u8 {
                let across = (8..=10).contains(&z);
                let along = (7..=9).contains(&x);
                if across || along {
                    cells.push([x, z]);
                }
            }
        }
        cells
    }

    /// The expected places of the plus on `one_tile_world`, x 2–14, z 3–15 (legend in the test).
    pub const PLUS_EXPECTED: [&str; 13] = [
        ".............",
        "....1NNN2....",
        "....WCCCE....",
        "....WCCCE....",
        ".1NN5CCC6NN2.",
        ".WCCCCCCCCCE.",
        ".WCCCCCCCCCE.",
        ".WCCCCCCCCCE.",
        ".3SS7CCC8SS4.",
        "....WCCCE....",
        "....WCCCE....",
        "....3SSS4....",
        ".............",
    ];
}

#[cfg(test)]
mod tests;
