//! Play mode live in the app (3d.9): the connection to the GDB stub of the running melonDS
//! (or g4emu), a worker thread that keeps the snapshot fresh, the breakpoints at events and the
//! status the webview gets as `live-event`.
//!
//! Three locks, always taken in this order and never the other way round:
//! - `session` holds the [`LiveSession`]: it serialises refresh, set, watches and resume (the
//!   session keeps only the last pending write). It is held for one exchange with the stub,
//!   which waits at most one reply timeout per packet ([`g4_kernel::live::REPLY_TIMEOUT`]).
//! - `published` orders the notifier calls.
//! - `inner` holds the status: never held across a network wait, so [`LiveHub::status`] never
//!   waits for the stub.
//!
//! States: `starting` (melonDS launched) → `connecting` (retry every 500 ms up to 10 s, then
//! `unavailable` with the reason) → `waiting_for_save` / `live` / `halted` (refresh every
//! 250 ms, every second while halted). A stub that stops answering is `lost` (the session is
//! dropped, which lets melonDS run, and a new connection is tried every 2 s); a closed
//! connection or our melonDS ending is `off` with „melonDS wurde beendet“. Breakpoints survive
//! all of it; `close_project` ([`LiveHub::close`]) clears them.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use g4_kernel::live::melonds::{self, MelonDs};
use g4_kernel::live::{GameCheck, Halt, LiveError, LiveSession, REPLY_TIMEOUT, Snapshot, Target};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::commands::{LivePlayDto, LiveSetDto};
use crate::errors::CmdError;
use crate::state::AppState;

/// melonDS closed the connection, or our melonDS ended.
pub const QUIT: &str = "melonDS wurde beendet";
/// The stub stopped answering after the connection stood.
pub const NOT_ANSWERING: &str = "melonDS antwortet nicht: melonDS neu starten";
/// The pointer to the save is not valid yet (title screen, intro).
pub const WAITING_FOR_SAVE: &str = "Wartet auf den Spielstand (Spiel fortsetzen)";
/// melonDS runs already, so its config cannot be changed (it writes the file back on exit).
pub const RUNS_ALREADY: &str =
    "melonDS läuft schon; GDB-Stub oder JIT nicht einstellbar (Emu settings › Devtools)";
/// A command that needs the connection, without one.
pub const NOT_LIVE: &str = "Nicht live: melonDS ist nicht verbunden";

/// Where play mode stands.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveState {
    Off,
    Starting,
    Connecting,
    WaitingForSave,
    Live,
    Halted,
    Lost,
    Unavailable,
}

/// What `live_status` answers and `live-event` carries.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LiveStatusDto {
    pub state: LiveState,
    /// Why the state is what it is (`unavailable`, `lost`, `off` after melonDS quit,
    /// `waiting_for_save`).
    pub reason: Option<String>,
    /// The game code of the project the connection is for.
    pub game: Option<String>,
    pub game_check: Option<GameCheck>,
    pub port: Option<u16>,
    /// The studio launched melonDS itself (and ends it on stop).
    pub own_process: bool,
    /// The last snapshot while `live` or `halted`.
    pub snapshot: Option<Snapshot>,
    /// The watch halt, its `watch.id` the breakpoint's id.
    pub halt: Option<Halt>,
    pub breakpoints: Vec<BreakpointDto>,
    /// After `live_set`: `None` until the next snapshot, then whether the value held.
    pub read_back: Option<bool>,
    /// The melonDS of the last „Hier spielen“.
    pub melonds: Option<MelonDs>,
}

impl LiveStatusDto {
    /// Whether `other` shows the same: everything but the snapshot's revision and timing,
    /// which move with every refresh.
    fn same_view(&self, other: &LiveStatusDto) -> bool {
        let strip = |s: &LiveStatusDto| {
            let mut s = s.clone();
            if let Some(snapshot) = &mut s.snapshot {
                snapshot.revision = 0;
                snapshot.taken_ms = 0;
            }
            s
        };
        strip(self) == strip(other)
    }
}

/// An event of the project: `kind` is `object`, `coord` (or `warp`, refused).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventRef {
    pub event_file: u16,
    pub kind: String,
    pub index: u16,
}

/// A breakpoint at an event: the game halts when its flag or variable changes.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BreakpointDto {
    pub id: u32,
    pub event: EventRef,
    pub target: Target,
    pub label: String,
}

/// Gets every status that shows something new.
pub type Notifier = Arc<dyn Fn(&LiveStatusDto) + Send + Sync>;

/// The worker's rhythm; tests shorten it.
#[derive(Clone, Copy, Debug)]
pub struct Timings {
    /// Between two snapshots.
    pub refresh: Duration,
    /// Between two snapshots while a watch halts the game.
    pub halted_refresh: Duration,
    /// Between two connection attempts while `lost`.
    pub reconnect: Duration,
    /// Between two connection attempts while `connecting`.
    pub connect_retry: Duration,
    /// How long `connecting` tries before it is `unavailable`.
    pub connect_deadline: Duration,
    /// TCP connect and each handshake reply of one attempt.
    pub connect_timeout: Duration,
}

impl Default for Timings {
    fn default() -> Timings {
        Timings {
            refresh: Duration::from_millis(250),
            halted_refresh: Duration::from_secs(1),
            reconnect: Duration::from_secs(2),
            connect_retry: Duration::from_millis(500),
            connect_deadline: Duration::from_secs(10),
            connect_timeout: REPLY_TIMEOUT,
        }
    }
}

/// A breakpoint and the watch it is in the current session.
#[derive(Clone, Debug)]
struct Breakpoint {
    dto: BreakpointDto,
    watch: Option<u32>,
}

/// The status behind [`LiveHub::status`].
struct Inner {
    state: LiveState,
    reason: Option<String>,
    game: Option<String>,
    game_check: Option<GameCheck>,
    port: Option<u16>,
    own_process: bool,
    snapshot: Option<Snapshot>,
    halt: Option<Halt>,
    read_back: Option<bool>,
    melonds: Option<MelonDs>,
    breakpoints: Vec<Breakpoint>,
    next_breakpoint: u32,
    config_path: Option<PathBuf>,
    timings: Timings,
}

impl Default for Inner {
    fn default() -> Inner {
        Inner {
            state: LiveState::Off,
            reason: None,
            game: None,
            game_check: None,
            port: None,
            own_process: false,
            snapshot: None,
            halt: None,
            read_back: None,
            melonds: None,
            breakpoints: Vec::new(),
            next_breakpoint: 1,
            config_path: melonds::default_config_path(),
            timings: Timings::default(),
        }
    }
}

impl Inner {
    fn status(&self) -> LiveStatusDto {
        LiveStatusDto {
            state: self.state,
            reason: self.reason.clone(),
            game: self.game.clone(),
            game_check: self.game_check,
            port: self.port,
            own_process: self.own_process,
            snapshot: self.snapshot.clone(),
            halt: self.halt.clone(),
            breakpoints: self.breakpoints.iter().map(|b| b.dto.clone()).collect(),
            read_back: self.read_back,
            melonds: self.melonds.clone(),
        }
    }

    /// Back to no connection with `state` and `reason`; breakpoints stay, their watches go.
    fn disconnected(&mut self, state: LiveState, reason: Option<&str>) {
        self.state = state;
        self.reason = reason.map(str::to_owned);
        self.snapshot = None;
        self.halt = None;
        self.read_back = None;
        for breakpoint in &mut self.breakpoints {
            breakpoint.watch = None;
        }
    }

    /// The session's halt with the breakpoint's id instead of the session's watch id.
    fn halt_of(&self, halt: Option<&Halt>) -> Option<Halt> {
        let mut halt = halt?.clone();
        if let Some(breakpoint) = self
            .breakpoints
            .iter()
            .find(|b| b.watch == Some(halt.watch.id))
        {
            halt.watch.id = breakpoint.dto.id;
        }
        Some(halt)
    }

    /// Takes over what the session knows after a snapshot (or `NoSave`).
    fn take_session(&mut self, session: &LiveSession, state: LiveState) {
        self.halt = self.halt_of(session.halt());
        self.state = if self.halt.is_some() {
            LiveState::Halted
        } else {
            state
        };
        self.reason = (self.state == LiveState::WaitingForSave).then(|| WAITING_FOR_SAVE.into());
        self.snapshot = session.last().cloned();
        self.read_back = session.read_back();
    }
}

/// What the hub and its worker share.
#[derive(Default)]
struct Shared {
    session: Mutex<Option<LiveSession>>,
    published: Mutex<Option<LiveStatusDto>>,
    inner: Mutex<Inner>,
    /// The melonDS we launched.
    child: Mutex<Option<Child>>,
    notifier: Mutex<Option<Notifier>>,
}

/// A poisoned lock (a panic while held) is taken over: the status is plain data.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    fn inner(&self) -> MutexGuard<'_, Inner> {
        lock(&self.inner)
    }

    fn status(&self) -> LiveStatusDto {
        self.inner().status()
    }

    /// Hands the status to the notifier when it shows something new (not on a revision alone).
    fn publish(&self) {
        let mut last = lock(&self.published);
        let status = self.status();
        if last.as_ref().is_some_and(|l| l.same_view(&status)) {
            return;
        }
        let notifier = lock(&self.notifier).clone();
        *last = Some(status.clone());
        if let Some(notify) = notifier {
            notify(&status);
        }
    }

    /// Whether our melonDS has ended (it is reaped then).
    fn own_child_ended(&self) -> bool {
        let mut child = lock(&self.child);
        let ended = child
            .as_mut()
            .is_some_and(|c| !matches!(c.try_wait(), Ok(None)));
        if ended {
            *child = None;
        }
        ended
    }

    /// After an error of an established session: the session is dropped (closing the socket
    /// lets melonDS run). A closed connection is `off`, anything else `lost` (reconnect).
    fn fail(&self, slot: &mut Option<LiveSession>, e: &LiveError) {
        *slot = None;
        let mut inner = self.inner();
        match e {
            LiveError::Closed => inner.disconnected(LiveState::Off, Some(QUIT)),
            _ => inner.disconnected(LiveState::Lost, Some(NOT_ANSWERING)),
        }
    }

    /// A new session: the breakpoints become its watches.
    fn install(&self, mut session: LiveSession, check: GameCheck) {
        let mut slot = lock(&self.session);
        let mut inner = self.inner();
        for breakpoint in &mut inner.breakpoints {
            breakpoint.watch = session
                .add_watch(breakpoint.dto.target, breakpoint.dto.label.clone())
                .ok()
                .map(|w| w.id);
        }
        inner.game_check = Some(check);
        inner.snapshot = None;
        inner.halt = None;
        inner.read_back = None;
        inner.state = if session.has_save() {
            LiveState::Live
        } else {
            LiveState::WaitingForSave
        };
        inner.reason = (inner.state == LiveState::WaitingForSave).then(|| WAITING_FOR_SAVE.into());
        *slot = Some(session);
    }

    /// One snapshot; answers how long to wait before the next one, `None` when the
    /// connection ended.
    fn refresh(&self, timings: &Timings) -> Option<Duration> {
        let mut slot = lock(&self.session);
        let session = slot.as_mut()?;
        let result = session.refresh().map(|_| ());
        match result {
            Ok(()) => self.inner().take_session(session, LiveState::Live),
            Err(LiveError::NoSave) => self
                .inner()
                .take_session(session, LiveState::WaitingForSave),
            Err(e) => {
                self.fail(&mut slot, &e);
                return None;
            }
        }
        Some(if session_halted(&slot) {
            timings.halted_refresh
        } else {
            timings.refresh
        })
    }
}

fn session_halted(slot: &Option<LiveSession>) -> bool {
    slot.as_ref().is_some_and(|s| s.halt().is_some())
}

/// The reason after `connecting` ran out of time: nobody listened, or somebody listened but
/// never answered (another client holds melonDS's single stub).
fn no_stub(port: u16, accepted: bool) -> String {
    if accepted {
        format!("Kein GDB-Stub auf Port {port} (lauscht ein anderes Programm?)")
    } else {
        format!("Kein GDB-Stub auf Port {port}")
    }
}

/// An error that no retry mends: the game has no known save layout, or another game runs.
fn final_error(e: &LiveError) -> bool {
    matches!(e, LiveError::WrongGame { .. })
        || matches!(e, LiveError::Io(io) if io.kind() == ErrorKind::Unsupported)
}

/// Sleeps `duration` in small steps; `true` when told to stop meanwhile.
fn sleep_or_stop(stop: &AtomicBool, duration: Duration) -> bool {
    let until = Instant::now() + duration;
    loop {
        if stop.load(Ordering::SeqCst) {
            return true;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        std::thread::sleep(left.min(Duration::from_millis(10)));
    }
}

/// The worker: connects, refreshes, reconnects; ends with `off` or `unavailable`, or when
/// told to stop.
fn work(shared: Arc<Shared>, stop: Arc<AtomicBool>, addr: String, port: u16, game: String) {
    let timings = shared.inner().timings;
    let deadline = Instant::now() + timings.connect_deadline;
    let mut accepted = false;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        if shared.own_child_ended() {
            let mut slot = lock(&shared.session);
            *slot = None;
            shared.inner().disconnected(LiveState::Off, Some(QUIT));
            drop(slot);
            shared.publish();
            return;
        }
        let state = shared.inner().state;
        let pause = match state {
            LiveState::Starting | LiveState::Connecting | LiveState::Lost => {
                let lost = state == LiveState::Lost;
                if !lost {
                    shared.inner().state = LiveState::Connecting;
                    shared.publish();
                }
                match LiveSession::connect(&addr, &game, timings.connect_timeout) {
                    Ok((session, check)) => {
                        shared.install(session, check);
                        Duration::ZERO
                    }
                    Err(e) if final_error(&e) => {
                        shared
                            .inner()
                            .disconnected(LiveState::Unavailable, Some(&e.to_string()));
                        shared.publish();
                        return;
                    }
                    // Nobody listens any more: the stub (melonDS) is gone.
                    Err(LiveError::Connect { .. }) if lost => {
                        shared.inner().disconnected(LiveState::Off, Some(QUIT));
                        shared.publish();
                        return;
                    }
                    Err(e) => {
                        accepted |= matches!(e, LiveError::NoReply);
                        if lost {
                            timings.reconnect
                        } else if Instant::now() >= deadline {
                            let reason = no_stub(port, accepted);
                            shared
                                .inner()
                                .disconnected(LiveState::Unavailable, Some(&reason));
                            shared.publish();
                            return;
                        } else {
                            timings.connect_retry
                        }
                    }
                }
            }
            LiveState::WaitingForSave | LiveState::Live | LiveState::Halted => {
                match shared.refresh(&timings) {
                    Some(pause) => pause,
                    None => {
                        let state = shared.inner().state;
                        shared.publish();
                        if state != LiveState::Lost {
                            return;
                        }
                        timings.reconnect
                    }
                }
            }
            LiveState::Off | LiveState::Unavailable => return,
        };
        shared.publish();
        if sleep_or_stop(&stop, pause) {
            return;
        }
    }
}

/// The running worker.
struct Worker {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Play mode live of the app: one connection at a time, its worker and the breakpoints.
#[derive(Default)]
pub struct LiveHub {
    shared: Arc<Shared>,
    worker: Mutex<Option<Worker>>,
}

impl LiveHub {
    /// The current status; never waits for the stub.
    pub fn status(&self) -> LiveStatusDto {
        self.shared.status()
    }

    /// Every status that shows something new goes to `f` (the app emits `live-event`).
    pub fn set_notifier(&self, f: Notifier) {
        *lock(&self.shared.notifier) = Some(f);
    }

    /// The melonDS config `play` reads and changes; `None` refuses the live start. Defaults
    /// to [`melonds::default_config_path`]; tests point it into a temp folder.
    pub fn set_config_path(&self, path: Option<PathBuf>) {
        self.shared.inner().config_path = path;
    }

    /// The worker's rhythm for the next connection (tests shorten it).
    pub fn set_timings(&self, timings: Timings) {
        self.shared.inner().timings = timings;
    }

    /// Connects to the stub at `host:port` for the project's `game_code`. Answers at once
    /// (`connecting`); the worker goes on. A running play mode stops first.
    pub fn connect(
        &self,
        host: &str,
        port: u16,
        game_code: &str,
    ) -> Result<LiveStatusDto, CmdError> {
        self.stop();
        self.begin(port, game_code, LiveState::Connecting, false, None);
        Ok(self.start_worker(host, port, game_code))
    }

    /// „Hier spielen“ with the production launcher ([`g4_kernel::run::launch_emulator`]).
    pub fn play(
        &self,
        bundle: &Path,
        rom: &Path,
        game_code: &str,
    ) -> Result<LivePlayDto, CmdError> {
        self.play_with(bundle, rom, game_code, &|emulator, rom| {
            g4_kernel::run::launch_emulator(emulator, rom).map_err(CmdError::from)
        })
    }

    /// „Hier spielen“: a melonDS 1.1 bundle starts with its GDB stub switched on (the config
    /// is changed only when melonDS does not run) and the worker connects; anything else
    /// starts through `launcher` as before, with `live: false` and the reason (state
    /// `unavailable`). Bundle and version are checked before the config is read. Waits only
    /// for the launch.
    pub fn play_with(
        &self,
        bundle: &Path,
        rom: &Path,
        game_code: &str,
        launcher: &dyn Fn(&Path, &Path) -> Result<u32, CmdError>,
    ) -> Result<LivePlayDto, CmdError> {
        self.stop();
        let config_path = self.shared.inner().config_path.clone();
        let (melonds, port) = match prepare(bundle, config_path.as_deref()) {
            Ok(ready) => ready,
            Err(refusal) => {
                let pid = launcher(bundle, rom)?;
                {
                    let mut inner = self.shared.inner();
                    inner.disconnected(LiveState::Unavailable, Some(&refusal.reason));
                    inner.game = Some(game_code.to_owned());
                    inner.port = None;
                    inner.own_process = false;
                    inner.game_check = None;
                    inner.melonds = refusal.melonds;
                }
                self.shared.publish();
                return Ok(LivePlayDto {
                    live: false,
                    reason: Some(refusal.reason),
                    pid: Some(pid),
                });
            }
        };
        let child = melonds::launch(&melonds.binary, rom)?;
        let pid = child.id();
        *lock(&self.shared.child) = Some(child);
        self.begin(port, game_code, LiveState::Starting, true, Some(melonds));
        self.start_worker("127.0.0.1", port, game_code);
        Ok(LivePlayDto {
            live: true,
            reason: None,
            pid: Some(pid),
        })
    }

    /// Ends play mode: the worker stops, the stub is detached (the game runs on), our
    /// melonDS ends. Breakpoints stay.
    pub fn stop(&self) -> LiveStatusDto {
        self.stop_worker();
        if let Some(session) = lock(&self.shared.session).take() {
            // Best effort: closing the socket lets melonDS run as well.
            let _ = session.detach();
        }
        if let Some(mut child) = lock(&self.shared.child).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        {
            let mut inner = self.shared.inner();
            inner.disconnected(LiveState::Off, None);
            inner.game_check = None;
            inner.port = None;
            inner.own_process = false;
        }
        self.shared.publish();
        self.status()
    }

    /// [`LiveHub::stop`] and the breakpoints go (the project closes).
    pub fn close(&self) {
        self.stop();
        self.shared.inner().breakpoints.clear();
        self.shared.publish();
    }

    /// Lets the game run out of a watch halt; a no-op while it runs.
    pub fn resume(&self) -> Result<LiveStatusDto, CmdError> {
        let mut slot = lock(&self.shared.session);
        let Some(session) = slot.as_mut() else {
            return Err(not_live());
        };
        if session.halt().is_some() {
            if let Err(e) = session.resume() {
                self.shared.fail(&mut slot, &e);
                drop(slot);
                self.shared.publish();
                return Err(e.into());
            }
            let mut inner = self.shared.inner();
            inner.halt = None;
            inner.state = LiveState::Live;
        }
        drop(slot);
        self.shared.publish();
        Ok(self.status())
    }

    /// Writes `value` into the running game; the next snapshot decides `read_back`. Not a
    /// step in any history.
    pub fn set(&self, target: Target, value: u16) -> Result<LiveSetDto, CmdError> {
        let mut slot = lock(&self.shared.session);
        let Some(session) = slot.as_mut() else {
            return Err(not_live());
        };
        match session.set(target, value) {
            Ok(()) => {
                self.shared.inner().read_back = None;
            }
            Err(e @ (LiveError::NoSave | LiveError::OutsideSave { .. })) => {
                return Err(e.into());
            }
            Err(e) => {
                self.shared.fail(&mut slot, &e);
                drop(slot);
                self.shared.publish();
                return Err(e.into());
            }
        }
        drop(slot);
        self.shared.publish();
        let value = match target {
            Target::Flag { .. } => u16::from(value != 0),
            Target::Var { .. } => value,
        };
        Ok(LiveSetDto {
            target,
            value,
            read_back: None,
        })
    }

    /// A breakpoint at `event`; the same event again answers the one it has. Without a
    /// connection it becomes a watch at the next one.
    pub fn add_breakpoint(
        &self,
        event: EventRef,
        target: Target,
        label: String,
    ) -> Result<BreakpointDto, CmdError> {
        let mut slot = lock(&self.shared.session);
        let mut inner = self.shared.inner();
        if let Some(found) = inner.breakpoints.iter().find(|b| b.dto.event == event) {
            return Ok(found.dto.clone());
        }
        let watch = match slot.as_mut() {
            Some(session) => Some(session.add_watch(target, label.clone())?.id),
            None => None,
        };
        let dto = BreakpointDto {
            id: inner.next_breakpoint,
            event,
            target,
            label,
        };
        inner.next_breakpoint += 1;
        inner.breakpoints.push(Breakpoint {
            dto: dto.clone(),
            watch,
        });
        drop(inner);
        drop(slot);
        self.shared.publish();
        Ok(dto)
    }

    /// Removes a breakpoint; `false` for an unknown id. A halt it caused stays until resumed.
    pub fn remove_breakpoint(&self, id: u32) -> bool {
        let mut slot = lock(&self.shared.session);
        let mut inner = self.shared.inner();
        let Some(at) = inner.breakpoints.iter().position(|b| b.dto.id == id) else {
            return false;
        };
        let removed = inner.breakpoints.remove(at);
        if let (Some(session), Some(watch)) = (slot.as_mut(), removed.watch) {
            session.remove_watch(watch);
        }
        drop(inner);
        drop(slot);
        self.shared.publish();
        true
    }

    /// The status of a new connection attempt.
    /// The melonDS found by the last „Hier spielen“ stays when `melonds` is `None`.
    fn begin(
        &self,
        port: u16,
        game_code: &str,
        state: LiveState,
        own_process: bool,
        melonds: Option<MelonDs>,
    ) {
        let mut inner = self.shared.inner();
        inner.disconnected(state, None);
        inner.game = Some(game_code.to_owned());
        inner.game_check = None;
        inner.port = Some(port);
        inner.own_process = own_process;
        if melonds.is_some() {
            inner.melonds = melonds;
        }
    }

    /// Starts the worker for `host:port`; answers the status as it was before the worker ran.
    fn start_worker(&self, host: &str, port: u16, game_code: &str) -> LiveStatusDto {
        let status = self.status();
        self.shared.publish();
        let addr = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]:{port}")
        } else {
            format!("{host}:{port}")
        };
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&self.shared);
        let game = game_code.to_owned();
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || work(shared, flag, addr, port, game));
        *lock(&self.worker) = Some(Worker { stop, handle });
        status
    }

    /// Tells the worker to stop and waits for it (at most one exchange with the stub).
    fn stop_worker(&self) {
        if let Some(worker) = lock(&self.worker).take() {
            worker.stop.store(true, Ordering::SeqCst);
            let _ = worker.handle.join();
        }
    }
}

impl Drop for LiveHub {
    fn drop(&mut self) {
        self.stop_worker();
        if let Some(mut child) = lock(&self.shared.child).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn not_live() -> CmdError {
    CmdError::refused("not_live", NOT_LIVE)
}

/// Why „Hier spielen“ starts without the live connection, and what was found of melonDS.
struct Refusal {
    reason: String,
    melonds: Option<MelonDs>,
}

impl Refusal {
    fn new(reason: impl Into<String>) -> Box<Refusal> {
        Box::new(Refusal {
            reason: reason.into(),
            melonds: None,
        })
    }
}

/// Checks the bundle (binary, then version), then reads the config and switches the stub on
/// when it is off and melonDS does not run. Answers what was found and the stub's port.
fn prepare(bundle: &Path, config_path: Option<&Path>) -> Result<(MelonDs, u16), Box<Refusal>> {
    if !cfg!(target_os = "macos") {
        return Err(Refusal::new("Spielmodus live gibt es nur unter macOS"));
    }
    if melonds::bundle_binary(bundle).is_none() {
        return Err(Refusal::new(format!(
            "Kein melonDS-Bundle: {} hat kein Contents/MacOS/melonDS",
            bundle.display()
        )));
    }
    let version = melonds::bundle_version(bundle).unwrap_or_default();
    if !melonds::version_ok(&version) {
        let found = if version.is_empty() {
            "keine Version gefunden".to_owned()
        } else {
            format!("gefunden {version}")
        };
        return Err(Refusal::new(format!(
            "melonDS 1.1 oder neuer nötig ({found})"
        )));
    }
    let Some(config_path) = config_path else {
        return Err(Refusal::new(
            "Keine melonDS-Konfiguration gefunden (HOME fehlt)",
        ));
    };
    let Some(mut found) = melonds::inspect(bundle, Some(config_path)) else {
        return Err(Refusal::new(format!(
            "Kein melonDS-Bundle: {} hat kein Contents/MacOS/melonDS",
            bundle.display()
        )));
    };
    if let Some(config) = found.config.as_ref().filter(|c| melonds::stub_ready(c)) {
        let port = config.port;
        return Ok((found, port));
    }
    if melonds::is_running() {
        return Err(Box::new(Refusal {
            reason: RUNS_ALREADY.to_owned(),
            melonds: Some(found),
        }));
    }
    match melonds::enable_stub(config_path) {
        Ok(config) => {
            let port = config.port;
            found.config = Some(config);
            Ok((found, port))
        }
        Err(e) => Err(Box::new(Refusal {
            reason: format!("melonDS-Konfiguration nicht änderbar: {e}"),
            melonds: Some(found),
        })),
    }
}

/// At the app's exit: detach, end our melonDS, stop the worker.
pub fn stop_on_exit(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.live.stop();
    }
}
