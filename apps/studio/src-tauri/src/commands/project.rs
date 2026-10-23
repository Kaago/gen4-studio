//! Opening, saving, undo, build and run.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use g4_kernel::build::{BuildReport, build_prepared, prepare_for_build};
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::run::launch_emulator;
use g4_kernel::sprites::SpriteCache;
use g4_kernel::store::Store;
use serde::Serialize;
use tauri::{Manager, State};

use super::{ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

#[derive(Serialize)]
pub struct ProjectInfo {
    pub root: String,
    pub game_code: String,
    pub family: String,
    pub language: String,
    pub header_count: usize,
}

pub fn open_project_impl(state: &AppState, path: &str) -> Result<ProjectInfo, CmdError> {
    let project = Project::open(path).map_err(CmdError::from)?;
    // Only the header table: location names are cosmetic and must not gate opening.
    let header_count = project.header_count().map_err(CmdError::from)?;
    let info = ProjectInfo {
        root: project.root.display().to_string(),
        game_code: project.game_code.clone(),
        family: format!("{:?}", project.family),
        language: format!("{:?}", project.language),
        header_count,
    };
    let project = Arc::new(project);
    let store = Store::new(Arc::clone(&project));
    let old = lock(state)?.replace(Session {
        project,
        store,
        sprites: SpriteCache::new(),
        lsp: None,
    });
    // The old project's language server ends with its session, outside the state lock.
    drop(old);
    Ok(info)
}

/// What `build_rom` did: how many dirty assets were saved first, the packed asset kinds (by
/// unpacked folder name, e.g. `maps`, `eventFiles`) with their member counts, the ROM path,
/// the script files compiled and the text archives encoded before packing, and the warnings
/// of the pre-build save and of those two steps (`source` tells them apart).
#[derive(Debug, Serialize)]
pub struct BuildDto {
    pub saved: usize,
    pub packed: Vec<(String, usize)>,
    pub rom: String,
    pub compiled: Vec<u16>,
    pub encoded: Vec<u16>,
    pub problems: Vec<ProblemDto>,
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

/// Reverts the latest change and names the asset it restored (`None`: nothing to undo), so
/// the UI reloads exactly what that asset affects.
pub fn undo_impl(state: &AppState) -> Result<Option<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.undo())
}

pub fn redo_impl(state: &AppState) -> Result<Option<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.redo())
}

pub fn dirty_impl(state: &AppState) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.dirty())
}

/// What `save` did: the assets written and the warnings found while encoding them (text
/// warnings never block the save).
#[derive(Debug, Serialize)]
pub struct SaveDto {
    pub written: Vec<AssetId>,
    pub problems: Vec<ProblemDto>,
}

pub fn save_impl(state: &AppState) -> Result<SaveDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let report = session.store.save()?;
    let root = &session.project.root;
    Ok(SaveDto {
        written: report.written,
        problems: report
            .problems
            .iter()
            .map(|p| ProblemDto::from_kernel(p, root, "text"))
            .collect(),
    })
}

/// Drops the own changes (and their undo/redo entries) of one asset and reloads it from its
/// file on disk.
pub fn reload_asset_impl(state: &AppState, kind: AssetKind, index: u16) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.reload_from_disk(AssetId { kind, index })?;
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

/// Saves dirty assets (a stale file aborts with nothing built), then builds the ROM. The
/// state lock is held for the save, for compiling scripts and encoding texts (they write
/// sources, members and the compile state, which the script and text commands write too, see
/// `g4_kernel::build::prepare`) and for the reload afterwards, not for packing and the ds-rom
/// build. An error in the compile step is `CmdError { code: "compile" }` and nothing is
/// packed. Text archives the compile appended messages to are loaded again in the store.
pub fn build_rom_impl(state: &AppState, out: &str) -> Result<BuildDto, CmdError> {
    let out = Path::new(out);
    let (project, prepared, saved, problems) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
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
    })
}

pub fn run_rom_impl(emulator: &str, rom: &str) -> Result<u32, CmdError> {
    launch_emulator(Path::new(emulator), Path::new(rom)).map_err(CmdError::from)
}

#[tauri::command]
pub fn open_project(state: State<AppState>, path: String) -> Result<ProjectInfo, CmdError> {
    open_project_impl(&state, &path)
}

#[tauri::command]
pub fn member_counts(state: State<AppState>) -> Result<HashMap<AssetKind, usize>, CmdError> {
    member_counts_impl(&state)
}

#[tauri::command]
pub fn undo(state: State<AppState>) -> Result<Option<AssetId>, CmdError> {
    undo_impl(&state)
}

#[tauri::command]
pub fn redo(state: State<AppState>) -> Result<Option<AssetId>, CmdError> {
    redo_impl(&state)
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
/// without holding the state lock.
#[tauri::command]
pub async fn build_rom(app: tauri::AppHandle, out: String) -> Result<BuildDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || build_rom_impl(&app.state::<AppState>(), &out))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
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
        assert_eq!(counts[&AssetKind::TextArchive], 434);
        assert_eq!(counts[&AssetKind::HeaderTable], 1);
        assert_eq!(counts[&AssetKind::Script], 3);
        assert!(!counts.contains_key(&AssetKind::DynamicHeaders));
        // Serialized for the frontend as an object keyed by the kind name.
        let json = serde_json::to_value(&counts).unwrap();
        assert_eq!(json["AreaData"], 2);
        assert!(member_counts_impl(&AppState::default()).is_err());
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
        assert!(build_rom_impl(&state, out.to_str().unwrap()).is_err());
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
        let err = build_rom_impl(&state, out.to_str().unwrap()).unwrap_err();
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
        let err = build_rom_impl(&state, out.to_str().unwrap()).unwrap_err();
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
        assert!(undo_impl(&state).unwrap().is_some());
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
        assert!(undo_impl(&state).unwrap().is_some());
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
        let report = build_rom_impl(&state, out.to_str().unwrap()).unwrap();
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
