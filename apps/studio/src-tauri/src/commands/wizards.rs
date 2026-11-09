//! Wizard commands: one backend call each for a new area, an extension of an area and a new
//! room with its entrance. The recipes themselves are the kernel's ([`g4_kernel::recipes`]):
//! every call checks everything first, then creates the new members (not undoable), then makes
//! the edits as one undo step. A call that may apply the Dynamic Headers patch does so after
//! the checks, as its first step. The specs and reports keep their shape; a failure after the
//! first member is a `"partial"` error that names what was created.

use g4_kernel::recipes::{self, types};
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

pub use g4_kernel::recipes::area::{AreaReport, AreaSpec};
pub use g4_kernel::recipes::extend::{ExtendReport, ExtendSpec};
pub use g4_kernel::recipes::room::{RoomReport, RoomSpec};
/// Shared with the room commands (`split_room`).
pub(super) use g4_kernel::recipes::types::{Created, check_member, check_name, check_writable, id};

/// The error of a step that failed during a run (see [`types::late`]): code `"partial"` once a
/// member was created, otherwise a plain `"error"`.
pub(super) fn late(created: &Created, step: &str, e: impl std::fmt::Display) -> CmdError {
    types::late(created, step, e).into()
}

/// Creates a matrix of `width` x `height` from copies of maps and, if asked, points a header at
/// it, or places the copies into free cells of a world map (see [`recipes::area::apply`]).
pub fn create_area_impl(state: &AppState, spec: AreaSpec) -> Result<AreaReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let outcome = recipes::area::apply(&mut session.store, &spec);
    // Members may have been created also when a later step failed.
    session.after_write();
    Ok(outcome?.report)
}

/// Copies a map into the empty cell (`x`, `y`) of a matrix, growing the matrix if the cell lies
/// outside it, and optionally sets the cell's header (see [`recipes::extend::apply`]).
pub fn extend_area_impl(state: &AppState, spec: ExtendSpec) -> Result<ExtendReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let outcome = recipes::extend::apply(&mut session.store, &spec);
    session.after_write();
    Ok(outcome?.report)
}

/// Creates a 1 x 1 matrix with a copy of a map, points a header at it and, with an entrance,
/// links a warp outside with a warp inside (see [`recipes::room::apply`]).
pub fn create_room_impl(state: &AppState, spec: RoomSpec) -> Result<RoomReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let outcome = recipes::room::apply(&mut session.store, &spec);
    session.after_write();
    Ok(outcome?.report)
}

#[tauri::command]
pub fn create_area(state: State<AppState>, spec: AreaSpec) -> Result<AreaReport, CmdError> {
    create_area_impl(&state, spec)
}

#[tauri::command]
pub fn extend_area(state: State<AppState>, spec: ExtendSpec) -> Result<ExtendReport, CmdError> {
    extend_area_impl(&state, spec)
}

#[tauri::command]
pub fn create_room(state: State<AppState>, spec: RoomSpec) -> Result<RoomReport, CmdError> {
    create_room_impl(&state, spec)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{
        EventKind, add_event_impl, apply_header_patch_impl, create_matrix_impl, dirty_impl,
        get_events_impl, get_header_impl, get_map_impl, get_matrix_impl, header_patch_status_impl,
        history_impl, list_headers_impl, member_counts_impl, place_slots_impl, reload_asset_impl,
        resize_matrix_impl, set_header_impl, set_matrix_cell_impl, set_matrix_sections_impl,
        set_permission_impl, undo_impl,
    };
    use super::*;
    use g4_kernel::project::{AssetId, AssetKind};
    use g4_kernel::recipes::area::{
        AreaCell, AreaTarget, AssignHeader, EventFileChoice, NewHeader, PlaceChoice,
        auto_internal_name,
    };
    use g4_kernel::recipes::room::{DoorTile, Entrance};
    use g4_kernel::recipes::types::created_text;
    use g4_kernel::store::MapHeader;
    use std::collections::HashMap;

    fn counts(state: &AppState) -> HashMap<AssetKind, usize> {
        member_counts_impl(state).unwrap()
    }

    fn asset(kind: AssetKind, index: u16) -> AssetId {
        AssetId { kind, index }
    }

    fn cell(x: u8, y: u8, source_map: u16, clear_buildings: bool) -> AreaCell {
        AreaCell {
            x,
            y,
            source_map,
            clear_buildings,
        }
    }

    fn area_spec() -> AreaSpec {
        AreaSpec {
            name: "Route 230".into(),
            width: 2,
            height: 1,
            cells: vec![cell(0, 0, 1, true), cell(1, 0, 0, false)],
            with_headers: true,
            header: Some(AssignHeader {
                id: 1,
                area_data_id: None,
                event_file: EventFileChoice::New,
                new_header: None,
                allow_patch: false,
                place: None,
            }),
            target: None,
        }
    }

    /// Points header `id` at event file `file`.
    fn set_event_file(state: &AppState, id: u16, file: u16) {
        let mut h = get_header_impl(state, id).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.event_file_id = file;
        }
        set_header_impl(state, id, h, "edit header").unwrap();
    }

    #[test]
    fn create_area_copies_maps_builds_the_matrix_and_assigns_the_header() {
        let (_f, state) = open_synthetic();
        let report = create_area_impl(&state, area_spec()).unwrap();
        assert_eq!(report.matrix, 1);
        assert_eq!(report.maps, vec![(0, 0, 3), (1, 0, 4)]);
        assert_eq!((report.event_file, report.header), (Some(2), Some(1)));

        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!(m.maps, vec![Some(3), Some(4)]);
        assert_eq!(m.headers, Some(vec![1, 1]));
        assert_eq!(m.name, "Route 230");
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        assert!(get_map_impl(&state, 3).unwrap().buildings.is_empty());
        assert_eq!(get_map_impl(&state, 4).unwrap().buildings.len(), 1);
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![
                asset(AssetKind::Matrix, 1),
                asset(AssetKind::HeaderTable, 0)
            ]
        );

        // One undo takes the edits (matrix, header) back; the new members stay.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(get_header_impl(&state, 1).unwrap().matrix_id(), 0);
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.width, m.height, m.maps), (1, 1, vec![None]));
        let counts = counts(&state);
        assert_eq!(counts[&AssetKind::Map], 5);
        assert_eq!(counts[&AssetKind::Matrix], 2);
        assert_eq!(counts[&AssetKind::Event], 3);
    }

    #[test]
    fn create_area_without_a_header_only_builds_the_matrix() {
        let (_f, state) = open_synthetic();
        let spec = AreaSpec {
            with_headers: false,
            header: None,
            ..area_spec()
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.event_file, report.header), (None, None));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.maps, m.headers), (vec![Some(3), Some(4)], None));
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![asset(AssetKind::Matrix, 1)]
        );
    }

    #[test]
    fn create_area_refuses_bad_specs_without_creating_members() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let bad: Vec<AreaSpec> = vec![
            AreaSpec {
                header: Some(AssignHeader {
                    id: 5,
                    area_data_id: None,
                    event_file: EventFileChoice::Keep,
                    new_header: None,
                    allow_patch: false,
                    place: None,
                }),
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(2, 0, 0, false)],
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(0, 0, 0, false), cell(0, 0, 1, false)],
                ..area_spec()
            },
            AreaSpec {
                width: 0,
                ..area_spec()
            },
            AreaSpec {
                cells: vec![cell(0, 0, 9, false)],
                ..area_spec()
            },
            AreaSpec {
                header: Some(AssignHeader {
                    id: 1,
                    area_data_id: None,
                    event_file: EventFileChoice::Copy(9),
                    new_header: None,
                    allow_patch: false,
                    place: None,
                }),
                ..area_spec()
            },
            AreaSpec {
                name: "x".repeat(256),
                ..area_spec()
            },
        ];
        for spec in bad {
            assert!(create_area_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
    }

    #[test]
    fn an_unknown_area_data_is_refused_up_front() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 1,
                area_data_id: Some(200),
                event_file: EventFileChoice::Keep,
                new_header: None,
                allow_patch: false,
                place: None,
            }),
            ..area_spec()
        };
        assert!(create_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    fn extend_spec() -> ExtendSpec {
        ExtendSpec {
            matrix: 0,
            x: 2,
            y: 0,
            source_map: 2,
            clear_buildings: false,
            header_cell: None,
        }
    }

    #[test]
    fn extend_area_grows_the_matrix_and_places_the_copy() {
        let (_f, state) = open_synthetic();
        let report = extend_area_impl(&state, extend_spec()).unwrap();
        assert_eq!((report.map, report.width, report.height), (3, 3, 1));
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(0), Some(1), Some(3)]
        );

        // The cell (0, 0) is taken: refused, and no map is copied for it.
        let maps = counts(&state)[&AssetKind::Map];
        let taken = ExtendSpec {
            x: 0,
            ..extend_spec()
        };
        assert!(extend_area_impl(&state, taken).is_err());
        assert_eq!(counts(&state)[&AssetKind::Map], maps);
    }

    #[test]
    fn extend_area_can_set_the_header_of_the_new_cell() {
        let (_f, state) = open_synthetic();
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let spec = ExtendSpec {
            clear_buildings: true,
            header_cell: Some(1),
            ..extend_spec()
        };
        extend_area_impl(&state, spec).unwrap();
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().headers,
            Some(vec![0, 0, 1])
        );
    }

    #[test]
    fn extend_area_refuses_what_cannot_work_before_copying_the_map() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let bad = [
            // The matrix has no header section; unknown map and matrix; the grid would grow
            // past 255 cells.
            ExtendSpec {
                header_cell: Some(1),
                ..extend_spec()
            },
            ExtendSpec {
                source_map: 9,
                ..extend_spec()
            },
            ExtendSpec {
                matrix: 7,
                ..extend_spec()
            },
            ExtendSpec {
                x: 255,
                ..extend_spec()
            },
        ];
        for spec in bad {
            assert!(extend_area_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
        // With the section, a header the game does not have is still refused.
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let spec = ExtendSpec {
            header_cell: Some(5),
            ..extend_spec()
        };
        assert!(extend_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
    }

    fn room_spec(event_file: EventFileChoice) -> RoomSpec {
        RoomSpec {
            name: "Haus".into(),
            source_map: 2,
            clear_buildings: true,
            header: AssignHeader {
                id: 1,
                area_data_id: None,
                event_file,
                new_header: None,
                allow_patch: false,
                place: None,
            },
            entrance: Some(Entrance {
                outside_header: 0,
                x: 5,
                z: 6,
                inside_x: 16,
                inside_z: 30,
                door_tiles: false,
            }),
        }
    }

    #[test]
    fn create_room_with_entrance_links_both_warps() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (
                report.matrix,
                report.map,
                report.event_file,
                report.outside_warp,
                report.inside_warp
            ),
            (1, 3, Some(2), Some(0), Some(0))
        );
        let outside = get_events_impl(&state, 0).unwrap().warps;
        let w = &outside[0];
        assert_eq!(
            (w.x, w.z, w.dest_header_id, w.dest_warp_id, w.height),
            (5, 6, 1, 0, 0)
        );
        let inside = get_events_impl(&state, 2).unwrap().warps;
        let w = &inside[0];
        assert_eq!(
            (w.x, w.z, w.dest_header_id, w.dest_warp_id, w.height),
            (16, 30, 0, 0, 0)
        );
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!((m.name.as_str(), m.maps), ("Haus", vec![Some(3)]));
        assert!(get_map_impl(&state, 3).unwrap().buildings.is_empty());

        // One undo takes the two warps, the header change and the matrix edit back.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
        assert!(get_events_impl(&state, 2).unwrap().warps.is_empty());
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (0, 1));
    }

    #[test]
    fn create_room_with_shared_event_file_cross_links_both_warps() {
        let (_f, state) = open_synthetic();
        let report = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap();
        assert_eq!(report.event_file, None);
        assert_eq!(
            (report.outside_warp, report.inside_warp),
            (Some(0), Some(1))
        );
        let warps = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(warps.len(), 2);
        assert_eq!((warps[0].dest_header_id, warps[0].dest_warp_id), (1, 1));
        assert_eq!((warps[1].dest_header_id, warps[1].dest_warp_id), (0, 0));
        assert_eq!((warps[1].x, warps[1].z), (16, 30));
    }

    /// Adds `n` warps to event file `file`.
    fn add_warps(state: &AppState, file: u16, n: usize) {
        for i in 0..n {
            add_event_impl(state, file, EventKind::Warp, i as i32, 0, None, None).unwrap();
        }
    }

    #[test]
    fn create_room_links_to_the_end_of_files_that_already_have_warps() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_warps(&state, 0, 1);
        add_warps(&state, 1, 2);
        // The copy of file 1 has its 2 warps, so the room warp becomes warp 2 of file 2; the
        // outside warp becomes warp 1 of file 0.
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), Some(1), Some(2))
        );
        let outside = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(outside.len(), 2);
        let w = &outside[1];
        assert_eq!((w.x, w.z, w.dest_header_id, w.dest_warp_id), (5, 6, 1, 2));
        let inside = get_events_impl(&state, 2).unwrap().warps;
        assert_eq!(inside.len(), 3);
        let w = &inside[2];
        assert_eq!((w.x, w.z, w.dest_header_id, w.dest_warp_id), (16, 30, 0, 1));
    }

    #[test]
    fn create_room_in_a_shared_file_with_warps_appends_both_after_them() {
        let (_f, state) = open_synthetic();
        add_warps(&state, 0, 2);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (None, Some(2), Some(3))
        );
        let warps = get_events_impl(&state, 0).unwrap().warps;
        assert_eq!(warps.len(), 4);
        // The outside warp points at the room warp (3), the room warp back at the outside warp (2).
        assert_eq!(
            (warps[2].x, warps[2].dest_header_id, warps[2].dest_warp_id),
            (5, 1, 3)
        );
        assert_eq!(
            (warps[3].x, warps[3].dest_header_id, warps[3].dest_warp_id),
            (16, 0, 2)
        );
    }

    /// The warps of event file `file` as (x, z, destination header, destination warp).
    fn warps(state: &AppState, file: u16) -> Vec<(u16, u16, u16, u16)> {
        get_events_impl(state, file)
            .unwrap()
            .warps
            .iter()
            .map(|w| (w.x, w.z, w.dest_header_id, w.dest_warp_id))
            .collect()
    }

    #[test]
    fn create_room_points_the_copied_warp_on_the_inside_tile_outside() {
        // Gate 3a (A) 12: the template's exit warp stood on the inside door tile, the wizard
        // added the way back as a second warp on that tile, and the game took the first one.
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_warps(&state, 1, 1);
        add_event_impl(&state, 1, EventKind::Warp, 16, 30, None, None).unwrap();
        let template = warps(&state, 1);
        let report = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), Some(0), Some(1))
        );
        assert!(report.inside_warp_reused);
        // The copy keeps its two warps; the one on the door tile now leads outside.
        let inside = warps(&state, 2);
        assert_eq!(inside.len(), 2);
        assert_eq!(inside[0], template[0]);
        assert_eq!(inside[1], (16, 30, 0, 0));
        assert_eq!(warps(&state, 0), vec![(5, 6, 1, 1)]);
        // The template's own file is untouched.
        assert_eq!(warps(&state, 1), template);
    }

    #[test]
    fn create_room_refuses_a_kept_file_with_a_warp_on_the_inside_tile() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        add_event_impl(&state, 1, EventKind::Warp, 16, 30, None, None).unwrap();
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap_err();
        assert!(
            err.message.contains("warp 0 on tile (16, 30)"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn create_room_refuses_an_outside_tile_that_already_has_a_warp() {
        let (_f, state) = open_synthetic();
        add_event_impl(&state, 0, EventKind::Warp, 5, 6, None, None).unwrap();
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap_err();
        assert!(
            err.message.contains("warp 0 on tile (5, 6)"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn create_room_refuses_an_outside_tile_without_a_map_of_the_outside_header() {
        // Gate 3a (F): x 999 was clamped to the last tile and the entrance landed in an empty cell.
        let (_f, state) = open_synthetic();
        let at = |x: u16, z: u16| {
            let mut spec = room_spec(EventFileChoice::New);
            let e = spec.entrance.as_mut().unwrap();
            (e.x, e.z) = (x, z);
            spec
        };
        let before = counts(&state);
        // Matrix 0 is 2 x 1 maps: 64 x 32 tiles.
        let err = create_room_impl(&state, at(64, 6)).unwrap_err();
        assert!(
            err.message.contains("outside matrix 0000"),
            "{}",
            err.message
        );
        let err = create_room_impl(&state, at(5, 32)).unwrap_err();
        assert!(
            err.message.contains("outside matrix 0000"),
            "{}",
            err.message
        );
        let mut inside = room_spec(EventFileChoice::New);
        inside.entrance.as_mut().unwrap().inside_x = 32;
        assert!(create_room_impl(&state, inside).is_err());
        assert_eq!(counts(&state), before);

        // A third, empty cell.
        resize_matrix_impl(&state, 0, 3, 1).unwrap();
        let err = create_room_impl(&state, at(70, 6)).unwrap_err();
        assert!(err.message.contains("has no map"), "{}", err.message);
        // A cell the header section gives to header 1.
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        set_matrix_cell_impl(&state, 0, 1, 0, None, Some(1), None).unwrap();
        let err = create_room_impl(&state, at(40, 6)).unwrap_err();
        assert!(
            err.message.contains("of header 0001, not of header 0000"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
        // Header 0's own cell is fine.
        assert!(create_room_impl(&state, at(5, 6)).is_ok());
    }

    fn tile(state: &AppState, map: u16, x: usize, y: usize) -> [u8; 2] {
        get_map_impl(state, map).unwrap().permissions[y * 32 + x]
    }

    #[test]
    fn create_room_makes_doors_of_tiles_a_warp_would_not_fire_on() {
        let (_f, state) = open_synthetic();
        let mut spec = room_spec(EventFileChoice::New);
        spec.entrance.as_mut().unwrap().door_tiles = true;
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(
            report.door_tiles,
            vec![
                DoorTile { map: 0, x: 5, y: 6 },
                DoorTile {
                    map: report.map,
                    x: 16,
                    y: 30
                }
            ]
        );
        assert_eq!(tile(&state, 0, 5, 6), [0x69, 0x80]);
        assert_eq!(tile(&state, report.map, 16, 30), [0x69, 0x80]);
        // The source map of the room keeps its tile.
        assert_ne!(tile(&state, 2, 16, 30), [0x69, 0x80]);
        // Both tiles are part of the room's one undo step, with the warps.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_ne!(tile(&state, 0, 5, 6), [0x69, 0x80]);
        assert!(warps(&state, 0).is_empty());
    }

    #[test]
    fn create_room_leaves_tiles_alone_that_already_start_a_warp() {
        let (_f, state) = open_synthetic();
        // Stairs outside (any collision), a blocking door inside the source map.
        set_permission_impl(&state, 0, 5, 6, 0x5E, 0).unwrap();
        set_permission_impl(&state, 2, 16, 30, 0x69, 0x80).unwrap();
        let mut spec = room_spec(EventFileChoice::New);
        spec.entrance.as_mut().unwrap().door_tiles = true;
        let report = create_room_impl(&state, spec).unwrap();
        assert!(report.door_tiles.is_empty());
        assert_eq!(tile(&state, 0, 5, 6), [0x5E, 0]);
        // Without the option nothing is checked or set.
        let (_f, state) = open_synthetic();
        let report = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap();
        assert!(report.door_tiles.is_empty());
    }

    #[test]
    fn create_room_without_entrance_makes_no_warps() {
        let (_f, state) = open_synthetic();
        let spec = RoomSpec {
            entrance: None,
            ..room_spec(EventFileChoice::New)
        };
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(
            (report.event_file, report.outside_warp, report.inside_warp),
            (Some(2), None, None)
        );
        assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
    }

    /// Unpacks the event archive and fills event file 1 with 17 zero bytes (read-only: the
    /// codec would write 16 back).
    fn lock_event_file_1(fixture: &g4_kernel::test_support::Synthetic, state: &AppState) {
        reload_asset_impl(state, AssetKind::Event, 0).unwrap();
        std::fs::write(
            fixture.dir.path().join("unpacked/eventFiles/0001"),
            [0u8; 17],
        )
        .unwrap();
    }

    #[test]
    fn create_room_refuses_a_locked_outside_file_before_creating_anything() {
        let (fixture, state) = open_synthetic();
        lock_event_file_1(&fixture, &state);
        set_event_file(&state, 0, 1);
        let before = counts(&state);
        let err = create_room_impl(&state, room_spec(EventFileChoice::New)).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        assert_eq!(counts(&state), before);
        // Only the header edit of the test's own setup is pending.
        assert_eq!(
            dirty_impl(&state).unwrap(),
            vec![asset(AssetKind::HeaderTable, 0)]
        );
    }

    #[test]
    fn create_room_refuses_a_locked_room_file_before_creating_anything() {
        let (fixture, state) = open_synthetic();
        lock_event_file_1(&fixture, &state);
        let before = counts(&state);
        // Copy: the file to copy from is the locked one.
        let err = create_room_impl(&state, room_spec(EventFileChoice::Copy(1))).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        // Keep: the room header's own file is the locked one.
        set_event_file(&state, 1, 1);
        let err = create_room_impl(&state, room_spec(EventFileChoice::Keep)).unwrap_err();
        assert!(err.message.contains("read-only"), "{}", err.message);
        assert_eq!(counts(&state), before);
        // Without an entrance no event file is written to, so the same room is fine.
        let spec = RoomSpec {
            entrance: None,
            ..room_spec(EventFileChoice::Keep)
        };
        assert!(create_room_impl(&state, spec).is_ok());
    }

    #[test]
    fn create_room_refuses_bad_specs_without_creating_members() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let with_outside = |outside_header: u16| {
            let mut spec = room_spec(EventFileChoice::New);
            spec.entrance.as_mut().unwrap().outside_header = outside_header;
            spec
        };
        let bad = [
            RoomSpec {
                source_map: 9,
                ..room_spec(EventFileChoice::New)
            },
            RoomSpec {
                name: "x".repeat(256),
                ..room_spec(EventFileChoice::New)
            },
            room_spec(EventFileChoice::Copy(9)),
            with_outside(9),
            // The room's header cannot be its own outside header.
            with_outside(1),
        ];
        for spec in bad {
            assert!(create_room_impl(&state, spec.clone()).is_err(), "{spec:?}");
            assert_eq!(counts(&state), before);
            assert!(dirty_impl(&state).unwrap().is_empty());
        }
    }

    #[test]
    fn created_text_names_the_members_of_a_partial_run() {
        let created = Created {
            matrices: vec![1],
            maps: vec![3, 4],
            ..Created::default()
        };
        assert_eq!(
            created_text(&created),
            "matrix 0001 and maps 0003, 0004 were created"
        );
        assert_eq!(created_text(&Created::default()), "nothing was created");
        let created = Created {
            matrices: vec![1],
            maps: vec![3],
            event_files: vec![2],
            ..Created::default()
        };
        assert_eq!(
            created_text(&created),
            "matrix 0001, map 0003 and event file 0002 were created"
        );
        let created = Created {
            maps: vec![3],
            ..Created::default()
        };
        assert_eq!(created_text(&created), "map 0003 was created");
        let created = Created {
            headers: vec![470],
            ..Created::default()
        };
        assert_eq!(created_text(&created), "header 0470 was created");
        let mut created = Created {
            patch: false,
            headers: vec![470],
            matrices: vec![1],
            maps: vec![3, 4],
            event_files: vec![2],
            trainers: Vec::new(),
        };
        assert_eq!(
            created_text(&created),
            "header 0470, matrix 0001, maps 0003, 0004 and event file 0002 were created"
        );
        created.patch = true;
        assert_eq!(
            created_text(&created),
            "the Dynamic Headers patch was applied and stays, header 0470, matrix 0001, \
             maps 0003, 0004 and event file 0002 were created"
        );
        let patch_only = Created {
            patch: true,
            ..Created::default()
        };
        assert_eq!(
            created_text(&patch_only),
            "the Dynamic Headers patch was applied and stays"
        );
        assert_eq!(late(&patch_only, "place", "boom").code, "partial");
    }

    /// The patched fixture with 2 headers; a new header would get id 2.
    fn patched() -> (g4_kernel::test_support::Synthetic, AppState) {
        let (f, state) = open_synthetic();
        apply_header_patch_impl(&state).unwrap();
        (f, state)
    }

    fn new_header(template: u16) -> Option<NewHeader> {
        Some(NewHeader {
            template,
            internal_name: Some("T_NEW".into()),
        })
    }

    fn header_count(state: &AppState) -> usize {
        list_headers_impl(state).unwrap().headers.len()
    }

    #[test]
    fn new_header_deserializes_when_absent() {
        let a: AssignHeader =
            serde_json::from_str(r#"{"id":1,"area_data_id":null,"event_file":"Keep"}"#).unwrap();
        assert!(a.new_header.is_none());
        let a: AssignHeader = serde_json::from_str(
            r#"{"id":2,"area_data_id":null,"event_file":"New",
                "new_header":{"template":1,"internal_name":"T_NEW"}}"#,
        )
        .unwrap();
        let n = a.new_header.unwrap();
        assert_eq!((n.template, n.internal_name.as_deref()), (1, Some("T_NEW")));
    }

    #[test]
    fn check_assign_refuses_new_header_with_wrong_id() {
        let (_f, state) = patched();
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 5,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("0002"), "{}", err.message);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn check_assign_refuses_new_header_without_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("patch"), "{}", err.message);
        assert_eq!(counts(&state), before);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn check_assign_refuses_a_template_the_game_lacks() {
        let (_f, state) = patched();
        let before = counts(&state);
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(7),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert!(err.message.contains("header 7"), "{}", err.message);
        assert_eq!(counts(&state), before);
        assert_eq!(header_count(&state), 2);
    }

    #[test]
    fn create_area_with_new_header_creates_it_first() {
        let (f, state) = patched();
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.header, report.matrix), (Some(2), 1));
        assert_eq!(header_count(&state), 3);
        let h = get_header_impl(&state, 2).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (1, 2));
        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!(m.headers, Some(vec![2, 2]));
        // The record in mapname.bin is the name, NUL padded.
        let names =
            std::fs::read(f.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
        assert_eq!(names.len(), 3 * 16);
        assert_eq!(&names[32..37], b"T_NEW");
        // The new header is not undoable, the edits (matrix, header) are one undo step.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(header_count(&state), 3);
        assert_eq!(get_header_impl(&state, 2).unwrap().matrix_id(), 0);
    }

    #[test]
    fn create_room_with_new_header_uses_the_template_event_file_on_keep() {
        let (_f, state) = patched();
        set_event_file(&state, 1, 1);
        let mut spec = room_spec(EventFileChoice::Keep);
        spec.header.id = 2;
        spec.header.new_header = new_header(1);
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(header_count(&state), 3);
        let h = get_header_impl(&state, 2).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (report.matrix, 1));
        assert_eq!(report.event_file, None);
        assert!(report.outside_warp.is_some());
    }

    #[test]
    fn a_failure_after_the_new_header_is_partial_and_names_it_first() {
        let created = Created {
            headers: vec![2],
            ..Created::default()
        };
        let err = late(&created, "map copy", "boom");
        assert_eq!(err.code, "partial");
        assert_eq!(
            err.message,
            "header 0002 was created; map copy failed: boom"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_real_failure_after_the_new_header_leaves_it_and_says_so() {
        use std::os::unix::fs::PermissionsExt;
        let (f, state) = patched();
        // Unpack the map archive, then make it read-only: the map copy right after the
        // header creation cannot write.
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        let maps = f.dir.path().join("unpacked/maps");
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = maps.join("probe");
        if std::fs::write(&probe, b"").is_ok() {
            // Running as a user the mode does not bind (root): the failure cannot be made.
            std::fs::remove_file(probe).unwrap();
            return;
        }
        let spec = AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(err.code, "partial");
        assert!(
            err.message
                .starts_with("header 0002 was created; map copy failed"),
            "{}",
            err.message
        );
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn a_failure_after_creating_is_partial_and_one_before_is_a_plain_error() {
        let created = Created {
            maps: vec![3],
            ..Created::default()
        };
        let err = late(&created, "header edit", "boom");
        assert_eq!(err.code, "partial");
        assert_eq!(
            err.message,
            "map 0003 was created; header edit failed: boom"
        );
        assert!(err.asset.is_none());
        let err = late(&Created::default(), "map copy", "boom");
        assert_eq!(err.code, "error");
        assert_eq!(err.message, "nothing was created; map copy failed: boom");
    }

    #[test]
    fn event_file_choices_deserialize_externally_tagged() {
        let keep: EventFileChoice = serde_json::from_str(r#""Keep""#).unwrap();
        let new: EventFileChoice = serde_json::from_str(r#""New""#).unwrap();
        let copy: EventFileChoice = serde_json::from_str(r#"{"Copy":5}"#).unwrap();
        assert_eq!(
            (keep, new, copy),
            (
                EventFileChoice::Keep,
                EventFileChoice::New,
                EventFileChoice::Copy(5)
            )
        );
    }

    /// The new header `2` of the synthetic game as a copy of header 1, with `place` and
    /// `allow_patch`.
    fn area_with_new_header(place: Option<PlaceChoice>, allow_patch: bool) -> AreaSpec {
        AreaSpec {
            header: Some(AssignHeader {
                id: 2,
                new_header: new_header(1),
                place,
                allow_patch,
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        }
    }

    fn summary(state: &AppState, id: u16) -> g4_kernel::project::HeaderSummary {
        list_headers_impl(state).unwrap().headers[usize::from(id)].clone()
    }

    fn patch_applied(state: &AppState) -> bool {
        header_patch_status_impl(state).unwrap().applied
    }

    #[test]
    fn create_area_applies_the_patch_when_allowed() {
        let (f, state) = open_synthetic();
        assert!(!patch_applied(&state));
        // An older header folder is copied away first; the report names the copy.
        let old = f.dir.path().join("unpacked/dynamicHeaders");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("0000"), [1u8; 24]).unwrap();
        let report = create_area_impl(&state, area_with_new_header(None, true)).unwrap();
        assert!(report.patched);
        let snapshot = std::path::PathBuf::from(report.snapshot.unwrap());
        assert!(
            snapshot.starts_with(f.dir.path().join(".g4/backup")),
            "{snapshot:?}"
        );
        assert_eq!(std::fs::read(snapshot.join("0000")).unwrap(), [1u8; 24]);
        assert!(patch_applied(&state));
        assert_eq!(header_count(&state), 3);
        assert_eq!(
            get_header_impl(&state, 2).unwrap().matrix_id(),
            report.matrix
        );
        // With the patch in, `allow_patch` does nothing more.
        let mut again = area_with_new_header(None, true);
        again.header.as_mut().unwrap().id = 3;
        let again = create_area_impl(&state, again).unwrap();
        assert_eq!((again.patched, again.snapshot), (false, None));
    }

    #[test]
    fn create_area_still_refuses_without_allow_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let err = create_area_impl(&state, area_with_new_header(None, false)).unwrap_err();
        assert_eq!(
            err.message,
            "new headers need the dynamic headers patch, apply the patch first"
        );
        assert!(!patch_applied(&state));
        assert_eq!((counts(&state), header_count(&state)), (before, 2));
        // `null` is no permission either.
        let a: AssignHeader = serde_json::from_str(
            r#"{"id":2,"area_data_id":null,"event_file":"New","allow_patch":null}"#,
        )
        .unwrap();
        assert!(!a.allow_patch);
    }

    #[test]
    fn create_area_refuses_the_patch_a_game_does_not_have() {
        let fixture = g4_kernel::test_support::synthetic_project();
        std::fs::write(
            fixture.dir.path().join("header.yaml"),
            "game_title: \"POKEMON D\"\ngamecode: ADAE\nmaker_code: \"01\"\n",
        )
        .unwrap();
        // The fixture's arm9 is sized for Platinum; Diamond/Pearl's table is longer.
        let arm9 = fixture.dir.path().join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes.resize(0xF0000, 0);
        std::fs::write(&arm9, bytes).unwrap();
        let state = AppState::default();
        super::super::open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let before = counts(&state);
        // No cells: the fixture has no Diamond/Pearl map archive to copy from.
        let spec = AreaSpec {
            cells: Vec::new(),
            ..area_with_new_header(None, true)
        };
        let err = create_area_impl(&state, spec).unwrap_err();
        assert_eq!(err.code, "error");
        assert_eq!(
            err.message,
            "new headers need the dynamic headers patch, which this game does not have"
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn an_invalid_place_is_refused_before_the_patch() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        for (place, refusal) in [
            (
                PlaceChoice::New("  ".into()),
                "Der Ort braucht einen Namen.",
            ),
            (
                PlaceChoice::Existing(9),
                "Den Ort 9 gibt es nicht: die Ortsnamen-Bank hat 3 Namen.",
            ),
        ] {
            let spec = area_with_new_header(Some(place), true);
            let err = create_area_impl(&state, spec).unwrap_err();
            assert_eq!((err.code, err.message.as_str()), ("error", refusal));
            assert!(!patch_applied(&state));
            assert_eq!((counts(&state), header_count(&state)), (before.clone(), 2));
        }
    }

    #[test]
    fn create_area_with_a_new_place_appends_it_and_sets_the_header_in_one_step() {
        let (_f, state) = patched();
        let template = summary(&state, 1).location_index;
        let place = Some(PlaceChoice::New(" Neuheim ".into()));
        let report = create_area_impl(&state, area_with_new_header(place, false)).unwrap();
        assert_eq!((report.place, report.patched), (Some(3), false));
        let h = summary(&state, 2);
        assert_eq!((h.location_index, h.location_name.as_str()), (3, "Neuheim"));
        assert_eq!(place_slots_impl(&state).unwrap().count, 4);
        let undo = history_impl(&state).unwrap().undo;
        assert_eq!(undo.last().unwrap(), "Area “Route 230” created");
        // One undo takes the name and the place of the header back; the header stays.
        undo_impl(&state).unwrap();
        assert_eq!(place_slots_impl(&state).unwrap().count, 3);
        let h = summary(&state, 2);
        assert_eq!((h.location_index, h.matrix_id), (template, 0));
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn create_room_with_an_existing_place_sets_it() {
        let (_f, state) = open_synthetic();
        set_event_file(&state, 1, 1);
        let mut spec = room_spec(EventFileChoice::Keep);
        spec.header.place = Some(PlaceChoice::Existing(2));
        let report = create_room_impl(&state, spec).unwrap();
        assert_eq!(report.place, Some(2));
        let h = summary(&state, 1);
        assert_eq!((h.location_index, h.matrix_id), (2, report.matrix));
        let undo = history_impl(&state).unwrap().undo;
        assert!(undo.contains(&"Room “Haus” created".to_owned()), "{undo:?}");
        assert_eq!(place_slots_impl(&state).unwrap().count, 3);
    }

    #[test]
    fn create_room_refuses_a_place_the_bank_lacks_before_creating_anything() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        let mut spec = room_spec(EventFileChoice::New);
        spec.header.place = Some(PlaceChoice::Existing(9));
        let err = create_room_impl(&state, spec).unwrap_err();
        assert_eq!(err.code, "error");
        assert!(
            err.message.contains("Den Ort 9 gibt es nicht"),
            "{}",
            err.message
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn names_the_new_header_automatically_and_uniquely() {
        let (f, state) = patched();
        let unnamed = |id: u16| AreaSpec {
            name: "Neuheim".into(),
            header: Some(AssignHeader {
                id,
                new_header: Some(NewHeader {
                    template: 1,
                    internal_name: None,
                }),
                ..area_spec().header.unwrap()
            }),
            ..area_spec()
        };
        create_area_impl(&state, unnamed(2)).unwrap();
        let mut blank = unnamed(3);
        blank
            .header
            .as_mut()
            .unwrap()
            .new_header
            .as_mut()
            .unwrap()
            .internal_name = Some("  ".into());
        create_area_impl(&state, blank).unwrap();
        assert_eq!(summary(&state, 2).internal_name, "NEUHEIM");
        assert_eq!(summary(&state, 3).internal_name, "NEUHEIM_2");
        let names =
            std::fs::read(f.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
        assert_eq!(&names[48..57], b"NEUHEIM_2");
    }

    #[test]
    fn auto_internal_names_follow_the_frontend_rule_and_stay_within_16_bytes() {
        let none: [String; 0] = [];
        assert_eq!(
            auto_internal_name(&none, " Neues  Gebiet! "),
            "NEUES_GEBIET"
        );
        assert_eq!(
            auto_internal_name(&none, "Zweiblattdorf-Süd"),
            "ZWEIBLATTDORFSD"
        );
        assert_eq!(auto_internal_name(&none, "!!!"), "NEWMAP");
        assert_eq!(auto_internal_name(&none, "a_b c__d_"), "A_B_C_D");
        let long = "ABCDEFGHIJKLMNOPQRS";
        assert_eq!(auto_internal_name(&none, long), "ABCDEFGHIJKLMNOP");
        let taken = ["ABCDEFGHIJKLMNOP".to_owned(), "ABCDEFGHIJKLMN_2".to_owned()];
        assert_eq!(auto_internal_name(&taken, long), "ABCDEFGHIJKLMN_3");
        // A cut that ends in `_` loses it.
        let taken = ["ABCDEFGHIJKLM_OP".to_owned()];
        assert_eq!(
            auto_internal_name(&taken, "ABCDEFGHIJKLM_OP"),
            "ABCDEFGHIJKLM_2"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_after_the_patch_says_the_patch_stays() {
        use std::os::unix::fs::PermissionsExt;
        let (f, state) = open_synthetic();
        // Unpack the map archive, then make it read-only: the map copy after the patch and
        // the header creation cannot write.
        reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
        let maps = f.dir.path().join("unpacked/maps");
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = maps.join("probe");
        if std::fs::write(&probe, b"").is_ok() {
            // Running as a user the mode does not bind (root): the failure cannot be made.
            std::fs::remove_file(probe).unwrap();
            return;
        }
        let err = create_area_impl(&state, area_with_new_header(None, true)).unwrap_err();
        std::fs::set_permissions(&maps, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(err.code, "partial");
        assert!(
            err.message.starts_with(
                "the Dynamic Headers patch was applied and stays, header 0002 was created; \
                 map copy failed"
            ),
            "{}",
            err.message
        );
        assert!(patch_applied(&state));
        assert_eq!(header_count(&state), 3);
    }

    #[test]
    fn old_specs_without_the_new_fields_work_as_before() {
        let (_f, state) = patched();
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 230", "width": 1, "height": 1,
            "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": false }],
            "with_headers": true,
            "header": { "id": 2, "area_data_id": null, "event_file": "New",
                        "new_header": { "template": 1, "internal_name": "T_NEW" } },
        }))
        .unwrap();
        let assign = spec.header.as_ref().unwrap();
        assert_eq!((assign.allow_patch, &assign.place), (false, &None));
        let template = summary(&state, 1).location_index;
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.patched, report.place), (false, None));
        let h = summary(&state, 2);
        assert_eq!(
            (h.internal_name.as_str(), h.location_index),
            ("T_NEW", template)
        );
        let undo = history_impl(&state).unwrap().undo;
        assert_eq!(undo.last().unwrap(), "Area “Route 230” created");
        // A new header without an internal name deserializes too.
        let n: NewHeader = serde_json::from_str(r#"{"template":1}"#).unwrap();
        assert_eq!(n.internal_name, None);
        let p: PlaceChoice = serde_json::from_str(r#"{"New":"Neuheim"}"#).unwrap();
        assert_eq!(p, PlaceChoice::New("Neuheim".into()));
        let p: PlaceChoice = serde_json::from_str(r#"{"Existing":3}"#).unwrap();
        assert_eq!(p, PlaceChoice::Existing(3));
    }

    /// A world map as matrix 1 (4 x 2, header section): the area of header 1 at (0, 0),
    /// landscape (map 1, header 0, altitude 5) at (1, 0), the other cells empty.
    fn world_map(state: &AppState) -> u16 {
        let world = create_matrix_impl(state, "Welt").unwrap();
        set_matrix_sections_impl(state, world, true, false).unwrap();
        resize_matrix_impl(state, world, 4, 2).unwrap();
        set_matrix_cell_impl(state, world, 0, 0, Some(0), Some(1), None).unwrap();
        set_matrix_cell_impl(state, world, 1, 0, Some(1), Some(0), Some(5)).unwrap();
        world
    }

    fn on_world(world: u16, x: u8, y: u8) -> AreaSpec {
        AreaSpec {
            target: Some(AreaTarget {
                matrix: world,
                x,
                y,
            }),
            ..area_spec()
        }
    }

    const E: Option<u16> = None;

    #[test]
    fn places_an_area_into_free_cells_of_a_world_map() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let matrices = counts(&state)[&AssetKind::Matrix];
        let report = create_area_impl(&state, on_world(world, 2, 0)).unwrap();
        assert_eq!((report.matrix, report.replaced), (world, 0));
        assert_eq!(report.maps, vec![(0, 0, 3), (1, 0, 4)]);
        assert_eq!((report.event_file, report.header), (Some(2), Some(1)));
        assert_eq!(counts(&state)[&AssetKind::Matrix], matrices);

        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.maps, vec![Some(0), Some(1), Some(3), Some(4), E, E, E, E]);
        assert_eq!(m.headers, Some(vec![1, 0, 1, 1, 0, 0, 0, 0]));
        assert_eq!((m.width, m.height, m.name.as_str()), (4, 2, "Welt"));
        let h = get_header_impl(&state, 1).unwrap();
        assert_eq!((h.matrix_id(), h.event_file_id()), (world, 2));
    }

    #[test]
    fn replaces_landscape_cells_and_counts_them() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let report = create_area_impl(&state, on_world(world, 1, 0)).unwrap();
        assert_eq!(report.replaced, 1);
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.maps, vec![Some(0), Some(3), Some(4), E, E, E, E, E]);
        assert_eq!(m.headers, Some(vec![1, 1, 1, 0, 0, 0, 0, 0]));
        // The altitude of the replaced landscape cell stays.
        assert_eq!(m.altitudes, Some(vec![0, 5, 0, 0, 0, 0, 0, 0]));
    }

    /// The text of a refused block at x `x`..`x2`, y `y`..`y2`.
    fn not_free(x: u8, x2: u16, y: u8, y2: u16) -> String {
        format!(
            "The cells x {x} to {x2}, y {y} to {y2} are not free: areas lie there or they lie outside the world map."
        )
    }

    #[test]
    fn refuses_a_block_over_an_area_before_copying() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = counts(&state);
        let history = history_impl(&state).unwrap().undo.len();
        let err = create_area_impl(&state, on_world(world, 0, 0)).unwrap_err();
        assert_eq!((err.code, err.message), ("error", not_free(0, 1, 0, 0)));
        assert_eq!(counts(&state), before);
        assert_eq!(history_impl(&state).unwrap().undo.len(), history);
        // A block over the area that only leaves the area's cell unfilled is refused too.
        let spec = AreaSpec {
            cells: vec![cell(1, 0, 0, false)],
            ..on_world(world, 0, 0)
        };
        assert!(create_area_impl(&state, spec).is_err());
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn refuses_a_block_outside_the_world_map() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = counts(&state);
        let err = create_area_impl(&state, on_world(world, 3, 0)).unwrap_err();
        assert_eq!(err.message, not_free(3, 4, 0, 0));
        let tall = AreaSpec {
            height: 2,
            ..on_world(world, 2, 1)
        };
        let err = create_area_impl(&state, tall).unwrap_err();
        assert_eq!(err.message, not_free(2, 3, 1, 2));
        let far = on_world(world, 255, 255);
        let err = create_area_impl(&state, far).unwrap_err();
        assert_eq!(err.message, not_free(255, 256, 255, 255));
        assert_eq!(counts(&state), before);
        // An unknown matrix is refused as well.
        assert!(create_area_impl(&state, on_world(9, 0, 0)).is_err());
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn refuses_a_target_without_a_header_section() {
        let (_f, state) = open_synthetic();
        resize_matrix_impl(&state, 0, 4, 1).unwrap();
        let before = counts(&state);
        let err = create_area_impl(&state, on_world(0, 2, 0)).unwrap_err();
        assert_eq!(
            err.message,
            "Matrix 0000 is not a world map: its cells have no places assigned."
        );
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn one_undo_takes_the_cells_back() {
        let (_f, state) = open_synthetic();
        let world = world_map(&state);
        let before = get_matrix_impl(&state, world).unwrap();
        let maps = counts(&state)[&AssetKind::Map];
        let spec = AreaSpec {
            header: None,
            ..on_world(world, 1, 0)
        };
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.header, report.replaced), (None, 1));
        // Without a header the cells become landscape.
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!(m.headers, Some(vec![1, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(m.maps, vec![Some(0), Some(3), Some(4), E, E, E, E, E]);
        assert_eq!(
            history_impl(&state).unwrap().undo.last().unwrap(),
            "Area “Route 230” created"
        );

        assert_eq!(
            undo_impl(&state).unwrap(),
            vec![asset(AssetKind::Matrix, world)]
        );
        let m = get_matrix_impl(&state, world).unwrap();
        assert_eq!((m.maps, m.headers), (before.maps, before.headers));
        // The copies stay.
        assert_eq!(counts(&state)[&AssetKind::Map], maps + 2);
    }

    #[test]
    fn old_specs_without_target_still_make_their_own_matrix() {
        let (_f, state) = open_synthetic();
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 230", "width": 1, "height": 1,
            "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": false }],
            "with_headers": true, "header": null,
        }))
        .unwrap();
        assert_eq!(spec.target, None);
        let report = create_area_impl(&state, spec).unwrap();
        assert_eq!((report.matrix, report.replaced), (1, 0));
        assert_eq!(counts(&state)[&AssetKind::Matrix], 2);
        let spec: AreaSpec = serde_json::from_value(serde_json::json!({
            "name": "Route 231", "width": 1, "height": 1, "cells": [],
            "with_headers": false, "header": null, "target": null,
        }))
        .unwrap();
        assert_eq!(spec.target, None);
        let t: AreaTarget = serde_json::from_str(r#"{"matrix":1,"x":2,"y":3}"#).unwrap();
        assert_eq!(
            t,
            AreaTarget {
                matrix: 1,
                x: 2,
                y: 3
            }
        );
    }
}
