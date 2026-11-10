#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::area_data::AreaData;
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::map::{Cell, MapFile};
use g4_formats::matrix::MATRIX_EMPTY;
use g4_kernel::error::KernelError;
use g4_kernel::places;
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
    assert_eq!(st.undo().unwrap(), [matrix_id(0)]);
    assert_eq!(st.undo().unwrap(), [map_id(1)]);
    assert!(st.undo().unwrap().is_empty());
    assert_eq!(st.redo().unwrap(), [map_id(1)]);
    assert_eq!(st.redo().unwrap(), [matrix_id(0)]);
    assert!(st.redo().unwrap().is_empty());
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
    assert_eq!(st.undo().unwrap(), [map_id(0)]); // one map changeset on the redo stack
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut theirs = std::fs::read(&path).unwrap();
    theirs[16 + 2 * (6 * 32 + 5) + 1] = 0x80;
    std::fs::write(&path, &theirs).unwrap();
    st.accept_disk_as_base(map_id(0)).unwrap();
    assert!(
        st.redo().unwrap().is_empty(),
        "the map's redo entry is gone"
    );
    assert_eq!(
        st.undo().unwrap(),
        [matrix_id(0)],
        "other assets keep their history"
    );
    assert!(
        st.undo().unwrap().is_empty(),
        "the map's undo entries are gone"
    );
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
    assert!(!st.undo().unwrap().is_empty());
    assert!(!st.is_dirty(id));
    assert!(!st.redo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
}

#[test]
fn edit_that_changes_nothing_records_no_changeset() {
    let (_s, mut st) = store();
    st.edit_map(1, "noop", |_| Ok(())).unwrap();
    let same = st.map(1).unwrap().cell(2, 2).unwrap();
    st.edit_map(1, "same cell", |m| m.set_cell(2, 2, same))
        .unwrap();
    assert!(!st.is_dirty(map_id(1)));
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
    st.edit_map(1, "b", |m| m.set_cell(0, 0, b)).unwrap();
    assert!(st.redo().unwrap().is_empty());
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
    assert_eq!(st.undo().unwrap(), [matrix_id(0)]);
    assert_eq!(st.undo().unwrap(), [map_id(0)]);
    assert!(st.undo().unwrap().is_empty());
    assert_eq!(st.redo().unwrap(), [map_id(0)]);
    assert_eq!(st.redo().unwrap(), [matrix_id(0)]);
    assert!(st.redo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
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
        kind: AssetKind::MapTexture,
        index: 0,
    };
    assert!(matches!(
        st.reload_from_disk(id).unwrap_err(),
        KernelError::NotEditable(AssetKind::MapTexture)
    ));
    assert!(!st.project().unpacked_dir(AssetKind::MapTexture).exists());
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
    assert!(st.undo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
    assert!(!st.is_dirty(event_id(1)));
    assert!(st.event(1).unwrap().warp_events.is_empty());
    assert!(!st.redo().unwrap().is_empty());
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
    assert!(
        st.undo().unwrap().is_empty(),
        "a refused edit leaves no changeset"
    );
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
    assert!(!st.undo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
    assert!(st.redo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty()); // leaves one undone map changeset on the redo stack
    st.reload_from_disk(map_id(0)).unwrap();
    assert!(st.redo().unwrap().is_empty());
    assert_eq!(st.undo().unwrap(), [matrix_id(0)]);
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
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
    assert!(!st.undo().unwrap().is_empty());
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
    assert!(st.undo().unwrap().is_empty());
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

#[test]
fn create_and_duplicate_event_files_append_members() {
    let (s, mut st) = store();
    let n = st.member_count(AssetKind::Event).unwrap();
    let empty = st.create_event_file().unwrap();
    assert_eq!(usize::from(empty), n);
    assert_eq!(
        std::fs::read(s.dir.path().join(format!("unpacked/eventFiles/{empty:04}"))).unwrap(),
        [0u8; 16]
    );
    assert!(st.event(empty).unwrap().warp_events.is_empty());
    assert_eq!(st.locked(event_id(empty)), None);

    // A copy of a file that was never loaded is the file on disk; one of a loaded, edited
    // file holds the unsaved edit.
    let plain = st.duplicate_event_file(0).unwrap();
    assert_eq!(
        std::fs::read(s.dir.path().join(format!("unpacked/eventFiles/{plain:04}"))).unwrap(),
        std::fs::read(s.dir.path().join("unpacked/eventFiles/0000")).unwrap()
    );
    st.edit_event(0, "warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 1,
            z: 2,
            dest_header_id: 1,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    let edited = st.duplicate_event_file(0).unwrap();
    assert_eq!(st.event(edited).unwrap().warp_events.len(), 1);
    assert_eq!(st.member_count(AssetKind::Event).unwrap(), n + 3);
    assert!(st.duplicate_event_file(99).is_err());
    // The new files leave nothing dirty beyond the edited source and are not undoable.
    assert_eq!(st.dirty(), vec![event_id(0)]);
}

#[test]
fn duplicate_map_with_clear_buildings_keeps_everything_else() {
    let (s, mut st) = store();
    let source = std::fs::read(s.dir.path().join("unpacked/maps/0001")).unwrap();
    let source_map = MapFile::read(&source, Family::Pt).unwrap();
    assert!(!source_map.buildings.is_empty());

    // Never loaded: parsed from disk.
    let cleared = st.duplicate_map_with(1, true).unwrap();
    let read = |i: u16| {
        MapFile::read(
            &std::fs::read(s.dir.path().join(format!("unpacked/maps/{i:04}"))).unwrap(),
            Family::Pt,
        )
        .unwrap()
    };
    let copy = read(cleared);
    assert!(copy.buildings.is_empty());
    assert_eq!(copy.permissions, source_map.permissions);
    assert_eq!(copy.model, source_map.model);
    assert_eq!(copy.bdhc, source_map.bdhc);
    assert_eq!(copy.buildings_tail, source_map.buildings_tail);

    // Loaded with an unsaved edit: the copy has the edit and no buildings.
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
    let edited = read(st.duplicate_map_with(1, true).unwrap());
    assert_eq!(edited.cell(0, 0).unwrap().kind, 7);
    assert!(edited.buildings.is_empty());
    // Without the flag, `duplicate_map_with` copies the buildings, like `duplicate_map`.
    let full = read(st.duplicate_map_with(1, false).unwrap());
    assert_eq!(full.buildings.len(), source_map.buildings.len());
    assert!(st.duplicate_map_with(99, true).is_err());
}

/// Platinum DE map 506 has two bytes after its last building record (`0D 0A`). A copy without
/// buildings keeps them, and the building section shrinks to exactly those bytes.
#[test]
fn duplicate_map_with_clear_buildings_keeps_a_buildings_tail_from_disk() {
    let (s, mut st) = store();
    let path = s.dir.path().join("unpacked/maps/0002");
    let mut map = MapFile::read(&std::fs::read(&path).unwrap(), Family::Pt).unwrap();
    assert!(!map.buildings.is_empty());
    map.buildings_tail = vec![0x0D, 0x0A];
    map.validate().unwrap();
    std::fs::write(&path, map.write(Family::Pt)).unwrap();

    // Never loaded: the store parses the file on disk.
    let index = st.duplicate_map_with(2, true).unwrap();
    let bytes = std::fs::read(s.dir.path().join(format!("unpacked/maps/{index:04}"))).unwrap();
    let copy = MapFile::read(&bytes, Family::Pt).unwrap();
    assert!(copy.buildings.is_empty());
    assert_eq!(copy.buildings_tail, vec![0x0D, 0x0A]);
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
    // The copy loads editable: maps never lock, because `MapFile::read` refuses any file
    // that `MapFile::write` would not reproduce byte for byte.
    st.map(index).unwrap();
    assert_eq!(st.locked(map_id(index)), None);
    assert_eq!(st.locked(map_id(2)), None);
}

/// A synthetic Platinum project with the Dynamic Headers patch applied: the table now lives
/// in `unpacked/dynamicHeaders`, one 24-byte file per header.
fn patched_store() -> (common::Synthetic, Store, std::path::PathBuf) {
    let s = common::synthetic_project();
    let project = Arc::new(Project::open(s.dir.path()).unwrap());
    let headers = Store::new(project.clone()).header_table().unwrap().to_vec();
    g4_kernel::dynamic_headers::apply(&project, &headers).unwrap();
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    (s, Store::new(project), dir)
}

#[test]
fn patched_project_reads_and_saves_headers_from_files() {
    use g4_kernel::store::MapHeader;
    let (s, mut st, dir) = patched_store();
    let arm9_before = std::fs::read(s.dir.path().join("arm9/arm9.bin")).unwrap();
    let first_before = std::fs::read(dir.join("0000")).unwrap();
    assert_eq!(st.header_table().unwrap().len(), 2);
    st.edit_header(1, "x", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 7;
            h.text_archive_id = 433;
        }
        Ok(())
    })
    .unwrap();
    let table = AssetId {
        kind: AssetKind::HeaderTable,
        index: 0,
    };
    assert!(st.is_dirty(table));
    assert_eq!(st.save().unwrap().written, vec![table]);
    assert!(!st.is_dirty(table));
    let second = std::fs::read(dir.join("0001")).unwrap();
    assert_eq!(second.len(), 24);
    assert_eq!(&second[2..4], &7u16.to_le_bytes());
    assert_eq!(std::fs::read(dir.join("0000")).unwrap(), first_before);
    assert_eq!(
        std::fs::read(s.dir.path().join("arm9/arm9.bin")).unwrap(),
        arm9_before
    );
    // A fresh project reads the edit back through the files.
    let reread = Project::open(s.dir.path()).unwrap();
    assert_eq!(reread.headers().unwrap()[1].matrix_id, 7);
    // The listing carries the archive a header reads its messages from.
    assert_eq!(reread.headers().unwrap()[1].text_archive_id, 433);
    assert_eq!(reread.header_count().unwrap(), 2);
}

#[cfg(unix)] // compares inode numbers
#[test]
fn patched_header_save_writes_only_the_changed_files() {
    use g4_kernel::store::MapHeader;
    let (_s, mut st, dir) = patched_store();
    st.header_table().unwrap();
    let unchanged = dir.join("0000");
    // A rewrite goes through a temp file and a rename, which gives the file a new inode.
    let inode = |p: &std::path::Path| {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(p).unwrap().ino()
    };
    let before = inode(&unchanged);
    st.edit_header(1, "x", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 9;
        }
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    assert_eq!(inode(&unchanged), before, "file 0000 was rewritten");
}

#[test]
fn patched_header_save_detects_a_changed_file() {
    use g4_kernel::store::MapHeader;
    let (_s, mut st, dir) = patched_store();
    st.header_table().unwrap();
    let mut bytes = std::fs::read(dir.join("0000")).unwrap();
    bytes[0] ^= 1;
    std::fs::write(dir.join("0000"), &bytes).unwrap();
    st.edit_header(1, "x", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = 7;
        }
        Ok(())
    })
    .unwrap();
    let table = AssetId {
        kind: AssetKind::HeaderTable,
        index: 0,
    };
    assert!(matches!(
        st.save().unwrap_err(),
        KernelError::Stale(id) if id == table
    ));
    // Nothing was written: file 0001 still has its original matrix.
    assert_eq!(
        &std::fs::read(dir.join("0001")).unwrap()[2..4],
        &0u16.to_le_bytes()
    );
}

#[test]
fn missing_header_file_is_reported_not_panicked() {
    let (s, mut st, dir) = patched_store();
    std::fs::remove_file(dir.join("0001")).unwrap();
    let message = st.header_table().unwrap_err().to_string();
    assert!(message.contains("0001"), "{message}");
    assert!(message.contains("dynamicHeaders"), "{message}");
    // The listing reports it the same way.
    let message = Project::open(s.dir.path())
        .unwrap()
        .header_listing()
        .unwrap_err()
        .to_string();
    assert!(message.contains("0001"), "{message}");
}

#[test]
fn dynamic_headers_kind_is_store_only() {
    let (_s, st, _dir) = patched_store();
    assert!(
        st.project()
            .member_count(AssetKind::DynamicHeaders)
            .is_err()
    );
    assert!(
        st.project()
            .ensure_unpacked(AssetKind::DynamicHeaders)
            .is_err()
    );
}

fn header_table_id() -> AssetId {
    AssetId {
        kind: AssetKind::HeaderTable,
        index: 0,
    }
}

fn set_matrix(st: &mut Store, header: u16, matrix: u16) {
    use g4_kernel::store::MapHeader;
    st.edit_header(header, "matrix", |h| {
        if let MapHeader::Pt(h) = h {
            h.matrix_id = matrix;
        }
        Ok(())
    })
    .unwrap();
}

/// A synthetic Platinum project, not patched, with a store on it.
fn unpatched_store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let project = Arc::new(Project::open(s.dir.path()).unwrap());
    (s, Store::new(project))
}

#[test]
fn applies_the_patch_with_an_unsaved_header_edit_and_keeps_it() {
    let (s, mut st) = unpatched_store();
    let old = matrix_of(&mut st, 1);
    set_matrix(&mut st, 1, old + 3);
    assert_eq!(st.apply_dynamic_headers().unwrap(), None);
    assert!(st.dynamic_headers_applied().unwrap());
    // The store keeps the unsaved value; the file holds the table as it was on disk.
    assert_eq!(matrix_of(&mut st, 1), old + 3);
    let file = std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0001")).unwrap();
    assert_eq!(&file[2..4], &old.to_le_bytes());
    assert_eq!(st.dirty(), vec![header_table_id()]);
    // Saving now writes the header file, not arm9.bin.
    let arm9 = std::fs::read(s.dir.path().join("arm9/arm9.bin")).unwrap();
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    let file = std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0001")).unwrap();
    assert_eq!(&file[2..4], &(old + 3).to_le_bytes());
    assert_eq!(
        std::fs::read(s.dir.path().join("arm9/arm9.bin")).unwrap(),
        arm9
    );
}

#[test]
fn undo_and_redo_cross_the_patch() {
    let (s, mut st) = unpatched_store();
    let file = |s: &common::Synthetic| {
        let bytes = std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0001")).unwrap();
        u16::from_le_bytes([bytes[2], bytes[3]])
    };
    let a = matrix_of(&mut st, 1);
    let (b, c) = (a + 1, a + 2);
    // One edit saved into arm9.bin, one unsaved, then the patch.
    set_matrix(&mut st, 1, b);
    st.save().unwrap();
    set_matrix(&mut st, 1, c);
    st.apply_dynamic_headers().unwrap();
    assert_eq!(file(&s), b);
    assert_eq!(matrix_of(&mut st, 1), c);

    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(matrix_of(&mut st, 1), b);
    assert!(st.dirty().is_empty());
    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(matrix_of(&mut st, 1), a);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    assert_eq!(file(&s), a);

    assert_eq!(st.redo().unwrap(), [header_table_id()]);
    assert_eq!(st.redo().unwrap(), [header_table_id()]);
    assert_eq!(matrix_of(&mut st, 1), c);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    assert_eq!(file(&s), c);
}

/// The names of the entries of `dir`, sorted.
fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn snapshots_an_existing_dynamic_headers_folder() {
    let (s, mut st) = unpatched_store();
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0000"), [1u8; 24]).unwrap();
    std::fs::write(dir.join("0001"), [2u8; 7]).unwrap();
    let snapshot = st.apply_dynamic_headers().unwrap().unwrap();
    assert_eq!(snapshot.parent().unwrap(), s.dir.path().join(".g4/backup"));
    let name = snapshot.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("dynamicHeaders-"), "{name}");
    // dynamicHeaders-YYYYMMDD-HHMMSS
    assert_eq!(name.len(), "dynamicHeaders-".len() + 15, "{name}");
    assert_eq!(names_in(&snapshot), ["0000", "0001"]);
    assert_eq!(std::fs::read(snapshot.join("0000")).unwrap(), [1u8; 24]);
    assert_eq!(std::fs::read(snapshot.join("0001")).unwrap(), [2u8; 7]);
    // The folder itself now holds the patched table.
    assert_eq!(names_in(&dir), ["0000", "0001"]);
    assert_eq!(std::fs::read(dir.join("0001")).unwrap().len(), 24);
    assert_ne!(std::fs::read(dir.join("0000")).unwrap(), [1u8; 24]);
}

#[test]
fn takes_no_snapshot_without_a_folder() {
    let (s, mut st) = unpatched_store();
    assert_eq!(st.apply_dynamic_headers().unwrap(), None);
    assert!(!s.dir.path().join(".g4/backup").exists());
}

#[test]
fn still_refuses_a_table_changed_on_disk() {
    use g4_kernel::project::Location;
    let (s, mut st) = unpatched_store();
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("0000"), [1u8; 24]).unwrap();
    // An unsaved edit, then another tool edits the table in arm9.bin.
    set_matrix(&mut st, 0, 5);
    let Location::Slice { offset, .. } = st
        .project()
        .asset_location(AssetKind::HeaderTable, 0)
        .unwrap()
    else {
        panic!("the header table is a slice of arm9.bin before the patch");
    };
    let arm9_path = s.dir.path().join("arm9/arm9.bin");
    let mut arm9 = std::fs::read(&arm9_path).unwrap();
    arm9[offset as usize + 30] ^= 0x5A;
    std::fs::write(&arm9_path, &arm9).unwrap();

    assert!(matches!(
        st.apply_dynamic_headers().unwrap_err(),
        KernelError::Stale(id) if id == header_table_id()
    ));
    assert_eq!(std::fs::read(&arm9_path).unwrap(), arm9, "arm9.bin changed");
    assert_eq!(names_in(&dir), ["0000"]);
    assert!(!s.dir.path().join(".g4/backup").exists());
    assert!(!s.dir.path().join("arm9/arm9.bin.backup").exists());
    assert_eq!(matrix_of(&mut st, 0), 5);
}

#[test]
fn apply_dynamic_headers_refuses_a_table_that_is_behind_the_disk() {
    use g4_kernel::project::Location;
    let (s, mut st) = unpatched_store();
    st.header_table().unwrap();
    // Another tool (DSPRE) edits a header in arm9.bin while the store holds the old table.
    let Location::Slice { offset, .. } = st
        .project()
        .asset_location(AssetKind::HeaderTable, 0)
        .unwrap()
    else {
        panic!("the header table is a slice of arm9.bin before the patch");
    };
    let arm9_path = s.dir.path().join("arm9/arm9.bin");
    let mut arm9 = std::fs::read(&arm9_path).unwrap();
    arm9[offset as usize + 2] ^= 0x5A;
    std::fs::write(&arm9_path, &arm9).unwrap();

    assert!(matches!(
        st.apply_dynamic_headers().unwrap_err(),
        KernelError::Stale(id) if id == header_table_id()
    ));
    assert_eq!(std::fs::read(&arm9_path).unwrap(), arm9, "arm9.bin changed");
    assert!(!s.dir.path().join("unpacked/dynamicHeaders").exists());
    assert!(!s.dir.path().join("arm9/arm9.bin.backup").exists());
    assert!(!st.dynamic_headers_applied().unwrap());
}

#[test]
fn apply_dynamic_headers_reloads_the_table() {
    let (_s, mut st) = unpatched_store();
    let before = st.header_table().unwrap().to_vec();
    assert!(!st.dynamic_headers_applied().unwrap());
    st.apply_dynamic_headers().unwrap();
    assert!(st.dynamic_headers_applied().unwrap());
    assert_eq!(st.header_table().unwrap(), before.as_slice());
    assert!(st.dirty().is_empty());
    assert!(st.undo().unwrap().is_empty());
    // The patch cannot be applied twice.
    assert!(st.apply_dynamic_headers().is_err());
}

#[test]
fn apply_dynamic_headers_works_before_the_table_was_loaded() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    assert_eq!(
        std::fs::read_dir(s.dir.path().join("unpacked/dynamicHeaders"))
            .unwrap()
            .count(),
        2
    );
    assert_eq!(st.header_table().unwrap().len(), 2);
}

#[test]
fn create_header_appends_file_and_name() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    assert_eq!(st.create_header(1, "ROUTE_230").unwrap(), 2);
    assert_eq!(
        std::fs::read(dir.join("0002")).unwrap(),
        std::fs::read(dir.join("0001")).unwrap()
    );
    let names = std::fs::read(s.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
    assert_eq!(names.len(), 48);
    assert_eq!(&names[32..], b"ROUTE_230\0\0\0\0\0\0\0");
    assert_eq!(st.header_table().unwrap().len(), 3);
    let table = st.header_table().unwrap().to_vec();
    let listing = st.project().header_listing_for(&table).unwrap();
    assert_eq!(listing.headers[2].internal_name, "ROUTE_230");
    assert!(st.dirty().is_empty());
    // The table is in step with the disk: a later edit saves without a stale error.
    set_matrix(&mut st, 2, 4);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    assert_eq!(
        &std::fs::read(dir.join("0002")).unwrap()[2..4],
        &4u16.to_le_bytes()
    );
    // A fresh project agrees about the count.
    assert_eq!(
        Project::open(s.dir.path()).unwrap().header_count().unwrap(),
        3
    );
}

#[test]
fn create_header_before_the_table_was_loaded() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    assert_eq!(st.create_header(0, "FIRST").unwrap(), 2);
    assert_eq!(st.header_table().unwrap().len(), 3);
    assert!(st.dirty().is_empty());
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0002")).unwrap(),
        std::fs::read(s.dir.path().join("unpacked/dynamicHeaders/0000")).unwrap()
    );
}

#[test]
fn create_header_copies_unsaved_edits_of_the_template() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    let on_disk = std::fs::read(dir.join("0000")).unwrap();
    set_matrix(&mut st, 0, 9);
    let id = st.create_header(0, "X").unwrap();
    assert_eq!(id, 2);
    let copy = std::fs::read(dir.join("0002")).unwrap();
    assert_eq!(&copy[2..4], &9u16.to_le_bytes());
    // Header 0 itself is still unsaved, the new header is not part of the dirty state.
    assert_eq!(std::fs::read(dir.join("0000")).unwrap(), on_disk);
    assert_eq!(st.dirty(), vec![header_table_id()]);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    assert_eq!(
        &std::fs::read(dir.join("0000")).unwrap()[2..4],
        &9u16.to_le_bytes()
    );
    assert_eq!(std::fs::read(dir.join("0002")).unwrap(), copy);
}

#[test]
fn create_header_without_patch_is_refused() {
    let (s, mut st) = unpatched_store();
    let message = st.create_header(0, "X").unwrap_err().to_string();
    assert!(message.contains("patch"), "{message}");
    assert_eq!(
        std::fs::metadata(s.dir.path().join("files/fielddata/maptable/mapname.bin"))
            .unwrap()
            .len(),
        32
    );
}

#[test]
fn create_header_rejects_non_ascii_or_long_names() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    assert!(st.create_header(0, "Straße").is_err());
    assert!(st.create_header(0, "ABCDEFGHIJKLMNOPQ").is_err());
    // 16 bytes fit exactly, without a terminator.
    assert_eq!(st.create_header(0, "ABCDEFGHIJKLMNOP").unwrap(), 2);
    let names = std::fs::read(s.dir.path().join("files/fielddata/maptable/mapname.bin")).unwrap();
    assert_eq!(names.len(), 48);
    assert_eq!(&names[32..], b"ABCDEFGHIJKLMNOP");
}

#[test]
fn create_header_rejects_an_unknown_template() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    assert!(st.create_header(2, "X").is_err());
    assert_eq!(
        std::fs::read_dir(s.dir.path().join("unpacked/dynamicHeaders"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn create_header_refuses_to_overwrite_a_stray_file() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    let stray = s.dir.path().join("unpacked/dynamicHeaders/0002");
    std::fs::write(&stray, b"stray").unwrap();
    assert!(st.create_header(0, "X").is_err());
    assert_eq!(std::fs::read(&stray).unwrap(), b"stray");
    assert_eq!(
        std::fs::metadata(s.dir.path().join("files/fielddata/maptable/mapname.bin"))
            .unwrap()
            .len(),
        32
    );
}

fn matrix_of(st: &mut Store, header: u16) -> u16 {
    st.header_table().unwrap()[usize::from(header)].matrix_id()
}

#[test]
fn undo_after_create_header_keeps_the_new_header() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    let old = matrix_of(&mut st, 0);
    set_matrix(&mut st, 0, 5);
    assert_eq!(st.create_header(1, "NEW").unwrap(), 2);
    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(st.header_table().unwrap().len(), 3);
    assert_eq!(matrix_of(&mut st, 0), old);
    assert!(st.dirty().is_empty());
    // The table is still savable: an edit now writes through the files.
    set_matrix(&mut st, 2, 8);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    assert_eq!(
        &std::fs::read(dir.join("0002")).unwrap()[2..4],
        &8u16.to_le_bytes()
    );
    // Redo of the edit that was undone keeps all three headers as well.
    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(st.header_table().unwrap().len(), 3);
}

#[test]
fn redo_after_create_header_keeps_the_new_header() {
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    let old = matrix_of(&mut st, 0);
    set_matrix(&mut st, 0, 5);
    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(st.create_header(1, "NEW").unwrap(), 2);
    assert_eq!(st.redo().unwrap(), [header_table_id()]);
    assert_eq!(st.header_table().unwrap().len(), 3);
    assert_eq!(matrix_of(&mut st, 0), 5);
    assert_eq!(st.dirty(), vec![header_table_id()]);
    assert_eq!(st.save().unwrap().written, vec![header_table_id()]);
    let dir = s.dir.path().join("unpacked/dynamicHeaders");
    assert_eq!(
        &std::fs::read(dir.join("0000")).unwrap()[2..4],
        &5u16.to_le_bytes()
    );
    assert_eq!(std::fs::read(dir.join("0002")).unwrap().len(), 24);
    // And back: undo restores the old value, still with three headers.
    assert_eq!(st.undo().unwrap(), [header_table_id()]);
    assert_eq!(st.header_table().unwrap().len(), 3);
    assert_eq!(matrix_of(&mut st, 0), old);
}

#[cfg(unix)]
#[test]
fn create_header_rolls_back_when_mapname_cannot_be_written() {
    use std::os::unix::fs::PermissionsExt;
    let (s, mut st) = unpatched_store();
    st.apply_dynamic_headers().unwrap();
    st.header_table().unwrap();
    let table_before = st.header_table().unwrap().to_vec();
    let maptable = s.dir.path().join("files/fielddata/maptable");
    let names_path = maptable.join("mapname.bin");
    let names_before = std::fs::read(&names_path).unwrap();
    // A folder that cannot be written refuses the rename of the new mapname.bin.
    std::fs::set_permissions(&maptable, std::fs::Permissions::from_mode(0o555)).unwrap();
    let result = st.create_header(0, "X");
    std::fs::set_permissions(&maptable, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert!(!s.dir.path().join("unpacked/dynamicHeaders/0002").exists());
    assert_eq!(std::fs::read(&names_path).unwrap(), names_before);
    assert_eq!(st.header_table().unwrap(), table_before.as_slice());
    assert!(st.dirty().is_empty());
    // Nothing is left behind: the same call works once the folder is writable.
    assert_eq!(st.create_header(0, "X").unwrap(), 2);
}

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

#[test]
fn search_skips_archives_that_do_not_decode() {
    // Members 0..=432 of the fixture are garbage; the search must not trip over them.
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    let hits = st.search_text("sandgem", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].archive, hits[0].index), (433, 2));
    assert!(matches!(
        st.text_archive(0).unwrap_err(),
        KernelError::InvalidProject(m) if m.contains("textArchives/0000")
    ));
}

#[test]
fn search_sees_a_saved_edit_but_not_an_unsaved_one() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert_eq!(st.search_text("twinleaf", 10).unwrap().len(), 1);
    st.edit_text_archive(433, "rename", |d| {
        d.messages[1].text = "Zweiblattdorf".into()
    })
    .unwrap();
    assert_eq!(st.search_text("twinleaf", 10).unwrap().len(), 1);
    st.save().unwrap();
    assert!(st.search_text("twinleaf", 10).unwrap().is_empty());
    let hits = st.search_text("ZWEIBLATT", 10).unwrap();
    assert_eq!((hits[0].archive, hits[0].index), (433, 1));
}

#[test]
fn text_archive_save_reports_warnings_and_is_stale_after_an_external_change() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    st.edit_text_archive(433, "wide", |d| d.messages[1].text = "W [NOPE]".into())
        .unwrap();
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![text_id(433)]);
    assert!(!report.problems.is_empty(), "an encoder problem warns");
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity == g4_kernel::text::Severity::Warning)
    );

    // Another tool rewrites the JSON; the next save refuses to overwrite it.
    let json = s.dir.path().join("expanded/textArchives/0433.json");
    let other = std::fs::read_to_string(&json).unwrap().replace('W', "V");
    std::fs::write(&json, other).unwrap();
    st.edit_text_archive(433, "again", |d| d.messages[1].text = "x".into())
        .unwrap();
    assert!(matches!(
        st.save().unwrap_err(),
        KernelError::Stale(id) if id == text_id(433)
    ));
    st.accept_disk_as_base(text_id(433)).unwrap();
    st.save().unwrap();
    assert!(std::fs::read_to_string(&json).unwrap().contains("\"x\""));
}

#[test]
fn save_records_the_written_text_archives() {
    use g4_kernel::build::state::BuildState;
    use g4_kernel::text::{TextSide, TextSync};

    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    for n in [433, 392] {
        st.edit_text_archive(n, "edit", |d| d.messages[0].text = "Neu".into())
            .unwrap();
    }
    st.edit_map(1, "cell", |m| {
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
    assert!(BuildState::load(s.dir.path()).texts.is_empty());
    st.save().unwrap();

    let texts = BuildState::load(s.dir.path()).texts;
    assert_eq!(texts.keys().copied().collect::<Vec<_>>(), [392, 433]);
    for record in texts.values() {
        // Hashed, so a copy of the project (other inodes) still compares by content.
        assert!(
            record.json.is_some_and(|fp| fp.xxh3.is_some()),
            "{record:?}"
        );
        assert!(
            record.member.is_some_and(|fp| fp.xxh3.is_some()),
            "{record:?}"
        );
    }
    let p = st.project();
    let sync = TextSync::load(p);
    assert_eq!(sync.side(p, 433).unwrap(), TextSide::InSync);
    assert_eq!(sync.side(p, 392).unwrap(), TextSide::InSync);
    // An archive the save did not write has no record.
    assert_eq!(sync.side(p, 6).unwrap(), TextSide::OnlyMember);
}

#[test]
fn broken_text_json_is_an_error_naming_the_file() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    st.project()
        .ensure_unpacked(AssetKind::TextArchive)
        .unwrap();
    let json = s.dir.path().join("expanded/textArchives/0433.json");
    std::fs::create_dir_all(json.parent().unwrap()).unwrap();
    std::fs::write(&json, "{ not json").unwrap();
    // The JSON was written after the member, so it is the source.
    let err = st.text_archive(433).unwrap_err().to_string();
    assert!(
        err.contains("textArchives/0433.json: text archive JSON:"),
        "{err}"
    );
    assert!(!st.is_dirty(text_id(433)));
}

#[test]
fn location_names_read_the_member_even_when_the_json_differs() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    st.edit_text_archive(433, "rename", |d| {
        d.messages[1].text = "Zweiblattdorf".into()
    })
    .unwrap();
    st.save().unwrap();
    let names = Project::open(s.dir.path())
        .unwrap()
        .location_names()
        .unwrap();
    assert_eq!(names[1], "Zweiblattdorf");
}

#[test]
fn store_names_include_an_unsaved_message() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert_eq!(places::location_bank(st.project()), 433);
    st.edit_text_archive(433, "Ort angelegt", |d| {
        let mut message = d.messages[2].clone();
        message.text = "Neuheim".into();
        d.messages.push(message);
    })
    .unwrap();
    assert_eq!(
        places::location_names(&mut st).unwrap(),
        ["Nowhere", "Twinleaf Town", "Sandgem Town", "Neuheim"]
    );
    // Not saved: the member on disk still has three names.
    assert_eq!(st.project().location_names().unwrap().len(), 3);
}

/// A JSON another tool left half-edited, newer than the member.
fn break_text_json(root: &std::path::Path) -> std::path::PathBuf {
    let json = root.join("expanded/textArchives/0433.json");
    std::fs::create_dir_all(json.parent().unwrap()).unwrap();
    std::fs::write(&json, "{ \"key\": 0, \"messages\": [").unwrap();
    json
}

#[test]
fn undecodable_text_json_is_stale_and_accept_disk_as_base_recovers() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    st.edit_text_archive(433, "rename", |d| {
        d.messages[1].text = "Zweiblattdorf".into()
    })
    .unwrap();
    st.edit_map(1, "cell", |m| {
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
    let json = break_text_json(s.dir.path());
    let map_member = s.dir.path().join("unpacked/maps/0001");
    let map_before = std::fs::read(&map_member).unwrap();

    // Stale, not a decode error; as for any stale asset nothing at all is written.
    assert!(matches!(
        st.save().unwrap_err(),
        KernelError::Stale(id) if id == text_id(433)
    ));
    assert_eq!(std::fs::read(&map_member).unwrap(), map_before);
    assert_eq!(st.dirty(), vec![map_id(1), text_id(433)]);

    // Accepting the broken file keeps the asset dirty; the save overwrites both files.
    st.accept_disk_as_base(text_id(433)).unwrap();
    assert!(st.is_dirty(text_id(433)));
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![map_id(1), text_id(433)]);
    let doc = g4_kernel::text::TextArchiveDoc::from_json(&std::fs::read(&json).unwrap(), "en_US")
        .unwrap();
    assert_eq!(doc.messages[1].text, "Zweiblattdorf");
    let member = std::fs::read(s.dir.path().join("unpacked/textArchives/0433")).unwrap();
    let doc = g4_kernel::text::TextArchiveDoc::from_member(&member, "en_US").unwrap();
    assert_eq!(doc.messages[1].text, "Zweiblattdorf");
    assert!(st.dirty().is_empty());
}

#[test]
fn reload_of_an_undecodable_text_archive_fails_and_drops_its_index_entry() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert_eq!(st.search_text("twinleaf", 10).unwrap().len(), 1);
    break_text_json(s.dir.path());
    let err = st.reload_from_disk(text_id(433)).unwrap_err().to_string();
    assert!(err.contains("textArchives/0433.json"), "{err}");
    assert!(st.search_text("twinleaf", 10).unwrap().is_empty());
}

// ---- Data kinds (personal, learnset, evolution, move, item, encounter, trainer texts) ----

use g4_formats::data::encounter::Encounters;
use g4_formats::data::evolution::EvoEntry;
use g4_formats::data::learnset::LearnEntry;
use g4_formats::data::record::RawRecord;
use g4_kernel::store::ChangeId;

/// A store over the synthetic project with nothing unpacked yet.
fn data_store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    (s, Store::new(Arc::new(p)))
}

/// One editable data kind of the fixture, for the four tests every kind runs.
struct DataCase {
    kind: AssetKind,
    /// Members of the kind in the fixture.
    count: usize,
    /// The member the edit tests change.
    index: u16,
    /// The member the lock test reads with a wrong length, and the bytes it writes there
    /// first (`None`: the fixture's member is already too long).
    locked: u16,
    wrong: Option<Vec<u8>>,
    /// The lock reason, verbatim.
    reason: &'static str,
    /// Reads the member through the typed getter, serialized.
    read: fn(&mut Store, u16) -> Result<Vec<u8>, KernelError>,
    /// One edit through the typed edit function of the store, labelled "edit".
    edit: fn(&mut Store, u16) -> Result<Option<ChangeId>, KernelError>,
}

impl DataCase {
    fn id(&self, index: u16) -> AssetId {
        AssetId {
            kind: self.kind,
            index,
        }
    }
}

fn personal_case() -> DataCase {
    DataCase {
        kind: AssetKind::Personal,
        count: common::PT_SPECIES_MEMBERS,
        index: 3,
        locked: 3,
        wrong: Some(vec![0; 45]),
        reason: "personal data 0003: 45 bytes, expected 44",
        read: |st, i| st.personal(i).map(|p| p.write()),
        edit: |st, i| st.edit_personal(i, "edit", |p| p.set_hp(p.hp() + 1)),
    }
}

fn learnset_case() -> DataCase {
    DataCase {
        kind: AssetKind::Learnset,
        count: common::PT_SPECIES_MEMBERS,
        index: 3,
        locked: 4,
        wrong: Some(vec![0x01, 0x02, 0x02]),
        reason: "learnset 0004: 3 bytes, expected 4",
        read: |st, i| st.learnset(i).map(|l| l.write()),
        edit: |st, i| {
            st.edit_learnset(i, "edit", |l| {
                l.entries.push(LearnEntry {
                    move_id: 3,
                    level: 9,
                })
            })
        },
    }
}

fn evolution_case() -> DataCase {
    DataCase {
        kind: AssetKind::Evolution,
        count: common::PT_SPECIES_MEMBERS,
        index: 1,
        locked: 2,
        wrong: Some(vec![0; 40]),
        reason: "evolution data 0002: 40 bytes, expected 44",
        read: |st, i| st.evolution(i).map(|e| e.write()),
        edit: |st, i| {
            st.edit_evolution(i, "edit", |e| {
                e.set_entry(
                    0,
                    EvoEntry {
                        method: 4,
                        param: 30,
                        target: 3,
                    },
                )
            })
        },
    }
}

fn move_case() -> DataCase {
    DataCase {
        kind: AssetKind::Move,
        count: common::PT_MOVE_MEMBERS,
        index: 1,
        locked: 2,
        wrong: Some(vec![0; 14]),
        reason: "move 0002: 14 bytes, expected 16",
        read: |st, i| st.move_data(i).map(|m| m.write()),
        edit: |st, i| st.edit_move(i, "edit", |m| m.set_power(m.power() + 5)),
    }
}

fn item_case() -> DataCase {
    DataCase {
        kind: AssetKind::Item,
        count: common::PT_ITEM_MEMBERS,
        index: 1,
        locked: common::LONG_ITEM,
        wrong: None,
        reason: "item 0017: 36 bytes, expected 34",
        read: |st, i| st.item(i).map(|it| it.write()),
        edit: |st, i| st.edit_item(i, "edit", |it| it.set_price(4321)),
    }
}

fn encounter_case() -> DataCase {
    DataCase {
        kind: AssetKind::Encounter,
        count: common::PT_ENCOUNTER_MEMBERS,
        index: 1,
        locked: 0,
        wrong: Some(vec![0; 196]),
        reason: "encounter file 0000: 196 bytes, expected 424",
        read: |st, i| st.encounter(i).map(|e| e.write()),
        edit: |st, i| {
            st.edit_encounter(i, "edit", |e| match e {
                Encounters::Pt(e) => e.set_grass_rate(25),
                Encounters::Hgss(_) => {}
            })
        },
    }
}

fn trainer_text_table_case() -> DataCase {
    DataCase {
        kind: AssetKind::TrainerTextTable,
        count: 1,
        index: 0,
        locked: 0,
        wrong: Some(vec![1, 0, 0, 0, 1, 0]),
        reason: "trainer text table 0000: 6 bytes, expected 8",
        read: |st, i| st.trainer_text_table(i).map(|t| t.write()),
        edit: |st, i| st.edit_trainer_text_table(i, "edit", |t| t.entries.push((2, 0))),
    }
}

fn trainer_text_offsets_case() -> DataCase {
    DataCase {
        kind: AssetKind::TrainerTextOffsets,
        count: 1,
        index: 0,
        locked: 0,
        wrong: Some(vec![0; 5]),
        reason: "trainer text offsets 0000: 5 bytes, expected 6",
        read: |st, i| st.trainer_text_offsets(i).map(|o| o.write()),
        edit: |st, i| st.edit_trainer_text_offsets(i, "edit", |o| o.offsets[1] = 8),
    }
}

fn check_loads_byte_identical(case: DataCase) {
    let (_s, mut st) = data_store();
    let id = case.id(case.index);
    assert_eq!(st.current_bytes(id).unwrap(), None, "nothing loaded yet");
    assert_eq!(st.saved_bytes(id), None, "nothing loaded yet");
    let bytes = (case.read)(&mut st, case.index).unwrap();
    assert_eq!(st.saved_bytes(id), Some(bytes.as_slice()));
    let path = st.project().member_path(case.kind, case.index);
    assert_eq!(bytes, std::fs::read(path).unwrap());
    assert_eq!(st.current_bytes(id).unwrap(), Some(bytes));
    assert_eq!(st.locked(id), None);
    assert!(!st.is_dirty(id));
    assert_eq!(st.member_count(case.kind).unwrap(), case.count);
}

fn check_edit_records_undo_and_dirty(case: DataCase) {
    let (_s, mut st) = data_store();
    let id = case.id(case.index);
    let before = (case.read)(&mut st, case.index).unwrap();
    assert!((case.edit)(&mut st, case.index).unwrap().is_some());
    let after = (case.read)(&mut st, case.index).unwrap();
    assert_ne!(after, before);
    assert_eq!(
        st.saved_bytes(id),
        Some(before.as_slice()),
        "the baseline stays"
    );
    assert_eq!(st.dirty(), vec![id]);
    assert_eq!(st.history(), (vec!["edit".to_owned()], vec![]));
    assert_eq!(st.undo().unwrap(), vec![id]);
    assert_eq!((case.read)(&mut st, case.index).unwrap(), before);
    assert!(st.dirty().is_empty());
    assert_eq!(st.redo().unwrap(), vec![id]);
    assert_eq!((case.read)(&mut st, case.index).unwrap(), after);
    assert_eq!(st.dirty(), vec![id]);
}

fn check_save_writes_member_atomically(case: DataCase) {
    let (_s, mut st) = data_store();
    let id = case.id(case.index);
    (case.edit)(&mut st, case.index).unwrap();
    let edited = st.current_bytes(id).unwrap().unwrap();
    let path = st.project().member_path(case.kind, case.index);
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![id]);
    assert_eq!(std::fs::read(&path).unwrap(), edited);
    assert_eq!(st.saved_bytes(id), Some(edited.as_slice()));
    assert!(dot_entries(path.parent().unwrap()).is_empty());
    assert!(!st.is_dirty(id));
    // A file changed on disk after the save is not overwritten by the next one (undo makes
    // the asset dirty again).
    assert_eq!(st.undo().unwrap(), vec![id]);
    std::fs::write(&path, &edited[..edited.len() - 1]).unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(s) if s == id));
}

fn check_wrong_length(case: DataCase) {
    let (_s, mut st) = data_store();
    let id = case.id(case.locked);
    st.project().ensure_unpacked(case.kind).unwrap();
    let path = st.project().member_path(case.kind, case.locked);
    let good = std::fs::read(st.project().member_path(case.kind, case.index)).unwrap();
    if let Some(wrong) = &case.wrong {
        std::fs::write(&path, wrong).unwrap();
    }
    let raw = std::fs::read(&path).unwrap();
    let err = (case.read)(&mut st, case.locked).unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { asset, reason } if *asset == id && reason == case.reason),
        "{err}"
    );
    assert_eq!(st.locked(id), Some(case.reason));
    assert_eq!(st.current_bytes(id).unwrap(), Some(raw.clone()));
    let err = (case.edit)(&mut st, case.locked).unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { asset, .. } if *asset == id),
        "{err}"
    );
    assert!(!st.is_dirty(id));
    assert!(st.save().unwrap().written.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), raw);
    // A member of the right length on disk unlocks it again.
    std::fs::write(&path, &good).unwrap();
    st.reload_from_disk(id).unwrap();
    assert_eq!((case.read)(&mut st, case.locked).unwrap(), good);
    assert_eq!(st.locked(id), None);
}

macro_rules! data_kind_tests {
    ($($module:ident => $case:expr;)*) => {$(
        mod $module {
            use super::*;

            #[test]
            fn loads_byte_identical() {
                check_loads_byte_identical($case);
            }

            #[test]
            fn edit_records_undo_and_dirty() {
                check_edit_records_undo_and_dirty($case);
            }

            #[test]
            fn save_writes_member_atomically() {
                check_save_writes_member_atomically($case);
            }

            #[test]
            fn wrong_length_member_is_locked_with_reason_and_raw_bytes() {
                check_wrong_length($case);
            }
        }
    )*};
}

data_kind_tests! {
    personal => personal_case();
    learnset => learnset_case();
    evolution => evolution_case();
    move_data => move_case();
    item => item_case();
    encounter => encounter_case();
    trainer_text_table => trainer_text_table_case();
    trainer_text_offsets => trainer_text_offsets_case();
}

/// Every member of every editable data kind of a vanilla project loads through the store,
/// unlocked and byte-identical to its file. Reads only: DSPRE unpacked every folder, so the
/// store unpacks nothing.
#[test]
fn vanilla_data_members_load_unlocked_and_byte_identical() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let mut st = Store::new(project.clone());
    for case in [
        personal_case(),
        learnset_case(),
        evolution_case(),
        move_case(),
        item_case(),
        encounter_case(),
        trainer_text_table_case(),
        trainer_text_offsets_case(),
    ] {
        assert!(project.unpacked_dir(case.kind).is_dir(), "{:?}", case.kind);
        let count = st.member_count(case.kind).unwrap();
        assert!(count > 0, "{:?} has no members", case.kind);
        for index in 0..count as u16 {
            let id = case.id(index);
            let bytes = (case.read)(&mut st, index).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(
                bytes,
                std::fs::read(project.member_path(case.kind, index)).unwrap(),
                "{id}"
            );
            assert_eq!(st.locked(id), None, "{id}");
        }
        eprintln!("{:?}: {count} members", case.kind);
    }
    assert!(st.dirty().is_empty());
}

#[test]
fn trainer_properties_and_party_stay_read_only() {
    let (s, mut st) = data_store();
    for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
        assert_eq!(st.member_count(kind).unwrap(), common::PT_TRAINER_MEMBERS);
        assert!(matches!(
            st.reload_from_disk(AssetId { kind, index: 0 }),
            Err(KernelError::NotEditable(k)) if k == kind
        ));
    }
    assert!(!s.dir.path().join("unpacked/trainerProperties").exists());
}

#[test]
fn moves_and_tm_bits_are_copied_in_one_transaction() {
    let (_s, mut st) = data_store();
    let label = "Ivysaur: TM/VM und Lernset wie Bulbasaur";
    let ((), change) = st
        .transaction(label, |tx| {
            let source = tx.personal(1)?.clone();
            let mut learnset = tx.learnset(1)?.clone();
            learnset.entries.push(LearnEntry {
                move_id: 3,
                level: 20,
            });
            tx.edit_personal(2, |p| {
                for n in 0..100 {
                    p.set_tm(n, source.tm(n));
                }
            })?;
            tx.edit_learnset(2, |l| l.entries = learnset.entries)?;
            // Inside the transaction the getter sees the edited value.
            assert!(tx.personal(2)?.tm(0));
            Ok(())
        })
        .unwrap();
    assert!(change.is_some());
    assert_eq!(st.history(), (vec![label.to_owned()], vec![]));
    let personal = |index| AssetId {
        kind: AssetKind::Personal,
        index,
    };
    let learnset = |index| AssetId {
        kind: AssetKind::Learnset,
        index,
    };
    assert_eq!(st.dirty(), vec![personal(2), learnset(2)]);
    assert_eq!(
        st.personal(2).unwrap().tm_bits(),
        st.personal(1).unwrap().tm_bits()
    );
    assert_eq!(st.learnset(2).unwrap().entries.len(), 3);
    assert_eq!(st.undo().unwrap(), vec![personal(2), learnset(2)]);
    assert!(st.dirty().is_empty());
    assert!(!st.personal(2).unwrap().tm(0));
}

// ---- Trainers: one asset over the pair of a properties and a party member ----

use g4_formats::data::trainer::PartyFormat;

fn trainer_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Trainer,
        index,
    }
}

/// The store's canonical bytes of a trainer: `u32 len(props) | props | party`.
fn pair(props: &[u8], party: &[u8]) -> Vec<u8> {
    let mut out = (props.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(props);
    out.extend_from_slice(party);
    out
}

fn trainer_files(st: &Store, index: u16) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        st.project()
            .member_path(AssetKind::TrainerProperties, index),
        st.project().member_path(AssetKind::TrainerParty, index),
    )
}

#[test]
fn trainer_loads_byte_identical_from_the_pair() {
    let (_s, mut st) = data_store();
    let id = trainer_id(2);
    assert_eq!(st.current_bytes(id).unwrap(), None, "nothing loaded yet");
    let doc = st.trainer(2).unwrap().clone();
    assert_eq!(doc.class, 0);
    assert_eq!(
        doc.format,
        PartyFormat {
            moves: true,
            items: true
        }
    );
    assert_eq!((doc.items, doc.ai, doc.battle_type), ([28, 0, 0, 0], 7, 0));
    let levels: Vec<u16> = doc.party.iter().map(|m| m.level).collect();
    assert_eq!(levels, [10, 12, 15]);
    assert_eq!(doc.party[2].moves, Some([20, 21, 22, 0]));
    assert_eq!(doc.party[0].item, Some(25));
    let (props, party) = trainer_files(&st, 2);
    let expected = pair(
        &std::fs::read(props).unwrap(),
        &std::fs::read(party).unwrap(),
    );
    assert_eq!(expected.len(), 4 + 20 + 56);
    assert_eq!(st.current_bytes(id).unwrap(), Some(expected.clone()));
    assert_eq!(st.saved_bytes(id), Some(expected.as_slice()));
    assert_eq!(st.locked(id), None);
    assert!(!st.is_dirty(id));
    // A plain trainer of the fixture loads too.
    assert_eq!(st.trainer(0).unwrap().party.len(), 1);
    assert_eq!(
        st.member_count(AssetKind::Trainer).unwrap(),
        common::PT_TRAINER_MEMBERS
    );
}

#[test]
fn trainer_count_is_the_smaller_of_both_archives() {
    let (_s, mut st) = data_store();
    st.project()
        .ensure_unpacked(AssetKind::TrainerProperties)
        .unwrap();
    st.project()
        .ensure_unpacked(AssetKind::TrainerParty)
        .unwrap();
    let (props, _) = trainer_files(&st, 3);
    std::fs::write(props, [0u8; 20]).unwrap();
    assert_eq!(
        st.member_count(AssetKind::TrainerProperties).unwrap(),
        common::PT_TRAINER_MEMBERS + 1
    );
    assert_eq!(
        st.member_count(AssetKind::Trainer).unwrap(),
        common::PT_TRAINER_MEMBERS
    );
    // A properties member without its party is missing, named by its party file (an
    // absolute path, with the platform's separator).
    let err = st.trainer(3).unwrap_err().to_string();
    let party = std::path::Path::new("trainerParty").join("0003");
    assert!(err.contains(&party.display().to_string()), "{err}");
}

#[test]
fn edit_trainer_records_one_undo_step_for_both_files() {
    let (_s, mut st) = data_store();
    let id = trainer_id(2);
    st.trainer(2).unwrap();
    let before = st.current_bytes(id).unwrap().unwrap();
    let change = st
        .edit_trainer(2, "Trainer 2: Team", |t| {
            t.class = 1;
            t.format.items = false;
            t.party.pop();
        })
        .unwrap();
    assert!(change.is_some());
    assert_eq!(st.history(), (vec!["Trainer 2: Team".to_owned()], vec![]));
    assert_eq!(st.dirty(), vec![id]);
    let after = st.current_bytes(id).unwrap().unwrap();
    // Properties: flags 1 (moves only), class 1, party size 2; party: two 16-byte entries.
    assert_eq!(&after[..4], &20u32.to_le_bytes());
    assert_eq!((after[4], after[5], after[7]), (1, 1, 2));
    assert_eq!(after.len(), 4 + 20 + 32);
    assert_eq!(st.undo().unwrap(), vec![id]);
    assert_eq!(st.current_bytes(id).unwrap(), Some(before));
    assert!(st.dirty().is_empty());
    assert_eq!(st.redo().unwrap(), vec![id]);
    assert_eq!(st.current_bytes(id).unwrap(), Some(after));
    // An edit that leaves the bytes as they are records nothing.
    assert_eq!(st.edit_trainer(2, "noop", |_| {}).unwrap(), None);
    // More than six Pokémon is refused and changes nothing.
    let err = st
        .edit_trainer(2, "seven", |t| t.party = vec![t.party[0]; 7])
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Format(FormatError::OutOfRange { .. })),
        "{err}"
    );
    assert_eq!(st.trainer(2).unwrap().party.len(), 2);
    assert_eq!(st.history().0.len(), 1);
}

#[test]
fn trainer_edits_join_a_transaction_with_other_assets() {
    let (_s, mut st) = data_store();
    let ((), change) = st
        .transaction("Trainer 2 umbenannt", |tx| {
            let level = tx.trainer(2)?.party[0].level;
            tx.edit_trainer(2, |t| t.party[0].level = level + 1)?;
            // Inside the transaction the getter sees the edit.
            assert_eq!(tx.trainer(2)?.party[0].level, 11);
            tx.edit_trainer_text_offsets(0, |o| o.offsets[0] = 4)?;
            Ok(())
        })
        .unwrap();
    assert!(change.is_some());
    let offsets = AssetId {
        kind: AssetKind::TrainerTextOffsets,
        index: 0,
    };
    assert_eq!(st.dirty(), vec![offsets, trainer_id(2)]);
    assert_eq!(st.undo().unwrap(), vec![trainer_id(2), offsets]);
    assert_eq!(st.trainer(2).unwrap().party[0].level, 10);
    assert!(st.dirty().is_empty());
}

#[test]
fn save_writes_both_members_atomically_and_stale_checks_both() {
    let (_s, mut st) = data_store();
    let id = trainer_id(2);
    st.edit_trainer(2, "edit", |t| {
        t.ai = 1;
        t.party[1].level = 50;
    })
    .unwrap();
    let edited = st.current_bytes(id).unwrap().unwrap();
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![id]);
    let (props, party) = trainer_files(&st, 2);
    let (p, q) = (
        std::fs::read(&props).unwrap(),
        std::fs::read(&party).unwrap(),
    );
    assert_eq!(pair(&p, &q), edited);
    assert_eq!(p[12], 1, "AI in the properties");
    assert_eq!(q[18 + 2], 50, "level of the second Pokémon in the party");
    assert_eq!(st.saved_bytes(id), Some(edited.as_slice()));
    assert!(dot_entries(props.parent().unwrap()).is_empty());
    assert!(dot_entries(party.parent().unwrap()).is_empty());
    assert!(!st.is_dirty(id));
    assert!(st.save().unwrap().written.is_empty());

    // Undo makes the trainer dirty again; a change to either file blocks the save.
    st.undo().unwrap();
    for path in [&party, &props] {
        let original = std::fs::read(path).unwrap();
        let mut changed = original.clone();
        changed[2] ^= 0xFF;
        std::fs::write(path, &changed).unwrap();
        assert!(matches!(st.save().unwrap_err(), KernelError::Stale(s) if s == id));
        assert_eq!(std::fs::read(&props).unwrap().len(), 20);
        std::fs::write(path, &original).unwrap();
    }
    assert_eq!(std::fs::read(&props).unwrap(), p, "nothing written");
    // A deleted party member is a change on disk as well.
    std::fs::remove_file(&party).unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(s) if s == id));
    std::fs::write(&party, &q).unwrap();
    assert_eq!(st.save().unwrap().written, vec![id]);
    let original = pair(&common::TRAINER_2_PROPS, &common::TRAINER_2_PARTY);
    assert_eq!(
        pair(
            &std::fs::read(&props).unwrap(),
            &std::fs::read(&party).unwrap()
        ),
        original
    );

    // accept_disk_as_base takes both files as the base.
    st.redo().unwrap();
    std::fs::write(&props, [0u8; 20]).unwrap();
    std::fs::write(&party, [0u8; 8]).unwrap();
    st.accept_disk_as_base(id).unwrap();
    assert_eq!(st.saved_bytes(id), Some(pair(&[0; 20], &[0; 8]).as_slice()));
    assert!(st.is_dirty(id));
    assert_eq!(st.save().unwrap().written, vec![id]);
    assert_eq!(
        pair(
            &std::fs::read(&props).unwrap(),
            &std::fs::read(&party).unwrap()
        ),
        edited
    );
}

#[test]
fn mismatched_pair_is_locked_with_reason() {
    let (_s, mut st) = data_store();
    st.project()
        .ensure_unpacked(AssetKind::TrainerProperties)
        .unwrap();
    st.project()
        .ensure_unpacked(AssetKind::TrainerParty)
        .unwrap();
    let (props, party) = trainer_files(&st, 1);
    let id = trainer_id(1);

    // Moves on, one Pokémon: 16 bytes expected, the party file has 20.
    let mut p = std::fs::read(&props).unwrap();
    p[0] = 1;
    std::fs::write(&props, &p).unwrap();
    let mut long = vec![0, 0, 5, 0, 1, 0, 1, 0];
    long.resize(20, 0);
    std::fs::write(&party, &long).unwrap();
    let reason = "trainer 0001: party 20 bytes, expected 16";
    // Shown, but read-only.
    assert_eq!(st.trainer(1).unwrap().party[0].moves, Some([1, 0, 0, 0]));
    assert_eq!(st.locked(id), Some(reason));
    let err = st.edit_trainer(1, "edit", |t| t.class = 9).unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { asset, reason: r } if *asset == id && r == reason),
        "{err}"
    );
    let err = st
        .transaction("tx", |tx| tx.edit_trainer(1, |t| t.class = 9))
        .unwrap_err();
    assert!(matches!(err, KernelError::ReadOnly { .. }), "{err}");
    assert!(!st.is_dirty(id));
    assert!(st.save().unwrap().written.is_empty());
    assert_eq!(std::fs::read(&party).unwrap(), long);

    // Properties of the wrong length: not decodable, the raw pair is kept.
    std::fs::write(&props, &p[..19]).unwrap();
    st.reload_from_disk(id).unwrap();
    let reason = "trainer 0001: properties 19 bytes, expected 20";
    assert_eq!(st.locked(id), Some(reason));
    let err = st.trainer(1).unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { reason: r, .. } if r == reason),
        "{err}"
    );
    assert_eq!(st.current_bytes(id).unwrap(), Some(pair(&p[..19], &long)));

    // A canonical pair on disk unlocks it again.
    std::fs::write(&props, &p).unwrap();
    std::fs::write(&party, &long[..16]).unwrap();
    st.reload_from_disk(id).unwrap();
    assert_eq!(st.locked(id), None);
    assert!(
        st.edit_trainer(1, "edit", |t| t.class = 9)
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_trainer_the_store_locks_stays_locked_with_its_reason() {
    let (_s, mut st) = data_store();
    let id = trainer_id(1);
    // Locking before the first load applies on load, and survives a reload.
    st.lock(id, "placeholder for the sight script slot")
        .unwrap();
    assert_eq!(st.trainer(1).unwrap().party.len(), 1);
    assert_eq!(st.locked(id), Some("placeholder for the sight script slot"));
    let err = st.edit_trainer(1, "edit", |t| t.class = 9).unwrap_err();
    assert!(
        matches!(err, KernelError::ReadOnly { asset, .. } if asset == id),
        "{err}"
    );
    st.reload_from_disk(id).unwrap();
    assert_eq!(st.locked(id), Some("placeholder for the sight script slot"));
    // Locking a loaded trainer drops its unsaved edit and its history.
    st.edit_trainer(2, "edit", |t| t.class = 9).unwrap();
    st.lock(trainer_id(2), "placeholder").unwrap();
    assert_eq!(st.locked(trainer_id(2)), Some("placeholder"));
    assert!(st.dirty().is_empty());
    assert!(st.undo().unwrap().is_empty());
    assert_eq!(st.trainer(2).unwrap().class, 0);
}

/// Every trainer of a vanilla project loads through the store as one asset, unlocked and
/// byte-identical to its two files. Reads only.
#[test]
fn vanilla_trainers_load_unlocked_through_the_store() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let mut st = Store::new(project.clone());
    let count = st.member_count(AssetKind::Trainer).unwrap();
    let expected = match project.family {
        Family::Pt => 928,
        _ => 738,
    };
    assert_eq!(count, expected);
    for index in 0..count as u16 {
        let id = trainer_id(index);
        st.trainer(index).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(st.locked(id), None, "{id}");
        let props = std::fs::read(project.member_path(AssetKind::TrainerProperties, index));
        let party = std::fs::read(project.member_path(AssetKind::TrainerParty, index));
        assert_eq!(
            st.current_bytes(id).unwrap(),
            Some(pair(&props.unwrap(), &party.unwrap())),
            "{id}"
        );
    }
    assert!(st.dirty().is_empty());
    eprintln!("{count} trainers load through the store");
}

// --- tombstones (3d.4) ---

use g4_kernel::delete::{
    DeleteTarget, no_tombstone_for_binaries, no_tombstone_for_files, not_a_text_project,
};
use g4_kernel::store::HEADER_TABLE;
use g4_kernel::textproject::Art;

const TRAINER_1_JSON: &str = "data/trainers/0001.json";
const ENVELOPE_END: &[u8] = b",\n  \"deleted\": true\n}\n";

fn text_store(fx: &common::TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// The deleted trainers and encounters (the arts these tests delete in), unsaved steps
/// included, sorted.
fn deleted(st: &Store) -> Vec<DeleteTarget> {
    let trainers = st.deleted_in(Art::Trainers).into_iter();
    let encounters = st.deleted_in(Art::Encounters).into_iter();
    trainers
        .map(|index| DeleteTarget::Trainer { index })
        .chain(encounters.map(|index| DeleteTarget::Encounter { index }))
        .collect()
}

/// Every file below `data/`, relative to the root, with its bytes.
fn data_files(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn go(
        root: &std::path::Path,
        rel: &std::path::Path,
        out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap();
            let r = rel.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                go(root, &r, out);
            } else {
                out.insert(r.clone(), std::fs::read(root.join(&r)).unwrap());
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    go(root, std::path::Path::new("data"), &mut out);
    out
}

fn changed_files(
    a: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    b: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = a
        .keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

#[test]
fn set_deleted_is_one_undo_step_and_save_writes_the_envelope() {
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let trainer = DeleteTarget::Trainer { index: 1 };
    let before = fx.read(TRAINER_1_JSON);
    st.trainer(1).unwrap();
    let member = st.current_bytes(trainer_id(1)).unwrap();
    assert!(member.is_some());
    assert!(!st.is_deleted(trainer));

    let depth = st.undo_depth();
    let ((), change) = st
        .transaction("Gelöscht: Trainer 0001", |tx| {
            tx.set_deleted(trainer, true)
        })
        .unwrap();
    assert!(change.is_some());
    assert_eq!(st.undo_depth(), depth + 1);
    assert!(st.is_deleted(trainer));
    assert_eq!(deleted(&st), [trainer]);
    assert_eq!(st.dirty(), [trainer_id(1)]);
    // Setting it again records nothing.
    let ((), again) = st
        .transaction("noch mal", |tx| tx.set_deleted(trainer, true))
        .unwrap();
    assert_eq!(again, None);
    assert_eq!(st.undo_depth(), depth + 1);
    // Nothing is written before the save.
    assert_eq!(fx.read(TRAINER_1_JSON), before);

    st.save().unwrap();
    let saved = fx.read(TRAINER_1_JSON);
    assert!(
        saved.ends_with(ENVELOPE_END),
        "{}",
        String::from_utf8_lossy(&saved)
    );
    // Content and members stay as they were.
    assert_eq!(
        saved[..saved.len() - ENVELOPE_END.len()],
        before[..before.len() - b"\n}\n".len()]
    );
    assert_eq!(st.current_bytes(trainer_id(1)).unwrap(), member);
    assert!(st.dirty().is_empty());

    st.undo().unwrap();
    assert!(!st.is_deleted(trainer));
    assert!(deleted(&st).is_empty());
    assert_eq!(st.dirty(), [trainer_id(1)]);
    st.save().unwrap();
    assert_eq!(fx.read(TRAINER_1_JSON), before);
    assert!(st.dirty().is_empty());

    // Redo deletes it again.
    st.redo().unwrap();
    assert!(st.is_deleted(trainer));
    st.save().unwrap();
    assert_eq!(fx.read(TRAINER_1_JSON), saved);
}

/// The encounter rate of encounter file 0 (Platinum).
fn grass_rate(st: &mut Store) -> u32 {
    match st.encounter(0).unwrap() {
        Encounters::Pt(e) => e.grass_rate(),
        Encounters::Hgss(_) => panic!("the synthetic project is Platinum"),
    }
}

#[test]
fn undo_restores_the_tombstone_and_the_bytes_in_order() {
    const JSON: &str = "data/encounters/0000.json";
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let encounter = DeleteTarget::Encounter { index: 0 };
    let original = fx.read(JSON);
    let member = fx.project().member_path(AssetKind::Encounter, 0);
    let member_before = std::fs::read(&member).ok();
    let rate = grass_rate(&mut st);
    let depth = st.undo_depth();

    // (1) Deleted and saved: the envelope, the member untouched.
    st.transaction("Gelöscht: Begegnungsdatei 0000", |tx| {
        tx.set_deleted(encounter, true)
    })
    .unwrap();
    st.save().unwrap();
    let deleted = fx.read(JSON);
    assert!(deleted.ends_with(ENVELOPE_END));
    assert_eq!(std::fs::read(&member).ok(), member_before);

    // (2) The rate edited in a step of its own and saved: envelope and new rate.
    st.edit_encounter(0, "Rate", |e| match e {
        Encounters::Pt(e) => e.set_grass_rate(rate + 5),
        Encounters::Hgss(_) => {}
    })
    .unwrap()
    .unwrap();
    st.save().unwrap();
    let edited = fx.read(JSON);
    assert!(edited.ends_with(ENVELOPE_END));
    assert_ne!(edited, deleted);
    assert_eq!(grass_rate(&mut st), rate + 5);
    let value: serde_json::Value = serde_json::from_slice(&edited).unwrap();
    assert_eq!(value["data"]["grass_rate"], rate + 5, "{value}");

    // (3) Undo takes the rate back, the tombstone stays.
    st.undo().unwrap();
    assert_eq!(grass_rate(&mut st), rate);
    assert!(st.is_deleted(encounter));
    st.save().unwrap();
    assert_eq!(fx.read(JSON), deleted);

    // (4) Undo takes the tombstone back: byte-equal to before the delete.
    st.undo().unwrap();
    assert!(!st.is_deleted(encounter));
    assert_eq!(st.undo_depth(), depth);
    st.save().unwrap();
    assert_eq!(fx.read(JSON), original);
    assert_eq!(std::fs::read(&member).ok(), member_before);
}

#[test]
fn an_envelope_removed_outside_is_not_written_back() {
    const JSON: &str = "data/encounters/0000.json";
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let encounter = DeleteTarget::Encounter { index: 0 };
    let original = fx.read(JSON);
    // Trainer 1 is deleted on disk before the store opens; it is never loaded.
    let trainer = DeleteTarget::Trainer { index: 1 };
    let mut other = text_store(&fx);
    other
        .transaction("Gelöscht", |tx| tx.set_deleted(trainer, true))
        .unwrap();
    other.save().unwrap();
    drop(other);
    assert!(st.is_deleted(trainer));

    st.transaction("Gelöscht", |tx| tx.set_deleted(encounter, true))
        .unwrap();
    st.save().unwrap();
    assert!(fx.read(JSON).ends_with(ENVELOPE_END));

    // Outside the app (a `git checkout`): both envelopes go.
    fx.write(JSON, &original);
    let trainer_json = fx.read(TRAINER_1_JSON);
    let without = [
        &trainer_json[..trainer_json.len() - ENVELOPE_END.len()],
        b"\n}\n",
    ]
    .concat();
    fx.write(TRAINER_1_JSON, &without);
    assert!(!st.is_deleted(encounter));
    assert!(!st.is_deleted(trainer));
    assert!(deleted(&st).is_empty());
    assert!(st.snapshot_overlay().tombstones.is_empty());
    assert!(st.dirty().is_empty());

    // An edit and a save do not bring the envelopes back.
    st.edit_encounter(0, "Rate", |e| match e {
        Encounters::Pt(e) => e.set_grass_rate(e.grass_rate() + 1),
        Encounters::Hgss(_) => {}
    })
    .unwrap()
    .unwrap();
    st.edit_trainer(1, "KI", |t| t.ai = 7).unwrap().unwrap();
    st.save().unwrap();
    assert!(!fx.read(JSON).ends_with(ENVELOPE_END));
    assert!(!fx.read(TRAINER_1_JSON).ends_with(ENVELOPE_END));
    assert!(!text_store(&fx).is_deleted(encounter));
    assert!(!text_store(&fx).is_deleted(trainer));
}

#[test]
fn squash_since_carries_the_tombstones() {
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let trainer = DeleteTarget::Trainer { index: 1 };
    let original = fx.read(TRAINER_1_JSON);
    st.trainer(1).unwrap();
    let bytes0 = st.current_bytes(trainer_id(1)).unwrap();

    // Step A edits the trainer; step B edits it again and deletes it in one transaction.
    st.edit_trainer(1, "A", |t| t.ai = 7).unwrap().unwrap();
    let bytes_a = st.current_bytes(trainer_id(1)).unwrap();
    st.transaction("B", |tx| {
        tx.edit_trainer(1, |t| t.ai = 9)?;
        tx.set_deleted(trainer, true)
    })
    .unwrap();
    let bytes_b = st.current_bytes(trainer_id(1)).unwrap();
    assert!(st.is_deleted(trainer));
    assert_eq!(st.history().0, ["A", "B"]);

    // A tombstone alone and the step that follows it: undone newest first.
    st.transaction("C", |tx| {
        tx.set_deleted(DeleteTarget::Trainer { index: 2 }, true)
    })
    .unwrap();
    assert_eq!(deleted(&st), [trainer, DeleteTarget::Trainer { index: 2 }]);

    assert_eq!(st.undo().unwrap(), [trainer_id(2)]);
    assert_eq!(deleted(&st), [trainer]);
    assert_eq!(st.undo().unwrap(), [trainer_id(1)]);
    assert!(!st.is_deleted(trainer));
    assert_eq!(st.current_bytes(trainer_id(1)).unwrap(), bytes_a);
    st.undo().unwrap();
    assert_eq!(st.current_bytes(trainer_id(1)).unwrap(), bytes0);
    assert!(st.dirty().is_empty());

    st.redo().unwrap();
    st.redo().unwrap();
    assert!(st.is_deleted(trainer));
    assert_eq!(st.current_bytes(trainer_id(1)).unwrap(), bytes_b);
    st.save().unwrap();
    let saved = fx.read(TRAINER_1_JSON);
    assert!(saved.ends_with(ENVELOPE_END));
    assert_ne!(
        saved[..saved.len() - ENVELOPE_END.len()],
        original[..original.len() - b"\n}\n".len()]
    );

    // Squashed with the steps around it, the tombstone is one step with them.
    let mark = st.undo_depth();
    st.transaction("D1", |tx| tx.set_deleted(trainer, false))
        .unwrap();
    st.edit_trainer(1, "D2", |t| t.ai = 0).unwrap();
    st.transaction("D3", |tx| {
        tx.set_deleted(DeleteTarget::Trainer { index: 0 }, true)
    })
    .unwrap();
    st.squash_since(mark, "D");
    assert_eq!(st.undo_depth(), mark + 1);
    assert!(!st.is_deleted(trainer));
    assert!(st.is_deleted(DeleteTarget::Trainer { index: 0 }));
    st.undo().unwrap();
    assert!(st.is_deleted(trainer));
    assert!(!st.is_deleted(DeleteTarget::Trainer { index: 0 }));
    assert_eq!(st.current_bytes(trainer_id(1)).unwrap(), bytes_b);
    // A tombstone set and taken back within the squash falls away.
    let mark = st.undo_depth();
    st.transaction("E1", |tx| {
        tx.set_deleted(DeleteTarget::Trainer { index: 2 }, true)
    })
    .unwrap();
    st.transaction("E2", |tx| {
        tx.set_deleted(DeleteTarget::Trainer { index: 2 }, false)
    })
    .unwrap();
    st.squash_since(mark, "E");
    assert_eq!(st.undo_depth(), mark);

    // Back to the start (steps A and B are left): the file byte for byte as exported.
    assert_eq!(st.undo_depth(), 2);
    st.undo().unwrap();
    st.undo().unwrap();
    assert_eq!(st.undo_depth(), 0);
    st.save().unwrap();
    assert_eq!(fx.read(TRAINER_1_JSON), original);
}

#[test]
fn a_tombstone_survives_reopen() {
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let encounter = DeleteTarget::Encounter { index: 0 };
    st.transaction("Gelöscht", |tx| tx.set_deleted(encounter, true))
        .unwrap();
    st.save().unwrap();
    drop(st);

    let mut st = text_store(&fx);
    assert!(st.is_deleted(encounter));
    assert_eq!(deleted(&st), [encounter]);
    assert!(st.dirty().is_empty());
    // Restoring it is a step like deleting it.
    st.transaction("Wiederhergestellt", |tx| tx.set_deleted(encounter, false))
        .unwrap();
    assert_eq!(
        st.dirty(),
        [AssetId {
            kind: AssetKind::Encounter,
            index: 0
        }]
    );
    st.save().unwrap();
    assert!(!text_store(&fx).is_deleted(encounter));
}

#[test]
fn set_deleted_refuses_the_dspre_layout() {
    let fx = common::TextFixture::dspre();
    let mut st = text_store(&fx);
    let err = st
        .transaction("Gelöscht", |tx| {
            tx.set_deleted(DeleteTarget::Trainer { index: 1 }, true)
        })
        .unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(why) if *why == not_a_text_project()),
        "{err}"
    );
    assert_eq!(st.undo_depth(), 0);
    assert!(deleted(&st).is_empty());

    // In a text project, kinds without an envelope are refused with their sentence.
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    for target in [
        DeleteTarget::Script { file: 0 },
        DeleteTarget::TextArchive { index: 0 },
    ] {
        let err = st
            .transaction("Gelöscht", |tx| tx.set_deleted(target, true))
            .unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(why) if *why == no_tombstone_for_files()),
            "{err}"
        );
    }
    // Textures and models are binary: no envelope either.
    let err = st
        .transaction("Gelöscht", |tx| {
            tx.set_deleted(DeleteTarget::MapTexture { index: 0 }, true)
        })
        .unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(why) if *why == no_tombstone_for_binaries()),
        "{err}"
    );
    // A failing transaction keeps a tombstone it staged out of the store.
    let err = st
        .transaction("zwei", |tx| {
            tx.set_deleted(DeleteTarget::Trainer { index: 1 }, true)?;
            tx.set_deleted(DeleteTarget::Script { file: 0 }, true)
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Refused(_)), "{err}");
    assert!(deleted(&st).is_empty());
    assert_eq!(st.undo_depth(), 0);
    // An entity that does not exist cannot be deleted.
    assert!(
        st.transaction("weg", |tx| tx
            .set_deleted(DeleteTarget::Trainer { index: 999 }, true))
            .is_err()
    );
    assert!(deleted(&st).is_empty());
}

#[test]
fn set_deleted_on_a_header_writes_its_file_only() {
    let fx = common::TextFixture::exported();
    let mut st = text_store(&fx);
    let arm9 = fx.read("arm9/arm9.bin");
    let names = fx.read("files/fielddata/maptable/mapname.bin");
    let before = data_files(fx.root());
    let header = DeleteTarget::Header { id: 1 };
    st.transaction("Gelöscht: Header 0001", |tx| tx.set_deleted(header, true))
        .unwrap();
    assert_eq!(st.dirty(), [HEADER_TABLE]);
    st.save().unwrap();
    let after = data_files(fx.root());
    assert_eq!(
        changed_files(&before, &after),
        [std::path::PathBuf::from("data/headers/0001.json")]
    );
    assert!(fx.read("data/headers/0001.json").ends_with(ENVELOPE_END));
    assert_eq!(fx.read("arm9/arm9.bin"), arm9);
    assert_eq!(fx.read("files/fielddata/maptable/mapname.bin"), names);
    assert!(st.dirty().is_empty());

    // The header file keeps its envelope when an edit of another header saves the table.
    st.edit_header(0, "Header 0", |h| {
        if let g4_kernel::store::MapHeader::Pt(h) = h {
            h.area_data_id = 1;
        }
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    let again = data_files(fx.root());
    assert_eq!(
        changed_files(&after, &again),
        [std::path::PathBuf::from("data/headers/0000.json")]
    );
    assert!(text_store(&fx).is_deleted(header));
}
