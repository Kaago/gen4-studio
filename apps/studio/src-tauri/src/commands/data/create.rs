//! Creating trainers and encounter files, battle texts and "split for this place": the four
//! commands of plan 3b.3b over `g4_kernel::data::{trainer_create, encounter_create}`.
//! Creating writes the members at once and is not undoable (like `append_member`); a
//! refusal (`KernelError::Refused`) comes back as code "error" with the kernel's text as it
//! is, a failure after something was written (`KernelError::Partial`) as code "partial".

use std::path::Path;

use g4_kernel::data::encounter_create;
use g4_kernel::data::trainer_create::{
    self, CreateTrainerReport, TrainerText, TrainerTextsOutcome,
};
use g4_kernel::project::AssetKind;
use tauri::State;

use super::dto::{SplitDto, TrainerTextsDto};
use super::{asset, unlocked_family};
use crate::commands::{ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Creates a trainer of class `class` named `name` at the next ID (see
/// `trainer_create::create_trainer`: adopted scripts, a locked placeholder at the sight slot,
/// battle script slots for IDs above them). Not undoable: its history step (name and text
/// offset) is an undo barrier, and `undo` there is refused with "Trainer <id> anlegen lässt
/// sich nicht rückgängig machen".
pub fn create_trainer_impl(
    state: &AppState,
    class: u8,
    name: &str,
) -> Result<CreateTrainerReport, CmdError> {
    creating_trainer(state, |project, store, names| {
        trainer_create::create_trainer(project, store, names, class, name)
    })
}

/// [`create_trainer_impl`] with the battle script `meta` and flag capacity given (the
/// synthetic fixture has no `trainer_scripts.yaml` entry of its own).
#[cfg(test)]
pub(crate) fn create_trainer_with_impl(
    state: &AppState,
    meta: &g4_kernel::scripts::TrainerScriptMeta,
    class: u8,
    name: &str,
) -> Result<CreateTrainerReport, CmdError> {
    creating_trainer(state, |project, store, names| {
        trainer_create::create_trainer_with(
            project,
            store,
            names,
            meta,
            trainer_create::TRAINER_FLAG_CAPACITY,
            class,
            name,
        )
    })
}

/// Runs `create` under the session lock, then notes its writes (also those of one that
/// failed partway): the store's, and the members and battle script written past it, so the
/// checks' last report is of an older state.
fn creating_trainer(
    state: &AppState,
    create: impl FnOnce(
        &g4_kernel::project::Project,
        &mut g4_kernel::store::Store,
        &mut g4_kernel::data::names::NameTables,
    ) -> Result<CreateTrainerReport, g4_kernel::error::KernelError>,
) -> Result<CreateTrainerReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    unlocked_family(&session.project)?;
    let project = std::sync::Arc::clone(&session.project);
    let report = create(&project, &mut session.store, &mut session.names);
    session.after_write();
    // The members and the battle script are written at once, past the store.
    session.files_written(&[]);
    Ok(report?)
}

/// Sets battle texts of trainer `trainer` as one undo step and returns the assets it changed
/// with the trainer's Problems rows (source `trainer`; the text before the battle should end
/// with `\r`).
pub fn set_trainer_texts_impl(
    state: &AppState,
    trainer: u16,
    texts: Vec<TrainerText>,
) -> Result<TrainerTextsDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    super::trainer::refuse_placeholder(&mut session.store, trainer)?;
    let (ids, before) = super::trainer::text_parts_before(session)?;
    let TrainerTextsOutcome { problems, .. } =
        trainer_create::set_trainer_texts(&mut session.store, &mut session.names, trainer, texts)?;
    session.after_write();
    Ok(TrainerTextsDto {
        // Only the parts whose bytes changed (an update of an existing text touches the
        // archive alone).
        changed: super::trainer::changed(&session.store, &ids, &before),
        problems: problems
            .iter()
            .map(|p| ProblemDto::from_kernel(p, Path::new(""), "trainer"))
            .collect(),
    })
}

/// Appends an encounter file (a copy of `copy_of` as the editor shows it, or empty) and
/// returns its index. Written at once, not undoable.
pub fn create_encounter_file_impl(state: &AppState, copy_of: Option<u16>) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    let project = std::sync::Arc::clone(&session.project);
    let index = encounter_create::create_encounter_file(&project, &mut session.store, copy_of);
    session.after_write();
    Ok(index?)
}

/// Gives header `header` an encounter file of its own: a copy of its file (not undoable)
/// and the header pointed at it (one undo step, `changed`).
pub fn split_encounter_file_impl(state: &AppState, header: u16) -> Result<SplitDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    unlocked_family(&session.project)?;
    let project = std::sync::Arc::clone(&session.project);
    let split = encounter_create::split_encounter_file(&project, &mut session.store, header);
    session.after_write();
    let (index, change) = split?;
    Ok(SplitDto {
        index,
        changed: change
            .map(|_| vec![asset(AssetKind::HeaderTable, 0)])
            .unwrap_or_default(),
    })
}

#[tauri::command]
pub fn create_trainer(
    state: State<AppState>,
    class: u8,
    name: String,
) -> Result<CreateTrainerReport, CmdError> {
    create_trainer_impl(&state, class, &name)
}

#[tauri::command]
pub fn set_trainer_texts(
    state: State<AppState>,
    trainer: u16,
    texts: Vec<TrainerText>,
) -> Result<TrainerTextsDto, CmdError> {
    set_trainer_texts_impl(&state, trainer, texts)
}

#[tauri::command]
pub fn create_encounter_file(
    state: State<AppState>,
    copy_of: Option<u16>,
) -> Result<u16, CmdError> {
    create_encounter_file_impl(&state, copy_of)
}

#[tauri::command]
pub fn split_encounter_file(state: State<AppState>, header: u16) -> Result<SplitDto, CmdError> {
    split_encounter_file_impl(&state, header)
}
