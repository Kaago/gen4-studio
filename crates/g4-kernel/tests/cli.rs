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
            "warning: Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.",
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
        .stderr(contains("Falsches Spiel"));
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
            "message": "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.",
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
            "warning: Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.",
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
        .stderr(contains("Falsches Spiel"));
    let check: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(check["warnings"], serde_json::json!([]), "{check}");
    assert!(
        check["refused"]
            .as_str()
            .unwrap()
            .starts_with("Falsches Spiel")
    );
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
            "{} Ordne sie mit `g4k base assign <Projekt> <nds>` zu.",
            g4_kernel::patch::NO_MARK
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
        .stderr(contains(g4_kernel::patch::NO_MARK))
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
    assert!(exported_manifest.contains("\"schemaVersion\": 1"));
    std::fs::write(
        &manifest,
        exported_manifest.replacen("\"schemaVersion\": 1", "\"schema\": 0", 1),
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
    let changed = format!("migrated 0 -> 1 changed={} snapshot=", registers.len() + 1);
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
        (Some(1), Some(1))
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
        .stderr(contains("neueren Fassung"));
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
