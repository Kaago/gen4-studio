//! The story recipes of phase 3d.7 (Geschichten erzählen): „Neue Geschichte“ (names a variable
//! of the pool `VAR_STORY_…` and makes the story with the step „Anfang“, one undo step), the
//! step recipes that grow a story („Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab
//! Schritt“, „Szene beim Betreten“, „Belohnung einmalig“) and `stories::entities::append_step`,
//! which they share.
//!
//! Synthetic tests run on `test_support::TextFixture::exported()` (the synthetic Platinum
//! project as a text project with the scripts adopted; header 0 has script file 0, event file
//! 0 and text archive 433), as `recipes_erzaehlen.rs` does; a reward saved between applying and
//! undoing is compared file by file over the whole folder. Gated (`G4_TEST_PROJECT` of the
//! family and the base ROM next to it): a talk in the start town of a vanilla copy set up as
//! text project, its commands per family; prints one line `talk …` each. The story chain on a
//! vanilla copy (Platinum, HeartGold DE and US; the adopted text project is made once per test
//! process): trainer B is placed, saved and committed first (its members stay after undo),
//! then `new_story` → `new_figure` → `talk_advances` → `figure_appears` → `scene_on_enter` →
//! `reward_once` on the start town, one undo step each; git names only the expected paths and
//! is clean after undo to the mark, the pool loses one variable and two flags, the project
//! builds and the checks find no error. Prints `chain …`, `pool …` and `chain-build …`:
//!
//! ```sh
//! G4_TEST_PROJECT=…/_testkopien/3d-7-pt cargo test --release -p g4-kernel --all-features \
//!   --test recipes_story -- vanilla --nocapture --test-threads 1
//! ```
#![cfg(feature = "test-support")]
#![allow(clippy::unwrap_used)]
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::base::BaseRom;
use g4_kernel::data::trainer_create::adopt_first;
use g4_kernel::dialog::{Dialog, DialogKind, DialogSource, dialog_of};
use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, NameRequest, name_flag, taken_text};
use g4_kernel::project::{AssetId, AssetKind, Project};
use g4_kernel::recipes::figure_appears::{self, FigureAppearsInput};
use g4_kernel::recipes::new_figure::{self, NewFigureInput};
use g4_kernel::recipes::new_story::{self, NewStoryInput};
use g4_kernel::recipes::new_trainer_here::{
    self, BattleTexts, NewTrainerHereInput, PartyMon as TrainerPartyMon,
};
use g4_kernel::recipes::reward_once::{self, RewardOnceInput};
use g4_kernel::recipes::scene_on_enter::{self, SceneOnEnterInput};
use g4_kernel::recipes::story_common::FigureRef;
use g4_kernel::recipes::talk_advances::{self, TalkAdvancesInput};
use g4_kernel::recipes::{self, Draft, DraftMessage, RecipeCall, RecipeId, next_steps};
use g4_kernel::scripts::{self, ScriptSource};
use g4_kernel::start::assign_base;
use g4_kernel::store::{MapHeader, ObjectEvent, Store};
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
fn list_names_the_story_recipes_before_the_painted_map() {
    let list = recipes::list();
    assert_eq!(list.len(), 13);
    assert_eq!(list[12].id, RecipeId::NewPaintedMap);
    let shown: Vec<(RecipeId, &str, bool)> = list[7..12]
        .iter()
        .map(|r| (r.id, r.title.as_str(), r.irreversible))
        .collect();
    assert_eq!(
        shown,
        [
            (RecipeId::NewStory, "New story", false),
            (RecipeId::TalkAdvances, "Talk advances the story", false),
            (
                RecipeId::FigureAppears,
                "Figure appears/disappears from step",
                false
            ),
            (RecipeId::SceneOnEnter, "Scene on entering", false),
            (RecipeId::RewardOnce, "One-time reward", false),
        ]
    );
    assert_eq!(
        serde_json::to_value(&RecipeId::all()[7..12]).unwrap(),
        serde_json::json!([
            "new_story",
            "talk_advances",
            "figure_appears",
            "scene_on_enter",
            "reward_once"
        ])
    );
    // Without the project only a visibility and a scene may create a level script file
    // (`is_irreversible_in` decides from their plan); the others never ask. A visibility reads
    // its input first: other input is refused.
    for id in &RecipeId::all()[7..12] {
        let call = RecipeCall {
            recipe: *id,
            input: serde_json::json!({ "name": "Testfaden", "var": POOL_VAR }),
        };
        match id {
            RecipeId::FigureAppears => {
                assert!(refused(recipes::is_irreversible(&call)).starts_with(
                    "The input for “Figure appears/disappears from step” does not fit"
                ),)
            }
            RecipeId::SceneOnEnter => {
                assert!(
                    refused(recipes::is_irreversible(&call))
                        .starts_with("The input for “Scene on entering” does not fit"),
                )
            }
            _ => assert_eq!(recipes::is_irreversible(&call).unwrap(), None, "{id:?}"),
        }
    }
    let visibility = RecipeCall {
        recipe: RecipeId::FigureAppears,
        input: serde_json::json!({
            "story": "STORY_X", "figure": { "header": 0, "local_id": 4 },
            "step": 1, "appears": true, "flag": 256,
        }),
    };
    assert_eq!(
        recipes::is_irreversible(&visibility).unwrap().as_deref(),
        Some("Create level script file")
    );
    let scene = RecipeCall {
        recipe: RecipeId::SceneOnEnter,
        input: serde_json::json!({
            "story": "STORY_X", "header": 0, "step": 1, "text": "Hallo",
        }),
    };
    assert_eq!(
        recipes::is_irreversible(&scene).unwrap().as_deref(),
        Some("Create level script file")
    );
}

#[test]
fn next_steps_after_3d7_free_story_join_and_say_later() {
    let steps = next_steps(&[
        "story_join",
        "talk_advances",
        "figure_appears",
        "play_here",
        "adjust_terrain",
        "nonsense",
    ]);
    let short: BTreeMap<&str, (bool, Option<&str>)> = steps
        .iter()
        .map(|s| (s.id.as_str(), (s.available, s.reason.as_deref())))
        .collect();
    assert_eq!(short["story_join"], (true, None));
    assert_eq!(short["talk_advances"], (true, None));
    assert_eq!(short["figure_appears"], (true, None));
    assert_eq!(short["play_here"], (false, Some("Comes with 3d.9")));
    assert_eq!(short["adjust_terrain"], (true, None));
    assert_eq!(short["nonsense"], (false, Some("Comes later")));
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
                "Creates story “Testfaden” with the step “Start”.".to_owned(),
                "Names variable 0x4030 as VAR_STORY_TESTFADEN.".to_owned(),
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
            "description": "Progress of the story “Testfaden”",
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
            ..step("s0", 0, StepKind::Start, "Start")
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
        Some("Story “Testfaden” created")
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
        (0x8001, "Variable 0x8001 is not in the save.".to_owned()),
        (
            0x4000,
            "Variable 0x4000 is not in the general area.".to_owned(),
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
        "The story needs a name."
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
        "Names variable 0x4031 as VAR_STORY_TESTFADEN_2."
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
        "Unknown ID: STORY_GIBTS_NICHT."
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
                "Figure 5 gets the branch “from step 0” and advances to 1.".to_owned(),
                format!("Script #3 in script file 0, message {messages} in archive {ARCHIVE}."),
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
    assert!(source.contains("script gespraech_5_3 #3:"), "{source}");
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
            ..step("s1", 1, StepKind::Dialog, "Talk with figure 5")
        }
    );

    // One undo step: undo puts everything back, redo makes it again.
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Talk with figure 5 advances “Testfaden” to 1")
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
    assert!(source.contains("script gespraech_7_3 #3:"), "{source}");
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
    let hull = &source[source.find("script gespraech_5_4 #4:").unwrap()..];
    assert!(
        hull.contains("    if VAR_STORY_TESTFADEN == 1 then\n"),
        "{hull}"
    );
    assert!(hull.ends_with("    Jump gespraech_5_3\n"), "{hull}");
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
fn two_stories_advance_through_the_same_figure() {
    let (_fx, mut store) = talk_fixture();
    new_story::apply(&mut store, &new_story_input("Nebenfaden", POOL_VAR + 1)).unwrap();
    let first = talk_advances::apply(&mut store, &talk(5, 0, "Erste")).unwrap();
    let second = talk_advances::apply(
        &mut store,
        &TalkAdvancesInput {
            story: "STORY_NEBENFADEN".to_owned(),
            ..talk(5, 0, "Zweite")
        },
    )
    .unwrap();
    // Both switch from 0 to 1, each in its own story; the hulls are named after their slots.
    assert_eq!((first.report.slot, second.report.slot), (3, 4));
    assert_eq!(second.report.previous_slot, 3);
    let source = source_of(&store, 0);
    let hull = &source[source.find("script gespraech_5_4 #4:").unwrap()..];
    assert!(
        hull.contains("    if VAR_STORY_NEBENFADEN == 0 then\n"),
        "{hull}"
    );
    assert!(hull.ends_with("    Jump gespraech_5_3\n"), "{hull}");
    assert!(source.contains("script gespraech_5_3 #3:"), "{source}");
    assert_eq!(store.event(0).unwrap().object_events[0].script, 4);
    for (number, var) in [(0, "VAR_STORY_TESTFADEN"), (1, "VAR_STORY_NEBENFADEN")] {
        let story = read_story(store.project(), number).unwrap().unwrap();
        assert_eq!(story.steps[1].key, "s1");
        assert!(
            story.steps[1].uses.contains(&Use::Var(var.to_owned())),
            "{story:?}"
        );
    }
}

/// Sets the story folder and its files read-only (`readonly`) or writable again.
#[cfg(unix)]
fn story_files_readonly(store: &Store, readonly: bool) {
    let dir = store.project().root.join("data/stories");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.push(dir);
    for path in paths {
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(readonly);
        std::fs::set_permissions(&path, permissions).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn a_failure_after_the_compile_takes_everything_back() {
    let (_fx, mut store) = talk_fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let before = state(&store);
    let source = source_of(&store, 0);
    let mark = store.undo_depth();
    let history = store.history();
    // The story can be read but not written: `append_step` fails after the hull compiled.
    story_files_readonly(&store, true);
    let result = talk_advances::apply(&mut store, &talk(5, 0, "Hallo!"));
    story_files_readonly(&store, false);
    assert!(result.is_err(), "{result:?}");
    assert!(
        !matches!(result, Err(KernelError::Refused(_))),
        "{result:?}"
    );
    assert_eq!(state(&store), before);
    assert_eq!(source_of(&store, 0), source);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.history(), history);
    assert_eq!(store.event(0).unwrap().object_events[0].script, 2);
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages.len(),
        messages
    );
    // Applied again, it goes through.
    talk_advances::apply(&mut store, &talk(5, 0, "Hallo!")).unwrap();
    assert_eq!(store.undo_depth(), mark + 1);
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
    let why = "Step 1 already exists: “Talk with figure 5”.";
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
            "Trainers speak through their battle texts (Pokémon area).".to_owned(),
        ),
        (
            talk(9, 0, "Gemeinsam"),
            "Figure 9 speaks through the common script 2000; the recipe needs a script of its own."
                .to_owned(),
        ),
        (
            talk(5, 4, "Später"),
            "Step 4 does not exist in “Testfaden”.".to_owned(),
        ),
        (
            talk(12, 0, "Niemand"),
            "Header 0 has no figure 12.".to_owned(),
        ),
        (talk(5, 0, "  \n"), "The talk needs a text.".to_owned()),
        (
            TalkAdvancesInput {
                story: "STORY_GIBTS_NICHT".to_owned(),
                ..talk(5, 0, "x")
            },
            "Unknown ID: STORY_GIBTS_NICHT.".to_owned(),
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
        adopt_first()
    );
    assert_eq!(store.undo_depth(), mark);
}

// ---------------------------------------------------------------------------------------
// „Belohnung einmalig“
// ---------------------------------------------------------------------------------------

/// The first flag of the general area (Platinum) the reward tests take from the pool.
const POOL_FLAG: u16 = 0x0100;

/// The item the reward tests give (the fixture has `PT_ITEM_MEMBERS` items, 0 is none).
const ITEM: u16 = 17;

fn reward(local_id: u16, from_step: u16, text: &str) -> RewardOnceInput {
    RewardOnceInput {
        story: "STORY_TESTFADEN".to_owned(),
        figure: FigureRef {
            header: 0,
            local_id,
        },
        from_step,
        item: ITEM,
        text: text.to_owned(),
        flag: POOL_FLAG,
        step_name: None,
    }
}

fn reward_call() -> RecipeCall {
    RecipeCall {
        recipe: RecipeId::RewardOnce,
        input: serde_json::json!({
            "story": "STORY_TESTFADEN",
            "figure": { "header": 0, "local_id": 5 },
            "from_step": 0,
            "item": ITEM,
            "text": "Ein Geschenk!",
            "flag": POOL_FLAG,
        }),
    }
}

#[test]
fn apply_of_a_reward_names_the_flag_wraps_the_script_and_lists_the_flag_in_the_story() {
    let (_fx, mut store) = talk_fixture();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let message = u16::try_from(messages).unwrap();
    let before = state(&store);
    let old_source = source_of(&store, 0);
    let mark = store.undo_depth();
    let flag_id = "FLAG_STORY_TESTFADEN_BELOHNUNG_5";

    // The plan writes nothing and names flag, slot and message.
    let draft = recipes::plan(&mut store, &reward_call()).unwrap();
    assert_eq!(
        draft,
        Draft {
            messages: vec![DraftMessage {
                archive: Some(ARCHIVE),
                text: "Ein Geschenk!".to_owned(),
            }],
            notes: vec![
                format!("Figure 5 gives item {ITEM} once from step 0."),
                format!("Names flag 0x0100 as {flag_id}."),
                format!("Script #3 in script file 0, message {messages} in archive {ARCHIVE}."),
            ],
            ..Draft::default()
        }
    );
    assert_eq!(recipes::is_irreversible(&reward_call()).unwrap(), None);
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);

    let outcome = recipes::apply(&mut store, &reward_call()).unwrap();
    assert_eq!(
        outcome.report,
        serde_json::json!({
            "step": "r256",
            "flag": POOL_FLAG,
            "flag_id": flag_id,
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
    assert_eq!(next, [("play_here", false)]);

    // The flag has its own name and the reward's purpose.
    assert_eq!(
        json(&store, "data/ids/flags.json")[flag_id],
        serde_json::json!(POOL_FLAG)
    );
    assert_eq!(
        json(&store, "data/flags/0256.json"),
        serde_json::json!({
            "id": flag_id,
            "description": "Reward from figure 5 in “Testfaden”",
        })
    );
    // The object points at the hull: from step 0 on and while the flag is clear it gives the
    // item once and sets the flag, then it goes on with the old script.
    assert_eq!(store.event(0).unwrap().object_events[0].script, 3);
    let source = source_of(&store, 0);
    assert!(source.starts_with(&old_source), "{source}");
    let hull = &source[source.find("script belohnung_5_3 #3:").unwrap()..];
    for line in [
        "    if VAR_STORY_TESTFADEN >= 0 then\n".to_owned(),
        format!("        if !{flag_id} then\n"),
        format!("            Message {message}\n"),
        format!("            GiveItem {ITEM}, 1, VAR_RESULT\n"),
        format!("            SetFlag {flag_id}\n"),
    ] {
        assert!(hull.contains(&line), "{line}: {hull}");
    }
    assert!(hull.ends_with("    Jump zwei\n"), "{hull}");
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages.len(), messages + 1);
    assert_eq!(doc.messages[messages].text, "Ein Geschenk!");

    // The story has the step and names the flag.
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "r256"]);
    assert_eq!(
        story.steps[1],
        Step {
            made: Some(Made {
                recipe: "reward_once".to_owned(),
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
                Use::Flag(flag_id.to_owned()),
            ],
            ..step("r256", 0, StepKind::Reward, "Reward from figure 5")
        }
    );
    assert_eq!(story.flags, [flag_id]);
    assert_eq!(
        json(&store, "data/stories/0000.json")["steps"][1]["kind"],
        "reward"
    );

    // One undo step for naming, wrapping and the step: undo takes the name back too.
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Reward from figure 5 in “Testfaden”")
    );
    let after = state(&store);
    store.undo().unwrap();
    assert_eq!(state(&store), before);
    assert_eq!(store.event(0).unwrap().object_events[0].script, 2);
    store.redo().unwrap();
    assert_eq!(state(&store), after);
}

#[test]
fn a_reward_takes_its_step_name_and_ends_for_a_figure_without_a_script() {
    let (_fx, mut store) = talk_fixture();
    let input = RewardOnceInput {
        step_name: Some("Belohnung: Trank".to_owned()),
        ..reward(7, 0, "Nimm das.")
    };
    let outcome = reward_once::apply(&mut store, &input).unwrap();
    assert_eq!(
        (outcome.report.object, outcome.report.previous_slot),
        (2, 0)
    );
    assert_eq!(outcome.report.flag_id, "FLAG_STORY_TESTFADEN_BELOHNUNG_7");
    let source = source_of(&store, 0);
    assert!(source.contains("script belohnung_7_3 #3:"), "{source}");
    assert!(
        source.ends_with("    endif\n    ReleaseAll\n    End\n"),
        "{source}"
    );
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.steps[1].name, "Belohnung: Trank");
}

/// Every file of the project folder by path relative to it, without the build's own record
/// (`.g4/build/state.json`: each save records when it synced the text archives, times and
/// sizes of their files; the content hashes stay).
fn project_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = files_below(root);
    files.remove(Path::new(".g4/build/state.json"));
    files
}

fn differ(
    a: &BTreeMap<PathBuf, Vec<u8>>,
    b: &BTreeMap<PathBuf, Vec<u8>>,
) -> std::collections::BTreeSet<PathBuf> {
    a.keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}

#[test]
fn a_saved_reward_is_undone_byte_equal() {
    let (fx, mut store) = talk_fixture();
    // The figures of the fixture saved: the folder before the reward.
    store.save().unwrap();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let before_files = project_files(fx.root());
    let before = state(&store);

    let outcome = reward_once::apply(&mut store, &reward(5, 0, "Ein Geschenk!")).unwrap();
    store.save().unwrap();
    let saved_files = project_files(fx.root());
    let changed = differ(&saved_files, &before_files);
    for rel in [
        "data/ids/flags.json",
        "data/flags/0256.json",
        "data/stories/0000.json",
    ] {
        assert!(changed.contains(Path::new(rel)), "{rel}: {changed:?}");
    }
    assert!(changed.len() >= 6, "{changed:?}");

    // Undone and saved: script, archive, event file, story and the flag's name as before.
    store.undo().unwrap();
    assert_eq!(store.event(0).unwrap().object_events[0].script, 2);
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages.len(),
        messages
    );
    assert!(!source_of(&store, 0).contains(&format!("#{}:", outcome.report.slot)));
    store.save().unwrap();
    assert_eq!(
        differ(&project_files(fx.root()), &before_files),
        Default::default()
    );
    assert_eq!(state(&store), before);

    // Redone and saved: as after the first save.
    store.redo().unwrap();
    store.save().unwrap();
    assert_eq!(
        differ(&project_files(fx.root()), &saved_files),
        Default::default()
    );
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages[messages].text, "Ein Geschenk!");
    assert_eq!(store.event(0).unwrap().object_events[0].script, 3);
}

#[cfg(unix)]
#[test]
fn a_failed_reward_takes_the_flag_name_back() {
    let (fx, mut store) = talk_fixture();
    store.save().unwrap();
    let before_files = project_files(fx.root());
    let before = state(&store);
    let mark = store.undo_depth();
    let history = store.history();
    // The story can be read but not written: `append_step` fails after the flag was named
    // and the hull compiled.
    story_files_readonly(&store, true);
    let result = reward_once::apply(&mut store, &reward(5, 0, "Ein Geschenk!"));
    story_files_readonly(&store, false);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(state(&store), before);
    // Every file as before, the ids rotom reads included.
    assert_eq!(
        differ(&project_files(fx.root()), &before_files),
        Default::default()
    );
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.history(), history);
    assert!(
        json(&store, "data/ids/flags.json")
            .get("FLAG_STORY_TESTFADEN_BELOHNUNG_5")
            .is_none()
    );
    // Applied again, it goes through with the same name.
    let outcome = reward_once::apply(&mut store, &reward(5, 0, "Ein Geschenk!")).unwrap();
    assert_eq!(outcome.report.flag_id, "FLAG_STORY_TESTFADEN_BELOHNUNG_5");
    assert_eq!(store.undo_depth(), mark + 1);
}

#[test]
fn a_used_flag_number_a_bad_item_and_a_trainer_are_refused() {
    let (_fx, mut store) = talk_fixture();
    name_flag(
        &mut store,
        &NameRequest {
            kind: FlagKind::Flag,
            number: POOL_FLAG + 1,
            id: Some("FLAG_EIGEN".to_owned()),
            description: None,
        },
    )
    .unwrap();
    let before = state(&store);
    let mark = store.undo_depth();
    let items = u16::try_from(common::PT_ITEM_MEMBERS).unwrap();
    let with = |change: &dyn Fn(&mut RewardOnceInput)| {
        let mut input = reward(5, 0, "Ein Geschenk!");
        change(&mut input);
        input
    };
    for (input, why) in [
        (
            with(&|i| i.flag = POOL_FLAG + 1),
            taken_text(
                "FLAG_EIGEN",
                g4_kernel::flags::TakenBy::Register(u32::from(POOL_FLAG + 1)),
            ),
        ),
        (
            with(&|i| i.flag = 0x0010),
            "Flag 0x0010 is not in the general area.".to_owned(),
        ),
        (
            with(&|i| i.flag = 0x0B60),
            "Flag 0x0B60 is not in the save.".to_owned(),
        ),
        (with(&|i| i.item = 0), "Item 0 does not exist.".to_owned()),
        (
            with(&|i| i.item = items),
            format!("Item {items} does not exist."),
        ),
        (
            reward(8, 0, "Kampf!"),
            "Trainers speak through their battle texts (Pokémon area).".to_owned(),
        ),
        (
            reward(9, 0, "Gemeinsam"),
            "Figure 9 speaks through the common script 2000; the recipe needs a script of its own."
                .to_owned(),
        ),
        (
            reward(5, 3, "Später"),
            "Step 3 does not exist in “Testfaden”.".to_owned(),
        ),
        (reward(5, 0, "  \n"), "The reward needs a text.".to_owned()),
        (
            with(&|i| i.story = "STORY_GIBTS_NICHT".to_owned()),
            "Unknown ID: STORY_GIBTS_NICHT.".to_owned(),
        ),
    ] {
        assert_eq!(
            refused(reward_once::plan(&mut store, &input)),
            why,
            "{input:?}"
        );
        assert_eq!(
            refused(reward_once::apply(&mut store, &input)),
            why,
            "{input:?}"
        );
    }
    assert_eq!(state(&store), before);
    assert_eq!(store.undo_depth(), mark);
    // The highest item is given.
    let highest = with(&|i| i.item = items - 1);
    assert!(reward_once::plan(&mut store, &highest).is_ok());
}

#[test]
fn reward_after_talk_chains_both_wrappers() {
    let (_fx, mut store) = talk_fixture();
    let talked = talk_advances::apply(&mut store, &talk(5, 0, "Erst reden")).unwrap();
    let rewarded = reward_once::apply(&mut store, &reward(5, 1, "Dann schenken")).unwrap();
    assert_eq!(
        (rewarded.report.slot, rewarded.report.previous_slot),
        (talked.report.slot + 1, talked.report.slot)
    );
    // The object points at the reward's hull, which goes on with the talk's hull, that with
    // `zwei`.
    assert_eq!(
        store.event(0).unwrap().object_events[0].script,
        rewarded.report.slot
    );
    let source = source_of(&store, 0);
    let hull = &source[source.find("script belohnung_5_4 #4:").unwrap()..];
    assert!(
        hull.contains("    if VAR_STORY_TESTFADEN >= 1 then\n"),
        "{hull}"
    );
    assert!(hull.ends_with("    Jump gespraech_5_3\n"), "{hull}");
    let talk_hull = &source[source.find("script gespraech_5_3 #3:").unwrap()..];
    assert!(talk_hull.contains("    Jump zwei\n"), "{talk_hull}");
    // The dialog shows both messages and the old one.
    let dialog = dialog_of(&mut store, 0, 0, DialogKind::Object, 0).unwrap();
    assert!(
        matches!(dialog.source, DialogSource::Local { slot: 4, .. }),
        "{dialog:?}"
    );
    let shown: Vec<u16> = dialog.messages.iter().map(|m| m.index).collect();
    assert_eq!(shown, [rewarded.report.message, talked.report.message, 0]);
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1", "r256"]);
    assert_eq!(story.steps[2].value, 1);
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
    let start = source.find(&format!(
        "script gespraech_{local_id}_{0} #{0}:",
        report.slot
    ))?;
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

// ---------------------------------------------------------------------------------------
// „Figur erscheint/verschwindet ab Schritt“
// ---------------------------------------------------------------------------------------

/// The level script file of header 0 of the fixture.
const LEVEL: u16 = 1;

/// A second flag of the general area (Platinum).
const POOL_FLAG_2: u16 = 0x0101;

/// The id the visibility flag of figure 4 in „Testfaden“ gets.
const FLAG_4: &str = "FLAG_STORY_TESTFADEN_SICHTBAR_4";

/// [`talk_fixture`] with figure 4 without a script (object 5) and the dialog step `s1` (value
/// 1) in „Testfaden“.
fn appear_fixture() -> (common::TextFixture, Store) {
    let (fx, mut store) = talk_fixture();
    store
        .transaction("Figur 4", |tx| {
            tx.edit_event(0, |f| {
                f.object_events.push(object(4, 0, 0));
                Ok(())
            })
        })
        .unwrap();
    append_step(
        &mut store,
        "STORY_TESTFADEN",
        step("s1", 1, StepKind::Dialog, "Gespräch"),
        &[],
    )
    .unwrap();
    (fx, store)
}

fn appear(local_id: u16, step: u16, appears: bool, flag: u16) -> FigureAppearsInput {
    FigureAppearsInput {
        story: "STORY_TESTFADEN".to_owned(),
        figure: FigureRef {
            header: 0,
            local_id,
        },
        step,
        appears,
        flag,
        step_name: None,
    }
}

fn appear_call(input: &FigureAppearsInput) -> RecipeCall {
    RecipeCall {
        recipe: RecipeId::FigureAppears,
        input: serde_json::json!({
            "story": input.story,
            "figure": { "header": input.figure.header, "local_id": input.figure.local_id },
            "step": input.step,
            "appears": input.appears,
            "flag": input.flag,
        }),
    }
}

/// Level script file `file` as JSON.
fn level_json(store: &Store, file: u16) -> serde_json::Value {
    match scripts::read_source(store.project(), file).unwrap() {
        ScriptSource::LevelScript(doc) => serde_json::to_value(&doc).unwrap(),
        other => panic!("script file {file} is no level script: {other:?}"),
    }
}

/// Writes `json` as level script file `file` and compiles.
fn set_level(store: &Store, file: u16, json: serde_json::Value) {
    let doc = scripts::LevelScriptDoc::from_json(&json.to_string()).unwrap();
    scripts::write_source(store.project(), file, &ScriptSource::LevelScript(doc)).unwrap();
    scripts::compile(store.project()).unwrap();
}

/// The level script JSON with `entries` and no conditions.
fn entries(entries: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "header_entries": entries, "var_conditions": [] })
}

/// [`state`] and the level script file of header 0.
fn appear_state(store: &Store) -> (State, scripts::ScriptSnapshot) {
    (
        state(store),
        scripts::snapshot_script(store.project(), LEVEL).unwrap(),
    )
}

/// The block of story 0 at header `header` as the recipe writes it: slot `#n`, the rules, the
/// tail line.
fn block(header: u16, n: u16, rules: &str, tail: &str) -> String {
    format!(
        "script sichtbarkeit_0_{header} #{n}:\n{rules}    {tail}\n// ende sichtbarkeit_0_{header}\n"
    )
}

fn appear_rule(step: u16, flag: &str) -> String {
    format!(
        "    if VAR_STORY_TESTFADEN >= {step} then\n        ClearFlag {flag}\n    else\n        \
         SetFlag {flag}\n    endif\n"
    )
}

fn vanish_rule(step: u16, flag: &str) -> String {
    format!(
        "    if VAR_STORY_TESTFADEN >= {step} then\n        SetFlag {flag}\n    else\n        \
         ClearFlag {flag}\n    endif\n"
    )
}

fn set_header_level_script(store: &mut Store, header: u16, file: u16) {
    store
        .transaction("Level-Script", |tx| {
            tx.edit_header(header, |h| {
                match h {
                    MapHeader::DP(h) => h.level_script_id = file,
                    MapHeader::Pt(h) => h.level_script_id = file,
                    MapHeader::HGSS(h) => h.level_script_id = file,
                }
                Ok(())
            })
        })
        .unwrap();
}

#[test]
fn apply_of_an_appearance_sets_hidden_flag_names_the_flag_and_adds_the_block() {
    let (_fx, mut store) = appear_fixture();
    assert_eq!(level_json(&store, LEVEL), entries(serde_json::json!([])));
    let before = appear_state(&store);
    let old_source = source_of(&store, 0);
    let mark = store.undo_depth();
    let call = appear_call(&appear(4, 1, true, POOL_FLAG));

    // The plan writes nothing; the header's own level script is changed in place.
    let draft = recipes::plan(&mut store, &call).unwrap();
    assert_eq!(
        draft,
        Draft {
            notes: vec![
                format!("Figure 4 gets the visibility flag {FLAG_4} and appears from step 1."),
                "Block sichtbarkeit_0_0 in script file 0.".to_owned(),
            ],
            ..Draft::default()
        }
    );
    assert_eq!(
        recipes::is_irreversible_in(&mut store, &call).unwrap(),
        None
    );
    // Without the project it may create a level script file.
    assert_eq!(
        recipes::is_irreversible(&call).unwrap().as_deref(),
        Some(recipes::level_script_file().as_str())
    );
    assert_eq!(appear_state(&store), before);
    assert_eq!(store.undo_depth(), mark);

    let outcome = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(
        outcome.report,
        serde_json::json!({
            "step": "a256",
            "flag": POOL_FLAG,
            "flag_id": FLAG_4,
            "event_file": 0,
            "object": 5,
            "level_script": LEVEL,
            "level_script_created": false,
            "script_file": 0,
            "slot": 3,
            "rules": 1,
        })
    );
    assert!(outcome.created.is_empty());
    assert!(outcome.change.is_some());
    let next: Vec<(&str, bool)> = outcome
        .next
        .iter()
        .map(|s| (s.id.as_str(), s.available))
        .collect();
    assert_eq!(next, [("play_here", false)]);

    // The figure hides behind the flag, which has its own name.
    assert_eq!(
        store.event(0).unwrap().object_events[5].hidden_flag,
        POOL_FLAG
    );
    assert_eq!(
        json(&store, "data/ids/flags.json")[FLAG_4],
        serde_json::json!(POOL_FLAG)
    );
    assert_eq!(
        json(&store, "data/flags/0256.json"),
        serde_json::json!({
            "id": FLAG_4,
            "description": "Visibility of figure 4 in “Testfaden”",
        })
    );
    // The block is appended; without an on_transition entry before it ends.
    let source = source_of(&store, 0);
    assert_eq!(
        source,
        format!(
            "{old_source}\n{}",
            block(0, 3, &appear_rule(1, FLAG_4), "End")
        )
    );
    // The level script starts it on entering; the compile wrote its member.
    assert_eq!(
        level_json(&store, LEVEL),
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 3 }]))
    );
    let level_after = scripts::snapshot_script(store.project(), LEVEL).unwrap();
    assert_ne!(level_after.member, before.1.member);

    // The story has the step and names the flag.
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1", "a256"]);
    assert_eq!(
        story.steps[2],
        Step {
            made: Some(Made {
                recipe: "figure_appears".to_owned(),
                studio: env!("CARGO_PKG_VERSION").to_owned(),
            }),
            uses: vec![
                Use::Object {
                    file: 0,
                    local_id: 4
                },
                Use::Flag(FLAG_4.to_owned()),
                Use::Script { file: 0, slot: 3 },
            ],
            ..step(
                "a256",
                1,
                StepKind::Appearance,
                "Figure 4 appears from step 1"
            )
        }
    );
    assert_eq!(story.flags, [FLAG_4]);
    assert_eq!(
        json(&store, "data/stories/0000.json")["steps"][2]["kind"],
        "appearance"
    );

    // One undo step: undo puts everything back, the level script too; redo makes it again.
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Figure 4 appears from step 1 in “Testfaden”")
    );
    let after = appear_state(&store);
    store.undo().unwrap();
    assert_eq!(appear_state(&store), before);
    assert_eq!(store.event(0).unwrap().object_events[5].hidden_flag, 0);
    store.redo().unwrap();
    assert_eq!(appear_state(&store), after);
}

#[test]
fn a_previous_transition_script_becomes_the_jump_target() {
    let (_fx, mut store) = appear_fixture();
    set_level(
        &store,
        LEVEL,
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 2 }])),
    );
    let before = appear_state(&store);
    let outcome = figure_appears::apply(&mut store, &appear(4, 1, true, POOL_FLAG)).unwrap();
    assert_eq!((outcome.report.slot, outcome.report.rules), (3, 1));
    let source = source_of(&store, 0);
    assert!(
        source.ends_with(&block(0, 3, &appear_rule(1, FLAG_4), "Jump zwei")),
        "{source}"
    );
    // The entry now starts the block, which goes on with the old script.
    assert_eq!(
        level_json(&store, LEVEL),
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 3 }]))
    );
    store.undo().unwrap();
    assert_eq!(appear_state(&store), before);
    assert_eq!(
        level_json(&store, LEVEL),
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 2 }]))
    );
}

#[test]
fn a_second_rule_grows_the_block_and_undo_removes_it() {
    let (_fx, mut store) = appear_fixture();
    set_level(
        &store,
        LEVEL,
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 2 }])),
    );
    let mark = store.undo_depth();
    figure_appears::apply(&mut store, &appear(4, 1, true, POOL_FLAG)).unwrap();
    let first = appear_state(&store);
    let first_source = source_of(&store, 0);
    let first_hidden: Vec<u16> = store
        .event(0)
        .unwrap()
        .object_events
        .iter()
        .map(|o| o.hidden_flag)
        .collect();

    // Figure 7 vanishes from step 1 on: a second rule of the same block.
    let flag_7 = "FLAG_STORY_TESTFADEN_SICHTBAR_7";
    let draft = figure_appears::plan(&mut store, &appear(7, 1, false, POOL_FLAG_2)).unwrap();
    assert_eq!(
        draft.notes,
        [
            format!("Figure 7 gets the visibility flag {flag_7} and vanishes from step 1."),
            "Block sichtbarkeit_0_0 in script file 0.".to_owned(),
        ]
    );
    assert!(draft.irreversible.is_empty());
    let outcome = figure_appears::apply(&mut store, &appear(7, 1, false, POOL_FLAG_2)).unwrap();
    assert_eq!(
        serde_json::to_value(&outcome.report).unwrap(),
        serde_json::json!({
            "step": "a257",
            "flag": POOL_FLAG_2,
            "flag_id": flag_7,
            "event_file": 0,
            "object": 2,
            "level_script": LEVEL,
            "level_script_created": false,
            "script_file": 0,
            "slot": 3,
            "rules": 2,
        })
    );
    // The block grows by the rule above its `Jump` line, nothing else of the source changes.
    let source = source_of(&store, 0);
    let tail = "    Jump zwei\n// ende sichtbarkeit_0_0\n";
    assert_eq!(
        source,
        first_source.replace(tail, &format!("{}{tail}", vanish_rule(1, flag_7)))
    );
    assert_eq!(source.lines().count(), first_source.lines().count() + 5);
    assert!(
        source.ends_with(&block(
            0,
            3,
            &format!("{}{}", appear_rule(1, FLAG_4), vanish_rule(1, flag_7)),
            "Jump zwei"
        )),
        "{source}"
    );
    // The level script stays as it is (its compile-state entry only gets a new time); the
    // compile ran (the script file's member changed).
    let level = appear_state(&store).1;
    assert_eq!(
        (&level.json, &level.member),
        (&first.1.json, &first.1.member)
    );
    assert_ne!(appear_state(&store).0.script.member, first.0.script.member);
    assert_eq!(
        store.event(0).unwrap().object_events[2].hidden_flag,
        POOL_FLAG_2
    );
    assert_eq!(store.undo_depth(), mark + 2);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Figure 7 vanishes from step 1 in “Testfaden”")
    );

    // Undo takes back only the second rule, its flag's name and its step.
    let second = appear_state(&store);
    store.undo().unwrap();
    assert_eq!(appear_state(&store), first);
    assert_eq!(source_of(&store, 0), first_source);
    let hidden: Vec<u16> = store
        .event(0)
        .unwrap()
        .object_events
        .iter()
        .map(|o| o.hidden_flag)
        .collect();
    assert_eq!(hidden, first_hidden);
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.flags, [FLAG_4]);
    assert!(json(&store, "data/ids/flags.json").get(flag_7).is_none());
    store.redo().unwrap();
    assert_eq!(appear_state(&store), second);
}

#[test]
fn vanish_swaps_set_and_clear() {
    let (_fx, mut store) = appear_fixture();
    let input = FigureAppearsInput {
        step_name: Some("Figur 4 geht".to_owned()),
        ..appear(4, 1, false, POOL_FLAG)
    };
    figure_appears::apply(&mut store, &input).unwrap();
    let source = source_of(&store, 0);
    assert!(
        source.ends_with(&block(0, 3, &vanish_rule(1, FLAG_4), "End")),
        "{source}"
    );
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.steps[2].name, "Figur 4 geht");
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Figure 4 vanishes from step 1 in “Testfaden”")
    );
    // Without a step name: „… verschwindet ab Schritt …“.
    let (_fx, mut store) = appear_fixture();
    figure_appears::apply(&mut store, &appear(4, 0, false, POOL_FLAG)).unwrap();
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.steps[2].name, "Figure 4 vanishes from step 0");
}

#[test]
fn a_figure_with_a_flag_and_a_common_transition_script_are_refused() {
    let (_fx, mut store) = appear_fixture();
    let refuse = |store: &mut Store, input: &FigureAppearsInput| -> String {
        let before = appear_state(store);
        let mark = store.undo_depth();
        let why = refused(figure_appears::plan(store, input));
        assert_eq!(refused(figure_appears::apply(store, input)), why);
        assert_eq!(appear_state(store), before, "{why}");
        assert_eq!(store.undo_depth(), mark, "{why}");
        why
    };

    // The place's transition script is a common script, or no slot of the file.
    for target in [2000, 9] {
        set_level(
            &store,
            LEVEL,
            entries(serde_json::json!([{ "type": "on_transition", "script_id": target }])),
        );
        assert_eq!(
            refuse(&mut store, &appear(4, 1, true, POOL_FLAG)),
            "The transition script of the place is not a slot of its own."
        );
    }
    set_level(&store, LEVEL, entries(serde_json::json!([])));

    // A figure that has a visibility flag already.
    name_flag(
        &mut store,
        &NameRequest {
            kind: FlagKind::Flag,
            number: POOL_FLAG_2,
            id: Some("FLAG_GESEHEN".to_owned()),
            description: None,
        },
    )
    .unwrap();
    store
        .transaction("Flag", |tx| {
            tx.edit_event(0, |f| {
                f.object_events[5].hidden_flag = POOL_FLAG_2;
                Ok(())
            })
        })
        .unwrap();
    assert_eq!(
        refuse(&mut store, &appear(4, 1, true, POOL_FLAG)),
        "Figure 4 already has the visibility flag FLAG_GESEHEN."
    );
    // A missing step, a missing figure, a flag with a name, a taken key.
    assert_eq!(
        refuse(&mut store, &appear(7, 4, true, POOL_FLAG)),
        "Step 4 does not exist in “Testfaden”."
    );
    assert_eq!(
        refuse(&mut store, &appear(42, 1, true, POOL_FLAG)),
        "Header 0 has no figure 42."
    );
    assert!(refuse(&mut store, &appear(7, 1, true, POOL_FLAG_2)).contains("FLAG_GESEHEN"));
    append_step(
        &mut store,
        "STORY_TESTFADEN",
        step("a256", 1, StepKind::Appearance, "Schon da"),
        &[],
    )
    .unwrap();
    assert_eq!(
        refuse(&mut store, &appear(7, 1, true, POOL_FLAG)),
        duplicate_key_text("a256")
    );
}

#[test]
fn a_shared_level_script_is_copied_for_this_header() {
    let (_fx, mut store) = appear_fixture();
    set_level(
        &store,
        LEVEL,
        entries(serde_json::json!([{ "type": "on_load", "script_id": 2 }])),
    );
    // Header 1 uses the same level script file.
    set_header_level_script(&mut store, 1, LEVEL);
    let shared = level_json(&store, LEVEL);
    let source = source_of(&store, 0);
    let n = scripts::next_script_file(store.project()).unwrap();
    let call = appear_call(&appear(4, 1, true, POOL_FLAG));

    let draft = recipes::plan(&mut store, &call).unwrap();
    assert_eq!(
        draft.irreversible,
        [format!("Level script file {n} is created and stays")]
    );
    assert_eq!(
        recipes::is_irreversible_in(&mut store, &call)
            .unwrap()
            .as_deref(),
        Some("Create level script file")
    );

    let mark = store.undo_depth();
    let outcome = recipes::apply(&mut store, &call).unwrap();
    let created: Vec<(AssetKind, u16)> =
        outcome.created.iter().map(|c| (c.kind, c.index)).collect();
    assert_eq!(created, [(AssetKind::Script, n)]);
    assert_eq!(outcome.report["level_script"], serde_json::json!(n));
    assert_eq!(
        outcome.report["level_script_created"],
        serde_json::json!(true)
    );
    let levels = |store: &mut Store| -> Vec<u16> {
        store.header_table().unwrap()[..2]
            .iter()
            .map(MapHeader::level_script_id)
            .collect()
    };
    // This header gets the copy with the entry, header 1 keeps the old file as it was.
    assert_eq!(levels(&mut store), [n, LEVEL]);
    assert_eq!(level_json(&store, LEVEL), shared);
    assert_eq!(
        level_json(&store, n),
        entries(serde_json::json!([
            { "type": "on_load", "script_id": 2 },
            { "type": "on_transition", "script_id": 3 },
        ]))
    );
    assert_eq!(store.undo_depth(), mark + 1);

    // Undo: the header points at the shared file again, the copy stays as it was created.
    store.undo().unwrap();
    assert_eq!(levels(&mut store), [LEVEL, LEVEL]);
    assert_eq!(source_of(&store, 0), source);
    assert_eq!(level_json(&store, n), shared);
    store.redo().unwrap();
    assert_eq!(levels(&mut store), [n, LEVEL]);
    assert_eq!(
        level_json(&store, n),
        entries(serde_json::json!([
            { "type": "on_load", "script_id": 2 },
            { "type": "on_transition", "script_id": 3 },
        ]))
    );
}

#[test]
fn a_header_without_a_level_script_gets_a_new_one() {
    let (_fx, mut store) = appear_fixture();
    // Header 1 (script file 0, event file 0) has level script 0.
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), 0);
    let n = scripts::next_script_file(store.project()).unwrap();
    let input = FigureAppearsInput {
        figure: FigureRef {
            header: 1,
            local_id: 4,
        },
        ..appear(4, 1, true, POOL_FLAG)
    };
    let outcome = figure_appears::apply(&mut store, &input).unwrap();
    assert_eq!(
        (
            outcome.report.level_script,
            outcome.report.level_script_created
        ),
        (n, true)
    );
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), n);
    assert_eq!(
        level_json(&store, n),
        entries(serde_json::json!([{ "type": "on_transition", "script_id": 3 }]))
    );
    assert!(
        source_of(&store, 0).ends_with(&block(1, 3, &appear_rule(1, FLAG_4), "End")),
        "{}",
        source_of(&store, 0)
    );
    store.undo().unwrap();
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), 0);
    assert_eq!(level_json(&store, n), entries(serde_json::json!([])));
}

#[cfg(unix)]
#[test]
fn a_failed_visibility_takes_back_the_block_and_the_level_script() {
    let (_fx, mut store) = appear_fixture();
    let before = appear_state(&store);
    let mark = store.undo_depth();
    let history = store.history();
    // `append_step` fails after both compiles.
    story_files_readonly(&store, true);
    let result = figure_appears::apply(&mut store, &appear(4, 1, true, POOL_FLAG));
    story_files_readonly(&store, false);
    assert!(
        result.is_err() && !matches!(result, Err(KernelError::Refused(_))),
        "{result:?}"
    );
    assert_eq!(appear_state(&store), before);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.history(), history);
    assert_eq!(store.event(0).unwrap().object_events[5].hidden_flag, 0);
    figure_appears::apply(&mut store, &appear(4, 1, true, POOL_FLAG)).unwrap();
    assert_eq!(store.undo_depth(), mark + 1);
}

/// The error of a recipe that failed after it created a level script file: a
/// [`KernelError::Partial`] that names the file, which stays (empty, the header points back at
/// its old level script).
#[cfg(unix)]
fn assert_kept_level_script<T: std::fmt::Debug>(
    store: &mut Store,
    result: Result<T, KernelError>,
    n: u16,
) {
    match result {
        Err(KernelError::Partial(m)) => assert!(
            m.ends_with(&format!(". Level script file {n} was created and stays.")),
            "{m}"
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), 0);
    assert_eq!(level_json(store, n), entries(serde_json::json!([])));
}

#[cfg(unix)]
#[test]
fn a_failed_visibility_names_the_level_script_file_it_created() {
    let (_fx, mut store) = appear_fixture();
    let n = scripts::next_script_file(store.project()).unwrap();
    let mark = store.undo_depth();
    let input = FigureAppearsInput {
        figure: FigureRef {
            header: 1,
            local_id: 4,
        },
        ..appear(4, 1, true, POOL_FLAG)
    };
    // `append_step` fails after the new level script file was created.
    story_files_readonly(&store, true);
    let result = figure_appears::apply(&mut store, &input);
    story_files_readonly(&store, false);
    assert_kept_level_script(&mut store, result, n);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.event(0).unwrap().object_events[5].hidden_flag, 0);
}

#[cfg(unix)]
#[test]
fn a_failed_scene_names_the_level_script_file_it_created() {
    let (_fx, mut store) = talk_fixture();
    // Header 1 (level script 0) takes header 0's text archive, as in the scene tests.
    store
        .transaction("Text archive", |tx| {
            tx.edit_header(1, |h| {
                match h {
                    MapHeader::DP(h) => h.text_archive_id = ARCHIVE,
                    MapHeader::Pt(h) => h.text_archive_id = ARCHIVE,
                    MapHeader::HGSS(h) => h.text_archive_id = ARCHIVE,
                }
                Ok(())
            })
        })
        .unwrap();
    let n = scripts::next_script_file(store.project()).unwrap();
    let mark = store.undo_depth();
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    story_files_readonly(&store, true);
    let result = scene_on_enter::apply(&mut store, &scene(1, 0, SCENE_TEXT));
    story_files_readonly(&store, false);
    assert_kept_level_script(&mut store, result, n);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages.len(),
        messages
    );
}

#[test]
fn a_binary_level_script_is_replaced_without_its_entries_and_says_so() {
    let (_fx, mut store) = appear_fixture();
    // Level script 1 of header 0 loses its source: rotom could not decompile it (binary).
    let json = scripts::source_path(store.project(), LEVEL, scripts::SourceKind::LevelScript);
    std::fs::remove_file(json).unwrap();
    assert!(matches!(
        scripts::read_source(store.project(), LEVEL).unwrap(),
        ScriptSource::Binary { .. }
    ));
    let n = scripts::next_script_file(store.project()).unwrap();
    let draft = recipes::plan(&mut store, &appear_call(&appear(4, 1, true, POOL_FLAG))).unwrap();
    assert_eq!(
        draft.irreversible,
        [format!(
            "Level script file {n} is created and stays; the entries of the binary level script file {LEVEL} are not carried over"
        )]
    );
    let draft = scene_on_enter::plan(&mut store, &scene(0, 1, SCENE_TEXT)).unwrap();
    assert_eq!(
        draft.irreversible,
        [format!(
            "Level script file {n} is created and stays; the entries of the binary level script file {LEVEL} are not carried over"
        )]
    );
}

// ---------------------------------------------------------------------------------------
// „Szene beim Betreten“
// ---------------------------------------------------------------------------------------

/// The text of the scenes of the tests.
const SCENE_TEXT: &str = "Ein Brief liegt auf dem Tisch.";

fn scene(header: u16, step: u16, text: &str) -> SceneOnEnterInput {
    SceneOnEnterInput {
        story: "STORY_TESTFADEN".to_owned(),
        header,
        step,
        text: text.to_owned(),
        step_name: None,
    }
}

fn scene_call(input: &SceneOnEnterInput) -> RecipeCall {
    RecipeCall {
        recipe: RecipeId::SceneOnEnter,
        input: serde_json::json!({
            "story": input.story,
            "header": input.header,
            "step": input.step,
            "text": input.text,
        }),
    }
}

/// The scene `szene_<n> #n` (Platinum) as the recipe appends it: message `message`, then
/// `VAR_STORY_TESTFADEN` to `next`.
fn scene_source(n: u16, message: usize, next: u16) -> String {
    format!(
        "script szene_{n} #{n}:\n    LockAll\n    Message {message}\n    WaitButton\n    \
         CloseMessage\n    SetVarFromValue VAR_STORY_TESTFADEN, {next}\n    ReleaseAll\n    End\n"
    )
}

/// A level script JSON with `entries` and `conditions` (var, value, script).
fn level_with(entries: serde_json::Value, conditions: &[(u16, u16, u16)]) -> serde_json::Value {
    let conditions: Vec<serde_json::Value> = conditions
        .iter()
        .map(|&(var, value, script_id)| {
            serde_json::json!({ "var": var, "value": value, "script_id": script_id })
        })
        .collect();
    serde_json::json!({ "header_entries": entries, "var_conditions": conditions })
}

/// The id the register `data/ids/maps.json` gives header `header`, if any.
fn map_id(store: &Store, header: u16) -> Option<String> {
    let maps: BTreeMap<String, u32> =
        serde_json::from_value(json(store, "data/ids/maps.json")).unwrap();
    maps.into_iter()
        .find(|&(_, n)| n == u32::from(header))
        .map(|(id, _)| id)
}

#[test]
fn a_scene_names_the_place_only_when_the_register_does() {
    let is_map = |u: &Use| matches!(u, Use::Map(_));
    // Named: the step uses the header's id from the register.
    let (_fx, mut store) = talk_fixture();
    let id = map_id(&store, 0).expect("the export names header 0");
    scene_on_enter::apply(&mut store, &scene(0, 0, SCENE_TEXT)).unwrap();
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let maps: Vec<&Use> = story.steps[1].uses.iter().filter(|u| is_map(u)).collect();
    assert_eq!(maps, [&Use::Map(id.clone())]);

    // Unnamed: without a register entry for header 0 (its file then carries the fallback id)
    // the step names no place.
    let (_fx, mut store) = talk_fixture();
    let root = store.project().root.clone();
    let path = root.join("data/ids/maps.json");
    let mut maps: BTreeMap<String, u32> =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    maps.retain(|_, n| *n != 0);
    std::fs::write(&path, serde_json::to_vec_pretty(&maps).unwrap()).unwrap();
    let header = root.join("data/headers/0000.json");
    let text = std::fs::read_to_string(&header).unwrap();
    std::fs::write(&header, text.replacen(&id, "MAP_0000", 1)).unwrap();
    assert_eq!(map_id(&store, 0), None);
    scene_on_enter::apply(&mut store, &scene(0, 0, SCENE_TEXT)).unwrap();
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert!(
        !story.steps[1].uses.iter().any(is_map),
        "{:?}",
        story.steps[1].uses
    );
}

#[test]
fn apply_of_a_scene_adds_a_condition_and_a_script_and_a_step() {
    let (_fx, mut store) = talk_fixture();
    assert_eq!(level_json(&store, LEVEL), entries(serde_json::json!([])));
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let message = u16::try_from(messages).unwrap();
    let before = appear_state(&store);
    let old_source = source_of(&store, 0);
    let mark = store.undo_depth();
    let call = scene_call(&scene(0, 0, SCENE_TEXT));

    // The plan writes nothing; the header's own level script is changed in place.
    let draft = recipes::plan(&mut store, &call).unwrap();
    assert_eq!(
        draft,
        Draft {
            messages: vec![DraftMessage {
                archive: Some(ARCHIVE),
                text: SCENE_TEXT.to_owned(),
            }],
            notes: vec![
                "On entering Header 0 at step 0 the scene plays and advances to 1.".to_owned()
            ],
            ..Draft::default()
        }
    );
    assert_eq!(
        recipes::is_irreversible_in(&mut store, &call).unwrap(),
        None
    );
    assert_eq!(
        recipes::is_irreversible(&call).unwrap().as_deref(),
        Some(recipes::level_script_file().as_str())
    );
    assert_eq!(appear_state(&store), before);
    assert_eq!(store.undo_depth(), mark);

    let outcome = recipes::apply(&mut store, &call).unwrap();
    assert_eq!(
        outcome.report,
        serde_json::json!({
            "step": "s1",
            "value": 1,
            "level_script": LEVEL,
            "level_script_created": false,
            "condition": 0,
            "script_file": 0,
            "slot": 3,
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
    assert_eq!(next, [("play_here", false)]);

    // The scene is appended as a new slot, its text is the archive's new message.
    assert_eq!(
        source_of(&store, 0),
        format!("{old_source}\n{}", scene_source(3, messages, 1))
    );
    let doc = store.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages.len(), messages + 1);
    assert_eq!(doc.messages[messages].text, SCENE_TEXT);
    // The level script runs it while the variable is 0; the compile wrote its member.
    assert_eq!(
        level_json(&store, LEVEL),
        level_with(
            serde_json::json!([{ "type": "on_var_condition" }]),
            &[(POOL_VAR, 0, 3)]
        )
    );
    assert_ne!(
        scripts::snapshot_script(store.project(), LEVEL)
            .unwrap()
            .member,
        before.1.member
    );
    // No figure changes.
    assert_eq!(state(&store).events_store, before.0.events_store);

    // The story has the step, made by this recipe, with what it uses.
    let story = read_story(store.project(), 0).unwrap().unwrap();
    let keys: Vec<&str> = story.steps.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s0", "s1"]);
    assert_eq!(
        story.steps[1],
        Step {
            made: Some(Made {
                recipe: "scene_on_enter".to_owned(),
                studio: env!("CARGO_PKG_VERSION").to_owned(),
            }),
            uses: vec![
                Use::LevelScript {
                    file: LEVEL,
                    condition: 0
                },
                Use::Script { file: 0, slot: 3 },
                Use::Message {
                    archive: ARCHIVE,
                    index: message
                },
                Use::Map(map_id(&store, 0).unwrap()),
                Use::Var("VAR_STORY_TESTFADEN".to_owned()),
            ],
            ..step("s1", 1, StepKind::Dialog, "Scene on entering")
        }
    );
    assert!(story.flags.is_empty());
    assert_eq!(store.undo_depth(), mark + 1);
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Scene on entering Header 0 in “Testfaden”")
    );
}

#[test]
fn a_scene_on_a_shared_level_script_copies_it() {
    let (_fx, mut store) = talk_fixture();
    set_level(
        &store,
        LEVEL,
        entries(serde_json::json!([{ "type": "on_load", "script_id": 2 }])),
    );
    // Header 1 uses the same level script file.
    set_header_level_script(&mut store, 1, LEVEL);
    let shared = level_json(&store, LEVEL);
    let source = source_of(&store, 0);
    let n = scripts::next_script_file(store.project()).unwrap();
    let call = scene_call(&scene(0, 0, SCENE_TEXT));

    let draft = recipes::plan(&mut store, &call).unwrap();
    assert_eq!(
        draft.irreversible,
        [format!("Level script file {n} is created and stays")]
    );
    assert_eq!(
        recipes::is_irreversible_in(&mut store, &call)
            .unwrap()
            .as_deref(),
        Some("Create level script file")
    );

    let mark = store.undo_depth();
    let outcome = recipes::apply(&mut store, &call).unwrap();
    let created: Vec<(AssetKind, u16)> =
        outcome.created.iter().map(|c| (c.kind, c.index)).collect();
    assert_eq!(created, [(AssetKind::Script, n)]);
    assert_eq!(outcome.report["level_script"], serde_json::json!(n));
    assert_eq!(
        outcome.report["level_script_created"],
        serde_json::json!(true)
    );
    let levels = |store: &mut Store| -> Vec<u16> {
        store.header_table().unwrap()[..2]
            .iter()
            .map(MapHeader::level_script_id)
            .collect()
    };
    // This header gets the copy with the condition, header 1 keeps the old file as it was.
    let copy = level_with(
        serde_json::json!([
            { "type": "on_load", "script_id": 2 },
            { "type": "on_var_condition" },
        ]),
        &[(POOL_VAR, 0, 3)],
    );
    assert_eq!(levels(&mut store), [n, LEVEL]);
    assert_eq!(level_json(&store, LEVEL), shared);
    assert_eq!(level_json(&store, n), copy);
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(
        story.steps[1].uses[0],
        Use::LevelScript {
            file: n,
            condition: 0
        }
    );
    assert_eq!(store.undo_depth(), mark + 1);

    // Undo: the header points at the shared file again, the copy stays as it was created.
    store.undo().unwrap();
    assert_eq!(levels(&mut store), [LEVEL, LEVEL]);
    assert_eq!(source_of(&store, 0), source);
    assert_eq!(level_json(&store, n), shared);
    store.redo().unwrap();
    assert_eq!(levels(&mut store), [n, LEVEL]);
    assert_eq!(level_json(&store, n), copy);

    // A header without a level script gets a new one, which is announced too (header 1 takes
    // header 0's text archive: the fixture's archive 0 has no messages to read).
    let (_fx, mut store) = talk_fixture();
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), 0);
    store
        .transaction("Text archive", |tx| {
            tx.edit_header(1, |h| {
                match h {
                    MapHeader::DP(h) => h.text_archive_id = ARCHIVE,
                    MapHeader::Pt(h) => h.text_archive_id = ARCHIVE,
                    MapHeader::HGSS(h) => h.text_archive_id = ARCHIVE,
                }
                Ok(())
            })
        })
        .unwrap();
    let n = scripts::next_script_file(store.project()).unwrap();
    let draft = scene_on_enter::plan(&mut store, &scene(1, 0, SCENE_TEXT)).unwrap();
    assert_eq!(
        draft.irreversible,
        [format!("Level script file {n} is created and stays")]
    );
    let outcome = scene_on_enter::apply(&mut store, &scene(1, 0, SCENE_TEXT)).unwrap();
    assert_eq!(
        (
            outcome.report.level_script,
            outcome.report.level_script_created
        ),
        (n, true)
    );
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), n);
    assert_eq!(
        level_json(&store, n),
        level_with(
            serde_json::json!([{ "type": "on_var_condition" }]),
            &[(POOL_VAR, 0, 3)]
        )
    );
    store.undo().unwrap();
    assert_eq!(store.header_table().unwrap()[1].level_script_id(), 0);
    assert_eq!(level_json(&store, n), entries(serde_json::json!([])));
}

#[test]
fn a_second_scene_for_the_same_step_and_a_taken_value_are_refused() {
    let (_fx, mut store) = talk_fixture();
    let refuse = |store: &mut Store, input: &SceneOnEnterInput| -> String {
        let before = appear_state(store);
        let mark = store.undo_depth();
        let why = refused(scene_on_enter::plan(store, input));
        assert_eq!(refused(scene_on_enter::apply(store, input)), why);
        assert_eq!(appear_state(store), before, "{why}");
        assert_eq!(store.undo_depth(), mark, "{why}");
        why
    };

    // The place has a condition for the story's variable at that value already (made by
    // hand, without a step in the story).
    set_level(
        &store,
        LEVEL,
        level_with(
            serde_json::json!([{ "type": "on_var_condition" }]),
            &[(POOL_VAR, 0, 2)],
        ),
    );
    assert_eq!(
        refuse(&mut store, &scene(0, 0, SCENE_TEXT)),
        "The place already has a scene for step 0."
    );
    // A condition for another value or another variable is no obstacle.
    set_level(
        &store,
        LEVEL,
        level_with(
            serde_json::json!([{ "type": "on_var_condition" }]),
            &[(POOL_VAR, 5, 2), (POOL_VAR + 1, 0, 2)],
        ),
    );
    scene_on_enter::plan(&mut store, &scene(0, 0, SCENE_TEXT)).unwrap();

    // A missing step, an empty text, a missing header.
    assert_eq!(
        refuse(&mut store, &scene(0, 4, SCENE_TEXT)),
        "Step 4 does not exist in “Testfaden”."
    );
    assert_eq!(
        refuse(&mut store, &scene(0, 0, "  ")),
        "The scene needs a text."
    );
    assert!(refuse(&mut store, &scene(9999, 0, SCENE_TEXT)).contains("9999"));

    // The first scene goes through; a second one for the same step is refused by its step.
    scene_on_enter::apply(&mut store, &scene(0, 0, SCENE_TEXT)).unwrap();
    assert_eq!(
        refuse(&mut store, &scene(0, 0, "Noch einmal")),
        "Step 1 already exists: “Scene on entering”."
    );
    // A value a talk takes already.
    let (_fx, mut store) = talk_fixture();
    talk_advances::apply(&mut store, &talk(5, 0, "Hallo!")).unwrap();
    assert_eq!(
        refuse(&mut store, &scene(0, 0, SCENE_TEXT)),
        "Step 1 already exists: “Talk with figure 5”."
    );
}

#[test]
fn undo_of_a_scene_restores_level_script_source_and_story() {
    let (_fx, mut store) = talk_fixture();
    // The place has an entry script and a condition of its own already.
    let level_before = level_with(
        serde_json::json!([
            { "type": "on_transition", "script_id": 2 },
            { "type": "on_var_condition" },
        ]),
        &[(POOL_VAR + 1, 5, 2)],
    );
    set_level(&store, LEVEL, level_before.clone());
    let messages = store.text_archive(ARCHIVE).unwrap().messages.len();
    let before = appear_state(&store);
    let old_source = source_of(&store, 0);
    let story_before = read_story(store.project(), 0).unwrap().unwrap();
    let mark = store.undo_depth();
    let input = SceneOnEnterInput {
        step_name: Some("Szene beim Betreten von Zweiblattdorf".to_owned()),
        ..scene(0, 0, SCENE_TEXT)
    };
    let outcome = scene_on_enter::apply(&mut store, &input).unwrap();
    assert_eq!((outcome.report.condition, outcome.report.slot), (1, 3));
    // The condition is appended, the `on_var_condition` entry is not doubled.
    assert_eq!(
        level_json(&store, LEVEL),
        level_with(
            serde_json::json!([
                { "type": "on_transition", "script_id": 2 },
                { "type": "on_var_condition" },
            ]),
            &[(POOL_VAR + 1, 5, 2), (POOL_VAR, 0, 3)],
        )
    );
    let story = read_story(store.project(), 0).unwrap().unwrap();
    assert_eq!(story.steps[1].name, "Szene beim Betreten von Zweiblattdorf");
    assert_eq!(
        story.steps[1].uses[0],
        Use::LevelScript {
            file: LEVEL,
            condition: 1
        }
    );
    assert_eq!(store.undo_depth(), mark + 1);

    // One undo step takes back the level script (JSON and member), the source, the message
    // and the step; redo makes it all again.
    let after = appear_state(&store);
    store.undo().unwrap();
    assert_eq!(appear_state(&store), before);
    assert_eq!(level_json(&store, LEVEL), level_before);
    assert_eq!(source_of(&store, 0), old_source);
    assert_eq!(
        store.text_archive(ARCHIVE).unwrap().messages.len(),
        messages
    );
    assert_eq!(
        read_story(store.project(), 0).unwrap().unwrap(),
        story_before
    );
    store.redo().unwrap();
    assert_eq!(appear_state(&store), after);
    assert_eq!(
        source_of(&store, 0),
        format!("{old_source}\n{}", scene_source(3, messages, 1))
    );
}

#[cfg(unix)]
#[test]
fn a_failed_scene_takes_back_the_script_and_the_level_script() {
    let (_fx, mut store) = talk_fixture();
    let before = appear_state(&store);
    let mark = store.undo_depth();
    let history = store.history();
    // `append_step` fails after both compiles.
    story_files_readonly(&store, true);
    let result = scene_on_enter::apply(&mut store, &scene(0, 0, SCENE_TEXT));
    story_files_readonly(&store, false);
    assert!(
        result.is_err() && !matches!(result, Err(KernelError::Refused(_))),
        "{result:?}"
    );
    assert_eq!(appear_state(&store), before);
    assert_eq!(store.undo_depth(), mark);
    assert_eq!(store.history(), history);
    scene_on_enter::apply(&mut store, &scene(0, 0, SCENE_TEXT)).unwrap();
    assert_eq!(store.undo_depth(), mark + 1);
}

// ---------------------------------------------------------------------------------------
// Gated: the story chain on a vanilla copy (Platinum, HeartGold DE and US)
// ---------------------------------------------------------------------------------------

/// The isolated `git` with an identity from the environment (as in `tests/stories.rs`);
/// `None` (the test skips) without one.
fn git_cli(home: &Path) -> Option<g4_kernel::git::GitCli> {
    let mut cli = common::GitFixture::cli(home)?;
    for (key, value) in [
        ("GIT_AUTHOR_NAME", "Gen4 Studio Gate"),
        ("GIT_AUTHOR_EMAIL", "gate@gen4.invalid"),
        ("GIT_COMMITTER_NAME", "Gen4 Studio Gate"),
        ("GIT_COMMITTER_EMAIL", "gate@gen4.invalid"),
        ("GIT_CONFIG_COUNT", "2"),
        ("GIT_CONFIG_KEY_0", "maintenance.auto"),
        ("GIT_CONFIG_VALUE_0", "false"),
        ("GIT_CONFIG_KEY_1", "gc.auto"),
        ("GIT_CONFIG_VALUE_1", "0"),
    ] {
        cli.env.push((key.to_owned(), value.to_owned()));
    }
    Some(cli)
}

/// The paths `git status` shows, sorted.
fn git_changed(root: &Path) -> Vec<String> {
    let status = g4_kernel::git::GitRepo::discover(root)
        .unwrap()
        .expect("a repository")
        .status()
        .unwrap();
    let mut paths: Vec<String> = status.paths.iter().map(|p| p.path.clone()).collect();
    paths.sort();
    paths
}

fn commit(project: &Project, cli: &g4_kernel::git::GitCli, message: &str) {
    let commit = g4_kernel::git::commit_all(
        project,
        Some(cli),
        &g4_kernel::git::CommitOptions {
            message: Some(message.to_owned()),
            notes: None,
        },
        &mut |_| {},
    )
    .unwrap();
    assert!(commit.committed, "{commit:?}");
}

/// The free numbers of the pool of `kind`, from an index built anew over `store`.
fn pool_free(store: &Store, kind: FlagKind) -> Vec<u16> {
    let usage = g4_kernel::data::refs::RefIndex::new()
        .usage(store, kind)
        .unwrap();
    g4_kernel::flags::pool(store.project(), &usage, kind)
        .unwrap()
        .free
}

/// A vanilla copy as text project after the story chain: the story, figure A (talks from step
/// 0, rewards from step 2), trainer B (appears from step 1, placed and saved before the
/// chain, in Git when `git`), the scene on the same header (step 1 → 2).
struct VanillaChain {
    _tmp: tempfile::TempDir,
    _home: tempfile::TempDir,
    root: PathBuf,
    project: Arc<Project>,
    cli: Option<g4_kernel::git::GitCli>,
    store: Store,
    game: String,
    header: u16,
    /// The undo depth before the chain.
    mark: usize,
    /// The variable and the two flags the chain took from the pool.
    var: u16,
    flags: [u16; 2],
    /// The pool sizes (variables, flags) before the chain.
    pool_before: (usize, usize),
    /// What the chain wrote, as git shows it after a save (empty without `git`).
    changed: Vec<String>,
}

/// A copy of `G4_TEST_PROJECT` set up as text project with the base ROM next to it and the
/// scripts adopted, made once per test process below `CARGO_TARGET_TMPDIR` (adopting takes
/// minutes under load; as `tests/flags.rs`); each chain copies it. `None` when skipped.
fn adopted_vanilla() -> Option<&'static Path> {
    static ADOPTED: OnceLock<Option<PathBuf>> = OnceLock::new();
    ADOPTED
        .get_or_init(|| {
            let src = common::vanilla_project()?;
            let nds = base_rom(&src)?;
            let folder = src.file_name()?.to_string_lossy().into_owned();
            let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("story-{folder}"));
            let _ = std::fs::remove_dir_all(&root);
            common::copy_dir(&src, &root);
            let started = Instant::now();
            let (assigned, _) = assign_base(&root, &nds).unwrap();
            assert_eq!(assigned.refused, None);
            let base = BaseRom::open(&nds, None).unwrap();
            adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
            eprintln!(
                "adopt {} ms={} (unter Last)",
                base.game_code(),
                started.elapsed().as_millis()
            );
            Some(root)
        })
        .as_deref()
}

/// The chain on a vanilla copy of `G4_TEST_PROJECT` (with the base ROM next to it), set up as
/// text project (in Git when `git`); `None` when skipped (no project, no base ROM, no git,
/// Diamond/Pearl). Asserts one undo step per recipe (`new_trainer_here`: its barrier and one
/// step, before the chain) and prints one line `chain …` with the ms of each recipe.
fn vanilla_chain(git: bool) -> Option<VanillaChain> {
    let src = common::vanilla_project()?;
    let family = Project::open(&src).unwrap().family;
    // The start town and the ground floor of the player's house (as `recipes_erzaehlen.rs`).
    let (header, house) = match family {
        Family::Pt => (411u16, 414u16),
        Family::Hgss => (60, 63),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return None;
        }
    };
    let home = tempfile::tempdir().unwrap();
    let cli = if git {
        Some(git_cli(home.path())?)
    } else {
        None
    };
    let adopted = adopted_vanilla()?;
    let started = Instant::now();
    // The copy for this test (written only there, as `vanilla_copy`).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    common::copy_dir(adopted, &root);
    let project = Arc::new(Project::open(&root).unwrap());
    assert!(scripts::is_adopted(&project));
    if let Some(cli) = &cli {
        let init = g4_kernel::git::git_init(
            &project,
            Some(cli),
            &g4_kernel::git::InitOptions::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(init.committed, "{init:?}");
        assert!(git_changed(&root).is_empty(), "{:?}", git_changed(&root));
    }
    let setup_ms = started.elapsed().as_millis();
    let game = project.game_code.clone();
    let mut store = Store::new(project.clone());

    let file = store.header_table().unwrap()[usize::from(header)].event_file_id();
    let door = store
        .event(file)
        .unwrap()
        .warp_events
        .iter()
        .find(|w| w.dest_header_id == house)
        .expect("the town has a door into the house")
        .clone();
    let near = |offsets: &[(i32, i32)]| -> Vec<(u16, u16)> {
        offsets
            .iter()
            .map(|&(dx, dz)| {
                (
                    u16::try_from(i32::from(door.x) + dx).unwrap(),
                    u16::try_from(i32::from(door.z) + dz).unwrap(),
                )
            })
            .collect()
    };

    // Trainer B, before the chain: its members stay after undo (the barrier), so it is
    // placed, saved and committed first.
    let trainer = |x: u16, z: u16| NewTrainerHereInput {
        header,
        x,
        z,
        dir: 0,
        sight: 3,
        class: 9,
        name: "Bert".into(),
        party: vec![TrainerPartyMon {
            species: 25,
            level: 5,
        }],
        texts: BattleTexts {
            before: "Halt! Ich bin Bert!\\r".into(),
            lose: "Du bist stark!".into(),
            after: "Gut gekämpft.".into(),
        },
        graphics_id: None,
    };
    let (bx, bz) = near(&[(2, 3), (-2, 3), (3, 3), (-3, 3), (2, 4), (-2, 4)])
        .into_iter()
        .find(|&(x, z)| new_trainer_here::plan(&mut store, &trainer(x, z)).is_ok())
        .expect("a free tile for trainer B");
    let depth = store.undo_depth();
    let started = Instant::now();
    let placed = new_trainer_here::apply(&mut store, &trainer(bx, bz)).unwrap();
    let trainer_ms = started.elapsed().as_millis();
    // The barrier „Trainer N angelegt“ and one step.
    assert_eq!(store.undo_depth(), depth + 2);
    let b_local =
        store.event(file).unwrap().object_events[usize::from(placed.report.object)].local_id;
    store.save().unwrap();
    if let Some(cli) = &cli {
        commit(&project, cli, "Trainer B");
        assert!(git_changed(&root).is_empty(), "{:?}", git_changed(&root));
    }

    // The numbers the caller takes from the pool (the index is the session's).
    let free_vars = pool_free(&store, FlagKind::Var);
    let free_flags = pool_free(&store, FlagKind::Flag);
    let pool_before = (free_vars.len(), free_flags.len());
    let var = free_vars[0];
    let flags = [free_flags[0], free_flags[1]];

    let mark = store.undo_depth();
    let mut ms: Vec<(&str, u128)> = Vec::new();
    let mut timed = |name: &'static str, store: &mut Store, run: &mut dyn FnMut(&mut Store)| {
        let depth = store.undo_depth();
        let started = Instant::now();
        run(store);
        ms.push((name, started.elapsed().as_millis()));
        assert_eq!(store.undo_depth(), depth + 1, "{name} is one undo step");
    };

    let mut story = String::new();
    timed("new_story", &mut store, &mut |store| {
        let outcome = new_story::apply(
            store,
            &NewStoryInput {
                name: "Gate".to_owned(),
                var,
                notes: String::new(),
            },
        )
        .unwrap();
        story = outcome.report.story;
    });
    assert_eq!(story, "STORY_GATE");
    let a_tiles = near(&[(1, 1), (-1, 1), (2, 1), (-2, 1), (1, 2), (-1, 2)]);
    let figure = |x: u16, z: u16| NewFigureInput {
        header,
        x,
        z,
        graphics_id: None,
        movement_type: 15,
        dir: 1,
        text: "Ich bin Figur A.".into(),
        hidden_flag: None,
    };
    let (ax, az) = a_tiles
        .into_iter()
        .find(|&(x, z)| new_figure::plan(&mut store, &figure(x, z)).is_ok())
        .expect("a free tile for figure A");
    let mut a_local = 0;
    timed("new_figure", &mut store, &mut |store| {
        a_local = new_figure::apply(store, &figure(ax, az))
            .unwrap()
            .report
            .local_id;
    });
    let a = FigureRef {
        header,
        local_id: a_local,
    };
    timed("talk_advances", &mut store, &mut |store| {
        let report = talk_advances::apply(
            store,
            &TalkAdvancesInput {
                story: story.clone(),
                figure: a,
                from_step: 0,
                text: "Geh zum Haus!".to_owned(),
                step_name: None,
            },
        )
        .unwrap()
        .report;
        assert_eq!((report.step.as_str(), report.value), ("s1", 1));
    });
    timed("figure_appears", &mut store, &mut |store| {
        let report = figure_appears::apply(
            store,
            &FigureAppearsInput {
                story: story.clone(),
                figure: FigureRef {
                    header,
                    local_id: b_local,
                },
                step: 1,
                appears: true,
                flag: flags[0],
                step_name: None,
            },
        )
        .unwrap()
        .report;
        assert_eq!(report.rules, 1);
    });
    timed("scene_on_enter", &mut store, &mut |store| {
        let report = scene_on_enter::apply(
            store,
            &SceneOnEnterInput {
                story: story.clone(),
                header,
                step: 1,
                text: "Ein Brief liegt hier.".to_owned(),
                step_name: None,
            },
        )
        .unwrap()
        .report;
        assert_eq!((report.step.as_str(), report.value), ("s2", 2));
    });
    timed("reward_once", &mut store, &mut |store| {
        reward_once::apply(
            store,
            &RewardOnceInput {
                story: story.clone(),
                figure: a,
                from_step: 2,
                item: 17,
                text: "Nimm diesen Trank!".to_owned(),
                flag: flags[1],
                step_name: None,
            },
        )
        .unwrap();
    });
    assert_eq!(store.undo_depth(), mark + 6);

    store.save().unwrap();
    let changed = if git { git_changed(&root) } else { Vec::new() };
    let times: Vec<String> = ms.iter().map(|(n, t)| format!("{n}_ms={t}")).collect();
    eprintln!(
        "chain {game} header={header} var={var:#06X} flags={:#06X},{:#06X} trainer_b={b_local} \
         figure_a={a_local} setup_ms={setup_ms} new_trainer_here_ms={trainer_ms} {} (unter Last)",
        flags[0],
        flags[1],
        times.join(" ")
    );
    Some(VanillaChain {
        _tmp: tmp,
        _home: home,
        root,
        project,
        cli,
        store,
        game,
        header,
        mark,
        var,
        flags,
        pool_before,
        changed,
    })
}

#[test]
fn vanilla_story_chain_is_five_undo_steps_and_git_status_is_empty_after_undo() {
    let Some(mut chain) = vanilla_chain(true) else {
        return;
    };
    eprintln!("chain {} git: {:?}", chain.game, chain.changed);
    assert!(chain.cli.is_some());
    // Git names only what the chain writes: the story and its register, the variable's and
    // the two flags' own names, the header's event file, script source, level script and
    // text archive (no new level script file: the town owns its own).
    let h = chain.store.header_table().unwrap()[usize::from(chain.header)].clone();
    let mut expected = vec![
        format!("data/events/{:04}.json", h.event_file_id()),
        format!("data/flags/{:04}.json", chain.flags[0]),
        format!("data/flags/{:04}.json", chain.flags[1]),
        "data/ids/flags.json".to_owned(),
        "data/ids/stories.json".to_owned(),
        "data/ids/vars.json".to_owned(),
        "data/stories/0000.json".to_owned(),
        "data/stories/0000.layout.json".to_owned(),
        format!("data/vars/{:04}.json", chain.var),
        format!("expanded/scripts/{:04}.json", h.level_script_id()),
        format!("expanded/scripts/{:04}.rotom", h.script_file_id()),
        format!("expanded/textArchives/{:04}.json", h.text_archive_id()),
    ];
    expected.sort();
    assert_eq!(chain.changed, expected);
    let story = read_story(&chain.project, 0).unwrap().unwrap();
    let steps: Vec<(&str, u16, StepKind)> = story
        .steps
        .iter()
        .map(|s| (s.key.as_str(), s.value, s.kind))
        .collect();
    let (f1, f2) = (chain.flags[0], chain.flags[1]);
    assert_eq!(
        steps,
        [
            ("s0", 0, StepKind::Start),
            ("s1", 1, StepKind::Dialog),
            (format!("a{f1}").as_str(), 1, StepKind::Appearance),
            ("s2", 2, StepKind::Dialog),
            (format!("r{f2}").as_str(), 2, StepKind::Reward),
        ]
    );

    // Undo to the mark: the working tree is the commit again.
    let started = Instant::now();
    while chain.store.undo_depth() > chain.mark {
        chain.store.undo().unwrap();
    }
    let undo_ms = started.elapsed().as_millis();
    chain.store.save().unwrap();
    let after = git_changed(&chain.root);
    eprintln!(
        "chain-undo {} undo_ms={undo_ms} git_after_undo={after:?}",
        chain.game
    );
    assert!(after.is_empty(), "{after:?}");
}

#[test]
fn vanilla_pool_shrinks_by_one_var_and_two_flags() {
    let Some(chain) = vanilla_chain(false) else {
        return;
    };
    let vars = pool_free(&chain.store, FlagKind::Var);
    let flags = pool_free(&chain.store, FlagKind::Flag);
    eprintln!(
        "pool {} vars {} -> {} flags {} -> {}",
        chain.game,
        chain.pool_before.0,
        vars.len(),
        chain.pool_before.1,
        flags.len()
    );
    assert_eq!(vars.len(), chain.pool_before.0 - 1);
    assert_eq!(flags.len(), chain.pool_before.1 - 2);
    assert!(!vars.contains(&chain.var));
    assert!(!flags.contains(&chain.flags[0]) && !flags.contains(&chain.flags[1]));
}

#[test]
fn vanilla_story_chain_builds_and_checks_clean() {
    use g4_kernel::checks::{
        Group, IgnoreSet, Limits, Overlay, Runner, Severity, Snapshot, SnapshotCache,
    };

    let Some(chain) = vanilla_chain(false) else {
        return;
    };
    let project = chain.project.clone();
    let started = Instant::now();
    let compiled = scripts::compile(&project).unwrap();
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    let started_build = Instant::now();
    let built = g4_kernel::build::build(&project, &chain.root.join("build/out.nds")).unwrap();
    let build_ms = started_build.elapsed().as_millis();
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert!(chain.root.join("build/out.nds").is_file());

    let snap = Snapshot::build(
        project.clone(),
        Overlay::default(),
        SnapshotCache::default(),
    );
    let checks_started = Instant::now();
    let report = Runner::run(
        &snap,
        Limits::get(),
        &IgnoreSet::default(),
        &Group::ALL,
        |_, _, _| {},
    );
    let checks_ms = checks_started.elapsed().as_millis();
    let errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.code, f.subject, f.key.clone(), f.message.clone()))
        .collect();
    eprintln!(
        "chain-build {} header={} compile_ms={} build_ms={build_ms} checks_ms={checks_ms} \
         findings={} errors={} skipped={:?}",
        chain.game,
        chain.header,
        started_build.duration_since(started).as_millis(),
        report.findings.len(),
        errors.len(),
        report.skipped
    );
    assert!(errors.is_empty(), "{errors:?}");
}
