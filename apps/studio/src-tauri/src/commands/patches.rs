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
    /// The patch is in, but its routine calls a wrong function (the game hangs after the
    /// intro) and `repair_header_patch` can fix it: what DSPRE and Studio before the fix
    /// wrote for Platinum DE/FR/IT/ES.
    pub repairable: bool,
    /// The NARC that holds the headers once patched, relative to the project root.
    pub narc: Option<String>,
}

fn status(session: &crate::state::Session) -> Result<HeaderPatchDto, CmdError> {
    let plan = dynamic_headers::plan(&session.project)?;
    Ok(HeaderPatchDto {
        supported: plan.is_some(),
        applied: session.store.dynamic_headers_applied()?,
        repairable: dynamic_headers::needs_repair(&session.project)?,
        narc: plan.map(|p| p.narc.to_string_lossy().into_owned()),
    })
}

pub fn header_patch_status_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError> {
    let mut guard = lock(state)?;
    status(session(&mut guard)?)
}

/// What `apply_header_patch` did: the new state of the patch and where the old header
/// folder went.
#[derive(Debug, Serialize)]
pub struct PatchReport {
    #[serde(flatten)]
    pub status: HeaderPatchDto,
    /// The copy of an older `unpacked/dynamicHeaders` under `.g4/backup/`; `None` when
    /// there was no such folder or it was empty.
    pub snapshot: Option<String>,
}

/// Applies the patch and reports the new state and the snapshot of an older header folder.
/// Runs under the state lock: it writes a few small files and the header files, which is
/// quick. Unsaved header edits stay unsaved (the patch writes the table as it is on disk);
/// the kernel refuses a table that is behind `arm9.bin` on disk (code `stale`).
pub fn apply_header_patch_impl(state: &AppState) -> Result<PatchReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let snapshot = session.store.apply_dynamic_headers()?;
    Ok(PatchReport {
        status: status(session)?,
        snapshot: snapshot.map(|p| p.to_string_lossy().into_owned()),
    })
}

/// Repairs a patch whose routine calls a wrong function and reports the new state. Only
/// bytes of `arm9.bin` inside the routine's two calls change; the header table is not
/// involved, so unsaved edits stay as they are.
pub fn repair_header_patch_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    dynamic_headers::repair(&session.project)?;
    status(session)
}

#[tauri::command]
pub fn header_patch_status(state: State<AppState>) -> Result<HeaderPatchDto, CmdError> {
    header_patch_status_impl(&state)
}

#[tauri::command]
pub fn apply_header_patch(state: State<AppState>) -> Result<PatchReport, CmdError> {
    apply_header_patch_impl(&state)
}

#[tauri::command]
pub fn repair_header_patch(state: State<AppState>) -> Result<HeaderPatchDto, CmdError> {
    repair_header_patch_impl(&state)
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

        let applied = apply_header_patch_impl(&state).unwrap().status;
        assert!(applied.supported && applied.applied);
        assert_eq!(
            applied.narc.as_deref(),
            Some("files/debug/cb_edit/d_test.narc")
        );
        let after = header_patch_status_impl(&state).unwrap();
        assert!(after.applied && !after.repairable);
        // Applying twice is refused by the kernel, and so is repairing an intact patch.
        assert!(apply_header_patch_impl(&state).is_err());
        assert!(repair_header_patch_impl(&state).is_err());
    }

    #[test]
    fn a_patch_with_a_wrong_call_is_reported_and_repaired() {
        let (f, state) = open_synthetic();
        assert!(!header_patch_status_impl(&state).unwrap().repairable);
        apply_header_patch_impl(&state).unwrap();
        // The routine's free call (byte 14 of the routine at 0x3A024) lands two bytes short.
        let arm9 = f.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        let good = bytes[0x3A032..0x3A036].to_vec();
        bytes[0x3A034] -= 1;
        std::fs::write(&arm9, &bytes).unwrap();

        let broken = header_patch_status_impl(&state).unwrap();
        assert!(broken.applied && broken.repairable);
        let repaired = repair_header_patch_impl(&state).unwrap();
        assert!(repaired.applied && !repaired.repairable);
        assert_eq!(std::fs::read(&arm9).unwrap()[0x3A032..0x3A036], good);
    }

    #[test]
    fn apply_reports_a_table_that_changed_on_disk_as_stale() {
        use g4_kernel::project::{AssetKind, Location, Project};
        let (f, state) = open_synthetic();
        // Load the table, then let another tool edit it in arm9.bin.
        super::super::get_header_impl(&state, 0).unwrap();
        let Location::Slice { offset, .. } = Project::open(f.dir.path())
            .unwrap()
            .asset_location(AssetKind::HeaderTable, 0)
            .unwrap()
        else {
            panic!("the header table is a slice of arm9.bin before the patch");
        };
        let arm9 = f.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes[offset as usize + 2] ^= 0x5A;
        std::fs::write(&arm9, &bytes).unwrap();
        let err = apply_header_patch_impl(&state).unwrap_err();
        assert_eq!(err.code, "stale");
        assert_eq!(err.asset.map(|a| a.kind), Some(AssetKind::HeaderTable));
        assert!(!header_patch_status_impl(&state).unwrap().applied);
        assert_eq!(std::fs::read(&arm9).unwrap(), bytes);
    }

    #[test]
    fn apply_keeps_unsaved_header_edits() {
        use g4_kernel::project::AssetKind;
        use g4_kernel::store::MapHeader;
        let (f, state) = open_synthetic();
        let dir = f.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0000"), [1u8; 24]).unwrap();
        let mut h = super::super::get_header_impl(&state, 0).unwrap();
        let MapHeader::Pt(p) = &mut h else {
            panic!("the fixture is Platinum");
        };
        let old = p.matrix_id;
        p.matrix_id = old + 1;
        set_header_impl(&state, 0, h, "edit header").unwrap();

        let report = apply_header_patch_impl(&state).unwrap();
        assert!(report.status.applied);
        let snapshot = report.snapshot.expect("the old folder was kept");
        assert!(
            std::path::Path::new(&snapshot).starts_with(f.dir.path().join(".g4/backup")),
            "{snapshot}"
        );
        assert_eq!(
            std::fs::read(std::path::Path::new(&snapshot).join("0000")).unwrap(),
            [1u8; 24]
        );
        // The edit stays unsaved; the header file holds the value from disk.
        let MapHeader::Pt(p) = super::super::get_header_impl(&state, 0).unwrap() else {
            panic!("the fixture is Platinum");
        };
        assert_eq!(p.matrix_id, old + 1);
        assert_eq!(
            &std::fs::read(dir.join("0000")).unwrap()[2..4],
            &old.to_le_bytes()
        );
        let dirty = super::super::dirty_impl(&state).unwrap();
        assert!(dirty.iter().any(|a| a.kind == AssetKind::HeaderTable));
        assert!(
            apply_header_patch_impl(&state).is_err(),
            "applying twice is refused"
        );
    }

    #[test]
    fn the_report_has_the_state_and_the_snapshot() {
        let (_f, state) = open_synthetic();
        let report = serde_json::to_value(apply_header_patch_impl(&state).unwrap()).unwrap();
        assert_eq!(report["applied"], true);
        assert_eq!(report["supported"], true);
        assert!(report["snapshot"].is_null(), "{report}");
    }
}
