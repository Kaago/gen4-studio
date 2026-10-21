#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::map::Cell;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};

fn store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    p.ensure_unpacked(AssetKind::Matrix).unwrap();
    (s, Store::new(Arc::new(p)))
}

fn map_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Map,
        index,
    }
}

#[test]
fn edit_marks_dirty_and_undo_clears_it() {
    let (_s, mut st) = store();
    let id = AssetId {
        kind: AssetKind::Map,
        index: 1,
    };
    assert!(!st.is_dirty(id));
    st.edit_map(1, "block tile", |m| {
        m.set_cell(
            4,
            4,
            Cell {
                kind: 0,
                collision: 0x80,
            },
        )
        .unwrap()
    })
    .unwrap();
    assert!(st.is_dirty(id));
    assert_eq!(st.dirty(), vec![id]);
    assert!(st.undo().is_some());
    assert!(!st.is_dirty(id));
    assert!(st.redo().is_some());
    assert_eq!(st.map(1).unwrap().cell(4, 4).unwrap().collision, 0x80);
}

#[test]
fn save_writes_only_dirty_members() {
    let (s, mut st) = store();
    st.edit_map(2, "x", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 9,
                collision: 0,
            },
        )
        .unwrap()
    })
    .unwrap();
    let before0 = std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap();
    let report = st.save().unwrap();
    assert_eq!(
        report.written,
        vec![AssetId {
            kind: AssetKind::Map,
            index: 2
        }]
    );
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap(),
        before0
    );
    let saved = std::fs::read(s.dir.path().join("unpacked/maps/0002")).unwrap();
    assert_eq!(
        g4_formats::map::MapFile::read(&saved, g4_formats::game::Family::Pt)
            .unwrap()
            .cell(0, 0)
            .unwrap()
            .kind,
        9
    );
    assert!(st.dirty().is_empty());
}

#[test]
fn save_refuses_when_disk_changed_since_load() {
    let (s, mut st) = store();
    st.edit_map(0, "x", |m| {
        m.set_cell(
            1,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
        .unwrap()
    })
    .unwrap();
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut foreign = std::fs::read(&path).unwrap();
    foreign[16] ^= 0xFF;
    std::fs::write(&path, &foreign).unwrap();
    let err = st.save().unwrap_err();
    assert!(matches!(
        err,
        KernelError::Stale(AssetId {
            kind: AssetKind::Map,
            index: 0
        })
    ));
    assert_eq!(std::fs::read(&path).unwrap(), foreign);
}

#[test]
fn stale_member_blocks_the_whole_save() {
    let (s, mut st) = store();
    let cell = Cell {
        kind: 3,
        collision: 0,
    };
    st.edit_map(0, "a", |m| m.set_cell(0, 0, cell).unwrap())
        .unwrap();
    st.edit_map(1, "b", |m| m.set_cell(0, 0, cell).unwrap())
        .unwrap();
    let stale = s.dir.path().join("unpacked/maps/0001");
    let mut foreign = std::fs::read(&stale).unwrap();
    foreign[17] ^= 0xFF;
    std::fs::write(&stale, &foreign).unwrap();
    let untouched = std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap();

    assert!(matches!(
        st.save().unwrap_err(),
        KernelError::Stale(id) if id == map_id(1)
    ));
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap(),
        untouched
    );
    assert_eq!(std::fs::read(&stale).unwrap(), foreign);
    assert_eq!(st.dirty(), vec![map_id(0), map_id(1)]);
}

#[test]
fn matrix_edit_round_trips_through_save() {
    let (s, mut st) = store();
    st.edit_matrix(0, "assign", |m| m.maps[1] = 2).unwrap();
    st.save().unwrap();
    let bytes = std::fs::read(s.dir.path().join("unpacked/matrices/0000")).unwrap();
    assert_eq!(
        g4_formats::matrix::Matrix::read(&bytes).unwrap().maps,
        vec![0, 2]
    );
}

#[test]
fn invalid_edit_is_rolled_back() {
    let (_s, mut st) = store();
    let id = map_id(1);
    let before = st.map(1).unwrap().clone();
    let err = st
        .edit_map(1, "break", |m| m.permissions.truncate(10))
        .unwrap_err();
    assert!(matches!(err, KernelError::Format(_)));
    assert!(!st.is_dirty(id));
    assert_eq!(*st.map(1).unwrap(), before);
    assert!(st.undo().is_none());
}

#[test]
fn invalid_matrix_edit_is_rolled_back() {
    let (_s, mut st) = store();
    let before = st.matrix(0).unwrap().clone();
    let err = st.edit_matrix(0, "break", |m| {
        m.maps.pop();
    });
    assert!(matches!(err, Err(KernelError::Format(_))));
    assert_eq!(*st.matrix(0).unwrap(), before);
    assert!(st.dirty().is_empty());
    assert!(st.undo().is_none());
}

#[test]
fn edit_that_changes_nothing_records_no_changeset() {
    let (_s, mut st) = store();
    st.edit_map(1, "noop", |_| {}).unwrap();
    let same = st.map(1).unwrap().cell(2, 2).unwrap();
    st.edit_map(1, "same cell", |m| m.set_cell(2, 2, same).unwrap())
        .unwrap();
    assert!(!st.is_dirty(map_id(1)));
    assert!(st.undo().is_none());
}

#[test]
fn new_edit_after_undo_clears_redo() {
    let (_s, mut st) = store();
    let a = Cell {
        kind: 1,
        collision: 0,
    };
    let b = Cell {
        kind: 2,
        collision: 0,
    };
    st.edit_map(1, "a", |m| m.set_cell(0, 0, a).unwrap())
        .unwrap();
    assert!(st.undo().is_some());
    st.edit_map(1, "b", |m| m.set_cell(0, 0, b).unwrap())
        .unwrap();
    assert!(st.redo().is_none());
    assert_eq!(st.map(1).unwrap().cell(0, 0).unwrap(), b);
}

#[test]
fn undo_and_redo_walk_the_history_in_order() {
    let (_s, mut st) = store();
    let c1 = st
        .edit_map(0, "one", |m| {
            m.set_cell(
                0,
                0,
                Cell {
                    kind: 1,
                    collision: 0,
                },
            )
            .unwrap()
        })
        .unwrap();
    let c2 = st.edit_matrix(0, "two", |m| m.maps[1] = 2).unwrap();
    assert_ne!(c1, c2);
    assert_eq!(st.undo(), Some(c2));
    assert_eq!(st.undo(), Some(c1));
    assert_eq!(st.undo(), None);
    assert_eq!(st.redo(), Some(c1));
    assert_eq!(st.redo(), Some(c2));
    assert_eq!(st.redo(), None);
    assert_eq!(
        st.dirty(),
        vec![
            map_id(0),
            AssetId {
                kind: AssetKind::Matrix,
                index: 0
            }
        ]
    );
}

#[test]
fn undo_after_save_makes_the_asset_dirty_again() {
    let (_s, mut st) = store();
    st.edit_map(1, "a", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 5,
                collision: 0,
            },
        )
        .unwrap()
    })
    .unwrap();
    st.save().unwrap();
    assert!(!st.is_dirty(map_id(1)));
    assert!(st.undo().is_some());
    assert!(st.is_dirty(map_id(1)));
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![map_id(1)]);
    assert!(st.dirty().is_empty());
}

#[test]
fn vanilla_edit_save_round_trip_on_copy() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let family = project.family;
    let path = project.member_path(AssetKind::Map, 0);
    let original = std::fs::read(&path).unwrap();
    let old = g4_formats::map::MapFile::read(&original, family)
        .unwrap()
        .cell(3, 5)
        .unwrap();
    let new = Cell {
        kind: old.kind ^ 1,
        collision: old.collision,
    };

    let mut st = Store::new(Arc::new(project));
    st.edit_map(0, "flip", |m| m.set_cell(3, 5, new).unwrap())
        .unwrap();
    assert_eq!(st.save().unwrap().written, vec![map_id(0)]);

    let project = Project::open(&root).unwrap();
    let mut fresh = Store::new(Arc::new(project));
    assert_eq!(fresh.map(0).unwrap().cell(3, 5).unwrap(), new);
    assert!(fresh.dirty().is_empty());

    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved.len(), original.len());
    let differing = saved.iter().zip(&original).filter(|(a, b)| a != b).count();
    assert_eq!(differing, 1, "exactly the edited cell byte changes");
}
