//! The gate tool of phase 6.1: `bake_test` bakes the test surface (Gutachten A §2.7) into a
//! map and appends its two textures to the map's texture pack, as one undo step
//! (`g4_kernel::bake::bake_test`). No curated MCP tool: agents reach it through `g4_call` with
//! `--expert`; painting (6.4a) replaces it.

use std::path::PathBuf;

use g4_kernel::bake::{self, BakeTestOptions, BakeTestReport};
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Bakes the test surface into map `map` (the block's tile `[x, z]`, or the middle of the old
/// model's frame) as one undo step „Testfläche gebacken: Karte NNNN“; with `out_dir` also
/// `bake-NNNN.nsbmd` and `maptex-NNNN.nsbtx` there. `out_dir` is taken as it is: `dispatch`
/// confines it to `build/` first. A refusal (unreadable pack, a block off the map, the bake's
/// budget) writes nothing.
pub fn bake_test_impl(
    state: &AppState,
    map: u16,
    block: Option<[u8; 2]>,
    out_dir: Option<PathBuf>,
) -> Result<BakeTestReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let opts = BakeTestOptions {
        map,
        block: block.map(|[x, z]| (x, z)),
        out_dir,
    };
    let report = bake::bake_test(&mut session.store, &opts)?;
    session.after_write();
    Ok(report)
}

/// The app's way in: a relative `out_dir` is relative to the project folder, as over MCP.
#[tauri::command]
pub fn bake_test(
    state: State<AppState>,
    map: u16,
    block: Option<[u8; 2]>,
    out_dir: Option<String>,
) -> Result<BakeTestReport, CmdError> {
    let out_dir = match out_dir {
        Some(dir) => Some(super::open_project_arc(&state)?.root.join(dir)),
        None => None,
    };
    bake_test_impl(&state, map, block, out_dir)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) mod tests {
    use super::*;
    use crate::commands::{history_impl, open_synthetic};
    use g4_formats::nsbtx::{self, NewTexture};
    use g4_formats::texconv::palette_name;
    use g4_kernel::project::{AssetKind, Project};

    /// A `side` × `side` 16-colour texture with palette `palette_name(name)`.
    fn tex16(name: &str, side: u32) -> NewTexture {
        NewTexture {
            name: name.to_string(),
            palette: palette_name(name),
            width: side,
            height: side,
            format: 3,
            color0: false,
            texels: vec![0x5A; (side * side / 2) as usize],
            colors: (0..16).map(|c| c * 0x421).collect(),
        }
    }

    /// Map texture pack 0 with two 16 × 16 textures, building pack 0 with one (as the
    /// kernel's own `bake_test` tests prepare the synthetic project).
    pub(crate) fn prepare(root: &std::path::Path) {
        let project = Project::open(root).unwrap();
        project.ensure_unpacked(AssetKind::MapTexture).unwrap();
        project.ensure_unpacked(AssetKind::BuildingTexture).unwrap();
        let map = nsbtx::build(&[tex16("gras", 16), tex16("weg", 16)]).unwrap();
        let buildings = nsbtx::build(&[tex16("haus", 32)]).unwrap();
        std::fs::write(project.member_path(AssetKind::MapTexture, 0), map).unwrap();
        std::fs::write(
            project.member_path(AssetKind::BuildingTexture, 0),
            buildings,
        )
        .unwrap();
    }

    #[test]
    fn bake_test_over_the_command_path() {
        let (fixture, state) = open_synthetic();
        prepare(fixture.dir.path());
        let out = fixture.dir.path().join("build/bake");
        let report = bake_test_impl(&state, 0, None, Some(out.clone())).unwrap();
        assert_eq!((report.map, report.pack), (0, 0));
        assert_eq!(report.appended, ["g4_bake_a", "g4_bake_b"]);
        let model_file = out.join("bake-0000.nsbmd");
        let pack_file = out.join("maptex-0000.nsbtx");
        assert_eq!(report.model_file.as_deref(), Some(model_file.as_path()));
        assert_eq!(report.pack_file.as_deref(), Some(pack_file.as_path()));
        assert_eq!(std::fs::read(&model_file).unwrap(), report.report.nsbmd);
        assert!(pack_file.is_file());
        assert_eq!(
            history_impl(&state).unwrap().undo,
            ["Testfläche gebacken: Karte 0000"]
        );
        // `block` is the tile (x, z) as the command layer sends it.
        let at = bake_test_impl(&state, 0, Some([3, 4]), None).unwrap();
        assert_eq!(at.block, (3, 4));
        assert!(at.appended.is_empty(), "{:?}", at.appended);
    }

    #[test]
    fn bake_test_refuses_an_unreadable_pack() {
        let (fixture, state) = open_synthetic();
        prepare(fixture.dir.path());
        let pack = Project::open(fixture.dir.path())
            .unwrap()
            .member_path(AssetKind::MapTexture, 0);
        std::fs::write(&pack, [0; 4]).unwrap();
        let out = fixture.dir.path().join("build/bake");
        let err = bake_test_impl(&state, 0, None, Some(out.clone())).unwrap_err();
        assert_eq!(err.code, "error", "{}", err.message);
        assert_eq!(std::fs::read(&pack).unwrap(), [0; 4]);
        assert!(!out.exists());
        assert!(history_impl(&state).unwrap().undo.is_empty());
    }
}
