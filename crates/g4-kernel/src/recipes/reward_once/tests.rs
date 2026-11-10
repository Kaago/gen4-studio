use super::*;

#[test]
fn key_label_and_refusal_name_flag_figure_and_item() {
    assert_eq!(step_key(256), "r256");
    assert_eq!(
        label(5, "Testfaden"),
        "Reward from character 5 in “Testfaden”"
    );
    assert_eq!(no_item_text(0), "Item 0 does not exist.");
}

#[test]
fn the_input_reads_without_a_step_name() {
    let input: RewardOnceInput = serde_json::from_value(serde_json::json!({
        "story": "STORY_TESTFADEN",
        "figure": { "header": 0, "local_id": 5 },
        "from_step": 2,
        "item": 17,
        "text": "Ein Trank!",
        "flag": 256,
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        input.figure,
        FigureRef {
            header: 0,
            local_id: 5
        }
    );
    assert_eq!((input.from_step, input.item, input.flag), (2, 17, 256));
    assert_eq!(input.step_name, None);
}
