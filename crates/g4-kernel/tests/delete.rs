#![cfg(feature = "test-support")]
//! Deleting entities (3d.4): the kinds of target, their rule, their words.
mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::narc::Narc;
use g4_kernel::build;
use g4_kernel::data::refs::{Access, Ref, RefIndex, Target};
use g4_kernel::delete::{
    DeletePlan, DeleteTarget, FreeInfo, Rule, cannot_free_original, guard_header, guard_reference,
    later_entries_live, no_tombstone_for_binaries, no_tombstone_for_files, not_a_text_project,
    original_kind, plan_delete,
};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, NameRequest, name_flag_files};
use g4_kernel::original::{Original, ResetTarget, plan_reset};
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::store::{
    HEADER_TABLE, MapHeader, ObjectEvent, PLACEHOLDER_REASON, Store, WarpEvent,
};
use g4_kernel::test_support::{ProjectOriginal, TextFixture};
use g4_kernel::textproject::Art;

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
        "Script file 1124"
    );
    assert_eq!(label(DeleteTarget::Place { index: 125 }), "Place 125");
    assert_eq!(label(DeleteTarget::Flag { number: 0x94 }), "Flag 0x0094");
    assert_eq!(
        label(DeleteTarget::Var { number: 0x4000 }),
        "Variable 0x4000"
    );
    assert_eq!(
        label(DeleteTarget::Encounter { index: 3 }),
        "Encounter file 0003"
    );
    assert_eq!(
        label(DeleteTarget::EventFile { index: 3 }),
        "Event file 0003"
    );
    assert_eq!(label(DeleteTarget::Matrix { index: 3 }), "Matrix 0003");
    assert_eq!(label(DeleteTarget::Map { index: 3 }), "Map 0003");
    assert_eq!(label(DeleteTarget::AreaData { index: 3 }), "Area data 0003");
    assert_eq!(
        label(DeleteTarget::TextArchive { index: 3 }),
        "Text archive 0003"
    );
    assert_eq!(
        label(DeleteTarget::MapTexture { index: 3 }),
        "Map texture 0003"
    );

    // The word of a kind, for the sentences.
    assert_eq!(DeleteTarget::Trainer { index: 0 }.kind_word(), "Trainer");
    assert_eq!(
        DeleteTarget::Encounter { index: 0 }.kind_word(),
        "Encounter file"
    );
    assert_eq!(DeleteTarget::Header { id: 0 }.kind_word(), "Header");
    // `{Art}` of the refusal of an original is the plural.
    assert_eq!(
        original_kind(DeleteTarget::Header { id: 0 }),
        "Original headers cannot be deleted: the game code refers to them invisibly (ARM9, overlays)."
    );
    assert_eq!(
        original_kind(DeleteTarget::Script { file: 0 }),
        "Original script files cannot be deleted: the game code refers to them invisibly (ARM9, overlays)."
    );
    let plurals: Vec<String> = [
        DeleteTarget::Header { id: 0 },
        DeleteTarget::Script { file: 0 },
        DeleteTarget::TextArchive { index: 0 },
        DeleteTarget::AreaData { index: 0 },
        DeleteTarget::MapTexture { index: 0 },
        DeleteTarget::BuildingTexture { index: 0 },
        DeleteTarget::BuildingModel { index: 0 },
        DeleteTarget::InteriorBuildingModel { index: 0 },
    ]
    .iter()
    .map(|t| t.kind_plural())
    .collect();
    assert_eq!(
        plurals,
        [
            "headers",
            "script files",
            "text archives",
            "area data",
            "map textures",
            "building textures",
            "building models",
            "interior building models",
        ]
    );
    // One later entry, several.
    assert_eq!(
        later_entries_live(593, 1),
        "Number 593 stays taken because 1 later entry still exists."
    );
    assert_eq!(
        later_entries_live(901, 2),
        "Number 901 stays taken because 2 later entries still exist."
    );
    assert_eq!(
        no_tombstone_for_files(),
        "Script files and text archives have no tombstone; new ones at the end can be removed with “Free the number”."
    );
    assert_eq!(
        no_tombstone_for_binaries(),
        "Textures and models have no tombstone; new ones at the end can be removed with “Free the number”."
    );
    assert_eq!(
        not_a_text_project(),
        "Real deleting is available in a text project (Technical › Text project)."
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
            Some(original_kind(target).as_str()),
            "{target:?}"
        );
        assert_eq!(p.label, target.label());
        assert_eq!(p.rule, target.rule());
        assert!(p.can_free.is_none(), "{target:?}");
        assert_eq!(
            p.cannot_free.as_deref(),
            Some(cannot_free_original().as_str())
        );
    }
    assert_eq!(cannot_free_original(), "Original entries cannot be freed.");

    // A new script file (the base has two, the project three) gets no tombstone; as the last
    // one its number can be freed.
    let newer = Base::of(fx.root(), &[(AssetKind::Script, 2)]);
    let p = plan(&mut st, &newer, DeleteTarget::Script { file: 2 });
    assert!(!p.original);
    assert_eq!(
        p.refusal.as_deref(),
        Some(no_tombstone_for_files().as_str())
    );
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
        Some("Number 1 stays taken because 1 later entry still exists.")
    );

    // Textures and models have no envelope either.
    let newer = Base::of(fx.root(), &[(AssetKind::BuildingModel, 0)]);
    let p = plan(&mut st, &newer, DeleteTarget::BuildingModel { index: 0 });
    assert_eq!(
        p.refusal.as_deref(),
        Some(no_tombstone_for_binaries().as_str())
    );
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
fn a_text_archive_is_referenced_by_its_messages() {
    // A message a script or dialog shows (3d.5's `Target::Message`) locks its archive too.
    let archive = DeleteTarget::TextArchive { index: 3 };
    assert!(archive.refers(Target::TextArchive(3)));
    assert!(archive.refers(Target::Message {
        archive: 3,
        index: 5
    }));
    assert!(!archive.refers(Target::Message {
        archive: 4,
        index: 5
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
    assert_eq!(p.refusal.as_deref(), Some(not_a_text_project().as_str()));
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
            value: None,
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
    assert_eq!(refused(Target::Trainer(1)), "Trainer 0001 is deleted.");
    assert_eq!(refused(Target::EventFile(1)), "Event file 0001 is deleted.");
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
        value: None,
    };
    let e = KernelError::referenced("Trainer 0001".into(), vec![r, r]);
    assert_eq!(
        e.to_string(),
        "Trainer 0001 is still in use (2 references)."
    );
    let one = KernelError::referenced("Trainer 0001".into(), vec![r]);
    assert_eq!(
        one.to_string(),
        "Trainer 0001 is still in use (1 reference)."
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

// --- delete, restore and „Nummer freigeben“ (task 5) ---

use g4_kernel::delete::{
    DeleteResult, delete, free_number, irreversible, locked_refusal, place_only_last, restore,
};
use g4_kernel::places::{create_place, location_names};
use g4_kernel::textproject::check;
use std::collections::BTreeMap;

const FLAG_REGISTER: &str = "data/ids/flags.json";
const FLAG_FILE: &str = "data/flags/0148.json";
const FLAG_IDS: &str = ".rotom/command_database/.g4-ids-flags.json";
const PLACE_REGISTER: &str = "data/ids/places.json";
const EVENT_JSON_0: &str = "data/events/0000.json";

fn refused<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn trainer_asset(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Trainer,
        index,
    }
}

/// The base of the fixture with `first` trainers: the trainers from `first` on are new.
fn new_trainers_from(fx: &TextFixture, first: usize) -> Base {
    Base::of(
        fx.root(),
        &[
            (AssetKind::TrainerProperties, first),
            (AssetKind::TrainerParty, first),
        ],
    )
}

fn last_label(st: &Store) -> String {
    st.history().0.last().cloned().unwrap_or_default()
}

/// Whether `rel` (relative to the root) exists.
fn exists(fx: &TextFixture, rel: impl AsRef<Path>) -> bool {
    fx.root().join(rel).exists()
}

#[test]
fn delete_sets_the_tombstone_as_one_step_and_restore_takes_it_back() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    let target = DeleteTarget::Trainer { index: 1 };
    let json = "data/trainers/0001.json";
    let before = fx.read(json);
    let depth = st.undo_depth();

    let result: DeleteResult = delete(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert!(result.change.is_some());
    assert_eq!(result.changed, [trainer_asset(1)]);
    assert_eq!(result.freed, None);
    assert!(st.is_deleted(target));
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(last_label(&st), "Deleted: Trainer 0001");

    // ⌘Z takes it back; saved, the file is the one before.
    st.undo().unwrap();
    assert!(!st.is_deleted(target));
    st.save().unwrap();
    assert_eq!(fx.read(json), before);

    // Deleted and saved: the envelope; „Wiederherstellen“ takes it back as its own step.
    st.redo().unwrap();
    st.save().unwrap();
    assert!(
        fx.text(json).contains("\"deleted\": true"),
        "{}",
        fx.text(json)
    );
    assert_eq!(
        refused(delete(&mut st, &mut RefIndex::new(), &base, target)),
        "Trainer 0001 is already deleted."
    );
    let result = restore(&mut st, target).unwrap();
    assert!(result.change.is_some());
    assert_eq!(result.changed, [trainer_asset(1)]);
    assert_eq!(last_label(&st), "Restored: Trainer 0001");
    assert!(!st.is_deleted(target));
    st.save().unwrap();
    assert_eq!(fx.read(json), before);

    assert_eq!(
        refused(restore(&mut st, target)),
        "Trainer 0001 is not deleted."
    );
}

#[test]
fn delete_refuses_a_referenced_trainer_with_the_list() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    place_trainer(&mut st, 1);
    let depth = st.undo_depth();
    let target = DeleteTarget::Trainer { index: 1 };
    match delete(&mut st, &mut RefIndex::new(), &base, target) {
        Err(KernelError::Referenced {
            label, references, ..
        }) => {
            assert_eq!(label, "Trainer 0001");
            assert_eq!(references.len(), 1, "{references:?}");
        }
        other => panic!("expected Referenced, got {other:?}"),
    }
    assert_eq!(st.undo_depth(), depth);
    assert!(!st.is_deleted(target));

    // Outside a text project there is no tombstone.
    let dspre = TextFixture::dspre();
    let mut st = Store::new(Arc::new(dspre.project()));
    let base = ProjectOriginal::new(dspre.root());
    assert_eq!(
        refused(delete(
            &mut st,
            &mut RefIndex::new(),
            &base,
            DeleteTarget::Trainer { index: 2 }
        )),
        not_a_text_project()
    );
}

#[test]
fn delete_of_a_flag_name_removes_file_and_register_entry_as_a_file_step() {
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
    // An event file names the flag by its own name (the export writes it so).
    st.edit_event(0, "Figur", |e| {
        let mut figure = object(0);
        figure.hidden_flag = 148;
        e.object_events.push(figure);
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    assert!(
        fx.text(EVENT_JSON_0)
            .contains("\"hidden_flag\": \"FLAG_HAT_MEDAILLE\""),
        "{}",
        fx.text(EVENT_JSON_0)
    );
    let register = fx.read(FLAG_REGISTER);
    let entity = fx.read(FLAG_FILE);
    let event = fx.read(EVENT_JSON_0);
    assert!(fx.text(FLAG_IDS).contains("FLAG_HAT_MEDAILLE"));
    let depth = st.undo_depth();

    let target = DeleteTarget::Flag { number: 148 };
    let result = delete(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert!(result.change.is_some());
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(last_label(&st), "Deleted: Flag 0x0094");
    assert!(!exists(&fx, FLAG_FILE));
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    // The event names the number again, which stays valid.
    let text = fx.text(EVENT_JSON_0);
    assert!(text.contains("\"hidden_flag\": 148"), "{text}");
    assert!(!text.contains("FLAG_HAT_MEDAILLE"), "{text}");
    // The id file for rotom is written anew without it.
    assert!(
        !exists(&fx, FLAG_IDS) || !fx.text(FLAG_IDS).contains("FLAG_HAT_MEDAILLE"),
        "{}",
        fx.text(FLAG_IDS)
    );
    // The import reads every file: no unknown id.
    let report = check(&fx.project()).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != g4_kernel::text::Severity::Error),
        "{:?}",
        report.problems
    );

    // ⌘Z brings the file, the register entry and the event's name back.
    st.undo().unwrap();
    assert_eq!(fx.read(FLAG_FILE), entity);
    assert_eq!(fx.read(FLAG_REGISTER), register);
    assert_eq!(fx.read(EVENT_JSON_0), event);
    assert!(fx.text(FLAG_IDS).contains("FLAG_HAT_MEDAILLE"));
}

#[test]
fn delete_of_the_last_own_place_is_one_step() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    let index = create_place(&mut st, "Neuort", &[]).unwrap().index;
    let count = location_names(&mut st).unwrap().len();
    assert_eq!(count, usize::from(index) + 1);
    // Its register entry (the export christens places).
    let mut places: BTreeMap<String, u32> =
        serde_json::from_slice(&fx.read(PLACE_REGISTER)).unwrap();
    places.insert("PLACE_NEUORT".into(), u32::from(index));
    let mut bytes = serde_json::to_vec_pretty(&places).unwrap();
    bytes.push(b'\n');
    fx.write(PLACE_REGISTER, &bytes);
    let depth = st.undo_depth();

    // Not the last one, or an original: refused.
    assert_eq!(
        refused(delete(
            &mut st,
            &mut RefIndex::new(),
            &base,
            DeleteTarget::Place { index: 0 }
        )),
        place_only_last()
    );

    let target = DeleteTarget::Place {
        index: u16::from(index),
    };
    let result = delete(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert!(result.change.is_some());
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(last_label(&st), "Place removed: Neuort");
    assert_eq!(location_names(&mut st).unwrap().len(), count - 1);
    assert!(!fx.text(PLACE_REGISTER).contains("PLACE_NEUORT"));

    // ⌘Z: the name and the register entry come back together.
    st.undo().unwrap();
    assert_eq!(location_names(&mut st).unwrap().len(), count);
    assert_eq!(fx.read(PLACE_REGISTER), bytes);
}

/// Header `id` of `st` names place `place` (one step).
fn name_place(st: &mut Store, id: u16, place: u16) {
    st.edit_header(id, "Ort", |h| {
        match h {
            MapHeader::DP(h) => h.location_name = place,
            MapHeader::Pt(h) => h.location_name = u8::try_from(place).unwrap(),
            MapHeader::HGSS(h) => h.location_name = u8::try_from(place).unwrap(),
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn plan_of_a_place_allows_only_the_last_own_one() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    let first = u16::from(create_place(&mut st, "Erstort", &[]).unwrap().index);
    let last = u16::from(create_place(&mut st, "Zweitort", &[]).unwrap().index);
    assert_eq!(last, first + 1);

    // The last own place: allowed; it goes as a file step, there is no number to free.
    let p = plan(&mut st, &base, DeleteTarget::Place { index: last });
    assert_eq!(p.refusal, None);
    assert_eq!(p.cannot_free, None);
    assert!(p.can_free.is_none());
    assert!(!p.original);
    assert!(p.references.is_empty(), "{:?}", p.references);

    // An own place before it and an original one: refused with the sentence.
    for index in [first, 0] {
        let p = plan(&mut st, &base, DeleteTarget::Place { index });
        assert_eq!(
            p.refusal.as_deref(),
            Some(place_only_last().as_str()),
            "{index}"
        );
        assert_eq!(
            p.cannot_free.as_deref(),
            Some(place_only_last().as_str()),
            "{index}"
        );
        assert_eq!(
            place_only_last(),
            "Only the last place of your own can be removed; places do not move up."
        );
    }

    // A header naming the last one: its reference locks it.
    name_place(&mut st, 1, last);
    let p = plan(&mut st, &base, DeleteTarget::Place { index: last });
    let found: Vec<(AssetId, &str, Option<u16>)> = p
        .references
        .iter()
        .map(|r| (r.source, r.field, r.slot))
        .collect();
    assert_eq!(found, [(HEADER_TABLE, "location_name", Some(1))]);
    assert_eq!(p.refusal, None);

    // A tombstoned header's entry in the table does not count.
    tombstone(&mut st, DeleteTarget::Header { id: 1 });
    let p = plan(&mut st, &base, DeleteTarget::Place { index: last });
    assert!(p.references.is_empty(), "{:?}", p.references);
}

#[test]
fn free_number_truncates_a_new_trainer_at_the_end() {
    let (fx, mut st) = fixture();
    let base = new_trainers_from(&fx, 2);
    let mut refs = RefIndex::new();
    let target = DeleteTarget::Trainer { index: 2 };
    let count = st.member_count(AssetKind::Trainer).unwrap();
    let files = [
        PathBuf::from("data/trainers/0002.json"),
        rel(
            &fx,
            fx.project().member_path(AssetKind::TrainerProperties, 2),
        ),
        rel(&fx, fx.project().member_path(AssetKind::TrainerParty, 2)),
    ];
    for file in &files {
        assert!(exists(&fx, file), "{}", file.display());
    }
    delete(&mut st, &mut refs, &base, target).unwrap();

    let result = free_number(&mut st, &mut refs, &base, target).unwrap();
    let freed = result.freed.expect("freed");
    assert_eq!(freed.count, 1);
    for file in &files {
        assert!(freed.files.contains(file), "{file:?} in {:?}", freed.files);
        assert!(!exists(&fx, file), "{}", file.display());
    }
    assert_eq!(result.changed, [trainer_asset(2)]);
    assert_eq!(st.member_count(AssetKind::Trainer).unwrap(), count - 1);
    assert!(!st.is_deleted(target));

    // The barrier stops undo; the tombstone step before it stays in the history.
    let labels = st.history().0;
    assert_eq!(
        labels[labels.len() - 2..],
        [
            "Deleted: Trainer 0002".to_owned(),
            "Number freed: Trainer 0002".to_owned()
        ]
    );
    assert_eq!(refused(st.undo()), irreversible());
    assert_eq!(irreversible(), "Freeing the number cannot be undone");
    // Unsaved are only the trainer names and battle text offsets the barrier shortened.
    assert!(
        st.dirty().iter().all(|id| matches!(
            id.kind,
            AssetKind::TextArchive | AssetKind::TrainerTextOffsets
        )),
        "{:?}",
        st.dirty()
    );
    st.save().unwrap();

    // The import finds JSON and members in step.
    let report = check(&fx.project()).unwrap();
    assert!(report.differs.is_empty(), "{:?}", report.differs);
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != g4_kernel::text::Severity::Error),
        "{:?}",
        report.problems
    );
}

#[test]
fn free_number_takes_the_trailing_tombstones_with_it() {
    let (fx, mut st) = fixture();
    let base = new_trainers_from(&fx, 1);
    let mut refs = RefIndex::new();
    for index in [2, 1] {
        delete(&mut st, &mut refs, &base, DeleteTarget::Trainer { index }).unwrap();
    }
    let result = free_number(
        &mut st,
        &mut refs,
        &base,
        DeleteTarget::Trainer { index: 1 },
    )
    .unwrap();
    assert_eq!(result.freed.map(|f| f.count), Some(2));
    assert_eq!(result.changed, [trainer_asset(1), trainer_asset(2)]);
    assert_eq!(st.member_count(AssetKind::Trainer).unwrap(), 1);
    for json in ["data/trainers/0001.json", "data/trainers/0002.json"] {
        assert!(!exists(&fx, json), "{json}");
    }
    assert!(exists(&fx, "data/trainers/0000.json"));
    assert_eq!(last_label(&st), "Number freed: Trainer 0001");
}

#[test]
fn free_number_refuses_a_tombstone_before_a_living_entry() {
    let (fx, mut st) = fixture();
    let base = new_trainers_from(&fx, 1);
    let mut refs = RefIndex::new();
    let target = DeleteTarget::Trainer { index: 1 };
    delete(&mut st, &mut refs, &base, target).unwrap();
    let depth = st.undo_depth();
    assert_eq!(
        refused(free_number(&mut st, &mut refs, &base, target)),
        "Number 1 stays taken because 1 later entry still exists."
    );
    assert_eq!(st.undo_depth(), depth);
    assert!(exists(&fx, "data/trainers/0001.json"));
    assert_eq!(st.member_count(AssetKind::Trainer).unwrap(), 3);
    assert!(st.is_deleted(target));
}

#[test]
fn plan_refuses_a_locked_trainer_up_front_in_german() {
    // Gate 3d.4 B9: the placeholder trainer of the sight script slot was planned with
    // `refusal: null` and `can_free`, then the delete failed with an English `read_only`.
    let (fx, mut st) = fixture();
    let base = new_trainers_from(&fx, 1);
    let mut refs = RefIndex::new();
    let target = DeleteTarget::Trainer { index: 2 };
    st.lock(trainer_asset(2), PLACEHOLDER_REASON).unwrap();
    let sentence = locked_refusal("Trainer 0002", PLACEHOLDER_REASON);
    assert_eq!(
        sentence,
        "Trainer 0002 is the placeholder for the sight script and cannot be deleted."
    );
    let p = plan(&mut st, &base, target);
    assert_eq!(p.refusal.as_deref(), Some(sentence.as_str()));
    assert_eq!(p.can_free, None);
    assert_eq!(p.cannot_free.as_deref(), Some(sentence.as_str()));
    let depth = st.undo_depth();
    assert_eq!(refused(delete(&mut st, &mut refs, &base, target)), sentence);
    assert_eq!(
        refused(free_number(&mut st, &mut refs, &base, target)),
        sentence
    );
    assert_eq!(st.undo_depth(), depth);
    assert!(exists(&fx, "data/trainers/0002.json"));

    // Any other lock says its reason in the same German frame.
    st.lock(trainer_asset(1), "17 bytes read, 16 would be written")
        .unwrap();
    let other = plan(&mut st, &base, DeleteTarget::Trainer { index: 1 });
    assert_eq!(
        other.refusal.as_deref(),
        Some(
            "Trainer 0001 is read-only (17 bytes read, 16 would be written) and cannot be \
             deleted."
        )
    );
    assert_eq!(other.can_free, None);
}

#[test]
fn free_number_refuses_an_original() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    let depth = st.undo_depth();
    assert_eq!(
        refused(free_number(
            &mut st,
            &mut RefIndex::new(),
            &base,
            DeleteTarget::Trainer { index: 2 }
        )),
        cannot_free_original()
    );
    assert_eq!(st.undo_depth(), depth);
    assert!(exists(&fx, "data/trainers/0002.json"));
    assert!(!st.is_deleted(DeleteTarget::Trainer { index: 2 }));
}

#[test]
fn free_number_of_a_new_script_removes_source_member_and_compile_entry() {
    use g4_kernel::scripts::snapshot_script;
    let (fx, mut st) = fixture();
    let base = Base::of(fx.root(), &[(AssetKind::Script, 2)]);
    let target = DeleteTarget::Script { file: 2 };
    // The new script file gets a source of its own, compiled: all three parts are there.
    fx.write("expanded/scripts/0002.rotom", b"script Neu #1:\n    End\n");
    let compiled = g4_kernel::scripts::compile(&fx.project()).unwrap();
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    let before = snapshot_script(&fx.project(), 2).unwrap();
    assert!(before.member.is_some());
    assert!(
        before.rotom.is_some() || before.json.is_some(),
        "{before:?}"
    );
    assert!(before.entry.is_some(), "{before:?}");

    let result = free_number(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert_eq!(result.freed.map(|f| f.count), Some(1));
    let after = snapshot_script(&fx.project(), 2).unwrap();
    assert_eq!(after, Default::default());
    assert_eq!(st.member_count(AssetKind::Script).unwrap(), 2);
    assert_eq!(last_label(&st), "Number freed: Script file 0002");
    assert_eq!(refused(st.undo()), irreversible());
    // No tombstone step before it: script files have none.
    assert!(
        !st.history().0.iter().any(|l| l.starts_with("Deleted")),
        "{:?}",
        st.history().0
    );
}

#[test]
fn free_number_of_a_new_text_archive_removes_json_and_member() {
    use g4_kernel::build::state::BuildState;
    let (fx, mut st) = fixture();
    let count = st.member_count(AssetKind::TextArchive).unwrap();
    let last = u16::try_from(count - 1).unwrap();
    let base = Base::of(fx.root(), &[(AssetKind::TextArchive, count - 1)]);
    let target = DeleteTarget::TextArchive { index: last };
    let json = PathBuf::from(format!("expanded/textArchives/{last:04}.json"));
    let member = rel(&fx, fx.project().member_path(AssetKind::TextArchive, last));
    assert!(exists(&fx, &member));
    // The text sync knows it.
    g4_kernel::text::record_text_sync(&fx.project(), &[last]).unwrap();
    assert!(BuildState::load(fx.root()).texts.contains_key(&last));

    let result = free_number(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    let freed = result.freed.unwrap();
    assert!(freed.files.contains(&member), "{:?}", freed.files);
    assert!(!exists(&fx, &json));
    assert!(!exists(&fx, &member));
    assert_eq!(st.member_count(AssetKind::TextArchive).unwrap(), count - 1);
    assert!(!BuildState::load(fx.root()).texts.contains_key(&last));
    assert_eq!(refused(st.undo()), irreversible());
}

/// The base of the fixture with its first `headers` headers only (the ones after are new).
struct HeaderBase {
    inner: ProjectOriginal,
    headers: usize,
}

impl Original for HeaderBase {
    fn sha1(&self) -> &str {
        self.inner.sha1()
    }
    fn member(&self, kind: AssetKind, index: u16) -> Result<Option<Vec<u8>>, KernelError> {
        self.inner.member(kind, index)
    }
    fn count(&self, kind: AssetKind) -> Result<Option<usize>, KernelError> {
        self.inner.count(kind)
    }
    fn header_table(&self) -> Result<Vec<MapHeader>, KernelError> {
        let mut table = self.inner.header_table()?;
        table.truncate(self.headers);
        Ok(table)
    }
    fn narc_at(&self, packed: &Path) -> Result<Option<Narc>, KernelError> {
        self.inner.narc_at(packed)
    }
}

#[test]
fn free_number_of_a_new_header_shortens_the_table() {
    let (fx, mut st) = fixture();
    st.apply_dynamic_headers().unwrap();
    let id = st.create_header(0, "NEU").unwrap();
    assert_eq!(id, 2);
    let base = HeaderBase {
        inner: ProjectOriginal::new(fx.root()),
        headers: 2,
    };
    let mut refs = RefIndex::new();
    let target = DeleteTarget::Header { id };
    let files = [
        PathBuf::from("data/headers/0002.json"),
        PathBuf::from("unpacked/dynamicHeaders/0002"),
    ];
    for file in &files {
        assert!(exists(&fx, file), "{}", file.display());
    }
    let mapname = fx.project().mapname_path();
    assert_eq!(std::fs::read(&mapname).unwrap().len(), 48);

    delete(&mut st, &mut refs, &base, target).unwrap();
    let result = free_number(&mut st, &mut refs, &base, target).unwrap();
    assert_eq!(result.freed.map(|f| f.count), Some(1));
    assert_eq!(result.changed, [HEADER_TABLE]);
    for file in &files {
        assert!(!exists(&fx, file), "{}", file.display());
    }
    assert_eq!(std::fs::read(&mapname).unwrap().len(), 32);
    assert_eq!(st.header_table().unwrap().len(), 2);
    assert_eq!(fx.project().header_count().unwrap(), 2);
    assert_eq!(refused(st.undo()), irreversible());
    assert!(st.dirty().is_empty(), "{:?}", st.dirty());
    st.save().unwrap();
    let report = check(&fx.project()).unwrap();
    assert!(report.differs.is_empty(), "{:?}", report.differs);
}

#[test]
fn free_number_works_in_the_dspre_layout() {
    let fx = TextFixture::dspre();
    let mut st = Store::new(Arc::new(fx.project()));
    let base = new_trainers_from(&fx, 2);
    let target = DeleteTarget::Trainer { index: 2 };
    let members = [
        rel(
            &fx,
            fx.project().member_path(AssetKind::TrainerProperties, 2),
        ),
        rel(&fx, fx.project().member_path(AssetKind::TrainerParty, 2)),
    ];
    // The members are unpacked first.
    st.trainer(2).unwrap();
    for member in &members {
        assert!(exists(&fx, member), "{}", member.display());
    }
    let result = free_number(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert_eq!(result.freed.map(|f| f.count), Some(1));
    for member in &members {
        assert!(!exists(&fx, member), "{}", member.display());
    }
    assert_eq!(st.member_count(AssetKind::Trainer).unwrap(), 2);
    assert_eq!(st.history().0, ["Number freed: Trainer 0002"]);
    assert_eq!(refused(st.undo()), irreversible());
}

// --- fix round 1: a freed trainer number is taken again ---

/// A battle script for three trainers (slots 1 to 3), slot 4 the sight script.
const BATTLE: &str = "script script_1 #[1-3]:\n    LockAll\n    FacePlayer\n    \
                      ReleaseAll\n    End\nscript script_4 #4:\n    LockAll\n    \
                      ReleaseAll\n    End\n";

/// Creates a trainer with the real `create_trainer_with` (a battle script of its own, the
/// sight script 3003), adopting the scripts first where they are not.
fn create_trainer_in(
    project: &Arc<g4_kernel::project::Project>,
    st: &mut Store,
    names: &mut g4_kernel::data::names::NameTables,
    battle_file: &mut Option<u16>,
    name: &str,
) -> u16 {
    use g4_kernel::data::trainer_create::{TRAINER_FLAG_CAPACITY, create_trainer_with};
    use g4_kernel::scripts::{self, ScriptSource, SourceKind, TrainerScriptMeta};
    let file = *battle_file.get_or_insert_with(|| {
        if !scripts::is_adopted(project) {
            scripts::adopt(project, &mut |_| {}).unwrap();
        }
        let file = scripts::new_script_file(project, SourceKind::Script).unwrap();
        scripts::write_source(project, file, &ScriptSource::Rotom(BATTLE.into())).unwrap();
        let report = scripts::compile(project).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        file
    });
    let meta = TrainerScriptMeta {
        battle_file: file,
        seen_script: 3003,
    };
    create_trainer_with(project, st, names, &meta, TRAINER_FLAG_CAPACITY, 1, name)
        .unwrap()
        .id
}

#[test]
fn a_freed_trainer_number_is_created_again() {
    for text_project in [true, false] {
        let fx = if text_project {
            TextFixture::exported()
        } else {
            TextFixture::dspre()
        };
        let project = Arc::new(fx.project());
        let mut st = Store::new(project.clone());
        let mut names = g4_kernel::data::names::NameTables::new();
        let mut battle = None;
        let id = create_trainer_in(&project, &mut st, &mut names, &mut battle, "Jens");
        assert_eq!(id, 3, "text project: {text_project}");
        // The base has three trainers: trainer 3 is new.
        let base = ProjectOriginal::new(fx.root());
        let mut refs = RefIndex::new();
        let target = DeleteTarget::Trainer { index: id };
        if text_project {
            delete(&mut st, &mut refs, &base, target).unwrap();
        }
        free_number(&mut st, &mut refs, &base, target).unwrap();
        assert_eq!(st.member_count(AssetKind::Trainer).unwrap(), 3);
        // `trtblofs` is the base's again: two offsets (fewer than its three trainers), the
        // base member's values.
        let base_offsets = g4_formats::data::trainer_text::TrainerTextOffsets::read(
            &base
                .member(AssetKind::TrainerTextOffsets, 0)
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .offsets;
        assert_eq!(base_offsets.len(), 2);
        assert_eq!(
            st.trainer_text_offsets(0).unwrap().offsets,
            base_offsets,
            "text project: {text_project}"
        );
        assert_eq!(refused(st.undo()), irreversible());
        st.save().unwrap();

        // The number is free: the next trainer takes it, also in a store opened anew.
        let again = create_trainer_in(&project, &mut st, &mut names, &mut battle, "Mia");
        assert_eq!(again, id, "text project: {text_project}");
        st.save().unwrap();
        let mut fresh = Store::new(project.clone());
        assert_eq!(fresh.member_count(AssetKind::Trainer).unwrap(), 4);
        assert_eq!(fresh.trainer_text_offsets(0).unwrap().offsets.len(), 4);
    }
}

/// Copies the side of the text project Git tracks (`data/`, `expanded/`) to `to`.
fn snapshot_tracked(fx: &TextFixture, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for dir in ["data", "expanded"] {
        g4_kernel::test_support::clone_tree(&fx.root().join(dir), &to.join(dir));
    }
}

/// `git switch` as the build sees it: the tracked side as in `from`, where it differs (a
/// file with the same bytes keeps its `stat`, as Git leaves it), the members (not tracked,
/// `unpacked/`) and the build's state stay.
fn switch_tracked(fx: &TextFixture, from: &Path) {
    fn files(dir: &Path, rel: &Path, out: &mut BTreeSet<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir.join(rel)) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap();
            let rel = rel.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                files(dir, &rel, out);
            } else {
                out.insert(rel);
            }
        }
    }
    for dir in ["data", "expanded"] {
        let (mut now, mut then) = (BTreeSet::new(), BTreeSet::new());
        files(fx.root(), Path::new(dir), &mut now);
        files(from, Path::new(dir), &mut then);
        for gone in now.difference(&then) {
            std::fs::remove_file(fx.root().join(gone)).unwrap();
        }
        for rel in &then {
            let bytes = std::fs::read(from.join(rel)).unwrap();
            let to = fx.root().join(rel);
            if std::fs::read(&to).ok().as_deref() != Some(bytes.as_slice()) {
                std::fs::create_dir_all(to.parent().unwrap()).unwrap();
                std::fs::write(&to, bytes).unwrap();
            }
        }
    }
}

/// The step `textproject`'s problems of a build's preparation that succeeded.
fn step_problems(prepared: &g4_kernel::build::Prepared) -> Vec<String> {
    prepared
        .problems
        .iter()
        .filter(|p| p.message.starts_with("textproject."))
        .map(|p| p.message.clone())
        .collect()
}

#[test]
fn freeing_a_trainer_puts_the_battle_text_offsets_back_to_the_base() {
    // The fixture's `trtblofs` has fewer offsets than there are trainers, as HeartGold's (735
    // offsets for 738 trainers): `create_trainer` pads it up to the new trainer.
    let fx = TextFixture::exported();
    let project = Arc::new(fx.project());
    let mut st = Store::new(project.clone());
    let base_offsets = st.trainer_text_offsets(0).unwrap().offsets.clone();
    assert!(base_offsets.len() < 3, "{base_offsets:?}");
    let mut names = g4_kernel::data::names::NameTables::new();
    let mut battle = None;
    let id = create_trainer_in(&project, &mut st, &mut names, &mut battle, "Jens");
    assert_eq!(id, 3);
    st.save().unwrap();
    let main = tempfile::tempdir().unwrap();
    snapshot_tracked(&fx, main.path());

    // Another branch: freed. The offsets are the base's again, not padded.
    let base = ProjectOriginal::new(fx.root());
    let mut refs = RefIndex::new();
    let target = DeleteTarget::Trainer { index: id };
    delete(&mut st, &mut refs, &base, target).unwrap();
    free_number(&mut st, &mut refs, &base, target).unwrap();
    assert_eq!(st.trainer_text_offsets(0).unwrap().offsets, base_offsets);
    st.save().unwrap();
    drop(st);
    let offsets_member = project.member_path(AssetKind::TrainerTextOffsets, 0);
    let base_member = base
        .member(AssetKind::TrainerTextOffsets, 0)
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read(&offsets_member).unwrap(), base_member);

    // Back on the branch with the trainer, the first build (a full run, judged by the base)
    // appends its members and takes the offsets from the JSON.
    switch_tracked(&fx, main.path());
    let prepared = build::prepare(&fx.project()).unwrap();
    assert!(
        step_problems(&prepared).is_empty(),
        "{:?}",
        prepared.problems
    );
    for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
        assert!(project.member_path(kind, id).is_file(), "{kind:?}");
    }
    // The member holds the JSON's four offsets (two bytes each).
    assert_eq!(std::fs::read(&offsets_member).unwrap().len(), 8);
}

#[test]
fn a_number_freed_and_taken_again_builds_after_a_switch_back() {
    let fx = TextFixture::exported();
    let project = Arc::new(fx.project());
    let mut st = Store::new(project.clone());
    let mut names = g4_kernel::data::names::NameTables::new();
    let mut battle = None;
    let base = ProjectOriginal::new(fx.root());
    let mut refs = RefIndex::new();

    // Trainer 3 built (the step's entry knows its files), freed, and created anew.
    let id = create_trainer_in(&project, &mut st, &mut names, &mut battle, "Gate");
    st.save().unwrap();
    build::prepare(&fx.project()).unwrap();
    let target = DeleteTarget::Trainer { index: id };
    delete(&mut st, &mut refs, &base, target).unwrap();
    free_number(&mut st, &mut refs, &base, target).unwrap();
    st.save().unwrap();
    let again = create_trainer_in(&project, &mut st, &mut names, &mut battle, "Zweig");
    assert_eq!(again, id);
    st.save().unwrap();
    let main = tempfile::tempdir().unwrap();
    snapshot_tracked(&fx, main.path());

    // Another branch frees it again; back on the first, the build appends it as new.
    delete(&mut st, &mut refs, &base, target).unwrap();
    free_number(&mut st, &mut refs, &base, target).unwrap();
    st.save().unwrap();
    drop(st);
    switch_tracked(&fx, main.path());
    let prepared = build::prepare(&fx.project()).unwrap();
    assert!(
        step_problems(&prepared).is_empty(),
        "{:?}",
        prepared.problems
    );
    for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
        assert!(project.member_path(kind, id).is_file(), "{kind:?}");
    }
    // The offsets come from the JSON too, not kept short as „newer“ than it.
    let offsets_member = project.member_path(AssetKind::TrainerTextOffsets, 0);
    assert_eq!(std::fs::read(&offsets_member).unwrap().len(), 8);
}

#[test]
fn delete_of_a_flag_name_waits_while_the_store_is_busy() {
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
    let register = fx.read(FLAG_REGISTER);
    st.set_busy(Some("Build läuft".into()));
    let result = delete(
        &mut st,
        &mut RefIndex::new(),
        &base,
        DeleteTarget::Flag { number: 148 },
    );
    assert!(matches!(result, Err(KernelError::Busy(_))), "{result:?}");
    assert_eq!(fx.read(FLAG_REGISTER), register);
    assert!(exists(&fx, FLAG_FILE));
}

// --- delete_warp: incoming warps shift, a warp or a script by index locks (task 6) ---

use g4_kernel::delete::warps::{WarpDelete, delete_warp};

const EVENT_1: AssetId = AssetId {
    kind: AssetKind::Event,
    index: 1,
};

fn warp(x: u16, dest_header_id: u16, dest_warp_id: u16) -> WarpEvent {
    WarpEvent {
        x,
        z: 0,
        dest_header_id,
        dest_warp_id,
        height: 0,
    }
}

/// The fixture's two headers on their own event files (header 0 → event file 0, header 1 →
/// event file 1), event file 0 with three doors at x 10, 11 and 12 (each leading to door 0 of
/// header 1) and event file 1 with one warp per `incoming` (destination header, door), saved.
fn doors(incoming: &[(u16, u16)]) -> (TextFixture, Store) {
    let (fx, mut st) = fixture();
    st.edit_header(1, "Eventdatei", |h| {
        if let MapHeader::Pt(h) = h {
            h.event_file_id = 1;
        }
        Ok(())
    })
    .unwrap();
    st.edit_event(0, "Türen", |e| {
        e.warp_events = vec![warp(10, 1, 0), warp(11, 1, 0), warp(12, 1, 0)];
        Ok(())
    })
    .unwrap();
    st.edit_event(1, "Eingänge", |e| {
        e.warp_events = incoming.iter().map(|&(h, w)| warp(0, h, w)).collect();
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    (fx, st)
}

/// The destinations (header, door) of the warps of event file `file`.
fn destinations(st: &mut Store, file: u16) -> Vec<(u16, u16)> {
    st.event(file)
        .unwrap()
        .warp_events
        .iter()
        .map(|w| (w.dest_header_id, w.dest_warp_id))
        .collect()
}

fn referenced<T: std::fmt::Debug>(result: Result<T, KernelError>) -> (String, Vec<Ref>) {
    match result {
        Err(KernelError::Referenced {
            label, references, ..
        }) => (label, references),
        other => panic!("expected Referenced, got {other:?}"),
    }
}

#[test]
fn deleting_a_door_nobody_targets_shifts_later_doors() {
    let (_fx, mut st) = doors(&[(0, 1), (0, 2)]);
    let file0 = bytes(&st, EVENT_0);
    let file1 = bytes(&st, EVENT_1);
    let depth = st.undo_depth();

    let result: WarpDelete = delete_warp(&mut st, &mut RefIndex::new(), 0, 0).unwrap();
    assert!(result.change.is_some());
    assert_eq!(result.shifted, [(1, 0), (1, 1)]);
    assert_eq!(destinations(&mut st, 0).len(), 2);
    // The doors the warps of event file 1 led to moved up by one: they are 0 and 1 now.
    assert_eq!(destinations(&mut st, 1), [(0, 0), (0, 1)]);
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(last_label(&st), "delete warp");

    // ⌘Z restores both files.
    st.undo().unwrap();
    assert_eq!(bytes(&st, EVENT_0), file0);
    assert_eq!(bytes(&st, EVENT_1), file1);
    assert_eq!(destinations(&mut st, 1), [(0, 1), (0, 2)]);
}

/// The bytes of loaded asset `id` as the store holds it now.
fn bytes(st: &Store, id: AssetId) -> Vec<u8> {
    st.current_bytes(id).unwrap().unwrap()
}

#[test]
fn incoming_warps_on_later_doors_are_shifted() {
    // Event file 1: a warp to door 0 (before the deleted one), one to door 2 (after it) and one
    // to door 2 of header 1, which is not event file 0.
    let (_fx, mut st) = doors(&[(0, 0), (0, 2), (1, 2)]);
    let result = delete_warp(&mut st, &mut RefIndex::new(), 0, 1).unwrap();
    assert_eq!(result.shifted, [(1, 1)]);
    assert_eq!(destinations(&mut st, 1), [(0, 0), (0, 1), (1, 2)]);
    // The door that was 2 (at x 12) is door 1 now: the warp still leads through the same door.
    let doors: Vec<u16> = st
        .event(0)
        .unwrap()
        .warp_events
        .iter()
        .map(|w| w.x)
        .collect();
    assert_eq!(doors, [10, 12]);
}

#[test]
fn deleting_a_targeted_door_is_refused_with_the_list() {
    let (_fx, mut st) = doors(&[(0, 1), (0, 2)]);
    let depth = st.undo_depth();
    let (label, references) = referenced(delete_warp(&mut st, &mut RefIndex::new(), 0, 1));
    assert_eq!(label, "Event file 0000, door 1");
    assert_eq!(references.len(), 1, "{references:?}");
    assert_eq!(
        references[0].target,
        Target::Warp {
            header: 0,
            index: 1
        }
    );
    assert_eq!(
        (references[0].source, references[0].slot),
        (EVENT_1, Some(0))
    );
    // Nothing changed, no step.
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(destinations(&mut st, 0).len(), 3);
    assert_eq!(destinations(&mut st, 1), [(0, 1), (0, 2)]);
}

#[test]
fn a_later_door_of_the_same_file_leading_into_it_is_shifted() {
    // Door 2 of event file 0 leads to door 2 of header 0: its own event file.
    let (_fx, mut st) = doors(&[]);
    st.edit_event(0, "Tür", |e| {
        e.warp_events[2].dest_header_id = 0;
        e.warp_events[2].dest_warp_id = 2;
        Ok(())
    })
    .unwrap();
    let result = delete_warp(&mut st, &mut RefIndex::new(), 0, 1).unwrap();
    // It moved up to slot 1 and still leads to itself, now door 1.
    assert_eq!(result.shifted, [(0, 1)]);
    assert_eq!(destinations(&mut st, 0), [(1, 0), (0, 1)]);
    let doors: Vec<u16> = st
        .event(0)
        .unwrap()
        .warp_events
        .iter()
        .map(|w| w.x)
        .collect();
    assert_eq!(doors, [10, 12]);
}

#[test]
fn a_script_moving_a_later_warp_blocks_the_delete() {
    let (fx, mut st) = doors(&[]);
    let mut refs = RefIndex::new();
    // The index is built before the script is written: the delete stats the sources anew.
    refs.refs(&st, Target::Header(0)).unwrap();
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    SetWarpEventPos 2, 10, 12\n    End\nscript Second #2:\n    End\n",
    );
    let (label, references) = referenced(delete_warp(&mut st, &mut refs, 0, 0));
    assert_eq!(label, "Event file 0000, door 0");
    let found: Vec<(AssetKind, &str, Target)> = references
        .iter()
        .map(|r| (r.source.kind, r.field, r.target))
        .collect();
    assert_eq!(
        found,
        [(AssetKind::Script, "SetWarpEventPos", Target::WarpIndex(2))]
    );
    assert_eq!(destinations(&mut st, 0).len(), 3);
}

#[test]
fn a_script_moving_an_earlier_warp_does_not_block() {
    let (fx, mut st) = doors(&[]);
    fx.write(
        SOURCE_0,
        b"script Main #1:\n    SetWarpEventPos 0, 10, 12\n    End\nscript Second #2:\n    End\n",
    );
    let result = delete_warp(&mut st, &mut RefIndex::new(), 0, 1).unwrap();
    assert!(result.change.is_some());
    assert!(result.shifted.is_empty());
    assert_eq!(destinations(&mut st, 0).len(), 2);
}

// --- the commands' helpers (task 8) ---

/// `header` with its event file set to `file`.
fn with_event_file(header: &MapHeader, file: u16) -> MapHeader {
    let mut header = header.clone();
    match &mut header {
        MapHeader::DP(h) => h.event_file_id = file,
        MapHeader::Pt(h) => h.event_file_id = file,
        MapHeader::HGSS(h) => h.event_file_id = file,
    }
    header
}

#[test]
fn guard_header_refuses_only_a_new_reference_to_a_tombstone() {
    let (_fx, mut st) = fixture();
    let before = st.header_table().unwrap()[0].clone();
    let other = if before.event_file_id() == 0 { 1 } else { 0 };
    let after = with_event_file(&before, other);
    assert!(guard_header(&st, &before, &after).is_ok());
    tombstone(&mut st, DeleteTarget::EventFile { index: other });
    match guard_header(&st, &before, &after) {
        Err(KernelError::Refused(why)) => {
            assert_eq!(why, format!("Event file {other:04} is deleted."));
        }
        other => panic!("{other:?}"),
    }
    // A field left as it was is never refused, also when its entity is a tombstone now.
    tombstone(
        &mut st,
        DeleteTarget::EventFile {
            index: before.event_file_id(),
        },
    );
    assert!(guard_header(&st, &before, &before).is_ok());
}

#[test]
fn deleted_in_follows_an_envelope_changed_outside() {
    let (fx, mut st) = fixture();
    tombstone(&mut st, DeleteTarget::Trainer { index: 1 });
    st.save().unwrap();
    const JSON: &str = "data/trainers/0001.json";
    let with = fx.read(JSON);
    // Asked twice, the same answer; the second comes from the store's cache (a file whose
    // `stat` did not change is not read again), which only the unit test
    // `read_deleted_cached_reads_a_file_again_only_when_its_stat_changed` (tombstone.rs) can
    // see. From here on the cache is warm: every change below happens behind it.
    assert_eq!(st.deleted_in(Art::Trainers), BTreeSet::from([1]));
    assert_eq!(st.deleted_in(Art::Trainers), BTreeSet::from([1]));

    // Outside the app (`git switch`): the envelope goes, comes back, and another file gets one.
    let end = b",\n  \"deleted\": true\n}\n";
    assert!(with.ends_with(end));
    let without = [&with[..with.len() - end.len()], b"\n}\n"].concat();
    fx.write(JSON, &without);
    assert!(st.deleted_in(Art::Trainers).is_empty());
    fx.write(JSON, &with);
    assert_eq!(st.deleted_in(Art::Trainers), BTreeSet::from([1]));
    let other = fx.read("data/trainers/0002.json");
    let other_with = [&other[..other.len() - 3], end.as_slice()].concat();
    fx.write("data/trainers/0002.json", &other_with);
    assert_eq!(st.deleted_in(Art::Trainers), BTreeSet::from([1, 2]));
    // A file gone is no longer deleted.
    std::fs::remove_file(fx.root().join(JSON)).unwrap();
    assert_eq!(st.deleted_in(Art::Trainers), BTreeSet::from([2]));
}

#[test]
fn deleted_in_reads_the_envelopes_with_the_own_flags_over_them() {
    let (fx, mut st) = fixture();
    assert!(st.deleted_in(Art::Trainers).is_empty());
    tombstone(&mut st, DeleteTarget::Trainer { index: 1 });
    st.save().unwrap();
    // A fresh store reads the envelope from disk.
    let mut fresh = Store::new(Arc::new(fx.project()));
    assert_eq!(fresh.deleted_in(Art::Trainers), BTreeSet::from([1]));
    tombstone(&mut fresh, DeleteTarget::Trainer { index: 2 });
    fresh
        .transaction("Wiederhergestellt", |tx| {
            tx.set_deleted(DeleteTarget::Trainer { index: 1 }, false)
        })
        .unwrap();
    assert_eq!(fresh.deleted_in(Art::Trainers), BTreeSet::from([2]));
    assert!(fresh.deleted_in(Art::Encounters).is_empty());
    // Outside a text project nothing is deleted.
    let dspre = TextFixture::dspre();
    let st = Store::new(Arc::new(dspre.project()));
    assert!(st.deleted_in(Art::Trainers).is_empty());
}

// --- stories as weak references (3d.6 task 10) ---

use g4_kernel::stories::{
    DANGLING_REF_CODE, Progress, Step, StepKind, StoryRequest, StoryUse, Use, create_story_files,
};

/// Story „Testfaden“ (number 0) whose step `s1` battles trainer 1 (`TRAINER_YOUNGSTER_KAI`).
fn story_naming_trainer_1(fx: &TextFixture) {
    let req = StoryRequest {
        name: "Testfaden".to_owned(),
        progress: Progress::Var("VAR_UNUSED_0x406C".to_owned()),
        notes: String::new(),
        steps: vec![Step {
            key: "s1".to_owned(),
            value: 1,
            kind: StepKind::Battle,
            name: "Kampf gegen Kai".to_owned(),
            notes: String::new(),
            made: None,
            uses: vec![Use::Trainer("TRAINER_YOUNGSTER_KAI".to_owned())],
        }],
        flags: Vec::new(),
    };
    create_story_files(&fx.project(), &req).unwrap();
}

fn testfaden_s1() -> StoryUse {
    StoryUse {
        story_id: "STORY_TESTFADEN".to_owned(),
        story_name: "Testfaden".to_owned(),
        number: 0,
        step_key: Some("s1".to_owned()),
        step_name: Some("Kampf gegen Kai".to_owned()),
        file: PathBuf::from("data/stories/0000.json"),
    }
}

#[test]
fn deleting_a_trainer_a_story_mentions_warns_and_does_not_lock() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    story_naming_trainer_1(&fx);
    let target = DeleteTarget::Trainer { index: 1 };

    // The plan allows it and lists the story beside the (empty) references.
    let p = plan(&mut st, &base, target);
    assert_eq!(p.refusal, None);
    assert!(p.references.is_empty() && p.warnings.is_empty(), "{p:?}");
    assert_eq!(p.mentions, [testfaden_s1()]);
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["mentions"][0]["story_id"], "STORY_TESTFADEN");
    assert_eq!(json["mentions"][0]["step_key"], "s1");
    assert_eq!(json["mentions"][0]["file"], "data/stories/0000.json");
    // Without a story naming it, the plan's list is empty (and still there).
    let other = plan(&mut st, &base, DeleteTarget::Trainer { index: 2 });
    assert!(other.mentions.is_empty());
    assert_eq!(
        serde_json::to_value(&other).unwrap()["mentions"],
        serde_json::json!([])
    );

    // The delete sets the tombstone as one step and answers the same list.
    let depth = st.undo_depth();
    let result = delete(&mut st, &mut RefIndex::new(), &base, target).unwrap();
    assert!(st.is_deleted(target));
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(result.mentions, [testfaden_s1()]);
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["mentions"][0]["step_name"], "Kampf gegen Kai");

    // Saved, the check reports the story's use as open.
    st.save().unwrap();
    let report = check(&fx.project()).unwrap();
    let open: Vec<&str> = report
        .problems
        .iter()
        .filter(|p| p.message.starts_with(DANGLING_REF_CODE))
        .map(|p| p.message.as_str())
        .collect();
    assert_eq!(
        open,
        ["story.dangling-ref: steps[0].uses[0].trainer: Trainer 0001 is deleted."]
    );

    // ⌘Z takes the tombstone back; restoring answers no mentions.
    st.undo().unwrap();
    assert!(!st.is_deleted(target));
    let json = serde_json::to_value(delete(&mut st, &mut RefIndex::new(), &base, target).unwrap())
        .unwrap();
    assert!(json.get("mentions").is_some());
    let restored = restore(&mut st, target).unwrap();
    assert!(restored.mentions.is_empty());
    assert!(
        serde_json::to_value(&restored)
            .unwrap()
            .get("mentions")
            .is_none()
    );
}

#[test]
fn a_hard_reference_still_locks_with_the_story_beside_it() {
    let (fx, mut st) = fixture();
    let base = ProjectOriginal::new(fx.root());
    story_naming_trainer_1(&fx);
    place_trainer(&mut st, 1);
    let target = DeleteTarget::Trainer { index: 1 };

    let p = plan(&mut st, &base, target);
    assert_eq!(p.references.len(), 1, "{:?}", p.references);
    assert_eq!(p.mentions, [testfaden_s1()]);

    // The figure locks it; the story changes nothing about that.
    let depth = st.undo_depth();
    match delete(&mut st, &mut RefIndex::new(), &base, target) {
        Err(KernelError::Referenced {
            label, references, ..
        }) => {
            assert_eq!(label, "Trainer 0001");
            assert_eq!(references.len(), 1, "{references:?}");
        }
        other => panic!("expected Referenced, got {other:?}"),
    }
    assert_eq!(st.undo_depth(), depth);
    assert!(!st.is_deleted(target));
}
