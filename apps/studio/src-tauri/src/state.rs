use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use g4_kernel::checks::{ChecksReport, Group, IgnoreSet, Overlay, Runner, Snapshot, SnapshotCache};
use g4_kernel::data::battle_sprites::BattleSpriteCache;
use g4_kernel::data::names::NameTables;
use g4_kernel::data::refs::RefIndex;
use g4_kernel::git::GitCli;
use g4_kernel::project::{AssetId, AssetKind, Project, SourceMode};
use g4_kernel::sprites::SpriteCache;
use g4_kernel::store::Store;
use g4_kernel::textproject::ids::Resolver;
use g4_kernel::textproject::include::id_file_paths;
use g4_kernel::textproject::source::after_member_write;

use crate::base_roms::BaseRoms;
use crate::errors::CmdError;
use crate::lsp::LspProcess;
use crate::mcp::hub::AgentHub;

/// The open project and its asset store.
pub struct Session {
    pub project: Arc<Project>,
    pub store: Store,
    /// The overworld table and the decoded object sprites of this project.
    pub sprites: SpriteCache,
    /// The decoded battle sprites of Pokémon and trainers of this project, as PNG.
    pub battle_sprites: BattleSpriteCache,
    /// The `rotom-lsp` of this project, started by `lsp_start` and stopped with the session.
    pub lsp: Option<LspProcess>,
    /// Names of species, moves, items, ... read from the store's text archives, cached per bank.
    pub names: NameTables,
    /// Who references a species, move, item, trainer or encounter file; built on first use.
    pub refs: RefIndex,
    /// The last check run, the one running and the checks' cache.
    pub checks: ChecksState,
    /// The id resolver of the project (an id or a number in `g4-world.json`, `g4-checks.json`
    /// and the MCP arguments); made anew by [`Session::ids`] when its registers changed.
    ids: Resolver,
    /// The id files for rotom as they were last seen (size, modification time and content
    /// hash per path),
    /// to tell the language server when they change ([`Session::id_files_changed`]).
    id_stamps: Vec<(PathBuf, IdStamp)>,
}

/// Size, modification time and a hash of the content of an id file, `None` when it is
/// missing (the content too: a rename and its undo keep the length and may fall into one tick
/// of the modification time).
type IdStamp = Option<(u64, SystemTime, u64)>;

impl Session {
    /// A session of `project` with a fresh store and empty caches.
    pub fn new(project: Arc<Project>) -> Session {
        let store = Store::new(Arc::clone(&project));
        let id_stamps = id_stamps(&project);
        Session {
            project,
            store,
            sprites: SpriteCache::new(),
            battle_sprites: BattleSpriteCache::new(),
            lsp: None,
            names: NameTables::new(),
            refs: RefIndex::new(),
            checks: ChecksState::new(),
            // Never current: the first use makes the project's.
            ids: Resolver::fixed(),
            id_stamps,
        }
    }

    /// The id resolver of the project, made anew when its registers, its manifest or
    /// `mapname.bin` changed on disk ([`Resolver::is_current`]). Outside a text project of
    /// the current schema a project whose resolver cannot be made (an unreadable
    /// `mapname.bin`) gets the fixed tables, so a DSPRE folder keeps reading its ignore list
    /// and world file; in a text project the error is the register's.
    pub fn ids(&mut self) -> Result<&Resolver, CmdError> {
        if !self.ids.is_current(&self.project) {
            self.ids = project_ids(&self.project)?;
        }
        Ok(&self.ids)
    }

    /// Brings the reference index up to date after a command changed the store. Cheap when
    /// the index was never built or nothing it reads changed. A failed refresh leaves the index
    /// as it was; its next use tries again (and reports the error there).
    pub fn after_write(&mut self) {
        let _ = self.refs.refresh(&self.store);
    }

    /// Brings the reference index up to date after a command wrote script sources past the
    /// store (a save, a new script file, a new script): the sources are statted now, whatever
    /// the index's time gate says, so a query right after it sees them. Cheap when the index
    /// was never built; a failed refresh is tried again on the next use.
    pub fn scripts_written(&mut self) {
        let _ = self.refs.refresh_scripts_now(&self.store);
    }

    /// Forgets the reference index after the files on disk may have changed behind the store
    /// (a member reloaded from disk): the next use builds it again from the disk, which also
    /// sees members the store never loaded.
    pub fn disk_changed(&mut self) {
        self.refs = RefIndex::new();
    }

    /// `Ok` while the store takes writes; `busy` with its reason while the project is set up
    /// as text project ([`g4_kernel::store::Store::set_busy`]). The store refuses its own
    /// writes itself; the commands that write files past it ask here first.
    pub fn writable(&self) -> Result<(), CmdError> {
        match self.store.busy() {
            Some(why) => Err(CmdError::refused("busy", why)),
            None => Ok(()),
        }
    }

    /// Notes a write to the project's files that does not go through the store (a script
    /// source or member, a new script file, the adoption, `create_script`, the members of a
    /// new trainer): the checks read those from the disk, so their last report and a pass
    /// running now are of an older state ([`ChecksState::disk_written`]).
    ///
    /// `written` names the assets whose files were written: in a text project the binary
    /// members among them (textures and models, reset to the original or by its undo) are
    /// exported to `data/` (`textproject::source::after_member_write`), so the next sync finds
    /// JSON and member in step (and records them) instead of taking the own change for one
    /// from DSPRE. When that fails the sync takes the member over later (from DSPRE): nothing
    /// is lost, so it is only logged.
    pub fn files_written(&mut self, written: &[AssetId]) {
        self.checks.disk_written();
        let members: Vec<PathBuf> = written
            .iter()
            .filter(|id| is_binary_member(id.kind))
            .map(|id| {
                self.project
                    .unpacked_dir(id.kind)
                    .join(format!("{:04}", id.index))
            })
            .collect();
        if !members.is_empty()
            && let Err(e) = after_member_write(&self.project, &members)
        {
            eprintln!("data/ nach dem Schreiben von Membern nicht nachgezogen: {e}");
        }
    }

    /// After a step that may have written the id files for rotom back or anew (the undo or
    /// redo of an id rename or a naming, the rollback of a batch): the language server hears
    /// of the id files that changed ([`Session::id_files_changed`]) and, when any did, the
    /// checks see the files as written ([`Session::files_written`]).
    pub fn id_files_followup(&mut self) {
        if !self.id_files_changed().is_empty() {
            self.files_written(&[]);
        }
    }

    /// Tells the running language server (`rotom-lsp`) which id files for rotom changed since
    /// they were last seen, so the script editor knows new and renamed ids; answers their
    /// paths (relative to the root).
    ///
    /// The notification is `workspace/didChangeWatchedFiles`: rotom-lsp drops its cached
    /// command database and constants on it and checks the open scripts again. A change is a
    /// new size, modification time or content hash (the hash is only kept in memory, so the
    /// std hasher does). Reads the id files, a few hundred KB at most. Call it after anything
    /// that may write them: a rename and its undo or redo, a sync that christened, the choice
    /// of a conflict.
    pub fn id_files_changed(&mut self) -> Vec<PathBuf> {
        let now = id_stamps(&self.project);
        let changes: Vec<(PathBuf, u8)> = now
            .iter()
            .zip(&self.id_stamps)
            .filter(|((_, new), (_, old))| new != old)
            .map(|((path, new), (_, old))| {
                // LSP FileChangeType: 1 created, 2 changed, 3 deleted.
                let kind = match (old, new) {
                    (None, _) => 1,
                    (_, None) => 3,
                    _ => 2,
                };
                (path.clone(), kind)
            })
            .collect();
        self.id_stamps = now;
        if changes.is_empty() {
            return Vec::new();
        }
        if let Some(lsp) = &self.lsp {
            let body = watched_files_changed(&self.project.root, &changes);
            // A server that does not read is restarted by the editor's next request, and
            // reads the files anew then.
            let _ = lsp.send(&body);
        }
        changes.into_iter().map(|(path, _)| path).collect()
    }
}

/// The id resolver of `project`; outside a text project of the current schema the fixed
/// tables when it cannot be made (see [`Session::ids`]).
pub fn project_ids(project: &Project) -> Result<Resolver, CmdError> {
    match Resolver::for_project(project) {
        Ok(ids) => Ok(ids),
        Err(_) if *project.source_mode() != SourceMode::TextProject => Ok(Resolver::fixed()),
        Err(e) => Err(e.into()),
    }
}

/// Whether assets of `kind` are binary members written past the store (a reset of a texture
/// or model, and its undo).
fn is_binary_member(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::MapTexture
            | AssetKind::BuildingTexture
            | AssetKind::BuildingModel
            | AssetKind::InteriorBuildingModel
    )
}

/// The id files of `project` (relative to the root) with their stamps.
fn id_stamps(project: &Project) -> Vec<(PathBuf, IdStamp)> {
    id_file_paths(project)
        .into_iter()
        .map(|rel| {
            let path = project.root.join(&rel);
            let stamp = std::fs::metadata(&path).ok().and_then(|m| {
                let bytes = std::fs::read(&path).ok()?;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hasher::write(&mut hasher, &bytes);
                Some((
                    m.len(),
                    m.modified().ok()?,
                    std::hash::Hasher::finish(&hasher),
                ))
            });
            (rel, stamp)
        })
        .collect()
}

/// The notification `workspace/didChangeWatchedFiles` for `changes` (path relative to `root`,
/// LSP change type).
fn watched_files_changed(root: &Path, changes: &[(PathBuf, u8)]) -> String {
    let changes: Vec<serde_json::Value> = changes
        .iter()
        .map(|(path, kind)| serde_json::json!({ "uri": file_uri(&root.join(path)), "type": kind }))
        .collect();
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeWatchedFiles",
        "params": { "changes": changes },
    })
    .to_string()
}

/// `path` as a `file://` URL, percent-encoded like the script editor's `projectFileUri` (the
/// WHATWG path set, with `%`, `#` and `?`).
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'/' | b'-'
                    | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
            );
        if keep {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// Progress of a running check pass: a short label (`cheap`, `scripts`, `texts`, `load`) the
/// UI words, files or checks done, and their total.
pub type ChecksProgress = (String, u32, u32);

/// The checks of a session (spec 3b.4 §5): the last current report, whether a pass runs and
/// how far it got, the disk epoch, the ignore list and the cache of decoded text archives and
/// script sources between runs. The automatic run after an edit is a full run the frontend
/// asks for after a pause of its own (`useChecks`, rulings R4 and R12); nothing here waits.
///
/// A report is current while the store has its change generation and the disk epoch is the
/// one it began at: the generation counts the store's changes, the epoch the writes to the
/// project's files outside the store (script saves and the like, see
/// [`Session::files_written`]), which the checks read from the disk.
pub struct ChecksState {
    /// The last report that was current when it finished; never one of an older generation.
    pub last: Option<ChecksReport>,
    /// The disk epoch `last` was made at (see `epoch`).
    last_epoch: u64,
    /// Rises with every write to the project's files outside the store
    /// ([`ChecksState::disk_written`]).
    epoch: u64,
    /// Set while a pass runs; shared with it, which clears it when it ends in any way
    /// (finished, dropped or panicked, see [`PassGuard`]).
    running: Arc<AtomicBool>,
    /// Set when a pass was discarded because the store changed during it (or a check it
    /// left out was switched on): run again.
    pub rerun: bool,
    /// Disabled checks and ignored findings (from `g4-checks.json`).
    pub ignore: IgnoreSet,
    /// Why `g4-checks.json` could not be read when the project opened (ruling R15): `ignore`
    /// is then empty, every report names the file, and writing it is refused until it reads
    /// again, so it is never overwritten unread. `None`: it was read (or is not there).
    pub ignore_broken: Option<String>,
    /// Shared with the running pass, which writes it without the session lock.
    progress: Arc<Mutex<Option<ChecksProgress>>>,
    /// `None` while a pass holds it.
    cache: Option<SnapshotCache>,
}

impl Default for ChecksState {
    fn default() -> Self {
        ChecksState::new()
    }
}

impl ChecksState {
    pub fn new() -> ChecksState {
        ChecksState {
            last: None,
            last_epoch: 0,
            epoch: 0,
            running: Arc::new(AtomicBool::new(false)),
            rerun: false,
            ignore: IgnoreSet::default(),
            ignore_broken: None,
            progress: Arc::new(Mutex::new(None)),
            cache: Some(SnapshotCache::default()),
        }
    }

    /// Whether a pass runs.
    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// The last report if it is of the store's current generation `store_generation`
    /// (`Store::change_generation`), of the current disk epoch and of the current ignore list;
    /// `None` when there is none, the store changed since, the project's files were written
    /// outside the store since ([`ChecksState::disk_written`]), or a check it left out as
    /// disabled was switched on since (its findings are missing), so a caller never shows
    /// findings of an older state.
    pub fn current_report(&self, store_generation: u64) -> Option<&ChecksReport> {
        self.last.as_ref().filter(|report| {
            report.generation == store_generation
                && self.last_epoch == self.epoch
                && !report.lacks_enabled(&self.ignore)
        })
    }

    /// The disk epoch: how often the project's files were written outside the store (the
    /// tests watch it rise; a report holds it in `last_epoch`).
    #[cfg(test)]
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Notes a write to the project's files outside the store: the last report and a pass
    /// running now are of an older state.
    pub fn disk_written(&mut self) {
        self.epoch += 1;
    }

    /// How far the running pass got; `None` when none runs or it reported nothing yet.
    pub fn progress(&self) -> Option<ChecksProgress> {
        lock(&self.progress).clone()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// What [`AppState::run_checks_pass`] did.
#[derive(Debug)]
pub enum PassOutcome {
    /// The report, current with the store; also kept as [`ChecksState::last`].
    Done(Box<ChecksReport>),
    /// The store changed while the pass ran: its report was discarded and
    /// [`ChecksState::rerun`] is set.
    Stale,
    /// Another pass is running.
    Busy,
    /// No project is open, or it was closed or replaced while the pass ran.
    NoSession,
}

/// What a pass takes from the session under the lock; run it with [`PassInput::run`]
/// without the lock.
pub struct PassInput {
    project: Arc<Project>,
    /// The disk epoch the pass began at.
    epoch: u64,
    overlay: Overlay,
    cache: SnapshotCache,
    ignore: IgnoreSet,
    guard: PassGuard,
}

/// A finished pass, for [`AppState::finish_checks`].
pub struct PassResult {
    project: Arc<Project>,
    epoch: u64,
    cache: SnapshotCache,
    report: ChecksReport,
    guard: PassGuard,
}

/// Marks a pass running from [`AppState::begin_checks`] until it is dropped: with its
/// [`PassInput`] or [`PassResult`], whether the pass finished, was abandoned or panicked. Then
/// the session's `running` and progress are cleared; the cache the pass held is lost (the
/// next pass starts with an empty one) and no report is kept.
struct PassGuard {
    running: Arc<AtomicBool>,
    progress: Arc<Mutex<Option<ChecksProgress>>>,
}

impl Drop for PassGuard {
    fn drop(&mut self) {
        *lock(&self.progress) = None;
        self.running.store(false, Ordering::SeqCst);
    }
}

impl PassInput {
    /// Builds the snapshot and runs the checks of `groups` on this thread, writing the
    /// progress where `checks_status` reads it.
    pub fn run(self, groups: &[Group]) -> PassResult {
        let snap = Snapshot::build(Arc::clone(&self.project), self.overlay, self.cache);
        let progress = &self.guard.progress;
        let report = Runner::run(
            &snap,
            Project::limits(),
            &self.ignore,
            groups,
            |label, done, total| report_progress(progress, label, done, total),
        );
        PassResult {
            project: self.project,
            epoch: self.epoch,
            cache: snap.into_cache(),
            report,
            guard: self.guard,
        }
    }

    /// Writes the progress of this pass.
    pub fn report_progress(&self, label: &str, done: u32, total: u32) {
        report_progress(&self.guard.progress, label, done, total);
    }
}

fn report_progress(progress: &Mutex<Option<ChecksProgress>>, label: &str, done: u32, total: u32) {
    *lock(progress) = Some((label.to_owned(), done, total));
}

/// Tauri-managed state.
#[derive(Default)]
pub struct AppState {
    /// The open session: `None` until a project is opened.
    pub session: Mutex<Option<Session>>,
    /// Set by `force_quit` once the user agreed to discard unsaved edits, so the exit it
    /// triggers is not blocked again.
    pub force_quit: AtomicBool,
    /// Agent settings, connections, the question to the user and the agent events.
    pub agents: AgentHub,
    /// The remembered base ROMs (`base-roms.json`) and where else to search for them.
    pub base_roms: BaseRoms,
    /// The `git` CLI as last probed, so a status read does not run `git --version` each time.
    pub git_cli: GitCliCache,
    /// Play mode live: the connection to melonDS's GDB stub, its worker and the breakpoints.
    pub live: crate::live_hub::LiveHub,
}

/// The `git` CLI the Studio found (`git::find_git`), probed once: a found one is kept until
/// [`GitCliCache::forget`] (after „Git einrichten“), a missing one is probed again at most
/// every [`GitCliCache::RETRY`], so a `git` installed while the app runs is noticed.
#[derive(Default)]
pub struct GitCliCache {
    /// The last probe's answer and when it was made; `None` before the first.
    last: Mutex<Option<(Option<GitCli>, Instant)>>,
}

impl GitCliCache {
    /// How long a missing `git` stays missing before it is looked for again.
    pub const RETRY: Duration = Duration::from_secs(30);

    /// The cached `git` CLI, probing with `probe` (`git::find_git`) when nothing is cached or
    /// the cached absence is [`GitCliCache::RETRY`] old.
    pub fn get(&self, probe: impl FnOnce() -> Option<GitCli>) -> Option<GitCli> {
        self.get_at(Instant::now(), probe)
    }

    /// [`GitCliCache::get`] at `now`.
    pub(crate) fn get_at(
        &self,
        now: Instant,
        probe: impl FnOnce() -> Option<GitCli>,
    ) -> Option<GitCli> {
        let mut last = lock(&self.last);
        match &*last {
            Some((Some(cli), _)) => return Some(cli.clone()),
            Some((None, at)) if now.saturating_duration_since(*at) < Self::RETRY => return None,
            _ => {}
        }
        let found = probe();
        *last = Some((found.clone(), now));
        found
    }

    /// Keeps `found` as the answer of a probe made now (`git_init` probes itself).
    pub fn set(&self, found: Option<GitCli>) {
        *lock(&self.last) = Some((found, Instant::now()));
    }

    /// Forgets the answer: the next [`GitCliCache::get`] probes again.
    pub fn forget(&self) {
        *lock(&self.last) = None;
    }
}

impl AppState {
    /// Runs one check pass over `groups` on the calling thread (a worker: Tauri's
    /// `spawn_blocking`, or the bridge's request thread): the overlay is taken under the
    /// session lock, the snapshot is built and checked without it, and the result is kept
    /// only when the store is still at the overlay's generation.
    ///
    /// A pass that panics re-raises the panic after its input was dropped, which clears
    /// `running` and the progress ([`PassGuard`]): no report is kept and no rerun is asked
    /// for, so a pass that always panics is not started again by itself.
    pub fn run_checks_pass(&self, groups: &[Group]) -> PassOutcome {
        self.run_checks_pass_with(groups, PassInput::run)
    }

    /// [`AppState::run_checks_pass`] with the run itself given (tests make it panic).
    fn run_checks_pass_with(
        &self,
        groups: &[Group],
        run: impl FnOnce(PassInput, &[Group]) -> PassResult,
    ) -> PassOutcome {
        let input = match self.begin_checks() {
            Ok(input) => input,
            Err(outcome) => return outcome,
        };
        match catch_unwind(AssertUnwindSafe(|| run(input, groups))) {
            Ok(result) => self.finish_checks(result),
            // The input is gone with the unwinding: its guard cleared `running`.
            Err(panic) => resume_unwind(panic),
        }
    }

    /// Starts a pass: under the session lock, marks it running and takes the overlay, the
    /// cache and the ignore list. `Err` with [`PassOutcome::Busy`] or
    /// [`PassOutcome::NoSession`] when no pass can start.
    pub fn begin_checks(&self) -> Result<PassInput, PassOutcome> {
        let mut guard = self.lock_session();
        let Some(session) = guard.as_mut() else {
            return Err(PassOutcome::NoSession);
        };
        let checks = &mut session.checks;
        if checks.running() {
            return Err(PassOutcome::Busy);
        }
        checks.running.store(true, Ordering::SeqCst);
        checks.rerun = false;
        *lock(&checks.progress) = None;
        Ok(PassInput {
            project: Arc::clone(&session.project),
            epoch: checks.epoch,
            overlay: session.store.snapshot_overlay(),
            cache: checks.cache.take().unwrap_or_default(),
            ignore: checks.ignore.clone(),
            guard: PassGuard {
                running: Arc::clone(&checks.running),
                progress: Arc::clone(&checks.progress),
            },
        })
    }

    /// Ends a pass under the session lock: keeps its cache, and its report when the store is
    /// still at its generation, nothing was written outside the store (the disk epoch is the
    /// one it began at) and no check the pass left out as disabled was switched on meanwhile;
    /// else discards it and sets [`ChecksState::rerun`]. A session closed or replaced
    /// meanwhile gets nothing. Either way the pass no longer runs.
    pub fn finish_checks(&self, result: PassResult) -> PassOutcome {
        let PassResult {
            project,
            epoch,
            cache,
            report,
            guard: pass,
        } = result;
        let mut guard = self.lock_session();
        let Some(session) = guard.as_mut().filter(|s| Arc::ptr_eq(&s.project, &project)) else {
            return PassOutcome::NoSession;
        };
        let checks = &mut session.checks;
        // The pass ran with the ignore list it began with: a check switched on since is
        // missing from its report as much as a change of the store would be.
        let current = report.is_current(&session.store)
            && epoch == checks.epoch
            && !report.lacks_enabled(&checks.ignore);
        // Under the session lock: a status query never sees the pass ended without its
        // report and cache in place.
        drop(pass);
        checks.cache = Some(cache);
        if current {
            // The ignore list may have changed while the pass ran with its copy.
            let mut report = report;
            report.apply_ignore(&checks.ignore);
            checks.last = Some(report.clone());
            checks.last_epoch = epoch;
            PassOutcome::Done(Box::new(report))
        } else {
            checks.rerun = true;
            PassOutcome::Stale
        }
    }

    /// The session lock, recovered when a panic poisoned it: the session is plain in-memory
    /// data, and refusing it would lose the unsaved edits it holds (a quit would see no
    /// dirty asset). The single place that locks the session.
    pub fn lock_session(&self) -> MutexGuard<'_, Option<Session>> {
        self.session.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Whether an exit request must be held back to ask the user first.
pub fn exit_is_blocked(dirty: usize, force: bool) -> bool {
    dirty > 0 && !force
}

/// What a quit request (our Quit menu item or an exit request) does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitAction {
    /// Quit now.
    Exit,
    /// Ask the frontend first ("confirm-quit").
    Confirm,
}

/// Asks only while a window can show the prompt and unsaved edits exist, unless the user
/// already chose to discard them (`force`).
pub fn quit_action(window_open: bool, dirty: usize, force: bool) -> QuitAction {
    if window_open && exit_is_blocked(dirty, force) {
        QuitAction::Confirm
    } else {
        QuitAction::Exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poisoned_session_lock_is_recovered_with_its_contents() {
        let state = Arc::new(AppState::default());
        let poisoner = Arc::clone(&state);
        let panicked = std::thread::spawn(move || {
            let _guard = poisoner.session.lock().unwrap();
            panic!("poisoned on purpose");
        })
        .join();
        assert!(panicked.is_err());
        assert!(state.session.is_poisoned());
        assert!(state.lock_session().is_none());
    }

    /// An app state with the synthetic project open.
    fn open_state(fixture: &g4_kernel::test_support::Synthetic) -> AppState {
        let state = AppState::default();
        let project = Arc::new(Project::open(fixture.dir.path()).unwrap());
        *state.lock_session() = Some(Session::new(project));
        state
    }

    fn add_warp(state: &AppState) {
        let mut guard = state.lock_session();
        let session = guard.as_mut().unwrap();
        session
            .store
            .edit_event(0, "warp", |e| {
                e.warp_events.push(g4_kernel::store::WarpEvent {
                    x: 1,
                    z: 1,
                    dest_header_id: 1,
                    dest_warp_id: 0,
                    height: 0,
                });
                Ok(())
            })
            .unwrap();
        session.after_write();
    }

    #[test]
    fn a_pass_keeps_its_report_and_the_cache() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        add_warp(&state);
        let PassOutcome::Done(report) = state.run_checks_pass(&Group::ALL) else {
            panic!("the pass did not finish");
        };
        let guard = state.lock_session();
        let session = guard.as_ref().unwrap();
        assert_eq!(report.generation, session.store.change_generation());
        assert_eq!(report.groups, Group::ALL.to_vec());
        let checks = &session.checks;
        assert_eq!(checks.last.as_ref().unwrap().generation, report.generation);
        assert!(!checks.running() && !checks.rerun);
        assert!(checks.progress().is_none());
        assert!(checks.cache.is_some());
    }

    #[test]
    fn a_pass_over_an_old_generation_is_discarded_and_asks_for_a_rerun() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let input = state.begin_checks().unwrap();
        assert!(state.lock_session().as_ref().unwrap().checks.running());
        // The store changes while the pass runs without the lock.
        add_warp(&state);
        let result = input.run(&[Group::Cheap]);
        assert!(matches!(state.finish_checks(result), PassOutcome::Stale));
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert!(checks.last.is_none(), "a stale report is never kept");
            assert!(checks.rerun && !checks.running());
            assert!(checks.cache.is_some());
        }
        // The rerun is current.
        assert!(matches!(
            state.run_checks_pass(&[Group::Cheap]),
            PassOutcome::Done(_)
        ));
        assert!(!state.lock_session().as_ref().unwrap().checks.rerun);
    }

    #[test]
    fn a_second_pass_while_one_runs_is_busy_and_progress_is_readable() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let input = state.begin_checks().unwrap();
        assert!(matches!(
            state.run_checks_pass(&Group::ALL),
            PassOutcome::Busy
        ));
        // The pass writes its progress without the session lock; a status query reads it.
        input.report_progress("texts", 312, 724);
        assert_eq!(
            state.lock_session().as_ref().unwrap().checks.progress(),
            Some(("texts".to_owned(), 312, 724))
        );
        assert!(matches!(
            state.finish_checks(input.run(&[])),
            PassOutcome::Done(_)
        ));
        assert!(
            state
                .lock_session()
                .as_ref()
                .unwrap()
                .checks
                .progress()
                .is_none()
        );
    }

    #[test]
    fn a_pass_of_a_closed_or_replaced_project_is_dropped() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let input = state.begin_checks().unwrap();
        // Reopened meanwhile: another session, not running.
        let project = Arc::new(Project::open(fixture.dir.path()).unwrap());
        *state.lock_session() = Some(Session::new(project));
        assert!(matches!(
            state.finish_checks(input.run(&[Group::Cheap])),
            PassOutcome::NoSession
        ));
        assert!(state.lock_session().as_ref().unwrap().checks.last.is_none());
        *state.lock_session() = None;
        assert!(matches!(
            state.run_checks_pass(&Group::ALL),
            PassOutcome::NoSession
        ));
    }

    #[test]
    fn a_pass_moves_to_a_worker_thread() {
        // Tauri's `spawn_blocking` takes the input; the checks share the snapshot.
        fn worker<T: Send + 'static>() {}
        fn shared<T: Send + Sync>() {}
        worker::<PassInput>();
        worker::<PassResult>();
        shared::<Snapshot>();
    }

    #[test]
    fn a_panicking_pass_keeps_no_report_and_asks_for_no_rerun() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        // Store generation 0, as after opening: an empty report of generation 0 would look
        // current.
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            state.run_checks_pass_with(&Group::ALL, |input, _| {
                input.report_progress("texts", 3, 10);
                panic!("reading the texts failed");
            })
        }));
        assert!(panicked.is_err());
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert!(checks.last.is_none());
            assert!(!checks.running() && !checks.rerun);
            assert!(checks.progress().is_none());
        }
        // The same after a change (generation 1): no stale report, no rerun loop.
        add_warp(&state);
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            state.run_checks_pass_with(&Group::ALL, |_, _| panic!("again"))
        }));
        assert!(panicked.is_err());
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert!(checks.last.is_none() && !checks.running() && !checks.rerun);
        }
        // The next pass runs normally.
        assert!(matches!(
            state.run_checks_pass(&Group::ALL),
            PassOutcome::Done(_)
        ));
    }

    #[test]
    fn an_abandoned_pass_stops_running() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let input = state.begin_checks().unwrap();
        input.report_progress("texts", 1, 2);
        assert!(matches!(state.begin_checks(), Err(PassOutcome::Busy)));
        drop(input);
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert!(!checks.running() && checks.progress().is_none());
        }
        // Also a result that never reaches `finish_checks`.
        let result = state.begin_checks().unwrap().run(&[Group::Cheap]);
        assert!(state.lock_session().as_ref().unwrap().checks.running());
        drop(result);
        assert!(!state.lock_session().as_ref().unwrap().checks.running());
        assert!(matches!(
            state.run_checks_pass(&[Group::Cheap]),
            PassOutcome::Done(_)
        ));
    }

    #[test]
    fn only_a_report_of_the_current_generation_is_shown() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        assert!(matches!(
            state.run_checks_pass(&[Group::Cheap]),
            PassOutcome::Done(_)
        ));
        let generation = |state: &AppState| {
            state
                .lock_session()
                .as_ref()
                .unwrap()
                .store
                .change_generation()
        };
        let now = generation(&state);
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert_eq!(checks.current_report(now).unwrap().generation, now);
        }
        add_warp(&state);
        let now = generation(&state);
        let guard = state.lock_session();
        let checks = &guard.as_ref().unwrap().checks;
        assert!(checks.last.is_some());
        assert!(checks.current_report(now).is_none());
    }

    #[test]
    fn a_pass_is_sorted_out_by_the_ignore_list_it_finishes_with() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let PassOutcome::Done(first) = state.run_checks_pass(&[Group::Cheap]) else {
            panic!("the pass did not finish");
        };
        let split = first
            .findings
            .iter()
            .find(|f| f.code == "header.text-split")
            .unwrap()
            .clone();
        // The list changes while the next pass runs with its copy.
        let input = state.begin_checks().unwrap();
        state
            .lock_session()
            .as_mut()
            .unwrap()
            .checks
            .ignore
            .ignored
            .insert(g4_kernel::checks::IgnoreKey::of(&split));
        let PassOutcome::Done(report) = state.finish_checks(input.run(&[Group::Cheap])) else {
            panic!("the pass did not finish");
        };
        assert_eq!(report.ignored, std::slice::from_ref(&split));
        assert!(!report.findings.contains(&split));
        let guard = state.lock_session();
        let last = guard.as_ref().unwrap().checks.last.as_ref().unwrap();
        assert_eq!(last.ignored, [split]);
    }

    #[test]
    fn a_check_switched_on_during_a_pass_is_never_reported_as_disabled() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let code = "encounter.empty-slot";
        let switch = |on: bool| {
            let mut guard = state.lock_session();
            let ignore = &mut guard.as_mut().unwrap().checks.ignore;
            if on {
                ignore.disabled.remove(code);
            } else {
                ignore.disabled.insert(code.to_owned());
            }
        };
        let disabled = |report: &ChecksReport| {
            report
                .skipped
                .iter()
                .any(|(c, why)| *c == code && why == "disabled")
        };
        let current = || {
            let guard = state.lock_session();
            let session = guard.as_ref().unwrap();
            session
                .checks
                .current_report(session.store.change_generation())
                .cloned()
        };

        // Off: the report says so.
        switch(false);
        let PassOutcome::Done(off) = state.run_checks_pass(&[Group::Cheap]) else {
            panic!("the pass did not finish");
        };
        assert!(disabled(&off));
        // Switched on: that report lacks the check's findings and is no longer current.
        switch(true);
        assert!(current().is_none());

        // Switched on while a pass runs with the list it began with: the pass is discarded
        // and asks for a rerun, which reports the check.
        switch(false);
        let input = state.begin_checks().unwrap();
        switch(true);
        assert!(matches!(
            state.finish_checks(input.run(&[Group::Cheap])),
            PassOutcome::Stale
        ));
        assert!(state.lock_session().as_ref().unwrap().checks.rerun);
        assert!(current().is_none_or(|r| !disabled(&r)));
        let PassOutcome::Done(on) = state.run_checks_pass(&[Group::Cheap]) else {
            panic!("the pass did not finish");
        };
        assert!(!disabled(&on));
        assert_eq!(on.findings.iter().filter(|f| f.code == code).count(), 2);
        let kept = current().unwrap();
        assert_eq!((kept.findings, kept.skipped), (on.findings, on.skipped));
    }

    #[test]
    fn id_files_changed_names_new_changed_and_removed_files_once() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let mut guard = state.lock_session();
        let session = guard.as_mut().unwrap();
        assert!(session.id_files_changed().is_empty());
        let paths = id_file_paths(&session.project);
        let file = session.project.root.join(&paths[0]);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "{}\n").unwrap();
        assert_eq!(session.id_files_changed(), [paths[0].clone()]);
        assert!(session.id_files_changed().is_empty(), "told once");
        std::fs::remove_file(&file).unwrap();
        assert_eq!(session.id_files_changed(), [paths[0].clone()]);
        // The same length within one modification time (a rename and its undo): noticed by
        // the content.
        std::fs::write(&file, "{}\n").unwrap();
        assert_eq!(session.id_files_changed(), [paths[0].clone()]);
        let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::fs::write(&file, "[]\n").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
        assert_eq!(session.id_files_changed(), [paths[0].clone()]);
        let body =
            watched_files_changed(Path::new("/Projekte/Mein Hack#1"), &[(paths[0].clone(), 1)]);
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["method"], "workspace/didChangeWatchedFiles");
        let change = &body["params"]["changes"][0];
        assert_eq!(change["type"], 1);
        let uri = change["uri"].as_str().unwrap();
        assert!(
            uri.starts_with("file:///Projekte/Mein%20Hack%231/.rotom/"),
            "{uri}"
        );
    }

    #[test]
    fn exit_is_blocked_only_with_dirty_assets_and_without_force() {
        assert!(exit_is_blocked(1, false));
        assert!(!exit_is_blocked(0, false));
        assert!(!exit_is_blocked(3, true));
    }

    #[test]
    fn quit_asks_only_with_an_open_window_dirty_assets_and_no_force() {
        assert_eq!(quit_action(true, 1, false), QuitAction::Confirm);
        assert_eq!(quit_action(true, 0, false), QuitAction::Exit);
        assert_eq!(quit_action(true, 3, true), QuitAction::Exit);
        assert_eq!(quit_action(false, 2, false), QuitAction::Exit);
    }
}
