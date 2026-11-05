//! Gated round trips of the Nitro codecs against a vanilla project (`G4_TEST_PROJECT`,
//! read-only). Prints only counts, map names and offsets, never ROM bytes.

use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::nitro_dict;
use g4_formats::nsbmd::{Nsbmd, dict_spans};

pub fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipped: G4_TEST_PROJECT not set");
            None
        }
    }
}

/// The game code of `root/header.yaml` (`gamecode: CPUD`).
fn game_code(root: &Path) -> String {
    let yaml = std::fs::read_to_string(root.join("header.yaml")).unwrap();
    yaml.lines()
        .find_map(|l| l.strip_prefix("gamecode:"))
        .unwrap_or_else(|| panic!("no gamecode in {}/header.yaml", root.display()))
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

fn family_of(code: &str) -> Family {
    Family::from_game_code(code).unwrap_or_else(|| panic!("unknown game code {code}"))
}

/// The error without the values it carries (they may be ROM bytes).
fn kind(e: &FormatError) -> String {
    match e {
        FormatError::Truncated { needed, have } => format!("truncated at {needed} of {have}"),
        FormatError::BadMagic { expected } => format!("bad magic, expected {expected}"),
        FormatError::BadSignature { .. } => "bad signature".into(),
        FormatError::BadSize { .. } => "bad size".into(),
        FormatError::OutOfRange { what, .. } => format!("{what} out of range"),
        FormatError::InvalidUtf8 { what } => format!("{what} not UTF-8"),
        FormatError::Unsupported { what } => format!("{what} unsupported"),
    }
}

/// Number of models and the maps whose head counters differ from the display lists.
struct Expected {
    models: usize,
    stale: &'static [&'static str],
}

fn expected(code: &str) -> Expected {
    match code {
        "CPUD" => Expected {
            models: 666,
            stale: &[],
        },
        "IPKD" => Expected {
            models: 676,
            stale: &["0468", "0475", "0476"],
        },
        "IPKE" => Expected {
            models: 676,
            stale: &["0468", "0473", "0475", "0476"],
        },
        _ => panic!("no expectation for game code {code}"),
    }
}

#[test]
fn vanilla_map_models_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let code = game_code(&root);
    let family = family_of(&code);
    let want = expected(&code);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("unpacked/maps"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    paths.sort();

    let start = Instant::now();
    let (mut models, mut equal, mut dicts, mut spans_total) = (0, 0, 0, 0);
    let mut stale = Vec::new();
    let mut problems = Vec::new();
    for path in &paths {
        let map = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(path).unwrap();
        let file = MapFile::read(&bytes, family).unwrap_or_else(|e| panic!("{map}: {e}"));
        let model = &file.model;
        models += 1;
        let x = match Nsbmd::read(model) {
            Ok(x) => x,
            Err(e) => {
                problems.push(format!("{map}: read failed: {}", kind(&e)));
                continue;
            }
        };
        match x.write() {
            Ok(w) if &w == model => equal += 1,
            Ok(w) => {
                let at = w
                    .iter()
                    .zip(model)
                    .position(|(a, b)| a != b)
                    .unwrap_or(w.len().min(model.len()));
                problems.push(format!(
                    "{map}: not byte-identical: len {} vs {}, first diff at offset {at}",
                    w.len(),
                    model.len()
                ));
            }
            Err(e) => problems.push(format!("{map}: write failed: {}", kind(&e))),
        }
        let spans = dict_spans(model).unwrap_or_else(|e| panic!("{map}: {}", kind(&e)));
        spans_total += spans.len();
        for s in spans {
            let same = nitro_dict::read(model, s.start)
                .ok()
                .and_then(|(d, _)| d.write().ok())
                .is_some_and(|w| w == model[s.clone()]);
            if same {
                dicts += 1;
            } else {
                problems.push(format!("{map}: dictionary at offset {} differs", s.start));
            }
        }
        if x.models.iter().any(|(_, m)| !m.counters_match()) {
            stale.push(map);
        }
    }
    let ms = start.elapsed().as_millis();
    println!(
        "nitro-models {code} models={models} equal={equal} dicts={dicts} stale={} ms={ms}",
        stale.join(",")
    );
    for p in problems.iter().take(20) {
        println!("  {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
    assert_eq!(models, want.models);
    assert_eq!(equal, want.models);
    assert_eq!(spans_total, 6 * want.models);
    assert_eq!(dicts, 6 * want.models);
    assert_eq!(stale, want.stale);
}
