#![allow(clippy::unwrap_used)]

use super::*;
use crate::recipes::level_script::created_text;

#[test]
fn key_label_and_texts() {
    assert_eq!(step_key(256), "a256");
    assert_eq!(
        label(4, 1, true, "Testfaden"),
        "Figure 4 appears from step 1 in “Testfaden”"
    );
    assert_eq!(
        label(7, 2, false, "Testfaden"),
        "Figure 7 vanishes from step 2 in “Testfaden”"
    );
    assert_eq!(
        description(4, "Testfaden"),
        "Visibility of figure 4 in “Testfaden”"
    );
    assert_eq!(
        has_flag_text(4, "FLAG_X"),
        "Figure 4 already has the visibility flag FLAG_X."
    );
    assert_eq!(created_text(3), "Level script file 3 is created and stays");
}

#[test]
fn rules_in_counts_the_ifs_of_one_block() {
    let source = "script main #1:\n    if VAR_A >= 1 then\n        End\n    endif\n    End\n\n\
                  script sichtbarkeit_0_0 #2:\n    if VAR_A >= 1 then\n        ClearFlag F\n    \
                  else\n        SetFlag F\n    endif\n    if VAR_A >= 2 then\n        SetFlag G\n    \
                  else\n        ClearFlag G\n    endif\n    End\n// ende sichtbarkeit_0_0\n\n\
                  script danach #3:\n    if VAR_A >= 1 then\n        End\n    endif\n    End\n";
    assert_eq!(
        rules_in(source, "sichtbarkeit_0_0", "// ende sichtbarkeit_0_0"),
        2
    );
    assert_eq!(
        rules_in(source, "sichtbarkeit_1_0", "// ende sichtbarkeit_1_0"),
        0
    );
}

#[test]
fn input_without_step_name() {
    let input: FigureAppearsInput = serde_json::from_value(serde_json::json!({
        "story": "STORY_X",
        "figure": { "header": 0, "local_id": 4 },
        "step": 1,
        "appears": true,
        "flag": 256,
    }))
    .unwrap();
    assert_eq!(input.step_name, None);
    assert!(input.appears);
}
