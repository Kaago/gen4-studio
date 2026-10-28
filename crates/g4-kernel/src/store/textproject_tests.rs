//! The store in a text project: assets load from their entity JSON and save to it, the members
//! follow as the build's intermediate state (phase 4.3b, task 3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use super::*;
use crate::build::state::{BuildState, SyncRecord};
use crate::data::names::NameTables;
use crate::data::trainer_create::create_trainer_with;
use crate::project::SourceMode;
use crate::scripts::{self, ScriptSource, SourceKind, TrainerScriptMeta};
use crate::test_support::TextFixture;
use crate::textproject::source::EntitySource;
use crate::textproject::{ExportOptions, export};

const MOVE_1: AssetId = AssetId {
    kind: AssetKind::Move,
    index: 1,
};
const MOVE_1_JSON: &str = "data/moves/0001.json";
const MOVE_1_MEMBER: &str = "unpacked/moveData/0001";

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

/// Every file below `rel` (a folder or a file), relative to the root, with its bytes.
fn files(fx: &TextFixture, rels: &[&str]) -> BTreeMap<PathBuf, Vec<u8>> {
    fn go(root: &Path, rel: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let path = root.join(rel);
        if path.is_file() {
            out.insert(rel.to_path_buf(), std::fs::read(&path).unwrap());
            return;
        }
        let Ok(entries) = std::fs::read_dir(&path) else {
            return;
        };
        for entry in entries {
            go(root, &rel.join(entry.unwrap().file_name()), out);
        }
    }
    let mut out = BTreeMap::new();
    for rel in rels {
        go(fx.root(), Path::new(rel), &mut out);
    }
    out
}

/// The files whose bytes differ between `a` and `b` (or that only one has).
fn changed(a: &BTreeMap<PathBuf, Vec<u8>>, b: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = a
        .keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

fn json(fx: &TextFixture, rel: &str) -> Value {
    serde_json::from_slice(&fx.read(rel)).unwrap()
}

/// `power` of move 1 in its JSON.
fn power(fx: &TextFixture) -> Value {
    json(fx, MOVE_1_JSON)["data"]["power"].clone()
}

fn full_export(fx: &TextFixture) {
    export(
        &fx.project(),
        &fx.base(),
        &ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
}

fn set_power(st: &mut Store, power: u8) {
    st.edit_move(1, "Stärke", |m| m.set_power(power))
        .unwrap()
        .unwrap();
}

#[test]
fn without_a_manifest_locations_are_as_before() {
    let fx = TextFixture::dspre();
    let p = fx.project();
    assert_eq!(*p.source_mode(), SourceMode::Dspre);
    p.editable().unwrap();
    assert_eq!(
        p.asset_location(AssetKind::Move, 1).unwrap(),
        Location::Member(p.member_path(AssetKind::Move, 1))
    );
    assert_eq!(
        p.asset_location(AssetKind::Trainer, 1).unwrap(),
        Location::MemberPair {
            props: p.member_path(AssetKind::TrainerProperties, 1),
            party: p.member_path(AssetKind::TrainerParty, 1),
        }
    );
    assert!(matches!(
        p.asset_location(AssetKind::HeaderTable, 0).unwrap(),
        Location::Slice { .. }
    ));
    // The store reads and writes the members, nothing under `data/`.
    let mut st = store(&fx);
    set_power(&mut st, 99);
    st.save().unwrap();
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 99);
    assert!(!fx.root().join("data").exists());
}

#[test]
fn in_a_text_project_assets_load_from_their_json() {
    let fx = TextFixture::exported();
    let p = fx.project();
    assert_eq!(*p.source_mode(), SourceMode::TextProject);
    p.editable().unwrap();
    match p.asset_location(AssetKind::Move, 1).unwrap() {
        Location::Entity { unit, members } => {
            assert_eq!(unit.key, MOVE_1_JSON);
            assert_eq!(unit.assets, vec![MOVE_1]);
            assert_eq!(
                *members,
                Location::Member(p.member_path(AssetKind::Move, 1))
            );
        }
        other => panic!("{other:?}"),
    }
    match p.asset_location(AssetKind::HeaderTable, 0).unwrap() {
        Location::Entity { unit, members } => {
            assert_eq!(unit.key, "data/headers/");
            assert!(matches!(*members, Location::Slice { .. }), "{members:?}");
        }
        other => panic!("{other:?}"),
    }
    // Text archives keep their JSON and member.
    assert!(matches!(
        p.asset_location(AssetKind::TextArchive, 433).unwrap(),
        Location::TextSource { .. }
    ));

    let member = fx.read(MOVE_1_MEMBER);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 99");
    let mut st = store(&fx);
    assert_eq!(st.move_data(1).unwrap().power(), 99);
    assert!(!st.is_dirty(MOVE_1));
    assert_eq!(fx.read(MOVE_1_MEMBER), member, "loading writes nothing");
}

#[test]
fn save_writes_json_then_member_and_records_both() {
    let fx = TextFixture::exported();
    // A build records the step `textproject` with every member in sync.
    crate::build::prepare(&fx.project()).unwrap();
    // `record_all` (task 4) stands for the recorded sync; a save adds to it.
    BuildState::update(fx.root(), |state| {
        state.sync = Some(SyncRecord::default());
    })
    .unwrap();
    let mut st = store(&fx);
    set_power(&mut st, 99);
    let before = files(&fx, &["data", "unpacked"]);
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![MOVE_1]);
    assert_eq!(
        changed(&before, &files(&fx, &["data", "unpacked"])),
        paths(&[MOVE_1_JSON, MOVE_1_MEMBER])
    );
    assert_eq!(power(&fx), 99);
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 99);
    assert!(!st.is_dirty(MOVE_1));

    let sync = BuildState::load(fx.root()).sync.unwrap();
    for rel in [MOVE_1_JSON, MOVE_1_MEMBER] {
        let fp = sync
            .files
            .get(rel)
            .unwrap_or_else(|| panic!("{rel}: {sync:?}"));
        assert_eq!(
            fp.xxh3,
            Some(xxhash_rust::xxh3::xxh3_64(&fx.read(rel))),
            "{rel}"
        );
    }

    // The build step finds JSON and member alike: nothing to write, nothing newer.
    let member = fx.read(MOVE_1_MEMBER);
    let prepared = crate::build::prepare(&fx.project()).unwrap();
    let row = &prepared.steps[0];
    assert_eq!((row.id.as_str(), row.done), ("textproject", 0), "{row:?}");
    let own: Vec<_> = prepared
        .problems
        .iter()
        .filter(|p| p.message.starts_with("textproject."))
        .collect();
    assert!(own.is_empty(), "{own:?}");
    assert_eq!(fx.read(MOVE_1_MEMBER), member);
}

#[test]
fn save_detects_a_json_changed_on_disk_as_stale() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    set_power(&mut st, 99);
    fx.edit(MOVE_1_JSON, "\"power\": 10", "\"power\": 50");
    let before = files(&fx, &["data", "unpacked"]);
    match st.save() {
        Err(KernelError::Stale(id)) => assert_eq!(id, MOVE_1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        changed(&before, &files(&fx, &["data", "unpacked"])),
        paths(&[])
    );
    // Taking the file as the base keeps the edit; the next save writes it.
    st.accept_disk_as_base(MOVE_1).unwrap();
    assert!(st.is_dirty(MOVE_1));
    st.save().unwrap();
    assert_eq!(power(&fx), 99);
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 99);
}

#[test]
fn loading_a_broken_entity_json_names_file_line_and_column() {
    let fx = TextFixture::exported();
    fx.edit(MOVE_1_JSON, "\"power\"", "\"powr\"");
    let mut st = store(&fx);
    match st.move_data(1) {
        Err(KernelError::Compile(problems)) => {
            assert_eq!(problems.len(), 1, "{problems:?}");
            let p = &problems[0];
            assert_eq!(p.file, Path::new(MOVE_1_JSON));
            assert!(p.line > 1 && p.col > 1, "{p:?}");
            assert!(p.message.starts_with("textproject.schema: "), "{p:?}");
            assert!(p.message.contains("powr"), "{p:?}");
        }
        Err(e) => panic!("{e}"),
        Ok(_) => panic!("loaded"),
    }
    assert!(!st.is_loaded(MOVE_1));
}

#[test]
fn an_appended_matrix_gets_its_json_at_once() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    let index = st.create_matrix("NEU").unwrap();
    let rel = format!("data/matrices/{index:04}.json");
    assert_eq!(json(&fx, &rel)["name"], "NEU");
    let p = fx.project();
    let id = AssetId {
        kind: AssetKind::Matrix,
        index,
    };
    let mut source = EntitySource::open(&p).unwrap();
    assert_eq!(
        source.read(&p, id).unwrap(),
        Some(std::fs::read(p.member_path(AssetKind::Matrix, index)).unwrap())
    );
    // A duplicated map gets its JSON as well.
    let map = st.duplicate_map(0).unwrap();
    assert!(fx.root().join(format!("data/maps/{map:04}.json")).is_file());
    // What a full export writes.
    let after = files(&fx, &["data"]);
    full_export(&fx);
    assert_eq!(changed(&after, &files(&fx, &["data"])), paths(&[]));
}

/// A battle script like Platinum's file 1114, for three trainers (slots 1 to 3), slot 4 the
/// sight script (as in the tests of `trainer_create`).
const BATTLE: &str = "script script_1 #[1-3]:\n    LockAll\n    FacePlayer\n    ReleaseAll\n    \
                      End\nscript script_4 #4:\n    LockAll\n    ReleaseAll\n    End\n";

#[test]
fn a_new_trainer_gets_its_json_and_register_at_once() {
    let fx = TextFixture::exported();
    let project = Arc::new(fx.project());
    let battle_file = scripts::new_script_file(&project, SourceKind::Script).unwrap();
    scripts::write_source(&project, battle_file, &ScriptSource::Rotom(BATTLE.into())).unwrap();
    let compiled = scripts::compile(&project).unwrap();
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    let mut st = Store::new(Arc::clone(&project));
    let meta = TrainerScriptMeta {
        battle_file,
        seen_script: 3003,
    };
    let report = create_trainer_with(
        &project,
        &mut st,
        &mut NameTables::new(),
        &meta,
        1040,
        1,
        "Lea",
    )
    .unwrap();
    assert_eq!(report.id, 3);
    // The name is still unsaved in its text archive, yet the id comes from class and name.
    let rel = "data/trainers/0003.json";
    assert_eq!(json(&fx, rel)["id"], "TRAINER_YOUNGSTER_LEA");
    assert_eq!(
        json(&fx, "data/ids/trainers.json")["TRAINER_YOUNGSTER_LEA"],
        3
    );
    let trainer = AssetId {
        kind: AssetKind::Trainer,
        index: 3,
    };
    let mut source = EntitySource::open(&project).unwrap();
    assert_eq!(
        source.read(&project, trainer).unwrap(),
        Some(trainer_pair_bytes(
            &std::fs::read(project.member_path(AssetKind::TrainerProperties, 3)).unwrap(),
            &std::fs::read(project.member_path(AssetKind::TrainerParty, 3)).unwrap(),
        ))
    );
    // Saving the name and the offset keeps the id; a full export writes the same files.
    st.save().unwrap();
    let after = files(&fx, &["data"]);
    full_export(&fx);
    assert_eq!(changed(&after, &files(&fx, &["data"])), paths(&[]));
}

#[test]
fn older_and_newer_schemas_refuse_edits_with_the_manifest_sentence() {
    let fx = TextFixture::exported();
    let refused = |fx: &TextFixture, expected: SourceMode, words: &str| {
        let p = fx.project();
        assert_eq!(*p.source_mode(), expected);
        let why = match p.editable() {
            Err(KernelError::Refused(why)) => why,
            other => panic!("{other:?}"),
        };
        assert!(why.contains(words), "{why}");
        let mut st = store(fx);
        // Reading goes, from the members.
        assert_eq!(st.move_data(1).unwrap().power(), 10);
        let before = files(fx, &["data", "unpacked"]);
        match st.edit_move(1, "Stärke", |m| m.set_power(99)) {
            Err(KernelError::Refused(text)) => assert_eq!(text, why),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            st.create_matrix("NEU"),
            Err(KernelError::Refused(_))
        ));
        assert!(matches!(
            st.create_event_file(),
            Err(KernelError::Refused(_))
        ));
        assert!(!st.is_dirty(MOVE_1));
        assert_eq!(
            changed(&before, &files(fx, &["data", "unpacked"])),
            paths(&[])
        );
    };
    fx.edit(
        "g4-project.json",
        "\"schemaVersion\": 1",
        "\"schemaVersion\": 0",
    );
    refused(&fx, SourceMode::Older { schema: 0 }, "g4k text migrate");
    fx.edit(
        "g4-project.json",
        "\"schemaVersion\": 0",
        "\"schemaVersion\": 99",
    );
    refused(&fx, SourceMode::Newer { schema: 99 }, "neueren Fassung");
    fx.write("g4-project.json", b"{\n  \"format\": \n");
    let p = fx.project();
    let SourceMode::Broken { reason } = p.source_mode().clone() else {
        panic!("{:?}", p.source_mode());
    };
    assert!(reason.starts_with("g4-project.json:"), "{reason}");
    refused(&fx, SourceMode::Broken { reason }, "g4-project.json:");
}

#[test]
fn undo_and_redo_in_a_text_project_write_nothing_until_save() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    let before = files(&fx, &["data", "unpacked"]);
    set_power(&mut st, 99);
    st.undo().unwrap();
    assert_eq!(st.move_data(1).unwrap().power(), 10);
    st.redo().unwrap();
    assert_eq!(st.move_data(1).unwrap().power(), 99);
    assert_eq!(
        changed(&before, &files(&fx, &["data", "unpacked"])),
        paths(&[])
    );
    st.save().unwrap();
    assert_eq!(power(&fx), 99);
    st.undo().unwrap();
    assert_eq!(power(&fx), 99, "undo writes nothing");
    st.save().unwrap();
    assert_eq!(power(&fx), 10);
    assert_eq!(fx.read(MOVE_1_MEMBER)[3], 10);
}

const PERSONAL_1: AssetId = AssetId {
    kind: AssetKind::Personal,
    index: 1,
};
const SPECIES_1_JSON: &str = "data/species/0001.json";

fn raise_attack(st: &mut Store) {
    st.edit_personal(1, "Angriff", |p| p.set_attack(p.attack().wrapping_add(1)))
        .unwrap()
        .unwrap();
}

/// The personal member of species 1, relative to the root.
fn personal_1_member(fx: &TextFixture) -> String {
    let p = fx.project();
    p.member_path(AssetKind::Personal, 1)
        .strip_prefix(&p.root)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn seed_sync(fx: &TextFixture) {
    BuildState::update(fx.root(), |state| {
        state.sync = Some(SyncRecord::default());
    })
    .unwrap();
}

#[test]
fn a_clean_part_of_the_unit_changed_on_disk_makes_the_save_stale() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    // The learnset is loaded and clean; its part of the species JSON changes by hand.
    st.learnset(1).unwrap();
    raise_attack(&mut st);
    let mut species = json(&fx, SPECIES_1_JSON);
    let level = &mut species["learnset"]["moves"][0]["level"];
    assert!(level.is_u64(), "{species}");
    *level = Value::from(level.as_u64().unwrap() + 1);
    fx.write(
        SPECIES_1_JSON,
        &serde_json::to_vec_pretty(&species).unwrap(),
    );
    let before = files(&fx, &["data", "unpacked"]);
    match st.save() {
        Err(KernelError::Stale(id)) => assert_eq!(
            id,
            AssetId {
                kind: AssetKind::Learnset,
                index: 1
            }
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        changed(&before, &files(&fx, &["data", "unpacked"])),
        paths(&[])
    );
    assert!(st.is_dirty(PERSONAL_1));
}

#[test]
fn a_save_records_only_what_it_wrote() {
    let fx = TextFixture::exported();
    seed_sync(&fx);
    let mut st = store(&fx);
    raise_attack(&mut st);
    st.save().unwrap();
    let sync = BuildState::load(fx.root()).sync.unwrap();
    let keys: Vec<&str> = sync.files.keys().map(String::as_str).collect();
    let member = personal_1_member(&fx);
    let mut expected = vec![SPECIES_1_JSON, member.as_str()];
    expected.sort_unstable();
    assert_eq!(keys, expected, "not the learnset or evolution member");
}

#[test]
fn a_member_changed_outside_since_the_sync_makes_the_save_stale() {
    let fx = TextFixture::exported();
    seed_sync(&fx);
    let member = personal_1_member(&fx);
    let mut st = store(&fx);
    raise_attack(&mut st);
    st.save().unwrap();
    // DSPRE writes the member afterwards.
    let mut dspre = fx.read(&member);
    dspre[2] ^= 0x01;
    fx.write(&member, &dspre);
    raise_attack(&mut st);
    let before = files(&fx, &["data", "unpacked"]);
    match st.save() {
        Err(KernelError::Stale(id)) => assert_eq!(id, PERSONAL_1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        changed(&before, &files(&fx, &["data", "unpacked"])),
        paths(&[])
    );

    // Without a recorded sync the member is the build's intermediate and simply written.
    BuildState::update(fx.root(), |state| state.sync = None).unwrap();
    st.save().unwrap();
    assert_ne!(fx.read(&member), dspre);
}

#[cfg(unix)]
#[test]
fn after_a_failed_member_write_the_saved_json_is_not_stale() {
    use std::os::unix::fs::PermissionsExt;

    let fx = TextFixture::exported();
    let member = personal_1_member(&fx);
    let mut st = store(&fx);
    raise_attack(&mut st);
    let attack = st.personal(1).unwrap().attack();
    // The member's folder takes no new file: the member cannot be written, only read.
    let folder = fx.root().join(&member).parent().unwrap().to_path_buf();
    let mode = |m: u32| std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(m));
    mode(0o555).unwrap();
    let failed = st.save();
    mode(0o755).unwrap();
    assert!(failed.is_err(), "{failed:?}");
    // The JSON was written first and is the asset's source now.
    assert_eq!(json(&fx, SPECIES_1_JSON)["personal"]["attack"], attack);
    assert!(!st.is_dirty(PERSONAL_1));
    assert!(st.save().unwrap().written.is_empty());
}

#[test]
fn a_busy_store_refuses_every_write_and_names_why() {
    let fx = TextFixture::dspre();
    let mut st = store(&fx);
    st.edit_event(0, "warp", |e| {
        e.warp_events.push(crate::store::WarpEvent {
            x: 1,
            z: 1,
            dest_header_id: 1,
            dest_warp_id: 0,
            height: 0,
        });
        Ok(())
    })
    .unwrap();
    st.save().unwrap();
    let depth = st.undo_depth();
    assert!(depth > 0);
    let why = "Das Projekt wird gerade als Textprojekt eingerichtet.";
    st.set_busy(Some(why.to_owned()));
    assert_eq!(st.busy(), Some(why));
    let busy = |r: Result<(), KernelError>| match r {
        Err(KernelError::Busy(text)) => assert_eq!(text, why),
        other => panic!("{other:?}"),
    };
    busy(st.edit_event(0, "x", |_| Ok(())).map(|_| ()));
    busy(st.undo().map(|_| ()));
    busy(st.save().map(|_| ()));
    busy(st.append_encounter(&[0; 424]).map(|_| ()));
    assert_eq!(st.undo_depth(), depth, "nothing recorded or undone");
    // Lifted: writes again.
    st.set_busy(None);
    assert_eq!(st.undo().unwrap().len(), 1);
}
