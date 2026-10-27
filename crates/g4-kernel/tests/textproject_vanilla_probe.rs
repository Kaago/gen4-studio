#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): measurements of the text project on a vanilla DSPRE folder,
//! printed for the gate document.
mod common;

use g4_kernel::project::{AssetKind, Project};
use g4_kernel::textproject::item_owners;

/// How the item ids of the ARM9 item table map onto the `itemData` members: `shared` ids
/// write `same_member_as`, `unreferenced` members have no id (and no item file).
#[test]
fn vanilla_shared_item_members_are_counted() {
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&copy).unwrap();
    let Some(table) = project.item_table() else {
        eprintln!("skipped: no verified item table for {}", project.game_code);
        return;
    };
    let member_of: Vec<u16> = table.entries.iter().map(|e| e.data).collect();
    let members = project.member_count(AssetKind::Item).unwrap();
    let owners = item_owners(&member_of, members);
    let ids = member_of.len();
    let carriers = owners.iter().filter(|o| o.is_some()).count();
    let shared = ids - carriers;
    let unreferenced = owners.iter().filter(|o| o.is_none()).count();
    eprintln!(
        "items {} ids={ids} members={members} shared={shared} unreferenced={unreferenced}",
        project.game_code
    );
    assert!(ids >= members, "ids={ids} members={members}");
}
