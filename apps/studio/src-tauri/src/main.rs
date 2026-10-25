// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::OsStr;
use std::process::ExitCode;

fn main() -> ExitCode {
    // `studio mcp …` serves the tools headless over stdio; nothing of Tauri starts. The
    // arguments stay `OsString`s: a path that is not UTF-8 must not panic here.
    if std::env::args_os().nth(1).as_deref() == Some(OsStr::new("mcp")) {
        return studio_lib::mcp::main(std::env::args_os().collect());
    }
    studio_lib::run();
    ExitCode::SUCCESS
}
