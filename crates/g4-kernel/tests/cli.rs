#![cfg(feature = "test-support")]
mod common;

use assert_cmd::Command;
use predicates::str::contains;

fn g4k() -> Command {
    Command::cargo_bin("g4k").unwrap()
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
