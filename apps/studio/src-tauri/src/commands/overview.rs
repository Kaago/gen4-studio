//! Read-only overviews of all matrices and all warps, for the place tree and the world map.

use g4_formats::matrix::MATRIX_EMPTY;
use g4_kernel::project::AssetKind;
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// One matrix in the overview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixSummary {
    pub index: u16,
    pub name: String,
    pub width: u8,
    pub height: u8,
    pub has_headers: bool,
    pub has_altitudes: bool,
    /// Cells that hold a map.
    pub filled: u32,
}

/// All matrices by index; `unreadable` lists the members that could not be loaded, with the
/// reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixListing {
    pub matrices: Vec<MatrixSummary>,
    pub unreadable: Vec<(u16, String)>,
}

/// One warp of the game and where it leads. `x`/`z` are absolute tiles of the event file's
/// matrix, as in the event editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WarpLink {
    pub event_file: u16,
    pub index: u16,
    pub x: u16,
    pub z: u16,
    pub dest_header_id: u16,
    pub dest_warp_id: u16,
}

/// All warps of all event files, ordered by event file and index; `unreadable` lists the event
/// files that could not be loaded, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WarpListing {
    pub links: Vec<WarpLink>,
    pub unreadable: Vec<(u16, String)>,
}

/// Indices `0..count` as the `u16` the store takes (an archive never holds more members).
fn member_range(count: usize) -> std::ops::Range<u16> {
    0..u16::try_from(count).unwrap_or(u16::MAX)
}

pub fn list_matrices_impl(state: &AppState) -> Result<MatrixListing, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let count = session.store.member_count(AssetKind::Matrix)?;
    let mut listing = MatrixListing {
        matrices: Vec::new(),
        unreadable: Vec::new(),
    };
    for index in member_range(count) {
        match session.store.matrix(index) {
            Ok(m) => listing.matrices.push(MatrixSummary {
                index,
                name: m.name.clone(),
                width: m.width,
                height: m.height,
                has_headers: m.headers.is_some(),
                has_altitudes: m.altitudes.is_some(),
                filled: m.maps.iter().filter(|&&map| map != MATRIX_EMPTY).count() as u32,
            }),
            Err(e) => listing.unreadable.push((index, e.to_string())),
        }
    }
    Ok(listing)
}

pub fn list_warps_impl(state: &AppState) -> Result<WarpListing, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let count = session.store.member_count(AssetKind::Event)?;
    let mut listing = WarpListing {
        links: Vec::new(),
        unreadable: Vec::new(),
    };
    for event_file in member_range(count) {
        match session.store.event(event_file) {
            Ok(file) => {
                for (i, w) in file.warp_events.iter().enumerate() {
                    listing.links.push(WarpLink {
                        event_file,
                        // A class never holds more than u16::MAX events (the editor refuses them).
                        index: u16::try_from(i).unwrap_or(u16::MAX),
                        x: w.x,
                        z: w.z,
                        dest_header_id: w.dest_header_id,
                        dest_warp_id: w.dest_warp_id,
                    });
                }
            }
            Err(e) => listing.unreadable.push((event_file, e.to_string())),
        }
    }
    Ok(listing)
}

/// `async` keeps the first listing of a project (it reads every matrix) off the main thread.
#[tauri::command(async)]
pub fn list_matrices(state: State<AppState>) -> Result<MatrixListing, CmdError> {
    list_matrices_impl(&state)
}

/// `async` keeps the first listing of a project (it reads every event file) off the main thread.
#[tauri::command(async)]
pub fn list_warps(state: State<AppState>) -> Result<WarpListing, CmdError> {
    list_warps_impl(&state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        EventKind, EventPatch, add_event_impl, create_matrix_impl, get_events_impl,
        member_counts_impl, open_project_impl, open_synthetic, reload_asset_impl,
        set_matrix_sections_impl, update_event_impl,
    };
    use g4_kernel::project::AssetKind;

    fn summary(index: u16, name: &str, width: u8, height: u8, filled: u32) -> MatrixSummary {
        MatrixSummary {
            index,
            name: name.to_owned(),
            width,
            height,
            has_headers: false,
            has_altitudes: false,
            filled,
        }
    }

    #[test]
    fn matrix_listing_summarises_every_member_and_follows_new_ones() {
        let (_f, state) = open_synthetic();
        let listing = list_matrices_impl(&state).unwrap();
        assert_eq!(listing.matrices, vec![summary(0, "m", 2, 1, 2)]);
        assert!(listing.unreadable.is_empty());

        create_matrix_impl(&state, "neu").unwrap();
        let listing = list_matrices_impl(&state).unwrap();
        assert_eq!(listing.matrices.len(), 2);
        assert_eq!(listing.matrices[1].index, 1);
        assert_eq!(listing.matrices[1].name, "neu");
        assert_eq!(listing.matrices[1].filled, 0);

        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let listing = list_matrices_impl(&state).unwrap();
        assert!(listing.matrices[0].has_headers);
        assert!(!listing.matrices[0].has_altitudes);
    }

    #[test]
    fn warp_listing_collects_warps_of_all_files() {
        let (_f, state) = open_synthetic();
        assert!(list_warps_impl(&state).unwrap().links.is_empty());

        add_event_impl(&state, 1, EventKind::Warp, 5, 6, None, None).unwrap();
        update_event_impl(
            &state,
            1,
            EventKind::Warp,
            0,
            EventPatch {
                dest_header_id: Some(1),
                dest_warp_id: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
        let listing = list_warps_impl(&state).unwrap();
        assert!(listing.unreadable.is_empty());
        assert_eq!(
            listing.links,
            vec![WarpLink {
                event_file: 1,
                index: 0,
                x: 5,
                z: 6,
                dest_header_id: 1,
                dest_warp_id: 3,
            }]
        );
    }

    #[test]
    fn an_unreadable_event_file_is_reported_not_fatal() {
        let (fixture, state) = open_synthetic();
        reload_asset_impl(&state, AssetKind::Event, 0).unwrap();
        add_event_impl(&state, 0, EventKind::Warp, 1, 2, None, None).unwrap();
        std::fs::write(
            fixture.dir.path().join("unpacked/eventFiles/0001"),
            [1u8, 2, 3],
        )
        .unwrap();
        let listing = list_warps_impl(&state).unwrap();
        assert_eq!(listing.unreadable.len(), 1);
        assert_eq!(listing.unreadable[0].0, 1);
        assert!(!listing.unreadable[0].1.is_empty());
        assert_eq!(listing.links.len(), 1);
        assert_eq!(listing.links[0].event_file, 0);
    }

    #[test]
    fn listings_need_an_open_project() {
        let state = AppState::default();
        assert_eq!(
            list_matrices_impl(&state).unwrap_err().message,
            "No project is open."
        );
        assert_eq!(
            list_warps_impl(&state).unwrap_err().message,
            "No project is open."
        );
    }

    /// Gated: over a whole vanilla game the warp listing is large, and for one header it
    /// equals the warps `get_events` reports for the same event file.
    #[test]
    fn vanilla_warps_agree_with_get_events() {
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let header = if info.family == "Hgss" { 60 } else { 415 };
        let headers = crate::commands::list_headers_impl(&state).unwrap().headers;
        let event_file = headers[header].event_file_id;

        let listing = list_warps_impl(&state).unwrap();
        assert!(listing.links.len() > 1000, "{} links", listing.links.len());
        let counts = member_counts_impl(&state).unwrap();
        assert!(
            listing.unreadable.len() < counts[&AssetKind::Event],
            "{:?}",
            listing.unreadable
        );

        let mut of_file: Vec<_> = listing
            .links
            .iter()
            .filter(|l| l.event_file == event_file)
            .map(|l| (l.index, l.x, l.z, l.dest_header_id, l.dest_warp_id))
            .collect();
        of_file.sort_unstable();
        let expected: Vec<_> = get_events_impl(&state, event_file)
            .unwrap()
            .warps
            .iter()
            .map(|w| (w.index, w.x, w.z, w.dest_header_id, w.dest_warp_id))
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(of_file, expected);
    }
}
