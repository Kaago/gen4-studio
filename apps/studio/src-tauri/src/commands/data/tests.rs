//! The data commands on the synthetic Platinum (CPUE) fixture: 508 personal members with
//! names for 1 to 3 and forms 496 to 507, four moves, 20 item members (member 17 has 36
//! bytes), two encounter files and two trainers; the arm9 tables of CPUE are unverified, so
//! `item_table` is null.

#![allow(clippy::unwrap_used)]

use g4_kernel::project::{AssetId, AssetKind};
use serde_json::json;

use super::*;
use crate::commands::{history_impl, open_synthetic, undo_impl};

fn id(kind: AssetKind, index: u16) -> AssetId {
    AssetId { kind, index }
}

fn personal(field: &str, value: u32) -> DataPatch {
    DataPatch::Personal {
        field: field.to_owned(),
        value,
    }
}

fn last_label(state: &AppState) -> String {
    history_impl(state).unwrap().undo.last().unwrap().clone()
}

fn species(
    state: &AppState,
    index: u16,
) -> (PersonalDto, PersonalDto, Vec<LearnEntryDto>, Vec<bool>) {
    match data_get_impl(state, DataKind::Species, index).unwrap() {
        DataDocDto::Species {
            personal,
            saved,
            learnset,
            tm,
            ..
        } => (personal, saved, learnset, tm),
        other => panic!("{other:?}"),
    }
}

#[test]
fn data_list_species_names_forms() {
    let (_fixture, state) = open_synthetic();
    let rows = data_list_impl(&state, DataKind::Species, None).unwrap();
    assert_eq!(rows.len(), 508);
    assert_eq!(
        (rows[1].name.as_str(), rows[1].tech.as_str()),
        ("Bulbasaur", "0001")
    );
    // synthetic_personal_bytes(1): stats 2..=7, types 1 and 2.
    assert_eq!(
        rows[1].extra,
        json!({ "types": [1, 2], "number": 1, "stats": [2, 3, 4, 5, 6, 7] })
    );
    assert_eq!(rows[496].name, "Deoxys, Angriffsform");
    assert_eq!(rows[503].name, "Hitze-Rotom");
    // No name in the bank: the kernel's fallback, which the UI words in German.
    assert_eq!(rows[4].name, "species 0004");
    assert!(rows.iter().all(|r| r.locked.is_none()));

    let filter = |query: Option<&str>, types: Vec<u8>| {
        Some(DataFilter {
            query: query.map(str::to_owned),
            types,
            ..DataFilter::default()
        })
    };
    let names = |rows: Vec<DataRowDto>| rows.into_iter().map(|r| r.name).collect::<Vec<_>>();
    assert_eq!(
        names(data_list_impl(&state, DataKind::Species, filter(Some("BULB"), vec![])).unwrap()),
        ["Bulbasaur"]
    );
    assert_eq!(
        names(data_list_impl(&state, DataKind::Species, filter(Some("497"), vec![])).unwrap()),
        ["Deoxys, Verteidigungsform"]
    );
    // Types 1 or 2: members whose type 1 or type 2 is one of them (index % 17 and + 1).
    let typed = data_list_impl(&state, DataKind::Species, filter(None, vec![1])).unwrap();
    assert!(typed.iter().any(|r| r.index == 0) && typed.iter().any(|r| r.index == 1));
    assert!(typed.iter().all(|r| r.index % 17 <= 1));
}

#[test]
fn data_edit_personal_records_a_speaking_label() {
    let (_fixture, state) = open_synthetic();
    let changed =
        data_edit_impl(&state, DataKind::Species, 1, personal("attack", 85), None).unwrap();
    assert_eq!(changed, [id(AssetKind::Personal, 1)]);
    assert_eq!(last_label(&state), "Bulbasaur: Angriff 3 → 85");
    let (now, saved, _, _) = species(&state, 1);
    assert_eq!(
        (now.attack, saved.attack),
        (85, 3),
        "saved is the last saved state"
    );

    // A form, a bit field and a member without a name.
    data_edit_impl(
        &state,
        DataKind::Species,
        496,
        personal("ev_yield_speed", 3),
        None,
    )
    .unwrap();
    assert_eq!(
        last_label(&state),
        "Deoxys, Angriffsform: EV-Ertrag Initiative 0 → 3"
    );
    data_edit_impl(&state, DataKind::Species, 4, personal("speed", 1), None).unwrap();
    assert_eq!(last_label(&state), "Art 0004: Initiative 8 → 1");
    data_edit_impl(
        &state,
        DataKind::Species,
        1,
        DataPatch::Tm { n: 39, on: true },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Bulbasaur: TM40 aus → an");
    // The caller's own label wins.
    data_edit_impl(
        &state,
        DataKind::Species,
        1,
        personal("hp", 9),
        Some("eigener Text".into()),
    )
    .unwrap();
    assert_eq!(last_label(&state), "eigener Text");
    // The same value again changes nothing and records nothing.
    let steps = history_impl(&state).unwrap().undo.len();
    assert!(
        data_edit_impl(&state, DataKind::Species, 1, personal("hp", 9), None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps);

    for (patch, kind) in [
        (personal("attack", 256), DataKind::Species),
        (personal("ev_yield_hp", 4), DataKind::Species),
        (personal("tm_bits", 1), DataKind::Species),
        (personal("pad_1a", 1), DataKind::Species),
        (DataPatch::Tm { n: 100, on: true }, DataKind::Species),
        (personal("attack", 1), DataKind::Move),
    ] {
        let err = data_edit_impl(&state, kind, 1, patch, None).unwrap_err();
        assert_eq!(err.code, "error", "{}", err.message);
    }
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps);
}

#[test]
fn copy_moves_is_one_history_step() {
    let (_fixture, state) = open_synthetic();
    let three = vec![
        LearnEntryDto {
            move_id: 1,
            level: 1,
        },
        LearnEntryDto {
            move_id: 3,
            level: 7,
        },
        LearnEntryDto {
            move_id: 2,
            level: 9,
        },
    ];
    data_edit_impl(
        &state,
        DataKind::Learnset,
        1,
        DataPatch::Learnset {
            entries: three.clone(),
        },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Bulbasaur: Lernset 2 → 3 Attacken");
    let (_, _, before_learnset, before_tm) = species(&state, 2);

    let changed = data_edit_impl(
        &state,
        DataKind::Species,
        2,
        DataPatch::CopyMoves { from: 1 },
        None,
    )
    .unwrap();
    assert_eq!(
        changed,
        [id(AssetKind::Personal, 2), id(AssetKind::Learnset, 2)]
    );
    let history = history_impl(&state).unwrap();
    assert_eq!(history.undo.len(), 2);
    assert_eq!(history.undo[1], "Ivysaur: TM/VM und Lernset wie Bulbasaur");
    let (_, _, learnset, tm) = species(&state, 2);
    let (_, _, _, source_tm) = species(&state, 1);
    assert_eq!((learnset, &tm), (three, &source_tm));

    // One undo restores both parts.
    assert_eq!(
        undo_impl(&state).unwrap(),
        [id(AssetKind::Personal, 2), id(AssetKind::Learnset, 2)]
    );
    let (_, _, learnset, tm) = species(&state, 2);
    assert_eq!((learnset, tm), (before_learnset, before_tm));
}

#[test]
fn data_get_locked_member_reports_reason_and_raw_bytes() {
    let (_fixture, state) = open_synthetic();
    let reason = "item 0017: 36 bytes, expected 34";
    match data_get_impl(&state, DataKind::Item, 17).unwrap() {
        DataDocDto::Item {
            fields,
            locked,
            raw_locked,
            shared_with,
            description,
        } => {
            assert!(fields.is_empty());
            assert_eq!(locked.as_deref(), Some(reason));
            let raw = raw_locked.unwrap();
            assert_eq!(raw.len(), 36);
            assert_eq!(&raw[..2], &1700u16.to_le_bytes());
            assert!(shared_with.is_empty() && description.is_none());
        }
        other => panic!("{other:?}"),
    }
    let rows = data_list_impl(&state, DataKind::Item, None).unwrap();
    assert_eq!(rows[17].locked.as_deref(), Some(reason));
    assert_eq!(rows[17].extra, serde_json::Value::Null);
    let raw = data_raw_impl(&state, DataKind::Item, 17).unwrap();
    assert_eq!((raw.bytes.len(), raw.fields.len(), raw.member), (36, 0, 17));
    assert_eq!(raw.path, "files/itemtool/itemdata/pl_item_data.narc");

    let err = data_edit_impl(
        &state,
        DataKind::Item,
        17,
        DataPatch::Item {
            field: "price".into(),
            value: 1,
        },
        None,
    )
    .unwrap_err();
    assert_eq!(
        (err.code, err.asset),
        ("read_only", Some(id(AssetKind::Item, 17)))
    );

    // A sound neighbour reads and edits; without the item table its name is its member.
    match data_get_impl(&state, DataKind::Item, 16).unwrap() {
        DataDocDto::Item { fields, locked, .. } => {
            assert!(locked.is_none());
            assert_eq!(fields[0].name, "price");
            assert_eq!((fields[0].value, fields[0].bits), (1600, 16));
        }
        other => panic!("{other:?}"),
    }
    data_edit_impl(
        &state,
        DataKind::Item,
        16,
        DataPatch::Item {
            field: "price".into(),
            value: 250,
        },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Daten 0016: Preis 1600 → 250");
}

#[test]
fn items_are_keyed_by_data_member_for_an_unverified_game() {
    let (_fixture, state) = open_synthetic();
    assert_eq!(item_table_impl(&state).unwrap(), None);
    assert_eq!(machine_table_impl(&state).unwrap(), None);
    // Without the table, items are listed by data member, without names.
    let rows = data_list_impl(&state, DataKind::Item, None).unwrap();
    assert_eq!(rows.len(), 20);
    assert!(rows.iter().all(|r| r.name.is_empty()));
    assert_eq!(
        rows[3].extra,
        json!({ "price": 300, "field_pocket": 0, "icon": false, "data_index": 3 })
    );
}

#[test]
fn get_item_icon_returns_empty_without_table() {
    let (_fixture, state) = open_synthetic();
    assert!(get_item_icon_impl(&state, 17).unwrap().is_empty());
    assert!(get_mon_icon_impl(&state, 1).unwrap().is_empty());
    assert!(get_item_icon_impl(&AppState::default(), 17).is_err());
}

#[test]
fn moves_encounters_and_trainers() {
    let (_fixture, state) = open_synthetic();
    let moves = data_list_impl(&state, DataKind::Move, None).unwrap();
    assert_eq!(moves.len(), 4);
    assert_eq!(moves[1].name, "Pound");
    assert_eq!(
        moves[1].extra,
        json!({ "type": 0, "category": 0, "power": 10, "accuracy": 100, "pp": 35 })
    );
    match data_get_impl(&state, DataKind::Move, 1).unwrap() {
        DataDocDto::Move { description, .. } => assert_eq!(
            description,
            Some(DescriptionDto {
                archive: 646,
                message: 1,
                text: "Pounds the foe.".into()
            })
        ),
        other => panic!("{other:?}"),
    }
    data_edit_impl(
        &state,
        DataKind::Move,
        1,
        DataPatch::Move {
            field: "power".into(),
            value: 65,
        },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Pound: Stärke 10 → 65");
    // Move 3 has no name in the bank.
    data_edit_impl(
        &state,
        DataKind::Move,
        3,
        DataPatch::Move {
            field: "priority".into(),
            value: -1,
        },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Attacke 0003: Priorität 0 → -1");

    let encounters = data_list_impl(&state, DataKind::Encounter, None).unwrap();
    assert_eq!(encounters.len(), 2);
    let users = encounters[0].extra["users"].as_array().unwrap();
    assert!(users.contains(&json!(0)) && users.contains(&json!(1)));
    let encounter =
        |method: &str, slot: Option<u16>, field: &str, value: i64| DataPatch::Encounter {
            method: method.into(),
            slot,
            field: field.into(),
            value,
        };
    data_edit_impl(
        &state,
        DataKind::Encounter,
        1,
        encounter("grass", Some(0), "species", 3),
        None,
    )
    .unwrap();
    assert_eq!(
        last_label(&state),
        "Begegnungen 0001: Gras Platz 1 Pokémon 1 → 3"
    );
    data_edit_impl(
        &state,
        DataKind::Encounter,
        1,
        encounter("grass", None, "rate", 25),
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Begegnungen 0001: Gras Rate 10 → 25");
    match data_get_impl(&state, DataKind::Encounter, 1).unwrap() {
        DataDocDto::Encounter {
            methods, extras, ..
        } => {
            assert_eq!((methods[0].rate, methods[0].slots[0].species), (25, 3));
            assert_eq!(extras[0].list, "swarm");
        }
        other => panic!("{other:?}"),
    }
    for bad in [
        encounter("grass", Some(12), "species", 3),
        encounter("rock_smash", Some(0), "species", 3),
        encounter("grass", Some(0), "level", 300),
    ] {
        let err = data_edit_impl(&state, DataKind::Encounter, 1, bad, None).unwrap_err();
        assert_eq!(err.code, "error", "{}", err.message);
    }

    let trainers = data_list_impl(&state, DataKind::Trainer, None).unwrap();
    assert_eq!(trainers.len(), 2);
    assert_eq!(trainers[1].name, "Kai");
    assert_eq!(trainers[1].extra, json!({ "class": 1, "party_count": 1 }));
    for err in [
        data_get_impl(&state, DataKind::Trainer, 1).unwrap_err(),
        data_edit_impl(&state, DataKind::Trainer, 1, personal("hp", 1), None).unwrap_err(),
    ] {
        assert_eq!((err.code, err.message.as_str()), ("error", TRAINER_LATER));
    }
}

#[test]
fn raw_bytes_name_their_fields_and_archive() {
    let (_fixture, state) = open_synthetic();
    let raw = data_raw_impl(&state, DataKind::Species, 1).unwrap();
    assert_eq!((raw.bytes.len(), raw.member), (44, 1));
    assert_eq!(raw.path, "files/poketool/personal/pl_personal.narc");
    let attack = raw.fields.iter().find(|f| f.name == "attack").unwrap();
    assert_eq!((attack.offset, attack.kind), (1, "u8"));
    let learnset = data_raw_impl(&state, DataKind::Learnset, 1).unwrap();
    let names: Vec<_> = learnset.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["entry_00", "entry_01", "end", "pad_06"]);
    let encounter = data_raw_impl(&state, DataKind::Encounter, 0).unwrap();
    assert_eq!(
        (encounter.bytes.len(), encounter.fields[0].name.as_str()),
        (424, "grass_rate")
    );
}

#[test]
fn name_tables_follow_the_members() {
    let (_fixture, state) = open_synthetic();
    let tables = name_tables_impl(&state).unwrap();
    assert_eq!(tables.species.len(), 508);
    assert_eq!(tables.species[2], "Ivysaur");
    assert_eq!(tables.species[507], "Schneid-Rotom");
    assert_eq!(tables.moves, ["-", "Pound", "Karate Chop", "move 0003"]);
    assert_eq!(tables.items, ["None", "Master Ball", "Ultra Ball"]);
    assert_eq!(tables.abilities, ["-", "Stench", "Drizzle"]);
    assert_eq!(tables.types, ["Normal", "Fighting", "Flying"]);
    assert_eq!(tables.classes, ["Pokémon Trainer", "Youngster"]);
    assert_eq!(tables.forms, ["Normal Form", "Attack Form"]);
}

#[test]
fn refs_name_the_place_and_where_a_click_leads() {
    let (_fixture, state) = open_synthetic();
    let refs = refs_impl(&state, TargetDto::Species { index: 2 }).unwrap();
    let evolution = refs
        .iter()
        .find(|r| r.source == id(AssetKind::Evolution, 1))
        .unwrap();
    assert_eq!(evolution.label, "Bulbasaur, Entwicklung 1 Ziel");
    assert_eq!(
        serde_json::to_value(&evolution.nav).unwrap(),
        json!({ "kind": "species", "index": 1, "tab": "evolution" })
    );

    let refs = refs_impl(&state, TargetDto::Species { index: 1 }).unwrap();
    let wild = refs
        .iter()
        .find(|r| r.source == id(AssetKind::Encounter, 0))
        .unwrap();
    assert_eq!(wild.label, "Begegnungen 0000, Gras Platz 1");
    assert_eq!(
        serde_json::to_value(&wild.nav).unwrap(),
        json!({ "kind": "header", "id": 0 })
    );
    let party = refs
        .iter()
        .find(|r| r.source == id(AssetKind::TrainerParty, 1))
        .unwrap();
    assert_eq!(party.label, "Trainer 0001 Kai, Team Platz 1");
    assert_eq!(
        serde_json::to_value(&party.nav).unwrap(),
        json!({ "kind": "trainer", "index": 1 })
    );

    // An edit shows in the next answer: the index is refreshed after every data edit.
    data_edit_impl(
        &state,
        DataKind::Evolution,
        2,
        DataPatch::Evolution {
            slot: 0,
            method: 4,
            param: 32,
            target: 3,
        },
        None,
    )
    .unwrap();
    assert_eq!(last_label(&state), "Ivysaur: Entwicklung 1 geändert");
    let refs = refs_impl(&state, TargetDto::Species { index: 3 }).unwrap();
    assert!(
        refs.iter()
            .any(|r| r.label == "Ivysaur, Entwicklung 1 Ziel")
    );

    let learned = refs_impl(&state, TargetDto::Move { index: 2 }).unwrap();
    assert_eq!(learned.len(), 508, "every synthetic learnset knows move 2");
    assert_eq!(learned[1].label, "Bulbasaur, Lernset Platz 2");
}

#[test]
fn diamond_and_pearl_are_locked() {
    let (fixture, _) = open_synthetic();
    let header = fixture.dir.path().join("header.yaml");
    let yaml = std::fs::read_to_string(&header)
        .unwrap()
        .replace("CPUE", "ADAE");
    std::fs::write(&header, yaml).unwrap();
    // Long enough for the Diamond header table (zeros: empty headers).
    let arm9 = fixture.dir.path().join("arm9/arm9.bin");
    let mut bytes = std::fs::read(&arm9).unwrap();
    bytes.resize(0x10_0000, 0);
    std::fs::write(&arm9, bytes).unwrap();
    let state = AppState::default();
    crate::commands::open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
    for err in [
        data_list_impl(&state, DataKind::Species, None).unwrap_err(),
        data_get_impl(&state, DataKind::Move, 1).unwrap_err(),
        data_raw_impl(&state, DataKind::Item, 1).unwrap_err(),
        data_edit_impl(&state, DataKind::Species, 1, personal("hp", 1), None).unwrap_err(),
    ] {
        assert_eq!((err.code, err.message.as_str()), ("read_only", DP_LOCKED));
    }
}

#[test]
fn data_list_species_is_fast() {
    let (_fixture, state) = open_synthetic();
    data_list_impl(&state, DataKind::Species, None).unwrap();
    let start = std::time::Instant::now();
    for _ in 0..5 {
        data_list_impl(&state, DataKind::Species, None).unwrap();
    }
    // Loaded members and cached names: a second list is far below the 50 ms of a frame budget.
    assert!(
        start.elapsed() < std::time::Duration::from_millis(250),
        "{:?}",
        start.elapsed()
    );
}

/// Gated (`G4_TEST_PROJECT`): the commands on a copy of a vanilla project, with the verified
/// arm9 tables, real names and icons.
#[test]
fn vanilla_data_commands_on_copy() {
    let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
        return;
    };
    let state = AppState::default();
    crate::commands::open_project_impl(&state, root.to_str().unwrap()).unwrap();

    let start = std::time::Instant::now();
    let rows = data_list_impl(&state, DataKind::Species, None).unwrap();
    let first = start.elapsed();
    let start = std::time::Instant::now();
    data_list_impl(&state, DataKind::Species, None).unwrap();
    eprintln!(
        "data_list species: first {first:?}, again {:?}",
        start.elapsed()
    );
    assert_eq!(rows.len(), 508);
    assert!(rows.iter().all(|r| r.locked.is_none()));
    assert!(!rows[397].name.starts_with("species"), "{}", rows[397].name);
    assert_eq!(rows[496].name, "Deoxys, Angriffsform");

    let table = item_table_impl(&state)
        .unwrap()
        .expect("verified item table");
    assert!(table.len() == 468 || table.len() == 537, "{}", table.len());
    assert_eq!(machine_table_impl(&state).unwrap().unwrap().len(), 100);
    let items = data_list_impl(&state, DataKind::Item, None).unwrap();
    assert_eq!(items.len(), table.len());
    assert!(!items[17].name.is_empty());
    let png = get_item_icon_impl(&state, 17).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let png = get_mon_icon_impl(&state, 397).unwrap();
    assert_eq!(&png[1..4], b"PNG");

    let (before, ..) = species(&state, 397);
    let to = u32::from(before.attack) + 10;
    data_edit_impl(&state, DataKind::Species, 397, personal("attack", to), None).unwrap();
    assert_eq!(
        last_label(&state),
        format!("{}: Angriff {} → {to}", rows[397].name, before.attack)
    );

    let start = std::time::Instant::now();
    let refs = refs_impl(&state, TargetDto::Species { index: 397 }).unwrap();
    eprintln!("refs species 397: {} in {:?}", refs.len(), start.elapsed());
    assert!(!refs.is_empty());
    for kind in [DataKind::Move, DataKind::Encounter, DataKind::Trainer] {
        assert!(!data_list_impl(&state, kind, None).unwrap().is_empty());
    }
    assert!(matches!(
        data_get_impl(&state, DataKind::Item, 17).unwrap(),
        DataDocDto::Item {
            description: Some(_),
            locked: None,
            ..
        }
    ));
}

/// The fixture as Platinum DE (CPUD, verified arm9 offsets) with an item table whose ids
/// point at data member `id % 20`, TM01 teaching `tm01` and HM05 Defog.
fn verified_platinum(tm01: u16) -> (g4_kernel::test_support::Synthetic, AppState) {
    let (fixture, _) = open_synthetic();
    let root = fixture.dir.path();
    let header = root.join("header.yaml");
    let yaml = std::fs::read_to_string(&header)
        .unwrap()
        .replace("CPUE", "CPUD");
    std::fs::write(&header, yaml).unwrap();
    let path = root.join("arm9/arm9.bin");
    let mut arm9 = std::fs::read(&path).unwrap();
    arm9.resize(0xF2000, 0);
    let (machines, items) = (0xF0C54, 0xF0D1C);
    arm9[machines..machines + 2].copy_from_slice(&tm01.to_le_bytes());
    arm9[machines + 2 * 96..machines + 2 * 97].copy_from_slice(&432u16.to_le_bytes());
    for id in 0..468u16 {
        let at = items + 8 * usize::from(id);
        arm9[at..at + 2].copy_from_slice(&(id % 20).to_le_bytes());
    }
    std::fs::write(&path, arm9).unwrap();
    let state = AppState::default();
    crate::commands::open_project_impl(&state, root.to_str().unwrap()).unwrap();
    (fixture, state)
}

#[test]
fn item_table_is_null_when_implausible() {
    // TM01 must be Focus Punch (264): a patched arm9 that moved the table reads as missing.
    let (_fixture, state) = verified_platinum(0);
    assert_eq!(item_table_impl(&state).unwrap(), None);
    assert_eq!(machine_table_impl(&state).unwrap(), None);
    let rows = data_list_impl(&state, DataKind::Item, None).unwrap();
    assert_eq!(rows.len(), 20, "by data member");
    assert!(rows.iter().all(|r| r.name.is_empty()));
    assert!(get_item_icon_impl(&state, 1).unwrap().is_empty());
}

#[test]
fn items_are_keyed_by_item_id_with_a_plausible_table() {
    let (_fixture, state) = verified_platinum(264);
    let table = item_table_impl(&state).unwrap().unwrap();
    assert_eq!(table.len(), 468);
    assert_eq!(table[23].data, 3);
    assert_eq!(machine_table_impl(&state).unwrap().unwrap()[0], 264);
    let rows = data_list_impl(&state, DataKind::Item, None).unwrap();
    assert_eq!(rows.len(), 468);
    assert_eq!(rows[1].name, "Master Ball");
    assert_eq!(rows[23].extra["data_index"], 3);
    assert_eq!(
        rows[37].locked.as_deref(),
        Some("item 0017: 36 bytes, expected 34")
    );
    match data_get_impl(&state, DataKind::Item, 2).unwrap() {
        DataDocDto::Item {
            shared_with,
            description,
            ..
        } => {
            assert_eq!(&shared_with[..3], &[22, 42, 62]);
            assert_eq!(description.unwrap().text, "A good Ball.");
        }
        other => panic!("{other:?}"),
    }
    let changed = data_edit_impl(
        &state,
        DataKind::Item,
        2,
        DataPatch::Item {
            field: "price".into(),
            value: 250,
        },
        None,
    )
    .unwrap();
    assert_eq!(changed, [id(AssetKind::Item, 2)], "the data member");
    // Ids 2, 22, ..., 462 share member 2.
    let shares = (2..468).step_by(20).count();
    assert_eq!(
        last_label(&state),
        format!("Ultra Ball: Preis 200 → 250, gilt für {shares} Gegenstände")
    );
}

#[test]
fn learnsets_stay_within_what_the_game_reads() {
    let (_fixture, state) = open_synthetic();
    let entry = |move_id, level| LearnEntryDto { move_id, level };
    let learnset = |entries| DataPatch::Learnset { entries };
    for (patch, message) in [
        (
            learnset(vec![entry(1, 5); 21]),
            "learnset of 21 entries, the game reads at most 20",
        ),
        // The fixture has moves 0 to 3; move 511 at level 127 would be the terminator.
        (
            learnset(vec![entry(511, 127)]),
            "move 511 at level 127 does not fit a learnset entry (moves 0 to 3, levels up to 127)",
        ),
        (
            learnset(vec![entry(1, 128)]),
            "move 1 at level 128 does not fit a learnset entry (moves 0 to 3, levels up to 127)",
        ),
    ] {
        let err = data_edit_impl(&state, DataKind::Learnset, 1, patch, None).unwrap_err();
        assert_eq!((err.code, err.message.as_str()), ("error", message));
    }
    assert!(history_impl(&state).unwrap().undo.is_empty());
    data_edit_impl(
        &state,
        DataKind::Learnset,
        1,
        learnset(vec![entry(3, 7); 20]),
        None,
    )
    .unwrap();
}

#[test]
fn a_locked_species_part_names_its_asset() {
    let (fixture, state) = open_synthetic();
    let project = g4_kernel::project::Project::open(fixture.dir.path()).unwrap();
    project.ensure_unpacked(AssetKind::Personal).unwrap();
    std::fs::write(project.member_path(AssetKind::Personal, 5), [7u8; 40]).unwrap();
    match data_get_impl(&state, DataKind::Species, 5).unwrap() {
        DataDocDto::Species {
            tm,
            locked,
            raw_locked,
            locked_asset,
            learnset,
            ..
        } => {
            assert_eq!(
                locked.as_deref(),
                Some("personal data 0005: 40 bytes, expected 44")
            );
            assert_eq!(raw_locked, Some(vec![7; 40]));
            assert_eq!(locked_asset, Some(id(AssetKind::Personal, 5)));
            assert_eq!(tm, vec![false; 100]);
            assert_eq!(learnset.len(), 2, "the sound parts still read");
        }
        other => panic!("{other:?}"),
    }
    match data_get_impl(&state, DataKind::Species, 6).unwrap() {
        DataDocDto::Species { locked_asset, .. } => assert_eq!(locked_asset, None),
        other => panic!("{other:?}"),
    }
}
