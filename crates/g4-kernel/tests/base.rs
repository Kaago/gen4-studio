#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): assigning the DSPRE folder's own `.nds` as its base ROM and
//! rebuilding it byte for byte.
mod common;

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
