//! The base ROM of the open project and the patch against it (Technik › Veröffentlichen):
//! status, check, assign, register a moved ROM, create the patch.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_kernel::base::{BaseRom, BaseSource, Located, base_label, missing_text, other_rom};
use g4_kernel::patch::{self, PatchReport, PatchTargets};
use g4_kernel::project::Project;
use g4_kernel::start::{self, BaseCheck, BaseMark, MarkState};
use serde::Serialize;
use tauri::Manager;

use super::BuildDto;
use crate::errors::CmdError;
use crate::mcp::settings::ProjectKey;
use crate::state::AppState;

/// Why there is no patch for a project without a base mark; the kernel's sentence.
pub fn no_mark() -> String {
    g4_kernel::patch::no_mark()
}

/// The base ROM of the open project, as Technik › Veröffentlichen shows it.
#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum BaseStatusDto {
    /// No base mark; `suggestion` is the `.nds` DSPRE unpacked the folder from (canonical),
    /// if it lies next to it and is no original.
    Unassigned { suggestion: Option<String> },
    /// A mark, but its ROM is nowhere to be found; `reason` says which ROM is missing
    /// (`base::missing_text`), or why the ROM `--base-rom` names is not it.
    Missing {
        mark: BaseMark,
        label: String,
        reason: String,
    },
    /// `patch_name` is `<Projekt>-<Spielcode>`, the name `create_patch` writes under `build/`
    /// (`patch::patch_name`), so the app can say where before it asks.
    Found {
        mark: BaseMark,
        label: String,
        path: String,
        source: BaseSource,
        patch_name: String,
    },
    /// `.g4/base.json` is broken or from a newer version; `reason` is the kernel's sentence
    /// (`MarkState::refusal`). The build is not affected.
    Unreadable { reason: String },
}

/// What `create_patch` did: the patch report, and the build of the ROM it patches to.
#[derive(Debug, Serialize)]
pub struct PatchDto {
    #[serde(flatten)]
    pub report: PatchReport,
    pub build: BuildDto,
}

/// What the project at `root` has as its base ROM.
enum Base {
    Unassigned(Option<PathBuf>),
    Unreadable(String),
    Missing(BaseMark, String),
    Found(BaseMark, Located),
}

impl Base {
    fn dto(self, project: &Project) -> BaseStatusDto {
        match self {
            Base::Unassigned(suggestion) => BaseStatusDto::Unassigned {
                suggestion: suggestion.map(|p| p.display().to_string()),
            },
            Base::Unreadable(reason) => BaseStatusDto::Unreadable { reason },
            Base::Missing(mark, reason) => BaseStatusDto::Missing {
                label: base_label(&mark),
                mark,
                reason,
            },
            Base::Found(mark, hit) => BaseStatusDto::Found {
                label: base_label(&mark),
                mark,
                path: hit.path.display().to_string(),
                source: hit.source,
                patch_name: patch::patch_name(project),
            },
        }
    }
}

/// Reads the mark of the project at `root` and searches its base ROM (which may hash a
/// whole ROM, see `BaseRoms::locate`). A broken mark is a state, never an error.
fn base_of(state: &AppState, root: &Path) -> Base {
    let mark = match start::mark_state(root) {
        MarkState::Missing => return Base::Unassigned(suggestion(state, root)),
        MarkState::Present(mark) => mark,
        other => return Base::Unreadable(other.refusal().unwrap_or_default()),
    };
    match state.base_roms.locate(root, &mark) {
        Ok(Some(hit)) => Base::Found(mark, hit),
        Ok(None) => {
            let reason = missing_text(&mark);
            Base::Missing(mark, reason)
        }
        // The ROM `--base-rom` names is another one, or a candidate could not be read.
        Err(e) => {
            let reason = e.to_string();
            Base::Missing(mark, reason)
        }
    }
}

/// `suggest_base(root)`, canonical, unless it is an original under `G4_ROMS_DIR`.
fn suggestion(state: &AppState, root: &Path) -> Option<PathBuf> {
    start::suggest_base(root)
        .map(|p| p.canonicalize().unwrap_or(p))
        .filter(|p| !state.base_roms.is_original(p))
}

/// The `base_missing` error: no base ROM found, `message` says why.
pub(crate) fn base_missing(message: String) -> CmdError {
    CmdError {
        code: "base_missing",
        message,
        asset: None,
        problems: Vec::new(),
        references: Vec::new(),
    }
}

/// The mark and the found base ROM of the project at `root` (searched as [`base_of`]);
/// without one `base_missing` with the reason of the status ([`no_mark`],
/// `base::missing_text`, the refusal of an unreadable mark).
pub(crate) fn found_base(state: &AppState, root: &Path) -> Result<(BaseMark, Located), CmdError> {
    match base_of(state, root) {
        Base::Found(mark, hit) => Ok((mark, hit)),
        Base::Unassigned(_) => Err(base_missing(no_mark())),
        Base::Missing(_, reason) | Base::Unreadable(reason) => Err(base_missing(reason)),
    }
}

/// The base ROM of the open project; searches it like the patch does.
pub fn base_status_impl(state: &AppState) -> Result<BaseStatusDto, CmdError> {
    let project = super::open_project_arc(state)?;
    Ok(base_of(state, &project.root).dto(&project))
}

/// Checks the `.nds` at `nds` as the base ROM of the open project; writes nothing.
pub fn check_base_impl(state: &AppState, nds: &str) -> Result<BaseCheck, CmdError> {
    let project = super::open_project_arc(state)?;
    Ok(start::check_base(&project.root, Path::new(nds))?)
}

/// Whether `assign_base_impl` would take the `.nds` at `nds` (its check, or the refusal it
/// would answer); writes nothing.
pub fn check_assign_impl(state: &AppState, nds: &str) -> Result<BaseCheck, CmdError> {
    let project = super::open_project_arc(state)?;
    Ok(start::check_assign(&project.root, Path::new(nds))?)
}

/// Assigns the `.nds` at `nds` (the kernel writes the mark), moves the agent grant of the
/// project to its new key (`<sha1>/<created>`), remembers the ROM and answers the new status.
/// A ROM that cannot be remembered is searched for again later; a grant that cannot be moved
/// is `partial`, since the mark is written.
pub fn assign_base_impl(state: &AppState, nds: &str) -> Result<BaseStatusDto, CmdError> {
    let project = super::open_project_arc(state)?;
    let root = &project.root;
    let before = ProjectKey::of(root)?;
    let nds = Path::new(nds);
    let (check, _mark) = start::assign_base(root, nds)?;
    let after = ProjectKey::of(root)?;
    if after != before {
        state.agents.move_grant(&before, after).map_err(|e| {
            CmdError::partial(format!(
                "Basis-ROM zugeordnet, die Agenten-Freigabe aber nicht umgeschrieben: {e}"
            ))
        })?;
    }
    if let Err(e) = state.base_roms.register(nds, &check.info) {
        eprintln!("Basis-ROM nicht gemerkt: {e}");
    }
    Ok(base_of(state, root).dto(&project))
}

/// „Basis-ROM suchen…“: remembers the ROM at `path` as the base ROM of the open project, but
/// only when it has the SHA-1 of the mark; another ROM is refused and the remembered entry
/// stays as it was.
pub fn register_base_rom_impl(state: &AppState, path: &str) -> Result<BaseStatusDto, CmdError> {
    let project = super::open_project_arc(state)?;
    register_base_rom_at(state, &project.root, Path::new(path))?;
    Ok(base_of(state, &project.root).dto(&project))
}

/// Remembers the ROM at `path` as the base ROM of the project (or project folder) at `root`,
/// as [`register_base_rom_impl`] does: only with the SHA-1 of its mark, never an original.
/// Also for a folder that does not open yet (a cloned text project before it is completed).
pub(crate) fn register_base_rom_at(
    state: &AppState,
    root: &Path,
    path: &Path,
) -> Result<(), CmdError> {
    let mark = match start::mark_state(root) {
        MarkState::Present(mark) => mark,
        MarkState::Missing => return Err(no_mark().into()),
        other => return Err(other.refusal().unwrap_or_default().into()),
    };
    if state.base_roms.is_original(path) {
        return Err(format!(
            "Originale sind gesperrt, nur Kopien unter _testkopien: {}",
            path.display()
        )
        .into());
    }
    let info = start::inspect_rom(path)?;
    if !info.sha1.eq_ignore_ascii_case(&mark.sha1) {
        return Err(other_rom(&info.sha1, &mark).into());
    }
    state.base_roms.register(path, &info)?;
    Ok(())
}

/// Builds the open project to `build/<Name>.nds` (as `build_rom`) and writes the patch
/// against its base ROM to `build/<Name>.xdelta`. Without a found base ROM it is
/// `base_missing` with the status's reason (or [`no_mark`]) and nothing is built. The user's
/// patch (the Tauri command): the sync before the build takes the agents' turn.
pub fn create_patch_impl(state: &AppState) -> Result<PatchDto, CmdError> {
    create_patch_checked(state, true, &|_| Ok(()))
}

/// [`create_patch_impl`] with `check` asked about the targets once the base ROM is found,
/// before anything is built (the policy of the dispatch caller). `turn`: the sync before the
/// build takes the agents' turn (the user's patch; an agent's call holds it already).
pub(crate) fn create_patch_checked(
    state: &AppState,
    turn: bool,
    check: &dyn Fn(&PatchTargets) -> Result<(), CmdError>,
) -> Result<PatchDto, CmdError> {
    let project = super::open_project_arc(state)?;
    let (mark, hit) = found_base(state, &project.root)?;
    let targets = patch::default_targets(&project);
    check(&targets)?;
    let rom = targets.rom.to_str().ok_or_else(|| {
        CmdError::from(format!(
            "Pfad ist kein gültiger Text: {}",
            targets.rom.display()
        ))
    })?;
    super::sync_before_build(state, turn)?;
    let build = super::build_synced(state, rom, false)?;
    // The build takes the session as it is then: another project opened meanwhile was built.
    if !Arc::ptr_eq(&super::open_project_arc(state)?, &project) {
        return Err(
            "Das Projekt wurde während des Exports gewechselt; kein Patch erstellt.".into(),
        );
    }
    let base = BaseRom::open(&hit.path, Some(&mark.sha1))?;
    let report = patch::create_patch(
        &base,
        &targets.rom,
        &targets.patch,
        &patch::meta(&project, &mark),
    )?;
    Ok(PatchDto { report, build })
}

/// Where `create_patch` writes in the open project (`build/<Name>.nds`, `.xdelta`).
#[cfg(test)]
pub(crate) fn patch_targets(state: &AppState) -> Result<PatchTargets, CmdError> {
    Ok(patch::default_targets(&*super::open_project_arc(state)?))
}

/// The `.nds` DSPRE unpacked the open project from, canonical, unless it is an original.
#[cfg(test)]
pub(crate) fn suggested_base(state: &AppState) -> Option<PathBuf> {
    suggestion(state, &super::open_project_arc(state).ok()?.root)
}

/// [`suggested_base`], if `nds` (relative to the project folder) names it.
pub(crate) fn suggested_base_named(state: &AppState, nds: &str) -> Option<PathBuf> {
    let project = super::open_project_arc(state).ok()?;
    let suggested = suggestion(state, &project.root)?;
    let given = project.root.join(nds).canonicalize().ok()?;
    (given == suggested).then_some(suggested)
}

#[tauri::command]
pub async fn base_status(app: tauri::AppHandle) -> Result<BaseStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || base_status_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn check_base(app: tauri::AppHandle, nds: String) -> Result<BaseCheck, CmdError> {
    tauri::async_runtime::spawn_blocking(move || check_base_impl(&app.state::<AppState>(), &nds))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn assign_base(app: tauri::AppHandle, nds: String) -> Result<BaseStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || assign_base_impl(&app.state::<AppState>(), &nds))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn register_base_rom(
    app: tauri::AppHandle,
    path: String,
) -> Result<BaseStatusDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        register_base_rom_impl(&app.state::<AppState>(), &path)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn create_patch(app: tauri::AppHandle) -> Result<PatchDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || create_patch_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{build_rom_impl, open_project_impl};
    use crate::mcp::settings::{GrantLevel, ProjectKey};
    use g4_kernel::base::{base_label, missing_text, other_rom};
    use g4_kernel::start::read_base_mark;
    use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom, with_game_code};
    use serde_json::json;

    /// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds` (the
    /// synthetic ROM), the project open in a fresh state.
    struct Dspre {
        tmp: tempfile::TempDir,
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
            tmp,
            root,
            nds,
            state,
        }
    }

    fn text(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    fn canonical(path: &str) -> PathBuf {
        std::fs::canonicalize(path).unwrap()
    }

    fn put_mark(root: &Path, text: &str) {
        std::fs::create_dir_all(root.join(".g4")).unwrap();
        std::fs::write(root.join(".g4/base.json"), text).unwrap();
    }

    #[test]
    fn status_without_a_mark_suggests_the_sibling_nds() {
        let d = dspre();
        let status = base_status_impl(&d.state).unwrap();
        let BaseStatusDto::Unassigned {
            suggestion: Some(suggestion),
        } = &status
        else {
            panic!("{status:?}");
        };
        assert_eq!(canonical(suggestion), canonical(text(&d.nds)));
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["state"], "unassigned", "{json}");
        assert_eq!(json["suggestion"], json!(suggestion));

        // Without the sibling there is nothing to suggest.
        std::fs::remove_file(&d.nds).unwrap();
        assert_eq!(
            serde_json::to_value(base_status_impl(&d.state).unwrap()).unwrap(),
            json!({ "state": "unassigned", "suggestion": null })
        );
        // No project open: an error, not a status.
        assert!(base_status_impl(&AppState::default()).is_err());
    }

    #[test]
    fn check_refuses_another_game_without_writing() {
        let d = dspre();
        let check = check_base_impl(&d.state, text(&d.nds)).unwrap();
        assert_eq!(check.refused, None);
        assert_eq!(check.info.game_code, "CPUE");
        with_game_code(&d.nds, "IPKE");
        let check = check_base_impl(&d.state, text(&d.nds)).unwrap();
        assert_eq!(
            check.refused.as_deref(),
            Some("Wrong game: the ROM is IPKE, the project CPUE.")
        );
        assert!(check.warnings.is_empty());
        assert!(!d.root.join(".g4").exists());
        assert!(d.state.base_roms.settings().roms.is_empty());
        // Assigning it is refused with the same sentence, and writes nothing either.
        let err = assign_base_impl(&d.state, text(&d.nds)).unwrap_err();
        assert_eq!(
            err.message,
            "Wrong game: the ROM is IPKE, the project CPUE."
        );
        assert!(!d.root.join(".g4").exists());
    }

    #[test]
    fn assign_writes_the_mark_registers_the_rom_and_moves_the_grant() {
        let d = dspre();
        let before = ProjectKey::of(&d.root).unwrap();
        assert_eq!(before.base_id, None);
        let other = ProjectKey {
            path: "/anderes/Projekt".into(),
            base_id: None,
        };
        d.state
            .agents
            .set_grant(before.clone(), GrantLevel::Write)
            .unwrap();
        d.state
            .agents
            .set_grant(other.clone(), GrantLevel::Read)
            .unwrap();

        let status = assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let mark = read_base_mark(&d.root).unwrap();
        assert_eq!(mark.origin, "dspre");
        let BaseStatusDto::Found {
            mark: shown,
            label,
            path,
            source,
            patch_name,
        } = &status
        else {
            panic!("{status:?}");
        };
        assert_eq!(shown, &mark);
        assert_eq!(label, &base_label(&mark));
        assert_eq!(canonical(path), canonical(text(&d.nds)));
        assert_eq!(*source, BaseSource::Known);
        assert_eq!(
            patch_name,
            &patch::patch_name(&crate::commands::open_project_arc(&d.state).unwrap())
        );
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["state"], "found", "{json}");
        assert_eq!(json["source"], "known", "{json}");
        assert_eq!(json["patch_name"], json!(patch_name), "{json}");
        assert_eq!(json["mark"]["sha1"], json!(mark.sha1), "{json}");

        // The ROM is remembered.
        let entry = d.state.base_roms.settings().find(&mark.sha1).cloned();
        assert_eq!(
            entry.map(|e| PathBuf::from(e.path)),
            Some(canonical(text(&d.nds)))
        );
        // The grant moved to the new key; the other project's grant stays.
        let after = ProjectKey::of(&d.root).unwrap();
        assert_eq!(
            after.base_id,
            Some(format!("{}/{}", mark.sha1, mark.created))
        );
        let settings = d.state.agents.settings();
        assert_eq!(settings.grant_for(&after), Some(GrantLevel::Write));
        assert_eq!(settings.grant_for(&before), None);
        assert_eq!(settings.grant_for(&other), Some(GrantLevel::Read));

        // The same ROM again: nothing changes.
        let again = assign_base_impl(&d.state, text(&d.nds)).unwrap();
        assert!(matches!(again, BaseStatusDto::Found { .. }), "{again:?}");
        assert_eq!(read_base_mark(&d.root).unwrap(), mark);
        assert_eq!(
            d.state.agents.settings().grant_for(&after),
            Some(GrantLevel::Write)
        );
    }

    #[test]
    fn register_refuses_another_rom_and_keeps_the_entry() {
        let d = dspre();
        assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let mark = read_base_mark(&d.root).unwrap();
        let entry = d.state.base_roms.settings().find(&mark.sha1).cloned();
        assert!(entry.is_some());

        let elsewhere = tempfile::tempdir().unwrap();
        let wrong = elsewhere.path().join("andere.nds");
        let mut bytes = std::fs::read(&d.nds).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&wrong, &bytes).unwrap();
        let found = g4_kernel::start::inspect_rom(&wrong).unwrap().sha1;
        let err = register_base_rom_impl(&d.state, text(&wrong)).unwrap_err();
        assert_eq!(err.message, other_rom(&found, &mark));
        assert_eq!(
            d.state.base_roms.settings().find(&mark.sha1).cloned(),
            entry
        );

        // The same ROM moved elsewhere is taken, and the entry follows it.
        let moved = elsewhere.path().join("verschoben.nds");
        std::fs::rename(&d.nds, &moved).unwrap();
        let status = register_base_rom_impl(&d.state, text(&moved)).unwrap();
        let BaseStatusDto::Found { path, .. } = &status else {
            panic!("{status:?}");
        };
        assert_eq!(canonical(path), canonical(text(&moved)));
        assert_eq!(
            d.state
                .base_roms
                .settings()
                .find(&mark.sha1)
                .map(|e| PathBuf::from(&e.path)),
            Some(canonical(text(&moved)))
        );
    }

    #[test]
    fn register_without_a_mark_names_it() {
        let d = dspre();
        let err = register_base_rom_impl(&d.state, text(&d.nds)).unwrap_err();
        assert_eq!(err.message, no_mark());
        assert!(d.state.base_roms.settings().roms.is_empty());
    }

    #[test]
    fn status_names_a_missing_base_with_label_and_sha1() {
        let d = dspre();
        assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let mark = read_base_mark(&d.root).unwrap();
        // Moved away: neither the remembered entry nor the sibling finds it.
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::rename(&d.nds, elsewhere.path().join("weg.nds")).unwrap();
        let status = base_status_impl(&d.state).unwrap();
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(
            json,
            json!({
                "state": "missing",
                "mark": serde_json::to_value(&mark).unwrap(),
                "label": base_label(&mark),
                "reason": missing_text(&mark),
            })
        );
        let reason = json["reason"].as_str().unwrap();
        assert!(reason.contains(&mark.sha1[..8]), "{reason}");
        assert!(reason.contains(&base_label(&mark)), "{reason}");

        // Headless `--base-rom` naming another ROM: missing, with the reason why.
        let wrong = elsewhere.path().join("andere.nds");
        let mut bytes = std::fs::read(elsewhere.path().join("weg.nds")).unwrap();
        bytes[0x7FFF] ^= 0xFF;
        std::fs::write(&wrong, &bytes).unwrap();
        let found = g4_kernel::start::inspect_rom(&wrong).unwrap().sha1;
        d.state.base_roms.set_explicit(Some(wrong));
        let status = base_status_impl(&d.state).unwrap();
        let BaseStatusDto::Missing { reason, .. } = &status else {
            panic!("{status:?}");
        };
        assert_eq!(reason, &other_rom(&found, &mark));
        let err = create_patch_impl(&d.state).unwrap_err();
        assert_eq!((err.code, &err.message), ("base_missing", reason));
        let _ = &d.tmp;
    }

    #[test]
    fn status_of_a_broken_mark_is_unreadable_and_build_still_runs() {
        let d = dspre();
        let out = d.root.join("build/out.nds");
        // The synthetic project has no real ds-rom config: the build fails the same way with
        // and without a mark, never because of the base.
        let plain = build_rom_impl(&d.state, text(&out), false).unwrap_err();

        put_mark(&d.root, "{kaputt");
        let status = base_status_impl(&d.state).unwrap();
        let BaseStatusDto::Unreadable { reason } = &status else {
            panic!("{status:?}");
        };
        assert!(
            reason.starts_with("The base mark .g4/base.json cannot be read ("),
            "{reason}"
        );
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "unreadable", "reason": reason })
        );
        let broken = build_rom_impl(&d.state, text(&out), false).unwrap_err();
        assert_eq!((broken.code, &broken.message), (plain.code, &plain.message));
        // No patch either, with the same reason.
        let err = create_patch_impl(&d.state).unwrap_err();
        assert_eq!((err.code, &err.message), ("base_missing", reason));
        // Nothing overwrote the mark.
        assert_eq!(
            std::fs::read(d.root.join(".g4/base.json")).unwrap(),
            b"{kaputt"
        );

        put_mark(&d.root, r#"{"version": 3, "origin": "dspre"}"#);
        let status = base_status_impl(&d.state).unwrap();
        let BaseStatusDto::Unreadable { reason } = &status else {
            panic!("{status:?}");
        };
        assert_eq!(
            reason,
            "The base mark comes from a newer version of Gen4 Studio (version 3)."
        );
        let newer = build_rom_impl(&d.state, text(&out), false).unwrap_err();
        assert_eq!((newer.code, &newer.message), (plain.code, &plain.message));
    }

    #[test]
    fn create_patch_without_a_base_answers_base_missing() {
        let d = dspre();
        let err = create_patch_impl(&d.state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", no_mark().as_str())
        );
        assert!(!d.root.join("build").exists());

        // Assigned, then moved away: the reason names the missing ROM, nothing is built.
        assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let mark = read_base_mark(&d.root).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::rename(&d.nds, elsewhere.path().join("weg.nds")).unwrap();
        let err = create_patch_impl(&d.state).unwrap_err();
        assert_eq!(
            (err.code, err.message),
            ("base_missing", missing_text(&mark))
        );
        assert!(!d.root.join("build").exists());
    }

    #[test]
    fn create_patch_with_a_base_builds_first() {
        // With the base found, the build runs (and fails on the synthetic project as
        // build_rom does): not base_missing.
        let d = dspre();
        assign_base_impl(&d.state, text(&d.nds)).unwrap();
        let err = create_patch_impl(&d.state).unwrap_err();
        assert_ne!(err.code, "base_missing", "{}", err.message);
        let targets = patch_targets(&d.state).unwrap();
        let plain = build_rom_impl(&d.state, text(&targets.rom), false).unwrap_err();
        assert_eq!((err.code, &err.message), (plain.code, &plain.message));
        assert!(!targets.patch.exists());
    }

    #[test]
    fn patch_dto_flattens_the_report_beside_the_build() {
        let dto = PatchDto {
            report: PatchReport {
                patch: "/p/build/X-CPUE.xdelta".into(),
                notes: "/p/build/X-CPUE.xdelta.txt".into(),
                rom: "/p/build/X-CPUE.nds".into(),
                size: 812,
                base_sha1: "a".repeat(40),
                target_sha1: "b".repeat(40),
                encode_ms: 120,
                verify_ms: 30,
            },
            build: BuildDto {
                saved: 0,
                packed: vec![("maps".into(), 3)],
                rom: "/p/build/X-CPUE.nds".into(),
                compiled: vec![],
                encoded: vec![],
                problems: vec![],
                steps: vec![],
                ms: 42,
            },
        };
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({
                "patch": "/p/build/X-CPUE.xdelta",
                "notes": "/p/build/X-CPUE.xdelta.txt",
                "rom": "/p/build/X-CPUE.nds",
                "size": 812,
                "base_sha1": "a".repeat(40),
                "target_sha1": "b".repeat(40),
                "encode_ms": 120,
                "verify_ms": 30,
                "build": {
                    "saved": 0,
                    "packed": [["maps", 3]],
                    "rom": "/p/build/X-CPUE.nds",
                    "compiled": [],
                    "encoded": [],
                    "problems": [],
                    "steps": [],
                    "ms": 42,
                },
            })
        );
    }

    #[test]
    fn suggestion_skips_an_original() {
        let d = dspre();
        assert_eq!(suggested_base(&d.state), Some(canonical(text(&d.nds))));
        // The sibling lies under G4_ROMS_DIR, outside `_testkopien`: not suggested.
        d.state
            .base_roms
            .configure_with(None, None, Some(d.tmp.path().to_path_buf()));
        assert_eq!(suggested_base(&d.state), None);
        assert_eq!(
            serde_json::to_value(base_status_impl(&d.state).unwrap()).unwrap(),
            json!({ "state": "unassigned", "suggestion": null })
        );
    }
}
