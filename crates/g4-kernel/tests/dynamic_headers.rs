#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_kernel::dynamic_headers::{self, PATCH_SIGNATURE};
use g4_kernel::project::{AssetKind, Location, Project};
use g4_kernel::store::Store;

/// The file offset a Thumb `bl` at file offset `site` calls (decoded here, independently of
/// the kernel).
fn thumb_bl(site: usize, bytes: &[u8]) -> Option<usize> {
    let high = u16::from_le_bytes([bytes[0], bytes[1]]);
    let low = u16::from_le_bytes([bytes[2], bytes[3]]);
    if high & 0xF800 != 0xF000 || low & 0xF800 != 0xF800 {
        return None;
    }
    // 22-bit halfword offset, sign-extended.
    let halfwords = ((i32::from(high & 0x7FF) << 21) >> 10) | i32::from(low & 0x7FF);
    usize::try_from(site as i64 + 4 + 2 * i64::from(halfwords)).ok()
}

/// The (load, free) functions the routine has to call, found in each game's own arm9.bin:
/// the load function is called from 15 places, the free function (`push {r3-r7,lr}`, so the
/// routine's r3 survives) from 642 (CPUD) and 662 (IPKD, IPKE). `None` for a game without a
/// test project.
fn verified_calls(game_code: &str) -> Option<[usize; 2]> {
    match game_code {
        "CPUD" => Some([0x6AD4, 0x18234]),
        "IPKD" | "IPKE" => Some([0x7524, 0x1AB0C]),
        _ => None,
    }
}

/// Applies the patch to a copy of the vanilla project (Platinum or HGSS, any language) and
/// checks every byte it should and should not have written.
#[test]
fn vanilla_project_takes_the_patch() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let Some(plan) = dynamic_headers::plan(&project).unwrap() else {
        eprintln!(
            "skipped: {} has no dynamic headers patch",
            project.game_code
        );
        return;
    };
    assert!(!dynamic_headers::is_applied(&project).unwrap());
    if matches!(project.game_code.as_str(), "IPKD" | "IPKE") {
        assert_eq!(plan.special_case, Some([0x3B522, 0x3B526, 0x3B53C]));
    }
    let arm9_path = root.join("arm9/arm9.bin");
    let before = std::fs::read(&arm9_path).unwrap();
    let count_before = project.header_listing().unwrap().headers.len();
    let Location::Slice { offset: table, .. } =
        project.asset_location(AssetKind::HeaderTable, 0).unwrap()
    else {
        panic!("the header table is a slice of arm9.bin before the patch");
    };
    let table = table as usize;

    let headers = Store::new(project.clone()).header_table().unwrap().to_vec();
    assert_eq!(headers.len(), count_before);
    dynamic_headers::apply(&project, &headers).unwrap();

    assert!(dynamic_headers::is_applied(&project).unwrap());
    assert_eq!(
        project.header_listing().unwrap().headers.len(),
        count_before
    );
    let after = std::fs::read(&arm9_path).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        std::fs::read(root.join("arm9/arm9.bin.backup")).unwrap(),
        before
    );

    let init = plan.init_offset as usize;
    assert_eq!(
        u16::from_le_bytes([after[init], after[init + 1]]),
        PATCH_SIGNATURE
    );
    assert_eq!(&after[init..init + 20], plan.init_bytes.as_slice());
    // Both calls of the routine reach the functions verified for this game.
    let calls = [
        thumb_bl(init + 8, &after[init + 8..init + 12]),
        thumb_bl(init + 14, &after[init + 14..init + 18]),
    ];
    if let Some(expected) = verified_calls(&project.game_code) {
        assert!(plan.verified, "{}", project.game_code);
        assert_eq!(calls, expected.map(Some), "{}", project.game_code);
    }
    let mut touched = vec![false; before.len()];
    touched[init..init + 20].fill(true);
    for &(mul, literal) in &plan.references {
        let (mul, literal) = (mul as usize, literal as usize);
        assert_eq!(&after[mul..mul + 4], [0x19, 0x00, 0xC0, 0x46], "{mul:#X}");
        let n = u32::from_le_bytes(after[literal..literal + 4].try_into().unwrap());
        assert!(n < 24, "literal at {literal:#X} is {n}");
        touched[mul..mul + 4].fill(true);
        touched[literal..literal + 4].fill(true);
    }
    if let Some([mov, mul, literal]) = plan.special_case {
        let (mov, mul, literal) = (mov as usize, mul as usize, literal as usize);
        assert_eq!(&after[mov..mov + 2], [0x19, 0x00]);
        assert_eq!(&after[mul..mul + 2], [0xC0, 0x46]);
        assert_eq!(&after[literal..literal + 4], [0; 4]);
        touched[mov..mov + 2].fill(true);
        touched[mul..mul + 2].fill(true);
        touched[literal..literal + 4].fill(true);
    }
    for (i, (a, b)) in before.iter().zip(&after).enumerate() {
        assert!(
            touched[i] || a == b,
            "byte {i:#X} changed outside the patch sites"
        );
    }

    // One file per header, each the 24 bytes the table held.
    let dir = dynamic_headers::unpacked_dir(&project);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), count_before);
    for i in 0..count_before {
        let file = std::fs::read(dir.join(format!("{i:04}"))).unwrap();
        assert_eq!(
            file,
            before[table + i * 24..table + (i + 1) * 24],
            "header {i}"
        );
    }

    // A second application is refused and changes nothing.
    assert!(dynamic_headers::apply(&project, &headers).is_err());
    assert_eq!(std::fs::read(&arm9_path).unwrap(), after);
}

/// The store applies the patch to a copy, adds a header, and the project still builds a ROM.
#[test]
fn store_applies_the_patch_creates_a_header_and_builds() {
    let Some((tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    if dynamic_headers::plan(&project).unwrap().is_none() {
        eprintln!(
            "skipped: {} has no dynamic headers patch",
            project.game_code
        );
        return;
    }
    let mut store = Store::new(project.clone());
    let count = project.header_listing().unwrap().headers.len();
    if !store.dynamic_headers_applied().unwrap() {
        store.apply_dynamic_headers().unwrap();
    }
    assert!(store.dynamic_headers_applied().unwrap());
    assert_eq!(store.header_table().unwrap().len(), count);
    assert!(store.dirty().is_empty());

    assert_eq!(store.create_header(0, "G4TEST").unwrap(), count as u16);
    assert_eq!(store.header_table().unwrap().len(), count + 1);
    assert!(store.dirty().is_empty());

    let reread = Project::open(&root).unwrap();
    let listing = reread.header_listing().unwrap();
    assert_eq!(listing.headers.len(), count + 1);
    assert_eq!(listing.headers[count].internal_name, "G4TEST");
    let dir = dynamic_headers::unpacked_dir(&reread);
    assert_eq!(
        std::fs::read(dir.join(format!("{count:04}"))).unwrap(),
        std::fs::read(dir.join("0000")).unwrap()
    );

    let rom = tmp.path().join("tmp.nds");
    g4_kernel::build::build(&reread, &rom).unwrap();
    assert!(rom.exists());
}

/// A copy patched with a routine whose free call misses (for Platinum DE exactly DSPRE's
/// bytes, which hang the game after the intro) is detected and repaired; the repaired
/// routine calls the verified functions.
#[test]
fn a_routine_with_a_wrong_free_call_is_detected_and_repaired() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let Some(plan) = dynamic_headers::plan(&project).unwrap() else {
        eprintln!(
            "skipped: {} has no dynamic headers patch",
            project.game_code
        );
        return;
    };
    let headers = Store::new(project.clone()).header_table().unwrap().to_vec();
    dynamic_headers::apply(&project, &headers).unwrap();
    assert!(!dynamic_headers::needs_repair(&project).unwrap());

    let arm9_path = root.join("arm9/arm9.bin");
    let init = plan.init_offset as usize;
    let mut arm9 = std::fs::read(&arm9_path).unwrap();
    let wrong: [u8; 4] = if project.game_code == "CPUD" {
        // DSPRE ToolboxDB.cs: the routine's second call for CPUS/CPUI/CPUF/CPUD.
        [0xCC, 0xF7, 0x74, 0xFC]
    } else {
        // One halfword further: a call into the middle of the free function.
        let mut call: [u8; 4] = arm9[init + 14..init + 18].try_into().unwrap();
        call[2] = call[2].wrapping_add(1);
        call
    };
    arm9[init + 14..init + 18].copy_from_slice(&wrong);
    std::fs::write(&arm9_path, &arm9).unwrap();
    assert!(dynamic_headers::is_applied(&project).unwrap());
    assert!(dynamic_headers::needs_repair(&project).unwrap());

    dynamic_headers::repair(&project).unwrap();
    let after = std::fs::read(&arm9_path).unwrap();
    assert_eq!(&after[init..init + 20], plan.init_bytes.as_slice());
    for (i, (a, b)) in arm9.iter().zip(&after).enumerate() {
        assert!((init + 14..init + 18).contains(&i) || a == b, "byte {i:#X}");
    }
    if let Some([_, free]) = verified_calls(&project.game_code) {
        assert_eq!(
            thumb_bl(init + 14, &after[init + 14..init + 18]),
            Some(free)
        );
    }
    assert!(!dynamic_headers::needs_repair(&project).unwrap());
}
