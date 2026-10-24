#![cfg(feature = "test-support")]
//! Changesets with several parts: `Store::transaction`, and undo and redo over them.
mod common;

use std::collections::HashSet;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use g4_formats::error::FormatError;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, MapHeader, Store, WarpEvent};
use g4_kernel::text::TextArchiveDoc;
use proptest::prelude::*;

fn store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    (s, st)
}

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

const MATRIX: AssetId = AssetId {
    kind: AssetKind::Matrix,
    index: 0,
};
const EVENT: AssetId = AssetId {
    kind: AssetKind::Event,
    index: 0,
};
const HEADERS: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};
const TEXT: AssetId = AssetId {
    kind: AssetKind::TextArchive,
    index: 433,
};

fn warp(x: u16) -> WarpEvent {
    WarpEvent {
        x,
        z: 0,
        dest_header_id: 0,
        dest_warp_id: 0,
        height: 0,
    }
}

/// Puts map 2 into matrix cell 0 and a warp into event file 0, as one step.
fn matrix_and_event(
    st: &mut Store,
    label: &str,
) -> Result<((), Option<g4_kernel::store::ChangeId>), KernelError> {
    st.transaction(label, |tx| {
        tx.edit_matrix(0, |m| {
            m.maps[0] = 2;
            Ok(())
        })?;
        tx.edit_event(0, |e| {
            e.warp_events.push(warp(7));
            Ok(())
        })
    })
}

#[test]
fn transaction_records_one_history_step_for_two_assets() {
    let (_s, mut st) = store();
    let ((), change) = matrix_and_event(&mut st, "Matrix und Warp").unwrap();
    assert!(change.is_some());
    assert_eq!(st.history(), (vec!["Matrix und Warp".to_owned()], vec![]));
    assert_eq!(st.dirty(), vec![MATRIX, EVENT]);
    assert_eq!(st.matrix(0).unwrap().maps[0], 2);
    assert_eq!(st.event(0).unwrap().warp_events, vec![warp(7)]);
}

#[test]
fn transaction_returns_the_closures_value_and_no_change_when_nothing_changed() {
    let (_s, mut st) = store();
    let (value, change) = st
        .transaction("nichts", |tx| {
            tx.edit_matrix(0, |_| Ok(()))?;
            Ok(17)
        })
        .unwrap();
    assert_eq!((value, change), (17, None));
    assert_eq!(st.history(), (vec![], vec![]));
    assert!(st.undo().unwrap().is_empty());
}

#[test]
fn undo_restores_every_part_byte_for_byte() {
    let (s, mut st) = store();
    let matrix_before = st.matrix(0).unwrap().write();
    let event_before = st.event(0).unwrap().clone();
    matrix_and_event(&mut st, "beides").unwrap();
    assert_eq!(st.undo().unwrap(), vec![MATRIX, EVENT]);
    assert_eq!(st.matrix(0).unwrap().write(), matrix_before);
    assert_eq!(*st.event(0).unwrap(), event_before);
    // Dirty compares the serialized value with the bytes on disk: nothing differs.
    assert!(st.dirty().is_empty());
    assert_eq!(
        std::fs::read(s.dir.path().join("unpacked/eventFiles/0000")).unwrap(),
        vec![0u8; 16]
    );
    assert_eq!(st.history(), (vec![], vec!["beides".to_owned()]));
}

#[test]
fn redo_reapplies_every_part() {
    let (_s, mut st) = store();
    matrix_and_event(&mut st, "beides").unwrap();
    st.undo().unwrap();
    assert_eq!(st.redo().unwrap(), vec![MATRIX, EVENT]);
    assert_eq!(st.matrix(0).unwrap().maps[0], 2);
    assert_eq!(st.event(0).unwrap().warp_events, vec![warp(7)]);
    assert_eq!(st.dirty(), vec![MATRIX, EVENT]);
    assert!(st.redo().unwrap().is_empty());
}

#[test]
fn failing_closure_changes_nothing() {
    let (s, mut st) = store();
    st.event(0).unwrap(); // unpacks the event files
    // Four bytes after the counters: uxie would drop them on write, so the file is locked.
    let path = s.dir.path().join("unpacked/eventFiles/0001");
    std::fs::write(&path, [0u8; 20]).unwrap();
    let err = st
        .transaction("zwei", |tx| {
            tx.edit_matrix(0, |m| {
                m.maps[0] = 2;
                Ok(())
            })?;
            tx.edit_event(1, |e| {
                e.warp_events.push(warp(1));
                Ok(())
            })
        })
        .unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { asset, .. } if *asset == id(AssetKind::Event, 1)),
        "{err}"
    );
    assert!(err.to_string().contains("event file 0001"), "{err}");
    assert_eq!(st.matrix(0).unwrap().maps[0], 0);
    assert!(st.event(1).unwrap().warp_events.is_empty());
    assert!(st.dirty().is_empty());
    assert_eq!(st.history(), (vec![], vec![]));
    assert!(st.undo().unwrap().is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), [0u8; 20]);
}

#[test]
fn an_error_of_the_closure_itself_changes_nothing() {
    let (_s, mut st) = store();
    let err = st
        .transaction("zwei", |tx| {
            tx.edit_matrix(0, |m| {
                m.maps[0] = 2;
                Ok(())
            })?;
            tx.edit_text_archive(433, |d| d.messages[1].text = "Zweiblattdorf".into())?;
            Err::<(), _>(KernelError::Format(FormatError::OutOfRange {
                what: "test",
                value: 1,
                max: 0,
            }))
        })
        .unwrap_err();
    assert!(matches!(err, KernelError::Format(_)), "{err}");
    assert_eq!(st.matrix(0).unwrap().maps[0], 0);
    assert_eq!(
        st.text_archive(433).unwrap().messages[1].text,
        "Twinleaf Town"
    );
    assert!(st.dirty().is_empty());
    assert_eq!(st.history(), (vec![], vec![]));
}

#[test]
fn undo_with_undecodable_part_changes_nothing() {
    let (_s, mut st) = store();
    st.edit_matrix(0, "vorher", |m| {
        m.maps[1] = 2;
        Ok(())
    })
    .unwrap();
    matrix_and_event(&mut st, "beides").unwrap();
    let matrix = st.matrix(0).unwrap().clone();
    let event = st.event(0).unwrap().clone();
    // Damage the second part (the event file) from outside.
    st.corrupt_undo_part(1);
    let err = st.undo().unwrap_err();
    assert!(err.to_string().contains("event file 0000"), "{err}");
    assert_eq!(
        *st.matrix(0).unwrap(),
        matrix,
        "the first part is unchanged"
    );
    assert_eq!(*st.event(0).unwrap(), event);
    assert_eq!(
        st.history(),
        (vec!["vorher".to_owned(), "beides".to_owned()], vec![]),
        "the step stays where it was"
    );
}

#[test]
fn redo_with_undecodable_part_changes_nothing() {
    let (_s, mut st) = store();
    matrix_and_event(&mut st, "beides").unwrap();
    st.undo().unwrap();
    st.corrupt_redo_part(0);
    let err = st.redo().unwrap_err();
    assert!(err.to_string().contains("matrix 0000"), "{err}");
    assert_eq!(st.matrix(0).unwrap().maps[0], 0);
    assert!(st.event(0).unwrap().warp_events.is_empty());
    assert_eq!(st.history(), (vec![], vec!["beides".to_owned()]));
}

#[test]
fn same_asset_twice_is_refused() {
    let (_s, mut st) = store();
    let err = st
        .transaction("doppelt", |tx| {
            tx.edit_matrix(0, |m| {
                m.maps[0] = 2;
                Ok(())
            })?;
            tx.edit_matrix(0, |m| {
                m.maps[1] = 2;
                Ok(())
            })
        })
        .unwrap_err();
    assert!(matches!(&err, KernelError::InvalidProject(_)), "{err}");
    assert!(err.to_string().contains("matrix 0000"), "{err}");
    assert_eq!(st.matrix(0).unwrap().maps, vec![0, 1]);
    assert_eq!(st.history(), (vec![], vec![]));
}

#[test]
fn reload_of_one_part_keeps_the_other_in_history() {
    let (_s, mut st) = store();
    matrix_and_event(&mut st, "beides").unwrap();
    st.reload_from_disk(EVENT).unwrap();
    assert!(st.event(0).unwrap().warp_events.is_empty());
    assert_eq!(st.history(), (vec!["beides".to_owned()], vec![]));
    assert_eq!(st.undo().unwrap(), vec![MATRIX]);
    assert_eq!(st.matrix(0).unwrap().maps[0], 0);
    assert_eq!(st.redo().unwrap(), vec![MATRIX]);
    assert_eq!(st.matrix(0).unwrap().maps[0], 2);
    assert!(st.event(0).unwrap().warp_events.is_empty());
}

#[test]
fn reload_of_every_part_drops_the_step() {
    let (_s, mut st) = store();
    st.edit_matrix(0, "davor", |m| {
        m.maps[1] = 2;
        Ok(())
    })
    .unwrap();
    st.transaction("Warp und Text", |tx| {
        tx.edit_event(0, |e| {
            e.warp_events.push(warp(7));
            Ok(())
        })?;
        tx.edit_text_archive(433, |d| d.messages[1].text = "Zweiblattdorf".into())
    })
    .unwrap();
    st.reload_from_disk(EVENT).unwrap();
    st.accept_disk_as_base(TEXT).unwrap();
    assert_eq!(st.history(), (vec!["davor".to_owned()], vec![]));
    assert_eq!(st.undo().unwrap(), vec![MATRIX]);
    assert!(st.undo().unwrap().is_empty());
}

#[test]
fn accept_disk_as_base_keeps_the_other_part_on_the_redo_stack() {
    let (_s, mut st) = store();
    matrix_and_event(&mut st, "beides").unwrap();
    st.undo().unwrap();
    st.accept_disk_as_base(MATRIX).unwrap();
    assert_eq!(st.history(), (vec![], vec!["beides".to_owned()]));
    assert_eq!(st.redo().unwrap(), vec![EVENT]);
    assert_eq!(st.matrix(0).unwrap().maps[0], 0);
}

#[test]
fn history_with_two_hundred_steps_answers_under_50_ms() {
    let (_s, mut st) = store();
    for step in 0..200u16 {
        st.transaction(&format!("Schritt {step}"), |tx| {
            tx.edit_matrix(0, |m| {
                m.maps[0] = step + 2;
                Ok(())
            })?;
            tx.edit_text_archive(433, |d| d.messages[1].text = format!("Text {step}"))
        })
        .unwrap();
    }
    let started = Instant::now();
    let (undo, redo) = st.history();
    let took = started.elapsed();
    assert_eq!((undo.len(), redo.len()), (200, 0));
    assert_eq!(undo[199], "Schritt 199");
    assert!(took < Duration::from_millis(50), "history() took {took:?}");
    let started = Instant::now();
    assert_eq!(st.dirty(), vec![MATRIX, TEXT]);
    let took = started.elapsed();
    assert!(took < Duration::from_millis(50), "dirty() took {took:?}");
}

/// One step of the property test: an edit of one asset through the store's own edit
/// functions, a transaction over two assets, or undo or redo.
#[derive(Clone, Debug)]
enum Op {
    Single(usize, u16),
    Pair(usize, usize, u16),
    Undo,
    Redo,
}

/// The four assets of the property test, one per kind.
const KINDS: [AssetId; 4] = [MATRIX, EVENT, HEADERS, TEXT];

/// What every asset of [`KINDS`] holds.
#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    matrix: Vec<u16>,
    event: Vec<WarpEvent>,
    header: MapHeader,
    text: TextArchiveDoc,
}

impl Snapshot {
    fn of(st: &mut Store) -> Snapshot {
        Snapshot {
            matrix: st.matrix(0).unwrap().maps.clone(),
            event: st.event(0).unwrap().warp_events.clone(),
            header: st.header_table().unwrap()[0].clone(),
            text: st.text_archive(433).unwrap().clone(),
        }
    }

    /// The assets in which `self` and `other` differ, in [`KINDS`] order.
    fn changed(&self, other: &Snapshot) -> Vec<AssetId> {
        let differs = [
            self.matrix != other.matrix,
            self.event != other.event,
            self.header != other.header,
            self.text != other.text,
        ];
        KINDS
            .into_iter()
            .zip(differs)
            .filter_map(|(id, d)| d.then_some(id))
            .collect()
    }
}

/// The synthetic project the property test reads; it never saves, so the cases share it.
static FIXTURE: LazyLock<common::Synthetic> = LazyLock::new(common::synthetic_project);

fn edit_alone(st: &mut Store, kind: usize, value: u16) {
    match kind {
        0 => st
            .edit_matrix(0, "Matrix", |m| {
                m.maps[0] = value;
                Ok(())
            })
            .map(drop),
        1 => st
            .edit_event(0, "Event", |e| {
                e.warp_events = (0..value).map(warp).collect();
                Ok(())
            })
            .map(drop),
        2 => st
            .edit_header(0, "Header", |h| {
                if let MapHeader::Pt(h) = h {
                    h.music_day_id = value;
                }
                Ok(())
            })
            .map(drop),
        _ => st
            .edit_text_archive(433, "Text", |d| {
                d.messages[1].text = format!("Text {value}")
            })
            .map(drop),
    }
    .unwrap();
}

fn edit_in(tx: &mut g4_kernel::store::Tx<'_>, kind: usize, value: u16) -> Result<(), KernelError> {
    match kind {
        0 => tx.edit_matrix(0, |m| {
            m.maps[0] = value;
            Ok(())
        }),
        1 => tx.edit_event(0, |e| {
            e.warp_events = (0..value).map(warp).collect();
            Ok(())
        }),
        2 => tx.edit_header(0, |h| {
            if let MapHeader::Pt(h) = h {
                h.music_day_id = value;
            }
            Ok(())
        }),
        _ => tx.edit_text_archive(433, |d| d.messages[1].text = format!("Text {value}")),
    }
}

fn set(ids: Vec<AssetId>) -> HashSet<AssetId> {
    ids.into_iter().collect()
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..4usize, 0..3u16).prop_map(|(k, v)| Op::Single(k, v)),
        (0..4usize, 1..4usize, 0..3u16).prop_map(|(a, d, v)| Op::Pair(a, (a + d) % 4, v)),
        Just(Op::Undo),
        Just(Op::Redo),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every undo returns exactly the assets it restored and brings back the state before
    /// the step; every redo the state after it; a new step drops what could be redone.
    #[test]
    fn undo_redo_are_inverse_over_mixed_kinds(ops in proptest::collection::vec(op(), 1..24)) {
        let project = Project::open(FIXTURE.dir.path()).unwrap();
        let mut st = Store::new(Arc::new(project));
        let mut states = vec![Snapshot::of(&mut st)];
        let mut at = 0;
        for op in ops {
            match op {
                Op::Single(kind, value) => edit_alone(&mut st, kind, value),
                Op::Pair(a, b, value) => {
                    st.transaction("Paar", |tx| {
                        edit_in(tx, a, value)?;
                        edit_in(tx, b, value + 1)
                    })
                    .unwrap();
                }
                Op::Undo => {
                    let restored = st.undo().unwrap();
                    if at == 0 {
                        prop_assert!(restored.is_empty());
                    } else {
                        at -= 1;
                        let now = Snapshot::of(&mut st);
                        prop_assert_eq!(&now, &states[at]);
                        prop_assert_eq!(set(restored), set(now.changed(&states[at + 1])));
                    }
                    continue;
                }
                Op::Redo => {
                    let restored = st.redo().unwrap();
                    if at + 1 == states.len() {
                        prop_assert!(restored.is_empty());
                    } else {
                        at += 1;
                        let now = Snapshot::of(&mut st);
                        prop_assert_eq!(&now, &states[at]);
                        prop_assert_eq!(set(restored), set(now.changed(&states[at - 1])));
                    }
                    continue;
                }
            }
            // An edit that changed something is a new step and drops the redo states.
            let now = Snapshot::of(&mut st);
            if now != states[at] {
                states.truncate(at + 1);
                states.push(now);
                at += 1;
            }
            prop_assert_eq!(st.history().0.len(), at);
            prop_assert_eq!(st.history().1.len(), states.len() - 1 - at);
        }
        while !st.undo().unwrap().is_empty() {}
        prop_assert_eq!(Snapshot::of(&mut st), states[0].clone());
        prop_assert!(st.dirty().is_empty());
    }
}
