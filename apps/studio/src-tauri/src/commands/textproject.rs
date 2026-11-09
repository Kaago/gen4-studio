//! The text project of the open project (4.3a): check it, export it. Importing it stays a
//! `g4k text import` (it writes members under the open store without undo).

use std::sync::Arc;

use g4_kernel::base::BaseRom;
use g4_kernel::error::KernelError;
use g4_kernel::project::Project;
use g4_kernel::textproject::{
    self, CheckReport, ExportOptions, ExportReport, not_adopted, unsynced_export_prefix,
};
use tauri::Manager;

use crate::errors::CmdError;
use crate::state::AppState;

/// What the text project of the open project holds and what an import would write: its
/// schema state, every problem with file, line and column, the members an import would
/// write. Writes nothing.
pub fn text_check_impl(state: &AppState) -> Result<CheckReport, CmdError> {
    let project = super::open_project_arc(state)?;
    Ok(textproject::check(&project)?)
}

/// The refusals of [`text_export_impl`] that need no export, in its order: unsaved changes
/// (`unsaved`), no found base ROM (`base_missing` with the reason of `base_status`), scripts
/// that are not adopted (`not_adopted`), changes of the text project the members lack, which
/// the export would throw away (`unsynced`, the build step's judgement without writing,
/// [`textproject::unsynced_export`]). Writes nothing; over MCP in the app it runs before the
/// question „Textprojekt exportieren“, so such a refusal comes back unasked. The open
/// project, its base mark and where its base ROM lies when nothing refuses.
pub fn check_text_export(
    state: &AppState,
) -> Result<(Arc<Project>, g4_kernel::start::BaseMark, std::path::PathBuf), CmdError> {
    let checked = check_before_export(state)?;
    if let Some(message) = textproject::unsynced_export(&checked.0)? {
        return Err(CmdError::refused("unsynced", message));
    }
    Ok(checked)
}

/// [`check_text_export`] up to `not_adopted`: the export judges `unsynced` itself.
fn check_before_export(
    state: &AppState,
) -> Result<(Arc<Project>, g4_kernel::start::BaseMark, std::path::PathBuf), CmdError> {
    let (project, unsaved) = {
        let mut guard = super::lock(state)?;
        let session = super::session(&mut guard)?;
        (Arc::clone(&session.project), session.store.dirty().len())
    };
    if unsaved > 0 {
        return Err(CmdError::refused("unsaved", unsaved_message(unsaved)));
    }
    let (mark, hit) = super::found_base(state, &project.root)?;
    if !g4_kernel::scripts::is_adopted(&project) {
        return Err(CmdError::refused("not_adopted", not_adopted()));
    }
    Ok((project, mark, hit.path))
}

/// Writes the open project as a text project (`g4-project.json`, `data/`) against its base
/// ROM. Refuses before writing anything: the refusals of [`check_text_export`] (adopting is
/// its own step, the export never does it here; `unsynced` from the kernel's own check);
/// every other refusal of the kernel comes with its own message.
pub fn text_export_impl(state: &AppState) -> Result<ExportReport, CmdError> {
    let (project, mark, path) = check_before_export(state)?;
    let base = BaseRom::open(&path, Some(&mark.sha1))?;
    let opts = ExportOptions {
        adopt_scripts: false,
        force: false,
    };
    textproject::export(&project, &base, &opts).map_err(export_error)
}

/// The kernel's export refusals with a code of their own: `not_adopted`, `unsynced`.
fn export_error(e: KernelError) -> CmdError {
    match e {
        KernelError::Refused(message) if message == not_adopted() => {
            CmdError::refused("not_adopted", message)
        }
        KernelError::Refused(message) if message.starts_with(&unsynced_export_prefix()) => {
            CmdError::refused("unsynced", message)
        }
        other => other.into(),
    }
}

/// The `unsaved` message of the export with the German singular for one change.
pub(crate) fn unsaved_message(n: usize) -> String {
    match n {
        1 => "Erst speichern: 1 ungespeicherte Änderung.".to_owned(),
        n => format!("Erst speichern: {n} ungespeicherte Änderungen."),
    }
}

#[tauri::command]
pub async fn text_check(app: tauri::AppHandle) -> Result<CheckReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || text_check_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn text_export(app: tauri::AppHandle) -> Result<ExportReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || text_export_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        NO_MARK, assign_base_impl, dirty_impl, open_project_impl, open_synthetic, save_impl,
        set_matrix_cell_impl,
    };
    use g4_kernel::base::missing_text;
    use g4_kernel::start::read_base_mark;
    use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};
    use g4_kernel::textproject::{DATA_DIR, MANIFEST_FILE, not_adopted};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds` (the
    /// synthetic ROM), the project open in a fresh state.
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

    impl Dspre {
        fn assign(&self) {
            assign_base_impl(&self.state, self.nds.to_str().unwrap()).unwrap();
        }
    }

    /// No manifest, no `data/`, no base layer, no adopted scripts: the export wrote nothing.
    fn wrote_nothing(root: &Path) {
        for path in [
            MANIFEST_FILE,
            DATA_DIR,
            ".g4/cache/base-layer",
            "rotom.toml",
        ] {
            assert!(!root.join(path).exists(), "{path}");
        }
    }

    #[test]
    fn text_check_without_a_manifest_says_missing() {
        let (fixture, state) = open_synthetic();
        let report = text_check_impl(&state).unwrap();
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            json!({ "schema": { "state": "missing" }, "problems": [], "differs": [] })
        );
        wrote_nothing(fixture.dir.path());
        // No project open: an error, not a report.
        assert!(text_check_impl(&AppState::default()).is_err());
    }

    #[test]
    fn unsaved_message_has_the_german_plural() {
        assert_eq!(
            unsaved_message(1),
            "Erst speichern: 1 ungespeicherte Änderung."
        );
        assert_eq!(
            unsaved_message(2),
            "Erst speichern: 2 ungespeicherte Änderungen."
        );
        assert_eq!(
            unsaved_message(7),
            "Erst speichern: 7 ungespeicherte Änderungen."
        );
    }

    #[test]
    fn text_export_refuses_unsaved_changes() {
        let d = dspre();
        d.assign();
        set_matrix_cell_impl(&d.state, 0, 0, 0, Some(1), None, None).unwrap();
        let n = dirty_impl(&d.state).unwrap().len();
        assert!(n > 0);
        let err = text_export_impl(&d.state).unwrap_err();
        assert_eq!((err.code, err.message), ("unsaved", unsaved_message(n)));
        wrote_nothing(&d.root);
        // Saved, the export goes past it (and stops at the scripts).
        save_impl(&d.state).unwrap();
        assert_eq!(text_export_impl(&d.state).unwrap_err().code, "not_adopted");
        // No project open: an error.
        assert!(text_export_impl(&AppState::default()).is_err());
    }

    #[test]
    fn the_kernels_unsynced_refusal_answers_unsynced() {
        let message = format!("{} (data/moves/0001.json); …", unsynced_export_prefix());
        let err = export_error(KernelError::Refused(message.clone()));
        assert_eq!((err.code, err.message), ("unsynced", message));
        let err = export_error(KernelError::Refused(not_adopted()));
        assert_eq!(err.code, "not_adopted");
        let err = export_error(KernelError::Refused("anders".to_owned()));
        assert_eq!(err.code, "error");
    }

    #[test]
    fn text_export_refuses_unadopted_scripts() {
        let d = dspre();
        d.assign();
        let err = text_export_impl(&d.state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("not_adopted", not_adopted().as_str())
        );
        assert!(err.problems.is_empty());
        wrote_nothing(&d.root);
    }

    #[test]
    fn text_export_without_a_base_answers_base_missing() {
        let d = dspre();
        let err = text_export_impl(&d.state).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("base_missing", NO_MARK));
        wrote_nothing(&d.root);

        // Assigned, then moved away: the reason names the missing ROM.
        d.assign();
        let mark = read_base_mark(&d.root).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::rename(&d.nds, elsewhere.path().join("weg.nds")).unwrap();
        let err = text_export_impl(&d.state).unwrap_err();
        assert_eq!(
            (err.code, err.message),
            ("base_missing", missing_text(&mark))
        );
        wrote_nothing(&d.root);

        // A broken mark: the reason is the mark's.
        std::fs::write(d.root.join(".g4/base.json"), "{kaputt").unwrap();
        let err = text_export_impl(&d.state).unwrap_err();
        assert_eq!(err.code, "base_missing");
        assert!(
            err.message
                .starts_with("Die Basismarke .g4/base.json ist nicht lesbar ("),
            "{}",
            err.message
        );
        wrote_nothing(&d.root);
    }
}
