//! The editor's command layer for coding agents over the Model Context Protocol: which
//! commands an agent may run ([`access`]), the curated tools it sees ([`catalog`]), the
//! toolbox that runs them ([`tools`]) and the server over stdio ([`server`], started by
//! `studio mcp`, see [`main`]).

pub mod access;
pub mod catalog;
pub mod server;
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
    about = "Serve the Gen 4 Studio tools for one project over MCP (stdio), read-only"
)]
struct Cli {
    /// The project folder (with header.yaml).
    #[arg(long, value_name = "DIR")]
    project: Option<PathBuf>,
    /// An extra folder for imports and exports (repeatable).
    #[arg(long = "allow-dir", value_name = "DIR")]
    allow_dir: Vec<PathBuf>,
    /// Also offer g4_call and g4_commands (any command by name).
    #[arg(long)]
    expert: bool,
}

/// Runs `studio mcp` with the process arguments `args` (`studio`, `mcp`, options…), as the
/// operating system gives them (a path need not be UTF-8). Exit code 2 with a message on
/// stderr when nothing could be served.
pub fn main(args: Vec<OsString>) -> ExitCode {
    // `mcp` stands in for the program name, clap's own name for it is "studio mcp".
    let cli = match Cli::try_parse_from(args.into_iter().skip(1)) {
        Ok(cli) => cli,
        Err(e) if !e.use_stderr() => {
            // --help and --version.
            let _ = e.print();
            return ExitCode::SUCCESS;
        }
        Err(e) => return usage(&format!("ungültige Argumente\n{e}")),
    };
    let Some(project) = cli.project else {
        return usage("--project fehlt (die laufende App kommt mit 3h.3)");
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
    let policy = match UserPolicy::new(&project, &cli.allow_dir, None) {
        Ok(policy) => policy,
        Err(e) => return usage(&e),
    };
    // Read-only in 3h.1: the switches for writing come with 3h.2.
    let access = Access {
        write: false,
        irreversible: false,
        expert: cli.expert,
    };
    let toolbox = match Toolbox::open(policy, access) {
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
    let served = runtime.block_on(Server::new(toolbox).serve(tokio::io::stdin(), protocol));
    // A command still running on a blocking worker has no client left to answer.
    runtime.shutdown_timeout(Duration::from_secs(1));
    match served {
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

#[cfg(not(unix))]
mod stdout {
    /// Elsewhere the protocol writes to stdout as it is.
    pub fn protect() -> std::io::Result<tokio::io::Stdout> {
        Ok(tokio::io::stdout())
    }
}
