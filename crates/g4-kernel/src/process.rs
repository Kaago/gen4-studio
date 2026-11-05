//! Starting console programs (git, apicula, rotom-lsp) from the app.

use std::process::Command;

/// `CREATE_NO_WINDOW` of `CreateProcess`.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Starts `command` without a console window on Windows: the installed app is a GUI program
/// (`windows_subsystem = "windows"`), so each console program it starts would open a window
/// of its own. Its pipes work as before. Elsewhere nothing changes.
pub fn without_console(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}
