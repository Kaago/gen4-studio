#![cfg(feature = "test-support")]
mod common;

use assert_cmd::Command;
use predicates::str::contains;

/// `g4k` in English whatever `LANG` the test runs under (`g4k` picks its locale from
/// `G4_LANG`, then `LANG`): the tests assert the English messages.
fn g4k() -> Command {
    let mut cmd = Command::cargo_bin("g4k").unwrap();
    cmd.env("G4_LANG", "en");
    cmd
}

#[test]
fn info_prints_identity() {
    let s = common::synthetic_project();
    g4k()
        .args(["info"])
        .arg(s.dir.path())
        .assert()
        .success()
        .stdout(contains("game_code=CPUE"))
        .stdout(contains("family=Pt"))
        .stdout(contains("headers=2"));
}

#[test]
fn headers_lists_id_location_name_internal_name_and_matrix() {
    let s = common::synthetic_project();
    let out = g4k()
        .args(["headers"])
        .arg(s.dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    let fields: Vec<_> = lines[0].split("  ").collect();
    assert_eq!(fields, ["0000", "Twinleaf Town", "T01", "M0"], "{text}");
}

#[test]
fn headers_reports_unreadable_names_on_stderr_and_still_lists() {
    let s = common::synthetic_project();
    std::fs::write(
        s.dir.path().join("files/msgdata/pl_msg.narc"),
        g4_formats::narc::Narc {
            members: vec![vec![0xAA; 8]],
        }
        .write(),
    )
    .unwrap();
    g4k()
        .args(["headers"])
        .arg(s.dir.path())
        .assert()
        .success()
        .stdout(contains("0000    T01  M0"))
        .stderr(contains("location names unreadable"));
}

#[test]
fn set_perm_then_dump_shows_change() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    g4k()
        .args(["map", "set-perm"])
        .arg(root)
        .args(["1", "3", "4", "--kind", "2", "--collision", "128"])
        .assert()
        .success()
        .stdout(contains("saved 1 file(s)"));
    let out = g4k()
        .args(["map", "dump"])
        .arg(root)
        .args(["1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["index"], 1);
    assert_eq!(v["permissions"].as_array().unwrap().len(), 1024);
    assert_eq!(v["permissions"][4 * 32 + 3], serde_json::json!([2, 128]));
    assert!(v["buildings"].is_array());
}

#[test]
fn set_perm_out_of_range_fails_cleanly() {
    let s = common::synthetic_project();
    g4k()
        .args(["map", "set-perm"])
        .arg(s.dir.path())
        .args(["1", "32", "0", "--kind", "1", "--collision", "0"])
        .assert()
        .code(1)
        .stderr(contains("out of range"));
}

#[test]
fn info_on_non_project_fails_with_message() {
    let d = tempfile::tempdir().unwrap();
    g4k()
        .args(["info"])
        .arg(d.path())
        .assert()
        .failure()
        .stderr(contains("not a ds-rom project"));
}

#[test]
fn build_on_synthetic_reports_build_error_exit_1() {
    let s = common::synthetic_project();
    let out = s.dir.path().join("out.nds");
    g4k()
        .args(["build"])
        .arg(s.dir.path())
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(contains("build:"));
}

#[test]
fn build_no_cache_removes_the_state_first() {
    let s = common::synthetic_project();
    let out = s.dir.path().join("out.nds");
    let marker = s.dir.path().join(".g4/build/blz/marker");
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(&marker, b"x").unwrap();
    // The synthetic project cannot be built into a ROM, but the cache goes before that.
    g4k()
        .args(["build"])
        .arg(s.dir.path())
        .arg("--out")
        .arg(&out)
        .arg("--no-cache")
        .assert()
        .code(1)
        .stderr(contains("build:"));
    assert!(!marker.exists());
}

#[test]
fn build_timings_prints_one_json_line() {
    // The synthetic project does not build up to the ROM: gated on a copy.
    let Some((tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let out = tmp.path().join("out.nds");
    let assert = g4k()
        .args(["build"])
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .arg("--timings")
        .assert()
        .success();
    let text = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    println!("{text}");
    assert_eq!(text.lines().count(), 1, "{text}");
    let json: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert!(json["ms"].is_u64(), "{json}");
    assert_eq!(json["rom"], out.display().to_string());
    let steps = json["steps"].as_array().unwrap();
    let ids: Vec<&str> = steps.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"rom"), "{ids:?}");
    assert!(ids.contains(&"chatot"), "{ids:?}");
    let rom = steps.iter().find(|s| s["id"] == "rom").unwrap();
    assert_eq!(rom["status"], "full", "{json}");
    assert!(out.exists());

    // Nothing changed: the second build skips the ROM step.
    let assert = g4k()
        .args(["build"])
        .arg(&root)
        .arg("--out")
        .arg(&out)
        .arg("--timings")
        .assert()
        .success();
    let text = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    let rom = json["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "rom")
        .unwrap()
        .clone();
    assert_eq!(rom["status"], "skipped", "{json}");
}

#[test]
fn vanilla_info_prints_identity() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let out = g4k().args(["info"]).arg(&root).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    println!("{text}");
    assert!(text.starts_with("game_code="));
    assert!(text.contains(" family="));
    assert!(text.contains(" language="));
    assert!(text.contains(" headers="));
    assert!(!text.contains("headers=?"));
    assert_eq!(text.lines().count(), 1);
}

/// personalPokeData/0397 (Staravia), Platin DE: attack 75.
const STARAVIA: [u8; 44] = [
    0x37, 0x4B, 0x32, 0x50, 0x28, 0x28, 0x00, 0x02, 0x78, 0x71, 0x80, 0x00, 0x00, 0x00, 0xBC, 0x00,
    0x7F, 0x0F, 0x46, 0x03, 0x04, 0x04, 0x16, 0x00, 0x3C, 0x05, 0x00, 0x00, 0x20, 0x06, 0x13, 0x84,
    0x80, 0x7E, 0x04, 0x02, 0x00, 0x20, 0xC6, 0x23, 0x01, 0x00, 0x00, 0x00,
];

/// moveData/0017 (Wing Attack), Platin DE: power 60.
const WING_ATTACK: [u8; 16] = [
    0x00, 0x00, 0x00, 0x3C, 0x02, 0x64, 0x23, 0x00, 0x00, 0x00, 0x00, 0x73, 0x05, 0x00, 0x00, 0x00,
];

/// itemData/0017 (Potion), Platin DE: price 300.
const POTION: [u8; 34] = [
    0x2C, 0x01, 0x00, 0x14, 0x00, 0x00, 0x1E, 0x00, 0x9F, 0x20, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,
];

/// learnsets/0397 (Staravia), Platin DE: twelve entries.
const STARAVIA_LEARNSET: [u8; 28] = [
    0x21, 0x02, 0x2D, 0x02, 0x62, 0x02, 0x62, 0x0A, 0x11, 0x12, 0x68, 0x1A, 0x1B, 0x25, 0x12, 0x2E,
    0x4C, 0x39, 0x24, 0x42, 0x61, 0x4C, 0x9D, 0x57, 0xFF, 0xFF, 0x00, 0x00,
];

fn narc(members: &[&[u8]]) -> Vec<u8> {
    g4_formats::narc::Narc {
        members: members.iter().map(|m| m.to_vec()).collect(),
    }
    .write()
}

/// A minimal `.nds`: a zeroed header with `game_code`, the FNT ds-rom builds for `files`
/// (paths below the ROM's file system root), a FAT, then the files. No ARM9 or ARM7:
/// `check-data` reads only the header and the file system.
fn rom_with_files(game_code: &str, files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    use ds_rom::rom::FileSystem;
    use ds_rom::rom::raw::{Header, TableOffset};

    let dir = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let fs = FileSystem::load(dir.path(), 0).unwrap();
    let fnt = fs.build_fnt().unwrap().build().unwrap();
    let count = usize::from(fs.max_file_id()) + 1;
    let fnt_at = 0x4000;
    let fat_at = (fnt_at + fnt.len()).next_multiple_of(4);
    let mut rom = vec![0u8; fat_at + 8 * count];
    rom[fnt_at..fnt_at + fnt.len()].copy_from_slice(&fnt);
    for id in 0..count {
        rom.resize(rom.len().next_multiple_of(4), 0);
        let start = rom.len() as u32;
        rom.extend_from_slice(fs.file(id as u16).contents());
        let end = rom.len() as u32;
        let entry = fat_at + 8 * id;
        rom[entry..entry + 4].copy_from_slice(&start.to_le_bytes());
        rom[entry + 4..entry + 8].copy_from_slice(&end.to_le_bytes());
    }
    let header = Header::borrow_from_slice_mut(&mut rom).unwrap();
    header.gamecode = ds_rom::str::AsciiArray(game_code.as_bytes().try_into().unwrap());
    header.file_names = TableOffset {
        offset: fnt_at as u32,
        size: fnt.len() as u32,
    };
    header.file_allocs = TableOffset {
        offset: fat_at as u32,
        size: (8 * count) as u32,
    };
    rom
}

/// A Platinum ROM whose data archives hold the vanilla members above at index 1 (index 0
/// is a copy, so member 1 is not the first file of its archive).
fn data_rom() -> (tempfile::TempDir, std::path::PathBuf) {
    let rom = rom_with_files(
        "CPUE",
        &[
            (
                "poketool/personal/pl_personal.narc",
                narc(&[&STARAVIA, &STARAVIA]),
            ),
            (
                "poketool/personal/wotbl.narc",
                narc(&[&STARAVIA_LEARNSET, &STARAVIA_LEARNSET]),
            ),
            (
                "poketool/waza/pl_waza_tbl.narc",
                narc(&[&WING_ATTACK, &WING_ATTACK]),
            ),
            (
                "itemtool/itemdata/pl_item_data.narc",
                narc(&[&POTION, &POTION]),
            ),
        ],
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.nds");
    std::fs::write(&path, rom).unwrap();
    (dir, path)
}

#[test]
fn check_data_reads_the_built_narcs_and_fails_on_a_wrong_expectation() {
    let (_dir, rom) = data_rom();
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "species:1:attack=85"])
        .assert()
        .code(1)
        .stderr(contains("species 1 attack: expected 85, found 75"));
}

#[test]
fn check_data_passes_when_every_expectation_holds() {
    let (_dir, rom) = data_rom();
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "species:1:attack=75"])
        .args(["--expect", "move:1:power=60"])
        .args(["--expect", "item:1:price=300"])
        .args(["--expect", "learnset:1:len=12"])
        .assert()
        .success()
        .stdout(contains("species 1 attack: 75"))
        .stdout(contains("learnset 1 len: 12"))
        .stdout(contains("4 expectation(s) met"));
}

#[test]
fn check_data_reports_every_mismatch_and_a_missing_member() {
    let (_dir, rom) = data_rom();
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "move:1:power=65"])
        .args(["--expect", "item:1:price=300"])
        .args(["--expect", "learnset:2:len=13"])
        .assert()
        .code(1)
        .stdout(contains("item 1 price: 300"))
        .stderr(contains("move 1 power: expected 65, found 60"))
        .stderr(contains("learnset 2: the archive has 2 members"))
        .stderr(contains("2 of 3 expectation(s) failed"));
}

#[test]
fn check_data_rejects_an_unknown_field_before_reading() {
    g4k()
        .args([
            "check-data",
            "does-not-exist.nds",
            "--expect",
            "species:1:power=1",
        ])
        .assert()
        .code(2)
        .stderr(contains("species has no field power"));
}

/// A HeartGold US ROM with the egg move NARC `a/2/2/9` of the HGSS fixture (one member:
/// species 1 with moves 5 and 7, species 3 without moves, species 4 with move 9).
fn egg_move_rom() -> (tempfile::TempDir, std::path::PathBuf) {
    use g4_kernel::test_support::{EGG_MOVE_WORDS, egg_move_bytes};
    let rom = rom_with_files(
        "IPKE",
        &[("a/2/2/9", narc(&[&egg_move_bytes(&EGG_MOVE_WORDS)]))],
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.nds");
    std::fs::write(&path, rom).unwrap();
    (dir, path)
}

#[test]
fn check_data_reads_egg_moves_of_a_species() {
    let (_dir, rom) = egg_move_rom();
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "egg_moves:1:count=2"])
        .args(["--expect", "egg_moves:1:move0=5"])
        .args(["--expect", "egg_moves:1:move1=7"])
        .args(["--expect", "egg_moves:3:count=0"])
        .args(["--expect", "egg_moves:2:count=0"])
        .args(["--expect", "egg_moves:4:move0=9"])
        .assert()
        .success()
        .stdout(contains("egg_moves 1 count: 2"))
        .stdout(contains("egg_moves 1 move1: 7"))
        .stdout(contains("6 expectation(s) met"));
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "egg_moves:1:count=3"])
        .args(["--expect", "egg_moves:1:move2=1"])
        .assert()
        .code(1)
        .stderr(contains("egg_moves 1 count: expected 3, found 2"))
        .stderr(contains("egg_moves 1: species 1 has 2 egg moves"))
        .stderr(contains("2 of 2 expectation(s) failed"));
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "egg_moves:1:move16=1"])
        .assert()
        .code(2)
        .stderr(contains("egg_moves has no field move16"));
}

#[test]
fn check_data_egg_moves_of_platinum_without_the_patch_has_no_archive() {
    let (_dir, rom) = data_rom();
    g4k()
        .args(["check-data"])
        .arg(&rom)
        .args(["--expect", "egg_moves:1:count=2"])
        .assert()
        .code(1)
        .stderr(contains("CPUE has no egg_moves archive"));
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn check_data_compares_the_model_hash_in_the_rom() {
    let maps = [
        g4_kernel::test_support::synthetic_map_bytes(1),
        g4_kernel::test_support::synthetic_map_bytes(7),
    ];
    let rom = rom_with_files(
        "CPUE",
        &[(
            "fielddata/land_data/land_data.narc",
            narc(&[&maps[0], &maps[1]]),
        )],
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.nds");
    std::fs::write(&path, rom).unwrap();
    let model = sha1_hex(&[7u8; 6]);
    let per = sha1_hex(&maps[1][16..16 + 2048]);
    g4k()
        .args(["check-data"])
        .arg(&path)
        .args([
            "--expect",
            &format!("map:1:model_sha1={}", model.to_uppercase()),
        ])
        .args(["--expect", &format!("map:1:per_sha1={per}")])
        .assert()
        .success()
        .stdout(contains(format!("map 1 model_sha1: {model}")))
        .stdout(contains("2 expectation(s) met"));
    let other = sha1_hex(&[1u8; 6]);
    g4k()
        .args(["check-data"])
        .arg(&path)
        .args(["--expect", &format!("map:1:model_sha1={other}")])
        .assert()
        .code(1)
        .stderr(contains(format!(
            "map 1 model_sha1: expected {other}, found {model}"
        )));
}

#[test]
fn check_data_reads_the_sha1_of_a_map_texture() {
    // Two synthetic members (no NSBTX needed: the digest is of the whole member's bytes).
    let packs: [Vec<u8>; 2] = [b"BTX0 pack zero".to_vec(), vec![0xA5; 300]];
    let rom = rom_with_files(
        "CPUE",
        &[(
            "fielddata/areadata/area_map_tex/map_tex_set.narc",
            narc(&[&packs[0], &packs[1]]),
        )],
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.nds");
    std::fs::write(&path, rom).unwrap();
    let sha1 = sha1_hex(&packs[0]);
    g4k()
        .args(["check-data"])
        .arg(&path)
        .args(["--expect", &format!("map_texture:0:sha1={sha1}")])
        .args([
            "--expect",
            &format!("map_texture:1:sha1={}", sha1_hex(&packs[1]).to_uppercase()),
        ])
        .assert()
        .success()
        .stdout(contains(format!("map_texture 0 sha1: {sha1}")))
        .stdout(contains("2 expectation(s) met"));
    let other = sha1_hex(&packs[1]);
    g4k()
        .args(["check-data"])
        .arg(&path)
        .args(["--expect", &format!("map_texture:0:sha1={other}")])
        .args(["--expect", &format!("map_texture:2:sha1={other}")])
        .assert()
        .code(1)
        .stderr(contains(format!(
            "map_texture 0 sha1: expected {other}, found {sha1}"
        )))
        .stderr(contains("map_texture 2: the archive has 2 members"))
        .stderr(contains("2 of 2 expectation(s) failed"));
    g4k()
        .args(["check-data"])
        .arg(&path)
        .args(["--expect", "map_texture:0:size=1"])
        .assert()
        .code(2)
        .stderr(contains("map_texture has no field size (fields: sha1)"));
}

// The synthetic project's findings that are no error: `header.text-split` once (script file 0
// runs with text archives 0 and 433) and `encounter.empty-slot` twice (encounter files 0, 1).

#[test]
fn g4k_check_expect_fails_with_code_soll_ist() {
    let s = common::synthetic_project();
    g4k()
        .args(["check"])
        .arg(s.dir.path())
        .args(["--expect", "header.text-split=1"])
        .args(["--expect", "encounter.empty-slot=1"])
        .args(["--expect", "warp.dest-warp=0"])
        .assert()
        .code(1)
        .stdout(contains("header.text-split: 1"))
        .stdout(contains("warp.dest-warp: 0"))
        .stderr(contains("encounter.empty-slot: expected 1, found 2"))
        .stderr(contains("1 of 3 expectation(s) failed"));
    g4k()
        .args(["check"])
        .arg(s.dir.path())
        .args(["--expect", "header.text-split=1"])
        .args(["--expect", "encounter.empty-slot=2"])
        .assert()
        .success()
        .stdout(contains("2 expectation(s) met"));
}

#[test]
fn g4k_check_prints_the_report_as_json_and_honours_the_ignore_file() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let out = g4k()
        .args(["check"])
        .arg(root)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let findings = report["findings"].as_array().unwrap();
    let split: Vec<_> = findings
        .iter()
        .filter(|f| f["code"] == "header.text-split")
        .collect();
    assert_eq!(split.len(), 1, "{report}");
    let split = split[0];
    assert_eq!(split["severity"], "warning");
    assert_eq!(
        split["subject"],
        serde_json::json!({ "kind": "scriptFile", "index": 0 })
    );
    assert_eq!(split["key"], "text_archive");
    // A u64 does not survive a JavaScript number: 16 hex digits.
    let fingerprint = split["fingerprint"].as_str().unwrap();
    assert_eq!(fingerprint.len(), 16, "{fingerprint}");
    assert!(fingerprint.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(report["ignored"], serde_json::json!([]));
    assert!(
        report["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["code"] == "script.message-missing" && s["why"].is_string()),
        "{report}"
    );

    // Like the app, `g4k check` leaves out what g4-checks.json ignores or switches off.
    std::fs::write(
        root.join("g4-checks.json"),
        serde_json::json!({
            "ignored": [{
                "code": "header.text-split",
                "subject": split["subject"],
                "key": "text_archive",
                "fingerprint": fingerprint,
            }],
            "disabled": ["encounter.empty-slot"],
        })
        .to_string(),
    )
    .unwrap();
    g4k()
        .args(["check"])
        .arg(root)
        .args(["--expect", "header.text-split=0"])
        .args(["--expect", "encounter.empty-slot=0"])
        .assert()
        .success()
        .stdout(contains("1 ignored"))
        .stdout(contains("skipped encounter.empty-slot: disabled"));
    let out = g4k()
        .args(["check"])
        .arg(root)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(report["ignored"][0]["code"], "header.text-split");
}

#[test]
fn g4k_check_rejects_an_unknown_code_and_a_broken_ignore_file() {
    let s = common::synthetic_project();
    g4k()
        .args(["check"])
        .arg(s.dir.path())
        .args(["--expect", "warp.no-such-check=0"])
        .assert()
        .code(2)
        .stderr(contains("unknown check code warp.no-such-check"));
    std::fs::write(s.dir.path().join("g4-checks.json"), "{ not json").unwrap();
    g4k()
        .args(["check"])
        .arg(s.dir.path())
        .assert()
        .code(1)
        .stderr(contains("g4-checks.json"));
}

#[test]
fn inspect_rom_prints_json() {
    let (_tmp, rom) = common::synthetic_rom();
    g4k()
        .args(["inspect-rom"])
        .arg(&rom)
        .assert()
        .success()
        .stdout(contains("\"game_code\": \"CPUE\""));
}

#[test]
fn create_fails_on_an_unextractable_rom_and_leaves_nothing() {
    let (tmp, rom) = common::synthetic_rom();
    let dest = tmp.path().join("neu");
    g4k().args(["create"]).arg(&rom).arg(&dest).assert().code(1);
    assert!(!dest.exists());
    assert!(!tmp.path().join(".neu.g4-new").exists());
}

#[test]
fn template_without_a_project_fails_with_message() {
    let d = tempfile::tempdir().unwrap();
    g4k()
        .args(["template"])
        .arg(d.path())
        .arg("empty-region")
        .assert()
        .code(1)
        .stderr(contains("not a ds-rom project"));
}

#[test]
fn template_rejects_an_unknown_template() {
    let s = common::synthetic_project();
    g4k()
        .args(["template"])
        .arg(s.dir.path())
        .arg("volcano")
        .assert()
        .failure()
        .stderr(contains("unknown template"));
}

#[test]
fn template_refuses_before_adopting_the_scripts() {
    use std::sync::Arc;

    use g4_kernel::project::Project;
    use g4_kernel::store::Store;

    let s = common::synthetic_project();
    let root = s.dir.path();
    let untouched = || {
        assert!(!root.join("rotom.toml").exists());
        assert!(!root.join("expanded/scripts").exists());
    };
    // Matrix 0 of the fixture names no headers: the start town T01 has no cell.
    g4k()
        .args(["template"])
        .arg(root)
        .arg("empty-region")
        .assert()
        .code(1)
        .stderr(contains("start town T01 has no cell in matrix 0"));
    untouched();
    // Both cells carry the town and it has no figures: the world is already empty.
    let mut store = Store::new(Arc::new(Project::open(root).unwrap()));
    store
        .edit_matrix(0, "town", |m| {
            m.headers = Some(vec![0, 0]);
            Ok(())
        })
        .unwrap();
    store.save().unwrap();
    g4k()
        .args(["template"])
        .arg(root)
        .arg("empty-region")
        .assert()
        .code(1)
        .stderr(contains("the world is already empty"));
    untouched();
}

#[test]
fn create_accepts_the_empty_region_template() {
    let (tmp, rom) = common::synthetic_rom();
    let dest = tmp.path().join("neu");
    // The template is parsed; the synthetic ROM then fails to extract (exit 1, not a usage error).
    g4k()
        .args(["create"])
        .arg(&rom)
        .arg(&dest)
        .args(["--template", "empty-region"])
        .assert()
        .code(1);
    assert!(!dest.exists());
}

/// `<tmp>/X_DSPRE_contents` (a copy of the synthetic project) and `<tmp>/X.nds` (the synthetic
/// ROM, game code CPUE like the project): the layout DSPRE leaves behind.
fn dspre_layout() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let s = common::synthetic_project();
    let (_rom_dir, rom) = common::synthetic_rom();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("X_DSPRE_contents");
    common::copy_dir(s.dir.path(), &root);
    let nds = tmp.path().join("X.nds");
    std::fs::copy(&rom, &nds).unwrap();
    (tmp, root, nds)
}

fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[test]
fn cli_base_assign_writes_a_dspre_mark() {
    let (_tmp, root, nds) = dspre_layout();
    let assert = g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .success();
    let text = stdout_of(&assert);
    println!("{text}");
    let mark = g4_kernel::start::read_base_mark(&root).expect("a mark");
    assert_eq!(mark.origin, "dspre");
    assert_eq!(mark.version, 2);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "warning: Unknown or modified ROM; patches apply only to exactly this file.",
            format!(
                "assigned sha1={} label={}",
                mark.sha1,
                g4_kernel::base::base_label(&mark)
            )
            .as_str(),
        ],
        "{text}"
    );

    // Now `base check` names the mark and finds the sibling.
    let assert = g4k()
        .args(["base", "check"])
        .arg(&root)
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .success();
    let text = stdout_of(&assert);
    println!("{text}");
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "mark=dspre".to_owned(),
            format!("sha1={}", mark.sha1),
            format!("base={} (sibling)", nds.display()),
        ],
        "{text}"
    );
}

#[test]
fn cli_base_assign_refuses_another_game_with_exit_1() {
    let (_tmp, root, nds) = dspre_layout();
    common::with_game_code(&nds, "IPKE");
    g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .code(1)
        .stderr(contains("Wrong game"));
    assert!(!root.join(".g4").exists());
}

#[test]
fn cli_base_check_names_the_suggestion() {
    let (_tmp, root, nds) = dspre_layout();
    let assert = g4k()
        .args(["base", "check"])
        .arg(&root)
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .success();
    let text = stdout_of(&assert);
    println!("{text}");
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "mark=none".to_owned(),
            format!("suggestion={}", nds.display())
        ],
        "{text}"
    );
    assert!(!root.join(".g4").exists());
}

/// Gate 4.3a: `g4k text export .` from inside the project did not find the sibling `.nds`,
/// because `.` has no folder name; the same for `base check .`.
#[test]
fn cli_base_check_finds_the_suggestion_from_inside_the_project() {
    let (_tmp, root, nds) = dspre_layout();
    let assert = g4k()
        .args(["base", "check", "."])
        .current_dir(&root)
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .success();
    let text = stdout_of(&assert);
    println!("{text}");
    let suggestion = text
        .lines()
        .find_map(|l| l.strip_prefix("suggestion="))
        .unwrap_or_else(|| panic!("no suggestion: {text}"));
    assert_eq!(
        std::fs::canonicalize(suggestion).unwrap(),
        std::fs::canonicalize(&nds).unwrap(),
        "{text}"
    );
    // From a subfolder with `..` as well.
    std::fs::create_dir(root.join("sub")).unwrap();
    let assert = g4k()
        .args(["base", "check", ".."])
        .current_dir(root.join("sub"))
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .success();
    let text = stdout_of(&assert);
    assert!(text.contains("suggestion="), "{text}");
}

#[test]
fn cli_base_check_with_rom_prints_the_warnings_as_json() {
    let (_tmp, root, nds) = dspre_layout();
    let out = g4k()
        .args(["base", "check"])
        .arg(&root)
        .arg("--rom")
        .arg(&nds)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let check: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(check["refused"], serde_json::Value::Null, "{check}");
    assert_eq!(check["info"]["game_code"], "CPUE", "{check}");
    assert_eq!(
        check["warnings"],
        serde_json::json!([{
            "kind": "unknown_dump",
            "message": "Unknown or modified ROM; patches apply only to exactly this file.",
        }]),
        "{check}"
    );
    // Without --json: one line per warning; nothing written either way.
    g4k()
        .args(["base", "check"])
        .arg(&root)
        .arg("--rom")
        .arg(&nds)
        .assert()
        .success()
        .stdout(contains(
            "warning: Unknown or modified ROM; patches apply only to exactly this file.",
        ));
    // Another game: exit 1 with the reason, as JSON too.
    common::with_game_code(&nds, "IPKE");
    let assert = g4k()
        .args(["base", "check"])
        .arg(&root)
        .arg("--rom")
        .arg(&nds)
        .arg("--json")
        .assert()
        .code(1)
        .stderr(contains("Wrong game"));
    let check: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(check["warnings"], serde_json::json!([]), "{check}");
    assert!(check["refused"].as_str().unwrap().starts_with("Wrong game"));
    assert!(!root.join(".g4").exists());
}

#[test]
fn cli_base_check_hints_at_a_base_dir_that_is_not_a_folder() {
    let (tmp, root, nds) = dspre_layout();
    g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .success();
    std::fs::remove_file(&nds).unwrap();
    let typo = tmp.path().join("romz");
    g4k()
        .args(["base", "check"])
        .arg(&root)
        .env("G4_BASE_ROM_DIR", &typo)
        .assert()
        .success()
        .stdout(contains("mark=dspre"))
        .stdout(contains("base=missing"))
        .stderr(contains(format!(
            "hint: G4_BASE_ROM_DIR is not a folder: {}",
            typo.display()
        )));
}

#[test]
fn cli_patch_create_without_a_mark_fails() {
    let (_tmp, root, _nds) = dspre_layout();
    g4k()
        .args(["patch", "create"])
        .arg(&root)
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .code(1)
        .stderr(contains(format!(
            "{} Assign it with `g4k base assign <project> <nds>`.",
            g4_kernel::patch::no_mark()
        )));
    assert!(!root.join("build").exists());
}

#[test]
fn cli_patch_create_refuses_a_broken_or_newer_mark_in_the_app_words() {
    let (_tmp, root, _nds) = dspre_layout();
    std::fs::create_dir_all(root.join(".g4")).unwrap();
    for mark in ["{ broken", r#"{"version": 3}"#] {
        std::fs::write(root.join(".g4/base.json"), mark).unwrap();
        let refusal = g4_kernel::start::mark_state(&root)
            .refusal()
            .expect("a refusal");
        g4k()
            .args(["patch", "create"])
            .arg(&root)
            .env_remove("G4_BASE_ROM_DIR")
            .assert()
            .code(1)
            .stderr(contains(refusal));
        assert!(!root.join("build").exists());
    }
}

#[test]
fn cli_patch_create_with_a_missing_base_names_it() {
    let (_tmp, root, nds) = dspre_layout();
    g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .success();
    // No sibling any more, and an empty search folder.
    std::fs::remove_file(&nds).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let mark = g4_kernel::start::read_base_mark(&root).unwrap();
    let missing = g4_kernel::base::missing_text(&mark);
    let assert = g4k()
        .args(["patch", "create"])
        .arg(&root)
        .env("G4_BASE_ROM_DIR", empty.path())
        .assert()
        .code(1)
        .stderr(contains(missing.as_str()));
    println!("{}", String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(!root.join("build").exists());
}

#[test]
fn cli_publish_prints_the_files_and_the_count() {
    use g4_kernel::patch::{
        RELEASES_FILE, bad_file, bad_version, exists, no_changes, release_file_name,
    };
    use g4_kernel::project::{AssetKind, Project};

    // The synthetic project does not build to a ROM: here only the refusals before the build.
    let (_tmp, root, nds) = dspre_layout();
    g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .success();
    let publish = |root: &std::path::Path, nds: &std::path::Path, args: &[&str]| {
        let mut command = g4k();
        command
            .arg("publish")
            .arg(root)
            .args(args)
            .arg("--base-rom")
            .arg(nds);
        command
    };
    let build = root.join("build");
    std::fs::create_dir_all(&build).unwrap();
    let old = build.join("X-v0.1.0.xdelta");
    std::fs::write(&old, b"old patch").unwrap();
    publish(
        &root,
        &nds,
        &["--version", "0.1.0", "--notes", "Erste Fassung", "--readme"],
    )
    .assert()
    .code(1)
    .stderr(contains(format!("X-v0.1.0.xdelta {}", exists())));
    publish(&root, &nds, &["--version", "1 0"])
        .assert()
        .code(1)
        .stderr(contains(bad_version()));
    publish(&root, &nds, &["--version", "0.1.0", "--file", "a/b"])
        .assert()
        .code(1)
        .stderr(contains(bad_file()));
    publish(
        &root,
        &nds,
        &["--version", "0.1.0", "--notes", "a", "--notes-file", "b"],
    )
    .assert()
    .code(2);
    assert_eq!(std::fs::read(&old).unwrap(), b"old patch");
    assert!(!root.join(RELEASES_FILE).exists());
    assert_eq!(std::fs::read_dir(&build).unwrap().count(), 1);

    // The whole run: gated on a copy of the vanilla project.
    let Some((_copy_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    g4k()
        .args(["base", "assign"])
        .arg(&copy)
        .arg(&nds)
        .assert()
        .success();
    // Unchanged, the build is the base ROM: nothing to publish, nothing written.
    publish(&copy, &nds, &["--version", "0.1.0"])
        .assert()
        .code(1)
        .stderr(contains(no_changes()));
    assert!(!copy.join(RELEASES_FILE).exists());
    // One permission byte of map 3.
    let project = Project::open(&copy).unwrap();
    project.ensure_unpacked(AssetKind::Map).unwrap();
    let map = project.member_path(AssetKind::Map, 3);
    let mut bytes = std::fs::read(&map).unwrap();
    bytes[0x20] ^= 0x80;
    std::fs::write(&map, bytes).unwrap();
    let assert = publish(
        &copy,
        &nds,
        &["--version", "0.1.0", "--notes", "Erste Fassung", "--readme"],
    )
    .assert()
    .success();
    let out = stdout_of(&assert);
    println!("{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with("published version=0.1.0 patch="),
        "{out}"
    );
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("file ")).count(),
        2,
        "{out}"
    );
    assert!(lines[1].starts_with("file notes "), "{out}");
    assert!(lines[2].starts_with("file readme "), "{out}");
    assert_eq!(lines.last(), Some(&"releases=1"), "{out}");

    // Another version onto the same file: refused without --overwrite.
    let first = release_file_name(&Project::open(&copy).unwrap(), "0.1.0");
    publish(&copy, &nds, &["--version", "0.1.1", "--file", &first])
        .assert()
        .code(1)
        .stderr(contains(format!("{first} {}", exists())));
    let releases = g4_kernel::patch::read_releases(&copy).unwrap();
    assert_eq!(releases.releases.len(), 1);
}

/// Unpacks every archive of the project the way the base layer has them.
fn unpack_all(root: &std::path::Path) {
    use g4_kernel::project::{AssetKind, Project};
    let p = Project::open(root).unwrap();
    let kinds: Vec<AssetKind> = p.archives.kinds().collect();
    for kind in kinds {
        if kind == AssetKind::DynamicHeaders {
            continue;
        }
        if let Some(packed) = p.archives.packed(kind)
            && root.join(packed).is_file()
        {
            p.ensure_unpacked(kind).unwrap();
        }
    }
}

/// [`dspre_layout`] with the sibling `.nds` assigned as the base and the base layer prepared:
/// the synthetic ROM cannot be extracted, so the layer is a copy of the project as it is
/// (without `.g4`), its `.complete` carrying the mark's SHA-1, like `tests/textproject.rs`.
fn text_layout() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let (tmp, root, nds) = dspre_layout();
    g4k()
        .args(["base", "assign"])
        .arg(&root)
        .arg(&nds)
        .assert()
        .success();
    unpack_all(&root);
    let sha1 = g4_kernel::start::read_base_mark(&root).unwrap().sha1;
    let copy = tempfile::tempdir().unwrap();
    let copy = copy.path().join("layer");
    common::copy_dir(&root, &copy);
    std::fs::remove_dir_all(copy.join(".g4")).unwrap();
    let layer = root.join(".g4/cache/base-layer").join(&sha1);
    common::copy_dir(&copy, &layer);
    std::fs::write(layer.join(".complete"), &sha1).unwrap();
    (tmp, root, nds)
}

/// `g4k text <args> <root>` with no base folder in the environment.
fn text(args: &[&str], root: &std::path::Path) -> Command {
    let mut cmd = g4k();
    cmd.arg("text")
        .args(args)
        .arg(root)
        .env_remove("G4_BASE_ROM_DIR");
    cmd
}

/// The export's summary line (the one after the art lines).
fn summary_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|l| l.starts_with("raw="))
        .unwrap_or_else(|| panic!("no summary line:\n{stdout}"))
}

#[test]
fn cli_text_export_then_check_is_clean() {
    let (_tmp, root, _nds) = text_layout();
    // The base ROM is the sibling `.nds`; the scripts are adopted on the way.
    let assert = text(&["export"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("adopted scripts=") && l.len() > "adopted scripts=".len()),
        "{out}"
    );
    let moves = out
        .lines()
        .find(|l| l.starts_with("art=moves "))
        .unwrap_or_else(|| panic!("no moves line:\n{out}"));
    for key in ["entities=", "json=", "side=", "written=", "removed=", "ms="] {
        assert!(moves.contains(&format!(" {key}")), "{key} in {moves}");
    }
    let summary = summary_line(&out);
    for key in [
        "raw=",
        " texts=",
        " scripts=",
        " base_layer_ms=",
        " created=false",
        " files=",
        " bytes=",
        " written=",
        " removed=",
        " ms=",
    ] {
        assert!(summary.contains(key), "{key} in {summary}");
    }
    assert!(root.join("g4-project.json").is_file());
    assert!(root.join("data/moves/0001.json").is_file());

    let assert = text(&["check"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.first(), Some(&"schema=current"), "{out}");
    assert_eq!(lines.last(), Some(&"differs=0"), "{out}");

    // As JSON: the whole report.
    let assert = text(&["check", "--json"], &root).assert().success();
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(report["schema"]["state"], "current", "{report}");
    assert_eq!(report["differs"], serde_json::json!([]), "{report}");

    // An edit shows up as a file the import would write.
    let path = root.join("data/moves/0002.json");
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"power\": 20"), "{json}");
    std::fs::write(&path, json.replacen("\"power\": 20", "\"power\": 99", 1)).unwrap();
    let assert = text(&["check"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let lines: Vec<_> = out.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with("differs="))
        .unwrap();
    assert_eq!(lines[at], "differs=1", "{out}");
    assert!(lines[at + 1].contains("moveData"), "{out}");
}

#[test]
fn cli_text_export_twice_writes_nothing() {
    let (_tmp, root, sibling) = text_layout();
    // The base ROM is no sibling any more and no search folder is set: only `--base-rom`
    // finds it.
    let elsewhere = tempfile::tempdir().unwrap();
    let nds = elsewhere.path().join("base.nds");
    std::fs::rename(&sibling, &nds).unwrap();
    let run = || {
        let assert = g4k()
            .args(["text", "export"])
            .arg(&root)
            .arg("--base-rom")
            .arg(&nds)
            .env_remove("G4_BASE_ROM_DIR")
            .assert()
            .success();
        stdout_of(&assert)
    };
    run();
    let out = run();
    println!("{out}");
    assert!(!out.contains("adopted"), "{out}");
    let summary = summary_line(&out);
    assert!(summary.contains(" written=0 removed=0 "), "{summary}");
    let arts: Vec<_> = out.lines().filter(|l| l.starts_with("art=")).collect();
    assert!(!arts.is_empty(), "{out}");
    for line in arts {
        assert!(line.contains(" written=0 removed=0 "), "{line}");
    }

    // As JSON: the export report.
    let assert = g4k()
        .args(["text", "export", "--json"])
        .arg(&root)
        .arg("--base-rom")
        .arg(&nds)
        .env_remove("G4_BASE_ROM_DIR")
        .assert()
        .success();
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(report["written"], 0, "{report}");
    assert_eq!(report["adopted"], serde_json::Value::Null, "{report}");
    assert!(report["arts"].as_array().is_some_and(|a| !a.is_empty()));
}

#[test]
fn cli_text_export_without_a_mark_fails() {
    let (_tmp, root, _nds) = dspre_layout();
    text(&["export"], &root)
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::patch::no_mark()))
        .stderr(contains("g4k base assign"));
    assert!(!root.join("data").exists());
    assert!(!root.join("g4-project.json").exists());
}

#[test]
fn cli_text_export_with_a_missing_base_names_it() {
    let (_tmp, root, nds) = text_layout();
    std::fs::remove_file(&nds).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let mark = g4_kernel::start::read_base_mark(&root).unwrap();
    g4k()
        .args(["text", "export"])
        .arg(&root)
        .env("G4_BASE_ROM_DIR", empty.path())
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::base::missing_text(&mark)));
    assert!(!root.join("data").exists());
}

#[test]
fn cli_text_import_reports_problems_with_exit_1() {
    let (_tmp, root, _nds) = text_layout();
    text(&["export"], &root).assert().success();

    // A clean import writes the members back (they equal their JSON) and says what it did.
    let assert = text(&["import"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let line = out
        .lines()
        .find(|l| l.starts_with("members="))
        .unwrap_or_else(|| panic!("no import line:\n{out}"));
    for key in [" raw=", " quirks=", " compiled=", " encoded=", " ms="] {
        assert!(line.contains(key), "{key} in {line}");
    }
    let assert = text(&["import", "--json"], &root).assert().success();
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(report["members_written"], 0, "{report}");

    // A misspelt key: file and line on stderr, exit 1.
    let path = root.join("data/moves/0001.json");
    let json = std::fs::read_to_string(&path).unwrap();
    let line = json
        .lines()
        .position(|l| l.contains("\"power\""))
        .expect("a power line")
        + 1;
    std::fs::write(&path, json.replacen("\"power\"", "\"powr\"", 1)).unwrap();
    let assert = text(&["import"], &root).assert().code(1);
    let err = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    println!("{err}");
    assert!(
        err.contains(&format!("data/moves/0001.json:{line}:")),
        "{err}"
    );
    assert!(err.contains("powr"), "{err}");
}

/// 4.3a I1: `text export` refuses JSON changes the members lack, `text import` member changes
/// the JSON lacks; `--force` goes over them.
#[test]
fn cli_text_export_refuses_unbuilt_changes_unless_forced() {
    let (_tmp, root, _nds) = text_layout();
    text(&["export"], &root).assert().success();
    let json = root.join("data/moves/0002.json");
    let edited =
        std::fs::read_to_string(&json)
            .unwrap()
            .replacen("\"power\": 20", "\"power\": 99", 1);
    std::fs::write(&json, &edited).unwrap();

    // Not built yet: the export would throw the edit away.
    text(&["export"], &root)
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::textproject::unsynced_export_prefix()))
        .stderr(contains("data/moves/0002.json"))
        .stderr(contains("build first (g4k build)"));
    assert_eq!(std::fs::read_to_string(&json).unwrap(), edited);
    text(&["export", "--force"], &root).assert().success();
    assert!(
        std::fs::read_to_string(&json)
            .unwrap()
            .contains("\"power\": 20")
    );

    // A member changed after the export: the import would throw it away.
    let member = root.join("unpacked/moveData/0002");
    let mut bytes = std::fs::read(&member).unwrap();
    bytes[3] = 77;
    std::fs::write(&member, &bytes).unwrap();
    text(&["import"], &root).assert().code(1).stderr(contains(
        "Members are newer than their JSON (unpacked/moveData/0002)",
    ));
    assert_eq!(std::fs::read(&member).unwrap()[3], 77);
    text(&["import", "--force"], &root).assert().success();
    assert_eq!(std::fs::read(&member).unwrap()[3], 20);
}

#[test]
fn cli_text_sources_lists_manifest_and_data() {
    let (_tmp, root, _nds) = text_layout();
    text(&["export"], &root).assert().success();
    let assert = text(&["sources"], &root).assert().success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    for want in [
        "g4-project.json",
        "data/moves/0001.json",
        "data/ids/maps.json",
        "data/rotom-quirks.json",
        ".g4/base.json",
    ] {
        assert!(lines.contains(&want), "missing {want}:\n{out}");
    }
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted);
    assert!(!lines.iter().any(|l| l.starts_with("unpacked/")), "{out}");
}

#[test]
fn cli_text_migrate_v0_fixture() {
    let (_tmp, root, _nds) = text_layout();
    text(&["export"], &root).assert().success();
    // The exported project brought down to the v0 form of the Task 1 fixture
    // (`tests/fixtures/textproject/v0`): the manifest says `"schema": 0`, every register is
    // `{"ids": {"NNNN": "ID"}}`. The migration must give back the exported bytes.
    let manifest = root.join("g4-project.json");
    let exported_manifest = std::fs::read_to_string(&manifest).unwrap();
    assert!(exported_manifest.contains("\"schemaVersion\": 4"));
    std::fs::write(
        &manifest,
        exported_manifest.replacen("\"schemaVersion\": 4", "\"schema\": 0", 1),
    )
    .unwrap();
    let mut registers = Vec::new();
    for entry in std::fs::read_dir(root.join("data/ids")).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        let by_id: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&bytes).unwrap();
        let ids: serde_json::Map<String, serde_json::Value> = by_id
            .iter()
            .map(|(id, n)| (format!("{:04}", n.as_u64().unwrap()), id.clone().into()))
            .collect();
        let v0 = serde_json::json!({ "ids": ids });
        std::fs::write(&path, serde_json::to_vec_pretty(&v0).unwrap()).unwrap();
        registers.push((path, bytes));
    }
    assert!(
        registers
            .iter()
            .any(|(p, _)| p.ends_with("data/ids/maps.json")),
        "{registers:?}"
    );

    let assert = text(&["check"], &root).assert().code(1);
    assert_eq!(stdout_of(&assert).lines().next(), Some("schema=older 0"));
    assert!(
        String::from_utf8_lossy(&assert.get_output().stderr).contains("g4k text migrate"),
        "the sentence names the way out"
    );

    let assert = text(&["migrate"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let changed = format!("migrated 0 -> 4 changed={} snapshot=", registers.len() + 1);
    assert!(out.starts_with(&changed), "{changed} in {out}");
    let snapshot = out.trim_end().split("snapshot=").nth(1).unwrap();
    assert!(std::path::Path::new(snapshot).is_dir(), "{snapshot}");
    assert_eq!(
        std::fs::read_to_string(&manifest).unwrap(),
        exported_manifest
    );
    for (path, bytes) in &registers {
        assert_eq!(&std::fs::read(path).unwrap(), bytes, "{}", path.display());
    }

    text(&["migrate"], &root)
        .assert()
        .success()
        .stdout("already current\n");
    let assert = text(&["migrate", "--json"], &root).assert().success();
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(
        (report["from"].as_u64(), report["to"].as_u64()),
        (Some(4), Some(4))
    );

    // Current again, and the import would write nothing.
    text(&["check"], &root)
        .assert()
        .success()
        .stdout(predicates::str::starts_with("schema=current\n"))
        .stdout(predicates::str::ends_with("differs=0\n"));

    // A newer version is refused.
    std::fs::write(
        &manifest,
        b"{\"format\": \"g4-project\", \"schemaVersion\": 99}\n",
    )
    .unwrap();
    text(&["migrate"], &root)
        .assert()
        .code(1)
        .stderr(contains("newer version"));
    let assert = text(&["check"], &root).assert().code(1);
    assert_eq!(stdout_of(&assert).lines().next(), Some("schema=newer 99"));
}

#[test]
fn cli_text_import_refuses_a_broken_or_newer_mark() {
    let (_tmp, root, _nds) = text_layout();
    text(&["export"], &root).assert().success();
    // A valid edit that an import would write.
    let path = root.join("data/moves/0002.json");
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"power\": 20"), "{json}");
    std::fs::write(&path, json.replacen("\"power\": 20", "\"power\": 99", 1)).unwrap();
    let member = std::fs::read(root.join("unpacked/moveData/0002")).unwrap();
    for mark in ["{ broken", r#"{"version": 3}"#] {
        std::fs::write(root.join(".g4/base.json"), mark).unwrap();
        let refusal = g4_kernel::start::mark_state(&root)
            .refusal()
            .expect("a refusal");
        text(&["import"], &root)
            .assert()
            .code(1)
            .stderr(contains(refusal));
        assert_eq!(
            std::fs::read(root.join("unpacked/moveData/0002")).unwrap(),
            member,
            "nothing written"
        );
    }
}

#[test]
fn cli_text_adopt_sync_and_revert() {
    let (_tmp, root, _nds) = text_layout();
    // Set up as text project, each phase timed.
    let assert = text(&["adopt"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let phases: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("phase="))
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert_eq!(
        phases,
        ["scripts", "base_layer", "entities", "sync"],
        "{out}"
    );
    let summary = out.lines().last().unwrap();
    assert!(summary.starts_with("adopted=scripts:"), "{summary}");
    assert!(summary.contains(" sync_files="), "{summary}");
    assert!(root.join("g4-project.json").is_file());
    let toml = std::fs::read_to_string(root.join("rotom.toml")).unwrap();
    assert!(!toml.contains("include_roots"), "{toml}");
    assert!(
        root.join(".rotom/command_database/.g4-ids-species.json")
            .is_file()
    );

    // Nothing changed: nothing to take over.
    let out = stdout_of(&text(&["sync"], &root).assert().success());
    assert!(out.lines().all(|l| l.starts_with("stat=")), "{out}");

    // DSPRE changes move 1: taken over into its JSON.
    let member = root.join("unpacked/moveData/0001");
    let mut bytes = std::fs::read(&member).unwrap();
    bytes[3] = 44;
    std::fs::write(&member, &bytes).unwrap();
    let out = stdout_of(&text(&["sync"], &root).assert().success());
    println!("{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("dspre data/moves/0001.json Move 0001")),
        "{out}"
    );
    let json = std::fs::read_to_string(root.join("data/moves/0001.json")).unwrap();
    assert!(json.contains("\"power\": 44"), "{json}");

    // Both sides change move 2: a conflict, exit 1, until one side is kept.
    let json_path = root.join("data/moves/0002.json");
    let json = std::fs::read_to_string(&json_path).unwrap();
    std::fs::write(
        &json_path,
        json.replacen("\"power\": 20", "\"power\": 99", 1),
    )
    .unwrap();
    let member = root.join("unpacked/moveData/0002");
    let mut bytes = std::fs::read(&member).unwrap();
    bytes[3] = 55;
    std::fs::write(&member, &bytes).unwrap();
    let assert = text(&["sync"], &root).assert().code(1);
    let out = stdout_of(&assert);
    println!("{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("conflict data/moves/0002.json Move 0002")),
        "{out}"
    );
    // Pending: the revert is refused.
    text(&["revert"], &root)
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::textproject::adopt::revert_pending()));
    text(&["sync", "--keep", "studio", "data/moves/0002.json"], &root)
        .assert()
        .success()
        .stdout(contains("kept studio data/moves/0002.json"));
    assert_eq!(std::fs::read(&member).unwrap()[3], 99, "the JSON's power");
    text(&["sync"], &root).assert().success();
    text(&["sync", "--keep", "both", "data/moves/0002.json"], &root)
        .assert()
        .code(1);

    // A broken JSON is not settled either: listed, exit 1, until it is fixed.
    let json_path = root.join("data/moves/0003.json");
    let good = std::fs::read(&json_path).unwrap();
    std::fs::write(&json_path, b"{ broken").unwrap();
    let assert = text(&["sync"], &root).assert().code(1);
    let out = stdout_of(&assert);
    println!("{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("broken data/moves/0003.json Move 0003")),
        "{out}"
    );
    assert!(
        String::from_utf8_lossy(&assert.get_output().stderr).contains("broken file"),
        "{:?}",
        assert.get_output()
    );
    std::fs::write(&json_path, good).unwrap();
    text(&["sync"], &root).assert().success();

    // Taken back: the manifest and data/ are in the backup.
    let assert = text(&["revert"], &root).assert().success();
    let out = stdout_of(&assert);
    println!("{out}");
    let backup = out
        .lines()
        .find_map(|l| l.strip_prefix("backup="))
        .and_then(|l| l.split(" rewritten=").next())
        .unwrap_or_else(|| panic!("no backup line:\n{out}"));
    assert!(
        std::path::Path::new(backup)
            .join("g4-project.json")
            .is_file()
    );
    assert!(
        std::path::Path::new(backup)
            .join("data/moves/0001.json")
            .is_file()
    );
    assert!(!root.join("g4-project.json").exists());
    assert!(!root.join("data").exists());
}

#[test]
fn cli_text_rename_id() {
    let (_tmp, root, _nds) = text_layout();
    text(&["adopt"], &root).assert().success();
    let rename = |old: &str, new: &str| {
        let mut cmd = g4k();
        cmd.args(["text", "rename-id"]).arg(&root).args([old, new]);
        cmd
    };

    // Renamed: one line per file with its count, then the totals.
    let assert = rename("TRAINER_YOUNGSTER_KAI", "TRAINER_KID_KAI")
        .assert()
        .success();
    let out = stdout_of(&assert);
    println!("{out}");
    assert!(
        out.lines().any(|l| l == "data/ids/trainers.json 1"),
        "{out}"
    );
    assert!(
        out.lines().any(|l| l == "data/trainers/0001.json 1"),
        "{out}"
    );
    let summary = out.lines().last().unwrap();
    assert!(
        summary.starts_with(
            "renamed TRAINER_YOUNGSTER_KAI -> TRAINER_KID_KAI kind=trainer number=1 files=2 refs=2 \
             id_files=true ms="
        ),
        "{summary}"
    );
    let json = std::fs::read_to_string(root.join("data/trainers/0001.json")).unwrap();
    assert!(json.contains("\"id\": \"TRAINER_KID_KAI\""), "{json}");
    let ids = std::fs::read_to_string(root.join(".rotom/command_database/.g4-ids-trainers.json"))
        .unwrap();
    assert!(ids.contains("TRAINER_KID_KAI"), "{ids}");

    // A taken id: refused with the sentence, exit 1, nothing written.
    rename("TRAINER_KID_KAI", "TRAINER_POKEMON_TRAINER_MIA")
        .assert()
        .code(1)
        .stderr(contains(
            "TRAINER_POKEMON_TRAINER_MIA already exists (number 2).",
        ));
    assert_eq!(
        std::fs::read_to_string(root.join("data/trainers/0001.json")).unwrap(),
        json
    );

    // With --json: the plan as one line.
    let assert = rename("TRAINER_KID_KAI", "TRAINER_KAI")
        .arg("--json")
        .assert()
        .success();
    let line = stdout_of(&assert);
    let plan: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(plan["old_id"], "TRAINER_KID_KAI");
    assert_eq!(plan["new_id"], "TRAINER_KAI");
    assert_eq!(plan["total"], 2);
}

#[test]
fn cli_git_templates() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let templates = || {
        let mut cmd = g4k();
        cmd.args(["git", "templates"]).arg(root);
        cmd
    };
    let assert = templates().assert().success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(
        lines,
        [
            "created .gitignore",
            "created .gitattributes",
            "layout=dspre ignored_archives=0",
        ],
        "{out}"
    );
    let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(
        ignore.starts_with(g4_kernel::git::templates::BLOCK_START),
        "{ignore}"
    );

    let assert = templates().assert().success();
    let out = stdout_of(&assert);
    assert!(out.contains("unchanged .gitignore\n"), "{out}");
    assert!(out.contains("unchanged .gitattributes\n"), "{out}");

    // A broken block: refused with the file name, exit 1.
    std::fs::write(
        root.join(".gitignore"),
        format!("{}\n", g4_kernel::git::templates::BLOCK_START),
    )
    .unwrap();
    templates().assert().code(1).stderr(contains(format!(
        ".gitignore: {}",
        g4_kernel::git::templates::broken_block()
    )));
}

/// `g4k` with the isolated Git environment of `GitFixture::cli` (child only, never `set_var`).
fn g4k_isolated(home: &std::path::Path) -> Option<Command> {
    let cli = common::GitFixture::cli(home)?;
    let mut cmd = g4k();
    cmd.envs(cli.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    cmd.env_remove("G4_GIT");
    Some(cmd)
}

#[test]
fn cli_git_status_honours_g4_git() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let assert = g4k()
        .args(["git", "status"])
        .arg(root)
        .env("G4_GIT", "/nicht/da")
        .assert()
        .success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines, ["no repository", "git not found"], "{out}");

    let Some(found) = g4_kernel::git::find_git() else {
        eprintln!("skipped: git not found");
        return;
    };
    let assert = g4k()
        .args(["git", "status"])
        .arg(root)
        .env("G4_GIT", &found.path)
        .assert()
        .success();
    let out = stdout_of(&assert);
    let last = out.lines().last().unwrap();
    assert!(last.starts_with("git version"), "{out}");
}

#[test]
fn cli_git_init_and_status() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let home = tempfile::tempdir().unwrap();
    let Some(mut init) = g4k_isolated(home.path()) else {
        return;
    };
    let assert = init.args(["git", "init"]).arg(root).assert().success();
    let out = stdout_of(&assert);
    assert!(out.contains("created .gitignore\n"), "{out}");
    let last = out.lines().last().unwrap();
    assert!(last.starts_with("committed Initial state files="), "{out}");
    assert!(last.contains(" branch="), "{out}");

    let mut status = g4k_isolated(home.path()).unwrap();
    let assert = status.args(["git", "status"]).arg(root).assert().success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(lines[0].starts_with("branch "), "{out}");
    assert!(lines[0].ends_with(" changes 0"), "{out}");
    assert!(!lines[0].contains("head none"), "{out}");
    assert!(lines[1].starts_with("git version"), "{out}");

    // A second time: the repository is there, only the templates.
    let mut again = g4k_isolated(home.path()).unwrap();
    let assert = again.args(["git", "init"]).arg(root).assert().success();
    let out = stdout_of(&assert);
    assert!(
        out.ends_with(&format!("not committed: {}\n", g4_kernel::git::in_repo())),
        "{out}"
    );
}

#[test]
fn cli_git_init_without_git_writes_the_templates_and_succeeds() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    // The reason is decided by its code, not by the sentence: no `git` is no failure.
    let assert = g4k()
        .args(["git", "init"])
        .arg(root)
        .env("G4_GIT", "/nicht/da")
        .assert()
        .success();
    let out = stdout_of(&assert);
    assert!(
        out.ends_with(&format!(
            "not committed: {}\n",
            g4_kernel::git::not_installed()
        )),
        "{out}"
    );
    assert!(!root.join(".git").exists());
    assert!(root.join(".gitattributes").is_file());
}

#[test]
fn cli_git_init_fails_when_the_first_commit_fails() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let home = tempfile::tempdir().unwrap();
    let Some(mut init) = g4k_isolated(home.path()) else {
        return;
    };
    // A signing program that fails: Git's own refusal, neither an existing repository nor a
    // missing Git, so the exit code says it went wrong.
    init.env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "true")
        .env("GIT_CONFIG_KEY_1", "gpg.program")
        .env("GIT_CONFIG_VALUE_1", "false");
    let assert = init.args(["git", "init"]).arg(root).assert().failure();
    let out = stdout_of(&assert);
    assert!(out.contains("not committed: "), "{out}");
    assert!(
        !out.contains(&g4_kernel::git::in_repo())
            && !out.contains(&g4_kernel::git::not_installed()),
        "{out}"
    );
    assert!(root.join(".git").is_dir());
}

#[test]
fn cli_git_init_templates_only() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let assert = g4k()
        .args(["git", "init"])
        .arg(root)
        .arg("--templates-only")
        .assert()
        .success();
    let out = stdout_of(&assert);
    assert!(out.ends_with("not committed\n"), "{out}");
    assert!(!root.join(".git").exists());
    assert!(root.join(".gitattributes").is_file());
}

#[test]
fn cli_git_commit_all_prints_the_commit() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    let home = tempfile::tempdir().unwrap();
    let Some(mut init) = g4k_isolated(home.path()) else {
        return;
    };
    init.args(["git", "init"]).arg(root).assert().success();
    std::fs::write(root.join("notiz.txt"), "neu\n").unwrap();

    let commit_all = || {
        let mut cmd = g4k_isolated(home.path()).unwrap();
        cmd.args(["git", "commit-all"]).arg(root).args([
            "--message",
            "Stand Test",
            "--notes",
            "Notiz",
        ]);
        cmd
    };
    let assert = commit_all().assert().success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 1, "{out}");
    assert!(lines[0].starts_with("committed "), "{out}");
    assert!(lines[0].ends_with(" \"Stand Test\" files=1"), "{out}");
    let status = g4_kernel::git::status(root).unwrap();
    assert!(status.paths.is_empty(), "{:?}", status.paths);
    assert_eq!(status.head.unwrap().summary, "Stand Test");
    let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
    assert!(!config.contains("[remote"), "{config}");

    // Nothing changed since: refused with the sentence, exit 1.
    commit_all()
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::git::nothing_to_commit()));
}

/// A clone of the exported fixture `fx`: its sources only (no DSPRE part, no members).
fn text_clone(fx: &common::TextFixture) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let clone = tmp.path().join("klon");
    for rel in g4_kernel::textproject::sources(&fx.project()).unwrap() {
        let to = clone.join(&rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(fx.root().join(&rel), &to).unwrap();
    }
    (tmp, clone)
}

#[test]
fn cli_text_restore() {
    use g4_kernel::textproject::restore::{has_game_files, needs_restore};

    let fx = common::TextFixture::exported();
    // A project with its game files: refused, nothing written.
    g4k()
        .args(["text", "restore"])
        .arg(fx.root())
        .arg("--base-rom")
        .arg(fx.rom())
        .assert()
        .code(1)
        .stderr(contains(has_game_files()));

    let (_tmp, clone) = text_clone(&fx);
    assert!(needs_restore(&clone));
    // ds-rom cannot extract the header-only synthetic ROM: the restore fails, the clone stays
    // as it was and no temp folder is left (the whole restore runs in the gated `tests/git.rs`).
    g4k()
        .args(["text", "restore"])
        .arg(&clone)
        .arg("--base-rom")
        .arg(fx.rom())
        .assert()
        .code(1)
        .stdout(predicates::str::is_empty());
    assert!(needs_restore(&clone));
    assert!(
        !clone.join(".g4/tmp").exists()
            || std::fs::read_dir(clone.join(".g4/tmp"))
                .unwrap()
                .next()
                .is_none()
    );

    // Without a mark: the refusal of `text export`.
    std::fs::remove_file(clone.join(".g4/base.json")).unwrap();
    g4k()
        .args(["text", "restore"])
        .arg(&clone)
        .arg("--base-rom")
        .arg(fx.rom())
        .assert()
        .code(1)
        .stderr(contains(g4_kernel::patch::no_mark()));
}

/// The synthetic project with its trainers unpacked, committed with gix only, then trainer 1's
/// first team member one level up and saved.
fn committed_with_trainer_1_changed() -> common::Synthetic {
    use g4_kernel::project::{AssetKind, Project};

    let s = common::synthetic_project();
    let root = s.dir.path();
    let project = Project::open(root).unwrap();
    project
        .ensure_unpacked(AssetKind::TrainerProperties)
        .unwrap();
    project.ensure_unpacked(AssetKind::TrainerParty).unwrap();
    common::GitFixture::init(root);
    common::GitFixture::commit_all(root, "Ausgangsstand");
    let mut store = g4_kernel::store::Store::new(std::sync::Arc::new(project));
    store
        .edit_trainer(1, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .unwrap();
    store.save().unwrap();
    s
}

#[test]
fn cli_diff_prints_fields_against_head() {
    let s = committed_with_trainer_1_changed();
    let root = s.dir.path();
    for id in ["1", "TRAINER_0001"] {
        let assert = g4k()
            .args(["diff"])
            .arg(root)
            .args(["trainer", id])
            .assert()
            .success();
        let out = stdout_of(&assert);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(
            lines,
            ["field data.party[0].level 5 -> 6", "state changed"],
            "{id}: {out}"
        );
    }
    let assert = g4k()
        .args(["diff"])
        .arg(root)
        .args(["trainer", "0", "--against", "head", "--json"])
        .assert()
        .success();
    let report: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(report["state"], "same", "{report}");
    assert_eq!(report["subject"]["kind"], "trainer", "{report}");
    assert_eq!(report["diff"]["shape"], "fields", "{report}");

    // Against the base without a base mark: the app's refusal, exit 1.
    g4k()
        .args(["diff"])
        .arg(root)
        .args(["trainer", "1", "--against", "base"])
        .assert()
        .failure()
        .stderr(contains(g4_kernel::patch::no_mark()));
    // An unknown kind.
    g4k()
        .args(["diff"])
        .arg(root)
        .args(["trainers", "1"])
        .assert()
        .failure();
}

#[test]
fn cli_git_status_lists_entries() {
    let s = committed_with_trainer_1_changed();
    let root = s.dir.path();
    let assert = g4k()
        .args(["git", "status"])
        .arg(root)
        .arg("--entries")
        .env("G4_GIT", "/nicht/da")
        .assert()
        .success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert!(lines[0].starts_with("branch main head "), "{out}");
    assert!(lines[0].ends_with(" changes 1"), "{out}");
    assert_eq!(lines[1], "changed Trainers Trainer 0001", "{out}");
    assert_eq!(lines[2], "git not found", "{out}");
}

/// Without a commit there is nothing to compare against: `--entries` lists no entity (as the
/// app and MCP), while the count still names every path.
#[test]
fn cli_git_status_lists_no_entries_without_a_commit() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    common::GitFixture::init(root);
    let assert = g4k()
        .args(["git", "status"])
        .arg(root)
        .arg("--entries")
        .env("G4_GIT", "/nicht/da")
        .assert()
        .success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[0].starts_with("branch main head none changes "),
        "{out}"
    );
    assert!(!lines[0].ends_with(" changes 0"), "{out}");
    assert_eq!(lines[1], "git not found", "{out}");
}

#[test]
fn cli_recipe_list() {
    let assert = g4k().args(["recipe", "list"]).assert().success();
    let out = stdout_of(&assert);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(
        lines,
        [
            "create_area  Create area",
            "create_room  Create room",
            "extend_area  Extend area",
            "empty_region  Empty world  irreversible",
            "new_figure  New character",
            "connect_doors  Connect door",
            "new_trainer_here  New Trainer here  irreversible",
            "new_story  New story",
            "talk_advances  Conversation advances",
            "figure_appears  Character appears/disappears from step",
            "scene_on_enter  Scene on entering",
            "reward_once  One-time reward",
            "new_painted_map  Paint new map  irreversible",
        ],
        "{out}"
    );
}

/// Every file below `root` with its bytes, by path.
fn files_of(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// The new cell (2, 0) of matrix 0 of the synthetic project (its matrix is 2 × 1).
const EXTEND_INPUT: &str =
    r#"{"matrix":0,"x":2,"y":0,"source_map":1,"clear_buildings":false,"header_cell":null}"#;

#[test]
fn cli_recipe_plan_extend_prints_a_draft() {
    let s = common::synthetic_project();
    let before = files_of(s.dir.path());
    let assert = g4k()
        .args(["recipe", "plan"])
        .arg(s.dir.path())
        .args(["extend_area", "--input", EXTEND_INPUT])
        .assert()
        .success();
    let draft: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(
        draft["cells"],
        serde_json::json!([{ "matrix": 0, "x": 2, "y": 0, "source_map": 1 }]),
        "{draft}"
    );
    assert!(!draft["notes"].as_array().unwrap().is_empty(), "{draft}");
    // Planning writes nothing; loading a member unpacks it (a new file below unpacked/).
    let root = s.dir.path();
    let changed: Vec<_> = files_of(root)
        .into_iter()
        .map(|(path, bytes)| (path.strip_prefix(root).unwrap().to_path_buf(), bytes))
        .filter(|(path, bytes)| match before.get(&root.join(path)) {
            Some(old) => old != bytes,
            None => !path.starts_with("unpacked"),
        })
        .map(|(path, _)| path)
        .collect();
    assert!(changed.is_empty(), "plan wrote {changed:?}");
    // A recipe it does not know, or input of another shape, is refused.
    g4k()
        .args(["recipe", "plan"])
        .arg(s.dir.path())
        .arg("volcano")
        .assert()
        .code(1)
        .stderr(contains("unknown recipe"));
    g4k()
        .args(["recipe", "plan"])
        .arg(s.dir.path())
        .args(["extend_area", "--input", r#"{"matrix":0}"#])
        .assert()
        .code(1)
        .stderr(contains("The input for “Extend area” does not fit"));
}

#[test]
fn cli_recipe_apply_extend_prints_the_outcome_and_saves() {
    let s = common::synthetic_project();
    let assert = g4k()
        .args(["recipe", "apply"])
        .arg(s.dir.path())
        .args(["extend_area", "--input", EXTEND_INPUT])
        .assert()
        .success();
    let outcome: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(outcome["report"]["width"], 3, "{outcome}");
    assert!(
        outcome["changed"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({ "kind": "Matrix", "index": 0 })),
        "{outcome}"
    );
    // Saved: the cell is taken now.
    g4k()
        .args(["recipe", "plan"])
        .arg(s.dir.path())
        .args(["extend_area", "--input", EXTEND_INPUT])
        .assert()
        .code(1);
}

#[test]
fn cli_recipe_apply_empty_region_needs_the_flag() {
    let s = common::synthetic_project();
    g4k()
        .args(["recipe", "apply"])
        .arg(s.dir.path())
        .arg("empty_region")
        .assert()
        .code(1)
        .stderr(contains(
            "error: Empty world cannot be undone; use --allow-irreversible",
        ));
    assert!(!s.dir.path().join("rotom.toml").exists());
    // With the flag it gets past the question; the synthetic world then has no start town.
    let assert = g4k()
        .args(["recipe", "apply"])
        .arg(s.dir.path())
        .args(["empty_region", "--allow-irreversible"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(!stderr.contains("cannot be undone"), "{stderr}");
}

/// `g4k recipe apply new_story` on the text fixture: the input names the story and the pool
/// variable (`--input`), the outcome is printed as JSON and saved (the variable's own name, the
/// story file); a second story on the same variable is refused, its name is taken now.
#[test]
fn cli_recipe_apply_new_story_prints_the_outcome_and_saves() {
    let fx = common::TextFixture::exported();
    let root = fx.root();
    let input = r#"{"name":"Testfaden","var":16432,"notes":"Aus g4k."}"#;
    let assert = g4k()
        .args(["recipe", "apply"])
        .arg(root)
        .args(["new_story", "--input", input])
        .assert()
        .success();
    let outcome: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(outcome["report"]["story"], "STORY_TESTFADEN", "{outcome}");
    assert_eq!(outcome["report"]["number"], 0, "{outcome}");
    assert_eq!(outcome["report"]["var"], 16432, "{outcome}");
    assert_eq!(
        outcome["report"]["var_id"], "VAR_STORY_TESTFADEN",
        "{outcome}"
    );
    assert_eq!(outcome["created"], serde_json::json!([]), "{outcome}");
    let next: Vec<&str> = outcome["next"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(next, ["talk_advances", "figure_appears"], "{outcome}");

    // Saved: the variable's own name and the story with its first step are on disk.
    let read = |rel: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(root.join(rel)).unwrap()).unwrap()
    };
    assert_eq!(read("data/ids/vars.json")["VAR_STORY_TESTFADEN"], 16432);
    assert_eq!(read("data/ids/stories.json")["STORY_TESTFADEN"], 0);
    let story = read("data/stories/0000.json");
    assert_eq!(story["name"], "Testfaden", "{story}");
    assert_eq!(story["notes"], "Aus g4k.", "{story}");
    assert_eq!(story["steps"][0]["key"], "s0", "{story}");
    assert_eq!(story["steps"][0]["kind"], "start", "{story}");
    assert_eq!(story["steps"][0]["made"]["recipe"], "new_story", "{story}");

    // The variable has its own name now: a second story on it is refused, nothing is written.
    let before = files_of(root);
    g4k()
        .args(["recipe", "apply"])
        .arg(root)
        .args([
            "new_story",
            "--input",
            r#"{"name":"Zweiter Faden","var":16432}"#,
        ])
        .assert()
        .code(1)
        .stderr(contains("VAR_STORY_TESTFADEN"));
    assert!(!root.join("data/stories/0001.json").exists());
    assert_eq!(
        before.get(&root.join("data/ids/vars.json")),
        files_of(root).get(&root.join("data/ids/vars.json"))
    );
}

/// `g4k recipe apply` asks the project whether a visibility creates a level script file: a
/// figure on header 0, which owns its level script file, goes through without
/// `--allow-irreversible`; one on header 1, which has none (a new file would stay after undo),
/// needs the flag.
#[test]
fn cli_recipe_apply_figure_appears_asks_only_when_a_level_script_file_is_created() {
    use std::sync::Arc;

    use g4_kernel::recipes::new_story::{self, NewStoryInput};
    use g4_kernel::store::{ObjectEvent, Store};

    let fx = common::TextFixture::exported();
    let root = fx.root();
    {
        let mut store = Store::new(Arc::new(fx.project()));
        new_story::apply(
            &mut store,
            &NewStoryInput {
                name: "Testfaden".to_owned(),
                var: 0x4030,
                notes: String::new(),
            },
        )
        .unwrap();
        let object = |local_id: u16| ObjectEvent {
            local_id,
            graphics_id: 3,
            movement_type: 15,
            trainer_type: 0,
            hidden_flag: 0,
            script: 0,
            dir: 0,
            data: [0; 3],
            movement_range_x: 0,
            movement_range_z: 0,
            x: 30 + local_id,
            z: 7,
            y: 0,
        };
        store
            .transaction("Figuren", |tx| {
                tx.edit_event(0, |f| {
                    f.object_events.extend([object(4), object(5)]);
                    Ok(())
                })
            })
            .unwrap();
        store.save().unwrap();
    }
    let input = |header: u16, local_id: u16, flag: u16| {
        serde_json::json!({
            "story": "STORY_TESTFADEN",
            "figure": { "header": header, "local_id": local_id },
            "step": 0, "appears": true, "flag": flag,
        })
        .to_string()
    };
    let assert = g4k()
        .args(["recipe", "apply"])
        .arg(root)
        .args(["figure_appears", "--input", &input(0, 4, 0x0100)])
        .assert()
        .success();
    let outcome: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(
        outcome["report"]["level_script_created"], false,
        "{outcome}"
    );
    // Header 1 (same event and script file) has level script 0: a new file is created.
    g4k()
        .args(["recipe", "apply"])
        .arg(root)
        .args(["figure_appears", "--input", &input(1, 5, 0x0101)])
        .assert()
        .code(1)
        .stderr(contains(
            "error: Create level script file cannot be undone; use --allow-irreversible",
        ));
    let assert = g4k()
        .args(["recipe", "apply"])
        .arg(root)
        .args([
            "figure_appears",
            "--input",
            &input(1, 5, 0x0101),
            "--allow-irreversible",
        ])
        .assert()
        .success();
    let outcome: serde_json::Value = serde_json::from_str(&stdout_of(&assert)).unwrap();
    assert_eq!(outcome["report"]["level_script_created"], true, "{outcome}");
}

/// `script Main #1:` with `lines`, then `End`.
fn rotom_script(lines: &[&str]) -> Vec<u8> {
    let mut out = String::from("script Main #1:\n");
    for line in lines {
        out.push_str(&format!("    {line}\n"));
    }
    out.push_str("    End\n");
    out.into_bytes()
}

#[test]
fn story_create_and_list_on_a_text_fixture() {
    let fx = common::TextFixture::exported();
    let root = fx.root();
    // A thread: 0x4030 set to 1 and 2 in script file 0 (header 0), to 3 in script file 0102
    // (no header names it).
    fx.write(
        "expanded/scripts/0000.rotom",
        &rotom_script(&["SetVar 0x4030, 1", "SetVar 0x4030, 2"]),
    );
    fx.write(
        "expanded/scripts/0102.rotom",
        &rotom_script(&["SetVar 0x4030, 3"]),
    );
    let story = |args: &[&str]| {
        let mut cmd = g4k();
        cmd.args(["story", args[0]]).arg(root).args(&args[1..]);
        cmd
    };

    // Nothing yet.
    let out = stdout_of(&story(&["list"]).assert().success());
    assert!(out.is_empty(), "{out}");

    let assert = story(&["create", "--name", "Test", "--var", "VAR_UNUSED_0x406C"])
        .assert()
        .success();
    let out = stdout_of(&assert);
    assert!(
        out.starts_with(
            "created STORY_TEST number=0 files=data/ids/stories.json,data/stories/0000.json,\
             data/stories/0000.layout.json ms="
        ),
        "{out}"
    );
    let file = std::fs::read_to_string(root.join("data/stories/0000.json")).unwrap();
    assert!(file.contains("\"id\": \"STORY_TEST\""), "{file}");
    assert!(file.contains("\"var\": \"VAR_UNUSED_0x406C\""), "{file}");
    assert!(root.join("data/stories/0000.layout.json").is_file());

    let out = stdout_of(&story(&["list"]).assert().success());
    assert_eq!(
        out.trim_end(),
        "0000 STORY_TEST var=VAR_UNUSED_0x406C steps=0 Test"
    );

    let out = stdout_of(&story(&["get", "STORY_TEST"]).assert().success());
    let got: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(got["number"], 0);
    assert_eq!(got["story"]["name"], "Test");
    assert_eq!(got["layout"], serde_json::json!({ "nodes": {} }));
    story(&["get", "STORY_NOPE"])
        .assert()
        .code(1)
        .stderr(contains("Unknown ID: STORY_NOPE."));

    // A refusal: unknown progress, nothing written.
    story(&["create", "--name", "Zwei", "--var", "VAR_NOPE"])
        .assert()
        .code(1)
        .stderr(contains("Unknown ID: VAR_NOPE."));
    assert!(!root.join("data/stories/0001.json").exists());

    // The threads: one line each, or JSON.
    let out = stdout_of(&story(&["threads"]).assert().success());
    assert!(
        out.lines()
            .any(|l| l == "0x4030 VAR_PLAYER_STARTER 3 values · 2 places"),
        "{out}"
    );
    let out = stdout_of(&story(&["threads", "--json"]).assert().success());
    let threads: serde_json::Value = serde_json::from_str(&out).unwrap();
    let thread = threads
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["var"] == 0x4030)
        .unwrap();
    assert_eq!(thread["values"], serde_json::json!([1, 2, 3]));

    // The graph of the thread by its name.
    let out = stdout_of(&story(&["graph", "VAR_PLAYER_STARTER"]).assert().success());
    let graph: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(graph["var"], 0x4030);
    let values = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "value")
        .count();
    assert_eq!(values, 3);

    // Deleting takes register entry, story and side file; an unknown id writes nothing.
    story(&["delete", "STORY_NOPE"])
        .assert()
        .code(1)
        .stderr(contains("Unknown ID: STORY_NOPE."));
    assert!(root.join("data/stories/0000.json").is_file());
    let out = stdout_of(&story(&["delete", "STORY_TEST"]).assert().success());
    assert!(
        out.starts_with(
            "deleted STORY_TEST number=0 files=data/ids/stories.json,data/stories/0000.json,\
             data/stories/0000.layout.json ms="
        ),
        "{out}"
    );
    assert!(!root.join("data/stories/0000.json").exists());
    assert!(!root.join("data/stories/0000.layout.json").exists());
    let out = stdout_of(&story(&["list"]).assert().success());
    assert!(out.is_empty(), "{out}");
}

#[test]
fn cli_delete_plan_apply_restore() {
    use g4_kernel::delete::DeleteTarget;
    use g4_kernel::flags::{FlagKind, NameRequest, name_flag_files};
    use serde_json::{Value, json};

    let fx = common::TextFixture::exported();
    name_flag_files(
        &fx.project(),
        &NameRequest {
            kind: FlagKind::Flag,
            number: 148,
            id: Some("FLAG_HAT_MEDAILLE".into()),
            description: None,
        },
    )
    .unwrap();
    let flag_file = fx.root().join("data/flags/0148.json");
    assert!(flag_file.is_file());
    let json_of = |assert: &assert_cmd::assert::Assert| -> Value {
        serde_json::from_str(&stdout_of(assert)).unwrap()
    };

    // An own flag name needs no base ROM; the plan writes nothing.
    let assert = g4k()
        .args(["delete", "plan"])
        .arg(fx.root())
        .args(["flag", "148"])
        .assert()
        .success();
    let plan = json_of(&assert);
    assert_eq!(plan["target"], json!({ "kind": "flag", "number": 148 }));
    assert_eq!(plan["label"], "Flag 0x0094");
    assert_eq!(plan["refusal"], Value::Null);
    assert_eq!(plan["references"], json!([]));
    assert!(flag_file.is_file());
    // A trainer asks the base ROM: the header-only ROM of the fixture holds no archives.
    g4k()
        .args(["delete", "plan"])
        .arg(fx.root())
        .args(["trainer", "1", "--base-rom"])
        .arg(fx.rom())
        .assert()
        .code(1);
    // A kind that is never deleted.
    g4k()
        .args(["delete", "plan"])
        .arg(fx.root())
        .args(["species", "1"])
        .assert()
        .code(1)
        .stderr(contains("unknown kind"));

    let assert = g4k()
        .args(["delete", "apply"])
        .arg(fx.root())
        .args(["flag", "148"])
        .assert()
        .success();
    let result = json_of(&assert);
    assert!(!result["change"].is_null(), "{result}");
    assert_eq!(result["freed"], Value::Null);
    assert!(!flag_file.exists());

    // „Nummer freigeben“ needs the flag; nothing is written without it.
    g4k()
        .args(["delete", "apply"])
        .arg(fx.root())
        .args(["trainer", "2", "--free-number"])
        .assert()
        .code(1)
        .stderr(contains(
            "error: delete_entity cannot be undone (free the number); headless only with \
             --allow-irreversible",
        ));
    assert!(fx.root().join("data/trainers/0002.json").is_file());

    // A tombstone saved by the store comes back, saved.
    let trainer = "data/trainers/0001.json";
    let mut store = g4_kernel::store::Store::new(std::sync::Arc::new(fx.project()));
    store
        .transaction("Gelöscht", |tx| {
            tx.set_deleted(DeleteTarget::Trainer { index: 1 }, true)
        })
        .unwrap();
    store.save().unwrap();
    assert!(fx.text(trainer).contains("\"deleted\": true"));
    let assert = g4k()
        .args(["delete", "restore"])
        .arg(fx.root())
        .args(["trainer", "1"])
        .assert()
        .success();
    let result = json_of(&assert);
    assert_eq!(
        result["changed"],
        json!([{ "kind": "Trainer", "index": 1 }])
    );
    assert!(
        !fx.text(trainer).contains("\"deleted\""),
        "{}",
        fx.text(trainer)
    );
    g4k()
        .args(["delete", "restore"])
        .arg(fx.root())
        .args(["trainer", "1"])
        .assert()
        .code(1)
        .stderr(contains("Trainer 0001 is not deleted."));
}

#[test]
fn import_samples_writes_the_files() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("samples");
    let out = g4k()
        .args(["import-samples"])
        .arg(&target)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 14, "{text}");
    for line in &lines {
        let path = std::path::Path::new(line);
        assert!(path.is_file(), "{line}");
        assert_eq!(path.parent().unwrap(), target);
    }
    assert!(target.join("g4_imp_plane.glb").is_file());
}
