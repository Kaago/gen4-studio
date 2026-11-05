//! The editor's command layer for coding agents over the Model Context Protocol: which
//! commands an agent may run ([`access`]), the curated tools it sees ([`catalog`]), the
//! toolbox that runs them ([`tools`]), the server over stdio ([`server`], started by
//! `studio mcp`, see [`main`]) and, in the running app, the agent settings ([`settings`]),
//! the hub of connections, questions and events ([`hub`]), the state the toolbox then
//! shares with the user ([`host`]) and the server over Streamable HTTP on `127.0.0.1`
//! ([`http`]) with its discovery file ([`discovery`]); `studio mcp` without `--project`
//! forwards stdio to that server ([`proxy`]).

pub mod access;
pub mod catalog;
pub mod discovery;
pub mod docs;
pub mod host;
pub mod http;
pub mod hub;
pub mod prompts;
pub mod proxy;
pub mod server;
pub mod settings;
pub mod symbolic;
pub mod tools;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use crate::commands::policy::UserPolicy;
use crate::mcp::access::Access;
use crate::mcp::server::Server;
use crate::mcp::tools::Toolbox;

/// Version of the tool set; it stands in the server's instructions and in `project_info`.
pub const TOOLSET: &str = "g4.v1";

/// Exit code for bad arguments or a project that does not open: nothing was served.
const EXIT_USAGE: u8 = 2;

/// `studio mcp`: the editor's tools for one project, over stdin and stdout.
#[derive(Parser)]
#[command(
    name = "studio mcp",
    about = "Serve the Gen 4 Studio tools over MCP (stdio): without --project through the \
             running app (\"Lokale Agenten zulassen\"), which decides what the agent may do; \
             with --project headless for that project, read and edit, irreversible steps \
             only with --allow-irreversible"
)]
struct Cli {
    /// The project folder (with header.yaml): serve it headless instead of through the app.
    #[arg(long, value_name = "DIR")]
    project: Option<PathBuf>,
    /// The app data folder with the running app's mcp.json (default: G4_APP_DATA_DIR, else
    /// the platform's folder of Gen 4 Studio).
    #[arg(long = "app-data", value_name = "DIR", conflicts_with = "project")]
    app_data: Option<PathBuf>,
    /// An extra folder for imports and exports (repeatable).
    #[arg(long = "allow-dir", value_name = "DIR")]
    allow_dir: Vec<PathBuf>,
    /// Only read: no tool that changes the project.
    #[arg(long = "read-only")]
    read_only: bool,
    /// Also allow what undo does not bring back (header patch, new trainer, new script,
    /// build outside build/, emulator, checks baseline).
    #[arg(long = "allow-irreversible", conflicts_with = "read_only")]
    allow_irreversible: bool,
    /// The one emulator run_rom may start (needs --allow-irreversible too).
    #[arg(long, value_name = "PATH")]
    emulator: Option<PathBuf>,
    /// Also offer g4_call and g4_commands (any command by name).
    #[arg(long)]
    expert: bool,
    /// The base ROM of the project (the .nds whose SHA-1 .g4/base.json names), for the patch;
    /// otherwise it is searched next to the project and in G4_BASE_ROM_DIR.
    #[arg(long = "base-rom", value_name = "PATH")]
    base_rom: Option<PathBuf>,
}

/// Runs `studio mcp` with the process arguments `args` (`studio`, `mcp`, options…), as the
/// operating system gives them (a path need not be UTF-8). Exit code 2 with a message on
/// stderr when nothing could be served.
pub fn main(args: Vec<OsString>) -> ExitCode {
    // `mcp` stands in for the program name, clap's own name for it is "studio mcp".
    let cli = match Cli::try_parse_from(args.into_iter().skip(1)) {
        Ok(cli) => cli,
        Err(e) if !e.use_stderr() => {
            // --help.
            let _ = e.print();
            return ExitCode::SUCCESS;
        }
        Err(e) => return usage(&format!("ungültige Argumente\n{e}")),
    };
    let Some(project) = cli.project.clone() else {
        return forward(&cli);
    };
    // From here on, fd 1 is stderr: whatever prints (the kernel, apicula, rotom) cannot
    // break the protocol, which writes to its own copy of stdout.
    let protocol = match stdout::protect() {
        Ok(protocol) => protocol,
        Err(e) => return usage(&format!("stdout nicht umlenkbar: {e}")),
    };
    #[cfg(debug_assertions)]
    if std::env::var_os("G4_MCP_TEST_NOISE").is_some_and(|v| v == "1") {
        println!("G4_MCP_TEST_NOISE: this line goes to stderr, not into the protocol");
    }
    let policy = match UserPolicy::new(&project, &cli.allow_dir, cli.emulator.as_deref()) {
        Ok(policy) => policy,
        Err(e) => return usage(&e),
    };
    // Whoever sets up `studio mcp --project` chose the project: reading and editing by
    // default, irreversible steps only when asked for.
    let access = Access {
        write: !cli.read_only,
        irreversible: cli.allow_irreversible,
        expert: cli.expert,
    };
    let toolbox = match Toolbox::open_with_base(policy, access, cli.base_rom.clone()) {
        Ok(toolbox) => toolbox,
        Err(e) => return usage(&e),
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return usage(&format!("Laufzeit nicht startbar: {e}")),
    };
    // `serve` returns once the client has gone and no call runs any more (at most
    // `server::DRAIN_LIMIT`), so a write is never cut off in the middle of its step.
    let served = runtime.block_on(Server::new(toolbox).serve(tokio::io::stdin(), protocol));
    runtime.shutdown_timeout(Duration::from_secs(1));
    match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("studio mcp: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The flag of a headless option given without `--project` (the app decides access).
fn headless_flag(cli: &Cli) -> Option<&'static str> {
    [
        (cli.read_only, "--read-only"),
        (cli.allow_irreversible, "--allow-irreversible"),
        (cli.emulator.is_some(), "--emulator"),
        (cli.expert, "--expert"),
        (!cli.allow_dir.is_empty(), "--allow-dir"),
        (cli.base_rom.is_some(), "--base-rom"),
    ]
    .into_iter()
    .find_map(|(given, flag)| given.then_some(flag))
}

/// `studio mcp` without `--project`: forwards stdio to the running app named by the
/// discovery file in `--app-data` (else the app data folder); exit code 2 when no app runs.
fn forward(cli: &Cli) -> ExitCode {
    if let Some(flag) = headless_flag(cli) {
        return usage(&format!("{flag} gilt nur kopflos mit --project"));
    }
    let Some(dir) = cli.app_data.clone().or_else(discovery::default_dir) else {
        return usage(proxy::NOT_RUNNING);
    };
    let found = match proxy::locate(&dir) {
        Ok(found) => found,
        Err(e) => return usage(&e),
    };
    let protocol = match stdout::protect() {
        Ok(protocol) => protocol,
        Err(e) => return usage(&format!("stdout nicht umlenkbar: {e}")),
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return usage(&format!("Laufzeit nicht startbar: {e}")),
    };
    let forwarded = runtime.block_on(proxy::run(dir, found, tokio::io::stdin(), protocol));
    runtime.shutdown_timeout(Duration::from_secs(1));
    match forwarded {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("studio mcp: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("studio mcp: {message}");
    ExitCode::from(EXIT_USAGE)
}

#[cfg(unix)]
mod stdout {
    use std::io::Write;
    use std::os::fd::{FromRawFd, OwnedFd};

    /// Duplicates fd 1 for the protocol (close-on-exec, so child processes do not get it)
    /// and points fd 1 at stderr. Answers the protocol's copy.
    pub fn protect() -> std::io::Result<tokio::fs::File> {
        std::io::stdout().flush()?;
        // SAFETY: fcntl with F_DUPFD_CLOEXEC only creates a new descriptor for fd 1.
        let copy = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_DUPFD_CLOEXEC, 0) };
        if copy < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `copy` is a fresh descriptor that nothing else owns.
        let copy = unsafe { OwnedFd::from_raw_fd(copy) };
        // SAFETY: dup2 replaces fd 1 with a duplicate of fd 2; both stay open.
        if unsafe { libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(tokio::fs::File::from_std(std::fs::File::from(copy)))
    }
}

#[cfg(windows)]
mod stdout {
    use std::io::Write;
    use std::os::windows::io::{FromRawHandle, OwnedHandle};

    use windows_sys::Win32::Foundation::{
        DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// As on Unix: a private copy of the stdout handle for the protocol (not inheritable,
    /// so child processes do not get it), and the process's stdout pointed at stderr. Rust
    /// code (`println!`, chatot's "Encoding text" lines) asks `GetStdHandle` on every write;
    /// C code writes to the C runtime's fd 1, which `_dup2` points at fd 2 (that also closes
    /// the inherited original handle, so no child holds the client's pipe open).
    pub fn protect() -> std::io::Result<tokio::fs::File> {
        std::io::stdout().flush()?;
        // SAFETY: handle calls on this process's own standard handles; `copy` is a fresh
        // handle that only the returned file owns.
        unsafe {
            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            if out.is_null() || out == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::other("kein stdout"));
            }
            let process = GetCurrentProcess();
            let mut copy: HANDLE = std::ptr::null_mut();
            if DuplicateHandle(
                process,
                out,
                process,
                &raw mut copy,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            let copy = OwnedHandle::from_raw_handle(copy);
            if SetStdHandle(STD_OUTPUT_HANDLE, GetStdHandle(STD_ERROR_HANDLE)) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            // Best effort: without a C runtime fd 1 there is nothing to point elsewhere.
            libc::dup2(2, 1);
            Ok(tokio::fs::File::from_std(std::fs::File::from(copy)))
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod stdout {
    /// Elsewhere the protocol writes to stdout as it is.
    pub fn protect() -> std::io::Result<tokio::io::Stdout> {
        Ok(tokio::io::stdout())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("mcp").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn base_rom_flag_needs_project() {
        let without = cli(&["--base-rom", "/roms/basis.nds"]);
        assert_eq!(headless_flag(&without), Some("--base-rom"));
        let with = cli(&["--project", "/projekt", "--base-rom", "/roms/basis.nds"]);
        assert_eq!(
            with.base_rom.as_deref(),
            Some(std::path::Path::new("/roms/basis.nds"))
        );
        assert_eq!(headless_flag(&cli(&[])), None);
    }
}
