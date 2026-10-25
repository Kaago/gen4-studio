//! World templates: recipes that reshape a vanilla world. The empty region keeps the start
//! town and the rooms reached from it, empties every other header and fills matrix 0 around
//! the town with one blocked border map; a start script in the town gives the starter. This
//! module reads the recipe, plans it ([`plan_empty_region`], changes nothing) and applies it
//! as one store step ([`apply_empty_region`]). See `docs/formats/templates.md`.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::OnceLock;

use g4_formats::error::FormatError;
use g4_formats::game::{Family, Language};
use g4_formats::matrix::MATRIX_EMPTY;
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, guard_panic, io_at};
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, LevelScriptDoc, ScriptSource, SourceKind};
use crate::start::DP_LOCKED;
use crate::store::{ChangeId, EventFile, HEADER_TABLE, MapHeader, Store};

const TEMPLATES_YAML: &str = include_str!("../metadata/templates.yaml");
/// The start script of each family, with the placeholders `{n}` (script number), `{species}`,
/// `{level}`, `{done_var}` and `{message}`.
const START_PT: &str = include_str!("../metadata/templates/start_pt.rotom");
const START_HGSS: &str = include_str!("../metadata/templates/start_hgss.rotom");

/// The label of the store step [`apply_empty_region`] records.
pub const REGION_LABEL: &str = "Vorlage: Leere Region mit Startstadt";

/// Collision byte of a blocked cell; a map whose cells all carry it (with kind 0) is a border
/// map of the empty region.
const BLOCKED: u8 = 0x80;
/// Collision cells of a map (32 × 32).
const MAP_CELLS: usize = 1024;

/// The empty-region recipe of one family, as `templates.yaml` names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegionRecipe {
    /// Internal name (`mapname.bin`) of the start town's header.
    pub start_town: String,
    pub starter_species: u16,
    pub starter_level: u8,
    /// Variable (command database name) the start script sets once it ran.
    pub done_var: String,
    /// The start script's message for German games.
    pub message_de: String,
    /// The start script's message for every other language.
    pub message_en: String,
}

#[derive(Debug, Deserialize)]
struct TemplatesYaml {
    empty_region: EmptyRegionYaml,
}

#[derive(Debug, Deserialize)]
struct EmptyRegionYaml {
    pt: FamilyYaml,
    hgss: FamilyYaml,
    message: MessageYaml,
}

#[derive(Debug, Deserialize)]
struct FamilyYaml {
    start_town: String,
    starter: StarterYaml,
    done_var: String,
}

#[derive(Debug, Deserialize)]
struct StarterYaml {
    species: u16,
    level: u8,
}

#[derive(Debug, Deserialize)]
struct MessageYaml {
    de: String,
    en: String,
}

/// `templates.yaml`, parsed once.
fn templates() -> Result<&'static TemplatesYaml, KernelError> {
    static TABLE: OnceLock<Result<TemplatesYaml, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            serde_yaml::from_str(TEMPLATES_YAML).map_err(|e| format!("templates.yaml: {e}"))
        })
        .as_ref()
        .map_err(|e| KernelError::InvalidProject(e.clone()))
}

#[derive(Debug, Deserialize)]
struct VarEntry {
    id: u16,
}

#[derive(Debug, Deserialize)]
struct VarTable {
    vars: HashMap<String, VarEntry>,
}

/// Number of the variable `name` in the bundled command database of `family`
/// (`scripts::scrcmd_database`); `None` when the database does not name it.
pub(crate) fn variable_id(family: Family, name: &str) -> Option<u16> {
    static TABLES: [OnceLock<Option<VarTable>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    TABLES[slot]
        .get_or_init(|| serde_json::from_str(scripts::scrcmd_database(family).1).ok())
        .as_ref()?
        .vars
        .get(name)
        .map(|v| v.id)
}

/// The empty-region recipe of `family`. Diamond and Pearl are refused with [`DP_LOCKED`]; a
/// `done_var` the command database does not know is a metadata error.
pub fn region_recipe(family: Family) -> Result<RegionRecipe, KernelError> {
    let yaml = &templates()?.empty_region;
    let game = match family {
        Family::Dp => return Err(KernelError::Refused(DP_LOCKED.into())),
        Family::Pt => &yaml.pt,
        Family::Hgss => &yaml.hgss,
    };
    if variable_id(family, &game.done_var).is_none() {
        return Err(KernelError::InvalidProject(format!(
            "templates.yaml: the command database has no variable {}",
            game.done_var
        )));
    }
    Ok(RegionRecipe {
        start_town: game.start_town.clone(),
        starter_species: game.starter.species,
        starter_level: game.starter.level,
        done_var: game.done_var.clone(),
        message_de: yaml.message.de.clone(),
        message_en: yaml.message.en.clone(),
    })
}

/// What the empty-region recipe would do to the store's world (unsaved edits included).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RegionPlan {
    /// The start town's header.
    pub start_header: u16,
    /// The town's in-game location name, for example "Zweiblattdorf" (empty when the text
    /// bank does not name it).
    pub town_name: String,
    /// `(x, y)` of the town's cells in matrix 0, row by row.
    pub town_cells: Vec<(u8, u8)>,
    /// Map of the first town cell: the source of the border map.
    pub town_map: u16,
    /// Altitude of the first town cell, which every border cell gets.
    pub town_altitude: u8,
    pub town_event_file: u16,
    pub town_script_file: u16,
    /// The town and the headers reached from it through warps without entering another
    /// header of matrix 0, ascending.
    pub kept: Vec<u16>,
    /// Headers that get event file 0 and the empty level script, ascending; headers that
    /// already have both are not listed.
    pub emptied: Vec<u16>,
    /// Cells of matrix 0 with a map outside the town: they get the border map.
    pub border_cells: usize,
    /// The level script every emptied header gets.
    pub empty_level_script: u16,
    /// `(event file, warp index)` of the warps of kept event files that lead into a header
    /// that is not kept; they are removed.
    pub dropped_warps: Vec<(u16, u16)>,
    /// Figures (object events) of the town's event file, removed.
    pub removed_objects: usize,
    /// Triggers (coord events) of the town's event file, removed.
    pub removed_triggers: usize,
    /// Species the start script gives (the recipe's starter).
    pub starter_species: u16,
    /// Level of that Pokémon.
    pub starter_level: u8,
    /// The world already looks like the recipe's result: every cell of matrix 0 with a map
    /// carries the town's header, the cells outside the town share one map, and the town has
    /// no figures.
    pub already_empty: bool,
}

/// Plans the empty region on the store's world; reads, never edits (maps, matrix 0, event
/// files and the header table may be loaded). Refused: Diamond/Pearl ([`DP_LOCKED`]), a
/// start town that is no header of matrix 0 or has no cell there, and a project without an
/// empty level script (a script member of exactly four zero bytes).
///
/// Town cells are the cells of matrix 0 with the town's header and a map, except cells whose
/// map is a border map (every collision cell `{kind 0, collision 0x80}`), so a world the recipe
/// already emptied still shows the town apart from its border.
pub fn plan_empty_region(store: &mut Store) -> Result<RegionPlan, KernelError> {
    let recipe = region_recipe(store.project().family)?;
    let headers = store.header_table()?.to_vec();
    let listing = store.project().header_listing_for(&headers)?.headers;
    let town = listing
        .iter()
        .find(|h| h.internal_name == recipe.start_town && h.matrix_id == 0)
        .ok_or_else(|| {
            KernelError::Refused(format!("start town {} not found", recipe.start_town))
        })?;
    let start_header = town.id;
    let town_name = town.location_name.clone();
    let town_event_file = town.event_file_id;
    let town_script_file = town.script_file_id;

    // Cells of matrix 0: (x, y, map, header) of every cell with a map.
    let matrix = store.matrix(0)?.clone();
    let width = usize::from(matrix.width);
    let cells: Vec<(u8, u8, u16, Option<u16>)> = matrix
        .maps
        .iter()
        .enumerate()
        .filter(|&(_, &map)| map != MATRIX_EMPTY)
        .map(|(i, &map)| {
            let header = matrix.headers.as_ref().and_then(|h| h.get(i).copied());
            // A matrix is at most 255 × 255 cells, so both coordinates fit in u8.
            ((i % width) as u8, (i / width) as u8, map, header)
        })
        .collect();
    let with_town: Vec<_> = cells.iter().filter(|c| c.3 == Some(start_header)).collect();
    if with_town.is_empty() {
        return Err(KernelError::Refused(format!(
            "start town {} has no cell in matrix 0",
            recipe.start_town
        )));
    }
    let mut border_maps = BTreeSet::new();
    for map in with_town.iter().map(|c| c.2).collect::<BTreeSet<_>>() {
        let blocked = is_border_map(&store.map(map)?.permissions);
        if blocked {
            border_maps.insert(map);
        }
    }
    let mut town_part: Vec<_> = with_town
        .iter()
        .filter(|c| !border_maps.contains(&c.2))
        .collect();
    if town_part.is_empty() {
        // Only border maps carry the town: they are the town.
        town_part = with_town.iter().collect();
    }
    let town_cells: Vec<(u8, u8)> = town_part.iter().map(|c| (c.0, c.1)).collect();
    let first = town_part[0];
    let town_map = first.2;
    let town_altitude = matrix
        .altitudes
        .as_ref()
        .and_then(|a| {
            a.get(usize::from(first.1) * width + usize::from(first.0))
                .copied()
        })
        .unwrap_or(0);
    let outside: Vec<_> = cells
        .iter()
        .filter(|c| !town_cells.contains(&(c.0, c.1)))
        .collect();
    let border_cells = outside.len();

    let empty_level_script = empty_level_script(store, &headers)?;

    // Kept: the town plus a breadth-first walk over warps into headers off matrix 0.
    let mut kept = BTreeSet::from([start_header]);
    let mut queue = VecDeque::from([start_header]);
    while let Some(id) = queue.pop_front() {
        let event = headers[usize::from(id)].event_file_id();
        let dests: Vec<u16> = store
            .event(event)?
            .warp_events
            .iter()
            .map(|w| w.dest_header_id)
            .collect();
        for dest in dests {
            let Some(header) = headers.get(usize::from(dest)) else {
                continue;
            };
            if header.matrix_id() != 0 && kept.insert(dest) {
                queue.push_back(dest);
            }
        }
    }

    let emptied: Vec<u16> = (0..headers.len())
        // The table holds at most 65536 headers (checked when it is read).
        .map(|i| i as u16)
        .filter(|id| !kept.contains(id))
        .filter(|&id| {
            let h = &headers[usize::from(id)];
            h.event_file_id() != 0 || h.level_script_id() != empty_level_script
        })
        .collect();

    let kept_events: BTreeSet<u16> = kept
        .iter()
        .map(|&id| headers[usize::from(id)].event_file_id())
        .collect();
    let mut dropped_warps = Vec::new();
    for event in kept_events {
        for (i, warp) in store.event(event)?.warp_events.iter().enumerate() {
            let into_dropped = usize::from(warp.dest_header_id) < headers.len()
                && !kept.contains(&warp.dest_header_id);
            if into_dropped {
                // An event file holds at most u16::MAX warps (its counter is 16 bits wide).
                dropped_warps.push((event, i as u16));
            }
        }
    }

    let town_events = store.event(town_event_file)?;
    let removed_objects = town_events.object_events.len();
    let removed_triggers = town_events.coord_events.len();
    let all_town = cells.iter().all(|c| c.3 == Some(start_header));
    let one_border = outside.windows(2).all(|w| w[0].2 == w[1].2);
    let already_empty = all_town && one_border && removed_objects == 0;

    Ok(RegionPlan {
        start_header,
        town_name,
        town_cells,
        town_map,
        town_altitude,
        town_event_file,
        town_script_file,
        kept: kept.into_iter().collect(),
        emptied,
        border_cells,
        empty_level_script,
        dropped_warps,
        removed_objects,
        removed_triggers,
        starter_species: recipe.starter_species,
        starter_level: recipe.starter_level,
        already_empty,
    })
}

/// Whether a map's collision cells make it a border map: all [`MAP_CELLS`] of them
/// `{kind 0, collision 0x80}` (an empty or short list is none).
fn is_border_map(permissions: &[g4_formats::map::Cell]) -> bool {
    permissions.len() == MAP_CELLS
        && permissions
            .iter()
            .all(|c| c.kind == 0 && c.collision == BLOCKED)
}

/// Whether a level script member is empty: exactly four zero bytes.
fn is_empty_level_script(bytes: &[u8]) -> bool {
    bytes == [0, 0, 0, 0]
}

/// The level script of header 0 when it is empty, else the first empty member of the script
/// archive; refused when there is none.
fn empty_level_script(
    store: &Store,
    headers: &[crate::store::MapHeader],
) -> Result<u16, KernelError> {
    let dir = scripts::ensure_members(store.project())?;
    let read = |n: u16| -> Result<Option<Vec<u8>>, KernelError> {
        let path = dir.join(format!("{n:04}"));
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_at(&path)(e)),
        }
    };
    if let Some(header) = headers.first() {
        let n = header.level_script_id();
        if read(n)?.is_some_and(|b| is_empty_level_script(&b)) {
            return Ok(n);
        }
    }
    for n in scripts::numbered_files(&dir, None)? {
        if read(n)?.is_some_and(|b| is_empty_level_script(&b)) {
            return Ok(n);
        }
    }
    Err(KernelError::Refused(
        "no empty level script (a script member of four zero bytes)".into(),
    ))
}

/// What [`apply_empty_region`] did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RegionReport {
    /// The plan it carried out.
    pub plan: RegionPlan,
    /// The new map of every cell outside the town (written at once, not undoable).
    pub border_map: u16,
    /// The new level script of the town (written at once, not undoable).
    pub level_script: u16,
    /// `(script file, number)` of the start script appended to the town's script file
    /// (written at once, not undoable).
    pub start_script: (u16, u16),
    /// The store step; `None` only when it changed nothing.
    pub change: Option<ChangeId>,
    /// Every asset it changed, in kind and index order: the header table, matrix 0, the
    /// border map and event files of the step, the two script files and the text archives
    /// the compile rewrote (the store loaded them again).
    pub changed: Vec<AssetId>,
}

/// The start script of `family` as script `#n`, filled from `recipe` with `message` as its
/// text literal (a `"` in it is escaped). Diamond and Pearl are refused with [`DP_LOCKED`].
pub fn start_script(
    recipe: &RegionRecipe,
    family: Family,
    n: u16,
    message: &str,
) -> Result<String, KernelError> {
    let template = match family {
        Family::Dp => return Err(KernelError::Refused(DP_LOCKED.into())),
        Family::Pt => START_PT,
        Family::Hgss => START_HGSS,
    };
    // `{message}` goes last, so a brace in the message is never taken for a placeholder.
    Ok(template
        .replace("{n}", &n.to_string())
        .replace("{species}", &recipe.starter_species.to_string())
        .replace("{level}", &recipe.starter_level.to_string())
        .replace("{done_var}", &recipe.done_var)
        .replace("{message}", &message.replace('"', "\\\"")))
}

/// Header `id` of `headers`; past the end is [`FormatError::OutOfRange`].
fn header_mut(headers: &mut [MapHeader], id: u16) -> Result<&mut MapHeader, FormatError> {
    let count = headers.len();
    headers
        .get_mut(usize::from(id))
        .ok_or(FormatError::OutOfRange {
            what: "header",
            value: usize::from(id),
            max: count.saturating_sub(1),
        })
}

/// `message` wrapped like the `text.too-wide` fix does (chatot's `word_wrap` at the text
/// window width, `dialog_line_px`): the game breaks no line by itself, and the recipe's text is
/// one line wider than the window. The `\r` chatot appends at the end is left out.
pub fn wrap_message(message: &str) -> Result<String, KernelError> {
    use chatot::{default_glyph_widths, get_default_charmap, word_wrap};
    let max = Project::limits().dialog_line_px.value;
    // chatot v0.8.1: src/format.rs word_wrap
    let wrapped = guard_panic("chatot", || {
        word_wrap(message, get_default_charmap(), default_glyph_widths(), max)
    })?
    .map_err(|e| KernelError::InvalidProject(format!("start script message: {e}")))?;
    Ok(match wrapped.strip_suffix("\\r") {
        Some(kept) if !message.ends_with("\\r") => kept.to_owned(),
        _ => wrapped,
    })
}

/// Sets the event file (when given) and the level script of `header`.
fn set_events(header: &mut MapHeader, event_file: Option<u16>, level_script: u16) {
    match header {
        MapHeader::DP(h) => {
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
            h.level_script_id = level_script;
        }
        MapHeader::Pt(h) => {
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
            h.level_script_id = level_script;
        }
        MapHeader::HGSS(h) => {
            h.event_file_id = event_file.unwrap_or(h.event_file_id);
            h.level_script_id = level_script;
        }
    }
}

/// The event files of the kept headers as the recipe leaves them, by number: the town's
/// without figures and triggers, the dropped warps removed and the warps into a file that lost
/// warps renumbered. Only files that change are returned.
fn kept_event_files(
    store: &mut Store,
    plan: &RegionPlan,
) -> Result<BTreeMap<u16, EventFile>, KernelError> {
    let headers = store.header_table()?.to_vec();
    let event_of = |id: u16| headers.get(usize::from(id)).map(MapHeader::event_file_id);
    let mut removed: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    for &(event, warp) in &plan.dropped_warps {
        removed.entry(event).or_default().push(warp);
    }
    let mut files = BTreeMap::new();
    for event in plan.kept.iter().filter_map(|&id| event_of(id)) {
        files.insert(event, store.event(event)?.clone());
    }
    let mut changed = BTreeSet::from([plan.town_event_file]);
    for (&event, file) in &mut files {
        for warp in &mut file.warp_events {
            if !plan.kept.contains(&warp.dest_header_id) {
                continue;
            }
            let Some(gone) = event_of(warp.dest_header_id).and_then(|e| removed.get(&e)) else {
                continue;
            };
            // At most u16::MAX warps in a file (its counter is 16 bits wide).
            let shift = gone.iter().filter(|&&i| i < warp.dest_warp_id).count() as u16;
            if shift > 0 {
                warp.dest_warp_id -= shift;
                changed.insert(event);
            }
        }
    }
    for (event, mut warps) in removed {
        if let Some(file) = files.get_mut(&event) {
            warps.sort_unstable();
            for &i in warps.iter().rev() {
                if usize::from(i) < file.warp_events.len() {
                    file.warp_events.remove(usize::from(i));
                }
            }
            changed.insert(event);
        }
    }
    if let Some(town) = files.get_mut(&plan.town_event_file) {
        town.object_events.clear();
        town.coord_events.clear();
    }
    files.retain(|event, _| changed.contains(event));
    Ok(files)
}

/// Applies the empty region to the store's world. Refused, each before anything is written:
/// Diamond/Pearl ([`DP_LOCKED`]), a world that is already empty ("the world is already
/// empty"), a project whose scripts are not adopted ("scripts are not adopted"), unsaved
/// changes in the store ("save first") and a town script file without a rotom source.
///
/// Then, written at once and not undoable: the border map (a copy of the town's map without
/// buildings), a new level script file that starts the start script while the recipe's
/// variable is 0, and the start script appended to the town's script file and compiled (its
/// message goes to the town's text archive, German games get the German text). Last, one store
/// step [`REGION_LABEL`]: the header table (emptied headers get event file 0 and the empty
/// level script, the town its new level script), matrix 0 (every cell outside the town gets
/// the border map, the town's header and its altitude), the border map (every cell blocked)
/// and the kept event files (the town without figures and triggers, the dropped warps gone).
/// Undo takes that step back; the files written before stay, nothing uses them then.
pub fn apply_empty_region(store: &mut Store) -> Result<RegionReport, KernelError> {
    let family = store.project().family;
    let recipe = region_recipe(family)?;
    let plan = plan_empty_region(store)?;
    if plan.already_empty {
        return Err(KernelError::Refused("the world is already empty".into()));
    }
    if !scripts::is_adopted(store.project()) {
        return Err(KernelError::Refused("scripts are not adopted".into()));
    }
    if !store.dirty().is_empty() {
        return Err(KernelError::Refused("save first".into()));
    }
    let var = variable_id(family, &recipe.done_var).ok_or_else(|| {
        KernelError::InvalidProject(format!(
            "templates.yaml: the command database has no variable {}",
            recipe.done_var
        ))
    })?;
    // Everything the steps read, before the first write.
    let town_file = plan.town_script_file;
    let town_source = scripts::rotom_source(store.project(), town_file)?;
    let n = scripts::declared_slots(&town_source)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    let n =
        u16::try_from(n).map_err(|_| KernelError::InvalidProject("no free script slot".into()))?;
    let message = wrap_message(if store.project().language == Language::German {
        &recipe.message_de
    } else {
        &recipe.message_en
    })?;
    let start = start_script(&recipe, family, n, &message)?;
    let level_doc: LevelScriptDoc = serde_json::from_value(serde_json::json!({
        "header_entries": [{ "type": "on_var_condition" }],
        "var_conditions": [{ "var": var, "value": 0, "script_id": n }],
    }))
    .map_err(|e| KernelError::InvalidProject(format!("level script JSON: {e}")))?;
    let events = kept_event_files(store, &plan)?;

    // Written at once.
    let border_map = store.duplicate_map_with(plan.town_map, true)?;
    let (level_script, mut touched) = {
        let project = store.project();
        let (level, mut touched) =
            scripts::new_script_file_reported(project, SourceKind::LevelScript)?;
        scripts::write_source(project, level, &ScriptSource::LevelScript(level_doc))?;
        let report = scripts::compile(project)?;
        touched.extend(&report.touched_archives);
        let errors = scripts::problems_in(project, &report, level, SourceKind::LevelScript);
        if !errors.is_empty() {
            return Err(KernelError::Compile(errors));
        }
        touched.extend(scripts::append_source(
            project,
            town_file,
            town_source,
            &start,
        )?);
        (level, touched)
    };
    touched.sort_unstable();
    touched.dedup();
    for &index in &touched {
        store.reload_from_disk(AssetId {
            kind: AssetKind::TextArchive,
            index,
        })?;
    }

    // The store step.
    let outside: Vec<usize> = {
        let matrix = store.matrix(0)?;
        let width = usize::from(matrix.width);
        matrix
            .maps
            .iter()
            .enumerate()
            .filter(|&(i, &map)| {
                // A matrix is at most 255 × 255 cells (see `plan_empty_region`).
                let cell = ((i % width) as u8, (i / width) as u8);
                map != MATRIX_EMPTY && !plan.town_cells.contains(&cell)
            })
            .map(|(i, _)| i)
            .collect()
    };
    let start_header = plan.start_header;
    let ((), change) = store.transaction(REGION_LABEL, |tx| {
        tx.edit_headers(|headers| {
            for &id in &plan.emptied {
                set_events(header_mut(headers, id)?, Some(0), plan.empty_level_script);
            }
            set_events(header_mut(headers, start_header)?, None, level_script);
            Ok(())
        })?;
        tx.edit_matrix(0, |m| {
            for &i in &outside {
                m.maps[i] = border_map;
                if let Some(h) = m.headers.as_mut() {
                    h[i] = start_header;
                }
                if let Some(a) = m.altitudes.as_mut() {
                    a[i] = plan.town_altitude;
                }
            }
            Ok(())
        })?;
        tx.edit_map(border_map, |m| {
            for cell in &mut m.permissions {
                cell.kind = 0;
                cell.collision = BLOCKED;
            }
            Ok(())
        })?;
        for (&event, file) in &events {
            tx.edit_event(event, |e| {
                *e = file.clone();
                Ok(())
            })?;
        }
        Ok(())
    })?;

    let mut changed = vec![
        HEADER_TABLE,
        AssetId {
            kind: AssetKind::Matrix,
            index: 0,
        },
        AssetId {
            kind: AssetKind::Map,
            index: border_map,
        },
        AssetId {
            kind: AssetKind::Script,
            index: level_script,
        },
        AssetId {
            kind: AssetKind::Script,
            index: town_file,
        },
    ];
    changed.extend(events.keys().map(|&index| AssetId {
        kind: AssetKind::Event,
        index,
    }));
    changed.extend(touched.iter().map(|&index| AssetId {
        kind: AssetKind::TextArchive,
        index,
    }));
    changed.sort_by_key(|id| (id.kind.slot(), id.index));
    changed.dedup();
    Ok(RegionReport {
        plan,
        border_map,
        level_script,
        start_script: (town_file, n),
        change,
        changed,
    })
}

#[cfg(test)]
mod tests {
    use g4_formats::map::Cell;

    use super::*;

    /// Compiles `source` on its own with rotom against the bundled command database of
    /// `family` and the flags and variables it names: an unknown command, a wrong number of
    /// parameters or an unknown name is a compile error. (In a project the names come from
    /// uxie's symbols; rotom's database type leaves the `flags` and `vars` of the JSON out,
    /// so they go in as constants files, `{"<id>": "<name>"}`, one name per id and file.)
    fn compile_with_database(family: Family, source: &str) -> Result<Vec<u8>, String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let (name, json) = scripts::scrcmd_database(family);
        let path = tmp.path().join(name);
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        // rotom v0.7.0: src/database.rs DatabaseV2::load, ConstantDb::new, load_from_db,
        // load_json
        let db = rotom::DatabaseV2::load(&path).map_err(|e| e.to_string())?;
        let mut constants = rotom::ConstantDb::new();
        constants.load_from_db(&db);
        let full: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut files: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        for section in ["flags", "vars"] {
            let entries = full[section].as_object().ok_or(section)?;
            for (symbol, entry) in entries {
                let id = entry["id"].to_string();
                match files.iter_mut().find(|f| !f.contains_key(&id)) {
                    Some(file) => {
                        file.insert(id, symbol.clone().into());
                    }
                    None => files.push(serde_json::Map::from_iter([(id, symbol.clone().into())])),
                }
            }
        }
        for (i, file) in files.into_iter().enumerate() {
            let at = tmp.path().join(format!("symbols{i}.json"));
            std::fs::write(&at, serde_json::Value::Object(file).to_string())
                .map_err(|e| e.to_string())?;
            constants.load_json(&at).map_err(|e| e.to_string())?;
        }
        // rotom v0.7.0: src/lib.rs compile (standalone, no project)
        rotom::compile(source, &db, &constants, rotom::BinaryQuirk::default())
            .map(|compiled| compiled.bytes)
            .map_err(|e| e.to_string())
    }

    /// The start script of `family` filled like `apply_empty_region` fills it (a free slot,
    /// the wrapped message), checked to parse as it is, then with its text literal swapped for
    /// message 0: outside a project rotom cannot lower a literal (it adds it to the map's
    /// text archive), and in one the literal becomes that message's number.
    fn filled_start_script(family: Family, message: &str) -> String {
        let recipe = region_recipe(family).unwrap();
        let wrapped = wrap_message(message).unwrap();
        let source = start_script(&recipe, family, 7, &wrapped).unwrap();
        // rotom v0.7.0: src/compiler/parser.rs Parser::new (strict), parse_script_file
        let parsed = rotom::Parser::new(rotom::Lexer::new(&source)).parse_script_file();
        assert!(parsed.is_ok(), "{family:?}: {parsed:?}\n{source}");
        let literal = format!("\"{}\"", wrapped.replace('"', "\\\""));
        assert_eq!(source.matches(&literal).count(), 1, "{source}");
        source.replace(&literal, "0")
    }

    #[test]
    fn the_start_scripts_compile_against_their_command_database() {
        for family in [Family::Pt, Family::Hgss] {
            let recipe = region_recipe(family).unwrap();
            for message in [&recipe.message_de, &recipe.message_en] {
                let source = filled_start_script(family, message);
                let bytes = compile_with_database(family, &source)
                    .unwrap_or_else(|e| panic!("{family:?}: {e}\n{source}"));
                assert!(!bytes.is_empty(), "{family:?}");
            }
            // The check bites: a command the database does not know, one parameter too many
            // and an unknown variable are refused.
            let source = filled_start_script(family, &recipe.message_de);
            for broken in [
                source.replace("GivePokedex", "GivePokedexTwice"),
                source.replace("GiveRunningShoes", "GiveRunningShoes 1"),
                source.replace("VAR_PLAYER_STARTER", "VAR_NO_SUCH_VARIABLE"),
            ] {
                assert_ne!(broken, source);
                assert!(
                    compile_with_database(family, &broken).is_err(),
                    "{family:?}: {broken}"
                );
            }
        }
    }

    #[test]
    fn a_border_map_has_every_cell_blocked() {
        let blocked = Cell {
            kind: 0,
            collision: BLOCKED,
        };
        assert!(is_border_map(&[blocked; MAP_CELLS]));
        // No cells, or too few, is no border map (all of none would be vacuously blocked).
        assert!(!is_border_map(&[]));
        assert!(!is_border_map(&[blocked; MAP_CELLS - 1]));
        let mut one_open = [blocked; MAP_CELLS];
        one_open[5].collision = 0;
        assert!(!is_border_map(&one_open));
        one_open[5] = Cell {
            kind: 1,
            collision: BLOCKED,
        };
        assert!(!is_border_map(&one_open));
    }
}
