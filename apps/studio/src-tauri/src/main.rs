// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    // `studio mcp …` serves the tools headless over stdio; nothing of Tauri starts.
    if args.get(1).map(String::as_str) == Some("mcp") {
        return studio_lib::mcp::main(args);
    }
    studio_lib::run();
    ExitCode::SUCCESS
}
