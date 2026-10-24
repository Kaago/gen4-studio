use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

use g4_kernel::data::names::NameTables;
use g4_kernel::data::refs::RefIndex;
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
    /// Names of species, moves, items, ... read from the store's text archives, cached per bank.
    pub names: NameTables,
    /// Who references a species, move, item, trainer or encounter file; built on first use.
    pub refs: RefIndex,
}

impl Session {
    /// A session of `project` with a fresh store and empty caches.
    pub fn new(project: Arc<Project>) -> Session {
        let store = Store::new(Arc::clone(&project));
        Session {
            project,
            store,
            sprites: SpriteCache::new(),
            lsp: None,
            names: NameTables::new(),
            refs: RefIndex::new(),
        }
    }

    /// Brings the reference index up to date after a command changed the store. Cheap when
    /// the index was never built or nothing it reads changed. A failed refresh leaves the index
    /// as it was; its next use tries again (and reports the error there).
    pub fn after_write(&mut self) {
        let _ = self.refs.refresh(&self.store);
    }

    /// Forgets the reference index after the files on disk may have changed behind the store
    /// (a member reloaded from disk): the next use builds it again from the disk, which also
    /// sees members the store never loaded.
    pub fn disk_changed(&mut self) {
        self.refs = RefIndex::new();
    }
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

impl AppState {
    /// The session lock, recovered when a panic poisoned it: the session is plain in-memory
    /// data, and refusing it would lose the unsaved edits it holds (a quit would see no
    /// dirty asset). The single place that locks the session.
    pub fn lock_session(&self) -> MutexGuard<'_, Option<Session>> {
        self.session.lock().unwrap_or_else(|e| e.into_inner())
    }
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
    fn a_poisoned_session_lock_is_recovered_with_its_contents() {
        let state = Arc::new(AppState::default());
        let poisoner = Arc::clone(&state);
        let panicked = std::thread::spawn(move || {
            let _guard = poisoner.session.lock().unwrap();
            panic!("poisoned on purpose");
        })
        .join();
        assert!(panicked.is_err());
        assert!(state.session.is_poisoned());
        assert!(state.lock_session().is_none());
    }

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
