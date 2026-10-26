//! Rooms: a room that shares its maps with other rooms gets copies of its own.

use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::error::KernelError;
use g4_kernel::project::AssetKind;
use g4_kernel::store::MapHeader;
use serde::Serialize;
use tauri::State;

use super::wizards::{Created, check_member, check_name, check_writable, id, late};
use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// What `split_room` did: the new matrix of the room and the map copies in it (each copied
/// map once, in the order the matrix first names it).
#[derive(Debug, Serialize)]
pub struct SplitReport {
    pub matrix: u16,
    pub maps: Vec<u16>,
}

fn set_matrix_id(header: &mut MapHeader, matrix: u16) {
    match header {
        MapHeader::DP(h) => h.matrix_id = matrix,
        MapHeader::Pt(h) => h.matrix_id = matrix,
        MapHeader::HGSS(h) => h.matrix_id = matrix,
    }
}

/// Gives room `header` copies of the maps of the matrix it shares with other headers: each
/// map of that matrix is copied once (a map in several cells gets one copy, entered in each),
/// a new matrix named "<old name> (<header>)" takes the size, sections and values of the
/// shared one with the copies in place of the maps, and the header moves onto it. Refuses a
/// matrix with a header section (a world map) and one no other header uses, before it
/// creates anything. The copies and the matrix are new members that stay; filling the
/// matrix and moving the header are one undo step, labelled with `name` (else the header
/// id).
pub fn split_room_impl(
    state: &AppState,
    header: u16,
    name: Option<String>,
) -> Result<SplitReport, CmdError> {
    let mut guard = lock(state)?;
    let store = &mut session(&mut guard)?.store;

    let headers = store.header_table()?;
    let Some(room) = headers.get(usize::from(header)) else {
        return Err(format!(
            "header {header} does not exist (the game has {})",
            headers.len()
        )
        .into());
    };
    let shared = room.matrix_id();
    let alone = headers
        .iter()
        .enumerate()
        .all(|(i, h)| i == usize::from(header) || h.matrix_id() != shared);
    let mut moved = room.clone();
    check_member(store, AssetKind::Matrix, shared, "matrix")?;
    let source = store.matrix(shared)?.clone();
    if source.headers.is_some() {
        return Err(KernelError::Refused(
            "Nur Räume lassen sich abspalten, keine Gebiete einer Weltkarte.".to_owned(),
        )
        .into());
    }
    if alone {
        return Err(KernelError::Refused(
            "Dieser Raum teilt seine Karten mit niemandem.".to_owned(),
        )
        .into());
    }
    let matrix_name = format!("{} ({header})", source.name);
    check_name(&matrix_name)?;
    // Each map once, in the order the matrix first names it.
    let mut sources: Vec<u16> = Vec::new();
    for &map in &source.maps {
        if map != MATRIX_EMPTY && !sources.contains(&map) {
            check_member(store, AssetKind::Map, map, "map")?;
            sources.push(map);
        }
    }
    check_writable(store, id(AssetKind::HeaderTable, 0))?;
    // The new matrix will be the next member of its archive.
    let next = u16::try_from(store.member_count(AssetKind::Matrix)?)
        .map_err(|_| CmdError::from("the matrix archive has no room for a new member"))?;
    set_matrix_id(&mut moved, next);
    if !store.header_fits(&moved) {
        return Err(format!(
            "header {header} would not fit the game's header layout with matrix {next}"
        )
        .into());
    }

    let mut created = Created::default();
    for &map in &sources {
        let copy = store
            .duplicate_map_with(map, false)
            .map_err(|e| late(&created, "map copy", e))?;
        created.maps.push(copy);
    }
    let matrix = store
        .create_matrix(&matrix_name)
        .map_err(|e| late(&created, "matrix creation", e))?;
    created.matrices.push(matrix);

    let copy_of = |map: u16| {
        sources
            .iter()
            .position(|&s| s == map)
            .map_or(map, |i| created.maps[i])
    };
    let filled = Matrix {
        name: matrix_name.clone(),
        maps: source.maps.iter().map(|&map| copy_of(map)).collect(),
        ..source
    };
    let room_name = name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| header.to_string(), str::to_owned);
    let label = format!("Raum „{room_name}“ abgespalten");
    store
        .transaction(&label, |tx| {
            tx.edit_matrix(matrix, |m| {
                *m = filled;
                Ok(())
            })?;
            tx.edit_header(header, |h| {
                set_matrix_id(h, matrix);
                Ok(())
            })
        })
        .map_err(|e| late(&created, "split", e))?;
    Ok(SplitReport {
        matrix,
        maps: created.maps,
    })
}

#[tauri::command]
pub fn split_room(
    state: State<AppState>,
    header: u16,
    name: Option<String>,
) -> Result<SplitReport, CmdError> {
    split_room_impl(&state, header, name)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{
        create_matrix_impl, get_header_impl, get_map_impl, get_matrix_impl, history_impl,
        member_counts_impl, set_header_impl, set_matrix_cell_impl, set_matrix_sections_impl,
        set_permission_impl, undo_impl,
    };
    use super::*;
    use std::collections::HashMap;

    fn counts(state: &AppState) -> HashMap<AssetKind, usize> {
        member_counts_impl(state).unwrap()
    }

    fn matrix_of(state: &AppState, header: u16) -> u16 {
        get_header_impl(state, header).unwrap().matrix_id()
    }

    // The synthetic project: headers 0 and 1 both on matrix 0 "m" (2 x 1, maps 0 and 1),
    // three maps.

    #[test]
    fn splits_a_shared_room_into_its_own_matrix_with_copies() {
        let (_f, state) = open_synthetic();
        // Sections and values of the shared matrix come along.
        set_matrix_sections_impl(&state, 0, false, true).unwrap();
        set_matrix_cell_impl(&state, 0, 1, 0, Some(1), None, Some(7)).unwrap();

        let report = split_room_impl(&state, 1, Some("Haus".into())).unwrap();
        assert_eq!((report.matrix, report.maps.clone()), (1, vec![3, 4]));

        let m = get_matrix_impl(&state, 1).unwrap();
        assert_eq!(m.name, "m (1)");
        assert_eq!((m.width, m.height), (2, 1));
        assert_eq!(m.maps, vec![Some(3), Some(4)]);
        assert_eq!((m.headers, m.altitudes), (None, Some(vec![0, 7])));
        assert_eq!(matrix_of(&state, 1), 1);
        assert_eq!(matrix_of(&state, 0), 0);
        let shared = get_matrix_impl(&state, 0).unwrap();
        assert_eq!(shared.maps, vec![Some(0), Some(1)]);
        assert_eq!(shared.name, "m");
        // The copies hold the maps as they are (buildings kept).
        assert_eq!(
            get_map_impl(&state, 3).unwrap().buildings.len(),
            get_map_impl(&state, 0).unwrap().buildings.len()
        );
        assert_eq!(
            get_map_impl(&state, 4).unwrap().permissions,
            get_map_impl(&state, 1).unwrap().permissions
        );
        assert_eq!(
            history_impl(&state)
                .unwrap()
                .undo
                .last()
                .map(String::as_str),
            Some("Raum „Haus“ abgespalten")
        );
        let counts = counts(&state);
        assert_eq!(counts[&AssetKind::Map], 5);
        assert_eq!(counts[&AssetKind::Matrix], 2);
    }

    #[test]
    fn without_a_name_the_label_names_the_header() {
        let (_f, state) = open_synthetic();
        split_room_impl(&state, 0, None).unwrap();
        assert_eq!(
            history_impl(&state)
                .unwrap()
                .undo
                .last()
                .map(String::as_str),
            Some("Raum „0“ abgespalten")
        );
        assert_eq!(get_matrix_impl(&state, 1).unwrap().name, "m (0)");
    }

    #[test]
    fn copies_a_map_used_twice_once() {
        let (_f, state) = open_synthetic();
        set_matrix_cell_impl(&state, 0, 1, 0, Some(0), None, None).unwrap();
        let report = split_room_impl(&state, 1, None).unwrap();
        assert_eq!(report.maps, vec![3]);
        assert_eq!(
            get_matrix_impl(&state, report.matrix).unwrap().maps,
            vec![Some(3), Some(3)]
        );
        assert_eq!(counts(&state)[&AssetKind::Map], 4);
    }

    #[test]
    fn refuses_a_room_that_shares_nothing() {
        let (_f, state) = open_synthetic();
        // Header 0 moves to a matrix of its own; header 1 is then alone on matrix 0.
        let own = create_matrix_impl(&state, "eigen").unwrap();
        let mut h = get_header_impl(&state, 0).unwrap();
        if let MapHeader::Pt(p) = &mut h {
            p.matrix_id = own;
        }
        set_header_impl(&state, 0, h, "edit header").unwrap();
        let before = counts(&state);

        let err = split_room_impl(&state, 1, None).unwrap_err();
        assert_eq!(err.message, "Dieser Raum teilt seine Karten mit niemandem.");
        assert_eq!(counts(&state), before);
        assert_eq!(matrix_of(&state, 1), 0);
    }

    #[test]
    fn refuses_an_area_of_a_world_map() {
        let (_f, state) = open_synthetic();
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let before = counts(&state);

        let err = split_room_impl(&state, 1, None).unwrap_err();
        assert_eq!(
            err.message,
            "Nur Räume lassen sich abspalten, keine Gebiete einer Weltkarte."
        );
        assert_eq!(counts(&state), before);
        assert_eq!(matrix_of(&state, 1), 0);
    }

    #[test]
    fn refuses_an_unknown_header_without_creating_anything() {
        let (_f, state) = open_synthetic();
        let before = counts(&state);
        assert!(split_room_impl(&state, 2, None).is_err());
        assert_eq!(counts(&state), before);
    }

    #[test]
    fn one_undo_puts_the_room_back_on_the_shared_matrix() {
        let (_f, state) = open_synthetic();
        let report = split_room_impl(&state, 1, None).unwrap();
        assert!(!undo_impl(&state).unwrap().is_empty());

        assert_eq!(matrix_of(&state, 1), 0);
        // The copies and the new matrix stay, the matrix empty again.
        let m = get_matrix_impl(&state, report.matrix).unwrap();
        assert_eq!((m.width, m.height, m.maps), (1, 1, vec![None]));
        let counts = counts(&state);
        assert_eq!(counts[&AssetKind::Map], 5);
        assert_eq!(counts[&AssetKind::Matrix], 2);
    }

    #[test]
    fn editing_the_split_copy_leaves_the_other_room_alone() {
        let (_f, state) = open_synthetic();
        let before = get_map_impl(&state, 0).unwrap().permissions;
        let report = split_room_impl(&state, 1, None).unwrap();
        let copy = report.maps[0];

        set_permission_impl(&state, copy, 5, 6, 0x5E, 0x80).unwrap();
        assert_ne!(get_map_impl(&state, copy).unwrap().permissions, before);
        assert_eq!(get_map_impl(&state, 0).unwrap().permissions, before);
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(0), Some(1)]
        );
    }
}
