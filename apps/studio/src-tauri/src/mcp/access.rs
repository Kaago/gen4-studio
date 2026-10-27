//! What each command does to the project, and whether an agent's access admits it.

use serde_json::Value;

use crate::commands::policy::UserPolicy;
use crate::errors::CmdError;

/// What a command does, as far as its name tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Changes nothing.
    Read,
    /// Changes the project in an undoable step (or only the agent's own state).
    Write,
    /// Changes something that undo does not bring back; the text says what.
    Irreversible(&'static str),
    /// Not for an agent; the text says why.
    Unavailable(&'static str),
    /// Decided per call by [`classify`] from the arguments.
    Depends,
}

const NEW_SCRIPT: Kind = Kind::Irreversible("Script anlegen");
const APP_ONLY: Kind = Kind::Unavailable("Nur in der App");

/// Why an agent may not name a project folder: the server serves one project.
const BOUND_PROJECT: &str = "Das Projekt ist fest (--project); ohne path aufrufen";
/// Why the fix `remove_hidden` cannot be undone: it moves the file past the store.
const REMOVE_HIDDEN: &str = "Versteckte Datei nach .g4/backup verschieben";

/// Why a build or export outside `build/` cannot be undone: it may overwrite any file there.
const OUTSIDE_BUILD: &str = "Bauen außerhalb von build/";

/// The kind of every command of the table (`dispatch::COMMANDS`), in its order.
pub const COMMAND_KINDS: &[(&str, Kind)] = &[
    (
        "open_project",
        Kind::Unavailable("Das Projekt ist fest (--project)"),
    ),
    ("list_headers", Kind::Read),
    ("get_header", Kind::Read),
    ("set_header", Kind::Write),
    ("get_area_data", Kind::Read),
    ("set_area_data", Kind::Write),
    ("member_counts", Kind::Read),
    ("get_matrix", Kind::Read),
    ("set_matrix_cell", Kind::Write),
    ("set_matrix_sections", Kind::Write),
    ("resize_matrix", Kind::Write),
    ("rename_matrix", Kind::Write),
    ("create_matrix", Kind::Write),
    ("list_matrices", Kind::Read),
    ("list_warps", Kind::Read),
    ("duplicate_map", Kind::Write),
    ("create_event_file", Kind::Write),
    ("duplicate_event_file", Kind::Write),
    ("create_area", Kind::Depends),
    ("extend_area", Kind::Write),
    ("create_room", Kind::Depends),
    ("split_room", Kind::Write),
    ("header_patch_status", Kind::Read),
    (
        "apply_header_patch",
        Kind::Irreversible("Header-Patch einspielen"),
    ),
    (
        "repair_header_patch",
        Kind::Irreversible("Header-Patch reparieren"),
    ),
    ("get_map", Kind::Read),
    ("get_events", Kind::Read),
    ("get_building_glb", Kind::Read),
    ("add_building", Kind::Write),
    ("update_building", Kind::Write),
    ("delete_building", Kind::Write),
    ("update_event", Kind::Write),
    ("get_text_archive", Kind::Read),
    ("edit_text_archive", Kind::Write),
    ("create_text_archive", Kind::Write),
    ("search_text", Kind::Read),
    ("validate_text", Kind::Read),
    ("scripts_status", Kind::Read),
    ("adopt_scripts", Kind::Write),
    ("list_script_files", Kind::Read),
    ("read_script", Kind::Read),
    ("save_script", Kind::Write),
    ("save_level_script", Kind::Write),
    ("new_script_file", NEW_SCRIPT),
    ("add_script", NEW_SCRIPT),
    ("resolve_script", Kind::Read),
    ("search_scripts", Kind::Read),
    ("lsp_start", APP_ONLY),
    ("lsp_send", APP_ONLY),
    ("lsp_stop", APP_ONLY),
    ("add_event", Kind::Write),
    ("delete_event", Kind::Write),
    ("get_map_model_glb", Kind::Read),
    ("get_object_sprite", Kind::Read),
    ("map_model_texture_status", Kind::Read),
    ("tile_behaviors", Kind::Read),
    ("set_permission", Kind::Write),
    ("undo", Kind::Write),
    ("redo", Kind::Write),
    ("history", Kind::Read),
    ("dirty", Kind::Read),
    ("save", Kind::Write),
    ("build_rom", Kind::Depends),
    ("run_rom", Kind::Irreversible("ROM im Emulator starten")),
    ("reload_asset", Kind::Write),
    ("accept_disk", Kind::Write),
    (
        "resolve_text_conflicts",
        Kind::Irreversible("Eine Fassung eines Textarchivs überschreiben"),
    ),
    ("force_quit", APP_ONLY),
    ("data_list", Kind::Read),
    ("data_get", Kind::Read),
    ("data_edit", Kind::Write),
    ("data_raw", Kind::Read),
    ("name_tables", Kind::Read),
    ("item_table", Kind::Read),
    ("machine_table", Kind::Read),
    ("refs", Kind::Read),
    ("get_item_icon", Kind::Read),
    ("get_mon_icon", Kind::Read),
    ("create_trainer", Kind::Irreversible("Trainer anlegen")),
    ("set_trainer_texts", Kind::Write),
    ("create_encounter_file", Kind::Write),
    ("split_encounter_file", Kind::Write),
    ("inspect_map_import", Kind::Read),
    ("import_map_parts", Kind::Write),
    ("export_map_part", Kind::Depends),
    ("run_checks", Kind::Read),
    ("checks_status", Kind::Read),
    ("ignore_finding", Kind::Write),
    ("unignore_finding", Kind::Write),
    (
        "accept_baseline",
        Kind::Irreversible("Ausgangslage übernehmen"),
    ),
    ("set_check_enabled", Kind::Write),
    ("plan_fix", Kind::Read),
    ("apply_fix", Kind::Depends),
    ("inspect_rom", Kind::Read),
    ("default_project_parent", APP_ONLY),
    ("check_project_dest", APP_ONLY),
    ("create_project", APP_ONLY),
    ("plan_empty_region", Kind::Read),
    ("apply_empty_region", Kind::Irreversible("Welt leeren")),
    ("agent_status", APP_ONLY),
    ("agent_poll", APP_ONLY),
    ("agent_settings", APP_ONLY),
    ("agent_set_enabled", APP_ONLY),
    ("agent_set_expert", APP_ONLY),
    ("agent_set_grant", APP_ONLY),
    ("agent_remove_grant", APP_ONLY),
    ("agent_answer", APP_ONLY),
    ("agent_disconnect", APP_ONLY),
    ("agent_set_read_only", APP_ONLY),
    ("world_meta_get", Kind::Read),
    ("world_meta_set", Kind::Write),
    ("place_slots", Kind::Read),
    ("create_place", Kind::Write),
    ("rename_place", Kind::Write),
    ("assign_place", Kind::Write),
    ("project_preview_get", Kind::Read),
    (
        "project_preview_set",
        Kind::Irreversible("Vorschaubild ändern"),
    ),
    ("base_status", Kind::Read),
    ("check_base", Kind::Read),
    ("assign_base", Kind::Irreversible("Basis-ROM zuordnen")),
    ("register_base_rom", APP_ONLY),
    ("create_patch", Kind::Irreversible("Patch erstellen")),
];

/// The kind of `command`; `None` for a name the table does not have.
pub fn kind_of(command: &str) -> Option<Kind> {
    COMMAND_KINDS
        .iter()
        .find(|(name, _)| *name == command)
        .map(|(_, kind)| *kind)
}

/// What one call does, with its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Read,
    Write,
    /// Cannot be undone; the text says what it does.
    Irreversible(String),
    /// Refused for every agent; the text says why.
    Unavailable(String),
}

impl From<Kind> for Class {
    /// The class of a kind that does not depend on the arguments (`Depends` counts as a
    /// write; [`classify`] resolves it).
    fn from(kind: Kind) -> Class {
        match kind {
            Kind::Read => Class::Read,
            Kind::Write | Kind::Depends => Class::Write,
            Kind::Irreversible(why) => Class::Irreversible(why.to_owned()),
            Kind::Unavailable(why) => Class::Unavailable(why.to_owned()),
        }
    }
}

/// What the call of `command` with `args` (keyed as `dispatch` takes them) does. A build or
/// map part export lands in `build/` (a write; `build_rom` without `out` goes there too) or
/// somewhere else the policy allows (irreversible); `apply_fix` with `create_script` creates
/// a script, with `remove_hidden` moves a file to `.g4/backup/` (both irreversible);
/// `create_area` and `create_room` with `header.allow_patch` may apply the Dynamic Headers
/// patch. The project preview commands with a `path` (another project folder) are
/// unavailable, also when it names the bound project. A name the table does not have is
/// unavailable.
pub fn classify(command: &str, args: &Value, policy: &UserPolicy) -> Class {
    let Some(kind) = kind_of(command) else {
        return Class::Unavailable(format!("Unbekannter Befehl: {command}"));
    };
    if matches!(command, "project_preview_get" | "project_preview_set")
        && args.get("path").is_some_and(|path| !path.is_null())
    {
        return Class::Unavailable(BOUND_PROJECT.to_owned());
    }
    if kind != Kind::Depends {
        return kind.into();
    }
    match command {
        "build_rom" | "export_map_part" => {
            let key = if command == "build_rom" {
                "out"
            } else {
                "path"
            };
            match args.get(key) {
                // A target that is not text is refused by the command itself.
                Some(Value::String(target)) if !policy.is_in_build(target) => {
                    Class::Irreversible(OUTSIDE_BUILD.to_owned())
                }
                _ => Class::Write,
            }
        }
        "apply_fix" if args.get("fix").and_then(Value::as_str) == Some("create_script") => {
            NEW_SCRIPT.into()
        }
        "apply_fix" if args.get("fix").and_then(Value::as_str) == Some("remove_hidden") => {
            Kind::Irreversible(REMOVE_HIDDEN).into()
        }
        "create_area" | "create_room"
            if args.pointer("/spec/header/allow_patch") == Some(&Value::Bool(true)) =>
        {
            Kind::Irreversible("Header-Patch einspielen").into()
        }
        _ => Class::Write,
    }
}

/// What an agent may do: change the project (`write`), do what undo does not bring back
/// (`irreversible`, needs `write`), and call any command by name (`expert`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    pub write: bool,
    pub irreversible: bool,
    pub expert: bool,
}

fn refusal(code: &'static str, message: String) -> CmdError {
    CmdError {
        code,
        message,
        asset: None,
        problems: Vec::new(),
    }
}

/// Whether `access` admits a call of `command` of class `class`: `unavailable` for every
/// agent, `read_only` for a change without `write`, `needs_confirmation` for an irreversible
/// one without `irreversible`.
pub fn admit(command: &str, class: &Class, access: &Access) -> Result<(), CmdError> {
    match class {
        Class::Read => Ok(()),
        Class::Unavailable(why) => Err(refusal("unavailable", why.clone())),
        Class::Write | Class::Irreversible(_) if !access.write => Err(refusal(
            "read_only",
            format!("Nur lesen: {command} ändert das Projekt"),
        )),
        Class::Irreversible(why) if !access.irreversible => Err(refusal(
            "needs_confirmation",
            format!(
                "{command} lässt sich nicht rückgängig machen ({why}); \
                 kopflos nur mit --allow-irreversible"
            ),
        )),
        Class::Write | Class::Irreversible(_) => Ok(()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::dispatch::COMMANDS;
    use crate::commands::open_synthetic;
    use serde_json::json;
    use std::collections::BTreeSet;

    fn policy_over(fixture: &g4_kernel::test_support::Synthetic) -> UserPolicy {
        UserPolicy::new(fixture.dir.path(), &[], None).unwrap()
    }

    #[test]
    fn every_command_has_a_kind() {
        let kinds: Vec<&str> = COMMAND_KINDS.iter().map(|(name, _)| *name).collect();
        let kind_set: BTreeSet<&str> = kinds.iter().copied().collect();
        let commands: BTreeSet<&str> = COMMANDS.iter().copied().collect();
        assert_eq!(kinds.len(), kind_set.len(), "a command has two kinds");
        let missing: Vec<_> = commands.difference(&kind_set).collect();
        let extra: Vec<_> = kind_set.difference(&commands).collect();
        assert!(missing.is_empty(), "without a kind: {missing:?}");
        assert!(extra.is_empty(), "not a command: {extra:?}");
    }

    #[test]
    fn irreversible_ones_are_the_decided_groups() {
        let of = |wanted: fn(&Kind) -> bool| -> Vec<(&str, Kind)> {
            COMMAND_KINDS
                .iter()
                .filter(|(_, kind)| wanted(kind))
                .map(|(name, kind)| (*name, *kind))
                .collect()
        };
        let mut irreversible = of(|k| matches!(k, Kind::Irreversible(_)));
        irreversible.sort_by_key(|(name, _)| *name);
        assert_eq!(
            irreversible,
            [
                (
                    "accept_baseline",
                    Kind::Irreversible("Ausgangslage übernehmen")
                ),
                ("add_script", Kind::Irreversible("Script anlegen")),
                ("apply_empty_region", Kind::Irreversible("Welt leeren")),
                (
                    "apply_header_patch",
                    Kind::Irreversible("Header-Patch einspielen")
                ),
                ("assign_base", Kind::Irreversible("Basis-ROM zuordnen")),
                ("create_patch", Kind::Irreversible("Patch erstellen")),
                ("create_trainer", Kind::Irreversible("Trainer anlegen")),
                ("new_script_file", Kind::Irreversible("Script anlegen")),
                (
                    "project_preview_set",
                    Kind::Irreversible("Vorschaubild ändern")
                ),
                (
                    "repair_header_patch",
                    Kind::Irreversible("Header-Patch reparieren")
                ),
                (
                    "resolve_text_conflicts",
                    Kind::Irreversible("Eine Fassung eines Textarchivs überschreiben")
                ),
                ("run_rom", Kind::Irreversible("ROM im Emulator starten")),
            ]
        );
        // The agent hub's commands: `agent_commands_are_app_only`.
        let mut unavailable = of(|k| matches!(k, Kind::Unavailable(_)));
        unavailable.retain(|(name, _)| !name.starts_with("agent_"));
        unavailable.sort_by_key(|(name, _)| *name);
        assert_eq!(
            unavailable,
            [
                ("check_project_dest", Kind::Unavailable("Nur in der App")),
                ("create_project", Kind::Unavailable("Nur in der App")),
                (
                    "default_project_parent",
                    Kind::Unavailable("Nur in der App")
                ),
                ("force_quit", Kind::Unavailable("Nur in der App")),
                ("lsp_send", Kind::Unavailable("Nur in der App")),
                ("lsp_start", Kind::Unavailable("Nur in der App")),
                ("lsp_stop", Kind::Unavailable("Nur in der App")),
                (
                    "open_project",
                    Kind::Unavailable("Das Projekt ist fest (--project)")
                ),
                ("register_base_rom", Kind::Unavailable("Nur in der App")),
            ]
        );
        let mut depends: Vec<&str> = of(|k| *k == Kind::Depends)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        depends.sort();
        assert_eq!(
            depends,
            [
                "apply_fix",
                "build_rom",
                "create_area",
                "create_room",
                "export_map_part"
            ]
        );
        // Every lister and getter only reads; a few writers by name.
        for (name, kind) in COMMAND_KINDS {
            if ["get_", "list_", "search_"]
                .iter()
                .any(|p| name.starts_with(p))
            {
                assert_eq!(*kind, Kind::Read, "{name}");
            }
        }
        let kind = |name: &str| COMMAND_KINDS.iter().find(|(n, _)| *n == name).unwrap().1;
        for name in [
            "data_list",
            "plan_fix",
            "run_checks",
            "inspect_map_import",
            "history",
            "dirty",
            "validate_text",
            "base_status",
            "check_base",
        ] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        for name in [
            "save",
            "undo",
            "set_header",
            "data_edit",
            "adopt_scripts",
            "save_script",
            "reload_asset",
            "ignore_finding",
        ] {
            assert_eq!(kind(name), Kind::Write, "{name}");
        }
    }

    #[test]
    fn preview_with_a_path_is_unavailable() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let root = fixture.dir.path().to_str().unwrap();
        for command in ["project_preview_get", "project_preview_set"] {
            assert_eq!(
                classify(command, &json!({ "path": root }), &policy),
                Class::Unavailable(BOUND_PROJECT.to_owned()),
                "{command}"
            );
        }
        // Without one (or with null, as `api.preview.ts` sends it): the table's kind.
        assert_eq!(
            classify("project_preview_get", &json!({ "path": null }), &policy),
            Class::Read
        );
        assert_eq!(
            classify("project_preview_set", &json!({ "png": "" }), &policy),
            Class::Irreversible("Vorschaubild ändern".to_owned())
        );
    }

    #[test]
    fn agent_commands_are_app_only() {
        // An agent must not grant itself access.
        let agents: Vec<&str> = COMMANDS
            .iter()
            .copied()
            .filter(|name| name.starts_with("agent_"))
            .collect();
        assert_eq!(
            agents,
            [
                "agent_status",
                "agent_poll",
                "agent_settings",
                "agent_set_enabled",
                "agent_set_expert",
                "agent_set_grant",
                "agent_remove_grant",
                "agent_answer",
                "agent_disconnect",
                "agent_set_read_only",
            ]
        );
        for name in agents {
            assert_eq!(
                kind_of(name),
                Some(Kind::Unavailable("Nur in der App")),
                "{name}"
            );
        }
    }

    #[test]
    fn build_inside_build_is_write_outside_irreversible() {
        let (fixture, _state) = open_synthetic();
        let elsewhere = tempfile::tempdir().unwrap();
        let allowed = [elsewhere.path().to_path_buf()];
        let policy = UserPolicy::new(fixture.dir.path(), &allowed, None).unwrap();
        let outside = elsewhere.path().join("out.nds");
        let outside = outside.to_str().unwrap();
        let outside_build = Class::Irreversible("Bauen außerhalb von build/".to_owned());
        for (command, key) in [("build_rom", "out"), ("export_map_part", "path")] {
            assert_eq!(
                classify(command, &json!({ key: "build/route.nds" }), &policy),
                Class::Write,
                "{command}"
            );
            assert_eq!(
                classify(command, &json!({ key: outside }), &policy),
                outside_build,
                "{command}"
            );
        }
        // Without a target the build goes to build/ (the toolbox fills it in).
        assert_eq!(classify("build_rom", &json!({}), &policy), Class::Write);
        // The other kinds come straight from the table.
        assert_eq!(
            classify("get_header", &json!({ "id": 0 }), &policy),
            Class::Read
        );
        assert_eq!(classify("save", &json!({}), &policy), Class::Write);
        assert_eq!(
            classify("create_trainer", &json!({}), &policy),
            Class::Irreversible("Trainer anlegen".to_owned())
        );
        assert_eq!(
            classify("open_project", &json!({}), &policy),
            Class::Unavailable("Das Projekt ist fest (--project)".to_owned())
        );
        assert_eq!(
            classify("no_such_command", &json!({}), &policy),
            Class::Unavailable("Unbekannter Befehl: no_such_command".to_owned())
        );
    }

    #[test]
    fn create_area_with_allow_patch_needs_a_confirmation() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let header = |allow_patch: Value| {
            json!({ "id": 2, "area_data_id": null, "event_file": "New",
                    "new_header": { "template": 1, "internal_name": null },
                    "allow_patch": allow_patch })
        };
        let patch = Class::Irreversible("Header-Patch einspielen".to_owned());
        assert_eq!(
            classify(
                "create_area",
                &json!({ "spec": { "header": header(json!(true)) } }),
                &policy
            ),
            patch
        );
        assert_eq!(
            classify(
                "create_room",
                &json!({ "spec": { "header": header(json!(true)) } }),
                &policy
            ),
            patch
        );
        // Without the flag (or without a header) it is an ordinary write.
        for spec in [
            json!({ "header": header(json!(false)) }),
            json!({ "header": header(Value::Null) }),
            json!({ "header": null }),
            json!({}),
        ] {
            for command in ["create_area", "create_room"] {
                assert_eq!(
                    classify(command, &json!({ "spec": spec }), &policy),
                    Class::Write,
                    "{command} {spec}"
                );
            }
        }
    }

    #[test]
    fn apply_fix_create_script_is_irreversible() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let finding = json!({ "code": "x", "subject": { "kind": "project", "index": 0 },
                              "key": "", "fingerprint": "0000000000000000" });
        assert_eq!(
            classify(
                "apply_fix",
                &json!({ "finding": finding, "fix": "create_script" }),
                &policy
            ),
            Class::Irreversible("Script anlegen".to_owned())
        );
        // Removing a hidden file moves it past the store.
        assert_eq!(
            classify(
                "apply_fix",
                &json!({ "finding": finding, "fix": "remove_hidden" }),
                &policy
            ),
            Class::Irreversible("Versteckte Datei nach .g4/backup verschieben".to_owned())
        );
        assert_eq!(
            classify(
                "apply_fix",
                &json!({ "finding": finding, "fix": "wrap" }),
                &policy
            ),
            Class::Write
        );
        // Planning a fix changes nothing, whatever the fix.
        assert_eq!(
            classify(
                "plan_fix",
                &json!({ "finding": finding, "fix": "create_script" }),
                &policy
            ),
            Class::Read
        );
    }

    #[test]
    fn resolve_text_conflicts_needs_a_confirmation() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let class = classify(
            "resolve_text_conflicts",
            &json!({ "choices": [{ "archive": 12, "keep": "json" }] }),
            &policy,
        );
        assert_eq!(
            class,
            Class::Irreversible("Eine Fassung eines Textarchivs überschreiben".to_owned())
        );
        let writer = Access {
            write: true,
            irreversible: false,
            expert: false,
        };
        let e = refused("resolve_text_conflicts", &class, writer);
        assert_eq!(e.code, "needs_confirmation");
        let confirmed = Access {
            irreversible: true,
            ..writer
        };
        assert!(admit("resolve_text_conflicts", &class, &confirmed).is_ok());
    }

    fn refused(command: &str, class: &Class, access: Access) -> CmdError {
        admit(command, class, &access).expect_err(command)
    }

    #[test]
    fn admit_refuses_writes_when_read_only() {
        let read_only = Access::default();
        assert!(admit("get_header", &Class::Read, &read_only).is_ok());
        let e = refused("save", &Class::Write, read_only);
        assert_eq!(
            (e.code, e.message.as_str()),
            ("read_only", "Nur lesen: save ändert das Projekt")
        );
        // An irreversible command writes as well; read-only says that first.
        let e = refused(
            "create_trainer",
            &Class::Irreversible("Trainer anlegen".to_owned()),
            Access {
                irreversible: true,
                ..read_only
            },
        );
        assert_eq!(
            (e.code, e.message.as_str()),
            ("read_only", "Nur lesen: create_trainer ändert das Projekt")
        );
        let writer = Access {
            write: true,
            ..read_only
        };
        assert!(admit("save", &Class::Write, &writer).is_ok());
        // Unavailable is unavailable for everyone, with its reason.
        let all = Access {
            write: true,
            irreversible: true,
            expert: true,
        };
        let e = refused(
            "lsp_start",
            &Class::Unavailable("Nur in der App".to_owned()),
            all,
        );
        assert_eq!(
            (e.code, e.message.as_str()),
            ("unavailable", "Nur in der App")
        );
    }

    #[test]
    fn admit_refuses_irreversible_without_confirmation() {
        let class = Class::Irreversible("Trainer anlegen".to_owned());
        let writer = Access {
            write: true,
            irreversible: false,
            expert: false,
        };
        let e = refused("create_trainer", &class, writer);
        assert_eq!(e.code, "needs_confirmation");
        assert_eq!(
            e.message,
            "create_trainer lässt sich nicht rückgängig machen (Trainer anlegen); \
             kopflos nur mit --allow-irreversible"
        );
        assert!(e.asset.is_none() && e.problems.is_empty());
        let confirmed = Access {
            irreversible: true,
            ..writer
        };
        assert!(admit("create_trainer", &class, &confirmed).is_ok());
    }
}
