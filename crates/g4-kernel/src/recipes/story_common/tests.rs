#![allow(clippy::unwrap_used)]

use super::*;

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

fn story(steps: Vec<Step>) -> Story {
    Story {
        id: "STORY_TESTFADEN".to_owned(),
        name: "Testfaden".to_owned(),
        notes: String::new(),
        progress: Progress::Var("VAR_STORY_TESTFADEN".to_owned()),
        steps,
        flags: Vec::new(),
    }
}

#[test]
fn made_names_the_recipe_and_the_studio_version() {
    assert_eq!(
        made(RecipeId::TalkAdvances),
        Made {
            recipe: "talk_advances".to_owned(),
            studio: env!("CARGO_PKG_VERSION").to_owned(),
        }
    );
    assert_eq!(made(RecipeId::NewStory).recipe, "new_story");
}

#[test]
fn var_ids_carry_the_story_name_and_count_on_a_clash() {
    let none = |_: &str| false;
    assert_eq!(story_var_id("Testfaden", &none), "VAR_STORY_TESTFADEN");
    assert_eq!(
        story_var_id("Ärger in Zweiblattdorf", &none),
        "VAR_STORY_AERGER_IN_ZWEIBLATTDORF"
    );
    // Digits alone stay, a name without letters or digits leaves the prefix.
    assert_eq!(story_var_id("1923", &none), "VAR_STORY_1923");
    assert_eq!(story_var_id("!!!", &none), "VAR_STORY");
    let taken = |id: &str| ["VAR_STORY_TESTFADEN", "VAR_STORY_TESTFADEN_2"].contains(&id);
    assert_eq!(story_var_id("Testfaden", &taken), "VAR_STORY_TESTFADEN_3");
    // A long name is cut to the id length, the suffix kept.
    let long = "Eine sehr lange Geschichte über den Weg von Zweiblattdorf nach Sandgemme";
    let id = story_var_id(long, &none);
    assert_eq!(id.len(), MAX_ID_LEN, "{id}");
    assert!(id.starts_with("VAR_STORY_EINE_SEHR_LANGE"), "{id}");
    let second = story_var_id(long, &|other: &str| other == id);
    assert!(
        second.ends_with("_2") && second.len() <= MAX_ID_LEN,
        "{second}"
    );
}

#[test]
fn flag_ids_keep_kind_and_figure_when_cut() {
    let none = |_: &str| false;
    assert_eq!(
        story_flag_id("Testfaden", "SICHTBAR", 3, &none),
        "FLAG_STORY_TESTFADEN_SICHTBAR_3"
    );
    assert_eq!(
        story_flag_id("Testfaden", "BELOHNUNG", 3, &|id: &str| {
            id == "FLAG_STORY_TESTFADEN_BELOHNUNG_3"
        }),
        "FLAG_STORY_TESTFADEN_BELOHNUNG_3_2"
    );
    let long = "Eine sehr lange Geschichte über den Weg von Zweiblattdorf nach Sandgemme";
    let id = story_flag_id(long, "SICHTBAR", 12, &none);
    assert!(id.len() <= MAX_ID_LEN, "{id}");
    assert!(id.starts_with("FLAG_STORY_EINE"), "{id}");
    assert!(id.ends_with("_SICHTBAR_12"), "{id}");
}

#[test]
fn a_dialog_step_takes_its_value_appearing_and_rewards_do_not() {
    let story = story(vec![
        step("s0", 0, StepKind::Start, "Anfang"),
        step(
            "a7",
            1,
            StepKind::Appearance,
            "Figur 3 erscheint ab Schritt 1",
        ),
        step("s1", 1, StepKind::Dialog, "Gespräch mit Figur 2"),
        step("r9", 2, StepKind::Reward, "Belohnung von Figur 2"),
    ]);
    assert_eq!(step_of(&story, 1).map(|s| s.key.as_str()), Some("a7"));
    assert_eq!(step_of(&story, 3), None);
    match dialog_step_taken(&story, 1) {
        Err(KernelError::Refused(why)) => {
            assert_eq!(why, "Schritt 1 gibt es schon: „Gespräch mit Figur 2“.");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(dialog_step_taken(&story, 2).is_ok());
    assert!(dialog_step_taken(&story, 3).is_ok());
}

#[test]
fn fill_replaces_each_placeholder_once() {
    let text = fill(
        "script figur_{local_id} #{n}:\n    {cmd} {n}\n    {unknown}\n",
        &[
            ("n", "7".to_owned()),
            ("local_id", "3".to_owned()),
            // A value is not searched for placeholders again.
            ("cmd", "{n}".to_owned()),
        ],
    );
    assert_eq!(text, "script figur_3 #7:\n    {n} 7\n    {unknown}\n");
    assert_eq!(fill("ohne { Klammer", &[]), "ohne { Klammer");
}

#[test]
fn slot_name_finds_the_script_of_a_slot() {
    let source = "script main #1:\n    End\n\nscript zwei #2:\n    End\n";
    assert_eq!(slot_name(source, 2).as_deref(), Some("zwei"));
    assert_eq!(slot_name(source, 3), None);
    // Parsed fault-tolerantly: a half-written file still names its scripts.
    assert_eq!(
        slot_name("script halb #1:\n    if\n", 1).as_deref(),
        Some("halb")
    );
}

#[test]
fn hgss_shows_messages_with_npcmsg() {
    assert_eq!(message_command(Family::Hgss), "NPCMsg");
    assert_eq!(message_command(Family::Pt), "Message");
    assert_eq!(message_command(Family::Dp), "Message");
}

#[cfg(feature = "test-support")]
mod with_fixture {
    use std::sync::Arc;

    use super::*;
    use crate::flags::{NameRequest, name_flag, names};
    use crate::scripts::ScriptSource;
    use crate::store::ObjectEvent;
    use crate::stories::{StoryRequest, create_story};
    use crate::test_support::TextFixture;

    const ARCHIVE: u16 = 433;

    fn store(fx: &TextFixture) -> Store {
        Store::new(Arc::new(fx.project()))
    }

    fn refused<T: std::fmt::Debug>(result: Result<T, KernelError>) -> String {
        match result {
            Err(KernelError::Refused(why)) => why,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn pool_numbers_must_be_in_the_save_general_and_unnamed() {
        let fx = TextFixture::exported();
        let mut store = store(&fx);
        let project = fx.project();
        assert!(check_pool_number(&project, FlagKind::Var, 0x4030).is_ok());
        assert!(check_pool_number(&project, FlagKind::Flag, 0x41).is_ok());
        assert_eq!(
            refused(check_pool_number(&project, FlagKind::Flag, 0xB60)),
            "Flag 0x0B60 liegt nicht im Spielstand."
        );
        assert_eq!(
            refused(check_pool_number(&project, FlagKind::Flag, 0x2DA)),
            "Flag 0x02DA liegt nicht im allgemeinen Bereich."
        );
        name_flag(
            &mut store,
            &NameRequest {
                kind: FlagKind::Flag,
                number: 0x41,
                id: Some("FLAG_EIGEN".to_owned()),
                description: None,
            },
        )
        .unwrap();
        assert_eq!(
            refused(check_pool_number(&project, FlagKind::Flag, 0x41)),
            "FLAG_EIGEN gibt es schon (Nummer 65)."
        );
    }

    #[test]
    fn header_files_figure_and_the_next_slot() {
        let fx = TextFixture::exported();
        let project = fx.project();
        scripts::write_source(
            &project,
            0,
            &ScriptSource::Rotom("script main #1:\n    End\n\nscript zwei #2:\n    End\n".into()),
        )
        .unwrap();
        let mut store = store(&fx);
        let files = header_files(&mut store, 0).unwrap();
        assert_eq!(
            (files.script_file, files.text_archive, files.event_file),
            (0, ARCHIVE, 0)
        );
        assert!(header_files(&mut store, 9999).is_err());

        let (slot, source) = next_slot(&project, 0).unwrap();
        assert_eq!(slot, 3);
        assert!(source.contains("script zwei #2:"));

        let object = |local_id, script, trainer_type| ObjectEvent {
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
            x: 4 + local_id,
            z: 4,
            y: 0,
        };
        store
            .transaction("Figuren", |tx| {
                tx.edit_event(0, |f| {
                    f.object_events.push(object(5, 2, 0));
                    f.object_events.push(object(6, 3000, 1));
                    Ok(())
                })
            })
            .unwrap();
        let at = |local_id| FigureRef {
            header: 0,
            local_id,
        };
        let talker = figure(&mut store, &at(5)).unwrap();
        assert_eq!(
            (
                talker.index,
                talker.script,
                talker.trainer,
                talker.hidden_flag
            ),
            (0, 2, false, 0)
        );
        assert!(figure(&mut store, &at(6)).unwrap().trainer);
        assert_eq!(
            refused(figure(&mut store, &at(9))),
            "Auf Header 0 gibt es keine Figur 9."
        );
    }

    #[test]
    fn push_message_appends_a_numbered_message() {
        let fx = TextFixture::exported();
        let mut store = store(&fx);
        let count = store.text_archive(ARCHIVE).unwrap().messages.len();
        let (number, _) = store
            .transaction("Nachricht", |tx| {
                push_message(tx, Family::Pt, ARCHIVE, "Hallo!")
            })
            .unwrap();
        assert_eq!(usize::from(number), count);
        let doc = store.text_archive(ARCHIVE).unwrap();
        assert_eq!(doc.messages.len(), count + 1);
        assert_eq!(doc.messages[count].text, "Hallo!");
    }

    #[test]
    fn load_story_needs_a_known_story_with_a_variable() {
        let fx = TextFixture::exported();
        let mut store = store(&fx);
        assert_eq!(
            refused(load_story(&mut store, "STORY_TESTFADEN")),
            "Unbekannte ID: STORY_TESTFADEN."
        );
        let request = |name: &str, progress| StoryRequest {
            name: name.to_owned(),
            progress,
            notes: String::new(),
            steps: Vec::new(),
            flags: Vec::new(),
        };
        create_story(
            &mut store,
            &request("Testfaden", Progress::Var("VAR_UNUSED_0x406C".to_owned())),
        )
        .unwrap();
        let loaded = load_story(&mut store, "STORY_TESTFADEN").unwrap();
        assert_eq!(
            (loaded.number, loaded.var, loaded.var_id.as_str()),
            (0, 0x406C, "VAR_UNUSED_0x406C")
        );
        let (_, flag) = names(Family::Pt, FlagKind::Flag).iter().next().unwrap();
        create_story(
            &mut store,
            &request("Flaggenfaden", Progress::Flag(flag.to_owned())),
        )
        .unwrap();
        assert_eq!(
            refused(load_story(&mut store, "STORY_FLAGGENFADEN")),
            "Geschichte „Flaggenfaden“ hält ihren Fortschritt in einem Flag; die Rezepte \
             brauchen eine Variable."
        );
    }
}
