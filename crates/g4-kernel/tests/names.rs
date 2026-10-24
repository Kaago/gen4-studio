#![cfg(feature = "test-support")]
//! Name tables read from the text archives through the store: bank numbers, form names,
//! cache invalidation, trainer name wrapping and (gated) the vanilla message counts.
mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use g4_formats::game::{Family, Language};
use g4_kernel::data::names::{Bank, NameTables, bank_index, trainer_name};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};
use proptest::prelude::*;

fn store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    (s, st)
}

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

#[test]
fn bank_index_knows_pt_and_hgss() {
    for language in [Language::English, Language::German, Language::Spanish] {
        assert_eq!(bank_index(Family::Pt, language, Bank::Species), Some(412));
        assert_eq!(bank_index(Family::Hgss, language, Bank::Species), Some(237));
        assert_eq!(
            bank_index(Family::Pt, language, Bank::MoveDescriptions),
            Some(646)
        );
        assert_eq!(
            bank_index(Family::Hgss, language, Bank::MoveDescriptions),
            Some(749)
        );
        assert_eq!(
            bank_index(Family::Pt, language, Bank::ItemDescriptions),
            Some(391)
        );
        assert_eq!(
            bank_index(Family::Hgss, language, Bank::ItemDescriptions),
            Some(221)
        );
        assert_eq!(bank_index(Family::Pt, language, Bank::Forms), Some(697));
        assert_eq!(bank_index(Family::Hgss, language, Bank::Forms), Some(802));
        assert_eq!(bank_index(Family::Pt, language, Bank::Moves), Some(647));
        assert_eq!(bank_index(Family::Hgss, language, Bank::Moves), Some(750));
    }
    // No Western fallback for a bank the Japanese set does not list.
    assert_eq!(
        bank_index(Family::Hgss, Language::Japanese, Bank::BattleTexts),
        None
    );
    assert_eq!(
        bank_index(Family::Hgss, Language::Japanese, Bank::Species),
        Some(232)
    );
    assert_eq!(bank_index(Family::Dp, Language::English, Bank::Forms), None);
}

#[test]
fn names_read_the_banks_of_the_project() {
    let (_s, mut st) = store();
    let mut names = NameTables::new();
    assert_eq!(
        names.names(&mut st, Bank::Species).unwrap(),
        ["-----", "Bulbasaur", "Ivysaur", "Venusaur"]
    );
    assert_eq!(names.species_name(&mut st, 2), "Ivysaur");
    assert_eq!(names.move_name(&mut st, 2), "Karate Chop");
    assert_eq!(names.item_name(&mut st, 1), "Master Ball");
    assert_eq!(names.class_name(&mut st, 1), "Youngster");
    assert_eq!(
        names.description(&mut st, Bank::ItemDescriptions, 1),
        Some("The best Ball.".to_owned())
    );
    assert_eq!(
        names.description(&mut st, Bank::MoveDescriptions, 2),
        Some("Chops the foe.".to_owned())
    );
    assert_eq!(names.description(&mut st, Bank::MoveDescriptions, 3), None);
}

#[test]
fn unknown_species_is_named_by_number() {
    let (_s, mut st) = store();
    let mut names = NameTables::with_forms(BTreeMap::new());
    assert_eq!(names.species_name(&mut st, 520), "species 0520");
    assert_eq!(names.move_name(&mut st, 99), "move 0099");
    assert_eq!(names.item_name(&mut st, 99), "item 0099");
    assert_eq!(names.class_name(&mut st, 130), "class 0130");
    assert_eq!(names.trainer_name(&mut st, 7), "trainer 0007");
    assert_eq!(names.description(&mut st, Bank::Items, 99), None);
}

#[test]
fn form_member_gets_its_form_name() {
    let (_s, mut st) = store();
    let mut names = NameTables::new();
    assert_eq!(names.species_name(&mut st, 503), "Hitze-Rotom");
    assert_eq!(names.species_name(&mut st, 496), "Deoxys, Angriffsform");
    // A species of the bank is not touched by the form table.
    assert_eq!(names.species_name(&mut st, 1), "Bulbasaur");
    // The table is injectable.
    let forms = BTreeMap::from([(503, "Test form".to_owned())]);
    let mut names = NameTables::with_forms(forms);
    assert_eq!(names.species_name(&mut st, 503), "Test form");
    assert_eq!(names.species_name(&mut st, 504), "species 0504");
}

fn write_member(st: &Store, index: u16, messages: &[&str]) {
    let path = st.project().member_path(AssetKind::TextArchive, index);
    std::fs::write(&path, common::text_archive_bytes(messages)).unwrap();
    let file = std::fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(100))
        .unwrap();
}

#[test]
fn cache_invalidates_after_a_text_edit_and_after_reload() {
    let (_s, mut st) = store();
    let mut names = NameTables::new();
    assert_eq!(names.species_name(&mut st, 1), "Bulbasaur");
    let generation = st.text_generation(412);

    // An unsaved rename shows at once.
    st.edit_text_archive(412, "rename", |d| d.messages[1].text = "Bisasam".into())
        .unwrap();
    assert!(st.text_generation(412) > generation);
    assert_eq!(names.species_name(&mut st, 1), "Bisasam");
    // An untouched archive keeps its generation.
    assert_eq!(st.text_generation(647), 0);

    let after_edit = st.text_generation(412);
    st.undo().unwrap();
    assert!(st.text_generation(412) > after_edit);
    assert_eq!(names.species_name(&mut st, 1), "Bulbasaur");
    let after_undo = st.text_generation(412);
    st.redo().unwrap();
    assert!(st.text_generation(412) > after_undo);
    assert_eq!(names.species_name(&mut st, 1), "Bisasam");

    // Reading does not change the generation.
    let stable = st.text_generation(412);
    names.species_name(&mut st, 2);
    assert_eq!(st.text_generation(412), stable);

    // The file changes on disk; the store reloads it.
    write_member(&st, 412, &["-----", "Reloaded", "Ivysaur"]);
    st.reload_from_disk(text_id(412)).unwrap();
    assert!(st.text_generation(412) > stable);
    assert_eq!(names.species_name(&mut st, 1), "Reloaded");
    assert_eq!(names.species_name(&mut st, 3), "species 0003");

    // Taking the disk as base is a change of the text asset as well.
    let before_accept = st.text_generation(412);
    st.accept_disk_as_base(text_id(412)).unwrap();
    assert!(st.text_generation(412) > before_accept);
}

#[test]
fn create_text_archive_raises_its_generation() {
    let (_s, mut st) = store();
    let before = st.text_generation(698);
    let index = st.create_text_archive().unwrap();
    assert_eq!(index, 698);
    assert!(st.text_generation(698) > before);
}

#[test]
fn trainer_names_lose_their_wrapper() {
    let (_s, mut st) = store();
    st.edit_text_archive(618, "wrap", |d| {
        d.messages[1].text = "{TRAINER_NAME:Kai}".into();
    })
    .unwrap();
    let mut names = NameTables::new();
    assert_eq!(names.trainer_name(&mut st, 1), "Kai");
    // An unwrapped name is shown as it is.
    assert_eq!(names.trainer_name(&mut st, 2), "Mia");
}

#[test]
fn trainer_name_wrap_and_unwrap_are_plain() {
    assert_eq!(trainer_name::wrap("Kai"), "{TRAINER_NAME:Kai}");
    assert_eq!(trainer_name::unwrap("{TRAINER_NAME:Kai}"), "Kai");
    assert_eq!(trainer_name::unwrap("Kai"), "Kai");
    assert_eq!(trainer_name::unwrap("{TRAINER_NAME:}"), "");
    assert_eq!(
        trainer_name::unwrap("{TRAINER_NAME:Kai"),
        "{TRAINER_NAME:Kai"
    );
}

proptest! {
    #[test]
    fn trainer_name_wrap_unwrap_round_trip(name in "\\PC{0,12}") {
        let wrapped = trainer_name::wrap(&name);
        prop_assert_eq!(trainer_name::unwrap(&wrapped), name);
        prop_assert_eq!(trainer_name::wrap(&trainer_name::unwrap(&wrapped)), wrapped);
    }
}

/// Message counts per bank for a family: (species, moves, items, trainer names, classes,
/// battle texts, move descriptions, item descriptions), see spec section 2.
fn expected_counts(family: Family) -> [usize; 8] {
    match family {
        Family::Pt => [496, 468, 468, 928, 105, 2497, 468, 468],
        Family::Hgss => [496, 468, 537, 738, 129, 1717, 468, 537],
        Family::Dp => unreachable!("no Diamond/Pearl project in the gate"),
    }
}

#[test]
fn vanilla_name_counts_match_task0() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let family = project.family;
    let mut st = Store::new(project);
    let mut names = NameTables::new();
    let banks = [
        Bank::Species,
        Bank::Moves,
        Bank::Items,
        Bank::TrainerNames,
        Bank::TrainerClasses,
        Bank::BattleTexts,
        Bank::MoveDescriptions,
        Bank::ItemDescriptions,
    ];
    let counts: Vec<usize> = banks
        .iter()
        .map(|bank| names.names(&mut st, *bank).unwrap().len())
        .collect();
    eprintln!("{family:?} counts {counts:?}");
    assert_eq!(counts, expected_counts(family));
    // The forms bank has a message for each of the twelve form members.
    assert!(names.names(&mut st, Bank::Forms).unwrap().len() >= 12);
    // Spot checks that do not depend on the language: a species, a form member, an unknown one.
    assert_ne!(names.species_name(&mut st, 1), "species 0001");
    assert_eq!(names.species_name(&mut st, 503), "Hitze-Rotom");
    assert_eq!(names.species_name(&mut st, 520), "species 0520");
}

#[test]
fn every_vanilla_trainer_name_round_trips() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let mut st = Store::new(project);
    let mut names = NameTables::new();
    let all = names.names(&mut st, Bank::TrainerNames).unwrap().to_vec();
    assert!(all.len() > 700);
    let wrapped = all
        .iter()
        .filter(|n| n.starts_with("{TRAINER_NAME:"))
        .count();
    eprintln!("{} trainer names, {wrapped} wrapped", all.len());
    for (i, name) in all.iter().enumerate() {
        let plain = trainer_name::unwrap(name);
        assert!(!plain.starts_with("{TRAINER_NAME:"), "{i}: {name}");
        if name.starts_with("{TRAINER_NAME:") {
            assert_eq!(trainer_name::wrap(&plain), *name, "{i}");
        }
        assert_eq!(names.trainer_name(&mut st, i as u16), plain);
    }
}
