//! „Figur erscheint/verschwindet ab Schritt“: a figure gets a flag of the pool as its
//! visibility flag (`hidden_flag`; the figure is missing while the flag is set), and the
//! place's visibility block of the story sets or clears the flag from the story's variable when
//! the player enters the place. The flag gets its own name `FLAG_STORY_<NAME>_SICHTBAR_<local_id>`
//! and the story the step `a{flag}` (kind `appearance`, value k) and the flag in its `flags`, as
//! one undo step.
//!
//! The block `sichtbarkeit_<S>_<H>` (story S, header H) is one script of the header's script
//! file per place and story: one rule per figure (`if VAR >= k then ClearFlag F else SetFlag F
//! endif`, swapped for a figure that vanishes), then `Jump` to the script the place's level
//! script started on entering so far, or `End`, and the end marker. The first rule appends the
//! block and points the level script's [`VISIBILITY_ENTRY`](super::level_script::VISIBILITY_ENTRY) entry at it (a level script file
//! another header shares, or none, becomes a new file for this header first: it stays after
//! undo, see [`level_script`](super::level_script)); every further rule is inserted above the
//! block's `Jump`/`End` line ([`insert_rule`]), the level script stays.
//!
//! The flag comes with the input (the caller takes it from the pool, `free_flag`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::level_script::{
    LevelScriptPlan, ensure_level_script, plan_level_script, set_level_script, set_transition,
    write_level_script,
};
use super::story_common::{
    Figure, FigureRef, HeaderFiles, LoadedStory, check_pool_number, figure, header_files,
    load_story, made, next_slot, slot_name, slot_named, step_of, story_flag_id,
};
use super::story_scripts::{
    Tail, VisibilityRule, insert_rule, visibility_block, visibility_end_marker, visibility_name,
    visibility_rule,
};
use super::story_wrap::{
    DiskBefore, compile_archives, finish_step, require_adopted, require_rotom, rolled_back,
};
use super::types::{Draft, Outcome, RecipeId, check_writable, id, next_steps, refused};
use crate::error::KernelError;
use crate::flags::{FlagKind, NameRequest, name_flag, names, own_names};
use crate::project::{AssetKind, Project};
use crate::scripts::{self, FIRST_COMMON_SCRIPT};
use crate::store::{ChangeId, Store};
use crate::stories::{Step, StepKind, Use, duplicate_key_text};

/// The kind word of the flag's id: `FLAG_STORY_<NAME>_SICHTBAR_<local_id>`.
const FLAG_KIND: &str = "SICHTBAR";

/// The refusal for a place whose level script starts something else than a slot of the
/// header's script file on entering (a common script, a slot the file does not declare).
pub(crate) const TRANSITION_TEXT: &str = "Das Übergangs-Script des Orts ist kein eigener Slot.";

/// The input of „Figur erscheint/verschwindet ab Schritt“: the story (its id), the figure, the
/// step from which on it appears (`appears`) or vanishes (value k; before it the other way),
/// its visibility flag (from the pool) and the step's name (`None`: „Figur {local_id} erscheint
/// ab Schritt {k}“ / „… verschwindet ab Schritt {k}“).
#[derive(Debug, Clone, Deserialize)]
pub struct FigureAppearsInput {
    pub story: String,
    pub figure: FigureRef,
    pub step: u16,
    pub appears: bool,
    pub flag: u16,
    #[serde(default)]
    pub step_name: Option<String>,
}

/// What [`apply`] did: step `step` (its key); flag `flag` named `flag_id` is the
/// `hidden_flag` of object `object` (its index in `event_file`); the block is slot `#slot` of
/// `script_file` with `rules` rules, started by level script file `level_script` (created by
/// the recipe: `level_script_created`).
#[derive(Debug, Clone, Serialize)]
pub struct FigureAppearsReport {
    pub step: String,
    pub flag: u16,
    pub flag_id: String,
    pub event_file: u16,
    pub object: u16,
    pub level_script: u16,
    pub level_script_created: bool,
    pub script_file: u16,
    pub slot: u16,
    pub rules: u32,
}

/// The key of the step a visibility with flag `flag` makes.
fn step_key(flag: u16) -> String {
    format!("a{flag}")
}

/// What the figure does from the step on.
fn verb(appears: bool) -> &'static str {
    if appears { "erscheint" } else { "verschwindet" }
}

/// The undo label.
fn label(local_id: u16, step: u16, appears: bool, story: &str) -> String {
    format!(
        "Figur {local_id} {} ab Schritt {step} in „{story}“",
        verb(appears)
    )
}

/// The flag's description.
fn description(local_id: u16, story: &str) -> String {
    format!("Sichtbarkeit von Figur {local_id} in „{story}“")
}

/// The refusal for a figure that has a visibility flag (named `flag`).
fn has_flag_text(local_id: u16, flag: &str) -> String {
    format!("Figur {local_id} hat schon das Sichtbarkeits-Flag {flag}.")
}

/// Flag `n` as a sentence names it: its own name, else pret's, else its number.
fn flag_text(project: &Project, n: u16) -> Result<String, KernelError> {
    if let Some(own) = own_names(project, FlagKind::Flag)?.get(&n) {
        return Ok(own.id.clone());
    }
    Ok(names(project.family, FlagKind::Flag)
        .name(n)
        .map_or_else(|| format!("0x{n:04X}"), str::to_owned))
}

/// The number of rules of the block `name` in `source`: the `if` lines between its head and
/// its end marker `marker`.
fn rules_in(source: &str, name: &str, marker: &str) -> u32 {
    let head = format!("script {name} #");
    let mut inside = false;
    let mut rules = 0;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with(&head) {
            inside = true;
        } else if line == marker {
            break;
        } else if inside && line.starts_with("if ") {
            rules += 1;
        }
    }
    rules
}

/// The block as the recipe leaves it: appended as a new slot (with the level script that
/// starts it on entering and the script it jumps to), or grown by the new rule.
#[derive(Debug, Clone)]
enum Block {
    New {
        slot: u16,
        tail: Tail,
        level: LevelScriptPlan,
    },
    Grow {
        slot: u16,
        /// The script file's source with the new rule.
        source: String,
        /// The rules with the new one.
        rules: u32,
        /// The header's level script file (unchanged).
        level_script: u16,
    },
}

impl Block {
    fn slot(&self) -> u16 {
        match self {
            Block::New { slot, .. } | Block::Grow { slot, .. } => *slot,
        }
    }

    /// The level script file a new file would get (`Some` only when one is created).
    fn created_level_script(&self) -> Option<u16> {
        match self {
            Block::New { level, .. } if level.create => Some(level.file),
            _ => None,
        }
    }
}

/// What [`check`] found, everything [`apply`] needs before its first write.
struct Checked {
    story: LoadedStory,
    figure: Figure,
    files: HeaderFiles,
    flag_id: String,
    rule: VisibilityRule,
    /// The script file's rotom source before the recipe.
    source: String,
    block: Block,
}

/// Every check before the first write, in this order: adopted scripts, the story with a
/// progress variable (a text project), its step `step`, the figure without a visibility flag,
/// the flag from the pool ([`check_pool_number`]), no step with key `a{flag}`, a rotom source
/// as the header's script file, a writable event file; the flag's id from the story's name
/// (`_2`, … when an own name or a pret name has it). Then the block: when the script file has
/// the block's end marker, the new rule goes above its `Jump`/`End` line ([`insert_rule`]);
/// else a free slot below the common scripts, the block's name not yet in the file, the level
/// script ([`plan_level_script`]) and the script its [`VISIBILITY_ENTRY`](super::level_script::VISIBILITY_ENTRY) entry starts so far
/// (none, or a slot the file declares: else [`TRANSITION_TEXT`]).
fn check(store: &mut Store, input: &FigureAppearsInput) -> Result<Checked, KernelError> {
    require_adopted(store)?;
    let story = load_story(store, &input.story)?;
    let value = input.step;
    if step_of(&story.story, value).is_none() {
        return Err(refused(format!(
            "Schritt {value} gibt es in „{}“ nicht.",
            story.story.name
        )));
    }
    let found = figure(store, &input.figure)?;
    let project = store.project_arc();
    if found.hidden_flag != 0 {
        return Err(refused(has_flag_text(
            found.local_id,
            &flag_text(&project, found.hidden_flag)?,
        )));
    }
    check_pool_number(&project, FlagKind::Flag, input.flag)?;
    let key = step_key(input.flag);
    if story.story.steps.iter().any(|s| s.key == key) {
        return Err(refused(duplicate_key_text(&key)));
    }
    let header = input.figure.header;
    let files = header_files(store, header)?;
    let script_file = files.script_file;
    require_rotom(&project, header, script_file)?;
    check_writable(store, id(AssetKind::Event, found.event_file))?;

    let own = own_names(&project, FlagKind::Flag)?;
    let pret = names(project.family, FlagKind::Flag);
    let taken = |id: &str| own.values().any(|e| e.id == id) || pret.number(id).is_some();
    let flag_id = story_flag_id(&story.story.name, FLAG_KIND, found.local_id, &taken);
    let rule = VisibilityRule {
        var_id: story.var_id.clone(),
        step: value,
        flag_id: flag_id.clone(),
        appears: input.appears,
    };

    let source = scripts::rotom_source(&project, script_file)?;
    let name = visibility_name(story.number, header);
    let marker = visibility_end_marker(story.number, header);
    let block = if source.lines().any(|l| l.trim_end() == marker) {
        let slot = slot_named(&source, &name).ok_or_else(|| {
            refused(format!(
                "Der Sichtbarkeits-Block {name} in Scriptdatei {script_file:04} hat keinen Slot."
            ))
        })?;
        let grown = insert_rule(
            &source,
            story.number,
            header,
            &visibility_rule(&rule),
            script_file,
        )?;
        let rules = rules_in(&grown, &name, &marker);
        Block::Grow {
            slot,
            source: grown,
            rules,
            level_script: files.level_script,
        }
    } else {
        if scripts::names_identifier(&source, &name) {
            return Err(refused(format!(
                "Die Scriptdatei {script_file:04} nennt schon {name}; in Bauen umbenennen."
            )));
        }
        let (slot, _) = next_slot(&project, script_file)?;
        let level = plan_level_script(store, header)?;
        let tail = match level.transition {
            None => Tail::End,
            Some(target) if target >= FIRST_COMMON_SCRIPT => {
                return Err(refused(TRANSITION_TEXT));
            }
            Some(target) => {
                Tail::Jump(slot_name(&source, target).ok_or_else(|| refused(TRANSITION_TEXT))?)
            }
        };
        Block::New { slot, tail, level }
    };
    Ok(Checked {
        story,
        figure: found,
        files,
        flag_id,
        rule,
        source,
        block,
    })
}

/// The sentences of the dialog: the figure's flag and the block.
fn notes(checked: &Checked, input: &FigureAppearsInput) -> Vec<String> {
    vec![
        format!(
            "Figur {} bekommt das Sichtbarkeits-Flag {} und {} ab Schritt {}.",
            checked.figure.local_id,
            checked.flag_id,
            verb(input.appears),
            input.step
        ),
        format!(
            "Block {} in Scriptdatei {}.",
            visibility_name(checked.story.number, input.figure.header),
            checked.files.script_file
        ),
    ]
}

/// The sentence of a level script file the recipe creates (it stays after undo).
fn created_text(file: u16) -> String {
    format!("Level-Script-Datei {file} wird angelegt und bleibt")
}

/// What [`apply`] would do, found with the same checks and without writing: the two sentences
/// of the dialog and, when the place needs its own level script file, that it is created and
/// stays.
pub fn plan(store: &mut Store, input: &FigureAppearsInput) -> Result<Draft, KernelError> {
    let checked = check(store, input)?;
    Ok(Draft {
        notes: notes(&checked, input),
        irreversible: checked
            .block
            .created_level_script()
            .map(created_text)
            .into_iter()
            .collect(),
        ..Draft::default()
    })
}

/// The step the visibility adds: key `a{flag}`, kind `appearance`, value `step`, the given name
/// or „Figur {local_id} erscheint ab Schritt {k}“ (… verschwindet …), made by this recipe,
/// using the object, the flag and the block.
fn appearance_step(input: &FigureAppearsInput, checked: &Checked) -> Step {
    let local_id = checked.figure.local_id;
    let name = input
        .step_name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(
            || {
                format!(
                    "Figur {local_id} {} ab Schritt {}",
                    verb(input.appears),
                    input.step
                )
            },
            str::to_owned,
        );
    Step {
        key: step_key(input.flag),
        value: input.step,
        kind: StepKind::Appearance,
        name,
        notes: String::new(),
        made: Some(made(RecipeId::FigureAppears)),
        uses: vec![
            Use::Object {
                file: checked.figure.event_file,
                local_id,
            },
            Use::Flag(checked.flag_id.clone()),
            Use::Script {
                file: checked.files.script_file,
                slot: checked.block.slot(),
            },
        ],
    }
}

/// Gives the figure its visibility from step k on. A level script file the place needs is
/// created first ([`ensure_level_script`], not undone, named in `created`). Then one undo step
/// „Figur {k} erscheint ab Schritt {s} in „X““: the flag is named (`name_flag`, „Sichtbarkeit
/// von Figur {k} in „X““), one transaction sets the object's `hidden_flag` (and the header's
/// level script to the new file), the block is appended (and the level script's
/// [`VISIBILITY_ENTRY`](super::level_script::VISIBILITY_ENTRY) entry points at it) or grown by the rule, the files the compiles wrote
/// are recorded as file steps, and the story gets the step and the flag. Any error after the
/// mark puts the written files back and rolls back to the mark.
pub fn apply(
    store: &mut Store,
    input: &FigureAppearsInput,
) -> Result<Outcome<FigureAppearsReport>, KernelError> {
    let checked = check(store, input)?;
    let generation = store.change_generation();
    let mut created = Vec::new();
    let (level_script, level_created) = match &checked.block {
        Block::New { level, .. } => (
            ensure_level_script(store, level, &mut created)?,
            level.create,
        ),
        Block::Grow { level_script, .. } => (*level_script, false),
    };
    let mark = store.undo_depth();
    let local_id = checked.figure.local_id;
    let label = label(
        local_id,
        input.step,
        input.appears,
        &checked.story.story.name,
    );
    let step = appearance_step(input, &checked);
    let report = FigureAppearsReport {
        step: step.key.clone(),
        flag: input.flag,
        flag_id: checked.flag_id.clone(),
        event_file: checked.figure.event_file,
        object: checked.figure.index,
        level_script,
        level_script_created: level_created,
        script_file: checked.files.script_file,
        slot: checked.block.slot(),
        rules: match &checked.block {
            Block::New { .. } => 1,
            Block::Grow { rules, .. } => *rules,
        },
    };
    let write = Write {
        input,
        label: &label,
        level_script,
        level_created,
        step,
    };
    let change = rolled_back(store, mark, |store, disk| {
        write_visibility(store, mark, checked, write, disk)
    })?;
    Ok(Outcome {
        report,
        created,
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[("play_here", "Hier spielen")]),
    })
}

/// What [`write_visibility`] writes besides [`Checked`]: the input, the undo label, the level
/// script file (created: the header points at it now) and the story's step.
struct Write<'a> {
    input: &'a FigureAppearsInput,
    label: &'a str,
    level_script: u16,
    level_created: bool,
    step: Step,
}

/// [`apply`]'s undo step without taking back: `disk` holds the files before the compiles from
/// the first compile on until their file steps are recorded.
fn write_visibility(
    store: &mut Store,
    mark: usize,
    checked: Checked,
    write: Write<'_>,
    disk: &mut Option<DiskBefore>,
) -> Result<Option<ChangeId>, KernelError> {
    let Checked {
        story,
        figure: found,
        files,
        flag_id,
        rule,
        source,
        block,
    } = checked;
    let Write {
        input,
        label,
        level_script,
        level_created,
        step,
    } = write;
    name_flag(
        store,
        &NameRequest {
            kind: FlagKind::Flag,
            number: input.flag,
            id: Some(flag_id.clone()),
            description: Some(description(found.local_id, &story.story.name)),
        },
    )?;

    // The files as they are before the first compile: the script file, the level script file
    // when the block is new, the compile state and the archives a compile of the script file
    // may write.
    let project = store.project_arc();
    let script_file = files.script_file;
    let mut before_scripts = BTreeMap::from([(
        script_file,
        scripts::snapshot_script(&project, script_file)?,
    )]);
    if matches!(block, Block::New { .. }) {
        before_scripts.insert(
            level_script,
            scripts::snapshot_script(&project, level_script)?,
        );
    }
    let before_state = scripts::snapshot_compile_state(&project)?;
    let saved = compile_archives(store, script_file, files.text_archive)?;

    let (header, event_file) = (input.figure.header, found.event_file);
    let (index, local_id, flag) = (found.index, found.local_id, input.flag);
    store.transaction(label, |tx| {
        let mut set = false;
        tx.edit_event(event_file, |f| {
            if let Some(object) = f
                .object_events
                .get_mut(usize::from(index))
                .filter(|o| o.local_id == local_id && o.hidden_flag == 0)
            {
                object.hidden_flag = flag;
                set = true;
            }
            Ok(())
        })?;
        if !set {
            // Checked before the first write; a failed transaction records nothing.
            return Err(refused(format!(
                "Objekt {index} der Eventdatei {event_file:04} ist nicht mehr Figur {local_id} \
                 ohne Sichtbarkeits-Flag."
            )));
        }
        if level_created {
            tx.edit_header(header, |h| set_level_script(h, level_script))?;
        }
        Ok(())
    })?;

    // From here on the compiles' files are on disk until their file steps are recorded.
    let before = disk.insert(DiskBefore {
        scripts: before_scripts,
        state: before_state,
        archives: saved,
    });
    let mut touched = match block {
        Block::New { slot, tail, level } => {
            let text = visibility_block(story.number, header, slot, &[rule], &tail);
            let mut touched = scripts::append_source(&project, script_file, source, &text)?;
            let mut doc = level.doc;
            set_transition(&mut doc, slot);
            touched.extend(write_level_script(&project, level_script, &doc)?);
            touched
        }
        Block::Grow { source: grown, .. } => {
            scripts::write_and_compile(&project, script_file, &grown)?
        }
    };
    touched.sort_unstable();
    touched.dedup();
    before.record(store, label, &touched, script_file)?;
    let saved = disk.take().map(|d| d.archives).unwrap_or_default();
    finish_step(
        store,
        mark,
        label,
        (&story.story.id, step, std::slice::from_ref(&flag_id)),
        &saved,
        &touched,
    )
}

#[cfg(test)]
mod tests;
