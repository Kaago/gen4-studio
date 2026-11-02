#![allow(clippy::unwrap_used)]

use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::flags::{FlagKind, NameRequest, name_flag_files};
use crate::store::HEADER_TABLE;
use crate::test_support::TextFixture;
use crate::text::Severity;
use crate::textproject::canon::to_canonical;
use crate::textproject::codes::{ID_DANGLING_CODE, ID_MISMATCH_CODE, SCHEMA_CODE};
use crate::textproject::ids::unknown_id_text;
use crate::textproject::rename::{NOT_A_TEXT_PROJECT, rename_id, rename_id_files};
use crate::textproject::symbols::SymbolKind;

use super::super::schema::{Position, StepKind, Use, layout_rel, read_story, story_rel};

const REGISTER: &str = "data/ids/stories.json";
const STORY_0: &str = "data/stories/0000.json";
const LAYOUT_0: &str = "data/stories/0000.layout.json";
const STORY_1: &str = "data/stories/0001.json";
const LAYOUT_1: &str = "data/stories/0001.layout.json";
const KAI: &str = "TRAINER_YOUNGSTER_KAI";

fn request(name: &str, progress: Progress) -> StoryRequest {
    StoryRequest {
        name: name.to_owned(),
        progress,
        notes: String::new(),
        steps: Vec::new(),
        flags: Vec::new(),
    }
}

fn testfaden() -> StoryRequest {
    request("Testfaden", Progress::Var("VAR_UNUSED_0x406C".to_owned()))
}

fn step(key: &str, value: u16, kind: StepKind, name: &str, uses: Vec<Use>) -> Step {
    Step {
        key: key.to_owned(),
        value,
        kind,
        name: name.to_owned(),
        notes: String::new(),
        made: None,
        uses,
    }
}

fn refused(result: Result<StoryReport, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

fn json(fx: &TextFixture, rel: &str) -> serde_json::Value {
    serde_json::from_slice(&fx.read(rel)).unwrap()
}

fn store(fx: &TextFixture) -> Store {
    Store::new(Arc::new(fx.project()))
}

#[test]
fn create_writes_register_json_and_empty_layout() {
    let fx = TextFixture::exported();
    // The export writes the register empty.
    assert_eq!(fx.text(REGISTER), "{}\n");
    let report = create_story_files(&fx.project(), &testfaden()).unwrap();
    assert_eq!(report.id, "STORY_TESTFADEN");
    assert_eq!(report.number, 0);
    assert_eq!(report.change, None);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(report.files, paths(&[REGISTER, STORY_0, LAYOUT_0]));

    assert_eq!(
        json(&fx, REGISTER),
        serde_json::json!({"STORY_TESTFADEN": 0})
    );
    assert_eq!(fx.text(REGISTER), "{\n  \"STORY_TESTFADEN\": 0\n}\n");
    let story = Story {
        id: "STORY_TESTFADEN".to_owned(),
        name: "Testfaden".to_owned(),
        notes: String::new(),
        progress: Progress::Var("VAR_UNUSED_0x406C".to_owned()),
        steps: Vec::new(),
        flags: Vec::new(),
    };
    assert_eq!(fx.read(STORY_0), to_canonical(&story));
    assert_eq!(
        fx.text(STORY_0),
        "{\n  \"id\": \"STORY_TESTFADEN\",\n  \"name\": \"Testfaden\",\n  \"progress\": {\n    \
         \"var\": \"VAR_UNUSED_0x406C\"\n  }\n}\n"
    );
    assert_eq!(json(&fx, LAYOUT_0), serde_json::json!({"nodes": {}}));
    assert_eq!(fx.text(LAYOUT_0), "{\n  \"nodes\": {}\n}\n");
    assert_eq!(read_story(&fx.project(), 0).unwrap(), Some(story));
    // No id file for rotom: it knows no stories.
    assert!(
        !fx.root()
            .join(".rotom/command_database/.g4-ids-stories.json")
            .exists()
    );
    // The check finds nothing.
    let manifest = crate::textproject::manifest::read_manifest(fx.root()).unwrap();
    let problems = crate::textproject::kinds::check_files(&fx.project(), &manifest).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_second_story_takes_the_next_number_and_a_clashing_name_gets_a_suffix() {
    let fx = TextFixture::exported();
    let p = fx.project();
    create_story_files(&p, &testfaden()).unwrap();
    let second = create_story_files(
        &p,
        &request("Testfaden", Progress::Flag("FLAG_0001".to_owned())),
    )
    .unwrap();
    assert_eq!(second.id, "STORY_TESTFADEN_2");
    assert_eq!(second.number, 1);
    assert_eq!(second.files, paths(&[REGISTER, STORY_1, LAYOUT_1]));
    assert_eq!(
        json(&fx, REGISTER),
        serde_json::json!({"STORY_TESTFADEN": 0, "STORY_TESTFADEN_2": 1})
    );
    assert_eq!(json(&fx, STORY_1)["id"], "STORY_TESTFADEN_2");
    assert_eq!(
        json(&fx, STORY_1)["progress"],
        serde_json::json!({"flag": "FLAG_0001"})
    );

    // Umlauts as in the trainers' baptism, a name of digits only by its number; a free number
    // below is taken first.
    let third = create_story_files(
        &p,
        &request(
            "  Rüpel & Co  ",
            Progress::Var("VAR_UNUSED_0x406C".to_owned()),
        ),
    )
    .unwrap();
    assert_eq!(
        (third.id.as_str(), third.number),
        ("STORY_RUEPEL_AND_CO", 2)
    );
    assert_eq!(json(&fx, "data/stories/0002.json")["name"], "Rüpel & Co");
    std::fs::remove_file(fx.root().join(STORY_0)).unwrap();
    std::fs::remove_file(fx.root().join(LAYOUT_0)).unwrap();
    fx.write(
        REGISTER,
        b"{\n  \"STORY_RUEPEL_AND_CO\": 2,\n  \"STORY_TESTFADEN_2\": 1\n}\n",
    );
    let fourth = create_story_files(
        &p,
        &request("2024", Progress::Var("VAR_UNUSED_0x406C".to_owned())),
    )
    .unwrap();
    assert_eq!((fourth.id.as_str(), fourth.number), ("STORY_0000", 0));

    // A long name is cut to an id of 64 characters, its suffix included.
    let long = "Eine sehr lange Geschichte über den Wächter am Tor und seine vielen Freunde";
    let first = create_story_files(
        &p,
        &request(long, Progress::Var("VAR_UNUSED_0x406C".to_owned())),
    )
    .unwrap();
    let again = create_story_files(
        &p,
        &request(long, Progress::Var("VAR_UNUSED_0x406C".to_owned())),
    )
    .unwrap();
    assert_eq!(first.id.len(), 64, "{}", first.id);
    assert!(
        first
            .id
            .starts_with("STORY_EINE_SEHR_LANGE_GESCHICHTE_UEBER")
    );
    assert!(again.id.len() <= 64, "{}", again.id);
    assert!(again.id.ends_with("_2"), "{}", again.id);
    assert_ne!(first.id, again.id);
}

#[test]
fn create_refuses_an_unknown_progress_id_and_an_empty_name() {
    let fx = TextFixture::exported();
    let p = fx.project();
    let before = fx.read(REGISTER);
    assert_eq!(
        refused(create_story_files(
            &p,
            &request("Testfaden", Progress::Var("VAR_NOPE".to_owned()))
        )),
        unknown_id_text("VAR_NOPE")
    );
    // A flag's id is no variable.
    assert_eq!(
        refused(create_story_files(
            &p,
            &request("Testfaden", Progress::Var("FLAG_0001".to_owned()))
        )),
        unknown_id_text("FLAG_0001")
    );
    for name in ["", "   "] {
        assert_eq!(
            refused(create_story_files(
                &p,
                &request(name, Progress::Var("VAR_UNUSED_0x406C".to_owned()))
            )),
            name_empty_text()
        );
    }
    let mut twice = testfaden();
    twice.steps = vec![
        step("s1", 1, StepKind::Start, "Eins", Vec::new()),
        step("s1", 2, StepKind::Dialog, "Zwei", Vec::new()),
    ];
    assert_eq!(
        refused(create_story_files(&p, &twice)),
        duplicate_key_text("s1")
    );
    assert_eq!(fx.read(REGISTER), before);
    assert!(!fx.root().join("data/stories").exists());
}

#[test]
fn create_refuses_the_dspre_layout() {
    let fx = TextFixture::dspre();
    assert_eq!(
        refused(create_story_files(&fx.project(), &testfaden())),
        NOT_A_TEXT_PROJECT
    );
    let mut st = store(&fx);
    assert_eq!(
        refused(create_story(&mut st, &testfaden())),
        NOT_A_TEXT_PROJECT
    );
    assert!(st.history().0.is_empty());
    assert!(!fx.root().join("data").exists());
    assert!(list_stories(&fx.project()).unwrap().is_empty());
}

#[test]
fn create_over_the_store_is_one_undo_step() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    let report = create_story(&mut st, &testfaden()).unwrap();
    assert!(report.change.is_some());
    assert_eq!(st.history().0, ["Geschichte „Testfaden“ angelegt"]);
    let (register, story, layout) = (fx.read(REGISTER), fx.read(STORY_0), fx.read(LAYOUT_0));

    // One undo takes both files and the register entry back.
    assert_eq!(st.undo().unwrap(), [HEADER_TABLE]);
    assert!(!fx.root().join(STORY_0).exists());
    assert!(!fx.root().join(LAYOUT_0).exists());
    assert_eq!(fx.text(REGISTER), "{}\n");
    assert!(list_stories(&fx.project()).unwrap().is_empty());

    // Redo puts them back as they were.
    st.redo().unwrap();
    assert_eq!(fx.read(REGISTER), register);
    assert_eq!(fx.read(STORY_0), story);
    assert_eq!(fx.read(LAYOUT_0), layout);
    assert_eq!(st.history().0, ["Geschichte „Testfaden“ angelegt"]);
}

#[test]
fn update_replaces_given_fields_and_records_one_step() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    create_story(&mut st, &testfaden()).unwrap();
    let before = fx.read(STORY_0);
    let layout = fx.read(LAYOUT_0);
    let steps = vec![
        step(
            "s0",
            0,
            StepKind::Start,
            "Ankunft",
            vec![Use::Map("MAP_T01".to_owned())],
        ),
        step(
            "s1",
            1,
            StepKind::Battle,
            "Herausforderung",
            vec![Use::Trainer(KAI.to_owned())],
        ),
    ];
    let patch = StoryPatch {
        notes: Some("Der Wächter am Tor.".to_owned()),
        steps: Some(steps.clone()),
        ..StoryPatch::default()
    };
    let report = update_story(&mut st, "STORY_TESTFADEN", &patch).unwrap();
    assert_eq!(report.id, "STORY_TESTFADEN");
    assert_eq!(report.number, 0);
    assert_eq!(report.files, paths(&[STORY_0]));
    assert!(report.change.is_some());
    assert_eq!(
        st.history().0,
        [
            "Geschichte „Testfaden“ angelegt",
            "Geschichte „Testfaden“ geändert"
        ]
    );
    let story = read_story(&fx.project(), 0).unwrap().unwrap();
    assert_eq!(story.notes, "Der Wächter am Tor.");
    assert_eq!(story.steps, steps);
    assert_eq!(story.name, "Testfaden");
    assert_eq!(
        story.progress,
        Progress::Var("VAR_UNUSED_0x406C".to_owned())
    );
    assert_eq!(fx.read(STORY_0), to_canonical(&story));
    assert_eq!(
        fx.read(LAYOUT_0),
        layout,
        "no layout given: the side file stays"
    );

    // A new name is in the label; the id stays.
    update_story(
        &mut st,
        "STORY_TESTFADEN",
        &StoryPatch {
            name: Some("Der Wächter".to_owned()),
            ..StoryPatch::default()
        },
    )
    .unwrap();
    assert_eq!(st.history().0[2], "Geschichte „Der Wächter“ geändert");
    assert_eq!(json(&fx, STORY_0)["id"], "STORY_TESTFADEN");

    // Undo puts the old bytes back, one step at a time.
    st.undo().unwrap();
    st.undo().unwrap();
    assert_eq!(fx.read(STORY_0), before);

    // The refusals of making one.
    for (patch, why) in [
        (
            StoryPatch {
                name: Some(" ".to_owned()),
                ..StoryPatch::default()
            },
            name_empty_text().to_owned(),
        ),
        (
            StoryPatch {
                progress: Some(Progress::Flag("FLAG_NOPE".to_owned())),
                ..StoryPatch::default()
            },
            unknown_id_text("FLAG_NOPE"),
        ),
        (
            StoryPatch {
                steps: Some(vec![
                    step("a", 0, StepKind::Start, "A", Vec::new()),
                    step("a", 1, StepKind::Reward, "B", Vec::new()),
                ]),
                ..StoryPatch::default()
            },
            duplicate_key_text("a"),
        ),
    ] {
        assert_eq!(
            refused(update_story(&mut st, "STORY_TESTFADEN", &patch)),
            why
        );
    }
    assert_eq!(
        refused(update_story_files(
            &fx.project(),
            "STORY_NIEMAND",
            &StoryPatch {
                notes: Some("x".to_owned()),
                ..StoryPatch::default()
            }
        )),
        unknown_id_text("STORY_NIEMAND")
    );
    assert_eq!(fx.read(STORY_0), before);
}

#[test]
fn update_with_layout_writes_the_side_file_only() {
    let fx = TextFixture::exported();
    let p = fx.project();
    create_story_files(&p, &testfaden()).unwrap();
    let story = fx.read(STORY_0);
    let register = fx.read(REGISTER);
    let layout = Layout {
        nodes: [("s0".to_owned(), Position { x: 120.0, y: -40.5 })]
            .into_iter()
            .collect(),
    };
    let report = update_story_files(
        &p,
        "STORY_TESTFADEN",
        &StoryPatch {
            layout: Some(layout.clone()),
            ..StoryPatch::default()
        },
    )
    .unwrap();
    assert_eq!(report.files, paths(&[LAYOUT_0]));
    assert_eq!(fx.read(STORY_0), story);
    assert_eq!(fx.read(REGISTER), register);
    assert_eq!(
        crate::stories::read_layout(&p, 0).unwrap(),
        Some(layout.clone())
    );
    assert_eq!(fx.read(LAYOUT_0), to_canonical(&layout));

    // A missing side file is written anew.
    std::fs::remove_file(fx.root().join(LAYOUT_0)).unwrap();
    let report = update_story_files(
        &p,
        "STORY_TESTFADEN",
        &StoryPatch {
            layout: Some(Layout::default()),
            ..StoryPatch::default()
        },
    )
    .unwrap();
    assert_eq!(report.files, paths(&[LAYOUT_0]));
    assert_eq!(fx.text(LAYOUT_0), "{\n  \"nodes\": {}\n}\n");
}

#[test]
fn update_without_change_is_refused() {
    let fx = TextFixture::exported();
    let mut st = store(&fx);
    create_story(&mut st, &testfaden()).unwrap();
    for patch in [
        StoryPatch::default(),
        StoryPatch {
            name: Some("Testfaden".to_owned()),
            notes: Some(String::new()),
            progress: Some(Progress::Var("VAR_UNUSED_0x406C".to_owned())),
            steps: Some(Vec::new()),
            flags: Some(Vec::new()),
            layout: Some(Layout::default()),
        },
    ] {
        assert_eq!(
            refused(update_story(&mut st, "STORY_TESTFADEN", &patch)),
            NOTHING_TO_CHANGE
        );
    }
    assert_eq!(st.history().0.len(), 1);
}

#[test]
fn list_stories_keeps_a_bad_file_as_its_problem() {
    let fx = TextFixture::exported();
    let p = fx.project();
    assert!(list_stories(&p).unwrap().is_empty());
    create_story_files(&p, &testfaden()).unwrap();
    create_story_files(&p, &request("Zwei", Progress::Flag("FLAG_0001".to_owned()))).unwrap();
    create_story_files(&p, &request("Drei", Progress::Flag("FLAG_0001".to_owned()))).unwrap();
    create_story_files(&p, &request("Vier", Progress::Flag("FLAG_0001".to_owned()))).unwrap();
    // 1 does not read, 2 has another id, 3 has no file.
    fx.write(
        STORY_1,
        b"{\n  \"id\": \"STORY_ZWEI\",\n  \"farbe\": 1\n}\n",
    );
    fx.edit("data/stories/0002.json", "STORY_DREI", "STORY_ANDERS");
    std::fs::remove_file(fx.root().join("data/stories/0003.json")).unwrap();

    let list = list_stories(&p).unwrap();
    assert_eq!(
        list.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(list[0].1.as_ref().unwrap().id, "STORY_TESTFADEN");
    let problem = |i: usize| list[i].1.as_ref().unwrap_err().clone();
    assert!(
        problem(1).message.starts_with(SCHEMA_CODE),
        "{:?}",
        problem(1)
    );
    assert_eq!(problem(1).file, Path::new(STORY_1));
    assert_eq!(problem(1).line, 3);
    assert!(
        problem(2).message.starts_with(ID_MISMATCH_CODE),
        "{:?}",
        problem(2)
    );
    assert_eq!(problem(2).line, 2);
    assert!(
        problem(3).message.starts_with(ID_DANGLING_CODE),
        "{:?}",
        problem(3)
    );
    // As the check says it: at the id's key in the register.
    assert_eq!(problem(3).severity, Severity::Warning);
    assert_eq!(problem(3).file, Path::new(REGISTER));
    assert_eq!(problem(3).line, 4);
}

#[test]
fn renaming_a_story_id_changes_register_and_file() {
    let fx = TextFixture::exported();
    create_story_files(&fx.project(), &testfaden()).unwrap();
    let layout = fx.read(LAYOUT_0);
    let mut st = store(&fx);
    let result = rename_id(&mut st, "STORY_TESTFADEN", "STORY_WAECHTER").unwrap();
    assert_eq!(result.plan.kind, Some(SymbolKind::Story));
    assert_eq!(result.plan.number, Some(0));
    assert_eq!(
        result
            .plan
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect::<Vec<_>>(),
        paths(&[REGISTER, STORY_0])
    );
    assert_eq!(fx.text(REGISTER), "{\n  \"STORY_WAECHTER\": 0\n}\n");
    assert_eq!(json(&fx, STORY_0)["id"], "STORY_WAECHTER");
    assert_eq!(json(&fx, STORY_0)["name"], "Testfaden");
    assert_eq!(fx.read(LAYOUT_0), layout);
    let manifest = crate::textproject::manifest::read_manifest(fx.root()).unwrap();
    let problems = crate::textproject::kinds::check_files(&fx.project(), &manifest).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");

    // One undo puts the old id back in register and file, redo the new one.
    st.undo().unwrap();
    assert_eq!(json(&fx, STORY_0)["id"], "STORY_TESTFADEN");
    assert_eq!(json(&fx, REGISTER)["STORY_TESTFADEN"], 0);
    st.redo().unwrap();
    assert_eq!(json(&fx, STORY_0)["id"], "STORY_WAECHTER");

    // The rules of an id hold; the number stays.
    let refusal = |new: &str| {
        crate::textproject::rename::plan_rename(&fx.project(), "STORY_WAECHTER", new)
            .unwrap()
            .refusal
    };
    assert_eq!(
        refusal("WAECHTER").as_deref(),
        Some(
            "Die neue ID muss mit STORY_ beginnen und darf danach nur A–Z, 0–9 und _ enthalten \
             (höchstens 64 Zeichen)."
        )
    );
    assert_eq!(
        refusal("STORY_0003").as_deref(),
        Some("STORY_0003 ist die Ersatz-ID der Nummer 3.")
    );
    create_story_files(
        &fx.project(),
        &request("Zwei", Progress::Flag("FLAG_0001".to_owned())),
    )
    .unwrap();
    assert_eq!(
        refusal("STORY_ZWEI").as_deref(),
        Some("STORY_ZWEI gibt es schon (Nummer 1).")
    );
    assert_eq!(
        crate::stories::number_of(&fx.project(), "STORY_WAECHTER").unwrap(),
        Some(0)
    );
}

#[test]
fn renaming_a_variable_follows_the_story_progress() {
    let fx = TextFixture::exported();
    let p = fx.project();
    name_flag_files(
        &p,
        &NameRequest {
            kind: FlagKind::Var,
            number: 0x406C,
            id: Some("VAR_MEIN".to_owned()),
            description: None,
        },
    )
    .unwrap();
    create_story_files(
        &p,
        &request("Testfaden", Progress::Var("VAR_MEIN".to_owned())),
    )
    .unwrap();
    rename_id_files(&p, "VAR_MEIN", "VAR_DEIN").unwrap();
    let story = read_story(&p, 0).unwrap().unwrap();
    assert_eq!(story.progress, Progress::Var("VAR_DEIN".to_owned()));
    assert_eq!(
        json(&fx, "data/ids/vars.json"),
        serde_json::json!({"VAR_DEIN": 0x406C})
    );
    let manifest = crate::textproject::manifest::read_manifest(fx.root()).unwrap();
    let problems = crate::textproject::kinds::check_files(&p, &manifest).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn renaming_a_trainer_follows_the_story_uses() {
    let fx = TextFixture::exported();
    let mut req = testfaden();
    req.steps = vec![step(
        "s1",
        1,
        StepKind::Battle,
        "Herausforderung",
        vec![Use::Trainer(KAI.to_owned()), Use::Map("MAP_T01".to_owned())],
    )];
    create_story_files(&fx.project(), &req).unwrap();
    let before = fx.read(STORY_0);
    let mut st = store(&fx);
    let result = rename_id(&mut st, KAI, "TRAINER_X").unwrap();
    assert!(
        result
            .plan
            .files
            .iter()
            .any(|f| f.path == Path::new(STORY_0) && f.count == 1),
        "{:?}",
        result.plan.files
    );
    let story = read_story(&fx.project(), 0).unwrap().unwrap();
    assert_eq!(
        story.steps[0].uses,
        [
            Use::Trainer("TRAINER_X".to_owned()),
            Use::Map("MAP_T01".to_owned())
        ]
    );
    assert_eq!(st.history().0.len(), 1);
    st.undo().unwrap();
    assert_eq!(fx.read(STORY_0), before);
}

#[test]
fn the_story_files_have_their_numbers() {
    assert_eq!(story_rel(7), Path::new("data/stories/0007.json"));
    assert_eq!(layout_rel(7), Path::new("data/stories/0007.layout.json"));
}
