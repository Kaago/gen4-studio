#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::area_data::AreaData;
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::MATRIX_EMPTY;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store, WarpEvent};

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

fn matrix_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Matrix,
        index,
    }
}

#[test]
fn undo_reports_the_asset_it_restored() {
    let (_s, mut st) = store();
    st.edit_map(1, "a", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    st.edit_matrix(0, "b", |m| {
        m.maps[0] = 2;
        Ok(())
    })
    .unwrap();
    assert_eq!(st.undo(), Some(matrix_id(0)));
    assert_eq!(st.undo(), Some(map_id(1)));
    assert_eq!(st.undo(), None);
    assert_eq!(st.redo(), Some(map_id(1)));
    assert_eq!(st.redo(), Some(matrix_id(0)));
    assert_eq!(st.redo(), None);
}

#[test]
fn accept_disk_as_base_drops_only_the_history_of_that_asset() {
    let (s, mut st) = store();
    let cell = |kind| Cell { kind, collision: 0 };
    st.edit_map(0, "map", |m| m.set_cell(0, 0, cell(1)))
        .unwrap();
    st.edit_map(0, "map again", |m| m.set_cell(0, 1, cell(1)))
        .unwrap();
    st.edit_matrix(0, "matrix", |m| {
        m.maps[1] = 2;
        Ok(())
    })
    .unwrap();
    st.edit_map(0, "map third", |m| m.set_cell(0, 2, cell(2)))
        .unwrap();
    assert_eq!(st.undo(), Some(map_id(0))); // one map changeset on the redo stack
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut theirs = std::fs::read(&path).unwrap();
    theirs[16 + 2 * (6 * 32 + 5) + 1] = 0x80;
    std::fs::write(&path, &theirs).unwrap();
    st.accept_disk_as_base(map_id(0)).unwrap();
    assert!(st.redo().is_none(), "the map's redo entry is gone");
    assert_eq!(
        st.undo(),
        Some(matrix_id(0)),
        "other assets keep their history"
    );
    assert!(st.undo().is_none(), "the map's undo entries are gone");
    // The store keeps my version of the map and saving overwrites the external edit.
    assert!(st.is_dirty(map_id(0)));
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

fn dot_entries(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('.'))
        .collect()
}

fn edit_cell(st: &mut Store, index: u16) {
    st.edit_map(index, "x", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 9,
                collision: 0,
            },
        )
    })
    .unwrap();
}

#[test]
fn save_leaves_no_temp_file_and_no_dot_file_in_the_member_folder() {
    let (s, mut st) = store();
    edit_cell(&mut st, 2);
    st.save().unwrap();
    assert!(dot_entries(&s.dir.path().join("unpacked/maps")).is_empty());
    let tmp = s.dir.path().join(".g4/tmp");
    assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
}

#[test]
fn failed_save_leaves_nothing_behind_in_the_member_folder() {
    let (s, mut st) = store();
    edit_cell(&mut st, 2);
    let before = std::fs::read(s.dir.path().join("unpacked/maps/0002")).unwrap();
    // `.g4/tmp` is a file, so no temp file can be created there.
    std::fs::create_dir_all(s.dir.path().join(".g4")).unwrap();
    std::fs::write(s.dir.path().join(".g4/tmp"), b"not a directory").unwrap();
    assert!(st.save().is_err());
    assert!(dot_entries(&s.dir.path().join("unpacked/maps")).is_empty());
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/maps/0002")).unwrap(),
        before
    );
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
    st.edit_map(0, "a", |m| m.set_cell(0, 0, cell)).unwrap();
    st.edit_map(1, "b", |m| m.set_cell(0, 0, cell)).unwrap();
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
    st.edit_matrix(0, "assign", |m| {
        m.maps[1] = 2;
        Ok(())
    })
    .unwrap();
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
        .edit_map(1, "break", |m| {
            m.permissions.truncate(10);
            Ok(())
        })
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
        Ok(())
    });
    assert!(matches!(err, Err(KernelError::Format(_))));
    assert_eq!(*st.matrix(0).unwrap(), before);
    assert!(st.dirty().is_empty());
    assert!(st.undo().is_none());
}

#[test]
fn edit_that_changes_nothing_records_no_changeset() {
    let (_s, mut st) = store();
    st.edit_map(1, "noop", |_| Ok(())).unwrap();
    let same = st.map(1).unwrap().cell(2, 2).unwrap();
    st.edit_map(1, "same cell", |m| m.set_cell(2, 2, same))
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
    st.edit_map(1, "a", |m| m.set_cell(0, 0, a)).unwrap();
    assert!(st.undo().is_some());
    st.edit_map(1, "b", |m| m.set_cell(0, 0, b)).unwrap();
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
        })
        .unwrap();
    let c2 = st
        .edit_matrix(0, "two", |m| {
            m.maps[1] = 2;
            Ok(())
        })
        .unwrap();
    assert_ne!(c1, c2);
    assert_eq!(st.undo(), Some(matrix_id(0)));
    assert_eq!(st.undo(), Some(map_id(0)));
    assert_eq!(st.undo(), None);
    assert_eq!(st.redo(), Some(map_id(0)));
    assert_eq!(st.redo(), Some(matrix_id(0)));
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
    st.edit_map(0, "flip", |m| m.set_cell(3, 5, new)).unwrap();
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

/// Reads every event file of the vanilla project (read-only, from the packed NARC) and
/// asserts that the codec reproduces its bytes. The store relies on this to be allowed to
/// write event files at all.
#[test]
fn vanilla_event_files_round_trip_byte_identically() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let family = match project.family {
        g4_formats::game::Family::Dp => uxie::GameFamily::DP,
        g4_formats::game::Family::Pt => uxie::GameFamily::Platinum,
        g4_formats::game::Family::Hgss => uxie::GameFamily::HGSS,
    };
    let packed = root.join(project.archives.packed(AssetKind::Event).unwrap());
    let narc = g4_formats::narc::Narc::read(&std::fs::read(packed).unwrap()).unwrap();
    assert!(!narc.members.is_empty());
    eprintln!("checked {} event files", narc.members.len());
    let mut bad = Vec::new();
    for (i, member) in narc.members.iter().enumerate() {
        let parsed = uxie::event_file::EventFile::from_binary(&mut std::io::Cursor::new(member));
        let reproduced = parsed.and_then(|e| {
            let mut out = std::io::Cursor::new(Vec::new());
            e.to_binary_for(&mut out, family)?;
            Ok(out.into_inner())
        });
        match reproduced {
            Ok(bytes) if bytes == *member => {}
            Ok(bytes) => bad.push(format!(
                "{i:04}: {} bytes in, {} out",
                member.len(),
                bytes.len()
            )),
            Err(e) => bad.push(format!("{i:04}: {e}")),
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} event files do not round-trip: {bad:#?}",
        bad.len(),
        narc.members.len()
    );
}

fn area_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::AreaData,
        index,
    }
}

fn event_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Event,
        index,
    }
}

#[test]
fn store_unpacks_on_first_access() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert!(!s.dir.path().join("unpacked/eventFiles").exists());
    assert_eq!(st.event(0).unwrap().warp_events.len(), 0);
    assert!(s.dir.path().join("unpacked/eventFiles/0000").exists());
    // Only the family that was touched is unpacked.
    assert!(!s.dir.path().join("unpacked/maps").exists());
    assert!(!s.dir.path().join("unpacked/areaData").exists());
    st.area_data(1).unwrap();
    assert!(s.dir.path().join("unpacked/areaData/0001").exists());
}

#[test]
fn read_only_kinds_are_not_unpacked_by_the_store() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    // No accessor exists for read-only kinds; the reload path shows the refusal.
    let id = AssetId {
        kind: AssetKind::TextArchive,
        index: 0,
    };
    assert!(matches!(
        st.reload_from_disk(id).unwrap_err(),
        KernelError::NotEditable(AssetKind::TextArchive)
    ));
    assert!(!s.dir.path().join("unpacked/textArchives").exists());
}

#[test]
fn failing_edit_closure_is_rolled_back() {
    let (_s, mut st) = store();
    let err = st
        .edit_map(1, "x", |m| m.set_cell(99, 0, Cell::default()))
        .unwrap_err();
    assert!(matches!(
        err,
        KernelError::Format(FormatError::OutOfRange { .. })
    ));
    assert!(!st.is_dirty(map_id(1)));
    assert!(st.undo().is_none());
}

#[test]
fn closure_error_keeps_changes_made_before_it_out_of_the_store() {
    let (_s, mut st) = store();
    let before = st.map(1).unwrap().clone();
    let err = st
        .edit_map(1, "half", |m| {
            m.set_cell(
                0,
                0,
                Cell {
                    kind: 9,
                    collision: 9,
                },
            )?;
            m.set_cell(99, 0, Cell::default())
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Format(_)));
    assert_eq!(*st.map(1).unwrap(), before);
    assert!(st.dirty().is_empty());
}

#[test]
fn failing_event_closure_is_rolled_back() {
    let (_s, mut st) = store();
    let err = st
        .edit_event(0, "x", |e| {
            e.warp_events.push(WarpEvent {
                x: 1,
                z: 1,
                dest_header_id: 1,
                dest_warp_id: 0,
                height: 0,
            });
            Err(FormatError::OutOfRange {
                what: "warp",
                value: 1,
                max: 0,
            })
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Format(_)));
    assert!(st.event(0).unwrap().warp_events.is_empty());
    assert!(st.undo().is_none());
}

#[test]
fn event_edit_round_trips_and_validates_warp_height() {
    let (s, mut st) = store(); // synthetic project is Platinum
    st.edit_event(0, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 8,
            z: 4,
            dest_header_id: 414,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    assert_eq!(st.dirty(), vec![event_id(0)]);
    st.save().unwrap();
    let bytes = std::fs::read(s.dir.path().join("unpacked/eventFiles/0000")).unwrap();
    assert_eq!(bytes.len(), 16 + 12);
    assert_eq!(&bytes[8..12], &1u32.to_le_bytes()); // warp count after bg and object counts
    assert_eq!(&bytes[12..14], &8u16.to_le_bytes()); // warp x
    assert_eq!(&bytes[16..18], &414u16.to_le_bytes()); // destination header
    assert_eq!(&bytes[24..28], &0u32.to_le_bytes()); // coord count
    // The other event file is untouched.
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/eventFiles/0001")).unwrap(),
        vec![0; 16]
    );

    let err = st
        .edit_event(0, "bad", |e| {
            e.warp_events[0].height = 1;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(&err, KernelError::Uxie(m) if m.contains("warp height must be 0")));
    assert_eq!(st.event(0).unwrap().warp_events[0].height, 0);
    assert!(!st.is_dirty(event_id(0)));
    // The rejected edit left no changeset: the only undo step is the first edit.
    assert!(st.undo().is_some());
    assert!(st.undo().is_none());
}

#[test]
fn event_edit_undo_redo_and_dirty_follow_the_bytes() {
    let (_s, mut st) = store();
    st.edit_event(1, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 1,
            z: 2,
            dest_header_id: 3,
            dest_warp_id: 4,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    assert!(st.is_dirty(event_id(1)));
    assert!(st.undo().is_some());
    assert!(!st.is_dirty(event_id(1)));
    assert!(st.event(1).unwrap().warp_events.is_empty());
    assert!(st.redo().is_some());
    assert_eq!(st.event(1).unwrap().warp_events.len(), 1);
}

/// A readable event file that the codec cannot write back byte-identically is shown but
/// locked: no edit, never dirty, `save` leaves its bytes alone.
fn assert_event_locked(s: &common::Synthetic, st: &mut Store, bytes: &[u8], reason: &str) {
    let path = s.dir.path().join("unpacked/eventFiles/0000");
    assert!(st.event(0).is_ok(), "the file is returned for display");
    let err = st.edit_event(0, "x", |_| Ok(())).unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { asset, reason: r }
            if *asset == event_id(0) && r.contains(reason)),
        "{err}"
    );
    assert!(
        err.to_string()
            .starts_with("event file 0000 is read-only: "),
        "{err}"
    );
    assert!(st.locked(event_id(0)).is_some_and(|r| r.contains(reason)));
    assert!(st.undo().is_none(), "a refused edit leaves no changeset");
    assert!(!st.is_dirty(event_id(0)));
    assert!(st.dirty().is_empty());
    assert!(st.save().unwrap().written.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn event_file_with_trailing_bytes_is_shown_but_locked() {
    let (s, mut st) = store();
    st.event(1).unwrap(); // unpacks the family
    let path = s.dir.path().join("unpacked/eventFiles/0000");
    // Trailing byte after the four counters: uxie would drop it on write.
    let mut bytes = vec![0u8; 16];
    bytes.push(0x5A);
    std::fs::write(&path, &bytes).unwrap();
    assert!(st.event(0).unwrap().warp_events.is_empty());
    assert_event_locked(&s, &mut st, &bytes, "17 bytes read, 16 would be written");
}

#[test]
fn event_file_with_platinum_warp_height_on_disk_is_shown_but_locked() {
    let (s, mut st) = store();
    st.event(1).unwrap(); // unpacks the family
    // bg count 0, object count 0, warp count 1, one warp with height 1, coord count 0.
    let mut warp = [0u8; 12];
    warp[0] = 8; // x
    warp[8] = 1; // height
    let mut file = vec![0u8; 8];
    file.extend_from_slice(&1u32.to_le_bytes());
    file.extend_from_slice(&warp);
    file.extend_from_slice(&0u32.to_le_bytes());
    let path = s.dir.path().join("unpacked/eventFiles/0000");
    std::fs::write(&path, &file).unwrap();
    let warps = &st.event(0).unwrap().warp_events;
    assert_eq!((warps.len(), warps[0].x, warps[0].height), (1, 8, 1));
    assert_event_locked(&s, &mut st, &file, "warp height must be 0");
}

#[test]
fn locked_event_does_not_block_saving_other_assets_and_reload_unlocks_it() {
    let (s, mut st) = store();
    st.event(1).unwrap();
    let path = s.dir.path().join("unpacked/eventFiles/0000");
    let mut bytes = vec![0u8; 16];
    bytes.push(0x5A);
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(st.locked(event_id(0)), None, "not loaded yet");
    st.event(0).unwrap();
    assert!(st.locked(event_id(0)).is_some());
    assert_eq!(
        st.locked(event_id(1)),
        None,
        "an editable asset is not locked"
    );
    st.edit_event(1, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 1,
            z: 1,
            dest_header_id: 1,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    assert_eq!(st.save().unwrap().written, vec![event_id(1)]);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // Another tool fixes the file; reloading makes it editable.
    std::fs::write(&path, vec![0u8; 16]).unwrap();
    st.reload_from_disk(event_id(0)).unwrap();
    assert_eq!(st.locked(event_id(0)), None);
    st.edit_event(0, "ok", |_| Ok(())).unwrap();
}

#[test]
fn map_tileset_index_follows_unsaved_edits_and_tolerates_a_missing_area() {
    let (_s, mut st) = store();
    assert_eq!(
        st.map_tileset_index(1).unwrap(),
        Some(st.area_data(1).unwrap().map_tileset())
    );
    assert_eq!(st.map_tileset_index(200).unwrap(), None);
    st.edit_area_data(0, "tileset", |a| {
        if let AreaData::Dppt { map_tileset, .. } = a {
            *map_tileset = 9;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(st.map_tileset_index(0).unwrap(), Some(9));
    assert!(st.undo().is_some());
    assert_eq!(st.map_tileset_index(0).unwrap(), Some(0));
}

#[test]
fn area_data_checked_gives_the_edited_record_or_none_past_the_archive() {
    let (_s, mut st) = store();
    assert_eq!(st.area_data_checked(200).unwrap(), None);
    let before = st.area_data_checked(1).unwrap().unwrap();
    assert_eq!(before.buildings_tileset(), 1);
    assert_eq!(before.area_type(), None);
    st.edit_area_data(1, "tileset", |a| {
        if let AreaData::Dppt {
            buildings_tileset, ..
        } = a
        {
            *buildings_tileset = 0;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        st.area_data_checked(1)
            .unwrap()
            .unwrap()
            .buildings_tileset(),
        0
    );
}

#[test]
fn area_data_edit_round_trips() {
    let (s, mut st) = store();
    st.edit_area_data(1, "tileset", |a| {
        if let AreaData::Dppt { map_tileset, .. } = a {
            *map_tileset = 7;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(st.area_data(1).unwrap().map_tileset(), 7);
    st.save().unwrap();
    let bytes = std::fs::read(s.dir.path().join("unpacked/areaData/0001")).unwrap();
    assert_eq!(bytes[2..4], 7u16.to_le_bytes());
    // The other fields keep their bytes.
    assert_eq!(bytes[0..2], 1u16.to_le_bytes());
    assert_eq!(bytes.len(), 8);
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/areaData/0000")).unwrap(),
        vec![0; 8]
    );
}

#[test]
fn stale_recovery_reload_and_accept() {
    let (s, mut st) = store();
    let id = map_id(0);
    let original = st.map(0).unwrap().cell(1, 1).unwrap();
    st.edit_map(0, "mine", |m| {
        m.set_cell(
            1,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    let path = s.dir.path().join("unpacked/maps/0000");
    // Cell (5, 6): x + 32 * y; the permission block starts after the 16-byte header.
    let mut theirs = std::fs::read(&path).unwrap();
    assert_eq!(theirs[16 + 2 * (6 * 32 + 5) + 1], 0);
    theirs[16 + 2 * (6 * 32 + 5) + 1] = 0x80;
    std::fs::write(&path, &theirs).unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(i) if i == id));
    st.reload_from_disk(id).unwrap();
    assert!(!st.is_dirty(id));
    assert_eq!(st.map(0).unwrap().cell(5, 6).unwrap().collision, 0x80); // theirs
    assert_eq!(st.map(0).unwrap().cell(1, 1).unwrap(), original); // mine gone
    assert!(st.undo().is_none());
    assert!(st.redo().is_none());
    assert_eq!(std::fs::read(&path).unwrap(), theirs);

    // Second round: keep mine.
    st.edit_map(0, "mine2", |m| {
        m.set_cell(
            2,
            2,
            Cell {
                kind: 2,
                collision: 0,
            },
        )
    })
    .unwrap();
    let mut t2 = theirs.clone();
    assert_eq!(t2[16 + 2 * (8 * 32 + 6) + 1], 0);
    t2[16 + 2 * (8 * 32 + 6) + 1] = 0x80;
    std::fs::write(&path, &t2).unwrap();
    assert!(st.save().is_err());
    st.accept_disk_as_base(id).unwrap();
    assert!(st.is_dirty(id));
    st.save().unwrap();
    let on_disk = MapFile::read(&std::fs::read(&path).unwrap(), Family::Pt).unwrap();
    assert_eq!(on_disk.cell(2, 2).unwrap().kind, 2);
    assert!(st.dirty().is_empty());
    // The base changed under my edits, so undoing them would write the old base over the
    // external edit: the asset's history is gone.
    assert!(st.undo().is_none());
    assert!(!st.is_dirty(id));
}

#[test]
fn reload_discards_only_the_changesets_of_that_asset() {
    let (_s, mut st) = store();
    st.edit_map(0, "map", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    st.edit_matrix(0, "matrix", |m| {
        m.maps[1] = 2;
        Ok(())
    })
    .unwrap();
    st.edit_map(0, "map again", |m| {
        m.set_cell(
            0,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    assert!(st.undo().is_some()); // leaves one undone map changeset on the redo stack
    st.reload_from_disk(map_id(0)).unwrap();
    assert!(st.redo().is_none());
    assert_eq!(st.undo(), Some(matrix_id(0)));
    assert!(st.undo().is_none());
    assert!(st.dirty().is_empty());
}

#[test]
fn reload_of_a_deleted_member_fails_without_touching_the_store() {
    let (s, mut st) = store();
    st.edit_map(0, "mine", |m| {
        m.set_cell(
            1,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    std::fs::remove_file(s.dir.path().join("unpacked/maps/0000")).unwrap();
    assert!(st.reload_from_disk(map_id(0)).is_err());
    assert!(st.is_dirty(map_id(0)));
    assert!(st.undo().is_some());
}

#[test]
fn accept_disk_as_base_of_a_deleted_member_recreates_it_on_save() {
    let (s, mut st) = store();
    let id = map_id(0);
    let path = s.dir.path().join("unpacked/maps/0000");
    st.edit_map(0, "mine", |m| {
        m.set_cell(
            1,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    std::fs::remove_file(&path).unwrap();
    // A deleted file is stale for a plain save ...
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(i) if i == id));
    // ... but "keep mine" accepts that the file is gone instead of failing.
    st.accept_disk_as_base(id).unwrap();
    assert!(st.is_dirty(id));
    assert!(!path.exists());
    assert_eq!(st.save().unwrap().written, vec![id]);
    let recreated = MapFile::read(&std::fs::read(&path).unwrap(), Family::Pt).unwrap();
    assert_eq!(recreated.cell(1, 1).unwrap().kind, 1);
    assert!(st.dirty().is_empty());
    assert_eq!(&recreated, st.map(0).unwrap());
    // The base changed, so the edit made against the old one is no longer undoable.
    assert!(st.undo().is_none());
    assert!(!st.is_dirty(id));
}

#[test]
fn accept_disk_as_base_of_a_deleted_member_makes_a_clean_asset_dirty_and_guards_reappearance() {
    let (s, mut st) = store();
    let id = map_id(1);
    let path = s.dir.path().join("unpacked/maps/0001");
    let original = std::fs::read(&path).unwrap();
    st.map(1).unwrap();
    std::fs::remove_file(&path).unwrap();
    st.accept_disk_as_base(id).unwrap();
    // The store's version is what the user keeps, so the file is to be recreated.
    assert!(st.is_dirty(id));
    // A file that reappears meanwhile is not silently overwritten.
    std::fs::write(&path, b"somebody else").unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(i) if i == id));
    std::fs::remove_file(&path).unwrap();
    st.save().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(st.dirty().is_empty());
}

#[test]
fn reload_of_garbage_on_disk_keeps_my_changes() {
    let (s, mut st) = store();
    st.edit_map(0, "mine", |m| {
        m.set_cell(
            1,
            1,
            Cell {
                kind: 1,
                collision: 0,
            },
        )
    })
    .unwrap();
    std::fs::write(s.dir.path().join("unpacked/maps/0000"), b"garbage").unwrap();
    assert!(st.reload_from_disk(map_id(0)).is_err());
    assert!(st.is_dirty(map_id(0)));
    assert_eq!(st.map(0).unwrap().cell(1, 1).unwrap().kind, 1);
    assert!(st.undo().is_some());
}

#[test]
fn vanilla_event_edit_save_round_trip_on_copy() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let hgss = project.family == Family::Hgss;
    let path = project.member_path(AssetKind::Event, 3);
    let mut st = Store::new(Arc::new(project));
    let before = st.event(3).unwrap().clone();
    let original = std::fs::read(&path).unwrap();
    let height = if hgss { 5 } else { 0 };
    st.edit_event(3, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 1,
            z: 2,
            dest_header_id: 3,
            dest_warp_id: 0,
            height,
        });
        Ok(())
    })
    .unwrap();
    if !hgss {
        let err = st
            .edit_event(3, "bad", |e| {
                e.warp_events.last_mut().unwrap().height = 1;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, KernelError::Uxie(_)));
    }
    assert_eq!(st.save().unwrap().written, vec![event_id(3)]);

    let mut fresh = Store::new(Arc::new(Project::open(&root).unwrap()));
    let after = fresh.event(3).unwrap();
    assert_eq!(after.warp_events.len(), before.warp_events.len() + 1);
    assert_eq!(after.warp_events.last().unwrap().height, height);
    assert_eq!(after.bg_events, before.bg_events);
    assert_eq!(after.object_events, before.object_events);
    assert_eq!(after.coord_events, before.coord_events);
    assert!(fresh.dirty().is_empty());
    assert_eq!(
        std::fs::read(&path).unwrap().len(),
        original.len() + 12,
        "one 12-byte warp added"
    );
}

#[test]
fn vanilla_area_data_edit_save_round_trip_on_copy() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let packed = root.join(project.archives.packed(AssetKind::AreaData).unwrap());
    let narc = g4_formats::narc::Narc::read(&std::fs::read(packed).unwrap()).unwrap();
    let original = narc.members[1].clone();
    let path = project.member_path(AssetKind::AreaData, 1);
    let mut st = Store::new(Arc::new(project));
    let new = st.area_data(1).unwrap().map_tileset() ^ 1;
    st.edit_area_data(1, "tileset", |a| {
        let (AreaData::Dppt { map_tileset, .. } | AreaData::Hgss { map_tileset, .. }) = a;
        *map_tileset = new;
        Ok(())
    })
    .unwrap();
    assert_eq!(st.save().unwrap().written, vec![area_id(1)]);
    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved.len(), original.len());
    assert_eq!(saved[2..4], new.to_le_bytes());
    let differing = saved.iter().zip(&original).filter(|(a, b)| a != b).count();
    assert_eq!(differing, 1, "only the tileset changes (low bit flipped)");
}

#[test]
fn create_matrix_and_duplicate_map_append_members() {
    let (s, mut st) = store();
    let n = st.member_count(AssetKind::Matrix).unwrap();
    let idx = st.create_matrix("neu").unwrap();
    assert_eq!(idx as usize, n);
    assert!(
        s.dir
            .path()
            .join(format!("unpacked/matrices/{idx:04}"))
            .exists()
    );
    assert_eq!(st.matrix(idx).unwrap().maps, vec![MATRIX_EMPTY]);
    assert_eq!(st.member_count(AssetKind::Matrix).unwrap(), n + 1);
    let m2 = st.duplicate_map(1).unwrap();
    assert_eq!(
        std::fs::read(s.dir.path().join(format!("unpacked/maps/{m2:04}"))).unwrap(),
        std::fs::read(s.dir.path().join("unpacked/maps/0001")).unwrap()
    );
    st.edit_matrix(idx, "assign", |m| {
        m.maps[0] = m2;
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    assert_eq!(
        g4_kernel::build::pack_dir(&s.dir.path().join("unpacked/matrices"))
            .unwrap()
            .members
            .len(),
        n + 1
    );
}

#[test]
fn create_and_duplicate_unpack_the_archive_first_and_never_overwrite() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert!(!s.dir.path().join("unpacked").exists());
    let idx = st.create_matrix("a").unwrap();
    assert_eq!(idx, 1);
    let second = st.create_matrix("b").unwrap();
    assert_eq!(second, 2);
    let dup = st.duplicate_map(0).unwrap();
    assert_eq!(dup, 3);
    // A bad source or an over-long name creates nothing.
    assert!(st.duplicate_map(99).is_err());
    assert!(st.create_matrix(&"x".repeat(256)).is_err());
    assert_eq!(st.member_count(AssetKind::Map).unwrap(), 4);
    assert_eq!(st.member_count(AssetKind::Matrix).unwrap(), 3);
    // A gap-free folder is required: a stray file at the next index is never overwritten.
    let stray = s.dir.path().join("unpacked/maps/0005");
    std::fs::write(&stray, b"keep").unwrap();
    // count is now 5 (0000-0003 plus 0005), so index 5 is taken.
    assert!(st.duplicate_map(0).is_err());
    assert_eq!(std::fs::read(&stray).unwrap(), b"keep");
    // New members are not undoable and leave nothing dirty.
    assert!(st.undo().is_none());
    assert!(st.dirty().is_empty());
}

#[test]
fn duplicate_map_copies_unsaved_edits_of_the_source() {
    let (s, mut st) = store();
    st.edit_map(1, "block", |m| {
        m.set_cell(
            0,
            0,
            Cell {
                kind: 7,
                collision: 0x80,
            },
        )
    })
    .unwrap();
    let dup = st.duplicate_map(1).unwrap();
    let copy = g4_formats::map::MapFile::read(
        &std::fs::read(s.dir.path().join(format!("unpacked/maps/{dup:04}"))).unwrap(),
        Family::Pt,
    )
    .unwrap();
    assert_eq!(copy.cell(0, 0).unwrap().kind, 7);
    // The source on disk is untouched until saved.
    assert!(st.is_dirty(map_id(1)));
}
