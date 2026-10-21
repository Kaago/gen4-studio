use std::path::{Path, PathBuf};

use g4_formats::area_data::AreaData;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;
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

/// Reads `root/header.yaml` (`gamecode: CPUD`) and maps it to a [`Family`].
fn family_of(root: &Path) -> Family {
    let yaml = std::fs::read_to_string(root.join("header.yaml")).unwrap();
    let code = yaml
        .lines()
        .find_map(|l| l.strip_prefix("gamecode:"))
        .unwrap_or_else(|| panic!("no gamecode in {}/header.yaml", root.display()))
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    Family::from_game_code(code).unwrap_or_else(|| panic!("unknown game code {code}"))
}

fn assert_map_round_trips(what: &str, bytes: &[u8], family: Family) {
    let map = MapFile::read(bytes, family).unwrap_or_else(|e| panic!("{what}: read failed: {e}"));
    let written = map.write(family);
    if written != bytes {
        let at = written.iter().zip(bytes).position(|(a, b)| a != b);
        panic!(
            "{what}: not byte-identical: len {} vs {}, first diff at {:?}, written {:?} vs original {:?}",
            written.len(),
            bytes.len(),
            at,
            at.map(|i| written[i]),
            at.map(|i| bytes[i]),
        );
    }
}

#[test]
fn vanilla_maps_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let family = family_of(&root);
    let dir = root.join("unpacked/maps");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        assert_map_round_trips(&path.display().to_string(), &bytes, family);
        n += 1;
    }
    assert!(n > 0);
    eprintln!("{n} unpacked maps round-tripped");
}

#[test]
fn vanilla_maps_narc_members_round_trip_as_maps() {
    let Some(root) = vanilla() else { return };
    let family = family_of(&root);
    let path = [
        "files/fielddata/land_data/land_data.narc",
        "files/fielddata/land_data/land_data_release.narc",
        "files/a/0/6/5",
    ]
    .iter()
    .map(|c| root.join(c))
    .find(|p| p.exists())
    .unwrap_or_else(|| panic!("maps narc not found under {}", root.display()));
    let narc = Narc::read(&std::fs::read(&path).unwrap()).unwrap();
    assert!(!narc.members.is_empty());
    for (i, member) in narc.members.iter().enumerate() {
        assert_map_round_trips(&format!("{} member {i}", path.display()), member, family);
    }
    eprintln!("{} narc members round-tripped", narc.members.len());
}

/// Returns every file of `root/unpacked/<dir>` (read-only), sorted by name.
fn unpacked_files(root: &Path, dir: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("unpacked").join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
}

fn assert_identical(what: &str, written: &[u8], bytes: &[u8]) {
    if written != bytes {
        let at = written.iter().zip(bytes).position(|(a, b)| a != b);
        panic!(
            "{what}: not byte-identical: len {} vs {}, first diff at {:?}, written {:?} vs original {:?}",
            written.len(),
            bytes.len(),
            at,
            at.map(|i| written[i]),
            at.map(|i| bytes[i]),
        );
    }
}

#[test]
fn vanilla_platinum_matrix_240_is_four_wide_five_high() {
    let Some(root) = vanilla() else { return };
    if family_of(&root) != Family::Pt {
        eprintln!("skipped: not Platinum");
        return;
    }
    let m = Matrix::read(&std::fs::read(root.join("unpacked/matrices/0240")).unwrap()).unwrap();
    assert_eq!((m.width, m.height), (4, 5));
    assert_eq!(m.map_at(0, 0), Some(176));
    assert_eq!(m.map_at(1, 1), Some(570));
    assert_eq!(m.map_at(2, 1), Some(571));
    assert_eq!(m.map_at(1, 3), Some(574));
    assert_eq!(m.map_at(4, 0), None);
}

#[test]
fn vanilla_matrices_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let files = unpacked_files(&root, "matrices");
    assert!(!files.is_empty());
    for path in &files {
        let what = path.display().to_string();
        let bytes = std::fs::read(path).unwrap();
        let matrix = Matrix::read(&bytes).unwrap_or_else(|e| panic!("{what}: read failed: {e}"));
        assert_identical(&what, &matrix.write(), &bytes);
    }
    eprintln!("{} unpacked matrices round-tripped", files.len());
}

#[test]
fn vanilla_area_data_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let family = family_of(&root);
    let files = unpacked_files(&root, "areaData");
    assert!(!files.is_empty());
    for path in &files {
        let what = path.display().to_string();
        let bytes = std::fs::read(path).unwrap();
        let area =
            AreaData::read(&bytes, family).unwrap_or_else(|e| panic!("{what}: read failed: {e}"));
        assert_identical(&what, &area.write(), &bytes);
    }
    eprintln!("{} unpacked area data files round-tripped", files.len());
}
