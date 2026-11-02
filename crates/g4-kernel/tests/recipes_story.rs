//! The story recipes of phase 3d.7 (Geschichten erzählen): „Neue Geschichte“ (names a variable
//! of the pool `VAR_STORY_…` and makes the story with the step „Anfang“, one undo step), the
//! step recipes that grow a story („Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab
//! Schritt“, „Szene beim Betreten“, „Belohnung einmalig“) and `stories::entities::append_step`,
//! which they share.
//!
//! Synthetic tests run on `test_support::TextFixture::exported()` (the synthetic Platinum
//! project as a text project with the scripts adopted; header 0 has script file 0, event file
//! 0 and text archive 433), as `recipes_erzaehlen.rs` does. Gated (`G4_TEST_PROJECT` of the
//! family and the base ROM next to it): a talk in the start town of a vanilla copy set up as
//! text project, its commands per family; prints one line `talk …` each.
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::base::BaseRom;
use g4_kernel::data::trainer_create::ADOPT_FIRST;
use g4_kernel::dialog::{Dialog, DialogKind, DialogSource, dialog_of};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, NameRequest, name_flag, taken_text};
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::new_story::{self, NewStoryInput};
use g4_kernel::recipes::story_common::FigureRef;
use g4_kernel::recipes::talk_advances::{self, TalkAdvancesInput};
use g4_kernel::recipes::{self, Draft, DraftMessage, RecipeCall, RecipeId, next_steps};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::start::assign_base;
use g4_kernel::store::{ObjectEvent, Store};
use g4_kernel::stories::entities::append_step;
use g4_kernel::stories::{Made, Progress, Step, StepKind, Use, duplicate_key_text, read_story};
use g4_kernel::textproject::adopt::adopt;

/// The text archive of header 0 of the fixture.
const ARCHIVE: u16 = 433;

const SOURCE: &str = "script main #1:\n    End\n\nscript zwei #2:\n    End\n";

/// The first variable of the general area (Platinum).
const POOL_VAR: u16 = 0x4030;

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

/// What the story recipes change: the text project's data (registers, flag and variable
/// entities, stories), the script source and member of header 0, rotom's compile state, the
/// text archive on disk and the store's bytes of the archive and the event file.
#[derive(Debug, PartialEq)]
struct State {
    data: BTreeMap<PathBuf, Vec<u8>>,
    script: scripts::ScriptSnapshot,
    compile_state: Option<Vec<u8>>,
    archive: scripts::ArchiveSnapshot,
    archive_store: Option<Vec<u8>>,
    events_store: Option<Vec<u8>>,
}

fn state(store: &Store) -> State {
    let project = store.project();
    State {
        data: files_below(&project.root.join("data")),
        script: scripts::snapshot_script(project, 0).unwrap(),
        compile_state: scripts::snapshot_compile_state(project).unwrap(),
        archive: scripts::snapshot_archive(project, ARCHIVE).unwrap(),
        archive_store: store.current_bytes(text_id(ARCHIVE)).unwrap(),
        events_store: store.current_bytes(event_id(0)).unwrap(),
    }
}

fn json(store: &Store, rel: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(store.project().root.join(rel)).unwrap()).unwrap()
}

fn new_story_input(name: &str, var: u16) -> NewStoryInput {
    NewStoryInput {
        name: name.to_owned(),
        var,
        notes: String::new(),
    }
}

fn step(key: &str, value: u16, kind: StepKind, name: &str) -> Step {
    Step {
        key: key.to_owned(),
        value,
        kind,
        name: name.to_owned(),
        notes: String::new(),
        made: None,
        uses: Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------
// The list and the next steps
// ---------------------------------------------------------------------------------------

#[test]
fn list_names_twelve_recipes_with_the_story_recipes_last() {
    let list = recipes::list();
    assert_eq!(list.len(), 12);
    let shown: Vec<(RecipeId, &str, bool)> = list[7..]
        .iter()
        .map(|r| (r.id, r.title, r.irreversible))
        .collect();
    assert_eq!(
        shown,
        [
            (RecipeId::NewStory, "Neue Geschichte", false),
            (RecipeId::TalkAdvances, "Gespräch schaltet weiter", false),
            (
                RecipeId::FigureAppears,
                "Figur erscheint/verschwindet ab Schritt",
                false
            ),
            (RecipeId::SceneOnEnter, "Szene beim Betreten", false),
            (RecipeId::RewardOnce, "Belohnung einmalig", false),
        ]
    );
    assert_eq!(
        serde_json::to_value(&RecipeId::all()[7..]).unwrap(),
        serde_json::json!([
            "new_story",
            "talk_advances",
            "figure_appears",
            "scene_on_enter",
            "reward_once"
        ])
    );
    // None of them asks for confirmation before its input is known.
    for id in &RecipeId::all()[7..] {
        let call = RecipeCall {
            recipe: *id,
            input: serde_json::json!({ "name": "Testfaden", "var": POOL_VAR }),
        };
        assert_eq!(recipes::is_irreversible(&call).unwrap(), None, "{id:?}");
    }
}

#[test]
fn next_steps_after_3d7_free_story_join_and_say_later() {
    let steps = next_steps(&[
        ("story_join", "Zu einer Geschichte"),
        ("talk_advances", "Gespräch schaltet weiter"),
        ("figure_appears", "Figur erscheint ab Schritt"),
        ("play_here", "Hier spielen"),
        ("adjust_terrain", "Gelände anpassen"),
        ("nonsense", "Unsinn"),
    ]);
    let short: BTreeMap<&str, (bool, Option<&str>)> = steps
        .iter()
        .map(|s| (s.id.as_str(), (s.available, s.reason.as_deref())))
        .collect();
    assert_eq!(short["story_join"], (true, None));
    assert_eq!(short["talk_advances"], (true, None));
    assert_eq!(short["figure_appears"], (true, None));
    assert_eq!(short["play_here"], (false, Some("Kommt mit 3d.9")));
    assert_eq!(short["adjust_terrain"], (false, Some("Kommt mit Phase 6")));
    assert_eq!(short["nonsense"], (false, Some("Kommt später")));
}

// ---------------------------------------------------------------------------------------
// „Neue Geschichte“
// ---------------------------------------------------------------------------------------

#[test]
fn plan_of_a_new_story_writes_nothing_and_names_var_and_step() {
    let (_fx, mut store) = fixture();
    let before = state(&store);
    let mark = store.undo_depth();
    let draft = new_story::plan(&mut store, &new_story_input("Testfaden", POOL_VAR)).unwrap();
    assert_eq!(
        draft,
        Draft {
            notes: vec![
                "Legt Geschichte „Testfaden“ mit dem Schritt „Anfang“ an.".to_owned(),
                "Benennt Variable 0x4030 als VAR_STORY_TESTFADEN.".to_owned(),
            ],
            ..Draft::default()
        }
    );
    // The same through the dispatcher, with `notes` left out.
    let call = RecipeCall {
        recipe: RecipeId::NewStory,
        input: serde_json::json!({ "name": "Testfaden", "var": POOL_VAR }),
    };
    assert_eq!(recipes::plan(&mut store, &call).unwrap(), draft);
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
}

#[test]
fn apply_of_a_new_story_names_the_var_and_is_one_undo_step() {
    let (_fx, mut store) = fixture();
    let before = state(&store);
    let mark = store.undo_depth();
    let call = RecipeCall {
        recipe: RecipeId::NewStory,
        input: serde_json::json!({
            "name": "Testfaden",
            "var": POOL_VAR,
            "notes": "Ein Faden zum Testen.",
        }),
    };
    let outcome = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(outcome.report["story"], "STORY_TESTFADEN");
    assert_eq!(outcome.report["number"], 0);
    assert_eq!(outcome.report["var"], POOL_VAR);
    assert_eq!(outcome.report["var_id"], "VAR_STORY_TESTFADEN");
    let files: Vec<&str> = outcome.report["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    for rel in [
        "data/ids/vars.json",
        "data/vars/16432.json",
        "data/ids/stories.json",
        "data/stories/0000.json",
        "data/stories/0000.layout.json",
    ] {
        assert!(files.contains(&rel), "{rel}: {files:?}");
    }
    assert!(outcome.created.is_empty());
    assert!(outcome.change.is_some());
    let next: Vec<(&str, bool)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available))
        .collect();
    assert_eq!(next, [("talk_advances", true), ("figure_appears", true)]);

    // The variable has its own name and the story's purpose.
    assert_eq!(
        json(&store, "data/ids/vars.json")["VAR_STORY_TESTFADEN"],
        serde_json::json!(POOL_VAR)
    );
    assert_eq!(
        json(&store, "data/vars/16432.json"),
        serde_json::json!({
            "id": "VAR_STORY_TESTFADEN",
            "description": "Fortschritt der Geschichte „Testfaden“",
        })
    );
    // The story keeps its progress in it and starts with „Anfang“.
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.id, "STORY_TESTFADEN");
    assert_eq!(story.notes, "Ein Faden zum Testen.");
    assert_eq!(
        story.progress,
        Progress::Var("VAR_STORY_TESTFADEN".to_owned())
    );
    assert_eq!(
        story.steps,
        [Step {
            made: Some(Made {
                recipe: "new_story".to_owned(),
                studio: env!("CARGO_PKG_VERSION").to_owned(),
            }),
            ..step("s0", 0, StepKind::Start, "Anfang")
        }]
    );
    assert_eq!(
        json(&store, "data/stories/0000.json")["steps"][0]["kind"],
        "start"
    );

    // One undo step for naming and making.
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Geschichte „Testfaden“ angelegt")
    );
    let after = state(&store);
    store.undo().unwrap();
    assert_eq!(state(&store), before);
    store.redo().unwrap();
    assert_eq!(state(&store), after);
}

#[test]
fn a_var_outside_the_pool_or_with_a_name_is_refused() {
    let (_fx, mut store) = fixture();
    name_flag(
        &mut store,
        &NameRequest {
            kind: FlagKind::Var,
            number: 0x4031,
            id: Some("VAR_EIGEN".to_owned()),
            description: None,
        },
    )
    .unwrap();
    let before = state(&store);
    let mark = store.undo_depth();
    for (var, why) in [
        (
            0x8001,
            "Variable 0x8001 liegt nicht im Spielstand.".to_owned(),
        ),
        (
            0x4000,
            "Variable 0x4000 liegt nicht im allgemeinen Bereich.".to_owned(),
        ),
        (
            0x4031,
            taken_text("VAR_EIGEN", g4_kernel::flags::TakenBy::Register(0x4031)),
        ),
    ] {
        let input = new_story_input("Testfaden", var);
        assert_eq!(
            refused(new_story::plan(&mut store, &input)),
            why,
            "{var:#x}"
        );
        assert_eq!(
            refused(new_story::apply(&mut store, &input)),
            why,
            "{var:#x}"
        );
    }
    assert_eq!(
        refused(new_story::apply(
            &mut store,
            &new_story_input("  ", POOL_VAR)
        )),
        "Die Geschichte braucht einen Namen."
    );
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
}

#[test]
fn a_second_story_with_the_same_name_gets_var_story_x_2() {
    let (_fx, mut store) = fixture();
    let first = new_story::apply(&mut store, &new_story_input("Testfaden", POOL_VAR)).unwrap();
    assert_eq!(first.report.var_id, "VAR_STORY_TESTFADEN");
    let input = new_story_input("Testfaden", POOL_VAR + 1);
    assert_eq!(
        new_story::plan(&mut store, &input).unwrap().notes[1],
        "Benennt Variable 0x4031 als VAR_STORY_TESTFADEN_2."
    );
    let second = new_story::apply(&mut store, &input).unwrap();
    assert_eq!(second.report.var_id, "VAR_STORY_TESTFADEN_2");
    assert_eq!(second.report.story, "STORY_TESTFADEN_2");
    assert_eq!(second.report.number, 1);
    let story = read_story(store.project(), 1).unwrap().unwrap();
    assert_eq!(
        story.progress,
        Progress::Var("VAR_STORY_TESTFADEN_2".to_owned())
    );
}

// ---------------------------------------------------------------------------------------
// Appending a step
// ---------------------------------------------------------------------------------------

#[test]
fn append_step_refuses_a_duplicate_key_and_adds_new_flags_once() {
    let (_fx, mut store) = fixture();
    new_story::apply(&mut store, &new_story_input("Testfaden", POOL_VAR)).unwrap();
    let id = "STORY_TESTFADEN";

    let report = append_step(
        &mut store,
        id,
        step("s1", 1, StepKind::Dialog, "Gespräch"),
        &["FLAG_STORY_TESTFADEN_SICHTBAR_1".to_owned()],
    )
    .unwrap();
    assert_eq!((report.id.as_str(), report.number), (id, 0));
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1"]);
    assert_eq!(story.flags, ["FLAG_STORY_TESTFADEN_SICHTBAR_1"]);

    // A key the story has: refused, nothing written.
    let before = state(&store);
    let mark = store.undo_depth();
    assert_eq!(
        refused(append_step(
            &mut store,
            id,
            step("s1", 2, StepKind::Dialog, "Noch ein Gespräch"),
            &[],
        )),
        duplicate_key_text("s1")
    );
    assert_eq!(
        refused(append_step(
            &mut store,
            "STORY_GIBTS_NICHT",
            step("s2", 2, StepKind::Dialog, "Gespräch"),
            &[],
        )),
        "Unbekannte ID: STORY_GIBTS_NICHT."
    );
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);

    // A flag the story has is not added twice; a new one is, in order.
    append_step(
        &mut store,
        id,
        step("r5", 1, StepKind::Reward, "Belohnung"),
        &[
            "FLAG_STORY_TESTFADEN_SICHTBAR_1".to_owned(),
            "FLAG_STORY_TESTFADEN_BELOHNUNG_1".to_owned(),
        ],
    )
    .unwrap();
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(
        story.flags,
        [
            "FLAG_STORY_TESTFADEN_SICHTBAR_1",
            "FLAG_STORY_TESTFADEN_BELOHNUNG_1"
        ]
    );
    assert_eq!(story.steps.len(), 3);
    assert_eq!(store.undo_depth(), mark + 1);
}

// ---------------------------------------------------------------------------------------
// „Gespräch schaltet weiter“
// ---------------------------------------------------------------------------------------

/// Script file 0 with `zwei #2` showing message 0, the script the figures 5 and 6 share.
const TALK_SOURCE: &str = "script main #1:\n    End\n\nscript zwei #2:\n    LockAll\n    \
                           FacePlayer\n    Message 0\n    WaitButton\n    CloseMessage\n    \
                           ReleaseAll\n    End\n";

fn object(local_id: u16, script: u16, trainer_type: u16) -> ObjectEvent {
    ObjectEvent {
        local_id,
        graphics_id: 3,
        movement_type: 15,
        trainer_type,
        hidden_flag: 0,
        script,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: 30 + local_id,
        z: 7,
        y: 0,
    }
}

/// The fixture with [`TALK_SOURCE`], story „Testfaden“ (`VAR_STORY_TESTFADEN`, step `s0`) and
/// five figures on header 0: 5 and 6 share script #2, 7 has none, 8 is a trainer, 9 speaks
/// through common script 2000 (objects 0 to 4 in that order).
fn talk_fixture() -> (common::TextFixture, Store) {
    let (fx, mut store) = fixture_with(TALK_SOURCE);
    new_story::apply(&mut store, &new_story_input("Testfaden", POOL_VAR)).unwrap();
    store
        .transaction("Figuren", |tx| {
            tx.edit_event(0, |f| {
                f.object_events.extend([
                    object(5, 2, 0),
                    object(6, 2, 0),
                    object(7, 0, 0),
                    object(8, 3000, 1),
                    object(9, 2000, 0),
                ]);
                Ok(())
            })
        })
        .unwrap();
    (fx, store)
}

fn talk(local_id: u16, from_step: u16, text: &str) -> TalkAdvancesInput {
    TalkAdvancesInput {
        story: "STORY_TESTFADEN".to_owned(),
        figure: FigureRef {
            header: 0,
            local_id,
        },
        from_step,
        text: text.to_owned(),
        step_name: None,
    }
}

fn source_of(store: &Store, file: u16) -> String {
    match scripts::read_source(store.project(), file).unwrap() {
        ScriptSource::Rotom(text) => text,
        other => panic!("script file {file} is no rotom source: {other:?}"),
    }
}

#[test]
fn plan_of_a_talk_writes_nothing_and_names_slot_message_and_value() {
    let (_fx, mut store) = talk_fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let before = state(&store);
    let mark = store.undo_depth();
    let draft = talk_advances::plan(&mut store, &talk(5, 0, "Hallo!")).unwrap();
    assert_eq!(
        draft,
        Draft {
            messages: vec![DraftMessage {
                archive: Some(ARCHIVE),
                text: "Hallo!".to_owned(),
            }],
            notes: vec![
                "Figur 5 bekommt den Zweig „ab Schritt 0“ und schaltet auf 1.".to_owned(),
                format!("Script #3 in Scriptdatei 0, Nachricht {messages} in Archiv {ARCHIVE}."),
            ],
            ..Draft::default()
        }
    );
    // The same through the dispatcher, with `step_name` left out.
    let call = RecipeCall {
        recipe: RecipeId::TalkAdvances,
        input: serde_json::json!({
            "story": "STORY_TESTFADEN",
            "figure": { "header": 0, "local_id": 5 },
            "from_step": 0,
            "text": "Hallo!",
        }),
    };
    assert_eq!(recipes::plan(&mut store, &call).unwrap(), draft);
    assert_eq!(recipes::is_irreversible(&call).unwrap(), None);
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
}

#[test]
fn apply_of_a_talk_wraps_the_old_script_and_is_one_undo_step() {
    let (_fx, mut store) = talk_fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let message = u16::try_from(messages).unwrap();
    let before = state(&store);
    let old_source = source_of(&store, 0);
    let mark = store.undo_depth();
    let call = RecipeCall {
        recipe: RecipeId::TalkAdvances,
        input: serde_json::json!({
            "story": "STORY_TESTFADEN",
            "figure": { "header": 0, "local_id": 5 },
            "from_step": 0,
            "text": "Hallo!",
        }),
    };
    let outcome = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(
        outcome.report,
        serde_json::json!({
            "step": "s1",
            "value": 1,
            "event_file": 0,
            "object": 0,
            "script_file": 0,
            "slot": 3,
            "previous_slot": 2,
            "archive": ARCHIVE,
            "message": message,
        })
    );
    assert!(outcome.created.is_empty());
    assert!(outcome.change.is_some());
    let next: Vec<(&str, bool)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available))
        .collect();
    assert_eq!(next, [("figure_appears", true), ("scene_on_enter", true)]);

    // The object points at the hull, which jumps to the old script; the old text stays.
    assert_eq!(store.event(0).unwrap().object_events[0].script, 3);
    let source = source_of(&store, 0);
    assert!(source.starts_with(&old_source), "{source}");
    assert!(source.contains("script gespraech_5_1 #3:"), "{source}");
    assert!(
        source.contains("    if VAR_STORY_TESTFADEN == 0 then\n"),
        "{source}"
    );
    assert!(
        source.contains(&format!("        Message {message}\n")),
        "{source}"
    );
    assert!(
        source.contains("        SetVarFromValue VAR_STORY_TESTFADEN, 1\n"),
        "{source}"
    );
    assert!(source.ends_with("    Jump zwei\n"), "{source}");
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages.len(), messages + 1);
    assert_eq!(doc.messages[messages].text, "Hallo!");
    // The dialog of the figure shows the new message and, through the jump, the old one.
    let dialog = dialog_of(&mut store, 0, 0, DialogKind::Object, 0).unwrap();
    assert!(
        matches!(dialog.source, DialogSource::Local { slot: 3, .. }),
        "{dialog:?}"
    );
    let shown: Vec<u16> = dialog.messages.iter().map(|m| m.index).collect();
    assert_eq!(shown, [message, 0]);

    // The story has the step, made by this recipe, with what it uses.
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1"]);
    assert_eq!(
        story.steps[1],
        Step {
            made: Some(Made {
                recipe: "talk_advances".to_owned(),
                studio: env!("CARGO_PKG_VERSION").to_owned(),
            }),
            uses: vec![
                Use::Object {
                    file: 0,
                    local_id: 5
                },
                Use::Script { file: 0, slot: 3 },
                Use::Message {
                    archive: ARCHIVE,
                    index: message
                },
                Use::Var("VAR_STORY_TESTFADEN".to_owned()),
            ],
            ..step("s1", 1, StepKind::Dialog, "Gespräch mit Figur 5")
        }
    );

    // One undo step: undo puts everything back, redo makes it again.
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Gespräch von Figur 5 schaltet „Testfaden“ auf 1")
    );
    let after = state(&store);
    store.undo().unwrap();
    assert_eq!(state(&store), before);
    assert_eq!(store.event(0).unwrap().object_events[0].script, 2);
    store.redo().unwrap();
    assert_eq!(state(&store), after);
}

#[test]
fn a_figure_without_a_script_gets_a_tail_end() {
    let (_fx, mut store) = talk_fixture();
    let input = TalkAdvancesInput {
        step_name: Some("Erstes Gespräch".to_owned()),
        ..talk(7, 0, "Ich sage sonst nichts.")
    };
    let outcome = talk_advances::apply(&mut store, &input).unwrap();
    assert_eq!((outcome.report.object, outcome.report.slot), (2, 3));
    assert_eq!(outcome.report.previous_slot, 0);
    assert_eq!(store.event(0).unwrap().object_events[2].script, 3);
    let source = source_of(&store, 0);
    assert!(source.contains("script gespraech_7_1 #3:"), "{source}");
    assert!(
        source.ends_with("    endif\n    ReleaseAll\n    End\n"),
        "{source}"
    );
    assert!(!source.contains("Jump"), "{source}");
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.steps[1].name, "Erstes Gespräch");
}

#[test]
fn a_second_talk_wraps_the_first_hull() {
    let (_fx, mut store) = talk_fixture();
    let first = talk_advances::apply(&mut store, &talk(5, 0, "Erste")).unwrap();
    let second = talk_advances::apply(&mut store, &talk(5, 1, "Zweite")).unwrap();
    assert_eq!(
        (
            second.report.slot,
            second.report.previous_slot,
            second.report.step.as_str()
        ),
        (first.report.slot + 1, first.report.slot, "s2")
    );
    assert_eq!(
        store.event(0).unwrap().object_events[0].script,
        second.report.slot
    );
    // The latest branch checks first and goes on with the first hull, that with `zwei`.
    let source = source_of(&store, 0);
    let hull = &source[source.find("script gespraech_5_2 #4:").unwrap()..];
    assert!(
        hull.contains("    if VAR_STORY_TESTFADEN == 1 then\n"),
        "{hull}"
    );
    assert!(hull.ends_with("    Jump gespraech_5_1\n"), "{hull}");
    let shown: Vec<u16> = dialog_of(&mut store, 0, 0, DialogKind::Object, 0)
        .unwrap()
        .messages
        .iter()
        .map(|m| m.index)
        .collect();
    assert_eq!(shown, [second.report.message, first.report.message, 0]);
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1", "s2"]);
}

#[test]
fn a_shared_script_stays_for_the_other_figure() {
    let (_fx, mut store) = talk_fixture();
    let other = dialog_of(&mut store, 0, 0, DialogKind::Object, 1).unwrap();
    assert!(
        matches!(other.source, DialogSource::Local { slot: 2, .. }),
        "{other:?}"
    );
    let old_source = source_of(&store, 0);
    let zwei = &old_source[old_source.find("script zwei #2:").unwrap()..];

    talk_advances::apply(&mut store, &talk(5, 0, "Nur ich.")).unwrap();
    let objects = store.event(0).unwrap().object_events.clone();
    assert_eq!((objects[0].script, objects[1].script), (3, 2));
    // Script #2 is as it was, the other figure's dialog the same: its source, its messages,
    // how many figures share them. Only why a split is refused changes: the message now also
    // stands in the hull's jump target.
    let source = source_of(&store, 0);
    assert!(source.contains(zwei), "{source}");
    let unsplit = |mut dialog: Dialog| {
        for m in &mut dialog.messages {
            assert!(m.split_refusal.take().is_some(), "{m:?}");
        }
        dialog
    };
    assert_eq!(
        unsplit(dialog_of(&mut store, 0, 0, DialogKind::Object, 1).unwrap()),
        unsplit(other)
    );
}

#[test]
fn a_taken_value_is_refused_before_writing() {
    let (_fx, mut store) = talk_fixture();
    talk_advances::apply(&mut store, &talk(5, 0, "Erste")).unwrap();
    let before = state(&store);
    let mark = store.undo_depth();
    let why = "Schritt 1 gibt es schon: „Gespräch mit Figur 5“.";
    assert_eq!(
        refused(talk_advances::plan(&mut store, &talk(7, 0, "Zweite"))),
        why
    );
    assert_eq!(
        refused(talk_advances::apply(&mut store, &talk(7, 0, "Zweite"))),
        why
    );
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.event(0).unwrap().object_events[2].script, 0);

    // A step of another kind that holds the key `s2` leaves value 2 free, but not its key.
    append_step(
        &mut store,
        "STORY_TESTFADEN",
        step("s2", 1, StepKind::Reward, "Von Hand"),
        &[],
    )
    .unwrap();
    let before = state(&store);
    let mark = store.undo_depth();
    assert_eq!(
        refused(talk_advances::apply(&mut store, &talk(7, 1, "Dritte"))),
        duplicate_key_text("s2")
    );
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
}

#[test]
fn a_trainer_a_common_script_and_a_missing_step_are_refused() {
    let (_fx, mut store) = talk_fixture();
    let before = state(&store);
    let mark = store.undo_depth();
    for (input, why) in [
        (
            talk(8, 0, "Kampf!"),
            "Trainer sprechen über ihre Kampftexte (Bereich Pokémon).".to_owned(),
        ),
        (
            talk(9, 0, "Gemeinsam"),
            "Figur 9 spricht über das gemeinsame Script 2000; das Rezept braucht ein eigenes \
             Script."
                .to_owned(),
        ),
        (
            talk(5, 4, "Später"),
            "Schritt 4 gibt es in „Testfaden“ nicht.".to_owned(),
        ),
        (
            talk(12, 0, "Niemand"),
            "Auf Header 0 gibt es keine Figur 12.".to_owned(),
        ),
        (
            talk(5, 0, "  \n"),
            "Das Gespräch braucht einen Text.".to_owned(),
        ),
        (
            TalkAdvancesInput {
                story: "STORY_GIBTS_NICHT".to_owned(),
                ..talk(5, 0, "x")
            },
            "Unbekannte ID: STORY_GIBTS_NICHT.".to_owned(),
        ),
    ] {
        assert_eq!(
            refused(talk_advances::plan(&mut store, &input)),
            why,
            "{input:?}"
        );
        assert_eq!(
            refused(talk_advances::apply(&mut store, &input)),
            why,
            "{input:?}"
        );
    }
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
}

#[test]
fn a_talk_needs_adopted_scripts() {
    let fx = common::TextFixture::dspre();
    let mut store = Store::new(Arc::new(fx.project()));
    let mark = store.undo_depth();
    assert_eq!(
        refused(talk_advances::plan(&mut store, &talk(5, 0, "x"))),
        ADOPT_FIRST
    );
    assert_eq!(store.undo_depth(), mark);
}

/// The base ROM of the vanilla project `project`: the one `suggest_base` finds or
/// `<name>.nds` next to it.
fn base_rom(project: &Path) -> Option<PathBuf> {
    if let Some(nds) = g4_kernel::start::suggest_base(project) {
        return Some(nds);
    }
    let nds = project
        .parent()?
        .join(format!("{}.nds", project.file_name()?.to_str()?));
    if !nds.is_file() {
        eprintln!("skipped: no base ROM next to {}", project.display());
        return None;
    }
    Some(nds)
}

/// What the vanilla talk changes on disk: script file `file`, rotom's compile state, text
/// archive `archive` and the story files below `data/stories`.
#[derive(Debug, PartialEq)]
struct VanillaFiles {
    script: scripts::ScriptSnapshot,
    compile_state: Option<Vec<u8>>,
    archive: scripts::ArchiveSnapshot,
    stories: BTreeMap<PathBuf, Vec<u8>>,
}

fn vanilla_files(store: &Store, file: u16, archive: u16) -> VanillaFiles {
    let project = store.project();
    VanillaFiles {
        script: scripts::snapshot_script(project, file).unwrap(),
        compile_state: scripts::snapshot_compile_state(project).unwrap(),
        archive: scripts::snapshot_archive(project, archive).unwrap(),
        stories: files_below(&project.root.join("data/stories")),
    }
}

/// Gated (`G4_TEST_PROJECT` of `family` and the base ROM next to it): a vanilla copy as text
/// project with the scripts adopted, a story from the pool and a talk on the first figure of
/// the start town that speaks through a local script. Answers the hull's source, or `None`
/// when skipped. Undo puts the script and text files back; prints one line `talk …`.
fn vanilla_talk(family: Family) -> Option<String> {
    let src = common::vanilla_project()?;
    // Asked before the copy: the project of the other family is skipped at once.
    if Project::open(&src).unwrap().family != family {
        eprintln!("skipped: G4_TEST_PROJECT is no {family:?} project");
        return None;
    }
    let nds = base_rom(&src)?;
    let (_tmp, root) = common::vanilla_copy()?;
    let header = match family {
        Family::Pt => 411,
        Family::Hgss => 60,
        Family::Dp => return None,
    };
    let started = Instant::now();
    let (assigned, _) = assign_base(&root, &nds).unwrap();
    assert_eq!(assigned.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    // Adopting the text project adopts the scripts.
    assert!(scripts::is_adopted(&project));
    let setup_ms = started.elapsed().as_millis();
    let mut store = Store::new(project.clone());

    // The first variable of the pool the recipe takes.
    let var = (0x4000u16..0x4200)
        .find(|&v| new_story::plan(&mut store, &new_story_input("Testfaden", v)).is_ok())
        .expect("a variable of the general area");
    new_story::apply(&mut store, &new_story_input("Testfaden", var)).unwrap();

    let file = store.header_table().unwrap()[usize::from(header)].event_file_id();
    let objects = store.event(file).unwrap().object_events.clone();
    let local_id = (0..objects.len())
        .find(|&i| {
            objects[i].trainer_type == 0
                && matches!(
                    dialog_of(
                        &mut store,
                        header,
                        file,
                        DialogKind::Object,
                        u16::try_from(i).unwrap()
                    )
                    .unwrap()
                    .source,
                    DialogSource::Local { .. }
                )
        })
        .map(|i| objects[i].local_id)
        .expect("a figure with a local script");
    let h = store.header_table().unwrap()[usize::from(header)].clone();
    let files = vanilla_files(&store, h.script_file_id(), h.text_archive_id());
    let input = TalkAdvancesInput {
        figure: FigureRef { header, local_id },
        ..talk(local_id, 0, "Gate 3d.7")
    };
    let started = Instant::now();
    let outcome = talk_advances::apply(&mut store, &input).unwrap();
    let apply_ms = started.elapsed().as_millis();
    let report = outcome.report;
    assert_ne!(report.previous_slot, 0);
    let source = source_of(&store, report.script_file);
    let text =
        &store.text_archive(report.archive).unwrap().messages[usize::from(report.message)].text;
    assert_eq!(text, "Gate 3d.7");
    assert_eq!(
        store.event(file).unwrap().object_events[usize::from(report.object)].script,
        report.slot
    );
    let started = Instant::now();
    store.undo().unwrap();
    let undo_ms = started.elapsed().as_millis();
    assert_eq!(
        vanilla_files(&store, report.script_file, report.archive),
        files
    );
    assert_eq!(
        store.event(file).unwrap().object_events[usize::from(report.object)].script,
        report.previous_slot
    );
    eprintln!(
        "talk {} header={header} local_id={local_id} slot={} previous={} message={} \
         setup_ms={setup_ms} apply_ms={apply_ms} undo_ms={undo_ms}",
        project.game_code, report.slot, report.previous_slot, report.message
    );
    let start = source.find(&format!("script gespraech_{local_id}_1 #{}:", report.slot))?;
    Some(source[start..].to_owned())
}

#[test]
fn vanilla_pt_talk_uses_message_and_setvarfromvalue() {
    let Some(hull) = vanilla_talk(Family::Pt) else {
        return;
    };
    assert!(hull.contains("        Message "), "{hull}");
    assert!(hull.contains("        CloseMessage\n"), "{hull}");
    assert!(
        hull.contains("        SetVarFromValue VAR_STORY_TESTFADEN, 1\n"),
        "{hull}"
    );
    assert!(hull.contains("    Jump "), "{hull}");
}

#[test]
fn hgss_talk_uses_npcmsg_and_setvar() {
    let Some(hull) = vanilla_talk(Family::Hgss) else {
        return;
    };
    assert!(hull.contains("        NPCMsg "), "{hull}");
    assert!(hull.contains("        CloseMsg\n"), "{hull}");
    assert!(
        hull.contains("        SetVar VAR_STORY_TESTFADEN, 1\n"),
        "{hull}"
    );
    assert!(
        !hull.contains("SetVarFromValue") && !hull.contains("Message "),
        "{hull}"
    );
    assert!(hull.contains("    Jump "), "{hull}");
}
