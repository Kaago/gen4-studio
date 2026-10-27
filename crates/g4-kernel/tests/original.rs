#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`): resetting each store kind of a vanilla copy to its base ROM gives
//! the base member back byte for byte, within the time of the gate, as one undo step.
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::data::encounter::Encounters;
use g4_kernel::base::BaseRom;
use g4_kernel::dynamic_headers;
use g4_kernel::original::{ResetTarget, header_fields, plan_reset, reset};
use g4_kernel::project::{AssetId, AssetKind, Location, Project};
use g4_kernel::store::{MapHeader, Store};
use uxie::map_header::write_map_header_to_bytes;

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

/// The header the gated header test resets.
const HEADER: u16 = 3;

/// Changes `music_day_id` and `weather_id` of header `id` (the lowest bit, which fits every
/// layout), one step.
fn edit_header(store: &mut Store, id: u16) {
    store
        .edit_header(id, "Musik und Wetter", |h| {
            match h {
                MapHeader::DP(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
                MapHeader::Pt(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
                MapHeader::HGSS(h) => {
                    h.music_day_id ^= 1;
                    h.weather_id ^= 1;
                }
            }
            Ok(())
        })
        .unwrap();
}

/// The names of the fields in which `a` and `b` differ.
fn differing(a: &MapHeader, b: &MapHeader) -> Vec<String> {
    header_fields(a)
        .into_iter()
        .zip(header_fields(b))
        .filter(|(x, y)| x != y)
        .map(|((name, _), _)| name)
        .collect()
}

/// The saved 24-byte record of header `id`: its slice of `arm9.bin`, or its file once the
/// Dynamic Headers patch is applied.
fn saved_record(project: &Project, id: u16) -> Vec<u8> {
    let at = usize::from(id) * 24;
    match project.asset_location(AssetKind::HeaderTable, 0).unwrap() {
        Location::Slice { path, offset, .. } => {
            let arm9 = std::fs::read(path).unwrap();
            let start = usize::try_from(offset).unwrap() + at;
            arm9[start..start + 24].to_vec()
        }
        Location::HeaderFiles { dir, .. } => std::fs::read(dir.join(format!("{id:04}"))).unwrap(),
        other => panic!("header table at {other:?}"),
    }
}

/// Opens the base ROM without a SHA-1 and resets header [`HEADER`] (only `fields`, or every
/// differing one); prints and returns the time from the opening to the recorded step.
fn timed_reset(
    store: &mut Store,
    nds: &std::path::Path,
    sha1: &str,
    fields: Option<&[String]>,
    what: &str,
) -> u128 {
    let started = Instant::now();
    let original = BaseRom::open_unchecked(nds, sha1).unwrap();
    let result = reset(
        store,
        &original,
        &ResetTarget::Header { id: HEADER },
        fields,
    )
    .unwrap();
    let ms = started.elapsed().as_millis();
    println!(
        "reset {} header ms={ms} (kernel {} ms, {what})",
        store.project().game_code,
        result.ms
    );
    assert!(result.recorded, "{what}");
    ms
}

#[test]
fn header_reset_field_by_field_gives_the_base_record_back() {
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, copy)) = common::vanilla_copy() else {
        return;
    };
    let base = BaseRom::open(&nds, None).unwrap();
    let base_header = base.header_table().unwrap()[usize::from(HEADER)].clone();
    let base_record = write_map_header_to_bytes(&base_header);
    let project = Arc::new(Project::open(&copy).unwrap());
    let mut store = Store::new(Arc::clone(&project));
    let target = ResetTarget::Header { id: HEADER };
    let mut times = Vec::new();

    edit_header(&mut store, HEADER);
    store.save().unwrap();
    let plan = plan_reset(&mut store, &base, &target).unwrap();
    let names: Vec<&str> = plan.fields.iter().map(|d| d.field.as_str()).collect();
    assert_eq!(names, ["music_day_id", "weather_id"]);

    let weather = ["weather_id".to_owned()];
    times.push(timed_reset(
        &mut store,
        &nds,
        base.sha1(),
        Some(&weather),
        "weather_id",
    ));
    store.save().unwrap();
    let saved = Project::open(&copy).unwrap().read_headers().unwrap()[usize::from(HEADER)].clone();
    assert_eq!(differing(&saved, &base_header), ["music_day_id"]);
    let half = store.header_table().unwrap()[usize::from(HEADER)].clone();

    times.push(timed_reset(&mut store, &nds, base.sha1(), None, "Rest"));
    store.save().unwrap();
    assert_eq!(saved_record(&project, HEADER), base_record);
    assert!(plan_reset(&mut store, &base, &target).unwrap().same);
    store.undo().unwrap();
    assert_eq!(store.header_table().unwrap()[usize::from(HEADER)], half);

    // With the Dynamic Headers patch the base stays in the ARM9, the project's header in its
    // file.
    let Some((_tmp2, copy2)) = common::vanilla_copy() else {
        return;
    };
    let project2 = Arc::new(Project::open(&copy2).unwrap());
    let mut store2 = Store::new(Arc::clone(&project2));
    if store2.dynamic_headers_applied().unwrap() {
        println!(
            "skipped dynamic headers: {} has the patch already",
            project2.game_code
        );
    } else if dynamic_headers::plan(&project2).unwrap().is_none() {
        println!(
            "skipped dynamic headers: {} has no dynamic headers patch",
            project2.game_code
        );
    } else {
        store2.apply_dynamic_headers().unwrap();
        edit_header(&mut store2, HEADER);
        store2.save().unwrap();
        let file = dynamic_headers::unpacked_dir(&project2).join(format!("{HEADER:04}"));
        assert_ne!(std::fs::read(&file).unwrap(), base_record);
        times.push(timed_reset(
            &mut store2,
            &nds,
            base.sha1(),
            None,
            "Dynamic Headers",
        ));
        store2.save().unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), base_record);
    }
    assert!(
        times.iter().all(|&ms| ms <= RESET_MS),
        "over {RESET_MS} ms: {times:?}"
    );
}
