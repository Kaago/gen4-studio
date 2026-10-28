//! Opening, saving, undo, build and run.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use g4_kernel::build::{BuildReport, StepReport, build_prepared, clear_cache, prepare_for_build};
use g4_kernel::project::{AssetId, AssetKind, Project, SourceMode};
use g4_kernel::run::launch_emulator;
use g4_kernel::store::{HistoryEntry, Origin, Store};
use g4_kernel::text::{TextKeep, TextSide, TextSync, resolve_conflict};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use super::scripts::reload_text_archives;
use super::textsource::{TextProjectStatusDto, status_of, sync_before_build};
use super::{ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub root: String,
    pub game_code: String,
    pub family: String,
    pub language: String,
    pub header_count: usize,
    /// What the project is as a source: a DSPRE folder (and whether it can be set up as text
    /// project), a text project of the current schema or of an older one.
    pub text_project: TextProjectStatusDto,
}

/// What `open_project` answers about `project` (`state` finds the base ROM of a DSPRE
/// folder). Only the header table is read: location names are cosmetic and must not gate
/// opening.
pub fn project_info_of(state: &AppState, project: &Project) -> Result<ProjectInfo, CmdError> {
    Ok(ProjectInfo {
        root: project.root.display().to_string(),
        game_code: project.game_code.clone(),
        family: format!("{:?}", project.family),
        language: format!("{:?}", project.language),
        header_count: project.header_count().map_err(CmdError::from)?,
        text_project: status_of(state, project),
    })
}

/// Opens the project at `path` in a fresh session, with its ignore file `g4-checks.json` read.
/// A broken one does not keep the project closed (ruling R15): nothing is ignored, the checks'
/// reports name the file, and it is never written until it reads again (see
/// `commands::checks`). A text project of a newer schema or with a manifest that does not
/// read is not opened (its sentence, `Project::editable`); one of an older schema opens to be
/// lifted (`text_project` says `older`, the store refuses every edit until then).
pub fn open_project_impl(state: &AppState, path: &str) -> Result<ProjectInfo, CmdError> {
    let (session, info) = new_session(state, path)?;
    // Under the agents' turn: an agent call admitted for the old project never runs on the
    // new one (it checks the project after taking the turn). Agent calls take the turn
    // before the session lock and never while waiting for the user, so this waits at most
    // for one agent call to finish.
    let turn = state.agents.agent_turn();
    let old = lock(state)?.replace(session);
    drop(turn);
    // The old project's language server ends with its session, outside the state lock.
    drop(old);
    Ok(info)
}

/// A fresh session of the project at `path` and what `open_project` answers about it (see
/// [`open_project_impl`]); nothing is replaced yet.
pub(crate) fn new_session(
    state: &AppState,
    path: &str,
) -> Result<(Session, ProjectInfo), CmdError> {
    let project = Project::open(path).map_err(CmdError::from)?;
    if matches!(
        project.source_mode(),
        SourceMode::Newer { .. } | SourceMode::Broken { .. }
    ) {
        project.editable()?;
    }
    // The id files for rotom, as the registers have them now (a branch switch, a hand edit):
    // the script editor's language server and the `rotom` CLI read them from disk. Only
    // written when they change; one that cannot be written now is written by the next
    // compile, which reports why, so it does not keep the project closed.
    let _ = g4_kernel::textproject::include::write_include(&project);
    let (ignore, ignore_broken) = super::checks::load_project_ignore(&project);
    let info = project_info_of(state, &project)?;
    let mut session = Session::new(Arc::new(project));
    session.checks.ignore = ignore;
    session.checks.ignore_broken = ignore_broken;
    Ok((session, info))
}

/// What `build_rom` did: how many dirty assets were saved first, the asset kinds whose pack
/// step ran (by unpacked folder name, e.g. `maps`, `eventFiles`; kinds whose step was skipped
/// are missing) with their member counts, the ROM path,
/// the script files compiled and the text archives encoded before packing, the warnings
/// of the pre-build save and of those two steps (`source` tells them apart), one row per
/// build step in build order and the milliseconds of the build (without the save).
#[derive(Debug, Serialize)]
pub struct BuildDto {
    pub saved: usize,
    pub packed: Vec<(String, usize)>,
    pub rom: String,
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    pub problems: Vec<ProblemDto>,
    /// See [`BuildReport::steps`]: `rotom`, `chatot`, `pack:*`, `watch:*` (not for
    /// display), then `rom` (skipped) or `blz:arm9`, `blz:overlays`, `rom`, `crc`.
    pub steps: Vec<StepReport>,
    pub ms: u64,
}

/// Member count of every asset kind the project has an archive for (the header table counts
/// as one asset). Kinds the game lacks are left out. Counting is I/O only (it counts the
/// unpacked members, or parses the whole NARC of a kind that is not unpacked yet), so it
/// runs without the state lock.
pub fn member_counts_impl(state: &AppState) -> Result<HashMap<AssetKind, usize>, CmdError> {
    let project = {
        let mut guard = lock(state)?;
        Arc::clone(&session(&mut guard)?.project)
    };
    // `member_count` fails for kinds without an archive in this family and for the store-only
    // `DynamicHeaders` kind (its files exist only once the patch is applied), so both are left out.
    Ok(AssetKind::ALL
        .into_iter()
        .filter_map(|kind| Some((kind, project.member_count(kind).ok()?)))
        .collect())
}

/// Reverts the latest change and names every asset it restored (empty: nothing to undo), so
/// the UI reloads exactly what those assets affect. A change that cannot be restored is an
/// error naming the asset, and nothing changes.
pub fn undo_impl(state: &AppState) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let restored = session.store.undo()?;
    after_step(session, &restored);
    Ok(restored)
}

/// Re-applies the latest undone change, see [`undo_impl`].
pub fn redo_impl(state: &AppState) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let restored = session.store.redo()?;
    after_step(session, &restored);
    Ok(restored)
}

/// Whether assets of `kind` are files rather than store values: their steps are file steps
/// (a reset of a script, texture or model) that undo and redo write at once.
fn is_file_kind(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::Script
            | AssetKind::MapTexture
            | AssetKind::BuildingTexture
            | AssetKind::BuildingModel
            | AssetKind::InteriorBuildingModel
    )
}

/// After an undo or redo that restored `restored`: a file step wrote the project's files
/// outside the store ([`crate::state::Session::files_written`], with the textures and models
/// it wrote); the step of an id rename (its asset a trainer or the header table) wrote the id
/// files for rotom with the JSON and the script sources: the language server is told, and
/// the checks see the files as written.
fn after_step(session: &mut crate::state::Session, restored: &[AssetId]) {
    if restored.iter().any(|a| is_file_kind(a.kind)) {
        session.files_written(restored);
    }
    if restored
        .iter()
        .any(|a| matches!(a.kind, AssetKind::Trainer | AssetKind::HeaderTable))
        && !session.id_files_changed().is_empty()
    {
        session.files_written(&[]);
    }
    session.after_write();
}

/// The labels of the undo and redo stacks: undo oldest first, redo in the order `redo` applies
/// them (the next one first). `undo_agents` and `redo_agents` run parallel to the labels: the
/// name of the agent client that made the step, `None` for the user's own steps.
#[derive(Debug, PartialEq, Serialize)]
pub struct HistoryDto {
    pub undo: Vec<String>,
    pub redo: Vec<String>,
    pub undo_agents: Vec<Option<String>>,
    pub redo_agents: Vec<Option<String>>,
}

pub fn history_impl(state: &AppState) -> Result<HistoryDto, CmdError> {
    let mut guard = lock(state)?;
    Ok(history_of(&session(&mut guard)?.store))
}

/// The undo and redo history of `store`, with who made each step.
pub fn history_of(store: &Store) -> HistoryDto {
    let (undo, redo) = store.history_entries();
    let agent = |e: &HistoryEntry| match &e.origin {
        Origin::User => None,
        Origin::Agent { client } => Some(client.clone()),
    };
    HistoryDto {
        undo_agents: undo.iter().map(agent).collect(),
        redo_agents: redo.iter().map(agent).collect(),
        undo: undo.into_iter().map(|e| e.label).collect(),
        redo: redo.into_iter().map(|e| e.label).collect(),
    }
}

pub fn dirty_impl(state: &AppState) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.dirty())
}

/// What `save` did: the assets written and the warnings found while encoding them (text
/// warnings never block the save). A save that wrote the header table of an adopted project
/// compiles what is stale afterwards: a header that now uses a script file or another text
/// archive makes that file stale, and a source whose text literal did not resolve (no saved
/// header used its file) is stale anyway, so its problem goes once the header is saved.
#[derive(Debug, Serialize)]
pub struct SaveDto {
    pub written: Vec<AssetId>,
    /// The text warnings of the save, then the problems of that compile.
    pub problems: Vec<ProblemDto>,
    /// Sources that compile compiled without an error (project-relative, forward slashes):
    /// their earlier compile problems are gone.
    pub compiled: Vec<String>,
    /// Text archives that compile appended to; the store holds them again.
    pub touched_archives: Vec<u16>,
}

pub fn save_impl(state: &AppState) -> Result<SaveDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let report = session.store.save()?;
    let project = Arc::clone(&session.project);
    let root = &project.root;
    let mut dto = SaveDto {
        written: report.written,
        problems: report
            .problems
            .iter()
            .map(|p| ProblemDto::from_kernel(p, root, "text"))
            .collect(),
        compiled: Vec::new(),
        touched_archives: Vec::new(),
    };
    let header_table_written = dto
        .written
        .iter()
        .any(|id| id.kind == AssetKind::HeaderTable);
    if header_table_written && g4_kernel::scripts::is_adopted(&project) {
        let compile = g4_kernel::scripts::compile(&project)?;
        reload_text_archives(&mut session.store, &compile.touched_archives)?;
        dto.problems.extend(
            compile
                .problems
                .iter()
                .map(|p| ProblemDto::by_origin(p, root)),
        );
        dto.compiled = compile
            .compiled
            .iter()
            .map(|&n| {
                let kind = g4_kernel::scripts::source_kind(&project, n);
                let source = g4_kernel::scripts::source_path(&project, n, kind);
                let relative = source.strip_prefix(root).unwrap_or(&source);
                relative.to_string_lossy().replace('\\', "/")
            })
            .collect();
        dto.touched_archives = compile.touched_archives;
    }
    Ok(dto)
}

/// Drops the own changes (and their undo/redo entries) of one asset and reloads it from its
/// file on disk.
pub fn reload_asset_impl(state: &AppState, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.reload_from_disk(AssetId { kind, index })?;
    session.disk_changed();
    Ok(())
}

/// Keeps the in-memory version of one asset and takes the file on disk as the new baseline,
/// so the next save no longer reports it as stale.
pub fn accept_disk_impl(state: &AppState, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.accept_disk_as_base(AssetId { kind, index })?;
    Ok(())
}

/// Saves dirty assets (a stale file aborts with nothing built), then builds the ROM. With
/// `fresh` the build cache (`.g4/build/`, all but the text sync, see `build::clear_cache`)
/// is deleted first, so every step runs
/// fully. The
/// state lock is held for the save, for compiling scripts and encoding texts (they write
/// sources, members and the compile state, which the script and text commands write too, see
/// `g4_kernel::build::prepare`) and for the reload afterwards, not for packing and the ds-rom
/// build. An error in the compile step is `CmdError { code: "compile" }` and nothing is
/// packed; text archives changed as JSON and as member since the last build stop it as
/// `CmdError { code: "text_conflict" }` before anything is written. Text archives the
/// compile appended messages to are loaded again in the store. In a text project the sync
/// runs first (without the agents' turn, which an agent's build holds already): what DSPRE
/// changed is taken over, and an open conflict refuses the build with `conflicts`.
pub fn build_rom_impl(state: &AppState, out: &str, fresh: bool) -> Result<BuildDto, CmdError> {
    // A text project takes over what DSPRE changed first; an open conflict stops the build
    // before anything is saved or written (`conflicts`). The agents' turn is the caller's
    // (an agent's build holds it).
    sync_before_build(state, false)?;
    build_synced(state, out, fresh)
}

/// [`build_rom_impl`] once the sync before it ran.
fn build_synced(state: &AppState, out: &str, fresh: bool) -> Result<BuildDto, CmdError> {
    let out = Path::new(out);
    let (project, prepared, saved, problems) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        if fresh {
            clear_cache(&session.project)?;
        }
        let (saved, problems) = if session.store.dirty().is_empty() {
            (0, Vec::new())
        } else {
            let report = session.store.save()?;
            let root = &session.project.root;
            let warnings = report
                .problems
                .iter()
                .map(|p| ProblemDto::from_kernel(p, root, "text"))
                .collect();
            (report.written.len(), warnings)
        };
        let prepared = prepare_for_build(&session.project, out).map_err(CmdError::from)?;
        (Arc::clone(&session.project), prepared, saved, problems)
    };
    let report = build_prepared(&project, out, prepared).map_err(CmdError::from)?;
    build_dto(state, &project, report, saved, problems)
}

/// The result of a finished build; reloads the text archives its compile changed. The lock
/// was released for packing, so the session may be another one by now (a project opened
/// meanwhile): then nothing is reloaded, the archives belong to `project`, not to it. An
/// archive edited in this session since the build started is kept and reported.
fn build_dto(
    state: &AppState,
    project: &Arc<Project>,
    report: BuildReport,
    saved: usize,
    mut problems: Vec<ProblemDto>,
) -> Result<BuildDto, CmdError> {
    problems.extend(
        report
            .problems
            .iter()
            .map(|p| ProblemDto::by_origin(p, &project.root)),
    );
    if !report.touched_archives.is_empty() {
        let mut guard = lock(state)?;
        if let Ok(session) = session(&mut guard)
            && Arc::ptr_eq(&session.project, project)
        {
            let warning = |index: u16, message: String| ProblemDto {
                severity: "warning",
                file: format!("expanded/textArchives/{index:04}.json"),
                line: 1,
                col: 1,
                message,
                source: "text",
                check: None,
            };
            for &index in &report.touched_archives {
                let id = AssetId {
                    kind: AssetKind::TextArchive,
                    index,
                };
                // The ROM is built: a failed reload is a warning, not a failed build.
                if session.store.is_dirty(id) {
                    problems.push(warning(
                        index,
                        format!(
                            "Textarchiv {index:04} wurde beim Bauen geändert, ist aber ungespeichert bearbeitet — nicht neu geladen"
                        ),
                    ));
                } else if let Err(e) = session.store.reload_from_disk(id) {
                    problems.push(warning(
                        index,
                        format!("Textarchiv {index:04} konnte nicht neu geladen werden: {e}"),
                    ));
                }
            }
        }
    }
    Ok(BuildDto {
        saved,
        packed: report
            .packed
            .iter()
            .map(|(kind, count)| (project.archives.unpacked_name(*kind).to_owned(), *count))
            .collect(),
        rom: report.rom.display().to_string(),
        compiled: report.compiled,
        encoded: report.encoded,
        problems,
        steps: report.steps,
        ms: report.ms,
    })
}

/// Which version of a text archive in conflict applies (see `resolve_text_conflicts`).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct TextChoice {
    pub archive: u16,
    pub keep: TextKeep,
}

/// Resolves text archives whose JSON and member both changed since the last build, each as
/// `choices` says (`g4_kernel::text::resolve_conflict`: the JSON kept encodes the member, the
/// member kept rewrites the JSON; either way the other version is overwritten), under the
/// state lock. Every archive must be in conflict (`TextSide::Conflict`), else nothing is
/// written for any of them: an archive whose member alone changed (DSPRE) would lose that
/// change. Archives the store holds unedited are loaded again; an edited one keeps the edit
/// (its next save is stale, and accepting the disk saves it). Answers the archives in the
/// order of `choices`. A write that fails stops it; those before it stay resolved.
pub fn resolve_text_conflicts_impl(
    state: &AppState,
    choices: Vec<TextChoice>,
) -> Result<Vec<u16>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let project = Arc::clone(&session.project);
    let sync = TextSync::load(&project);
    for choice in &choices {
        if sync.side(&project, choice.archive)? != TextSide::Conflict {
            return Err(CmdError::from(format!(
                "Textarchiv {:04} ist nicht in Konflikt: Text (JSON) und Spieldatei wurden \
                 seit dem letzten Bauen nicht beide geändert. Es wurde nichts überschrieben; \
                 baue erneut, dann nennt der Build die Archive, die noch in Konflikt sind.",
                choice.archive
            )));
        }
    }
    let attempted = !choices.is_empty();
    let mut resolved = Vec::with_capacity(choices.len());
    let mut result = Ok(());
    for choice in choices {
        result = resolve_one(&mut session.store, &project, choice);
        if result.is_err() {
            break;
        }
        resolved.push(choice.archive);
    }
    if attempted {
        // The checks read the archives the store does not hold from the disk. Also after a
        // failed first choice: it may have written one of the files before it failed.
        session.files_written(&[]);
    }
    result.map(|()| resolved)
}

/// One choice of [`resolve_text_conflicts_impl`]: resolves, then reloads the archive when the
/// store holds it unedited.
fn resolve_one(store: &mut Store, project: &Project, choice: TextChoice) -> Result<(), CmdError> {
    resolve_conflict(project, choice.archive, choice.keep)?;
    let id = AssetId {
        kind: AssetKind::TextArchive,
        index: choice.archive,
    };
    if store.is_loaded(id) && !store.is_dirty(id) {
        store.reload_from_disk(id)?;
    }
    Ok(())
}

pub fn run_rom_impl(emulator: &str, rom: &str) -> Result<u32, CmdError> {
    launch_emulator(Path::new(emulator), Path::new(rom)).map_err(CmdError::from)
}

/// Opening waits for a running agent call (the agent turn), so it runs on a blocking worker,
/// off the main thread.
#[tauri::command]
pub async fn open_project(app: tauri::AppHandle, path: String) -> Result<ProjectInfo, CmdError> {
    tauri::async_runtime::spawn_blocking(move || open_project_impl(&app.state::<AppState>(), &path))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub fn member_counts(state: State<AppState>) -> Result<HashMap<AssetKind, usize>, CmdError> {
    member_counts_impl(&state)
}

#[tauri::command]
pub fn undo(state: State<AppState>) -> Result<Vec<AssetId>, CmdError> {
    undo_impl(&state)
}

#[tauri::command]
pub fn redo(state: State<AppState>) -> Result<Vec<AssetId>, CmdError> {
    redo_impl(&state)
}

#[tauri::command]
pub fn history(state: State<AppState>) -> Result<HistoryDto, CmdError> {
    history_impl(&state)
}

#[tauri::command]
pub fn dirty(state: State<AppState>) -> Result<Vec<AssetId>, CmdError> {
    dirty_impl(&state)
}

#[tauri::command]
pub fn save(state: State<AppState>) -> Result<SaveDto, CmdError> {
    save_impl(&state)
}

#[tauri::command]
pub fn reload_asset(state: State<AppState>, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    reload_asset_impl(&state, kind, index)
}

#[tauri::command]
pub fn accept_disk(state: State<AppState>, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    accept_disk_impl(&state, kind, index)
}

/// The ds-rom build takes seconds, so it runs on a blocking worker, off the main thread and
/// without holding the state lock. `fresh` (default false) drops the build cache first. The
/// user's build takes the agents' turn for the sync before it (no agent call writes while it
/// takes changes over).
#[tauri::command]
pub async fn build_rom(
    app: tauri::AppHandle,
    out: String,
    fresh: Option<bool>,
) -> Result<BuildDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        sync_before_build(&state, true)?;
        build_synced(&state, &out, fresh.unwrap_or(false))
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub fn resolve_text_conflicts(
    state: State<AppState>,
    choices: Vec<TextChoice>,
) -> Result<Vec<u16>, CmdError> {
    resolve_text_conflicts_impl(&state, choices)
}

/// Quits after the user confirmed discarding unsaved edits: the flag lets the exit request
/// raised by `exit` through the exit guard.
#[tauri::command]
pub fn force_quit(app: tauri::AppHandle) {
    app.state::<AppState>()
        .force_quit
        .store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
pub fn run_rom(emulator: String, rom: String) -> Result<u32, CmdError> {
    run_rom_impl(&emulator, &rom)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;
    use g4_formats::map::MapFile;

    #[test]
    fn commands_work_on_the_synthetic_project() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(info.game_code, "CPUE");
        assert_eq!(info.header_count, 2);
        let headers = list_headers_impl(&state).unwrap().headers;
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].internal_name, "T01");
        let map = get_map_impl(&state, 0).unwrap();
        assert_eq!(map.permissions.len(), 1024);
        let matrix = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((matrix.width, matrix.height), (2, 1));
        assert_eq!(matrix.maps, vec![Some(0), Some(1)]);
    }

    #[test]
    fn member_counts_cover_every_archive_and_the_header_table() {
        let (_fixture, state) = open_synthetic();
        let counts = member_counts_impl(&state).unwrap();
        assert_eq!(counts[&AssetKind::Map], 3);
        assert_eq!(counts[&AssetKind::Matrix], 1);
        assert_eq!(counts[&AssetKind::AreaData], 2);
        assert_eq!(counts[&AssetKind::Event], 2);
        assert_eq!(counts[&AssetKind::TextArchive], 698);
        assert_eq!(counts[&AssetKind::HeaderTable], 1);
        assert_eq!(counts[&AssetKind::Script], 3);
        assert_eq!(counts[&AssetKind::Personal], 508);
        assert_eq!(counts[&AssetKind::Item], 20);
        assert_eq!(counts[&AssetKind::TrainerTextOffsets], 1);
        assert!(!counts.contains_key(&AssetKind::DynamicHeaders));
        // Serialized for the frontend as an object keyed by the kind name.
        let json = serde_json::to_value(&counts).unwrap();
        assert_eq!(json["AreaData"], 2);
        assert!(member_counts_impl(&AppState::default()).is_err());
    }

    #[test]
    fn history_names_the_undo_and_redo_entries() {
        let (_fixture, state) = open_synthetic();
        let none = HistoryDto {
            undo: vec![],
            redo: vec![],
            undo_agents: vec![],
            redo_agents: vec![],
        };
        assert_eq!(history_impl(&state).unwrap(), none);
        set_permission_impl(&state, 1, 5, 6, 2, 0x80).unwrap();
        set_permission_impl(&state, 1, 6, 6, 2, 0x80).unwrap();
        let labels = |n: usize| vec!["paint".to_owned(); n];
        assert_eq!(
            history_impl(&state).unwrap(),
            HistoryDto {
                undo: labels(2),
                redo: vec![],
                undo_agents: vec![None, None],
                redo_agents: vec![]
            }
        );
        undo_impl(&state).unwrap();
        assert_eq!(
            history_impl(&state).unwrap(),
            HistoryDto {
                undo: labels(1),
                redo: labels(1),
                undo_agents: vec![None],
                redo_agents: vec![None]
            }
        );
        // Serialized for the frontend with the agent names parallel to the labels.
        let json = serde_json::to_value(history_impl(&state).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "undo": ["paint"],
                "redo": ["paint"],
                "undo_agents": [null],
                "redo_agents": [null]
            })
        );
        assert!(history_impl(&AppState::default()).is_err());
    }

    #[test]
    fn history_names_agent_steps() {
        let (_fixture, state) = open_synthetic();
        set_permission_impl(&state, 1, 5, 6, 2, 0x80).unwrap();
        g4_kernel::store::with_origin(
            g4_kernel::store::Origin::Agent {
                client: "Claude Code".to_owned(),
            },
            || set_permission_impl(&state, 1, 6, 6, 2, 0x80).unwrap(),
        );
        let history = history_impl(&state).unwrap();
        assert_eq!(
            history.undo_agents,
            vec![None, Some("Claude Code".to_owned())]
        );
        assert!(history.redo_agents.is_empty());
        undo_impl(&state).unwrap();
        let history = history_impl(&state).unwrap();
        assert_eq!(history.undo_agents, vec![None]);
        assert_eq!(history.redo_agents, vec![Some("Claude Code".to_owned())]);
    }

    #[test]
    fn save_writes_dirty_maps_and_clears_dirty() {
        let (fixture, state) = open_synthetic();
        assert!(save_impl(&state).unwrap().written.is_empty());
        set_permission_impl(&state, 1, 5, 6, 2, 0x80).unwrap();
        let report = save_impl(&state).unwrap();
        assert_eq!(report.written.len(), 1);
        assert_eq!(report.written[0].index, 1);
        assert!(dirty_impl(&state).unwrap().is_empty());
        let bytes = std::fs::read(fixture.dir.path().join("unpacked/maps/0001")).unwrap();
        let map = MapFile::read(&bytes, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(map.permissions[6 * 32 + 5].collision, 0x80);
    }

    #[test]
    fn build_saves_first_and_a_stale_file_aborts_it() {
        let (fixture, state) = open_synthetic();
        let out = fixture.dir.path().join("out/rom.nds");
        set_permission_impl(&state, 0, 1, 1, 2, 0x80).unwrap();
        // The synthetic project has no real ds-rom config, so the ROM build itself fails,
        // but only after the dirty map was saved.
        assert!(build_rom_impl(&state, out.to_str().unwrap(), false).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let on_disk = std::fs::read(fixture.dir.path().join("unpacked/maps/0000")).unwrap();
        let map = MapFile::read(&on_disk, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(
            map.permissions[32 + 1],
            g4_formats::map::Cell {
                kind: 2,
                collision: 0x80
            }
        );

        // A member changed behind the editor's back: the save refuses, so nothing is built.
        set_permission_impl(&state, 0, 2, 2, 3, 0x80).unwrap();
        let member = fixture.dir.path().join("unpacked/maps/0000");
        let mut bytes = std::fs::read(&member).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&member, &bytes).unwrap();
        let err = build_rom_impl(&state, out.to_str().unwrap(), false).unwrap_err();
        assert!(err.message.contains("changed on disk"), "{}", err.message);
        assert!(!out.exists());
    }

    #[test]
    fn build_with_a_compile_error_is_code_compile_and_packs_nothing() {
        let (fixture, state) = open_synthetic();
        scripts::adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        scripts::save_script_impl(&state, 0, "script Main #1:\n    NoSuch\n    End\n".into())
            .unwrap();
        let out = fixture.dir.path().join("out/rom.nds");
        let err = build_rom_impl(&state, out.to_str().unwrap(), false).unwrap_err();
        assert_eq!(err.code, "compile");
        assert_eq!(err.message, "1 Fehler in Scripts oder Texten");
        assert_eq!(err.problems.len(), 1, "{:?}", err.problems);
        let problem = &err.problems[0];
        assert_eq!(
            (problem.source, problem.file.as_str(), problem.line),
            ("compile", "expanded/scripts/0000.rotom", 2)
        );
        assert!(!out.exists());
    }

    #[test]
    fn build_rom_with_fresh_clears_the_build_folder() {
        let (fixture, state) = open_synthetic();
        let out = fixture.dir.path().join("out/rom.nds");
        let marker = fixture.dir.path().join(".g4/build/blz/marker");
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"x").unwrap();
        let base = fixture.dir.path().join(".g4/base.json");
        std::fs::write(&base, b"{}").unwrap();
        // The synthetic project cannot be built into a ROM: the build fails after its
        // first steps either way. Without `fresh` the cache stays, with it it goes first.
        assert!(build_rom_impl(&state, out.to_str().unwrap(), false).is_err());
        assert!(marker.exists());
        assert!(build_rom_impl(&state, out.to_str().unwrap(), true).is_err());
        assert!(!marker.exists());
        // The project's own metadata at the top of .g4/ stays.
        assert_eq!(std::fs::read(&base).unwrap(), b"{}");
    }

    fn text_json_path(project: &Project, archive: u16) -> std::path::PathBuf {
        project
            .expanded_dir(AssetKind::TextArchive)
            .join(format!("{archive:04}.json"))
    }

    /// Writes the JSON of archive 433 from its member and records both as in sync.
    fn sync_433(project: &Project) {
        project.ensure_unpacked(AssetKind::TextArchive).unwrap();
        let member = project.member_path(AssetKind::TextArchive, 433);
        let doc =
            g4_kernel::text::TextArchiveDoc::from_member(&std::fs::read(member).unwrap(), "en_US")
                .unwrap();
        let json = text_json_path(project, 433);
        std::fs::create_dir_all(json.parent().unwrap()).unwrap();
        std::fs::write(&json, doc.to_json()).unwrap();
        g4_kernel::text::record_text_sync(project, &[433]).unwrap();
    }

    /// Changes the JSON and the member of archive 433 (recorded in sync) to different texts:
    /// a conflict.
    fn change_both_sides_of_433(project: &Project, json_text: &str, member_text: &str) {
        let bytes = |text: &str| g4_kernel::test_support::text_archive_bytes(&[text]);
        let doc = g4_kernel::text::TextArchiveDoc::from_member(&bytes(json_text), "en_US").unwrap();
        std::fs::write(text_json_path(project, 433), doc.to_json()).unwrap();
        std::fs::write(
            project.member_path(AssetKind::TextArchive, 433),
            bytes(member_text),
        )
        .unwrap();
        assert_eq!(
            g4_kernel::text::TextSync::load(project)
                .side(project, 433)
                .unwrap(),
            g4_kernel::text::TextSide::Conflict
        );
    }

    #[test]
    fn a_text_conflict_is_cmd_error_text_conflict_with_one_problem_per_archive() {
        let (fixture, state) = open_synthetic();
        let project = session_project(&state);
        sync_433(&project);
        change_both_sides_of_433(&project, "Aus der JSON", "Aus dem Member");
        let member = std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap();

        let out = fixture.dir.path().join("out/rom.nds");
        let err = build_rom_impl(&state, out.to_str().unwrap(), false).unwrap_err();
        assert_eq!(err.code, "text_conflict", "{}", err.message);
        assert_eq!(err.problems.len(), 1, "{:?}", err.problems);
        let problem = &err.problems[0];
        assert_eq!(
            (problem.severity, problem.source, problem.file.as_str()),
            ("error", "build", "expanded/textArchives/0433.json")
        );
        // Nothing was overwritten.
        assert_eq!(
            std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap(),
            member
        );

        // A store save of the archive meets the same conflict and says so the same way.
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 0,
                text: "Im Editor".into(),
            },
        )
        .unwrap();
        let err = save_impl(&state).unwrap_err();
        assert_eq!(err.code, "text_conflict", "{}", err.message);
        assert_eq!(
            err.problems
                .iter()
                .map(|p| p.file.as_str())
                .collect::<Vec<_>>(),
            ["expanded/textArchives/0433.json"]
        );
    }

    #[test]
    fn resolve_text_conflicts_reloads_clean_archives() {
        let (_fixture, state) = open_synthetic();
        let project = session_project(&state);
        sync_433(&project);
        // Loaded and clean.
        assert_eq!(archive_texts(&state).len(), 3);
        change_both_sides_of_433(&project, "Aus der JSON", "Aus dem Member");

        let resolved = resolve_text_conflicts_impl(
            &state,
            vec![TextChoice {
                archive: 433,
                keep: TextKeep::Member,
            }],
        )
        .unwrap();
        assert_eq!(resolved, [433]);
        assert_eq!(archive_texts(&state), ["Aus dem Member"]);
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert_eq!(
            g4_kernel::text::TextSync::load(&project)
                .side(&project, 433)
                .unwrap(),
            g4_kernel::text::TextSide::InSync
        );

        // An archive edited in the store keeps the edit: it is not reloaded.
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 0,
                text: "Meine Änderung".into(),
            },
        )
        .unwrap();
        change_both_sides_of_433(&project, "JSON zwei", "Member zwei");
        let resolved = resolve_text_conflicts_impl(
            &state,
            vec![TextChoice {
                archive: 433,
                keep: TextKeep::Json,
            }],
        )
        .unwrap();
        assert_eq!(resolved, [433]);
        assert_eq!(archive_texts(&state), ["Meine Änderung"]);
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
    }

    #[test]
    fn resolve_text_conflicts_refuses_an_archive_that_is_not_in_conflict() {
        let (_fixture, state) = open_synthetic();
        let project = session_project(&state);
        sync_433(&project);
        change_both_sides_of_433(&project, "Aus der JSON", "Aus dem Member");
        // Archive 7 has no recorded sync: not in conflict.
        let json = std::fs::read(text_json_path(&project, 433)).unwrap();
        let member = std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap();
        let err = resolve_text_conflicts_impl(
            &state,
            vec![
                TextChoice {
                    archive: 433,
                    keep: TextKeep::Json,
                },
                TextChoice {
                    archive: 7,
                    keep: TextKeep::Json,
                },
            ],
        )
        .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(
            err.message
                .starts_with("Textarchiv 0007 ist nicht in Konflikt"),
            "{}",
            err.message
        );
        // Nothing was written, not even for the archive in conflict.
        assert_eq!(std::fs::read(text_json_path(&project, 433)).unwrap(), json);
        assert_eq!(
            std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap(),
            member
        );

        // A member changed alone (DSPRE) is not in conflict either: keeping the JSON would
        // throw that change away.
        resolve_text_conflicts_impl(
            &state,
            vec![TextChoice {
                archive: 433,
                keep: TextKeep::Member,
            }],
        )
        .unwrap();
        std::fs::write(
            project.member_path(AssetKind::TextArchive, 433),
            g4_kernel::test_support::text_archive_bytes(&["Aus DSPRE"]),
        )
        .unwrap();
        let err = resolve_text_conflicts_impl(
            &state,
            vec![TextChoice {
                archive: 433,
                keep: TextKeep::Json,
            }],
        )
        .unwrap_err();
        assert!(err.message.contains("0433"), "{}", err.message);
        let on_disk = g4_kernel::text::TextArchiveDoc::from_member(
            &std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap(),
            "en_US",
        )
        .unwrap();
        assert_eq!(on_disk.messages[0].text, "Aus DSPRE");
    }

    /// Rewrites `expanded/textArchives/0433.json` with one more message, as a compile that
    /// appended a text literal would.
    fn append_message_to_archive_json(
        fixture: &g4_kernel::test_support::Synthetic,
        project: &Project,
    ) {
        let json = fixture.dir.path().join("expanded/textArchives/0433.json");
        std::fs::create_dir_all(json.parent().unwrap()).unwrap();
        let mut doc = g4_kernel::text::TextArchiveDoc::from_member(
            &std::fs::read(project.member_path(AssetKind::TextArchive, 433)).unwrap(),
            "en_US",
        )
        .unwrap();
        doc.messages.push(g4_kernel::text::TextMessage {
            id: "msg_00003".into(),
            text: "Neu".into(),
            other: Default::default(),
        });
        std::fs::write(&json, doc.to_json()).unwrap();
    }

    fn kernel_warning(file: &str) -> g4_kernel::text::Problem {
        g4_kernel::text::Problem {
            severity: g4_kernel::text::Severity::Warning,
            file: file.into(),
            line: 1,
            col: 1,
            message: "w".into(),
        }
    }

    fn report_touching_433(problems: Vec<g4_kernel::text::Problem>) -> BuildReport {
        BuildReport {
            packed: vec![(AssetKind::Map, 3)],
            rom: "out.nds".into(),
            compiled: vec![0],
            encoded: vec![7],
            touched_archives: vec![433],
            problems,
            steps: Vec::new(),
            ms: 0,
        }
    }

    fn archive_texts(state: &AppState) -> Vec<String> {
        get_text_archive_impl(state, 433)
            .unwrap()
            .messages
            .into_iter()
            .map(|m| m.text)
            .collect()
    }

    fn session_project(state: &AppState) -> Arc<Project> {
        Arc::clone(&lock(state).unwrap().as_ref().unwrap().project)
    }

    #[test]
    fn build_result_maps_problems_by_origin_and_reloads_touched_archives() {
        let (fixture, state) = open_synthetic();
        let project = session_project(&state);
        assert_eq!(archive_texts(&state).len(), 3);
        // A compile changed the JSON behind the store's back.
        append_message_to_archive_json(&fixture, &project);

        let report = report_touching_433(vec![
            kernel_warning("expanded/scripts/0000.rotom"),
            kernel_warning("expanded/textArchives/0007.json"),
        ]);
        let pre_save = ProblemDto::from_kernel(
            &kernel_warning("expanded/textArchives/0433.json"),
            &project.root,
            "text",
        );
        let dto = build_dto(&state, &project, report, 2, vec![pre_save]).unwrap();
        assert_eq!(
            (dto.saved, dto.compiled.clone(), dto.encoded.clone()),
            (2, vec![0], vec![7])
        );
        let sources: Vec<_> = dto.problems.iter().map(|p| p.source).collect();
        assert_eq!(sources, ["text", "compile", "text"]);
        assert_eq!(dto.packed, [("maps".to_owned(), 3)]);
        assert_eq!(
            archive_texts(&state).last().map(String::as_str),
            Some("Neu")
        );
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn build_keeps_a_touched_archive_that_was_edited_meanwhile() {
        let (fixture, state) = open_synthetic();
        let project = session_project(&state);
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 0,
                text: "Meine Änderung".into(),
            },
        )
        .unwrap();
        append_message_to_archive_json(&fixture, &project);

        let dto = build_dto(
            &state,
            &project,
            report_touching_433(Vec::new()),
            0,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(dto.problems.len(), 1, "{:?}", dto.problems);
        let problem = &dto.problems[0];
        assert_eq!(
            (problem.severity, problem.source, problem.file.as_str()),
            ("warning", "text", "expanded/textArchives/0433.json")
        );
        assert_eq!(
            problem.message,
            "Textarchiv 0433 wurde beim Bauen geändert, ist aber ungespeichert bearbeitet — nicht neu geladen"
        );
        // The edit survived: still dirty, still undoable, the appended message not loaded.
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        assert_eq!(
            archive_texts(&state),
            ["Meine Änderung", "Twinleaf Town", "Sandgem Town"]
        );
        assert!(!undo_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn build_does_not_reload_into_a_session_opened_meanwhile() {
        let (fixture_a, state) = open_synthetic();
        let project_a = session_project(&state);
        // Loading the archive unpacks its member, which the JSON below is built from.
        assert_eq!(archive_texts(&state).len(), 3);
        append_message_to_archive_json(&fixture_a, &project_a);

        // Another project replaces the session while A builds.
        let fixture_b = g4_kernel::test_support::synthetic_project();
        open_project_impl(&state, fixture_b.dir.path().to_str().unwrap()).unwrap();
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 0,
                text: "B bearbeitet".into(),
            },
        )
        .unwrap();
        crate::commands::save_impl(&state).unwrap();
        edit_text_archive_impl(
            &state,
            433,
            TextEdit::Set {
                index: 1,
                text: "B ungespeichert".into(),
            },
        )
        .unwrap();
        let dto = build_dto(
            &state,
            &project_a,
            report_touching_433(Vec::new()),
            0,
            Vec::new(),
        )
        .unwrap();
        assert!(dto.problems.is_empty(), "{:?}", dto.problems);
        // B's archive was neither reloaded nor warned about: edit and undo are intact.
        assert_eq!(
            archive_texts(&state)[..2],
            ["B bearbeitet", "B ungespeichert"]
        );
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(archive_texts(&state)[1], "Twinleaf Town");
    }

    fn corrupt_map_member_on_disk(fixture: &g4_kernel::test_support::Synthetic) {
        let path = fixture.dir.path().join("unpacked/maps/0000");
        let mut b = std::fs::read(&path).unwrap();
        b[20] ^= 0xFF;
        std::fs::write(&path, &b).unwrap();
    }

    #[test]
    fn save_stale_returns_structured_error() {
        let (s, state) = open_synthetic();
        set_permission_impl(&state, 0, 1, 1, 0, 0x80).unwrap();
        corrupt_map_member_on_disk(&s);
        let err = save_impl(&state).unwrap_err();
        assert_eq!(err.code, "stale");
        assert_eq!(
            err.asset,
            Some(AssetId {
                kind: AssetKind::Map,
                index: 0
            })
        );
        assert!(
            err.message.starts_with("map 0000 changed on disk"),
            "{}",
            err.message
        );
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn accept_disk_keeps_own_change_and_lets_save_proceed() {
        let (s, state) = open_synthetic();
        set_permission_impl(&state, 0, 1, 1, 2, 0x80).unwrap();
        corrupt_map_member_on_disk(&s);
        assert_eq!(save_impl(&state).unwrap_err().code, "stale");
        accept_disk_impl(&state, AssetKind::Map, 0).unwrap();
        // Own edit is still pending and now saves over the accepted disk state.
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        assert_eq!(save_impl(&state).unwrap().written.len(), 1);
        assert!(dirty_impl(&state).unwrap().is_empty());
        let bytes = std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap();
        let map = MapFile::read(&bytes, g4_formats::game::Family::Pt).unwrap();
        assert_eq!(map.permissions[32 + 1].kind, 2);
    }

    #[test]
    fn run_rom_reports_a_missing_emulator() {
        let err = run_rom_impl("/nonexistent/melonDS", "/tmp/x.nds").unwrap_err();
        assert!(err.message.contains("emulator"), "{}", err.message);
    }

    /// Gated end-to-end check on a copy of a vanilla project (`G4_TEST_PROJECT`): paint a
    /// permission on a map of header 0's matrix, build, and re-open the copy.
    #[test]
    fn open_and_list_survive_a_broken_text_archive() {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("files/msgdata/pl_msg.narc"),
            g4_formats::narc::Narc {
                members: vec![vec![0xAA; 8]],
            }
            .write(),
        )
        .unwrap();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(info.header_count, 2);
        // Opening never touches the text archive.
        assert!(!fixture.dir.path().join("unpacked/textArchives").exists());
        let listing = list_headers_impl(&state).unwrap();
        assert_eq!(listing.headers.len(), 2);
        assert!(listing.headers.iter().all(|h| h.location_name.is_empty()));
        assert!(listing.names_error.is_some_and(|e| e.contains("433")));
        assert!(get_map_impl(&state, 0).is_ok());
    }

    #[test]
    fn vanilla_paint_save_build_on_copy() {
        let Some((tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let root = root.to_str().unwrap();
        let state = AppState::default();
        open_project_impl(&state, root).unwrap();
        let headers = list_headers_impl(&state).unwrap().headers;
        let matrix = get_matrix_impl(&state, headers[0].matrix_id).unwrap();
        let map_id = matrix
            .maps
            .iter()
            .flatten()
            .next()
            .copied()
            .expect("matrix has a map");
        let (x, y) = (3u8, 3u8);
        let at = usize::from(y) * 32 + usize::from(x);
        let [kind, collision] = get_map_impl(&state, map_id).unwrap().permissions[at];
        let flipped = if collision == 0x80 { 0x00 } else { 0x80 };
        set_permission_impl(&state, map_id, x, y, kind, flipped).unwrap();
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);

        let out = tmp.path().join("out.nds");
        let report = build_rom_impl(&state, out.to_str().unwrap(), false).unwrap();
        assert_eq!(report.saved, 1);
        assert!(report.packed.iter().any(|(k, n)| k == "maps" && *n > 0));
        assert!(std::fs::metadata(&out).unwrap().len() > 1_000_000);
        assert_eq!(report.rom, out.display().to_string());
        assert!(dirty_impl(&state).unwrap().is_empty());

        let reopened = AppState::default();
        open_project_impl(&reopened, root).unwrap();
        assert_eq!(
            get_map_impl(&reopened, map_id).unwrap().permissions[at],
            [kind, flipped]
        );
    }
}
