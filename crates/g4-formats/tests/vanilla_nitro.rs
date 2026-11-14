//! Gated round trips of the Nitro codecs against a vanilla project (`G4_TEST_PROJECT`,
//! read-only). Prints only counts, map names and offsets, never ROM bytes.

use std::path::{Path, PathBuf};
use std::time::Instant;

use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::nitro_dict;
use g4_formats::nsbmd::{Nsbmd, dict_spans};
use g4_formats::nsbtx::{self, Btx0, NewTexture, Nsbtx};

pub fn vanilla() -> Option<PathBuf> {
    g4_formats::test_support::gate_var("G4_TEST_PROJECT").map(PathBuf::from)
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

/// The members of an unpacked NARC folder (files named by digits), sorted.
fn members(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    paths.sort();
    paths
}

fn member_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// Whether the dictionary at `span` writes back to the same bytes.
fn dict_same(bytes: &[u8], span: &std::ops::Range<usize>) -> bool {
    nitro_dict::read(bytes, span.start)
        .ok()
        .and_then(|(d, _)| d.write().ok())
        .is_some_and(|w| w == bytes[span.clone()])
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
    let paths = members(&root.join("unpacked/maps"));

    let start = Instant::now();
    let (mut models, mut equal, mut dicts, mut spans_total) = (0, 0, 0, 0);
    let mut stale = Vec::new();
    let mut problems = Vec::new();
    for path in &paths {
        let map = member_name(path);
        let bytes = std::fs::read(path).unwrap();
        let file = MapFile::read(&bytes, family).unwrap_or_else(|e| panic!("{map}: {}", kind(&e)));
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
            if dict_same(model, &s) {
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

/// Map and building texture packs (members starting with `BTX0`) and the dictionaries of
/// models plus packs.
fn expected_packs(code: &str) -> (usize, usize, usize) {
    match code {
        "CPUD" => (74, 71, 4286),
        "IPKD" | "IPKE" => (106, 94, 4456),
        _ => panic!("no expectation for game code {code}"),
    }
}

/// Dictionaries of all map models that write back byte-identical.
fn model_dicts(root: &Path, family: Family) -> usize {
    members(&root.join("unpacked/maps"))
        .iter()
        .map(|path| {
            let map = member_name(path);
            let bytes = std::fs::read(path).unwrap();
            let file =
                MapFile::read(&bytes, family).unwrap_or_else(|e| panic!("{map}: {}", kind(&e)));
            let spans = dict_spans(&file.model).unwrap_or_else(|e| panic!("{map}: {}", kind(&e)));
            spans.iter().filter(|s| dict_same(&file.model, s)).count()
        })
        .sum()
}

/// The `BTX0` members of `unpacked/<dir>` with their names.
fn packs(root: &Path, dir: &str) -> Vec<(String, Vec<u8>)> {
    members(&root.join("unpacked").join(dir))
        .iter()
        .map(|p| {
            (
                format!("{dir}/{}", member_name(p)),
                std::fs::read(p).unwrap(),
            )
        })
        .filter(|(_, b)| b.starts_with(b"BTX0"))
        .collect()
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()))
}

#[test]
fn vanilla_texture_packs_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let code = game_code(&root);
    let family = family_of(&code);
    let (want_map, want_building, want_dicts) = expected_packs(&code);
    let start = Instant::now();
    let mut counts = [0usize; 2];
    let (mut equal, mut pack_dicts) = (0, 0);
    // Palettes not marked 4-colour (`extra != 1`) off a 16-byte boundary.
    let mut odd16 = 0;
    let mut problems = Vec::new();
    for (k, dir) in ["mapTextures", "buildingTextures"].into_iter().enumerate() {
        for (name, bytes) in packs(&root, dir) {
            counts[k] += 1;
            if let Ok(p) = Btx0::read(&bytes) {
                odd16 += p
                    .palettes
                    .iter()
                    .filter(|e| e.extra != 1 && e.offset8 % 2 != 0)
                    .count();
            }
            match Btx0::read(&bytes).and_then(|p| nsbtx::write(&p)) {
                Ok(w) if w == bytes => equal += 1,
                Ok(w) => problems.push(format!(
                    "{name}: not byte-identical: len {} vs {}, first diff at offset {}",
                    w.len(),
                    bytes.len(),
                    first_diff(&w, &bytes)
                )),
                Err(e) => problems.push(format!("{name}: {}", kind(&e))),
            }
            match nsbtx::dict_spans(&bytes) {
                Ok(spans) => {
                    for s in spans {
                        if dict_same(&bytes, &s) {
                            pack_dicts += 1;
                        } else {
                            problems
                                .push(format!("{name}: dictionary at offset {} differs", s.start));
                        }
                    }
                }
                Err(e) => problems.push(format!("{name}: dict_spans: {}", kind(&e))),
            }
        }
    }
    let dicts_total = model_dicts(&root, family) + pack_dicts;
    eprintln!("nitro-packs {code} ms={}", start.elapsed().as_millis());
    println!(
        "nitro-packs {code} map={} building={} equal={equal} dicts_total={dicts_total} odd16={odd16}",
        counts[0], counts[1]
    );
    for p in problems.iter().take(20) {
        println!("  {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
    assert_eq!(counts, [want_map, want_building]);
    assert_eq!(equal, want_map + want_building);
    assert_eq!(pack_dicts, 2 * (want_map + want_building));
    assert_eq!(dicts_total, want_dicts);
    assert_eq!(odd16, 0, "16-colour palettes off a 16-byte boundary");
}

/// An 8 x 8 synthetic texture: format 3 with indices `i % 16` or format 2 with `i % 4`.
fn synthetic(name: &str, format: u8) -> NewTexture {
    let (per_byte, colours): (usize, u16) = if format == 2 { (4, 4) } else { (2, 16) };
    let bits = 8 / per_byte;
    let texels = (0..64 / per_byte)
        .map(|b| {
            (0..per_byte).fold(0u8, |acc, i| {
                acc | ((((b * per_byte + i) % colours as usize) as u8) << (bits * i))
            })
        })
        .collect();
    NewTexture {
        name: name.into(),
        palette: format!("{name}_pl"),
        width: 8,
        height: 8,
        format,
        color0: format == 3,
        texels,
        colors: (0..colours).map(|c| c * 0x0421).collect(),
    }
}

#[test]
fn vanilla_append_keeps_every_old_entry() {
    let Some(root) = vanilla() else { return };
    let code = game_code(&root);
    let (want_map, _, _) = expected_packs(&code);
    let new = [synthetic("g4_t_a", 3), synthetic("g4_t_b", 2)];
    let built = Nsbtx::read(&nsbtx::build(&new).unwrap()).unwrap();
    let start = Instant::now();
    let (mut packs_seen, mut ok) = (0, 0);
    let mut problems = Vec::new();
    for (name, bytes) in packs(&root, "mapTextures") {
        packs_seen += 1;
        let result = (|| -> Result<Vec<String>, FormatError> {
            let old = Btx0::read(&bytes)?;
            let appended = nsbtx::append(&bytes, &new)?;
            let after = Btx0::read(&appended)?;
            let (nt, np) = (old.textures.len(), old.palettes.len());
            let mut bad = Vec::new();
            if after.textures[..nt] != old.textures[..] || after.palettes[..np] != old.palettes[..]
            {
                bad.push("old entries changed".to_string());
            }
            if !after.texels.starts_with(&old.texels)
                || !after.palette_data.starts_with(&old.palette_data)
                || after.compressed != old.compressed
                || after.compressed_index != old.compressed_index
            {
                bad.push("old blocks changed".to_string());
            }
            if after.version != old.version || after.palette_flag != old.palette_flag | 0x8000 {
                bad.push("version or palette flag".to_string());
            }
            let before = Nsbtx::read(&bytes)?;
            let decoded = Nsbtx::read(&appended)?;
            if let Some(first) = before.textures.first() {
                let pal = before
                    .palettes
                    .iter()
                    .position(|p| p.name == first.name)
                    .unwrap_or(0);
                match (before.decode(0, pal), decoded.decode(0, pal)) {
                    (Ok(a), Ok(b)) if a == b => {}
                    (Ok(_), Ok(_)) => {
                        bad.push(format!("texture 0 with palette {pal} decodes differently"))
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        bad.push(format!("texture 0 with palette {pal}: {}", kind(&e)))
                    }
                }
            }
            for k in 0..new.len() {
                if decoded.decode(nt + k, np + k)? != built.decode(k, k)? {
                    bad.push(format!("new texture {k} decodes differently"));
                }
            }
            Ok(bad)
        })();
        match result {
            Ok(bad) if bad.is_empty() => ok += 1,
            Ok(bad) => problems.extend(bad.into_iter().map(|b| format!("{name}: {b}"))),
            Err(e) => problems.push(format!("{name}: {}", kind(&e))),
        }
    }
    let ms = start.elapsed().as_millis();
    println!("nitro-append {code} packs={packs_seen} ok={ok} ms={ms}");
    for p in problems.iter().take(20) {
        println!("  {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
    assert_eq!(packs_seen, want_map);
    assert_eq!(ok, want_map);
}
