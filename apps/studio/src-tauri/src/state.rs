use std::sync::{Arc, Mutex};

use g4_kernel::project::Project;
use g4_kernel::store::Store;

/// The open project and its asset store.
pub struct Session {
    pub project: Arc<Project>,
    pub store: Store,
}

/// Tauri-managed state: `None` until a project is opened.
#[derive(Default)]
pub struct AppState(pub Mutex<Option<Session>>);
