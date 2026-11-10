#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn key_label_and_texts() {
    assert_eq!(step_key(2), "s2");
    assert_eq!(
        label(415, "Testfaden"),
        "Scene on entering Header 415 in “Testfaden”"
    );
    assert_eq!(
        taken_condition_text(1),
        "The place already has a scene for step 1."
    );
    assert_eq!(no_text(), "The scene needs a text.");
    assert_eq!(default_name(), "Scene on entering");
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
