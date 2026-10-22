#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::error::FormatError;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind, Location, Project};
use g4_kernel::store::{MapHeader, MapHeaderDP, Store};

const OFF: usize = 0xE601C;
const HEADER: usize = 24;

fn table_id() -> AssetId {
    AssetId {
        kind: AssetKind::HeaderTable,
        index: 0,
    }
}

fn synthetic_store(s: &common::Synthetic) -> Store {
    Store::new(Arc::new(Project::open(s.dir.path()).unwrap()))
}

fn arm9_path(s: &common::Synthetic) -> std::path::PathBuf {
    s.dir.path().join("arm9/arm9.bin")
}

#[test]
fn header_table_save_touches_only_its_slice() {
    let s = common::synthetic_project();
    let p = Arc::new(Project::open(s.dir.path()).unwrap());
    let arm9_path = s.dir.path().join("arm9/arm9.bin");
    let before = std::fs::read(&arm9_path).unwrap();
    let mut st = Store::new(p.clone());
    assert_eq!(st.header_table().unwrap().len(), 2);
    st.edit_header(1, "matrix", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 7
        }
        Ok(())
    })
    .unwrap();
    assert!(st.is_dirty(AssetId {
        kind: AssetKind::HeaderTable,
        index: 0
    }));
    st.save().unwrap();
    let after = std::fs::read(&arm9_path).unwrap();
    assert_eq!(before.len(), after.len());
    let off = 0xE601C;
    assert_eq!(before[..off], after[..off]);
    assert_eq!(before[off + 48..], after[off + 48..]);
    assert_eq!(&after[off + 24 + 2..off + 24 + 4], &7u16.to_le_bytes());
    assert_eq!(
        Project::open(s.dir.path()).unwrap().headers().unwrap()[1].matrix_id,
        7
    );
}

#[test]
fn header_edit_out_of_range_is_an_error() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert!(matches!(
        st.edit_header(2, "x", |_| Ok(())).unwrap_err(),
        KernelError::Format(FormatError::OutOfRange { .. })
    ));
}

#[test]
fn header_table_kind_is_store_only() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    let kind = AssetKind::HeaderTable;
    assert_eq!(kind.to_string(), "header table");
    assert!(!p.has_archive(kind));
    assert!(p.archives.packed(kind).is_none());
    assert!(!p.archives.kinds().any(|k| k == kind));
    assert!(AssetKind::ALL.contains(&kind));
    assert_eq!(p.member_count(kind).unwrap(), 1);
    assert_eq!(p.ensure_unpacked(kind).unwrap(), 1);
    // Nothing was unpacked or created for it.
    assert!(!s.dir.path().join("unpacked").exists());
}

#[test]
fn asset_location_of_the_header_table_is_its_arm9_slice() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.header_count().unwrap(), 2);
    assert_eq!(
        p.asset_location(AssetKind::HeaderTable, 0).unwrap(),
        Location::Slice {
            path: s.dir.path().join("arm9/arm9.bin"),
            offset: OFF as u64,
            len: 2 * HEADER,
        }
    );
    assert!(matches!(
        p.asset_location(AssetKind::HeaderTable, 1).unwrap_err(),
        KernelError::InvalidProject(_)
    ));
    assert_eq!(
        p.asset_location(AssetKind::Map, 3).unwrap(),
        Location::Member(p.member_path(AssetKind::Map, 3))
    );
}

#[test]
fn asset_location_rejects_an_arm9_too_short_for_the_table() {
    let s = common::synthetic_project();
    std::fs::write(arm9_path(&s), vec![0u8; OFF + HEADER]).unwrap();
    let p = Project::open(s.dir.path()).unwrap();
    assert!(matches!(
        p.asset_location(AssetKind::HeaderTable, 0).unwrap_err(),
        KernelError::InvalidProject(_)
    ));
    assert!(p.header_count().is_err());
}

#[test]
fn header_edits_undo_redo_and_an_unchanged_table_is_clean() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    assert!(st.dirty().is_empty());
    let before = st.header_table().unwrap().to_vec();
    assert!(st.dirty().is_empty());
    // A no-op edit leaves the table clean.
    st.edit_header(0, "noop", |_| Ok(())).unwrap();
    assert!(st.dirty().is_empty());
    st.edit_header(0, "music", |h| {
        if let MapHeader::Pt(h) = h {
            h.music_day_id = 1234;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(st.dirty(), vec![table_id()]);
    assert_ne!(st.header_table().unwrap(), before);
    assert!(st.undo().is_some());
    assert!(st.dirty().is_empty());
    assert_eq!(st.header_table().unwrap(), before);
    assert!(st.redo().is_some());
    assert_eq!(st.dirty(), vec![table_id()]);
    let MapHeader::Pt(h) = &st.header_table().unwrap()[0] else {
        panic!("platinum project")
    };
    assert_eq!(h.music_day_id, 1234);
}

#[test]
fn a_failing_header_edit_rolls_back() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    let before = st.header_table().unwrap().to_vec();
    let err = st
        .edit_header(0, "bad", |h| {
            if let MapHeader::Pt(h) = h {
                h.matrix_id = 99;
            }
            Err(FormatError::OutOfRange {
                what: "x",
                value: 1,
                max: 0,
            })
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Format(_)));
    assert_eq!(st.header_table().unwrap(), before);
    assert!(st.dirty().is_empty());
}

#[test]
fn header_edits_that_the_game_cannot_store_are_rejected() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    let before = st.header_table().unwrap().to_vec();
    // The Platinum `flags` field has 4 bits.
    let err = st
        .edit_header(0, "flags", |h| {
            if let MapHeader::Pt(h) = h {
                h.flags = 16;
            }
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Uxie(_)), "{err:?}");
    // A header of another game family has another layout.
    let err = st
        .edit_header(1, "family", |h| {
            *h = MapHeader::DP(MapHeaderDP::default());
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Uxie(_)), "{err:?}");
    assert_eq!(st.header_table().unwrap(), before);
    assert!(st.dirty().is_empty());
    assert!(st.undo().is_none());
}

#[test]
fn header_fits_is_the_dry_run_of_the_edit_check() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    let mut header = st.header_table().unwrap()[0].clone();
    assert!(st.header_fits(&header));
    if let MapHeader::Pt(h) = &mut header {
        h.flags = 16;
    }
    assert!(!st.header_fits(&header));
    assert!(!st.header_fits(&MapHeader::DP(MapHeaderDP::default())));
}

#[test]
fn save_keeps_changes_made_to_the_rest_of_arm9_since_loading() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    st.edit_header(0, "x", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 3;
        }
        Ok(())
    })
    .unwrap();
    // Another tool patches code in arm9 before and after the table, after the store loaded.
    let mut theirs = std::fs::read(arm9_path(&s)).unwrap();
    theirs[0x100] = 0xAA;
    let last = theirs.len() - 1;
    theirs[last] = 0xBB;
    std::fs::write(arm9_path(&s), &theirs).unwrap();
    st.save().unwrap();
    let after = std::fs::read(arm9_path(&s)).unwrap();
    assert_eq!(after[0x100], 0xAA);
    assert_eq!(after[last], 0xBB);
    assert_eq!(&after[OFF + 2..OFF + 4], &3u16.to_le_bytes());
    assert!(st.dirty().is_empty());
}

#[test]
fn a_changed_slice_is_stale_and_nothing_is_written() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    st.edit_header(0, "mine", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 3;
        }
        Ok(())
    })
    .unwrap();
    let mut theirs = std::fs::read(arm9_path(&s)).unwrap();
    theirs[OFF + HEADER + 4] = 0x55; // header 1, script_file_id
    std::fs::write(arm9_path(&s), &theirs).unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(i) if i == table_id()));
    assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), theirs);
    assert!(
        !s.dir.path().join(".g4/tmp").exists() || {
            std::fs::read_dir(s.dir.path().join(".g4/tmp"))
                .unwrap()
                .count()
                == 0
        }
    );

    // Keep mine: my table replaces theirs, including their header 1 change.
    st.accept_disk_as_base(table_id()).unwrap();
    assert!(st.is_dirty(table_id()));
    st.save().unwrap();
    let after = std::fs::read(arm9_path(&s)).unwrap();
    assert_eq!(&after[OFF + 2..OFF + 4], &3u16.to_le_bytes());
    // My table replaced theirs: their change to header 1 is gone.
    assert_eq!(after[OFF + HEADER + 4], 0);
}

#[test]
fn reload_from_disk_takes_the_slice_on_disk_and_drops_my_changes() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    st.edit_header(0, "mine", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 3;
        }
        Ok(())
    })
    .unwrap();
    let mut theirs = std::fs::read(arm9_path(&s)).unwrap();
    theirs[OFF + HEADER + 2] = 9; // header 1, matrix_id
    std::fs::write(arm9_path(&s), &theirs).unwrap();
    assert!(st.save().is_err());
    st.reload_from_disk(table_id()).unwrap();
    assert!(st.dirty().is_empty());
    assert!(st.undo().is_none());
    assert_eq!(st.header_table().unwrap()[0].matrix_id(), 0);
    assert_eq!(st.header_table().unwrap()[1].matrix_id(), 9);
    assert_eq!(std::fs::read(arm9_path(&s)).unwrap(), theirs);
}

#[test]
fn a_missing_arm9_is_an_error_for_save_reload_and_accept() {
    let s = common::synthetic_project();
    let mut st = synthetic_store(&s);
    st.edit_header(0, "mine", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 3;
        }
        Ok(())
    })
    .unwrap();
    std::fs::remove_file(arm9_path(&s)).unwrap();
    assert!(st.save().is_err());
    assert!(st.reload_from_disk(table_id()).is_err());
    assert!(st.accept_disk_as_base(table_id()).is_err());
    assert!(!arm9_path(&s).exists());
    assert!(st.is_dirty(table_id()));
}

/// Toggles a field every game family has, so the same edit works on every vanilla project.
fn bump(h: &mut MapHeader) {
    match h {
        MapHeader::DP(h) => h.music_day_id ^= 1,
        MapHeader::Pt(h) => h.music_day_id ^= 1,
        MapHeader::HGSS(h) => h.music_day_id ^= 1,
    }
}

#[test]
fn vanilla_header_table_round_trips_byte_identical() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let arm9 = root.join("arm9/arm9.bin");
    let original = std::fs::read(&arm9).unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    let Location::Slice { offset, len, .. } =
        project.asset_location(AssetKind::HeaderTable, 0).unwrap()
    else {
        panic!("header table is a slice");
    };
    let slice = offset as usize..offset as usize + len;
    let mut st = Store::new(project.clone());
    let n = st.header_table().unwrap().len();
    assert!(n > 500);
    assert_eq!(len, n * HEADER);
    assert_eq!(project.header_count().unwrap(), n);
    assert!(st.dirty().is_empty());
    // Serializing every header reproduces the vanilla bytes exactly (else the table is locked).
    assert_eq!(st.locked(table_id()), None);

    // No-op edits change nothing on disk.
    for id in [0u16, 1, (n / 2) as u16, (n - 1) as u16] {
        st.edit_header(id, "noop", |_| Ok(())).unwrap();
    }
    assert!(st.dirty().is_empty());
    assert!(st.save().unwrap().written.is_empty());
    assert_eq!(std::fs::read(&arm9).unwrap(), original);

    // A real edit changes exactly its header; undoing it and saving restores vanilla.
    let target = (n - 1) as u16;
    let vanilla = st.header_table().unwrap()[usize::from(target)].clone();
    st.edit_header(target, "bump", |h| {
        bump(h);
        Ok(())
    })
    .unwrap();
    assert_eq!(st.save().unwrap().written, vec![table_id()]);
    let edited = std::fs::read(&arm9).unwrap();
    assert_eq!(edited.len(), original.len());
    let start = slice.start + usize::from(target) * HEADER;
    let diffs: Vec<usize> = (0..original.len())
        .filter(|&i| original[i] != edited[i])
        .collect();
    assert!(!diffs.is_empty());
    assert!(diffs.iter().all(|&i| (start..start + HEADER).contains(&i)));
    // A fresh store (and the project's own header reader) see the edit.
    let mut again = Store::new(Arc::new(Project::open(&root).unwrap()));
    assert_ne!(again.header_table().unwrap()[usize::from(target)], vanilla);
    assert_eq!(
        again.header_table().unwrap()[usize::from(target)],
        st.header_table().unwrap()[usize::from(target)]
    );
    assert_eq!(
        Project::open(&root).unwrap().headers().unwrap()[usize::from(target)].event_file_id,
        st.header_table().unwrap()[usize::from(target)].event_file_id()
    );
    assert!(st.undo().is_some());
    assert!(st.is_dirty(table_id()));
    st.save().unwrap();
    assert_eq!(std::fs::read(&arm9).unwrap(), original);
}
