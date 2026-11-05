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
use g4_kernel::data::refs::{
    Access, REFRESH_SCRIPTS_EVERY, Ref, RefIndex, SOFT_SOURCES, ScriptCounts, Strength, Target,
    Usage, strength,
};
use g4_kernel::flags::FlagKind;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{
    AssetId, BgEvent, CoordEvent, EventFile, MapHeader, ObjectEvent, Store, WarpEvent,
};
use g4_kernel::test_support::TextFixture;
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
/// - events: 0 one object with script 3000; 1 objects with scripts 4999, 5000 (partner of
///   trainer 1), 2999
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
        access: Access::Read,
        value: None,
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
        [
            r(
                id(AssetKind::Event, 0),
                "script",
                Some(0),
                Target::Trainer(1)
            ),
            // 5000 is trainer 1 as a double battle partner (5000 + ID - 1).
            r(
                id(AssetKind::Event, 1),
                "script",
                Some(1),
                Target::Trainer(1)
            ),
        ]
    );
    // 4999 is the last trainer script; 2999 is none.
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
    st.edit_move(0, "Stärke", |m| m.set_power(m.power().wrapping_add(1)))
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
        AssetKind::Matrix,
        AssetKind::AreaData,
        AssetKind::Map,
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
    /// Matrix 0: cell `.0` shows map `.1`.
    Matrix(usize, u16),
    /// Writes an encounter file to disk with grass slot 0 species `.1`, then reloads it.
    DiskReload(u16, u16),
    Reload(usize),
    Accept(usize),
    Undo,
    Redo,
}

/// Sources the reload and accept operations pick from.
const SOURCES: [AssetId; 8] = [
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
    AssetId {
        kind: AssetKind::Matrix,
        index: 0,
    },
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
        (0u16..2, small.clone()).prop_map(|(i, v)| Op::DiskReload(i, v)),
        (0usize..2, prop::sample::select(vec![0u16, 2, 5, 0xFFFF]))
            .prop_map(|(c, m)| Op::Matrix(c, m)),
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
        Op::Matrix(cell, map) => {
            st.edit_matrix(0, "Zelle", |m| {
                m.maps[cell] = map;
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
    all.extend([0, 1, 2, 5, 0xFFFF].map(Target::Map));
    all.extend([0, 1].map(Target::Matrix));
    all.extend([0, 1, 2].map(Target::AreaData));
    all.extend([0, 1].map(Target::Script));
    all.extend([0, 7].map(Target::BuildingModel));
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

// --- script sources ---

const SCRIPT_0: AssetId = AssetId {
    kind: AssetKind::Script,
    index: 0,
};
const SOURCE_0: &str = "expanded/scripts/0000.rotom";
/// More than one file timestamp tick (Windows updates mtime in steps of about 16 ms).
const MTIME_TICK: Duration = Duration::from_millis(50);

/// The synthetic text project (scripts adopted) with `source` as script file 0, and its store.
fn script_store(source: &str) -> (TextFixture, Store) {
    let fx = TextFixture::exported();
    fx.write(SOURCE_0, source.as_bytes());
    let st = Store::new(Arc::new(fx.project()));
    (fx, st)
}

/// The references from script sources to `target`.
fn script_refs(index: &mut RefIndex, st: &Store, target: Target) -> Vec<Ref> {
    index
        .refs(st, target)
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .collect()
}

#[test]
fn a_trainer_battle_in_a_script_is_a_reference_with_its_line() {
    let (_fx, st) = script_store(
        "script Main #1:\n    SetTrainerFlag 3\n    StartTrainerBattle 3, 0\n    End\n\
         script Second #2:\n    TrainerBattle 4, 0\n    End\n",
    );
    let mut index = RefIndex::new();
    assert_eq!(
        script_refs(&mut index, &st, Target::Trainer(3)),
        [
            r(SCRIPT_0, "SetTrainerFlag", Some(2), Target::Trainer(3)),
            r(SCRIPT_0, "StartTrainerBattle", Some(3), Target::Trainer(3)),
        ]
    );
    // DSPRE's (legacy) name of the command counts under rotom's.
    assert_eq!(
        script_refs(&mut index, &st, Target::Trainer(4)),
        [r(
            SCRIPT_0,
            "StartTrainerBattle",
            Some(6),
            Target::Trainer(4)
        )]
    );
    // Trainer 0 as the second trainer is "none".
    assert!(script_refs(&mut index, &st, Target::Trainer(0)).is_empty());
}

#[test]
fn an_id_constant_counts_like_its_number() {
    let (fx, st) = script_store(
        "alias 4 as MY_TRAINER\n\
         script Main #1:\n    StartTrainerBattle TRAINER_YOUNGSTER_KAI, MY_TRAINER\n\
         \x20   if CheckItem(ITEM_POTION, 1) == 1 then\n        GivePokemon SPECIES_PIKACHU, 5, 0, VAR_RESULT\n    endif\n\
         \x20   End\nscript Second #2:\n    End\n",
    );
    // The id files need not exist: the index resolves what the build would write.
    assert!(g4_kernel::textproject::include::id_files(&fx.project()).is_ok());
    let mut index = RefIndex::new();
    assert_eq!(
        script_refs(&mut index, &st, Target::Trainer(1)),
        [r(
            SCRIPT_0,
            "StartTrainerBattle",
            Some(3),
            Target::Trainer(1)
        )]
    );
    assert_eq!(
        script_refs(&mut index, &st, Target::Trainer(4)),
        [r(
            SCRIPT_0,
            "StartTrainerBattle",
            Some(3),
            Target::Trainer(4)
        )]
    );
    assert_eq!(
        script_refs(&mut index, &st, Target::Item(17)),
        [r(SCRIPT_0, "CheckItem", Some(4), Target::Item(17))]
    );
    assert_eq!(
        script_refs(&mut index, &st, Target::Species(25)),
        [r(SCRIPT_0, "GivePokemon", Some(5), Target::Species(25))]
    );
}

#[test]
fn a_variable_argument_is_no_reference() {
    let (_fx, st) = script_store(
        "script Main #1:\n    AddItem 0x8004, 1, VAR_RESULT\n    SetTrainerFlag VAR_RESULT\n\
         \x20   CheckItem VARS_START, 1, VAR_RESULT\n    StartTrainerBattle UNKNOWN_NAME, 0\n\
         \x20   End\nscript Second #2:\n    End\n",
    );
    let mut index = RefIndex::new();
    // The variables themselves are targets of their own (`data::refs::flagvars`).
    let all = index
        .refs_where(&st, |t| !matches!(t, Target::Flag(_) | Target::Var(_)))
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .collect::<Vec<_>>();
    assert!(all.is_empty(), "{all:?}");
    assert_eq!(
        script_refs(&mut index, &st, Target::Var(0x4000)),
        [r(SCRIPT_0, "CheckItem", Some(4), Target::Var(0x4000))]
    );
}

#[test]
fn a_script_saved_outside_the_store_is_reread_on_refresh() {
    let (fx, st) = script_store("script Main #1:\n    StartTrainerBattle 3, 0\n    End\n");
    let mut index = RefIndex::new();
    assert_eq!(script_refs(&mut index, &st, Target::Trainer(3)).len(), 1);
    assert_eq!(index.refresh(&st).unwrap(), []);

    // Saved by the script editor, DSPRE or a branch switch: the store knows nothing of it.
    // `refresh` stats the sources at most every 250 ms; `refresh_scripts_now` does it now.
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    End\nscript Other #2:\n    StartTrainerBattle 5, 0\n    End\n",
    );
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [SCRIPT_0]);
    assert!(script_refs(&mut index, &st, Target::Trainer(3)).is_empty());
    assert_eq!(
        script_refs(&mut index, &st, Target::Trainer(5)),
        [r(
            SCRIPT_0,
            "StartTrainerBattle",
            Some(4),
            Target::Trainer(5)
        )]
    );

    // A removed source takes its references with it.
    std::fs::remove_file(fx.root().join(SOURCE_0)).unwrap();
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [SCRIPT_0]);
    assert!(script_refs(&mut index, &st, Target::Trainer(5)).is_empty());
}

#[test]
fn without_adopted_scripts_there_is_no_script_source() {
    let fx = TextFixture::dspre();
    let st = Store::new(Arc::new(fx.project()));
    let mut index = RefIndex::new();
    let from_scripts = index
        .refs_where(&st, |_| true)
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .count();
    assert_eq!(from_scripts, 0);
}

#[test]
fn script_counts_follow_a_rewritten_source() {
    let (fx, st) = script_store("script Main #1:\n    SetFlag VAR_RESULT\n    End\n");
    let mut index = RefIndex::new();
    let rebuilt = |st: &Store| {
        let mut full = RefIndex::new();
        full.rebuild_full(st).unwrap();
        full.script_counts(st).unwrap()
    };
    assert_eq!(
        index.script_counts(&st).unwrap(),
        ScriptCounts {
            script_local: 0,
            dynamic: 1
        }
    );
    // The file's counts are replaced, not added.
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    SetVar VAR_RESULT, 1\n    SetVar VAR_RESULT, 2\n    End\n",
    );
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [SCRIPT_0]);
    let counts = index.script_counts(&st).unwrap();
    assert_eq!(
        counts,
        ScriptCounts {
            script_local: 2,
            dynamic: 0
        }
    );
    assert_eq!(counts, rebuilt(&st));
    // A removed source takes its counts with it.
    std::fs::remove_file(fx.root().join(SOURCE_0)).unwrap();
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [SCRIPT_0]);
    assert_eq!(index.script_counts(&st).unwrap(), ScriptCounts::default());
    assert_eq!(rebuilt(&st), ScriptCounts::default());
}

#[test]
fn a_saved_source_shows_after_the_gate() {
    let (fx, st) = script_store("script Main #1:\n    SetFlag 0x173\n    End\n");
    let mut index = RefIndex::new();
    assert_eq!(index.count(&st, Target::Flag(0x173)).unwrap(), 1);
    // The index sees a change by (size, mtime); a same-size rewrite within one timestamp tick
    // (about 16 ms on Windows) would look unchanged, so each rewrite waits for the next tick.
    std::thread::sleep(MTIME_TICK);
    fx.write(SOURCE_0, b"script Main #1:\n    SetFlag 0x174\n    End\n");
    // Saved right after the build: `refresh_scripts_now` reads it whatever the time gate says.
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [SCRIPT_0]);
    assert_eq!(index.count(&st, Target::Flag(0x173)).unwrap(), 0);
    assert_eq!(
        index.refs(&st, Target::Flag(0x174)).unwrap(),
        [Ref {
            source: SCRIPT_0,
            field: "SetFlag",
            slot: Some(2),
            target: Target::Flag(0x174),
            access: Access::Write,
            value: None,
        }]
    );
    // Once the gate has passed, a plain refresh sees the next save too.
    std::thread::sleep(REFRESH_SCRIPTS_EVERY + Duration::from_millis(20));
    fx.write(SOURCE_0, b"script Main #1:\n    SetFlag 0x175\n    End\n");
    assert_eq!(index.refresh(&st).unwrap(), [SCRIPT_0]);
    assert_eq!(index.count(&st, Target::Flag(0x175)).unwrap(), 1);
}

// --- flags and variables from events and level scripts ---

const EVENTS_0: AssetId = AssetId {
    kind: AssetKind::Event,
    index: 0,
};

fn ref_of(source: AssetId, field: &'static str, slot: u16, target: Target, access: Access) -> Ref {
    Ref {
        source,
        field,
        slot: Some(slot),
        target,
        access,
        value: None,
    }
}

/// `r` carrying `value` (a variable reference with the constant it sets or compares).
fn valued(r: Ref, value: u16) -> Ref {
    Ref {
        value: Some(value),
        ..r
    }
}

/// An event file with objects `(hidden_flag, script)`, background events `(script,
/// event_type)` and triggers on variable `var`.
fn events_with(objects: &[(u16, u16)], bgs: &[(u16, u16)], triggers: &[u16]) -> EventFile {
    EventFile {
        bg_events: bgs
            .iter()
            .map(|&(script, event_type)| BgEvent {
                script,
                event_type,
                x: 0,
                z: 0,
                y: 0,
                player_facing_dir: 0,
            })
            .collect(),
        object_events: objects
            .iter()
            .map(|&(hidden_flag, script)| ObjectEvent {
                hidden_flag,
                ..object(script)
            })
            .collect(),
        warp_events: Vec::new(),
        coord_events: triggers
            .iter()
            .map(|&var| CoordEvent {
                script: 1,
                x: 0,
                z: 0,
                width: 1,
                length: 1,
                y: 0,
                value: 1,
                var,
            })
            .collect(),
    }
}

/// The synthetic Platinum project with `events` as event file 0, and its store.
fn event_store(events: &EventFile) -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let project = Project::open(s.dir.path()).unwrap();
    let mut bytes = std::io::Cursor::new(Vec::new());
    events.to_binary(&mut bytes).unwrap();
    write(
        &project.member_path(AssetKind::Event, 0),
        &bytes.into_inner(),
    );
    let st = store_of(&s);
    (s, st)
}

fn is_flag_or_var(target: Target) -> bool {
    matches!(target, Target::Flag(_) | Target::Var(_))
}

#[test]
fn hidden_flag_reads_the_flag() {
    let (_s, st) = event_store(&events_with(&[(0, 0), (0x173, 0)], &[], &[]));
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs(&st, Target::Flag(0x173)).unwrap(),
        [ref_of(
            EVENTS_0,
            "hidden_flag",
            1,
            Target::Flag(0x173),
            Access::Read
        )]
    );
}

#[test]
fn a_hidden_item_reads_and_writes_the_base_plus_n() {
    // Platinum's hidden items start at flag 0x2DA (284 of them). 8300 lies past the range;
    // a background event of another type is no hidden item.
    let (_s, st) = event_store(&events_with(
        &[],
        &[(8003, 2), (8300, 2), (8283, 2), (8004, 0)],
        &[],
    ));
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [
            ref_of(
                EVENTS_0,
                "hidden_item",
                0,
                Target::Flag(0x2DD),
                Access::ReadWrite
            ),
            ref_of(
                EVENTS_0,
                "hidden_item",
                2,
                Target::Flag(0x3F5),
                Access::ReadWrite
            ),
        ]
    );
    // HeartGold (base 0x320): `data::refs` unit test `hgss_hidden_items_start_at_0x320`.
}

#[test]
fn a_trigger_reads_its_variable() {
    let (_s, st) = event_store(&events_with(&[], &[], &[0, 0x40A5]));
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [valued(
            ref_of(EVENTS_0, "var", 1, Target::Var(0x40A5), Access::Read),
            1
        )]
    );
}

#[test]
fn a_trigger_carries_its_value() {
    let mut events = events_with(&[], &[], &[0x40A5, 0x40A6]);
    events.coord_events[0].value = 2;
    // From 0x4000 on the value is no constant the index keeps.
    events.coord_events[1].value = 0x4001;
    let (_s, st) = event_store(&events);
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [
            valued(
                ref_of(EVENTS_0, "var", 0, Target::Var(0x40A5), Access::Read),
                2
            ),
            ref_of(EVENTS_0, "var", 1, Target::Var(0x40A6), Access::Read),
        ]
    );
}

#[test]
fn a_trainer_object_reads_and_writes_its_trainer_flag() {
    // 3000 is trainer 1, 5001 trainer 2 as a partner, 4999 trainer 2000: past the flags.
    let (_s, st) = event_store(&events_with(&[(0, 3000), (0, 5001), (0, 4999)], &[], &[]));
    let mut index = RefIndex::new();
    assert_eq!(
        index.refs(&st, Target::Trainer(1)).unwrap(),
        [r(EVENTS_0, "script", Some(0), Target::Trainer(1))]
    );
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [
            ref_of(
                EVENTS_0,
                "script",
                0,
                Target::Flag(0x551),
                Access::ReadWrite
            ),
            ref_of(
                EVENTS_0,
                "script",
                1,
                Target::Flag(0x552),
                Access::ReadWrite
            ),
        ]
    );
    assert_eq!(index.count(&st, Target::Trainer(2000)).unwrap(), 1);
}

#[test]
fn zero_flags_and_variables_are_no_reference() {
    let (_s, st) = event_store(&events_with(&[(0, 0), (0, 1)], &[(0, 2)], &[0, 0]));
    let mut index = RefIndex::new();
    assert_eq!(index.refs_where(&st, is_flag_or_var).unwrap(), []);
    assert_eq!(index.count(&st, Target::Flag(0)).unwrap(), 0);
    assert_eq!(index.count(&st, Target::Var(0)).unwrap(), 0);
}

const LEVEL_5: &str = "expanded/scripts/0005.json";

fn level_script(conditions: &[(u16, u16, u16)]) -> Vec<u8> {
    let conditions: Vec<String> = conditions
        .iter()
        .map(|(var, value, script)| {
            format!("{{\"var\": {var}, \"value\": {value}, \"script_id\": {script}}}")
        })
        .collect();
    format!(
        "{{\"header_entries\": [{{\"type\": \"on_var_condition\"}}], \"var_conditions\": [{}]}}",
        conditions.join(", ")
    )
    .into_bytes()
}

#[test]
fn a_level_script_condition_reads_its_variable() {
    let fx = TextFixture::exported();
    fx.write(LEVEL_5, &level_script(&[(0x4015, 1, 1), (0x4016, 0, 2)]));
    let st = Store::new(Arc::new(fx.project()));
    let mut index = RefIndex::new();
    let level = id(AssetKind::Script, 5);
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [
            valued(
                ref_of(
                    level,
                    "var_conditions",
                    0,
                    Target::Var(0x4015),
                    Access::Read
                ),
                1
            ),
            valued(
                ref_of(
                    level,
                    "var_conditions",
                    1,
                    Target::Var(0x4016),
                    Access::Read
                ),
                0
            ),
        ]
    );
    // Changed: read anew. `var` 0 ends the table, what follows is no condition.
    fx.write(
        LEVEL_5,
        &level_script(&[(0x4017, 1, 1), (0, 0, 0), (0x4018, 1, 2)]),
    );
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [level]);
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [valued(
            ref_of(
                level,
                "var_conditions",
                0,
                Target::Var(0x4017),
                Access::Read
            ),
            1
        )]
    );
    // A level script that does not read contributes nothing.
    fx.write(LEVEL_5, b"{ not json");
    assert_eq!(index.refresh_scripts_now(&st).unwrap(), [level]);
    assert_eq!(index.refs_where(&st, is_flag_or_var).unwrap(), []);
}

#[test]
fn a_level_script_condition_carries_its_value() {
    let fx = TextFixture::exported();
    // The engine compares the variable with the value; from 0x4000 on the value is a variable.
    fx.write(
        LEVEL_5,
        &level_script(&[(0x4015, 3, 1), (0x4016, 0x4017, 2), (0x4018, 0, 3)]),
    );
    let st = Store::new(Arc::new(fx.project()));
    let mut index = RefIndex::new();
    let level = id(AssetKind::Script, 5);
    assert_eq!(
        index.refs_where(&st, is_flag_or_var).unwrap(),
        [
            valued(
                ref_of(
                    level,
                    "var_conditions",
                    0,
                    Target::Var(0x4015),
                    Access::Read
                ),
                3
            ),
            ref_of(
                level,
                "var_conditions",
                1,
                Target::Var(0x4016),
                Access::Read
            ),
            valued(
                ref_of(
                    level,
                    "var_conditions",
                    2,
                    Target::Var(0x4018),
                    Access::Read
                ),
                0
            ),
        ]
    );
}

#[test]
fn usage_counts_read_write_and_sources_in_one_pass() {
    let (_fx, mut st) = script_store(
        "script Main #1:\n    SetFlag 0x173\n    CheckFlag 0x173\n    SetVar 0x4040, 1\n    End\n",
    );
    st.edit_event(0, "Objekt", |e| {
        e.object_events.push(ObjectEvent {
            hidden_flag: 0x173,
            ..object(3000)
        });
        Ok(())
    })
    .unwrap();
    let mut index = RefIndex::new();
    let flags = index.usage(&st, FlagKind::Flag).unwrap();
    assert_eq!(
        flags[&0x173],
        Usage {
            read: 2,
            write: 1,
            unknown: 0,
            sources: 2
        }
    );
    // The trainer flag of trainer 1: the engine reads and sets it.
    assert_eq!(
        flags[&0x551],
        Usage {
            read: 1,
            write: 1,
            unknown: 0,
            sources: 1
        }
    );
    assert_eq!(flags.len(), 2, "{flags:?}");
    let vars = index.usage(&st, FlagKind::Var).unwrap();
    assert_eq!(
        vars.into_iter().collect::<Vec<_>>(),
        [(
            0x4040,
            Usage {
                read: 0,
                write: 1,
                unknown: 0,
                sources: 1
            }
        )]
    );
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
    // Target 300 ms, limit 1000 ms. In the parallel test run the other tests of this binary
    // read the same member files at the same time, so the time measures the contention (2.8
    // to 3.0 s in the gate runs on main of 2026-10-30, 230 to 340 ms alone): printed, and
    // judged only with `G4_TIMING_STRICT=1`, which `scripts/gate.sh` sets for this test alone
    // in its timing step (release, one thread), like the build graph timings.
    let started = Instant::now();
    index.rebuild_full(&st).unwrap();
    let took = started.elapsed();
    eprintln!("reference index built warm in {took:?} (target 300 ms)");
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        assert!(took < Duration::from_millis(1000), "{took:?}");
    }
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

// --- the world: headers, matrices, warps, area data and maps (`metadata/entity_refs.yaml`) ---

/// Every reference of `source`, in the index's order.
fn refs_of(index: &mut RefIndex, st: &Store, source: AssetId) -> Vec<Ref> {
    index
        .refs_where(st, |_| true)
        .unwrap()
        .into_iter()
        .filter(|r| r.source == source)
        .collect()
}

#[test]
fn a_header_refers_to_its_matrix_event_file_scripts_archive_area_and_place() {
    let s = fixture();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    // Header 0 of the fixture: area data 0, matrix 0, script file 0, level script 1, text
    // archive 433, event file 0, place 1, encounter file 1.
    let header_0: Vec<Ref> = refs_of(&mut index, &st, HEADERS)
        .into_iter()
        .filter(|r| r.slot == Some(0))
        .collect();
    let at = |field, target| r(HEADERS, field, Some(0), target);
    assert_eq!(
        header_0,
        [
            at("matrix_id", Target::Matrix(0)),
            at("event_file_id", Target::EventFile(0)),
            at("script_file_id", Target::Script(0)),
            at("level_script_id", Target::Script(1)),
            at("text_archive_id", Target::TextArchive(433)),
            at("area_data_id", Target::AreaData(0)),
            at("location_name", Target::Place(1)),
            // `wild_pokemon` comes from the same table.
            at("wild_pokemon", Target::Encounter(1)),
        ]
    );
    // Header 1: area data 1, place 200, no encounters.
    assert_eq!(
        index.refs(&st, Target::AreaData(1)).unwrap(),
        [r(HEADERS, "area_data_id", Some(1), Target::AreaData(1))]
    );
    assert_eq!(
        index.refs(&st, Target::Place(200)).unwrap(),
        [r(HEADERS, "location_name", Some(1), Target::Place(200))]
    );
    assert_eq!(index.count(&st, Target::Matrix(0)).unwrap(), 2);
}

/// Matrix `m` written to the unpacked matrix folder.
fn write_matrix(project: &Project, index: u16, m: &g4_formats::matrix::Matrix) {
    write(&project.member_path(AssetKind::Matrix, index), &m.write());
}

#[test]
fn a_matrix_refers_to_its_maps_and_headers() {
    let s = fixture();
    let project = Project::open(s.dir.path()).unwrap();
    let plain = g4_formats::matrix::Matrix {
        name: "m".into(),
        width: 2,
        height: 1,
        headers: None,
        altitudes: None,
        maps: vec![0, 1],
    };
    write_matrix(&project, 0, &plain);
    // Matrix 1: three cells, the middle one empty, with a header layer.
    write_matrix(
        &project,
        1,
        &g4_formats::matrix::Matrix {
            name: "welt".into(),
            width: 3,
            height: 1,
            headers: Some(vec![1, 1, 0]),
            altitudes: None,
            maps: vec![7, 0xFFFF, 2],
        },
    );
    let st = store_of(&s);
    let mut index = RefIndex::new();
    let m1 = id(AssetKind::Matrix, 1);
    assert_eq!(
        refs_of(&mut index, &st, m1),
        [
            r(m1, "maps", Some(0), Target::Map(7)),
            r(m1, "maps", Some(2), Target::Map(2)),
            r(m1, "headers", Some(0), Target::Header(1)),
            r(m1, "headers", Some(1), Target::Header(1)),
            r(m1, "headers", Some(2), Target::Header(0)),
        ]
    );
    // An empty cell is no map.
    assert_eq!(index.count(&st, Target::Map(0xFFFF)).unwrap(), 0);
    let m0 = id(AssetKind::Matrix, 0);
    assert_eq!(
        refs_of(&mut index, &st, m0),
        [
            r(m0, "maps", Some(0), Target::Map(0)),
            r(m0, "maps", Some(1), Target::Map(1)),
        ]
    );
}

fn warp(dest_header_id: u16, dest_warp_id: u16) -> WarpEvent {
    WarpEvent {
        x: 0,
        z: 0,
        dest_header_id,
        dest_warp_id,
        height: 0,
    }
}

#[test]
fn a_warp_refers_to_its_header_and_its_door() {
    let mut events = events_with(&[], &[], &[]);
    events.warp_events = vec![warp(0, 0), warp(0, 1), warp(1, 2)];
    let (_s, st) = event_store(&events);
    let mut index = RefIndex::new();
    let e0 = id(AssetKind::Event, 0);
    assert_eq!(
        index.refs(&st, Target::Header(1)).unwrap(),
        [r(
            e0,
            "warp_events.dest_header_id",
            Some(2),
            Target::Header(1)
        )]
    );
    assert_eq!(
        index
            .refs(
                &st,
                Target::Warp {
                    header: 1,
                    index: 2
                }
            )
            .unwrap(),
        [r(
            e0,
            "warp_events.dest_warp_id",
            Some(2),
            Target::Warp {
                header: 1,
                index: 2
            }
        )]
    );
    // The door is the pair: warp 2 of header 0 has no reference.
    assert_eq!(
        index
            .count(
                &st,
                Target::Warp {
                    header: 0,
                    index: 2
                }
            )
            .unwrap(),
        0
    );
    assert_eq!(index.count(&st, Target::Header(0)).unwrap(), 2);
}

/// `bytes` of a Platinum map as HeartGold/SoulSilver has it: the empty background-sound block
/// after the four section lengths.
fn hgss_map(bytes: &[u8]) -> Vec<u8> {
    [&bytes[..16], &g4_formats::map::BLANK_BGS, &bytes[16..]].concat()
}

#[test]
fn area_data_and_maps_refer_to_textures_and_models() {
    // Platinum: area data 1 has building tileset 1 and map tileset 2; every map of the
    // fixture has one building of model 7.
    let s = common::synthetic_project();
    let st = store_of(&s);
    let mut index = RefIndex::new();
    let a1 = id(AssetKind::AreaData, 1);
    assert_eq!(
        index.refs(&st, Target::MapTexture(2)).unwrap(),
        [r(a1, "map_tileset", None, Target::MapTexture(2))]
    );
    assert_eq!(
        index.refs(&st, Target::BuildingTexture(1)).unwrap(),
        [r(a1, "building_tileset", None, Target::BuildingTexture(1))]
    );
    assert_eq!(
        index.refs(&st, Target::BuildingModel(7)).unwrap(),
        (0..3)
            .map(|m| r(
                id(AssetKind::Map, m),
                "buildings.model_id",
                Some(0),
                Target::BuildingModel(7)
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        index.count(&st, Target::InteriorBuildingModel(7)).unwrap(),
        0
    );

    // HeartGold: header 0 shows matrix 0 (map 0) with area data 0 (inside), header 1 matrix 1
    // (map 1) with area data 1 (outside).
    let s = common::synthetic_hgss_project(&["Innen", "Außen"], 2);
    let project = Project::open(s.dir.path()).unwrap();
    for kind in [
        AssetKind::Event,
        AssetKind::Encounter,
        AssetKind::Personal,
        AssetKind::Learnset,
        AssetKind::Evolution,
        AssetKind::TrainerProperties,
        AssetKind::TrainerParty,
    ] {
        std::fs::create_dir_all(project.unpacked_dir(kind)).unwrap();
    }
    for (i, area_type) in [(0u16, 0u8), (1, 1)] {
        write(
            &project.member_path(AssetKind::AreaData, i),
            &[0, 0, 0, 0, 0, 0, area_type, 0],
        );
        write_matrix(
            &project,
            i,
            &g4_formats::matrix::Matrix {
                name: "m".into(),
                width: 1,
                height: 1,
                headers: None,
                altitudes: None,
                maps: vec![i],
            },
        );
        write(
            &project.member_path(AssetKind::Map, i),
            &hgss_map(&common::synthetic_map_bytes(i as u8)),
        );
    }
    let mut st = store_of(&s);
    st.edit_header(1, "Welt", |h| {
        if let MapHeader::HGSS(h) = h {
            h.matrix_id = 1;
            h.area_data_id = 1;
        }
        Ok(())
    })
    .unwrap();
    let mut index = RefIndex::new();
    let (map0, map1) = (id(AssetKind::Map, 0), id(AssetKind::Map, 1));
    assert_eq!(
        index.refs(&st, Target::InteriorBuildingModel(7)).unwrap(),
        [r(
            map0,
            "interior_buildings.model_id",
            Some(0),
            Target::InteriorBuildingModel(7)
        )]
    );
    assert_eq!(
        index.refs(&st, Target::BuildingModel(7)).unwrap(),
        [r(
            map1,
            "buildings.model_id",
            Some(0),
            Target::BuildingModel(7)
        )]
    );
    // Area data 0 goes outside: map 0 follows on the next refresh, as a full build has it.
    st.edit_area_data(0, "Außen", |a| {
        if let g4_formats::area_data::AreaData::Hgss { area_type, .. } = a {
            *area_type = 1;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        index.count(&st, Target::InteriorBuildingModel(7)).unwrap(),
        0
    );
    let outside = index.refs(&st, Target::BuildingModel(7)).unwrap();
    assert_eq!(outside.len(), 2);
    let mut full = RefIndex::new();
    assert_eq!(full.refs(&st, Target::BuildingModel(7)).unwrap(), outside);
}

#[test]
fn strength_is_hard_for_every_source_in_3d_4() {
    assert!(SOFT_SOURCES.is_empty());
    for kind in AssetKind::ALL {
        assert_eq!(strength(id(kind, 0)), Strength::Hard, "{kind:?}");
    }
}

#[test]
fn refresh_follows_a_matrix_edit() {
    let s = fixture();
    let mut st = store_of(&s);
    let mut index = RefIndex::new();
    let m0 = id(AssetKind::Matrix, 0);
    assert_eq!(index.count(&st, Target::Map(5)).unwrap(), 0);
    st.edit_matrix(0, "Zelle", |m| {
        m.maps[1] = 5;
        Ok(())
    })
    .unwrap();
    assert_eq!(index.refresh(&st).unwrap(), [m0]);
    assert_eq!(
        index.refs(&st, Target::Map(5)).unwrap(),
        [r(m0, "maps", Some(1), Target::Map(5))]
    );
    assert_eq!(index.count(&st, Target::Map(1)).unwrap(), 0);
    st.undo().unwrap();
    assert_eq!(index.count(&st, Target::Map(5)).unwrap(), 0);
    assert_eq!(index.count(&st, Target::Map(1)).unwrap(), 1);
}

/// The world edges Gutachten C §2.4 measured, per game: header → matrix, matrix → map, warp →
/// header (Script → header follows with the script commands).
const WORLD_EDGES: [(&str, [usize; 3]); 2] =
    [("CPUD", [593, 1172, 1213]), ("IPKD", [540, 958, 1317])];

/// Script → header (`Warp`; a call with a variable is not counted) per game code; other games
/// are printed only. Platinum DE as measured in Gutachten C §2.4. HeartGold DE: the Gutachten's
/// 154 counted `MoveWarp` with a decimal first argument as a header too (its regex
/// `(Warp|MoveWarp) <number>`), but that argument is `warp_id`, a warp index of the script's own
/// header (`hgss_v2.json`): 154 = 114 `Warp` + 40 `MoveWarp` (counted in the adopted sources of
/// HeartGold DE on 2026-11-01; the index has 43 warp indices, 3 more `MoveWarp` write warp 1 as
/// `MAPTEMP_FLAG_BASE`, and one `Warp` takes a variable).
const SCRIPT_HEADER_EDGES: [(&str, usize); 2] = [("CPUD", 67), ("IPKD", 114)];

/// The references from script sources to headers on a copy of the vanilla project at `src`
/// set up as text project with adopted scripts (below the target's temporary folder, removed
/// afterwards; the warp indices printed); `None` without the base ROM next to the project.
fn script_header_edges(src: &Path) -> Option<usize> {
    use g4_kernel::base::BaseRom;
    use g4_kernel::textproject::adopt::adopt;

    let Some(nds) = g4_kernel::start::suggest_base(src) else {
        eprintln!(
            "script → header skipped: no base ROM next to {}",
            src.display()
        );
        return None;
    };
    let folder = src.file_name()?.to_string_lossy().into_owned();
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("refs-{folder}"));
    let _ = std::fs::remove_dir_all(&root);
    common::copy_dir(src, &root);
    let (check, _mark) = g4_kernel::start::assign_base(&root, &nds).unwrap();
    assert_eq!(check.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
    let st = Store::new(Arc::new(Project::open(&root).unwrap()));
    let scripts: Vec<Ref> = RefIndex::new()
        .refs_where(&st, |t| {
            matches!(t, Target::Header(_) | Target::WarpIndex(_))
        })
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .collect();
    let n = scripts
        .iter()
        .filter(|r| matches!(r.target, Target::Header(_)))
        .count();
    eprintln!("script → warp index: {}", scripts.len() - n);
    drop(st);
    let _ = std::fs::remove_dir_all(&root);
    Some(n)
}

/// An edge to count: its name, the source kind and which targets.
type Edge = (&'static str, AssetKind, fn(&Target) -> bool);

#[test]
fn vanilla_world_edges_match_the_measured_counts() {
    let Some(st) = vanilla_store() else {
        return;
    };
    let mut index = RefIndex::new();
    let all = index.refs_where(&st, |_| true).unwrap();
    let count = |source: AssetKind, target: fn(&Target) -> bool| {
        all.iter()
            .filter(|r| r.source.kind == source && target(&r.target))
            .count()
    };
    let distinct = |source: AssetKind, target: fn(&Target) -> bool| {
        all.iter()
            .filter(|r| r.source.kind == source && target(&r.target))
            .map(|r| r.target)
            .collect::<std::collections::HashSet<_>>()
            .len()
    };
    let edges: [Edge; 13] = [
        ("header → matrix", AssetKind::HeaderTable, |t| {
            matches!(t, Target::Matrix(_))
        }),
        ("matrix → map", AssetKind::Matrix, |t| {
            matches!(t, Target::Map(_))
        }),
        ("warp → header", AssetKind::Event, |t| {
            matches!(t, Target::Header(_))
        }),
        ("header → event file", AssetKind::HeaderTable, |t| {
            matches!(t, Target::EventFile(_))
        }),
        ("header → script", AssetKind::HeaderTable, |t| {
            matches!(t, Target::Script(_))
        }),
        ("header → text archive", AssetKind::HeaderTable, |t| {
            matches!(t, Target::TextArchive(_))
        }),
        ("header → area data", AssetKind::HeaderTable, |t| {
            matches!(t, Target::AreaData(_))
        }),
        ("header → place", AssetKind::HeaderTable, |t| {
            matches!(t, Target::Place(_))
        }),
        ("matrix → header", AssetKind::Matrix, |t| {
            matches!(t, Target::Header(_))
        }),
        ("warp → warp", AssetKind::Event, |t| {
            matches!(t, Target::Warp { .. })
        }),
        ("area data → textures", AssetKind::AreaData, |t| {
            matches!(t, Target::MapTexture(_) | Target::BuildingTexture(_))
        }),
        ("map → building model", AssetKind::Map, |t| {
            matches!(t, Target::BuildingModel(_))
        }),
        ("map → interior building model", AssetKind::Map, |t| {
            matches!(t, Target::InteriorBuildingModel(_))
        }),
    ];
    let code = st.project().game_code.clone();
    let mut counted = Vec::new();
    for (name, source, target) in edges {
        let n = count(source, target);
        eprintln!("{code} {name}: {n} / {}", distinct(source, target));
        counted.push(n);
    }
    if let Some(n) = script_header_edges(&st.project().root) {
        eprintln!("{code} script → header: {n}");
        if let Some(&(_, want)) = SCRIPT_HEADER_EDGES.iter().find(|(c, _)| *c == code) {
            let off = (n as f64 - want as f64).abs() / want as f64;
            assert!(off <= 0.01, "{code}: script → header {n} against {want}");
        }
    }
    let Some((_, expected)) = WORLD_EDGES.iter().find(|(c, _)| *c == code) else {
        eprintln!("{code}: no measured counts, printed only");
        return;
    };
    for (n, want) in counted.iter().zip(expected) {
        let off = (*n as f64 - *want as f64).abs() / *want as f64;
        assert!(off <= 0.01, "{code}: {counted:?} against {expected:?}");
    }
}

// --- scripts as a source for headers and warp indices (3d.4) ---

#[test]
fn warp_commands_refer_to_a_header() {
    let (_fx, st) = script_store(
        "script Main #1:\n    Warp 3, 0, 5, 5, 0\n    End\n\
         script Second #2:\n    Warp 0, 0, 1, 1, 0\n    End\n",
    );
    let mut index = RefIndex::new();
    assert_eq!(
        script_refs(&mut index, &st, Target::Header(3)),
        [r(SCRIPT_0, "Warp", Some(2), Target::Header(3))]
    );
    // Header 0 is a header like any other, not "none".
    assert_eq!(
        script_refs(&mut index, &st, Target::Header(0)),
        [r(SCRIPT_0, "Warp", Some(5), Target::Header(0))]
    );
}

#[test]
fn set_warp_event_pos_refers_to_a_warp_index() {
    let (_fx, st) = script_store(
        "script Main #1:\n    SetWarpEventPos 2, 10, 12\n    End\n\
         script Second #2:\n    SetWarpPosition 0, 1, 1\n    End\n",
    );
    let mut index = RefIndex::new();
    assert_eq!(
        script_refs(&mut index, &st, Target::WarpIndex(2)),
        [r(
            SCRIPT_0,
            "SetWarpEventPos",
            Some(2),
            Target::WarpIndex(2)
        )]
    );
    // DSPRE's (legacy) name counts under rotom's; warp 0 is the first warp, not "none".
    assert_eq!(
        script_refs(&mut index, &st, Target::WarpIndex(0)),
        [r(
            SCRIPT_0,
            "SetWarpEventPos",
            Some(5),
            Target::WarpIndex(0)
        )]
    );
}

#[test]
fn a_warp_with_a_variable_is_counted_not_indexed() {
    let (_fx, st) = script_store(
        "script Main #1:\n    Warp 0x4001, 0, 5, 5, 0\n    SetWarpEventPos 0x8004, 1, 1\n\
         \x20   End\nscript Second #2:\n    End\n",
    );
    let mut index = RefIndex::new();
    let world = index
        .refs_where(&st, |t| {
            matches!(t, Target::Header(_) | Target::WarpIndex(_))
        })
        .unwrap()
        .into_iter()
        .filter(|r| r.source.kind == AssetKind::Script)
        .count();
    assert_eq!(world, 0);
    // The variables are counted as what they are: a global one is a reference to it, a
    // local one a script-local count.
    assert_eq!(
        script_refs(&mut index, &st, Target::Var(0x4001)),
        [r(SCRIPT_0, "Warp", Some(2), Target::Var(0x4001))]
    );
    assert_eq!(index.script_counts(&st).unwrap().script_local, 1);
}
