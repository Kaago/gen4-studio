//! Starting a project from a ROM: look at the file, pick a place, create the project.
//!
//! None of these commands opens the project; the app calls `open_project` with the `root` of
//! [`CreatedDto`] when the user asks for it.

use std::path::{Path, PathBuf};

use g4_kernel::start::{self, CreateOptions, DestCheck, Phase, Progress, RomInfo, Template};
use serde::Serialize;
use tauri::{Manager, ipc::Channel};

use crate::errors::CmdError;
use crate::state::AppState;

/// Folder name under Documents where the app proposes to create new projects.
const PROJECTS_DIR: &str = "Gen4 Studio";

/// Where new projects go by default, and whether the folder name gets the `_DSPRE_contents`
/// ending (the app does, so DSPRE finds the project; the bridge does not, it refuses the
/// ending).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ProjectParentDto {
    pub parent: String,
    pub dspre_suffix: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PhaseTimeDto {
    pub phase: Phase,
    pub ms: u64,
}

/// What [`create_project_impl`] made: the project is on disk but not open.
#[derive(Debug, Serialize)]
pub struct CreatedDto {
    pub root: String,
    pub game_code: String,
    pub template: Template,
    pub sha1: String,
    pub phases: Vec<PhaseTimeDto>,
}

pub fn inspect_rom_impl(path: &str) -> Result<RomInfo, CmdError> {
    Ok(start::inspect_rom(Path::new(path))?)
}

/// `<documents>/Gen4 Studio`, with the `_DSPRE_contents` ending on.
pub fn default_project_parent_impl(documents: Option<PathBuf>) -> ProjectParentDto {
    ProjectParentDto {
        parent: documents
            .unwrap_or_default()
            .join(PROJECTS_DIR)
            .display()
            .to_string(),
        dspre_suffix: true,
    }
}

pub fn check_project_dest_impl(dest: &str, rom_size: u64) -> DestCheck {
    start::check_destination(Path::new(dest), rom_size)
}

/// Creates the project (with `text_project` set up as text project after the check, phase
/// `text_project`), then remembers its ROM as a base ROM (`base-roms.json`); a ROM that
/// cannot be remembered does not fail the project, it is searched for again later.
pub fn create_project_impl(
    state: &AppState,
    rom: &str,
    dest: &str,
    template: Template,
    text_project: bool,
    progress: &mut dyn FnMut(Progress),
) -> Result<CreatedDto, CmdError> {
    let opts = CreateOptions {
        template,
        text_project,
        git: false,
    };
    let created =
        start::create_project_with_options(Path::new(rom), Path::new(dest), &opts, progress)?;
    if let Err(e) = state.base_roms.register(Path::new(rom), &created.info) {
        eprintln!("Basis-ROM nicht gemerkt: {e}");
    }
    Ok(CreatedDto {
        root: created.root.display().to_string(),
        game_code: created.info.game_code,
        template: created.template,
        sha1: created.info.sha1,
        phases: created
            .phases
            .into_iter()
            .map(|(phase, ms)| PhaseTimeDto {
                phase,
                ms: u64::try_from(ms).unwrap_or(u64::MAX),
            })
            .collect(),
    })
}

/// Hashes the whole file (128 MB and more), so it runs on a blocking worker.
#[tauri::command]
pub async fn inspect_rom(path: String) -> Result<RomInfo, CmdError> {
    tauri::async_runtime::spawn_blocking(move || inspect_rom_impl(&path))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub fn default_project_parent(app: tauri::AppHandle) -> ProjectParentDto {
    default_project_parent_impl(app.path().document_dir().ok())
}

#[tauri::command]
pub fn check_project_dest(dest: String, rom_size: u64) -> DestCheck {
    check_project_dest_impl(&dest, rom_size)
}

/// Unpacks a ROM (seconds; set up as text project with `text_project`, default off, 10 to
/// 120 s more) on a blocking worker and reports each phase over `progress`.
#[tauri::command]
pub async fn create_project(
    app: tauri::AppHandle,
    rom: String,
    dest: String,
    template: Template,
    text_project: Option<bool>,
    progress: Channel<Progress>,
) -> Result<CreatedDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let text_project = text_project.unwrap_or(false);
        create_project_impl(
            &app.state::<AppState>(),
            &rom,
            &dest,
            template,
            text_project,
            &mut |p| {
                // A closed channel only means nobody watches the progress any more.
                let _ = progress.send(p);
            },
        )
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_kernel::test_support::{synthetic_rom, with_game_code};

    #[test]
    fn inspect_reads_the_header() {
        let (_tmp, rom) = synthetic_rom();
        let info = inspect_rom_impl(rom.to_str().unwrap()).unwrap();
        assert_eq!(info.game_code, "CPUE");
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["support"]["kind"], "untested");
        assert!(inspect_rom_impl("/no/such/file.nds").is_err());
    }

    #[test]
    fn the_default_parent_is_gen4_studio_in_documents_with_the_suffix() {
        let got = default_project_parent_impl(Some(PathBuf::from("/home/u/Documents")));
        assert_eq!(got.parent, "/home/u/Documents/Gen4 Studio");
        assert!(got.dspre_suffix);
    }

    #[test]
    fn check_dest_answers_with_the_kernel_check() {
        let tmp = tempfile::tempdir().unwrap();
        let got = check_project_dest_impl(tmp.path().join("neu").to_str().unwrap(), 1000);
        assert!(got.ok);
        assert_eq!(got.needed_bytes, 2500);
        let json = serde_json::to_value(got).unwrap();
        assert_eq!(json["state"], "missing");
    }

    #[test]
    fn create_fails_on_a_header_only_rom_and_leaves_nothing() {
        let (tmp, rom) = synthetic_rom();
        let dest = tmp.path().join("neu");
        let err = create_project_impl(
            &AppState::default(),
            rom.to_str().unwrap(),
            dest.to_str().unwrap(),
            Template::Original,
            false,
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[test]
    fn create_refuses_diamond_with_the_kernel_text() {
        let (tmp, rom) = synthetic_rom();
        with_game_code(&rom, "ADAD");
        let err = create_project_impl(
            &AppState::default(),
            rom.to_str().unwrap(),
            tmp.path().join("neu").to_str().unwrap(),
            Template::DataOnly,
            true,
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(err.message, start::DP_LOCKED);
    }
}
