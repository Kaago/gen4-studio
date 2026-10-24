use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Set while a pass runs; shared with it, which clears it when it ends in any way
    /// (finished, dropped or panicked, see [`PassGuard`]).
    running: Arc<AtomicBool>,
    /// Set when a pass was discarded because the store changed during it (or a check it
    /// left out was switched on): run again.
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
            running: Arc::new(AtomicBool::new(false)),
            rerun: false,
            debounce: Instant::now(),
            ignore: IgnoreSet::default(),
            progress: Arc::new(Mutex::new(None)),
            cache: Some(SnapshotCache::default()),
        }
    }

    /// Whether a pass runs.
    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// The last report if it is of the store's current generation `store_generation`
    /// (`Store::change_generation`) and of the current ignore list; `None` when there is
    /// none, the store changed since, or a check it left out as disabled was switched on
    /// since (its findings are missing), so a caller never shows findings of an older state.
    pub fn current_report(&self, store_generation: u64) -> Option<&ChecksReport> {
        self.last.as_ref().filter(|report| {
            report.generation == store_generation && !report.lacks_enabled(&self.ignore)
        })
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
    overlay: Overlay,
    cache: SnapshotCache,
    ignore: IgnoreSet,
    guard: PassGuard,
}

/// A finished pass, for [`AppState::finish_checks`].
pub struct PassResult {
    project: Arc<Project>,
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
    /// still at its generation and no check the pass left out as disabled was switched on
    /// meanwhile; else discards it and sets [`ChecksState::rerun`]. A session closed or
    /// replaced meanwhile gets nothing. Either way the pass no longer runs.
    pub fn finish_checks(&self, result: PassResult) -> PassOutcome {
        let PassResult {
            project,
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
        let current = report.is_current(&session.store) && !report.lacks_enabled(&checks.ignore);
        // Under the session lock: a status query never sees the pass ended without its
        // report and cache in place.
        drop(pass);
        checks.cache = Some(cache);
        if current {
            // The ignore list may have changed while the pass ran with its copy.
            let mut report = report;
            report.apply_ignore(&checks.ignore);
            checks.last = Some(report.clone());
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
