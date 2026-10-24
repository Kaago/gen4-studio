//! Tauri commands. Each command is a thin wrapper over a testable `*_impl` function.

mod area;
mod buildings;
mod data;
mod events;
mod headers;
mod matrix;
mod models;
mod overview;
mod patches;
mod problems;
mod project;
mod scripts;
mod sprites;
mod text;
mod wizards;
mod world;

pub use area::*;
pub use buildings::*;
pub use data::*;
pub use events::*;
pub use headers::*;
pub use matrix::*;
pub use models::*;
pub use overview::*;
pub use patches::*;
pub use problems::*;
pub use project::*;
pub use scripts::*;
pub use sprites::*;
pub use text::*;
pub use wizards::*;
pub use world::*;

use std::sync::MutexGuard;

use crate::errors::CmdError;
use crate::state::{AppState, Session};

pub(crate) fn lock(state: &AppState) -> Result<MutexGuard<'_, Option<Session>>, CmdError> {
    Ok(state.lock_session())
}

pub(crate) fn session(guard: &mut Option<Session>) -> Result<&mut Session, CmdError> {
    guard.as_mut().ok_or_else(|| "no project open".into())
}

/// printf argument of a minimal valid GLB (JSON chunk only, no images) for fake apicula scripts:
/// the real conversion step parses what the tool wrote, so a fake must write a real GLB.
#[cfg(test)]
const FAKE_GLB_PRINTF: &str =
    r#"glTF\002\000\000\000\060\000\000\000\034\000\000\000JSON{"asset":{"version":"2.0"}} "#;

/// The bytes [`FAKE_GLB_PRINTF`] prints.
#[cfg(test)]
fn fake_glb() -> Vec<u8> {
    let json = br#"{"asset":{"version":"2.0"}} "#;
    let mut glb = b"glTF".to_vec();
    glb.extend(2u32.to_le_bytes());
    glb.extend((20 + json.len() as u32).to_le_bytes());
    glb.extend((json.len() as u32).to_le_bytes());
    glb.extend(b"JSON");
    glb.extend(json);
    glb
}

/// A synthetic project opened in a fresh state (shared by the tests of all command modules).
#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) fn open_synthetic() -> (g4_kernel::test_support::Synthetic, AppState) {
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
    fn a_command_works_after_a_panic_poisoned_the_state() {
        let (_fixture, state) = open_synthetic();
        set_permission_impl(&state, 2, 0, 0, 1, 0).unwrap();
        let state = std::sync::Arc::new(state);
        let poisoner = std::sync::Arc::clone(&state);
        let panicked = std::thread::spawn(move || {
            let _guard = lock(&poisoner).unwrap();
            panic!("rotom panicked while the lock was held");
        })
        .join();
        assert!(panicked.is_err());
        assert!(state.session.is_poisoned());
        // The session and its edits survive: commands answer, the store is still there.
        assert_eq!(dirty_impl(&state).unwrap().len(), 1);
        get_map_impl(&state, 0).unwrap();
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
