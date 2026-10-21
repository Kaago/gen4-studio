//! The header list.

use std::sync::Arc;

use g4_formats::error::FormatError;
use g4_kernel::error::KernelError;
use g4_kernel::project::HeaderListing;
use g4_kernel::store::MapHeader;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The header list of the store's table, so edits show up before saving. Names (internal
/// and location, `names_error`) come from the project files and the location text bank; the
/// location text follows the edited `location_name` index.
pub fn list_headers_impl(state: &AppState) -> Result<HeaderListing, CmdError> {
    // One short lock: the project and a copy of the edited table of the same session.
    // Decoding the text archive (and unpacking it on first use) happens without the lock.
    let (project, table) = {
        let mut guard = lock(state)?;
        let session = session(&mut guard)?;
        let project = Arc::clone(&session.project);
        let table = session
            .store
            .header_table()
            .map_err(CmdError::from)?
            .to_vec();
        (project, table)
    };
    project.header_listing_for(&table).map_err(CmdError::from)
}

/// Map header `id` as the store holds it, edits included.
pub fn get_header_impl(state: &AppState, id: u16) -> Result<MapHeader, CmdError> {
    let mut guard = lock(state)?;
    let table = session(&mut guard)?
        .store
        .header_table()
        .map_err(CmdError::from)?;
    table.get(usize::from(id)).cloned().ok_or_else(|| {
        KernelError::Format(FormatError::OutOfRange {
            what: "header",
            value: usize::from(id),
            max: table.len().saturating_sub(1),
        })
        .into()
    })
}

/// Replaces header `id` as a whole; `label` names the change in the undo history. The table
/// cannot grow and a header keeps its game family; an identical header records no change.
pub fn set_header_impl(
    state: &AppState,
    id: u16,
    header: MapHeader,
    label: &str,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.edit_header(id, label, |slot| {
        *slot = header;
        Ok(())
    })?;
    Ok(())
}

#[tauri::command]
pub fn list_headers(state: State<AppState>) -> Result<HeaderListing, CmdError> {
    list_headers_impl(&state)
}

#[tauri::command]
pub fn get_header(state: State<AppState>, id: u16) -> Result<MapHeader, CmdError> {
    get_header_impl(&state, id)
}

#[tauri::command]
pub fn set_header(
    state: State<AppState>,
    id: u16,
    header: MapHeader,
    label: Option<String>,
) -> Result<(), CmdError> {
    set_header_impl(
        &state,
        id,
        header,
        label.as_deref().unwrap_or("edit header"),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{dirty_impl, open_project_impl, redo_impl, save_impl, undo_impl};
    use super::*;
    use g4_kernel::project::{AssetId, AssetKind};
    use g4_kernel::store::{MapHeader, MapHeaderHGSS};

    #[test]
    fn headers_carry_their_event_file_id() {
        let (_fixture, state) = open_synthetic();
        // The synthetic arm9 is zero-filled, so every header points at event file 0.
        let headers = list_headers_impl(&state).unwrap().headers;
        assert!(headers.iter().all(|h| h.event_file_id == 0));
    }

    fn header_table_id() -> AssetId {
        AssetId {
            kind: AssetKind::HeaderTable,
            index: 0,
        }
    }

    #[test]
    fn set_header_changes_the_list_before_saving() {
        let (_s, state) = open_synthetic();
        let mut h = get_header_impl(&state, 0).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.matrix_id = 1;
            p.weather_id = 3;
        }
        set_header_impl(&state, 0, h.clone(), "edit header").unwrap();
        let listing = list_headers_impl(&state).unwrap();
        assert_eq!(listing.headers[0].matrix_id, 1);
        // The names still come from the project files.
        assert_eq!(listing.headers[0].internal_name, "T01");
        assert_eq!(get_header_impl(&state, 0).unwrap(), h);
        assert_eq!(dirty_impl(&state).unwrap(), vec![header_table_id()]);
    }

    #[test]
    fn the_list_follows_undo_and_redo_of_a_header_edit() {
        let (_s, state) = open_synthetic();
        let before = list_headers_impl(&state).unwrap().headers[1].clone();
        let mut h = get_header_impl(&state, 1).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.area_data_id = 5;
            p.event_file_id = 1;
        }
        set_header_impl(&state, 1, h, "edit header").unwrap();
        let after = list_headers_impl(&state).unwrap().headers[1].clone();
        assert_eq!((after.area_data_id, after.event_file_id), (5, 1));
        assert!(undo_impl(&state).unwrap());
        let undone = list_headers_impl(&state).unwrap().headers[1].clone();
        assert_eq!(
            (undone.area_data_id, undone.event_file_id),
            (before.area_data_id, before.event_file_id)
        );
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(redo_impl(&state).unwrap());
        assert_eq!(
            list_headers_impl(&state).unwrap().headers[1].area_data_id,
            5
        );
    }

    #[test]
    fn the_listed_location_name_follows_the_edited_index() {
        let (_s, state) = open_synthetic();
        let names = |state: &AppState| -> Vec<String> {
            list_headers_impl(state)
                .unwrap()
                .headers
                .into_iter()
                .map(|h| h.location_name)
                .collect()
        };
        // Header 0 names "Twinleaf Town"; header 1 points past the end of the bank.
        assert_eq!(names(&state), ["Twinleaf Town", ""]);
        let set_location = |id: u16, index: u8| {
            let mut h = get_header_impl(&state, id).unwrap();
            if let MapHeader::Pt(p) = &mut h {
                p.location_name = index;
            }
            set_header_impl(&state, id, h, "edit header").unwrap();
        };
        set_location(0, 2);
        set_location(1, 1);
        // Not saved yet, still visible.
        assert_eq!(names(&state), ["Sandgem Town", "Twinleaf Town"]);
        assert_eq!(dirty_impl(&state).unwrap(), vec![header_table_id()]);
        // An index outside the bank gives an empty name again.
        set_location(0, 200);
        assert_eq!(names(&state), ["", "Twinleaf Town"]);
        assert!(undo_impl(&state).unwrap());
        assert_eq!(names(&state), ["Sandgem Town", "Twinleaf Town"]);
        assert!(undo_impl(&state).unwrap());
        assert!(undo_impl(&state).unwrap());
        assert_eq!(names(&state), ["Twinleaf Town", ""]);
    }

    #[test]
    fn an_identical_header_records_no_change() {
        let (_s, state) = open_synthetic();
        let h = get_header_impl(&state, 0).unwrap();
        set_header_impl(&state, 0, h, "edit header").unwrap();
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(!undo_impl(&state).unwrap());
    }

    #[test]
    fn set_header_rejects_a_family_switch_and_unfitting_values() {
        let (_s, state) = open_synthetic();
        let other = MapHeader::HGSS(MapHeaderHGSS::default());
        let err = set_header_impl(&state, 0, other, "edit header").unwrap_err();
        assert_eq!(err.code, "error");
        let mut h = get_header_impl(&state, 0).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            // The Platinum layout packs the flags into 4 bits.
            p.flags = 0xFF;
        }
        assert!(set_header_impl(&state, 0, h, "edit header").is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn unknown_headers_and_a_closed_project_are_errors() {
        let (_s, state) = open_synthetic();
        let err = get_header_impl(&state, 2).unwrap_err();
        assert!(err.message.contains("out of range"), "{}", err.message);
        let h = get_header_impl(&state, 0).unwrap();
        assert!(set_header_impl(&state, 2, h.clone(), "edit header").is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let closed = AppState::default();
        assert_eq!(
            get_header_impl(&closed, 0).unwrap_err().message,
            "no project open"
        );
        assert_eq!(
            set_header_impl(&closed, 0, h, "edit header")
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn a_saved_header_survives_reopening_the_project() {
        let (fixture, state) = open_synthetic();
        let mut h = get_header_impl(&state, 0).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.matrix_id = 1;
            p.weather_id = 3;
        }
        set_header_impl(&state, 0, h.clone(), "edit header").unwrap();
        let report = save_impl(&state).unwrap();
        assert_eq!(report.written, vec![header_table_id()]);
        assert!(dirty_impl(&state).unwrap().is_empty());

        let reopened = AppState::default();
        open_project_impl(&reopened, fixture.dir.path().to_str().unwrap()).unwrap();
        assert_eq!(get_header_impl(&reopened, 0).unwrap(), h);
        assert_eq!(
            list_headers_impl(&reopened).unwrap().headers[0].matrix_id,
            1
        );
    }

    #[test]
    fn a_header_serializes_externally_tagged_by_family() {
        let (_s, state) = open_synthetic();
        let json = serde_json::to_value(get_header_impl(&state, 0).unwrap()).unwrap();
        let pt = json.get("Pt").expect("tagged with the family");
        assert!(pt.get("matrix_id").is_some() && pt.get("weather_id").is_some());
        let back: MapHeader = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), json);
    }

    /// Gated: on every vanilla family the list and the per-header read agree, and the
    /// header is tagged with the project's family.
    #[test]
    fn vanilla_list_and_get_header_agree() {
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let headers = list_headers_impl(&state).unwrap().headers;
        for id in [0u16, 3, 415.min(info.header_count as u16 - 1)] {
            let header = get_header_impl(&state, id).unwrap();
            let summary = &headers[usize::from(id)];
            assert_eq!(header.matrix_id(), summary.matrix_id);
            assert_eq!(header.area_data_id(), summary.area_data_id);
            assert_eq!(header.event_file_id(), summary.event_file_id);
            let tag = match header {
                MapHeader::DP(_) => "DP",
                MapHeader::Pt(_) => "Pt",
                MapHeader::HGSS(_) => "HGSS",
            };
            let expected = if info.family == "Hgss" { "HGSS" } else { tag };
            assert_eq!(tag, expected);
        }
    }
}
