#![allow(clippy::unwrap_used)]

use super::*;

fn jump(name: &str) -> Tail {
    Tail::Jump(name.to_owned())
}

fn rule(var: &str, step: u16, flag: &str, appears: bool) -> VisibilityRule {
    VisibilityRule {
        var_id: var.to_owned(),
        step,
        flag_id: flag.to_owned(),
        appears,
    }
}

#[test]
fn talk_pt_matches_the_template() {
    let text = talk_script(
        Family::Pt,
        5,
        3,
        "VAR_STORY_TESTFADEN",
        1,
        12,
        &jump("figur_3"),
    );
    assert_eq!(
        text,
        "script gespraech_3_5 #5:\n\
         \x20   LockAll\n\
         \x20   FacePlayer\n\
         \x20   if VAR_STORY_TESTFADEN == 1 then\n\
         \x20       Message 12\n\
         \x20       WaitButton\n\
         \x20       CloseMessage\n\
         \x20       SetVarFromValue VAR_STORY_TESTFADEN, 2\n\
         \x20       ReleaseAll\n\
         \x20       End\n\
         \x20   endif\n\
         \x20   ReleaseAll\n\
         \x20   Jump figur_3\n"
    );
    // The hull is named after its slot, the name from one place.
    assert_eq!(talk_name(3, 5), "gespraech_3_5");
    assert!(text.starts_with(&format!("script {} #5:\n", talk_name(3, 5))));
    // Diamond and Pearl take Platinum's text; a figure without a script so far just ends.
    assert_eq!(
        talk_script(Family::Dp, 5, 3, "VAR_STORY_TESTFADEN", 1, 12, &Tail::End),
        talk_script(Family::Pt, 5, 3, "VAR_STORY_TESTFADEN", 1, 12, &Tail::End)
    );
    assert!(
        talk_script(Family::Pt, 5, 3, "V", 0, 1, &Tail::End).ends_with("    ReleaseAll\n    End\n")
    );
}

#[test]
fn talk_hgss_uses_npcmsg_closemsg_setvar() {
    let text = talk_script(Family::Hgss, 5, 3, "VAR_STORY_TESTFADEN", 1, 12, &Tail::End);
    for line in [
        "        NPCMsg 12\n",
        "        CloseMsg\n",
        "        SetVar VAR_STORY_TESTFADEN, 2\n",
    ] {
        assert!(text.contains(line), "{text}");
    }
    assert!(
        !text.contains("Message") && !text.contains("SetVarFromValue"),
        "{text}"
    );
    assert!(!text.contains('{'), "{text}");
}

#[test]
fn reward_script_checks_var_then_flag() {
    let pt = reward_script(
        Family::Pt,
        7,
        3,
        "VAR_STORY_TESTFADEN",
        2,
        "FLAG_STORY_TESTFADEN_BELOHNUNG_3",
        17,
        9,
        &jump("figur_3"),
    );
    assert_eq!(
        pt,
        "script belohnung_3_7 #7:\n\
         \x20   LockAll\n\
         \x20   FacePlayer\n\
         \x20   if VAR_STORY_TESTFADEN >= 2 then\n\
         \x20       if !FLAG_STORY_TESTFADEN_BELOHNUNG_3 then\n\
         \x20           Message 9\n\
         \x20           WaitButton\n\
         \x20           CloseMessage\n\
         \x20           GiveItem 17, 1, VAR_RESULT\n\
         \x20           SetFlag FLAG_STORY_TESTFADEN_BELOHNUNG_3\n\
         \x20           ReleaseAll\n\
         \x20           End\n\
         \x20       endif\n\
         \x20   endif\n\
         \x20   ReleaseAll\n\
         \x20   Jump figur_3\n"
    );
    // The hull is named after its slot, the name from one place.
    assert_eq!(reward_name(3, 7), "belohnung_3_7");
    assert!(pt.starts_with(&format!("script {} #7:\n", reward_name(3, 7))));
    let hgss = reward_script(Family::Hgss, 7, 3, "V", 2, "F", 17, 9, &Tail::End);
    for line in [
        "            NPCMsg 9\n",
        "            CloseMsg\n",
        "            GiveItem 17, 1, VAR_SPECIAL_RESULT\n",
        "            SetFlag F\n",
    ] {
        assert!(hgss.contains(line), "{hgss}");
    }
    assert!(!hgss.contains("VAR_RESULT"), "{hgss}");
}

#[test]
fn scene_script_switches_to_next() {
    assert_eq!(scene_name(4), "szene_4");
    assert_eq!(
        scene_script(Family::Pt, 4, "VAR_STORY_TESTFADEN", 3, 6),
        "script szene_4 #4:\n\
         \x20   LockAll\n\
         \x20   Message 6\n\
         \x20   WaitButton\n\
         \x20   CloseMessage\n\
         \x20   SetVarFromValue VAR_STORY_TESTFADEN, 3\n\
         \x20   ReleaseAll\n\
         \x20   End\n"
    );
    let hgss = scene_script(Family::Hgss, 4, "V", 3, 6);
    assert!(
        hgss.contains("    NPCMsg 6\n")
            && hgss.contains("    CloseMsg\n")
            && hgss.contains("    SetVar V, 3\n"),
        "{hgss}"
    );
}

#[test]
fn visibility_names_and_marker() {
    assert_eq!(visibility_name(2, 415), "sichtbarkeit_2_415");
    assert_eq!(visibility_end_marker(2, 415), "// ende sichtbarkeit_2_415");
    assert_eq!(
        visibility_rule(&rule("VAR_A", 2, "FLAG_A", true)),
        "    if VAR_A >= 2 then\n        ClearFlag FLAG_A\n    else\n        SetFlag FLAG_A\n    endif\n"
    );
    assert_eq!(
        visibility_rule(&rule("VAR_A", 2, "FLAG_A", false)),
        "    if VAR_A >= 2 then\n        SetFlag FLAG_A\n    else\n        ClearFlag FLAG_A\n    endif\n"
    );
}

const TWO_RULES: &str = "script sichtbarkeit_2_415 #4:\n\
    \x20   if VAR_A >= 2 then\n\
    \x20       ClearFlag FLAG_A\n\
    \x20   else\n\
    \x20       SetFlag FLAG_A\n\
    \x20   endif\n\
    \x20   if VAR_A >= 3 then\n\
    \x20       SetFlag FLAG_B\n\
    \x20   else\n\
    \x20       ClearFlag FLAG_B\n\
    \x20   endif\n\
    \x20   Jump uebergang\n\
    // ende sichtbarkeit_2_415\n";

#[test]
fn visibility_block_with_two_rules_and_a_jump() {
    let rules = [
        rule("VAR_A", 2, "FLAG_A", true),
        rule("VAR_A", 3, "FLAG_B", false),
    ];
    assert_eq!(
        visibility_block(2, 415, 4, &rules, &jump("uebergang")),
        TWO_RULES
    );
    // No rule and no script so far: the head, an End and the marker.
    assert_eq!(
        visibility_block(2, 415, 4, &[], &Tail::End),
        "script sichtbarkeit_2_415 #4:\n    End\n// ende sichtbarkeit_2_415\n"
    );
}

#[test]
fn insert_rule_adds_before_the_tail_and_keeps_the_marker() {
    let before = format!("script Main #1:\n    End\n\n{TWO_RULES}\nscript spaeter #5:\n    End\n");
    let added = visibility_rule(&rule("VAR_A", 4, "FLAG_C", true));
    let after = insert_rule(&before, 2, 415, &added, 0).unwrap();
    let expected = format!(
        "script Main #1:\n    End\n\n{}\nscript spaeter #5:\n    End\n",
        TWO_RULES.replace(
            "    Jump uebergang\n",
            &format!("{added}    Jump uebergang\n")
        )
    );
    assert_eq!(after, expected);
    // Again: the marker stays below the tail, the rules keep their order.
    let again = insert_rule(
        &after,
        2,
        415,
        &visibility_rule(&rule("VAR_A", 5, "FLAG_D", false)),
        0,
    )
    .unwrap();
    let lines: Vec<&str> = again.lines().collect();
    let marker = lines
        .iter()
        .position(|l| *l == "// ende sichtbarkeit_2_415")
        .unwrap();
    assert_eq!(lines[marker - 1], "    Jump uebergang");
    assert_eq!(again.matches("// ende sichtbarkeit_2_415").count(), 1);
    assert!(again.find("FLAG_C").unwrap() < again.find("FLAG_D").unwrap());
    // An End tail works the same way, and the other block's marker is left alone.
    let ended = format!(
        "{}{}",
        visibility_block(1, 9, 3, &[], &Tail::End),
        visibility_block(2, 415, 4, &[], &Tail::End)
    );
    let ended = insert_rule(&ended, 2, 415, &added, 0).unwrap();
    assert_eq!(
        ended,
        format!(
            "{}script sichtbarkeit_2_415 #4:\n{added}    End\n// ende sichtbarkeit_2_415\n",
            visibility_block(1, 9, 3, &[], &Tail::End)
        )
    );
}

#[test]
fn insert_rule_without_marker_is_refused() {
    let added = visibility_rule(&rule("VAR_A", 4, "FLAG_C", true));
    for source in ["script Main #1:\n    End\n", TWO_RULES] {
        let query = if source == TWO_RULES {
            (3, 415)
        } else {
            (2, 415)
        };
        let Err(KernelError::Refused(why)) = insert_rule(source, query.0, query.1, &added, 7)
        else {
            panic!("not refused");
        };
        assert_eq!(
            why,
            format!(
                "The visibility block of story {} is missing in script file 7.",
                query.0
            )
        );
    }
}

/// The compiler takes the templates, `if !FLAG then` among them, in both families.
#[cfg(feature = "test-support")]
mod compile {
    use super::*;
    use crate::flags::{FlagKind, names};
    use crate::project::Project;
    use crate::scripts::{self, ScriptSource};
    use crate::test_support::{HEADER_SIZE, TextFixture, text_archive_bytes};
    use crate::text::Severity;

    /// The fixture as `family` (HeartGold US by its game code), exported to a text project.
    fn fixture(family: Family) -> TextFixture {
        let fx = TextFixture::with(|p| {
            if family == Family::Hgss {
                std::fs::write(
                    p.root.join("header.yaml"),
                    "title: \"POKEMON HG\"\ngamecode: IPKE\nmakercode: \"01\"\n",
                )
                .unwrap();
                // The header table of IPKE lies further up; two headers as the fixture has.
                let arm9 = p.root.join("arm9/arm9.bin");
                let mut bytes = std::fs::read(&arm9).unwrap();
                bytes.resize(0xF6BE0 + 2 * HEADER_SIZE + 64, 0);
                std::fs::write(&arm9, bytes).unwrap();
                // Header 0 of IPKE has text archive 0; the fixture unpacks the Platinum paths, so
                // member 0 of its message NARC becomes a valid archive (no text literal needed).
                let msg = p.root.join("files/msgdata/pl_msg.narc");
                let mut narc = g4_formats::narc::Narc::read(&std::fs::read(&msg).unwrap()).unwrap();
                narc.members[0] = text_archive_bytes(&["eins", "zwei", "drei"]);
                std::fs::write(msg, narc.write()).unwrap();
            }
        });
        crate::textproject::export(
            &fx.project(),
            &fx.base(),
            &crate::textproject::ExportOptions {
                adopt_scripts: true,
                force: false,
            },
        )
        .unwrap();
        fx
    }

    fn compile_errors(project: &Project, source: &str) -> Vec<String> {
        scripts::write_source(project, 0, &ScriptSource::Rotom(source.to_owned())).unwrap();
        scripts::compile(project)
            .unwrap()
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .map(|p| format!("{p:?}"))
            .collect()
    }

    #[test]
    fn talk_and_reward_compile_in_both_families() {
        for family in [Family::Pt, Family::Hgss] {
            let fx = fixture(family);
            let project = fx.project();
            assert_eq!(project.family, family);
            let (_, flag) = names(family, FlagKind::Flag).iter().next().unwrap();
            let (_, var) = names(family, FlagKind::Var).iter().next().unwrap();
            let source = [
                "script figur_3 #1:\n    End\n".to_owned(),
                talk_script(family, 2, 3, var, 1, 0, &jump("figur_3")),
                reward_script(family, 3, 3, var, 2, flag, 17, 0, &jump("figur_3")),
                reward_script(family, 4, 3, var, 2, flag, 17, 0, &Tail::End),
                scene_script(family, 5, var, 3, 0),
                visibility_block(
                    1,
                    0,
                    6,
                    &[rule(var, 2, flag, true), rule(var, 3, flag, false)],
                    &jump("figur_3"),
                ),
            ]
            .join("\n");
            let errors = compile_errors(&project, &source);
            assert!(errors.is_empty(), "{family:?}: {errors:?}\n{source}");
        }
    }
}
