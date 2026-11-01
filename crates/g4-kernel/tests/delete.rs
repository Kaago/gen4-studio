#![cfg(feature = "test-support")]
//! Deleting entities (3d.4): the kinds of target, their rule, their words.
mod common;

use g4_kernel::delete::{
    DeleteTarget, NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT, ORIGINAL_KIND, Rule,
};
use g4_kernel::original::ResetTarget;
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::HEADER_TABLE;

/// One target of every kind, with its rule from the table „Tabelle der Arten“.
const TABLE: [(DeleteTarget, Rule); 16] = [
    (DeleteTarget::Trainer { index: 1 }, Rule::Tombstone),
    (DeleteTarget::Encounter { index: 1 }, Rule::Tombstone),
    (DeleteTarget::EventFile { index: 1 }, Rule::Tombstone),
    (DeleteTarget::Matrix { index: 1 }, Rule::Tombstone),
    (DeleteTarget::Map { index: 1 }, Rule::Tombstone),
    (DeleteTarget::Header { id: 1 }, Rule::OnlyNew),
    (DeleteTarget::Script { file: 1 }, Rule::Truncate),
    (DeleteTarget::TextArchive { index: 1 }, Rule::Truncate),
    (DeleteTarget::AreaData { index: 1 }, Rule::OnlyNew),
    (DeleteTarget::MapTexture { index: 1 }, Rule::OnlyNew),
    (DeleteTarget::BuildingTexture { index: 1 }, Rule::OnlyNew),
    (DeleteTarget::BuildingModel { index: 1 }, Rule::OnlyNew),
    (
        DeleteTarget::InteriorBuildingModel { index: 1 },
        Rule::OnlyNew,
    ),
    (DeleteTarget::Place { index: 1 }, Rule::Truncate),
    (DeleteTarget::Flag { number: 1 }, Rule::RemoveFile),
    (DeleteTarget::Var { number: 1 }, Rule::RemoveFile),
];

#[test]
fn rule_table_matches_the_spec() {
    for (target, rule) in TABLE {
        assert_eq!(target.rule(), rule, "{target:?}");
    }
    // An envelope exactly where the rule allows a tombstone and the art has JSON.
    for (target, _) in TABLE {
        let has_art = !matches!(
            target,
            DeleteTarget::Script { .. }
                | DeleteTarget::TextArchive { .. }
                | DeleteTarget::Place { .. }
                | DeleteTarget::Flag { .. }
                | DeleteTarget::Var { .. }
        );
        assert_eq!(target.art().is_some(), has_art, "{target:?}");
    }
    // The JSON form: `kind` in snake_case, the field names of `ResetTarget`.
    let json = |t: DeleteTarget| serde_json::to_value(t).unwrap();
    assert_eq!(
        json(DeleteTarget::Trainer { index: 928 }),
        serde_json::json!({"kind": "trainer", "index": 928})
    );
    assert_eq!(
        json(DeleteTarget::Header { id: 593 }),
        serde_json::json!({"kind": "header", "id": 593})
    );
    assert_eq!(
        json(DeleteTarget::InteriorBuildingModel { index: 2 }),
        serde_json::json!({"kind": "interior_building_model", "index": 2})
    );
    assert_eq!(
        json(DeleteTarget::Flag { number: 0x94 }),
        serde_json::json!({"kind": "flag", "number": 148})
    );
    let back: DeleteTarget =
        serde_json::from_value(serde_json::json!({"kind": "script", "file": 7})).unwrap();
    assert_eq!(back, DeleteTarget::Script { file: 7 });
}

#[test]
fn labels_use_the_words_of_the_app() {
    let label = |t: DeleteTarget| t.label();
    assert_eq!(label(DeleteTarget::Trainer { index: 928 }), "Trainer 0928");
    assert_eq!(label(DeleteTarget::Header { id: 593 }), "Header 0593");
    assert_eq!(
        label(DeleteTarget::Script { file: 1124 }),
        "Scriptdatei 1124"
    );
    assert_eq!(label(DeleteTarget::Place { index: 125 }), "Ort 125");
    assert_eq!(label(DeleteTarget::Flag { number: 0x94 }), "Flag 0x0094");
    assert_eq!(
        label(DeleteTarget::Var { number: 0x4000 }),
        "Variable 0x4000"
    );
    assert_eq!(
        label(DeleteTarget::Encounter { index: 3 }),
        "Begegnungsdatei 0003"
    );
    assert_eq!(
        label(DeleteTarget::EventFile { index: 3 }),
        "Eventdatei 0003"
    );
    assert_eq!(label(DeleteTarget::Matrix { index: 3 }), "Matrix 0003");
    assert_eq!(label(DeleteTarget::Map { index: 3 }), "Karte 0003");
    assert_eq!(
        label(DeleteTarget::AreaData { index: 3 }),
        "Gebietsdaten 0003"
    );
    assert_eq!(
        label(DeleteTarget::TextArchive { index: 3 }),
        "Textarchiv 0003"
    );
    assert_eq!(
        label(DeleteTarget::MapTexture { index: 3 }),
        "Kartentextur 0003"
    );

    // The word of a kind, for the sentences.
    assert_eq!(DeleteTarget::Trainer { index: 0 }.kind_word(), "Trainer");
    assert_eq!(
        DeleteTarget::Encounter { index: 0 }.kind_word(),
        "Begegnungsdatei"
    );
    assert_eq!(DeleteTarget::Header { id: 0 }.kind_word(), "Header");
    assert_eq!(
        ORIGINAL_KIND.replace("{Art}", DeleteTarget::Header { id: 0 }.kind_word()),
        "Original-Header lassen sich nicht löschen: der Spielcode verweist unsichtbar darauf (ARM9, Overlays)."
    );
    assert_eq!(
        NO_TOMBSTONE_FOR_FILES,
        "Scriptdateien und Textarchive haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen."
    );
    assert_eq!(
        NOT_A_TEXT_PROJECT,
        "Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt)."
    );

    // Assets as a reset names them; the header table for a header.
    let asset = |kind, index| AssetId { kind, index };
    assert_eq!(
        DeleteTarget::Trainer { index: 4 }.assets(),
        [asset(AssetKind::Trainer, 4)]
    );
    assert_eq!(DeleteTarget::Header { id: 4 }.assets(), [HEADER_TABLE]);
    assert_eq!(
        DeleteTarget::EventFile { index: 4 }.assets(),
        [asset(AssetKind::Event, 4)]
    );
    assert!(DeleteTarget::Flag { number: 4 }.assets().is_empty());

    // Translated into a reset target where one exists.
    assert_eq!(
        DeleteTarget::Trainer { index: 4 }.reset_target(),
        Some(ResetTarget::Trainer { index: 4 })
    );
    assert_eq!(
        DeleteTarget::Header { id: 4 }.reset_target(),
        Some(ResetTarget::Header { id: 4 })
    );
    assert_eq!(DeleteTarget::Matrix { index: 4 }.reset_target(), None);
    assert_eq!(DeleteTarget::Place { index: 4 }.reset_target(), None);
    for (target, _) in TABLE {
        if let Some(reset) = target.reset_target() {
            assert_eq!(reset.label(), target.label(), "{target:?}");
            assert_eq!(reset.assets(), target.assets(), "{target:?}");
        }
    }
}
