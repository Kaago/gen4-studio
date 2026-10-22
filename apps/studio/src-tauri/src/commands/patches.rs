//! The Dynamic Headers patch: whether the game supports it, has it, and applying it.

use g4_kernel::dynamic_headers;
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// State of the Dynamic Headers patch of the open game.
#[derive(Debug, Serialize)]
pub struct HeaderPatchDto {
    /// The game has a patch (false for Diamond/Pearl).
    pub supported: bool,
    /// The patch is in the game: headers live in one file each.
    pub applied: bool,
    /// The NARC that holds the headers once patched, relative to the project root.
    pub narc: Option<String>,
}

fn status(session: &crate::state::Session) -> Result<HeaderPatchDto, CmdError> {
    let plan = dynamic_headers::plan(&session.project)?;
    Ok(HeaderPatchDto {
        supported: plan.is_some(),
        applied: session.store.dynamic_headers_applied()?,
        narc: plan.map(|p| p.narc.to_string_lossy().into_owned()),
    })
}

pub fn header_patch_status_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError> {
    let mut guard = lock(state)?;
    status(session(&mut guard)?)
}

/// Applies the patch and reports the new state. Runs under the state lock: it writes a few
/// small files and the header files, which is quick. The kernel refuses a header table with
/// unsaved changes.
pub fn apply_header_patch_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.apply_dynamic_headers()?;
    status(session)
}

#[tauri::command]
pub fn header_patch_status(state: State<AppState>) -> Result<HeaderPatchDto, CmdError> {
    header_patch_status_impl(&state)
}

#[tauri::command]
pub fn apply_header_patch(state: State<AppState>) -> Result<HeaderPatchDto, CmdError> {
    apply_header_patch_impl(&state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{open_project_impl, open_synthetic, set_header_impl};
    use super::*;

    #[test]
    fn status_reports_unsupported_for_dp() {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("header.yaml"),
            "game_title: \"POKEMON D\"\ngamecode: ADAE\nmaker_code: \"01\"\n",
        )
        .unwrap();
        // The fixture's arm9 is sized for Platinum; Diamond/Pearl's table is longer.
        let arm9 = fixture.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes.resize(0xF0000, 0);
        std::fs::write(&arm9, bytes).unwrap();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let dto = header_patch_status_impl(&state).unwrap();
        assert!(!dto.supported && !dto.applied);
        assert_eq!(dto.narc, None);
        assert!(apply_header_patch_impl(&state).is_err());
    }

    #[test]
    fn status_then_apply_then_status() {
        let (_f, state) = open_synthetic();
        let before = header_patch_status_impl(&state).unwrap();
        assert!(before.supported && !before.applied);
        assert_eq!(
            before.narc.as_deref(),
            Some("files/debug/cb_edit/d_test.narc")
        );

        let applied = apply_header_patch_impl(&state).unwrap();
        assert!(applied.supported && applied.applied);
        assert_eq!(
            applied.narc.as_deref(),
            Some("files/debug/cb_edit/d_test.narc")
        );
        let after = header_patch_status_impl(&state).unwrap();
        assert!(after.applied);
        // Applying twice is refused by the kernel.
        assert!(apply_header_patch_impl(&state).is_err());
    }

    #[test]
    fn apply_refuses_unsaved_header_edits() {
        let (_f, state) = open_synthetic();
        let mut h = super::super::get_header_impl(&state, 0).unwrap();
        if let g4_kernel::store::MapHeader::Pt(p) = &mut h {
            p.matrix_id = 1;
        }
        set_header_impl(&state, 0, h, "edit header").unwrap();
        let err = apply_header_patch_impl(&state).unwrap_err();
        assert_eq!(err.code, "error");
        assert!(err.message.contains("save"), "{}", err.message);
        assert!(!header_patch_status_impl(&state).unwrap().applied);
    }
}
