//! The text project as the source of the open project (4.3b): its status, setting a DSPRE
//! folder up as text project and taking it back, lifting an older schema, the sync with what
//! DSPRE changed (taken over by the app, conflicts with their diff and the user's choice),
//! „ID umbenennen“ and the ids of a kind.
//!
//! Locks: the session lock is always taken before the kernel's sync lock of the project root
//! (a scan, a takeover, a choice and the undo of a rename take that one inside). A scan runs
//! without the session lock; its progress goes to the caller (a Tauri channel) and never
//! takes the session lock.

use std::collections::BTreeMap;
use std::sync::Arc;

use g4_formats::game::Family;
use g4_kernel::base::BaseRom;
use g4_kernel::build::state::BuildState;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, Project, SourceMode};
use g4_kernel::textproject::adopt::{
    self, AdoptProgress, AdoptReport, REVERT_PENDING, RevertReport,
};
use g4_kernel::textproject::migrate::{self, MigrateReport};
use g4_kernel::textproject::rename::{self, NOT_A_TEXT_PROJECT, RenamePlan, RenameResult};
use g4_kernel::textproject::symbols::SymbolKind;
use g4_kernel::textproject::sync::{
    self, ConflictDiff, Keep, ScanReport, SyncOutcome, SyncProgress, UnitSide,
};
use g4_kernel::textproject::{NO_DP, SCHEMA_VERSION};
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;

use super::{found_base, lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The refusal of the sync commands outside a text project.
pub const NOT_TEXT_PROJECT: &str =
    "Kein Textprojekt: der Abgleich mit DSPRE beginnt erst nach „Als Textprojekt einrichten…“.";

/// Why the store refuses writes while the project is set up as text project (code `busy`).
pub const ADOPTING: &str = "Das Projekt wird gerade als Textprojekt eingerichtet.";

/// The refusal of „Als Textprojekt einrichten“ on a text project (UI test 4.3b U2), code
/// `error`.
pub const ALREADY_TEXT_PROJECT: &str = "Das Projekt ist schon ein Textprojekt.";

/// The refusal of a build while the sync has open conflicts.
pub const CONFLICTS: &str = "Erst die Konflikte in Technik › Textprojekt lösen.";

/// What the project is as a source, as `open_project` and `text_project_status` answer it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TextProjectStatusDto {
    /// A DSPRE folder; `adoptable` when „Als Textprojekt einrichten…“ can run, else `reason`
    /// says why not (Diamond/Pearl, no base mark, the base ROM is not found).
    Dspre {
        adoptable: bool,
        reason: Option<String>,
    },
    /// A text project of the current schema; `last_sync_at` when the sync last recorded
    /// (Unix milliseconds), `None` before the first.
    Current {
        schema: u32,
        last_sync_at: Option<i64>,
    },
    /// A text project of an older schema: read only until it is lifted
    /// (`migrate_text_project`).
    Older { schema: u32 },
    /// A text project of a newer Studio: not opened.
    Newer { schema: u32 },
    /// A manifest that does not read: not opened; `reason` names file, line and column.
    Broken { reason: String },
}

/// What a sync of the project's files with its store is: what [`external_changes_impl`] found
/// (the units that are not in step, see `textproject::sync::scan`) and, when asked for, the
/// diff of each conflict by its key.
#[derive(Debug, Serialize)]
pub struct ExternalChangesDto {
    pub scan: ScanReport,
    pub diffs: BTreeMap<String, ConflictDiff>,
}

/// The status of `project` (`state` finds the base ROM of a DSPRE folder; that may hash it
/// once).
pub fn status_of(state: &AppState, project: &Project) -> TextProjectStatusDto {
    match project.source_mode() {
        SourceMode::Dspre => {
            let reason = if project.family == Family::Dp {
                Some(NO_DP.to_owned())
            } else {
                found_base(state, &project.root).err().map(|e| e.message)
            };
            TextProjectStatusDto::Dspre {
                adoptable: reason.is_none(),
                reason,
            }
        }
        SourceMode::TextProject => TextProjectStatusDto::Current {
            schema: SCHEMA_VERSION,
            last_sync_at: BuildState::load(&project.root)
                .sync
                .and_then(|sync| i64::try_from(sync.scanned_at_ns / 1_000_000).ok()),
        },
        SourceMode::Older { schema } => TextProjectStatusDto::Older { schema: *schema },
        SourceMode::Newer { schema } => TextProjectStatusDto::Newer { schema: *schema },
        SourceMode::Broken { reason } => TextProjectStatusDto::Broken {
            reason: reason.clone(),
        },
    }
}

/// `project`, when it is a text project of the current schema: `not_text_project` for a
/// DSPRE folder (with `message`), the refusal of its schema for any other.
fn text_project(project: &Project, message: &str) -> Result<(), CmdError> {
    match project.source_mode() {
        SourceMode::TextProject => Ok(()),
        SourceMode::Dspre => Err(CmdError::refused("not_text_project", message)),
        _ => Err(project.editable().err().map_or_else(
            || CmdError::refused("not_text_project", message),
            CmdError::from,
        )),
    }
}

/// A kernel error of the text project's commands as the app answers it: a refusal of a
/// pending change (`REVERT_PENDING`) is `pending`, the refusal outside a text project
/// `not_text_project`; anything else as everywhere.
fn text_error(e: KernelError) -> CmdError {
    match e {
        KernelError::Refused(why) if why == REVERT_PENDING => CmdError::refused("pending", why),
        KernelError::Refused(why) if why == NOT_A_TEXT_PROJECT => {
            CmdError::refused("not_text_project", why)
        }
        // The adoption of a text project exports it: changes the members lack refuse it.
        KernelError::Refused(why) if why.starts_with(g4_kernel::textproject::UNSYNCED_EXPORT) => {
            CmdError::refused("unsynced", why)
        }
        other => other.into(),
    }
}

/// The message when a session cannot be replaced after an adoption, a revert or a migration
/// because it was changed meanwhile.
pub const REOPEN: &str = "Die Dateien sind umgestellt, aber die offene Sitzung wurde \
                          inzwischen geändert und bleibt, wie sie ist; das Projekt bitte neu \
                          öffnen.";

/// The store of the session of `project` refuses writes ([`ADOPTING`], code `busy`) from
/// [`Busy::begin`] until this is dropped.
struct Busy<'a> {
    state: &'a AppState,
    project: Arc<Project>,
}

impl<'a> Busy<'a> {
    /// Under the session lock: the session must still be of `project` and hold no unsaved
    /// value (`unsaved`); marks its store busy and answers its change generation.
    fn begin(state: &'a AppState, project: &Arc<Project>) -> Result<(Busy<'a>, u64), CmdError> {
        let mut guard = lock(state)?;
        let session = same_session(&mut guard, project)?;
        let unsaved = session.store.dirty().len();
        if unsaved > 0 {
            return Err(CmdError::refused(
                "unsaved",
                super::textproject::unsaved_message(unsaved),
            ));
        }
        session.store.set_busy(Some(ADOPTING.to_owned()));
        let generation = session.store.change_generation();
        Ok((
            Busy {
                state,
                project: Arc::clone(project),
            },
            generation,
        ))
    }
}

impl Drop for Busy<'_> {
    /// Lifts the refusal, when the session is still the one it was set on (also after a
    /// failure or a panic).
    fn drop(&mut self) {
        let mut guard = self.state.lock_session();
        if let Some(session) = guard
            .as_mut()
            .filter(|s| Arc::ptr_eq(&s.project, &self.project))
        {
            session.store.set_busy(None);
        }
    }
}

/// The session of `project` under `guard`; an error when another project is open.
fn same_session<'g>(
    guard: &'g mut Option<crate::state::Session>,
    project: &Arc<Project>,
) -> Result<&'g mut crate::state::Session, CmdError> {
    match session(guard) {
        Ok(session) if Arc::ptr_eq(&session.project, project) => Ok(session),
        Ok(_) => Err("Das Projekt wurde gewechselt.".into()),
        Err(e) => Err(e),
    }
}

/// Replaces the session of `project` by `fresh` (the same folder, opened anew after an
/// adoption, a revert or a migration), but only while it is unchanged: still open, no
/// unsaved value, its change generation still `generation`. Else [`REOPEN`] and the session
/// stays as it is: nothing is lost, the files were only written past it. Without the agents'
/// turn: the caller holds it (the Tauri command, an agent's call).
fn replace_if_unchanged(
    state: &AppState,
    project: &Arc<Project>,
    generation: u64,
    fresh: crate::state::Session,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let unchanged = same_session(&mut guard, project).is_ok_and(|session| {
        session.store.dirty().is_empty() && session.store.change_generation() == generation
    });
    if !unchanged {
        return Err(REOPEN.into());
    }
    let old = guard.replace(fresh);
    drop(guard);
    // The old project's language server ends with its session, outside the state lock.
    drop(old);
    Ok(())
}

/// Opens `project`'s folder anew and puts it in place of the session if that is unchanged
/// since `generation` ([`replace_if_unchanged`]).
fn reopen(state: &AppState, project: &Arc<Project>, generation: u64) -> Result<(), CmdError> {
    let root = &project.root;
    let path = root.to_str().ok_or_else(|| {
        CmdError::from(format!("Pfad ist kein gültiger Text: {}", root.display()))
    })?;
    let (fresh, _info) = super::new_session(state, path)?;
    replace_if_unchanged(state, project, generation, fresh)
}

/// What the project is as a source; see [`TextProjectStatusDto`].
pub fn text_project_status_impl(state: &AppState) -> Result<TextProjectStatusDto, CmdError> {
    let project = super::open_project_arc(state)?;
    Ok(status_of(state, &project))
}

/// The refusals of [`adopt_text_project_impl`] that need no adoption, in its order: a text
/// project already ([`ALREADY_TEXT_PROJECT`], `error`, at once), unsaved changes (`unsaved`),
/// Diamond/Pearl (`error`, before a base ROM is looked for), no found base ROM
/// (`base_missing`). Writes nothing; over MCP in the app it runs before the question „Als
/// Textprojekt einrichten“. The open project, its base mark and where its base ROM lies when
/// nothing refuses.
///
/// A text project is refused although the kernel's `adopt` takes one (an export that writes
/// nothing plus the sync records, which `g4k text adopt` keeps as a repair): in the Studio it
/// only re-reads every file for a minute, and the sync records what a text project needs.
pub(crate) fn check_adopt(
    state: &AppState,
) -> Result<(Arc<Project>, g4_kernel::start::BaseMark, std::path::PathBuf), CmdError> {
    let (project, unsaved) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        (Arc::clone(&session.project), session.store.dirty().len())
    };
    if *project.source_mode() != SourceMode::Dspre {
        return Err(ALREADY_TEXT_PROJECT.into());
    }
    if unsaved > 0 {
        return Err(CmdError::refused(
            "unsaved",
            super::textproject::unsaved_message(unsaved),
        ));
    }
    if project.family == Family::Dp {
        return Err(NO_DP.into());
    }
    let (mark, hit) = found_base(state, &project.root)?;
    Ok((project, mark, hit.path))
}

/// Sets the open DSPRE folder up as text project (`textproject::adopt::adopt`: the scripts
/// adopted, the export with `progress`, the id files, the sync recorded), then opens the
/// session anew on the same folder, so the store reads and writes the JSON. Refuses before
/// writing anything what [`check_adopt`] refuses (a text project among it), and every refusal
/// of the kernel (another base ROM, texts in conflict). Runs without the session lock (10 to
/// 120 s) but with the store busy:
/// every write meanwhile (edit, save, undo, redo, reset, rename, a new entity) is refused with
/// `busy` ([`ADOPTING`]). The session is replaced only when it is unchanged since
/// ([`REOPEN`] otherwise).
pub fn adopt_text_project_impl(
    state: &AppState,
    progress: &mut dyn FnMut(AdoptProgress),
) -> Result<AdoptReport, CmdError> {
    let (project, mark, path) = check_adopt(state)?;
    let base = BaseRom::open(&path, Some(&mark.sha1))?;
    let (busy, generation) = Busy::begin(state, &project)?;
    let adopted = adopt::adopt(&project, &base, progress).map_err(text_error);
    // Also after a failure of a DSPRE folder whose manifest is there now (the export wrote it,
    // a later step failed): the store must not go on saving members only. A refused adoption
    // of a text project (`pending`) changed nothing: its session stays.
    let half_adopted = *project.source_mode() == SourceMode::Dspre
        && g4_kernel::textproject::is_text_project(&project.root);
    let reopened = if adopted.is_ok() || half_adopted {
        Some(reopen(state, &project, generation))
    } else {
        None
    };
    drop(busy);
    match (adopted, reopened) {
        (Ok(report), None | Some(Ok(()))) => Ok(report),
        (Ok(_), Some(Err(e))) => Err(e),
        // The adoption's error is the answer; the session that could not be replaced is
        // only logged (the next open reads the manifest).
        (Err(e), Some(Err(reopen))) => {
            eprintln!(
                "Sitzung nach gescheitertem Einrichten nicht neu geöffnet: {}",
                reopen.message
            );
            Err(e)
        }
        (Err(e), _) => Err(e),
    }
}

/// Takes the text project back to a DSPRE folder (`textproject::adopt::revert`: the manifest,
/// `data/` and the id files moved to `.g4/backup/textproject-<time>/`), then opens the
/// session anew as DSPRE folder, when it is unchanged since ([`REOPEN`] otherwise). The revert
/// runs under the session lock: refused with `unsaved` while the store holds unsaved values,
/// with `pending` while the sync finds a change from outside, a conflict or a broken file, and
/// while a script uses an id only the text project knows.
pub fn revert_text_project_impl(state: &AppState) -> Result<RevertReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    text_project(&project, NOT_TEXT_PROJECT)?;
    let unsaved = session.store.dirty().len();
    if unsaved > 0 {
        return Err(CmdError::refused(
            "unsaved",
            super::textproject::unsaved_message(unsaved),
        ));
    }
    let generation = session.store.change_generation();
    let report = adopt::revert(&project).map_err(text_error)?;
    drop(guard);
    reopen(state, &project, generation)?;
    Ok(report)
}

/// Lifts a text project of an older schema to the current one (`textproject::migrate`, with
/// its snapshot of the old files), then opens the session anew when it is unchanged since
/// ([`REOPEN`] otherwise). A current one is left alone; a DSPRE folder is `not_text_project`,
/// a newer schema refused.
pub fn migrate_text_project_impl(state: &AppState) -> Result<MigrateReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    if *project.source_mode() == SourceMode::Dspre {
        return Err(CmdError::refused("not_text_project", NOT_TEXT_PROJECT));
    }
    let generation = session.store.change_generation();
    let report = migrate::migrate(&project.root)?;
    drop(guard);
    if !report.changed.is_empty() || report.from != report.to {
        reopen(state, &project, generation)?;
    }
    Ok(report)
}

/// The sync of the open text project with what changed outside it: scans without the session
/// lock (`progress` gets the scan's counts and never takes that lock), then, under the agents'
/// turn and the session lock, takes the one-sided changes over (`textproject::sync::sync_store`:
/// a unit with an unsaved value is a conflict instead) and loads the assets it changed anew.
/// `not_text_project` outside a text project.
pub fn external_sync_impl(
    state: &AppState,
    progress: &mut dyn FnMut(SyncProgress),
) -> Result<SyncOutcome, CmdError> {
    sync_session(state, progress, true)
}

/// [`external_sync_impl`]; `turn`: take the agents' turn for the takeover (not when the caller
/// may hold it already, the build of an agent).
fn sync_session(
    state: &AppState,
    progress: &mut dyn FnMut(SyncProgress),
    turn: bool,
) -> Result<SyncOutcome, CmdError> {
    let project = super::open_project_arc(state)?;
    text_project(&project, NOT_TEXT_PROJECT)?;
    let report = sync::scan(&project, progress)?;
    let _turn = turn.then(|| state.agents.agent_turn());
    let mut guard = lock(state)?;
    let session = match session(&mut guard) {
        Ok(session) if Arc::ptr_eq(&session.project, &project) => session,
        _ => {
            // Another project meanwhile: the scan is applied without taking anything over
            // (its time settles the entries recorded just before it).
            drop(guard);
            sync::apply_one_sided(&project, &report, &|_| true)?;
            return Err("Das Projekt wurde gewechselt; nichts abgeglichen.".into());
        }
    };
    let outcome = sync::sync_store(&mut session.store, report)?;
    if !outcome.adopted.is_empty() || !outcome.applied.is_empty() {
        // The takeover wrote sources and members past the store.
        session.disk_changed();
        session.files_written(&[]);
        session.id_files_changed();
    }
    Ok(outcome)
}

/// Before a build: in a text project the sync, refused with `conflicts` while a conflict is
/// open; nothing outside a text project. `turn`: take the agents' turn for the takeover (the
/// user's build; an agent's build holds it already).
pub(crate) fn sync_before_build(state: &AppState, turn: bool) -> Result<(), CmdError> {
    let project = super::open_project_arc(state)?;
    if *project.source_mode() != SourceMode::TextProject {
        return Ok(());
    }
    let outcome = sync_session(state, &mut |_| {}, turn)?;
    if outcome.conflicts.is_empty() {
        Ok(())
    } else {
        Err(CmdError::refused("conflicts", CONFLICTS))
    }
}

/// What changed outside the open text project since the last sync, without taking anything
/// over: the units that are not in step, and with `diff` the diff of every conflict (both
/// sides changed, or one side of a unit the store holds unsaved). Takes nothing over and
/// writes no source and no member; it records only the units already in step (the scan is
/// applied without a takeover, so their entries settle). `not_text_project` outside a text project.
pub fn external_changes_impl(state: &AppState, diff: bool) -> Result<ExternalChangesDto, CmdError> {
    let project = super::open_project_arc(state)?;
    text_project(&project, NOT_TEXT_PROJECT)?;
    let mut scan = sync::scan(&project, &mut |_| {})?;
    sync::apply_one_sided(&project, &scan, &|_| true)?;
    scan.units.retain(|unit| unit.side != UnitSide::InSync);
    let mut diffs = BTreeMap::new();
    if diff {
        // The Studio's side of a unit the store holds unsaved is its unsaved value.
        let mut guard = lock(state)?;
        let session = same_session(&mut guard, &project)?;
        let dirty = session.store.dirty();
        for unit in &scan.units {
            let conflict = match unit.side {
                UnitSide::Conflict => true,
                UnitSide::Dspre | UnitSide::Files => {
                    unit.assets.iter().any(|id| dirty.contains(id))
                }
                UnitSide::InSync | UnitSide::Broken => false,
            };
            if conflict {
                diffs.insert(
                    unit.key.clone(),
                    sync::conflict_diff_in(&session.store, &unit.key)?,
                );
            }
        }
    }
    Ok(ExternalChangesDto { scan, diffs })
}

/// The diff of conflict `key`: the Studio's side (its source; the store's unsaved values when
/// it holds the unit unsaved, with `unsaved` set, so the user sees what the editor shows) and
/// DSPRE's (its members, exported in memory). Under the session lock; writes nothing.
pub fn conflict_diff_impl(state: &AppState, key: String) -> Result<ConflictDiff, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    text_project(&session.project, NOT_TEXT_PROJECT)?;
    Ok(sync::conflict_diff_in(&session.store, &key)?)
}

/// The user's choice for conflict `key` (`textproject::sync::resolve`): `studio` keeps the
/// source and writes the member from it, `dspre` takes the other side (DSPRE's member, or the
/// source changed on disk) and drops the unsaved values of its assets. Under the session lock;
/// answers the assets loaded anew. „Studio-Fassung behalten“ against a source changed on disk
/// (`other = files`) takes the file as the base of the unsaved value (`accept_disk_as_base`),
/// which drops that asset's undo history. A unit that is no conflict (any more) is refused
/// with `NO_CONFLICT`.
pub fn resolve_external_conflict_impl(
    state: &AppState,
    key: String,
    keep: Keep,
) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    text_project(&session.project, NOT_TEXT_PROJECT)?;
    let result = sync::resolve(&mut session.store, &key, keep);
    if !matches!(result, Err(KernelError::Refused(_))) {
        // It may have written a source or a member before it failed.
        session.disk_changed();
        session.files_written(&[]);
        session.id_files_changed();
    }
    Ok(result?)
}

/// What renaming register id `id` to `new_id` would change (`textproject::rename::plan_rename`);
/// a refusal is the plan's `refusal`, nothing is written. Reads every JSON under `data/`
/// (seconds on a large project), without the session lock. `not_text_project` outside a text
/// project.
pub fn plan_rename_id_impl(
    state: &AppState,
    id: String,
    new_id: String,
) -> Result<RenamePlan, CmdError> {
    let project = super::open_project_arc(state)?;
    text_project(&project, NOT_A_TEXT_PROJECT)?;
    rename::plan_rename(&project, &id, &new_id).map_err(text_error)
}

/// Renames register id `id` to `new_id` with every reference as one undo step
/// (`textproject::rename::rename_id`), under the session lock. A refusal (a taken or foreign
/// fallback id, wrong characters) writes nothing and is an `error` with its sentence;
/// `not_text_project` outside a text project. The language server is told about the id files.
pub fn rename_id_impl(
    state: &AppState,
    id: String,
    new_id: String,
) -> Result<RenameResult, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    text_project(&session.project, NOT_A_TEXT_PROJECT)?;
    let result = rename::rename_id(&mut session.store, &id, &new_id).map_err(text_error)?;
    // The JSON, the project files and the script sources were written past the store.
    session.files_written(&[]);
    session.id_files_changed();
    session.after_write();
    Ok(result)
}

/// Every id of `kind` by number: the fixed tables everywhere, the registers (maps, trainers,
/// trainer classes, places) in a text project (empty in a DSPRE folder).
pub fn entity_ids_impl(
    state: &AppState,
    kind: SymbolKind,
) -> Result<BTreeMap<u32, String>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.ids()?.ids(kind))
}

/// Runs `f` on a blocking worker (off the main thread).
async fn blocking<T: Send + 'static>(
    app: tauri::AppHandle,
    f: impl FnOnce(&AppState) -> Result<T, CmdError> + Send + 'static,
) -> Result<T, CmdError> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// Looks for the base ROM of a DSPRE folder (it may hash it once), so off the main thread.
#[tauri::command]
pub async fn text_project_status(app: tauri::AppHandle) -> Result<TextProjectStatusDto, CmdError> {
    blocking(app, text_project_status_impl).await
}

/// Adopts and exports for 10 to 120 s: on a blocking worker under the agents' turn (no agent
/// call writes meanwhile), with the progress over `progress`.
#[tauri::command]
pub async fn adopt_text_project(
    app: tauri::AppHandle,
    progress: Channel<AdoptProgress>,
) -> Result<AdoptReport, CmdError> {
    blocking(app, move |state| {
        let _turn = state.agents.agent_turn();
        adopt_text_project_impl(state, &mut |p| {
            // A closed channel only means nobody watches the progress any more.
            let _ = progress.send(p);
        })
    })
    .await
}

/// Scans before it moves the files, and opens the session anew: under the agents' turn.
#[tauri::command]
pub async fn revert_text_project(app: tauri::AppHandle) -> Result<RevertReport, CmdError> {
    blocking(app, |state| {
        let _turn = state.agents.agent_turn();
        revert_text_project_impl(state)
    })
    .await
}

/// Reads and writes every JSON of the project and opens the session anew: under the agents'
/// turn.
#[tauri::command]
pub async fn migrate_text_project(app: tauri::AppHandle) -> Result<MigrateReport, CmdError> {
    blocking(app, |state| {
        let _turn = state.agents.agent_turn();
        migrate_text_project_impl(state)
    })
    .await
}

/// The sync on opening, on focus and on „Jetzt abgleichen“: on a blocking worker, the scan's
/// progress over `progress`.
#[tauri::command]
pub async fn external_sync(
    app: tauri::AppHandle,
    progress: Channel<SyncProgress>,
) -> Result<SyncOutcome, CmdError> {
    blocking(app, move |state| {
        external_sync_impl(state, &mut |p| {
            let _ = progress.send(p);
        })
    })
    .await
}

#[tauri::command]
pub async fn external_changes(
    app: tauri::AppHandle,
    diff: Option<bool>,
) -> Result<ExternalChangesDto, CmdError> {
    blocking(app, move |state| {
        external_changes_impl(state, diff.unwrap_or(false))
    })
    .await
}

#[tauri::command]
pub async fn conflict_diff(app: tauri::AppHandle, key: String) -> Result<ConflictDiff, CmdError> {
    blocking(app, move |state| conflict_diff_impl(state, key)).await
}

#[tauri::command]
pub async fn resolve_external_conflict(
    app: tauri::AppHandle,
    key: String,
    keep: Keep,
) -> Result<Vec<AssetId>, CmdError> {
    blocking(app, move |state| {
        resolve_external_conflict_impl(state, key, keep)
    })
    .await
}

/// Reads every JSON under `data/` (seconds on HeartGold), so off the main thread.
#[tauri::command]
pub async fn plan_rename_id(
    app: tauri::AppHandle,
    id: String,
    new_id: String,
) -> Result<RenamePlan, CmdError> {
    blocking(app, move |state| plan_rename_id_impl(state, id, new_id)).await
}

#[tauri::command]
pub async fn rename_id(
    app: tauri::AppHandle,
    id: String,
    new_id: String,
) -> Result<RenameResult, CmdError> {
    blocking(app, move |state| rename_id_impl(state, id, new_id)).await
}

#[tauri::command]
pub fn entity_ids(
    state: tauri::State<AppState>,
    kind: SymbolKind,
) -> Result<BTreeMap<u32, String>, CmdError> {
    entity_ids_impl(&state, kind)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        DataKind, DataPatch, build_rom_impl, data_edit_impl, dirty_impl, history_impl,
        open_project_impl, open_synthetic, save_impl, set_matrix_cell_impl, undo_impl,
    };
    use g4_kernel::project::AssetKind;
    use g4_kernel::test_support::{TextFixture, synthetic_project};
    use g4_kernel::textproject::MANIFEST_FILE;
    use serde_json::json;

    const MOVE_1_JSON: &str = "data/moves/0001.json";
    const MOVE_1_MEMBER: &str = "unpacked/moveData/0001";
    const KAI: &str = "TRAINER_YOUNGSTER_KAI";

    /// The synthetic text-project fixture open in a fresh state, its base ROM found (as
    /// `--base-rom` names it).
    struct Text {
        fx: TextFixture,
        state: AppState,
    }

    fn dspre() -> Text {
        let fx = TextFixture::dspre();
        let state = AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        Text { fx, state }
    }

    /// [`dspre`], set up as text project over the command.
    fn adopted() -> Text {
        let t = dspre();
        adopt_text_project_impl(&t.state, &mut |_| {}).unwrap();
        t
    }

    fn power(fx: &TextFixture) -> u64 {
        let json: serde_json::Value = serde_json::from_slice(&fx.read(MOVE_1_JSON)).unwrap();
        json["data"]["power"].as_u64().unwrap()
    }

    fn status(state: &AppState) -> serde_json::Value {
        serde_json::to_value(text_project_status_impl(state).unwrap()).unwrap()
    }

    #[test]
    fn status_names_dspre_with_its_reason_and_current_after_adopt() {
        // No mark: the reason is the mark's.
        let (_fixture, state) = open_synthetic();
        assert_eq!(
            status(&state),
            json!({ "state": "dspre", "adoptable": false, "reason": crate::commands::NO_MARK })
        );
        // A mark and its ROM: adoptable.
        let t = dspre();
        assert_eq!(
            status(&t.state),
            json!({ "state": "dspre", "adoptable": true, "reason": null })
        );
        // The ROM gone: the reason says which.
        t.state.base_roms.set_explicit(None);
        let reason = status(&t.state)["reason"].clone();
        assert!(reason.as_str().unwrap().contains("Basis-ROM"), "{reason}");

        let t = adopted();
        let now = status(&t.state);
        assert_eq!(now["state"], "current", "{now}");
        assert_eq!(now["schema"], SCHEMA_VERSION);
        assert!(
            now["last_sync_at"].as_i64().unwrap() > 1_700_000_000_000,
            "{now}"
        );
        // `open_project` answers the same.
        let info = open_project_impl(&t.state, t.fx.root().to_str().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&info).unwrap()["text_project"]["state"],
            "current"
        );
    }

    #[test]
    fn adopt_refuses_unsaved_and_missing_base_before_asking() {
        let t = dspre();
        set_matrix_cell_impl(&t.state, 0, 0, 0, Some(1), None, None).unwrap();
        let err = adopt_text_project_impl(&t.state, &mut |_| panic!("no progress")).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("unsaved", "Erst speichern: 1 ungespeicherte Änderung.")
        );
        save_impl(&t.state).unwrap();
        t.state.base_roms.set_explicit(None);
        let err = adopt_text_project_impl(&t.state, &mut |_| panic!("no progress")).unwrap_err();
        assert_eq!(err.code, "base_missing", "{}", err.message);
        // The same refusals before the question (MCP in the app).
        assert_eq!(check_adopt(&t.state).unwrap_err().code, "base_missing");
        assert!(!t.fx.root().join(MANIFEST_FILE).exists());
        assert!(!t.fx.root().join("data").exists());
    }

    #[test]
    fn adopt_reopens_the_session_in_json_mode() {
        let t = dspre();
        let mut phases = Vec::new();
        let report = adopt_text_project_impl(&t.state, &mut |p| phases.push(p.phase)).unwrap();
        assert!(report.sync_files > 0);
        assert!(phases.contains(&adopt::AdoptPhase::Entities));
        assert!(t.fx.root().join(MANIFEST_FILE).is_file());
        assert_eq!(power(&t.fx), 10);
        data_edit_impl(
            &t.state,
            DataKind::Move,
            1,
            DataPatch::Move {
                field: "power".into(),
                value: 12,
            },
            None,
        )
        .unwrap();
        save_impl(&t.state).unwrap();
        assert_eq!(power(&t.fx), 12, "the store writes the JSON");
        // Nothing to take over afterwards: the save recorded what it wrote.
        let outcome = external_sync_impl(&t.state, &mut |_| {}).unwrap();
        assert!(outcome.adopted.is_empty() && outcome.applied.is_empty());
    }

    #[test]
    fn external_sync_takes_over_a_member_change_and_reports_it() {
        let t = adopted();
        // Outside a text project: refused.
        let (_fixture, plain) = open_synthetic();
        assert_eq!(
            external_sync_impl(&plain, &mut |_| {}).unwrap_err().code,
            "not_text_project"
        );

        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] += 5;
        t.fx.write(MOVE_1_MEMBER, &member);
        let mut progress = Vec::new();
        let outcome = external_sync_impl(&t.state, &mut |p| progress.push(p)).unwrap();
        let keys: Vec<&str> = outcome.adopted.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, [MOVE_1_JSON]);
        assert!(outcome.conflicts.is_empty() && outcome.applied.is_empty());
        assert_eq!(power(&t.fx), u64::from(member[3]));
        assert!(progress.last().is_some_and(|p| p.done == p.total));
        // The store reads the new value.
        let doc = crate::commands::data_get_impl(&t.state, DataKind::Move, 1).unwrap();
        let doc = serde_json::to_value(doc).unwrap();
        let shown = doc["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "power")
            .map(|f| f["value"].clone());
        assert_eq!(shown, Some(json!(member[3])), "{doc}");
        // A second sync finds nothing.
        let again = external_sync_impl(&t.state, &mut |_| {}).unwrap();
        assert!(again.adopted.is_empty() && again.conflicts.is_empty());
    }

    #[test]
    fn external_changes_reads_and_names_a_conflict_with_its_diff() {
        let t = adopted();
        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] += 5;
        t.fx.write(MOVE_1_MEMBER, &member);
        t.fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
        let changes = external_changes_impl(&t.state, true).unwrap();
        let sides: Vec<(&str, UnitSide)> = changes
            .scan
            .units
            .iter()
            .map(|u| (u.key.as_str(), u.side))
            .collect();
        assert_eq!(sides, [(MOVE_1_JSON, UnitSide::Conflict)]);
        let diff = &changes.diffs[MOVE_1_JSON];
        assert!(
            diff.lines.iter().any(|l| l.path == "data.power"),
            "{diff:?}"
        );
        assert_eq!(
            conflict_diff_impl(&t.state, MOVE_1_JSON.into()).unwrap(),
            *diff
        );
        // It wrote nothing.
        assert_eq!(power(&t.fx), 12);
        assert_eq!(t.fx.read(MOVE_1_MEMBER), member);
        // The Studio's version stays: the member is written from the JSON, and the move the
        // store holds unedited shows the JSON's value, loaded anew.
        crate::commands::data_get_impl(&t.state, DataKind::Move, 1).unwrap();
        let reloaded =
            resolve_external_conflict_impl(&t.state, MOVE_1_JSON.into(), Keep::Studio).unwrap();
        assert_eq!(
            reloaded,
            [AssetId {
                kind: AssetKind::Move,
                index: 1
            }]
        );
        assert_eq!(power(&t.fx), 12);
        assert!(
            external_changes_impl(&t.state, false)
                .unwrap()
                .scan
                .units
                .is_empty()
        );
        let err =
            resolve_external_conflict_impl(&t.state, MOVE_1_JSON.into(), Keep::Dspre).unwrap_err();
        assert_eq!(err.message, sync::NO_CONFLICT);
    }

    /// UI test 4.3b U3: the diff of a conflict the store holds unsaved shows the unsaved value
    /// as the Studio's side (what the editor shows), and says so; MCP's `external_changes`
    /// answers the same diff.
    #[test]
    fn the_diff_of_an_unsaved_conflict_shows_the_unsaved_value() {
        let t = adopted();
        data_edit_impl(
            &t.state,
            DataKind::Move,
            1,
            DataPatch::Move {
                field: "power".into(),
                value: 57,
            },
            None,
        )
        .unwrap();
        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] = 60;
        t.fx.write(MOVE_1_MEMBER, &member);
        let outcome = external_sync_impl(&t.state, &mut |_| {}).unwrap();
        assert!(outcome.conflicts[0].unsaved, "{outcome:?}");
        let diff = conflict_diff_impl(&t.state, MOVE_1_JSON.into()).unwrap();
        let power_line = diff.lines.iter().find(|l| l.path == "data.power").unwrap();
        assert_eq!(
            (power_line.studio.as_str(), power_line.dspre.as_str()),
            ("57", "60")
        );
        assert!(diff.unsaved);
        assert_eq!(
            external_changes_impl(&t.state, true).unwrap().diffs[MOVE_1_JSON],
            diff
        );
        // Nothing written: the JSON keeps the saved power, the edit stays unsaved.
        assert_eq!(power(&t.fx), 10);
        assert_eq!(dirty_impl(&t.state).unwrap().len(), 1);
    }

    #[test]
    fn build_with_a_conflict_is_refused_with_conflicts() {
        let t = adopted();
        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] += 5;
        t.fx.write(MOVE_1_MEMBER, &member);
        t.fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 12");
        let out = t.fx.root().join("build/out.nds");
        let err = build_rom_impl(&t.state, out.to_str().unwrap(), false).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("conflicts", CONFLICTS));
        assert!(!out.exists());
        // Nothing was overwritten.
        assert_eq!(power(&t.fx), 12);
        assert_eq!(t.fx.read(MOVE_1_MEMBER), member);
    }

    #[test]
    fn rename_over_the_command_records_one_undo_step() {
        let t = adopted();
        let plan = plan_rename_id_impl(&t.state, KAI.into(), "TRAINER_KID_KAI".into()).unwrap();
        assert_eq!(plan.refusal, None);
        assert!(plan.total >= 2, "{plan:?}");
        let result = rename_id_impl(&t.state, KAI.into(), "TRAINER_KID_KAI".into()).unwrap();
        assert_eq!(result.plan, plan);
        assert_eq!(
            history_impl(&t.state).unwrap().undo,
            ["ID umbenannt: TRAINER_YOUNGSTER_KAI → TRAINER_KID_KAI"]
        );
        let trainer = t.fx.text("data/trainers/0001.json");
        assert!(trainer.contains("TRAINER_KID_KAI"), "{trainer}");
        let ids = entity_ids_impl(&t.state, SymbolKind::Trainer).unwrap();
        assert_eq!(ids[&1], "TRAINER_KID_KAI");
        assert!(dirty_impl(&t.state).unwrap().is_empty());
        // Undo writes every file back.
        let restored = undo_impl(&t.state).unwrap();
        assert_eq!(restored[0].kind, AssetKind::Trainer);
        assert!(t.fx.text("data/trainers/0001.json").contains(KAI));
        assert_eq!(
            entity_ids_impl(&t.state, SymbolKind::Trainer).unwrap()[&1],
            KAI
        );
        // A taken id is refused before anything is written.
        let err = rename_id_impl(&t.state, KAI.into(), "SPECIES_X".into()).unwrap_err();
        assert_eq!(err.code, "error", "{}", err.message);
        // Outside a text project: refused with its code.
        let (_fixture, plain) = open_synthetic();
        for err in [
            plan_rename_id_impl(&plain, KAI.into(), "TRAINER_X".into()).unwrap_err(),
            rename_id_impl(&plain, KAI.into(), "TRAINER_X".into()).unwrap_err(),
        ] {
            assert_eq!(
                (err.code, err.message.as_str()),
                ("not_text_project", NOT_A_TEXT_PROJECT)
            );
        }
        // The fixed tables everywhere.
        assert_eq!(
            entity_ids_impl(&plain, SymbolKind::Species).unwrap()[&25],
            "SPECIES_PIKACHU"
        );
        assert!(
            entity_ids_impl(&plain, SymbolKind::Trainer)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn revert_takes_the_project_back_and_refuses_pending_changes() {
        let t = adopted();
        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] += 5;
        t.fx.write(MOVE_1_MEMBER, &member);
        let err = revert_text_project_impl(&t.state).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("pending", REVERT_PENDING)
        );
        external_sync_impl(&t.state, &mut |_| {}).unwrap();
        let report = revert_text_project_impl(&t.state).unwrap();
        assert!(report.backup.is_dir());
        assert!(!t.fx.root().join(MANIFEST_FILE).exists());
        assert_eq!(status(&t.state)["state"], "dspre");
        // Not a text project any more.
        assert_eq!(
            revert_text_project_impl(&t.state).unwrap_err().code,
            "not_text_project"
        );
    }

    /// The synthetic project with `manifest` as `g4-project.json`.
    fn with_manifest(manifest: &str) -> g4_kernel::test_support::Synthetic {
        let fixture = synthetic_project();
        std::fs::write(fixture.dir.path().join(MANIFEST_FILE), manifest).unwrap();
        fixture
    }

    #[test]
    fn open_refuses_a_newer_schema_with_the_manifest_sentence() {
        let fixture = with_manifest(r#"{ "schemaVersion": 99 }"#);
        let path = fixture.dir.path().to_str().unwrap();
        let state = AppState::default();
        let err = open_project_impl(&state, path).unwrap_err();
        assert_eq!(
            err.message,
            "Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 99)."
        );
        assert!(state.lock_session().is_none());
        // A broken manifest: refused with file, line and column.
        let fixture = with_manifest("{ kaputt");
        let err = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.message.starts_with("g4-project.json:1:"),
            "{}",
            err.message
        );
        // `studio mcp --project` does not start either.
        let policy =
            crate::commands::policy::UserPolicy::new(fixture.dir.path(), &[], None).unwrap();
        let access = crate::mcp::access::Access::default();
        let refused = crate::mcp::tools::Toolbox::open(policy, access)
            .err()
            .unwrap();
        assert_eq!(refused, err.message);
    }

    #[test]
    fn open_reports_an_older_schema() {
        let fixture = with_manifest(r#"{ "schema": 0 }"#);
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&info).unwrap()["text_project"],
            json!({ "state": "older", "schema": 0 })
        );
        // `studio mcp --project` refuses it with the manifest's sentence.
        let policy =
            crate::commands::policy::UserPolicy::new(fixture.dir.path(), &[], None).unwrap();
        let refused =
            crate::mcp::tools::Toolbox::open(policy, crate::mcp::access::Access::default())
                .err()
                .unwrap();
        assert!(
            refused.starts_with("Das Textprojekt hat Schema-Version 0"),
            "{refused}"
        );
    }

    #[test]
    fn files_written_exports_a_reset_texture() {
        use g4_formats::narc::Narc;
        use g4_kernel::original::ResetTarget;
        let t = adopted();
        let project = t.fx.project();
        // An original whose map texture 0 differs from the project's (and from the base layer).
        let other = tempfile::tempdir().unwrap();
        g4_kernel::test_support::copy_dir(t.fx.root(), other.path());
        let packed = other
            .path()
            .join(project.archives.packed(AssetKind::MapTexture).unwrap());
        let mut narc = Narc::read(&std::fs::read(&packed).unwrap()).unwrap();
        narc.members[0].extend_from_slice(b"original");
        std::fs::write(&packed, narc.write()).unwrap();
        let original = g4_kernel::test_support::ProjectOriginal::new(other.path());

        const SIDE: &str = "data/map_textures/0000.nsbtx";
        assert!(!t.fx.root().join(SIDE).exists());
        let result = crate::commands::reset_with_original(
            &t.state,
            &original,
            ResetTarget::MapTexture { index: 0 },
            None,
        )
        .unwrap();
        assert!(result.recorded);
        assert_eq!(t.fx.read(SIDE), narc.members[0]);
        // The sync finds nothing of its own to take over.
        let outcome = external_sync_impl(&t.state, &mut |_| {}).unwrap();
        assert!(
            outcome.adopted.is_empty() && outcome.conflicts.is_empty(),
            "{outcome:?}"
        );
        undo_impl(&t.state).unwrap();
        assert!(!t.fx.root().join(SIDE).exists());
        let outcome = external_sync_impl(&t.state, &mut |_| {}).unwrap();
        assert!(
            outcome.adopted.is_empty() && outcome.conflicts.is_empty(),
            "{outcome:?}"
        );
    }

    #[test]
    fn a_dspre_folder_with_an_unreadable_mapname_keeps_its_ids_and_ignore_list() {
        use g4_kernel::checks::{IgnoreKey, IgnoreSet, Subject};
        let fixture = synthetic_project();
        let ignored = IgnoreKey {
            code: "trainer.no-party".to_owned(),
            subject: Subject::Trainer(1),
            key: "party".to_owned(),
            fingerprint: 0xff,
        };
        IgnoreSet {
            disabled: ["encounter.empty-slot".to_owned()].into_iter().collect(),
            ignored: [ignored.clone()].into_iter().collect(),
        }
        .save(
            fixture.dir.path(),
            &g4_kernel::textproject::ids::Resolver::fixed(),
        )
        .unwrap();
        // `mapname.bin` there but not readable (a folder of that name).
        let project = g4_kernel::project::Project::open(fixture.dir.path()).unwrap();
        let mapname = project.mapname_path();
        if mapname.is_file() {
            std::fs::remove_file(&mapname).unwrap();
        }
        std::fs::create_dir_all(&mapname).unwrap();
        assert!(g4_kernel::textproject::ids::Resolver::for_project(&project).is_err());
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let mut guard = lock(&state).unwrap();
        let session = session(&mut guard).unwrap();
        // The ignore list read with the fixed tables: nothing broken, both entries kept.
        assert_eq!(session.checks.ignore_broken, None);
        assert!(session.checks.ignore.ignored.contains(&ignored));
        assert!(
            session
                .checks
                .ignore
                .disabled
                .contains("encounter.empty-slot")
        );
        let ids = session.ids().unwrap();
        assert_eq!(ids.parse(SymbolKind::Species, "SPECIES_PIKACHU"), Ok(25));
    }

    #[test]
    fn adopt_refuses_edits_saves_and_undo_while_it_runs() {
        let t = dspre();
        set_matrix_cell_impl(&t.state, 0, 0, 0, Some(1), None, None).unwrap();
        save_impl(&t.state).unwrap();
        let mut refused = Vec::new();
        adopt_text_project_impl(&t.state, &mut |_| {
            if !refused.is_empty() {
                return;
            }
            let edit = data_edit_impl(
                &t.state,
                DataKind::Move,
                1,
                DataPatch::Move {
                    field: "power".into(),
                    value: 12,
                },
                None,
            );
            for err in [
                edit.map(|_| ()).unwrap_err(),
                save_impl(&t.state).map(|_| ()).unwrap_err(),
                undo_impl(&t.state).map(|_| ()).unwrap_err(),
                set_matrix_cell_impl(&t.state, 0, 0, 0, Some(2), None, None)
                    .map(|_| ())
                    .unwrap_err(),
            ] {
                refused.push((err.code, err.message));
            }
            // Reading goes on.
            crate::commands::get_header_impl(&t.state, 0).unwrap();
        })
        .unwrap();
        assert_eq!(refused.len(), 4);
        for (code, message) in &refused {
            assert_eq!((*code, message.as_str()), ("busy", ADOPTING));
        }
        // Afterwards the new session writes the JSON.
        assert!(dirty_impl(&t.state).unwrap().is_empty());
        data_edit_impl(
            &t.state,
            DataKind::Move,
            1,
            DataPatch::Move {
                field: "power".into(),
                value: 12,
            },
            None,
        )
        .unwrap();
        save_impl(&t.state).unwrap();
        assert_eq!(power(&t.fx), 12);
    }

    #[test]
    fn a_session_changed_meanwhile_is_not_replaced() {
        let t = dspre();
        let (project, generation) = {
            let mut guard = lock(&t.state).unwrap();
            let session = session(&mut guard).unwrap();
            (
                Arc::clone(&session.project),
                session.store.change_generation(),
            )
        };
        set_matrix_cell_impl(&t.state, 0, 0, 0, Some(1), None, None).unwrap();
        let path = t.fx.root().to_str().unwrap();
        let (fresh, _) = super::super::new_session(&t.state, path).unwrap();
        let err = replace_if_unchanged(&t.state, &project, generation, fresh).unwrap_err();
        assert!(err.message.contains("neu öffnen"), "{}", err.message);
        // The edit is still there, in the same session.
        assert_eq!(dirty_impl(&t.state).unwrap().len(), 1);
        {
            let mut guard = lock(&t.state).unwrap();
            assert!(Arc::ptr_eq(&session(&mut guard).unwrap().project, &project));
        }
        // Unchanged: replaced.
        save_impl(&t.state).unwrap();
        let generation = {
            let mut guard = lock(&t.state).unwrap();
            session(&mut guard).unwrap().store.change_generation()
        };
        let (fresh, _) = super::super::new_session(&t.state, path).unwrap();
        replace_if_unchanged(&t.state, &project, generation, fresh).unwrap();
        let mut guard = lock(&t.state).unwrap();
        assert!(!Arc::ptr_eq(
            &session(&mut guard).unwrap().project,
            &project
        ));
    }

    #[test]
    fn diamond_and_pearl_are_not_adoptable() {
        let fixture = synthetic_project();
        let header = fixture.dir.path().join("header.yaml");
        let yaml = std::fs::read_to_string(&header)
            .unwrap()
            .replace("CPUE", "ADAE");
        std::fs::write(&header, yaml).unwrap();
        // Long enough for the Diamond header table (zeros: empty headers).
        let arm9 = fixture.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes.resize(0x10_0000, 0);
        std::fs::write(&arm9, bytes).unwrap();
        let state = AppState::default();
        let info = open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&info).unwrap()["text_project"],
            json!({ "state": "dspre", "adoptable": false, "reason": NO_DP })
        );
        // Refused before the base ROM is even looked for, and nothing written.
        let err = check_adopt(&state).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("error", NO_DP));
        let err = adopt_text_project_impl(&state, &mut |_| panic!("no progress")).unwrap_err();
        assert_eq!(err.message, NO_DP);
        assert!(!fixture.dir.path().join(MANIFEST_FILE).exists());
    }

    /// The writes that do not go through the store's own guard refuse a busy session too:
    /// a build (and the patch, which builds), the world file and the ignore list.
    #[test]
    fn builds_and_project_files_are_refused_while_busy() {
        let t = dspre();
        lock(&t.state)
            .unwrap()
            .as_mut()
            .unwrap()
            .store
            .set_busy(Some(ADOPTING.to_owned()));
        let out = t.fx.root().join("build/out.nds");
        let busy = |err: CmdError| assert_eq!((err.code, err.message.as_str()), ("busy", ADOPTING));
        busy(build_rom_impl(&t.state, out.to_str().unwrap(), false).unwrap_err());
        busy(crate::commands::create_patch_impl(&t.state).unwrap_err());
        busy(
            crate::commands::world_meta_set_impl(
                &t.state,
                g4_kernel::world_meta::WorldMeta::default(),
            )
            .unwrap_err(),
        );
        busy(
            crate::commands::set_check_enabled_impl(&t.state, "encounter.empty-slot", false)
                .unwrap_err(),
        );
        assert!(!out.exists());
        assert!(!t.fx.root().join(g4_kernel::world_meta::WORLD_FILE).exists());
        assert!(!t.fx.root().join(g4_kernel::checks::IGNORE_FILE).exists());
    }

    /// UI test 4.3b U2: setting up a text project again is refused at once with a sentence
    /// (before the question of an agent, before looking for the base ROM), whether something
    /// is pending or not, instead of exporting every file anew; nothing is written and the
    /// session stays.
    #[test]
    fn adopting_a_text_project_is_refused_at_once_and_keeps_the_session() {
        let t = adopted();
        let before = crate::commands::open_project_arc(&t.state).unwrap();
        let started = std::time::Instant::now();
        let err = adopt_text_project_impl(&t.state, &mut |_| panic!("no progress")).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("error", ALREADY_TEXT_PROJECT)
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(
            check_adopt(&t.state).unwrap_err().message,
            ALREADY_TEXT_PROJECT
        );
        // Also with the base ROM gone and a change from outside pending.
        let mut member = t.fx.read(MOVE_1_MEMBER);
        member[3] += 5;
        t.fx.write(MOVE_1_MEMBER, &member);
        t.state.base_roms.set_explicit(None);
        let err = adopt_text_project_impl(&t.state, &mut |_| {}).unwrap_err();
        assert_eq!(err.message, ALREADY_TEXT_PROJECT);
        let after = crate::commands::open_project_arc(&t.state).unwrap();
        assert!(Arc::ptr_eq(&before, &after), "the session was replaced");
        assert_eq!(t.fx.read(MOVE_1_MEMBER), member);

        // A 4.3a text project without a sync record whose JSON the members lack: the same.
        let t = adopted();
        g4_kernel::build::state::BuildState::update(t.fx.root(), |s| s.sync = None).unwrap();
        t.fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");
        let json = t.fx.read(MOVE_1_JSON);
        let err = adopt_text_project_impl(&t.state, &mut |_| {}).unwrap_err();
        assert_eq!(err.message, ALREADY_TEXT_PROJECT);
        assert_eq!(t.fx.read(MOVE_1_JSON), json);
    }
}
