//! The header list.

use std::sync::Arc;

use g4_kernel::project::HeaderListing;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

pub fn list_headers_impl(state: &AppState) -> Result<HeaderListing, CmdError> {
    // Decoding the text archive (and unpacking it on first use) happens without the lock.
    let project = {
        let mut guard = lock(state)?;
        Arc::clone(&session(&mut guard)?.project)
    };
    project.header_listing().map_err(CmdError::from)
}

#[tauri::command]
pub fn list_headers(state: State<AppState>) -> Result<HeaderListing, CmdError> {
    list_headers_impl(&state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;

    #[test]
    fn headers_carry_their_event_file_id() {
        let (_fixture, state) = open_synthetic();
        // The synthetic arm9 is zero-filled, so every header points at event file 0.
        let headers = list_headers_impl(&state).unwrap().headers;
        assert!(headers.iter().all(|h| h.event_file_id == 0));
    }
}
