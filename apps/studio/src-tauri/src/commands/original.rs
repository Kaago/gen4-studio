//! „Auf Original zurücksetzen“ and the list „geändert gegenüber Original“ (4.1b): the counts
//! of the base ROM per target kind, the plan of a reset, the reset as one undo step and what
//! of the saved project differs from the base ROM.
//!
//! The base ROM is found per call (`BaseRoms::locate`, which hashes only a file whose size or
//! mtime changed) and read with `BaseRom::open_unchecked`, never kept in memory. Without a
//! found base ROM the status is `unavailable` with the reason and the other commands answer
//! `base_missing` with it; the build is never affected.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::{BaseRom, base_label};
use g4_kernel::error::{KernelError, codes};
use g4_kernel::original::{
    self, ChangedProgress, ChangedReport, ResetPlan, ResetResult, ResetTarget,
};
use g4_kernel::project::Project;
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;

use super::base::found_base;
use crate::errors::CmdError;
use crate::state::AppState;

/// The base ROM of the open project as „Auf Original zurücksetzen“ needs it.
#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum OriginalStatusDto {
    /// The base ROM is found: its label and SHA-1, and the entries of the base per target kind
    /// (`original::base_counts`); the app greys out every entry at or past the count.
    Ready {
        label: String,
        sha1: String,
        counts: BTreeMap<&'static str, usize>,
    },
    /// No base mark, an unreadable mark or a base ROM that is not found: why (`no_mark()`,
    /// `base::missing_text`, the refusal of the mark).
    Unavailable { reason: String },
}

/// The open project and its base ROM, read without the session lock (the search may hash a
/// whole ROM, reading it takes a while): `base_missing` with the reason when it is not found.
pub(super) fn open_original(state: &AppState) -> Result<(Arc<Project>, BaseRom), CmdError> {
    let project = super::open_project_arc(state)?;
    let (mark, hit) = found_base(state, &project.root)?;
    let base = BaseRom::open_unchecked(&hit.path, &mark.sha1)?;
    Ok((project, base))
}

/// `session`, if it is still the one of `project` (the base ROM was read without the lock;
/// another project opened meanwhile is not planned or reset against it).
fn same_project<'s>(
    session: &'s mut crate::state::Session,
    project: &Arc<Project>,
) -> Result<&'s mut crate::state::Session, CmdError> {
    if Arc::ptr_eq(&session.project, project) {
        Ok(session)
    } else {
        Err(t!("commands.original.project_switched").into_owned().into())
    }
}

/// A kernel error of a plan or reset as the app answers it, by the kernel's code: an entry or
/// message the base does not have ([`original::no_original`], [`original::new_message`]) is
/// `no_original`; anything else as everywhere (a locked asset `read_only`, its reason after
/// „ is read-only: “).
fn reset_error(e: KernelError) -> CmdError {
    match e {
        KernelError::Coded {
            code: code @ codes::NO_ORIGINAL,
            message,
        } => CmdError::refused(code, message),
        other => other.into(),
    }
}

/// The base ROM of the open project as „Auf Original zurücksetzen“ needs it: `ready` with the
/// counts of the base per target kind, else `unavailable` with the reason. Reads the base ROM
/// once (the frontend asks once per project).
pub fn original_status_impl(state: &AppState) -> Result<OriginalStatusDto, CmdError> {
    let project = super::open_project_arc(state)?;
    let (mark, hit) = match found_base(state, &project.root) {
        Ok(found) => found,
        Err(e) if e.code == "base_missing" => {
            return Ok(OriginalStatusDto::Unavailable { reason: e.message });
        }
        Err(e) => return Err(e),
    };
    let base = BaseRom::open_unchecked(&hit.path, &mark.sha1)?;
    let counts = original::base_counts(&base, &project)?;
    Ok(OriginalStatusDto::Ready {
        label: base_label(&mark),
        sha1: mark.sha1,
        counts,
    })
}

/// What a reset of `target` would change (German lines before and after, the differing
/// header fields); changes nothing. `base_missing` without a found base ROM, `no_original`
/// for an entry the base does not have, `read_only` for a locked asset.
pub fn plan_reset_impl(state: &AppState, target: ResetTarget) -> Result<ResetPlan, CmdError> {
    let (project, base) = open_original(state)?;
    let mut guard = super::lock(state)?;
    let session = same_project(super::session(&mut guard)?, &project)?;
    original::plan_reset(&mut session.store, &base, &target).map_err(reset_error)
}

/// Resets `target` to the base ROM as one undo step (`fields`: the header fields to reset,
/// default every differing one). Store values stay unsaved until `save`; scripts, textures
/// and models are written at once (a file step undo writes back). Refused like
/// [`plan_reset_impl`], and nothing is recorded then. `ms` counts from entering this
/// function, finding and opening the base ROM included.
pub fn reset_to_original_impl(
    state: &AppState,
    target: ResetTarget,
    fields: Option<Vec<String>>,
) -> Result<ResetResult, CmdError> {
    // The gate counts from opening the base ROM to the recorded step: `ms` starts here, not
    // where the kernel's reset starts.
    let started = Instant::now();
    let (project, base) = open_original(state)?;
    let mut result = reset_in_session(state, &project, &base, target, fields)?;
    result.ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(result)
}

/// [`reset_to_original_impl`] against `original` (tests: another project's packed archives).
#[cfg(test)]
pub(crate) fn reset_with_original(
    state: &AppState,
    original: &dyn original::Original,
    target: ResetTarget,
    fields: Option<Vec<String>>,
) -> Result<ResetResult, CmdError> {
    let project = super::open_project_arc(state)?;
    reset_in_session(state, &project, original, target, fields)
}

/// The reset of [`reset_to_original_impl`] under the session lock, on the session of
/// `project` only.
fn reset_in_session(
    state: &AppState,
    project: &Arc<Project>,
    base: &dyn original::Original,
    target: ResetTarget,
    fields: Option<Vec<String>>,
) -> Result<ResetResult, CmdError> {
    let mut guard = super::lock(state)?;
    let session = same_project(super::session(&mut guard)?, project)?;
    // Scripts, textures and models are written past the store.
    session.writable()?;
    let result = original::reset(&mut session.store, base, &target, fields.as_deref());
    // A refusal (unavailable, locked) writes nothing; any other failure may come after a
    // file was written.
    let refused = matches!(
        result,
        Err(KernelError::Refused(_) | KernelError::Coded { .. } | KernelError::ReadOnly { .. })
    );
    if target.writes_files() && !refused {
        // In a text project the reset texture or model goes to `data/` as well.
        session.files_written(&target.assets());
    }
    let result = result.map_err(reset_error)?;
    session.after_write();
    Ok(result)
}

/// What of the saved project differs from the base ROM, entity by entity. The member hashes
/// of the base are cached in the app cache folder (`BaseRoms::cache_dir`; in memory without
/// one); the first run per base ROM hashes it, and every run hashes the members the build
/// state has no hash for (`progress`). The session lock is not held meanwhile.
pub fn changed_from_original_impl(
    state: &AppState,
    progress: &mut dyn FnMut(ChangedProgress),
) -> Result<ChangedReport, CmdError> {
    let (project, base) = open_original(state)?;
    let cache = state.base_roms.cache_dir();
    Ok(original::changed_from_original(
        &project,
        &base,
        cache.as_deref(),
        progress,
    )?)
}

#[tauri::command]
pub async fn original_status(app: tauri::AppHandle) -> Result<OriginalStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || original_status_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn plan_reset(app: tauri::AppHandle, target: ResetTarget) -> Result<ResetPlan, CmdError> {
    tauri::async_runtime::spawn_blocking(move || plan_reset_impl(&app.state::<AppState>(), target))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn reset_to_original(
    app: tauri::AppHandle,
    target: ResetTarget,
    fields: Option<Vec<String>>,
) -> Result<ResetResult, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        reset_to_original_impl(&app.state::<AppState>(), target, fields)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn changed_from_original(
    app: tauri::AppHandle,
    progress: Channel<ChangedProgress>,
) -> Result<ChangedReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        changed_from_original_impl(&app.state::<AppState>(), &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::commands::{
        adopt_scripts_impl, assign_base_impl, history_impl, no_mark, open_project_impl,
        open_synthetic, save_script_impl,
    };
    use g4_kernel::base::missing_text;
    use g4_kernel::original::no_original;
    use g4_kernel::project::{AssetId, AssetKind};
    use g4_kernel::start::read_base_mark;
    use g4_kernel::store::FileStep;
    use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};
    use serde_json::json;

    /// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds` (the
    /// synthetic ROM), the project open in a fresh state (as in `commands/base.rs`).
    struct Dspre {
        _tmp: tempfile::TempDir,
        root: PathBuf,
        nds: PathBuf,
        state: AppState,
    }

    fn dspre() -> Dspre {
        let s = synthetic_project();
        let (_rom_dir, rom) = synthetic_rom();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("X_DSPRE_contents");
        copy_dir(s.dir.path(), &root);
        let nds = tmp.path().join("X.nds");
        std::fs::copy(&rom, &nds).unwrap();
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        Dspre {
            _tmp: tmp,
            root,
            nds,
            state,
        }
    }

    fn text(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    /// Assigns the sibling `.nds` as the base ROM, then moves it where nothing finds it.
    /// The returned folder holds the moved ROM.
    fn assign_and_move_away(d: &Dspre) -> tempfile::TempDir {
        assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::rename(&d.nds, elsewhere.path().join("weg.nds")).unwrap();
        elsewhere
    }

    const TRAINER_1: ResetTarget = ResetTarget::Trainer { index: 1 };

    #[test]
    fn status_without_a_mark_is_unavailable_with_no_mark() {
        let d = dspre();
        let status = original_status_impl(&d.state).unwrap();
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "unavailable", "reason": no_mark() })
        );
        // No project open: an error, not a status.
        assert!(original_status_impl(&AppState::default()).is_err());
    }

    #[test]
    fn status_with_a_missing_rom_names_label_and_sha1() {
        let d = dspre();
        let _elsewhere = assign_and_move_away(&d);
        let mark = read_base_mark(&d.root).unwrap();
        let status = original_status_impl(&d.state).unwrap();
        let OriginalStatusDto::Unavailable { reason } = &status else {
            panic!("{status:?}");
        };
        assert_eq!(reason, &missing_text(&mark));
        assert!(reason.contains(&mark.sha1[..8]), "{reason}");
        assert!(reason.contains(&base_label(&mark)), "{reason}");
    }

    #[test]
    fn reset_without_a_found_base_answers_base_missing_and_records_nothing() {
        let d = dspre();
        // No mark: the reason is no_mark(), for the plan as for the reset.
        let err = plan_reset_impl(&d.state, TRAINER_1).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", no_mark().as_str())
        );
        let err = reset_to_original_impl(&d.state, TRAINER_1, None).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", no_mark().as_str())
        );

        // The base ROM vanishes between the dialog and „Zurücksetzen“ (moved away).
        let _elsewhere = assign_and_move_away(&d);
        let mark = read_base_mark(&d.root).unwrap();
        for target in [
            TRAINER_1,
            ResetTarget::Script { file: 0 },
            ResetTarget::Header { id: 0 },
        ] {
            let err = reset_to_original_impl(&d.state, target, None).unwrap_err();
            assert_eq!(
                (err.code, err.message.as_str()),
                ("base_missing", missing_text(&mark).as_str()),
                "{target:?}"
            );
            let err = plan_reset_impl(&d.state, target).unwrap_err();
            assert_eq!(err.code, "base_missing", "{target:?}");
        }
        let history = history_impl(&d.state).unwrap();
        assert!(history.undo.is_empty(), "{:?}", history.undo);
        assert!(history.redo.is_empty(), "{:?}", history.redo);
        assert!(crate::commands::dirty_impl(&d.state).unwrap().is_empty());
    }

    #[test]
    fn changed_without_a_base_answers_base_missing() {
        let d = dspre();
        let mut calls = 0;
        let err = changed_from_original_impl(&d.state, &mut |_| calls += 1).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", no_mark().as_str())
        );

        let _elsewhere = assign_and_move_away(&d);
        let mark = read_base_mark(&d.root).unwrap();
        let err = changed_from_original_impl(&d.state, &mut |_| calls += 1).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", missing_text(&mark).as_str())
        );
        assert_eq!(calls, 0, "no progress without a base");
        assert!(!d.root.join(".g4/build").exists());
    }

    #[test]
    fn availability_refusals_answer_no_original_and_the_rest_keeps_its_code() {
        // The kernel's code decides, not its words: a sentence no catalog holds.
        let foreign = "Ein deutscher Satz, den kein Katalog kennt";
        let err = reset_error(KernelError::coded("no_original", foreign));
        assert_eq!((err.code, err.message.as_str()), ("no_original", foreign));
        assert_eq!(err.asset, None);
        // A code this command does not answer is a plain error, as a refusal is.
        let err = reset_error(KernelError::coded("not_diffable", foreign));
        assert_eq!((err.code, err.message.as_str()), ("error", foreign));
        // Other refusals keep the plain mapping.
        let other = original::fields_only_for_headers();
        let err = reset_error(KernelError::Refused(other.clone()));
        assert_eq!((err.code, err.message.as_str()), ("error", other.as_str()));
        // A locked asset is `read_only` as everywhere, its German reason after the lead the
        // UI strips (`eventReadOnlyReason`).
        let asset = AssetId {
            kind: AssetKind::Trainer,
            index: 0,
        };
        let reason = "Platzhalter des Sicht-Scripts";
        let err = reset_error(KernelError::ReadOnly {
            asset,
            reason: reason.to_owned(),
        });
        assert_eq!(err.code, "read_only");
        assert_eq!(err.asset, Some(asset));
        assert!(
            err.message.ends_with(&format!(" is read-only: {reason}")),
            "{}",
            err.message
        );
    }

    #[test]
    fn save_script_forgets_the_file_steps_of_that_script() {
        let (_f, state) = open_synthetic();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        let script = |index| AssetId {
            kind: AssetKind::Script,
            index,
        };
        let step = |asset| FileStep {
            asset,
            files: Vec::new(),
            rotom_entry: None,
            kind: g4_kernel::store::FileStepKind::Reset,
        };
        {
            let mut guard = crate::commands::lock(&state).unwrap();
            let store = &mut crate::commands::session(&mut guard).unwrap().store;
            store.record_file_step("Reset to original: Script file 0002", step(script(2)));
            store.record_file_step("Reset to original: Script file 0000", step(script(0)));
        }
        let text = crate::commands::read_script_impl(&state, 0).unwrap();
        let crate::commands::ScriptSourceDto::Rotom { text } = text else {
            panic!("file 0 is a rotom script");
        };
        save_script_impl(&state, 0, text).unwrap();
        // The step of script 0 is gone; the one of script 2 stays.
        assert_eq!(
            history_impl(&state).unwrap().undo,
            ["Reset to original: Script file 0002"]
        );
    }

    /// Records empty file steps of `assets` (labelled by their index) on the open project.
    fn record_steps(state: &AppState, assets: &[AssetId]) {
        let mut guard = crate::commands::lock(state).unwrap();
        let store = &mut crate::commands::session(&mut guard).unwrap().store;
        for &asset in assets {
            let step = FileStep {
                asset,
                files: Vec::new(),
                rotom_entry: None,
                kind: g4_kernel::store::FileStepKind::Reset,
            };
            store.record_file_step(&original::asset_label(asset), step);
        }
    }

    fn script(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::Script,
            index,
        }
    }

    const MAP_0: AssetId = AssetId {
        kind: AssetKind::Map,
        index: 0,
    };

    fn disk_epoch(state: &AppState) -> u64 {
        let mut guard = crate::commands::lock(state).unwrap();
        crate::commands::session(&mut guard).unwrap().checks.epoch()
    }

    #[test]
    fn undo_and_redo_of_a_file_kind_note_a_write_to_the_files() {
        use crate::commands::{redo_impl, undo_impl};
        let (_f, state) = open_synthetic();
        record_steps(&state, &[MAP_0, script(0)]);
        let epoch = disk_epoch(&state);
        assert_eq!(undo_impl(&state).unwrap(), [script(0)]);
        assert_eq!(disk_epoch(&state), epoch + 1);
        assert_eq!(redo_impl(&state).unwrap(), [script(0)]);
        assert_eq!(disk_epoch(&state), epoch + 2);
        undo_impl(&state).unwrap();
        // A step of a store kind writes no file.
        let epoch = disk_epoch(&state);
        assert_eq!(undo_impl(&state).unwrap(), [MAP_0]);
        assert_eq!(redo_impl(&state).unwrap(), [MAP_0]);
        assert_eq!(disk_epoch(&state), epoch);
    }

    #[test]
    fn add_script_forgets_the_file_steps_of_that_script() {
        let (_f, state) = open_synthetic();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        record_steps(&state, &[script(2), script(0)]);
        crate::commands::add_script_impl(&state, 0).unwrap();
        assert_eq!(history_impl(&state).unwrap().undo, ["Script file 0002"]);
    }

    #[test]
    fn adopting_the_scripts_forgets_every_script_file_step() {
        let (_f, state) = open_synthetic();
        record_steps(&state, &[script(2), MAP_0, script(0)]);
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        assert_eq!(history_impl(&state).unwrap().undo, ["Map 0000"]);
    }

    /// The vanilla copy with its base ROM assigned (the `.nds` next to the vanilla project,
    /// only read), a trainer changed and saved, reset over the command path, saved again:
    /// the member equals the base's, and undo brings the edit back.
    #[test]
    fn reset_over_the_command_path_on_the_vanilla_copy() {
        use crate::commands::{DataKind, DataPatch, data_edit_impl, save_impl, undo_impl};
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let Some(base_rom) = g4_kernel::test_support::vanilla_base_rom() else {
            return;
        };
        let state = AppState::default();
        let cache = tempfile::tempdir().unwrap();
        state
            .base_roms
            .configure_cache(Some(cache.path().to_path_buf()));
        open_project_impl(&state, text(&root)).unwrap();
        // The copy has no sibling: assign the vanilla project's .nds by its path (only read);
        // assigning remembers it, so the search finds it again.
        assign_base_impl(&state, text(&base_rom)).unwrap();

        let OriginalStatusDto::Ready { counts, sha1, .. } = original_status_impl(&state).unwrap()
        else {
            panic!("the base ROM is found");
        };
        assert!(counts["trainer"] > 1, "{counts:?}");

        let mark = read_base_mark(&root).unwrap();
        assert_eq!(sha1, mark.sha1);
        let base = BaseRom::open_unchecked(&base_rom, &mark.sha1).unwrap();
        let project = Project::open(&root).unwrap();
        let members = |kind| {
            (
                std::fs::read(project.member_path(kind, 1)).unwrap(),
                base.member(kind, 1).unwrap().unwrap(),
            )
        };

        data_edit_impl(
            &state,
            DataKind::Trainer,
            1,
            DataPatch::Trainer {
                field: "ai".into(),
                value: 7,
            },
            None,
        )
        .unwrap();
        save_impl(&state).unwrap();
        let (project_props, base_props) = members(AssetKind::TrainerProperties);
        assert_ne!(project_props, base_props);

        // The list names the saved trainer, with its target.
        let mut steps = Vec::new();
        let started = std::time::Instant::now();
        let report = changed_from_original_impl(&state, &mut |p| steps.push(p)).unwrap();
        eprintln!(
            "changed list, first run: {:?} ({} files, {} hashed, {} recorded)",
            started.elapsed(),
            report.files,
            report.hashed,
            report.recorded
        );
        let entry = report
            .entries
            .iter()
            .find(|e| e.target == Some(TRAINER_1))
            .unwrap_or_else(|| panic!("trainer 1 not listed: {:?}", report.entries));
        assert_eq!(entry.state, original::ChangeState::Changed);
        assert!(steps.last().is_some_and(|p| p.done == p.total), "{steps:?}");
        assert!(
            cache
                .path()
                .join(format!("base/{}.json", mark.sha1))
                .is_file()
        );

        let plan = plan_reset_impl(&state, TRAINER_1).unwrap();
        assert!(!plan.same && !plan.changes.is_empty(), "{plan:?}");

        let started = std::time::Instant::now();
        let result = reset_to_original_impl(&state, TRAINER_1, None).unwrap();
        assert!(result.recorded, "{result:?}");
        eprintln!(
            "reset over the command path: {:?} (kernel {} ms)",
            started.elapsed(),
            result.ms
        );
        assert_eq!(
            history_impl(&state)
                .unwrap()
                .undo
                .last()
                .map(String::as_str),
            Some("Reset to original: Trainer 0001")
        );
        save_impl(&state).unwrap();
        for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
            let (now, base) = members(kind);
            assert_eq!(now, base, "{kind:?}");
        }
        let started = std::time::Instant::now();
        let report = changed_from_original_impl(&state, &mut |_| {}).unwrap();
        eprintln!(
            "changed list, second run (base hashes cached): {:?}",
            started.elapsed()
        );
        assert!(
            report.entries.iter().all(|e| e.target != Some(TRAINER_1)),
            "{:?}",
            report.entries
        );

        // Undo gives the asset back, with the edit.
        let restored = undo_impl(&state).unwrap();
        assert_eq!(
            restored,
            [AssetId {
                kind: AssetKind::Trainer,
                index: 1
            }]
        );
        save_impl(&state).unwrap();
        assert_eq!(members(AssetKind::TrainerProperties).0, project_props);

        // A new entity has no original.
        let past = u16::try_from(counts["trainer"]).unwrap();
        let err = plan_reset_impl(&state, ResetTarget::Trainer { index: past }).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("no_original", no_original().as_str())
        );

        // `ms` counts from entering the command (finding and opening the base included): a
        // reset waiting 150 ms for the session lock reports at least that.
        let barrier = std::sync::Barrier::new(2);
        let result = std::thread::scope(|scope| {
            scope.spawn(|| {
                let _held = crate::commands::lock(&state).unwrap();
                barrier.wait();
                std::thread::sleep(std::time::Duration::from_millis(150));
            });
            barrier.wait();
            reset_to_original_impl(&state, TRAINER_1, None).unwrap()
        });
        assert!(result.ms >= 140, "{result:?}");
    }
}
