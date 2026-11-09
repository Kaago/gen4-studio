#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use super::*;
use crate::test_support::TextFixture;
use crate::textproject::rename::not_a_text_project;

fn input(name: &str, var: u16) -> NewStoryInput {
    NewStoryInput {
        name: name.to_owned(),
        var,
        notes: String::new(),
    }
}

#[test]
fn the_name_is_trimmed_and_normalized_for_the_variable() {
    let fx = TextFixture::exported();
    let mut store = Store::new(Arc::new(fx.project()));
    let draft = plan(&mut store, &input("  Ärger im Wald  ", 0x4031)).unwrap();
    assert_eq!(
        draft.notes,
        [
            "Creates story “Ärger im Wald” with the step “Start”.",
            "Names variable 0x4031 as VAR_STORY_AERGER_IM_WALD.",
        ]
    );
    assert!(draft.irreversible.is_empty());
}

#[test]
fn a_project_without_text_project_is_refused_before_writing() {
    let fx = TextFixture::dspre();
    let mut store = Store::new(Arc::new(fx.project()));
    let mark = store.undo_depth();
    match apply(&mut store, &input("Testfaden", 0x4030)) {
        Err(KernelError::Refused(why)) => assert_eq!(why, not_a_text_project()),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(store.undo_depth(), mark);
    assert!(!fx.root().join("data/ids/vars.json").exists());
}
