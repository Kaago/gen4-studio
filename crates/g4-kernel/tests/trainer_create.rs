//! Gated: creating trainers on a copy of a vanilla project (needs `G4_TEST_PROJECT`):
//! Platinum DE 928, then 930 behind the placeholder 929; HeartGold 738, 739, then 741
//! behind the placeholder 740. The battle script gets the slots, compiles clean, and a
//! store opened anew shows the placeholder locked.
#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;

use g4_formats::game::Family;
use g4_kernel::data::names::{Bank, NameTables, trainer_name};
use g4_kernel::data::trainer_create::{
    CreateTrainerReport, PLACEHOLDER_REASON, TrainerText, create_trainer, set_trainer_texts,
};
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::store::Store;

fn trainer(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Trainer,
        index,
    }
}

fn source(project: &Project, file: u16) -> String {
    match scripts::read_source(project, file).unwrap() {
        ScriptSource::Rotom(text) => text,
        other => panic!("{other:?}"),
    }
}

#[test]
fn vanilla_creates_trainers_behind_the_sight_slot() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    // (battle file, first new ID, sight slot, battle slots before)
    let (battle_file, first, seen, slots) = match project.family {
        Family::Pt => (1114, 928, 929, "#[1-928]"),
        Family::Hgss => (953, 738, 740, "#[1-739]"),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    let mut store = Store::new(project.clone());
    let mut names = NameTables::new();

    // Without adopted scripts nothing happens.
    let err = create_trainer(&project, &mut store, &mut names, 2, "Jens").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Erst Scripts übernehmen (Bereich Welt, Panel Script)"
    );
    scripts::adopt(&project, &mut |_| {}).unwrap();
    assert_eq!(scripts::battle_script_file(&project).unwrap(), battle_file);
    let vanilla = source(&project, battle_file);
    assert!(vanilla.contains(&format!("script script_1 {slots}:")), "{slots}");
    let member = project.member_path(AssetKind::Script, battle_file);
    let old_member = std::fs::read(&member).unwrap();

    // Up to the sight slot: no change to the battle script.
    let mut created = Vec::new();
    for (id, name) in (first..seen).zip(["Jens", "Mia"]) {
        let report = create_trainer(&project, &mut store, &mut names, 2, name).unwrap();
        assert_eq!(
            report,
            CreateTrainerReport {
                id,
                placeholder: None,
                script_extended: false
            }
        );
        created.push((id, name));
    }
    assert_eq!(source(&project, battle_file), vanilla);

    // The sight slot gets the placeholder, the next ID a battle slot.
    let report = create_trainer(&project, &mut store, &mut names, 2, "Lars").unwrap();
    assert_eq!(
        report,
        CreateTrainerReport {
            id: seen + 1,
            placeholder: Some(seen),
            script_extended: true
        }
    );
    created.push((seen + 1, "Lars"));
    let extended = vanilla.replacen(
        &format!("script script_1 {slots}:"),
        &format!(
            "script script_1 {}, {}]:",
            slots.trim_end_matches(']'),
            seen + 1
        ),
        1,
    );
    assert_eq!(source(&project, battle_file), extended);
    let new_member = std::fs::read(&member).unwrap();
    assert_eq!(new_member.len(), old_member.len() + 4);
    // The code after the jump table (and its end marker) is unchanged.
    let table = 4 * usize::from(seen);
    assert_eq!(old_member[table..], new_member[table + 4..]);
    let compiled = scripts::compile(&project).unwrap();
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);

    // Texts for the last trainer, then save everything.
    let texts = vec![
        TrainerText {
            kind: 0,
            text: "Ich bin Lars!\\r".into(),
        },
        TrainerText {
            kind: 1,
            text: "Verloren.".into(),
        },
        TrainerText {
            kind: 2,
            text: "Gut gekämpft.".into(),
        },
    ];
    let out = set_trainer_texts(&mut store, &mut names, seen + 1, texts).unwrap();
    assert!(out.change.is_some());
    assert!(out.problems.is_empty(), "{:?}", out.problems);
    store.save().unwrap();

    // A store opened anew: the placeholder is locked, the new trainers are not; names and
    // offsets are on disk.
    let mut store = Store::new(project.clone());
    let mut names = NameTables::new();
    store.trainer(seen).unwrap();
    assert_eq!(store.locked(trainer(seen)), Some(PLACEHOLDER_REASON));
    let bank = names.names(&mut store, Bank::TrainerNames).unwrap().to_vec();
    assert_eq!(bank.len(), usize::from(seen) + 2);
    assert_eq!(trainer_name::unwrap(&bank[usize::from(seen)]), "-");
    for (id, name) in created {
        let doc = store.trainer(id).unwrap();
        assert_eq!((doc.class, doc.party.len()), (2, 0));
        assert_eq!(store.locked(trainer(id)), None);
        assert_eq!(trainer_name::unwrap(&bank[usize::from(id)]), name);
    }
    let offsets = store.trainer_text_offsets(0).unwrap().offsets.clone();
    assert_eq!(offsets.len(), usize::from(seen) + 2);
    let entries = store.trainer_text_table(0).unwrap().entries.clone();
    let start = usize::from(offsets[usize::from(seen + 1)]) / 4;
    assert_eq!(
        entries[start..],
        [(seen + 1, 0), (seen + 1, 1), (seen + 1, 2)]
    );
    let battle_texts = names.names(&mut store, Bank::BattleTexts).unwrap();
    assert_eq!(battle_texts.len(), entries.len());
    assert_eq!(battle_texts[start], "Ich bin Lars!\\r");
}
