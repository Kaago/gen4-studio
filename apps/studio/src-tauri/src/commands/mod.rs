//! Tauri commands. Each command is a thin wrapper over a testable `*_impl` function.

mod area;
mod buildings;
mod events;
mod headers;
mod matrix;
mod models;
mod project;
mod world;

pub use area::*;
pub use buildings::*;
pub use events::*;
pub use headers::*;
pub use matrix::*;
pub use models::*;
pub use project::*;
pub use world::*;

use std::sync::MutexGuard;

use crate::errors::CmdError;
use crate::state::{AppState, Session};

fn lock(state: &AppState) -> Result<MutexGuard<'_, Option<Session>>, CmdError> {
    state
        .session
        .lock()
        .map_err(|_| "internal state is poisoned".into())
}

fn session(guard: &mut Option<Session>) -> Result<&mut Session, CmdError> {
    guard.as_mut().ok_or_else(|| "no project open".into())
}

/// A synthetic project opened in a fresh state (shared by the tests of all command modules).
#[cfg(test)]
#[allow(clippy::unwrap_used)]
fn open_synthetic() -> (g4_kernel::test_support::Synthetic, AppState) {
    let fixture = g4_kernel::test_support::synthetic_project();
    let state = AppState::default();
    open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
    (fixture, state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_kernel::project::AssetKind;

    #[test]
    fn commands_unpack_on_first_access_without_help_from_the_wrapper() {
        // None of these call ensure_unpacked themselves; the store unpacks what it loads.
        let (fixture, state) = open_synthetic();
        assert!(!fixture.dir.path().join("unpacked").exists());
        reload_asset_impl(&state, AssetKind::Event, 0).unwrap();
        assert!(
            fixture
                .dir
                .path()
                .join("unpacked/eventFiles/0000")
                .is_file()
        );
        get_matrix_impl(&state, 0).unwrap();
        get_map_impl(&state, 0).unwrap();
        get_events_impl(&state, 1).unwrap();
        set_permission_impl(&state, 2, 0, 0, 1, 0).unwrap();
        for dir in ["matrices", "maps", "eventFiles"] {
            assert!(
                fixture.dir.path().join("unpacked").join(dir).is_dir(),
                "{dir}"
            );
        }
    }

    #[test]
    fn missing_project_and_missing_tool_have_their_own_codes() {
        let err = dirty_impl(&AppState::default()).unwrap_err();
        assert_eq!(
            (err.code, err.message.as_str()),
            ("error", "no project open")
        );
        let (_s, state) = open_synthetic();
        let err = get_map_model_glb_with(&state, 0, 0, None).unwrap_err();
        assert_eq!(err.code, "tool_missing");
        assert!(reload_asset_impl(&AppState::default(), AssetKind::Map, 0).is_err());
    }

    #[test]
    fn commands_fail_without_an_open_project() {
        let state = AppState::default();
        assert!(list_headers_impl(&state).is_err());
        assert!(get_map_impl(&state, 0).is_err());
        assert!(get_events_impl(&state, 0).is_err());
    }
}
