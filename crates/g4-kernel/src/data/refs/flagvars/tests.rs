#![allow(clippy::unwrap_used)]

use super::*;

/// The role and access of parameter `param` of `command` in the table of `family`.
fn role(family: Family, command: &str, param: &str) -> Option<(ParamRole, Access)> {
    flagvar_table(family)
        .param(command, param)
        .map(|p| (p.role, p.access))
}

#[test]
fn script_flagvars_name_existing_commands_and_params() {
    let raw = raw_flagvars().expect("metadata/script_flagvars.yaml parses");
    // rotom v0.7.0 (fb11503) src/decompiler/ir_to_source.rs:28-58 `param_semantics`: 22 names
    // of a variable and 4 of a flag (the plan expected 3; the file follows rotom).
    assert_eq!(raw.name_rule.var.len(), 22);
    assert_eq!(raw.name_rule.flag.len(), 4);
    for name in raw.name_rule.var.iter().chain(&raw.name_rule.flag) {
        assert_eq!(*name, name.to_ascii_lowercase(), "lower case: {name}");
    }
    for family in [Family::Pt, Family::Hgss] {
        let db = raw_database(family).unwrap();
        // Every `access` of the database has its word.
        for (name, command) in &db.commands {
            for param in &command.params {
                if let Some(access) = &param.access {
                    assert!(
                        raw.access_words.contains_key(access),
                        "{family:?}: {name}.{}: no word for {access}",
                        param.name
                    );
                }
            }
        }
        let rules = raw.family(family).unwrap();
        let table = flagvar_table(family);
        let mut seen = std::collections::BTreeSet::new();
        for (list, entries, role) in [
            (
                "trainer_flags",
                &rules.trainer_flags,
                ParamRole::TrainerFlag,
            ),
            ("flags", &rules.flags, ParamRole::Flag),
            ("vars", &rules.vars, ParamRole::Var),
        ] {
            for (name, params) in entries {
                let command = db
                    .commands
                    .get(name)
                    .unwrap_or_else(|| panic!("{family:?} {list}: no command {name}"));
                assert_ne!(command.kind, "movement", "{family:?} {list}: {name}");
                for (param, access) in params {
                    let arg = command
                        .params
                        .iter()
                        .position(|p| p.name == *param)
                        .unwrap_or_else(|| {
                            panic!("{family:?} {list}: {name} has no parameter {param}")
                        });
                    // One entry per parameter.
                    assert!(
                        seen.insert((name.clone(), param.clone())),
                        "{family:?}: {name}.{param} twice"
                    );
                    let entry = FlagVarParam {
                        name: param.clone(),
                        arg,
                        role,
                        access: *access,
                        direct: !matches!(command.params[arg].ty.as_str(), "var" | "flex"),
                    };
                    assert_eq!(
                        table.param(name, param),
                        Some(&entry),
                        "{family:?} {list}: {name}.{param}"
                    );
                }
            }
        }
    }
    // Diamond and Pearl have no rules.
    assert!(flagvar_table(Family::Dp).commands.is_empty());
}

#[test]
fn the_name_rule_and_the_type_make_flags_and_variables() {
    // Type `flag` (a macro): the flag default.
    assert_eq!(
        role(Family::Pt, "GoToIfSet", "flagID"),
        Some((ParamRole::Flag, Access::Read))
    );
    // Type `var` with `access` in the database.
    assert_eq!(
        role(Family::Pt, "CanFitItem", "destVarID"),
        Some((ParamRole::Var, Access::Write))
    );
    // Type `flex`: a value or a variable.
    assert_eq!(
        role(Family::Pt, "CanFitItem", "item"),
        Some((ParamRole::Var, Access::Read))
    );
    // `may_write` and `read_may_write`.
    assert_eq!(
        role(Family::Pt, "StartVsSeeker", "destVar"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Pt, "CallBattleTowerFunction", "destVar"),
        Some((ParamRole::Var, Access::ReadWrite))
    );
    // The name rule on a `u16`: HeartGold's `HasItem` names its result `variable`.
    assert_eq!(
        role(Family::Hgss, "HasItem", "variable").map(|(r, _)| r),
        Some(ParamRole::Var)
    );
    // A `u8` is never a flag, whatever its name.
    assert_eq!(role(Family::Hgss, "WildBattle", "shiny_flag"), None);
    assert_eq!(role(Family::Hgss, "SafariZoneAction", "flag"), None);
    // A plain `u16` without a fitting name is nothing.
    assert_eq!(role(Family::Pt, "CompareVarToValue", "value"), None);
}

#[test]
fn platinum_variables_take_the_database_access_in_the_table() {
    // The database gives the macro's parameters no `access`: the entry of the file holds.
    let db = raw_database(Family::Pt).unwrap();
    assert!(
        db.commands["SetVar"]
            .params
            .iter()
            .all(|p| p.access.is_none())
    );
    assert_eq!(
        role(Family::Pt, "SetVar", "destVarID"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Pt, "SetVar", "valueOrVarID"),
        Some((ParamRole::Var, Access::Read))
    );
    assert_eq!(
        role(Family::Pt, "SetVarFromValue", "destVarID"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Pt, "AddVar", "destVarID"),
        Some((ParamRole::Var, Access::ReadWrite))
    );
    // A `var` without `access` in the database.
    let unknown = db.commands["HasCoinsFromVar"].params[0].access.is_none();
    assert!(unknown);
    assert_eq!(
        role(Family::Pt, "HasCoinsFromVar", "varID"),
        Some((ParamRole::Var, Access::Unknown))
    );
    // A legacy name finds rotom's: `CompareVarValue` is DSPRE's `CompareVarToValue`.
    let pt = flagvar_table(Family::Pt);
    assert_eq!(pt.params("CompareVarValue").unwrap().0, "CompareVarToValue");
    assert!(pt.params("Message").is_none());
}

#[test]
fn heartgold_borrows_from_platinum_with_exceptions() {
    // Borrowed: Platinum's `CompareVarToValue` reads its `varID`.
    assert_eq!(
        role(Family::Hgss, "CompareVarToValue", "variable"),
        Some((ParamRole::Var, Access::Read))
    );
    assert_eq!(
        role(Family::Hgss, "CompareVarToVar", "variable_2"),
        Some((ParamRole::Var, Access::Read))
    );
    // The exceptions.
    assert_eq!(
        role(Family::Hgss, "SetVar", "variable"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Hgss, "CopyVar", "variable_1"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Hgss, "CopyVar", "variable_2"),
        Some((ParamRole::Var, Access::Read))
    );
    assert_eq!(
        role(Family::Hgss, "SetOrCopyVar", "dst"),
        Some((ParamRole::Var, Access::Write))
    );
    assert_eq!(
        role(Family::Hgss, "AddVar", "variable"),
        Some((ParamRole::Var, Access::ReadWrite))
    );
    // Without a Platinum command of the same name.
    assert_eq!(
        role(Family::Hgss, "HasItem", "variable"),
        Some((ParamRole::Var, Access::Unknown))
    );
    // Flags and trainer flags.
    assert_eq!(
        role(Family::Hgss, "SetFlag", "flag"),
        Some((ParamRole::Flag, Access::Write))
    );
    assert_eq!(
        role(Family::Hgss, "CheckFlagVar", "var_flag"),
        Some((ParamRole::Flag, Access::Read))
    );
    assert_eq!(
        role(Family::Hgss, "SetTrainerFlag", "var_or_trno"),
        Some((ParamRole::TrainerFlag, Access::Write))
    );
}

#[cfg(feature = "test-support")]
mod sources {
    use std::sync::Arc;

    use super::super::super::{RefIndex, script_refs};
    use super::*;
    use crate::flags::{FlagKind, NameRequest, name_flag_files};
    use crate::project::{AssetId, AssetKind};
    use crate::store::Store;
    use crate::test_support::TextFixture;
    use crate::textproject::include::ScriptLookup;

    const SCRIPT_0: AssetId = AssetId {
        kind: AssetKind::Script,
        index: 0,
    };
    const SOURCE_0: &str = "expanded/scripts/0000.rotom";

    fn r(field: &'static str, line: u16, target: Target, access: Access) -> Ref {
        Ref {
            source: SCRIPT_0,
            field,
            slot: Some(line),
            target,
            access,
        }
    }

    fn is_flag_or_var(target: Target) -> bool {
        matches!(target, Target::Flag(_) | Target::Var(_))
    }

    /// The flag and variable references of the index over `fx` with `source` as script file 0,
    /// and its counts.
    fn index_over(fx: &TextFixture, source: &str) -> (Vec<Ref>, ScriptCounts, Store, RefIndex) {
        fx.write(SOURCE_0, source.as_bytes());
        let st = Store::new(Arc::new(fx.project()));
        let mut index = RefIndex::new();
        let refs = index.refs_where(&st, is_flag_or_var).unwrap();
        let counts = index.script_counts(&st).unwrap();
        (refs, counts, st, index)
    }

    /// [`index_over`] on a fresh text project.
    fn flagvars_of(source: &str) -> (Vec<Ref>, ScriptCounts) {
        let fx = TextFixture::exported();
        let (refs, counts, ..) = index_over(&fx, source);
        (refs, counts)
    }

    #[test]
    fn platinum_flag_commands_read_or_write() {
        let (refs, counts) = flagvars_of(
            "script Main #1:\n    SetFlag 148\n    ClearFlag 149\n    CheckFlag 150\n\
             \x20   GoToIfSet 151, Second\n    CallIfUnset FLAG_UNUSED_0x0094, Second\n    End\n\
             script Second #2:\n    End\n",
        );
        assert_eq!(
            refs,
            [
                r("SetFlag", 2, Target::Flag(148), Access::Write),
                r("ClearFlag", 3, Target::Flag(149), Access::Write),
                r("CheckFlag", 4, Target::Flag(150), Access::Read),
                r("GoToIfSet", 5, Target::Flag(151), Access::Read),
                r("CallIfUnset", 6, Target::Flag(148), Access::Read),
            ]
        );
        assert_eq!(counts, ScriptCounts::default());
    }

    #[test]
    fn platinum_variables_take_the_database_access() {
        let (refs, counts) = flagvars_of(
            "script Main #1:\n    CompareVarToValue 0x4030, 2\n    SetVar VAR_UNUSED_0x406C, 1\n\
             \x20   AddVar 0x4031, 0x4032\n    SetVarFromValue 0x4033, 5\n\
             \x20   CompareVarValue 0x4034, 0x4035\n    End\n",
        );
        assert_eq!(
            refs,
            [
                r("CompareVarToValue", 2, Target::Var(0x4030), Access::Read),
                // The macro: the access of its variants, from the file (the database has none).
                r("SetVar", 3, Target::Var(0x406C), Access::Write),
                r("AddVar", 4, Target::Var(0x4031), Access::ReadWrite),
                r("AddVar", 4, Target::Var(0x4032), Access::Read),
                r("SetVarFromValue", 5, Target::Var(0x4033), Access::Write),
                // DSPRE's name of the command counts under rotom's; `value` is a value.
                r("CompareVarToValue", 6, Target::Var(0x4034), Access::Read),
            ]
        );
        assert_eq!(counts, ScriptCounts::default());
    }

    #[test]
    fn heartgold_sources_borrow_and_follow_the_exceptions() {
        let fx = TextFixture::exported();
        let lookup = ScriptLookup::of(&fx.project());
        let (refs, counts) = script_refs(
            0,
            "script Main #1:\n    CompareVarToValue 0x4030, 2\n    SetVar 0x4031, 1\n\
             \x20   CopyVar 0x4032, 0x4033\n    HasItem 1, 1, 0x4034\n    SetTrainerFlag 3\n\
             \x20   SetFlagVar 0x8004\n    End\n",
            Family::Hgss,
            &lookup,
            None,
        )
        .unwrap();
        let refs: Vec<Ref> = refs
            .into_iter()
            .filter(|r| is_flag_or_var(r.target))
            .collect();
        assert_eq!(
            refs,
            [
                r("CompareVarToValue", 2, Target::Var(0x4030), Access::Read),
                r("SetVar", 3, Target::Var(0x4031), Access::Write),
                r("CopyVar", 4, Target::Var(0x4032), Access::Write),
                r("CopyVar", 4, Target::Var(0x4033), Access::Read),
                r("HasItem", 5, Target::Var(0x4034), Access::Unknown),
                r("SetTrainerFlag", 6, Target::Flag(0x553), Access::Write),
            ]
        );
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 0,
                dynamic: 1
            }
        );
    }

    #[test]
    fn trainer_flags_follow_the_trainer_number() {
        let fx = TextFixture::exported();
        let (refs, counts, st, mut index) = index_over(
            &fx,
            "script Main #1:\n    SetTrainerFlag 3\n    CheckTrainerFlag 4\n\
             \x20   ClearTrainerFlag 5\n    SetTrainerFlag VAR_RESULT\n    End\n",
        );
        assert_eq!(
            refs,
            [
                r("SetTrainerFlag", 2, Target::Flag(0x553), Access::Write),
                r("CheckTrainerFlag", 3, Target::Flag(0x554), Access::Read),
                r("ClearTrainerFlag", 4, Target::Flag(0x555), Access::Write),
            ]
        );
        // The trainer stays a reference of its own.
        let trainer = index.refs(&st, Target::Trainer(3)).unwrap();
        assert_eq!(trainer.len(), 1);
        assert_eq!(trainer[0].field, "SetTrainerFlag");
        // A trainer from a variable is only counted.
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 0,
                dynamic: 1
            }
        );
    }

    #[test]
    fn conditions_count_as_reads() {
        let (refs, counts) = flagvars_of(
            "script Main #1:\n\
             \x20   if FLAG_UNUSED_0x0094 then\n\
             \x20       End\n\
             \x20   else if !FLAG_UNUSED_0x0095 && VAR_UNUSED_0x406C != 2 then\n\
             \x20       End\n\
             \x20   endif\n\
             \x20   while VAR_UNUSED_0x406C == 0 do\n\
             \x20       AddVar VAR_UNUSED_0x406C, 1\n\
             \x20   endwhile\n\
             \x20   if CheckItem(ITEM_POTION, 1) == 1 then\n\
             \x20       End\n\
             \x20   endif\n\
             \x20   match VAR_UNUSED_0x406C with\n\
             \x20       case 1:\n\
             \x20           End\n\
             \x20   endmatch\n\
             \x20   if 2 < 0x4031 || VAR_RESULT == 1 then\n\
             \x20       End\n\
             \x20   endif\n\
             \x20   End\n",
        );
        assert_eq!(
            refs,
            [
                r("if", 2, Target::Flag(148), Access::Read),
                r("if", 4, Target::Flag(149), Access::Read),
                r("if", 4, Target::Var(0x406C), Access::Read),
                r("while", 7, Target::Var(0x406C), Access::Read),
                r("AddVar", 8, Target::Var(0x406C), Access::ReadWrite),
                // `CheckItem(…) == 1`: a command, its result no variable.
                r("match", 13, Target::Var(0x406C), Access::Read),
                r("if", 17, Target::Var(0x4031), Access::Read),
            ]
        );
        // `VAR_RESULT` in a comparison is local to the script.
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 1,
                dynamic: 0
            }
        );
    }

    #[test]
    fn dynamic_flags_and_script_locals_are_counted_not_indexed() {
        let fx = TextFixture::exported();
        let (refs, counts, st, mut index) = index_over(
            &fx,
            "alias UNSET as NO_VALUE\n\
             script Main #1:\n    SetFlag VAR_RESULT\n    SetFlagFromVar VAR_UNUSED_0x406C\n\
             \x20   CheckFlagFromVar 0x8004, VAR_RESULT\n    SetVar VAR_RESULT, 1\n\
             \x20   CompareVarToValue 5, 1\n    SetVar UNKNOWN_NAME, 1\n    SetFlag NO_VALUE\n\
             \x20   ClearFlag Main\n    End\n",
        );
        assert_eq!(refs, []);
        assert!(index.refs(&st, Target::Var(0x800C)).unwrap().is_empty());
        assert_eq!(
            counts,
            ScriptCounts {
                // `CheckFlagFromVar`'s result and `SetVar VAR_RESULT`.
                script_local: 2,
                // `SetFlag VAR_RESULT`, `SetFlagFromVar`, `CheckFlagFromVar`.
                dynamic: 3
            }
        );
    }

    #[test]
    fn an_own_name_counts_like_its_number() {
        let fx = TextFixture::exported();
        name_flag_files(
            &fx.project(),
            &NameRequest {
                kind: FlagKind::Flag,
                number: 148,
                id: Some("FLAG_HAT_MEDAILLE".into()),
                description: None,
            },
        )
        .unwrap();
        // The build's lookup knows the id as an own flag name (R17: from 0x4000 on it counts).
        let lookup = ScriptLookup::of(&fx.project());
        assert!(lookup.is_flag_id("FLAG_HAT_MEDAILLE"));
        assert!(!lookup.is_flag_id("FLAG_UNUSED_0x0094"));
        let (refs, ..) = index_over(
            &fx,
            "script Main #1:\n    SetFlag FLAG_HAT_MEDAILLE\n    if FLAG_HAT_MEDAILLE then\n\
             \x20       End\n    endif\n    End\n",
        );
        assert_eq!(
            refs,
            [
                r("SetFlag", 2, Target::Flag(148), Access::Write),
                r("if", 3, Target::Flag(148), Access::Read),
            ]
        );
    }

    #[test]
    fn an_alias_of_the_file_resolves() {
        let (refs, _) = flagvars_of(
            "alias FLAG_UNUSED_0x0094 as MEIN_FLAG\n\
             script Main #1:\n    SetFlag MEIN_FLAG\n    End\n",
        );
        assert_eq!(refs, [r("SetFlag", 3, Target::Flag(148), Access::Write)]);
    }

    #[test]
    fn a_source_that_does_not_parse_contributes_nothing_for_flags() {
        let (refs, counts) = flagvars_of(
            "SetFlag 148 ((( ]]] \u{0}\nif FLAG_UNUSED_0x0094 then\nSetVar 0x4030, VAR_RESULT\n",
        );
        assert_eq!(refs, []);
        assert_eq!(counts, ScriptCounts::default());
    }

    #[test]
    fn a_zero_flag_is_no_reference() {
        let (refs, counts) = flagvars_of(
            "script Main #1:\n    SetFlag 0\n    CheckFlag 0\n    SetTrainerFlag 0\n\
             \x20   SetVar 0, 1\n    End\n",
        );
        assert_eq!(refs, []);
        assert_eq!(counts, ScriptCounts::default());
    }

    /// The flag and variable references of `source` as script file 0 of a HeartGold project
    /// whose build knows `ids` (name, number), `flag_ids` among them own flag names, and its
    /// counts.
    fn heartgold_refs(
        source: &str,
        ids: &[(&str, i32)],
        flag_ids: &[&str],
    ) -> (Vec<Ref>, ScriptCounts) {
        let lookup = ScriptLookup::of_ids(ids, flag_ids);
        let (refs, counts) = script_refs(0, source, Family::Hgss, &lookup, None).unwrap();
        let refs = refs
            .into_iter()
            .filter(|r| is_flag_or_var(r.target))
            .collect();
        (refs, counts)
    }

    #[test]
    fn a_named_flag_from_0x4000_on_is_a_flag() {
        // HeartGold's temporary flags (R17): a pret flag name, an alias of it and an own flag
        // name count; a plain number and a variable name stay dynamic.
        let (refs, counts) = heartgold_refs(
            "alias FLAG_TEMP_x4002 as MEIN_TEMP\n\
             script Main #1:\n    SetFlag FLAG_TEMP_x4000\n    CheckFlag MEIN_TEMP\n\
             \x20   ClearFlag FLAG_EIGENES_TEMP\n    SetFlag VAR_RESULT\n    SetFlag 0x4001\n\
             \x20   if FLAG_TEMP_x4000 then\n        End\n    endif\n    End\n",
            &[
                ("FLAG_TEMP_x4000", 0x4000),
                ("FLAG_TEMP_x4002", 0x4002),
                ("FLAG_EIGENES_TEMP", 0x4003),
                ("VAR_RESULT", 0x800C),
            ],
            &["FLAG_EIGENES_TEMP"],
        );
        assert_eq!(
            refs,
            [
                r("SetFlag", 3, Target::Flag(0x4000), Access::Write),
                r("CheckFlag", 4, Target::Flag(0x4002), Access::Read),
                r("ClearFlag", 5, Target::Flag(0x4003), Access::Write),
                r("if", 8, Target::Flag(0x4000), Access::Read),
            ]
        );
        // `SetFlag VAR_RESULT` and `SetFlag 0x4001`.
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 0,
                dynamic: 2
            }
        );
    }

    #[test]
    fn a_flag_parameter_read_as_a_variable_stays_dynamic_whatever_its_name() {
        // `SetFlagVar` reads its parameter (type `var`) as a variable from 0x4000 on: the
        // decompiler's `FLAG_TEMP_x4001` there is variable 0x4001 holding the flag (vanilla
        // HeartGold script 0952 sets `VAR_TEMP_x4001` to 653 first).
        let (refs, counts) = heartgold_refs(
            "script Main #1:\n    SetFlagVar FLAG_TEMP_x4001\n    ClearFlagVar FLAG_TEMP_x4001\n\
             \x20   SetFlagVar 148\n    End\n",
            &[("FLAG_TEMP_x4001", 0x4001)],
            &[],
        );
        assert_eq!(refs, [r("SetFlagVar", 4, Target::Flag(148), Access::Write)]);
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 0,
                dynamic: 2
            }
        );
    }

    #[test]
    fn platinum_galactic_hq_state_is_a_flag() {
        // 0x410D, the one Platinum flag name from 0x4000 on (R17).
        let (refs, counts) = flagvars_of(
            "alias FLAG_GALACTIC_HQ_CONTROL_ROOM_STATE as ZENTRALE\n\
             script Main #1:\n    CheckFlag FLAG_GALACTIC_HQ_CONTROL_ROOM_STATE\n\
             \x20   SetFlag ZENTRALE\n    SetFlag 0x410D\n    End\n",
        );
        assert_eq!(
            refs,
            [
                r("CheckFlag", 3, Target::Flag(0x410D), Access::Read),
                r("SetFlag", 4, Target::Flag(0x410D), Access::Write),
            ]
        );
        assert_eq!(
            counts,
            ScriptCounts {
                script_local: 0,
                dynamic: 1
            }
        );
    }
}
