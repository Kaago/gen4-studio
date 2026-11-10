//! „Gebiet anlegen“: a matrix of copies of maps (or the copies placed into free cells of a
//! world map) and, if asked, a header pointed at it. Also home of the header assignment that
//! „Raum anlegen“ shares: the new header, its event file, its place and the Dynamic Headers
//! patch a new header may need.

use std::collections::HashSet;

use g4_formats::matrix::{MATRIX_EMPTY, resize_matrix as resized};
use serde::{Deserialize, Serialize};

use super::types::{
    Created, Draft, DraftCell, Outcome, cell_index, check_header_id, check_member, check_name,
    check_writable, id, late, next_steps, refused,
};
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::places::{self, NewPlace};
use crate::project::AssetKind;
use crate::store::{MapHeader, Store, Tx};

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
    pub(super) fn source_header(&self) -> u16 {
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

/// Event file `index` exists, can be loaded and is editable.
pub(super) fn check_event_file_writable(store: &mut Store, index: u16) -> Result<(), KernelError> {
    check_member(store, AssetKind::Event, index, "event file")?;
    store.event(index)?;
    check_writable(store, id(AssetKind::Event, index))
}

/// Everything about the header assignment that can be known without writing: the header
/// exists and the table can be edited, the area data and the event file to copy exist.
/// `matrix` is the existing matrix the header will point at, `None` for a new one.
pub(super) fn check_assign(
    store: &mut Store,
    assign: &AssignHeader,
    matrix: Option<u16>,
) -> Result<(), KernelError> {
    match &assign.new_header {
        None => check_header_id(store, assign.id)?,
        Some(new) => {
            if !store.dynamic_headers_applied()? {
                if !assign.allow_patch {
                    return Err(refused(
                        "new headers need the dynamic headers patch, apply the patch first",
                    ));
                }
                if dynamic_headers::plan(store.project())?.is_none() {
                    return Err(refused(
                        "new headers need the dynamic headers patch, which this game does not \
                         have",
                    ));
                }
            }
            // The store's view: the count includes headers created earlier in this session.
            let count = store.header_table()?.len();
            if usize::from(assign.id) != count {
                return Err(refused(format!(
                    "a new header gets id {count:04}, not {:04}",
                    assign.id
                )));
            }
            check_header_id(store, new.template)?;
        }
    }
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

/// The internal name a new header gets: its own, or without one [`auto_internal_name`] of
/// `base`.
fn new_header_name(store: &Store, new: &NewHeader, base: &str) -> Result<String, KernelError> {
    match new.internal_name.as_deref() {
        Some(name) if !name.trim().is_empty() => Ok(name.to_owned()),
        _ => Ok(auto_internal_name(&store.project().internal_names()?, base)),
    }
}

/// Whether the run will apply the Dynamic Headers patch: the assignment creates a header,
/// allows the patch and the game lacks it.
fn needs_patch(store: &Store, assign: &AssignHeader) -> Result<bool, KernelError> {
    Ok(assign.new_header.is_some() && assign.allow_patch && !store.dynamic_headers_applied()?)
}

/// Applies the Dynamic Headers patch when [`needs_patch`]: the first write of a run, after
/// every check (the place's too, see [`plan_place`]), so a refusal never leaves the patch in
/// and a failing patch leaves nothing behind; once it is in, it stays ([`Created::patch`]).
/// Answers the snapshot of an older header folder (see [`Store::apply_dynamic_headers`]).
pub(super) fn provide_patch(
    store: &mut Store,
    assign: &AssignHeader,
    created: &mut Created,
) -> Result<Option<String>, KernelError> {
    if needs_patch(store, assign)? {
        let snapshot = store.apply_dynamic_headers()?;
        created.patch = true;
        return Ok(snapshot.map(|p| p.to_string_lossy().into_owned()));
    }
    Ok(None)
}

/// The place the header gets, checked: an existing index, or a new name and where it goes.
#[derive(Debug, Clone, Copy)]
pub(super) enum PlannedPlace {
    Existing(u8),
    New(NewPlace),
}

impl PlannedPlace {
    pub(super) fn index(self) -> u8 {
        match self {
            PlannedPlace::Existing(index) => index,
            PlannedPlace::New(plan) => plan.index,
        }
    }
}

/// Checks the place of the assignment (the refusals of the place commands), changing
/// nothing; one of the checks before the patch and the members.
pub(super) fn plan_place(
    store: &mut Store,
    assign: &AssignHeader,
) -> Result<Option<PlannedPlace>, KernelError> {
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
pub(super) fn provide_header(
    store: &mut Store,
    assign: &AssignHeader,
    base: &str,
    created: &mut Created,
) -> Result<(), KernelError> {
    if let Some(new) = &assign.new_header {
        let name =
            new_header_name(store, new, base).map_err(|e| late(created, "header creation", e))?;
        let header = store
            .create_header(new.template, &name)
            .map_err(|e| late(created, "header creation", e))?;
        created.headers.push(header);
    }
    Ok(())
}

/// Creates the event file the choice asks for (`Keep` creates none).
pub(super) fn provide_event_file(
    store: &mut Store,
    choice: EventFileChoice,
    created: &mut Created,
) -> Result<Option<u16>, KernelError> {
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
/// and gives it the planned place, a new place's name in location bank `bank` included: edits
/// of the recipe's one transaction.
pub(super) fn assign_header(
    tx: &mut Tx<'_>,
    bank: u16,
    assign: &AssignHeader,
    matrix: u16,
    event_file: Option<u16>,
    place: Option<PlannedPlace>,
) -> Result<(), KernelError> {
    if let (Some(PlannedPlace::New(plan)), Some(PlaceChoice::New(name))) = (place, &assign.place) {
        places::apply_new_place(tx, bank, &plan, name)?;
    }
    tx.edit_header(assign.id, |header| {
        apply_assign(header, assign, matrix, event_file);
        if let Some(place) = place {
            places::set_location_name(header, place.index());
        }
        Ok(())
    })
}

/// Dry run of the header edit: the new matrix (unless `matrix` names an existing one) and event
/// file will be the next members of their archives, so the final header is known before
/// anything is created. Refuses a header the game's layout could not store (the three fields
/// are full-width in every layout, so this only guards against a layout change).
fn check_header_fits(
    store: &mut Store,
    assign: &AssignHeader,
    matrix: Option<u16>,
) -> Result<(), KernelError> {
    let next = |kind| -> Result<u16, KernelError> {
        let count = store.member_count(kind)?;
        u16::try_from(count)
            .map_err(|_| refused(format!("the {kind} archive has no room for a new member")))
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
        .ok_or_else(|| refused(format!("header {source} does not exist")))?;
    apply_assign(&mut header, assign, matrix, event_file);
    if !store.header_fits(&header) {
        return Err(refused(format!(
            "header {} would not fit the game's header layout with matrix {matrix}{}",
            assign.id,
            event_file.map_or(String::new(), |f| format!(" and event file {f}"))
        )));
    }
    Ok(())
}

/// „1 map“, „2 maps“.
pub(super) fn maps_text(n: usize) -> String {
    if n == 1 {
        t!("recipes.maps_one").into_owned()
    } else {
        t!("recipes.maps_other", n = n).into_owned()
    }
}

/// The sentences of a draft about the header assignment: the new header, where it points,
/// its event file and its place. `matrix` is the existing matrix it will point at, `None` for
/// a new one.
pub(super) fn assign_notes(
    store: &mut Store,
    assign: &AssignHeader,
    base: &str,
    matrix: Option<u16>,
) -> Result<Vec<String>, KernelError> {
    let id = assign.id;
    let mut notes = Vec::new();
    if let Some(new) = &assign.new_header {
        let name = new_header_name(store, new, base)?;
        notes.push(
            t!(
                "recipes.note_new_header",
                id = id,
                name = name,
                template = new.template
            )
            .into_owned(),
        );
    }
    notes.push(match matrix {
        Some(m) => t!("recipes.note_header_matrix", id = id, matrix = m : {:04}).into_owned(),
        None => t!("recipes.note_header_new_matrix", id = id).into_owned(),
    });
    match assign.event_file {
        EventFileChoice::Keep => {}
        EventFileChoice::New => {
            notes.push(t!("recipes.note_header_empty_event_file", id = id).into_owned());
        }
        EventFileChoice::Copy(n) => notes.push(
            t!("recipes.note_header_event_file_copy", id = id, event_file = n : {:04}).into_owned(),
        ),
    }
    match &assign.place {
        None => {}
        Some(PlaceChoice::Existing(i)) => {
            let names = places::location_names(store)?;
            let name = names.get(usize::from(*i)).map_or("", String::as_str);
            notes.push(t!("recipes.note_header_place", id = id, name = name).into_owned());
        }
        Some(PlaceChoice::New(name)) => notes
            .push(t!("recipes.note_header_new_place", id = id, name = name.trim()).into_owned()),
    }
    Ok(notes)
}

/// What the assignment creates before the step, in the order a run creates it: the patch and
/// the new header.
pub(super) fn assign_irreversible(
    store: &Store,
    assign: &AssignHeader,
) -> Result<Vec<String>, KernelError> {
    let mut out = Vec::new();
    if needs_patch(store, assign)? {
        out.push(t!("recipes.irreversible_patch_applied").into_owned());
    }
    if assign.new_header.is_some() {
        out.push(t!("recipes.irreversible_header_created", id = assign.id).into_owned());
    }
    Ok(out)
}

/// The event file the assignment creates, as a sentence of [`Draft::irreversible`].
pub(super) fn event_file_irreversible(choice: EventFileChoice) -> Option<String> {
    match choice {
        EventFileChoice::Keep => None,
        EventFileChoice::New => Some(t!("recipes.irreversible_empty_event_file").into_owned()),
        EventFileChoice::Copy(n) => {
            Some(t!("recipes.irreversible_event_file_copied", event_file = n : {:04}).into_owned())
        }
    }
}

/// What the checks of an area found: the landscape cells it replaces and its place.
struct Checked {
    replaced: u16,
    place: Option<PlannedPlace>,
}

/// Every check before the first write: the spec, the target, the header and the place.
fn check(store: &mut Store, spec: &AreaSpec) -> Result<Checked, KernelError> {
    check_area(store, spec)?;
    let replaced = check_target(store, spec)?;
    let place = match &spec.header {
        Some(assign) => plan_place(store, assign)?,
        None => None,
    };
    Ok(Checked { replaced, place })
}

fn check_area(store: &mut Store, spec: &AreaSpec) -> Result<(), KernelError> {
    if spec.width == 0 || spec.height == 0 {
        return Err(refused("an area needs at least one cell"));
    }
    check_name(&spec.name)?;
    let mut seen = HashSet::new();
    for c in &spec.cells {
        if c.x >= spec.width || c.y >= spec.height {
            return Err(refused(format!(
                "cell ({}, {}) is outside the {} x {} area",
                c.x, c.y, spec.width, spec.height
            )));
        }
        if !seen.insert((c.x, c.y)) {
            return Err(refused(format!("cell ({}, {}) is given twice", c.x, c.y)));
        }
        check_member(store, AssetKind::Map, c.source_map, "map")?;
    }
    if let Some(assign) = &spec.header {
        check_assign(store, assign, spec.target.map(|t| t.matrix))?;
    }
    Ok(())
}

/// Checks the target of an area on a world map, changing nothing: the matrix exists, can be
/// edited and has a header section, and the block of `width` x `height` cells at the target
/// lies inside it on empty cells and landscape (a map with header 0) only. Answers how many
/// landscape cells the area's maps replace (0 without a target).
fn check_target(store: &mut Store, spec: &AreaSpec) -> Result<u16, KernelError> {
    let Some(target) = spec.target else {
        return Ok(0);
    };
    check_member(store, AssetKind::Matrix, target.matrix, "matrix")?;
    let replaced = {
        let m = store.matrix(target.matrix)?;
        let Some(headers) = &m.headers else {
            return Err(refused(t!(
                "recipes.matrix_not_world_map",
                matrix = target.matrix : {:04}
            )));
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
            return Err(refused(t!(
                "recipes.cells_not_free",
                x1 = target.x,
                x2 = x2,
                y1 = target.y,
                y2 = y2
            )));
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
    u16::try_from(replaced).map_err(|_| refused("the area has too many cells"))
}

/// The label of the area's undo step.
fn label(spec: &AreaSpec) -> String {
    t!("recipes.area_created", name = spec.name).into_owned()
}

/// What [`apply`] would do, found with the same checks and without writing: a cell per
/// [`AreaCell`] (on the target world map, else on the new matrix: `matrix` `None`), the
/// sentences of the wizard and what undo would not take back.
pub fn plan(store: &mut Store, spec: &AreaSpec) -> Result<Draft, KernelError> {
    let checked = check(store, spec)?;
    let mut draft = Draft {
        cells: spec
            .cells
            .iter()
            .map(|c| match spec.target {
                Some(t) => DraftCell {
                    matrix: Some(t.matrix),
                    x: t.x + c.x,
                    y: t.y + c.y,
                    source_map: c.source_map,
                },
                None => DraftCell {
                    matrix: None,
                    x: c.x,
                    y: c.y,
                    source_map: c.source_map,
                },
            })
            .collect(),
        ..Draft::default()
    };
    let n = spec.cells.len();
    draft
        .notes
        .push(t!("recipes.note_area", name = spec.name, maps = maps_text(n)).into_owned());
    match spec.target {
        Some(target) => {
            draft.notes.push(
                t!(
                    "recipes.note_area_position",
                    matrix = target.matrix : {:04},
                    x = target.x,
                    y = target.y
                )
                .into_owned(),
            );
            if checked.replaced > 0 {
                draft.notes.push(if checked.replaced == 1 {
                    t!("recipes.note_replaces_landscape_one").into_owned()
                } else {
                    t!(
                        "recipes.note_replaces_landscape_other",
                        n = checked.replaced
                    )
                    .into_owned()
                });
            }
        }
        None => draft.notes.push(
            t!(
                "recipes.note_area_own_matrix",
                width = spec.width,
                height = spec.height
            )
            .into_owned(),
        ),
    }
    if let Some(assign) = &spec.header {
        let matrix = spec.target.map(|t| t.matrix);
        draft
            .notes
            .extend(assign_notes(store, assign, &spec.name, matrix)?);
        draft
            .irreversible
            .extend(assign_irreversible(store, assign)?);
    }
    match n {
        0 => {}
        1 => draft.irreversible.push(
            t!("recipes.irreversible_map_copied", map = spec.cells[0].source_map : {:04})
                .into_owned(),
        ),
        n => draft
            .irreversible
            .push(t!("recipes.irreversible_maps_copied", n = n).into_owned()),
    }
    if spec.target.is_none() {
        draft
            .irreversible
            .push(t!("recipes.irreversible_new_matrix").into_owned());
    }
    if let Some(assign) = &spec.header {
        draft
            .irreversible
            .extend(event_file_irreversible(assign.event_file));
    }
    Ok(draft)
}

/// Creates a matrix of `width` x `height` from copies of maps and, if asked, points a header at
/// it. With a target the copies go into the cells of that world map instead (no new matrix,
/// `with_headers` ignored): each filled cell gets its copy and the header (0, landscape,
/// without one), its altitude stays. Checks everything before it writes; the copies, the
/// matrix and the event file are new members that stay when a later edit fails, the edits
/// (matrix, header and place) are one undo step `Gebiet „…“ angelegt`.
pub fn apply(store: &mut Store, spec: &AreaSpec) -> Result<Outcome<AreaReport>, KernelError> {
    let Checked { replaced, place } = check(store, spec)?;
    let generation = store.change_generation();

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
    let bank = places::location_bank(store.project());
    let ((), change) = store.transaction(&label(spec), |tx| {
        tx.edit_matrix(matrix, |m| {
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
            assign_header(tx, bank, assign, matrix, event_file, place)
                .map_err(|e| late(&created, "header edit", e))?;
        }
        Ok(())
    })?;
    Ok(Outcome {
        report: AreaReport {
            matrix,
            maps,
            event_file,
            header: header_id,
            patched: created.patch,
            snapshot,
            place: place.map(PlannedPlace::index),
            replaced,
        },
        created: created.objects(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&["connect_doors", "new_figure"]),
    })
}
