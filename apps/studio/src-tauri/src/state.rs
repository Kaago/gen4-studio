use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use g4_kernel::project::Project;
use g4_kernel::store::Store;

/// The open project and its asset store.
pub struct Session {
    pub project: Arc<Project>,
    pub store: Store,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_is_blocked_only_with_dirty_assets_and_without_force() {
        assert!(exit_is_blocked(1, false));
        assert!(!exit_is_blocked(0, false));
        assert!(!exit_is_blocked(3, true));
    }
}
