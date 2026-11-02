//! The story recipes of phase 3d.7 (Geschichten erzählen): „Neue Geschichte“ (names a variable
//! of the pool `VAR_STORY_…` and makes the story with the step „Anfang“, one undo step), the
//! step recipes that grow a story („Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab
//! Schritt“, „Szene beim Betreten“, „Belohnung einmalig“) and `stories::entities::append_step`,
//! which they share.
//!
//! Synthetic tests run on `test_support::TextFixture::exported()` (the synthetic Platinum
//! project as a text project with the scripts adopted; header 0 has script file 0, event file
//! 0 and text archive 433), as `recipes_erzaehlen.rs` does.
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, NameRequest, name_flag, taken_text};
use g4_kernel::project::{AssetId, AssetKind};
use g4_kernel::recipes::new_story::{self, NewStoryInput};
use g4_kernel::recipes::{self, Draft, RecipeCall, RecipeId, next_steps};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::store::Store;
use g4_kernel::stories::entities::append_step;
use g4_kernel::stories::{Made, Progress, Step, StepKind, duplicate_key_text, read_story};

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
