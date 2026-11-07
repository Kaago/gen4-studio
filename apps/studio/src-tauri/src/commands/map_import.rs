//! Map part import and export (`g4_kernel::map_import`): inspect the parts of files or of
//! another map, import the chosen ones as one undo step, export one part to a file.
//!
//! The Tauri commands take the paths of the native file dialog as they are; the bridge
//! confines them to its roots first (`Roots::read_path` / `Roots::write_path`).

use std::path::Path;

use g4_kernel::map_import::{self, Part, PartReport, Source};
use g4_kernel::model_import::ImportOptions;
use g4_kernel::project::AssetId;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Checks the parts of `source` for map `map`; changes nothing. OBJ, glTF and PNG files are
/// converted with `options` (refused when out of range).
pub fn inspect_map_import_impl(
    state: &AppState,
    map: u16,
    source: &Source,
    options: &ImportOptions,
) -> Result<Vec<PartReport>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_import::inspect_with(
        &mut session.store,
        map,
        source,
        options,
    )?)
}

/// Imports `parts` of `source` into map `map` as one undo step and returns the changed
/// assets. A part with an error refuses the whole call and changes nothing. OBJ, glTF and PNG
/// files are converted with `options`; their new textures go into the area's map texture pack
/// in the same step.
pub fn import_map_parts_impl(
    state: &AppState,
    map: u16,
    source: &Source,
    parts: &[Part],
    keep_buildings: bool,
    options: &ImportOptions,
) -> Result<Vec<AssetId>, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(map_import::import_with(
        &mut session.store,
        map,
        source,
        parts,
        keep_buildings,
        options,
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
    options: Option<ImportOptions>,
) -> Result<Vec<PartReport>, CmdError> {
    inspect_map_import_impl(&state, map, &source, &options.unwrap_or_default())
}

#[tauri::command]
pub fn import_map_parts(
    state: State<AppState>,
    map: u16,
    source: Source,
    parts: Vec<Part>,
    keep_buildings: Option<bool>,
    options: Option<ImportOptions>,
) -> Result<Vec<AssetId>, CmdError> {
    import_map_parts_impl(
        &state,
        map,
        &source,
        &parts,
        keep_buildings.unwrap_or(false),
        &options.unwrap_or_default(),
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
    use crate::commands::{dirty_impl, open_synthetic, open_synthetic_for_models};
    use g4_formats::nsbtx::Nsbtx;
    use g4_kernel::map_import::PartCode;
    use g4_kernel::model_import::samples::CUBE_OBJ;
    use g4_kernel::project::{AssetKind, Project};

    fn cube(fixture: &g4_kernel::test_support::Synthetic) -> Source {
        Source::Files(vec![fixture.dir.path().join("import").join(CUBE_OBJ)])
    }

    /// The dialog's German sentences (`importParts.ts`): `PART_TEXTS` has an entry for every
    /// code of the part checks, in the kernel's order, and none for a code the kernel lacks.
    #[test]
    fn every_part_code_has_its_german_sentence() {
        const IMPORT_PARTS_TS: &str =
            include_str!("../../../src/workbench/inspector/importParts.ts");
        let start = IMPORT_PARTS_TS
            .find("export const PART_TEXTS")
            .expect("importParts.ts declares PART_TEXTS");
        let body = &IMPORT_PARTS_TS[start..];
        let body = &body[..body.find("\n};").expect("PART_TEXTS ends with `};`")];
        let codes: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                line.strip_prefix("  \"")?
                    .split_once("\": \"")
                    .map(|(code, _)| code)
            })
            .collect();
        let kernel: Vec<&str> = PartCode::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(codes, kernel);
    }

    #[test]
    fn export_then_import_from_files_and_a_refusal_changes_nothing() {
        let (fixture, state) = open_synthetic();
        let per = fixture.dir.path().join("out.per");
        export_map_part_impl(&state, 2, Part::Permissions, &per).unwrap();
        let source = Source::Files(vec![per.clone()]);
        let report =
            inspect_map_import_impl(&state, 0, &source, &ImportOptions::default()).unwrap();
        assert_eq!(report[0].part, Part::Permissions);
        // The fixture's maps share their permissions: nothing changes, nothing is dirty.
        let changed = import_map_parts_impl(
            &state,
            0,
            &source,
            &[Part::Permissions],
            false,
            &ImportOptions::default(),
        )
        .unwrap();
        assert!(changed.is_empty());

        let bad = fixture.dir.path().join("bad.per");
        std::fs::write(&bad, [0u8; 10]).unwrap();
        let err = import_map_parts_impl(
            &state,
            0,
            &Source::Files(vec![bad]),
            &[Part::Permissions],
            false,
            &ImportOptions::default(),
        )
        .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(err.message.contains("10 bytes"), "{}", err.message);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    /// An OBJ goes through the command like any part: the report is the model with its preview.
    #[test]
    fn inspect_converts_an_obj_over_the_command_path() {
        let (fixture, state) = open_synthetic_for_models();
        let report =
            inspect_map_import_impl(&state, 0, &cube(&fixture), &ImportOptions::default()).unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].part, Part::Model);
        let preview = report[0].preview.as_ref().unwrap();
        assert_eq!(preview.model.as_ref().unwrap().stats.quads, 6);
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    /// `colors: 4` reaches the converter: the cube's new texture is a 4-colour one (format 2).
    #[test]
    fn import_with_options_four_colours() {
        let (fixture, state) = open_synthetic_for_models();
        let options = ImportOptions {
            colors: 4,
            ..ImportOptions::default()
        };
        let report = inspect_map_import_impl(&state, 0, &cube(&fixture), &options).unwrap();
        let textures = &report[0].preview.as_ref().unwrap().textures;
        assert_eq!((textures[0].new, textures[0].colors), (true, 4));
        import_map_parts_impl(&state, 0, &cube(&fixture), &[Part::Model], false, &options).unwrap();
        let project = Project::open(fixture.dir.path()).unwrap();
        let pack = std::fs::read(project.member_path(AssetKind::MapTexture, 0)).unwrap();
        let pack = Nsbtx::read(&pack).unwrap();
        let texture = pack
            .textures
            .iter()
            .find(|t| t.name == "g4_imp_cube")
            .unwrap();
        assert_eq!(texture.format, 2);
    }

    /// Options out of range are refused before anything is checked.
    #[test]
    fn options_out_of_range_are_refused() {
        let (fixture, state) = open_synthetic_for_models();
        let options = ImportOptions {
            unit: 0.0,
            ..ImportOptions::default()
        };
        let err = inspect_map_import_impl(&state, 0, &cube(&fixture), &options).unwrap_err();
        assert!(err.message.contains("unit"), "{}", err.message);
    }
}
