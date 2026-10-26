#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): assigning the DSPRE folder's own `.nds` as its base ROM and
//! rebuilding it byte for byte; reading archives and headers from that `.nds`.
mod common;

use std::path::PathBuf;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::project::Project;
use g4_kernel::start::{BaseWarningKind, assign_base, inspect_rom, suggest_base};

/// The vanilla DSPRE folder takes its sibling `.nds` without a warning, and a build of the
/// copy with the new mark equals that `.nds` (the mark restores the secure area CRC).
#[test]
fn base_assign_on_the_vanilla_folder_has_no_warning_and_rebuilds_the_nds() {
    let Some(orig) = common::vanilla_project() else {
        return;
    };
    // The original is only read: its sibling `.nds` is found and hashed, never written.
    let Some(nds) = suggest_base(&orig) else {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            orig.display()
        );
        return;
    };
    let Some((tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let (check, mark) = assign_base(&copy, &nds).unwrap();
    assert_eq!(check.refused, None);
    assert!(check.warnings.is_empty(), "{:#?}", check.warnings);
    assert!(check.info.known_base.is_some(), "{:?}", check.info);
    assert_eq!(mark.origin, "dspre");
    assert_eq!(mark.sha1, check.info.sha1);

    let project = Project::open(&copy).unwrap();
    let out = tmp.path().join("out.nds");
    g4_kernel::build::build(&project, &out).unwrap();
    let rebuilt = inspect_rom(&out).unwrap();
    assert_eq!(
        rebuilt.sha1, check.info.sha1,
        "the rebuilt ROM differs from the base ROM"
    );
    eprintln!(
        "rebuild {} sha1={} equal",
        check.info.game_code, rebuilt.sha1
    );

    // The ARM7 and banner comparisons ran on both sides: a changed `arm7.bin` and a changed
    // German banner title in the copy each warn.
    let arm7_path = copy.join("arm7/arm7.bin");
    let mut arm7 = std::fs::read(&arm7_path).unwrap();
    arm7[0] ^= 0xFF;
    std::fs::write(&arm7_path, arm7).unwrap();
    let banner_path = copy.join("banner/banner.yaml");
    let mut banner: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(&banner_path).unwrap()).unwrap();
    banner["title"]["german"] = serde_yaml::Value::String("Anderes Spiel".into());
    std::fs::write(&banner_path, serde_yaml::to_string(&banner).unwrap()).unwrap();
    let kinds: Vec<_> = g4_kernel::start::check_base(&copy, &nds)
        .unwrap()
        .warnings
        .into_iter()
        .map(|w| w.kind)
        .collect();
    assert_eq!(
        kinds,
        [BaseWarningKind::Arm7Differs, BaseWarningKind::BannerDiffers]
    );
}

/// The vanilla project and its sibling `.nds`, both only read; `None` skips the test.
fn vanilla_with_base() -> Option<(PathBuf, PathBuf)> {
    let orig = common::vanilla_project()?;
    let Some(nds) = suggest_base(&orig) else {
        eprintln!(
            "skipped: no <Name>.nds with the same game code next to {}",
            orig.display()
        );
        return None;
    };
    Some((orig, nds))
}

/// Every archive the vanilla folder has unpacked has the same members in its `.nds`.
#[test]
fn base_rom_members_equal_the_vanilla_folders() {
    let Some((orig, nds)) = vanilla_with_base() else {
        return;
    };
    let project = Project::open(&orig).unwrap();
    let started = Instant::now();
    let base = BaseRom::open(&nds, None).unwrap();
    let open_ms = started.elapsed().as_millis();
    assert_eq!(base.game_code(), project.game_code);
    let started = Instant::now();
    let mut compared = Vec::new();
    for kind in project.archives.kinds() {
        let dir = project.unpacked_dir(kind);
        if !dir.is_dir() {
            continue;
        }
        let ours = g4_kernel::build::pack_dir(&dir).unwrap().members;
        let theirs = base
            .narc(kind)
            .unwrap()
            .unwrap_or_else(|| panic!("{kind}: no archive in the base ROM"))
            .members;
        assert_eq!(ours.len(), theirs.len(), "{kind}: member count");
        if let Some(i) = (0..ours.len()).find(|&i| ours[i] != theirs[i]) {
            panic!("{kind}: member {i:04} differs");
        }
        compared.push(kind);
    }
    // A member read alone is the member of the archive.
    let first = compared[0];
    assert_eq!(
        base.member(first, 0).unwrap(),
        Some(std::fs::read(project.member_path(first, 0)).unwrap())
    );
    assert!(compared.len() >= 10, "{compared:?}");
    eprintln!(
        "{}: {} archives equal; open+sha1 {open_ms} ms, archives {} ms",
        base.game_code(),
        compared.len(),
        started.elapsed().as_millis()
    );
}

/// The header table read from the `.nds` (ARM9 decompressed for HGSS) equals the project's.
#[test]
fn base_rom_header_table_equals_the_vanilla_project() {
    let Some((orig, nds)) = vanilla_with_base() else {
        return;
    };
    let project = Project::open(&orig).unwrap();
    let base = BaseRom::open(&nds, None).unwrap();
    let started = Instant::now();
    let arm9 = base.arm9().unwrap();
    let arm9_ms = started.elapsed().as_millis();
    assert_eq!(
        arm9,
        std::fs::read(orig.join("arm9/arm9.bin")).unwrap(),
        "arm9 differs from arm9/arm9.bin"
    );
    let started = Instant::now();
    let headers = base.header_table().unwrap();
    let headers_ms = started.elapsed().as_millis();
    let expected = project.read_headers().unwrap();
    assert_eq!(headers.len(), expected.len());
    assert!(headers == expected, "the header tables differ");
    eprintln!(
        "{}: {} headers equal; arm9 {arm9_ms} ms, header table {headers_ms} ms",
        base.game_code(),
        headers.len()
    );
}
