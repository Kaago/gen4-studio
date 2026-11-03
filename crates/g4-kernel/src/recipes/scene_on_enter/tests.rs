#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn key_label_and_texts() {
    assert_eq!(step_key(2), "s2");
    assert_eq!(
        label(415, "Testfaden"),
        "Szene beim Betreten von Header 415 in „Testfaden“"
    );
    assert_eq!(
        taken_condition_text(1),
        "Der Ort hat schon eine Szene für Schritt 1."
    );
    assert_eq!(NO_TEXT, "Die Szene braucht einen Text.");
    assert_eq!(DEFAULT_NAME, "Szene beim Betreten");
}

#[test]
fn input_without_step_name() {
    let input: SceneOnEnterInput = serde_json::from_value(serde_json::json!({
        "story": "STORY_X",
        "header": 415,
        "step": 1,
        "text": "Hallo",
    }))
    .unwrap();
    assert_eq!(input.step_name, None);
    assert_eq!((input.header, input.step), (415, 1));
}
