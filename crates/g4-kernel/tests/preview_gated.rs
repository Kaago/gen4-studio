//! The project preview's standard map on a vanilla project, gated by `G4_TEST_PROJECT` (read
//! only: `standard_map` reads the header table, `mapname.bin` and matrix 0 and never unpacks):
//!
//! ```sh
//! G4_TEST_PROJECT=".../Roms/_testkopien/4-0-kernel-pt" \
//!   cargo test -p g4-kernel --all-features standard_map -- --nocapture
//! ```
#![cfg(feature = "test-support")]
mod common;

use g4_formats::game::Family;
use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use g4_kernel::preview::standard_map;
use g4_kernel::project::{AssetKind, Project};

#[test]
fn standard_map_names_the_start_towns() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let expected = match p.family {
        Family::Pt => "T01",
        Family::Hgss => "T20",
        Family::Dp => {
            assert_eq!(standard_map(&p), None, "Diamond/Pearl has no recipe");
            return;
        }
    };
    let found = standard_map(&p).expect("the start town has a map");
    eprintln!("{}: standard map {found:?}", p.game_code);

    let names = p.internal_names().unwrap();
    let headers = p.read_headers().unwrap();
    let header = &headers[usize::from(found.header)];
    assert_eq!(names[usize::from(found.header)], expected);
    assert_eq!(header.matrix_id(), 0);
    assert_eq!(found.area_data_id, u16::from(header.area_data_id()));
    assert!(usize::from(found.map) < p.member_count(AssetKind::Map).unwrap());
    assert!(usize::from(found.area_data_id) < p.member_count(AssetKind::AreaData).unwrap());

    // The map is the town's first cell with a map in matrix 0.
    let bytes = std::fs::read(p.member_path(AssetKind::Matrix, 0)).unwrap();
    let matrix = Matrix::read(&bytes).unwrap();
    let layer = matrix
        .headers
        .as_ref()
        .expect("matrix 0 has a header layer");
    let first = (0..layer.len())
        .position(|i| layer[i] == found.header && matrix.maps[i] != MATRIX_EMPTY)
        .unwrap();
    assert_eq!(matrix.maps[first], found.map);
}
