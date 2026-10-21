//! Matrix editing: cells, sections, size, name; new matrices and map copies.

use g4_formats::matrix::{MATRIX_EMPTY, Matrix, resize_matrix as resized};
use g4_kernel::project::AssetKind;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// The matrix `index` of the store, or a clear error.
fn matrix_of(session: &mut Session, index: u16) -> Result<&Matrix, CmdError> {
    session.store.matrix(index).map_err(CmdError::from)
}

/// Sets the given fields of the cell at (`x`, `y`); `None` leaves a field as it is. `map` 0xFFFF
/// empties the cell. `header` and `altitude` need the matrix to have that section. All checks
/// run before anything changes, so a refused call edits nothing.
pub fn set_matrix_cell_impl(
    state: &AppState,
    index: u16,
    x: u8,
    y: u8,
    map: Option<u16>,
    header: Option<u16>,
    altitude: Option<u8>,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let (cell, has_headers, has_altitudes) = {
        let m = matrix_of(session, index)?;
        if x >= m.width || y >= m.height {
            return Err(format!(
                "cell ({x}, {y}) is outside the {} x {} matrix",
                m.width, m.height
            )
            .into());
        }
        (
            usize::from(y) * usize::from(m.width) + usize::from(x),
            m.headers.is_some(),
            m.altitudes.is_some(),
        )
    };
    if header.is_some() && !has_headers {
        return Err("the matrix has no header section".into());
    }
    if altitude.is_some() && !has_altitudes {
        return Err("the matrix has no altitude section".into());
    }
    if let Some(map) = map.filter(|&m| m != MATRIX_EMPTY) {
        let count = session.store.member_count(AssetKind::Map)?;
        if usize::from(map) >= count {
            return Err(format!("map {map} does not exist (the game has {count} maps)").into());
        }
    }
    if let Some(header) = header {
        let count = session.project.header_count()?;
        if usize::from(header) >= count {
            return Err(format!("header {header} does not exist (the game has {count})").into());
        }
    }
    session.store.edit_matrix(index, "edit matrix cell", |m| {
        if let Some(map) = map {
            m.maps[cell] = map;
        }
        if let (Some(header), Some(headers)) = (header, m.headers.as_mut()) {
            headers[cell] = header;
        }
        if let (Some(altitude), Some(altitudes)) = (altitude, m.altitudes.as_mut()) {
            altitudes[cell] = altitude;
        }
        Ok(())
    })?;
    Ok(())
}

/// Adds the header and/or altitude section (filled with 0) or removes it, so that the matrix
/// has exactly the sections asked for.
pub fn set_matrix_sections_impl(
    state: &AppState,
    index: u16,
    headers: bool,
    altitudes: bool,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session
        .store
        .edit_matrix(index, "edit matrix sections", |m| {
            let cells = usize::from(m.width) * usize::from(m.height);
            match (headers, m.headers.is_some()) {
                (true, false) => m.headers = Some(vec![0; cells]),
                (false, true) => m.headers = None,
                _ => {}
            }
            match (altitudes, m.altitudes.is_some()) {
                (true, false) => m.altitudes = Some(vec![0; cells]),
                (false, true) => m.altitudes = None,
                _ => {}
            }
            Ok(())
        })?;
    Ok(())
}

/// Resizes the grid to `width` x `height` (each 1 to 255). Cells inside both sizes keep their
/// values, new cells are empty.
pub fn resize_matrix_impl(
    state: &AppState,
    index: u16,
    width: u8,
    height: u8,
) -> Result<(), CmdError> {
    if width == 0 || height == 0 {
        return Err("a matrix needs at least one cell".into());
    }
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.edit_matrix(index, "resize matrix", |m| {
        *m = resized(m, width, height);
        Ok(())
    })?;
    Ok(())
}

pub fn rename_matrix_impl(state: &AppState, index: u16, name: &str) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.store.edit_matrix(index, "rename matrix", |m| {
        m.name = name.to_owned();
        Ok(())
    })?;
    Ok(())
}

/// Appends a new 1 x 1 matrix and returns its index. Written to disk at once, not undoable.
pub fn create_matrix_impl(state: &AppState, name: &str) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.create_matrix(name)?)
}

/// Appends a copy of map `source` and returns its index. Written to disk at once, not undoable.
pub fn duplicate_map_impl(state: &AppState, source: u16) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.duplicate_map(source)?)
}

#[tauri::command]
pub fn set_matrix_cell(
    state: State<AppState>,
    index: u16,
    x: u8,
    y: u8,
    map: Option<u16>,
    header: Option<u16>,
    altitude: Option<u8>,
) -> Result<(), CmdError> {
    set_matrix_cell_impl(&state, index, x, y, map, header, altitude)
}

#[tauri::command]
pub fn set_matrix_sections(
    state: State<AppState>,
    index: u16,
    headers: bool,
    altitudes: bool,
) -> Result<(), CmdError> {
    set_matrix_sections_impl(&state, index, headers, altitudes)
}

#[tauri::command]
pub fn resize_matrix(
    state: State<AppState>,
    index: u16,
    width: u8,
    height: u8,
) -> Result<(), CmdError> {
    resize_matrix_impl(&state, index, width, height)
}

#[tauri::command]
pub fn rename_matrix(state: State<AppState>, index: u16, name: String) -> Result<(), CmdError> {
    rename_matrix_impl(&state, index, &name)
}

#[tauri::command]
pub fn create_matrix(state: State<AppState>, name: String) -> Result<u16, CmdError> {
    create_matrix_impl(&state, &name)
}

#[tauri::command]
pub fn duplicate_map(state: State<AppState>, source: u16) -> Result<u16, CmdError> {
    duplicate_map_impl(&state, source)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{dirty_impl, get_matrix_impl, member_counts_impl, save_impl, undo_impl};
    use super::*;

    #[test]
    fn matrix_dto_carries_the_optional_sections() {
        let (_f, state) = open_synthetic();
        let m = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((m.headers.is_some(), m.altitudes.is_some()), (false, false));
        set_matrix_sections_impl(&state, 0, true, true).unwrap();
        let m = get_matrix_impl(&state, 0).unwrap();
        assert_eq!(m.headers, Some(vec![0, 0]));
        assert_eq!(m.altitudes, Some(vec![0, 0]));
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["altitudes"], serde_json::json!([0, 0]));
    }

    #[test]
    fn cell_edit_changes_only_given_fields_and_needs_the_sections() {
        let (_f, state) = open_synthetic();
        // No sections yet: header and altitude are refused and nothing changes.
        assert!(set_matrix_cell_impl(&state, 0, 1, 0, None, Some(1), None).is_err());
        assert!(set_matrix_cell_impl(&state, 0, 1, 0, None, None, Some(3)).is_err());
        assert!(set_matrix_cell_impl(&state, 0, 1, 0, Some(0), Some(1), None).is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(0), Some(1)]
        );

        set_matrix_sections_impl(&state, 0, true, true).unwrap();
        set_matrix_cell_impl(&state, 0, 1, 0, None, Some(1), Some(5)).unwrap();
        let m = get_matrix_impl(&state, 0).unwrap();
        assert_eq!(m.maps, vec![Some(0), Some(1)]);
        assert_eq!(m.headers, Some(vec![0, 1]));
        assert_eq!(m.altitudes, Some(vec![0, 5]));
        set_matrix_cell_impl(&state, 0, 0, 0, Some(2), None, None).unwrap();
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(2), Some(1)]
        );
        // 0xFFFF empties the cell.
        set_matrix_cell_impl(&state, 0, 0, 0, Some(MATRIX_EMPTY), None, None).unwrap();
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![None, Some(1)]
        );
    }

    #[test]
    fn cell_edit_rejects_out_of_range_values_without_changing_anything() {
        let (_f, state) = open_synthetic();
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        let before = get_matrix_impl(&state, 0).unwrap();
        // Outside the grid, a map the game does not have (3 maps), a header past the table (2).
        assert!(set_matrix_cell_impl(&state, 0, 2, 0, Some(0), None, None).is_err());
        assert!(set_matrix_cell_impl(&state, 0, 0, 1, Some(0), None, None).is_err());
        assert!(set_matrix_cell_impl(&state, 0, 0, 0, Some(3), None, None).is_err());
        assert!(set_matrix_cell_impl(&state, 0, 0, 0, None, Some(2), None).is_err());
        assert!(set_matrix_cell_impl(&state, 7, 0, 0, Some(0), None, None).is_err());
        let after = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((before.maps, before.headers), (after.maps, after.headers));
        assert_eq!(
            set_matrix_cell_impl(&AppState::default(), 0, 0, 0, None, None, None)
                .unwrap_err()
                .message,
            "no project open"
        );
    }

    #[test]
    fn sections_are_added_zero_filled_and_removed_and_undo_restores_them() {
        let (_f, state) = open_synthetic();
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        set_matrix_cell_impl(&state, 0, 0, 0, None, Some(1), None).unwrap();
        // Asking again for the same sections changes nothing, dropping a section removes it.
        set_matrix_sections_impl(&state, 0, true, false).unwrap();
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().headers,
            Some(vec![1, 0])
        );
        set_matrix_sections_impl(&state, 0, false, false).unwrap();
        assert_eq!(get_matrix_impl(&state, 0).unwrap().headers, None);
        assert!(undo_impl(&state).unwrap());
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().headers,
            Some(vec![1, 0])
        );
    }

    #[test]
    fn resize_and_rename_are_undoable_edits() {
        let (_f, state) = open_synthetic();
        resize_matrix_impl(&state, 0, 3, 2).unwrap();
        let m = get_matrix_impl(&state, 0).unwrap();
        assert_eq!((m.width, m.height), (3, 2));
        assert_eq!(m.maps, vec![Some(0), Some(1), None, None, None, None]);
        assert!(resize_matrix_impl(&state, 0, 0, 2).is_err());
        assert!(resize_matrix_impl(&state, 0, 2, 0).is_err());
        rename_matrix_impl(&state, 0, "Neu").unwrap();
        assert_eq!(get_matrix_impl(&state, 0).unwrap().name, "Neu");
        assert!(rename_matrix_impl(&state, 0, &"x".repeat(256)).is_err());
        assert_eq!(get_matrix_impl(&state, 0).unwrap().name, "Neu");
        assert!(undo_impl(&state).unwrap());
        assert!(undo_impl(&state).unwrap());
        assert_eq!(
            get_matrix_impl(&state, 0).unwrap().maps,
            vec![Some(0), Some(1)]
        );
    }

    #[test]
    fn create_matrix_and_duplicate_map_extend_the_archives_and_the_counts() {
        let (fixture, state) = open_synthetic();
        let before = member_counts_impl(&state).unwrap();
        let matrix = create_matrix_impl(&state, "neu").unwrap();
        let map = duplicate_map_impl(&state, 1).unwrap();
        assert_eq!(usize::from(matrix), before[&AssetKind::Matrix]);
        assert_eq!(usize::from(map), before[&AssetKind::Map]);
        let after = member_counts_impl(&state).unwrap();
        assert_eq!(after[&AssetKind::Matrix], before[&AssetKind::Matrix] + 1);
        assert_eq!(after[&AssetKind::Map], before[&AssetKind::Map] + 1);

        // The new matrix is a clean, editable 1 x 1 matrix; the copy can be placed in it.
        let m = get_matrix_impl(&state, matrix).unwrap();
        assert_eq!((m.width, m.height, m.maps), (1, 1, vec![None]));
        assert!(dirty_impl(&state).unwrap().is_empty());
        set_matrix_cell_impl(&state, matrix, 0, 0, Some(map), None, None).unwrap();
        assert_eq!(save_impl(&state).unwrap().written.len(), 1);
        assert!(
            fixture
                .dir
                .path()
                .join(format!("unpacked/maps/{map:04}"))
                .is_file()
        );
        // Creating is not undoable: undo only reverts the cell edit.
        assert!(undo_impl(&state).unwrap());
        assert!(!undo_impl(&state).unwrap());
        assert_eq!(get_matrix_impl(&state, matrix).unwrap().maps, vec![None]);
        assert!(duplicate_map_impl(&state, 99).is_err());
    }
}
