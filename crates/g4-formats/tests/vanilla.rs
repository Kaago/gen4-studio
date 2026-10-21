use std::path::{Path, PathBuf};

use g4_formats::narc::Narc;

pub fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

/// Reads the first existing candidate (read-only) and asserts that
/// `Narc::read` followed by `Narc::write` reproduces it byte for byte.
fn assert_round_trips(root: &Path, what: &str, candidates: &[&str]) {
    let path = candidates
        .iter()
        .map(|c| root.join(c))
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("{what} narc not found under {}", root.display()));
    let bytes = std::fs::read(&path).unwrap();
    let narc = Narc::read(&bytes).unwrap();
    let written = narc.write();
    if written != bytes {
        let at = written.iter().zip(&bytes).position(|(a, b)| a != b);
        panic!(
            "{what} ({}) not byte-identical: len {} vs {}, first diff at {:?}, written {:?} vs original {:?}",
            path.display(),
            written.len(),
            bytes.len(),
            at,
            at.map(|i| written[i]),
            at.map(|i| bytes[i]),
        );
    }
}

#[test]
fn vanilla_area_data_narc_round_trips_byte_identical() {
    let Some(root) = vanilla() else { return };
    assert_round_trips(
        &root,
        "area data",
        &["files/fielddata/areadata/area_data.narc", "files/a/0/4/2"],
    );
}

#[test]
fn vanilla_maps_narc_round_trips_byte_identical() {
    let Some(root) = vanilla() else { return };
    assert_round_trips(
        &root,
        "maps",
        &[
            "files/fielddata/land_data/land_data.narc",
            "files/fielddata/land_data/land_data_release.narc",
            "files/a/0/6/5",
        ],
    );
}

#[test]
fn vanilla_matrix_narc_round_trips_byte_identical() {
    let Some(root) = vanilla() else { return };
    assert_round_trips(
        &root,
        "matrix",
        &["files/fielddata/mapmatrix/map_matrix.narc", "files/a/0/4/1"],
    );
}
