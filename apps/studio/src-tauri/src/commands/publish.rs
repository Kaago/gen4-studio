//! Veröffentlichen (4.5): the releases list of the open project (`releases`), a release built
//! and published at a chosen place (`publish_patch`) and a release file shown in the file
//! manager (`reveal_path`).
//!
//! A release is the saved project, built over the build path of `create_patch` (the sync of a
//! text project first, then `build/<patch_name>.nds`), then `patch::publish_built`: the
//! self-checked patch, its notes, optionally `README.txt`, and the entry in
//! `.g4/releases.json`. Every refusal that needs no build comes before it
//! ([`check_publish_patch`]), so a taken version or an existing file builds nothing.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::base::{BaseRom, Located};
use g4_kernel::error::KernelError;
use g4_kernel::patch::{
    self, BAD_FILE, BAD_VERSION, EXISTS, NO_CHANGES, NOT_A_FOLDER, PublishOptions, PublishPhase,
    PublishProgress, PublishReport, PublishTargets, RELEASES_BROKEN, Release, VERSION_TAKEN,
};
use g4_kernel::project::Project;
use g4_kernel::start::BaseMark;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri::ipc::Channel;

use super::BuildDto;
use super::base::{base_missing, found_base};
use super::policy::Policy;
use crate::errors::CmdError;
use crate::state::AppState;

/// The refusal of `publish_patch` with unsaved changes, before anything is asked or built.
pub const PUBLISH_UNSAVED: &str = "Erst speichern: veröffentlicht wird der gespeicherte Stand.";

/// `reveal_path` of a path that is neither a release file of the list nor its folder.
pub const NOT_A_RELEASE_FILE: &str = "Diesen Pfad kennt die Liste der Veröffentlichungen nicht.";

/// The releases of the open project, as Technik › Veröffentlichen lists them. `stem` is the
/// project part of a release's file name (`patch::project_stem`), `next_version` the proposal
/// for the next release (`patch::next_version`), `dir` the default place (`<root>/build`,
/// absolute), `releases` the list, oldest first.
#[derive(Debug, Serialize)]
pub struct ReleasesDto {
    pub stem: String,
    pub next_version: String,
    pub dir: String,
    pub releases: Vec<Release>,
}

/// What `publish_patch` did: the kernel's report of the release (flattened: `release`,
/// `patch`, `files`, `releases`, `phase_ms`, `ms`) and the report of the build it made (its
/// steps and problems are the session's last build, as with `create_patch`).
#[derive(Debug, Serialize)]
pub struct PublishDto {
    #[serde(flatten)]
    pub report: PublishReport,
    pub build: BuildDto,
}

/// The arguments of `publish_patch`, keyed as `dispatch`, the Tauri command and the MCP tool
/// take them (all single words, so camelCase and snake_case agree).
#[derive(Debug, Deserialize)]
pub(crate) struct PublishArgs {
    pub version: String,
    pub notes: Option<String>,
    pub dir: Option<String>,
    pub file: Option<String>,
    pub readme: Option<bool>,
    pub overwrite: Option<bool>,
}

impl PublishArgs {
    /// The kernel's options: no notes are empty notes, `dir` relative to the project root
    /// (as the kernel joins it), `readme` and `overwrite` off unless set.
    pub(crate) fn options(self) -> PublishOptions {
        PublishOptions {
            version: self.version,
            notes: self.notes.unwrap_or_default(),
            dir: self.dir.map(PathBuf::from),
            file: self.file,
            readme: self.readme.unwrap_or(false),
            overwrite: self.overwrite.unwrap_or(false),
        }
    }
}

/// The kernel's refusals of a release with a code of their own: `releases_broken`
/// ([`RELEASES_BROKEN`] with path and reason), `bad_version` ([`BAD_VERSION`]),
/// `version_taken` ([`VERSION_TAKEN`]), `bad_file` ([`BAD_FILE`]), `bad_dir` (the place is a
/// file or lies below one, [`NOT_A_FOLDER`] with the path), `exists` (`<Datei> ` + [`EXISTS`])
/// and `no_changes` ([`NO_CHANGES`]); `base_missing` for the kernel's [`patch::NO_MARK`]. Any
/// other error as it is (the failed self-check is an `error` with its sentence).
fn publish_error(e: KernelError) -> CmdError {
    let KernelError::Refused(why) = &e else {
        return e.into();
    };
    let code = if why.starts_with(RELEASES_BROKEN) {
        "releases_broken"
    } else if why == BAD_VERSION {
        "bad_version"
    } else if why == VERSION_TAKEN {
        "version_taken"
    } else if why == BAD_FILE {
        "bad_file"
    } else if why.starts_with(NOT_A_FOLDER) {
        "bad_dir"
    } else if why.ends_with(EXISTS) {
        "exists"
    } else if why == NO_CHANGES {
        "no_changes"
    } else if why == patch::NO_MARK {
        return base_missing(why.clone());
    } else {
        return e.into();
    };
    CmdError::refused(code, why.clone())
}

/// The releases list of the open project with the proposal for the next version; a broken or
/// newer list is `releases_broken` with the kernel's sentence. Writes nothing.
pub fn releases_impl(state: &AppState) -> Result<ReleasesDto, CmdError> {
    let project = super::open_project_arc(state)?;
    let releases = patch::read_releases(&project.root).map_err(publish_error)?;
    Ok(ReleasesDto {
        stem: patch::project_stem(&project),
        next_version: patch::next_version(&releases),
        dir: project.root.join("build").display().to_string(),
        releases: releases.releases,
    })
}

/// What a checked release needs: the project, its mark and found base ROM, and the targets.
struct Checked {
    project: Arc<Project>,
    mark: BaseMark,
    hit: Located,
    targets: PublishTargets,
}

/// Asks `check` of `policy` about every file a release writes: the patch, its notes, the
/// README when it is written, and the built ROM.
fn place(
    policy: &dyn Policy,
    targets: &PublishTargets,
    check: fn(&dyn Policy, &str) -> Result<PathBuf, CmdError>,
) -> Result<(), CmdError> {
    let files = [
        Some(&targets.patch),
        Some(&targets.notes),
        targets.readme.as_ref(),
        Some(&targets.rom),
    ];
    for path in files.into_iter().flatten() {
        let text = path.to_str().ok_or_else(|| {
            CmdError::from(format!("Pfad ist kein gültiger Text: {}", path.display()))
        })?;
        check(policy, text)?;
    }
    Ok(())
}

/// The refusals of a release that need no build, in this order: a session that refuses
/// writes (`busy`), unsaved changes (`unsaved`, [`PUBLISH_UNSAVED`]), the list and the version
/// (`releases_broken`, `bad_version`, `version_taken`), the file name (`bad_file`), then the
/// place under `policy` (`Policy::check_target`, which makes nothing) before anything at it is
/// looked at, so a place outside the allowed folders is refused alike whether it exists or
/// not; then the files there (`bad_dir`, `exists` without `overwrite`) and last the base ROM
/// (`base_missing` with the reason of the base status; searching it may hash a ROM). `policy`
/// is the dispatch caller's (`None`: the user's own release in the app). Writes nothing.
fn checked(
    state: &AppState,
    opts: &PublishOptions,
    policy: Option<&dyn Policy>,
) -> Result<Checked, CmdError> {
    let project = {
        let mut guard = super::lock(state)?;
        let session = super::session(&mut guard)?;
        session.writable()?;
        if !session.store.dirty().is_empty() {
            return Err(CmdError::refused("unsaved", PUBLISH_UNSAVED));
        }
        Arc::clone(&session.project)
    };
    // `patch::check_publish`, with the policy before the place is looked at.
    let releases = patch::read_releases(&project.root).map_err(publish_error)?;
    patch::check_version(&opts.version, &releases)
        .map_err(|why| publish_error(KernelError::Refused(why)))?;
    let targets = patch::targets_for(&project, opts).map_err(publish_error)?;
    if let Some(policy) = policy {
        place(policy, &targets, |policy, path| policy.check_target(path))?;
    }
    patch::check_targets(&targets, opts.overwrite).map_err(publish_error)?;
    let (mark, hit) = found_base(state, &project.root)?;
    Ok(Checked {
        project,
        mark,
        hit,
        targets,
    })
}

/// The refusals before the question and before the build (`unsaved`, `base_missing`,
/// `bad_version`, `version_taken`, `bad_file`, `bad_dir`, `exists`, `releases_broken`, `busy`
/// while the project is set up as text project or its state is saved in Git, and the
/// refusal of `policy` for a place it does not allow, before the place is looked at); writes
/// nothing. The targets the release would write.
pub(crate) fn check_publish_patch(
    state: &AppState,
    opts: &PublishOptions,
    policy: Option<&dyn Policy>,
) -> Result<PublishTargets, CmdError> {
    Ok(checked(state, opts, policy)?.targets)
}

/// The build of [`publish_patch_with`]: the state and the built ROM's path in, the build's
/// report out.
pub(crate) type Build<'a> = dyn Fn(&AppState, &str) -> Result<BuildDto, CmdError> + 'a;

/// Publishes a release of the open project: refused as [`check_publish_patch`] says, then
/// `policy` (the dispatch caller's; `None` in the app) has the last word on every target with
/// `Policy::check_output` (which makes a missing `build/`), then the build
/// (phase `Build`: the sync of a text project, the session's save and the build to
/// `build/<patch_name>.nds`, as `create_patch` builds), then `patch::publish_built` (phases
/// `Encode`, `Verify`, `Write`, `Record`). `turn`: the sync before the build takes the agents'
/// turn (the user's release; an agent's call holds it already). Answers the release's report
/// with the build's.
pub(crate) fn publish_patch_checked(
    state: &AppState,
    opts: PublishOptions,
    turn: bool,
    policy: Option<&dyn Policy>,
    progress: &mut dyn FnMut(PublishProgress),
) -> Result<PublishDto, CmdError> {
    publish_patch_with(state, opts, turn, policy, progress, &|state, rom| {
        super::build_synced(state, rom, false)
    })
}

/// [`publish_patch_checked`] with the build replaced, for tests (the synthetic project does
/// not build to a ROM).
pub(crate) fn publish_patch_with(
    state: &AppState,
    opts: PublishOptions,
    turn: bool,
    policy: Option<&dyn Policy>,
    progress: &mut dyn FnMut(PublishProgress),
    build: &Build<'_>,
) -> Result<PublishDto, CmdError> {
    let started = Instant::now();
    let Checked {
        project,
        mark,
        hit,
        targets,
    } = checked(state, &opts, policy)?;
    if let Some(policy) = policy {
        place(policy, &targets, |policy, path| policy.check_output(path))?;
    }
    let rom = targets.rom.to_str().ok_or_else(|| {
        CmdError::from(format!(
            "Pfad ist kein gültiger Text: {}",
            targets.rom.display()
        ))
    })?;

    progress(PublishProgress {
        phase: PublishPhase::Build,
    });
    let build_started = Instant::now();
    super::sync_before_build(state, turn)?;
    let built = build(state, rom)?;
    let build_ms = elapsed_ms(build_started);
    // The build takes the session as it is then: another project opened meanwhile was built.
    if !Arc::ptr_eq(&super::open_project_arc(state)?, &project) {
        return Err(
            "Das Projekt wurde während des Exports gewechselt; nichts veröffentlicht.".into(),
        );
    }

    let base = BaseRom::open(&hit.path, Some(&mark.sha1))?;
    let mut report = patch::publish_built(
        &project,
        &base,
        Path::new(&built.rom),
        &opts,
        &mut *progress,
    )
    .map_err(publish_error)?;
    report.phase_ms.insert(PublishPhase::Build, build_ms);
    report.ms = elapsed_ms(started);
    Ok(PublishDto {
        report,
        build: built,
    })
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The user's release (the Tauri command): see [`publish_patch_checked`]; the sync before the
/// build takes the agents' turn.
pub fn publish_patch_impl(
    state: &AppState,
    opts: PublishOptions,
    progress: &mut dyn FnMut(PublishProgress),
) -> Result<PublishDto, CmdError> {
    publish_patch_checked(state, opts, true, None, progress)
}

/// Shows `path` in the file manager (macOS `open -R`, Windows `explorer /select,`, Linux
/// `xdg-open` on its folder). `path` (relative ones from the project root) must exist and be
/// a release file of the list (its entry relative to the project root, or absolute) or that
/// file's folder; else [`NOT_A_RELEASE_FILE`]. A broken list is `releases_broken`.
pub fn reveal_path_impl(state: &AppState, path: &str) -> Result<(), CmdError> {
    let project = super::open_project_arc(state)?;
    let releases = patch::read_releases(&project.root).map_err(publish_error)?;
    let given = project
        .root
        .join(path)
        .canonicalize()
        .map_err(|e| CmdError::from(format!("Pfad nicht gefunden: {path}: {e}")))?;
    let known = releases.releases.iter().any(|release| {
        let file = project.root.join(&release.file);
        let folder = file.parent().map(Path::to_path_buf);
        [Some(file), folder]
            .into_iter()
            .flatten()
            .filter_map(|candidate| candidate.canonicalize().ok())
            .any(|candidate| candidate == given)
    });
    if !known {
        return Err(NOT_A_RELEASE_FILE.into());
    }
    reveal(&given)
}

/// Asks the system's file manager to show `path` (an existing file or folder): macOS
/// `open -R` marks it in the Finder; a file manager that could not be started or that
/// reports a failure is an error.
#[cfg(all(not(test), target_os = "macos"))]
fn reveal(path: &Path) -> Result<(), CmdError> {
    ran(std::process::Command::new("open").arg("-R").arg(path), path)
}

/// Windows: `explorer /select,"<path>"` (one argument, quoted only around the path). Its
/// exit code says nothing (it is 1 also when it worked), so only a start that failed counts.
#[cfg(all(not(test), target_os = "windows"))]
fn reveal(path: &Path) -> Result<(), CmdError> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .status()
        .map(|_| ())
        .map_err(not_started)
}

/// Elsewhere: `xdg-open` on the folder (it opens a folder, it cannot mark a file in it).
#[cfg(all(not(test), not(any(target_os = "macos", target_os = "windows"))))]
fn reveal(path: &Path) -> Result<(), CmdError> {
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    ran(std::process::Command::new("xdg-open").arg(folder), path)
}

/// Runs `command` to its end: a failed start or a failing exit status is an error naming
/// `path`.
#[cfg(all(not(test), not(target_os = "windows")))]
fn ran(command: &mut std::process::Command, path: &Path) -> Result<(), CmdError> {
    let status = command.status().map_err(not_started)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Dateimanager meldet {status}: {}", path.display()).into())
    }
}

#[cfg(not(test))]
fn not_started(e: std::io::Error) -> CmdError {
    CmdError::from(format!("Dateimanager nicht startbar: {e}"))
}

/// In tests the file manager is never started.
#[cfg(test)]
fn reveal(_path: &Path) -> Result<(), CmdError> {
    Ok(())
}

#[tauri::command]
pub async fn releases(app: tauri::AppHandle) -> Result<ReleasesDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || releases_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// A release, with the flat keys `api.publish.ts` sends (the keys of `dispatch`); reports
/// each phase over `progress`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn publish_patch(
    app: tauri::AppHandle,
    version: String,
    notes: Option<String>,
    dir: Option<String>,
    file: Option<String>,
    readme: Option<bool>,
    overwrite: Option<bool>,
    progress: Channel<PublishProgress>,
) -> Result<PublishDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let opts = PublishArgs {
            version,
            notes,
            dir,
            file,
            readme,
            overwrite,
        }
        .options();
        publish_patch_impl(&app.state::<AppState>(), opts, &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub async fn reveal_path(app: tauri::AppHandle, path: String) -> Result<(), CmdError> {
    tauri::async_runtime::spawn_blocking(move || reveal_path_impl(&app.state::<AppState>(), &path))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        DataKind, DataPatch, assign_base_impl, data_edit_impl, open_project_impl, save_impl,
        set_matrix_cell_impl,
    };
    use g4_kernel::patch::{FIRST_VERSION, PublishedKind, RELEASES_FILE};
    use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};

    /// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds` (the
    /// synthetic ROM) beside it, the project open in a fresh state.
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
        // The root as the session holds it, so paths compare as the commands build them.
        let root = super::super::open_project_arc(&state).unwrap().root.clone();
        Dspre {
            _tmp: tmp,
            root,
            nds,
            state,
        }
    }

    fn opts(version: &str) -> PublishOptions {
        PublishArgs {
            version: version.into(),
            notes: None,
            dir: None,
            file: None,
            readme: None,
            overwrite: None,
        }
        .options()
    }

    /// A build that writes the base ROM with one byte flipped and 300 bytes appended to its
    /// target, as a real build of a changed project would (the synthetic project does not
    /// build to a ROM).
    fn fake_build(base: PathBuf) -> impl Fn(&AppState, &str) -> Result<BuildDto, CmdError> {
        move |_state, rom| {
            let mut bytes = std::fs::read(&base).unwrap();
            bytes[0x7000] ^= 0x55;
            bytes.extend((0..300u32).map(|i| (i % 13) as u8));
            let out = Path::new(rom);
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            std::fs::write(out, bytes).unwrap();
            Ok(build_dto(rom))
        }
    }

    /// What a build reports that wrote `rom` and nothing else.
    fn build_dto(rom: &str) -> BuildDto {
        BuildDto {
            saved: 0,
            packed: Vec::new(),
            rom: rom.to_owned(),
            compiled: Vec::new(),
            encoded: Vec::new(),
            problems: Vec::new(),
            steps: Vec::new(),
            ms: 0,
        }
    }

    fn release(version: &str, file: &str) -> Release {
        Release {
            version: version.into(),
            date: "2026-10-30T12:00:00Z".into(),
            file: file.into(),
            notes: String::new(),
            base_sha1: "a".repeat(40),
            size: 1,
        }
    }

    fn put_releases(root: &Path, releases: &[Release]) {
        let list = patch::Releases {
            version: patch::RELEASES_VERSION,
            releases: releases.to_vec(),
        };
        std::fs::create_dir_all(root.join(".g4")).unwrap();
        std::fs::write(
            root.join(RELEASES_FILE),
            serde_json::to_string_pretty(&list).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn releases_start_empty_with_the_first_version_and_the_build_dir() {
        let d = dspre();
        let dto = releases_impl(&d.state).unwrap();
        assert_eq!(dto.stem, "X");
        assert_eq!(dto.next_version, FIRST_VERSION);
        assert_eq!(PathBuf::from(&dto.dir), d.root.join("build"));
        assert!(dto.releases.is_empty());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["next_version"], "0.1.0", "{json}");
        // No project open: an error.
        assert!(releases_impl(&AppState::default()).is_err());
    }

    #[test]
    fn releases_report_a_broken_file_as_releases_broken() {
        let d = dspre();
        std::fs::create_dir_all(d.root.join(".g4")).unwrap();
        std::fs::write(d.root.join(RELEASES_FILE), "{kaputt").unwrap();
        let err = releases_impl(&d.state).unwrap_err();
        assert_eq!(err.code, "releases_broken");
        assert!(err.message.starts_with(RELEASES_BROKEN), "{}", err.message);
        // Publishing is refused with it too, and the file stays as it was.
        let err = check_publish_patch(&d.state, &opts("0.1.0"), None).unwrap_err();
        assert_eq!(err.code, "releases_broken");
        assert_eq!(
            std::fs::read(d.root.join(RELEASES_FILE)).unwrap(),
            b"{kaputt"
        );
        // A newer list as well.
        std::fs::write(
            d.root.join(RELEASES_FILE),
            r#"{"version": 2, "releases": []}"#,
        )
        .unwrap();
        let err = releases_impl(&d.state).unwrap_err();
        assert_eq!(err.code, "releases_broken", "{}", err.message);
    }

    #[test]
    fn publish_refuses_unsaved_taken_version_and_existing_file_before_the_build() {
        let d = dspre();
        assert!(!d.root.join("build").exists());
        let build_called = std::cell::Cell::new(false);
        let build = |_: &AppState, _: &str| -> Result<BuildDto, CmdError> {
            build_called.set(true);
            Err("gebaut".into())
        };
        let publish = |opts: PublishOptions| {
            let mut phases = Vec::new();
            let err = publish_patch_with(
                &d.state,
                opts,
                false,
                None,
                &mut |p| phases.push(p.phase),
                &build,
            )
            .unwrap_err();
            assert!(phases.is_empty(), "{phases:?}");
            err
        };

        // Unsaved changes first.
        set_matrix_cell_impl(&d.state, 0, 0, 0, Some(1), None, None).unwrap();
        let err = publish(opts("0.1.0"));
        assert_eq!(
            (err.code, err.message.as_str()),
            ("unsaved", PUBLISH_UNSAVED)
        );
        save_impl(&d.state).unwrap();

        // A taken and an unsafe version.
        put_releases(&d.root, &[release("0.1.0", "build/X-v0.1.0.xdelta")]);
        let err = publish(opts("0.1.0"));
        assert_eq!(
            (err.code, err.message.as_str()),
            ("version_taken", VERSION_TAKEN)
        );
        let err = publish(opts("0.1 beta"));
        assert_eq!(
            (err.code, err.message.as_str()),
            ("bad_version", BAD_VERSION)
        );
        let mut bad = opts("0.2.0");
        bad.file = Some("a/b".into());
        let err = publish(bad);
        assert_eq!((err.code, err.message.as_str()), ("bad_file", BAD_FILE));

        // An existing file at the chosen place, without overwrite.
        std::fs::create_dir_all(d.root.join("out")).unwrap();
        std::fs::write(d.root.join("out/X-v0.2.0.xdelta"), b"alt").unwrap();
        let mut existing = opts("0.2.0");
        existing.dir = Some("out".into());
        let err = publish(existing.clone());
        assert_eq!(err.code, "exists", "{}", err.message);
        assert_eq!(err.message, format!("X-v0.2.0.xdelta {EXISTS}"));
        // A place that is a file.
        let mut file_dir = opts("0.2.0");
        file_dir.dir = Some("out/X-v0.2.0.xdelta".into());
        let err = publish(file_dir);
        assert_eq!(err.code, "bad_dir", "{}", err.message);

        assert!(!build_called.get());
        assert!(!d.root.join("build").exists());
        assert_eq!(
            std::fs::read(d.root.join("out/X-v0.2.0.xdelta")).unwrap(),
            b"alt"
        );
        // Without a base ROM the same options with overwrite end at the base.
        existing.overwrite = true;
        assert_eq!(publish(existing).code, "base_missing");
        assert!(!build_called.get());
    }

    #[test]
    fn publish_without_a_base_answers_base_missing() {
        let d = dspre();
        let err = publish_patch_impl(&d.state, opts("0.1.0"), &mut |_| {}).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("base_missing", patch::NO_MARK)
        );
        assert!(!d.root.join("build").exists());
        assert!(!d.root.join(RELEASES_FILE).exists());
    }

    #[test]
    fn publish_with_a_base_builds_writes_and_records() {
        let d = dspre();
        assign_base_impl(&d.state, d.nds.to_str().unwrap()).unwrap();
        data_edit_impl(
            &d.state,
            DataKind::Trainer,
            1,
            DataPatch::Trainer {
                field: "ai".into(),
                value: 7,
            },
            None,
        )
        .unwrap();
        save_impl(&d.state).unwrap();

        // Through the user's policy: it allows build/ and makes it.
        let policy = super::super::policy::UserPolicy::new(&d.root, &[], None).unwrap();
        let mut phases = Vec::new();
        let mut options = opts("0.1.0");
        options.notes = "Erste Fassung".into();
        let dto = publish_patch_with(
            &d.state,
            options,
            false,
            Some(&policy),
            &mut |p| phases.push(p.phase),
            &fake_build(d.nds.clone()),
        )
        .unwrap();
        let report = &dto.report;
        // The build's own report comes along: its ROM is the one the patch was made from.
        assert_eq!(Path::new(&dto.build.rom), report.patch.rom);
        // Flattened: the release's fields and the build side by side.
        let json = serde_json::to_value(&dto).unwrap();
        for key in [
            "release", "patch", "files", "releases", "phase_ms", "ms", "build",
        ] {
            assert!(json.get(key).is_some(), "{key} in {json}");
        }
        assert_eq!(json["build"]["rom"], json["patch"]["rom"]);
        assert_eq!(report.files.len(), 2, "{:?}", report.files);
        assert_eq!(report.files[0].kind, PublishedKind::Patch);
        assert_eq!(report.files[1].kind, PublishedKind::Notes);
        assert_eq!(report.files[0].path, d.root.join("build/X-v0.1.0.xdelta"));
        assert!(report.files[0].path.is_file());
        assert_eq!(report.releases.releases.len(), 1);
        assert_eq!(report.release.file, "build/X-v0.1.0.xdelta");
        assert_eq!(report.release.notes, "Erste Fassung");
        assert_eq!(
            phases,
            [
                PublishPhase::Build,
                PublishPhase::Encode,
                PublishPhase::Verify,
                PublishPhase::Write,
                PublishPhase::Record
            ]
        );
        assert!(report.phase_ms.contains_key(&PublishPhase::Build));
        assert_eq!(
            report.patch.rom,
            patch::default_targets(&Project::open(&d.root).unwrap()).rom
        );

        let dto = releases_impl(&d.state).unwrap();
        assert_eq!(dto.next_version, "0.1.1");
        assert_eq!(dto.releases.len(), 1);
        // The same version again: taken, before anything is built.
        let err = publish_patch_with(
            &d.state,
            opts("0.1.0"),
            false,
            None,
            &mut |_| {},
            &|_, _| panic!("built"),
        )
        .unwrap_err();
        assert_eq!(err.code, "version_taken");
        assert_eq!(releases_impl(&d.state).unwrap().releases.len(), 1);
    }

    #[test]
    fn a_build_equal_to_the_base_is_no_changes() {
        let d = dspre();
        assign_base_impl(&d.state, d.nds.to_str().unwrap()).unwrap();
        let base = d.nds.clone();
        let err = publish_patch_with(
            &d.state,
            opts("0.1.0"),
            false,
            None,
            &mut |_| {},
            &move |_, rom| {
                std::fs::create_dir_all(Path::new(rom).parent().unwrap()).unwrap();
                std::fs::copy(&base, rom).unwrap();
                Ok(build_dto(rom))
            },
        )
        .unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("no_changes", NO_CHANGES));
        assert!(!d.root.join(RELEASES_FILE).exists());
        assert!(!d.root.join("build/X-v0.1.0.xdelta").exists());
    }

    #[test]
    fn reveal_path_refuses_a_path_outside_the_releases() {
        let d = dspre();
        std::fs::create_dir_all(d.root.join("build")).unwrap();
        std::fs::write(d.root.join("build/X-v0.1.0.xdelta"), b"p").unwrap();
        std::fs::write(d.root.join("build/anderes.txt"), b"x").unwrap();
        // Nothing listed yet: even the file is unknown.
        let err = reveal_path_impl(&d.state, "build/X-v0.1.0.xdelta").unwrap_err();
        assert_eq!(err.message, NOT_A_RELEASE_FILE);

        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().join("Y-v0.2.0.xdelta");
        std::fs::write(&outside, b"p").unwrap();
        put_releases(
            &d.root,
            &[
                release("0.1.0", "build/X-v0.1.0.xdelta"),
                release("0.2.0", outside.to_str().unwrap()),
            ],
        );
        // A listed file (relative or absolute), its folder, and an absolute entry.
        reveal_path_impl(&d.state, "build/X-v0.1.0.xdelta").unwrap();
        reveal_path_impl(&d.state, d.root.join("build").to_str().unwrap()).unwrap();
        reveal_path_impl(&d.state, outside.to_str().unwrap()).unwrap();
        reveal_path_impl(&d.state, elsewhere.path().to_str().unwrap()).unwrap();
        // Anything else is refused: another file in the folder, the project, an existing file
        // outside (absolute; this test binary exists on every platform).
        let stranger = std::env::current_exe().unwrap();
        for path in ["build/anderes.txt", ".", stranger.to_str().unwrap()] {
            let err = reveal_path_impl(&d.state, path).unwrap_err();
            assert_eq!(err.message, NOT_A_RELEASE_FILE, "{path}");
        }
        let err = reveal_path_impl(&d.state, "build/fehlt.xdelta").unwrap_err();
        assert!(
            err.message
                .starts_with("Pfad nicht gefunden: build/fehlt.xdelta"),
            "{}",
            err.message
        );
    }

    #[test]
    fn publish_args_take_the_flat_keys_with_defaults() {
        let args: PublishArgs = serde_json::from_value(serde_json::json!({
            "version": "1.0.0", "notes": null, "dir": "out", "readme": true,
        }))
        .unwrap();
        assert_eq!(
            args.options(),
            PublishOptions {
                version: "1.0.0".into(),
                notes: String::new(),
                dir: Some(PathBuf::from("out")),
                file: None,
                readme: true,
                overwrite: false,
            }
        );
    }
    /// The vanilla copy (`G4_TEST_PROJECT`, written only as a copy, its base ROM copied beside
    /// it) over the command path: Git set up with an isolated `git`, trainer 1's first Pokémon
    /// one level up over `data_edit` and saved, published as 0.1.0 (patch and notes under
    /// `build/`, one release), then „Stand sichern“ commits everything and the tree is clean.
    /// Prints the times.
    #[test]
    fn publish_and_commit_over_the_command_path_on_the_vanilla_copy() {
        use crate::commands::dispatch::{DispatchError, Reply, dispatch};
        use crate::commands::policy::UserPolicy;
        use crate::commands::{git_commit_all_with, git_init_with, git_status_impl};
        use g4_kernel::git::{CommitOptions, InitOptions};
        use g4_kernel::test_support::{GitFixture, vanilla_base_rom, vanilla_copy};
        use serde_json::{Value, json};

        let home = tempfile::tempdir().unwrap();
        let Some(cli) = GitFixture::cli(home.path()) else {
            return;
        };
        let Some((tmp, root)) = vanilla_copy() else {
            return;
        };
        let Some(vanilla_rom) = vanilla_base_rom() else {
            return;
        };
        let base = tmp.path().join("base.nds");
        std::fs::copy(&vanilla_rom, &base).unwrap();
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        state.base_roms.set_explicit(Some(base.clone()));
        if !root.join(".g4/base.json").is_file() {
            assign_base_impl(&state, base.to_str().unwrap()).unwrap();
        }
        let policy = UserPolicy::new(&root, &[], None).unwrap();

        let started = Instant::now();
        let init = git_init_with(&state, Some(&cli), InitOptions::default(), &mut |_| {}).unwrap();
        assert!(init.committed, "{init:?}");
        eprintln!("git init: {:?} ({} files)", started.elapsed(), init.files);

        let call = |name: &str, args: Value| match dispatch(&state, &policy, name, args) {
            Ok(Reply::Json(value)) => value,
            Ok(Reply::Bytes(_)) => panic!("bytes"),
            Err(DispatchError::Cmd(e)) => panic!("{name}: {}: {}", e.code, e.message),
            Err(DispatchError::Unknown(n)) => panic!("{n}"),
        };
        let doc = call("data_get", json!({ "kind": "trainer", "index": 1 }));
        let mut party = doc["party"].clone();
        let level = party[0]["level"].as_u64().unwrap();
        party[0]["level"] = json!(level + 1);
        call(
            "data_edit",
            json!({ "kind": "trainer", "index": 1, "patch": { "TrainerParty": { "party": party } } }),
        );
        call("save", json!({}));

        let started = Instant::now();
        let mut options = opts("0.1.0");
        options.notes = "Erste Fassung".into();
        let mut phases = Vec::new();
        let dto = publish_patch_impl(&state, options, &mut |p| phases.push(p.phase)).unwrap();
        let report = &dto.report;
        assert_eq!(Path::new(&dto.build.rom), report.patch.rom);
        assert!(!dto.build.steps.is_empty());
        eprintln!(
            "publish 0.1.0: {:?} (report {} ms, phases {:?}, patch {} bytes)",
            started.elapsed(),
            report.ms,
            report.phase_ms,
            report.patch.size
        );
        assert_eq!(report.files.len(), 2, "{:?}", report.files);
        let build = root.join("build");
        for file in &report.files {
            assert!(file.path.starts_with(&build), "{}", file.path.display());
            assert!(file.path.is_file(), "{}", file.path.display());
        }
        assert_eq!(report.releases.releases.len(), 1);
        assert_eq!(phases.first(), Some(&PublishPhase::Build));
        assert_eq!(releases_impl(&state).unwrap().next_version, "0.1.1");

        let started = Instant::now();
        let subject = CommitOptions {
            message: Some("Veröffentlicht v0.1.0".into()),
            notes: Some("Erste Fassung".into()),
        };
        let commit = git_commit_all_with(&state, Some(&cli), subject, &mut |_| {}).unwrap();
        eprintln!(
            "commit all: {:?} ({} files, phases {:?})",
            started.elapsed(),
            commit.files,
            commit.phase_ms
        );
        assert!(commit.committed, "{commit:?}");
        let status = git_status_impl(&state).unwrap();
        eprintln!("status after the commit: {} ms", status.ms);
        assert_eq!(status.changes, 0, "{:?}", status.entries);
        assert_eq!(status.head.unwrap().summary, "Veröffentlicht v0.1.0");
        let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
        assert!(!config.contains("[remote"), "{config}");
    }
}
