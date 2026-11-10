//! `g4k live` against the fake GDB stub: the JSON lines and exit codes are a contract (the
//! `g4emu` integration test drives the same commands).

#[path = "../src/live/fake_stub.rs"]
mod fake_stub;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use fake_stub::{FakeStub, Quirks};
use serde_json::Value;

/// Platinum: the u32 at `POINTER` points at `BLOCK`; the layout counts from `BASE`.
const GAME_CODE: u32 = 0x023F_FE0C;
const POINTER: u32 = 0x0210_1EE0;
const BLOCK: u32 = 0x0227_E354;
const BASE: u32 = BLOCK + 0x14;
const FLAGS: u32 = BASE + 0xFEC;
const VARS: u32 = BASE + 0xDAC;
/// Flag 0x173 is bit 3 of byte 0x2E.
const FLAG_173_BYTE: u32 = FLAGS + 0x2E;
const VAR_40A5: u32 = VARS + 2 * 0xA5;

fn platinum(code: &[u8; 4]) -> FakeStub {
    let stub = FakeStub::start(Quirks::melonds_1_1());
    stub.poke(GAME_CODE, code);
    stub.poke(POINTER, &BLOCK.to_le_bytes());
    stub.poke(FLAG_173_BYTE, &[0b0000_1000]);
    stub.poke(VAR_40A5, &[1, 0]);
    stub
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// `g4k live …` in English (g4k follows `G4_LANG`, then `LANG`): the tests assert the English
/// messages.
fn g4k(stub: &FakeStub, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_g4k"));
    cmd.env("G4_LANG", "en");
    cmd.args(["live", args[0], "--port"])
        .arg(stub.port.to_string())
        .args(["--game", "CPUD"])
        .args(&args[1..]);
    cmd
}

/// Runs to the end; returns the exit code and the single JSON line on stdout.
fn run(stub: &FakeStub, args: &[&str]) -> (i32, Value) {
    let out = g4k(stub, args)
        .stdin(Stdio::null())
        .output()
        .expect("run g4k");
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {text:?} stderr: {:?}", out.stderr);
    (
        out.status.code().expect("exit code"),
        serde_json::from_str(lines[0]).expect("a JSON line"),
    )
}

struct Waiting {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl Waiting {
    fn spawn(stub: &FakeStub, args: &[&str]) -> Waiting {
        let mut child = g4k(stub, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn g4k");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());
        Waiting {
            child,
            stdin,
            stdout,
            stderr,
        }
    }

    /// Reads stderr until `{"watching": true}`: the baseline snapshot is taken, so a change from
    /// now on is seen.
    fn watching(&mut self) {
        let mut line = String::new();
        self.stderr.read_line(&mut line).expect("read stderr");
        let value: Value =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON ({e}): {line:?}"));
        assert_eq!(value, serde_json::json!({ "watching": true }));
    }

    fn line(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read a line");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON ({e}): {line:?}"))
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn status_set_and_wait_work_against_the_fake_stub() {
    let stub = platinum(b"CPUD");

    let (code, status) = run(&stub, &["status"]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(status["game"], "CPUD");
    assert_eq!(status["game_check"], "verified");
    assert_eq!(status["has_save"], true);
    assert_eq!(status["flags_set"], serde_json::json!([0x173]));
    assert_eq!(status["vars"], serde_json::json!({ "0x40A5": 1 }));
    assert!(status["took_ms"].is_u64(), "{status}");
    wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));

    let (code, set) = run(&stub, &["set", "--flag", "0x173", "--value", "0"]);
    assert_eq!(code, 0, "{set}");
    assert_eq!(
        set["target"],
        serde_json::json!({"kind": "flag", "number": 0x173})
    );
    assert_eq!(set["value"], 0);
    assert_eq!(set["read_back"], true);
    assert_eq!(stub.peek(FLAG_173_BYTE, 1), [0]);

    // A decimal variable number and a value above 1 for a variable.
    let (code, set) = run(&stub, &["set", "--var", "16549", "--value", "0x1234"]);
    assert_eq!(code, 0, "{set}");
    assert_eq!(set["value"], 0x1234);
    assert_eq!(set["read_back"], true);
    assert_eq!(stub.peek(VAR_40A5, 2), [0x34, 0x12]);
    stub.poke(VAR_40A5, &[1, 0]);

    let mut waiting = Waiting::spawn(&stub, &["wait", "--var", "0x40A5", "--timeout", "5"]);
    waiting.watching();
    std::thread::sleep(Duration::from_millis(300));
    stub.poke(VAR_40A5, &[0, 0]);
    let halted = waiting.line();
    assert_eq!(halted["halted"], true, "{halted}");
    assert_eq!(halted["before"], 1);
    assert_eq!(halted["after"], 0);
    assert!(halted["took_ms"].as_u64().unwrap() >= 250, "{halted}");
    assert!(stub.halted.load(Ordering::SeqCst));
    // The process stays connected and holds the game until a line arrives.
    std::thread::sleep(Duration::from_millis(300));
    assert!(waiting.child.try_wait().unwrap().is_none());
    assert!(stub.halted.load(Ordering::SeqCst));

    let mut stdin = waiting.stdin.take().unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    let status = waiting.child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(!stub.halted.load(Ordering::SeqCst));
    wait_until("the stub to be free", || stub.open_connections() == 0);
}

#[test]
fn wait_resumes_on_eof_of_stdin() {
    let stub = platinum(b"CPUD");
    let mut waiting = Waiting::spawn(&stub, &["wait", "--flag", "371", "--timeout", "5"]);
    waiting.watching();
    stub.poke(FLAG_173_BYTE, &[0]);
    let halted = waiting.line();
    assert_eq!(
        (halted["before"].clone(), halted["after"].clone()),
        (1.into(), 0.into()),
        "{halted}"
    );
    drop(waiting.stdin.take());
    assert_eq!(waiting.child.wait().unwrap().code(), Some(0));
    assert!(!stub.halted.load(Ordering::SeqCst));
}

#[test]
fn wait_without_a_change_times_out_and_lets_the_game_run() {
    let stub = platinum(b"CPUD");
    let (code, out) = run(&stub, &["wait", "--var", "0x40A5", "--timeout", "0.4"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(out["error"]["code"], "timeout");
    assert!(out["error"]["message"].is_string());
    wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));
    wait_until("the stub to be free", || stub.open_connections() == 0);
}

#[test]
fn resume_lets_the_game_run() {
    let stub = platinum(b"CPUD");
    let (code, out) = run(&stub, &["resume"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, serde_json::json!({ "resumed": true }));
    wait_until("the stub to run", || !stub.halted.load(Ordering::SeqCst));
}

#[test]
fn a_wrong_game_is_refused_without_writing() {
    let stub = platinum(b"IPKD");
    let before = stub.peek(FLAG_173_BYTE, 1);
    let (code, out) = run(&stub, &["set", "--flag", "0x173", "--value", "0"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(out["error"]["code"], "wrong_game");
    let message = out["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("IPKD") && message.contains("CPUD"),
        "{message}"
    );
    assert_eq!(stub.peek(FLAG_173_BYTE, 1), before);
    assert!(!stub.log_lines().iter().any(|l| l.starts_with('M')));
}

#[test]
fn errors_carry_their_codes() {
    let stub = platinum(b"CPUD");
    let (code, out) = run(&stub, &["set", "--flag", "3000", "--value", "1"]);
    assert_eq!(
        (code, &out["error"]["code"]),
        (1, &Value::from("outside_save")),
        "{out}"
    );
    let (code, out) = run(&stub, &["set", "--var", "0x3FFF", "--value", "1"]);
    assert_eq!(
        (code, &out["error"]["code"]),
        (1, &Value::from("outside_save")),
        "{out}"
    );
    let (code, out) = run(&stub, &["set", "--flag", "5", "--value", "2"]);
    assert_eq!(
        (code, &out["error"]["code"]),
        (1, &Value::from("invalid_value")),
        "{out}"
    );
    assert!(!stub.log_lines().iter().any(|l| l.starts_with('M')));

    // No save loaded yet.
    stub.poke(POINTER, &[0; 4]);
    let (code, out) = run(&stub, &["set", "--flag", "5", "--value", "1"]);
    assert_eq!(
        (code, &out["error"]["code"]),
        (1, &Value::from("no_save")),
        "{out}"
    );
    let (code, out) = run(&stub, &["status"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["has_save"], false);
    assert_eq!(out["flags_set"], serde_json::json!([]));
    assert_eq!(out["vars"], serde_json::json!({}));

    // Nothing listens on a port that was just free; a game without a save layout.
    let port = stub.port;
    let (code, out) = {
        let o = Command::new(env!("CARGO_BIN_EXE_g4k"))
            .env("G4_LANG", "en")
            .args([
                "live",
                "status",
                "--port",
                &port.to_string(),
                "--game",
                "ADAE",
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (
            o.status.code().unwrap(),
            serde_json::from_slice::<Value>(&o.stdout).unwrap(),
        )
    };
    assert_eq!(
        (code, &out["error"]["code"]),
        (1, &Value::from("unsupported_game")),
        "{out}"
    );
    drop(stub);
    let o = Command::new(env!("CARGO_BIN_EXE_g4k"))
        .env("G4_LANG", "en")
        .args([
            "live",
            "status",
            "--port",
            &port.to_string(),
            "--game",
            "CPUD",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let out: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        (o.status.code(), &out["error"]["code"]),
        (Some(1), &Value::from("connect")),
        "{out}"
    );
}

/// B2: `g4k live status` twenty times in a row against a stub that reads its socket only every
/// 5 ms (melonDS 1.1 under load: `c` and the packet after it arrive together, and melonDS keeps
/// what it did not handle for the next client).
#[test]
fn twenty_status_calls_in_a_row_all_answer() {
    let stub = FakeStub::start(Quirks {
        read_pause: Duration::from_millis(5),
        ..Quirks::melonds_1_1()
    });
    stub.poke(GAME_CODE, b"CPUD");
    stub.poke(POINTER, &BLOCK.to_le_bytes());
    stub.poke(VAR_40A5, &[1, 0]);
    for call in 0..20 {
        let (code, out) = run(&stub, &["status"]);
        assert_eq!(code, 0, "call {call}: {out}");
        assert_eq!(out["vars"]["0x40A5"], 1, "call {call}: {out}");
    }
    wait_until("the game to run", || !stub.halted.load(Ordering::SeqCst));
    assert_eq!(
        stub.log_lines().iter().filter(|l| *l == "D").count(),
        20,
        "every call detaches"
    );
}

/// B3: a stub that closes every connection at once (melonDS 1.1 after a stray byte in its
/// buffer) is code `closed` with a sentence, not `io` „Invalid argument (os error 22)“.
#[test]
fn a_stub_that_closes_every_connection_is_closed_with_a_sentence() {
    let stub = platinum(b"CPUD");
    let mut stray = std::net::TcpStream::connect(stub.addr()).unwrap();
    stray.write_all(b"+x").unwrap();
    // `open_connections() == 0` is already true before the stub accepted the connection: wait
    // for the stray byte to be seen first.
    wait_until("the stub to see the stray byte", || {
        stub.log_lines().contains(&"<stray 0x78>".to_string())
    });
    wait_until("the stub to close it", || stub.open_connections() == 0);
    drop(stray);
    let (code, out) = run(&stub, &["status"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(out["error"]["code"], "closed", "{out}");
    assert_eq!(
        out["error"]["message"],
        format!(
            "The GDB stub on port {} closes every connection at once: restart melonDS",
            stub.port
        )
    );
}

#[test]
fn a_silent_stub_is_no_reply() {
    let stub = FakeStub::start(Quirks {
        answer: false,
        ..Quirks::melonds_1_1()
    });
    let (code, out) = run(&stub, &["status"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(out["error"]["code"], "no_reply");
}

/// A timeout too large for a `Duration` is a usage error (exit 2), not a panic.
#[test]
fn a_timeout_too_large_is_a_usage_error() {
    let stub = platinum(b"CPUD");
    let out = g4k(&stub, &["wait", "--flag", "371", "--timeout", "1e30"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("--timeout"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(stub.log_lines().is_empty(), "never connected");
}
