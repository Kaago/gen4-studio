use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use g4_kernel::project::Project;
use g4_kernel::sprites::SpriteCache;
use g4_kernel::store::Store;

use crate::lsp::LspProcess;

/// The open project and its asset store.
pub struct Session {
    pub project: Arc<Project>,
    pub store: Store,
    /// The overworld table and the decoded object sprites of this project.
    pub sprites: SpriteCache,
    /// The `rotom-lsp` of this project, started by `lsp_start` and stopped with the session.
    pub lsp: Option<LspProcess>,
}

/// Tauri-managed state.
#[derive(Default)]
pub struct AppState {
    /// The open session: `None` until a project is opened.
    pub session: Mutex<Option<Session>>,
    /// Set by `force_quit` once the user agreed to discard unsaved edits, so the exit it
    /// triggers is not blocked again.
    pub force_quit: AtomicBool,
}

/// Whether an exit request must be held back to ask the user first.
pub fn exit_is_blocked(dirty: usize, force: bool) -> bool {
    dirty > 0 && !force
}

/// What a quit request (our Quit menu item or an exit request) does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitAction {
    /// Quit now.
    Exit,
    /// Ask the frontend first ("confirm-quit").
    Confirm,
}

/// Asks only while a window can show the prompt and unsaved edits exist, unless the user
/// already chose to discard them (`force`).
pub fn quit_action(window_open: bool, dirty: usize, force: bool) -> QuitAction {
    if window_open && exit_is_blocked(dirty, force) {
        QuitAction::Confirm
    } else {
        QuitAction::Exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_is_blocked_only_with_dirty_assets_and_without_force() {
        assert!(exit_is_blocked(1, false));
        assert!(!exit_is_blocked(0, false));
        assert!(!exit_is_blocked(3, true));
    }

    #[test]
    fn quit_asks_only_with_an_open_window_dirty_assets_and_no_force() {
        assert_eq!(quit_action(true, 1, false), QuitAction::Confirm);
        assert_eq!(quit_action(true, 0, false), QuitAction::Exit);
        assert_eq!(quit_action(true, 3, true), QuitAction::Exit);
        assert_eq!(quit_action(false, 2, false), QuitAction::Exit);
    }
}
