//! Where a shared toolbox ([`crate::mcp::tools::Toolbox::shared`]) finds the user's session:
//! the app's state behind Tauri, or the dev bridge's own.

use std::sync::Arc;

use crate::state::AppState;

/// Hands out the state whose session the agents work on.
pub trait StateSource: Send + Sync + 'static {
    fn state(&self) -> &AppState;
}

/// The app: Tauri manages the state.
impl<R: tauri::Runtime> StateSource for tauri::AppHandle<R> {
    fn state(&self) -> &AppState {
        tauri::Manager::state::<AppState>(self).inner()
    }
}

/// The dev bridge (and the tests) hold the state in an `Arc`.
impl StateSource for Arc<AppState> {
    fn state(&self) -> &AppState {
        self
    }
}
