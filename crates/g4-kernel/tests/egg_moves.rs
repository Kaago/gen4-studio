#![cfg(feature = "test-support")]
//! Egg moves: where the table lives (HGSS member, Platinum overlay 5 or DSPRE's patch NARC),
//! the signature and budget of the overlay slice, editing through the store, on the synthetic
//! projects (`test_support::synthetic_hgss_egg_moves`, `synthetic_platinum_egg_moves`) and,
//! gated by `G4_TEST_PROJECT` (read only), the measured counts of the vanilla tables.
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::data::egg_moves::EggMoves;
use g4_formats::game::Family;
use g4_kernel::data::egg_moves::{EggMovesSource, budget_full, egg_moves_place, signature_missing};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetKind, Location, Project};
use g4_kernel::store::{EGG_MOVES, Store};

fn store_of(root: &std::path::Path) -> Store {
    Store::new(Arc::new(Project::open(root).unwrap()))
}

fn overlay_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("arm9_overlays/ov005.bin")
}

#[test]
fn hgss_fixture_reads_the_member_as_narc_source() {
    let s = common::synthetic_hgss_egg_moves();
    let root = s.dir.path();
    let p = Project::open(root).unwrap();
    assert_eq!(p.member_count(AssetKind::EggMoves).unwrap(), 1);
    let place = egg_moves_place(&p).unwrap().unwrap();
    assert_eq!(place.source, EggMovesSource::Narc);
    assert_eq!(
        place.location,
        Location::Member(root.join("unpacked/eggMoves/0000"))
    );
    assert_eq!(place.budget, None);
    assert_eq!(
        p.member_location(AssetKind::EggMoves, 0).unwrap(),
        place.location
    );
    let mut st = store_of(root);
    let table = st.egg_moves().unwrap();
    assert_eq!(table.entries.len(), 3);
    assert_eq!(table.moves_of(1), Some([5, 7].as_slice()));
    assert_eq!(table.moves_of(3), Some([].as_slice()));
    assert_eq!(table.budget, None);
    assert!(root.join("unpacked/eggMoves/0000").is_file());
    // A member grows: adding a move needs no budget.
    st.edit_egg_moves("add", |t| t.set_moves(4, vec![9, 10]))
        .unwrap()
        .unwrap();
    st.save().unwrap();
    let saved = std::fs::read(root.join("unpacked/eggMoves/0000")).unwrap();
    assert_eq!(
        saved,
        common::egg_move_bytes(&[20001, 5, 7, 20003, 20004, 9, 10, 0xFFFF])
    );
}

#[test]
fn platinum_overlay_slice_is_used_when_the_signature_holds() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET);
    let root = s.dir.path();
    let p = Project::open(root).unwrap();
    // Nothing to unpack, no error.
    assert_eq!(p.ensure_unpacked(AssetKind::EggMoves).unwrap(), 1);
    assert!(!root.join("unpacked/eggMoves").exists());
    assert_eq!(p.member_count(AssetKind::EggMoves).unwrap(), 1);
    let place = egg_moves_place(&p).unwrap().unwrap();
    assert_eq!(place.source, EggMovesSource::Overlay);
    assert_eq!(place.budget, Some(3820));
    assert_eq!(
        place.location,
        Location::Slice {
            path: overlay_path(root),
            offset: 0x2923E,
            len: 3820
        }
    );
    let mut st = store_of(root);
    let table = st.egg_moves().unwrap();
    assert_eq!(table.budget, Some(3820));
    assert_eq!(table.words(), 1910);
    assert_eq!(table.budget_words(), Some(1910));
    assert_eq!(table.moves_of(4), Some([9].as_slice()));
    assert_eq!(st.locked(EGG_MOVES), None);
}

#[test]
fn with_the_patch_narc_the_overlay_is_left_alone() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET);
    let root = s.dir.path();
    common::write_egg_move_patch(root);
    let overlay_before = std::fs::read(overlay_path(root)).unwrap();
    let p = Project::open(root).unwrap();
    let place = egg_moves_place(&p).unwrap().unwrap();
    assert_eq!(place.source, EggMovesSource::Patch);
    assert_eq!(
        place.location,
        Location::Member(root.join("unpacked/eggMoves/0000"))
    );
    assert_eq!(place.budget, None);
    let mut st = store_of(root);
    assert_eq!(st.egg_moves().unwrap().entries.len(), 3);
    assert_eq!(st.egg_moves().unwrap().budget, None);
    st.edit_egg_moves("swap", |t| t.set_moves(1, vec![5, 8]))
        .unwrap()
        .unwrap();
    st.save().unwrap();
    assert_eq!(std::fs::read(overlay_path(root)).unwrap(), overlay_before);
    assert_eq!(
        std::fs::read(root.join("unpacked/eggMoves/0000")).unwrap(),
        common::egg_move_bytes(&[20001, 5, 8, 20003, 20004, 9, 0xFFFF])
    );
}

#[test]
fn a_wrong_offset_fails_the_signature_and_locks_the_asset() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET + 2);
    let root = s.dir.path();
    let overlay_before = std::fs::read(overlay_path(root)).unwrap();
    let p = Project::open(root).unwrap();
    assert_eq!(
        egg_moves_place(&p).unwrap(),
        Err(signature_missing(0x2923E))
    );
    let mut st = store_of(root);
    let reason = "Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)";
    match st.egg_moves() {
        Err(KernelError::ReadOnly { asset, reason: r }) => {
            assert_eq!(asset, EGG_MOVES);
            assert_eq!(r, reason);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(st.locked(EGG_MOVES), Some(reason));
    let err = st
        .edit_egg_moves("swap", |t| t.set_moves(1, vec![5, 8]))
        .unwrap_err();
    assert!(
        matches!(&err, KernelError::ReadOnly { reason: r, .. } if r == reason),
        "{err:?}"
    );
    assert!(st.dirty().is_empty());
    assert_eq!(st.history().0.len(), 0);
    st.save().unwrap();
    assert_eq!(std::fs::read(overlay_path(root)).unwrap(), overlay_before);
}

#[test]
fn an_unverified_code_is_locked_with_the_reason() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET);
    let root = s.dir.path();
    std::fs::write(
        root.join("header.yaml"),
        "title: \"POKEMON PL\"\ngamecode: CPUE\nmakercode: \"01\"\n",
    )
    .unwrap();
    let p = Project::open(root).unwrap();
    let reason = "Ei-Attacken: Lage in Overlay 5 für CPUE nicht geprüft";
    assert_eq!(egg_moves_place(&p).unwrap(), Err(reason.to_owned()));
    let mut st = store_of(root);
    match st.egg_moves() {
        Err(KernelError::ReadOnly { reason: r, .. }) => assert_eq!(r, reason),
        other => panic!("{other:?}"),
    }
}

#[test]
fn edit_within_the_budget_records_one_step_and_undo_restores_the_bytes() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET);
    let root = s.dir.path();
    let overlay_before = std::fs::read(overlay_path(root)).unwrap();
    let mut st = store_of(root);
    let original = st.current_bytes(EGG_MOVES).unwrap();
    assert_eq!(original, None, "not loaded yet");
    st.egg_moves().unwrap();
    let original = st.current_bytes(EGG_MOVES).unwrap().unwrap();
    assert_eq!(original, common::full_egg_move_table());
    // Swap: one step.
    st.edit_egg_moves("swap", |t| t.set_moves(1, vec![5, 8]))
        .unwrap()
        .unwrap();
    assert_eq!(st.history().0, vec!["swap".to_owned()]);
    assert_eq!(st.dirty(), vec![EGG_MOVES]);
    // One away, one more: still within the budget, padded with 0xFFFF.
    st.edit_egg_moves("trade", |t| {
        t.set_moves(1, vec![5]);
        t.set_moves(3, vec![11]);
    })
    .unwrap()
    .unwrap();
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![EGG_MOVES]);
    let after = std::fs::read(overlay_path(root)).unwrap();
    assert_eq!(after.len(), overlay_before.len());
    let range = common::PT_EGG_MOVES_OFFSET..common::PT_EGG_MOVES_OFFSET + 3820;
    assert_eq!(after[..range.start], overlay_before[..range.start]);
    assert_eq!(after[range.end..], overlay_before[range.end..]);
    let slice = &after[range.clone()];
    assert!(EggMoves::signature_ok(slice));
    let table = EggMoves::read_fixed(slice).unwrap();
    assert_eq!(table.moves_of(1), Some([5].as_slice()));
    assert_eq!(table.moves_of(3), Some([11].as_slice()));
    assert_eq!(table.words(), 1910);
    // Undo both steps: the bytes are the original ones, and so is the file after a save.
    st.undo().unwrap();
    st.undo().unwrap();
    assert_eq!(st.current_bytes(EGG_MOVES).unwrap().unwrap(), original);
    st.save().unwrap();
    assert_eq!(std::fs::read(overlay_path(root)).unwrap(), overlay_before);
    // The budget survives undo and redo: an add over it is still refused.
    st.redo().unwrap();
    assert_eq!(st.egg_moves().unwrap().budget, Some(3820));
    assert!(matches!(
        st.edit_egg_moves("add", |t| t.set_moves(4, vec![9, 10])),
        Err(KernelError::Refused(_))
    ));
}

#[test]
fn edit_over_the_budget_is_refused_and_records_nothing() {
    let s = common::synthetic_platinum_egg_moves(common::PT_EGG_MOVES_OFFSET);
    let root = s.dir.path();
    let overlay_before = std::fs::read(overlay_path(root)).unwrap();
    let mut st = store_of(root);
    let err = st
        .edit_egg_moves("add", |t| t.set_moves(4, vec![9, 10]))
        .unwrap_err();
    match err {
        KernelError::Refused(why) => {
            assert_eq!(why, budget_full(1910));
            assert_eq!(why, "Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen");
        }
        other => panic!("{other:?}"),
    }
    // So is a table that would no longer start with species 1.
    assert!(matches!(
        st.edit_egg_moves("drop", |t| t.set_moves(1, Vec::new())),
        Err(KernelError::Refused(_))
    ));
    assert_eq!(st.history().0.len(), 0);
    assert!(st.dirty().is_empty());
    assert_eq!(st.egg_moves().unwrap().moves_of(4), Some([9].as_slice()));
    st.save().unwrap();
    assert_eq!(std::fs::read(overlay_path(root)).unwrap(), overlay_before);
}

#[test]
fn transactions_see_and_edit_the_table() {
    let s = common::synthetic_hgss_egg_moves();
    let mut st = store_of(s.dir.path());
    let ((), change) = st
        .transaction("both", |tx| {
            assert_eq!(tx.egg_moves()?.moves_of(1), Some([5, 7].as_slice()));
            tx.edit_egg_moves(|t| t.set_moves(2, vec![4]))?;
            assert_eq!(tx.egg_moves()?.moves_of(2), Some([4].as_slice()));
            Ok(())
        })
        .unwrap();
    assert!(change.is_some());
    assert_eq!(
        st.egg_moves()
            .unwrap()
            .entries
            .iter()
            .map(|e| e.species)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn diamond_pearl_has_no_egg_moves() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    std::fs::write(
        root.join("header.yaml"),
        "title: \"POKEMON D\"\ngamecode: ADAE\nmakercode: \"01\"\n",
    )
    .unwrap();
    let p = Project::open(root).unwrap();
    assert_eq!(p.family, Family::Dp);
    assert!(!p.has_archive(AssetKind::EggMoves));
    assert_eq!(p.member_count(AssetKind::EggMoves).unwrap(), 0);
    assert!(egg_moves_place(&p).unwrap().is_err());
}

/// Gate "Ei-Attacken lesen": the measured counts per game (gutachten B §5.1), `write()`
/// byte-identical to the member or the slice. Reads the project only.
#[test]
fn vanilla_egg_moves_read_with_the_measured_counts() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let p = Project::open(&root).unwrap();
    let t = Instant::now();
    let place = egg_moves_place(&p).unwrap().unwrap();
    let mut st = Store::new(Arc::new(Project::open(&root).unwrap()));
    let table = st.egg_moves().unwrap().clone();
    let ms = t.elapsed().as_millis();
    let moves: usize = table.entries.iter().map(|e| e.moves.len()).sum();
    eprintln!(
        "egg_moves {} source={} species={} moves={moves} words={} ms={ms}",
        p.game_code,
        serde_json::to_value(place.source)
            .unwrap()
            .as_str()
            .unwrap(),
        table.entries.len(),
        table.words()
    );
    let on_disk = match &place.location {
        Location::Member(path) => std::fs::read(path).unwrap(),
        Location::Slice { path, offset, len } => {
            let file = std::fs::read(path).unwrap();
            let start = usize::try_from(*offset).unwrap();
            file[start..start + len].to_vec()
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(table.write().unwrap(), on_disk);
    assert_eq!(table.entries.len(), 198);
    match p.family {
        Family::Hgss => {
            assert_eq!(place.source, EggMovesSource::Narc);
            assert_eq!(moves, 1847);
            assert_eq!(table.entries.iter().map(|e| e.moves.len()).max(), Some(14));
            assert_eq!(on_disk.len(), 4092);
        }
        Family::Pt => {
            assert_eq!(place.source, EggMovesSource::Overlay);
            assert_eq!(moves, 1711);
            assert_eq!(table.words(), 1910);
            assert_eq!(table.budget_words(), Some(1910));
        }
        Family::Dp => panic!("no egg moves for Diamond/Pearl"),
    }
}

/// Whether the vanilla marks ascend (Task 11 decides from it); prints, never fails.
#[test]
fn vanilla_egg_moves_are_in_ascending_species_order() {
    let Some(root) = common::vanilla_project() else {
        return;
    };
    let mut st = Store::new(Arc::new(Project::open(&root).unwrap()));
    let game_code = st.project().game_code.clone();
    let table = st.egg_moves().unwrap();
    let species: Vec<u16> = table.entries.iter().map(|e| e.species).collect();
    let descents: Vec<(u16, u16)> = species
        .windows(2)
        .filter(|w| w[0] >= w[1])
        .map(|w| (w[0], w[1]))
        .collect();
    eprintln!(
        "egg_moves {game_code} ascending={} first={:?} last={:?} not_ascending_at={descents:?}",
        descents.is_empty(),
        species.first(),
        species.last()
    );
}
