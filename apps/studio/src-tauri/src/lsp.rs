//! The `rotom-lsp` sidecar: one long-lived language-server process per open project, spoken to
//! over stdio with `Content-Length` framing and bridged to the frontend through a Tauri channel.
//! No port is opened, so the CSP stays as it is.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use g4_kernel::error::KernelError;
use g4_kernel::export::kill_tree;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::commands::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// A dead server is restarted by the next `lsp_send` only when this long has passed since its
/// last start; a faster death is a crash loop and the server stays off until `lsp_start`.
const RESTART_AFTER: Duration = Duration::from_secs(60);

/// Longest body accepted from the server. A larger `Content-Length` is a protocol error.
const MAX_BODY: usize = 64 * 1024 * 1024;

/// How long `stop` waits for each helper thread. They end at once after the kill; the bound only
/// matters when a process outside the killed group still holds a pipe open.
const JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// Frames queued for the writer thread before `send` gives up (a server that does not read).
const WRITE_QUEUE: usize = 256;

/// Shown when the binary is missing; the editor works without language services.
const MISSING: &str = "rotom-lsp fehlt — Completion und Diagnosen aus";

/// Finds the rotom-lsp executable: `G4_ROTOM_LSP`, then `rotom-lsp` next to the running binary,
/// then (debug builds only) the repository's `tools/rotom/bin/rotom-lsp`. A set `G4_ROTOM_LSP`
/// always wins, even when it points nowhere, so a typo is reported instead of silently ignored.
/// `libnitroarc_ffi.dylib` must sit next to the binary or in `../Frameworks` (see
/// `docs/dev-setup.md`).
pub fn find_rotom_lsp() -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("G4_ROTOM_LSP").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(env));
    }
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("rotom-lsp")));
    let mut candidates = vec![beside_exe];
    if cfg!(debug_assertions) {
        candidates.push(Some(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/rotom/bin/rotom-lsp"
        ))));
    }
    // An empty file is the build-time placeholder for the bundled sidecar (see build.rs).
    candidates.into_iter().flatten().find(|p| is_real_file(p))
}

fn is_real_file(path: &Path) -> bool {
    path.metadata()
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

/// `body` as one LSP message: `Content-Length` counts bytes, not characters.
pub fn frame(body: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// Removes every complete frame from the front of `buf` and returns the bodies. A partial frame
/// stays in `buf` for the next read. Headers other than `Content-Length` are ignored and header
/// names are case-insensitive; a header block without a length, or with one above 64 MiB, is
/// dropped (a protocol error; reading goes on after it). A body that is not valid UTF-8 is converted lossily (JSON-RPC bodies are UTF-8).
pub fn parse_frames(buf: &mut Vec<u8>) -> Vec<String> {
    let mut bodies = Vec::new();
    while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        let body_start = end + 4;
        let Some(len) = content_length(&buf[..end]).filter(|&len| len <= MAX_BODY) else {
            buf.drain(..body_start);
            continue;
        };
        let Some(frame_end) = body_start.checked_add(len) else {
            buf.drain(..body_start);
            continue;
        };
        if buf.len() < frame_end {
            break;
        }
        bodies.push(String::from_utf8_lossy(&buf[body_start..frame_end]).into_owned());
        buf.drain(..frame_end);
    }
    bodies
}

fn content_length(header: &[u8]) -> Option<usize> {
    String::from_utf8_lossy(header).lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse().ok())?
    })
}

/// A helper thread that can be waited for with a time limit.
struct Worker {
    handle: JoinHandle<()>,
    done: Receiver<()>,
}

impl Worker {
    fn spawn(work: impl FnOnce() + Send + 'static) -> Worker {
        let (done_tx, done) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            work();
            let _ = done_tx.send(());
        });
        Worker { handle, done }
    }

    /// Joins the thread when it ends within `timeout`; otherwise lets it go (it ends on its own
    /// once its pipe closes) so the caller never hangs.
    fn join(self, timeout: Duration) {
        // A disconnected `done` means the thread ended (or panicked): joining is immediate.
        if self.done.recv_timeout(timeout) != Err(mpsc::RecvTimeoutError::Timeout) {
            let _ = self.handle.join();
        }
    }
}

/// Pushed into the channel after an automatic restart, so the client can resync (initialize and open its
/// documents again). Synthetic: the server never sends it.
pub const RESTARTED_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"$/g4/restarted"}"#;

/// A running `rotom-lsp`: the child (leader of its own process group), a writer thread that owns
/// its stdin (`send` only queues, so a server that does not read never blocks the caller) and
/// the reader thread that turns its stdout into channel messages.
pub struct LspProcess {
    binary: PathBuf,
    root: PathBuf,
    sink: Channel<String>,
    child: Child,
    /// Frames for the writer thread; `None` once stopped.
    queue: Option<SyncSender<Vec<u8>>>,
    reader: Option<Worker>,
    writer: Option<Worker>,
    /// Automatic restarts since `start`; only the tests read it.
    #[cfg_attr(not(test), allow(dead_code))]
    restarts: u8,
    last_start: Instant,
    restart_after: Duration,
    stopped: bool,
}

struct Spawned {
    child: Child,
    queue: SyncSender<Vec<u8>>,
    reader: Worker,
    writer: Worker,
}

fn spawn(binary: &Path, root: &Path, sink: Channel<String>) -> Result<Spawned, KernelError> {
    let mut command = Command::new(binary);
    command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Nobody reads it; an undrained pipe would block the server once it fills up.
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group, so `kill_tree` ends the server and anything it started.
        command.process_group(0);
    }
    let missing = |reason: String| KernelError::ToolMissing {
        name: "rotom-lsp",
        reason,
    };
    let mut child = command
        .spawn()
        .map_err(|e| missing(format!("cannot run {}: {e}", binary.display())))?;
    let (Some(mut stdin), Some(mut stdout)) = (child.stdin.take(), child.stdout.take()) else {
        kill_tree(&mut child);
        return Err(missing("stdio pipes missing".into()));
    };
    let reader = Worker::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    for body in parse_frames(&mut buf) {
                        // A closed channel (webview gone) must not stop the draining.
                        let _ = sink.send(body);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    });
    let (queue, frames) = mpsc::sync_channel::<Vec<u8>>(WRITE_QUEUE);
    let writer = Worker::spawn(move || {
        // Ends when `stop` drops the queue, or when a write fails because the pipe is closed
        // (the process died or was killed, which also unblocks a write stuck on a full pipe).
        while let Ok(bytes) = frames.recv() {
            if stdin
                .write_all(&bytes)
                .and_then(|()| stdin.flush())
                .is_err()
            {
                break;
            }
        }
    });
    Ok(Spawned {
        child,
        queue,
        reader,
        writer,
    })
}

impl LspProcess {
    /// Spawns `binary` with `root` as working directory and stdio piped; the reader thread parses
    /// `Content-Length` frames from its stdout and sends each body to `sink`.
    ///
    /// # Errors
    /// [`KernelError::ToolMissing`] when the process cannot be started.
    pub fn start(
        binary: &Path,
        root: &Path,
        sink: Channel<String>,
    ) -> Result<LspProcess, KernelError> {
        let spawned = spawn(binary, root, sink.clone())?;
        Ok(LspProcess {
            binary: binary.to_path_buf(),
            root: root.to_path_buf(),
            sink,
            child: spawned.child,
            queue: Some(spawned.queue),
            reader: Some(spawned.reader),
            writer: Some(spawned.writer),
            restarts: 0,
            last_start: Instant::now(),
            restart_after: RESTART_AFTER,
            stopped: false,
        })
    }

    /// Queues `body` as one frame for the server's stdin. Never blocks: when the server does not
    /// read and the queue is full, the frame is refused.
    pub fn send(&self, body: &str) -> Result<(), KernelError> {
        let io = |kind, msg: &str| KernelError::Io(std::io::Error::new(kind, msg.to_owned()));
        let Some(queue) = &self.queue else {
            return Err(io(
                std::io::ErrorKind::BrokenPipe,
                "rotom-lsp is not running",
            ));
        };
        queue.try_send(frame(body)).map_err(|e| match e {
            TrySendError::Full(_) => io(
                std::io::ErrorKind::WouldBlock,
                "rotom-lsp reads its input too slowly",
            ),
            TrySendError::Disconnected(_) => {
                io(std::io::ErrorKind::BrokenPipe, "rotom-lsp is not running")
            }
        })
    }

    /// Kills the server and everything it started (process group), then ends the helper threads
    /// (each waited for at most [`JOIN_TIMEOUT`]). Harmless on a process that is already stopped.
    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        // Dropping the queue ends a writer that is waiting for frames.
        self.queue = None;
        kill_tree(&mut self.child);
        // The kill closes the pipes: the reader sees EOF and a writer stuck on a full pipe fails.
        for worker in [self.reader.take(), self.writer.take()]
            .into_iter()
            .flatten()
        {
            worker.join(JOIN_TIMEOUT);
        }
    }

    /// Whether the server is still running (reaps it when it exited).
    pub fn alive(&mut self) -> bool {
        !self.stopped && matches!(self.child.try_wait(), Ok(None))
    }

    /// Number of automatic restarts since `start`.
    #[cfg(test)]
    fn restarts(&self) -> u8 {
        self.restarts
    }

    #[cfg(test)]
    fn set_restart_after(&mut self, interval: Duration) {
        self.restart_after = interval;
    }

    /// `send`, after restarting a dead server when its last start is at least the restart
    /// interval (60 s) ago. A server that died sooner is not restarted: it stays off until the
    /// next `lsp_start`. The restarted server knows nothing of the client's `initialize`; the
    /// client has to repeat it (it gets no message about the restart).
    pub fn send_or_restart(&mut self, body: &str) -> Result<(), CmdError> {
        if !self.alive() {
            if self.last_start.elapsed() < self.restart_after {
                return Err("rotom-lsp beendet sich wiederholt".into());
            }
            self.restart()?;
        }
        Ok(self.send(body)?)
    }

    fn restart(&mut self) -> Result<(), KernelError> {
        self.stop();
        // Counted before the attempt, so a binary that cannot start is not retried at once.
        self.restarts = self.restarts.saturating_add(1);
        self.last_start = Instant::now();
        let spawned = spawn(&self.binary, &self.root, self.sink.clone())?;
        self.child = spawned.child;
        self.queue = Some(spawned.queue);
        self.reader = Some(spawned.reader);
        self.writer = Some(spawned.writer);
        self.stopped = false;
        // The new server has lost the client's `initialize` and every opened document: tell the client, so it
        // starts over (the frontend resyncs on this method; it is not part of the language server protocol).
        let _ = self.sink.send(RESTARTED_NOTIFICATION.to_owned());
        Ok(())
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts (or replaces) the language server of the open project. `binary` is what
/// [`find_rotom_lsp`] found.
pub fn lsp_start_impl(
    state: &AppState,
    binary: Option<PathBuf>,
    sink: Channel<String>,
) -> Result<(), CmdError> {
    let binary = binary.filter(|p| is_real_file(p)).ok_or_else(|| CmdError {
        code: "tool_missing",
        message: MISSING.to_owned(),
        asset: None,
        problems: Vec::new(),
    })?;
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    if let Some(mut old) = session.lsp.take() {
        old.stop();
    }
    session.lsp = Some(LspProcess::start(&binary, &session.project.root, sink)?);
    Ok(())
}

pub fn lsp_send_impl(state: &AppState, body: &str) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let lsp = session(&mut guard)?
        .lsp
        .as_mut()
        .ok_or("rotom-lsp läuft nicht")?;
    lsp.send_or_restart(body)
}

pub fn lsp_stop_impl(state: &AppState) -> Result<(), CmdError> {
    let old = {
        let mut guard = lock(state)?;
        session(&mut guard)?.lsp.take()
    };
    // Dropping stops it, outside the state lock.
    drop(old);
    Ok(())
}

/// Starts `rotom-lsp` for the open project; its messages arrive on `channel`. `tool_missing`
/// when the binary is not there.
#[tauri::command]
pub async fn lsp_start(app: AppHandle, channel: Channel<String>) -> Result<(), CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        lsp_start_impl(&app.state::<AppState>(), find_rotom_lsp(), channel)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// Writes one JSON-RPC message (without framing) to the server.
#[tauri::command]
pub fn lsp_send(state: tauri::State<AppState>, body: String) -> Result<(), CmdError> {
    lsp_send_impl(&state, &body)
}

#[tauri::command]
pub fn lsp_stop(state: tauri::State<AppState>) -> Result<(), CmdError> {
    lsp_stop_impl(&state)
}

/// Ends the server when the app exits (also reached through `Session` being dropped).
pub fn stop_on_exit(app: &AppHandle) {
    let old = app
        .state::<AppState>()
        .session
        .lock()
        .ok()
        .and_then(|mut guard| guard.as_mut().and_then(|s| s.lsp.take()));
    drop(old);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use tauri::ipc::InvokeResponseBody;

    #[test]
    fn frames_round_trip() {
        let mut buf = frame("{}");
        assert_eq!(buf, b"Content-Length: 2\r\n\r\n{}");
        assert_eq!(parse_frames(&mut buf), ["{}"]);
        assert!(buf.is_empty());
    }

    #[test]
    fn partial_frame_waits() {
        let whole = frame(r#"{"id":1}"#);
        let mut buf = Vec::new();
        // Fed byte by byte, nothing comes out until the last byte arrives.
        for (i, byte) in whole.iter().enumerate() {
            buf.push(*byte);
            let out = parse_frames(&mut buf);
            if i + 1 < whole.len() {
                assert!(out.is_empty(), "frame completed early at byte {i}");
            } else {
                assert_eq!(out, [r#"{"id":1}"#]);
                assert!(buf.is_empty());
            }
        }
    }

    #[test]
    fn several_frames_in_one_read_and_a_trailing_partial() {
        let mut buf = frame("{\"a\":1}");
        buf.extend(frame("{\"b\":2}"));
        let third = frame("{\"c\":3}");
        buf.extend(&third[..10]);
        assert_eq!(parse_frames(&mut buf), ["{\"a\":1}", "{\"b\":2}"]);
        assert_eq!(buf, &third[..10]);
        buf.extend(&third[10..]);
        assert_eq!(parse_frames(&mut buf), ["{\"c\":3}"]);
    }

    #[test]
    fn content_length_counts_bytes_not_chars() {
        let body = r#"{"m":"Grüße 日本 🎮"}"#;
        assert!(body.len() > body.chars().count());
        let framed = frame(body);
        assert!(framed.starts_with(format!("Content-Length: {}\r\n", body.len()).as_bytes()));
        // Split in the middle of a multi-byte character, then whole.
        let cut = framed.iter().position(|&b| b == 0xC3).unwrap() + 1;
        let mut buf = framed[..cut].to_vec();
        assert!(parse_frames(&mut buf).is_empty());
        buf.extend(&framed[cut..]);
        assert_eq!(parse_frames(&mut buf), [body]);
    }

    #[test]
    fn other_headers_and_header_case_are_tolerated() {
        let mut buf =
            b"content-type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: 2\r\n\r\n{}"
                .to_vec();
        assert_eq!(parse_frames(&mut buf), ["{}"]);
    }

    #[test]
    fn header_block_without_length_is_skipped() {
        let mut buf = b"X-Junk: 1\r\n\r\n".to_vec();
        buf.extend(frame("{}"));
        assert_eq!(parse_frames(&mut buf), ["{}"]);
    }

    /// A unique scratch directory that is removed again.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Scratch {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "g4-lsp-{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
        fn script(&self, name: &str, text: &str) -> PathBuf {
            use std::os::unix::fs::PermissionsExt;
            let path = self.0.join(name);
            std::fs::write(&path, text).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn channel() -> (Channel<String>, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel();
        let channel = Channel::new(move |msg: InvokeResponseBody| {
            if let InvokeResponseBody::Json(json) = msg {
                let _ = tx.send(serde_json::from_str::<String>(&json).unwrap());
            }
            Ok(())
        });
        (channel, rx)
    }

    /// Reads one frame, answers `{"id":1,"result":{}}`, then ignores stdin EOF and SIGHUP.
    const FAKE_LSP: &str = r#"#!/bin/sh
trap '' HUP
len=0
while IFS= read -r line; do
  line=$(printf %s "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in Content-Length:*) len=${line#Content-Length: } ;; esac
done
head -c "$len" >/dev/null
body='{"id":1,"result":{}}'
printf 'Content-Length: %d\r\n\r\n%s' ${#body} "$body"
while :; do sleep 1; done
"#;

    #[test]
    fn fake_lsp_answers_initialize() {
        let dir = Scratch::new("init");
        let fake = dir.script("rotom-lsp", FAKE_LSP);
        let (sink, rx) = channel();
        let mut lsp = LspProcess::start(&fake, &dir.0, sink).unwrap();
        assert!(lsp.alive());
        lsp.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .unwrap();
        let answer = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(answer, r#"{"id":1,"result":{}}"#);
        lsp.stop();
        assert!(!lsp.alive());
    }

    #[test]
    fn missing_lsp_is_tool_missing() {
        let dir = Scratch::new("missing");
        let state = AppState::default();
        let (sink, _rx) = channel();
        let gone = dir.0.join("no-such-rotom-lsp");
        let err = lsp_start_impl(&state, Some(gone), sink).unwrap_err();
        assert_eq!(err.code, "tool_missing");
        assert_eq!(
            err.message,
            "rotom-lsp fehlt — Completion und Diagnosen aus"
        );
        // No binary found at all is the same error.
        let (sink, _rx) = channel();
        let err = lsp_start_impl(&state, None, sink).unwrap_err();
        assert_eq!(err.code, "tool_missing");
    }

    /// Starts a long-lived grandchild that shares stdout with the server and writes its pid.
    fn group_script(pid_file: &Path) -> String {
        format!(
            "#!/bin/sh\ntrap '' HUP\nsleep 300 &\necho $! > '{}'\nwhile :; do sleep 1; done\n",
            pid_file.display()
        )
    }

    fn pid_alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc_kill(pid, 0) == 0 }
    }

    unsafe extern "C" {
        #[link_name = "kill"]
        fn libc_kill(pid: i32, sig: i32) -> i32;
    }

    #[test]
    fn stop_kills_the_process_group() {
        let dir = Scratch::new("group");
        let pid_file = dir.0.join("grandchild.pid");
        let fake = dir.script("rotom-lsp", &group_script(&pid_file));
        let (sink, _rx) = channel();
        let mut lsp = LspProcess::start(&fake, &dir.0, sink).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let grandchild = loop {
            if let Ok(text) = std::fs::read_to_string(&pid_file)
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                break pid;
            }
            assert!(
                Instant::now() < deadline,
                "fake never wrote its grandchild pid"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(pid_alive(grandchild));
        // The grandchild keeps the stdout pipe open and the server ignores stdin EOF, so
        // only a group kill lets the reader thread end: stop must not hang.
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::spawn(move || {
            lsp.stop();
            let _ = done_tx.send(lsp);
        });
        let mut lsp = done_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("stop hung");
        assert!(!lsp.alive());
        // The kill is asynchronous for processes that are not our child: give init a moment.
        let deadline = Instant::now() + Duration::from_secs(5);
        while pid_alive(grandchild) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!pid_alive(grandchild), "grandchild survived stop");
        // A second stop is harmless, and a stopped process refuses to send.
        lsp.stop();
        assert!(lsp.send("{}").is_err());
    }

    /// Dies right away on its first start (the marker file is still empty), stays up on later ones.
    fn flaky_script(marker: &Path) -> String {
        format!(
            "#!/bin/sh\nif [ ! -s '{m}' ]; then echo first >> '{m}'; exit 0; fi\necho again >> '{m}'\ncat >/dev/null\n",
            m = marker.display()
        )
    }

    fn wait_dead(lsp: &mut LspProcess) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while lsp.alive() {
            assert!(Instant::now() < deadline, "process did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn dead_process_restarts_once_the_interval_has_passed() {
        let dir = Scratch::new("restart");
        let marker = dir.0.join("starts");
        let fake = dir.script("rotom-lsp", &flaky_script(&marker));
        let (sink, rx) = channel();
        let mut lsp = LspProcess::start(&fake, &dir.0, sink).unwrap();
        wait_dead(&mut lsp);
        // Nothing is pushed before the restart.
        assert!(rx.try_recv().is_err());
        lsp.set_restart_after(Duration::ZERO);
        lsp.send_or_restart("{}").unwrap();
        assert_eq!(lsp.restarts(), 1);
        // The client is told to resync, with a notification that is valid JSON-RPC.
        let pushed = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(pushed, RESTARTED_NOTIFICATION);
        let json: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        assert_eq!(json["method"], "$/g4/restarted");
        assert!(json.get("id").is_none());
        assert!(lsp.alive());
        // The restarted shell writes its marker line a moment after it started.
        let deadline = Instant::now() + Duration::from_secs(10);
        let starts = loop {
            let text = std::fs::read_to_string(&marker).unwrap();
            if text.lines().count() == 2 || Instant::now() > deadline {
                break text;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(starts.lines().collect::<Vec<_>>(), ["first", "again"]);
        lsp.stop();
    }

    #[test]
    fn dead_process_within_the_interval_is_not_restarted() {
        let dir = Scratch::new("norestart");
        let marker = dir.0.join("starts");
        let fake = dir.script("rotom-lsp", &flaky_script(&marker));
        let (sink, _rx) = channel();
        let mut lsp = LspProcess::start(&fake, &dir.0, sink).unwrap();
        wait_dead(&mut lsp);
        let err = lsp.send_or_restart("{}").unwrap_err();
        assert_eq!(err.message, "rotom-lsp beendet sich wiederholt");
        assert_eq!(err.code, "error");
        assert_eq!(lsp.restarts(), 0);
        assert!(!lsp.alive());
        assert_eq!(std::fs::read_to_string(&marker).unwrap().lines().count(), 1);
    }

    /// Writes its own pid, then idles; ignores stdin EOF and SIGHUP.
    fn pid_script(pid_file: &Path) -> String {
        format!(
            "#!/bin/sh\ntrap '' HUP\necho $$ > '{}'\nwhile :; do sleep 1; done\n",
            pid_file.display()
        )
    }

    fn wait_for_pid(pid_file: &Path) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = std::fs::read_to_string(pid_file)
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                return pid;
            }
            assert!(Instant::now() < deadline, "fake never wrote its pid");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn lsp_commands_follow_the_session() {
        use crate::commands::open_project_impl;
        let fixture = g4_kernel::test_support::synthetic_project();
        let project = fixture.dir.path().to_str().unwrap();
        let dir = Scratch::new("session");
        let pid_file = dir.0.join("pid");
        let fake = dir.script("rotom-lsp", &pid_script(&pid_file));
        let state = AppState::default();

        // Without a project there is nothing to start for.
        let (sink, _rx) = channel();
        assert!(lsp_start_impl(&state, Some(fake.clone()), sink).is_err());
        open_project_impl(&state, project).unwrap();
        // Sending before a start is an error, stopping is not.
        assert!(lsp_send_impl(&state, "{}").is_err());
        lsp_stop_impl(&state).unwrap();

        let (sink, _rx) = channel();
        lsp_start_impl(&state, Some(fake.clone()), sink).unwrap();
        let pid = wait_for_pid(&pid_file);
        assert!(pid_alive(pid));
        lsp_send_impl(&state, "{}").unwrap();
        lsp_stop_impl(&state).unwrap();
        assert!(!pid_alive(pid), "lsp_stop left the server running");
        assert!(
            state
                .session
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .lsp
                .is_none()
        );

        // Opening another project ends the server of the old one.
        std::fs::remove_file(&pid_file).unwrap();
        let (sink, _rx) = channel();
        lsp_start_impl(&state, Some(fake), sink).unwrap();
        let pid = wait_for_pid(&pid_file);
        assert!(pid_alive(pid));
        open_project_impl(&state, project).unwrap();
        assert!(!pid_alive(pid), "open_project left the old server running");
    }

    #[test]
    fn absurd_content_length_is_dropped_and_reading_goes_on() {
        // usize::MAX would overflow `body_start + len`; one byte over the cap is refused too;
        // a number that does not even fit a usize has no length at all.
        for len in [
            usize::MAX.to_string(),
            (MAX_BODY + 1).to_string(),
            "99999999999999999999999999".to_owned(),
        ] {
            let mut buf = format!("Content-Length: {len}\r\n\r\n").into_bytes();
            buf.extend(frame("{\"ok\":true}"));
            assert_eq!(parse_frames(&mut buf), ["{\"ok\":true}"], "length {len}");
            assert!(buf.is_empty());
        }
        // A length at the cap is legal and just waits for its body.
        let mut buf = format!("Content-Length: {MAX_BODY}\r\n\r\nabc").into_bytes();
        assert!(parse_frames(&mut buf).is_empty());
        assert!(!buf.is_empty());
    }

    /// Never reads its stdin.
    const DEAF_LSP: &str = "#!/bin/sh\ntrap '' HUP\nwhile :; do sleep 1; done\n";

    #[test]
    fn a_server_that_never_reads_cannot_block_the_app() {
        use crate::commands::open_project_impl;
        let fixture = g4_kernel::test_support::synthetic_project();
        let dir = Scratch::new("deaf");
        let fake = dir.script("rotom-lsp", DEAF_LSP);
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let (sink, _rx) = channel();
        lsp_start_impl(&state, Some(fake), sink).unwrap();

        // Far more than a pipe holds (64 KiB): the writer thread gets stuck, the queue fills
        // up, and every send still returns at once, finally with an error.
        let big = "x".repeat(70 * 1024);
        let started = Instant::now();
        let mut refused = false;
        for _ in 0..(WRITE_QUEUE + 20) {
            if lsp_send_impl(&state, &big).is_err() {
                refused = true;
                break;
            }
        }
        assert!(refused, "the queue never filled up");
        assert!(started.elapsed() < Duration::from_secs(5), "send blocked");

        // The state lock is free: other commands answer promptly, and stop ends the stuck writer.
        let started = Instant::now();
        drop(state.session.lock().unwrap());
        lsp_stop_impl(&state).unwrap();
        assert!(started.elapsed() < Duration::from_secs(5), "stop hung");
        assert!(
            state
                .session
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .lsp
                .is_none()
        );
    }

    #[test]
    fn stop_does_not_hang_when_a_process_outside_the_group_keeps_stdout_open() {
        let dir = Scratch::new("escape");
        let pid_file = dir.0.join("escaped.pid");
        // perl is on every macOS; setsid puts the sleeper into its own session and group, so
        // the group kill misses it while it keeps the inherited stdout pipe open.
        let script = format!(
            "#!/bin/sh\ntrap '' HUP\nperl -e 'use POSIX; setsid(); open F, \">$ARGV[0]\"; print F $$; close F; exec \"sleep\", \"300\"' '{}' </dev/null 2>/dev/null &\nwhile :; do sleep 1; done\n",
            pid_file.display()
        );
        let fake = dir.script("rotom-lsp", &script);
        let (sink, _rx) = channel();
        let mut lsp = LspProcess::start(&fake, &dir.0, sink).unwrap();
        let escaped = wait_for_pid(&pid_file);
        let started = Instant::now();
        lsp.stop();
        let took = started.elapsed();
        assert!(
            took < JOIN_TIMEOUT * 3,
            "stop waited {took:?} for a reader that cannot end"
        );
        assert!(!lsp.alive());
        // The escapee is outside our reach by design; clean it up.
        // SAFETY: plain syscall on the pid the test itself started.
        unsafe { libc_kill(escaped, 9) };
    }
}
