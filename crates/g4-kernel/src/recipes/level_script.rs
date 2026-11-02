//! The level script of a place as the story recipes of 3d.7 change it: „Figur erscheint/
//! verschwindet ab Schritt“ points its entry [`VISIBILITY_ENTRY`] at the visibility block of the
//! place, „Szene beim Betreten“ adds a `var_conditions` entry ([`push_condition`]).
//!
//! A level script file that another header uses as well, or none (`level_script_id` 0, a
//! binary member rotom could not decompile) is not changed in place: [`plan_level_script`]
//! plans a new file for this header (a copy of the shared one, else an empty level script),
//! [`ensure_level_script`] creates it before the recipe's undo step (a new member stays after
//! undo, like the level script of `empty_region`), and the recipe points the header at it in
//! its transaction ([`set_level_script`]).

#![allow(
    dead_code,
    reason = "scene_on_enter (Task 6 of 3d.7) is the caller of push_condition"
)]

use g4_formats::error::FormatError;
use rotom::{LevelScriptHeaderEntry, LevelScriptVarConditionEntry};

use super::story_common::header_files;
use super::types::{CreatedObject, id, refused};
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::scripts::{self, LevelScriptDoc, ScriptSource, SourceKind};
use crate::store::{MapHeader, Store};

/// The level script entry that runs the visibility block of a place: `on_transition` runs when
/// the player enters the map, before its objects are placed, so a figure whose `hidden_flag`
/// the block sets is not there from the first step on (pret's convention: vanilla Route 201's
/// `on_transition` script sets `VAR_OBJ_GFX_ID_0` for its objects).
///
/// Measured on 02.11.2026 (phase 3d.7 Task 5, Platin DE): a new place from template 342 (new
/// header 593 with level script 909, one entry `on_transition` → script 1), its new script file
/// with `script Main #1: SetFlag F; End` and a figure with `hidden_flag = F` (flag 66) at
/// (54, 17). On the **first** entry flag 66 is 1 and `walk-until-blocked up` from (54, 21)
/// passes the figure's tile to (54, 17) in 4 steps: the figure is not there. Scratch scenario
/// after `pt-flag.yaml` with its images under `target/agent/gate-2026-11-02/3d-7/messung/`.
pub(crate) const VISIBILITY_ENTRY: &str = "on_transition";

/// The level script a recipe changes for a header: the file (the header's own, or the number a
/// new file gets), whether it is created ([`ensure_level_script`]), its document as it is now
/// (the shared file's copy, or empty) and the script its [`VISIBILITY_ENTRY`] entry starts so
/// far.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LevelScriptPlan {
    pub file: u16,
    pub create: bool,
    pub doc: LevelScriptDoc,
    /// The script [`VISIBILITY_ENTRY`] starts so far (a slot of the header's script file).
    pub transition: Option<u16>,
}

/// The refusal for a header whose level script id names a rotom script.
fn not_a_level_script_text(file: u16, header: u16) -> String {
    format!(
        "Die Level-Script-Datei {file:04} von Header {header} ist ein Script; in Bauen eine \
         Level-Script-Datei zuweisen."
    )
}

/// The level script of header `header` as a recipe changes it: its own file in place; a new
/// file (`create`) with a copy of the document when another header uses the same file, with
/// an empty document when the header has none (0) or its file has no level script source
/// (binary). Refused: a header that does not exist, a level script id that names a rotom
/// script, a header table that is not writable when a new file is needed.
pub(crate) fn plan_level_script(
    store: &mut Store,
    header: u16,
) -> Result<LevelScriptPlan, KernelError> {
    let current = header_files(store, header)?.level_script;
    let project = store.project_arc();
    let (create, doc) = if current == 0 {
        (true, LevelScriptDoc::default())
    } else {
        match scripts::read_source(&project, current)? {
            ScriptSource::LevelScript(doc) => {
                let shared = store
                    .header_table()?
                    .iter()
                    .enumerate()
                    .any(|(i, h)| i != usize::from(header) && h.level_script_id() == current);
                (shared, doc)
            }
            ScriptSource::Binary { .. } => (true, LevelScriptDoc::default()),
            ScriptSource::Rotom(_) => {
                return Err(refused(not_a_level_script_text(current, header)));
            }
        }
    };
    let file = if create {
        super::types::check_writable(store, crate::store::HEADER_TABLE)?;
        scripts::next_script_file(&project)?
    } else {
        current
    };
    let transition = transition_entry(&doc);
    Ok(LevelScriptPlan {
        file,
        create,
        doc,
        transition,
    })
}

/// The file of `plan`: created when the plan says so, as a new level script file with the
/// plan's document (compiled), named in `created`; else the header's own file. A new file
/// stays after undo (it is a new member, no undo step takes it back).
pub(crate) fn ensure_level_script(
    store: &mut Store,
    plan: &LevelScriptPlan,
    created: &mut Vec<CreatedObject>,
) -> Result<u16, KernelError> {
    if !plan.create {
        return Ok(plan.file);
    }
    let project = store.project_arc();
    let (file, mut touched) = scripts::new_script_file_reported(&project, SourceKind::LevelScript)?;
    created.push(CreatedObject {
        kind: AssetKind::Script,
        index: file,
    });
    if plan.doc != LevelScriptDoc::default() {
        touched.extend(write_level_script(&project, file, &plan.doc)?);
    }
    for a in touched {
        let archive = id(AssetKind::TextArchive, a);
        if store.is_loaded(archive) {
            store.reload_from_disk(archive)?;
        }
    }
    Ok(file)
}

/// Writes `doc` as level script file `file` and compiles; answers the text archives the
/// compile changed. A compile error in the file is an error (the caller puts the files back).
pub(crate) fn write_level_script(
    project: &Project,
    file: u16,
    doc: &LevelScriptDoc,
) -> Result<Vec<u16>, KernelError> {
    scripts::write_source(project, file, &ScriptSource::LevelScript(doc.clone()))?;
    let report = scripts::compile(project)?;
    let errors = scripts::problems_in(project, &report, file, SourceKind::LevelScript);
    if !errors.is_empty() {
        return Err(KernelError::Compile(errors));
    }
    Ok(report.touched_archives)
}

/// The type of `entry` as the level script JSON names it.
fn entry_type(entry: &LevelScriptHeaderEntry) -> &'static str {
    match entry {
        LevelScriptHeaderEntry::OnTransition { .. } => "on_transition",
        LevelScriptHeaderEntry::OnResume { .. } => "on_resume",
        LevelScriptHeaderEntry::OnLoad { .. } => "on_load",
        LevelScriptHeaderEntry::OnVarCondition => "on_var_condition",
    }
}

/// The script id of `entry` (none for `on_var_condition`), mutable.
fn script_id_mut(entry: &mut LevelScriptHeaderEntry) -> Option<&mut u32> {
    match entry {
        LevelScriptHeaderEntry::OnTransition { script_id }
        | LevelScriptHeaderEntry::OnResume { script_id }
        | LevelScriptHeaderEntry::OnLoad { script_id } => Some(script_id),
        LevelScriptHeaderEntry::OnVarCondition => None,
    }
}

/// A new entry of type [`VISIBILITY_ENTRY`] that starts `script_id`.
fn visibility_entry(script_id: u16) -> LevelScriptHeaderEntry {
    let script_id = u32::from(script_id);
    match VISIBILITY_ENTRY {
        "on_load" => LevelScriptHeaderEntry::OnLoad { script_id },
        "on_resume" => LevelScriptHeaderEntry::OnResume { script_id },
        _ => LevelScriptHeaderEntry::OnTransition { script_id },
    }
}

/// The script the first [`VISIBILITY_ENTRY`] entry of `doc` starts; `None` without one or for
/// script 0. An id past `u16` answers `u16::MAX` (no slot of a script file).
pub(crate) fn transition_entry(doc: &LevelScriptDoc) -> Option<u16> {
    doc.header_entries
        .iter()
        .filter(|e| entry_type(e) == VISIBILITY_ENTRY)
        .find_map(|e| match e {
            LevelScriptHeaderEntry::OnTransition { script_id }
            | LevelScriptHeaderEntry::OnResume { script_id }
            | LevelScriptHeaderEntry::OnLoad { script_id } => Some(*script_id),
            LevelScriptHeaderEntry::OnVarCondition => None,
        })
        .filter(|&id| id != 0)
        .map(|id| u16::try_from(id).unwrap_or(u16::MAX))
}

/// Points the first [`VISIBILITY_ENTRY`] entry of `doc` at `script_id`; appends one when
/// there is none.
pub(crate) fn set_transition(doc: &mut LevelScriptDoc, script_id: u16) {
    let found = doc
        .header_entries
        .iter_mut()
        .filter(|e| entry_type(e) == VISIBILITY_ENTRY)
        .find_map(script_id_mut);
    match found {
        Some(id) => *id = u32::from(script_id),
        None => doc.header_entries.push(visibility_entry(script_id)),
    }
}

/// Appends the condition `var == value` → `script_id` to `doc` (and an `on_var_condition`
/// entry when it has none, the game reads the conditions through it); answers its index.
pub(crate) fn push_condition(
    doc: &mut LevelScriptDoc,
    var: u16,
    value: u16,
    script_id: u16,
) -> u16 {
    if !doc.has_var_condition() {
        doc.header_entries
            .push(LevelScriptHeaderEntry::OnVarCondition);
    }
    doc.var_conditions.push(LevelScriptVarConditionEntry {
        var,
        value,
        script_id,
    });
    u16::try_from(doc.var_conditions.len() - 1).unwrap_or(u16::MAX)
}

/// Sets the level script of `header` to `file`.
pub(crate) fn set_level_script(header: &mut MapHeader, file: u16) -> Result<(), FormatError> {
    match header {
        MapHeader::DP(h) => h.level_script_id = file,
        MapHeader::Pt(h) => h.level_script_id = file,
        MapHeader::HGSS(h) => h.level_script_id = file,
    }
    Ok(())
}

#[cfg(test)]
mod tests;
