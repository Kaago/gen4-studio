//! Play mode live (3d.9): the `live_*` commands over the app's [`LiveHub`]. „Hier spielen“
//! starts melonDS with its GDB stub (`live_play`), `live_connect` reaches a stub that runs
//! already (also g4emu's), `live_set` writes a flag or variable into the running game, and
//! breakpoints sit at events (a figure's visibility flag, a trigger's variable).
//!
//! Every command runs off the main thread: the hub waits at most one packet timeout for the
//! stub, never the window.

use std::path::Path;

use g4_kernel::flags::{FlagKind, save_block};
use g4_kernel::live::Target;
use serde::Serialize;

use super::data::off_main;
use super::{get_events_impl, lock, session};
use crate::errors::CmdError;
use crate::live_hub::{BreakpointDto, EventRef, LiveStatusDto};
use crate::state::AppState;

/// `live_play`: whether the game runs live (`live`), why not (`reason`, the game was started as
/// before), and the PID of what was started.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LivePlayDto {
    pub live: bool,
    pub reason: Option<String>,
    pub pid: Option<u32>,
}

/// `live_set`: what was written; `read_back` is `None` until the next snapshot decided it.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LiveSetDto {
    pub target: Target,
    pub value: u16,
    pub read_back: Option<bool>,
}

/// The game code of the open project.
fn project_game(state: &AppState) -> Result<String, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.project.game_code.clone())
}

pub fn live_status_impl(state: &AppState) -> Result<LiveStatusDto, CmdError> {
    Ok(state.live.status())
}

/// Starts the ROM in melonDS with its GDB stub and connects (in the hub's worker). Falls back to
/// [`g4_kernel::run::launch_emulator`] with `live: false` and the reason when the emulator is no
/// melonDS 1.1 bundle or its config cannot be set up.
pub fn live_play_impl(
    state: &AppState,
    emulator: &str,
    rom: &str,
) -> Result<LivePlayDto, CmdError> {
    let game = project_game(state)?;
    state.live.play(Path::new(emulator), Path::new(rom), &game)
}

/// [`live_play_impl`] with the fallback launcher given (tests record the call instead of
/// running `open -a`).
#[cfg(test)]
pub(crate) fn live_play_with(
    state: &AppState,
    emulator: &str,
    rom: &str,
    launcher: &dyn Fn(&Path, &Path) -> Result<u32, CmdError>,
) -> Result<LivePlayDto, CmdError> {
    let game = project_game(state)?;
    state
        .live
        .play_with(Path::new(emulator), Path::new(rom), &game, launcher)
}

/// Connects to a stub that runs already (`host` defaults to `127.0.0.1`); answers at once with
/// state `connecting`, the hub's worker goes on.
pub fn live_connect_impl(
    state: &AppState,
    host: Option<String>,
    port: u16,
) -> Result<LiveStatusDto, CmdError> {
    let game = project_game(state)?;
    let host = host.filter(|h| !h.trim().is_empty());
    state
        .live
        .connect(host.as_deref().unwrap_or("127.0.0.1"), port, &game)
}

pub fn live_stop_impl(state: &AppState) -> Result<LiveStatusDto, CmdError> {
    Ok(state.live.stop())
}

pub fn live_resume_impl(state: &AppState) -> Result<LiveStatusDto, CmdError> {
    state.live.resume()
}

/// Writes a flag (0 or 1) or a variable into the running game; `not_live` without a
/// connection.
pub fn live_set_impl(
    state: &AppState,
    kind: FlagKind,
    number: u16,
    value: u16,
) -> Result<LiveSetDto, CmdError> {
    let target = match kind {
        FlagKind::Flag if value > 1 => return Err(CmdError::from("Ein Flag ist 0 oder 1")),
        FlagKind::Flag => Target::Flag { number },
        FlagKind::Var => Target::Var { number },
    };
    state.live.set(target, value)
}

pub fn live_breakpoints_impl(state: &AppState) -> Result<Vec<BreakpointDto>, CmdError> {
    Ok(state.live.status().breakpoints)
}

/// A breakpoint at an event: a figure halts on its visibility flag, a trigger on its variable.
pub fn live_breakpoint_set_impl(
    state: &AppState,
    event: EventRef,
) -> Result<BreakpointDto, CmdError> {
    let file = event.event_file;
    let index = usize::from(event.index);
    let events = get_events_impl(state, file)?;
    let (target, label) = match event.kind.as_str() {
        "object" => {
            let object = events
                .objects
                .get(index)
                .ok_or_else(|| format!("Eventdatei {file} hat keine Figur {index}"))?;
            if object.hidden_flag == 0 {
                return Err(CmdError::from("Die Figur hat kein Sichtbarkeits-Flag"));
            }
            (
                Target::Flag {
                    number: object.hidden_flag,
                },
                format!("Figur {index} · Eventdatei {file}"),
            )
        }
        "coord" => {
            let trigger = events
                .coords
                .get(index)
                .ok_or_else(|| format!("Eventdatei {file} hat keinen Auslöser {index}"))?;
            if trigger.var == 0 {
                return Err(CmdError::from("Der Auslöser hat keine Variable"));
            }
            (
                Target::Var {
                    number: trigger.var,
                },
                format!("Auslöser {index} · Eventdatei {file}"),
            )
        }
        _ => {
            return Err(CmdError::from(
                "Haltepunkte gibt es an Figuren und Auslösern",
            ));
        }
    };
    // Numbers outside the save are refused before anything is watched (a game without a
    // measured layout never connects, so its breakpoints are kept as they are).
    if let Some(block) = save_block(&project_game(state)?)
        && !in_save(&block, target)
    {
        return Err(target.outside_save().into());
    }
    state.live.add_breakpoint(event, target, label)
}

/// Whether `target` lies in the save of `block` (as the kernel's session checks it).
fn in_save(block: &g4_kernel::flags::SaveBlock, target: Target) -> bool {
    match target {
        Target::Flag { number } => u32::from(number) < block.flags_bytes * 8,
        Target::Var { number } => number
            .checked_sub(0x4000)
            .is_some_and(|i| u32::from(i) < block.vars_count),
    }
}

pub fn live_breakpoint_clear_impl(state: &AppState, id: u32) -> Result<bool, CmdError> {
    Ok(state.live.remove_breakpoint(id))
}

#[tauri::command]
pub async fn live_status(app: tauri::AppHandle) -> Result<LiveStatusDto, CmdError> {
    off_main(app, live_status_impl).await
}

#[tauri::command]
pub async fn live_play(
    app: tauri::AppHandle,
    emulator: String,
    rom: String,
) -> Result<LivePlayDto, CmdError> {
    off_main(app, move |state| live_play_impl(state, &emulator, &rom)).await
}

#[tauri::command]
pub async fn live_connect(
    app: tauri::AppHandle,
    host: Option<String>,
    port: u16,
) -> Result<LiveStatusDto, CmdError> {
    off_main(app, move |state| live_connect_impl(state, host, port)).await
}

#[tauri::command]
pub async fn live_stop(app: tauri::AppHandle) -> Result<LiveStatusDto, CmdError> {
    off_main(app, live_stop_impl).await
}

#[tauri::command]
pub async fn live_resume(app: tauri::AppHandle) -> Result<LiveStatusDto, CmdError> {
    off_main(app, live_resume_impl).await
}

#[tauri::command]
pub async fn live_set(
    app: tauri::AppHandle,
    kind: FlagKind,
    number: u16,
    value: u16,
) -> Result<LiveSetDto, CmdError> {
    off_main(app, move |state| live_set_impl(state, kind, number, value)).await
}

#[tauri::command]
pub async fn live_breakpoints(app: tauri::AppHandle) -> Result<Vec<BreakpointDto>, CmdError> {
    off_main(app, live_breakpoints_impl).await
}

#[tauri::command]
pub async fn live_breakpoint_set(
    app: tauri::AppHandle,
    event_file: u16,
    kind: String,
    index: u16,
) -> Result<BreakpointDto, CmdError> {
    let event = EventRef {
        event_file,
        kind,
        index,
    };
    off_main(app, move |state| live_breakpoint_set_impl(state, event)).await
}

#[tauri::command]
pub async fn live_breakpoint_clear(app: tauri::AppHandle, id: u32) -> Result<bool, CmdError> {
    off_main(app, move |state| live_breakpoint_clear_impl(state, id)).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        EventKind, EventPatch, add_event_impl, close_project_impl, open_project_impl,
        update_event_impl,
    };
    use crate::live_hub::{LiveState, Timings};
    use g4_kernel::live::fake_stub::{FakeStub, Quirks};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// Platinum DE: the u32 at `POINTER` points at `BLOCK`; the layout counts from `BASE`.
    const POINTER: u32 = 0x0210_1EE0;
    const BLOCK: u32 = 0x0227_E354;
    const BASE: u32 = BLOCK + 0x14;
    const FLAGS: u32 = BASE + 0xFEC;
    const GAME_CODE: u32 = 0x023F_FE0C;
    /// Flag 0x173 is bit 3 of byte 0x2E.
    const FLAG_173_BYTE: u32 = FLAGS + 0x2E;

    fn strict() -> bool {
        std::env::var("G4_TIMING_STRICT").is_ok_and(|v| v == "1")
    }

    /// Waits until `cond` holds: within `ms` under `G4_TIMING_STRICT=1`, 10 s otherwise (the
    /// machine is shared); prints how long it took.
    fn within(ms: u64, what: &str, mut cond: impl FnMut() -> bool) {
        let bound = Duration::from_millis(if strict() { ms } else { 10_000 });
        let started = Instant::now();
        while !cond() {
            assert!(started.elapsed() < bound, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(5));
        }
        eprintln!("{what}: {} ms", started.elapsed().as_millis());
    }

    /// The synthetic project as Platinum DE (`CPUD`, a game with a measured save layout).
    fn open_cpud() -> (g4_kernel::test_support::Synthetic, AppState) {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("header.yaml"),
            "title: \"POKEMON PL\"\ngamecode: CPUD\nmakercode: \"01\"\n",
        )
        .unwrap();
        let state = AppState::default();
        // Whether the user's melonDS runs never decides a test.
        state.live.set_running_check(|| false);
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        (fixture, state)
    }

    /// A melonDS 1.1 fake whose RAM holds Platinum DE with a loaded save, flag 0x173 set.
    fn platinum_stub() -> FakeStub {
        let stub = FakeStub::start(Quirks::melonds_1_1());
        stub.poke(GAME_CODE, b"CPUD");
        stub.poke(POINTER, &BLOCK.to_le_bytes());
        stub.poke(FLAG_173_BYTE, &[1 << 3]);
        stub
    }

    /// A notifier that counts its calls.
    fn count_events(state: &AppState) -> Arc<AtomicUsize> {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        state.live.set_notifier(Arc::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        calls
    }

    fn state_of(state: &AppState) -> LiveState {
        state.live.status().state
    }

    fn connected(state: &AppState, stub: &FakeStub) {
        live_connect_impl(state, None, stub.port).unwrap();
        // The first snapshot is the baseline the watches compare with.
        within(1000, "a live snapshot", || {
            let status = state.live.status();
            status.state == LiveState::Live && status.snapshot.is_some()
        });
    }

    /// An object in event file 0 whose visibility flag is `flag`; answers its index.
    fn figure_with_flag(state: &AppState, flag: u16) -> u16 {
        let index = add_event_impl(state, 0, EventKind::Object, 12, 13, None, None).unwrap();
        let patch = EventPatch {
            hidden_flag: Some(flag),
            ..EventPatch::default()
        };
        update_event_impl(state, 0, EventKind::Object, index, patch).unwrap();
        index
    }

    fn figure(index: u16) -> EventRef {
        EventRef {
            event_file: 0,
            kind: "object".into(),
            index,
        }
    }

    /// A file that runs: `#!/bin/sh` and `body`.
    fn script(path: &std::path::Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// `melonDS.app` in `dir` with `Contents/MacOS/melonDS` running `body` and the version.
    fn bundle(dir: &std::path::Path, version: &str, body: &str) -> PathBuf {
        let app = dir.join("melonDS.app");
        script(&app.join("Contents/MacOS/melonDS"), body);
        std::fs::write(
            app.join("Contents/Info.plist"),
            format!(
                "<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict>\n\
                 <key>CFBundleShortVersionString</key>\n<string>{version}</string>\n\
                 </dict></plist>"
            ),
        )
        .unwrap();
        app
    }

    /// The calls a [`recording`] launcher saw: (emulator, ROM).
    type Calls = Arc<Mutex<Vec<(PathBuf, PathBuf)>>>;

    /// A launcher that records its calls and answers PID 4242.
    fn recording() -> (
        Calls,
        impl Fn(&std::path::Path, &std::path::Path) -> Result<u32, CmdError>,
    ) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&calls);
        (
            calls,
            move |emulator: &std::path::Path, rom: &std::path::Path| {
                log.lock()
                    .unwrap()
                    .push((emulator.to_path_buf(), rom.to_path_buf()));
                Ok(4242)
            },
        )
    }

    #[test]
    fn live_commands_refuse_without_a_connection() {
        let (_fixture, state) = open_cpud();
        let err = live_set_impl(&state, FlagKind::Flag, 0x173, 0).unwrap_err();
        assert_eq!(err.code, "not_live");
        let err = live_resume_impl(&state).unwrap_err();
        assert_eq!(err.code, "not_live");
        let status = live_status_impl(&state).unwrap();
        assert_eq!(status.state, LiveState::Off);
        assert!(status.snapshot.is_none());
        // A flag is 0 or 1, refused before anything else.
        let err = live_set_impl(&state, FlagKind::Flag, 0x173, 2).unwrap_err();
        assert_eq!(err.message, "Ein Flag ist 0 oder 1");
    }

    #[test]
    fn connect_reads_the_snapshot_and_publishes_it() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        let calls = count_events(&state);
        let status = live_connect_impl(&state, None, stub.port).unwrap();
        assert!(
            matches!(status.state, LiveState::Connecting | LiveState::Live),
            "{:?}",
            status.state
        );
        within(1000, "a live snapshot", || {
            let s = state.live.status();
            s.state == LiveState::Live && s.snapshot.is_some()
        });
        let status = live_status_impl(&state).unwrap();
        assert_eq!(status.game.as_deref(), Some("CPUD"));
        assert_eq!(status.port, Some(stub.port));
        assert!(!status.own_process);
        assert_eq!(
            serde_json::to_value(status.game_check).unwrap(),
            serde_json::json!("verified")
        );
        let snapshot = status.snapshot.unwrap();
        assert_eq!(snapshot.flag(0x173), Some(true));
        assert!(calls.load(Ordering::SeqCst) >= 1);
        // The DTO speaks the frontend's names.
        let json = serde_json::to_value(live_status_impl(&state).unwrap()).unwrap();
        assert_eq!(json["state"], "live");
        assert!(json["snapshot"]["flags"].is_array());
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn publishing_skips_a_refresh_that_changed_nothing() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        state.live.set_timings(Timings {
            refresh: Duration::from_millis(20),
            ..Timings::default()
        });
        let calls = count_events(&state);
        connected(&state, &stub);
        std::thread::sleep(Duration::from_millis(100));
        let before = calls.load(Ordering::SeqCst);
        let revision = state.live.status().snapshot.unwrap().revision;
        std::thread::sleep(Duration::from_millis(300));
        assert!(state.live.status().snapshot.unwrap().revision > revision);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            before,
            "only the revision moved"
        );
        // A value the game changes is published.
        stub.poke(FLAG_173_BYTE, &[0]);
        within(1000, "the change to be published", || {
            calls.load(Ordering::SeqCst) > before
        });
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn set_over_the_command_path_writes_and_reads_back() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        connected(&state, &stub);
        let set = live_set_impl(&state, FlagKind::Flag, 0x173, 0).unwrap();
        assert_eq!(set.target, Target::Flag { number: 0x173 });
        assert_eq!(set.value, 0);
        assert_eq!(stub.peek(FLAG_173_BYTE, 1), [0]);
        within(600, "the read-back", || {
            state.live.status().read_back == Some(true)
        });
        let status = state.live.status();
        assert_eq!(status.snapshot.unwrap().flag(0x173), Some(false));
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn set_outside_the_save_is_refused_with_its_code() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        connected(&state, &stub);
        // 364 flag bytes hold 2912 flags; 288 variables from 0x4000.
        let err = live_set_impl(&state, FlagKind::Flag, 2912, 1).unwrap_err();
        assert_eq!(err.code, "outside_save");
        let err = live_set_impl(&state, FlagKind::Var, 0x4000 + 288, 1).unwrap_err();
        assert_eq!(err.code, "outside_save");
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn a_breakpoint_on_a_figure_halts_on_its_flag_and_resume_continues() {
        let (_fixture, state) = open_cpud();
        let index = figure_with_flag(&state, 0x173);
        let breakpoint = live_breakpoint_set_impl(&state, figure(index)).unwrap();
        assert_eq!(breakpoint.target, Target::Flag { number: 0x173 });
        assert_eq!(breakpoint.label, format!("Figur {index} · Eventdatei 0"));
        assert_eq!(
            live_breakpoints_impl(&state).unwrap(),
            std::slice::from_ref(&breakpoint)
        );
        // The same event again is the same breakpoint.
        assert_eq!(
            live_breakpoint_set_impl(&state, figure(index)).unwrap(),
            breakpoint
        );

        let stub = platinum_stub();
        connected(&state, &stub);
        // The game clears the flag (the figure shows up).
        stub.poke(FLAG_173_BYTE, &[0]);
        within(600, "the halt", || state_of(&state) == LiveState::Halted);
        let status = state.live.status();
        let halt = status.halt.unwrap();
        assert_eq!((halt.before, halt.after), (1, 0));
        assert_eq!(halt.watch.id, breakpoint.id);
        assert_eq!(halt.watch.label, breakpoint.label);
        assert!(stub.halted.load(Ordering::SeqCst), "the game stays halted");

        let status = live_resume_impl(&state).unwrap();
        assert_eq!(status.state, LiveState::Live);
        assert!(status.halt.is_none());
        within(1000, "the game to run", || {
            !stub.halted.load(Ordering::SeqCst)
        });
        assert!(live_breakpoint_clear_impl(&state, breakpoint.id).unwrap());
        assert!(!live_breakpoint_clear_impl(&state, breakpoint.id).unwrap());
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn a_breakpoint_on_a_door_or_a_flagless_figure_is_refused() {
        let (_fixture, state) = open_cpud();
        let flagless = figure_with_flag(&state, 0);
        let err = live_breakpoint_set_impl(&state, figure(flagless)).unwrap_err();
        assert_eq!(err.message, "Die Figur hat kein Sichtbarkeits-Flag");
        let coord = add_event_impl(&state, 0, EventKind::Coord, 4, 5, None, None).unwrap();
        let trigger = EventRef {
            event_file: 0,
            kind: "coord".into(),
            index: coord,
        };
        let err = live_breakpoint_set_impl(&state, trigger.clone()).unwrap_err();
        assert_eq!(err.message, "Der Auslöser hat keine Variable");
        let door = EventRef {
            event_file: 0,
            kind: "warp".into(),
            index: 0,
        };
        let err = live_breakpoint_set_impl(&state, door).unwrap_err();
        assert_eq!(err.message, "Haltepunkte gibt es an Figuren und Auslösern");
        // A trigger with a variable gets one, labelled as a trigger.
        let patch = EventPatch {
            var: Some(0x40A5),
            ..EventPatch::default()
        };
        update_event_impl(&state, 0, EventKind::Coord, coord, patch).unwrap();
        let breakpoint = live_breakpoint_set_impl(&state, trigger).unwrap();
        assert_eq!(breakpoint.target, Target::Var { number: 0x40A5 });
        assert_eq!(breakpoint.label, format!("Auslöser {coord} · Eventdatei 0"));
        // A visibility flag outside the save is refused with its code.
        let outside = figure_with_flag(&state, 2912);
        let err = live_breakpoint_set_impl(&state, figure(outside)).unwrap_err();
        assert_eq!(err.code, "outside_save");
    }

    #[test]
    fn a_closed_stub_ends_the_play_mode_and_keeps_the_breakpoints() {
        let (_fixture, state) = open_cpud();
        let index = figure_with_flag(&state, 0x173);
        live_breakpoint_set_impl(&state, figure(index)).unwrap();
        let stub = platinum_stub();
        connected(&state, &stub);
        // melonDS quits: every connection and the listener close.
        drop(stub);
        within(1000, "off", || state_of(&state) == LiveState::Off);
        let status = state.live.status();
        assert_eq!(status.reason.as_deref(), Some("melonDS wurde beendet"));
        assert!(status.snapshot.is_none());
        assert_eq!(status.breakpoints.len(), 1);
        // The next connection watches it again.
        let stub = platinum_stub();
        connected(&state, &stub);
        stub.poke(FLAG_173_BYTE, &[0]);
        within(1000, "the halt", || state_of(&state) == LiveState::Halted);
        live_stop_impl(&state).unwrap();
        assert_eq!(state.live.status().breakpoints.len(), 1);
    }

    #[test]
    fn a_silent_stub_turns_lost_blocks_nothing_and_reconnects() {
        let (_fixture, state) = open_cpud();
        state.live.set_timings(Timings {
            reconnect: Duration::from_millis(200),
            ..Timings::default()
        });
        // Per connection: the handshake, the game code, the pointer, `c`, then a few
        // snapshots, then silence.
        let stub = FakeStub::start(Quirks {
            answer_after: Some(12),
            ..Quirks::melonds_1_1()
        });
        stub.poke(GAME_CODE, b"CPUD");
        stub.poke(POINTER, &BLOCK.to_le_bytes());
        live_connect_impl(&state, None, stub.port).unwrap();
        // While the worker waits for the stub, the status answers at once.
        let mut slowest = Duration::ZERO;
        within(2000, "lost", || {
            let asked = Instant::now();
            let lost = state_of(&state) == LiveState::Lost;
            slowest = slowest.max(asked.elapsed());
            lost
        });
        eprintln!("slowest status while the worker waited: {slowest:?}");
        if strict() {
            assert!(
                slowest < Duration::from_millis(100),
                "status waited {slowest:?}"
            );
        }
        let status = live_status_impl(&state).unwrap();
        assert_eq!(
            status.reason.as_deref(),
            Some("melonDS antwortet nicht: melonDS neu starten")
        );
        // A new connection is answered again.
        within(5000, "live again", || state_of(&state) == LiveState::Live);
        live_stop_impl(&state).unwrap();
    }

    #[test]
    fn no_stub_until_the_deadline_is_unavailable_with_its_reason() {
        let (_fixture, state) = open_cpud();
        state.live.set_timings(Timings {
            connect_deadline: Duration::from_millis(600),
            connect_retry: Duration::from_millis(50),
            connect_timeout: Duration::from_millis(100),
            ..Timings::default()
        });
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);
        live_connect_impl(&state, None, port).unwrap();
        within(3000, "unavailable", || {
            state_of(&state) == LiveState::Unavailable
        });
        assert_eq!(
            state.live.status().reason,
            Some(format!("Kein GDB-Stub auf Port {port}"))
        );
        // A stub that accepts but never answers (another client holds melonDS).
        let silent = FakeStub::start(Quirks {
            answer: false,
            ..Quirks::melonds_1_1()
        });
        live_connect_impl(&state, None, silent.port).unwrap();
        within(3000, "unavailable", || {
            state_of(&state) == LiveState::Unavailable
        });
        assert_eq!(
            state.live.status().reason,
            Some(format!(
                "Kein GDB-Stub auf Port {} (lauscht ein anderes Programm?)",
                silent.port
            ))
        );
    }

    #[test]
    fn a_game_without_a_save_layout_is_unavailable_not_a_reconnect_loop() {
        // The plain synthetic project is Platinum US (CPUE): no measured layout.
        let (_fixture, state) = crate::commands::open_synthetic();
        let stub = FakeStub::start(Quirks::melonds_1_1());
        stub.poke(GAME_CODE, b"CPUE");
        live_connect_impl(&state, None, stub.port).unwrap();
        within(1000, "unavailable", || {
            state_of(&state) == LiveState::Unavailable
        });
        assert_eq!(
            state.live.status().reason.as_deref(),
            Some("Für CPUE ist die Lage des Spielstands nicht bekannt")
        );
        assert!(stub.log_lines().is_empty(), "never connected");
    }

    #[test]
    fn another_game_in_melonds_is_unavailable_and_nothing_is_written() {
        let (_fixture, state) = open_cpud();
        let stub = FakeStub::start(Quirks::melonds_1_1());
        stub.poke(GAME_CODE, b"IPKE");
        live_connect_impl(&state, None, stub.port).unwrap();
        within(1000, "unavailable", || {
            state_of(&state) == LiveState::Unavailable
        });
        assert_eq!(
            state.live.status().reason.as_deref(),
            Some("melonDS spielt IPKE, das Projekt ist CPUD")
        );
        assert!(!stub.log_lines().iter().any(|l| l.starts_with('M')));
    }

    #[test]
    fn close_project_stops_and_forgets_the_breakpoints() {
        let (_fixture, state) = open_cpud();
        let index = figure_with_flag(&state, 0x173);
        live_breakpoint_set_impl(&state, figure(index)).unwrap();
        let stub = platinum_stub();
        connected(&state, &stub);
        close_project_impl(&state).unwrap();
        let status = state.live.status();
        assert_eq!(status.state, LiveState::Off);
        assert!(status.breakpoints.is_empty());
        within(1000, "the stub to be left", || stub.open_connections() == 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn live_play_without_a_melonds_bundle_falls_back_and_says_why() {
        let (_fixture, state) = open_cpud();
        let dir = tempfile::tempdir().unwrap();
        state
            .live
            .set_config_path(Some(dir.path().join("melonDS.toml")));
        let emulator = dir.path().join("emulator.sh");
        script(&emulator, "exit 0");
        let rom = dir.path().join("game.nds");
        std::fs::write(&rom, b"").unwrap();
        let played =
            live_play_impl(&state, emulator.to_str().unwrap(), rom.to_str().unwrap()).unwrap();
        assert!(!played.live);
        assert!(played.pid.is_some());
        let reason = played.reason.unwrap();
        assert!(reason.starts_with("Kein melonDS-Bundle: "), "{reason}");
        let status = state.live.status();
        assert_eq!(status.state, LiveState::Unavailable);
        assert_eq!(status.reason, Some(reason));
        let err = live_play_impl(&state, "/nonexistent", rom.to_str().unwrap()).unwrap_err();
        assert_eq!(err.code, "tool_missing");
        assert!(
            err.message.contains("not found: /nonexistent"),
            "{}",
            err.message
        );
        assert!(
            !dir.path().join("melonDS.toml").exists(),
            "config untouched"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn live_play_with_an_old_bundle_falls_back() {
        let (_fixture, state) = open_cpud();
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("melonDS.toml");
        state.live.set_config_path(Some(config.clone()));
        let app = bundle(dir.path(), "1.0", "exit 0");
        let rom = dir.path().join("game.nds");
        let (calls, launcher) = recording();
        let played = live_play_with(
            &state,
            app.to_str().unwrap(),
            rom.to_str().unwrap(),
            &launcher,
        )
        .unwrap();
        assert_eq!(
            played,
            LivePlayDto {
                live: false,
                reason: Some("melonDS 1.1 oder neuer nötig (gefunden 1.0)".into()),
                pid: Some(4242),
            }
        );
        assert_eq!(*calls.lock().unwrap(), [(app, rom)]);
        assert!(!config.exists(), "config untouched");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn live_play_with_a_ready_bundle_launches_and_connects() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("melonDS.toml");
        let text = format!(
            "[Instance0.Gdb]\nEnabled = true\n\n[Instance0.Gdb.ARM9]\nBreakOnStartup = false\n\
             Port = {}\n\n[JIT]\nEnable = false\n",
            stub.port
        );
        std::fs::write(&config, &text).unwrap();
        state.live.set_config_path(Some(config.clone()));
        let app = bundle(dir.path(), "1.1", "exec sleep 30");
        let rom = dir.path().join("game.nds");
        let (calls, launcher) = recording();
        let played = live_play_with(
            &state,
            app.to_str().unwrap(),
            rom.to_str().unwrap(),
            &launcher,
        )
        .unwrap();
        assert!(played.live, "{played:?}");
        assert_eq!(played.reason, None);
        let pid = played.pid.unwrap();
        assert!(calls.lock().unwrap().is_empty(), "no fallback");
        within(1000, "live", || state_of(&state) == LiveState::Live);
        let status = state.live.status();
        assert!(status.own_process);
        assert_eq!(status.port, Some(stub.port));
        assert_eq!(status.melonds.unwrap().version, "1.1");
        assert_eq!(
            std::fs::read_to_string(&config).unwrap(),
            text,
            "ready: kept"
        );
        // Stopping ends our melonDS.
        let status = live_stop_impl(&state).unwrap();
        assert_eq!(status.state, LiveState::Off);
        assert!(!exists(pid), "melonDS was stopped");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn our_melonds_quitting_while_connecting_ends_the_play_mode() {
        let (_fixture, state) = open_cpud();
        let dir = tempfile::tempdir().unwrap();
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);
        let config = dir.path().join("melonDS.toml");
        std::fs::write(
            &config,
            format!(
                "[Instance0.Gdb]\nEnabled = true\n\n[Instance0.Gdb.ARM9]\n\
                 BreakOnStartup = false\nPort = {port}\n\n[JIT]\nEnable = false\n"
            ),
        )
        .unwrap();
        state.live.set_config_path(Some(config));
        let app = bundle(dir.path(), "1.1", "exit 0");
        let (_calls, launcher) = recording();
        let played = live_play_with(&state, app.to_str().unwrap(), "game.nds", &launcher).unwrap();
        assert!(played.live);
        within(3000, "off", || state_of(&state) == LiveState::Off);
        assert_eq!(
            state.live.status().reason.as_deref(),
            Some("melonDS wurde beendet")
        );
    }

    /// „Stoppen“ ends our melonDS, and its end closes the stub: the worker meeting the closed
    /// connection must not say „melonDS wurde beendet“ before the plain `off` of the stop.
    #[cfg(target_os = "macos")]
    #[test]
    fn stopping_our_melonds_never_says_it_quit() {
        let (_fixture, state) = open_cpud();
        let stub = Arc::new(Mutex::new(Some(platinum_stub())));
        let port = stub.lock().unwrap().as_ref().unwrap().port;
        let dir = tempfile::tempdir().unwrap();
        state
            .live
            .set_config_path(Some(ready_config(dir.path(), port)));
        state.live.set_timings(Timings {
            refresh: Duration::from_millis(20),
            ..Timings::default()
        });
        let published = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&published);
        state.live.set_notifier(Arc::new(move |s: &LiveStatusDto| {
            log.lock().unwrap().push((s.state, s.reason.clone()));
        }));
        // melonDS's end closes its stub; long enough for the worker's next refresh to meet it.
        let closing = Arc::clone(&stub);
        state.live.set_after_kill(Box::new(move || {
            drop(closing.lock().unwrap().take());
            std::thread::sleep(Duration::from_millis(200));
        }));
        let app = bundle(dir.path(), "1.1", "exec sleep 30");
        let (_calls, launcher) = recording();
        let played = live_play_with(&state, app.to_str().unwrap(), "game.nds", &launcher).unwrap();
        assert!(played.live, "{played:?}");
        within(1000, "live", || state_of(&state) == LiveState::Live);
        let status = live_stop_impl(&state).unwrap();
        assert_eq!((status.state, status.reason), (LiveState::Off, None));
        let seen = published.lock().unwrap().clone();
        assert!(
            !seen
                .iter()
                .any(|(_, reason)| reason.as_deref() == Some("melonDS wurde beendet")),
            "{seen:?}"
        );
        assert_eq!(seen.last(), Some(&(LiveState::Off, None)));
    }

    /// A melonDS that runs already (the user's own) is not launched a second time: with a ready
    /// config the studio would connect to whichever holds the port. The game starts as before
    /// (the fallback) and the reason says what to do.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_melonds_running_already_is_not_launched_again() {
        let (_fixture, state) = open_cpud();
        state.live.set_running_check(|| true);
        let stub = platinum_stub();
        let dir = tempfile::tempdir().unwrap();
        let config = ready_config(dir.path(), stub.port);
        let text = std::fs::read_to_string(&config).unwrap();
        state.live.set_config_path(Some(config.clone()));
        let app = bundle(dir.path(), "1.1", "exec sleep 30");
        let rom = dir.path().join("game.nds");
        let (calls, launcher) = recording();
        let played = live_play_with(
            &state,
            app.to_str().unwrap(),
            rom.to_str().unwrap(),
            &launcher,
        )
        .unwrap();
        assert_eq!(
            played,
            LivePlayDto {
                live: false,
                reason: Some("melonDS läuft schon: erst beenden, dann „Hier spielen“".into()),
                pid: Some(4242),
            }
        );
        assert_eq!(*calls.lock().unwrap(), [(app.clone(), rom.clone())]);
        let status = state.live.status();
        assert_eq!(status.state, LiveState::Unavailable);
        assert_eq!(status.reason, played.reason);
        assert!(!status.own_process);
        assert_eq!(status.melonds.unwrap().version, "1.1");
        assert_eq!(state.live.workers_alive(), 0);
        assert!(stub.log_lines().is_empty(), "never connected");
        assert_eq!(std::fs::read_to_string(&config).unwrap(), text);
        // A config that is not ready yet cannot be changed while melonDS runs.
        std::fs::write(&config, "").unwrap();
        let played = live_play_with(
            &state,
            app.to_str().unwrap(),
            rom.to_str().unwrap(),
            &launcher,
        )
        .unwrap();
        assert_eq!(
            played.reason.as_deref(),
            Some(
                "melonDS läuft schon; GDB-Stub oder JIT nicht einstellbar (Emu settings › Devtools)"
            )
        );
        assert_eq!(std::fs::read_to_string(&config).unwrap(), "");
    }

    #[test]
    fn a_lost_connection_keeps_trying_while_nobody_listens() {
        let (_fixture, state) = open_cpud();
        state.live.set_timings(Timings {
            reconnect: Duration::from_millis(100),
            ..Timings::default()
        });
        let stub = FakeStub::start(Quirks {
            answer_after: Some(12),
            ..Quirks::melonds_1_1()
        });
        stub.poke(GAME_CODE, b"CPUD");
        stub.poke(POINTER, &BLOCK.to_le_bytes());
        live_connect_impl(&state, None, stub.port).unwrap();
        within(2000, "lost", || state_of(&state) == LiveState::Lost);
        // The stub goes away (a foreign melonDS restarting takes longer than 2 s).
        drop(stub);
        std::thread::sleep(Duration::from_millis(600));
        assert_eq!(state_of(&state), LiveState::Lost);
        assert_eq!(state.live.workers_alive(), 1, "still trying");
        live_stop_impl(&state).unwrap();
        assert_eq!(state.live.workers_alive(), 0);
    }

    #[test]
    fn concurrent_connects_leave_one_worker_and_stop_leaves_none() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| live_connect_impl(&state, None, stub.port).unwrap());
            }
        });
        assert_eq!(state.live.workers_alive(), 1);
        within(1000, "live", || state_of(&state) == LiveState::Live);
        live_stop_impl(&state).unwrap();
        assert_eq!(state.live.workers_alive(), 0);
        within(1000, "the stub to be left", || stub.open_connections() == 0);
    }

    /// Whether process `pid` exists (a zombie counts).
    #[cfg(target_os = "macos")]
    fn exists(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    }

    /// A ready config for the stub at `port` in `dir`.
    #[cfg(target_os = "macos")]
    fn ready_config(dir: &std::path::Path, port: u16) -> PathBuf {
        let config = dir.join("melonDS.toml");
        std::fs::write(
            &config,
            format!(
                "[Instance0.Gdb]\nEnabled = true\n\n[Instance0.Gdb.ARM9]\n\
                 BreakOnStartup = false\nPort = {port}\n\n[JIT]\nEnable = false\n"
            ),
        )
        .unwrap();
        config
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn concurrent_plays_leave_no_melonds_behind() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        let dir = tempfile::tempdir().unwrap();
        state
            .live
            .set_config_path(Some(ready_config(dir.path(), stub.port)));
        let app = bundle(dir.path(), "1.1", "exec sleep 30");
        let (_calls, launcher) = recording();
        let pids: Vec<u32> = std::thread::scope(|scope| {
            let plays: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        live_play_with(&state, app.to_str().unwrap(), "game.nds", &launcher)
                            .unwrap()
                            .pid
                            .unwrap()
                    })
                })
                .collect();
            plays.into_iter().map(|p| p.join().unwrap()).collect()
        });
        assert_eq!(state.live.workers_alive(), 1);
        live_stop_impl(&state).unwrap();
        assert_eq!(state.live.workers_alive(), 0);
        for pid in pids {
            assert!(!exists(pid), "melonDS {pid} was left running");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn our_melonds_is_reaped_when_its_stub_closes_first() {
        let (_fixture, state) = open_cpud();
        let stub = platinum_stub();
        let dir = tempfile::tempdir().unwrap();
        state
            .live
            .set_config_path(Some(ready_config(dir.path(), stub.port)));
        // melonDS closes its stub, the process ends a moment later.
        let app = bundle(dir.path(), "1.1", "exec sleep 3");
        let (_calls, launcher) = recording();
        let pid = live_play_with(&state, app.to_str().unwrap(), "game.nds", &launcher)
            .unwrap()
            .pid
            .unwrap();
        within(1000, "live", || state_of(&state) == LiveState::Live);
        drop(stub);
        within(1000, "off", || state_of(&state) == LiveState::Off);
        let status = state.live.status();
        assert_eq!(status.reason.as_deref(), Some("melonDS wurde beendet"));
        assert!(!status.own_process);
        assert_eq!(status.port, None);
        within(1000, "the worker to end", || {
            state.live.workers_alive() == 0
        });
        // No zombie: once it exits, the process is reaped while the hub lives on.
        within(5000, "melonDS to be reaped", || !exists(pid));
    }
}
