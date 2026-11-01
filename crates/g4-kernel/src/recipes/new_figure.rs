//! „Neue Figur“: an object event on a tile of a header, a new script slot in the header's
//! script file that shows a message, and that message appended to the header's text archive,
//! as one undo step.
//!
//! The script names the message by its number (`Message n`, HeartGold `NPCMsg n`), not as a
//! text literal: rotom's compile reuses the index of an equal text and gives a changed literal a
//! new index, so a literal would share messages and leave old ones behind. The message goes into
//! the archive through the store, in one transaction with the object; the script is appended to
//! the source and compiled, its files recorded as file steps and merged with the transaction.
//! See `docs/formats/recipes.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::doors::{header_event_file, outside_map};
use super::events::next_free_local_id;
use super::types::{
    Draft, DraftEvent, DraftEventKind, DraftMessage, Outcome, check_header_id, check_writable, id,
    next_steps, refused,
};
use crate::data::trainer_create::ADOPT_FIRST;
use crate::dialog::highest_message;
use crate::error::KernelError;
use crate::flags::{FlagKind, in_save, not_in_save_text};
use crate::project::AssetKind;
use crate::scripts::{self, ArchiveSnapshot, FIRST_COMMON_SCRIPT, SourceKind};
use crate::store::{MapHeader, ObjectEvent, Store};
use crate::text::{TextMessage, default_message_id};

const RECIPES_YAML: &str = include_str!("../../metadata/recipes.yaml");
/// The script of a figure per family, with the placeholders `{n}` (script slot), `{local_id}`
/// and `{message}` (message number).
const FIGURE_PT: &str = include_str!("../../metadata/recipes/figure_pt.rotom");
const FIGURE_HGSS: &str = include_str!("../../metadata/recipes/figure_hgss.rotom");

/// Characters of the text the undo label quotes.
const LABEL_CHARS: usize = 20;

/// The input of „Neue Figur“: the tile (`x`, `z`, absolute tiles of the header's matrix), the
/// look (`None`: the family's [`DefaultLook::figure`]), how the figure moves and looks, the text
/// it says and the flag that hides it while set (`None` or 0: always visible; a free flag is
/// the caller's to find, `free_flag`).
#[derive(Debug, Clone, Deserialize)]
pub struct NewFigureInput {
    pub header: u16,
    pub x: u16,
    pub z: u16,
    pub graphics_id: Option<u16>,
    pub movement_type: u16,
    pub dir: i16,
    pub text: String,
    pub hidden_flag: Option<u16>,
}

/// What [`apply`] did: object `object` (its index in `event_file`) with `local_id`, script
/// `#slot` in `script_file`, message `message` in text archive `archive`.
#[derive(Debug, Clone, Serialize)]
pub struct NewFigureReport {
    pub event_file: u16,
    pub object: u16,
    pub local_id: u16,
    pub script_file: u16,
    pub slot: u16,
    pub archive: u16,
    pub message: u16,
}

/// The looks a recipe gives when its input names none (`metadata/recipes.yaml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct DefaultLook {
    /// The overworld graphics id of a new figure.
    pub figure: u16,
}

#[derive(Debug, Deserialize)]
struct RecipesYaml {
    default_look: FamilyLooks,
}

#[derive(Debug, Deserialize)]
struct FamilyLooks {
    pt: DefaultLook,
    hgss: DefaultLook,
}

/// The default looks of `family` (Diamond/Pearl take Platinum's).
pub fn default_look(family: Family) -> Result<DefaultLook, KernelError> {
    static TABLE: OnceLock<Result<RecipesYaml, String>> = OnceLock::new();
    let table = TABLE
        .get_or_init(|| {
            serde_yaml::from_str(RECIPES_YAML).map_err(|e| format!("recipes.yaml: {e}"))
        })
        .as_ref()
        .map_err(|e| KernelError::InvalidProject(e.clone()))?;
    Ok(match family {
        Family::Hgss => table.default_look.hgss,
        Family::Dp | Family::Pt => table.default_look.pt,
    })
}

/// The script text of a figure for `family` (`metadata/recipes/figure_{pt,hgss}.rotom`, Dp like
/// Pt): `{n}`, `{local_id}`, `{message}` filled.
pub fn figure_script(family: Family, n: u16, local_id: u16, message: u16) -> String {
    let template = match family {
        Family::Hgss => FIGURE_HGSS,
        Family::Dp | Family::Pt => FIGURE_PT,
    };
    template
        .replace("{n}", &n.to_string())
        .replace("{local_id}", &local_id.to_string())
        .replace("{message}", &message.to_string())
}

/// The message command of the figure script of `family`.
fn message_command(family: Family) -> &'static str {
    match family {
        Family::Hgss => "NPCMsg",
        Family::Dp | Family::Pt => "Message",
    }
}

/// The undo label: `Figur „…“ aufgestellt` with the text cut to [`LABEL_CHARS`] characters.
fn label(text: &str) -> String {
    let text = text.trim();
    let mut short: String = text.chars().take(LABEL_CHARS).collect();
    if short.len() < text.len() {
        short = format!("{}…", short.trim_end());
    }
    format!("Figur „{short}“ aufgestellt")
}

/// What [`check`] found, everything [`apply`] needs before its first write.
struct Checked {
    family: Family,
    event_file: u16,
    script_file: u16,
    archive: u16,
    /// The matrix the tile lies on (the header's).
    matrix: u16,
    /// The index the object gets in its event file.
    object: u16,
    local_id: u16,
    slot: u16,
    message: u16,
    look: u16,
    /// 0: none.
    hidden_flag: u16,
    /// The script file's rotom source before the figure's script is appended.
    source: String,
}

/// Every check before the first write, in this order: the header, adopted scripts
/// ([`ADOPT_FIRST`]), a text, a flag the save holds, a rotom source as the header's script file,
/// a writable event file and text archive, a tile of the header's matrix no object of the file
/// stands on, a local slot below the common scripts, a script name the file does not use, a
/// message number the message command takes.
fn check(store: &mut Store, input: &NewFigureInput) -> Result<Checked, KernelError> {
    check_header_id(store, input.header)?;
    let family = store.project().family;
    if !scripts::is_adopted(store.project()) {
        return Err(refused(ADOPT_FIRST));
    }
    if input.text.trim().is_empty() {
        return Err(refused("Die Figur braucht einen Text."));
    }
    let hidden_flag = input.hidden_flag.unwrap_or(0);
    if hidden_flag != 0 && !in_save(family, FlagKind::Flag, hidden_flag) {
        return Err(refused(not_in_save_text(
            FlagKind::Flag,
            hidden_flag,
            family,
        )));
    }
    let (script_file, archive) = store
        .header_table()?
        .get(usize::from(input.header))
        .map(|h| (h.script_file_id(), h.text_archive_id()))
        .ok_or_else(|| refused(format!("header {} does not exist", input.header)))?;
    if scripts::source_kind(store.project(), script_file) != SourceKind::Script {
        return Err(refused(format!(
            "Die Scriptdatei {script_file:04} von Header {} ist keine rotom-Quelle; in Bauen \
             eine Scriptdatei zuweisen.",
            input.header
        )));
    }
    let event_file = header_event_file(store, input.header)?;
    let objects = store.event(event_file)?.object_events.clone();
    check_writable(store, id(AssetKind::Event, event_file))?;
    let messages = store.text_archive(archive)?.messages.len();
    check_writable(store, id(AssetKind::TextArchive, archive))?;
    let (matrix, _) = outside_map(store, input.header, input.x, input.z)?;
    if let Some(k) = objects
        .iter()
        .position(|o| (o.x, o.z) == (input.x, input.z))
    {
        return Err(refused(format!(
            "auf {}, {} steht schon Objekt {k}",
            input.x, input.z
        )));
    }

    let source = scripts::rotom_source(store.project(), script_file)?;
    let next = scripts::declared_slots(&source)?
        .iter()
        .map(|(id, _)| *id)
        .max()
        .map_or(1, |id| id + 1);
    if next >= u32::from(FIRST_COMMON_SCRIPT) {
        return Err(refused(format!(
            "Die Scriptdatei {script_file:04} hat keinen freien lokalen Slot: #{next} wäre ein \
             gemeinsames Script (ab #{FIRST_COMMON_SCRIPT})."
        )));
    }
    // Below FIRST_COMMON_SCRIPT, so it fits.
    let slot = next as u16;
    let local_id = next_free_local_id(&objects)?;
    let object = u16::try_from(objects.len())
        .map_err(|_| refused(format!("event file {event_file:04} has too many objects")))?;
    let name = format!("figur_{local_id}");
    if scripts::names_identifier(&source, &name) {
        return Err(refused(format!(
            "Die Scriptdatei {script_file:04} nennt schon {name}; in Bauen umbenennen."
        )));
    }
    let command = message_command(family);
    let highest = highest_message(family, command);
    let message = u16::try_from(messages)
        .ok()
        .filter(|&m| u32::from(m) <= highest)
        .ok_or_else(|| {
            refused(format!(
                "Textarchiv {archive} hat schon {messages} Nachrichten; {command} nimmt Nummern \
                 bis {highest}, die neue Nachricht bekäme {messages}."
            ))
        })?;
    let look = match input.graphics_id {
        Some(look) => look,
        None => default_look(family)?.figure,
    };
    Ok(Checked {
        family,
        event_file,
        script_file,
        archive,
        matrix,
        object,
        local_id,
        slot,
        message,
        look,
        hidden_flag,
        source,
    })
}

/// What [`apply`] would do, found with the same checks and without writing: the object on the
/// header's matrix with its look, the message for the header's archive, and the sentences of
/// the wizard (script slot, message number, visibility flag). Nothing is irreversible.
pub fn plan(store: &mut Store, input: &NewFigureInput) -> Result<Draft, KernelError> {
    let checked = check(store, input)?;
    let mut notes = vec![format!(
        "Stellt Figur mit Script #{} und Nachricht {} in Archiv {} auf.",
        checked.slot, checked.message, checked.archive
    )];
    if checked.hidden_flag != 0 {
        notes.push(format!(
            "Sichtbar, solange Flag 0x{:04X} 0 ist.",
            checked.hidden_flag
        ));
    }
    Ok(Draft {
        events: vec![DraftEvent {
            kind: DraftEventKind::Object,
            matrix: Some(checked.matrix),
            x: input.x,
            z: input.z,
            look: Some(checked.look),
            target: None,
        }],
        messages: vec![DraftMessage {
            archive: Some(checked.archive),
            text: input.text.clone(),
        }],
        notes,
        ..Draft::default()
    })
}

/// Places the figure as one undo step `Figur „…“ aufgestellt`: one store transaction appends
/// the message to the header's text archive and the object (next free `local_id`, script the
/// new slot, `trainer_type` 0) to its event file; then the figure's script is appended to the
/// header's script file and compiled. The files the compile wrote (the source and its member,
/// rotom's compile state as a whole file, the text archives of the headers with that script
/// file it touched) are recorded as file steps from their state before and merged with the
/// transaction; the JSON of an archive the compile only wrote from its member (a DSPRE project
/// that has not expanded it) goes back with the script file. A compile error puts the source and the compile state back and drops the
/// transaction (nothing to redo); a compile that wrote a text literal into the figure's archive
/// is rolled back and refused (its number would collide with the store's new message).
pub fn apply(
    store: &mut Store,
    input: &NewFigureInput,
) -> Result<Outcome<NewFigureReport>, KernelError> {
    let checked = check(store, input)?;
    let Checked {
        family,
        event_file,
        script_file,
        archive,
        object,
        local_id,
        slot,
        message,
        look,
        hidden_flag,
        ..
    } = checked;
    let generation = store.change_generation();
    let mark = store.undo_depth();
    let project = store.project_arc();
    let label = label(&input.text);

    // The files as they are before the first write: the script file, the compile state and
    // the archives a compile of the file may write (its headers').
    let before_script = scripts::snapshot_script(&project, script_file)?;
    let before_state = scripts::snapshot_compile_state(&project)?;
    let saved = store
        .header_table()?
        .iter()
        .filter(|h| h.script_file_id() == script_file)
        .map(MapHeader::text_archive_id)
        .chain([archive])
        .collect::<BTreeSet<u16>>()
        .into_iter()
        .map(|a| Ok((a, scripts::snapshot_archive(&project, a)?)))
        .collect::<Result<BTreeMap<u16, ArchiveSnapshot>, KernelError>>()?;

    let figure = ObjectEvent {
        local_id,
        graphics_id: look,
        movement_type: input.movement_type,
        trainer_type: 0,
        hidden_flag,
        script: slot,
        dir: input.dir,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: input.x,
        z: input.z,
        y: 0,
    };
    let ((), transaction) = store.transaction(&label, |tx| {
        tx.edit_text_archive(archive, |doc| {
            doc.messages.push(TextMessage {
                id: default_message_id(doc.messages.len()),
                text: input.text.clone(),
                other: Default::default(),
            });
        })?;
        tx.edit_event(event_file, |f| {
            f.object_events.push(figure);
            Ok(())
        })
    })?;
    let script = figure_script(family, slot, local_id, message);
    let touched = match scripts::append_source(&project, script_file, checked.source, &script) {
        Ok(touched) => touched,
        Err(e) => {
            // The source is back; the compile state the failed compile wrote goes back too.
            scripts::restore_compile_state(&project, before_state.as_deref())?;
            store.rollback_to(mark)?;
            return Err(e);
        }
    };

    // The file steps, merged with the transaction into one step. The compile state goes back
    // as a whole file (last step): the compile wrote all of it anew.
    let mut steps = vec![scripts::script_file_step(
        &project,
        script_file,
        &before_script,
    )?];
    steps[0].rotom_entry = None;
    for (&a, snapshot) in &saved {
        let mut step = scripts::archive_file_step(&project, a, snapshot)?;
        if !touched.contains(&a) {
            // Only the JSON a compile writes from the member where DSPRE has not expanded
            // the archive (same content): it goes back with the script file, and undo does
            // not reload the archive, whose store value (the figure's message) is a part of
            // this step.
            step.asset = id(AssetKind::Script, script_file);
        }
        steps.push(step);
    }
    steps.push(scripts::compile_state_step(
        &project,
        id(AssetKind::Script, script_file),
        before_state,
    )?);
    let mut change = transaction;
    for step in steps {
        if !step.files.is_empty() || step.rotom_entry.is_some() {
            change = Some(store.record_file_step(&label, step));
        }
    }
    if touched.contains(&archive) {
        // rotom appended a text literal of the file where the figure's message goes.
        store.rollback_to(mark)?;
        return Err(refused(format!(
            "Der Compile der Scriptdatei {script_file:04} schrieb ein Text-Literal in Textarchiv \
             {archive}, in das die Nachricht der Figur kommt; erst die Scriptdatei in Bauen \
             kompilieren und speichern."
        )));
    }
    for &a in &touched {
        let touched_id = id(AssetKind::TextArchive, a);
        if store.is_loaded(touched_id) {
            store.reload_from_disk(touched_id)?;
        }
    }
    store.squash_since(mark, &label);

    Ok(Outcome {
        report: NewFigureReport {
            event_file,
            object,
            local_id,
            script_file,
            slot,
            archive,
            message,
        },
        created: Vec::new(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&[
            ("edit_dialog", "Dialog bearbeiten"),
            ("story_join", "Zu einer Geschichte"),
        ]),
    })
}
