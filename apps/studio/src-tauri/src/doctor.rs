//! `studio doctor`: checks that the packaged app is complete, without a project and without a
//! ROM. The release workflow runs it inside `Gen4 Studio.app` (macOS) and in the installed
//! folder (Windows). It looks for the sidecars beside the running binary only: neither
//! `G4_APICULA`/`G4_ROTOM_LSP` nor the debug paths of the repository count, so a development
//! shell cannot make a broken package pass. An empty file is the build-time placeholder (see
//! build.rs) and counts as missing.
//!
//! Output: one line per tool and a last line `doctor: ok` or `doctor: Sidecar fehlt oder startet
//! nicht`; `--json` prints the [`DoctorReport`] instead. Exit status 0 when apicula and
//! rotom-lsp start, 1 when one is missing or does not start, 2 for bad arguments.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use g4_kernel::export::kill_tree;
use g4_kernel::git::find_git;
use g4_kernel::live::melonds;
use serde::Serialize;

use crate::lsp::{frame, parse_frames};
use crate::sidecar::{find_in, sidecar_name};

/// How long each sidecar check may take before the process is killed.
const LIMIT: Duration = Duration::from_secs(10);
/// How long rotom-lsp gets to end after `exit` (its answer to `initialize` already counted).
const EXIT_GRACE: Duration = Duration::from_secs(2);

const USAGE: &str = "Aufruf: studio doctor [--json]";

/// One sidecar: not there, or found and tried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Probe {
    /// No real file of that name beside the binary.
    Missing,
    /// `starts`: the check ran through (apicula `--help` ended with status 0, rotom-lsp
    /// answered `initialize`); `detail` says what happened.
    Found {
        path: PathBuf,
        starts: bool,
        detail: String,
    },
}

impl Probe {
    fn starts(&self) -> bool {
        matches!(self, Probe::Found { starts: true, .. })
    }
}

/// What `studio doctor` found. `git` and `melonds` are informational (the editor works without
/// them); `nitroarc`: the shared library is loaded and answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub version: String,
    pub binary: PathBuf,
    pub apicula: Probe,
    pub rotom_lsp: Probe,
    pub git: Option<PathBuf>,
    pub melonds: Option<PathBuf>,
    pub nitroarc: bool,
}

impl DoctorReport {
    /// 0 when apicula and rotom-lsp start, else 1.
    pub fn exit_status(&self) -> u8 {
        u8::from(!(self.apicula.starts() && self.rotom_lsp.starts()))
    }

    /// The report as text, one line per tool, German like the rest of the app.
    pub fn text(&self) -> String {
        let probe = |name: &str, probe: &Probe| match probe {
            Probe::Missing => format!("{name}: fehlt"),
            Probe::Found {
                path,
                starts: true,
                detail,
            } => format!("{name}: {} (startet; {detail})", path.display()),
            Probe::Found {
                path,
                starts: false,
                detail,
            } => format!("{name}: {} (startet nicht; {detail})", path.display()),
        };
        let optional = |name: &str, path: &Option<PathBuf>| match path {
            Some(path) => format!("{name}: {}", path.display()),
            None => format!("{name}: fehlt"),
        };
        let nitroarc = if self.nitroarc {
            "nitroarc: geladen"
        } else {
            "nitroarc: antwortet nicht"
        };
        let last = if self.exit_status() == 0 {
            "doctor: ok"
        } else {
            "doctor: Sidecar fehlt oder startet nicht"
        };
        [
            format!("studio {}: {}", self.version, self.binary.display()),
            probe("apicula", &self.apicula),
            probe("rotom-lsp", &self.rotom_lsp),
            optional("git", &self.git),
            optional("melonDS", &self.melonds),
            nitroarc.to_owned(),
            last.to_owned(),
        ]
        .join("\n")
    }
}

/// `studio doctor [--json]` with the process arguments `args` (`studio`, `doctor`, …).
pub fn main(args: Vec<OsString>) -> ExitCode {
    let mut json = false;
    for arg in args.iter().skip(2) {
        match arg.to_str() {
            Some("--json") => json = true,
            Some("--help" | "-h") => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("studio doctor: ungültiges Argument {arg:?}\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("studio doctor: Pfad des Programms unbekannt: {e}");
            return ExitCode::from(2);
        }
    };
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut report = probe_all(&dir);
    report.binary = exe;
    let out = if json {
        serde_json::to_string_pretty(&report).unwrap_or_default()
    } else {
        report.text()
    };
    // A closed pipe must not panic a diagnostic.
    let _ = writeln!(std::io::stdout(), "{out}");
    ExitCode::from(report.exit_status())
}

/// Checks the package whose binary lives in `exe_dir`: the sidecars are looked for in that
/// folder only.
pub fn probe_all(exe_dir: &Path) -> DoctorReport {
    let probe = |stem: &str, check: fn(&Path, Duration) -> (bool, String)| match find_in(
        &[exe_dir.to_path_buf()],
        &sidecar_name(stem),
    ) {
        None => Probe::Missing,
        Some(path) => {
            let (starts, detail) = check(&path, LIMIT);
            Probe::Found {
                path,
                starts,
                detail,
            }
        }
    };
    DoctorReport {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        binary: exe_dir.join(sidecar_name("studio")),
        apicula: probe("apicula", check_apicula),
        rotom_lsp: probe("rotom-lsp", check_lsp),
        git: find_git().map(|git| git.path),
        melonds: find_melonds(),
        nitroarc: nitroarc_answers(),
    }
}

/// The kernel has no melonDS search (the emulator is the one the user picks); its own
/// reference is the installed bundle `/Applications/melonDS.app`, so that is what is reported.
fn find_melonds() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        melonds::bundle_binary(Path::new("/Applications/melonDS.app"))
    } else {
        None
    }
}

/// An empty buffer is not a NARC: nitroarc must refuse it with its own error. That the call
/// returns at all proves the shared library is loaded; the refusal proves it is usable.
fn nitroarc_answers() -> bool {
    match uxie::narc::Narc::from_bytes(&[]) {
        Ok(_) => false,
        Err(uxie::error::UxieError::InvalidFormat { message }) => {
            message.starts_with("nitroarc error")
        }
        Err(_) => false,
    }
}

/// Starts `command` with `stdin` piped as asked, in its own process group (Unix), without a
/// console window (Windows), so [`kill_tree`] ends it and whatever it started.
fn spawn(command: &mut Command, stdin: Stdio, stdout: Stdio) -> std::io::Result<Child> {
    command
        .current_dir(std::env::temp_dir())
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up.
        command.creation_flags(0x0800_0000);
    }
    command.spawn()
}

/// `apicula --help` must end with status 0 within `limit`.
fn check_apicula(path: &Path, limit: Duration) -> (bool, String) {
    let mut command = Command::new(path);
    command.arg("--help");
    let mut child = match spawn(&mut command, Stdio::null(), Stdio::null()) {
        Ok(child) => child,
        Err(e) => return (false, format!("nicht ausführbar: {e}")),
    };
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return (true, "--help: Status 0".into()),
            Ok(Some(status)) => return (false, format!("--help: {status}")),
            Ok(None) => {}
            Err(e) => {
                kill_tree(&mut child);
                return (false, format!("Warten fehlgeschlagen: {e}"));
            }
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            return (
                false,
                format!("--help: keine Antwort in {} s", limit.as_secs()),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Whether `body` is the successful answer to request 1.
fn is_initialize_answer(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .is_ok_and(|v| v["id"] == 1 && v.get("result").is_some() && v.get("error").is_none())
}

/// rotom-lsp: `initialize` (rootUri null), read the answer, `shutdown`, `exit`; all within
/// `limit`. What counts is the answer to `initialize`; a server that then does not end is
/// killed and mentioned in the detail.
fn check_lsp(path: &Path, limit: Duration) -> (bool, String) {
    let mut command = Command::new(path);
    let mut child = match spawn(&mut command, Stdio::piped(), Stdio::piped()) {
        Ok(child) => child,
        Err(e) => return (false, format!("nicht ausführbar: {e}")),
    };
    let deadline = Instant::now() + limit;
    let (Some(mut stdin), Some(mut stdout)) = (child.stdin.take(), child.stdout.take()) else {
        kill_tree(&mut child);
        return (false, "stdio nicht verfügbar".into());
    };
    // The reader ends with the process (EOF), which also wakes the wait below.
    let (bodies, answers) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
        while let Ok(n) = stdout.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            for body in parse_frames(&mut buf) {
                if bodies.send(body).is_err() {
                    return;
                }
            }
        }
    });
    let send = |stdin: &mut std::process::ChildStdin, body: &str| {
        stdin.write_all(&frame(body)).and_then(|()| stdin.flush())
    };
    let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#;
    if let Err(e) = send(&mut stdin, initialize) {
        kill_tree(&mut child);
        return (false, format!("initialize nicht gesendet: {e}"));
    }
    let outcome = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match answers.recv_timeout(left) {
            Ok(body) if is_initialize_answer(&body) => break Ok(()),
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => {
                break Err(format!(
                    "initialize: keine Antwort in {} s",
                    limit.as_secs()
                ));
            }
            Err(RecvTimeoutError::Disconnected) => {
                break Err("beendete sich ohne Antwort auf initialize".to_owned());
            }
        }
    };
    if let Err(detail) = outcome {
        kill_tree(&mut child);
        return (false, detail);
    }
    // A failed write only means the server is gone already; the answer counted.
    let _ = send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#,
    );
    let _ = send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    drop(stdin);
    let grace = Instant::now() + EXIT_GRACE.min(limit);
    let ended = loop {
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) if Instant::now() < grace => std::thread::sleep(Duration::from_millis(20)),
            _ => break false,
        }
    };
    if ended {
        (true, "initialize beantwortet".into())
    } else {
        kill_tree(&mut child);
        (
            true,
            "initialize beantwortet; beendete sich nach exit nicht".into(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Whether the JSON object `value` has every one of `keys`.
    fn has_keys(value: &serde_json::Value, keys: &[&str]) -> bool {
        keys.iter().all(|k| value.get(k).is_some())
    }

    #[test]
    fn missing_sidecars_exit_1() {
        let dir = tempfile::tempdir().unwrap();
        let report = probe_all(dir.path());
        assert_eq!(report.apicula, Probe::Missing);
        assert_eq!(report.rotom_lsp, Probe::Missing);
        assert_eq!(report.exit_status(), 1);
        assert!(
            report
                .text()
                .ends_with("doctor: Sidecar fehlt oder startet nicht")
        );
        assert!(report.text().contains("apicula: fehlt"));
    }

    #[test]
    fn placeholder_counts_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        for stem in ["apicula", "rotom-lsp"] {
            std::fs::write(dir.path().join(sidecar_name(stem)), b"").unwrap();
        }
        let report = probe_all(dir.path());
        assert_eq!(report.apicula, Probe::Missing);
        assert_eq!(report.rotom_lsp, Probe::Missing);
        assert_eq!(report.exit_status(), 1);
    }

    #[test]
    fn json_shape() {
        let dir = tempfile::tempdir().unwrap();
        let json = serde_json::to_value(probe_all(dir.path())).unwrap();
        assert!(
            has_keys(
                &json,
                &[
                    "version",
                    "apicula",
                    "rotom_lsp",
                    "git",
                    "melonds",
                    "nitroarc"
                ]
            ),
            "{json}"
        );
        assert_eq!(json["apicula"], "missing");
    }

    #[test]
    fn nitroarc_refuses_an_empty_buffer_with_its_own_error() {
        assert!(nitroarc_answers());
    }

    #[test]
    fn help_text_is_not_an_error() {
        let out = main(vec!["studio".into(), "doctor".into(), "--help".into()]);
        assert_eq!(format!("{out:?}"), format!("{:?}", ExitCode::SUCCESS));
        let out = main(vec!["studio".into(), "doctor".into(), "--nope".into()]);
        assert_eq!(format!("{out:?}"), format!("{:?}", ExitCode::from(2)));
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        fn script(dir: &Path, name: &str, text: &str) -> PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        /// Reads one frame, answers `initialize`, then ends once stdin closes.
        const FAKE_LSP: &str = r#"#!/bin/sh
len=0
while IFS= read -r line; do
  line=$(printf %s "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in Content-Length:*) len=${line#Content-Length: } ;; esac
done
head -c "$len" >/dev/null
body='{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
printf 'Content-Length: %d\r\n\r\n%s' ${#body} "$body"
cat >/dev/null
"#;

        #[test]
        fn lsp_handshake_detects_dead_server() {
            let dir = tempfile::tempdir().unwrap();
            let dead = script(dir.path(), "rotom-lsp", "#!/bin/sh\nexit 0\n");
            let (starts, detail) = check_lsp(&dead, Duration::from_secs(10));
            assert!(!starts, "{detail}");
        }

        #[test]
        fn lsp_handshake_accepts_an_answering_server() {
            let dir = tempfile::tempdir().unwrap();
            let fake = script(dir.path(), "rotom-lsp", FAKE_LSP);
            let (starts, detail) = check_lsp(&fake, Duration::from_secs(10));
            assert!(starts, "{detail}");
        }

        #[test]
        fn lsp_that_never_answers_is_killed_at_the_limit() {
            let dir = tempfile::tempdir().unwrap();
            let mute = script(dir.path(), "rotom-lsp", "#!/bin/sh\nexec sleep 30\n");
            let began = Instant::now();
            let (starts, detail) = check_lsp(&mute, Duration::from_millis(300));
            assert!(!starts, "{detail}");
            assert!(began.elapsed() < Duration::from_secs(5));
        }

        #[test]
        fn apicula_status_and_timeout() {
            let dir = tempfile::tempdir().unwrap();
            let good = script(dir.path(), "good", "#!/bin/sh\nexit 0\n");
            let bad = script(dir.path(), "bad", "#!/bin/sh\nexit 3\n");
            let hang = script(dir.path(), "hang", "#!/bin/sh\nexec sleep 30\n");
            assert!(check_apicula(&good, Duration::from_secs(10)).0);
            assert!(!check_apicula(&bad, Duration::from_secs(10)).0);
            let began = Instant::now();
            assert!(!check_apicula(&hang, Duration::from_millis(300)).0);
            assert!(began.elapsed() < Duration::from_secs(5));
        }

        #[test]
        fn working_sidecars_exit_0() {
            let dir = tempfile::tempdir().unwrap();
            script(dir.path(), "apicula", "#!/bin/sh\nexit 0\n");
            script(dir.path(), "rotom-lsp", FAKE_LSP);
            let report = probe_all(dir.path());
            assert_eq!(report.exit_status(), 0, "{}", report.text());
            assert!(report.text().ends_with("doctor: ok"));
        }
    }
}
