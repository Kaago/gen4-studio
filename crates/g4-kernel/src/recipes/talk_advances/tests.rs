use super::*;

#[test]
fn key_and_label_name_the_value_the_talk_switches_to() {
    assert_eq!(step_key(1), "s1");
    assert_eq!(
        label(5, "Testfaden", 1),
        "Talk with character 5 advances “Testfaden” to 1"
    );
}

#[test]
fn notes_name_branch_slot_and_message() {
    assert_eq!(
        notes(5, 0, (3, 0, 12, 433)),
        [
            "Character 5 gets the branch “from step 0” and advances to 1.",
            "Script #3 in script file 0, message 12 in archive 433.",
        ]
    );
    // The value after the highest step is named without overflow.
    assert!(notes(5, u16::MAX, (3, 0, 12, 433))[0].ends_with("to 65536."));
}

#[test]
fn the_input_reads_without_a_step_name() {
    let input: TalkAdvancesInput = serde_json::from_value(serde_json::json!({
        "story": "STORY_TESTFADEN",
        "figure": { "header": 0, "local_id": 5 },
        "from_step": 0,
        "text": "Hallo!",
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        input.figure,
        FigureRef {
            header: 0,
            local_id: 5
        }
    );
    assert_eq!(input.step_name, None);
}
