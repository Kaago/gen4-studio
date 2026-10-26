//! Places of the project's own (phase 3f.2): the location names a map header selects. Each
//! change is one undo step of the store (`g4_kernel::places`).

use g4_kernel::places::{self, PlaceCreated, PlaceSlots};
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The room for new places in the location bank, unsaved names included.
pub fn place_slots_impl(state: &AppState) -> Result<PlaceSlots, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(places::place_slots(&mut session.store)?)
}

/// Creates the place `name` and gives it to `headers`, see [`places::create_place`].
pub fn create_place_impl(
    state: &AppState,
    name: &str,
    headers: &[u16],
) -> Result<PlaceCreated, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(places::create_place(&mut session.store, name, headers)?)
}

/// Renames the place at `index` for all its headers, see [`places::rename_place`].
pub fn rename_place_impl(state: &AppState, index: u8, name: &str) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(places::rename_place(&mut session.store, index, name)?)
}

/// Gives the place at `index` to `headers`, see [`places::assign_place`].
pub fn assign_place_impl(state: &AppState, headers: &[u16], index: u8) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(places::assign_place(&mut session.store, headers, index)?)
}

#[tauri::command]
pub fn place_slots(state: State<AppState>) -> Result<PlaceSlots, CmdError> {
    place_slots_impl(&state)
}

#[tauri::command]
pub fn create_place(
    state: State<AppState>,
    name: String,
    headers: Vec<u16>,
) -> Result<PlaceCreated, CmdError> {
    create_place_impl(&state, &name, &headers)
}

#[tauri::command]
pub fn rename_place(state: State<AppState>, index: u8, name: String) -> Result<(), CmdError> {
    rename_place_impl(&state, index, &name)
}

#[tauri::command]
pub fn assign_place(state: State<AppState>, headers: Vec<u16>, index: u8) -> Result<(), CmdError> {
    assign_place_impl(&state, &headers, index)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{list_headers_impl, open_synthetic, undo_impl};

    fn location_names(state: &AppState) -> Vec<String> {
        list_headers_impl(state)
            .unwrap()
            .headers
            .into_iter()
            .map(|h| h.location_name)
            .collect()
    }

    #[test]
    fn a_new_place_shows_in_the_header_list_until_undo() {
        let (_fixture, state) = open_synthetic();
        assert_eq!(place_slots_impl(&state).unwrap().free, 253);
        let created = create_place_impl(&state, "Neuheim", &[0]).unwrap();
        assert_eq!((created.index, created.reused), (3, None));
        assert_eq!(location_names(&state)[0], "Neuheim");
        assert_eq!(place_slots_impl(&state).unwrap().count, 4);
        undo_impl(&state).unwrap();
        assert_eq!(location_names(&state)[0], "Twinleaf Town");
    }

    #[test]
    fn renaming_and_assigning_go_through_the_store() {
        let (_fixture, state) = open_synthetic();
        assign_place_impl(&state, &[1], 1).unwrap();
        rename_place_impl(&state, 1, "Zweiblattdorf").unwrap();
        assert_eq!(location_names(&state), ["Zweiblattdorf", "Zweiblattdorf"]);
        let err = create_place_impl(&state, "", &[0]).unwrap_err();
        assert_eq!(err.message, "Der Ort braucht einen Namen.");
    }

    #[test]
    fn the_commands_need_an_open_project() {
        let state = AppState::default();
        assert!(place_slots_impl(&state).is_err());
        assert!(create_place_impl(&state, "Neuheim", &[0]).is_err());
        assert!(rename_place_impl(&state, 1, "Neuheim").is_err());
        assert!(assign_place_impl(&state, &[0], 1).is_err());
    }
}
