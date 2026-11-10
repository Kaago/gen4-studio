//! The recipes of phase 3d.5 (Erzählen): „Neue Figur“ (an object event, a script slot in the
//! header's script file and a numbered message in the header's text archive, one undo step) and
//! „Tür verbinden“ (a warp pair between two places over one or two event files, door tiles, one
//! undo step) and „Neuer Trainer hier“ (a trainer behind its create barrier, then one undo step
//! for its team, its battle texts and an object with the battle script and a sight line).
//!
//! Synthetic figure tests run on `test_support::TextFixture::exported()` (the synthetic Platinum
//! project as a text project with the scripts adopted; header 0 has script file 0, event file
//! 0 and text archive 433, matrix 0 of 2 x 1 maps); door tests on `test_support::synthetic_project()`
//! (headers 0 „Twinleaf Town“ and 1 „T02“ on matrix 0, both with event file 0). The gated tests
//! (`G4_TEST_PROJECT`, Platinum DE or HeartGold DE) apply a figure beside the door of the player's
//! house in the start town, connect the stairs of the player's room with a tile in front of
//! the house and place a trainer beside the door, on a copy, undo them and compare the files
//! (trainer tests: the synthetic battle script of `trainer_create`'s tests in a new script file):
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
use g4_kernel::data::names::{Bank, NameTables, bank_index};
use g4_kernel::data::trainer_create::{
    TRAINER_FLAG_CAPACITY, adopt_first, create_trainer_with, trainer_texts,
};
use g4_kernel::error::KernelError;
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::connect_doors::{self, ConnectDoorsInput, DoorEnd, Tile};
use g4_kernel::recipes::new_figure::{self, NewFigureInput, figure_script};
use g4_kernel::recipes::new_trainer_here::{
    self, BattleTexts, NewTrainerHereInput, PartyMon, apply_interrupted, apply_with, plan_with,
};
use g4_kernel::recipes::room::DoorTile;
use g4_kernel::recipes::{
    self, CreatedObject, DraftEventKind, DraftTarget, DraftTile, RecipeId, next_steps,
};
use g4_kernel::scripts::{self, ScriptSource, SourceKind, TrainerScriptMeta};
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
fn list_names_the_seven_recipes_up_to_new_trainer_here_first() {
    // The story recipes of 3d.7 follow (`recipes_story.rs`).
    let list = recipes::list();
    let shown: Vec<(RecipeId, &str, bool)> = list[..7]
        .iter()
        .map(|r| (r.id, r.title.as_str(), r.irreversible))
        .collect();
    assert_eq!(
        shown,
        [
            (RecipeId::CreateArea, "Create area", false),
            (RecipeId::CreateRoom, "Create room", false),
            (RecipeId::ExtendArea, "Extend area", false),
            (RecipeId::EmptyRegion, "Empty world", true),
            (RecipeId::NewFigure, "New character", false),
            (RecipeId::ConnectDoors, "Connect door", false),
            (RecipeId::NewTrainerHere, "New Trainer here", true),
        ]
    );
    assert_eq!(
        serde_json::to_value(RecipeId::NewFigure).unwrap(),
        serde_json::json!("new_figure")
    );
    assert_eq!(
        serde_json::to_value(RecipeId::ConnectDoors).unwrap(),
        serde_json::json!("connect_doors")
    );
    assert_eq!(
        serde_json::to_value(RecipeId::NewTrainerHere).unwrap(),
        serde_json::json!("new_trainer_here")
    );
}

#[test]
fn next_steps_after_3d5_lock_with_3d9_and_later() {
    let listed = |id: &str| {
        recipes::list()
            .iter()
            .any(|r| serde_json::to_value(r.id).unwrap() == serde_json::json!(id))
    };
    let steps = next_steps(&[
        "edit_dialog",
        "story_join",
        "play_here",
        "new_figure",
        "connect_doors",
        "new_trainer_here",
        "adjust_terrain",
    ]);
    let short: BTreeMap<&str, (bool, Option<&str>)> = steps
        .iter()
        .map(|s| (s.id.as_str(), (s.available, s.reason.as_deref())))
        .collect();
    assert_eq!(short["edit_dialog"], (true, None));
    assert_eq!(short["story_join"], (true, None));
    assert_eq!(short["play_here"], (false, Some("Coming with 3d.9")));
    assert_eq!(short["new_figure"], (true, None));
    assert_eq!(short["adjust_terrain"], (true, None));
    // All three recipes of 3d.5 are in the list, so free.
    for id in ["connect_doors", "new_trainer_here"] {
        assert!(listed(id), "{id}");
        assert_eq!(short[id], (true, None), "{id}");
    }
    // An unknown step comes later (the transitional „Kommt mit 3d.7“ is gone).
    let unknown = next_steps(&["something_new"]);
    assert_eq!(unknown[0].reason.as_deref(), Some("Coming later"));
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
        notes.contains(&format!("message {messages} in archive {ARCHIVE}")),
        "{notes}"
    );
    assert!(notes.contains("flag 0x0100"), "{notes}");

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
        Some("Character “Hallo, ich bin neu h…” placed")
    );
    let next: Vec<(&str, bool)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available))
        .collect();
    assert_eq!(next, [("edit_dialog", true), ("story_join", true)]);

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
    // As before: the first save wrote the archive's JSON and member; saved with the texts
    // the member had, the member is back and the JSON gone (UI test 3d.5, U1).
    assert_eq!(differ(&files(fx.root()), &before_files), Default::default());
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
    assert_eq!(why, "object 0 already stands at 40, 7");
    assert_eq!(
        refused(new_figure::apply(&mut store, &input(40, 7, "Zweite"))),
        why
    );
    for text in ["", "  \n"] {
        let why = refused(new_figure::apply(&mut store, &input(41, 7, text)));
        assert!(why.contains("text"), "{why}");
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
        adopt_first()
    );
    assert_eq!(
        refused(new_figure::apply(&mut store, &input(40, 7, "x"))),
        adopt_first()
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

// ---------------------------------------------------------------------------------------
// „Tür verbinden“
// ---------------------------------------------------------------------------------------

fn map_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Map,
        index,
    }
}

fn warp_end(header: u16, warp: u16) -> DoorEnd {
    DoorEnd {
        header,
        warp: Some(warp),
        tile: None,
    }
}

fn tile_end(header: u16, x: u16, z: u16) -> DoorEnd {
    DoorEnd {
        header,
        warp: None,
        tile: Some(Tile { x, z }),
    }
}

fn doors(a: DoorEnd, b: DoorEnd) -> ConnectDoorsInput {
    ConnectDoorsInput {
        a,
        b,
        back: true,
        door_tiles: true,
    }
}

/// The warps of event file `file` as (x, z, dest header, dest warp).
fn warps(store: &mut Store, file: u16) -> Vec<(u16, u16, u16, u16)> {
    store
        .event(file)
        .unwrap()
        .warp_events
        .iter()
        .map(|w| (w.x, w.z, w.dest_header_id, w.dest_warp_id))
        .collect()
}

/// The synthetic project (headers 0 „Twinleaf Town“ and 1 „T02“ on matrix 0) with a stairs warp
/// 0 at (10, 10) leading to header 0, warp 7, in event file `file`; with `own_file` header 1
/// gets event file 1 (and the warp goes there), else both headers keep event file 0.
fn door_fixture(own_file: bool) -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let mut store = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    let file = if own_file {
        store
            .edit_header(1, "event file", |h| {
                let g4_kernel::store::MapHeader::Pt(h) = h else {
                    unreachable!("the fixture is Platinum")
                };
                h.event_file_id = 1;
                Ok(())
            })
            .unwrap();
        1
    } else {
        0
    };
    store
        .edit_event(file, "stairs", |e| {
            e.warp_events.push(g4_kernel::store::WarpEvent {
                x: 10,
                z: 10,
                dest_header_id: 0,
                dest_warp_id: 7,
                height: 0,
            });
            Ok(())
        })
        .unwrap();
    // Loaded, so the byte comparisons see them from the start.
    for index in [0, 1] {
        store.event(index).unwrap();
    }
    store.map(0).unwrap();
    (s, store)
}

/// The store's bytes of event files 0 and 1 and map 0.
fn door_bytes(store: &Store) -> Vec<Option<Vec<u8>>> {
    [event_id(0), event_id(1), map_id(0)]
        .into_iter()
        .map(|id| store.current_bytes(id).unwrap())
        .collect()
}

/// The bytes of asset `id` in its member file on disk.
fn member_on_disk(store: &Store, id: AssetId) -> Vec<u8> {
    match store.project().asset_location(id.kind, id.index).unwrap() {
        g4_kernel::project::Location::Member(path) => std::fs::read(path).unwrap(),
        other => panic!("{id:?} is no member: {other:?}"),
    }
}

#[test]
fn plan_of_two_doors_names_both_ends_and_the_door_tiles() {
    let (_s, mut store) = door_fixture(true);
    let (depth, dirty, bytes) = (store.undo_depth(), store.dirty(), door_bytes(&store));

    // A: the stairs of header 1 (warp 0), B: a new door at (5, 6) of header 0.
    let input = doors(warp_end(1, 0), tile_end(0, 5, 6));
    let draft = connect_doors::plan(&mut store, &input).unwrap();
    type Short = (DraftEventKind, Option<u16>, u16, u16, Option<DraftTarget>);
    let events: Vec<Short> = draft
        .events
        .iter()
        .map(|e| (e.kind, e.matrix, e.x, e.z, e.target.clone()))
        .collect();
    let to = |header, warp| {
        Some(DraftTarget {
            header,
            warp: Some(warp),
        })
    };
    assert_eq!(
        events,
        [
            (DraftEventKind::Warp, Some(0), 10, 10, to(0, 0)),
            (DraftEventKind::Warp, Some(0), 5, 6, to(1, 0)),
        ]
    );
    assert_eq!(
        draft.tiles,
        [DraftTile {
            matrix: Some(0),
            x: 5,
            z: 6
        }]
    );
    assert!(draft.cells.is_empty() && draft.messages.is_empty());
    assert!(draft.irreversible.is_empty(), "{:?}", draft.irreversible);
    assert_eq!(
        draft.notes[0],
        "Door A: T02 door 0 → door B: Twinleaf Town door 0, and back."
    );
    let notes = draft.notes.join(" ");
    assert!(notes.contains("x 5, z 6"), "{notes}");
    assert!(notes.contains("becomes a door"), "{notes}");

    // Without door tiles the plan has none.
    let plain = ConnectDoorsInput {
        door_tiles: false,
        ..input.clone()
    };
    assert!(
        connect_doors::plan(&mut store, &plain)
            .unwrap()
            .tiles
            .is_empty()
    );

    // Nothing written.
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(store.dirty(), dirty);
    assert_eq!(door_bytes(&store), bytes);
}

#[test]
fn apply_connects_two_event_files_in_one_step() {
    let (_s, mut store) = door_fixture(true);
    let depth = store.undo_depth();
    let before = door_bytes(&store);

    let input = doors(warp_end(1, 0), tile_end(0, 5, 6));
    let outcome = connect_doors::apply(&mut store, &input).unwrap();
    let report = &outcome.report;
    let short = |end: &connect_doors::DoorReport| {
        (
            end.header,
            end.event_file,
            end.warp,
            end.created,
            end.door_tile,
        )
    };
    assert_eq!(short(&report.a), (1, 1, 0, false, None));
    assert_eq!(
        short(&report.b),
        (0, 0, 0, true, Some(DoorTile { map: 0, x: 5, y: 6 }))
    );
    assert!(report.back);
    assert!(outcome.created.is_empty());
    assert!(outcome.change.is_some());
    for id in [event_id(0), event_id(1), map_id(0)] {
        assert!(
            outcome.changed.contains(&id),
            "{id:?} in {:?}",
            outcome.changed
        );
    }
    // Both warps lead to each other; the tile is a door.
    assert_eq!(warps(&mut store, 1), [(10, 10, 0, 0)]);
    assert_eq!(warps(&mut store, 0), [(5, 6, 1, 0)]);
    let cell = store.map(0).unwrap().cell(5, 6).unwrap();
    assert_eq!(
        (cell.kind, cell.collision),
        (g4_kernel::tiles::DOOR, g4_kernel::tiles::DOOR_COLLISION)
    );
    // One step with the label, the next step locked until 3d.9.
    assert_eq!(store.undo_depth(), depth + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Door “T02” ↔ “Twinleaf Town” connected")
    );
    let next: Vec<(&str, bool, Option<&str>)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available, s.reason.as_deref()))
        .collect();
    assert_eq!(next, [("play_here", false, Some("Coming with 3d.9"))]);

    // Undo puts both event files and the map back byte for byte; redo brings the step back.
    let after = door_bytes(&store);
    store.undo().unwrap();
    assert_eq!(door_bytes(&store), before);
    store.redo().unwrap();
    assert_eq!(door_bytes(&store), after);
    store.undo().unwrap();

    // Through the dispatcher as well.
    let call: recipes::RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "connect_doors",
        "input": { "a": { "header": 1, "warp": 0 }, "b": { "header": 0, "tile": { "x": 5, "z": 6 } } },
    }))
    .unwrap();
    assert_eq!(recipes::is_irreversible(&call).unwrap(), None);
    assert_eq!(recipes::plan(&mut store, &call).unwrap().events.len(), 2);
    let dto = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(dto.report["b"]["warp"], 0);
    assert_eq!(dto.report["b"]["door_tile"]["map"], 0);
    // Unknown keys of an end are refused.
    let wrong: recipes::RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "connect_doors",
        "input": { "a": { "header": 1, "warp": 0, "tür": 1 }, "b": { "header": 0, "warp": 0 } },
    }))
    .unwrap();
    assert!(refused(recipes::plan(&mut store, &wrong)).contains("Connect door"));
}

#[test]
fn a_shared_event_file_is_one_edit() {
    // Both headers have event file 0 with the stairs as warp 0: two new doors go into that one
    // file, A first.
    let (_s, mut store) = door_fixture(false);
    let depth = store.undo_depth();
    let before = door_bytes(&store);
    let input = ConnectDoorsInput {
        door_tiles: false,
        ..doors(tile_end(0, 5, 6), tile_end(1, 20, 6))
    };
    let draft = connect_doors::plan(&mut store, &input).unwrap();
    let targets: Vec<Option<DraftTarget>> = draft.events.iter().map(|e| e.target.clone()).collect();
    assert_eq!(
        targets,
        [
            Some(DraftTarget {
                header: 1,
                warp: Some(2)
            }),
            Some(DraftTarget {
                header: 0,
                warp: Some(1)
            }),
        ]
    );
    let outcome = connect_doors::apply(&mut store, &input).unwrap();
    assert_eq!((outcome.report.a.warp, outcome.report.b.warp), (1, 2));
    assert_eq!(outcome.report.a.event_file, outcome.report.b.event_file);
    assert_eq!(
        warps(&mut store, 0),
        [(10, 10, 0, 7), (5, 6, 1, 2), (20, 6, 0, 1)]
    );
    // One part for the one file.
    assert_eq!(outcome.changed, [event_id(0)]);
    assert_eq!(store.undo_depth(), depth + 1);
    store.undo().unwrap();
    assert_eq!(door_bytes(&store), before);
}

#[test]
fn the_same_door_twice_and_a_taken_tile_are_refused() {
    let (_s, mut store) = door_fixture(false);
    let (depth, dirty, bytes) = (store.undo_depth(), store.dirty(), door_bytes(&store));
    let both = |store: &mut Store, input: &ConnectDoorsInput| {
        let why = refused(connect_doors::plan(store, input));
        assert_eq!(refused(connect_doors::apply(store, input)), why);
        why
    };
    const SAME: &str = "Door A and door B are the same door";

    // The same warp, also through two headers that share the event file.
    assert_eq!(
        both(&mut store, &doors(warp_end(0, 0), warp_end(0, 0))),
        SAME
    );
    assert_eq!(
        both(&mut store, &doors(warp_end(0, 0), warp_end(1, 0))),
        SAME
    );
    // The same new tile of one event file.
    assert_eq!(
        both(&mut store, &doors(tile_end(0, 5, 6), tile_end(1, 5, 6))),
        SAME
    );
    // A tile a warp stands on.
    assert_eq!(
        both(&mut store, &doors(tile_end(0, 5, 6), tile_end(1, 10, 10))),
        "door 0 already lies at 10, 10; choose it as the door"
    );
    // An end needs exactly one of door and tile.
    let neither = DoorEnd {
        header: 0,
        warp: None,
        tile: None,
    };
    assert_eq!(
        both(&mut store, &doors(neither.clone(), warp_end(0, 0))),
        "Door A needs a door or a tile"
    );
    let two = DoorEnd {
        tile: Some(Tile { x: 5, z: 6 }),
        ..warp_end(1, 0)
    };
    assert_eq!(
        both(&mut store, &doors(tile_end(0, 5, 6), two)),
        "Door B needs a door or a tile"
    );
    // A door the event file does not have, a header the game does not have, a tile outside
    // the header's matrix.
    let why = both(&mut store, &doors(warp_end(0, 3), tile_end(0, 5, 6)));
    assert!(why.contains("door 3"), "{why}");
    let why = both(&mut store, &doors(warp_end(9, 0), tile_end(0, 5, 6)));
    assert!(why.contains("header 9"), "{why}");
    let why = both(&mut store, &doors(warp_end(0, 0), tile_end(0, 64, 6)));
    assert!(why.contains("outside matrix"), "{why}");

    // Nothing written.
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(store.dirty(), dirty);
    assert_eq!(door_bytes(&store), bytes);
}

#[test]
fn without_back_only_a_points_to_b() {
    let (_s, mut store) = door_fixture(true);
    // B: a door of header 0 that leads elsewhere (to header 1, warp 3).
    store
        .edit_event(0, "door", |e| {
            e.warp_events.push(g4_kernel::store::WarpEvent {
                x: 5,
                z: 6,
                dest_header_id: 1,
                dest_warp_id: 3,
                height: 0,
            });
            Ok(())
        })
        .unwrap();
    let file_b = store.current_bytes(event_id(0)).unwrap();
    let input = ConnectDoorsInput {
        back: false,
        ..doors(warp_end(1, 0), warp_end(0, 0))
    };
    let draft = connect_doors::plan(&mut store, &input).unwrap();
    assert_eq!(draft.events.len(), 1);
    assert_eq!((draft.events[0].x, draft.events[0].z), (10, 10));
    assert_eq!(
        draft.notes[0],
        "Door A: T02 door 0 → door B: Twinleaf Town door 0."
    );

    let outcome = connect_doors::apply(&mut store, &input).unwrap();
    assert!(!outcome.report.back);
    assert_eq!(warps(&mut store, 1), [(10, 10, 0, 0)]);
    // B's warp still leads where it led.
    assert_eq!(warps(&mut store, 0), [(5, 6, 1, 3)]);
    assert_eq!(store.current_bytes(event_id(0)).unwrap(), file_b);
    assert_eq!(outcome.changed, [event_id(1)]);
    // Again: A already leads to B, nothing changes and no step is recorded.
    let depth = store.undo_depth();
    let again = connect_doors::apply(&mut store, &input).unwrap();
    assert_eq!((again.change, again.changed.len()), (None, 0));
    assert_eq!(store.undo_depth(), depth);

    // A new door B without a way back would lead nowhere: refused.
    let input = ConnectDoorsInput {
        back: false,
        ..doors(warp_end(1, 0), tile_end(0, 20, 6))
    };
    let why = refused(connect_doors::plan(&mut store, &input));
    assert!(why.contains("way back"), "{why}");
}

#[test]
fn a_save_between_apply_and_undo_keeps_undo_byte_equal() {
    let (_s, mut store) = door_fixture(true);
    store.save().unwrap();
    let before = door_bytes(&store);
    let ids = [event_id(0), event_id(1), map_id(0)];
    let disk_before: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();

    connect_doors::apply(&mut store, &doors(warp_end(1, 0), tile_end(0, 5, 6))).unwrap();
    let after = door_bytes(&store);
    store.save().unwrap();
    let disk_after: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();
    for (i, (a, b)) in disk_after.iter().zip(&disk_before).enumerate() {
        assert_ne!(a, b, "{:?} saved", ids[i]);
    }

    // Every part is the store's: undo after the save needs no file and is byte-equal.
    store.undo().unwrap();
    assert_eq!(door_bytes(&store), before);
    store.save().unwrap();
    let disk: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();
    assert_eq!(disk, disk_before);
    store.redo().unwrap();
    assert_eq!(door_bytes(&store), after);
    store.save().unwrap();
    let disk: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();
    assert_eq!(disk, disk_after);
}

// ---------------------------------------------------------------------------------------
// Gated: the stairs of the player's room and a tile in front of the house
// ---------------------------------------------------------------------------------------

#[test]
fn vanilla_doors_round_trip_in_the_town() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    // The player's room with its stairs (warp 0), the start town and the ground floor of the
    // house (pt-raum-assistent.yaml: 415, event file 394; hg-leere-region.yaml: 64, 63, 60).
    let (room, room_file, town, house) = match project.family {
        Family::Pt => (415, 394, 411, 414),
        Family::Hgss => (64, 61, 60, 63),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    let mut store = Store::new(project.clone());
    let game = project.game_code.clone();
    let headers = store.header_table().unwrap().to_vec();
    assert_eq!(headers[usize::from(room)].event_file_id(), room_file);
    let town_file = headers[usize::from(town)].event_file_id();
    let town_matrix = headers[usize::from(town)].matrix_id();
    let events = store.event(town_file).unwrap().clone();
    let door = events
        .warp_events
        .iter()
        .find(|w| w.dest_header_id == house)
        .expect("the town has a door into the house")
        .clone();
    // In front of the house, beside the door: the first tile no warp or object stands on that
    // the plan takes.
    let taken = |x: u16, z: u16| {
        events.object_events.iter().any(|o| (o.x, o.z) == (x, z))
            || events.warp_events.iter().any(|w| (w.x, w.z) == (x, z))
    };
    let input = |x, z| doors(warp_end(room, 0), tile_end(town, x, z));
    let (x, z) = [(2i32, 1i32), (-2, 1), (3, 1), (-3, 1), (2, 2), (-2, 2)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                u16::try_from(i32::from(door.x) + dx).unwrap(),
                u16::try_from(i32::from(door.z) + dz).unwrap(),
            )
        })
        .find(|&(x, z)| !taken(x, z) && connect_doors::plan(&mut store, &input(x, z)).is_ok())
        .expect("a free tile in front of the house");
    // The map under the tile.
    let m = store.matrix(town_matrix).unwrap().clone();
    let cell = usize::from(z / 32) * usize::from(m.width) + usize::from(x / 32);
    let map = m.maps[cell];
    let ids = [event_id(room_file), event_id(town_file), map_id(map)];
    let bytes = |store: &mut Store| -> Vec<Option<Vec<u8>>> {
        ids.iter()
            .map(|&id| {
                if id.kind == AssetKind::Event {
                    store.event(id.index).unwrap();
                } else {
                    store.map(id.index).unwrap();
                }
                store.current_bytes(id).unwrap()
            })
            .collect()
    };
    let before = bytes(&mut store);
    let disk_before: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();
    let town_warps = u16::try_from(events.warp_events.len()).unwrap();

    let started = Instant::now();
    let outcome = connect_doors::apply(&mut store, &input(x, z)).unwrap();
    let ms = started.elapsed().as_millis();
    let report = outcome.report;
    assert_eq!(
        (
            report.a.header,
            report.a.event_file,
            report.a.warp,
            report.a.created
        ),
        (room, room_file, 0, false)
    );
    assert_eq!(
        (
            report.b.header,
            report.b.event_file,
            report.b.warp,
            report.b.created
        ),
        (town, town_file, town_warps, true)
    );
    // As `get` reads them: the stairs lead to the town's new door, the door to the stairs.
    let stairs = store.event(room_file).unwrap().warp_events[0].clone();
    assert_eq!(
        (stairs.dest_header_id, stairs.dest_warp_id),
        (town, report.b.warp)
    );
    let new = store.event(town_file).unwrap().warp_events[usize::from(report.b.warp)].clone();
    assert_eq!(
        (new.x, new.z, new.dest_header_id, new.dest_warp_id),
        (x, z, room, 0)
    );
    // The door tile lies on the map the comparison reads.
    if let Some(tile) = report.b.door_tile {
        assert_eq!(
            (tile.map, u16::from(tile.x), u16::from(tile.y)),
            (map, x % 32, z % 32)
        );
    }
    let after = bytes(&mut store);

    // Undo byte-equal, also with a save between apply and undo.
    store.undo().unwrap();
    assert_eq!(bytes(&mut store), before);
    store.redo().unwrap();
    assert_eq!(bytes(&mut store), after);
    store.save().unwrap();
    store.undo().unwrap();
    assert_eq!(bytes(&mut store), before);
    store.save().unwrap();
    let disk: Vec<Vec<u8>> = ids.iter().map(|&id| member_on_disk(&store, id)).collect();
    assert_eq!(disk, disk_before);
    eprintln!(
        "doors {game} a={room_file}:0 b={town_file}:{} tile=({x}, {z}) map={map} door_tile={} ms={ms} (unter Last)",
        report.b.warp,
        report.b.door_tile.is_some()
    );
}

// ---------------------------------------------------------------------------------------
// „Neuer Trainer hier“
// ---------------------------------------------------------------------------------------

/// A battle script like Platinum's file 1114, for the fixture's three trainers: slots 1 to 3
/// share the battle code, slot 4 is the sight script (as in the tests of `trainer_create`).
const BATTLE: &str = "script script_1 #[1-3]:\n    LockAll\n    FacePlayer\n    ReleaseAll\n    \
                      End\nscript script_4 #4:\n    LockAll\n    ReleaseAll\n    End\n";

/// The fixture's battle text bank (Platinum).
const BATTLE_TEXTS: u16 = 617;

fn trainer_id(index: u16) -> AssetId {
    AssetId {
        kind: AssetKind::Trainer,
        index,
    }
}

/// The text project fixture with the battle script [`BATTLE`] in a new script file, compiled,
/// a store over it and the battle script's meta (sight slot 4).
fn trainer_fixture() -> (common::TextFixture, Store, TrainerScriptMeta) {
    let fx = common::TextFixture::exported();
    let project = Arc::new(fx.project());
    assert!(scripts::is_adopted(&project));
    let battle_file = scripts::new_script_file(&project, SourceKind::Script).unwrap();
    scripts::write_source(&project, battle_file, &ScriptSource::Rotom(BATTLE.into())).unwrap();
    let compiled = scripts::compile(&project).unwrap();
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    let mut store = Store::new(project);
    store.event(0).unwrap();
    (
        fx,
        store,
        TrainerScriptMeta {
            battle_file,
            seen_script: 3003,
        },
    )
}

/// Class 1 („Youngster“ in the fixture), „Jens“ with one Pokémon, the texts of
/// `pt-trainer-begegnung.yaml`, looking north with sight 3.
fn trainer_input(x: u16, z: u16) -> NewTrainerHereInput {
    NewTrainerHereInput {
        header: 0,
        x,
        z,
        dir: 0,
        sight: 3,
        class: 1,
        name: "Jens".into(),
        party: vec![PartyMon {
            species: 25,
            level: 12,
        }],
        texts: BattleTexts {
            before: "Halt! Ich bin Jens!\\r".into(),
            lose: "Du bist stark!".into(),
            after: "Gut gekämpft.".into(),
        },
        graphics_id: None,
    }
}

fn plan_trainer(
    store: &mut Store,
    meta: &TrainerScriptMeta,
    input: &NewTrainerHereInput,
) -> Result<recipes::Draft, KernelError> {
    plan_with(store, input, meta, TRAINER_FLAG_CAPACITY)
}

fn apply_trainer(
    store: &mut Store,
    meta: &TrainerScriptMeta,
    input: &NewTrainerHereInput,
) -> Result<recipes::Outcome<new_trainer_here::NewTrainerHereReport>, KernelError> {
    apply_with(store, input, meta, TRAINER_FLAG_CAPACITY)
}

/// The battle texts of trainer `id` as (kind, text).
fn texts_of(store: &mut Store, id: u16) -> Vec<(u16, String)> {
    let mut names = NameTables::new();
    trainer_texts(store, &mut names, id)
        .unwrap()
        .into_iter()
        .map(|t| (t.kind, t.text))
        .collect()
}

/// The party of trainer `id` as (species, level).
fn party_of(store: &mut Store, id: u16) -> Vec<(u16, u16)> {
    store
        .trainer(id)
        .unwrap()
        .party
        .iter()
        .map(|m| (m.species(), m.level))
        .collect()
}

fn trainer_count(store: &Store) -> usize {
    store.member_count(AssetKind::Trainer).unwrap()
}

#[test]
fn plan_of_a_trainer_names_the_next_id_the_flag_and_three_messages() {
    let (fx, mut store, meta) = trainer_fixture();
    assert_eq!(
        bank_index(
            store.project().family,
            store.project().language,
            Bank::BattleTexts
        ),
        Some(BATTLE_TEXTS)
    );
    let (depth, dirty, count) = (store.undo_depth(), store.dirty(), trainer_count(&store));
    let data = files_below(&fx.root().join("data"));

    let input = trainer_input(40, 7);
    let draft = plan_trainer(&mut store, &meta, &input).unwrap();
    assert_eq!(draft.events.len(), 1);
    let event = &draft.events[0];
    assert_eq!(event.kind, DraftEventKind::Object);
    assert_eq!((event.matrix, event.x, event.z), (Some(0), 40, 7));
    assert_eq!(event.look, Some(20), "Platinum's default trainer look");
    let messages: Vec<(Option<u16>, &str)> = draft
        .messages
        .iter()
        .map(|m| (m.archive, m.text.as_str()))
        .collect();
    assert_eq!(
        messages,
        [
            (Some(BATTLE_TEXTS), "Halt! Ich bin Jens!\\r"),
            (Some(BATTLE_TEXTS), "Du bist stark!"),
            (Some(BATTLE_TEXTS), "Gut gekämpft."),
        ]
    );
    assert_eq!(
        draft.notes[0],
        "Creates trainer “Youngster Jens” as number 3 (next free), party 1 Pokémon, trainer flag 0x0553."
    );
    assert_eq!(draft.irreversible, ["Trainer 3 is created (undo barrier)"]);
    // A text before the battle without \r is a warning, no refusal.
    let mut open = trainer_input(40, 7);
    open.texts.before = "Halt!".into();
    let notes = plan_trainer(&mut store, &meta, &open)
        .unwrap()
        .notes
        .join(" ");
    assert!(notes.contains("\\r"), "{notes}");
    // The given look wins over the default.
    let look = NewTrainerHereInput {
        graphics_id: Some(97),
        ..trainer_input(40, 7)
    };
    assert_eq!(
        plan_trainer(&mut store, &meta, &look).unwrap().events[0].look,
        Some(97)
    );
    // Through the dispatcher, with the battle script of the game's metadata (the fixture has
    // none: the recipe's own checks answer first, the sight default is 3).
    let call = |sight: u16| -> recipes::RecipeCall {
        serde_json::from_value(serde_json::json!({
            "recipe": "new_trainer_here",
            "input": { "header": 0, "x": 40, "z": 7, "dir": 0, "sight": sight, "class": 1,
                       "name": "Jens", "party": [{ "species": 25, "level": 12 }],
                       "texts": { "before": "Halt!\\r", "lose": "a", "after": "b" },
                       "graphics_id": null },
        }))
        .unwrap()
    };
    let why = refused(recipes::plan(&mut store, &call(0)));
    assert!(why.contains("Sight range 0"), "{why}");
    let why = refused(recipes::apply(&mut store, &call(16)));
    assert!(why.contains("Sight range 16"), "{why}");
    let default: NewTrainerHereInput = serde_json::from_value(serde_json::json!({
        "header": 0, "x": 40, "z": 7, "dir": 0, "class": 1, "name": "Jens",
        "party": [{ "species": 25, "level": 12 }],
        "texts": { "before": "", "lose": "", "after": "" }, "graphics_id": null,
    }))
    .unwrap();
    assert_eq!(default.sight, 3);

    // Nothing written.
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(store.dirty(), dirty);
    assert_eq!(trainer_count(&store), count);
    assert_eq!(files_below(&fx.root().join("data")), data);
}

#[test]
fn apply_creates_the_trainer_then_one_step_for_team_texts_and_object() {
    let (_fx, mut store, meta) = trainer_fixture();
    // The session's name tables, read before the recipe: they follow it (generation cache).
    let mut session_names = NameTables::new();
    assert_eq!(session_names.trainer_name(&mut store, 2), "Mia");
    let depth = store.undo_depth();
    let input = trainer_input(40, 7);

    let outcome = apply_trainer(&mut store, &meta, &input).unwrap();
    let report = &outcome.report;
    assert_eq!(
        (
            report.trainer,
            report.placeholder,
            report.script_extended,
            report.event_file,
            report.object,
            report.script,
            report.trainer_flag
        ),
        (3, None, false, 0, 0, 3002, 0x553)
    );
    assert!(
        report.text_problems.is_empty(),
        "{:?}",
        report.text_problems
    );
    assert_eq!(
        outcome.created,
        [CreatedObject {
            kind: AssetKind::Trainer,
            index: 3
        }]
    );
    assert!(outcome.change.is_some());
    for id in [event_id(0), trainer_id(3), text_id(BATTLE_TEXTS)] {
        assert!(
            outcome.changed.contains(&id),
            "{id:?} in {:?}",
            outcome.changed
        );
    }
    let next: Vec<(&str, bool, Option<&str>)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available, s.reason.as_deref()))
        .collect();
    assert_eq!(next, [("play_here", false, Some("Coming with 3d.9"))]);

    // The trainer: class, name, team, texts.
    assert_eq!(trainer_count(&store), 4);
    assert_eq!(store.trainer(3).unwrap().class, 1);
    assert_eq!(session_names.trainer_name(&mut store, 3), "Jens");
    assert_eq!(party_of(&mut store, 3), [(25, 12)]);
    let texts = vec![
        (0, "Halt! Ich bin Jens!\\r".to_owned()),
        (1, "Du bist stark!".to_owned()),
        (2, "Gut gekämpft.".to_owned()),
    ];
    assert_eq!(texts_of(&mut store, 3), texts);
    // The object: battle script 3000 + id − 1, trainer type 1, sight in data[0].
    let object = store.event(0).unwrap().object_events[0].clone();
    assert_eq!(
        (
            object.x,
            object.z,
            object.script,
            object.trainer_type,
            object.data[0],
            object.dir,
            object.movement_type,
            object.graphics_id
        ),
        (40, 7, 3002, 1, 3, 0, 14, 20)
    );

    // The barrier, then one step.
    assert_eq!(store.undo_depth(), depth + 2);
    let history = store.history().0;
    assert_eq!(
        history[history.len() - 2..],
        ["Trainer 3 created", "Trainer “Jens” placed here"]
    );
    store.undo().unwrap();
    assert!(store.event(0).unwrap().object_events.is_empty());
    assert!(party_of(&mut store, 3).is_empty());
    assert!(texts_of(&mut store, 3).is_empty());
    assert_eq!(trainer_count(&store), 4);
    assert_eq!(session_names.trainer_name(&mut store, 3), "Jens");
    assert_eq!(store.history().0.last().unwrap(), "Trainer 3 created");
    let err = store.undo().unwrap_err();
    assert!(
        matches!(&err, KernelError::Refused(r) if r == "Creating trainer 3 cannot be undone"),
        "{err}"
    );
    store.redo().unwrap();
    assert_eq!(party_of(&mut store, 3), [(25, 12)]);
    assert_eq!(texts_of(&mut store, 3), texts);
    assert_eq!(store.event(0).unwrap().object_events[0].script, 3002);

    // A second trainer on another tile takes the next id (behind the placeholder of the sight
    // slot 4), the next object and local id.
    let second = apply_trainer(&mut store, &meta, &trainer_input(41, 7)).unwrap();
    assert_eq!(
        (
            second.report.trainer,
            second.report.placeholder,
            second.report.object
        ),
        (5, Some(4), 1)
    );
    let objects = store.event(0).unwrap().object_events.clone();
    assert_ne!(objects[0].local_id, objects[1].local_id);
}

#[test]
fn a_save_between_apply_and_undo_takes_back_team_texts_and_object() {
    let (fx, mut store, meta) = trainer_fixture();
    let ids = [
        event_id(0),
        text_id(BATTLE_TEXTS),
        AssetId {
            kind: AssetKind::TrainerTextTable,
            index: 0,
        },
    ];
    let bytes = |store: &mut Store| -> Vec<Option<Vec<u8>>> {
        store.event(0).unwrap();
        store.text_archive(BATTLE_TEXTS).unwrap();
        store.trainer_text_table(0).unwrap();
        ids.iter()
            .map(|&id| store.current_bytes(id).unwrap())
            .collect()
    };
    let before = bytes(&mut store);
    apply_trainer(&mut store, &meta, &trainer_input(40, 7)).unwrap();
    let after = bytes(&mut store);
    store.save().unwrap();
    let saved = files_below(&fx.root().join("data"));

    // Team, texts and object are store parts: undo after the save needs no file.
    store.undo().unwrap();
    assert_eq!(bytes(&mut store), before);
    assert!(party_of(&mut store, 3).is_empty());
    store.save().unwrap();
    assert_ne!(files_below(&fx.root().join("data")), saved);
    store.redo().unwrap();
    assert_eq!(bytes(&mut store), after);
    assert_eq!(party_of(&mut store, 3), [(25, 12)]);
    store.save().unwrap();
    assert_eq!(files_below(&fx.root().join("data")), saved);
}

#[test]
fn a_failed_placement_leaves_the_trainer_and_no_half_step() {
    let (fx, mut store, meta) = trainer_fixture();
    let project = Arc::new(fx.project());
    let input = trainer_input(40, 7);

    // The event file locked after the trainer was created: the transaction fails.
    let err = apply_interrupted(&mut store, &input, &meta, TRAINER_FLAG_CAPACITY, |s| {
        s.lock(event_id(0), "gesperrt im Test")
    })
    .unwrap_err();
    match &err {
        KernelError::Partial(why) => {
            assert!(
                why.starts_with("trainer 0003 was created; placing it failed"),
                "{why}"
            );
        }
        other => panic!("expected Partial, got {other:?}"),
    }
    assert_eq!(trainer_count(&store), 4);
    assert_eq!(
        store.history(),
        (vec!["Trainer 3 created".to_owned()], vec![])
    );
    assert!(store.event(0).unwrap().object_events.is_empty());
    assert!(party_of(&mut store, 3).is_empty());
    assert!(texts_of(&mut store, 3).is_empty());

    // The battle text table locked: team and object were written, the texts fail; the step
    // is rolled back, nothing to redo. A store opened anew takes the next id (5, behind the
    // placeholder of the sight slot 4).
    store.save().unwrap();
    let mut store = Store::new(project.clone());
    store.event(0).unwrap();
    let table = AssetId {
        kind: AssetKind::TrainerTextTable,
        index: 0,
    };
    let err = apply_interrupted(&mut store, &input, &meta, TRAINER_FLAG_CAPACITY, |s| {
        s.lock(table, "gesperrt im Test")
    })
    .unwrap_err();
    assert!(
        matches!(&err, KernelError::Partial(why)
            if why.starts_with("trainers 0004, 0005 were created; placing it failed")),
        "{err:?}"
    );
    assert_eq!(
        store.history(),
        (vec!["Trainer 5 created".to_owned()], vec![])
    );
    assert!(store.event(0).unwrap().object_events.is_empty());
    assert!(party_of(&mut store, 5).is_empty());
    assert!(!store.dirty().contains(&event_id(0)), "{:?}", store.dirty());

    // Without the lock the next call places trainer 6.
    store.save().unwrap();
    let mut store = Store::new(project);
    let outcome = apply_trainer(&mut store, &meta, &input).unwrap();
    assert_eq!(outcome.report.trainer, 6);
    assert_eq!(party_of(&mut store, 6), [(25, 12)]);
}

#[test]
fn is_irreversible_names_trainer_anlegen() {
    let call: recipes::RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "new_trainer_here",
        "input": { "header": 0, "x": 40, "z": 7, "dir": 0, "sight": 2, "class": 1,
                   "name": "Jens", "party": [{ "species": 25, "level": 12 }],
                   "texts": { "before": "Halt!\\r", "lose": "a", "after": "b" },
                   "graphics_id": 20 },
    }))
    .unwrap();
    assert_eq!(
        recipes::is_irreversible(&call).unwrap().as_deref(),
        Some("Create trainer")
    );
    let unreadable: recipes::RecipeCall = serde_json::from_value(serde_json::json!({
        "recipe": "new_trainer_here", "input": { "header": 0 },
    }))
    .unwrap();
    assert!(recipes::is_irreversible(&unreadable).is_err());
}

#[test]
fn the_seen_slot_is_skipped_like_create_trainer() {
    let (fx, mut store, meta) = trainer_fixture();
    let project = Arc::new(fx.project());
    let mut names = NameTables::new();
    let first =
        create_trainer_with(&project, &mut store, &mut names, &meta, 1040, 1, "Kai").unwrap();
    assert_eq!(first.id, 3);

    let input = trainer_input(40, 7);
    let notes = plan_trainer(&mut store, &meta, &input).unwrap().notes;
    assert!(notes[0].contains("as number 5"), "{notes:?}");
    assert!(notes.join(" ").contains("Number 4"), "{notes:?}");
    let outcome = apply_trainer(&mut store, &meta, &input).unwrap();
    let report = &outcome.report;
    assert_eq!(
        (
            report.trainer,
            report.placeholder,
            report.script_extended,
            report.script,
            report.trainer_flag
        ),
        (5, Some(4), true, 3004, 0x555)
    );
    assert_eq!(
        outcome.created,
        [
            CreatedObject {
                kind: AssetKind::Trainer,
                index: 4
            },
            CreatedObject {
                kind: AssetKind::Trainer,
                index: 5
            },
        ]
    );
    assert_eq!(party_of(&mut store, 5), [(25, 12)]);
    assert_eq!(store.event(0).unwrap().object_events[0].script, 3004);
    let history = store.history().0;
    assert_eq!(
        history[history.len() - 2..],
        ["Trainer 5 created", "Trainer “Jens” placed here"]
    );
}

#[test]
fn a_trainer_with_a_bad_team_name_sight_or_tile_is_refused() {
    let (_fx, mut store, meta) = trainer_fixture();
    apply_trainer(&mut store, &meta, &trainer_input(40, 7)).unwrap();
    let (depth, count) = (store.undo_depth(), trainer_count(&store));
    let species = store.member_count(AssetKind::Personal).unwrap();
    let with = |change: &dyn Fn(&mut NewTrainerHereInput)| {
        let mut input = trainer_input(41, 7);
        change(&mut input);
        input
    };
    let mon = |species: u16, level: u8| PartyMon { species, level };
    let cases: Vec<(NewTrainerHereInput, &str)> = vec![
        (trainer_input(40, 7), "object 0 already stands at 40, 7"),
        (with(&|i| i.x = 64), "outside matrix"),
        (with(&|i| i.party.clear()), "1 to 6"),
        (with(&|i| i.party = vec![mon(25, 12); 7]), "1 to 6"),
        (
            with(&|i| i.party = vec![mon(species as u16, 12)]),
            "does not exist",
        ),
        (with(&|i| i.party = vec![mon(25, 0)]), "Level"),
        (with(&|i| i.party = vec![mon(25, 101)]), "Level"),
        (with(&|i| i.sight = 0), "Sight range"),
        (with(&|i| i.sight = 16), "Sight range"),
        (with(&|i| i.dir = 4), "Facing direction"),
        (with(&|i| i.name = "Jens-Peter".into()), "at most 7 fit"),
        (with(&|i| i.name = String::new()), "name"),
    ];
    for (input, expected) in cases {
        let why = refused(plan_trainer(&mut store, &meta, &input));
        assert!(why.contains(expected), "{expected}: {why}");
        assert_eq!(
            refused(apply_trainer(&mut store, &meta, &input)),
            why,
            "{expected}"
        );
    }
    assert_eq!(store.undo_depth(), depth);
    assert_eq!(trainer_count(&store), count);
    assert_eq!(store.event(0).unwrap().object_events.len(), 1);
}

#[test]
fn a_trainer_needs_adopted_scripts() {
    let fx = common::TextFixture::dspre();
    let project = fx.project();
    assert!(!scripts::is_adopted(&project));
    let mut store = Store::new(Arc::new(project));
    let meta = TrainerScriptMeta {
        battle_file: 3,
        seen_script: 3003,
    };
    let input = trainer_input(40, 7);
    assert_eq!(
        refused(plan_trainer(&mut store, &meta, &input)),
        adopt_first()
    );
    assert_eq!(
        refused(apply_trainer(&mut store, &meta, &input)),
        adopt_first()
    );
    assert_eq!(trainer_count(&store), 3);
}

// ---------------------------------------------------------------------------------------
// Gated: a trainer beside the door of the player's house
// ---------------------------------------------------------------------------------------

#[test]
fn vanilla_trainer_here_in_the_town() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    // The start town and the ground floor of the house; the first new trainer id
    // (`tests/trainer_create.rs`: Platinum 928, HeartGold 738).
    let (town, house, expected) = match project.family {
        Family::Pt => (411, 414, 928),
        Family::Hgss => (60, 63, 738),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(project.clone());
    let game = project.game_code.clone();
    let file = store.header_table().unwrap()[usize::from(town)].event_file_id();
    let events = store.event(file).unwrap().clone();
    let door = events
        .warp_events
        .iter()
        .find(|w| w.dest_header_id == house)
        .expect("the town has a door into the house")
        .clone();
    // Class 9 (Wanderer), „Jens“ with Pikachu at level 12 and the texts of the trainer
    // scenarios, looking north with sight 3 as Jens there; the tile: beside the door, the
    // first one the plan takes (the scenarios' tile lies on a place they create).
    let input = |x: u16, z: u16| NewTrainerHereInput {
        header: town,
        x,
        z,
        dir: 0,
        sight: 3,
        class: 9,
        name: "Jens".into(),
        party: vec![PartyMon {
            species: 25,
            level: 12,
        }],
        texts: BattleTexts {
            before: "Halt! Ich bin Jens!\\r".into(),
            lose: "Du bist stark!".into(),
            after: "Gut gekämpft.".into(),
        },
        graphics_id: None,
    };
    let (x, z) = [(2i32, 2i32), (-2, 2), (3, 2), (-3, 2), (2, 3), (-2, 3)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                u16::try_from(i32::from(door.x) + dx).unwrap(),
                u16::try_from(i32::from(door.z) + dz).unwrap(),
            )
        })
        .find(|&(x, z)| new_trainer_here::plan(&mut store, &input(x, z)).is_ok())
        .expect("a free tile in front of the house");
    let draft = new_trainer_here::plan(&mut store, &input(x, z)).unwrap();
    assert!(
        draft.notes[0].contains(&format!("as number {expected} ")),
        "{:?}",
        draft.notes
    );
    let objects = events.object_events.len();
    let event_member = member_on_disk(&store, event_id(file));

    let started = Instant::now();
    let outcome = new_trainer_here::apply(&mut store, &input(x, z)).unwrap();
    let ms = started.elapsed().as_millis();
    let report = outcome.report;
    let id = report.trainer;
    assert_eq!(id, expected, "the next id of vanilla");
    assert_eq!(report.placeholder, None);
    assert_eq!(store.trainer(id).unwrap().class, 9);
    assert_eq!(party_of(&mut store, id), [(25, 12)]);
    assert_eq!(
        texts_of(&mut store, id),
        [
            (0, "Halt! Ich bin Jens!\\r".to_owned()),
            (1, "Du bist stark!".to_owned()),
            (2, "Gut gekämpft.".to_owned()),
        ]
    );
    let object = store.event(file).unwrap().object_events[objects].clone();
    assert_eq!(
        (
            object.x,
            object.z,
            object.script,
            object.trainer_type,
            object.data[0]
        ),
        (x, z, 3000 + id - 1, 1, 3)
    );
    let history = store.history().0;
    assert_eq!(
        history[history.len() - 2..],
        [
            format!("Trainer {id} created"),
            "Trainer “Jens” placed here".to_owned()
        ]
    );

    // Saved, undone, saved: the event file is as before, the trainer stays without a team.
    store.save().unwrap();
    store.undo().unwrap();
    store.save().unwrap();
    assert_eq!(member_on_disk(&store, event_id(file)), event_member);
    let mut again = Store::new(project.clone());
    assert!(party_of(&mut again, id).is_empty());
    assert!(texts_of(&mut again, id).is_empty());
    assert_eq!(again.event(file).unwrap().object_events.len(), objects);
    eprintln!(
        "trainer {game} id={id} tile=({x}, {z}) object={} ms={ms} (unter Last)",
        report.object
    );
}
