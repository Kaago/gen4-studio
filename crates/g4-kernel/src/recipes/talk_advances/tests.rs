use super::*;

#[test]
fn key_name_and_label_name_the_value_the_talk_switches_to() {
    assert_eq!(step_key(1), "s1");
    assert_eq!(script_name(5, 1), "gespraech_5_1");
    assert_eq!(
        label(5, "Testfaden", 1),
        "Gespräch von Figur 5 schaltet „Testfaden“ auf 1"
    );
}

#[test]
fn notes_name_branch_slot_and_message() {
    assert_eq!(
        notes(5, 0, (3, 0, 12, 433)),
        [
            "Figur 5 bekommt den Zweig „ab Schritt 0“ und schaltet auf 1.",
            "Script #3 in Scriptdatei 0, Nachricht 12 in Archiv 433.",
        ]
    );
    // The value after the highest step is named without overflow.
    assert!(notes(5, u16::MAX, (3, 0, 12, 433))[0].ends_with("auf 65536."));
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
