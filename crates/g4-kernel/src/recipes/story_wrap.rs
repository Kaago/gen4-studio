//! The hull script („Hüll-Script“) a talk or reward of 3d.7 puts in front of a figure: a new
//! slot `#n` of the header's script file that runs its branch and then goes on with the
//! figure's script so far (`Jump <name>`) or ends (`End`, a figure without a script). Only
//! **this** object points at the hull; the old script stays as it is for every other figure
//! that shares it. Scripts are appended, never rewritten (decision „Scripts anhängen, nie
//! umschreiben“).
//!
//! [`check_hull`] runs every check of the figure and its files before the first write and
//! answers a [`Hull`] (the new slot, the old slot and the [`Tail`] to it, the message number);
//! the recipe builds its script text from it. [`apply_hull`] then writes as one undo step: a
//! store transaction appends the message to the header's text archive and points the object at
//! the hull, the hull is appended to the source and compiled, the compile's files are recorded
//! as file steps, the step goes into the story ([`append_step`]), and everything since the
//! recipe's mark is squashed into one step. Any error rolls back to the mark, and files the
//! compile wrote before their file steps were recorded are put back first.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use g4_formats::game::Family;

use super::story_common::{
    Figure, FigureRef, HeaderFiles, figure, header_files, message_number, next_slot, push_message,
    slot_name,
};
use super::story_scripts::Tail;
use super::types::{check_writable, id, refused};
use crate::data::trainer_create::ADOPT_FIRST;
use crate::dialog::{DialogKind, DialogSource, dialog_of};
use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};
use crate::scripts::{self, ArchiveSnapshot, ScriptSnapshot, SourceKind};
use crate::store::{ChangeId, MapHeader, Store};
use crate::stories::Step;
use crate::stories::entities::append_step;
use crate::text::json_path;

/// The refusal for a trainer: its texts are its battle texts.
pub(crate) const TRAINER_TEXT: &str = "Trainer sprechen über ihre Kampftexte (Bereich Pokémon).";

/// The refusal for a figure that speaks through common script `script`.
pub(crate) fn common_script_text(local_id: u16, script: u16) -> String {
    format!(
        "Figur {local_id} spricht über das gemeinsame Script {script}; das Rezept braucht ein \
         eigenes Script."
    )
}

/// Refuses a project whose scripts are not adopted ([`ADOPT_FIRST`]): a hull is rotom source.
pub(crate) fn require_adopted(store: &Store) -> Result<(), KernelError> {
    if scripts::is_adopted(store.project()) {
        Ok(())
    } else {
        Err(refused(ADOPT_FIRST))
    }
}

/// What a recipe asks of [`check_hull`]: the figure, the text of its message, the refusal for
/// an empty text and the name the hull script gets from the figure's `local_id` and the new
/// slot (`gespraech_<local_id>_<n>`, `belohnung_<local_id>_<n>`).
pub(crate) struct HullRequest<'a> {
    pub figure: &'a FigureRef,
    pub text: &'a str,
    pub no_text: &'a str,
    pub name: &'a dyn Fn(u16, u16) -> String,
}

/// What [`check_hull`] found, everything [`apply_hull`] needs before its first write.
#[derive(Debug, Clone)]
pub(crate) struct Hull {
    pub family: Family,
    pub files: HeaderFiles,
    pub figure: Figure,
    /// The hull's slot `#n`.
    pub slot: u16,
    /// The figure's script so far (0: none).
    pub previous_slot: u16,
    /// How the hull ends: `Jump` to the previous slot's script, or `End`.
    pub tail: Tail,
    /// The number the message gets in `files.text_archive`.
    pub message: u16,
    /// The message's text.
    pub text: String,
    /// The script file's rotom source before the hull is appended.
    source: String,
}

/// Every check of a hull before the first write, in this order: the figure (an object of the
/// header's event file), no trainer ([`TRAINER_TEXT`]), its dialog local or none
/// ([`common_script_text`] for a common script, an unreadable dialog with its reason), a rotom
/// source as the header's script file, a writable event file and text archive, a text, a local
/// slot below the common scripts, a script name the file does not use, a message number the
/// message command takes. The scripts must be adopted ([`require_adopted`], the recipe checks
/// it first).
pub(crate) fn check_hull(
    store: &mut Store,
    request: &HullRequest<'_>,
) -> Result<Hull, KernelError> {
    let at = request.figure;
    let found = figure(store, at)?;
    if found.trainer {
        return Err(refused(TRAINER_TEXT));
    }
    let files = header_files(store, at.header)?;
    let dialog = dialog_of(
        store,
        at.header,
        found.event_file,
        DialogKind::Object,
        found.index,
    )?;
    let previous_slot = match dialog.source {
        DialogSource::None => 0,
        DialogSource::Local { slot, .. } => slot,
        DialogSource::Common { script } => {
            return Err(refused(common_script_text(found.local_id, script)));
        }
        DialogSource::Trainer { .. } => return Err(refused(TRAINER_TEXT)),
        DialogSource::Unreadable { reason } => return Err(refused(reason)),
    };
    let project = store.project_arc();
    let script_file = files.script_file;
    if scripts::source_kind(&project, script_file) != SourceKind::Script {
        return Err(refused(format!(
            "Die Scriptdatei {script_file:04} von Header {} ist keine rotom-Quelle; in Bauen \
             eine Scriptdatei zuweisen.",
            at.header
        )));
    }
    check_writable(store, id(AssetKind::Event, found.event_file))?;
    let messages = store.text_archive(files.text_archive)?.messages.len();
    check_writable(store, id(AssetKind::TextArchive, files.text_archive))?;
    if request.text.trim().is_empty() {
        return Err(refused(request.no_text));
    }
    let (slot, source) = next_slot(&project, script_file)?;
    let tail = if previous_slot == 0 {
        Tail::End
    } else {
        let name = slot_name(&source, previous_slot).ok_or_else(|| {
            refused(format!(
                "Die Scriptdatei {script_file:04} hat kein Script #{previous_slot}."
            ))
        })?;
        Tail::Jump(name)
    };
    let name = (request.name)(found.local_id, slot);
    if scripts::names_identifier(&source, &name) {
        return Err(refused(format!(
            "Die Scriptdatei {script_file:04} nennt schon {name}; in Bauen umbenennen."
        )));
    }
    let family = project.family;
    let message = message_number(family, files.text_archive, messages)?;
    Ok(Hull {
        family,
        files,
        figure: found,
        slot,
        previous_slot,
        tail,
        message,
        text: request.text.to_owned(),
        source,
    })
}

/// What [`apply_hull`] writes besides the hull: the undo label, the hull's script text (built
/// from the [`Hull`]), the story and the step it gets, and the flags the story names from now
/// on (`story.flags`).
pub(crate) struct HullStep<'a> {
    pub label: &'a str,
    pub script: &'a str,
    pub story: &'a str,
    pub step: Step,
    pub flags: &'a [String],
}

/// Writes `hull` as one undo step `step.label`, everything since `mark` (the undo depth before
/// the recipe's first write: a name the recipe gave before, `name_flag`, is part of the step)
/// squashed into it: one store transaction appends the message to the header's text archive
/// and points the object at the hull's slot; the hull is appended to the script file and
/// compiled; the files the compile wrote (the source and its member, the compile state as a
/// whole file, the text archives of the headers with that script file it touched) are recorded
/// as file steps from their state before; the step goes into the story. The JSON a compile
/// writes from the member of an archive DSPRE has not expanded is removed again. A compile
/// that wrote a text literal into the hull's archive is refused (its number would collide with
/// the store's new message). Answers the change of the step.
///
/// Any error rolls everything back: files the compile wrote before their file steps are
/// recorded are put back from their state before ([`DiskBefore`]), then the store rolls back
/// to `mark` (which takes back the recorded file steps, the transaction and what the recipe
/// did before). Should taking back fail too, the error names both.
pub(crate) fn apply_hull(
    store: &mut Store,
    mark: usize,
    hull: Hull,
    step: HullStep<'_>,
) -> Result<Option<ChangeId>, KernelError> {
    let mut disk = None;
    match write_hull(store, mark, hull, step, &mut disk) {
        Ok(change) => Ok(change),
        Err(e) => {
            let project = store.project_arc();
            let restored = disk.as_ref().map_or(Ok(()), |d| d.restore(&project));
            let rolled = store.rollback_to(mark).map(|_| ());
            match restored.and(rolled) {
                Ok(()) => Err(e),
                Err(back) => Err(KernelError::InvalidProject(format!(
                    "{e}; das Zurücknehmen scheiterte: {back}"
                ))),
            }
        }
    }
}

/// The files a compile of script file `file` may write, as they were before: the script file,
/// rotom's compile state and the text archives of the headers with that script file.
struct DiskBefore {
    file: u16,
    script: ScriptSnapshot,
    state: Option<Vec<u8>>,
    archives: BTreeMap<u16, ArchiveSnapshot>,
}

impl DiskBefore {
    /// Puts every file back that differs from its state before (a file that was not there is
    /// removed).
    fn restore(&self, project: &Project) -> Result<(), KernelError> {
        let file = self.file;
        put_back(
            &scripts::source_path(project, file, SourceKind::Script),
            self.script.rotom.as_deref(),
        )?;
        put_back(
            &scripts::source_path(project, file, SourceKind::LevelScript),
            self.script.json.as_deref(),
        )?;
        put_back(
            &scripts::source_path(project, file, SourceKind::Binary),
            self.script.member.as_deref(),
        )?;
        scripts::restore_compile_state(project, self.state.as_deref())?;
        for (&a, snapshot) in &self.archives {
            put_back(&json_path(project, a), snapshot.json.as_deref())?;
            put_back(
                &project.member_path(AssetKind::TextArchive, a),
                snapshot.member.as_deref(),
            )?;
        }
        Ok(())
    }
}

/// Writes `before` to `path` (removes it for `None`) when the file differs from it.
fn put_back(path: &Path, before: Option<&[u8]>) -> Result<(), KernelError> {
    let now = match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_at(path)(e)),
    };
    if now.as_deref() == before {
        return Ok(());
    }
    match before {
        Some(bytes) => std::fs::write(path, bytes).map_err(io_at(path)),
        None => std::fs::remove_file(path).map_err(io_at(path)),
    }
}

/// [`apply_hull`] without taking back: `disk` holds the files before the compile from the
/// compile on until their file steps are recorded (then `rollback_to` takes them back).
fn write_hull(
    store: &mut Store,
    mark: usize,
    hull: Hull,
    step: HullStep<'_>,
    disk: &mut Option<DiskBefore>,
) -> Result<Option<ChangeId>, KernelError> {
    let project = store.project_arc();
    let HeaderFiles {
        script_file,
        text_archive: archive,
        event_file,
        ..
    } = hull.files;
    let label = step.label;

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

    let (index, local_id, slot) = (hull.figure.index, hull.figure.local_id, hull.slot);
    let (message, _) = store.transaction(label, |tx| {
        let message = push_message(tx, hull.family, archive, &hull.text)?;
        let mut pointed = false;
        tx.edit_event(event_file, |f| {
            if let Some(object) = f
                .object_events
                .get_mut(usize::from(index))
                .filter(|o| o.local_id == local_id)
            {
                object.script = slot;
                pointed = true;
            }
            Ok(())
        })?;
        if !pointed {
            // Checked before the first write; a failed transaction records nothing.
            return Err(refused(format!(
                "Objekt {index} der Eventdatei {event_file:04} ist nicht mehr Figur {local_id}."
            )));
        }
        Ok(message)
    })?;
    if message != hull.message {
        return Err(refused(format!(
            "Die Nachricht bekam Nummer {message} statt {} in Textarchiv {archive}.",
            hull.message
        )));
    }

    // From here on the compile's files are on disk until their file steps are recorded.
    *disk = Some(DiskBefore {
        file: script_file,
        script: before_script.clone(),
        state: before_state.clone(),
        archives: saved.clone(),
    });
    let touched = scripts::append_source(&project, script_file, hull.source, step.script)?;

    // The file steps, all worked out before the first is recorded. The compile state goes
    // back as a whole file (last step): the compile wrote all of it anew.
    let mut steps = vec![scripts::script_file_step(
        &project,
        script_file,
        &before_script,
    )?];
    steps[0].rotom_entry = None;
    for (&a, snapshot) in &saved {
        if touched.contains(&a) {
            steps.push(scripts::archive_file_step(&project, a, snapshot)?);
        }
    }
    steps.push(scripts::compile_state_step(
        &project,
        id(AssetKind::Script, script_file),
        before_state,
    )?);
    for file_step in steps {
        if !file_step.files.is_empty() || file_step.rotom_entry.is_some() {
            store.record_file_step(label, file_step);
        }
    }
    *disk = None;

    if touched.contains(&archive) {
        // rotom appended a text literal of the file where the hull's message goes.
        return Err(refused(format!(
            "Der Compile der Scriptdatei {script_file:04} schrieb ein Text-Literal in Textarchiv \
             {archive}, in das die neue Nachricht kommt; erst die Scriptdatei in Bauen \
             kompilieren und speichern."
        )));
    }
    for &a in &touched {
        let touched_id = id(AssetKind::TextArchive, a);
        if store.is_loaded(touched_id) {
            store.reload_from_disk(touched_id)?;
        }
    }
    let story = append_step(store, step.story, step.step, step.flags)?;
    store.squash_since(mark, label);
    scripts::remove_materialized_json(&project, &saved, &touched)?;
    Ok(story.change)
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
