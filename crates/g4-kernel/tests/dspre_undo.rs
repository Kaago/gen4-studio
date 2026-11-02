//! Undo in a DSPRE project, where the text archives have no JSON until something writes one
//! (UI test 3d.5, U1): after a change, its undo and a save, the files under `expanded/`,
//! `unpacked/` and `.rotom/` are the ones before the change. A store save writes an archive's
//! JSON and member; saved again with the texts it had when loaded from the member alone, it
//! puts the member back and removes the JSON. A split (`dialog::split_message`) compiles the
//! script file, which writes the JSON from the member; the split removes it again, as
//! „Neue Figur“ does (`tests/recipes_erzaehlen.rs`
//! `a_saved_figure_in_a_dspre_project_is_undone_and_redone`).
//!
//! Synthetic: `test_support::TextFixture::dspre()` (header 0: script file 0, event file 0,
//! text archive 433 with three messages in its member and no JSON). Gated (`G4_TEST_PROJECT`,
//! Platinum DE or HeartGold DE): the UI test's flows on a copy, a message of the start town's
//! archive edited and the split the UI test found (Pt header 75 object 0 message 95:4, HG header
//! 22 object 12 message 351:7), each saved, undone and saved:
//!
//! ```sh
//! G4_TEST_PROJECT=".../_testkopien/3d-5-fix-pt_DSPRE_contents" \
//!   cargo test -p g4-kernel --all-features --test dspre_undo -- vanilla --nocapture
//! ```
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_formats::game::Family;
use g4_kernel::dialog::split_message;
use g4_kernel::project::Project;
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::store::{ObjectEvent, Store};

/// The text archive of header 0 of the fixture.
const ARCHIVE: u16 = 433;

/// Two figures (scripts 3 and 7) showing message 1 of the archive.
const SHARED: &str = "script drei #3:\n    Message 1\n    End\n\
                      script sieben #7:\n    Message 1\n    End\n";

fn object(local_id: u16, script: u16) -> ObjectEvent {
    ObjectEvent {
        local_id,
        graphics_id: 0,
        movement_type: 0,
        trainer_type: 0,
        hidden_flag: 0,
        script,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: local_id,
        z: 0,
        y: 0,
    }
}

/// Every file below `expanded/`, `unpacked/` and `.rotom/` of `root` with its bytes.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    for dir in ["expanded", "unpacked", ".rotom"] {
        walk(root, &root.join(dir), &mut out);
    }
    out
}

/// The paths whose bytes differ between `a` and `b` (or that only one has).
fn changed(a: &BTreeMap<PathBuf, Vec<u8>>, b: &BTreeMap<PathBuf, Vec<u8>>) -> BTreeSet<PathBuf> {
    a.keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}

fn json_of(archive: u16) -> PathBuf {
    Path::new("expanded/textArchives").join(format!("{archive:04}.json"))
}

fn member_of(archive: u16) -> PathBuf {
    Path::new("unpacked/textArchives").join(format!("{archive:04}"))
}

/// The DSPRE fixture with the scripts adopted, `source` as script file 0 and one object per
/// entry of `scripts` in event file 0 (saved): the store over it and the files as they are.
fn dspre(
    source: &str,
    scripts: &[u16],
) -> (common::TextFixture, Store, BTreeMap<PathBuf, Vec<u8>>) {
    let fx = common::TextFixture::dspre();
    let project = fx.project();
    scripts::adopt(&project, &mut |_| {}).unwrap();
    scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
    let mut store = Store::new(Arc::new(project));
    if !scripts.is_empty() {
        store
            .transaction("objects", |tx| {
                tx.edit_event(0, |e| {
                    e.object_events = (0u16..).zip(scripts).map(|(i, s)| object(i, *s)).collect();
                    Ok(())
                })
            })
            .unwrap();
        store.save().unwrap();
    }
    let json = fx.root().join(json_of(ARCHIVE));
    if json.exists() {
        // The compile above wrote it from the member: the project as DSPRE left it has none.
        std::fs::remove_file(&json).unwrap();
    }
    let before = files(fx.root());
    assert!(!before.contains_key(&json_of(ARCHIVE)));
    assert!(before.contains_key(&member_of(ARCHIVE)));
    (fx, store, before)
}

#[test]
fn an_edit_saved_then_undone_and_saved_leaves_the_files_as_before() {
    let (fx, mut store, before) = dspre("script main #1:\n    End\n", &[]);
    store
        .edit_text_archive(ARCHIVE, "Dialog geändert", |doc| {
            doc.messages[1].text = "Zweiblattdorf".into();
        })
        .unwrap();
    store.save().unwrap();
    // The save wrote the JSON and the member.
    assert_eq!(
        changed(&files(fx.root()), &before),
        BTreeSet::from([json_of(ARCHIVE), member_of(ARCHIVE)])
    );

    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(changed(&files(fx.root()), &before), BTreeSet::new());
    assert!(store.dirty().is_empty());

    // Redo and save write both again.
    store.redo().unwrap();
    store.save().unwrap();
    assert_eq!(
        changed(&files(fx.root()), &before),
        BTreeSet::from([json_of(ARCHIVE), member_of(ARCHIVE)])
    );
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages[1].text,
        "Zweiblattdorf"
    );
}

#[test]
fn a_split_leaves_no_archive_json_and_its_undo_the_files_as_before() {
    let (fx, mut store, before) = dspre(SHARED, &[3, 7]);
    let report = split_message(&mut store, 0, 0, 0, ARCHIVE, 1).unwrap();
    assert_eq!(report.new_index, 3);
    // The compile wrote the JSON from the member; the split removed it again.
    let after = files(fx.root());
    assert!(!after.contains_key(&json_of(ARCHIVE)));
    assert!(changed(&after, &before).contains(Path::new("expanded/scripts/0000.rotom")));

    store.undo().unwrap();
    assert_eq!(changed(&files(fx.root()), &before), BTreeSet::new());
    store.save().unwrap();
    assert_eq!(changed(&files(fx.root()), &before), BTreeSet::new());
}

#[test]
fn a_split_saved_then_undone_and_saved_leaves_the_files_as_before() {
    let (fx, mut store, before) = dspre(SHARED, &[3, 7]);
    split_message(&mut store, 0, 0, 0, ARCHIVE, 1).unwrap();
    store.save().unwrap();
    let saved = files(fx.root());
    let differ = changed(&saved, &before);
    assert!(differ.contains(&json_of(ARCHIVE)), "{differ:?}");
    assert!(differ.contains(&member_of(ARCHIVE)), "{differ:?}");

    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(changed(&files(fx.root()), &before), BTreeSet::new());

    store.redo().unwrap();
    store.save().unwrap();
    assert_eq!(changed(&files(fx.root()), &saved), BTreeSet::new());
}

#[test]
fn a_store_json_is_kept_where_the_texts_differ_from_the_member_loaded() {
    // Not the unexpanded member's texts: the JSON stays (the save writes both as before).
    let (fx, mut store, _before) = dspre("script main #1:\n    End\n", &[]);
    store
        .edit_text_archive(ARCHIVE, "eins", |doc| {
            doc.messages[1].text = "Eins".into();
        })
        .unwrap();
    store.save().unwrap();
    store
        .edit_text_archive(ARCHIVE, "zwei", |doc| {
            doc.messages[1].text = "Zwei".into();
        })
        .unwrap();
    store.undo().unwrap();
    store.undo().unwrap();
    store.redo().unwrap();
    store.save().unwrap();
    let json = fx.root().join(json_of(ARCHIVE));
    assert!(json.is_file());
    assert!(String::from_utf8_lossy(&std::fs::read(json).unwrap()).contains("Eins"));
}

/// The files of text archive `archive` and script file `script`, and rotom's compile state.
fn watched(project: &Project, archive: u16, script: u16) -> impl PartialEq + std::fmt::Debug {
    (
        scripts::snapshot_archive(project, archive).unwrap(),
        scripts::snapshot_script(project, script).unwrap(),
        scripts::snapshot_compile_state(project).unwrap(),
    )
}

#[test]
fn vanilla_edit_and_split_saved_then_undone_and_saved_leave_the_files_as_before() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    // (start town, its archive), (header, event file, object, archive, message) of a split.
    let ((town, town_archive), (header, file, object, archive, message)) = match project.family {
        Family::Pt => ((411, 554), (75, 74, 0, 95, 4)),
        Family::Hgss => ((60, 542), (22, 19, 12, 351, 7)),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(project.clone());
    let script =
        |store: &mut Store, h: u16| store.header_table().unwrap()[usize::from(h)].script_file_id();

    // A dialog edit in the start town: saved, undone, saved.
    let town_script = script(&mut store, town);
    let before = watched(&project, town_archive, town_script);
    assert!(!root.join(json_of(town_archive)).exists());
    store
        .edit_text_archive(town_archive, "Dialog geändert", |doc| {
            doc.messages[0].text.push_str(" Gate-Test.");
        })
        .unwrap();
    store.save().unwrap();
    assert!(root.join(json_of(town_archive)).exists());
    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(watched(&project, town_archive, town_script), before);
    assert!(!root.join(json_of(town_archive)).exists());

    // The split: right after it, and saved, undone, saved.
    let split_script = script(&mut store, header);
    let before = watched(&project, archive, split_script);
    let report = split_message(&mut store, header, file, object, archive, message).unwrap();
    assert!(
        !root.join(json_of(archive)).exists(),
        "the compile's JSON stays"
    );
    store.save().unwrap();
    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(watched(&project, archive, split_script), before);
    eprintln!(
        "dspre-undo {} town archive {town_archive}, split {archive}:{message} -> {}",
        project.game_code, report.new_index
    );
}
