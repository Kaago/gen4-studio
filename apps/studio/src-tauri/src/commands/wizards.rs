//! Wizard commands: one backend call each for a new area, an extension of an area and a new
//! room with its entrance. Every call checks everything first, then creates the new members
//! (not undoable), then makes the undoable edits. A call that may apply the Dynamic Headers
//! patch does so after the checks, as its first step.

use std::collections::HashSet;

use g4_formats::game::Family;
use g4_formats::map::{Cell, MAP_SIZE};
use g4_formats::matrix::{MATRIX_EMPTY, resize_matrix as resized};
use g4_kernel::dynamic_headers;
use g4_kernel::error::KernelError;
use g4_kernel::places::{self, NewPlace};
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
    /// The name as it is; absent or blank: [`auto_internal_name`] of the area's or room's
    /// name.
    #[serde(default)]
    pub internal_name: Option<String>,
}

/// The place (in-game location name) the assigned header gets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum PlaceChoice {
    /// The place at this index of the location bank.
    Existing(u8),
    /// A new place of this name, made as `create_place` makes it.
    New(String),
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
    /// A new header may apply the Dynamic Headers patch first when the game lacks it (not
    /// undoable); without this flag (absent, `false` or `null`) that is refused.
    #[serde(default, deserialize_with = "null_as_false")]
    pub allow_patch: bool,
    /// The place the header gets, in the same undo step as the assignment; `None` keeps the
    /// one it has.
    #[serde(default)]
    pub place: Option<PlaceChoice>,
}

/// A flag given as `null` counts as `false`, as the MCP access check reads it.
fn null_as_false<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(d)?.unwrap_or(false))
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
    /// Where on an existing world map the area goes; absent or `null`: a matrix of its own.
    #[serde(default)]
    pub target: Option<AreaTarget>,
}

/// The cell (`x`, `y`) of matrix `matrix` (a world map: it has a header section) where the
/// upper left cell of an area goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct AreaTarget {
    pub matrix: u16,
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Serialize)]
pub struct AreaReport {
    pub matrix: u16,
    pub maps: Vec<(u8, u8, u16)>,
    pub event_file: Option<u16>,
    pub header: Option<u16>,
    /// The call applied the Dynamic Headers patch.
    pub patched: bool,
    /// The copy of an older `unpacked/dynamicHeaders` the patch made under `.g4/backup/`.
    pub snapshot: Option<String>,
    /// The place the header got.
    pub place: Option<u8>,
    /// The landscape cells (a map, header 0) of the world map that the area replaced.
    pub replaced: u16,
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
    /// The call applied the Dynamic Headers patch.
    pub patched: bool,
    /// The copy of an older `unpacked/dynamicHeaders` the patch made under `.g4/backup/`.
    pub snapshot: Option<String>,
    /// The place the header got.
    pub place: Option<u8>,
}

/// What a run did so far that undo does not take back: the patch and the members it created
/// (they stay, whatever happens next).
#[derive(Debug, Default)]
pub(crate) struct Created {
    /// The run applied the Dynamic Headers patch.
    pub patch: bool,
    pub headers: Vec<u16>,
    pub matrices: Vec<u16>,
    pub maps: Vec<u16>,
    pub event_files: Vec<u16>,
}

impl Created {
    fn is_empty(&self) -> bool {
        !self.patch
            && self.headers.is_empty()
            && self.matrices.is_empty()
            && self.maps.is_empty()
            && self.event_files.is_empty()
    }
}

/// Names the created members, e.g. "matrix 0001 and maps 0003, 0004 were created", after
/// "the Dynamic Headers patch was applied and stays" when the run applied it.
pub(crate) fn created_text(created: &Created) -> String {
    const PATCH: &str = "the Dynamic Headers patch was applied and stays";
    let members = members_text(created);
    match (created.patch, members) {
        (false, None) => "nothing was created".to_owned(),
        (false, Some(members)) => members,
        (true, None) => PATCH.to_owned(),
        (true, Some(members)) => format!("{PATCH}, {members}"),
    }
}

/// Names the created members, `None` when there are none.
fn members_text(created: &Created) -> Option<String> {
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
        [] => None,
        [only] => Some(format!("{only} {verb} created")),
        [rest @ .., last] => Some(format!("{} and {last} {verb} created", rest.join(", "))),
    }
}

/// The error of a step that failed during a run: says what stays behind. Code `"partial"`
/// once a member was created (the frontend shows what is left to finish by hand), otherwise
/// a plain `"error"`.
pub(super) fn late(created: &Created, step: &str, e: impl std::fmt::Display) -> CmdError {
    let message = format!("{}; {step} failed: {e}", created_text(created));
    if created.is_empty() {
        message.into()
    } else {
        CmdError::partial(message)
    }
}

pub(super) fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

/// A matrix name must fit the file format's length byte.
pub(super) fn check_name(name: &str) -> Result<(), CmdError> {
    if name.len() > usize::from(u8::MAX) {
        return Err(format!("the name is {} bytes long, at most 255 fit", name.len()).into());
    }
    Ok(())
}

pub(super) fn check_member(
    store: &Store,
    kind: AssetKind,
    index: u16,
    what: &str,
) -> Result<(), CmdError> {
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
pub(super) fn check_writable(store: &Store, asset: AssetId) -> Result<(), CmdError> {
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
/// `matrix` is the existing matrix the header will point at, `None` for a new one.
fn check_assign(
    session: &mut Session,
    assign: &AssignHeader,
    matrix: Option<u16>,
) -> Result<(), CmdError> {
    match &assign.new_header {
        None => check_header_id(session, assign.id)?,
        Some(new) => {
            if !session.store.dynamic_headers_applied()? {
                if !assign.allow_patch {
                    return Err(
                        "new headers need the dynamic headers patch, apply the patch first".into(),
                    );
                }
                if dynamic_headers::plan(&session.project)?.is_none() {
                    return Err(
                        "new headers need the dynamic headers patch, which this game \
                                does not have"
                            .into(),
                    );
                }
            }
            let store = &mut session.store;
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
    check_header_fits(store, assign, matrix)
}

/// The longest internal name `mapname.bin` holds.
const INTERNAL_NAME_MAX: usize = 16;

/// The internal name the frontend derives from a typed name (`internalName` in
/// `wizardSpecs.ts`): words joined by `_`, upper case, ASCII letters, digits and `_` only
/// (every other character is dropped), at most 16 bytes, "NEWMAP" when nothing is left.
fn internal_name_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        let c = if c.is_whitespace() { '_' } else { c };
        if !(c.is_ascii_alphanumeric() || c == '_') || (c == '_' && out.ends_with('_')) {
            continue;
        }
        out.push(c.to_ascii_uppercase());
    }
    let out = out.trim_matches('_');
    let out = &out[..out.len().min(INTERNAL_NAME_MAX)];
    if out.is_empty() {
        "NEWMAP".to_owned()
    } else {
        out.to_owned()
    }
}

/// The internal name of a new header named after `base` (an area's or room's name): the
/// frontend's rule, and when a header of `existing` already has it, `_2`, `_3` … appended,
/// the name cut so it stays at 16 bytes (a cut that ends in `_` loses it).
pub fn auto_internal_name(existing: &[String], base: &str) -> String {
    let name = internal_name_of(base);
    let taken = |candidate: &str| existing.iter().any(|e| e == candidate);
    if !taken(&name) {
        return name;
    }
    (2usize..)
        .map(|n| {
            let suffix = format!("_{n}");
            let keep = name.len().min(INTERNAL_NAME_MAX - suffix.len());
            format!("{}{suffix}", name[..keep].trim_end_matches('_'))
        })
        .find(|candidate| !taken(candidate))
        .unwrap_or(name)
}

/// Applies the Dynamic Headers patch when the assignment creates a header, allows the patch
/// and the game lacks it: the first write of a run, after every check (the place's too, see
/// [`plan_place`]), so a refusal never leaves the patch in and a failing patch leaves nothing
/// behind; once it is in, it stays ([`Created::patch`]). Answers the snapshot of an older
/// header folder (see [`Store::apply_dynamic_headers`]).
fn provide_patch(
    store: &mut Store,
    assign: &AssignHeader,
    created: &mut Created,
) -> Result<Option<String>, CmdError> {
    if assign.new_header.is_some() && assign.allow_patch && !store.dynamic_headers_applied()? {
        let snapshot = store.apply_dynamic_headers()?;
        created.patch = true;
        return Ok(snapshot.map(|p| p.to_string_lossy().into_owned()));
    }
    Ok(None)
}

/// The place the header gets, checked: an existing index, or a new name and where it goes.
#[derive(Debug, Clone, Copy)]
enum PlannedPlace {
    Existing(u8),
    New(NewPlace),
}

impl PlannedPlace {
    fn index(self) -> u8 {
        match self {
            PlannedPlace::Existing(index) => index,
            PlannedPlace::New(plan) => plan.index,
        }
    }
}

/// Checks the place of the assignment (the refusals of the place commands), changing
/// nothing; one of the checks before the patch and the members.
fn plan_place(store: &mut Store, assign: &AssignHeader) -> Result<Option<PlannedPlace>, CmdError> {
    let planned = match &assign.place {
        None => return Ok(None),
        Some(PlaceChoice::Existing(index)) => {
            places::check_place(store, *index).map(|()| PlannedPlace::Existing(*index))
        }
        Some(PlaceChoice::New(name)) => places::plan_new_place(store, name).map(PlannedPlace::New),
    };
    Ok(Some(planned?))
}

/// Creates the new header of an assignment, if it asks for one: the first member of a run.
/// Without an internal name it is named after `base` ([`auto_internal_name`]).
fn provide_header(
    store: &mut Store,
    assign: &AssignHeader,
    base: &str,
    created: &mut Created,
) -> Result<(), CmdError> {
    if let Some(new) = &assign.new_header {
        let name = match new.internal_name.as_deref() {
            Some(name) if !name.trim().is_empty() => name.to_owned(),
            _ => {
                let existing = store
                    .project()
                    .internal_names()
                    .map_err(|e| late(created, "header creation", e))?;
                auto_internal_name(&existing, base)
            }
        };
        let header = store
            .create_header(new.template, &name)
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

/// Points header `assign.id` at `matrix` (and at the new area data and event file, if given)
/// and gives it the planned place, a new place's name included: one undo step, labelled
/// `built` with a place ("Gebiet „…“ angelegt"), else "assign header …".
fn assign_header(
    store: &mut Store,
    assign: &AssignHeader,
    matrix: u16,
    event_file: Option<u16>,
    place: Option<PlannedPlace>,
    built: &str,
) -> Result<(), KernelError> {
    let label = match place {
        Some(_) => built.to_owned(),
        None => format!("assign header {:04} to matrix {matrix:04}", assign.id),
    };
    let bank = places::location_bank(store.project());
    store.transaction(&label, |tx| {
        if let (Some(PlannedPlace::New(plan)), Some(PlaceChoice::New(name))) =
            (place, &assign.place)
        {
            places::apply_new_place(tx, bank, &plan, name)?;
        }
        tx.edit_header(assign.id, |header| {
            apply_assign(header, assign, matrix, event_file);
            if let Some(place) = place {
                places::set_location_name(header, place.index());
            }
            Ok(())
        })
    })?;
    Ok(())
}

/// Dry run of the header edit: the new matrix (unless `matrix` names an existing one) and event
/// file will be the next members of their archives, so the final header is known before
/// anything is created. Refuses a header the game's layout could not store (the three fields
/// are full-width in every layout, so this only guards against a layout change).
fn check_header_fits(
    store: &mut Store,
    assign: &AssignHeader,
    matrix: Option<u16>,
) -> Result<(), CmdError> {
    let next = |kind| -> Result<u16, CmdError> {
        let count = store.member_count(kind)?;
        u16::try_from(count)
            .map_err(|_| CmdError::from(format!("the {kind} archive has no room for a new member")))
    };
    let matrix = match matrix {
        Some(matrix) => matrix,
        None => next(AssetKind::Matrix)?,
    };
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
        check_assign(session, assign, spec.target.map(|t| t.matrix))?;
    }
    Ok(())
}

/// Checks the target of an area on a world map, changing nothing: the matrix exists, can be
/// edited and has a header section, and the block of `width` x `height` cells at the target
/// lies inside it on empty cells and landscape (a map with header 0) only. Answers how many
/// landscape cells the area's maps replace (0 without a target).
fn check_target(store: &mut Store, spec: &AreaSpec) -> Result<u16, CmdError> {
    let Some(target) = spec.target else {
        return Ok(0);
    };
    check_member(store, AssetKind::Matrix, target.matrix, "matrix")?;
    let replaced = {
        let m = store.matrix(target.matrix)?;
        let Some(headers) = &m.headers else {
            return Err(KernelError::Refused(format!(
                "Anordnung {:04} ist keine Weltkarte: ihren Zellen sind keine Orte zugeordnet.",
                target.matrix
            ))
            .into());
        };
        // The last column and row of the block; the area has at least one cell.
        let x2 = u16::from(target.x) + u16::from(spec.width) - 1;
        let y2 = u16::from(target.y) + u16::from(spec.height) - 1;
        let free = |x: u16, y: u16| {
            let i = usize::from(y) * usize::from(m.width) + usize::from(x);
            m.maps[i] == MATRIX_EMPTY || headers[i] == 0
        };
        let inside = x2 < u16::from(m.width) && y2 < u16::from(m.height);
        if !inside
            || !(u16::from(target.y)..=y2).all(|y| (u16::from(target.x)..=x2).all(|x| free(x, y)))
        {
            return Err(KernelError::Refused(format!(
                "Die Zellen x {} bis {x2}, y {} bis {y2} sind nicht frei: dort liegen Gebiete \
                 oder sie liegen außerhalb der Weltkarte.",
                target.x, target.y
            ))
            .into());
        }
        // Inside the block every filled cell is landscape.
        spec.cells
            .iter()
            .filter(|c| {
                let i = cell_index(m.width, target.x + c.x, target.y + c.y);
                m.maps[i] != MATRIX_EMPTY
            })
            .count()
    };
    check_writable(store, id(AssetKind::Matrix, target.matrix))?;
    u16::try_from(replaced).map_err(|_| CmdError::from("the area has too many cells"))
}

/// Creates a matrix of `width` x `height` from copies of maps and, if asked, points a header at
/// it. With a target the copies go into the cells of that world map instead (no new matrix,
/// `with_headers` ignored): each filled cell gets its copy and the header (0, landscape,
/// without one), its altitude stays. Checks everything before it writes; the copies, the
/// matrix and the event file are new members that stay when a later edit fails, the edits
/// (matrix, then header) are undoable.
pub fn create_area_impl(state: &AppState, spec: AreaSpec) -> Result<AreaReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    check_area(session, &spec)?;
    let replaced = check_target(&mut session.store, &spec)?;
    let store = &mut session.store;

    let place = match &spec.header {
        Some(assign) => plan_place(store, assign)?,
        None => None,
    };

    let mut created = Created::default();
    let mut snapshot = None;
    if let Some(assign) = &spec.header {
        snapshot = provide_patch(store, assign, &mut created)?;
        provide_header(store, assign, &spec.name, &mut created)?;
    }
    let mut maps = Vec::with_capacity(spec.cells.len());
    for c in &spec.cells {
        let map = store
            .duplicate_map_with(c.source_map, c.clear_buildings)
            .map_err(|e| late(&created, "map copy", e))?;
        created.maps.push(map);
        maps.push((c.x, c.y, map));
    }
    let matrix = match spec.target {
        Some(target) => target.matrix,
        None => {
            let matrix = store
                .create_matrix(&spec.name)
                .map_err(|e| late(&created, "matrix creation", e))?;
            created.matrices.push(matrix);
            matrix
        }
    };
    let event_file = match &spec.header {
        Some(assign) => provide_event_file(store, assign.event_file, &mut created)?,
        None => None,
    };

    let header_id = spec.header.as_ref().map(|a| a.id);
    store
        .edit_matrix(matrix, "build area", |m| {
            if let Some(target) = spec.target {
                for &(x, y, map) in &maps {
                    let i = cell_index(m.width, target.x + x, target.y + y);
                    m.maps[i] = map;
                    if let Some(headers) = m.headers.as_mut() {
                        headers[i] = header_id.unwrap_or(0);
                    }
                }
                return Ok(());
            }
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
        let built = format!("Gebiet „{}“ angelegt", spec.name);
        assign_header(store, assign, matrix, event_file, place, &built)
            .map_err(|e| late(&created, "header edit", e))?;
    }
    Ok(AreaReport {
        matrix,
        maps,
        event_file,
        header: header_id,
        patched: created.patch,
        snapshot,
        place: place.map(PlannedPlace::index),
        replaced,
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
    check_assign(session, &spec.header, None)?;
    if let Some(entrance) = &spec.entrance {
        check_header_id(session, entrance.outside_header)?;
    }
    let plan = match &spec.entrance {
        Some(entrance) => Some(plan_entrance(session, &spec, entrance)?),
        None => None,
    };
    let store = &mut session.store;
    let place = plan_place(store, &spec.header)?;
    // A new header starts with its template's event file.
    let kept_file = header_event_file(store, spec.header.source_header())?;

    let mut created = Created::default();
    let snapshot = provide_patch(store, &spec.header, &mut created)?;
    provide_header(store, &spec.header, &spec.name, &mut created)?;
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
    let built = format!("Raum „{}“ angelegt", spec.name);
    assign_header(store, &spec.header, matrix, event_file, place, &built)
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
        patched: created.patch,
        snapshot,
        place: place.map(PlannedPlace::index),
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
        EventKind, add_event_impl, apply_header_patch_impl, create_matrix_impl, dirty_impl,
        get_events_impl, get_header_impl, get_map_impl, get_matrix_impl, header_patch_status_impl,
        history_impl, list_headers_impl, member_counts_impl, place_slots_impl, reload_asset_impl,
        resize_matrix_impl, set_header_impl, set_matrix_cell_impl, set_matrix_sections_impl,
        set_permission_impl, undo_impl,
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
                allow_patch: false,
                place: None,
            }),
            target: None,
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
                    allow_patch: false,
                    place: None,
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
                    allow_patch: false,
                    place: None,
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
                allow_patch: false,
                place: None,
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
                allow_patch: false,
                place: None,
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
        let mut created = Created {
            patch: false,
            headers: vec![470],
            matrices: vec![1],
            maps: vec![3, 4],
            event_files: vec![2],
        };
        assert_eq!(
            created_text(&created),
            "header 0470, matrix 0001, maps 0003, 0004 and event file 0002 were created"
        );
        created.patch = true;
        assert_eq!(
            created_text(&created),
            "the Dynamic Headers patch was applied and stays, header 0470, matrix 0001, \
             maps 0003, 0004 and event file 0002 were created"
        );
        let patch_only = Created {
            patch: true,
            ..Created::default()
        };
        assert_eq!(
            created_text(&patch_only),
            "the Dynamic Headers patch was applied and stays"
        );
        assert_eq!(late(&patch_only, "place", "boom").code, "partial");
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
            internal_name: Some("T_NEW".into()),
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
        assert_eq!((n.template, n.internal_name.as_deref()), (1, Some("T_NEW")));
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

    /// The new header `2` of the synthetic game as a copy of header 1, with `place` and
    /// `allow_patch`.
    fn area_with_new_header(place: Option<PlaceChoice>, allow_patch: bool) -> AreaSpec {
        AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                place,
                allow_patch,
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        }
    }

    fn summary(state: &AppState, id: u16) -> g4_kernel::project::HeaderSummary {
        list_headers_impl(state).unwrap().headers[usize::from(id)].clone()
    }

    fn patch_applied(state: &AppState) -> bool {
        header_patch_status_impl(state).unwrap().applied
    }

    #[test]
    fn create_area_applies_the_patch_when_allowed() {
        let (f, state) = open_synthetic();
        assert!(!patch_applied(&state));
        // An older header folder is copied away first; the report names the copy.
        let old = f.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("0000"), [1u8; 24]).unwrap();
        let report = create_area_impl(&state, area_with_new_header(None, true)).unwrap();
        assert!(report.patched);
        let snapshot = std::path::PathBuf::from(report.snapshot.unwrap());
        assert!(
            snapshot.starts_with(f.dir.path().join(".g4/backup")),
            "{snapshot:?}"
        );
        assert_eq!(std::fs::read(snapshot.join("0000")).unwrap(), [1u8; 24]);
        assert!(patch_applied(&state));
        assert_eq!(header_count(&state), 3);
        assert_eq!(
            get_header_impl(&state, 2).unwrap().matrix_id(),
            report.matrix
        );
        // With the patch in, `allow_patch` does nothing more.
        let mut again = area_with_new_header(None, true);
        again.header.as_mut().unwrap().id = 3;
        let again = create_area_impl(&state, again).unwrap();
        assert_eq!((again.patched, again.snapshot), (false, None));
    }

    #[test]
    fn create_area_still_refuses_without_allow_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let err = create_area_impl(&state, area_with_new_header(None, false)).unwrap_err();
        assert_eq!(
            err.message,
            "new headers need the dynamic headers patch, apply the patch first"
        );
        assert!(!patch_applied(&state));
        assert_eq!((counts(&state), header_count(&state)), (before, 2));
        // `null` is no permission either.
        let a: AssignHeader = serde_json::from_str(
            r#"{"id":2,"area_data_id":null,"event_file":"New","allow_patch":null}"#,
        )
        .unwrap();
        assert!(!a.allow_patch);
    }

    #[test]
    fn create_area_refuses_the_patch_a_game_does_not_have() {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("header.yaml"),
            "game_title: \"POKEMON D\"\ngamecode: ADAE\nmaker_code: \"01\"\n",
        )
        .unwrap();
        // The fixture's arm9 is sized for Platinum; Diamond/Pearl's table is longer.
        let arm9 = fixture.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes.resize(0xF0000, 0);
        std::fs::write(&arm9, bytes).unwrap();
        let state = AppState::default();
        super::super::open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let before = counts(&state);
        // No cells: the fixture has no Diamond/Pearl map archive to copy from.
        let spec = AreaSpec {
            cells: Vec::new(),
            ..area_with_new_header(None, true)
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert_eq!(err.code, "error");
        assert_eq!(
            err.message,
            "new headers need the dynamic headers patch, which this game does not have"
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn an_invalid_place_is_refused_before_the_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        for (place, refusal) in [
            (
                PlaceChoice::New("  ".into()),
                "Der Ort braucht einen Namen.",
            ),
            (
                PlaceChoice::Existing(9),
                "Den Ort 9 gibt es nicht: die Ortsnamen-Bank hat 3 Namen.",
            ),
        ] {
            let spec = area_with_new_header(Some(place), true);
            let err = create_area_impl(&state, spec).unwrap_err();
            assert_eq!((err.code, err.message.as_str()), ("error", refusal));
            assert!(!patch_applied(&state));
            assert_eq!((counts(&state), header_count(&state)), (before.clone(), 2));
        }
    }

    #[test]
    fn create_area_with_a_new_place_appends_it_and_sets_the_header_in_one_step() {
        let (_f, state) = patched();
        let template = summary(&state, 1).location_index;
        let place = Some(PlaceChoice::New(" Neuheim ".into()));
        let report = create_area_impl(&state, area_with_new_header(place, false)).unwrap();
        assert_eq!((report.place, report.patched), (Some(3), false));
        let h = summary(&state, 2);
        assert_eq!((h.location_index, h.location_name.as_str()), (3, "Neuheim"));
        assert_eq!(place_slots_impl(&state).unwrap().count, 4);
        let undo = history_impl(&state).unwrap().undo;
        assert_eq!(undo.last().unwrap(), "Gebiet „Route 230“ angelegt");
        // One undo takes the name and the place of the header back; the header stays.
        undo_impl(&state).unwrap();
        assert_eq!(place_slots_impl(&state).unwrap().count, 3);
        let h = summary(&state, 2);
        assert_eq!((h.location_index, h.matrix_id), (template, 0));
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn create_room_with_an_existing_place_sets_it() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        let mut spec = room_spec(EventFileChoice::Keep);
        spec.header.place = Some(PlaceChoice::Existing(2));
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(report.place, Some(2));
        let h = summary(&state, 1);
        assert_eq!((h.location_index, h.matrix_id), (2, report.matrix));
        let undo = history_impl(&state).unwrap().undo;
        assert!(
            undo.contains(&"Raum „Haus“ angelegt".to_owned()),
            "{undo:?}"
        );
        assert_eq!(place_slots_impl(&state).unwrap().count, 3);
    }

    #[test]
    fn create_room_refuses_a_place_the_bank_lacks_before_creating_anything() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let mut spec = room_spec(EventFileChoice::New);
        spec.header.place = Some(PlaceChoice::Existing(9));
        let err = create_room_impl(&state, spec).unwrap_err();
        assert_eq!(err.code, "error");
        assert!(
            err.message.contains("Den Ort 9 gibt es nicht"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn names_the_new_header_automatically_and_uniquely() {
        let (f, state) = patched();
        let unnamed = |id: u16| AreaSpec {
            name: "Neuheim".into(),
            header: Some(AssignHeader {
                id,
                new_header: Some(NewHeader {
                    template: 1,
                    internal_name: None,
                }),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        create_area_impl(&state, unnamed(2)).unwrap();
        let mut blank = unnamed(3);
        blank
            .header
            .as_mut()
            .unwrap()
            .new_header
            .as_mut()
            .unwrap()
            .internal_name = Some("  ".into());
        create_area_impl(&state, blank).unwrap();
        assert_eq!(summary(&state, 2).internal_name, "NEUHEIM");
        assert_eq!(summary(&state, 3).internal_name, "NEUHEIM_2");
        let names =
            std::fs::read(f.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
        assert_eq!(&names[48..57], b"NEUHEIM_2");
    }

    #[test]
    fn auto_internal_names_follow_the_frontend_rule_and_stay_within_16_bytes() {
        let none: [String; 0] = [];
        assert_eq!(
            auto_internal_name(&none, " Neues  Gebiet! "),
            "NEUES_GEBIET"
        );
        assert_eq!(
            auto_internal_name(&none, "Zweiblattdorf-Süd"),
            "ZWEIBLATTDORFSD"
        );
        assert_eq!(auto_internal_name(&none, "!!!"), "NEWMAP");
        assert_eq!(auto_internal_name(&none, "a_b c__d_"), "A_B_C_D");
        let long = "ABCDEFGHIJKLMNOPQRS";
        assert_eq!(auto_internal_name(&none, long), "ABCDEFGHIJKLMNOP");
        let taken = ["ABCDEFGHIJKLMNOP".to_owned(), "ABCDEFGHIJKLMN_2".to_owned()];
        assert_eq!(auto_internal_name(&taken, long), "ABCDEFGHIJKLMN_3");
        // A cut that ends in `_` loses it.
        let taken = ["ABCDEFGHIJKLM_OP".to_owned()];
        assert_eq!(
            auto_internal_name(&taken, "ABCDEFGHIJKLM_OP"),
            "ABCDEFGHIJKLM_2"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_after_the_patch_says_the_patch_stays() {
        use std::os::unix::fs::PermissionsExt;
        let (f, state) = open_synthetic();
        // Unpack the map archive, then make it read-only: the map copy after the patch and
        // the header creation cannot write.
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        let maps = f.dir.path().join("unpacked/maps");
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = maps.join("probe");
        if std::fs::write(&probe, b"").is_ok() {
            // Running as a user the mode does not bind (root): the failure cannot be made.
            std::fs::remove_file(probe).unwrap();
            return;
        }
        let err = create_area_impl(&state, area_with_new_header(None, true)).unwrap_err();
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(err.code, "partial");
        assert!(
            err.message.starts_with(
                "the Dynamic Headers patch was applied and stays, header 0002 was created; \
                 map copy failed"
            ),
            "{}",
            err.message
        );
        assert!(patch_applied(&state));
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn old_specs_without_the_new_fields_work_as_before() {
        let (_f, state) = patched();
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 230", "width": 1, "height": 1,
            "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": false }],
            "with_headers": true,
            "header": { "id": 2, "area_data_id": null, "event_file": "New",
                        "new_header": { "template": 1, "internal_name": "T_NEW" } },
        }))
        .unwrap();
        let assign = spec.header.as_ref().unwrap();
        assert_eq!((assign.allow_patch, &assign.place), (false, &None));
        let template = summary(&state, 1).location_index;
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.patched, report.place), (false, None));
        let h = summary(&state, 2);
        assert_eq!(
            (h.internal_name.as_str(), h.location_index),
            ("T_NEW", template)
        );
        let undo = history_impl(&state).unwrap().undo;
        assert_eq!(undo.last().unwrap(), "assign header 0002 to matrix 0001");
        // A new header without an internal name deserializes too.
        let n: NewHeader = serde_json::from_str(r#"{"template":1}"#).unwrap();
        assert_eq!(n.internal_name, None);
        let p: PlaceChoice = serde_json::from_str(r#"{"New":"Neuheim"}"#).unwrap();
        assert_eq!(p, PlaceChoice::New("Neuheim".into()));
        let p: PlaceChoice = serde_json::from_str(r#"{"Existing":3}"#).unwrap();
        assert_eq!(p, PlaceChoice::Existing(3));
    }

    /// A world map as matrix 1 (4 x 2, header section): the area of header 1 at (0, 0),
    /// landscape (map 1, header 0, altitude 5) at (1, 0), the other cells empty.
    fn world_map(state: &AppState) -> u16 {
        let world = create_matrix_impl(state, "Welt").unwrap();
        set_matrix_sections_impl(state, world, true, false).unwrap();
        resize_matrix_impl(state, world, 4, 2).unwrap();
        set_matrix_cell_impl(state, world, 0, 0, Some(0), Some(1), None).unwrap();
        set_matrix_cell_impl(state, world, 1, 0, Some(1), Some(0), Some(5)).unwrap();
        world
    }

    fn on_world(world: u16, x: u8, y: u8) -> AreaSpec {
        AreaSpec {
            target: Some(AreaTarget {
                matrix: world,
                x,
                y,
            }),
            ..area_spec()
        }
    }

    const E: Option<u16> = None;

    #[test]
    fn places_an_area_into_free_cells_of_a_world_map() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let matrices = counts(&state)[&AssetKind::Matrix];
        let report = create_area_impl(&state, on_world(world, 2, 0)).unwrap();
        assert_eq!((report.matrix, report.replaced), (world, 0));
        assert_eq!(report.maps, vec![(0, 0, 3), (1, 0, 4)]);
        assert_eq!((report.event_file, report.header), (Some(2), Some(1)));
        assert_eq!(counts(&state)[&AssetKind::Matrix], matrices);

        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.maps, vec![Some(0), Some(1), Some(3), Some(4), E, E, E, E]);
        assert_eq!(m.headers, Some(vec![1, 0, 1, 1, 0, 0, 0, 0]));
        assert_eq!((m.width, m.height, m.name.as_str()), (4, 2, "Welt"));
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (world, 2));
    }

    #[test]
    fn replaces_landscape_cells_and_counts_them() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let report = create_area_impl(&state, on_world(world, 1, 0)).unwrap();
        assert_eq!(report.replaced, 1);
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.maps, vec![Some(0), Some(3), Some(4), E, E, E, E, E]);
        assert_eq!(m.headers, Some(vec![1, 1, 1, 0, 0, 0, 0, 0]));
        // The altitude of the replaced landscape cell stays.
        assert_eq!(m.altitudes, Some(vec![0, 5, 0, 0, 0, 0, 0, 0]));
    }

    /// The text of a refused block at x `x`..`x2`, y `y`..`y2`.
    fn not_free(x: u8, x2: u16, y: u8, y2: u16) -> String {
        format!(
            "Die Zellen x {x} bis {x2}, y {y} bis {y2} sind nicht frei: dort liegen Gebiete \
             oder sie liegen außerhalb der Weltkarte."
        )
    }

    #[test]
    fn refuses_a_block_over_an_area_before_copying() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = counts(&state);
        let history = history_impl(&state).unwrap().undo.len();
        let err = create_area_impl(&state, on_world(world, 0, 0)).unwrap_err();
        assert_eq!((err.code, err.message), ("error", not_free(0, 1, 0, 0)));
        assert_eq!(counts(&state), before);
        assert_eq!(history_impl(&state).unwrap().undo.len(), history);
        // A block over the area that only leaves the area's cell unfilled is refused too.
        let spec = AreaSpec {
            cells: vec![cell(1, 0, 0, false)],
            ..on_world(world, 0, 0)
        };
        assert!(create_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn refuses_a_block_outside_the_world_map() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = counts(&state);
        let err = create_area_impl(&state, on_world(world, 3, 0)).unwrap_err();
        assert_eq!(err.message, not_free(3, 4, 0, 0));
        let tall = AreaSpec {
            height: 2,
            ..on_world(world, 2, 1)
        };
        let err = create_area_impl(&state, tall).unwrap_err();
        assert_eq!(err.message, not_free(2, 3, 1, 2));
        let far = on_world(world, 255, 255);
        let err = create_area_impl(&state, far).unwrap_err();
        assert_eq!(err.message, not_free(255, 256, 255, 255));
        assert_eq!(counts(&state), before);
        // An unknown matrix is refused as well.
        assert!(create_area_impl(&state, on_world(9, 0, 0)).is_err());
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn refuses_a_target_without_a_header_section() {
        let (_f, state) = open_synthetic();
        resize_matrix_impl(&state, 0, 4, 1).unwrap();
        let before = counts(&state);
        let err = create_area_impl(&state, on_world(0, 2, 0)).unwrap_err();
        assert_eq!(
            err.message,
            "Anordnung 0000 ist keine Weltkarte: ihren Zellen sind keine Orte zugeordnet."
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn one_undo_takes_the_cells_back() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = get_matrix_impl(&state, world).unwrap();
        let maps = counts(&state)[&AssetKind::Map];
        let spec = AreaSpec {
            header: None,
            ..on_world(world, 1, 0)
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.header, report.replaced), (None, 1));
        // Without a header the cells become landscape.
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.headers, Some(vec![1, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(m.maps, vec![Some(0), Some(3), Some(4), E, E, E, E, E]);
        assert_eq!(
            history_impl(&state).unwrap().undo.last().unwrap(),
            "build area"
        );

        assert_eq!(
            undo_impl(&state).unwrap(),
            vec![asset(AssetKind::Matrix, world)]
        );
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!((m.maps, m.headers), (before.maps, before.headers));
        // The copies stay.
        assert_eq!(counts(&state)[&AssetKind::Map], maps + 2);
    }

    #[test]
    fn old_specs_without_target_still_make_their_own_matrix() {
        let (_f, state) = open_synthetic();
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 230", "width": 1, "height": 1,
            "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": false }],
            "with_headers": true, "header": null,
        }))
        .unwrap();
        assert_eq!(spec.target, None);
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.matrix, report.replaced), (1, 0));
        assert_eq!(counts(&state)[&AssetKind::Matrix], 2);
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 231", "width": 1, "height": 1, "cells": [],
            "with_headers": false, "header": null, "target": null,
        }))
        .unwrap();
        assert_eq!(spec.target, None);
        let t: AreaTarget = serde_json::from_str(r#"{"matrix":1,"x":2,"y":3}"#).unwrap();
        assert_eq!(
            t,
            AreaTarget {
                matrix: 1,
                x: 2,
                y: 3
            }
        );
    }
}
