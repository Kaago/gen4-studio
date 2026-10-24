use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use g4_kernel::checks::{ChecksReport, Group, IgnoreSet, Overlay, Runner, Snapshot, SnapshotCache};
use g4_kernel::data::names::NameTables;
use g4_kernel::data::refs::RefIndex;
use g4_kernel::project::Project;
use g4_kernel::sprites::SpriteCache;
use g4_kernel::store::Store;

use crate::lsp::LspProcess;

/// The open project and its asset store.
pub struct Session {
    pub project: Arc<Project>,
    pub store: Store,
    /// The overworld table and the decoded object sprites of this project.
    pub sprites: SpriteCache,
    /// The `rotom-lsp` of this project, started by `lsp_start` and stopped with the session.
    pub lsp: Option<LspProcess>,
    /// Names of species, moves, items, ... read from the store's text archives, cached per bank.
    pub names: NameTables,
    /// Who references a species, move, item, trainer or encounter file; built on first use.
    pub refs: RefIndex,
    /// The last check run, the one running and the checks' cache.
    pub checks: ChecksState,
}

impl Session {
    /// A session of `project` with a fresh store and empty caches.
    pub fn new(project: Arc<Project>) -> Session {
        let store = Store::new(Arc::clone(&project));
        Session {
            project,
            store,
            sprites: SpriteCache::new(),
            lsp: None,
            names: NameTables::new(),
            refs: RefIndex::new(),
            checks: ChecksState::new(),
        }
    }

    /// Brings the reference index up to date after a command changed the store. Cheap when
    /// the index was never built or nothing it reads changed. A failed refresh leaves the index
    /// as it was; its next use tries again (and reports the error there).
    pub fn after_write(&mut self) {
        let _ = self.refs.refresh(&self.store);
        self.checks.store_changed();
    }

    /// Forgets the reference index after the files on disk may have changed behind the store
    /// (a member reloaded from disk): the next use builds it again from the disk, which also
    /// sees members the store never loaded.
    pub fn disk_changed(&mut self) {
        self.refs = RefIndex::new();
        self.checks.store_changed();
    }
}

/// Progress of a running check pass: a short label (`cheap`, `scripts`, `texts`, `load`) the
/// UI words, files or checks done, and their total.
pub type ChecksProgress = (String, u32, u32);

/// The checks of a session (spec 3b.4 §5): the last current report, whether a pass runs and
/// how far it got, when the store last changed (for the debounced cheap run), the ignore list
/// and the cache of decoded text archives and script sources between runs.
pub struct ChecksState {
    /// The last report that was current when it finished; never one of an older generation.
    pub last: Option<ChecksReport>,
    pub running: bool,
    /// Set when a pass was discarded because the store changed during it: run again.
    pub rerun: bool,
    /// When the store last changed ([`ChecksState::store_changed`]): the automatic cheap run
    /// waits until it has been quiet for a while ([`ChecksState::quiet_for`]).
    pub debounce: Instant,
    /// Disabled checks and ignored findings (from `g4-checks.json`).
    pub ignore: IgnoreSet,
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
            running: false,
            rerun: false,
            debounce: Instant::now(),
            ignore: IgnoreSet::default(),
            progress: Arc::new(Mutex::new(None)),
            cache: Some(SnapshotCache::default()),
        }
    }

    /// How far the running pass got; `None` when none runs or it reported nothing yet.
    pub fn progress(&self) -> Option<ChecksProgress> {
        lock(&self.progress).clone()
    }

    /// Notes a change of the store: restarts the debounce.
    pub fn store_changed(&mut self) {
        self.debounce = Instant::now();
    }

    /// Whether the store has not changed for `quiet` (the debounce of the automatic run).
    pub fn quiet_for(&self, quiet: Duration) -> bool {
        self.debounce.elapsed() >= quiet
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// What [`AppState::run_checks_pass`] did.
#[derive(Debug)]
pub enum PassOutcome {
    /// The report, current with the store; also kept as [`ChecksState::last`].
    Done(ChecksReport),
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
    overlay: Overlay,
    cache: SnapshotCache,
    ignore: IgnoreSet,
    progress: Arc<Mutex<Option<ChecksProgress>>>,
}

/// A finished pass, for [`AppState::finish_checks`].
pub struct PassResult {
    project: Arc<Project>,
    cache: SnapshotCache,
    report: ChecksReport,
}

impl PassInput {
    /// Builds the snapshot and runs the checks of `groups` on this thread, writing the
    /// progress where `checks_status` reads it.
    pub fn run(self, groups: &[Group]) -> PassResult {
        let snap = Snapshot::build(Arc::clone(&self.project), self.overlay, self.cache);
        let progress = &self.progress;
        let report = Runner::run(
            &snap,
            Project::limits(),
            &self.ignore,
            groups,
            |label, done, total| report_progress(progress, label, done, total),
        );
        PassResult {
            project: self.project,
            cache: snap.into_cache(),
            report,
        }
    }

    /// Writes the progress of this pass.
    pub fn report_progress(&self, label: &str, done: u32, total: u32) {
        report_progress(&self.progress, label, done, total);
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
}

impl AppState {
    /// Runs one check pass over `groups` on the calling thread (a worker: Tauri's
    /// `spawn_blocking`, or the bridge's request thread): the overlay is taken under the
    /// session lock, the snapshot is built and checked without it, and the result is kept
    /// only when the store is still at the overlay's generation.
    pub fn run_checks_pass(&self, groups: &[Group]) -> PassOutcome {
        let input = match self.begin_checks() {
            Ok(input) => input,
            Err(outcome) => return outcome,
        };
        let project = Arc::clone(&input.project);
        match catch_unwind(AssertUnwindSafe(|| input.run(groups))) {
            Ok(result) => self.finish_checks(result),
            Err(panic) => {
                // Not left running forever: the session gets an empty cache and no report.
                self.finish_checks(PassResult {
                    project,
                    cache: SnapshotCache::default(),
                    report: ChecksReport::default(),
                });
                resume_unwind(panic)
            }
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
        if checks.running {
            return Err(PassOutcome::Busy);
        }
        checks.running = true;
        checks.rerun = false;
        *lock(&checks.progress) = None;
        Ok(PassInput {
            project: Arc::clone(&session.project),
            overlay: session.store.snapshot_overlay(),
            cache: checks.cache.take().unwrap_or_default(),
            ignore: checks.ignore.clone(),
            progress: Arc::clone(&checks.progress),
        })
    }

    /// Ends a pass under the session lock: keeps its cache, and its report when the store is
    /// still at its generation; else discards it and sets [`ChecksState::rerun`]. A session
    /// closed or replaced meanwhile gets nothing.
    pub fn finish_checks(&self, result: PassResult) -> PassOutcome {
        let mut guard = self.lock_session();
        let Some(session) = guard
            .as_mut()
            .filter(|s| Arc::ptr_eq(&s.project, &result.project))
        else {
            return PassOutcome::NoSession;
        };
        let current = result.report.is_current(&session.store);
        let checks = &mut session.checks;
        checks.running = false;
        *lock(&checks.progress) = None;
        checks.cache = Some(result.cache);
        if current {
            checks.last = Some(result.report.clone());
            PassOutcome::Done(result.report)
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
        assert!(!checks.running && !checks.rerun);
        assert!(checks.progress().is_none());
        assert!(checks.cache.is_some());
    }

    #[test]
    fn a_pass_over_an_old_generation_is_discarded_and_asks_for_a_rerun() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let input = state.begin_checks().unwrap();
        assert!(state.lock_session().as_ref().unwrap().checks.running);
        // The store changes while the pass runs without the lock.
        add_warp(&state);
        let result = input.run(&[Group::Cheap]);
        assert!(matches!(state.finish_checks(result), PassOutcome::Stale));
        {
            let guard = state.lock_session();
            let checks = &guard.as_ref().unwrap().checks;
            assert!(checks.last.is_none(), "a stale report is never kept");
            assert!(checks.rerun && !checks.running);
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
    fn a_write_restarts_the_debounce() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = open_state(&fixture);
        let mut guard = state.lock_session();
        let session = guard.as_mut().unwrap();
        session.checks.debounce = Instant::now() - Duration::from_secs(1);
        assert!(session.checks.quiet_for(Duration::from_millis(300)));
        session.after_write();
        assert!(!session.checks.quiet_for(Duration::from_millis(300)));
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
