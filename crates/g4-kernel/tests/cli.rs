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
