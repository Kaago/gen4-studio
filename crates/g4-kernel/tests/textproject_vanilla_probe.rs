#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): measurements of the text project on a vanilla DSPRE folder,
//! printed for the gate document.
mod common;

use g4_kernel::project::{AssetKind, Project};
use g4_kernel::textproject::item_owners;

/// How the item ids of the ARM9 item table map onto the `itemData` members: `shared` ids
/// write `same_member_as` (their member is carried by another id), `unreferenced` members
/// have no id (and no item file).
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
    let shared = member_of
        .iter()
        .enumerate()
        .filter(|&(id, &m)| {
            owners
                .get(usize::from(m))
                .copied()
                .flatten()
                .is_some_and(|o| o as usize != id)
        })
        .count();
    let unreferenced = owners.iter().filter(|o| o.is_none()).count();
    eprintln!(
        "items {} ids={ids} members={members} shared={shared} unreferenced={unreferenced}",
        project.game_code
    );
    assert!(ids >= members, "ids={ids} members={members}");
}

/// Files and bytes below `dir`, recursively.
fn tree_size(dir: &std::path::Path) -> (u64, u64) {
    let (mut files, mut bytes) = (0, 0);
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        if ty.is_dir() {
            let (f, b) = tree_size(&entry.path());
            files += f;
            bytes += b;
        } else if ty.is_file() {
            files += 1;
            bytes += entry.metadata().unwrap().len();
        }
    }
    (files, bytes)
}

/// A vanilla DSPRE folder outside its modelled members equals the extracted base ROM with
/// every archive unpacked: no blob under `data/raw/`.
#[test]
fn vanilla_base_layer_has_no_raw_blobs() {
    use g4_kernel::base::BaseRom;
    use g4_kernel::start::suggest_base;
    use g4_kernel::textproject::base_layer::{BaseLayer, raw_diff};

    let Some(orig) = common::vanilla_project() else {
        return;
    };
    let Some(nds) = suggest_base(&orig) else {
        eprintln!("skipped: no base ROM next to {}", orig.display());
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&copy).unwrap();
    let base = BaseRom::open(&nds, None).unwrap();

    let start = std::time::Instant::now();
    let (layer, created) = BaseLayer::ensure(&copy, &base).unwrap();
    let ms = start.elapsed().as_millis();
    assert!(created);
    let (files, bytes) = tree_size(&layer.root);
    let (again, created_again) = BaseLayer::ensure(&copy, &base).unwrap();
    assert!(!created_again);
    assert_eq!(again.root, layer.root);

    let start = std::time::Instant::now();
    let diff = raw_diff(&project, &layer).unwrap();
    let diff_ms = start.elapsed().as_millis();
    eprintln!(
        "base-layer {} ms={ms} diff_ms={diff_ms} files={files} bytes={bytes} not_built={}",
        project.game_code,
        diff.not_built.join(",")
    );
    for path in &diff.changed {
        eprintln!("changed {}", path.display());
    }
    assert!(diff.changed.is_empty(), "{} blobs", diff.changed.len());
}
