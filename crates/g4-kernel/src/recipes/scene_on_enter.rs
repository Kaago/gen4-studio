//! „Szene beim Betreten“: entering a place while the story is at step k plays a text once and
//! switches the story to k+1. The scene is a new slot `szene_<n> #n` of the header's script
//! file (its message, then `SetVar` to k+1), the place's level script runs it through a
//! `var_conditions` entry `var == k` ([`push_condition`]); it runs exactly once because it
//! switches the variable on. The story gets the step `s{k+1}` (kind `dialog`), as one undo
//! step.
//!
//! A level script file another header shares, or none, becomes a new file for this header
//! first: it stays after undo (see [`level_script`](super::level_script)), the plan announces
//! it and [`Outcome::created`] names it.

use std::collections::BTreeMap;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::level_script::{
    LevelScriptPlan, ensure_level_script, kept_level_script, plan_level_script, push_condition,
    set_level_script, write_level_script,
};
use super::story_common::{
    HeaderFiles, LoadedStory, dialog_step_taken, header_files, load_story, made, message_number,
    next_slot, push_message, require_step, step_name,
};
use super::story_scripts::{scene_name, scene_script};
use super::story_wrap::{
    DiskBefore, check_message_number, check_no_literal, compile_archives, finish_step,
    require_adopted, require_rotom, rolled_back,
};
use super::types::{
    Draft, DraftMessage, Outcome, RecipeId, check_writable, id, next_steps, refused,
};
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::scripts::{self, LevelScriptDoc};
use crate::store::{ChangeId, Store};
use crate::stories::{Step, StepKind, Use, duplicate_key_text};
use crate::textproject::registry::Registry;
use crate::textproject::symbols::SymbolKind;

/// The refusal for an empty text.
fn no_text() -> String {
    t!("recipes.scene_needs_text").into_owned()
}

/// The step's name when the input gives none.
fn default_name() -> String {
    t!("recipes.scene_default_name").into_owned()
}

/// The input of „Szene beim Betreten“: the story (its id), the place (header), the step the
/// scene plays at (value k; it switches to k+1), its text and the step's name (`None`: „Szene
/// beim Betreten“).
#[derive(Debug, Clone, Deserialize)]
pub struct SceneOnEnterInput {
    pub story: String,
    pub header: u16,
    pub step: u16,
    pub text: String,
    #[serde(default)]
    pub step_name: Option<String>,
}

/// What [`apply`] did: step `step` (its key) with value `value`; condition `condition` (its
/// index in `var_conditions`) of level script file `level_script` (created by the recipe:
/// `level_script_created`) runs scene `#slot` of `script_file`, which shows message `message`
/// of text archive `archive`.
#[derive(Debug, Clone, Serialize)]
pub struct SceneOnEnterReport {
    pub step: String,
    pub value: u16,
    pub level_script: u16,
    pub level_script_created: bool,
    pub condition: u16,
    pub script_file: u16,
    pub slot: u16,
    pub archive: u16,
    pub message: u16,
}

/// The key of the step a scene switching to `value` makes.
fn step_key(value: u16) -> String {
    format!("s{value}")
}

/// The undo label.
fn label(header: u16, story: &str) -> String {
    t!("recipes.scene_label", header = header, story = story).into_owned()
}

/// The refusal for a place whose level script runs a script at that step of the story already.
fn taken_condition_text(step: u16) -> String {
    t!("recipes.scene_step_taken", step = step).into_owned()
}

/// What [`check`] found, everything [`apply`] needs before its first write.
struct Checked {
    story: LoadedStory,
    /// The value the scene switches to (k+1).
    value: u16,
    family: Family,
    files: HeaderFiles,
    /// The header's id in the register `data/ids/maps.json`, if it has one.
    map: Option<String>,
    level: LevelScriptPlan,
    /// The scene's slot `#n`.
    slot: u16,
    /// The number the message gets in `files.text_archive`.
    message: u16,
    /// The script file's rotom source before the scene is appended.
    source: String,
}

/// Every check before the first write, in this order: adopted scripts, the story with a
/// progress variable (a text project), its step `step`, no dialog step with value k+1 and no
/// step with key `s{k+1}`, a rotom source as the header's script file, a writable text archive,
/// the level script ([`plan_level_script`]) without a condition for the story's variable at k
/// ([`taken_condition_text`]), a text, a local slot below the common scripts with a name
/// ([`scene_name`]) the file does not use, a message number the message command takes.
fn check(store: &mut Store, input: &SceneOnEnterInput) -> Result<Checked, KernelError> {
    require_adopted(store)?;
    let story = load_story(store, &input.story)?;
    let k = input.step;
    require_step(&story.story, k)?;
    let value = k
        .checked_add(1)
        .ok_or_else(|| refused(t!("recipes.scene_step_highest", step = k)))?;
    dialog_step_taken(&story.story, value)?;
    let key = step_key(value);
    if story.story.steps.iter().any(|s| s.key == key) {
        return Err(refused(duplicate_key_text(&key)));
    }
    let header = input.header;
    let files = header_files(store, header)?;
    let project = store.project_arc();
    let script_file = files.script_file;
    require_rotom(&project, header, script_file)?;
    let map = Registry::load(&project.root, SymbolKind::Map)?
        .id_of(u32::from(header))
        .map(str::to_owned);
    let messages = store.text_archive(files.text_archive)?.messages.len();
    check_writable(store, id(AssetKind::TextArchive, files.text_archive))?;
    let level = plan_level_script(store, header)?;
    if level
        .doc
        .var_conditions
        .iter()
        .any(|c| c.var == story.var && c.value == k)
    {
        return Err(refused(taken_condition_text(k)));
    }
    if input.text.trim().is_empty() {
        return Err(refused(no_text()));
    }
    let (slot, source) = next_slot(&project, script_file)?;
    let name = scene_name(slot);
    if scripts::names_identifier(&source, &name) {
        return Err(refused(t!(
            "recipes.script_file_names_already",
            script_file = script_file : {:04},
            name = name
        )));
    }
    let family = project.family;
    let message = message_number(family, files.text_archive, messages)?;
    Ok(Checked {
        story,
        value,
        family,
        files,
        map,
        level,
        slot,
        message,
        source,
    })
}

/// What [`apply`] would do, found with the same checks and without writing: the message for
/// the header's archive, the sentence of the dialog and, when the place needs its own level
/// script file, that it is created and stays.
pub fn plan(store: &mut Store, input: &SceneOnEnterInput) -> Result<Draft, KernelError> {
    let checked = check(store, input)?;
    Ok(Draft {
        messages: vec![DraftMessage {
            archive: Some(checked.files.text_archive),
            text: input.text.clone(),
        }],
        notes: vec![
            t!(
                "recipes.note_scene",
                header = input.header,
                step = input.step,
                value = checked.value
            )
            .into_owned(),
        ],
        irreversible: checked.level.irreversible().into_iter().collect(),
        ..Draft::default()
    })
}

/// The step the scene adds: key `s{value}`, kind `dialog`, the given name or „Szene beim
/// Betreten“, made by this recipe, using the level script's condition, the scene's slot, the
/// message, the place (its id, when the register names the header) and the variable.
fn scene_step(input: &SceneOnEnterInput, checked: &Checked, level: (u16, u16)) -> Step {
    let (file, condition) = level;
    Step {
        key: step_key(checked.value),
        value: checked.value,
        kind: StepKind::Dialog,
        name: step_name(input.step_name.as_deref(), default_name),
        notes: String::new(),
        made: Some(made(RecipeId::SceneOnEnter)),
        uses: [
            Use::LevelScript { file, condition },
            Use::Script {
                file: checked.files.script_file,
                slot: checked.slot,
            },
            Use::Message {
                archive: checked.files.text_archive,
                index: checked.message,
            },
        ]
        .into_iter()
        .chain(checked.map.clone().map(Use::Map))
        .chain([Use::Var(checked.story.var_id.clone())])
        .collect(),
    }
}

/// Adds the scene. A level script file the place needs is created first
/// ([`ensure_level_script`], not undone, named in `created`). Then one undo step „Szene beim
/// Betreten von Header {h} in „X““: one transaction appends the message to the header's text
/// archive (and points the header at the new level script file), the scene is appended to the
/// script file, the level script gets the condition `var == k` (and an `on_var_condition`
/// entry when it has none), the files the compiles wrote are recorded as file steps, and the
/// story gets the step. Any error after the mark puts the written files back and rolls back to
/// the mark.
pub fn apply(
    store: &mut Store,
    input: &SceneOnEnterInput,
) -> Result<Outcome<SceneOnEnterReport>, KernelError> {
    let checked = check(store, input)?;
    let generation = store.change_generation();
    let mut created = Vec::new();
    let level_script = ensure_level_script(store, &checked.level, &mut created)?;
    let mark = store.undo_depth();
    let label = label(input.header, &checked.story.story.name);
    let mut doc = checked.level.doc.clone();
    let condition = push_condition(&mut doc, checked.story.var, input.step, checked.slot);
    let step = scene_step(input, &checked, (level_script, condition));
    let report = SceneOnEnterReport {
        step: step.key.clone(),
        value: checked.value,
        level_script,
        level_script_created: checked.level.create,
        condition,
        script_file: checked.files.script_file,
        slot: checked.slot,
        archive: checked.files.text_archive,
        message: checked.message,
    };
    let write = Write {
        input,
        label: &label,
        level_script,
        doc,
        step,
    };
    let kept = checked.level.create.then_some(level_script);
    let change = rolled_back(store, mark, |store, disk| {
        write_scene(store, mark, checked, write, disk)
    })
    .map_err(|e| kept_level_script(e, kept))?;
    Ok(Outcome {
        report,
        created,
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&["play_here"]),
    })
}

/// What [`write_scene`] writes besides [`Checked`]: the input, the undo label, the level script
/// file and its document with the condition, and the story's step.
struct Write<'a> {
    input: &'a SceneOnEnterInput,
    label: &'a str,
    level_script: u16,
    doc: LevelScriptDoc,
    step: Step,
}

/// [`apply`]'s undo step without taking back: `disk` holds the files before the compiles from
/// the first compile on until their file steps are recorded.
fn write_scene(
    store: &mut Store,
    mark: usize,
    checked: Checked,
    write: Write<'_>,
    disk: &mut Option<DiskBefore>,
) -> Result<Option<ChangeId>, KernelError> {
    let Checked {
        story,
        value,
        family,
        files,
        map: _,
        level,
        slot,
        message: expected,
        source,
    } = checked;
    let Write {
        input,
        label,
        level_script,
        doc,
        step,
    } = write;
    let project = store.project_arc();
    let (script_file, archive) = (files.script_file, files.text_archive);

    // The files as they are before the first write: the script file, the level script file,
    // the compile state and the archives a compile of the script file may write.
    let before_scripts = BTreeMap::from([
        (
            script_file,
            scripts::snapshot_script(&project, script_file)?,
        ),
        (
            level_script,
            scripts::snapshot_script(&project, level_script)?,
        ),
    ]);
    let before_state = scripts::snapshot_compile_state(&project)?;
    let saved = compile_archives(store, script_file, archive)?;

    let header = input.header;
    let (message, _) = store.transaction(label, |tx| {
        let message = push_message(tx, family, archive, &input.text)?;
        if level.create {
            tx.edit_header(header, |h| set_level_script(h, level_script))?;
        }
        Ok(message)
    })?;
    check_message_number(message, expected, archive)?;

    // From here on the compiles' files are on disk until their file steps are recorded.
    let before = disk.insert(DiskBefore {
        scripts: before_scripts,
        state: before_state,
        archives: saved,
    });
    let script = scene_script(family, slot, &story.var_id, value, message);
    let mut touched = scripts::append_source(&project, script_file, source, &script)?;
    touched.extend(write_level_script(&project, level_script, &doc)?);
    touched.sort_unstable();
    touched.dedup();
    before.record(store, label, &touched, script_file)?;
    let saved = disk.take().map(|d| d.archives).unwrap_or_default();
    check_no_literal(&touched, script_file, archive)?;
    finish_step(
        store,
        mark,
        label,
        (&story.story.id, step, &[]),
        &saved,
        &touched,
    )
}

#[cfg(test)]
mod tests;
