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

/// Applies `patch` to `nds` with the xdelta3 CLI, if it is in the PATH, and checks the
/// result's SHA-1; returns whether the cross-check ran.
fn xdelta3_applies(nds: &std::path::Path, patch: &std::path::Path, target_sha1: &str) -> bool {
    let out = patch.with_extension("xdelta3-cli.nds");
    let status = match std::process::Command::new("xdelta3")
        .arg("-d")
        .arg("-f")
        .arg("-s")
        .arg(nds)
        .arg(patch)
        .arg(&out)
        .status()
    {
        Ok(status) => status,
        Err(e) => {
            eprintln!("xdelta3 cross-check skipped: {e}");
            return false;
        }
    };
    assert!(status.success(), "xdelta3 -d failed: {status}");
    assert_eq!(inspect_rom(&out).unwrap().sha1, target_sha1);
    std::fs::remove_file(&out).unwrap();
    true
}

/// A patch after one map byte stays below 1 KiB, after map, script and text below 16 KiB;
/// both are written in under a second and apply with our decoder and with xdelta3.
#[test]
fn patch_after_map_and_after_map_script_text_is_small_and_applies() {
    use g4_kernel::patch::{build_and_create, default_targets};
    use g4_kernel::project::AssetKind;
    use g4_kernel::text::TextArchiveDoc;

    let Some((_orig, nds)) = vanilla_with_base() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let (_, mark) = assign_base(&copy, &nds).unwrap();
    let base = BaseRom::open(&nds, Some(&mark.sha1)).unwrap();
    let project = Project::open(&copy).unwrap();
    let targets = default_targets(&project);

    // Map: one permission byte of map 3, as Gutachten B `edit_map`.
    project.ensure_unpacked(AssetKind::Map).unwrap();
    let map = project.member_path(AssetKind::Map, 3);
    let mut bytes = std::fs::read(&map).unwrap();
    bytes[0x20] ^= 0x80;
    std::fs::write(&map, bytes).unwrap();
    let (build, map_report) = build_and_create(&project, &base, None).unwrap();
    assert_eq!(build.rom, targets.rom);
    assert_eq!(map_report.patch, targets.patch);
    assert_eq!(map_report.base_sha1, mark.sha1);
    assert_ne!(map_report.target_sha1, mark.sha1);
    assert_eq!(
        inspect_rom(&targets.rom).unwrap().sha1,
        map_report.target_sha1
    );
    assert!(map_report.notes.is_file());
    let crossed_map = xdelta3_applies(&nds, &map_report.patch, &map_report.target_sha1);

    // Text: message 0 of archive 0 gets " Studio", as Gutachten B `edit_text`.
    project.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let member = std::fs::read(project.member_path(AssetKind::TextArchive, 0)).unwrap();
    let mut doc = TextArchiveDoc::from_member(&member, project.language.locale_key()).unwrap();
    doc.messages[0].text.push_str(" Studio");
    let dir = project.expanded_dir(AssetKind::TextArchive);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0000.json"), doc.to_json()).unwrap();
    // Script: two bytes appended to script file 0.
    project.ensure_unpacked(AssetKind::Script).unwrap();
    let script = project.member_path(AssetKind::Script, 0);
    let mut bytes = std::fs::read(&script).unwrap();
    bytes.extend_from_slice(&[0x02, 0x00]);
    std::fs::write(&script, bytes).unwrap();
    let (_, all_report) = build_and_create(&project, &base, None).unwrap();
    assert_ne!(all_report.target_sha1, map_report.target_sha1);
    let crossed_all = xdelta3_applies(&nds, &all_report.patch, &all_report.target_sha1);

    eprintln!(
        "patch-sizes {} map={} all={} encode_ms={}/{} verify_ms={}/{}",
        base.game_code(),
        map_report.size,
        all_report.size,
        map_report.encode_ms,
        all_report.encode_ms,
        map_report.verify_ms,
        all_report.verify_ms
    );
    eprintln!("xdelta3 cross-check: map={crossed_map} all={crossed_all}");
    assert!(map_report.size <= 1024, "map patch {} B", map_report.size);
    assert!(
        all_report.size <= 16384,
        "map+script+text patch {} B",
        all_report.size
    );
    assert!(map_report.encode_ms <= 1000, "{} ms", map_report.encode_ms);
    assert!(all_report.encode_ms <= 1000, "{} ms", all_report.encode_ms);
}
