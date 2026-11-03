#![allow(clippy::unwrap_used)]

use super::*;

fn doc(json: &str) -> LevelScriptDoc {
    LevelScriptDoc::from_json(json).unwrap()
}

#[test]
fn the_visibility_entry_is_on_transition() {
    assert_eq!(VISIBILITY_ENTRY, "on_transition");
    assert_eq!(
        visibility_entry(4),
        LevelScriptHeaderEntry::OnTransition { script_id: 4 }
    );
}

#[test]
fn transition_entry_reads_the_first_on_transition_entry() {
    assert_eq!(transition_entry(&LevelScriptDoc::default()), None);
    let level = doc(
        r#"{"header_entries":[{"type":"on_load","script_id":2},{"type":"on_var_condition"},
            {"type":"on_transition","script_id":5},{"type":"on_transition","script_id":6}],
            "var_conditions":[]}"#,
    );
    assert_eq!(transition_entry(&level), Some(5));
    // Script 0 is none; an id past u16 is no slot.
    let zero = doc(r#"{"header_entries":[{"type":"on_transition","script_id":0}]}"#);
    assert_eq!(transition_entry(&zero), None);
    let far = doc(r#"{"header_entries":[{"type":"on_transition","script_id":70000}]}"#);
    assert_eq!(transition_entry(&far), Some(u16::MAX));
}

#[test]
fn set_transition_replaces_the_entry_or_appends_one() {
    let mut level = doc(
        r#"{"header_entries":[{"type":"on_load","script_id":2},{"type":"on_transition","script_id":5}]}"#,
    );
    set_transition(&mut level, 9);
    assert_eq!(
        level.header_entries,
        [
            LevelScriptHeaderEntry::OnLoad { script_id: 2 },
            LevelScriptHeaderEntry::OnTransition { script_id: 9 },
        ]
    );
    let mut empty = LevelScriptDoc::default();
    set_transition(&mut empty, 3);
    assert_eq!(
        empty.header_entries,
        [LevelScriptHeaderEntry::OnTransition { script_id: 3 }]
    );
    assert_eq!(transition_entry(&empty), Some(3));
}

#[test]
fn push_condition_adds_the_var_condition_entry_once() {
    let mut level = LevelScriptDoc::default();
    assert_eq!(push_condition(&mut level, 0x4030, 1, 4), 0);
    assert_eq!(push_condition(&mut level, 0x4030, 2, 5), 1);
    assert_eq!(
        level.header_entries,
        [LevelScriptHeaderEntry::OnVarCondition]
    );
    assert_eq!(
        level.var_conditions,
        [
            LevelScriptVarConditionEntry {
                var: 0x4030,
                value: 1,
                script_id: 4
            },
            LevelScriptVarConditionEntry {
                var: 0x4030,
                value: 2,
                script_id: 5
            },
        ]
    );
    assert!(level.validate().is_ok());
}

fn plan(create: bool, binary: Option<u16>) -> LevelScriptPlan {
    LevelScriptPlan {
        file: 910,
        create,
        doc: LevelScriptDoc::default(),
        transition: None,
        binary,
    }
}

#[test]
fn the_irreversible_sentence_names_a_replaced_binary_file() {
    assert_eq!(plan(false, None).irreversible(), None);
    assert_eq!(
        plan(true, None).irreversible().as_deref(),
        Some("Level-Script-Datei 910 wird angelegt und bleibt")
    );
    assert_eq!(
        plan(true, Some(909)).irreversible().as_deref(),
        Some(
            "Level-Script-Datei 910 wird angelegt und bleibt; die Einträge der binären \
             Level-Script-Datei 909 werden nicht übernommen"
        )
    );
}

#[test]
fn a_kept_level_script_is_named_in_the_error() {
    let refused = KernelError::Refused("Die Geschichte ist schreibgeschützt".to_owned());
    // Nothing created: the error as it is.
    assert!(matches!(
        kept_level_script(refused, None),
        KernelError::Refused(m) if m == "Die Geschichte ist schreibgeschützt"
    ));
    let failed = KernelError::Refused("Die Geschichte ist schreibgeschützt".to_owned());
    match kept_level_script(failed, Some(910)) {
        KernelError::Partial(m) => assert_eq!(
            m,
            "Die Geschichte ist schreibgeschützt. Level-Script-Datei 910 wurde angelegt und \
             bleibt."
        ),
        other => panic!("{other:?}"),
    }
    let compile = KernelError::Compile(vec![crate::text::Problem {
        severity: crate::text::Severity::Error,
        file: "expanded/scripts/0910.json".into(),
        line: 3,
        col: 7,
        message: "unknown script 4".to_owned(),
    }]);
    match kept_level_script(compile, Some(910)) {
        KernelError::Partial(m) => assert_eq!(
            m,
            "1 problem(s) in scripts, texts or the text project: expanded/scripts/0910.json:3:7: \
             unknown script 4. Level-Script-Datei 910 wurde angelegt und bleibt."
        ),
        other => panic!("{other:?}"),
    }
}
