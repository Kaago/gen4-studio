//! The recipes of phase 3d.5 (Erzählen): „Neue Figur“ (an object event, a script slot in the
//! header's script file and a numbered message in the header's text archive, one undo step);
//! „Tür verbinden“ and „Neuer Trainer hier“ follow.
//!
//! Synthetic tests run on `test_support::TextFixture::exported()` (the synthetic Platinum
//! project as a text project with the scripts adopted; header 0 has script file 0, event file
//! 0 and text archive 433, matrix 0 of 2 x 1 maps). The gated test (`G4_TEST_PROJECT`, Platinum
//! DE or HeartGold DE) applies a figure beside the door of the player's house in the start town
//! on a copy, undoes it and compares the files:
//!
//! ```sh
//! G4_TEST_PROJECT=".../_testkopien/3d-5-fig-pt_DSPRE_contents" \
//!   cargo test -p g4-kernel --all-features --test recipes_erzaehlen -- vanilla --nocapture
//! ```
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::data::trainer_create::ADOPT_FIRST;
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::new_figure::{self, NewFigureInput, figure_script};
use g4_kernel::recipes::{self, DraftEventKind, RecipeId, next_steps};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::store::Store;

/// The text archive of header 0 of the fixture.
const ARCHIVE: u16 = 433;

const SOURCE: &str = "script main #1:\n    End\n\nscript zwei #2:\n    End\n";

fn text_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::TextArchive,
        index,
    }
}

fn event_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Event,
        index,
    }
}

/// The fixture with `source` as script file 0 (header 0's) and a store over it.
fn fixture_with(source: &str) -> (common::TextFixture, Store) {
    let fx = common::TextFixture::exported();
    let project = fx.project();
    assert!(scripts::is_adopted(&project));
    scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
    let mut store = Store::new(Arc::new(project));
    // Loaded, so [`state`] reads their bytes from the start.
    store.text_archive(ARCHIVE).unwrap();
    store.event(0).unwrap();
    (fx, store)
}

fn fixture() -> (common::TextFixture, Store) {
    fixture_with(SOURCE)
}

fn input(x: u16, z: u16, text: &str) -> NewFigureInput {
    NewFigureInput {
        header: 0,
        x,
        z,
        graphics_id: None,
        movement_type: 15,
        dir: 1,
        text: text.into(),
        hidden_flag: None,
    }
}

fn source_of(store: &Store, file: u16) -> String {
    match scripts::read_source(store.project(), file).unwrap() {
        ScriptSource::Rotom(text) => text,
        other => panic!("script file {file} is no rotom source: {other:?}"),
    }
}

fn refused<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Every file below `dir` with its bytes, by path relative to `dir` (none when absent).
fn files_below(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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
    walk(dir, dir, &mut out);
    out
}

/// The files a figure changes: the script source and member, rotom's compile state, the text
/// archive's JSON and member on disk, and the store's bytes of the archive and the event file.
#[derive(Debug, PartialEq)]
struct State {
    script: scripts::ScriptSnapshot,
    compile_state: Option<Vec<u8>>,
    archive: scripts::ArchiveSnapshot,
    archive_store: Option<Vec<u8>>,
    events_store: Option<Vec<u8>>,
}

fn state(store: &Store) -> State {
    let project = store.project();
    State {
        script: scripts::snapshot_script(project, 0).unwrap(),
        compile_state: scripts::snapshot_compile_state(project).unwrap(),
        archive: scripts::snapshot_archive(project, ARCHIVE).unwrap(),
        archive_store: store.current_bytes(text_id(ARCHIVE)).unwrap(),
        events_store: store.current_bytes(event_id(0)).unwrap(),
    }
}

// ---------------------------------------------------------------------------------------
// The list and the next steps
// ---------------------------------------------------------------------------------------

#[test]
fn list_names_five_recipes_with_new_figure_last() {
    let list = recipes::list();
    let shown: Vec<(RecipeId, &str, bool)> = list
        .iter()
        .map(|r| (r.id, r.title, r.irreversible))
        .collect();
    assert_eq!(
        shown,
        [
            (RecipeId::CreateArea, "Gebiet anlegen", false),
            (RecipeId::CreateRoom, "Raum anlegen", false),
            (RecipeId::ExtendArea, "Gebiet erweitern", false),
            (RecipeId::EmptyRegion, "Welt leeren", true),
            (RecipeId::NewFigure, "Neue Figur", false),
        ]
    );
    assert_eq!(
        serde_json::to_value(RecipeId::NewFigure).unwrap(),
        serde_json::json!("new_figure")
    );
}

#[test]
fn next_steps_after_3d5_lock_with_3d7_and_3d9() {
    let listed = |id: &str| {
        recipes::list()
            .iter()
            .any(|r| serde_json::to_value(r.id).unwrap() == serde_json::json!(id))
    };
    let steps = next_steps(&[
        ("edit_dialog", "Dialog bearbeiten"),
        ("story_join", "Zu einer Geschichte"),
        ("play_here", "Hier spielen"),
        ("new_figure", "Neue Figur hier"),
        ("connect_doors", "Tür verbinden"),
        ("new_trainer_here", "Neuer Trainer hier"),
        ("adjust_terrain", "Gelände anpassen"),
    ]);
    let short: BTreeMap<&str, (bool, Option<&str>)> = steps
        .iter()
        .map(|s| (s.id.as_str(), (s.available, s.reason.as_deref())))
        .collect();
    assert_eq!(short["edit_dialog"], (true, None));
    assert_eq!(short["story_join"], (false, Some("Kommt mit 3d.7")));
    assert_eq!(short["play_here"], (false, Some("Kommt mit 3d.9")));
    assert_eq!(short["new_figure"], (true, None));
    assert_eq!(short["adjust_terrain"], (false, Some("Kommt mit Phase 6")));
    // The door and trainer recipes are free as soon as the list has them.
    for id in ["connect_doors", "new_trainer_here"] {
        let (available, reason) = short[id];
        assert_eq!(available, listed(id), "{id}");
        assert_eq!(reason.is_none(), available, "{id}");
    }
}

// ---------------------------------------------------------------------------------------
// The script per family
// ---------------------------------------------------------------------------------------

#[test]
fn hgss_writes_npcmsg_and_closemsg() {
    let hgss = figure_script(Family::Hgss, 5, 4, 3);
    assert!(hgss.contains("#5:"), "{hgss}");
    assert!(hgss.contains("figur_4"), "{hgss}");
    assert!(hgss.contains("NPCMsg 3"), "{hgss}");
    assert!(hgss.contains("CloseMsg"), "{hgss}");
    assert!(!hgss.contains("Message"), "{hgss}");
    assert!(!hgss.contains('{') && !hgss.contains('}'), "{hgss}");

    let pt = figure_script(Family::Pt, 5, 4, 3);
    assert!(pt.contains("script figur_4 #5:"), "{pt}");
    assert!(pt.contains("Message 3"), "{pt}");
    assert!(pt.contains("CloseMessage"), "{pt}");
    assert!(!pt.contains("NPCMsg") && !pt.contains('"'), "{pt}");
    // Diamond/Pearl writes Platinum's script.
    assert_eq!(figure_script(Family::Dp, 5, 4, 3), pt);
}

// ---------------------------------------------------------------------------------------
// Plan and apply
// ---------------------------------------------------------------------------------------

#[test]
fn plan_of_a_figure_writes_nothing_and_names_event_message_and_slot() {
    let (fx, mut store) = fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let (depth, dirty) = (store.undo_depth(), store.dirty());
    let expanded = files_below(&fx.root().join("expanded"));
    let before = state(&store);

    let figure = NewFigureInput {
        hidden_flag: Some(0x0100),
        ..input(40, 7, "Hallo, ich bin neu hier.")
    };
    let draft = new_figure::plan(&mut store, &figure).unwrap();
    assert_eq!(draft.events.len(), 1);
    let event = &draft.events[0];
    assert_eq!(event.kind, DraftEventKind::Object);
    assert_eq!((event.matrix, event.x, event.z), (Some(0), 40, 7));
    assert!(event.look.is_some(), "{event:?}");
    assert_eq!(event.target, None);
    assert_eq!(draft.messages.len(), 1);
    assert_eq!(draft.messages[0].archive, Some(ARCHIVE));
    assert_eq!(draft.messages[0].text, figure.text);
    assert!(draft.irreversible.is_empty(), "{:?}", draft.irreversible);
    let notes = draft.notes.join(" ");
    assert!(notes.contains("#3"), "{notes}");
    assert!(
        notes.contains(&format!("Nachricht {messages} in Archiv {ARCHIVE}")),
        "{notes}"
    );
    assert!(notes.contains("Flag 0x0100"), "{notes}");

    // The given look wins over the default.
    let look = NewFigureInput {
        graphics_id: Some(97),
        ..input(40, 7, "x")
    };
    assert_eq!(
        new_figure::plan(&mut store, &look).unwrap().events[0].look,
        Some(97)
    );

    // Nothing written.
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(store.dirty(), dirty);
    assert_eq!(files_below(&fx.root().join("expanded")), expanded);
    assert_eq!(state(&store), before);
}

#[test]
fn apply_of_a_figure_is_one_undo_step_with_a_numbered_message() {
    let (_fx, mut store) = fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let depth = store.undo_depth();
    let member_before = scripts::snapshot_script(store.project(), 0).unwrap().member;
    let figure = input(40, 7, "Hallo, ich bin neu hier und rede gern.");

    let outcome = new_figure::apply(&mut store, &figure).unwrap();
    let report = &outcome.report;
    assert_eq!(
        (
            report.event_file,
            report.object,
            report.local_id,
            report.script_file,
            report.slot,
            report.archive,
            usize::from(report.message)
        ),
        (0, 0, 0, 0, 3, ARCHIVE, messages)
    );
    assert!(outcome.created.is_empty(), "{:?}", outcome.created);
    assert!(outcome.change.is_some());
    for id in [event_id(0), text_id(ARCHIVE)] {
        assert!(
            outcome.changed.contains(&id),
            "{id:?} in {:?}",
            outcome.changed
        );
    }

    // The object.
    let object = store.event(0).unwrap().object_events[0].clone();
    assert_eq!(
        (
            object.x,
            object.z,
            object.script,
            object.movement_type,
            object.dir
        ),
        (40, 7, 3, 15, 1)
    );
    assert_eq!((object.trainer_type, object.hidden_flag), (0, 0));
    assert_eq!(object.graphics_id, 6, "Platinum's default look");
    // The archive: one message more, exactly the text.
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages.len(), messages + 1);
    assert_eq!(doc.messages[messages].text, figure.text);
    // The source names the number, no literal; the member is new.
    let source = source_of(&store, 0);
    assert!(source.starts_with(SOURCE), "{source}");
    assert!(source.contains("script figur_0 #3:"), "{source}");
    assert!(
        source.contains(&format!("Message {messages}\n")),
        "{source}"
    );
    assert!(!source.contains('"'), "{source}");
    let member_after = scripts::snapshot_script(store.project(), 0).unwrap().member;
    assert_ne!(member_after, member_before);

    // One step with the label.
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Figur „Hallo, ich bin neu h…“ aufgestellt")
    );
    let next: Vec<(&str, bool)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available))
        .collect();
    assert_eq!(next, [("edit_dialog", true), ("story_join", false)]);

    // A second figure gets the next local id, slot and message.
    let second = new_figure::apply(&mut store, &input(41, 7, "Zwei")).unwrap();
    assert_eq!(
        (
            second.report.object,
            second.report.local_id,
            second.report.slot
        ),
        (1, 1, 4)
    );
    assert_eq!(usize::from(second.report.message), messages + 1);

    // Through the dispatcher as well.
    let call: recipes::RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "new_figure",
        "input": { "header": 0, "x": 42, "z": 7, "graphics_id": null, "movement_type": 0,
                   "dir": 0, "text": "Drei", "hidden_flag": null },
    }))
    .unwrap();
    assert_eq!(recipes::is_irreversible(&call).unwrap(), None);
    assert_eq!(recipes::plan(&mut store, &call).unwrap().events.len(), 1);
    let third = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(third.report["slot"], 5);
}

#[test]
fn undo_and_redo_of_a_figure_restore_every_file() {
    let (_fx, mut store) = fixture();
    let before = state(&store);
    new_figure::apply(&mut store, &input(40, 7, "Bis bald!")).unwrap();
    let after = state(&store);
    assert_ne!(after.script.rotom, before.script.rotom);
    assert_ne!(after.script.member, before.script.member);
    assert_ne!(after.archive_store, before.archive_store);
    assert_ne!(after.events_store, before.events_store);

    store.undo().unwrap();
    assert_eq!(state(&store), before);
    assert!(store.event(0).unwrap().object_events.is_empty());

    store.redo().unwrap();
    assert_eq!(state(&store), after);
    store.undo().unwrap();
    assert_eq!(state(&store), before);
}

/// The fixture as a DSPRE project with the scripts adopted: no archive JSON until a compile
/// writes it from the member.
fn dspre_fixture() -> (common::TextFixture, Store, PathBuf) {
    let fx = common::TextFixture::dspre();
    let project = fx.project();
    scripts::adopt(&project, &mut |_| {}).unwrap();
    scripts::write_source(&project, 0, &ScriptSource::Rotom(SOURCE.into())).unwrap();
    let mut store = Store::new(Arc::new(project));
    store.text_archive(ARCHIVE).unwrap();
    store.event(0).unwrap();
    let json = store
        .project()
        .expanded_dir(AssetKind::TextArchive)
        .join(format!("{ARCHIVE:04}.json"));
    assert!(!json.exists());
    (fx, store, json)
}

#[test]
fn a_dspre_archive_json_the_compile_writes_is_removed_again() {
    let (_fx, mut store, json) = dspre_fixture();
    let before = state(&store);

    let outcome = new_figure::apply(&mut store, &input(40, 7, "Bis bald!")).unwrap();
    let message = usize::from(outcome.report.message);
    // The compile wrote the JSON from the member; the recipe removed it again.
    assert!(!json.exists());
    let after = state(&store);
    assert_eq!(after.archive, before.archive);
    store.undo().unwrap();
    assert_eq!(state(&store), before);
    store.redo().unwrap();
    assert_eq!(state(&store), after);
    // The store kept the figure's message through undo and redo.
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages[message].text, "Bis bald!");
}

#[test]
fn a_saved_figure_in_a_dspre_project_is_undone_and_redone() {
    let (fx, mut store, _json) = dspre_fixture();
    let files = |root: &Path| {
        let mut all = BTreeMap::new();
        for dir in ["expanded", "unpacked"] {
            for (path, bytes) in files_below(&root.join(dir)) {
                all.insert(Path::new(dir).join(path), bytes);
            }
        }
        all
    };
    let differ = |a: &BTreeMap<PathBuf, Vec<u8>>, b: &BTreeMap<PathBuf, Vec<u8>>| {
        a.keys()
            .chain(b.keys())
            .filter(|k| a.get(*k) != b.get(*k))
            .cloned()
            .collect::<std::collections::BTreeSet<PathBuf>>()
    };
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let before_files = files(fx.root());
    let before = state(&store);

    let outcome = new_figure::apply(&mut store, &input(40, 7, "Bis bald!")).unwrap();
    store.save().unwrap();
    let saved_files = files(fx.root());
    assert!(differ(&saved_files, &before_files).len() >= 5);

    store.undo().unwrap();
    assert!(store.event(0).unwrap().object_events.is_empty());
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages.len(),
        messages
    );
    assert!(!source_of(&store, 0).contains(&format!("#{}:", outcome.report.slot)));
    store.save().unwrap();
    // As before, but the archive JSON: the first save wrote it (a store save writes the
    // archive's JSON and member), now without the figure's message.
    let json = Path::new("expanded/textArchives").join(format!("{ARCHIVE:04}.json"));
    let now = files(fx.root());
    assert_eq!(
        differ(&now, &before_files),
        std::collections::BTreeSet::from([json.clone()])
    );
    assert!(!String::from_utf8_lossy(&now[&json]).contains("Bis bald!"));
    let undone = state(&store);
    assert_eq!(
        (undone.script, undone.compile_state),
        (before.script, before.compile_state)
    );

    store.redo().unwrap();
    store.save().unwrap();
    assert_eq!(differ(&files(fx.root()), &saved_files), Default::default());
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages[messages].text, "Bis bald!");
    assert_eq!(store.event(0).unwrap().object_events.len(), 1);
}

#[test]
fn undo_refuses_a_source_changed_meanwhile() {
    let (_fx, mut store) = fixture();
    new_figure::apply(&mut store, &input(40, 7, "Bis bald!")).unwrap();
    let path = scripts::source_path(store.project(), 0, scripts::SourceKind::Script);
    let by_hand = format!("{}\n// von Hand\n", source_of(&store, 0));
    std::fs::write(&path, &by_hand).unwrap();

    let why = refused(store.undo());
    assert!(!why.is_empty());
    // Nothing was put back: the hand edit stays.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), by_hand);
}

// ---------------------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------------------

#[test]
fn a_figure_on_a_taken_tile_and_an_empty_text_are_refused() {
    let (_fx, mut store) = fixture();
    new_figure::apply(&mut store, &input(40, 7, "Erste")).unwrap();
    let (depth, source) = (store.undo_depth(), source_of(&store, 0));

    let why = refused(new_figure::plan(&mut store, &input(40, 7, "Zweite")));
    assert_eq!(why, "auf 40, 7 steht schon Objekt 0");
    assert_eq!(
        refused(new_figure::apply(&mut store, &input(40, 7, "Zweite"))),
        why
    );
    for text in ["", "  \n"] {
        let why = refused(new_figure::apply(&mut store, &input(41, 7, text)));
        assert!(why.contains("Text"), "{why}");
    }
    // A tile outside the header's matrix and a flag outside the save.
    let why = refused(new_figure::plan(&mut store, &input(64, 7, "x")));
    assert!(why.contains("outside matrix"), "{why}");
    let flag = NewFigureInput {
        hidden_flag: Some(0xFFFF),
        ..input(41, 7, "x")
    };
    let why = refused(new_figure::plan(&mut store, &flag));
    assert!(why.contains("0xFFFF"), "{why}");

    assert_eq!(store.undo_depth(), depth);
    assert_eq!(source_of(&store, 0), source);
    assert_eq!(store.event(0).unwrap().object_events.len(), 1);
}

#[test]
fn a_figure_needs_adopted_scripts() {
    let fx = common::TextFixture::dspre();
    let project = fx.project();
    assert!(!scripts::is_adopted(&project));
    let mut store = Store::new(Arc::new(project));
    let depth = store.undo_depth();
    assert_eq!(
        refused(new_figure::plan(&mut store, &input(40, 7, "x"))),
        ADOPT_FIRST
    );
    assert_eq!(
        refused(new_figure::apply(&mut store, &input(40, 7, "x"))),
        ADOPT_FIRST
    );
    assert_eq!(store.undo_depth(), depth);
}

#[test]
fn a_slot_from_2000_on_is_refused() {
    let source = "script main #1:\n    End\n\nscript hoch #1999:\n    End\n";
    let (_fx, mut store) = fixture_with(source);
    let why = refused(new_figure::plan(&mut store, &input(40, 7, "x")));
    assert!(why.contains("2000"), "{why}");
    assert!(refused(new_figure::apply(&mut store, &input(40, 7, "x"))).contains("2000"));
    assert_eq!(source_of(&store, 0), source);
    assert!(store.event(0).unwrap().object_events.is_empty());
}

#[test]
fn a_message_number_past_the_command_is_refused() {
    let (_fx, mut store) = fixture();
    // Platinum's `Message` takes its number as a u8: 0 to 255.
    store
        .edit_text_archive(ARCHIVE, "voll", |doc| {
            while doc.messages.len() < 256 {
                let mut m = doc.messages[0].clone();
                m.id = g4_kernel::text::default_message_id(doc.messages.len());
                doc.messages.push(m);
            }
        })
        .unwrap();
    let depth = store.undo_depth();
    let why = refused(new_figure::plan(&mut store, &input(40, 7, "x")));
    assert!(why.contains("255") && why.contains("256"), "{why}");
    assert_eq!(store.undo_depth(), depth);
    // One message fewer: the figure takes 255.
    store
        .edit_text_archive(ARCHIVE, "kürzer", |doc| {
            doc.messages.pop();
        })
        .unwrap();
    let outcome = new_figure::apply(&mut store, &input(40, 7, "x")).unwrap();
    assert_eq!(outcome.report.message, 255);
}

// ---------------------------------------------------------------------------------------
// Gated: the start town of a vanilla game
// ---------------------------------------------------------------------------------------

/// The files under `expanded/scripts/`, `expanded/textArchives/` and `unpacked/textArchives/`.
fn script_and_text_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut all = BTreeMap::new();
    for dir in [
        "expanded/scripts",
        "expanded/textArchives",
        "unpacked/textArchives",
    ] {
        for (path, bytes) in files_below(&root.join(dir)) {
            all.insert(Path::new(dir).join(path), bytes);
        }
    }
    all
}

#[test]
fn vanilla_new_figure_message_equals_the_input() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    // The start town and the ground floor of the player's house, whose door is the town's warp
    // into it (pt-leere-region.yaml: 415, 414, 411; hg-leere-region.yaml: 64, 63, 60).
    let (header, house) = match project.family {
        Family::Pt => (411, 414),
        Family::Hgss => (60, 63),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(project.clone());
    let game = project.game_code.clone();
    let h = store.header_table().unwrap()[usize::from(header)].clone();
    let (file, archive) = (h.event_file_id(), h.text_archive_id());
    let events = store.event(file).unwrap().clone();
    let door = events
        .warp_events
        .iter()
        .find(|w| w.dest_header_id == house)
        .expect("the town has a door into the house");
    // Beside the door, one row in front of it: the first such tile no object stands on.
    let taken = |x: u16, z: u16| events.object_events.iter().any(|o| (o.x, o.z) == (x, z));
    let (x, z) = [(1i32, 1i32), (-1, 1), (2, 1), (-2, 1)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                u16::try_from(i32::from(door.x) + dx).unwrap(),
                u16::try_from(i32::from(door.z) + dz).unwrap(),
            )
        })
        .find(|&(x, z)| !taken(x, z))
        .unwrap();
    let figure = NewFigureInput {
        header,
        x,
        z,
        graphics_id: None,
        movement_type: 15,
        dir: 1,
        text: "Gate 3d.5".into(),
        hidden_flag: None,
    };
    let files = script_and_text_files(&root);

    let started = Instant::now();
    let outcome = new_figure::apply(&mut store, &figure).unwrap();
    let apply_ms = started.elapsed().as_millis();
    let report = outcome.report;
    assert_eq!(report.archive, archive);
    let text = &store.text_archive(archive).unwrap().messages[usize::from(report.message)].text;
    assert_eq!(text, "Gate 3d.5");
    let source = source_of(&store, report.script_file);
    assert!(source.contains(&format!("#{}:", report.slot)), "{source}");
    // The archive change is the store's until a save: on disk only the script file changes
    // and, where DSPRE had not expanded the archive, its JSON appears (written from the member).
    let after = script_and_text_files(&root);
    for (path, bytes) in &after {
        if files.get(path) != Some(bytes) {
            assert!(
                path.starts_with("expanded/scripts") || !files.contains_key(path),
                "{path:?} changed"
            );
        }
    }
    let started = Instant::now();
    store.undo().unwrap();
    let undo_ms = started.elapsed().as_millis();
    assert_eq!(script_and_text_files(&root), files);
    // Redo: the files of the step and the message are back.
    store.redo().unwrap();
    assert_eq!(script_and_text_files(&root), after);
    let text = &store.text_archive(archive).unwrap().messages[usize::from(report.message)].text;
    assert_eq!(text, "Gate 3d.5");
    assert!(
        store.event(file).unwrap().object_events[usize::from(report.object)].script == report.slot
    );
    store.undo().unwrap();
    assert_eq!(script_and_text_files(&root), files);
    eprintln!(
        "figure {game} slot={} message={} apply_ms={apply_ms} undo_ms={undo_ms}",
        report.slot, report.message
    );
}
