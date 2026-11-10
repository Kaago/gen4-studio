//! Script files: adopting a project into rotom, reading and saving sources (a save compiles),
//! creating files and scripts, resolving a script number to its file, and searching.

use std::sync::Arc;

use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::scripts::{
    self, AdoptProgress, CompileReport, LevelScriptDoc, ScriptSource, ScriptTarget, SearchHit,
    SourceKind,
};
use g4_kernel::store::Store;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{Manager, State};

use super::{ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ScriptsStatusDto {
    /// rotom has taken the project over (`rotom.toml` and the sources exist).
    pub adopted: bool,
    pub file_count: u16,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AdoptProgressDto {
    pub done: u32,
    pub total: u32,
}

impl From<AdoptProgress> for AdoptProgressDto {
    fn from(p: AdoptProgress) -> Self {
        AdoptProgressDto {
            done: p.done,
            total: p.total,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdoptReportDto {
    pub decompiled: u32,
    pub skipped_empty: u32,
    /// Script files rotom could not decompile with its message; they stay binary.
    pub failed: Vec<(u16, String)>,
}

#[derive(Debug, Serialize)]
pub struct ScriptFileDto {
    pub file: u16,
    /// `"script"`, `"levelScript"` or `"binary"`.
    pub kind: &'static str,
    pub size: u64,
    /// Ids of the map headers that use this file as their script file (or, for a level
    /// script, their level script; a binary file counts for both).
    pub headers: Vec<u16>,
}

/// A script file as the editor opens it. `kind` is `"script"`, `"levelScript"` or `"binary"`,
/// like [`ScriptFileDto::kind`].
#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum ScriptSourceDto {
    #[serde(rename = "script")]
    Rotom { text: String },
    #[serde(rename = "levelScript")]
    LevelScript { doc: LevelScriptDoc },
    #[serde(rename = "binary")]
    Binary { size: u64 },
}

#[derive(Debug, Serialize)]
pub struct SaveScriptDto {
    /// Every problem the compile reported (also those of other sources that were stale).
    pub problems: Vec<ProblemDto>,
    /// Text archives a text literal in a script appended to; the store holds them again.
    pub touched_archives: Vec<u16>,
}

/// Where a script lives. `kind` is the tag: `"file"`, `"ambiguous"` or `"unknown"`.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ScriptTargetDto {
    File { file: u16, line: u32 },
    Ambiguous { candidates: Vec<u16> },
    Unknown,
}

pub fn scripts_status_impl(state: &AppState) -> Result<ScriptsStatusDto, CmdError> {
    let mut guard = lock(state)?;
    let project = &session(&mut guard)?.project;
    let count = project.member_count(AssetKind::Script)?;
    Ok(ScriptsStatusDto {
        adopted: scripts::is_adopted(project),
        file_count: u16::try_from(count).unwrap_or(u16::MAX),
    })
}

/// Holds the state lock for the whole adoption: rotom writes the sources, the compile state
/// and the project config, and nothing else may touch the project meanwhile.
pub fn adopt_scripts_impl(
    state: &AppState,
    progress: &mut dyn FnMut(AdoptProgressDto),
) -> Result<AdoptReportDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let adopted = scripts::adopt(&session.project, &mut |p| progress(p.into()));
    // Also a failed adoption may have written sources: past the history, so the file steps
    // of the scripts (resets to the original) could no longer be undone.
    session.files_written(&[]);
    session.store.forget_kind_history(AssetKind::Script);
    let report = adopted?;
    Ok(AdoptReportDto {
        decompiled: report.decompiled,
        skipped_empty: report.skipped_empty,
        failed: report.failed,
    })
}

pub fn list_script_files_impl(state: &AppState) -> Result<Vec<ScriptFileDto>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let files = scripts::list_files(&session.project)?;
    let headers = session.store.header_table()?;
    Ok(files
        .into_iter()
        .map(|info| {
            let users = |pick: fn(&g4_kernel::store::MapHeader) -> u16| {
                headers
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| pick(h) == info.file)
                    // The table holds at most 65536 headers (checked when it is read).
                    .map(|(id, _)| id as u16)
                    .collect::<Vec<_>>()
            };
            let (kind, headers) = match info.kind {
                SourceKind::Script => ("script", users(|h| h.script_file_id())),
                SourceKind::LevelScript => ("levelScript", users(|h| h.level_script_id())),
                SourceKind::Binary => {
                    let mut both = users(|h| h.script_file_id());
                    both.extend(users(|h| h.level_script_id()));
                    both.sort_unstable();
                    both.dedup();
                    ("binary", both)
                }
            };
            ScriptFileDto {
                file: info.file,
                kind,
                size: info.size,
                headers,
            }
        })
        .collect())
}

pub fn read_script_impl(state: &AppState, file: u16) -> Result<ScriptSourceDto, CmdError> {
    let mut guard = lock(state)?;
    let project = &session(&mut guard)?.project;
    Ok(match scripts::read_source(project, file)? {
        ScriptSource::Rotom(text) => ScriptSourceDto::Rotom { text },
        ScriptSource::LevelScript(doc) => ScriptSourceDto::LevelScript { doc },
        ScriptSource::Binary { size } => ScriptSourceDto::Binary { size },
    })
}

/// A compile rewrites the JSON of the text archives it touches, so an unsaved edit of one
/// would be lost to the reload that follows: the first dirty archive refuses the command.
fn refuse_dirty_text(store: &Store) -> Result<(), CmdError> {
    match store
        .dirty()
        .into_iter()
        .find(|id| id.kind == AssetKind::TextArchive)
    {
        Some(id) => Err(format!("Textarchiv {:04} erst speichern", id.index).into()),
        None => Ok(()),
    }
}

/// Loads the text archives a compile changed from disk again (the store's copy, its search
/// index entry and its undo steps go), so the Text panel shows what rotom appended.
pub(super) fn reload_text_archives(store: &mut Store, archives: &[u16]) -> Result<(), CmdError> {
    for &index in archives {
        store.reload_from_disk(AssetId {
            kind: AssetKind::TextArchive,
            index,
        })?;
    }
    Ok(())
}

fn save_script_dto(
    store: &mut Store,
    project: &Project,
    report: CompileReport,
) -> Result<SaveScriptDto, CmdError> {
    reload_text_archives(store, &report.touched_archives)?;
    Ok(SaveScriptDto {
        problems: report
            .problems
            .iter()
            .map(|p| ProblemDto::by_origin(p, &project.root))
            .collect(),
        touched_archives: report.touched_archives,
    })
}

/// Writes the source, then compiles everything that is stale. A source that does not
/// compile is saved anyway and comes back as a problem. Refuses with a dirty text archive.
pub fn save_script_impl(
    state: &AppState,
    file: u16,
    text: String,
) -> Result<SaveScriptDto, CmdError> {
    save_source(state, file, ScriptSource::Rotom(text))
}

pub fn save_level_script_impl(
    state: &AppState,
    file: u16,
    doc: LevelScriptDoc,
) -> Result<SaveScriptDto, CmdError> {
    save_source(state, file, ScriptSource::LevelScript(doc))
}

fn save_source(
    state: &AppState,
    file: u16,
    source: ScriptSource,
) -> Result<SaveScriptDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    refuse_dirty_text(&session.store)?;
    let written = scripts::write_source(&session.project, file, &source);
    session.files_written(&[]);
    session.scripts_written();
    // The file is written past the history: a file step of it (a reset to the original)
    // could no longer be undone, and would block every older step.
    session.store.forget_asset_history(AssetId {
        kind: AssetKind::Script,
        index: file,
    });
    written?;
    let report = scripts::compile(&session.project)?;
    save_script_dto(&mut session.store, &session.project, report)
}

/// `kind` is `"script"` or `"levelScript"`. Same refusal as a save: the compile that makes
/// the member may touch text archives.
pub fn new_script_file_impl(state: &AppState, kind: String) -> Result<u16, CmdError> {
    let kind = match kind.as_str() {
        "script" => SourceKind::Script,
        "levelScript" => SourceKind::LevelScript,
        other => return Err(format!("unknown script file kind {other:?}").into()),
    };
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    refuse_dirty_text(&session.store)?;
    let created = scripts::new_script_file_reported(&session.project, kind);
    session.files_written(&[]);
    session.scripts_written();
    let (file, touched) = created?;
    reload_text_archives(&mut session.store, &touched)?;
    Ok(file)
}

/// Appends an empty script to script file `file` and returns its slot number.
pub fn add_script_impl(state: &AppState, file: u16) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    refuse_dirty_text(&session.store)?;
    let added = scripts::add_script_reported(&session.project, file);
    session.files_written(&[]);
    session.scripts_written();
    // Written past the history, as a save (`save_source`).
    session.store.forget_asset_history(AssetId {
        kind: AssetKind::Script,
        index: file,
    });
    let (n, touched) = added?;
    reload_text_archives(&mut session.store, &touched)?;
    Ok(n)
}

pub fn resolve_script_impl(
    state: &AppState,
    header: u16,
    n: u16,
) -> Result<ScriptTargetDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let headers = session.store.header_table()?;
    Ok(
        match scripts::resolve_script(&project, headers, header, n)? {
            ScriptTarget::File { file, line } => ScriptTargetDto::File { file, line },
            ScriptTarget::Ambiguous(candidates) => ScriptTargetDto::Ambiguous { candidates },
            ScriptTarget::Unknown => ScriptTargetDto::Unknown,
        },
    )
}

pub fn search_scripts_impl(
    state: &AppState,
    query: String,
    limit: usize,
) -> Result<Vec<SearchHit>, CmdError> {
    let mut guard = lock(state)?;
    let project = &session(&mut guard)?.project;
    Ok(scripts::search_sources(project, &query, limit)?)
}

#[tauri::command]
pub fn scripts_status(state: State<AppState>) -> Result<ScriptsStatusDto, CmdError> {
    scripts_status_impl(&state)
}

/// rotom decompiles every script file, which takes seconds: it runs on a blocking worker
/// (like `build_rom`) and sends its progress over `progress`. The state lock is held until it
/// is done.
#[tauri::command]
pub async fn adopt_scripts(
    app: tauri::AppHandle,
    progress: Channel<AdoptProgressDto>,
) -> Result<AdoptReportDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        adopt_scripts_impl(&app.state::<AppState>(), &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[tauri::command]
pub fn list_script_files(state: State<AppState>) -> Result<Vec<ScriptFileDto>, CmdError> {
    list_script_files_impl(&state)
}

#[tauri::command]
pub fn read_script(state: State<AppState>, file: u16) -> Result<ScriptSourceDto, CmdError> {
    read_script_impl(&state, file)
}

#[tauri::command]
pub fn save_script(
    state: State<AppState>,
    file: u16,
    text: String,
) -> Result<SaveScriptDto, CmdError> {
    save_script_impl(&state, file, text)
}

#[tauri::command]
pub fn save_level_script(
    state: State<AppState>,
    file: u16,
    doc: LevelScriptDoc,
) -> Result<SaveScriptDto, CmdError> {
    save_level_script_impl(&state, file, doc)
}

#[tauri::command]
pub fn new_script_file(state: State<AppState>, kind: String) -> Result<u16, CmdError> {
    new_script_file_impl(&state, kind)
}

#[tauri::command]
pub fn add_script(state: State<AppState>, file: u16) -> Result<u16, CmdError> {
    add_script_impl(&state, file)
}

#[tauri::command]
pub fn resolve_script(
    state: State<AppState>,
    header: u16,
    n: u16,
) -> Result<ScriptTargetDto, CmdError> {
    resolve_script_impl(&state, header, n)
}

#[tauri::command]
pub fn search_scripts(
    state: State<AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<SearchHit>, CmdError> {
    search_scripts_impl(&state, query, limit)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{
        TextEdit, dirty_impl, edit_text_archive_impl, get_header_impl, get_text_archive_impl,
        open_synthetic, save_impl, search_text_impl, set_header_impl,
    };
    use super::*;

    const ARCHIVE: u16 = 433;

    fn adopted() -> (g4_kernel::test_support::Synthetic, AppState) {
        let (fixture, state) = open_synthetic();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        (fixture, state)
    }

    fn texts(state: &AppState, index: u16) -> Vec<String> {
        get_text_archive_impl(state, index)
            .unwrap()
            .messages
            .into_iter()
            .map(|m| m.text)
            .collect()
    }

    #[test]
    fn status_before_and_after_adopt() {
        let (_f, state) = open_synthetic();
        let before = scripts_status_impl(&state).unwrap();
        assert!(!before.adopted);
        assert_eq!(before.file_count, 3);

        let mut events = Vec::new();
        let report = adopt_scripts_impl(&state, &mut |p| events.push(p)).unwrap();
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(report.decompiled + report.skipped_empty, 3);
        assert_eq!(events.first().map(|p| p.done), Some(0));
        assert_eq!(
            events.last().map(|p| p.done),
            events.last().map(|p| p.total)
        );

        let after = scripts_status_impl(&state).unwrap();
        assert!(after.adopted);
        assert_eq!(after.file_count, 3);
        // A second adoption is refused, nothing is overwritten.
        assert!(adopt_scripts_impl(&state, &mut |_| {}).is_err());
        assert!(scripts_status_impl(&AppState::default()).is_err());
    }

    #[test]
    fn list_and_read_script_files() {
        let (_f, state) = adopted();
        let files = list_script_files_impl(&state).unwrap();
        let kinds: Vec<_> = files.iter().map(|f| (f.file, f.kind)).collect();
        assert_eq!(kinds, [(0, "script"), (1, "levelScript"), (2, "binary")]);
        // The fixture's two headers share script file 0; only header 0 has level script 1.
        assert_eq!(files[0].headers, [0, 1]);
        assert_eq!(files[1].headers, [0]);
        assert!(files[2].headers.is_empty());

        let ScriptSourceDto::Rotom { text } = read_script_impl(&state, 0).unwrap() else {
            panic!("file 0 is a script");
        };
        assert!(text.contains("script"), "{text}");
        let json = serde_json::to_value(read_script_impl(&state, 1).unwrap()).unwrap();
        assert_eq!(json["kind"], "levelScript");
        assert!(json["doc"].is_object(), "{json}");
        let binary = serde_json::to_value(read_script_impl(&state, 2).unwrap()).unwrap();
        assert_eq!(binary, serde_json::json!({ "kind": "binary", "size": 0 }));
    }

    #[test]
    fn save_script_compiles_and_reports_problems() {
        let (f, state) = adopted();
        let ok = save_script_impl(&state, 0, "script Main #1:\n    End\n".into()).unwrap();
        assert!(ok.problems.is_empty(), "{:?}", ok.problems);
        assert!(ok.touched_archives.is_empty());

        // A source that does not compile is saved, comes back as a problem, and the
        // member stays as it was.
        let member = f.dir.path().join("unpacked/scripts/0000");
        let before = std::fs::read(&member).unwrap();
        let bad = "script Main #1:\n    NoSuch\n    End\n";
        let report = save_script_impl(&state, 0, bad.into()).unwrap();
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert_eq!(
            (problem.severity, problem.source, problem.file.as_str()),
            ("error", "compile", "expanded/scripts/0000.rotom")
        );
        assert_eq!((problem.line, problem.col), (2, 5));
        assert!(problem.message.contains("NoSuch"), "{}", problem.message);
        assert_eq!(std::fs::read(&member).unwrap(), before);
        let ScriptSourceDto::Rotom { text } = read_script_impl(&state, 0).unwrap() else {
            panic!("file 0 is a script");
        };
        assert_eq!(text, bad);
    }

    #[test]
    fn save_script_refuses_with_dirty_text_archive() {
        let (_f, state) = adopted();
        let ScriptSourceDto::Rotom { text: original } = read_script_impl(&state, 0).unwrap() else {
            panic!("file 0 is a script");
        };
        edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 0,
                text: "Ungespeichert".into(),
            },
            None,
        )
        .unwrap();
        let refusals = [
            save_script_impl(&state, 0, "script Main #1:\n    End\n".into()).map(drop),
            save_level_script_impl(&state, 1, LevelScriptDoc::default()).map(drop),
            new_script_file_impl(&state, "script".into()).map(drop),
            add_script_impl(&state, 0).map(drop),
        ];
        for refusal in refusals {
            let err = refusal.unwrap_err();
            assert_eq!(err.code, "error");
            assert_eq!(err.message, "Textarchiv 0433 erst speichern");
        }
        // Nothing was written or compiled, and the edit is still pending.
        let ScriptSourceDto::Rotom { text } = read_script_impl(&state, 0).unwrap() else {
            panic!("file 0 is a script");
        };
        assert_eq!(text, original);
        assert_eq!(list_script_files_impl(&state).unwrap().len(), 3);
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
    }

    #[test]
    fn save_script_with_literal_reloads_archive() {
        let (_f, state) = adopted();
        // Load the archive and its search index first, so the reload has something to drop.
        assert!(!texts(&state, ARCHIVE).contains(&"Hallo Welt".to_owned()));
        assert!(
            search_text_impl(&state, "Hallo".into(), 5)
                .unwrap()
                .is_empty()
        );
        assert!(dirty_impl(&state).unwrap().is_empty());

        let source = "script Main #1:\n    Message \"Hallo Welt\"\n    End\n";
        let report = save_script_impl(&state, 0, source.into()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.touched_archives, [ARCHIVE]);
        assert!(texts(&state, ARCHIVE).contains(&"Hallo Welt".to_owned()));
        let hits = search_text_impl(&state, "Hallo".into(), 5).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].archive, ARCHIVE);
        // The reloaded archive matches its file: nothing to save.
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn a_header_save_compiles_the_script_whose_literal_did_not_resolve() {
        use g4_kernel::store::MapHeader;

        let (_f, state) = adopted();
        let file = new_script_file_impl(&state, "script".into()).unwrap();
        let source = "script Main #1:\n    Message \"Hallo Welt\"\n    End\n";
        let report = save_script_impl(&state, file, source.into()).unwrap();
        let [problem] = report.problems.as_slice() else {
            panic!("{:?}", report.problems);
        };
        assert!(
            problem
                .message
                .starts_with("Text literal cannot be resolved"),
            "{problem:?}"
        );

        // Header 1 gets the new script file. Saving the header table compiles the source
        // again (nothing else changed it), so the problem goes without a second script save.
        let mut header = get_header_impl(&state, 1).unwrap();
        if let MapHeader::Pt(h) = &mut header {
            h.script_file_id = file;
            h.text_archive_id = ARCHIVE;
        }
        set_header_impl(&state, 1, header, "script file").unwrap();
        let saved = save_impl(&state).unwrap();
        assert!(saved.problems.is_empty(), "{:?}", saved.problems);
        assert_eq!(
            saved.compiled,
            [format!("expanded/scripts/{file:04}.rotom")]
        );
        assert_eq!(saved.touched_archives, [ARCHIVE]);
        assert!(texts(&state, ARCHIVE).contains(&"Hallo Welt".to_owned()));
        assert!(dirty_impl(&state).unwrap().is_empty());

        // A save without the header table compiles nothing.
        edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 0,
                text: "Anders".into(),
            },
            None,
        )
        .unwrap();
        let text_only = save_impl(&state).unwrap();
        assert!(text_only.compiled.is_empty());
        assert!(text_only.touched_archives.is_empty());
    }

    #[test]
    fn save_level_script_round_trip() {
        let (f, state) = adopted();
        let ScriptSourceDto::LevelScript { doc } = read_script_impl(&state, 1).unwrap() else {
            panic!("file 1 is a level script");
        };
        let mut edited = serde_json::to_value(&doc).unwrap();
        edited["header_entries"] = serde_json::json!([{ "type": "on_load", "script_id": 1 }]);
        let edited: LevelScriptDoc = serde_json::from_value(edited).unwrap();
        let report = save_level_script_impl(&state, 1, edited.clone()).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let ScriptSourceDto::LevelScript { doc: read } = read_script_impl(&state, 1).unwrap()
        else {
            panic!("file 1 is a level script");
        };
        assert_eq!(read, edited);
        // The member holds the compiled form, no longer the four zero bytes.
        let member = f.dir.path().join("unpacked/scripts/0001");
        assert_ne!(std::fs::read(member).unwrap(), [0u8; 4]);
    }

    #[test]
    fn save_level_script_that_rotom_rejects_is_saved_and_reported_as_problems() {
        let (f, state) = adopted();
        let mut doc = serde_json::json!({
            "header_entries": [{ "type": "on_var_condition" }],
            "var_conditions": [{ "var": 0, "value": 1, "script_id": 5 }],
        });
        let invalid: LevelScriptDoc = serde_json::from_value(doc.take()).unwrap();
        // A save is Ok with problems (like a script that does not compile), not a `compile` CmdError.
        let report = save_level_script_impl(&state, 1, invalid.clone()).unwrap();
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert_eq!(report.problems[0].file, "expanded/scripts/0001.json");
        // The source is saved anyway; the member keeps its old bytes.
        let ScriptSourceDto::LevelScript { doc: read } = read_script_impl(&state, 1).unwrap()
        else {
            panic!("file 1 is a level script");
        };
        assert_eq!(read, invalid);
        assert_eq!(
            std::fs::read(f.dir.path().join("unpacked/scripts/0001")).unwrap(),
            [0u8; 4]
        );
    }

    #[test]
    fn new_script_file_and_add_script() {
        let (f, state) = adopted();
        let script = new_script_file_impl(&state, "script".into()).unwrap();
        let level = new_script_file_impl(&state, "levelScript".into()).unwrap();
        assert_eq!((script, level), (3, 4));
        assert!(f.dir.path().join("unpacked/scripts/0003").is_file());
        assert!(f.dir.path().join("expanded/scripts/0004.json").is_file());
        assert_eq!(add_script_impl(&state, script).unwrap(), 2);
        assert!(add_script_impl(&state, level).is_err());
        assert!(new_script_file_impl(&state, "binary".into()).is_err());
        assert_eq!(list_script_files_impl(&state).unwrap().len(), 5);
    }

    /// The index stats the script sources at most every 250 ms; a save stats them at once, so
    /// a query right after it sees the new source (R10).
    #[test]
    fn a_saved_source_shows_up_in_refs_at_once() {
        use g4_kernel::flags::FlagKind;
        let (_f, state) = adopted();
        let refs = |state: &AppState| {
            super::super::flag_usage_impl(state, FlagKind::Flag, 0x173)
                .unwrap()
                .refs
                .len()
        };
        // Builds the index and stats the sources now.
        assert_eq!(refs(&state), 0);
        let text = "script script_1 #1:\n    SetFlag 0x173\n    End\n";
        save_script_impl(&state, 0, text.into()).unwrap();
        assert_eq!(refs(&state), 1);
        // A level script the same way.
        let doc: LevelScriptDoc = serde_json::from_value(serde_json::json!({
            "header_entries": [{ "type": "on_var_condition" }],
            "var_conditions": [{ "var": 0x40A5, "value": 1, "script_id": 1 }],
        }))
        .unwrap();
        save_level_script_impl(&state, 1, doc).unwrap();
        let var = super::super::flag_usage_impl(&state, FlagKind::Var, 0x40A5).unwrap();
        assert_eq!(var.refs.len(), 1);
    }

    #[test]
    fn resolve_script_dto() {
        let (_f, state) = adopted();
        // Header 0 uses script file 0; the fixture's source has `script … #1` on line 1.
        let target = resolve_script_impl(&state, 0, 1).unwrap();
        assert_eq!(target, ScriptTargetDto::File { file: 0, line: 1 });
        assert_eq!(
            serde_json::to_value(&target).unwrap(),
            serde_json::json!({ "kind": "file", "file": 0, "line": 1 })
        );
        assert_eq!(
            resolve_script_impl(&state, 99, 1).unwrap(),
            ScriptTargetDto::Unknown
        );
        assert_eq!(
            serde_json::to_value(ScriptTargetDto::Ambiguous {
                candidates: vec![1, 2]
            })
            .unwrap(),
            serde_json::json!({ "kind": "ambiguous", "candidates": [1, 2] })
        );
    }

    #[test]
    fn search_scripts_finds_lines() {
        let (_f, state) = adopted();
        save_script_impl(
            &state,
            0,
            "script Main #1:\n    Message 1\n    End\n".into(),
        )
        .unwrap();
        let hits = search_scripts_impl(&state, "message".into(), 10).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!((hits[0].file, hits[0].line), (0, 2));
        let json = serde_json::to_value(&hits[0]).unwrap();
        assert_eq!(json["file"], 0);
        assert!(
            search_scripts_impl(&state, "message".into(), 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn commands_fail_without_an_open_project() {
        let state = AppState::default();
        assert!(list_script_files_impl(&state).is_err());
        assert!(read_script_impl(&state, 0).is_err());
        assert!(save_script_impl(&state, 0, String::new()).is_err());
        assert!(resolve_script_impl(&state, 0, 1).is_err());
    }
}
