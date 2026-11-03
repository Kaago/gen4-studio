#![allow(clippy::unwrap_used)]

use std::path::Path;

use super::*;
use crate::stories::entities::{StoryRequest, create_story_files};
use crate::stories::schema::{Step, StepKind, Use};
use crate::test_support::TextFixture;

/// Trainer 1 of the fixture.
const KAI: &str = "TRAINER_YOUNGSTER_KAI";
const PROGRESS: &str = "VAR_UNUSED_0x406C";

fn step(key: &str, value: u16, name: &str, uses: Vec<Use>) -> Step {
    Step {
        key: key.to_owned(),
        value,
        kind: StepKind::Battle,
        name: name.to_owned(),
        notes: String::new(),
        made: None,
        uses,
    }
}

/// Makes story `name` with `steps` and `flags` (progress [`PROGRESS`]) in the fixture.
fn story(fx: &TextFixture, name: &str, steps: Vec<Step>, flags: &[&str]) {
    let req = StoryRequest {
        name: name.to_owned(),
        progress: Progress::Var(PROGRESS.to_owned()),
        notes: String::new(),
        steps,
        flags: flags.iter().map(|f| (*f).to_owned()).collect(),
    };
    create_story_files(&fx.project(), &req).unwrap();
}

fn row(id: &str, name: &str, number: u16, step: Option<(&str, &str)>) -> StoryUse {
    StoryUse {
        story_id: id.to_owned(),
        story_name: name.to_owned(),
        number,
        step_key: step.map(|(k, _)| k.to_owned()),
        step_name: step.map(|(_, n)| n.to_owned()),
        file: Path::new("data/stories").join(format!("{number:04}.json")),
    }
}

#[test]
fn a_trainer_named_in_a_step_is_found_with_story_and_step() {
    let fx = TextFixture::exported();
    story(
        &fx,
        "Testfaden",
        vec![
            step("s0", 0, "Ankunft", Vec::new()),
            step(
                "s1",
                1,
                "Kampf gegen Kai",
                vec![Use::Trainer(KAI.to_owned()), Use::Trainer(KAI.to_owned())],
            ),
            step(
                "s2",
                2,
                "Nachricht",
                vec![Use::Message {
                    archive: 23,
                    index: 5,
                }],
            ),
        ],
        &[],
    );
    let p = fx.project();
    let resolver = Resolver::for_project(&p).unwrap();
    // Named twice in one step: one row, with story and step.
    assert_eq!(
        uses_of(&p, &resolver, &UseTarget::Trainer(1)).unwrap(),
        [row(
            "STORY_TESTFADEN",
            "Testfaden",
            0,
            Some(("s1", "Kampf gegen Kai"))
        )]
    );
    assert!(
        uses_of(&p, &resolver, &UseTarget::Trainer(2))
            .unwrap()
            .is_empty()
    );
    // A place by numbers is found as it stands; the delete plan asks by the entity.
    let message = UseTarget::Message {
        archive: 23,
        index: 5,
    };
    assert_eq!(uses_of(&p, &resolver, &message).unwrap().len(), 1);
    assert_eq!(message.entity(), DeleteTarget::TextArchive { index: 23 });
    assert_eq!(
        mentions_of(&p, DeleteTarget::TextArchive { index: 23 }),
        [row(
            "STORY_TESTFADEN",
            "Testfaden",
            0,
            Some(("s2", "Nachricht"))
        )]
    );
    assert_eq!(
        mentions_of(&p, DeleteTarget::Trainer { index: 1 }),
        uses_of(&p, &resolver, &UseTarget::Trainer(1)).unwrap()
    );
    // A kind no use names (an encounter file) and the DSPRE layout: none.
    assert!(mentions_of(&p, DeleteTarget::Encounter { index: 1 }).is_empty());
    let dspre = TextFixture::dspre();
    assert!(mentions_of(&dspre.project(), DeleteTarget::Trainer { index: 1 }).is_empty());
}

#[test]
fn progress_var_counts_as_a_use_without_a_step() {
    let fx = TextFixture::exported();
    story(
        &fx,
        "Testfaden",
        vec![step(
            "s0",
            0,
            "Ankunft",
            vec![Use::Var(PROGRESS.to_owned())],
        )],
        &["FLAG_0001"],
    );
    let p = fx.project();
    let resolver = Resolver::for_project(&p).unwrap();
    // The progress names it (no step), and step s0 uses it too.
    assert_eq!(
        uses_of(&p, &resolver, &UseTarget::Var(0x406C)).unwrap(),
        [
            row("STORY_TESTFADEN", "Testfaden", 0, None),
            row("STORY_TESTFADEN", "Testfaden", 0, Some(("s0", "Ankunft"))),
        ]
    );
    // A flag of `flags` is a use without a step as well.
    assert_eq!(
        uses_of(&p, &resolver, &UseTarget::Flag(1)).unwrap(),
        [row("STORY_TESTFADEN", "Testfaden", 0, None)]
    );
    assert_eq!(
        mentions_of(&p, DeleteTarget::Var { number: 0x406C }).len(),
        2
    );
}

#[test]
fn an_unreadable_story_is_skipped() {
    let fx = TextFixture::exported();
    let uses = || vec![step("s1", 1, "Kampf", vec![Use::Trainer(KAI.to_owned())])];
    story(&fx, "Kaputt", uses(), &[]);
    story(&fx, "Heil", uses(), &[]);
    fx.write("data/stories/0000.json", b"nicht json");
    let p = fx.project();
    let resolver = Resolver::for_project(&p).unwrap();
    assert_eq!(
        uses_of(&p, &resolver, &UseTarget::Trainer(1)).unwrap(),
        [row("STORY_HEIL", "Heil", 1, Some(("s1", "Kampf")))]
    );
    // An id the project does not know names nothing.
    fx.write(
        "data/stories/0000.json",
        fx.text("data/stories/0001.json")
            .replace("STORY_HEIL", "STORY_KAPUTT")
            .replace(KAI, "TRAINER_NIEMAND")
            .as_bytes(),
    );
    assert_eq!(
        uses_of(&p, &resolver, &UseTarget::Trainer(1))
            .unwrap()
            .len(),
        1
    );
    // A register that does not read: the delete plan gets no mentions, never an error.
    fx.write("data/ids/stories.json", b"[]");
    assert!(uses_of(&p, &resolver, &UseTarget::Trainer(1)).is_err());
    assert!(mentions_of(&p, DeleteTarget::Trainer { index: 1 }).is_empty());
}
