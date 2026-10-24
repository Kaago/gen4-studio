#![cfg(feature = "test-support")]
//! The reference index: which data members, headers and events point at a species, move,
//! item, trainer, trainer class or encounter file; built lazily from the store and the disk
//! without loading anything into the store, refreshed per changed source.
mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use g4_formats::data::encounter::{Encounters, Slot};
use g4_formats::data::evolution::EvoEntry;
use g4_formats::data::learnset::LearnEntry;
use g4_kernel::data::refs::{Ref, RefIndex, Target};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, MapHeader, ObjectEvent, Store};
use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestRunner};

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

const HEADERS: AssetId = AssetId {
    kind: AssetKind::HeaderTable,
    index: 0,
};

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A Platinum encounter file: grass slot `grass.0` holds species `grass.1`, surf slot 2
/// species `surf`, swarm entry 0 species `swarm` (0: empty).
fn encounter_bytes(grass: (usize, u16), surf: u16, swarm: u16) -> Vec<u8> {
    let mut e = vec![0u8; 424];
    e[0] = 10;
    let at = 4 + 8 * grass.0;
    e[at] = 3;
    e[at + 4..at + 6].copy_from_slice(&grass.1.to_le_bytes());
    // Surf: rate at 0xCC, slots of 8 bytes from 0xD0, species word at +4.
    let surf_at = 0xD0 + 8 * 2 + 4;
    e[surf_at..surf_at + 2].copy_from_slice(&surf.to_le_bytes());
    e[0x64..0x66].copy_from_slice(&swarm.to_le_bytes());
    e
}

/// A learnset with `(move, level)` entries, the terminator and padding to four bytes.
fn learnset_bytes(entries: &[(u16, u16)]) -> Vec<u8> {
    let mut out: Vec<u8> = entries
        .iter()
        .flat_map(|&(m, l)| (m | l << 9).to_le_bytes())
        .collect();
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.resize(out.len().next_multiple_of(4), 0);
    out
}

/// An evolution member with the given `(method, param, target)` entries.
fn evolution_bytes(entries: &[(u16, u16, u16)]) -> Vec<u8> {
    let mut out = vec![0u8; 44];
    for (i, &(m, p, t)) in entries.iter().enumerate() {
        for (k, v) in [m, p, t].into_iter().enumerate() {
            out[i * 6 + k * 2..i * 6 + k * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// An event file with one object event per script, no other events.
fn event_bytes(scripts: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    for count in [0, scripts.len() as u32] {
        out.extend_from_slice(&count.to_le_bytes());
    }
    for (i, &script) in scripts.iter().enumerate() {
        let mut object = [0u8; 32];
        object[..2].copy_from_slice(&(i as u16).to_le_bytes());
        object[10..12].copy_from_slice(&script.to_le_bytes());
        out.extend_from_slice(&object);
    }
    out.extend_from_slice(&[0; 8]);
    out
}

/// The synthetic Platinum project with two members of every source kind, unpacked:
///
/// - encounters: 0 grass slot 0 species 1, surf slot 2 species 7, swarm 8; 1 grass slot 3
///   species 9
/// - personal: 0 without items, 1 items 40 and 41
/// - learnsets: 0 moves 1 and 2, 1 moves 3 and 2
/// - evolutions: 0 move method 20 (move 5 → species 9), species method 21 (species 10 →
///   species 11); 1 level 16 → species 2 and the item methods 6, 7, 16, 17, 18, 19 with
///   items 80 to 85 → species 3 to 8
/// - trainers: 0 class 1 with species 1; 1 class 2 with item 50, moves and items: species
///   6 (form 1) holding item 51 with moves 7 and 8
/// - events: 0 one object with script 3000; 1 objects with scripts 4999, 5000, 2999
/// - headers: 0 uses encounter file 1, 1 none (0xFFFF)
fn fixture() -> common::Synthetic {
    let s = common::synthetic_project();
    let project = Project::open(s.dir.path()).unwrap();
    let member = |kind, index| project.member_path(kind, index);
    write(
        &member(AssetKind::Encounter, 0),
        &encounter_bytes((0, 1), 7, 8),
    );
    write(
        &member(AssetKind::Encounter, 1),
        &encounter_bytes((3, 9), 0, 0),
    );
    let mut personal = common::synthetic_personal_bytes(1);
    personal[0x0C..0x0E].copy_from_slice(&40u16.to_le_bytes());
    personal[0x0E..0x10].copy_from_slice(&41u16.to_le_bytes());
    write(
        &member(AssetKind::Personal, 0),
        &common::synthetic_personal_bytes(0),
    );
    write(&member(AssetKind::Personal, 1), &personal);
    write(
        &member(AssetKind::Learnset, 0),
        &learnset_bytes(&[(1, 1), (2, 5)]),
    );
    write(
        &member(AssetKind::Learnset, 1),
        &learnset_bytes(&[(3, 7), (2, 9)]),
    );
    write(
        &member(AssetKind::Evolution, 0),
        &evolution_bytes(&[(20, 5, 9), (21, 10, 11)]),
    );
    write(
        &member(AssetKind::Evolution, 1),
        &evolution_bytes(&[
            (4, 16, 2),
            (6, 80, 3),
            (7, 81, 4),
            (16, 82, 5),
            (17, 83, 6),
            (18, 84, 7),
            (19, 85, 8),
        ]),
    );
    let mut props = vec![0u8; 20];
    props[1] = 1;
    props[3] = 1;
    write(&member(AssetKind::TrainerProperties, 0), &props);
    write(
        &member(AssetKind::TrainerParty, 0),
        &[0, 0, 5, 0, 1, 0, 0, 0],
    );
    let mut props = vec![0u8; 20];
    props[0] = 3; // moves and items
    props[1] = 2;
    props[3] = 1;
    props[4..6].copy_from_slice(&50u16.to_le_bytes());
    write(&member(AssetKind::TrainerProperties, 1), &props);
    let mut party = vec![0, 0, 10, 0];
    party.extend_from_slice(&(6u16 | 1 << 10).to_le_bytes());
    for v in [51u16, 7, 8, 0, 0, 0] {
        party.extend_from_slice(&v.to_le_bytes());
    }
    party.resize(20, 0);
    write(&member(AssetKind::TrainerParty, 1), &party);
    write(&member(AssetKind::Event, 0), &event_bytes(&[3000]));
    write(
        &member(AssetKind::Event, 1),
        &event_bytes(&[4999, 5000, 2999]),
    );
    // Platinum header byte 14: wild_pokemon (u16).
    let arm9 = s.dir.path().join("arm9/arm9.bin");
    let mut bytes = std::fs::read(&arm9).unwrap();
    let table = common::PT_HEADER_TABLE_CPUE;
    bytes[table + 14..table + 16].copy_from_slice(&1u16.to_le_bytes());
    let second = table + common::HEADER_SIZE;
    bytes[second + 14..second + 16].copy_from_slice(&0xFFFFu16.to_le_bytes());
    std::fs::write(&arm9, bytes).unwrap();
    s
}

fn store_of(s: &common::Synthetic) -> Store {
    Store::new(Arc::new(Project::open(s.dir.path()).unwrap()))
}

fn r(source: AssetId, field: &'static str, slot: Option<u16>, target: Target) -> Ref {
    Ref {
        source,
        field,
        slot,
        target,
    }
}

fn sorted(mut refs: Vec<Ref>) -> Vec<Ref> {
    refs.sort_by_key(|r| (format!("{:?}", r.source), r.field, r.slot));
    refs
}

#[test]
fn encounter_slots_reference_species() {
    let s = fixture();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    let enc0 = id(AssetKind::Encounter, 0);
    let enc1 = id(AssetKind::Encounter, 1);
    // Species 1 is also the party species of trainer 0.
    assert_eq!(
        sorted(index.refs(&st, Target::Species(1)).unwrap()),
        sorted(vec![
            r(enc0, "grass", Some(0), Target::Species(1)),
            r(
                id(AssetKind::TrainerParty, 0),
                "species",
                Some(0),
                Target::Species(1)
            ),
        ])
    );
    assert_eq!(
        index.refs(&st, Target::Species(7)).unwrap()[0],
        r(enc0, "surf", Some(2), Target::Species(7))
    );
    assert_eq!(
        index.refs(&st, Target::Species(8)).unwrap(),
        [
            r(enc0, "swarm", Some(0), Target::Species(8)),
            r(
                id(AssetKind::Evolution, 1),
                "target",
                Some(6),
                Target::Species(8)
            ),
        ]
    );
    assert!(index.refs(&st, Target::Species(9)).unwrap().contains(&r(
        enc1,
        "grass",
        Some(3),
        Target::Species(9)
    )));
    // Empty slots (species 0) are no references.
    assert_eq!(index.count(&st, Target::Species(0)).unwrap(), 0);
}

#[test]
fn learnset_references_moves() {
    let s = fixture();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    let l0 = id(AssetKind::Learnset, 0);
    let l1 = id(AssetKind::Learnset, 1);
    assert_eq!(
        index.refs(&st, Target::Move(2)).unwrap(),
        [
            r(l0, "move", Some(1), Target::Move(2)),
            r(l1, "move", Some(1), Target::Move(2)),
        ]
    );
    assert_eq!(
        index.refs(&st, Target::Move(3)).unwrap(),
        [r(l1, "move", Some(0), Target::Move(3))]
    );
    // Trainer 1's party knows moves 7 and 8.
    let party = id(AssetKind::TrainerParty, 1);
    assert_eq!(
        index.refs(&st, Target::Move(7)).unwrap(),
        [r(party, "move", Some(0), Target::Move(7))]
    );
    assert_eq!(index.count(&st, Target::Move(8)).unwrap(), 1);
}

#[test]
fn evolution_item_methods_reference_items_not_species() {
    let s = fixture();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    let e0 = id(AssetKind::Evolution, 0);
    let e1 = id(AssetKind::Evolution, 1);
    for (slot, item) in (1..=6).zip(80..=85) {
        assert_eq!(
            index.refs(&st, Target::Item(item)).unwrap(),
            [r(e1, "param", Some(slot), Target::Item(item))],
            "item {item}"
        );
        assert_eq!(index.count(&st, Target::Species(item)).unwrap(), 0);
    }
    // Targets are species; the level of method 4 is no reference.
    assert_eq!(
        index.refs(&st, Target::Species(2)).unwrap(),
        [r(e1, "target", Some(0), Target::Species(2))]
    );
    assert_eq!(index.count(&st, Target::Species(16)).unwrap(), 0);
    assert_eq!(index.count(&st, Target::Item(16)).unwrap(), 0);
    // Method 20 names a move, method 21 a species in the party.
    assert_eq!(
        index.refs(&st, Target::Move(5)).unwrap(),
        [r(e0, "param", Some(0), Target::Move(5))]
    );
    assert_eq!(
        index.refs(&st, Target::Species(10)).unwrap(),
        [r(e0, "param", Some(1), Target::Species(10))]
    );
    // Wild items of personal data and trainer items are items too.
    assert_eq!(
        index.refs(&st, Target::Item(41)).unwrap(),
        [r(
            id(AssetKind::Personal, 1),
            "item2",
            None,
            Target::Item(41)
        )]
    );
    assert_eq!(
        index.refs(&st, Target::Item(50)).unwrap(),
        [r(
            id(AssetKind::TrainerProperties, 1),
            "item",
            Some(0),
            Target::Item(50)
        )]
    );
    assert_eq!(
        index.refs(&st, Target::Item(51)).unwrap(),
        [r(
            id(AssetKind::TrainerParty, 1),
            "item",
            Some(0),
            Target::Item(51)
        )]
    );
    assert_eq!(
        index.refs(&st, Target::TrainerClass(2)).unwrap(),
        [r(
            id(AssetKind::TrainerProperties, 1),
            "class",
            None,
            Target::TrainerClass(2)
        )]
    );
    // The party species is the ten species bits, without the form.
    assert_eq!(index.count(&st, Target::Species(6)).unwrap(), 2);
}

#[test]
fn header_references_its_encounter_file_and_none_for_ffff() {
    let s = fixture();
    let mut st = store_of(&s);
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs(&st, Target::Encounter(1)).unwrap(),
        [r(HEADERS, "wild_pokemon", Some(0), Target::Encounter(1))]
    );
    assert!(index.refs(&st, Target::Encounter(0)).unwrap().is_empty());
    assert!(
        index
            .refs(&st, Target::Encounter(0xFFFF))
            .unwrap()
            .is_empty()
    );
    // An unsaved edit of the loaded table counts.
    st.edit_header(1, "Begegnungen", |h| {
        if let MapHeader::Pt(h) = h {
            h.wild_pokemon = 0;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        index.refs(&st, Target::Encounter(0)).unwrap(),
        [r(HEADERS, "wild_pokemon", Some(1), Target::Encounter(0))]
    );
}

#[test]
fn event_script_3000_references_trainer_1() {
    let s = fixture();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs(&st, Target::Trainer(1)).unwrap(),
        [r(
            id(AssetKind::Event, 0),
            "script",
            Some(0),
            Target::Trainer(1)
        )]
    );
    // 4999 is the last trainer script; 5000 (partner) and 2999 are none.
    assert_eq!(
        index.refs(&st, Target::Trainer(2000)).unwrap(),
        [r(
            id(AssetKind::Event, 1),
            "script",
            Some(0),
            Target::Trainer(2000)
        )]
    );
    assert_eq!(index.count(&st, Target::Trainer(2001)).unwrap(), 0);
    assert_eq!(index.count(&st, Target::Trainer(0)).unwrap(), 0);
}

#[test]
fn refresh_after_edit_updates_only_that_source() {
    let s = fixture();
    let mut st = store_of(&s);
    let mut index = RefIndex::new();
    // Not built yet: nothing to refresh, and nothing is built by it.
    assert!(index.refresh(&st).unwrap().is_empty());
    assert_eq!(index.count(&st, Target::Move(4)).unwrap(), 0);
    let l1 = id(AssetKind::Learnset, 1);
    st.edit_learnset(1, "Lernset", |l| {
        l.entries.push(LearnEntry {
            move_id: 4,
            level: 20,
        })
    })
    .unwrap();
    assert_eq!(index.refresh(&st).unwrap(), [l1]);
    assert_eq!(
        index.refs(&st, Target::Move(4)).unwrap(),
        [r(l1, "move", Some(2), Target::Move(4))]
    );
    // Nothing changed since: nothing to refresh.
    assert!(index.refresh(&st).unwrap().is_empty());
    // A store change of a kind that is no source is not refreshed either.
    st.edit_matrix(0, "Matrix", |m| {
        m.maps[0] = 2;
        Ok(())
    })
    .unwrap();
    assert!(index.refresh(&st).unwrap().is_empty());
    // Undo is a change of the learnset again.
    st.undo().unwrap();
    st.undo().unwrap();
    assert_eq!(index.refresh(&st).unwrap(), [l1]);
    assert_eq!(index.count(&st, Target::Move(4)).unwrap(), 0);
}

#[test]
fn index_does_not_load_assets_into_the_store() {
    let s = fixture();
    let mut st = store_of(&s);
    st.edit_personal(0, "Item", |p| p.set_item1(60)).unwrap();
    let loaded = st.loaded_count();
    assert_eq!(loaded, 1);
    let mut index = RefIndex::new();
    // The loaded member counts with its unsaved edit; the others come from the disk.
    assert_eq!(
        index.refs(&st, Target::Item(60)).unwrap(),
        [r(
            id(AssetKind::Personal, 0),
            "item1",
            None,
            Target::Item(60)
        )]
    );
    assert_eq!(index.count(&st, Target::Move(2)).unwrap(), 2);
    index.rebuild_full(&st).unwrap();
    assert_eq!(st.loaded_count(), loaded);
    assert!(st.dirty().len() == 1);
}

#[test]
fn index_reads_packed_archives_without_unpacking() {
    // The plain fixture: every archive is packed only.
    let s = common::synthetic_project();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs(&st, Target::Species(1)).unwrap(),
        [
            r(
                id(AssetKind::Encounter, 0),
                "grass",
                Some(0),
                Target::Species(1)
            ),
            r(
                id(AssetKind::Encounter, 1),
                "grass",
                Some(0),
                Target::Species(1)
            ),
            r(
                id(AssetKind::TrainerParty, 0),
                "species",
                Some(0),
                Target::Species(1)
            ),
            r(
                id(AssetKind::TrainerParty, 1),
                "species",
                Some(0),
                Target::Species(1)
            ),
            // Trainer 2 of the fixture: moves and items, species 1 in its third slot.
            r(
                id(AssetKind::TrainerParty, 2),
                "species",
                Some(2),
                Target::Species(1)
            ),
        ]
    );
    assert_eq!(
        index.refs(&st, Target::Species(2)).unwrap(),
        [
            r(
                id(AssetKind::Evolution, 1),
                "target",
                Some(0),
                Target::Species(2)
            ),
            r(
                id(AssetKind::TrainerParty, 2),
                "species",
                Some(0),
                Target::Species(2)
            ),
        ]
    );
    assert_eq!(index.count(&st, Target::TrainerClass(1)).unwrap(), 2);
    assert_eq!(index.count(&st, Target::TrainerClass(0)).unwrap(), 1);
    for kind in [
        AssetKind::Encounter,
        AssetKind::Evolution,
        AssetKind::Learnset,
        AssetKind::Personal,
        AssetKind::TrainerProperties,
        AssetKind::TrainerParty,
        AssetKind::Event,
    ] {
        assert!(!st.project().unpacked_dir(kind).exists(), "{kind:?}");
    }
    assert_eq!(st.loaded_count(), 0);
}

#[test]
fn locked_member_contributes_nothing() {
    let s = fixture();
    let project = Project::open(s.dir.path()).unwrap();
    // A personal member DSPRE could have written with two extra bytes.
    let mut long = common::synthetic_personal_bytes(1);
    long[0x0C..0x0E].copy_from_slice(&40u16.to_le_bytes());
    long.extend_from_slice(&[0, 0]);
    write(&project.member_path(AssetKind::Personal, 1), &long);
    let mut st = store_of(&s);
    let mut index = RefIndex::new();
    assert_eq!(index.count(&st, Target::Item(40)).unwrap(), 0);
    // Loaded, it is locked raw bytes, and still contributes nothing.
    let personal = id(AssetKind::Personal, 1);
    assert!(st.personal(1).is_err());
    assert!(st.locked(personal).is_some());
    index.rebuild_full(&st).unwrap();
    assert_eq!(index.count(&st, Target::Item(40)).unwrap(), 0);
}

#[test]
fn changes_since_reports_every_kind_of_change() {
    let s = fixture();
    let mut st = store_of(&s);
    let (g0, none) = st.changes_since(0);
    assert_eq!((g0, none), (0, vec![]));
    let e0 = id(AssetKind::Encounter, 0);
    st.edit_encounter(0, "Gras", |_| {}).unwrap();
    // An edit that changed nothing is no change.
    assert_eq!(st.changes_since(0), (0, vec![]));
    st.edit_encounter(0, "Gras", |e| {
        if let Encounters::Pt(p) = e {
            p.set_grass(
                1,
                Slot {
                    level: 4,
                    species: 5,
                },
            );
        }
    })
    .unwrap();
    let (g1, changed) = st.changes_since(g0);
    assert_eq!(changed, [e0]);
    st.undo().unwrap();
    let (g2, changed) = st.changes_since(g1);
    assert_eq!(changed, [e0]);
    st.redo().unwrap();
    let (g3, changed) = st.changes_since(g2);
    assert_eq!(changed, [e0]);
    st.reload_from_disk(id(AssetKind::Learnset, 1)).unwrap();
    let (g4, changed) = st.changes_since(g3);
    assert_eq!(changed, [id(AssetKind::Learnset, 1)]);
    st.accept_disk_as_base(e0).unwrap();
    let (g5, changed) = st.changes_since(g4);
    assert_eq!(changed, [e0]);
    let event = st.create_event_file().unwrap();
    let (g6, changed) = st.changes_since(g5);
    assert_eq!(changed, [id(AssetKind::Event, event)]);
    // Saving changes no value.
    st.save().unwrap();
    assert_eq!(st.changes_since(g6), (g6, vec![]));
    assert_eq!(st.change_generation(), g6);
    // Everything since the start, in kind order.
    assert_eq!(
        st.changes_since(0).1,
        [id(AssetKind::Event, event), e0, id(AssetKind::Learnset, 1)]
    );
}

// --- proptest: incremental refresh equals a full rebuild ---

#[derive(Debug, Clone)]
enum Op {
    Encounter(u16, usize, u16),
    Learnset(u16, u16),
    Evolution(u16, usize, u16, u16, u16),
    Personal(u16, u16),
    Header(u16, u16),
    Event(u16, u16),
    /// Writes an encounter file to disk with grass slot 0 species `.1`, then reloads it.
    DiskReload(u16, u16),
    Reload(usize),
    Accept(usize),
    Undo,
    Redo,
}

/// Sources the reload and accept operations pick from.
const SOURCES: [AssetId; 7] = [
    AssetId {
        kind: AssetKind::Encounter,
        index: 0,
    },
    AssetId {
        kind: AssetKind::Encounter,
        index: 1,
    },
    AssetId {
        kind: AssetKind::Learnset,
        index: 0,
    },
    AssetId {
        kind: AssetKind::Evolution,
        index: 1,
    },
    AssetId {
        kind: AssetKind::Personal,
        index: 1,
    },
    AssetId {
        kind: AssetKind::Event,
        index: 0,
    },
    HEADERS,
];

fn op() -> impl Strategy<Value = Op> {
    let small = 0u16..12;
    prop_oneof![
        (0u16..2, 0usize..12, small.clone()).prop_map(|(i, s, v)| Op::Encounter(i, s, v)),
        (0u16..2, small.clone()).prop_map(|(i, v)| Op::Learnset(i, v)),
        (
            0u16..2,
            0usize..7,
            prop::sample::select(vec![0u16, 4, 6, 7, 16, 19, 20, 21]),
            small.clone(),
            small.clone()
        )
            .prop_map(|(i, e, m, p, t)| Op::Evolution(i, e, m, p, t)),
        (0u16..2, small.clone()).prop_map(|(i, v)| Op::Personal(i, v)),
        (0u16..2, prop::sample::select(vec![0u16, 1, 2, 0xFFFF]))
            .prop_map(|(h, v)| Op::Header(h, v)),
        (
            0u16..2,
            prop::sample::select(vec![0u16, 2999, 3000, 3004, 4999, 5000])
        )
            .prop_map(|(i, v)| Op::Event(i, v)),
        (0u16..2, small).prop_map(|(i, v)| Op::DiskReload(i, v)),
        (0usize..SOURCES.len()).prop_map(Op::Reload),
        (0usize..SOURCES.len()).prop_map(Op::Accept),
        Just(Op::Undo),
        Just(Op::Redo),
    ]
}

fn apply(st: &mut Store, s: &common::Synthetic, op: Op) {
    match op {
        Op::Encounter(i, slot, species) => {
            st.edit_encounter(i, "Gras", |e| {
                if let Encounters::Pt(p) = e {
                    p.set_grass(slot, Slot { level: 5, species });
                }
            })
            .unwrap();
        }
        Op::Learnset(i, move_id) => {
            st.edit_learnset(i, "Lernset", |l| l.entries[0].move_id = move_id)
                .unwrap();
        }
        Op::Evolution(i, entry, method, param, target) => {
            st.edit_evolution(i, "Entwicklung", |e| {
                e.set_entry(
                    entry,
                    EvoEntry {
                        method,
                        param,
                        target,
                    },
                )
            })
            .unwrap();
        }
        Op::Personal(i, item) => {
            st.edit_personal(i, "Item", |p| p.set_item2(item)).unwrap();
        }
        Op::Header(h, wild) => {
            st.edit_header(h, "Begegnungen", |header| {
                if let MapHeader::Pt(header) = header {
                    header.wild_pokemon = wild;
                }
                Ok(())
            })
            .unwrap();
        }
        Op::Event(i, script) => {
            st.edit_event(i, "Script", |e| {
                match e.object_events.first_mut() {
                    Some(o) => o.script = script,
                    None => e.object_events.push(object(script)),
                }
                Ok(())
            })
            .unwrap();
        }
        Op::DiskReload(i, species) => {
            let project = Project::open(s.dir.path()).unwrap();
            write(
                &project.member_path(AssetKind::Encounter, i),
                &encounter_bytes((0, species), 0, 0),
            );
            st.reload_from_disk(id(AssetKind::Encounter, i)).unwrap();
        }
        Op::Reload(k) => st.reload_from_disk(SOURCES[k]).unwrap(),
        Op::Accept(k) => st.accept_disk_as_base(SOURCES[k]).unwrap(),
        Op::Undo => {
            st.undo().unwrap();
        }
        Op::Redo => {
            st.redo().unwrap();
        }
    }
}

fn object(script: u16) -> ObjectEvent {
    ObjectEvent {
        local_id: 0,
        graphics_id: 0,
        movement_type: 0,
        trainer_type: 0,
        hidden_flag: 0,
        script,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: 0,
        z: 0,
        y: 0,
    }
}

/// Every target the fixture and the operations can produce.
fn targets() -> Vec<Target> {
    let mut all = Vec::new();
    for v in 0..=12u16 {
        all.extend([Target::Species(v), Target::Move(v), Target::Item(v)]);
    }
    all.extend((40..=60).map(Target::Item));
    all.extend((80..=85).map(Target::Item));
    all.extend([0, 1, 2, 3, 0xFFFF].map(Target::Encounter));
    all.extend([0, 1, 2, 5, 2000].map(Target::Trainer));
    all.extend([0, 1, 2].map(Target::TrainerClass));
    all
}

#[test]
fn incremental_equals_full_rebuild() {
    let fixture = fixture();
    let config = ProptestConfig {
        cases: 48,
        // A runner outside `proptest!` knows no source file to store failures next to.
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let result = TestRunner::new(config).run(&proptest::collection::vec(op(), 1..16), |ops| {
        incremental_over(&fixture, ops)
    });
    if let Err(e) = result {
        panic!("{e}");
    }
}

fn incremental_over(fixture: &common::Synthetic, ops: Vec<Op>) -> Result<(), TestCaseError> {
    // Disk writes of earlier cases stay; both indexes read the same disk.
    let mut st = store_of(fixture);
    let mut incremental = RefIndex::new();
    incremental.refs(&st, Target::Species(1)).unwrap();
    for op in ops {
        apply(&mut st, fixture, op);
        incremental.refresh(&st).unwrap();
    }
    let mut full = RefIndex::new();
    let mut rebuilt = RefIndex::new();
    rebuilt.rebuild_full(&st).unwrap();
    for target in targets() {
        let expected = full.refs(&st, target).unwrap();
        prop_assert_eq!(
            &incremental.refs(&st, target).unwrap(),
            &expected,
            "{:?}",
            target
        );
        prop_assert_eq!(&rebuilt.refs(&st, target).unwrap(), &expected);
    }
    Ok(())
}

// --- gated: the vanilla projects ---

fn vanilla_store() -> Option<Store> {
    let root = common::vanilla_project()?;
    Some(Store::new(Arc::new(Project::open(root).unwrap())))
}

#[test]
fn vanilla_index_builds_under_a_second() {
    let Some(st) = vanilla_store() else {
        return;
    };
    let mut index = RefIndex::new();
    // The first build also pays for the first touch of thousands of member files (a cold OS
    // file cache, other tests reading at the same time in the gate run): printed, not judged.
    let started = Instant::now();
    index.rebuild_full(&st).unwrap();
    eprintln!("reference index built cold in {:?}", started.elapsed());
    // The index's own cost: a second full build reads every member file again, now warm.
    // Target 300 ms, limit 1000 ms.
    let started = Instant::now();
    index.rebuild_full(&st).unwrap();
    let took = started.elapsed();
    eprintln!("reference index built warm in {took:?} (target 300 ms)");
    assert!(took < Duration::from_millis(1000), "{took:?}");
    // Every source kind contributes: Zubat (wild in both games and in grunt parties),
    // Tackle, Ivysaur, Moon Stone, the class of trainer 1, some trainer behind an object
    // event and some encounter file behind a header.
    let props = std::fs::read(st.project().member_path(AssetKind::TrainerProperties, 1)).unwrap();
    let checks: [(Vec<Target>, AssetKind); 8] = [
        (vec![Target::Species(41)], AssetKind::Encounter),
        (vec![Target::Move(33)], AssetKind::Learnset),
        (vec![Target::Species(2)], AssetKind::Evolution),
        (vec![Target::Item(81)], AssetKind::Evolution),
        (
            vec![Target::TrainerClass(props[1])],
            AssetKind::TrainerProperties,
        ),
        (vec![Target::Species(41)], AssetKind::TrainerParty),
        ((1..=100).map(Target::Trainer).collect(), AssetKind::Event),
        (
            (0..=20).map(Target::Encounter).collect(),
            AssetKind::HeaderTable,
        ),
    ];
    for (targets, kind) in checks {
        let mut found = false;
        for &target in &targets {
            let refs = index.refs(&st, target).unwrap();
            found |= refs.iter().any(|r| r.source.kind == kind);
        }
        assert!(found, "{targets:?} from {kind:?}");
    }
    assert_eq!(st.loaded_count(), 0);
}

#[test]
fn vanilla_dirty_stays_fast_after_index_build() {
    let Some(mut st) = vanilla_store() else {
        return;
    };
    // One unsaved edit in memory (nothing is saved: the vanilla project stays read-only).
    st.edit_personal(1, "Item", |p| p.set_item1(1)).unwrap();
    let mut index = RefIndex::new();
    index.rebuild_full(&st).unwrap();
    assert_eq!(st.loaded_count(), 1);
    let started = Instant::now();
    let dirty = st.dirty();
    let took = started.elapsed();
    assert_eq!(dirty.len(), 1);
    eprintln!("dirty() after the index build: {took:?}");
    assert!(took < Duration::from_millis(5), "{took:?}");
}

#[test]
fn vanilla_route_201_encounters_reference_known_species() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    // Route 201 (Platinum) and Route 29 (HeartGold), the first route of each game, and
    // species of their grass: Starly, Bidoof / Pidgey, Sentret.
    let (route, species) = match project.family {
        g4_formats::game::Family::Hgss => ("Route 29", [16, 161]),
        _ => ("Route 201", [396, 399]),
    };
    let headers = project.read_headers().unwrap();
    let names = project.headers().unwrap();
    let st = Store::new(Arc::new(project));
    let mut index = RefIndex::new();
    let users: Vec<(u16, u16)> = names
        .iter()
        .filter(|h| h.location_name == route)
        .filter_map(|h| {
            let file = match &headers[usize::from(h.id)] {
                MapHeader::HGSS(x) => (x.wild_pokemon != 0xFF).then_some(u16::from(x.wild_pokemon)),
                MapHeader::Pt(x) => (x.wild_pokemon != 0xFFFF).then_some(x.wild_pokemon),
                MapHeader::DP(x) => (x.wild_pokemon != 0xFFFF).then_some(x.wild_pokemon),
            }?;
            Some((h.id, file))
        })
        .collect();
    assert!(!users.is_empty(), "no header of {route} has encounters");
    for (header, file) in users {
        let refs = index.refs(&st, Target::Encounter(file)).unwrap();
        assert!(
            refs.contains(&r(
                HEADERS,
                "wild_pokemon",
                Some(header),
                Target::Encounter(file)
            )),
            "{refs:?}"
        );
        for s in species {
            let refs = index.refs(&st, Target::Species(s)).unwrap();
            let from_file: Vec<_> = refs
                .iter()
                .filter(|r| r.source == id(AssetKind::Encounter, file))
                .collect();
            let fields: Vec<_> = from_file.iter().map(|r| (r.field, r.slot)).collect();
            eprintln!("{route} header {header} encounter file {file}: species {s} in {fields:?}");
            assert!(
                !from_file.is_empty(),
                "species {s} not in encounter file {file}"
            );
        }
    }
    assert_eq!(st.loaded_count(), 0);
}

#[test]
fn trainer_edits_refresh_both_member_sources_from_the_store() {
    let s = fixture();
    let mut st = store_of(&s);
    let mut index = RefIndex::new();
    assert_eq!(index.count(&st, Target::Species(300)).unwrap(), 0);
    // Trainer 1: moves and items, one Pokémon (species 6); its species becomes 300, class 4.
    st.edit_trainer(1, "Team", |t| {
        t.party[0].set_species(300);
        t.class = 4;
    })
    .unwrap();
    let changed = index.refresh(&st).unwrap();
    assert_eq!(
        changed,
        [
            id(AssetKind::TrainerProperties, 1),
            id(AssetKind::TrainerParty, 1)
        ]
    );
    assert_eq!(
        index.refs(&st, Target::Species(300)).unwrap(),
        [r(
            id(AssetKind::TrainerParty, 1),
            "species",
            Some(0),
            Target::Species(300)
        )]
    );
    assert!(
        index
            .refs(&st, Target::Species(6))
            .unwrap()
            .iter()
            .all(|r| r.source != id(AssetKind::TrainerParty, 1))
    );
    assert_eq!(
        index.refs(&st, Target::TrainerClass(4)).unwrap(),
        [r(
            id(AssetKind::TrainerProperties, 1),
            "class",
            None,
            Target::TrainerClass(4)
        )]
    );
}
