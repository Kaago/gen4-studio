#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_kernel::dynamic_headers::{self, PATCH_SIGNATURE};
use g4_kernel::project::{AssetKind, Location, Project};
use g4_kernel::store::Store;

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
