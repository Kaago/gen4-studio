//! Map part import and export (`g4_kernel::map_import`): inspect the parts of files or of
//! another map, import the chosen ones as one undo step, export one part to a file.
//!
//! The Tauri commands take the paths of the native file dialog as they are; the bridge
//! confines them to its roots first (`Roots::read_path` / `Roots::write_path`).

use std::path::Path;

use g4_kernel::map_import::{self, Part, PartReport, Source};
use g4_kernel::project::AssetId;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Checks the parts of `source` for map `map`; changes nothing.
pub fn inspect_map_import_impl(
    state: &AppState,
    map: u16,
    source: &Source,
) -> Result<Vec<PartReport>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_import::inspect(&mut session.store, map, source)?)
}

/// Imports `parts` of `source` into map `map` as one undo step and returns the changed
/// assets. A part with an error refuses the whole call and changes nothing.
pub fn import_map_parts_impl(
    state: &AppState,
    map: u16,
    source: &Source,
    parts: &[Part],
    keep_buildings: bool,
) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_import::import(
        &mut session.store,
        map,
        source,
        parts,
        keep_buildings,
    )?)
}

/// Writes `part` of map `map` (unsaved edits included) to `path`.
pub fn export_map_part_impl(
    state: &AppState,
    map: u16,
    part: Part,
    path: &Path,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_import::export(&mut session.store, map, part, path)?)
}

#[tauri::command]
pub fn inspect_map_import(
    state: State<AppState>,
    map: u16,
    source: Source,
) -> Result<Vec<PartReport>, CmdError> {
    inspect_map_import_impl(&state, map, &source)
}

#[tauri::command]
pub fn import_map_parts(
    state: State<AppState>,
    map: u16,
    source: Source,
    parts: Vec<Part>,
    keep_buildings: Option<bool>,
) -> Result<Vec<AssetId>, CmdError> {
    import_map_parts_impl(
        &state,
        map,
        &source,
        &parts,
        keep_buildings.unwrap_or(false),
    )
}

#[tauri::command]
pub fn export_map_part(
    state: State<AppState>,
    map: u16,
    part: Part,
    path: String,
) -> Result<(), CmdError> {
    export_map_part_impl(&state, map, part, Path::new(&path))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{dirty_impl, open_synthetic};

    #[test]
    fn export_then_import_from_files_and_a_refusal_changes_nothing() {
        let (fixture, state) = open_synthetic();
        let per = fixture.dir.path().join("out.per");
        export_map_part_impl(&state, 2, Part::Permissions, &per).unwrap();
        let source = Source::Files(vec![per.clone()]);
        let report = inspect_map_import_impl(&state, 0, &source).unwrap();
        assert_eq!(report[0].part, Part::Permissions);
        // The fixture's maps share their permissions: nothing changes, nothing is dirty.
        let changed =
            import_map_parts_impl(&state, 0, &source, &[Part::Permissions], false).unwrap();
        assert!(changed.is_empty());

        let bad = fixture.dir.path().join("bad.per");
        std::fs::write(&bad, [0u8; 10]).unwrap();
        let err = import_map_parts_impl(
            &state,
            0,
            &Source::Files(vec![bad]),
            &[Part::Permissions],
            false,
        )
        .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(err.message.contains("10 bytes"), "{}", err.message);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }
}
