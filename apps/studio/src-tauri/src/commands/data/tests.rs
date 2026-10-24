//! The data commands on the synthetic Platinum (CPUE) fixture: 508 personal members with
//! names for 1 to 3 and forms 496 to 507, four moves, 20 item members (member 17 has 36
//! bytes), two encounter files and three trainers; the arm9 tables of CPUE are unverified, so
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
    // A number finds the member of that number and the four-digit numbers that start with it,
    // like the list's own search (`matchesQuery`): "050" finds 0050 and 0500 to 0507, "50"
    // finds 0050 but none of the forms.
    let indices = |rows: Vec<DataRowDto>| rows.into_iter().map(|r| r.index).collect::<Vec<_>>();
    let found =
        indices(data_list_impl(&state, DataKind::Species, filter(Some("050"), vec![])).unwrap());
    assert!(found.contains(&50));
    assert!((500..=507).all(|i| found.contains(&i)));
    assert!(!found.iter().any(|i| (496..500).contains(i)));
    let found =
        indices(data_list_impl(&state, DataKind::Species, filter(Some("50"), vec![])).unwrap());
    assert!(found.contains(&50));
    assert!(!found.iter().any(|i| (496..=507).contains(i)), "{found:?}");
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
            name_archive,
        } => {
            assert!(fields.is_empty());
            // Without the item table the member has no item id, so no name either.
            assert_eq!(name_archive, None);
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
        DataDocDto::Move {
            description,
            name_archive,
            ..
        } => {
            assert_eq!(
                description,
                Some(DescriptionDto {
                    archive: 646,
                    message: 1,
                    text: "Pounds the foe.".into()
                })
            );
            assert_eq!(name_archive, Some(647));
        }
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
            methods,
            replacements,
            ..
        } => {
            assert_eq!((methods[0].rate, methods[0].slots[0].species), (25, 3));
            assert_eq!(replacements[0].list, "day");
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
    assert_eq!(trainers.len(), 3);
    assert_eq!(
        trainers[2].extra,
        json!({ "class": 0, "class_name": "Pokémon Trainer", "level_max": 15, "party_count": 3, "places": [] })
    );
    assert_eq!(trainers[1].name, "Kai");
    assert_eq!(
        trainers[1].extra,
        json!({ "class": 1, "class_name": "Youngster", "level_max": 5, "party_count": 1, "places": [] })
    );
    // A species patch does not apply to a trainer.
    let err = data_edit_impl(&state, DataKind::Trainer, 1, personal("hp", 1), None).unwrap_err();
    assert_eq!(err.code, "error", "{}", err.message);
}

/// The trainer document of `index`.
fn trainer(state: &AppState, index: u16) -> DataDocDto {
    let doc = data_get_impl(state, DataKind::Trainer, index).unwrap();
    assert!(matches!(doc, DataDocDto::Trainer { .. }), "{doc:?}");
    doc
}

/// The party of trainer `index`.
fn party(state: &AppState, index: u16) -> Vec<PartyMonDto> {
    match trainer(state, index) {
        DataDocDto::Trainer { party, .. } => party,
        _ => unreachable!(),
    }
}

/// Points the object event 0 of event file `file` at trainer `trainer` (script 3000 + id - 1)
/// and header 0 at that event file.
fn place_trainer(state: &AppState, file: u16, trainer: u16) {
    use crate::commands::{
        EventKind, add_event_impl, get_header_impl, set_header_impl, update_event_impl,
    };
    add_event_impl(state, file, EventKind::Object, 3, 4, None, None).unwrap();
    let patch = serde_json::from_value(json!({ "script": 3000 + trainer - 1 })).unwrap();
    update_event_impl(state, file, EventKind::Object, 0, patch).unwrap();
    let g4_kernel::store::MapHeader::Pt(mut header) = get_header_impl(state, 0).unwrap() else {
        panic!("a Platinum fixture");
    };
    header.event_file_id = file;
    set_header_impl(
        state,
        0,
        g4_kernel::store::MapHeader::Pt(header),
        "Eventdatei",
    )
    .unwrap();
}

#[test]
fn data_get_trainer_lists_party_texts_and_events() {
    let (_fixture, state) = open_synthetic();
    place_trainer(&state, 1, 1);
    match trainer(&state, 1) {
        DataDocDto::Trainer {
            class,
            name,
            format,
            party,
            texts,
            texts_locked,
            events,
            name_archive,
            text_archive,
            problems,
            placeholder,
            locked,
            ..
        } => {
            assert_eq!((class, name.as_str()), (1, "Kai"));
            assert_eq!(
                format,
                PartyFormatDto {
                    moves: false,
                    items: false
                }
            );
            assert_eq!(party.len(), 1);
            assert_eq!(
                (
                    party[0].species,
                    party[0].level,
                    party[0].moves,
                    party[0].item
                ),
                (1, 5, None, None)
            );
            // Platinum: no gender or ability of its own.
            assert_eq!((party[0].gender, party[0].ability), (None, None));
            assert_eq!(texts_locked, None);
            let texts: Vec<_> = texts
                .iter()
                .map(|t| (t.kind, t.archive, t.message, t.text.as_str()))
                .collect();
            assert_eq!(
                texts,
                [(0, 617, 0, "Let’s battle!"), (2, 617, 1, "I lost!")]
            );
            assert_eq!((name_archive, text_archive), (Some(618), Some(617)));
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].source, id(AssetKind::Event, 1));
            assert_eq!(
                serde_json::to_value(&events[0].nav).unwrap(),
                json!({ "kind": "event", "eventFile": 1, "eventKind": "object", "index": 0 })
            );
            // The text before the battle does not end with \r: a Problems row.
            assert_eq!(problems.len(), 1);
            assert_eq!(
                (
                    problems[0].source,
                    problems[0].file.as_str(),
                    problems[0].severity
                ),
                ("trainer", "trainers/0001", "warning")
            );
            assert!(!placeholder);
            assert_eq!(locked, None);
        }
        _ => unreachable!(),
    }
    // Trainer 2: moves and items; Platinum keeps the ball capsule.
    let two = party(&state, 2);
    assert_eq!(two.len(), 3);
    assert_eq!(
        (two[0].item, two[0].moves, two[0].ball_capsule),
        (Some(25), Some([20, 21, 0, 0]), Some(0))
    );
    // The list names the place of the event: header 0, "Twinleaf Town".
    let rows = data_list_impl(&state, DataKind::Trainer, None).unwrap();
    assert_eq!(
        rows[1].extra["places"],
        json!([{ "header": 0, "name": "Twinleaf Town" }])
    );
    assert_eq!(rows[2].extra["places"], json!([]));

    // Fields, items and the party, each one step with a speaking label.
    let steps = history_impl(&state).unwrap().undo.len();
    let field = |field: &str, value: i64| DataPatch::Trainer {
        field: field.into(),
        value,
    };
    data_edit_impl(&state, DataKind::Trainer, 1, field("ai", 7), None).unwrap();
    assert_eq!(last_label(&state), "Youngster Kai: KI 0 → 7");
    data_edit_impl(&state, DataKind::Trainer, 1, field("item2", 28), None).unwrap();
    assert_eq!(last_label(&state), "Youngster Kai: Gegenstand 3 0 → 28");
    let mut mons = party(&state, 1);
    mons[0].level = 14;
    let changed = data_edit_impl(
        &state,
        DataKind::Trainer,
        1,
        DataPatch::TrainerParty { party: mons },
        Some("Youngster Kai: Platz 1 Level 5 → 14".into()),
    )
    .unwrap();
    assert_eq!(changed, [id(AssetKind::Trainer, 1)]);
    assert_eq!(last_label(&state), "Youngster Kai: Platz 1 Level 5 → 14");
    assert_eq!(party(&state, 1)[0].level, 14);
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps + 3);
    for bad in [field("class", 256), field("item4", 1), field("name", 1)] {
        let err = data_edit_impl(&state, DataKind::Trainer, 1, bad, None).unwrap_err();
        assert_eq!(err.code, "error", "{}", err.message);
    }
    let seven = vec![PartyMonDto::default(); 7];
    let err = data_edit_impl(
        &state,
        DataKind::Trainer,
        1,
        DataPatch::TrainerParty { party: seven },
        None,
    )
    .unwrap_err();
    assert!(err.message.contains("at most 6"), "{}", err.message);
    // The reference index sees the edit at once.
    let refs = refs_impl(&state, TargetDto::Item { id: 28 }).unwrap();
    assert!(
        refs.iter()
            .any(|r| r.source == id(AssetKind::TrainerProperties, 1))
    );

    // Battle texts through data_edit: one step, with the caller's label.
    let changed = data_edit_impl(
        &state,
        DataKind::Trainer,
        1,
        DataPatch::TrainerTexts {
            texts: vec![TrainerTextInDto {
                kind: 0,
                text: "Los geht's!\\r".into(),
            }],
        },
        Some("Youngster Kai: Kampftext vor dem Kampf".into()),
    )
    .unwrap();
    assert_eq!(changed, [id(AssetKind::TextArchive, 617)]);
    assert_eq!(last_label(&state), "Youngster Kai: Kampftext vor dem Kampf");
    match trainer(&state, 1) {
        DataDocDto::Trainer {
            texts, problems, ..
        } => {
            assert_eq!(texts[0].text, "Los geht's!\\r");
            assert!(problems.is_empty(), "{problems:?}");
        }
        _ => unreachable!(),
    }
    let raw = data_raw_impl(&state, DataKind::Trainer, 1).unwrap();
    assert_eq!(raw.path, "files/poketool/trainer/trdata.narc");
    assert_eq!(
        (raw.fields[0].name.as_str(), raw.fields[1].offset),
        ("props_len", 4)
    );
    assert_eq!(raw.fields.last().unwrap().name, "party");
}

#[test]
fn trainer_format_on_fills_default_moves_in_one_step() {
    let (_fixture, state) = open_synthetic();
    let format =
        |moves: Option<bool>, items: Option<bool>| DataPatch::TrainerFormat { moves, items };
    let steps = history_impl(&state).unwrap().undo.len();
    // Trainer 1: species 1 at level 5; its learnset is Pound at 1 and Karate Chop at 5.
    let changed =
        data_edit_impl(&state, DataKind::Trainer, 1, format(Some(true), None), None).unwrap();
    assert_eq!(changed, [id(AssetKind::Trainer, 1)]);
    assert_eq!(last_label(&state), "Youngster Kai: eigene Attacken an");
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps + 1);
    assert_eq!(party(&state, 1)[0].moves, Some([1, 2, 0, 0]));
    match trainer(&state, 1) {
        DataDocDto::Trainer { format, .. } => assert!(format.moves && !format.items),
        _ => unreachable!(),
    }
    // Already on: nothing to do, nothing recorded.
    assert!(
        data_edit_impl(&state, DataKind::Trainer, 1, format(Some(true), None), None)
            .unwrap()
            .is_empty()
    );
    // Undo takes the moves away again.
    undo_impl(&state).unwrap();
    assert_eq!(party(&state, 1)[0].moves, None);
    // Both at once: one step; off drops them.
    data_edit_impl(
        &state,
        DataKind::Trainer,
        1,
        format(Some(true), Some(true)),
        None,
    )
    .unwrap();
    assert_eq!(
        last_label(&state),
        "Youngster Kai: eigene Attacken an, Items tragen an"
    );
    assert_eq!(party(&state, 1)[0].item, Some(0));
    data_edit_impl(
        &state,
        DataKind::Trainer,
        1,
        format(Some(false), None),
        Some("eigener Text".into()),
    )
    .unwrap();
    assert_eq!(last_label(&state), "eigener Text");
    assert_eq!(party(&state, 1)[0].moves, None);
    assert_eq!(party(&state, 1)[0].item, Some(0));
}

/// The trainer `index` as the store holds it must encode to bytes that decode to the same
/// trainer and encode again unchanged (`encode(decode(p, q)) == (p, q)`).
fn assert_trainer_round_trips(state: &AppState, index: u16) {
    use g4_formats::data::trainer_doc::{Checked, TrainerDoc};
    let raw = data_raw_impl(state, DataKind::Trainer, index).unwrap();
    let len = u32::from_le_bytes(raw.bytes[..4].try_into().unwrap()) as usize;
    let (props, party) = raw.bytes[4..].split_at(len);
    match TrainerDoc::check(props, party, g4_formats::game::Family::Pt) {
        Checked::Exact(_) => {}
        other => panic!("trainer {index}: {other:?}"),
    }
}

#[test]
fn six_pokemon_moves_off_undo_and_on_follow_the_game_rule() {
    let (_fixture, state) = open_synthetic();
    let format = |moves: Option<bool>| DataPatch::TrainerFormat { moves, items: None };
    // Trainer 2 has moves and items; give it six Pokémon with moves of their own.
    let custom = |i: u16| [20 + i, 21, 22, 3];
    let six: Vec<PartyMonDto> = (0..6u16)
        .map(|i| PartyMonDto {
            iv_or_difficulty: 10 * i as u8,
            level: [3, 5, 9, 12, 40, 100][usize::from(i)],
            species: 1 + i,
            item: Some(25),
            moves: Some(custom(i)),
            ball_capsule: Some(0),
            ..PartyMonDto::default()
        })
        .collect();
    data_edit_impl(
        &state,
        DataKind::Trainer,
        2,
        DataPatch::TrainerParty { party: six },
        None,
    )
    .unwrap();
    assert_eq!(
        last_label(&state),
        "Pokémon Trainer Mia: Team 3 → 6 Pokémon"
    );
    assert_trainer_round_trips(&state, 2);
    let moves = |state: &AppState| party(state, 2).iter().map(|m| m.moves).collect::<Vec<_>>();
    let mine: Vec<_> = (0..6).map(|i| Some(custom(i))).collect();
    assert_eq!(moves(&state), mine);

    // Off drops the moves of all six in one step; undo brings the custom moves back.
    let steps = history_impl(&state).unwrap().undo.len();
    data_edit_impl(&state, DataKind::Trainer, 2, format(Some(false)), None).unwrap();
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps + 1);
    assert_eq!(moves(&state), vec![None; 6]);
    assert_trainer_round_trips(&state, 2);
    undo_impl(&state).unwrap();
    assert_eq!(moves(&state), mine);
    assert_trainer_round_trips(&state, 2);

    // Off and on again: the moves the game gives (Pound at 1, Karate Chop at 5 in every
    // synthetic learnset), not the custom ones.
    data_edit_impl(&state, DataKind::Trainer, 2, format(Some(false)), None).unwrap();
    data_edit_impl(&state, DataKind::Trainer, 2, format(Some(true)), None).unwrap();
    let rule = [
        Some([1, 0, 0, 0]),
        Some([1, 2, 0, 0]),
        Some([1, 2, 0, 0]),
        Some([1, 2, 0, 0]),
        Some([1, 2, 0, 0]),
        Some([1, 2, 0, 0]),
    ];
    assert_eq!(moves(&state), rule);
    assert_trainer_round_trips(&state, 2);
    // Items stayed; the levels and species too.
    let now = party(&state, 2);
    assert!(now.iter().all(|m| m.item == Some(25)));
    assert_eq!(
        now.iter().map(|m| m.species).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn trainer_name_over_seven_chars_is_refused() {
    let (_fixture, state) = open_synthetic();
    let name = |name: &str| DataPatch::TrainerName { name: name.into() };
    let steps = history_impl(&state).unwrap().undo.len();
    let err = data_edit_impl(&state, DataKind::Trainer, 1, name("Bartholo"), None).unwrap_err();
    assert_eq!(
        err.message,
        "Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch"
    );
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps);
    // Seven fit.
    let changed = data_edit_impl(&state, DataKind::Trainer, 1, name("Wolfram"), None).unwrap();
    assert_eq!(changed, [id(AssetKind::TextArchive, 618)]);
    assert_eq!(last_label(&state), "Youngster Kai: Name Kai → Wolfram");
    match trainer(&state, 1) {
        DataDocDto::Trainer { name, .. } => assert_eq!(name, "Wolfram"),
        _ => unreachable!(),
    }
    let rows = data_list_impl(&state, DataKind::Trainer, None).unwrap();
    assert_eq!(rows[1].name, "Wolfram");
}

/// The encounter document of `index`.
fn encounter_doc(state: &AppState, index: u16) -> DataDocDto {
    let doc = data_get_impl(state, DataKind::Encounter, index).unwrap();
    assert!(matches!(doc, DataDocDto::Encounter { .. }), "{doc:?}");
    doc
}

#[test]
fn data_get_encounter_carries_shares_and_warnings() {
    let (_fixture, state) = open_synthetic();
    // Encounter file 0: grass rate 10, slot 0 species 1 at level 2.
    match encounter_doc(&state, 0) {
        DataDocDto::Encounter {
            methods,
            replacements,
            extras,
            warnings,
            problems,
            users,
            effective,
            locked,
            ..
        } => {
            assert_eq!(locked, None);
            let grass = &methods[0];
            assert_eq!((grass.method, grass.rate), ("grass", 10));
            assert_eq!(grass.slots[0].share, 20);
            assert_eq!(grass.slots[11].share, 1);
            let by: Vec<_> = grass.slots[0]
                .replaced_by
                .iter()
                .map(|r| (r.list, r.entry))
                .collect();
            assert_eq!(by, [("swarm", 0)]);
            let lists: Vec<_> = replacements.iter().map(|l| l.list).collect();
            assert_eq!(
                lists,
                [
                    "day",
                    "night",
                    "swarm",
                    "radar",
                    "dual_ruby",
                    "dual_sapphire",
                    "dual_emerald",
                    "dual_firered",
                    "dual_leafgreen"
                ]
            );
            assert_eq!(replacements[2].label, "Schwarm");
            assert_eq!(extras.form_rates, Some([0; 5]));
            // The empty swarm entry 0 replaces an occupied slot: a warning on grass slot 0.
            let swarm: Vec<_> = warnings
                .iter()
                .filter(|w| w.text.starts_with("swarm entry 0"))
                .map(|w| (w.method, w.slot))
                .collect();
            assert_eq!(swarm, [("grass", Some(0))]);
            assert_eq!(problems.len(), warnings.len());
            assert!(
                problems
                    .iter()
                    .all(|p| p.source == "encounter" && p.file == "encounters/0000")
            );
            assert_eq!(problems[0].line, 1);
            assert_eq!(
                &users[..2],
                [
                    PlaceDto {
                        header: 0,
                        name: "Twinleaf Town".into()
                    },
                    PlaceDto {
                        header: 1,
                        name: "T02".into()
                    },
                ]
            );
            let times: Vec<_> = effective.iter().map(|t| t.when).collect();
            assert_eq!(times, ["morning", "day", "night"]);
            assert_eq!(
                effective[0].species,
                [SpeciesShareDto {
                    species: 1,
                    share: 20
                }]
            );
        }
        _ => unreachable!(),
    }
    // A replacement list and an extra through data_edit.
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
        0,
        encounter("swarm", Some(0), "species", 2),
        None,
    )
    .unwrap();
    assert_eq!(
        last_label(&state),
        "Begegnungen 0000: Schwarm Platz 1 Pokémon 0 → 2"
    );
    data_edit_impl(
        &state,
        DataKind::Encounter,
        0,
        DataPatch::EncounterExtra {
            field: "unown_table".into(),
            value: 3,
        },
        Some("Route 201: Icognito-Tabelle 0 → 3".into()),
    )
    .unwrap();
    assert_eq!(last_label(&state), "Route 201: Icognito-Tabelle 0 → 3");
    match encounter_doc(&state, 0) {
        DataDocDto::Encounter {
            warnings,
            extras,
            replacements,
            ..
        } => {
            assert!(
                !warnings.iter().any(|w| w.text.starts_with("swarm entry 0")),
                "{warnings:?}"
            );
            assert_eq!(extras.unown_table, Some(3));
            assert_eq!(replacements[2].slots[0].species, 2);
        }
        _ => unreachable!(),
    }
}

#[test]
fn encounter_min_above_max_pulls_max() {
    let (_fixture, state) = open_synthetic();
    let encounter = |field: &str, value: i64| DataPatch::Encounter {
        method: "surf".into(),
        slot: Some(0),
        field: field.into(),
        value,
    };
    let surf = |state: &AppState| match encounter_doc(state, 1) {
        DataDocDto::Encounter { methods, .. } => (methods[1].slots[0].min, methods[1].slots[0].max),
        _ => unreachable!(),
    };
    data_edit_impl(&state, DataKind::Encounter, 1, encounter("max", 10), None).unwrap();
    let steps = history_impl(&state).unwrap().undo.len();
    data_edit_impl(&state, DataKind::Encounter, 1, encounter("min", 20), None).unwrap();
    assert_eq!(surf(&state), (20, 20));
    assert_eq!(
        last_label(&state),
        "Begegnungen 0001: Surfen Platz 1 Mindestlevel 0 → 20"
    );
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps + 1);
    data_edit_impl(&state, DataKind::Encounter, 1, encounter("max", 5), None).unwrap();
    assert_eq!(surf(&state), (5, 5));
    // One undo step each: both levels come back together.
    undo_impl(&state).unwrap();
    assert_eq!(surf(&state), (20, 20));
    undo_impl(&state).unwrap();
    assert_eq!(surf(&state), (0, 10));
}

/// The fixture with adopted scripts and a battle script like Platinum's for its three
/// trainers (slot 4 is the sight script), as `trainer_create`'s own tests set it up.
fn with_battle_script() -> (
    g4_kernel::test_support::Synthetic,
    AppState,
    g4_kernel::scripts::TrainerScriptMeta,
) {
    use g4_kernel::scripts::{self, ScriptSource, SourceKind, TrainerScriptMeta};
    let (fixture, state) = open_synthetic();
    let project = g4_kernel::project::Project::open(fixture.dir.path()).unwrap();
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let battle_file = scripts::new_script_file(&project, SourceKind::Script).unwrap();
    let battle = "script script_1 #[1-3]:\n    LockAll\n    FacePlayer\n    ReleaseAll\n    End\n\
                  script script_4 #4:\n    LockAll\n    ReleaseAll\n    End\n";
    scripts::write_source(&project, battle_file, &ScriptSource::Rotom(battle.into())).unwrap();
    let report = scripts::compile(&project).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    (
        fixture,
        state,
        TrainerScriptMeta {
            battle_file,
            seen_script: 3003,
        },
    )
}

#[test]
fn create_trainer_command_reports_placeholder() {
    let (_fixture, state) = open_synthetic();
    // Without adopted scripts: refused with the kernel's words, nothing written.
    let err = create_trainer_impl(&state, 1, "Jens").unwrap_err();
    assert_eq!(
        (err.code, err.message.as_str()),
        (
            "error",
            "Erst Scripts übernehmen (Bereich Welt, Panel Script)"
        )
    );

    let (_fixture, state, meta) = with_battle_script();
    let report = create_trainer_with_impl(&state, &meta, 1, "Jens").unwrap();
    assert_eq!(
        (report.id, report.placeholder, report.script_extended),
        (3, None, false)
    );
    let report = create_trainer_with_impl(&state, &meta, 1, "Lars").unwrap();
    assert_eq!(
        (report.id, report.placeholder, report.script_extended),
        (5, Some(4), true)
    );
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        json!({ "id": 5, "placeholder": 4, "script_extended": true })
    );
    match trainer(&state, 4) {
        DataDocDto::Trainer {
            placeholder,
            locked,
            ..
        } => {
            assert!(placeholder);
            assert_eq!(
                locked.as_deref(),
                Some(g4_kernel::data::trainer_create::PLACEHOLDER_REASON)
            );
        }
        _ => unreachable!(),
    }
    match trainer(&state, 5) {
        DataDocDto::Trainer {
            placeholder,
            locked,
            name,
            party,
            ..
        } => {
            assert!(!placeholder);
            assert_eq!((locked, name.as_str(), party.len()), (None, "Lars", 0));
        }
        _ => unreachable!(),
    }
    let rows = data_list_impl(&state, DataKind::Trainer, None).unwrap();
    assert_eq!(rows.len(), 6);
    assert!(rows[4].locked.is_some());
    // The placeholder refuses edits, its name and texts too; the lock comes before the name
    // length check.
    for patch in [
        DataPatch::TrainerName { name: "X".into() },
        DataPatch::TrainerName {
            name: "Bartholomeus".into(),
        },
        DataPatch::Trainer {
            field: "ai".into(),
            value: 1,
        },
        DataPatch::TrainerTexts {
            texts: vec![TrainerTextInDto {
                kind: 0,
                text: "x\\r".into(),
            }],
        },
    ] {
        let err = data_edit_impl(&state, DataKind::Trainer, 4, patch, None).unwrap_err();
        assert_eq!(err.code, "read_only", "{}", err.message);
    }
    let steps = history_impl(&state).unwrap().undo.len();
    let err = set_trainer_texts_impl(
        &state,
        4,
        vec![g4_kernel::data::trainer_create::TrainerText {
            kind: 0,
            text: "x\\r".into(),
        }],
    )
    .unwrap_err();
    assert_eq!(err.code, "read_only", "{}", err.message);
    assert_eq!(history_impl(&state).unwrap().undo.len(), steps);
}

#[test]
fn set_trainer_texts_reports_problems_and_encounter_files_split() {
    let (_fixture, state) = open_synthetic();
    let out = set_trainer_texts_impl(
        &state,
        2,
        vec![g4_kernel::data::trainer_create::TrainerText {
            kind: 0,
            text: "Hallo!".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        out.changed,
        [
            id(AssetKind::TextArchive, 617),
            id(AssetKind::TrainerTextTable, 0),
            id(AssetKind::TrainerTextOffsets, 0)
        ]
    );
    assert_eq!(out.problems.len(), 1);
    assert_eq!(
        (out.problems[0].source, out.problems[0].file.as_str()),
        ("trainer", "trainers/0002")
    );
    // A text the trainer has already changes the archive alone; that is all it reports.
    let out = set_trainer_texts_impl(
        &state,
        1,
        vec![g4_kernel::data::trainer_create::TrainerText {
            kind: 0,
            text: "Los!\\r".into(),
        }],
    )
    .unwrap();
    assert_eq!(out.changed, [id(AssetKind::TextArchive, 617)]);
    assert!(out.problems.is_empty(), "{:?}", out.problems);

    let index = create_encounter_file_impl(&state, Some(0)).unwrap();
    assert_eq!(index, 2);
    assert_eq!(create_encounter_file_impl(&state, None).unwrap(), 3);
    let split = split_encounter_file_impl(&state, 1).unwrap();
    assert_eq!(
        split,
        SplitDto {
            index: 4,
            changed: vec![id(AssetKind::HeaderTable, 0)]
        }
    );
    match encounter_doc(&state, 4) {
        DataDocDto::Encounter { users, methods, .. } => {
            assert_eq!(
                users,
                [PlaceDto {
                    header: 1,
                    name: "T02".into()
                }]
            );
            assert_eq!(methods[0].slots[0].species, 1);
        }
        _ => unreachable!(),
    }
    // Undo points the header back and leaves the copy.
    undo_impl(&state).unwrap();
    match encounter_doc(&state, 4) {
        DataDocDto::Encounter { users, .. } => assert!(users.is_empty()),
        _ => unreachable!(),
    }
    assert_eq!(
        data_list_impl(&state, DataKind::Encounter, None)
            .unwrap()
            .len(),
        5
    );
    let err = split_encounter_file_impl(&state, 9999).unwrap_err();
    assert_eq!(err.code, "error");
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
    // Where the names come from: a text edit in one of these archives changes them.
    assert_eq!(
        tables.archives,
        NameArchivesDto {
            species: Some(412),
            moves: Some(647),
            items: Some(392),
            abilities: Some(610),
            types: Some(624),
            classes: Some(619),
            forms: Some(697),
        }
    );
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
    // The name of the source on its own: the UI never cuts it out of the label.
    assert_eq!(evolution.source_name, "Bulbasaur");
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
    assert_eq!(wild.source_name, "Begegnungen 0000");
    assert_eq!(
        serde_json::to_value(&wild.nav).unwrap(),
        json!({ "kind": "header", "id": 0 })
    );
    let party = refs
        .iter()
        .find(|r| r.source == id(AssetKind::TrainerParty, 1))
        .unwrap();
    assert_eq!(party.label, "Trainer 0001 Kai, Team Platz 1");
    assert_eq!(party.source_name, "Trainer 0001 Kai");
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
    assert_eq!(learned[1].source_name, "Bulbasaur");
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
        let start = std::time::Instant::now();
        let rows = data_list_impl(&state, kind, None).unwrap();
        eprintln!(
            "data_list {kind:?}: {} rows in {:?}",
            rows.len(),
            start.elapsed()
        );
        assert!(!rows.is_empty());
    }
    // Trainer 1 and encounter file 1 read as documents, unlocked; some trainer stands in the
    // world and its row names the place.
    match data_get_impl(&state, DataKind::Trainer, 1).unwrap() {
        DataDocDto::Trainer {
            locked,
            party,
            name,
            ..
        } => {
            assert_eq!(locked, None);
            assert!(!party.is_empty() && !name.is_empty(), "{name}");
        }
        other => panic!("{other:?}"),
    }
    match data_get_impl(&state, DataKind::Encounter, 1).unwrap() {
        DataDocDto::Encounter {
            locked, methods, ..
        } => {
            assert_eq!(locked, None);
            assert!(!methods.is_empty());
        }
        other => panic!("{other:?}"),
    }
    let trainers = data_list_impl(&state, DataKind::Trainer, None).unwrap();
    assert!(
        trainers
            .iter()
            .any(|r| r.extra["places"].as_array().is_some_and(|p| !p.is_empty()))
    );
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
            name_archive,
            ..
        } => {
            assert_eq!(name_archive, Some(392));
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
