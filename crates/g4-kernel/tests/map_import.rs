//! Gated map part import tests against a copy of the vanilla project in
//! `G4_TEST_PROJECT` (skipped when it is not set). Run once per test project.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::game::Family;
use g4_formats::map::{BLANK_BGS, MapFile};
use g4_kernel::map_import::{self, Part, PartState, Source};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};

fn map_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Map,
        index,
    }
}

fn open_copy() -> Option<(tempfile::TempDir, tempfile::TempDir, Store)> {
    let (tmp, root) = common::vanilla_copy()?;
    let project = Project::open(&root).unwrap();
    assert_ne!(project.family, Family::Dp, "Diamond/Pearl is not supported");
    Some((
        tmp,
        tempfile::tempdir().unwrap(),
        Store::new(Arc::new(project)),
    ))
}

fn export(store: &mut Store, dir: &Path, map: u16, part: Part, ext: &str) -> PathBuf {
    let path = dir.join(format!("{map:04}.{ext}"));
    map_import::export(store, map, part, &path).unwrap();
    path
}

/// Current bytes of map `index`, loading it first.
fn bytes_of(store: &mut Store, index: u16) -> Vec<u8> {
    store.map(index).unwrap();
    store.current_bytes(map_id(index)).unwrap().unwrap()
}

#[test]
fn every_vanilla_model_reimported_into_itself_is_byte_identical() {
    let Some((_tmp, files, mut store)) = open_copy() else {
        return;
    };
    let count = store.member_count(AssetKind::Map).unwrap();
    let mut unchecked = 0;
    for index in 0..count as u16 {
        let path = export(&mut store, files.path(), index, Part::Model, "nsbmd");
        let source = Source::Files(vec![path]);
        let report = map_import::inspect(&mut store, index, &source).unwrap();
        assert_eq!(report.len(), 1);
        assert!(
            matches!(report[0].state, PartState::Ok),
            "map {index}: {:?}",
            report[0].state
        );
        // The replaced model is the reference: no name is newly missing.
        assert!(
            !report[0].notes.iter().any(|n| n.contains("missing in map")),
            "map {index}: {:?}",
            report[0].notes
        );
        unchecked += usize::from(report[0].notes.iter().any(|n| n.contains("not checked")));
        let changed =
            map_import::import(&mut store, index, &source, &[Part::Model], false).unwrap();
        assert!(changed.is_empty(), "map {index} changed");
        assert_eq!(
            store.current_bytes(map_id(index)).unwrap().as_deref(),
            store.saved_bytes(map_id(index)),
            "map {index}"
        );
    }
    eprintln!("{count} maps reimported, texture names not checked on {unchecked}");
    assert!(store.dirty().is_empty());
    assert!(store.history().0.is_empty());
}

#[test]
fn map_a_into_b_and_back_restores_b() {
    let Some((_tmp, files, mut store)) = open_copy() else {
        return;
    };
    let (a, b) = (5u16, 6u16);
    let original_b = bytes_of(&mut store, b);
    let hgss = store.project().family == Family::Hgss;
    let mut parts = vec![Part::Model, Part::Permissions, Part::Buildings, Part::Bdhc];
    let mut saved = vec![
        export(&mut store, files.path(), b, Part::Model, "nsbmd"),
        export(&mut store, files.path(), b, Part::Permissions, "per"),
        export(&mut store, files.path(), b, Part::Buildings, "bld"),
        export(&mut store, files.path(), b, Part::Bdhc, "bdhc"),
    ];
    if hgss {
        parts.push(Part::Bgs);
        saved.push(export(&mut store, files.path(), b, Part::Bgs, "bgs"));
    }

    let report = map_import::inspect(&mut store, b, &Source::Map(a)).unwrap();
    for r in &report {
        assert!(!matches!(r.state, PartState::Error(_)), "{r:?}");
    }
    let changed = map_import::import(&mut store, b, &Source::Map(a), &parts, false).unwrap();
    assert_eq!(changed, vec![map_id(b)]);
    let (ma, mb): (MapFile, MapFile) =
        (store.map(a).unwrap().clone(), store.map(b).unwrap().clone());
    assert_eq!(mb.model, ma.model);
    assert_eq!(mb.permissions, ma.permissions);
    assert_eq!(mb.buildings, ma.buildings);
    assert_eq!(mb.bdhc, ma.bdhc);
    assert_eq!(mb.bgs, ma.bgs);
    assert_eq!(store.history().0.len(), 1);

    map_import::import(&mut store, b, &Source::Files(saved), &parts, false).unwrap();
    assert_eq!(bytes_of(&mut store, b), original_b);
    assert!(!store.is_dirty(map_id(b)));
    // Both steps undo back to the same bytes.
    store.undo().unwrap();
    store.undo().unwrap();
    assert_eq!(bytes_of(&mut store, b), original_b);
}

/// A whole map file of the other family made from map `index`: HGSS gets a background sound
/// block with content, Platinum none.
fn foreign_bin(store: &mut Store, index: u16, family: Family) -> (MapFile, Vec<u8>) {
    let mut m = store.map(index).unwrap().clone();
    if family == Family::Pt {
        m.bgs = Some(vec![0x34, 0x12, 0x04, 0x00, 1, 2, 3, 4]);
        let bytes = m.write(Family::Hgss);
        (m, bytes)
    } else {
        m.bgs = None;
        let bytes = m.write(Family::Pt);
        (m, bytes)
    }
}

#[test]
fn hgss_bin_into_pt_and_back() {
    let Some((_tmp, files, mut store)) = open_copy() else {
        return;
    };
    let family = store.project().family;
    let (a, b) = (5u16, 6u16);
    let original_b = bytes_of(&mut store, b);
    let back = export(&mut store, files.path(), b, Part::Whole, "bin");
    let (src, bytes) = foreign_bin(&mut store, a, family);
    let foreign = files.path().join("foreign.bin");
    std::fs::write(&foreign, bytes).unwrap();
    let source = Source::Files(vec![foreign]);

    let report = map_import::inspect(&mut store, b, &source).unwrap();
    match (&report[0].state, family) {
        (PartState::Warning(w), Family::Pt) => assert!(w.contains("background sound"), "{w}"),
        (PartState::Ok, Family::Hgss) => {}
        (s, _) => panic!("{s:?}"),
    }
    let buildings_b = store.map(b).unwrap().buildings.clone();
    map_import::import(&mut store, b, &source, &[Part::Whole], false).unwrap();
    let m = store.map(b).unwrap().clone();
    assert_eq!(m.model, src.model);
    assert_eq!(m.permissions, src.permissions);
    assert_eq!(m.bdhc, src.bdhc);
    assert_eq!(m.buildings, buildings_b, "foreign buildings are left out");
    match family {
        Family::Pt => assert_eq!(m.bgs, None),
        _ => assert_eq!(m.bgs.as_deref(), Some(BLANK_BGS.as_slice())),
    }

    map_import::import(
        &mut store,
        b,
        &Source::Files(vec![back]),
        &[Part::Whole],
        false,
    )
    .unwrap();
    assert_eq!(bytes_of(&mut store, b), original_b);
    assert!(!store.is_dirty(map_id(b)));
}

#[test]
fn per_round_trip() {
    let Some((_tmp, files, mut store)) = open_copy() else {
        return;
    };
    let (a, b) = (5u16, 6u16);
    let original_b = bytes_of(&mut store, b);
    let per_a = export(&mut store, files.path(), a, Part::Permissions, "per");
    let per_b = export(&mut store, files.path(), b, Part::Permissions, "per");
    assert_eq!(std::fs::read(&per_a).unwrap().len(), 2048);
    map_import::import(
        &mut store,
        b,
        &Source::Files(vec![per_a.clone()]),
        &[Part::Permissions],
        false,
    )
    .unwrap();
    let again = files.path().join("again.per");
    map_import::export(&mut store, b, Part::Permissions, &again).unwrap();
    assert_eq!(
        std::fs::read(&again).unwrap(),
        std::fs::read(&per_a).unwrap()
    );
    map_import::import(
        &mut store,
        b,
        &Source::Files(vec![per_b]),
        &[Part::Permissions],
        false,
    )
    .unwrap();
    assert_eq!(bytes_of(&mut store, b), original_b);
}
