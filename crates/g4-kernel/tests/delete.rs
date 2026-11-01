#![cfg(feature = "test-support")]
//! Deleting entities (3d.4): the kinds of target, their rule, their words.
mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::narc::Narc;
use g4_kernel::data::refs::{Access, Ref, RefIndex, Target};
use g4_kernel::delete::{
    CANNOT_FREE_ORIGINAL, DeletePlan, DeleteTarget, FreeInfo, NO_TOMBSTONE_FOR_BINARIES,
    NO_TOMBSTONE_FOR_FILES, NOT_A_TEXT_PROJECT, ORIGINAL_KIND, Rule, guard_reference, plan_delete,
};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, NameRequest, name_flag_files};
use g4_kernel::original::{Original, ResetTarget, plan_reset};
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::{HEADER_TABLE, MapHeader, ObjectEvent, Store, WarpEvent};
use g4_kernel::test_support::{ProjectOriginal, TextFixture};

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
    (DeleteTarget::MapTexture { index: 1 }, Rule::Truncate),
    (DeleteTarget::BuildingTexture { index: 1 }, Rule::Truncate),
    (DeleteTarget::BuildingModel { index: 1 }, Rule::Truncate),
    (
        DeleteTarget::InteriorBuildingModel { index: 1 },
        Rule::Truncate,
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
        NO_TOMBSTONE_FOR_BINARIES,
        "Texturen und Modelle haben keinen Grabstein; neue am Ende lassen sich mit „Nummer freigeben“ entfernen."
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

// --- plan_delete and the reference lock (task 4) ---

/// The synthetic text project (scripts adopted) and its store.
fn fixture() -> (TextFixture, Store) {
    let fx = TextFixture::exported();
    let st = Store::new(Arc::new(fx.project()));
    (fx, st)
}

/// The base of the fixture: its packed archives, with the counts of `fewer` instead (so the
/// entries past them are new).
struct Base {
    inner: ProjectOriginal,
    fewer: Vec<(AssetKind, usize)>,
}

impl Base {
    fn of(root: &Path, fewer: &[(AssetKind, usize)]) -> Base {
        Base {
            inner: ProjectOriginal::new(root),
            fewer: fewer.to_vec(),
        }
    }
}

impl Original for Base {
    fn sha1(&self) -> &str {
        self.inner.sha1()
    }
    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
        self.inner.member(kind, index)
    }
    fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError> {
        match self.fewer.iter().find(|(k, _)| *k == kind) {
            Some(&(_, n)) => Ok(Some(n)),
            None => self.inner.count(kind),
        }
    }
    fn header_table(&self) -> Result<Vec<MapHeader>, KernelError> {
        self.inner.header_table()
    }
    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError> {
        self.inner.narc_at(packed)
    }
}

/// No base ROM found: every question fails with the reason.
struct NoBase;

const NO_BASE: &str = "Keine Basis-ROM gefunden: Pokemon Platin (DE) liegt nicht im Suchpfad.";

impl Original for NoBase {
    fn sha1(&self) -> &str {
        ""
    }
    fn member(&self, _: AssetKind, _: u16) -> Result<Option<Vec<u8>>, KernelError> {
        Err(KernelError::Refused(NO_BASE.to_owned()))
    }
    fn count(&self, _: AssetKind) -> Result<Option<usize>, KernelError> {
        Err(KernelError::Refused(NO_BASE.to_owned()))
    }
    fn header_table(&self) -> Result<Vec<MapHeader>, KernelError> {
        Err(KernelError::Refused(NO_BASE.to_owned()))
    }
    fn narc_at(&self, _: &Path) -> Result<Option<Narc>, KernelError> {
        Err(KernelError::Refused(NO_BASE.to_owned()))
    }
}

fn plan(st: &mut Store, base: &dyn Original, target: DeleteTarget) -> DeletePlan {
    plan_delete(st, &mut RefIndex::new(), base, target).unwrap()
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

/// Event file 0 of `st` gets a figure with script `3000 + n - 1`: trainer `n`.
fn place_trainer(st: &mut Store, n: u16) {
    st.edit_event(0, "Figur", |e| {
        e.object_events.push(object(3000 + n - 1));
        Ok(())
    })
    .unwrap();
}

fn tombstone(st: &mut Store, target: DeleteTarget) {
    st.transaction("Gelöscht", |tx| tx.set_deleted(target, true))
        .unwrap();
}

fn rel(fx: &TextFixture, path: PathBuf) -> PathBuf {
    path.strip_prefix(fx.root()).unwrap().to_path_buf()
}

const SOURCE_0: &str = "expanded/scripts/0000.rotom";

const EVENT_0: AssetId = AssetId {
    kind: AssetKind::Event,
    index: 0,
};

#[test]
fn plan_refuses_an_original_header_script_and_archive_with_the_reason() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    for target in [
        DeleteTarget::Header { id: 1 },
        DeleteTarget::Script { file: 0 },
        DeleteTarget::TextArchive { index: 0 },
    ] {
        let p = plan(&mut st, &base, target);
        assert!(p.original, "{target:?}");
        assert_eq!(
            p.refusal.as_deref(),
            Some(ORIGINAL_KIND.replace("{Art}", target.kind_word()).as_str()),
            "{target:?}"
        );
        assert_eq!(p.label, target.label());
        assert_eq!(p.rule, target.rule());
        assert!(p.can_free.is_none(), "{target:?}");
        assert_eq!(p.cannot_free.as_deref(), Some(CANNOT_FREE_ORIGINAL));
    }
    assert_eq!(
        CANNOT_FREE_ORIGINAL,
        "Original-Einträge lassen sich nicht freigeben."
    );

    // A new script file (the base has two, the project three) gets no tombstone; as the last
    // one its number can be freed.
    let newer = Base::of(fx.root(), &[(AssetKind::Script, 2)]);
    let p = plan(&mut st, &newer, DeleteTarget::Script { file: 2 });
    assert!(!p.original);
    assert_eq!(p.refusal.as_deref(), Some(NO_TOMBSTONE_FOR_FILES));
    let free = p.can_free.expect("the last script file can be freed");
    assert_eq!(free.count, 1);
    let member = rel(&fx, fx.project().member_path(AssetKind::Script, 2));
    assert!(free.files.contains(&member), "{:?}", free.files);
    for file in &free.files {
        assert!(fx.root().join(file).exists(), "{}", file.display());
    }
    assert_eq!(p.cannot_free, None);

    // File 1 is new as well, but file 2 lives behind it.
    let newer = Base::of(fx.root(), &[(AssetKind::Script, 1)]);
    let p = plan(&mut st, &newer, DeleteTarget::Script { file: 1 });
    assert!(p.can_free.is_none());
    assert_eq!(
        p.cannot_free.as_deref(),
        Some("Nummer 1 bleibt belegt, weil 1 spätere Einträge leben.")
    );

    // Textures and models have no envelope either.
    let newer = Base::of(fx.root(), &[(AssetKind::BuildingModel, 0)]);
    let p = plan(&mut st, &newer, DeleteTarget::BuildingModel { index: 0 });
    assert_eq!(p.refusal.as_deref(), Some(NO_TOMBSTONE_FOR_BINARIES));
    assert_eq!(p.can_free.map(|f| f.count), Some(1));
}

#[test]
fn plan_of_a_new_trainer_frees_the_trailing_tombstones_with_it() {
    let (fx, mut st) = fixture();
    // Trainers 1 and 2 are new; trainer 2 is a tombstone, so trainer 1 is the last that lives.
    let base = Base::of(
        fx.root(),
        &[
            (AssetKind::TrainerProperties, 1),
            (AssetKind::TrainerParty, 1),
        ],
    );
    tombstone(&mut st, DeleteTarget::Trainer { index: 2 });
    let p = plan(&mut st, &base, DeleteTarget::Trainer { index: 1 });
    assert!(!p.original);
    assert_eq!(p.refusal, None);
    let free = p.can_free.expect("only a tombstone lies behind it");
    assert_eq!(free.count, 2);
    let project = fx.project();
    for (kind, n) in [
        (AssetKind::TrainerProperties, 1),
        (AssetKind::TrainerParty, 1),
        (AssetKind::TrainerProperties, 2),
        (AssetKind::TrainerParty, 2),
    ] {
        let member = rel(&fx, project.member_path(kind, n));
        assert!(
            free.files.contains(&member),
            "{member:?} in {:?}",
            free.files
        );
    }
    for json in ["data/trainers/0001.json", "data/trainers/0002.json"] {
        assert!(
            free.files.contains(&PathBuf::from(json)),
            "{json} in {:?}",
            free.files
        );
    }
    // Trainer 0 (original) is not part of it.
    assert!(
        !free
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("0000")),
        "{:?}",
        free.files
    );
}

#[test]
fn plan_lists_the_references_of_a_used_trainer() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    place_trainer(&mut st, 1);
    let p = plan(&mut st, &base, DeleteTarget::Trainer { index: 1 });
    assert_eq!(p.references.len(), 1, "{:?}", p.references);
    let r = p.references[0];
    assert_eq!(
        (r.source, r.field, r.target),
        (EVENT_0, "script", Target::Trainer(1))
    );
    assert!(p.warnings.is_empty());
    // The lock is the list; the refusal is about tombstones only.
    assert_eq!(p.refusal, None);
}

#[test]
fn plan_lists_header_references_from_matrix_warps_and_scripts() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    Warp 1, 0, 5, 5, 0\n    End\nscript Second #2:\n    End\n",
    );
    st.edit_matrix(0, "Header", |m| {
        m.headers = Some(vec![1, 0]);
        Ok(())
    })
    .unwrap();
    st.edit_event(0, "Warp", |e| {
        e.warp_events.push(WarpEvent {
            x: 0,
            z: 0,
            dest_header_id: 1,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    let p = plan(&mut st, &base, DeleteTarget::Header { id: 1 });
    let found: Vec<(AssetKind, &str)> = p
        .references
        .iter()
        .map(|r| (r.source.kind, r.field))
        .collect();
    assert_eq!(
        found,
        [
            (AssetKind::Matrix, "headers"),
            (AssetKind::Event, "warp_events.dest_header_id"),
            (AssetKind::Script, "Warp"),
        ],
        "{:?}",
        p.references
    );
    // The warp's door is the same place as its header: one reference.
    assert_eq!(
        DeleteTarget::Header { id: 1 }.targets(),
        [Target::Header(1)]
    );
    assert!(DeleteTarget::Header { id: 1 }.refers(Target::Warp {
        header: 1,
        index: 7
    }));
    assert!(!DeleteTarget::Header { id: 1 }.refers(Target::Warp {
        header: 0,
        index: 1
    }));
}

#[test]
fn plan_ignores_references_from_tombstoned_sources() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    place_trainer(&mut st, 1);
    tombstone(&mut st, DeleteTarget::EventFile { index: 0 });
    let p = plan(&mut st, &base, DeleteTarget::Trainer { index: 1 });
    assert!(p.references.is_empty(), "{:?}", p.references);
}

#[test]
fn plan_warns_for_an_original_trainer_and_allows() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    let target = DeleteTarget::Trainer { index: 1 };
    let p = plan(&mut st, &base, target);
    assert!(p.original);
    assert_eq!(p.refusal, None);
    assert_eq!(p.rule, Rule::Tombstone);
    assert!(p.references.is_empty() && p.warnings.is_empty());
    assert!(!p.deleted);
    assert_eq!(p.label, "Trainer 0001");
    tombstone(&mut st, target);
    assert!(plan(&mut st, &base, target).deleted);

    // Outside a text project there is no tombstone.
    let dspre = TextFixture::dspre();
    let mut st = Store::new(Arc::new(dspre.project()));
    let p = plan(&mut st, &ProjectOriginal::new(dspre.root()), target);
    assert_eq!(p.refusal.as_deref(), Some(NOT_A_TEXT_PROJECT));
}

#[test]
fn plan_of_a_flag_name_lists_the_scripts_using_it() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    name_flag_files(
        &fx.project(),
        &NameRequest {
            kind: FlagKind::Flag,
            number: 148,
            id: Some("FLAG_HAT_MEDAILLE".into()),
            description: None,
        },
    )
    .unwrap();
    let target = DeleteTarget::Flag { number: 148 };
    let p = plan(&mut st, &base, target);
    assert!(p.references.is_empty(), "{:?}", p.references);
    assert_eq!(
        (p.rule, p.original, p.refusal),
        (Rule::RemoveFile, false, None)
    );

    fx.write(
        SOURCE_0,
        b"script Main #1:\n    SetFlag FLAG_HAT_MEDAILLE\n    End\nscript Second #2:\n    End\n",
    );
    let p = plan(&mut st, &base, target);
    let script_0 = AssetId {
        kind: AssetKind::Script,
        index: 0,
    };
    assert_eq!(
        p.references,
        [Ref {
            source: script_0,
            field: "identifier",
            slot: None,
            target: Target::Flag(148),
            access: Access::Unknown,
        }]
    );
    // The pret name and the number stay valid: a reference by number does not lock.
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    SetFlag 148\n    End\nscript Second #2:\n    End\n",
    );
    assert!(plan(&mut st, &base, target).references.is_empty());
}

#[test]
fn guard_reference_refuses_a_tombstone() {
    let (_fx, mut st) = fixture();
    assert!(guard_reference(&st, Target::Trainer(1)).is_ok());
    tombstone(&mut st, DeleteTarget::Trainer { index: 1 });
    tombstone(&mut st, DeleteTarget::EventFile { index: 1 });
    let refused = |target| match guard_reference(&st, target) {
        Err(KernelError::Refused(why)) => why,
        other => panic!("{target:?}: {other:?}"),
    };
    assert_eq!(refused(Target::Trainer(1)), "Trainer 0001 ist gelöscht.");
    assert_eq!(
        refused(Target::EventFile(1)),
        "Eventdatei 0001 ist gelöscht."
    );
    assert!(guard_reference(&st, Target::Trainer(0)).is_ok());
    assert!(guard_reference(&st, Target::Header(1)).is_ok());
    assert!(guard_reference(&st, Target::Species(1)).is_ok());
}

#[test]
fn plan_answers_base_missing_without_a_base_rom() {
    let (_fx, mut st) = fixture();
    let reset = plan_reset(&mut st, &NoBase, &ResetTarget::Trainer { index: 1 }).unwrap_err();
    let delete = plan_delete(
        &mut st,
        &mut RefIndex::new(),
        &NoBase,
        DeleteTarget::Trainer { index: 1 },
    )
    .unwrap_err();
    assert!(matches!(delete, KernelError::Refused(_)), "{delete:?}");
    assert_eq!(delete.to_string(), reset.to_string());
    assert_eq!(delete.to_string(), NO_BASE);
}

#[test]
fn referenced_names_the_entity_and_the_count() {
    let r = Ref {
        source: EVENT_0,
        field: "script",
        slot: Some(0),
        target: Target::Trainer(1),
        access: Access::Read,
    };
    let e = KernelError::Referenced {
        label: "Trainer 0001".into(),
        references: vec![r, r],
    };
    assert_eq!(
        e.to_string(),
        "Trainer 0001 wird noch verwendet (2 Verweise)."
    );
}

#[test]
fn free_info_serializes_its_count_and_files() {
    let info = FreeInfo {
        count: 1,
        files: vec![PathBuf::from("data/trainers/0003.json")],
    };
    assert_eq!(
        serde_json::to_value(&info).unwrap(),
        serde_json::json!({"count": 1, "files": ["data/trainers/0003.json"]})
    );
}
