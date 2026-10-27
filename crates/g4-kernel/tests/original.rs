#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): resetting each store kind of a vanilla copy to its base ROM gives
//! the base member back byte for byte, within the time of the gate, as one undo step.
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::data::encounter::Encounters;
use g4_kernel::base::BaseRom;
use g4_kernel::original::{ResetTarget, plan_reset, reset};
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::store::Store;

/// Longest a reset may take, from opening the base ROM to the recorded step (the gate of 4.1b).
const RESET_MS: u128 = 200;

/// Edits `target` the way a user would.
fn edit(store: &mut Store, target: &ResetTarget) {
    match *target {
        ResetTarget::Map { index } => {
            store
                .edit_map(index, "Zelle", |m| {
                    m.permissions[0].kind = m.permissions[0].kind.wrapping_add(1);
                    Ok(())
                })
                .unwrap();
        }
        ResetTarget::EventFile { index } => {
            store
                .edit_event(index, "Figur", |e| {
                    assert!(
                        !e.object_events.is_empty(),
                        "event file {index} has no object"
                    );
                    e.object_events[0].x += 1;
                    Ok(())
                })
                .unwrap();
        }
        ResetTarget::TextArchive { index } => {
            store
                .edit_text_archive(index, "Text", |d| d.messages[0].text.push_str(" Studio"))
                .unwrap();
        }
        ResetTarget::TextMessage { archive, message } => {
            store
                .edit_text_archive(archive, "Text", |d| {
                    d.messages[usize::from(message)].text.push_str(" Studio");
                })
                .unwrap();
        }
        ResetTarget::Trainer { index } => {
            store
                .edit_trainer(index, "Level", |t| t.party[0].level += 1)
                .unwrap();
        }
        ResetTarget::Species { index } => {
            store
                .edit_personal(index, "KP", |p| p.set_hp(p.hp().wrapping_add(1)))
                .unwrap();
        }
        ResetTarget::Encounter { index } => {
            store
                .edit_encounter(index, "Rate", |e| match e {
                    Encounters::Pt(e) => e.set_grass_rate(e.grass_rate() + 1),
                    Encounters::Hgss(e) => e.set_grass_rate(e.grass_rate().wrapping_add(1)),
                })
                .unwrap();
        }
        _ => unreachable!("no store kind: {target:?}"),
    }
}

/// The member files of `target` (archive kind, index): a trainer has two, a Pokémon three.
fn members(target: &ResetTarget) -> Vec<(AssetKind, u16)> {
    target
        .assets()
        .into_iter()
        .flat_map(|a| match a.kind {
            AssetKind::Trainer => vec![
                (AssetKind::TrainerProperties, a.index),
                (AssetKind::TrainerParty, a.index),
            ],
            kind => vec![(kind, a.index)],
        })
        .collect()
}

#[test]
fn reset_of_each_store_kind_gives_the_base_member_back() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    // Opened once with its SHA-1, for the checks; each reset opens it again without one.
    let base = BaseRom::open(&nds, None).unwrap();
    let project = Arc::new(Project::open(&copy).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    let targets = [
        ResetTarget::Map { index: 3 },
        ResetTarget::EventFile { index: 3 },
        ResetTarget::TextArchive { index: 0 },
        ResetTarget::TextMessage {
            archive: 1,
            message: 0,
        },
        ResetTarget::Trainer { index: 1 },
        ResetTarget::Species { index: 1 },
        ResetTarget::Encounter { index: 1 },
    ];
    let mut slow = Vec::new();
    for target in targets {
        let label = target.label();
        edit(&mut store, &target);
        store.save().unwrap();
        let members = members(&target);
        let member =
            |kind: AssetKind, index: u16| std::fs::read(project.member_path(kind, index)).unwrap();
        assert!(
            members
                .iter()
                .any(|&(kind, i)| member(kind, i) != base.member(kind, i).unwrap().unwrap()),
            "{label}: the edit left every member as in the base"
        );
        // The plan loads every asset of the target.
        let plan = plan_reset(&mut store, &base, &target).unwrap();
        assert!(!plan.same, "{label}");
        let edited: Vec<(AssetId, Vec<u8>)> = target
            .assets()
            .into_iter()
            .map(|a| (a, store.current_bytes(a).unwrap().unwrap()))
            .collect();

        let started = Instant::now();
        let original = BaseRom::open_unchecked(&nds, base.sha1()).unwrap();
        let result = reset(&mut store, &original, &target, None).unwrap();
        let ms = started.elapsed().as_millis();
        drop(original);
        println!(
            "reset {} {label} ms={ms} (kernel {} ms)",
            project.game_code, result.ms
        );
        assert!(result.recorded, "{label}");
        store.save().unwrap();
        for &(kind, i) in &members {
            assert!(
                member(kind, i) == base.member(kind, i).unwrap().unwrap(),
                "{label}: {kind} {i:04} differs from the base member after the reset"
            );
        }
        if ms > RESET_MS {
            slow.push(format!("{label}: {ms} ms"));
        }

        store.undo().unwrap();
        for (asset, bytes) in edited {
            assert!(
                store.current_bytes(asset).unwrap().unwrap() == bytes,
                "{label}: undo did not bring back the edit of {asset}"
            );
        }
    }
    assert!(slow.is_empty(), "over {RESET_MS} ms: {slow:?}");
}
