//! „Löschen…“ (3d.4): what deleting an entity would do, the tombstone as one undo step,
//! „Nummer freigeben“ (irreversible) and „Wiederherstellen“.
//!
//! Whether an entity is original asks the base ROM, so the plan and the delete find it per
//! call like „Auf Original zurücksetzen“ (`original.rs`): without a found base ROM they answer
//! `base_missing` with the reason, but for an own flag or variable name, which is never
//! original. A delete the references lock answers `referenced` with
//! them as the app shows them (label, source, where a click leads). Restoring needs no base.

use std::sync::Arc;
use std::time::Instant;

use g4_kernel::delete::{self, DeletePlan, DeleteResult, DeleteTarget, FreeInfo};
use g4_kernel::error::KernelError;
use g4_kernel::original::{MissingBase, Original};
use g4_kernel::project::Project;
use serde::Serialize;
use tauri::Manager;

use super::RefDto;
use super::original::open_original;
use super::refs::ref_dtos;
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// What deleting an entity would do, for the dialog „Löschen…“ and MCP `plan_delete`: the
/// kernel's plan with the references as the app shows them.
#[derive(Debug, Serialize)]
pub struct DeletePlanDto {
    pub target: DeleteTarget,
    pub label: String,
    /// Why a tombstone is refused; `None`: „Löschen“ is possible (unless `references` lock).
    pub refusal: Option<String>,
    /// The references that lock the delete: „{n} Verweise: zuerst umhängen“.
    pub references: Vec<RefDto>,
    /// The references that only warn (soft sources; none in 3d.4).
    pub warnings: Vec<RefDto>,
    /// An entity of the base ROM: deleting it warns (the entry stays in the ROM).
    pub original: bool,
    /// The entity is a tombstone now.
    pub deleted: bool,
    /// What „Nummer freigeben“ would remove; `None` when it cannot (`cannot_free`).
    pub can_free: Option<FreeInfo>,
    /// Why the number cannot be freed.
    pub cannot_free: Option<String>,
}

/// `session`, if it is still the one of `project` (the base ROM was read without the lock;
/// another project opened meanwhile is not planned or changed against it).
fn current<'s>(
    session: &'s mut Session,
    project: &Arc<Project>,
) -> Result<&'s mut Session, CmdError> {
    if Arc::ptr_eq(&session.project, project) {
        Ok(session)
    } else {
        Err("Das Projekt wurde gewechselt; nichts gelöscht.".into())
    }
}

/// The open project and the base ROM `target` is planned against, read without the session
/// lock: `base_missing` without a found base ROM, unless the target needs none.
fn original_for(
    state: &AppState,
    target: DeleteTarget,
) -> Result<(Arc<Project>, Box<dyn Original>), CmdError> {
    match open_original(state) {
        Ok((project, base)) => Ok((project, Box::new(base))),
        Err(e) if e.code == "base_missing" && !target.needs_base() => Ok((
            super::open_project_arc(state)?,
            Box::new(MissingBase(e.message)),
        )),
        Err(e) => Err(e),
    }
}

fn ms_since(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// What deleting `target` would do; changes nothing. `base_missing` without a found base ROM
/// (but for an own flag or variable name), an `error` for an entity that does not exist.
pub fn plan_delete_impl(state: &AppState, target: DeleteTarget) -> Result<DeletePlanDto, CmdError> {
    let (project, base) = original_for(state, target)?;
    plan_in_session(state, &project, base.as_ref(), target)
}

/// [`plan_delete_impl`] against `original` (tests: another project's packed archives).
#[cfg(test)]
pub(crate) fn plan_with_original(
    state: &AppState,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeletePlanDto, CmdError> {
    let project = super::open_project_arc(state)?;
    plan_in_session(state, &project, original, target)
}

fn plan_in_session(
    state: &AppState,
    project: &Arc<Project>,
    original: &dyn Original,
    target: DeleteTarget,
) -> Result<DeletePlanDto, CmdError> {
    let mut guard = super::lock(state)?;
    let session = current(super::session(&mut guard)?, project)?;
    let Session { store, refs, .. } = &mut *session;
    match delete::plan_delete(store, refs, original, target) {
        Ok(plan) => Ok(plan_dto(session, plan)),
        Err(e) => Err(CmdError::in_session(session, e)),
    }
}

fn plan_dto(session: &mut Session, plan: DeletePlan) -> DeletePlanDto {
    DeletePlanDto {
        target: plan.target,
        label: plan.label,
        refusal: plan.refusal,
        references: ref_dtos(session, &plan.references),
        warnings: ref_dtos(session, &plan.warnings),
        original: plan.original,
        deleted: plan.deleted,
        can_free: plan.can_free,
        cannot_free: plan.cannot_free,
    }
}

/// Deletes `target` as one undo step „Gelöscht: {label}“ (a tombstone; an own flag or
/// variable name and the last own place go as a file step), or with `free_number` frees its
/// number for good („Nummer freigegeben: {label}“, an undo barrier). Refused like the plan:
/// its refusal (`error`), the references that lock it (`referenced` with `references`),
/// `base_missing`; nothing is recorded then. Afterwards the reference index follows and the
/// checks see the files a file step or „Nummer freigeben“ wrote. `ms` counts from entering
/// this function, finding and opening the base ROM included.
pub fn delete_entity_impl(
    state: &AppState,
    target: DeleteTarget,
    free_number: bool,
) -> Result<DeleteResult, CmdError> {
    let started = Instant::now();
    let (project, base) = original_for(state, target)?;
    let mut result = delete_in_session(state, &project, base.as_ref(), target, free_number)?;
    result.ms = ms_since(started);
    Ok(result)
}

/// [`delete_entity_impl`] against `original` (tests: another project's packed archives).
#[cfg(test)]
pub(crate) fn delete_with_original(
    state: &AppState,
    original: &dyn Original,
    target: DeleteTarget,
    free_number: bool,
) -> Result<DeleteResult, CmdError> {
    let project = super::open_project_arc(state)?;
    delete_in_session(state, &project, original, target, free_number)
}

fn delete_in_session(
    state: &AppState,
    project: &Arc<Project>,
    original: &dyn Original,
    target: DeleteTarget,
    free_number: bool,
) -> Result<DeleteResult, CmdError> {
    let mut guard = super::lock(state)?;
    let session = current(super::session(&mut guard)?, project)?;
    // Names, places and „Nummer freigeben“ write files past the store.
    session.writable()?;
    let Session { store, refs, .. } = &mut *session;
    let result = if free_number {
        delete::free_number(store, refs, original, target)
    } else {
        delete::delete(store, refs, original, target)
    };
    let names = matches!(target, DeleteTarget::Flag { .. } | DeleteTarget::Var { .. });
    let writes_files = free_number || names || matches!(target, DeleteTarget::Place { .. });
    // A refusal writes nothing; any other failure may come after a file went.
    let refused = matches!(
        result,
        Err(KernelError::Refused(_)
            | KernelError::Referenced { .. }
            | KernelError::ReadOnly { .. })
    );
    if writes_files && !refused {
        session.files_written(&[]);
    }
    let result = match result {
        Ok(result) => result,
        Err(e) => return Err(CmdError::in_session(session, e)),
    };
    if names {
        // The id files for rotom were written without the name.
        session.id_files_changed();
    }
    session.after_write();
    if free_number && matches!(target, DeleteTarget::Script { .. }) {
        // The freed script sources are gone now, whatever the index's time gate says.
        session.scripts_written();
    }
    Ok(result)
}

/// Takes the tombstone of `target` back as one undo step „Wiederhergestellt: {label}“;
/// refused when it is not deleted. Needs no base ROM.
pub fn restore_entity_impl(
    state: &AppState,
    target: DeleteTarget,
) -> Result<DeleteResult, CmdError> {
    let started = Instant::now();
    let mut guard = super::lock(state)?;
    let session = super::session(&mut guard)?;
    let mut result = delete::restore(&mut session.store, target)?;
    session.after_write();
    result.ms = ms_since(started);
    Ok(result)
}

#[tauri::command]
pub async fn plan_delete(
    app: tauri::AppHandle,
    target: DeleteTarget,
) -> Result<DeletePlanDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || plan_delete_impl(&app.state::<AppState>(), target))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn delete_entity(
    app: tauri::AppHandle,
    target: DeleteTarget,
    free_number: Option<bool>,
) -> Result<DeleteResult, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_entity_impl(
            &app.state::<AppState>(),
            target,
            free_number.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub fn restore_entity(
    state: tauri::State<AppState>,
    target: DeleteTarget,
) -> Result<DeleteResult, CmdError> {
    restore_entity_impl(&state, target)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::commands::{
        DataKind, EventKind, EventPatch, NO_MARK, NavTargetDto, add_event_impl, data_list_impl,
        get_events_impl, get_header_impl, get_matrix_impl, history_impl, list_headers_impl,
        open_project_impl, open_synthetic, set_header_impl, set_matrix_cell_impl, undo_impl,
        update_event_impl,
    };
    use g4_formats::narc::Narc;
    use g4_kernel::delete::{CANNOT_FREE_ORIGINAL, IRREVERSIBLE, NOT_A_TEXT_PROJECT};
    use g4_kernel::project::AssetKind;
    use g4_kernel::store::MapHeader;
    use g4_kernel::test_support::{ProjectOriginal, TextFixture};

    /// The synthetic text project (exported, scripts adopted) open in a fresh state, its
    /// header-only ROM as the base.
    pub(crate) fn text_project() -> (TextFixture, AppState) {
        let fx = TextFixture::exported();
        let state = AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        (fx, state)
    }

    /// The base of the fixture: its packed archives, with `trainers` trainers (so the ones
    /// from there on are new).
    pub(crate) struct Base {
        inner: ProjectOriginal,
        trainers: Option<usize>,
    }

    impl Base {
        pub(crate) fn of(root: &Path, trainers: Option<usize>) -> Base {
            Base {
                inner: ProjectOriginal::new(root),
                trainers,
            }
        }
    }

    impl Original for Base {
        fn sha1(&self) -> &str {
            self.inner.sha1()
        }
        fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
            self.inner.member(kind, index)
        }
        fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError> {
            match (kind, self.trainers) {
                (AssetKind::TrainerProperties | AssetKind::TrainerParty, Some(n)) => Ok(Some(n)),
                _ => self.inner.count(kind),
            }
        }
        fn header_table(&self) -> Result<Vec<MapHeader>, KernelError> {
            self.inner.header_table()
        }
        fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError> {
            self.inner.narc_at(packed)
        }
    }

    const TRAINER_1: DeleteTarget = DeleteTarget::Trainer { index: 1 };

    fn last_step(state: &AppState) -> String {
        history_impl(state)
            .unwrap()
            .undo
            .last()
            .cloned()
            .unwrap_or_default()
    }

    fn trainer_deleted(state: &AppState, index: u16) -> bool {
        data_list_impl(state, DataKind::Trainer, None).unwrap()[usize::from(index)].deleted
    }

    #[test]
    fn plan_and_delete_over_the_command_path() {
        let (fx, state) = text_project();
        let base = Base::of(fx.root(), None);
        let plan = plan_with_original(&state, &base, TRAINER_1).unwrap();
        assert_eq!(plan.refusal, None);
        assert!(plan.references.is_empty() && plan.warnings.is_empty());
        assert!(plan.original && !plan.deleted);
        assert_eq!(plan.cannot_free.as_deref(), Some(CANNOT_FREE_ORIGINAL));
        assert_eq!(plan.label, "Trainer 0001");
        let json = serde_json::to_value(&plan).unwrap();
        assert_eq!(
            json["target"],
            serde_json::json!({ "kind": "trainer", "index": 1 })
        );
        assert_eq!(json["can_free"], serde_json::Value::Null);

        let depth = history_impl(&state).unwrap().undo.len();
        let result = delete_with_original(&state, &base, TRAINER_1, false).unwrap();
        assert!(result.change.is_some(), "{result:?}");
        assert_eq!(result.freed, None);
        assert_eq!(history_impl(&state).unwrap().undo.len(), depth + 1);
        assert_eq!(last_step(&state), "Gelöscht: Trainer 0001");
        assert!(trainer_deleted(&state, 1));
        assert!(
            plan_with_original(&state, &base, TRAINER_1)
                .unwrap()
                .deleted
        );

        let result = restore_entity_impl(&state, TRAINER_1).unwrap();
        assert!(result.change.is_some(), "{result:?}");
        assert_eq!(last_step(&state), "Wiederhergestellt: Trainer 0001");
        assert!(!trainer_deleted(&state, 1));
        let err = restore_entity_impl(&state, TRAINER_1).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("error", "Trainer 0001 ist nicht gelöscht.")
        );

        // Without a base ROM the plan and the delete answer base_missing, recording nothing.
        let (_s, dspre) = open_synthetic();
        let err = plan_delete_impl(&dspre, TRAINER_1).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("base_missing", NO_MARK));
        let err = delete_entity_impl(&dspre, TRAINER_1, false).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("base_missing", NO_MARK));
        assert!(history_impl(&dspre).unwrap().undo.is_empty());
        // An own flag name is never original: its plan needs no base ROM.
        let plan = plan_delete_impl(&dspre, DeleteTarget::Flag { number: 148 }).unwrap();
        assert_eq!(plan.refusal.as_deref(), Some(NOT_A_TEXT_PROJECT));
    }

    #[test]
    fn a_referenced_trainer_answers_referenced_with_nav() {
        let (fx, state) = text_project();
        let base = Base::of(fx.root(), None);
        // A figure of event file 0 starts the battle of trainer 1 (script 3000 + 1 - 1).
        let patch = EventPatch {
            script: Some(3000),
            trainer_type: Some(1),
            ..EventPatch::default()
        };
        let figure = add_event_impl(&state, 0, EventKind::Object, 1, 1, Some(patch), None).unwrap();
        let depth = history_impl(&state).unwrap().undo.len();

        let err = delete_with_original(&state, &base, TRAINER_1, false).unwrap_err();
        assert_eq!(err.code, "referenced", "{err:?}");
        assert_eq!(err.references.len(), 1, "{:?}", err.references);
        let NavTargetDto::Event {
            event_file,
            event_kind,
            index,
            ..
        } = &err.references[0].nav
        else {
            panic!("{:?}", err.references[0].nav);
        };
        assert_eq!((*event_file, *event_kind, *index), (0, "object", figure));
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["references"][0]["nav"]["kind"], "event");
        // The plan names the same reference; nothing was recorded.
        let plan = plan_with_original(&state, &base, TRAINER_1).unwrap();
        assert_eq!(plan.references, err.references);
        assert_eq!(history_impl(&state).unwrap().undo.len(), depth);
        assert!(!trainer_deleted(&state, 1));
    }

    #[test]
    fn free_number_needs_the_flag_and_is_irreversible() {
        let (fx, state) = text_project();
        // Trainer 2, the last, is new.
        let base = Base::of(fx.root(), Some(2));
        let target = DeleteTarget::Trainer { index: 2 };
        let plan = plan_with_original(&state, &base, target).unwrap();
        assert!(!plan.original);
        assert_eq!(plan.can_free.as_ref().map(|f| f.count), Some(1));

        // Without the flag: a tombstone, nothing freed.
        let result = delete_with_original(&state, &base, target, false).unwrap();
        assert_eq!(result.freed, None);
        assert!(trainer_deleted(&state, 2));
        assert!(fx.root().join("data/trainers/0002.json").is_file());

        let result = delete_with_original(&state, &base, target, true).unwrap();
        assert_eq!(
            result.freed.as_ref().map(|f| f.count),
            Some(1),
            "{result:?}"
        );
        assert!(!fx.root().join("data/trainers/0002.json").exists());
        assert_eq!(
            data_list_impl(&state, DataKind::Trainer, None)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(last_step(&state), "Nummer freigegeben: Trainer 0002");
        // Undo stops at the barrier.
        let err = undo_impl(&state).unwrap_err();
        assert_eq!(err.message, IRREVERSIBLE);
    }

    /// Sets tombstones on the open project's store as one step.
    fn tombstone(state: &AppState, targets: &[DeleteTarget]) {
        let mut guard = crate::commands::lock(state).unwrap();
        let store = &mut crate::commands::session(&mut guard).unwrap().store;
        store
            .transaction("Gelöscht", |tx| {
                targets
                    .iter()
                    .try_for_each(|&target| tx.set_deleted(target, true))
            })
            .unwrap();
    }

    #[test]
    fn the_lists_carry_deleted() {
        let (_fx, state) = text_project();
        let header = 1;
        let file = get_header_impl(&state, header).unwrap().event_file_id();
        tombstone(
            &state,
            &[
                TRAINER_1,
                DeleteTarget::Header { id: header },
                DeleteTarget::EventFile { index: file },
                DeleteTarget::Matrix { index: 0 },
                DeleteTarget::Encounter { index: 0 },
            ],
        );
        let trainers = data_list_impl(&state, DataKind::Trainer, None).unwrap();
        let deleted: Vec<u16> = trainers
            .iter()
            .filter(|r| r.deleted)
            .map(|r| r.index)
            .collect();
        assert_eq!(deleted, [1]);
        let encounters = data_list_impl(&state, DataKind::Encounter, None).unwrap();
        assert!(encounters[0].deleted);
        assert!(encounters.iter().skip(1).all(|r| !r.deleted));
        // Other kinds have no tombstones.
        let species = data_list_impl(&state, DataKind::Species, None).unwrap();
        assert!(species.iter().all(|r| !r.deleted));
        let headers = list_headers_impl(&state).unwrap().headers;
        let deleted: Vec<u16> = headers.iter().filter(|h| h.deleted).map(|h| h.id).collect();
        assert_eq!(deleted, [header]);
        assert!(get_events_impl(&state, file).unwrap().deleted);
        assert!(get_matrix_impl(&state, 0).unwrap().deleted);
        let json = serde_json::to_value(get_events_impl(&state, file).unwrap()).unwrap();
        assert_eq!(json["deleted"], true);

        // Undo takes them back.
        undo_impl(&state).unwrap();
        assert!(!get_matrix_impl(&state, 0).unwrap().deleted);
        assert!(
            list_headers_impl(&state)
                .unwrap()
                .headers
                .iter()
                .all(|h| !h.deleted)
        );
    }

    #[test]
    fn new_references_to_a_tombstone_are_refused() {
        let (_fx, state) = text_project();
        let header = get_header_impl(&state, 0).unwrap();
        let file = header.event_file_id();
        let other = if file == 0 { 1 } else { 0 };
        let to_header_1 = || EventPatch {
            dest_header_id: Some(1),
            ..EventPatch::default()
        };
        let warp = add_event_impl(&state, file, EventKind::Warp, 2, 2, None, None).unwrap();
        // A warp to header 1 before it was deleted.
        let kept = add_event_impl(
            &state,
            file,
            EventKind::Warp,
            5,
            5,
            Some(to_header_1()),
            None,
        )
        .unwrap();
        tombstone(
            &state,
            &[
                DeleteTarget::Header { id: 1 },
                DeleteTarget::EventFile { index: other },
                DeleteTarget::Map { index: 1 },
            ],
        );
        let depth = history_impl(&state).unwrap().undo.len();
        let refused = |result: Result<(), CmdError>, why: &str| {
            let err = result.unwrap_err();
            assert_eq!((err.code, err.message.as_str()), ("error", why));
        };

        // A warp to header 1.
        refused(
            update_event_impl(&state, file, EventKind::Warp, warp, to_header_1()),
            "Header 0001 ist gelöscht.",
        );
        refused(
            add_event_impl(
                &state,
                file,
                EventKind::Warp,
                3,
                3,
                Some(to_header_1()),
                None,
            )
            .map(drop),
            "Header 0001 ist gelöscht.",
        );
        // The header's event file.
        let mut edited = header.clone();
        match &mut edited {
            MapHeader::DP(h) => h.event_file_id = other,
            MapHeader::Pt(h) => h.event_file_id = other,
            MapHeader::HGSS(h) => h.event_file_id = other,
        }
        refused(
            set_header_impl(&state, 0, edited, "edit header"),
            &format!("Eventdatei {other:04} ist gelöscht."),
        );
        // A matrix cell's map and header.
        refused(
            set_matrix_cell_impl(&state, 0, 0, 0, Some(1), None, None),
            "Karte 0001 ist gelöscht.",
        );
        let matrix = get_matrix_impl(&state, 0).unwrap();
        if matrix.headers.is_some() {
            refused(
                set_matrix_cell_impl(&state, 0, 0, 0, None, Some(1), None),
                "Header 0001 ist gelöscht.",
            );
        }
        assert_eq!(history_impl(&state).unwrap().undo.len(), depth);

        // A field left as it is, or another one, is not refused.
        set_header_impl(&state, 0, header, "edit header").unwrap();
        let moved = EventPatch {
            x: Some(6),
            dest_header_id: Some(1),
            ..EventPatch::default()
        };
        update_event_impl(&state, file, EventKind::Warp, kept, moved).unwrap();
    }

    /// The vanilla copy, set up as text project with its base ROM: a new trainer deleted
    /// (one step), undone, then its number freed (the JSON goes). Prints the time of the
    /// trainer list with its tombstone flags.
    #[test]
    fn delete_over_the_command_path_on_the_vanilla_copy() {
        use crate::commands::{adopt_text_project_impl, assign_base_impl, create_trainer_impl};
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let Some(base_rom) = g4_kernel::test_support::vanilla_base_rom() else {
            return;
        };
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        assign_base_impl(&state, base_rom.to_str().unwrap()).unwrap();
        let started = Instant::now();
        adopt_text_project_impl(&state, &mut |_| {}).unwrap();
        eprintln!("text project set up: {:?}", started.elapsed());

        let created = create_trainer_impl(&state, 1, "Lösch").unwrap();
        let index = created.id;
        let target = DeleteTarget::Trainer { index };
        let json = root.join(format!("data/trainers/{index:04}.json"));

        let plan = plan_delete_impl(&state, target).unwrap();
        assert!(!plan.original && plan.refusal.is_none(), "{plan:?}");
        assert_eq!(plan.can_free.as_ref().map(|f| f.count), Some(1), "{plan:?}");

        let depth = history_impl(&state).unwrap().undo.len();
        let result = delete_entity_impl(&state, target, false).unwrap();
        eprintln!("delete over the command path: {} ms", result.ms);
        assert_eq!(history_impl(&state).unwrap().undo.len(), depth + 1);
        let started = Instant::now();
        let rows = data_list_impl(&state, DataKind::Trainer, None).unwrap();
        eprintln!(
            "trainer list with tombstones ({} rows): {:?}",
            rows.len(),
            started.elapsed()
        );
        assert!(rows[usize::from(index)].deleted);
        assert_eq!(rows.iter().filter(|r| r.deleted).count(), 1);
        undo_impl(&state).unwrap();
        assert!(!trainer_deleted(&state, index));

        let result = delete_entity_impl(&state, target, true).unwrap();
        assert_eq!(
            result.freed.as_ref().map(|f| f.count),
            Some(1),
            "{result:?}"
        );
        // As `git status` would see it: the JSON is gone.
        assert!(!json.exists(), "{}", json.display());
        assert_eq!(
            data_list_impl(&state, DataKind::Trainer, None)
                .unwrap()
                .len(),
            usize::from(index)
        );
        assert_eq!(undo_impl(&state).unwrap_err().message, IRREVERSIBLE);
    }
}
