//! Gated: every vanilla BDHC reads, writes back byte for byte and passes the
//! structure check (`G4_TEST_PROJECT`, read-only).

use std::path::{Path, PathBuf};

use g4_formats::bdhc::Bdhc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;

fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
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

/// Median as `statistics.median` (mean of the two middle values) and p90 as
/// `v[int(0.9 * (n - 1))]`, like `gutachten-b-messung.py`.
fn median_p90(sorted: &[usize]) -> (f64, usize) {
    let n = sorted.len();
    let median = if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
    };
    (median, sorted[(9 * (n - 1)) / 10])
}

#[test]
fn vanilla_bdhc_round_trip_byte_identical_and_valid() {
    let Some(root) = vanilla() else { return };
    let family = family_of(&root);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("unpacked/maps"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();

    let mut payloads = Vec::new();
    let mut warnings = Vec::new();
    let mut largest: Option<(usize, String)> = None;
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(path).unwrap();
        let map = MapFile::read(&bytes, family).unwrap_or_else(|e| panic!("{name}: map: {e}"));
        let bdhc = Bdhc::read(&map.bdhc).unwrap_or_else(|e| panic!("{name}: bdhc: {e}"));
        let written = bdhc.write();
        if written != map.bdhc {
            let at = written.iter().zip(&map.bdhc).position(|(a, b)| a != b);
            panic!(
                "{name}: not byte-identical: len {} vs {}, first diff at {at:?}",
                written.len(),
                map.bdhc.len(),
            );
        }
        for problem in bdhc.validate() {
            assert!(!problem.is_error(), "{name}: {}", problem.message());
            eprintln!("warning: map {name}: {}", problem.message());
            warnings.push((name.clone(), problem));
        }
        let payload = bdhc.payload_len();
        if largest.as_ref().is_none_or(|(p, _)| payload > *p) {
            largest = Some((payload, name.clone()));
        }
        payloads.push(payload);
    }

    payloads.sort_unstable();
    let (median, p90) = median_p90(&payloads);
    let (max, max_map) = largest.expect("no maps");
    let max_map: usize = max_map.parse().unwrap();
    eprintln!(
        "{family:?}: {} maps, {} warnings, payload min {} / median {median} / p90 {p90} / max {max} (map {max_map})",
        payloads.len(),
        warnings.len(),
        payloads[0],
    );

    match family {
        Family::Pt => {
            assert_eq!(payloads.len(), 666);
            assert_eq!(warnings.len(), 0);
            assert_eq!((max, max_map), (21_452, 188));
        }
        Family::Hgss => {
            assert_eq!(payloads.len(), 676);
            assert_eq!(warnings.len(), 1);
            assert_eq!((max, max_map), (10_778, 658));
        }
        Family::Dp => assert!(!payloads.is_empty()),
    }
}
