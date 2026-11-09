//! `studio-bridge`: dev-only HTTP bridge and batch runner for the editor's commands.
//!
//! Built only with the `dev-bridge` feature; it is never part of the app bundle.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use studio_lib::bridge::batch::run_batch;
use studio_lib::bridge::http::{BridgeServer, ServeConfig};
use studio_lib::bridge::roots::Roots;

#[derive(Parser)]
#[command(
    name = "studio-bridge",
    about = "Dev bridge for the Gen 4 Studio commands"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the commands over HTTP on 127.0.0.1 (token from G4_BRIDGE_TOKEN).
    Serve {
        #[arg(long, default_value_t = 1431)]
        port: u16,
        /// The only Origin a browser request may carry.
        #[arg(long, default_value = "http://127.0.0.1:1430")]
        origin: String,
        /// Folder projects may be opened from and ROMs built into (repeatable).
        #[arg(long = "allow-root", required = true)]
        allow_root: Vec<PathBuf>,
        /// Folder for the agent settings (agents.json), created if missing; without it they
        /// stay in memory.
        #[arg(long = "app-data")]
        app_data: Option<PathBuf>,
    },
    /// Run `[[command, args], …]` from a JSON file and print one JSON line per command.
    Batch {
        file: PathBuf,
        /// Folder projects may be opened from and ROMs built into (repeatable).
        #[arg(long = "allow-root", required = true)]
        allow_root: Vec<PathBuf>,
    },
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Serve {
            port,
            origin,
            allow_root,
            app_data,
        } => {
            let token = std::env::var("G4_BRIDGE_TOKEN")
                .ok()
                .filter(|t| !t.is_empty())
                .ok_or("G4_BRIDGE_TOKEN fehlt")?;
            let roots = Roots::new(&allow_root)?;
            let server = BridgeServer::start(ServeConfig {
                port,
                token,
                origin,
                roots,
                app_data,
            })?;
            println!(
                "studio-bridge listening on http://127.0.0.1:{}",
                server.addr.port()
            );
            // The worker threads do the work; this one only keeps the process alive.
            loop {
                std::thread::park();
            }
        }
        Command::Batch { file, allow_root } => {
            let roots = Roots::new(&allow_root)?;
            run_batch(&file, roots, &mut std::io::stdout().lock())
        }
    }
}

fn main() -> ExitCode {
    g4_kernel::i18n::init_from_env();
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
