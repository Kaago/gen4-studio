//! „Welt leeren“: the empty region of `templates.yaml` applied to the store's world. The plan
//! is [`crate::templates::plan_empty_region`]; this module checks, drafts and applies it. The
//! border map is created first (it stays after undo); the level script file, the start script
//! and the text archives its compile writes are recorded as file steps beside the store's
//! edits, all one undo step [`REGION_LABEL`], so undo and redo restore every file.

use std::collections::{BTreeMap, BTreeSet};

use g4_formats::error::FormatError;
use g4_formats::game::Language;
use g4_formats::matrix::MATRIX_EMPTY;
use serde::Deserialize;

use super::types::{CreatedObject, Draft, DraftCell, DraftMessage, Outcome, next_steps, refused};
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind};
use crate::scripts::{
    self, ArchiveSnapshot, LevelScriptDoc, ScriptSnapshot, ScriptSource, SourceKind,
};
use crate::store::{ChangeId, EventFile, FileStep, HEADER_TABLE, MapHeader, Store};
use crate::templates::{
    BLOCKED, REGION_LABEL, RegionPlan, RegionReport, plan_empty_region, region_recipe,
    start_script, variable_id, wrap_message,
};

/// The input of „Welt leeren“: none (`recipe_plan`/`recipe_apply` take `{}`).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct EmptyRegionInput {}

/// What [`check`] found, everything [`apply`] needs before its first write.
struct Checked {
    plan: RegionPlan,
    /// The message of the project's language, as `templates.yaml` has it.
    message: String,
    /// Script number of the start script in the town's script file.
    n: u16,
    /// The town's rotom source before the start script is appended.
    town_source: String,
    /// The start script as it is appended.
    start: String,
    /// The town's new level script.
    level_doc: LevelScriptDoc,
    /// The kept event files as the recipe leaves them.
    events: BTreeMap<u16, EventFile>,
    /// `(index, x, y)` of the cells of matrix 0 that get the border map.
    outside: Vec<(usize, u8, u8)>,
}

/// Every check of the recipe, in this order, each before anything is written: Diamond/Pearl
/// ([`crate::start::DP_LOCKED`]) and the refusals of the plan, a world that is already empty,
/// scripts that are not adopted, unsaved changes in the store, a town script file without a
/// rotom source, a message the text window cannot wrap.
fn check(store: &mut Store) -> Result<Checked, KernelError> {
    let family = store.project().family;
    let recipe = region_recipe(family)?;
    let plan = plan_empty_region(store)?;
    if plan.already_empty {
        return Err(refused("the world is already empty"));
    }
    if !scripts::is_adopted(store.project()) {
        return Err(refused("scripts are not adopted"));
    }
    if !store.dirty().is_empty() {
        return Err(refused("save first"));
    }
    let var = variable_id(family, &recipe.done_var).ok_or_else(|| {
        KernelError::InvalidProject(format!(
            "templates.yaml: the command database has no variable {}",
            recipe.done_var
        ))
    })?;
    let town_source = scripts::rotom_source(store.project(), plan.town_script_file)?;
    let n = scripts::declared_slots(&town_source)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    let n =
        u16::try_from(n).map_err(|_| KernelError::InvalidProject("no free script slot".into()))?;
    let message = if store.project().language == Language::German {
        recipe.message_de.clone()
    } else {
        recipe.message_en.clone()
    };
    let start = start_script(&recipe, family, n, &wrap_message(&message)?)?;
    let level_doc: LevelScriptDoc = serde_json::from_value(serde_json::json!({
        "header_entries": [{ "type": "on_var_condition" }],
        "var_conditions": [{ "var": var, "value": 0, "script_id": n }],
    }))
    .map_err(|e| KernelError::InvalidProject(format!("level script JSON: {e}")))?;
    let events = kept_event_files(store, &plan)?;
    let outside = {
        let matrix = store.matrix(0)?;
        let width = usize::from(matrix.width);
        matrix
            .maps
            .iter()
            .enumerate()
            .filter_map(|(i, &map)| {
                // A matrix is at most 255 × 255 cells (see `plan_empty_region`).
                let (x, y) = ((i % width) as u8, (i / width) as u8);
                (map != MATRIX_EMPTY && !plan.town_cells.contains(&(x, y))).then_some((i, x, y))
            })
            .collect()
    };
    Ok(Checked {
        plan,
        message,
        n,
        town_source,
        start,
        level_doc,
        events,
        outside,
    })
}

/// The text archive the start script's message goes to: that of the first header with the
/// town's script file (uxie's rule, see `scripts::project_compile`).
fn message_archive(headers: &[MapHeader], script_file: u16) -> Option<u16> {
    headers
        .iter()
        .find(|h| h.script_file_id() == script_file)
        .map(MapHeader::text_archive_id)
}

/// The text archives the compile of the start script can write, saved before it: the start
/// header's and that of every header with the town's script file (the first of them takes the
/// message), ascending.
pub fn town_archives(store: &mut Store, plan: &RegionPlan) -> Result<Vec<u16>, KernelError> {
    let headers = store.header_table()?;
    let mut archives = BTreeSet::new();
    if let Some(start) = headers.get(usize::from(plan.start_header)) {
        archives.insert(start.text_archive_id());
    }
    archives.extend(
        headers
            .iter()
            .filter(|h| h.script_file_id() == plan.town_script_file)
            .map(MapHeader::text_archive_id),
    );
    Ok(archives.into_iter().collect())
}

/// What [`apply`] would do, found with the same checks and without writing: the border cells
/// of matrix 0 (each a copy of the town's map), the start script's message, the sentences of
/// the plan and the border map undo does not take back.
pub fn plan(store: &mut Store, _: &EmptyRegionInput) -> Result<Draft, KernelError> {
    let checked = check(store)?;
    let plan = &checked.plan;
    let archive = message_archive(store.header_table()?, plan.town_script_file);
    let town = if plan.town_name.is_empty() {
        format!("Header {}", plan.start_header)
    } else {
        format!("{} (Header {})", plan.town_name, plan.start_header)
    };
    let rooms = plan.kept.len().saturating_sub(1);
    Ok(Draft {
        cells: checked
            .outside
            .iter()
            .map(|&(_, x, y)| DraftCell {
                matrix: Some(0),
                x,
                y,
                source_map: plan.town_map,
            })
            .collect(),
        messages: vec![DraftMessage {
            archive,
            text: checked.message.clone(),
        }],
        notes: vec![
            format!(
                "Behält {town} mit {rooms} Räumen; {} Header werden geleert.",
                plan.emptied.len()
            ),
            format!(
                "{} Zellen von Weltkarte 0000 bekommen die Rand-Karte, eine Kopie von Karte \
                 {:04} mit gesperrten Zellen.",
                plan.border_cells, plan.town_map
            ),
            format!(
                "Die Stadt verliert {} Figuren und {} Auslöser; {} Warps in geleerte Header \
                 fallen weg.",
                plan.removed_objects,
                plan.removed_triggers,
                plan.dropped_warps.len()
            ),
            format!(
                "Ein Start-Script gibt Pokémon {} auf Level {}.",
                plan.starter_species, plan.starter_level
            ),
        ],
        irreversible: vec![format!(
            "Karte {:04} wird als Rand-Karte kopiert",
            plan.town_map
        )],
        ..Draft::default()
    })
}

/// Applies the empty region to the store's world, refused by [`check`] before anything is
/// written.
///
/// First the border map (a copy of the town's map without buildings), the one write undo does
/// not take back. Then a new level script file that starts the start script while the
/// recipe's variable is 0, and the start script appended to the town's script file and
/// compiled (its message goes to the text archive of [`message_archive`]); the store loads the
/// touched text archives again. Then one store transaction [`REGION_LABEL`]: the header table
/// (emptied headers get event file 0 and the empty level script, the town its new level
/// script), matrix 0 (every cell outside the town gets the border map, the town's header and
/// its altitude), the border map (every cell blocked) and the kept event files (the town
/// without figures and triggers, the dropped warps gone). Last the file steps of the level
/// script file, the town's script file (sources, member, compile-state entry) and every text
/// archive the compile wrote (JSON and member, from the copies [`town_archives`] names, taken
/// before the first write; an archive outside them falls back to the store's copy, which is
/// the state before the compile) are merged with the transaction into one undo step.
pub fn apply(
    store: &mut Store,
    _: &EmptyRegionInput,
) -> Result<Outcome<RegionReport>, KernelError> {
    let Checked {
        plan,
        n,
        town_source,
        start,
        level_doc,
        events,
        outside,
        ..
    } = check(store)?;
    let generation = store.change_generation();
    let mark = store.undo_depth();
    let town_file = plan.town_script_file;

    // The files as they are before the first write.
    let archives = town_archives(store, &plan)?;
    let (before_town, before_level, next_level, mut saved) = {
        let project = store.project();
        let next_level = scripts::next_script_file(project)?;
        let saved = archives
            .iter()
            .map(|&a| Ok((a, scripts::snapshot_archive(project, a)?)))
            .collect::<Result<BTreeMap<u16, ArchiveSnapshot>, KernelError>>()?;
        (
            scripts::snapshot_script(project, town_file)?,
            scripts::snapshot_script(project, next_level)?,
            next_level,
            saved,
        )
    };

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
    // `new_script_file` numbers the file as `next_script_file` did; another number was free
    // of every file, so nothing was there before.
    let before_level = if level_script == next_level {
        before_level
    } else {
        ScriptSnapshot::default()
    };
    touched.sort_unstable();
    touched.dedup();
    for &index in &touched {
        let id = text_id(index);
        if !saved.contains_key(&index)
            && store.locked(id).is_none()
            && let Some(json) = store.current_bytes(id)?
        {
            // Loaded and saved (no unsaved changes), so the store's copy is the archive
            // before the compile; the member keeps what the compile wrote, the JSON that is
            // put back applies again and the next build encodes it.
            let member = scripts::snapshot_archive(store.project(), index)?.member;
            saved.insert(
                index,
                ArchiveSnapshot {
                    json: Some(json),
                    member,
                },
            );
        }
        store.reload_from_disk(id)?;
    }

    // The store step.
    let start_header = plan.start_header;
    let ((), transaction) = store.transaction(REGION_LABEL, |tx| {
        tx.edit_headers(|headers| {
            for &id in &plan.emptied {
                set_events(header_mut(headers, id)?, Some(0), plan.empty_level_script);
            }
            set_events(header_mut(headers, start_header)?, None, level_script);
            Ok(())
        })?;
        tx.edit_matrix(0, |m| {
            for &(i, _, _) in &outside {
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

    // The file steps, merged with the transaction into one step.
    let steps: Vec<FileStep> = {
        let project = store.project();
        let mut steps = vec![
            scripts::script_file_step(project, level_script, &before_level)?,
            scripts::script_file_step(project, town_file, &before_town)?,
        ];
        for (&index, before) in &saved {
            steps.push(scripts::archive_file_step(project, index, before)?);
        }
        steps
    };
    let mut change: Option<ChangeId> = transaction;
    for step in steps {
        if !step.files.is_empty() || step.rotom_entry.is_some() {
            change = Some(store.record_file_step(REGION_LABEL, step));
        }
    }
    store.squash_since(mark, REGION_LABEL);

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
    changed.extend(touched.iter().map(|&index| text_id(index)));
    changed.sort_by_key(|id| (id.kind.slot(), id.index));
    changed.dedup();
    Ok(Outcome {
        report: RegionReport {
            plan,
            border_map,
            level_script,
            start_script: (town_file, n),
            change,
            changed,
        },
        created: vec![CreatedObject {
            kind: AssetKind::Map,
            index: border_map,
        }],
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[("create_area", "Gebiet anlegen")]),
    })
}

/// `apply(store, &EmptyRegionInput::default())?.report` (what `templates::apply_empty_region`
/// was).
pub fn apply_empty_region(store: &mut Store) -> Result<RegionReport, KernelError> {
    apply(store, &EmptyRegionInput::default()).map(|outcome| outcome.report)
}

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
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
