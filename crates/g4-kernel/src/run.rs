//! Starts an external DS emulator on a built ROM.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::KernelError;

fn missing(reason: impl Into<String>) -> KernelError {
    KernelError::ToolMissing {
        name: "emulator",
        reason: reason.into(),
    }
}

/// Starts the emulator with the ROM as its argument, without waiting; returns the PID.
///
/// On macOS an `emulator` ending in `.app` (an application bundle) is started through
/// `open -a <bundle> <rom>`, and the returned PID is that of `open`, which exits right away.
/// Any other path is executed directly as `<emulator> <rom>`.
///
/// Play mode live starts melonDS itself through [`crate::live::melonds::launch`] so it owns the
/// child process.
pub fn launch_emulator(emulator: &Path, rom: &Path) -> Result<u32, KernelError> {
    if !emulator.exists() {
        return Err(missing(format!("not found: {}", emulator.display())));
    }
    let is_bundle = cfg!(target_os = "macos")
        && emulator
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("app"));
    let mut command = if is_bundle {
        let mut c = Command::new("open");
        c.arg("-a").arg(emulator);
        c
    } else {
        Command::new(emulator)
    };
    command
        .arg(rom)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|e| missing(format!("cannot start {}: {e}", emulator.display())))?;
    let pid = child.id();
    // Reap the child when it ends so it does not linger as a zombie while the editor runs.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}
