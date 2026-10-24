//! Wizard commands: one backend call each for a new area, an extension of an area and a new
//! room with its entrance. Every call checks everything first, then creates the new members
//! (not undoable), then makes the undoable edits.

use std::collections::HashSet;

use g4_formats::game::Family;
use g4_formats::map::{Cell, MAP_SIZE};
use g4_formats::matrix::{MATRIX_EMPTY, resize_matrix as resized};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::{MapHeader, Store, WarpEvent};
use g4_kernel::tiles::{DOOR, DOOR_COLLISION, warp_fires};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// Where the event file of an assigned header comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum EventFileChoice {
    /// The header keeps the event file it has.
    Keep,
    /// An empty new event file.
    New,
    /// A copy of event file `n`.
    Copy(u16),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AreaCell {
    pub x: u8,
    pub y: u8,
    pub source_map: u16,
    pub clear_buildings: bool,
}

/// A header to create first (needs the Dynamic Headers patch): a copy of header `template`,
/// named `internal_name` in `mapname.bin`. It gets the id the game's header count says, which
/// the assignment must name as `id`.
#[derive(Debug, Clone, Deserialize)]
pub struct NewHeader {
    pub template: u16,
    pub internal_name: String,
}

/// The header that is pointed at the new matrix.
#[derive(Debug, Clone, Deserialize)]
pub struct AssignHeader {
    pub id: u16,
    pub area_data_id: Option<u8>,
    pub event_file: EventFileChoice,
    /// Create the header (as the next id) instead of using an existing one.
    #[serde(default)]
    pub new_header: Option<NewHeader>,
}

impl AssignHeader {
    /// The header whose fields the assigned header starts from: the template of a new
    /// header, else the header itself.
    fn source_header(&self) -> u16 {
        self.new_header.as_ref().map_or(self.id, |n| n.template)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AreaSpec {
    pub name: String,
    pub width: u8,
    pub height: u8,
    pub cells: Vec<AreaCell>,
    pub with_headers: bool,
    pub header: Option<AssignHeader>,
}

#[derive(Debug, Serialize)]
pub struct AreaReport {
    pub matrix: u16,
    pub maps: Vec<(u8, u8, u16)>,
    pub event_file: Option<u16>,
    pub header: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtendSpec {
    pub matrix: u16,
    pub x: u8,
    pub y: u8,
    pub source_map: u16,
    pub clear_buildings: bool,
    pub header_cell: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct ExtendReport {
    pub map: u16,
    pub width: u8,
    pub height: u8,
}

/// The door between the outside and the room: the outside warp stands at (`x`, `z`) of the
/// outside header's matrix, the room warp at (`inside_x`, `inside_z`) of the room.
#[derive(Debug, Clone, Deserialize)]
pub struct Entrance {
    pub outside_header: u16,
    pub x: u16,
    pub z: u16,
    pub inside_x: u16,
    pub inside_z: u16,
    /// Turns each of the two tiles a warp would not fire on into a door (see [`DOOR`]).
    #[serde(default)]
    pub door_tiles: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomSpec {
    pub name: String,
    pub source_map: u16,
    pub clear_buildings: bool,
    pub header: AssignHeader,
    pub entrance: Option<Entrance>,
}

/// A tile of a map that the wizard turned into a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoorTile {
    pub map: u16,
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Serialize)]
pub struct RoomReport {
    pub matrix: u16,
    pub map: u16,
    pub event_file: Option<u16>,
    pub outside_warp: Option<u16>,
    pub inside_warp: Option<u16>,
    /// The room warp is the copied warp that stood on its tile, now leading outside.
    pub inside_warp_reused: bool,
    /// The tiles made doors, the outside one first.
    pub door_tiles: Vec<DoorTile>,
}

/// The members a run created so far (they stay, whatever happens next).
#[derive(Debug, Default)]
pub(crate) struct Created {
    pub headers: Vec<u16>,
    pub matrices: Vec<u16>,
    pub maps: Vec<u16>,
    pub event_files: Vec<u16>,
}

impl Created {
    fn is_empty(&self) -> bool {
        self.headers.is_empty()
            && self.matrices.is_empty()
            && self.maps.is_empty()
            && self.event_files.is_empty()
    }
}

/// Names the created members, e.g. "matrix 0001 and maps 0003, 0004 were created".
pub(crate) fn created_text(created: &Created) -> String {
    fn part(one: &str, many: &str, ids: &[u16]) -> Option<String> {
        let list = ids
            .iter()
            .map(|i| format!("{i:04}"))
            .collect::<Vec<_>>()
            .join(", ");
        match ids.len() {
            0 => None,
            1 => Some(format!("{one} {list}")),
            _ => Some(format!("{many} {list}")),
        }
    }
    let parts: Vec<String> = [
        part("header", "headers", &created.headers),
        part("matrix", "matrices", &created.matrices),
        part("map", "maps", &created.maps),
        part("event file", "event files", &created.event_files),
    ]
    .into_iter()
    .flatten()
    .collect();
    let total = created.headers.len()
        + created.matrices.len()
        + created.maps.len()
        + created.event_files.len();
    let verb = if total == 1 { "was" } else { "were" };
    match parts.as_slice() {
        [] => "nothing was created".to_owned(),
        [only] => format!("{only} {verb} created"),
        [rest @ .., last] => format!("{} and {last} {verb} created", rest.join(", ")),
    }
}

/// The error of a step that failed during a run: says what stays behind. Code `"partial"`
/// once a member was created (the frontend shows what is left to finish by hand), otherwise
/// a plain `"error"`.
fn late(created: &Created, step: &str, e: impl std::fmt::Display) -> CmdError {
    let message = format!("{}; {step} failed: {e}", created_text(created));
    if created.is_empty() {
        message.into()
    } else {
        CmdError::partial(message)
    }
}

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

/// A matrix name must fit the file format's length byte.
fn check_name(name: &str) -> Result<(), CmdError> {
    if name.len() > usize::from(u8::MAX) {
        return Err(format!("the name is {} bytes long, at most 255 fit", name.len()).into());
    }
    Ok(())
}

fn check_member(store: &Store, kind: AssetKind, index: u16, what: &str) -> Result<(), CmdError> {
    let count = store.member_count(kind)?;
    if usize::from(index) >= count {
        return Err(format!("{what} {index} does not exist (the game has {count})").into());
    }
    Ok(())
}

fn check_header_id(session: &Session, header: u16) -> Result<(), CmdError> {
    let count = session.project.header_count()?;
    if usize::from(header) >= count {
        return Err(format!("header {header} does not exist (the game has {count})").into());
    }
    Ok(())
}

/// Refuses a loaded asset the codec would not write back byte for byte.
fn check_writable(store: &Store, asset: AssetId) -> Result<(), CmdError> {
    match store.locked(asset) {
        Some(reason) => Err(KernelError::ReadOnly {
            asset,
            reason: reason.to_owned(),
        }
        .into()),
        None => Ok(()),
    }
}

/// Event file `index` exists, can be loaded and is editable.
fn check_event_file_writable(store: &mut Store, index: u16) -> Result<(), CmdError> {
    check_member(store, AssetKind::Event, index, "event file")?;
    store.event(index)?;
    check_writable(store, id(AssetKind::Event, index))
}

/// Everything about the header assignment that can be known without writing: the header
/// exists and the table can be edited, the area data and the event file to copy exist.
fn check_assign(session: &mut Session, assign: &AssignHeader) -> Result<(), CmdError> {
    match &assign.new_header {
        None => check_header_id(session, assign.id)?,
        Some(new) => {
            let store = &mut session.store;
            if !store.dynamic_headers_applied()? {
                return Err(
                    "new headers need the dynamic headers patch, apply the patch first".into(),
                );
            }
            // The store's view: the count includes headers created earlier in this session.
            let count = store.header_table()?.len();
            if usize::from(assign.id) != count {
                return Err(
                    format!("a new header gets id {count:04}, not {:04}", assign.id).into(),
                );
            }
            check_header_id(session, new.template)?;
        }
    }
    let store = &mut session.store;
    store.header_table()?;
    check_writable(store, id(AssetKind::HeaderTable, 0))?;
    if let Some(area_data) = assign.area_data_id {
        check_member(
            store,
            AssetKind::AreaData,
            u16::from(area_data),
            "area data",
        )?;
    }
    if let EventFileChoice::Copy(source) = assign.event_file {
        check_member(store, AssetKind::Event, source, "event file")?;
    }
    check_header_fits(store, assign)
}

/// Creates the new header of an assignment, if it asks for one: the first step of a run.
fn provide_header(
    store: &mut Store,
    assign: &AssignHeader,
    created: &mut Created,
) -> Result<(), CmdError> {
    if let Some(new) = &assign.new_header {
        let header = store
            .create_header(new.template, &new.internal_name)
            .map_err(|e| late(created, "header creation", e))?;
        created.headers.push(header);
    }
    Ok(())
}

/// Creates the event file the choice asks for (`Keep` creates none).
fn provide_event_file(
    store: &mut Store,
    choice: EventFileChoice,
    created: &mut Created,
) -> Result<Option<u16>, CmdError> {
    let file = match choice {
        EventFileChoice::Keep => return Ok(None),
        EventFileChoice::New => store.create_event_file(),
        EventFileChoice::Copy(source) => store.duplicate_event_file(source),
    }
    .map_err(|e| late(created, "event file", e))?;
    created.event_files.push(file);
    Ok(Some(file))
}

/// Sets the fields of `header` that the assignment changes: the matrix, the area data if
/// given and the event file if a new one was made.
fn apply_assign(
    header: &mut MapHeader,
    assign: &AssignHeader,
    matrix: u16,
    event_file: Option<u16>,
) {
    match header {
        MapHeader::DP(h) => {
            h.matrix_id = matrix;
            h.area_data_id = assign.area_data_id.unwrap_or(h.area_data_id);
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
        }
        MapHeader::Pt(h) => {
            h.matrix_id = matrix;
            h.area_data_id = assign.area_data_id.unwrap_or(h.area_data_id);
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
        }
        MapHeader::HGSS(h) => {
            h.matrix_id = matrix;
            h.area_data_id = assign.area_data_id.unwrap_or(h.area_data_id);
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
        }
    }
}

/// Points header `assign.id` at `matrix` (and at the new area data and event file, if given).
fn assign_header(
    store: &mut Store,
    assign: &AssignHeader,
    matrix: u16,
    event_file: Option<u16>,
) -> Result<(), KernelError> {
    let label = format!("assign header {:04} to matrix {matrix:04}", assign.id);
    store.edit_header(assign.id, &label, |header| {
        apply_assign(header, assign, matrix, event_file);
        Ok(())
    })?;
    Ok(())
}

/// Dry run of the header edit: the new matrix and event file will be the next members of their
/// archives, so the final header is known before anything is created. Refuses a header the
/// game's layout could not store (the three fields are full-width in every layout, so this
/// only guards against a layout change).
fn check_header_fits(store: &mut Store, assign: &AssignHeader) -> Result<(), CmdError> {
    let next = |kind| -> Result<u16, CmdError> {
        let count = store.member_count(kind)?;
        u16::try_from(count)
            .map_err(|_| CmdError::from(format!("the {kind} archive has no room for a new member")))
    };
    let matrix = next(AssetKind::Matrix)?;
    let event_file = match assign.event_file {
        EventFileChoice::Keep => None,
        EventFileChoice::New | EventFileChoice::Copy(_) => Some(next(AssetKind::Event)?),
    };
    // A new header starts as a copy of its template.
    let source = assign.source_header();
    let mut header = store
        .header_table()?
        .get(usize::from(source))
        .cloned()
        .ok_or_else(|| CmdError::from(format!("header {source} does not exist")))?;
    apply_assign(&mut header, assign, matrix, event_file);
    if !store.header_fits(&header) {
        return Err(format!(
            "header {} would not fit the game's header layout with matrix {matrix}{}",
            assign.id,
            event_file.map_or(String::new(), |f| format!(" and event file {f}"))
        )
        .into());
    }
    Ok(())
}

fn cell_index(width: u8, x: u8, y: u8) -> usize {
    usize::from(y) * usize::from(width) + usize::from(x)
}

fn check_area(session: &mut Session, spec: &AreaSpec) -> Result<(), CmdError> {
    if spec.width == 0 || spec.height == 0 {
        return Err("an area needs at least one cell".into());
    }
    check_name(&spec.name)?;
    let mut seen = HashSet::new();
    for c in &spec.cells {
        if c.x >= spec.width || c.y >= spec.height {
            return Err(format!(
                "cell ({}, {}) is outside the {} x {} area",
                c.x, c.y, spec.width, spec.height
            )
            .into());
        }
        if !seen.insert((c.x, c.y)) {
            return Err(format!("cell ({}, {}) is given twice", c.x, c.y).into());
        }
        check_member(&session.store, AssetKind::Map, c.source_map, "map")?;
    }
    if let Some(assign) = &spec.header {
        check_assign(session, assign)?;
    }
    Ok(())
}

/// Creates a matrix of `width` x `height` from copies of maps and, if asked, points a header at
/// it. Checks everything before it writes; the copies, the matrix and the event file are new
/// members that stay when a later edit fails, the edits (matrix, then header) are undoable.
pub fn create_area_impl(state: &AppState, spec: AreaSpec) -> Result<AreaReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    check_area(session, &spec)?;
    let store = &mut session.store;

    let mut created = Created::default();
    if let Some(assign) = &spec.header {
        provide_header(store, assign, &mut created)?;
    }
    let mut maps = Vec::with_capacity(spec.cells.len());
    for c in &spec.cells {
        let map = store
            .duplicate_map_with(c.source_map, c.clear_buildings)
            .map_err(|e| late(&created, "map copy", e))?;
        created.maps.push(map);
        maps.push((c.x, c.y, map));
    }
    let matrix = store
        .create_matrix(&spec.name)
        .map_err(|e| late(&created, "matrix creation", e))?;
    created.matrices.push(matrix);
    let event_file = match &spec.header {
        Some(assign) => provide_event_file(store, assign.event_file, &mut created)?,
        None => None,
    };

    let header_id = spec.header.as_ref().map(|a| a.id);
    store
        .edit_matrix(matrix, "build area", |m| {
            *m = resized(m, spec.width, spec.height);
            let cells = usize::from(spec.width) * usize::from(spec.height);
            if spec.with_headers && m.headers.is_none() {
                m.headers = Some(vec![0; cells]);
            }
            for &(x, y, map) in &maps {
                let i = cell_index(spec.width, x, y);
                m.maps[i] = map;
                if let (Some(header), Some(headers)) = (header_id, m.headers.as_mut()) {
                    headers[i] = header;
                }
            }
            Ok(())
        })
        .map_err(|e| late(&created, "matrix edit", e))?;
    if let Some(assign) = &spec.header {
        assign_header(store, assign, matrix, event_file)
            .map_err(|e| late(&created, "header edit", e))?;
    }
    Ok(AreaReport {
        matrix,
        maps,
        event_file,
        header: header_id,
    })
}

/// Copies a map into the empty cell (`x`, `y`) of a matrix, growing the matrix if the cell lies
/// outside it, and optionally sets the cell's header. One undoable matrix edit.
pub fn extend_area_impl(state: &AppState, spec: ExtendSpec) -> Result<ExtendReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let (width, height) = {
        let m = session.store.matrix(spec.matrix)?;
        let width = u8::try_from(u16::from(spec.x) + 1)
            .map(|w| w.max(m.width))
            .map_err(|_| CmdError::from("the area would be wider than 255 cells"))?;
        let height = u8::try_from(u16::from(spec.y) + 1)
            .map(|h| h.max(m.height))
            .map_err(|_| CmdError::from("the area would be taller than 255 cells"))?;
        let inside = spec.x < m.width && spec.y < m.height;
        if inside && m.maps[cell_index(m.width, spec.x, spec.y)] != MATRIX_EMPTY {
            return Err(format!("cell ({}, {}) is not empty", spec.x, spec.y).into());
        }
        if spec.header_cell.is_some() && m.headers.is_none() {
            return Err("the matrix has no header section".into());
        }
        (width, height)
    };
    check_writable(&session.store, id(AssetKind::Matrix, spec.matrix))?;
    check_member(&session.store, AssetKind::Map, spec.source_map, "map")?;
    if let Some(header) = spec.header_cell {
        check_header_id(session, header)?;
    }
    let store = &mut session.store;

    let mut created = Created::default();
    let map = store
        .duplicate_map_with(spec.source_map, spec.clear_buildings)
        .map_err(|e| late(&created, "map copy", e))?;
    created.maps.push(map);
    store
        .edit_matrix(spec.matrix, "extend area", |m| {
            *m = resized(m, width, height);
            let i = cell_index(width, spec.x, spec.y);
            m.maps[i] = map;
            if let (Some(header), Some(headers)) = (spec.header_cell, m.headers.as_mut()) {
                headers[i] = header;
            }
            Ok(())
        })
        .map_err(|e| late(&created, "matrix edit", e))?;
    Ok(ExtendReport { map, width, height })
}

/// Number of warps in event file `index` as a warp index.
fn warp_count(store: &mut Store, index: u16) -> Result<u16, CmdError> {
    let count = store.event(index)?.warp_events.len();
    u16::try_from(count)
        .map_err(|_| CmdError::from(format!("event file {index:04} has too many warps")))
}

fn push_warp(
    store: &mut Store,
    file: u16,
    warp: WarpEvent,
    created: &Created,
) -> Result<(), CmdError> {
    let label = format!("connect warp to header {:04}", warp.dest_header_id);
    store
        .edit_event(file, &label, |f| {
            f.warp_events.push(warp);
            Ok(())
        })
        .map_err(|e| late(created, "warp edit", e))?;
    Ok(())
}

/// Points warp `index` of event file `file` at warp `dest_warp_id` of header `dest_header_id`;
/// it keeps its tile and height.
fn retarget_warp(
    store: &mut Store,
    file: u16,
    index: u16,
    (dest_header_id, dest_warp_id): (u16, u16),
    created: &Created,
) -> Result<(), CmdError> {
    let label = format!("connect warp to header {dest_header_id:04}");
    store
        .edit_event(file, &label, |f| {
            if let Some(w) = f.warp_events.get_mut(usize::from(index)) {
                w.dest_header_id = dest_header_id;
                w.dest_warp_id = dest_warp_id;
            }
            Ok(())
        })
        .map_err(|e| late(created, "warp edit", e))?;
    Ok(())
}

/// The first warp of event file `file` on tile (`x`, `z`): the one the game takes there.
fn warp_at(store: &mut Store, file: u16, x: u16, z: u16) -> Result<Option<u16>, CmdError> {
    let found = store
        .event(file)?
        .warp_events
        .iter()
        .position(|w| w.x == x && w.z == z);
    found
        .map(|i| {
            u16::try_from(i)
                .map_err(|_| CmdError::from(format!("event file {file:04} has too many warps")))
        })
        .transpose()
}

/// The event file `header` has now.
fn header_event_file(store: &mut Store, header: u16) -> Result<u16, CmdError> {
    store
        .header_table()?
        .get(usize::from(header))
        .map(MapHeader::event_file_id)
        .ok_or_else(|| format!("header {header} does not exist").into())
}

/// The map under tile (`x`, `z`) of the matrix `header` uses. Refuses a tile outside that
/// matrix, in a cell without a map, and in a cell the matrix gives to another header: the game
/// loads a header's warps only while the player is in one of its cells.
fn outside_map(store: &mut Store, header: u16, x: u16, z: u16) -> Result<u16, CmdError> {
    let matrix_id = store
        .header_table()?
        .get(usize::from(header))
        .map(MapHeader::matrix_id)
        .ok_or_else(|| CmdError::from(format!("header {header} does not exist")))?;
    let m = store.matrix(matrix_id)?;
    let (cx, cy) = (usize::from(x) / MAP_SIZE, usize::from(z) / MAP_SIZE);
    let (width, height) = (usize::from(m.width), usize::from(m.height));
    if cx >= width || cy >= height {
        return Err(format!(
            "tile ({x}, {z}) lies outside matrix {matrix_id:04} ({} x {} tiles)",
            width * MAP_SIZE,
            height * MAP_SIZE
        )
        .into());
    }
    let i = cy * width + cx;
    let map = m.maps[i];
    if map == MATRIX_EMPTY {
        return Err(format!(
            "tile ({x}, {z}) lies in cell ({cx}, {cy}) of matrix {matrix_id:04}, which has no map"
        )
        .into());
    }
    if let Some(owner) = m.headers.as_ref().map(|h| h[i])
        && owner != header
    {
        return Err(format!(
            "tile ({x}, {z}) lies in cell ({cx}, {cy}) of header {owner:04}, not of header {header:04}"
        )
        .into());
    }
    Ok(map)
}

/// Whether the game would take a warp on tile (`x`, `y`) of `map`.
fn fires(store: &mut Store, family: Family, map: u16, x: u16, y: u16) -> Result<bool, CmdError> {
    let cell = store
        .map(map)?
        .cell(usize::from(x), usize::from(y))
        .map_err(KernelError::from)?;
    Ok(warp_fires(family, cell.kind, cell.collision))
}

/// Makes tile (`x`, `y`) of `map` a door the player bumps into.
fn set_door(
    store: &mut Store,
    map: u16,
    (x, y): (u16, u16),
    created: &Created,
) -> Result<DoorTile, CmdError> {
    let (x, y) = (
        u8::try_from(x).unwrap_or(u8::MAX),
        u8::try_from(y).unwrap_or(u8::MAX),
    );
    let door = Cell {
        kind: DOOR,
        collision: DOOR_COLLISION,
    };
    store
        .edit_map(map, "door tile", |m| {
            m.set_cell(usize::from(x), usize::from(y), door)
        })
        .map_err(|e| late(created, "door tile", e))?;
    Ok(DoorTile { map, x, y })
}

/// What the entrance of a room needs from the existing files, found before anything is
/// created: the outside event file and map, the warp of the room's event file that becomes the
/// way back, and which of the two tiles must be made doors.
struct EntrancePlan {
    outside_file: u16,
    outside_map: u16,
    /// The warp a copied event file already has on the inside tile; it is pointed outside
    /// instead of adding a second warp the game would never take.
    reused: Option<u16>,
    outside_door: bool,
    inside_door: bool,
}

fn plan_entrance(
    session: &mut Session,
    spec: &RoomSpec,
    entrance: &Entrance,
) -> Result<EntrancePlan, CmdError> {
    let family = session.project.family;
    let store = &mut session.store;
    if entrance.outside_header == spec.header.id {
        return Err("the room's header cannot also be the header outside".into());
    }
    let last = MAP_SIZE as u16 - 1;
    if entrance.inside_x > last || entrance.inside_z > last {
        return Err(format!(
            "tile ({}, {}) lies outside the room's map (0 to {last})",
            entrance.inside_x, entrance.inside_z
        )
        .into());
    }
    let outside_file = header_event_file(store, entrance.outside_header)?;
    check_event_file_writable(store, outside_file)?;
    let outside_map = outside_map(store, entrance.outside_header, entrance.x, entrance.z)?;
    if let Some(i) = warp_at(store, outside_file, entrance.x, entrance.z)? {
        return Err(format!(
            "event file {outside_file:04} already has warp {i} on tile ({}, {}) outside",
            entrance.x, entrance.z
        )
        .into());
    }
    let reused = match spec.header.event_file {
        EventFileChoice::Keep => {
            let own = header_event_file(store, spec.header.source_header())?;
            check_event_file_writable(store, own)?;
            // The file stays shared with whoever has it now: its warp there is not the wizard's
            // to repoint.
            if let Some(i) = warp_at(store, own, entrance.inside_x, entrance.inside_z)? {
                return Err(format!(
                    "event file {own:04} already has warp {i} on tile ({}, {}) of the room",
                    entrance.inside_x, entrance.inside_z
                )
                .into());
            }
            None
        }
        EventFileChoice::Copy(source) => {
            check_event_file_writable(store, source)?;
            warp_at(store, source, entrance.inside_x, entrance.inside_z)?
        }
        EventFileChoice::New => None,
    };
    let (mut outside_door, mut inside_door) = (false, false);
    if entrance.door_tiles {
        let local = |n: u16| n % MAP_SIZE as u16;
        outside_door = !fires(
            store,
            family,
            outside_map,
            local(entrance.x),
            local(entrance.z),
        )?;
        if outside_door {
            check_writable(store, id(AssetKind::Map, outside_map))?;
        }
        inside_door = !fires(
            store,
            family,
            spec.source_map,
            entrance.inside_x,
            entrance.inside_z,
        )?;
    }
    Ok(EntrancePlan {
        outside_file,
        outside_map,
        reused,
        outside_door,
        inside_door,
    })
}

/// Creates a 1 x 1 matrix with a copy of a map, points a header at it and, with an entrance,
/// links a warp outside with a warp inside. Order of the undoable edits: matrix, header,
/// outside warp, room warp, then the door tiles (outside, inside). When the outside header and
/// the room header share the event file (`Keep`), the room warp is appended right after the
/// outside warp in that one file. A copied event file that has a warp on the inside tile keeps
/// it as the way back, pointed outside: the game takes the first warp of a tile.
pub fn create_room_impl(state: &AppState, spec: RoomSpec) -> Result<RoomReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    check_name(&spec.name)?;
    check_member(&session.store, AssetKind::Map, spec.source_map, "map")?;
    check_assign(session, &spec.header)?;
    if let Some(entrance) = &spec.entrance {
        check_header_id(session, entrance.outside_header)?;
    }
    let plan = match &spec.entrance {
        Some(entrance) => Some(plan_entrance(session, &spec, entrance)?),
        None => None,
    };
    let store = &mut session.store;
    // A new header starts with its template's event file.
    let kept_file = header_event_file(store, spec.header.source_header())?;

    let mut created = Created::default();
    provide_header(store, &spec.header, &mut created)?;
    let map = store
        .duplicate_map_with(spec.source_map, spec.clear_buildings)
        .map_err(|e| late(&created, "map copy", e))?;
    created.maps.push(map);
    let matrix = store
        .create_matrix(&spec.name)
        .map_err(|e| late(&created, "matrix creation", e))?;
    created.matrices.push(matrix);
    let event_file = provide_event_file(store, spec.header.event_file, &mut created)?;
    let room_file = event_file.unwrap_or(kept_file);

    store
        .edit_matrix(matrix, "build room", |m| {
            m.maps[0] = map;
            Ok(())
        })
        .map_err(|e| late(&created, "matrix edit", e))?;
    assign_header(store, &spec.header, matrix, event_file)
        .map_err(|e| late(&created, "header edit", e))?;

    let (mut outside_warp, mut inside_warp) = (None, None);
    let mut door_tiles = Vec::new();
    let reused = plan.as_ref().and_then(|p| p.reused);
    if let (Some(entrance), Some(plan)) = (&spec.entrance, &plan) {
        let outside_file = plan.outside_file;
        let shared = outside_file == room_file;
        let late_err = |e: CmdError| late(&created, "warp edit", e.message);
        let outside_index = warp_count(store, outside_file).map_err(late_err)?;
        // With one shared file the room warp comes right after the outside warp.
        let room_index = match plan.reused {
            Some(i) => i,
            None if shared => outside_index.saturating_add(1),
            None => warp_count(store, room_file).map_err(late_err)?,
        };
        let warp = |x, z, dest_header_id, dest_warp_id| WarpEvent {
            x,
            z,
            dest_header_id,
            dest_warp_id,
            height: 0,
        };
        push_warp(
            store,
            outside_file,
            warp(entrance.x, entrance.z, spec.header.id, room_index),
            &created,
        )?;
        let back = (entrance.outside_header, outside_index);
        match plan.reused {
            Some(i) => retarget_warp(store, room_file, i, back, &created)?,
            None => push_warp(
                store,
                room_file,
                warp(entrance.inside_x, entrance.inside_z, back.0, back.1),
                &created,
            )?,
        }
        (outside_warp, inside_warp) = (Some(outside_index), Some(room_index));
        let local = |n: u16| n % MAP_SIZE as u16;
        if plan.outside_door {
            let tile = (local(entrance.x), local(entrance.z));
            door_tiles.push(set_door(store, plan.outside_map, tile, &created)?);
        }
        if plan.inside_door {
            let tile = (entrance.inside_x, entrance.inside_z);
            door_tiles.push(set_door(store, map, tile, &created)?);
        }
    }
    Ok(RoomReport {
        matrix,
        map,
        event_file,
        outside_warp,
        inside_warp,
        inside_warp_reused: reused.is_some(),
        door_tiles,
    })
}

#[tauri::command]
pub fn create_area(state: State<AppState>, spec: AreaSpec) -> Result<AreaReport, CmdError> {
    create_area_impl(&state, spec)
}

#[tauri::command]
pub fn extend_area(state: State<AppState>, spec: ExtendSpec) -> Result<ExtendReport, CmdError> {
    extend_area_impl(&state, spec)
}

#[tauri::command]
pub fn create_room(state: State<AppState>, spec: RoomSpec) -> Result<RoomReport, CmdError> {
    create_room_impl(&state, spec)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{
        EventKind, add_event_impl, apply_header_patch_impl, dirty_impl, get_events_impl,
        get_header_impl, get_map_impl, get_matrix_impl, list_headers_impl, member_counts_impl,
        reload_asset_impl, resize_matrix_impl, set_header_impl, set_matrix_cell_impl,
        set_matrix_sections_impl, set_permission_impl, undo_impl,
    };
    use super::*;
    use g4_kernel::store::MapHeader;
    use std::collections::HashMap;

    fn counts(state: &AppState) -> HashMap<AssetKind, usize> {
        member_counts_impl(state).unwrap()
    }

    fn asset(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    fn cell(x: u8, y: u8, source_map: u16, clear_buildings: bool) -> AreaCell {
        AreaCell {
            x,
            y,
            source_map,
            clear_buildings,
        }
    }

    fn area_spec() -> AreaSpec {
        AreaSpec {
            name: "Route 230".into(),
            width: 2,
            height: 1,
            cells: vec![cell(0, 0, 1, true), cell(1, 0, 0, false)],
            with_headers: true,
            header: Some(AssignHeader {
                id: 1,
                area_data_id: None,
                event_file: EventFileChoice::New,
                new_header: None,
            }),
        }
    }

    /// Points header `id` at event file `file`.
    fn set_event_file(state: &AppState, id: u16, file: u16) {
        let mut h = get_header_impl(state, id).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.event_file_id = file;
        }
        set_header_impl(state, id, h, "edit header").unwrap();
    }

    #[test]
    fn create_area_copies_maps_builds_the_matrix_and_assigns_the_header() {
        let (_f, state) = open_synthetic();
        let report = create_area_impl(&state, area_spec()).unwrap();
        assert_eq!(report.matrix, 1);
        assert_eq!(report.maps, vec![(0, 0, 3), (1, 0, 4)]);
        assert_eq!((report.event_file, report.header), (Some(2), Some(1)));

        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!(m.maps, vec![Some(3), Some(4)]);
        assert_eq!(m.headers, Some(vec![1, 1]));
        assert_eq!(m.name, "Route 230");
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        assert!(get_map_impl(&state, 3).unwrap().buildings.is_empty());
        assert_eq!(get_map_impl(&state, 4).unwrap().buildings.len(), 1);
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![
                asset(AssetKind::Matrix, 1),
                asset(AssetKind::HeaderTable, 0)
            ]
        );

        // Two edits (matrix, header) are undone; the new members stay.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(get_header_impl(&state, 1).unwrap().matrix_id(), 0);
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.width, m.height, m.maps), (1, 1, vec![None]));
        let counts = counts(&state);
        assert_eq!(counts[&AssetKind::Map], 5);
        assert_eq!(counts[&AssetKind::Matrix], 2);
        assert_eq!(counts[&AssetKind::Event], 3);
    }

    #[test]
    fn create_area_without_a_header_only_builds_the_matrix() {
        let (_f, state) = open_synthetic();
        let spec = AreaSpec {
            with_headers: false,
            header: None,
            ..area_spec()
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.event_file, report.header), (None, None));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.maps, m.headers), (vec![Some(3), Some(4)], None));
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![asset(AssetKind::Matrix, 1)]
        );
    }

    #[test]
    fn create_area_refuses_bad_specs_without_creating_members() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let bad: Vec<AreaSpec> = vec![
            AreaSpec {
                header: Some(AssignHeader {
                    id: 5,
                    area_data_id: None,
                    event_file: EventFileChoice::Keep,
                    new_header: None,
                }),
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(2, 0, 0, false)],
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(0, 0, 0, false), cell(0, 0, 1, false)],
                ..area_spec()
            },
            AreaSpec {
                width: 0,
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(0, 0, 9, false)],
                ..area_spec()
            },
            AreaSpec {
                header: Some(AssignHeader {
                    id: 1,
                    area_data_id: None,
                    event_file: EventFileChoice::Copy(9),
                    new_header: None,
                }),
                ..area_spec()
            },
            AreaSpec {
                name: "x".repeat(256),
                ..area_spec()
            },
        ];
        for spec in bad {
            assert!(create_area_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
    }

    #[test]
    fn an_unknown_area_data_is_refused_up_front() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 1,
                area_data_id: Some(200),
                event_file: EventFileChoice::Keep,
                new_header: None,
            }),
            ..area_spec()
        };
        assert!(create_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    fn extend_spec() -> ExtendSpec {
        ExtendSpec {
            matrix: 0,
            x: 2,
            y: 0,
            source_map: 2,
            clear_buildings: false,
            header_cell: None,
        }
    }

    #[test]
    fn extend_area_grows_the_matrix_and_places_the_copy() {
        let (_f, state) = open_synthetic();
        let report = extend_area_impl(&state, extend_spec()).unwrap();
        assert_eq!((report.map, report.width, report.height), (3, 3, 1));
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(0), Some(1), Some(3)]
        );

        // The cell (0, 0) is taken: refused, and no map is copied for it.
        let maps = counts(&state)[&AssetKind::Map];
        let taken = ExtendSpec {
            x: 0,
            ..extend_spec()
        };
        assert!(extend_area_impl(&state, taken).is_err());
        assert_eq!(counts(&state)[&AssetKind::Map], maps);
    }

    #[test]
    fn extend_area_can_set_the_header_of_the_new_cell() {
        let (_f, state) = open_synthetic();
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let spec = ExtendSpec {
            clear_buildings: true,
            header_cell: Some(1),
            ..extend_spec()
        };
        extend_area_impl(&state, spec).unwrap();
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().headers,
            Some(vec![0, 0, 1])
        );
    }

    #[test]
    fn extend_area_refuses_what_cannot_work_before_copying_the_map() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let bad = [
            // The matrix has no header section; unknown map and matrix; the grid would grow
            // past 255 cells.
            ExtendSpec {
                header_cell: Some(1),
                ..extend_spec()
            },
            ExtendSpec {
                source_map: 9,
                ..extend_spec()
            },
            ExtendSpec {
                matrix: 7,
                ..extend_spec()
            },
            ExtendSpec {
                x: 255,
                ..extend_spec()
            },
        ];
        for spec in bad {
            assert!(extend_area_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
        // With the section, a header the game does not have is still refused.
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let spec = ExtendSpec {
            header_cell: Some(5),
            ..extend_spec()
        };
        assert!(extend_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
    }

    fn room_spec(event_file: EventFileChoice) -> RoomSpec {
        RoomSpec {
            name: "Haus".into(),
            source_map: 2,
            clear_buildings: true,
            header: AssignHeader {
                id: 1,
                area_data_id: None,
                event_file,
                new_header: None,
            },
            entrance: Some(Entrance {
                outside_header: 0,
                x: 5,
                z: 6,
                inside_x: 16,
                inside_z: 30,
                door_tiles: false,
            }),
        }
    }

    #[test]
    fn create_room_with_entrance_links_both_warps() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (
                report.matrix,
                report.map,
                report.event_file,
                report.outside_warp,
                report.inside_warp
            ),
            (1, 3, Some(2), Some(0), Some(0))
        );
        let outside = get_events_impl(&state, 0).unwrap().warps;
        let w = &outside[0];
        assert_eq!(
            (w.x, w.z, w.dest_header_id, w.dest_warp_id, w.height),
            (5, 6, 1, 0, 0)
        );
        let inside = get_events_impl(&state, 2).unwrap().warps;
        let w = &inside[0];
        assert_eq!(
            (w.x, w.z, w.dest_header_id, w.dest_warp_id, w.height),
            (16, 30, 0, 0, 0)
        );
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.name.as_str(), m.maps), ("Haus", vec![Some(3)]));
        assert!(get_map_impl(&state, 3).unwrap().buildings.is_empty());

        // The two warps and the header change are three undo steps (the matrix edit is a
        // fourth).
        for _ in 0..3 {
            assert!(!undo_impl(&state).unwrap().is_empty());
        }
        assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
        assert!(get_events_impl(&state, 2).unwrap().warps.is_empty());
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (0, 1));
    }

    #[test]
    fn create_room_with_shared_event_file_cross_links_both_warps() {
        let (_f, state) = open_synthetic();
        let report = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap();
        assert_eq!(report.event_file, None);
        assert_eq!(
            (report.outside_warp, report.inside_warp),
            (Some(0), Some(1))
        );
        let warps = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(warps.len(), 2);
        assert_eq!((warps[0].dest_header_id, warps[0].dest_warp_id), (1, 1));
        assert_eq!((warps[1].dest_header_id, warps[1].dest_warp_id), (0, 0));
        assert_eq!((warps[1].x, warps[1].z), (16, 30));
    }

    /// Adds `n` warps to event file `file`.
    fn add_warps(state: &AppState, file: u16, n: usize) {
        for i in 0..n {
            add_event_impl(state, file, EventKind::Warp, i as i32, 0, None, None).unwrap();
        }
    }

    #[test]
    fn create_room_links_to_the_end_of_files_that_already_have_warps() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_warps(&state, 0, 1);
        add_warps(&state, 1, 2);
        // The copy of file 1 has its 2 warps, so the room warp becomes warp 2 of file 2; the
        // outside warp becomes warp 1 of file 0.
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), Some(1), Some(2))
        );
        let outside = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(outside.len(), 2);
        let w = &outside[1];
        assert_eq!((w.x, w.z, w.dest_header_id, w.dest_warp_id), (5, 6, 1, 2));
        let inside = get_events_impl(&state, 2).unwrap().warps;
        assert_eq!(inside.len(), 3);
        let w = &inside[2];
        assert_eq!((w.x, w.z, w.dest_header_id, w.dest_warp_id), (16, 30, 0, 1));
    }

    #[test]
    fn create_room_in_a_shared_file_with_warps_appends_both_after_them() {
        let (_f, state) = open_synthetic();
        add_warps(&state, 0, 2);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (None, Some(2), Some(3))
        );
        let warps = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(warps.len(), 4);
        // The outside warp points at the room warp (3), the room warp back at the outside warp (2).
        assert_eq!(
            (warps[2].x, warps[2].dest_header_id, warps[2].dest_warp_id),
            (5, 1, 3)
        );
        assert_eq!(
            (warps[3].x, warps[3].dest_header_id, warps[3].dest_warp_id),
            (16, 0, 2)
        );
    }

    /// The warps of event file `file` as (x, z, destination header, destination warp).
    fn warps(state: &AppState, file: u16) -> Vec<(u16, u16, u16, u16)> {
        get_events_impl(state, file)
            .unwrap()
            .warps
            .iter()
            .map(|w| (w.x, w.z, w.dest_header_id, w.dest_warp_id))
            .collect()
    }

    #[test]
    fn create_room_points_the_copied_warp_on_the_inside_tile_outside() {
        // Gate 3a (A) 12: the template's exit warp stood on the inside door tile, the wizard
        // added the way back as a second warp on that tile, and the game took the first one.
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_warps(&state, 1, 1);
        add_event_impl(&state, 1, EventKind::Warp, 16, 30, None, None).unwrap();
        let template = warps(&state, 1);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), Some(0), Some(1))
        );
        assert!(report.inside_warp_reused);
        // The copy keeps its two warps; the one on the door tile now leads outside.
        let inside = warps(&state, 2);
        assert_eq!(inside.len(), 2);
        assert_eq!(inside[0], template[0]);
        assert_eq!(inside[1], (16, 30, 0, 0));
        assert_eq!(warps(&state, 0), vec![(5, 6, 1, 1)]);
        // The template's own file is untouched.
        assert_eq!(warps(&state, 1), template);
    }

    #[test]
    fn create_room_refuses_a_kept_file_with_a_warp_on_the_inside_tile() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_event_impl(&state, 1, EventKind::Warp, 16, 30, None, None).unwrap();
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap_err();
        assert!(
            err.message.contains("warp 0 on tile (16, 30)"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn create_room_refuses_an_outside_tile_that_already_has_a_warp() {
        let (_f, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Warp, 5, 6, None, None).unwrap();
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap_err();
        assert!(
            err.message.contains("warp 0 on tile (5, 6)"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn create_room_refuses_an_outside_tile_without_a_map_of_the_outside_header() {
        // Gate 3a (F): x 999 was clamped to the last tile and the entrance landed in an empty cell.
        let (_f, state) = open_synthetic();
        let at = |x: u16, z: u16| {
            let mut spec = room_spec(EventFileChoice::New);
            let e = spec.entrance.as_mut().unwrap();
            (e.x, e.z) = (x, z);
            spec
        };
        let before = counts(&state);
        // Matrix 0 is 2 x 1 maps: 64 x 32 tiles.
        let err = create_room_impl(&state, at(64, 6)).unwrap_err();
        assert!(
            err.message.contains("outside matrix 0000"),
            "{}",
            err.message
        );
        let err = create_room_impl(&state, at(5, 32)).unwrap_err();
        assert!(
            err.message.contains("outside matrix 0000"),
            "{}",
            err.message
        );
        let mut inside = room_spec(EventFileChoice::New);
        inside.entrance.as_mut().unwrap().inside_x = 32;
        assert!(create_room_impl(&state, inside).is_err());
        assert_eq!(counts(&state), before);

        // A third, empty cell.
        resize_matrix_impl(&state, 0, 3, 1).unwrap();
        let err = create_room_impl(&state, at(70, 6)).unwrap_err();
        assert!(err.message.contains("has no map"), "{}", err.message);
        // A cell the header section gives to header 1.
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        set_matrix_cell_impl(&state, 0, 1, 0, None, Some(1), None).unwrap();
        let err = create_room_impl(&state, at(40, 6)).unwrap_err();
        assert!(
            err.message.contains("of header 0001, not of header 0000"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
        // Header 0's own cell is fine.
        assert!(create_room_impl(&state, at(5, 6)).is_ok());
    }

    fn tile(state: &AppState, map: u16, x: usize, y: usize) -> [u8; 2] {
        get_map_impl(state, map).unwrap().permissions[y * 32 + x]
    }

    #[test]
    fn create_room_makes_doors_of_tiles_a_warp_would_not_fire_on() {
        let (_f, state) = open_synthetic();
        let mut spec = room_spec(EventFileChoice::New);
        spec.entrance.as_mut().unwrap().door_tiles = true;
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(
            report.door_tiles,
            vec![
                DoorTile { map: 0, x: 5, y: 6 },
                DoorTile {
                    map: report.map,
                    x: 16,
                    y: 30
                }
            ]
        );
        assert_eq!(tile(&state, 0, 5, 6), [0x69, 0x80]);
        assert_eq!(tile(&state, report.map, 16, 30), [0x69, 0x80]);
        // The source map of the room keeps its tile.
        assert_ne!(tile(&state, 2, 16, 30), [0x69, 0x80]);
        // Both tiles are undo steps of their own, after the warps.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_ne!(tile(&state, 0, 5, 6), [0x69, 0x80]);
        assert_eq!(warps(&state, 0).len(), 1);
    }

    #[test]
    fn create_room_leaves_tiles_alone_that_already_start_a_warp() {
        let (_f, state) = open_synthetic();
        // Stairs outside (any collision), a blocking door inside the source map.
        set_permission_impl(&state, 0, 5, 6, 0x5E, 0).unwrap();
        set_permission_impl(&state, 2, 16, 30, 0x69, 0x80).unwrap();
        let mut spec = room_spec(EventFileChoice::New);
        spec.entrance.as_mut().unwrap().door_tiles = true;
        let report = create_room_impl(&state, spec).unwrap();
        assert!(report.door_tiles.is_empty());
        assert_eq!(tile(&state, 0, 5, 6), [0x5E, 0]);
        // Without the option nothing is checked or set.
        let (_f, state) = open_synthetic();
        let report = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap();
        assert!(report.door_tiles.is_empty());
    }

    #[test]
    fn create_room_without_entrance_makes_no_warps() {
        let (_f, state) = open_synthetic();
        let spec = RoomSpec {
            entrance: None,
            ..room_spec(EventFileChoice::New)
        };
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), None, None)
        );
        assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
    }

    /// Unpacks the event archive and fills event file 1 with 17 zero bytes (read-only: the
    /// codec would write 16 back).
    fn lock_event_file_1(fixture: &g4_kernel::test_support::Synthetic, state: &AppState) {
        reload_asset_impl(state, AssetKind::Event, 0).unwrap();
        std::fs::write(
            fixture.dir.path().join("unpacked/eventFiles/0001"),
            [0u8; 17],
        )
        .unwrap();
    }

    #[test]
    fn create_room_refuses_a_locked_outside_file_before_creating_anything() {
        let (fixture, state) = open_synthetic();
        lock_event_file_1(&fixture, &state);
        set_event_file(&state, 0, 1);
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        assert_eq!(counts(&state), before);
        // Only the header edit of the test's own setup is pending.
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![asset(AssetKind::HeaderTable, 0)]
        );
    }

    #[test]
    fn create_room_refuses_a_locked_room_file_before_creating_anything() {
        let (fixture, state) = open_synthetic();
        lock_event_file_1(&fixture, &state);
        let before = counts(&state);
        // Copy: the file to copy from is the locked one.
        let err = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        // Keep: the room header's own file is the locked one.
        set_event_file(&state, 1, 1);
        let err = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        assert_eq!(counts(&state), before);
        // Without an entrance no event file is written to, so the same room is fine.
        let spec = RoomSpec {
            entrance: None,
            ..room_spec(EventFileChoice::Keep)
        };
        assert!(create_room_impl(&state, spec).is_ok());
    }

    #[test]
    fn create_room_refuses_bad_specs_without_creating_members() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let with_outside = |outside_header: u16| {
            let mut spec = room_spec(EventFileChoice::New);
            spec.entrance.as_mut().unwrap().outside_header = outside_header;
            spec
        };
        let bad = [
            RoomSpec {
                source_map: 9,
                ..room_spec(EventFileChoice::New)
            },
            RoomSpec {
                name: "x".repeat(256),
                ..room_spec(EventFileChoice::New)
            },
            room_spec(EventFileChoice::Copy(9)),
            with_outside(9),
            // The room's header cannot be its own outside header.
            with_outside(1),
        ];
        for spec in bad {
            assert!(create_room_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
    }

    #[test]
    fn created_text_names_the_members_of_a_partial_run() {
        let created = Created {
            matrices: vec![1],
            maps: vec![3, 4],
            ..Created::default()
        };
        assert_eq!(
            created_text(&created),
            "matrix 0001 and maps 0003, 0004 were created"
        );
        assert_eq!(created_text(&Created::default()), "nothing was created");
        let created = Created {
            matrices: vec![1],
            maps: vec![3],
            event_files: vec![2],
            ..Created::default()
        };
        assert_eq!(
            created_text(&created),
            "matrix 0001, map 0003 and event file 0002 were created"
        );
        let created = Created {
            maps: vec![3],
            ..Created::default()
        };
        assert_eq!(created_text(&created), "map 0003 was created");
        let created = Created {
            headers: vec![470],
            ..Created::default()
        };
        assert_eq!(created_text(&created), "header 0470 was created");
        let created = Created {
            headers: vec![470],
            matrices: vec![1],
            maps: vec![3, 4],
            event_files: vec![2],
        };
        assert_eq!(
            created_text(&created),
            "header 0470, matrix 0001, maps 0003, 0004 and event file 0002 were created"
        );
    }

    /// The patched fixture with 2 headers; a new header would get id 2.
    fn patched() -> (g4_kernel::test_support::Synthetic, AppState) {
        let (f, state) = open_synthetic();
        apply_header_patch_impl(&state).unwrap();
        (f, state)
    }

    fn new_header(template: u16) -> Option<NewHeader> {
        Some(NewHeader {
            template,
            internal_name: "T_NEW".into(),
        })
    }

    fn header_count(state: &AppState) -> usize {
        list_headers_impl(state).unwrap().headers.len()
    }

    #[test]
    fn new_header_deserializes_when_absent() {
        let a: AssignHeader =
            serde_json::from_str(r#"{"id":1,"area_data_id":null,"event_file":"Keep"}"#).unwrap();
        assert!(a.new_header.is_none());
        let a: AssignHeader = serde_json::from_str(
            r#"{"id":2,"area_data_id":null,"event_file":"New",
                "new_header":{"template":1,"internal_name":"T_NEW"}}"#,
        )
        .unwrap();
        let n = a.new_header.unwrap();
        assert_eq!((n.template, n.internal_name.as_str()), (1, "T_NEW"));
    }

    #[test]
    fn check_assign_refuses_new_header_with_wrong_id() {
        let (_f, state) = patched();
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 5,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("0002"), "{}", err.message);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn check_assign_refuses_new_header_without_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("patch"), "{}", err.message);
        assert_eq!(counts(&state), before);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn check_assign_refuses_a_template_the_game_lacks() {
        let (_f, state) = patched();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(7),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("header 7"), "{}", err.message);
        assert_eq!(counts(&state), before);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn create_area_with_new_header_creates_it_first() {
        let (f, state) = patched();
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.header, report.matrix), (Some(2), 1));
        assert_eq!(header_count(&state), 3);
        let h = get_header_impl(&state, 2).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!(m.headers, Some(vec![2, 2]));
        // The record in mapname.bin is the name, NUL padded.
        let names =
            std::fs::read(f.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
        assert_eq!(names.len(), 3 * 16);
        assert_eq!(&names[32..37], b"T_NEW");
        // The new header is not undoable, the two edits (matrix, header) are.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(header_count(&state), 3);
        assert_eq!(get_header_impl(&state, 2).unwrap().matrix_id(), 0);
    }

    #[test]
    fn create_room_with_new_header_uses_the_template_event_file_on_keep() {
        let (_f, state) = patched();
        set_event_file(&state, 1, 1);
        let mut spec = room_spec(EventFileChoice::Keep);
        spec.header.id = 2;
        spec.header.new_header = new_header(1);
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(header_count(&state), 3);
        let h = get_header_impl(&state, 2).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (report.matrix, 1));
        assert_eq!(report.event_file, None);
        assert!(report.outside_warp.is_some());
    }

    #[test]
    fn a_failure_after_the_new_header_is_partial_and_names_it_first() {
        let created = Created {
            headers: vec![2],
            ..Created::default()
        };
        let err = late(&created, "map copy", "boom");
        assert_eq!(err.code, "partial");
        assert_eq!(
            err.message,
            "header 0002 was created; map copy failed: boom"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_real_failure_after_the_new_header_leaves_it_and_says_so() {
        use std::os::unix::fs::PermissionsExt;
        let (f, state) = patched();
        // Unpack the map archive, then make it read-only: the map copy right after the
        // header creation cannot write.
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        let maps = f.dir.path().join("unpacked/maps");
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = maps.join("probe");
        if std::fs::write(&probe, b"").is_ok() {
            // Running as a user the mode does not bind (root): the failure cannot be made.
            std::fs::remove_file(probe).unwrap();
            return;
        }
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(err.code, "partial");
        assert!(
            err.message
                .starts_with("header 0002 was created; map copy failed"),
            "{}",
            err.message
        );
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn a_failure_after_creating_is_partial_and_one_before_is_a_plain_error() {
        let created = Created {
            maps: vec![3],
            ..Created::default()
        };
        let err = late(&created, "header edit", "boom");
        assert_eq!(err.code, "partial");
        assert_eq!(
            err.message,
            "map 0003 was created; header edit failed: boom"
        );
        assert!(err.asset.is_none());
        let err = late(&Created::default(), "map copy", "boom");
        assert_eq!(err.code, "error");
        assert_eq!(err.message, "nothing was created; map copy failed: boom");
    }

    #[test]
    fn event_file_choices_deserialize_externally_tagged() {
        let keep: EventFileChoice = serde_json::from_str(r#""Keep""#).unwrap();
        let new: EventFileChoice = serde_json::from_str(r#""New""#).unwrap();
        let copy: EventFileChoice = serde_json::from_str(r#"{"Copy":5}"#).unwrap();
        assert_eq!(
            (keep, new, copy),
            (
                EventFileChoice::Keep,
                EventFileChoice::New,
                EventFileChoice::Copy(5)
            )
        );
    }
}
