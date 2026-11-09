//! What each command does to the project, and whether an agent's access admits it.

use g4_kernel::recipes::{self, RecipeCall};
use g4_kernel::store::Store;
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
    /// Changes something that undo does not bring back; the payload is the catalog key
    /// (`mcp.access.*`) of the reason, which [`Class`] resolves in the language of the server.
    Irreversible(&'static str),
    /// Not for an agent; the payload is the catalog key of the reason.
    Unavailable(&'static str),
    /// Decided per call by [`classify`] from the arguments.
    Depends,
}

const NEW_SCRIPT: Kind = Kind::Irreversible("mcp.access.new_script");
const APP_ONLY: Kind = Kind::Unavailable("mcp.access.app_only");
const HEADER_PATCH_APPLY: &str = "mcp.access.header_patch_apply";
const RUN_ROM: &str = "mcp.access.run_rom";

// The reasons below are catalog keys, resolved by [`Class`] when a call is classified.
/// Why an agent may not name a project folder: the server serves one project.
const BOUND_PROJECT: &str = "mcp.access.bound_project";
/// Why the fix `remove_hidden` cannot be undone: it moves the file past the store.
const REMOVE_HIDDEN: &str = "mcp.access.remove_hidden";

/// Why a build or export outside `build/` cannot be undone: it may overwrite any file there.
const OUTSIDE_BUILD: &str = "mcp.access.outside_build";
/// Why `publish_patch` with `overwrite` cannot be undone: it replaces the files of a release.
const REPLACE_PATCH: &str = "mcp.access.replace_patch";

/// The kind of every command of the table (`dispatch::COMMANDS`), in its order.
pub const COMMAND_KINDS: &[(&str, Kind)] = &[
    (
        "open_project",
        Kind::Unavailable("mcp.access.bound_project_short"),
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
    ("apply_header_patch", Kind::Irreversible(HEADER_PATCH_APPLY)),
    (
        "repair_header_patch",
        Kind::Irreversible("mcp.access.header_patch_repair"),
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
    ("run_rom", Kind::Irreversible(RUN_ROM)),
    ("reload_asset", Kind::Write),
    ("accept_disk", Kind::Write),
    (
        "resolve_text_conflicts",
        Kind::Irreversible("mcp.access.overwrite_text_archive"),
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
    (
        "create_trainer",
        Kind::Irreversible("mcp.access.create_trainer"),
    ),
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
        Kind::Irreversible("mcp.access.accept_baseline"),
    ),
    ("set_check_enabled", Kind::Write),
    ("plan_fix", Kind::Read),
    ("apply_fix", Kind::Depends),
    ("inspect_rom", Kind::Read),
    ("default_project_parent", APP_ONLY),
    ("check_project_dest", APP_ONLY),
    ("create_project", APP_ONLY),
    ("plan_empty_region", Kind::Read),
    (
        "apply_empty_region",
        Kind::Irreversible("mcp.access.clear_world"),
    ),
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
        Kind::Irreversible("mcp.access.change_preview"),
    ),
    ("base_status", Kind::Read),
    ("check_base", Kind::Read),
    ("assign_base", Kind::Irreversible("mcp.access.assign_base")),
    ("register_base_rom", APP_ONLY),
    (
        "create_patch",
        Kind::Irreversible("mcp.access.create_patch"),
    ),
    ("original_status", Kind::Read),
    ("plan_reset", Kind::Read),
    ("reset_to_original", Kind::Write),
    ("changed_from_original", Kind::Read),
    ("text_check", Kind::Read),
    ("text_export", Kind::Irreversible("mcp.access.export_text")),
    ("text_project_status", Kind::Read),
    (
        "adopt_text_project",
        Kind::Irreversible("mcp.access.adopt_text_project"),
    ),
    ("revert_text_project", APP_ONLY),
    ("migrate_text_project", APP_ONLY),
    ("external_sync", APP_ONLY),
    ("external_changes", Kind::Read),
    ("conflict_diff", Kind::Read),
    (
        "resolve_external_conflict",
        Kind::Irreversible("mcp.access.overwrite_version"),
    ),
    ("plan_rename_id", Kind::Read),
    ("rename_id", Kind::Write),
    ("entity_ids", Kind::Read),
    ("close_project", APP_ONLY),
    ("git_status", Kind::Read),
    ("git_init", Kind::Irreversible("mcp.access.init_git")),
    ("diff_entity", Kind::Read),
    ("unsaved_changes", Kind::Read),
    // A clone is completed before it opens: an agent has no project yet.
    ("restore_text_project", APP_ONLY),
    ("get_battle_sprite", Kind::Read),
    ("get_trainer_sprite", Kind::Read),
    ("releases", Kind::Read),
    ("publish_patch", Kind::Depends),
    // An agent reads the list from the answers of publish_patch; it opens no file manager.
    ("reveal_path", APP_ONLY),
    (
        "git_commit_all",
        Kind::Irreversible("mcp.access.commit_all"),
    ),
    // Writes the template block (as "Save a snapshot" would), then reads the status.
    ("git_commit_prepare", Kind::Write),
    ("recipe_list", Kind::Read),
    ("recipe_plan", Kind::Read),
    // Irreversible as the recipe says (`recipes::is_irreversible`), see [`classify`].
    ("recipe_apply", Kind::Depends),
    ("list_flags", Kind::Read),
    ("flag_usage", Kind::Read),
    ("free_flag", Kind::Read),
    // One undo step, like rename_id.
    ("name_flag", Kind::Write),
    ("dialog_of", Kind::Read),
    // One undo step: the archive's copy and the script file's step, squashed.
    ("split_message", Kind::Write),
    ("list_threads", Kind::Read),
    ("thread_graph", Kind::Read),
    ("story_list", Kind::Read),
    ("story_get", Kind::Read),
    // One undo step each (a file step: register, story, side file).
    ("story_create", Kind::Write),
    ("story_update", Kind::Write),
    // Play mode live: an agent reads the status and sets a value in the running game (no undo
    // step); starting melonDS asks like run_rom; the rest belongs to the app's surface.
    ("live_status", Kind::Read),
    ("live_play", Kind::Irreversible(RUN_ROM)),
    ("live_connect", APP_ONLY),
    ("live_stop", APP_ONLY),
    ("live_resume", APP_ONLY),
    ("live_set", Kind::Write),
    ("live_breakpoints", APP_ONLY),
    ("live_breakpoint_set", APP_ONLY),
    ("live_breakpoint_clear", APP_ONLY),
    ("dialogs_of_header", Kind::Read),
    ("plan_delete", Kind::Read),
    // Irreversible with `freeNumber` ("Free the number"), see [`classify`].
    ("delete_entity", Kind::Depends),
    // One undo step, like any edit.
    ("restore_entity", Kind::Write),
    // One undo step (a file step: register, story, side file), like story_create.
    ("story_delete", Kind::Write),
    // World maps (6.3a, over `g4_call` only): the readers read; „Welt anlegen“, a change of
    // cells and the tileset file are one undo step each.
    ("world_create", Kind::Write),
    ("world_edit", Kind::Write),
    ("world_get", Kind::Read),
    ("tileset_get", Kind::Read),
    ("tileset_save", Kind::Write),
    ("get_world_mesh", Kind::Read),
    ("get_tileset_texture", Kind::Read),
    // Terrain (6.2): set_map_terrain writes BDHC and permissions as one undo step.
    ("get_map_heights", Kind::Read),
    ("set_map_terrain", Kind::Write),
    ("get_map_budget", Kind::Read),
    // One undo step (appending to the map texture pack and the model import, squashed); the
    // files under `outDir` are build output, confined to `build/`. Only through `g4_call`.
    ("bake_test", Kind::Write),
    // World maps (6.3b, over `g4_call` only): „Karte als Quelle übernehmen“, „Als binäre Karte
    // übernehmen“ and a change of tileset tiles are one undo step each; the bake report reads.
    ("world_adopt", Kind::Write),
    ("world_to_binary", Kind::Write),
    ("world_bake", Kind::Read),
    ("tileset_edit", Kind::Write),
    // Free-form models, piece models and the exchange with Tiled (6.5b, over `g4_call` only):
    // inspect reads, add, remove, piece set and import are one undo step each; the TMJ export
    // writes files, into `build/` a write, elsewhere irreversible (as `export_map_part`).
    ("world_freeform_inspect", Kind::Read),
    ("world_freeform_add", Kind::Write),
    ("world_freeform_remove", Kind::Write),
    ("tileset_piece_set", Kind::Write),
    ("world_export_tmj", Kind::Depends),
    ("world_inspect_tmj", Kind::Read),
    ("world_import_tmj", Kind::Write),
    // Painting (6.4a, over `g4_call` only): adding or removing a layer is one undo step.
    ("world_layers", Kind::Write),
    // The language is the user's (Settings) or the environment's (`G4_LANG`), not an agent's.
    ("set_locale", APP_ONLY),
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
    /// write; [`classify`] resolves it), its reason in the language of the server.
    fn from(kind: Kind) -> Class {
        match kind {
            Kind::Read => Class::Read,
            Kind::Write | Kind::Depends => Class::Write,
            Kind::Irreversible(why) => Class::Irreversible(t!(why).into_owned()),
            Kind::Unavailable(why) => Class::Unavailable(t!(why).into_owned()),
        }
    }
}

/// What the call of `command` with `args` (keyed as `dispatch` takes them) does. A build, a
/// map part export or a TMJ export (`world_export_tmj`) lands in `build/` (a write; `build_rom` without `out` goes there too) or
/// somewhere else the policy allows (irreversible); `apply_fix` with `create_script` creates
/// a script, with `remove_hidden` moves a file to `.g4/backup/` (both irreversible);
/// `create_area` and `create_room` with `header.allow_patch` may apply the Dynamic Headers
/// patch; `recipe_apply` is irreversible when its recipe says so (`recipes::is_irreversible`:
/// the empty region, a new trainer, an area or room with `header.allow_patch`, and always
/// „Figur erscheint/verschwindet ab Schritt“ and „Szene beim Betreten“: they may create a level
/// script file, which only the project tells, and this check has no store, so it answers
/// conservatively; [`classify_in`] answers exactly with the store), arguments that are no
/// recipe call it can read are a write (the command refuses them). The project
/// preview commands with a `path` (another project folder) are unavailable, also when it names
/// the bound project. `publish_patch` with `overwrite` replaces files (irreversible), with a
/// `dir` outside `build/` writes there (irreversible), else it is a write. `delete_entity` with
/// `freeNumber` (or `free_number`, see [`frees_number`]) frees a number for good
/// (irreversible), without it is one undo step. A name the table does not have is unavailable.
pub fn classify(command: &str, args: &Value, policy: &UserPolicy) -> Class {
    let Some(kind) = kind_of(command) else {
        return Class::Unavailable(
            t!("mcp.access.unknown_command", command = command).into_owned(),
        );
    };
    if matches!(command, "project_preview_get" | "project_preview_set")
        && args.get("path").is_some_and(|path| !path.is_null())
    {
        return Class::Unavailable(t!(BOUND_PROJECT).into_owned());
    }
    if kind != Kind::Depends {
        return kind.into();
    }
    match command {
        "build_rom" | "export_map_part" | "world_export_tmj" => {
            let key = if command == "build_rom" {
                "out"
            } else {
                "path"
            };
            match args.get(key) {
                // A target that is not text is refused by the command itself.
                Some(Value::String(target)) if !policy.is_in_build(target) => {
                    Kind::Irreversible(OUTSIDE_BUILD).into()
                }
                _ => Class::Write,
            }
        }
        "publish_patch" if args.get("overwrite") == Some(&Value::Bool(true)) => {
            Kind::Irreversible(REPLACE_PATCH).into()
        }
        "publish_patch" => match args.get("dir") {
            // A place that is not text is refused by the command itself.
            Some(Value::String(dir)) if !policy.is_in_build(dir) => {
                Kind::Irreversible(OUTSIDE_BUILD).into()
            }
            _ => Class::Write,
        },
        "apply_fix" if args.get("fix").and_then(Value::as_str) == Some("create_script") => {
            NEW_SCRIPT.into()
        }
        "apply_fix" if args.get("fix").and_then(Value::as_str) == Some("remove_hidden") => {
            Kind::Irreversible(REMOVE_HIDDEN).into()
        }
        "create_area" | "create_room"
            if args.pointer("/spec/header/allow_patch") == Some(&Value::Bool(true)) =>
        {
            Kind::Irreversible(HEADER_PATCH_APPLY).into()
        }
        // Arguments that are no call the recipe can read fail in the command, writing nothing.
        "recipe_apply" => serde_json::from_value::<RecipeCall>(args.clone())
            .ok()
            .and_then(|call| recipes::is_irreversible(&call).ok().flatten())
            .map_or(Class::Write, |why| Class::Irreversible(why.to_owned())),
        "delete_entity" if frees_number(args) => {
            Kind::Irreversible("mcp.access.free_number").into()
        }
        _ => Class::Write,
    }
}

/// Whether the arguments of `delete_entity` free the number: `freeNumber` as `dispatch` takes
/// it, or `free_number` (the curated tool's key, which `g4_call` passes as it is and the
/// command honours too).
pub fn frees_number(args: &Value) -> bool {
    ["freeNumber", "free_number"]
        .iter()
        .any(|key| args.get(key) == Some(&Value::Bool(true)))
}

/// Whether [`classify`]'s answer `class` for `command` depends on the project, so that
/// [`classify_in`] answers it exactly: `recipe_apply` of a visibility or a scene, which
/// [`classify`] holds irreversible ([`recipes::LEVEL_SCRIPT_FILE`]) without knowing whether
/// the place needs a level script file of its own.
pub fn needs_store(command: &str, class: &Class) -> bool {
    command == "recipe_apply"
        && matches!(class, Class::Irreversible(why) if why == recipes::LEVEL_SCRIPT_FILE)
}

/// [`classify`] with the project's store at hand: where the answer depends on the project
/// ([`needs_store`]), `recipe_apply` asks as `recipes::is_irreversible_in` says: a visibility
/// or a scene is irreversible only when its plan creates a level script file (the place shares
/// its file with another header or has none), else a write. A plan the recipe refuses keeps
/// [`classify`]'s answer (fail closed): what it refuses now (a story not made yet, a step not
/// there yet) may be there when the call runs, and the recipe would then create the file
/// without the question. The plan reads the store, it writes nothing. Every other call as
/// [`classify`].
pub fn classify_in(command: &str, args: &Value, policy: &UserPolicy, store: &mut Store) -> Class {
    let class = classify(command, args, policy);
    if !needs_store(command, &class) {
        return class;
    }
    let Ok(call) = serde_json::from_value::<RecipeCall>(args.clone()) else {
        return class;
    };
    match recipes::is_irreversible_in(store, &call) {
        Ok(None) => Class::Write,
        Ok(Some(why)) => Class::Irreversible(why.to_owned()),
        Err(_) => class,
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
        references: Vec::new(),
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
            t!("mcp.access.read_only", command = command).into_owned(),
        )),
        Class::Irreversible(why) if !access.irreversible => Err(refusal(
            "needs_confirmation",
            t!(
                "mcp.access.needs_confirmation",
                command = command,
                why = why
            )
            .into_owned(),
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
                    Kind::Irreversible("mcp.access.accept_baseline")
                ),
                ("add_script", Kind::Irreversible("mcp.access.new_script")),
                (
                    "adopt_text_project",
                    Kind::Irreversible("mcp.access.adopt_text_project")
                ),
                (
                    "apply_empty_region",
                    Kind::Irreversible("mcp.access.clear_world")
                ),
                (
                    "apply_header_patch",
                    Kind::Irreversible("mcp.access.header_patch_apply")
                ),
                ("assign_base", Kind::Irreversible("mcp.access.assign_base")),
                (
                    "create_patch",
                    Kind::Irreversible("mcp.access.create_patch")
                ),
                (
                    "create_trainer",
                    Kind::Irreversible("mcp.access.create_trainer")
                ),
                (
                    "git_commit_all",
                    Kind::Irreversible("mcp.access.commit_all")
                ),
                ("git_init", Kind::Irreversible("mcp.access.init_git")),
                ("live_play", Kind::Irreversible("mcp.access.run_rom")),
                (
                    "new_script_file",
                    Kind::Irreversible("mcp.access.new_script")
                ),
                (
                    "project_preview_set",
                    Kind::Irreversible("mcp.access.change_preview")
                ),
                (
                    "repair_header_patch",
                    Kind::Irreversible("mcp.access.header_patch_repair")
                ),
                (
                    "resolve_external_conflict",
                    Kind::Irreversible("mcp.access.overwrite_version")
                ),
                (
                    "resolve_text_conflicts",
                    Kind::Irreversible("mcp.access.overwrite_text_archive")
                ),
                ("run_rom", Kind::Irreversible("mcp.access.run_rom")),
                ("text_export", Kind::Irreversible("mcp.access.export_text")),
            ]
        );
        // The agent hub's commands: `agent_commands_are_app_only`; play mode live's:
        // `live_commands_have_their_access`.
        let mut unavailable = of(|k| matches!(k, Kind::Unavailable(_)));
        unavailable.retain(|(name, _)| !name.starts_with("agent_") && !name.starts_with("live_"));
        unavailable.sort_by_key(|(name, _)| *name);
        assert_eq!(
            unavailable,
            [
                (
                    "check_project_dest",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                ("close_project", Kind::Unavailable("mcp.access.app_only")),
                ("create_project", Kind::Unavailable("mcp.access.app_only")),
                (
                    "default_project_parent",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                ("external_sync", Kind::Unavailable("mcp.access.app_only")),
                ("force_quit", Kind::Unavailable("mcp.access.app_only")),
                ("lsp_send", Kind::Unavailable("mcp.access.app_only")),
                ("lsp_start", Kind::Unavailable("mcp.access.app_only")),
                ("lsp_stop", Kind::Unavailable("mcp.access.app_only")),
                (
                    "migrate_text_project",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                (
                    "open_project",
                    Kind::Unavailable("mcp.access.bound_project_short")
                ),
                (
                    "register_base_rom",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                (
                    "restore_text_project",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                ("reveal_path", Kind::Unavailable("mcp.access.app_only")),
                (
                    "revert_text_project",
                    Kind::Unavailable("mcp.access.app_only")
                ),
                ("set_locale", Kind::Unavailable("mcp.access.app_only")),
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
                "delete_entity",
                "export_map_part",
                "publish_patch",
                "recipe_apply",
                "world_export_tmj"
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
                Class::Unavailable(
                    "The project is fixed (--project); call without path".to_owned()
                ),
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
            Class::Irreversible("Change preview picture".to_owned())
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
                Some(Kind::Unavailable("mcp.access.app_only")),
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
        let outside_build = Class::Irreversible("Build outside build/".to_owned());
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
            Class::Irreversible("Create trainer".to_owned())
        );
        assert_eq!(
            classify("open_project", &json!({}), &policy),
            Class::Unavailable("The project is fixed (--project)".to_owned())
        );
        assert_eq!(
            classify("no_such_command", &json!({}), &policy),
            Class::Unavailable("Unknown command: no_such_command".to_owned())
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
        let patch = Class::Irreversible("Apply header patch".to_owned());
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

    /// `recipe_apply` asks as its recipe says: „Header-Patch einspielen“ for an area or room
    /// with `header.allow_patch`, „Welt leeren“ always for the empty region, „Trainer anlegen“
    /// always for a new trainer, else a write;
    /// input the recipe cannot read is a write (the command fails on it, writing nothing).
    #[test]
    fn recipe_apply_is_irreversible_only_with_allow_patch_or_empty_region() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let header = |allow_patch: bool| {
            json!({ "id": 2, "area_data_id": null, "event_file": "New",
                    "new_header": { "template": 1, "internal_name": null },
                    "allow_patch": allow_patch, "place": null })
        };
        let area = |allow_patch: bool| {
            json!({ "recipe": "create_area", "input": {
                "name": "area", "width": 1, "height": 1,
                "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": true }],
                "with_headers": false, "header": header(allow_patch), "target": null,
            } })
        };
        let room = |allow_patch: bool| {
            json!({ "recipe": "create_room", "input": {
                "name": "room", "source_map": 0, "clear_buildings": true,
                "header": header(allow_patch), "entrance": null,
            } })
        };
        let classify = |args: Value| classify("recipe_apply", &args, &policy);
        // The reason is the recipe's own (the kernel names it), passed on as it is.
        let reason = |args: &Value| {
            let call: RecipeCall = serde_json::from_value(args.clone()).unwrap();
            Class::Irreversible(recipes::is_irreversible(&call).unwrap().unwrap().to_owned())
        };
        let patch = reason(&area(true));
        assert_eq!(classify(area(true)), patch);
        assert_eq!(classify(room(true)), patch);
        for empty in [
            json!({ "recipe": "empty_region" }),
            json!({ "recipe": "empty_region", "input": null }),
            json!({ "recipe": "empty_region", "input": {} }),
        ] {
            assert_eq!(classify(empty.clone()), reason(&empty), "{empty}");
        }
        let extend = json!({ "recipe": "extend_area", "input": {
            "matrix": 0, "x": 2, "y": 0, "source_map": 0,
            "clear_buildings": false, "header_cell": null,
        } });
        for args in [area(false), room(false), extend] {
            assert_eq!(classify(args.clone()), Class::Write, "{args}");
        }
        // A new trainer is always created behind an undo barrier.
        let trainer = json!({ "recipe": "new_trainer_here", "input": {
            "header": 0, "x": 40, "z": 7, "dir": 0, "class": 9, "name": "Jens",
            "party": [{ "species": 25, "level": 12 }],
            "texts": { "before": "Halt!\\r", "lose": "Du bist stark!", "after": "Gut." },
            "graphics_id": null,
        } });
        assert_eq!(classify(trainer.clone()), reason(&trainer));
        // Arguments that are no recipe call, or input the recipe cannot read.
        for args in [
            json!({ "recipe": "create_area", "input": { "header": header(true) } }),
            json!({ "recipe": "new_trainer_here", "input": { "header": 0 } }),
            json!({ "recipe": "no_such_recipe" }),
            json!({}),
        ] {
            assert_eq!(classify(args.clone()), Class::Write, "{args}");
        }
        assert_eq!(kind_of("recipe_list"), Some(Kind::Read));
        assert_eq!(kind_of("recipe_plan"), Some(Kind::Read));
        assert_eq!(kind_of("recipe_apply"), Some(Kind::Depends));
    }

    /// The story recipes of 3d.7 decide per call like every recipe: a story, a talk and a
    /// reward never ask; a visibility and a scene ask without the store (they may create a
    /// level script file) and, with the store at hand ([`classify_in`]), only when their plan
    /// creates one: a place whose level script file is its own needs no confirmation, a place
    /// without one does, and a plan the recipe refuses keeps the answer without the store (fail
    /// closed). Input the recipe cannot read stays a write.
    #[test]
    fn story_recipes_are_depends() {
        use g4_kernel::store::MapHeader;
        assert_eq!(kind_of("recipe_apply"), Some(Kind::Depends));
        let fx = g4_kernel::test_support::TextFixture::exported();
        let state = crate::state::AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        crate::commands::open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let var = crate::commands::free_flag_impl(&state, g4_kernel::flags::FlagKind::Var)
            .unwrap()
            .number;
        let flag = crate::commands::free_flag_impl(&state, g4_kernel::flags::FlagKind::Flag)
            .unwrap()
            .number;
        let story = "STORY_TESTFADEN";
        let figure = json!({ "header": 0, "local_id": 0 });
        let call = |recipe: &str, input: Value| json!({ "recipe": recipe, "input": input });
        let new_story = call("new_story", json!({ "name": "Testfaden", "var": var }));
        let talk = call(
            "talk_advances",
            json!({ "story": story, "figure": figure, "from_step": 0, "text": "Hallo" }),
        );
        let reward = call(
            "reward_once",
            json!({ "story": story, "figure": figure, "from_step": 0, "item": 17,
                    "text": "Hier!", "flag": flag }),
        );
        let appears = call(
            "figure_appears",
            json!({ "story": story, "figure": figure, "step": 0, "appears": true,
                    "flag": flag }),
        );
        let scene = |header: u16| {
            call(
                "scene_on_enter",
                json!({ "story": story, "header": header, "step": 0, "text": "Willkommen" }),
            )
        };
        let level = Class::Irreversible(recipes::LEVEL_SCRIPT_FILE.to_owned());

        // Without the store.
        let bare = |args: &Value| classify("recipe_apply", args, &policy);
        for args in [&new_story, &talk, &reward] {
            assert_eq!(bare(args), Class::Write, "{args}");
            assert!(!needs_store("recipe_apply", &bare(args)), "{args}");
        }
        for args in [&appears, &scene(0)] {
            assert_eq!(bare(args), level, "{args}");
            assert!(needs_store("recipe_apply", &bare(args)), "{args}");
        }
        assert!(!needs_store("recipe_plan", &level));
        for args in [
            call("figure_appears", json!({ "story": story })),
            call("scene_on_enter", json!({ "header": 0 })),
            call("new_story", json!({ "name": "Ohne Variable" })),
        ] {
            assert_eq!(bare(&args), Class::Write, "{args}");
        }

        // With the store, before the story exists: the plan refuses, and a refused plan keeps
        // the answer without the store (fail closed: the story may exist by the time the call
        // runs, and the scene then creates the file).
        {
            let mut guard = crate::commands::lock(&state).unwrap();
            let store = &mut crate::commands::session(&mut guard).unwrap().store;
            for header in [0, 1] {
                assert_eq!(
                    classify_in("recipe_apply", &scene(header), &policy, store),
                    level,
                    "{header}"
                );
            }
        }

        // With the store: the story first, then a scene at header 0 (level script file 1 is
        // its own) and at header 1 (no level script file: the recipe creates one; it gets the
        // text archive of header 0 for the scene's message).
        crate::commands::recipe_apply_impl(&state, serde_json::from_value(new_story).unwrap())
            .unwrap();
        let mut guard = crate::commands::lock(&state).unwrap();
        let store = &mut crate::commands::session(&mut guard).unwrap().store;
        let archive = store.header_table().unwrap()[0].text_archive_id();
        store
            .transaction("Textarchiv", |tx| {
                tx.edit_header(1, |h| {
                    match h {
                        MapHeader::DP(h) => h.text_archive_id = archive,
                        MapHeader::Pt(h) => h.text_archive_id = archive,
                        MapHeader::HGSS(h) => h.text_archive_id = archive,
                    }
                    Ok(())
                })
            })
            .unwrap();
        for (header, irreversible) in [(0, false), (1, true)] {
            let draft = recipes::plan(store, &serde_json::from_value(scene(header)).unwrap());
            assert_eq!(draft.unwrap().irreversible.is_empty(), !irreversible);
        }
        let mut exactly = |args: &Value| classify_in("recipe_apply", args, &policy, store);
        assert_eq!(exactly(&scene(0)), Class::Write);
        assert_eq!(exactly(&scene(1)), level);
        // A plan the recipe refuses (an unknown story) keeps the answer without the store.
        let unknown = call(
            "scene_on_enter",
            json!({ "story": "STORY_NOPE", "header": 0, "step": 0, "text": "x" }),
        );
        assert_eq!(exactly(&unknown), level);
        // Every other command and recipe as without the store.
        assert_eq!(exactly(&talk), Class::Write);
        assert_eq!(
            classify_in("save", &json!({}), &policy, store),
            Class::Write
        );
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
            Class::Irreversible("Create script".to_owned())
        );
        // Removing a hidden file moves it past the store.
        assert_eq!(
            classify(
                "apply_fix",
                &json!({ "finding": finding, "fix": "remove_hidden" }),
                &policy
            ),
            Class::Irreversible("Move hidden file to .g4/backup".to_owned())
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
            Class::Irreversible("Overwrite one version of a text archive".to_owned())
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
            ("read_only", "Read only: save changes the project")
        );
        // An irreversible command writes as well; read-only says that first.
        let e = refused(
            "create_trainer",
            &Class::Irreversible("Create trainer".to_owned()),
            Access {
                irreversible: true,
                ..read_only
            },
        );
        assert_eq!(
            (e.code, e.message.as_str()),
            ("read_only", "Read only: create_trainer changes the project")
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
            &Class::Unavailable("Only in the app".to_owned()),
            all,
        );
        assert_eq!(
            (e.code, e.message.as_str()),
            ("unavailable", "Only in the app")
        );
    }

    #[test]
    fn admit_refuses_irreversible_without_confirmation() {
        let class = Class::Irreversible("Create trainer".to_owned());
        let writer = Access {
            write: true,
            irreversible: false,
            expert: false,
        };
        let e = refused("create_trainer", &class, writer);
        assert_eq!(e.code, "needs_confirmation");
        assert_eq!(
            e.message,
            "create_trainer cannot be undone (Create trainer); \
             headless only with --allow-irreversible"
        );
        assert!(e.asset.is_none() && e.problems.is_empty());
        let confirmed = Access {
            irreversible: true,
            ..writer
        };
        assert!(admit("create_trainer", &class, &confirmed).is_ok());
    }

    #[test]
    fn text_project_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in [
            "text_project_status",
            "external_changes",
            "conflict_diff",
            "plan_rename_id",
            "entity_ids",
        ] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step.
        assert_eq!(kind("rename_id"), Kind::Write);
        assert_eq!(
            kind("adopt_text_project"),
            Kind::Irreversible("mcp.access.adopt_text_project")
        );
        assert_eq!(
            kind("resolve_external_conflict"),
            Kind::Irreversible("mcp.access.overwrite_version")
        );
        // The automatic sync is the app's; taking back and lifting are the user's steps.
        for name in [
            "external_sync",
            "revert_text_project",
            "migrate_text_project",
        ] {
            assert_eq!(kind(name), APP_ONLY, "{name}");
        }
    }

    #[test]
    fn original_commands_read_except_the_reset() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in ["original_status", "plan_reset", "changed_from_original"] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step, like any edit.
        assert_eq!(kind("reset_to_original"), Kind::Write);
    }

    #[test]
    fn git_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in ["git_status", "diff_entity", "unsaved_changes"] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        assert_eq!(kind("git_init"), Kind::Irreversible("mcp.access.init_git"));
        // A clone is completed before it opens.
        assert_eq!(kind("restore_text_project"), APP_ONLY);
    }

    #[test]
    fn publish_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        assert_eq!(kind("releases"), Kind::Read);
        assert_eq!(kind("publish_patch"), Kind::Depends);
        assert_eq!(kind("reveal_path"), APP_ONLY);
        assert_eq!(
            kind("git_commit_all"),
            Kind::Irreversible("mcp.access.commit_all")
        );
        assert_eq!(kind("git_commit_prepare"), Kind::Write);
    }

    #[test]
    fn publish_patch_classifies_by_overwrite_and_dir() {
        let (fixture, _state) = open_synthetic();
        let elsewhere = tempfile::tempdir().unwrap();
        let allowed = [elsewhere.path().to_path_buf()];
        let policy = UserPolicy::new(fixture.dir.path(), &allowed, None).unwrap();
        let outside = elsewhere.path().to_str().unwrap();
        let classify = |args: Value| classify("publish_patch", &args, &policy);
        assert_eq!(classify(json!({})), Class::Write);
        assert_eq!(classify(json!({ "version": "0.1.0" })), Class::Write);
        assert_eq!(
            classify(json!({ "overwrite": true })),
            Class::Irreversible("Replace patch".to_owned())
        );
        assert_eq!(
            classify(json!({ "dir": "/tmp/x" })),
            Class::Irreversible("Build outside build/".to_owned())
        );
        assert_eq!(
            classify(json!({ "dir": outside })),
            Class::Irreversible("Build outside build/".to_owned())
        );
        // Replacing outranks the place.
        assert_eq!(
            classify(json!({ "dir": outside, "overwrite": true })),
            Class::Irreversible("Replace patch".to_owned())
        );
        for inside in ["build", "build/releases", "build/"] {
            assert_eq!(classify(json!({ "dir": inside })), Class::Write, "{inside}");
        }
        assert_eq!(
            classify(json!({ "dir": null, "overwrite": false })),
            Class::Write
        );
    }

    #[test]
    fn delete_entity_with_free_number_is_irreversible() {
        let (fixture, _state) = open_synthetic();
        let policy = policy_over(&fixture);
        let target = json!({ "kind": "trainer", "index": 2 });
        assert_eq!(
            classify(
                "delete_entity",
                &json!({ "target": target, "freeNumber": true }),
                &policy
            ),
            Class::Irreversible("Free the number".to_owned())
        );
        // g4_call passes the curated tool's key as it is; the command honours it too.
        assert_eq!(
            classify(
                "delete_entity",
                &json!({ "target": target, "free_number": true }),
                &policy
            ),
            Class::Irreversible("Free the number".to_owned())
        );
        // A tombstone is one undo step.
        for args in [
            json!({ "target": target, "freeNumber": false }),
            json!({ "target": target, "free_number": false }),
            json!({ "target": target }),
        ] {
            assert_eq!(
                classify("delete_entity", &args, &policy),
                Class::Write,
                "{args}"
            );
        }
        // Planning frees nothing, whatever the arguments; restoring is one undo step.
        assert_eq!(
            classify(
                "plan_delete",
                &json!({ "target": target, "freeNumber": true }),
                &policy
            ),
            Class::Read
        );
        assert_eq!(kind_of("restore_entity"), Some(Kind::Write));
        // Without the confirmation an agent hears needs_confirmation.
        let writer = Access {
            write: true,
            irreversible: false,
            expert: false,
        };
        let class = classify(
            "delete_entity",
            &json!({ "target": target, "freeNumber": true }),
            &policy,
        );
        let e = admit("delete_entity", &class, &writer).unwrap_err();
        assert_eq!(e.code, "needs_confirmation");
        assert_eq!(
            e.message,
            "delete_entity cannot be undone (Free the number); \
             headless only with --allow-irreversible"
        );
    }

    #[test]
    fn flag_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in ["list_flags", "flag_usage", "free_flag"] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step, like rename_id.
        assert_eq!(kind("name_flag"), Kind::Write);
    }

    #[test]
    fn dialog_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        assert_eq!(kind("dialog_of"), Kind::Read);
        assert_eq!(kind("dialogs_of_header"), Kind::Read);
        // One undo step (the archive's copy and the script file's step, squashed).
        assert_eq!(kind("split_message"), Kind::Write);
    }

    #[test]
    fn story_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in ["list_threads", "thread_graph", "story_list", "story_get"] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step each.
        for name in ["story_create", "story_update", "story_delete"] {
            assert_eq!(kind(name), Kind::Write, "{name}");
        }
    }

    #[test]
    fn live_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        assert_eq!(kind("live_status"), Kind::Read);
        // A live value is no undo step, but it changes the running game.
        assert_eq!(kind("live_set"), Kind::Write);
        assert_eq!(kind("live_play"), Kind::Irreversible("mcp.access.run_rom"));
        for name in [
            "live_connect",
            "live_stop",
            "live_resume",
            "live_breakpoints",
            "live_breakpoint_set",
            "live_breakpoint_clear",
        ] {
            assert_eq!(kind(name), APP_ONLY, "{name}");
        }
    }

    #[test]
    fn world_map_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in [
            "world_get",
            "tileset_get",
            "get_world_mesh",
            "get_tileset_texture",
        ] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step each.
        for name in ["world_create", "world_edit", "tileset_save", "world_layers"] {
            assert_eq!(kind(name), Kind::Write, "{name}");
        }
    }

    #[test]
    fn world_bake_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        assert_eq!(kind("world_bake"), Kind::Read);
        // One undo step each.
        for name in ["world_adopt", "world_to_binary", "tileset_edit"] {
            assert_eq!(kind(name), Kind::Write, "{name}");
        }
    }

    #[test]
    fn terrain_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        assert_eq!(kind("get_map_heights"), Kind::Read);
        assert_eq!(kind("get_map_budget"), Kind::Read);
        // One undo step for BDHC and permissions.
        assert_eq!(kind("set_map_terrain"), Kind::Write);
    }

    #[test]
    fn bake_test_is_a_write() {
        // One undo step (appending to the pack and the model import, squashed); the files
        // under `outDir` are build output under `build/`.
        assert_eq!(kind_of("bake_test"), Some(Kind::Write));
    }

    #[test]
    fn freeform_and_tmj_commands_have_their_access() {
        let kind = |name: &str| kind_of(name).unwrap_or_else(|| panic!("{name} has no kind"));
        for name in ["world_freeform_inspect", "world_inspect_tmj"] {
            assert_eq!(kind(name), Kind::Read, "{name}");
        }
        // One undo step each.
        for name in [
            "world_freeform_add",
            "world_freeform_remove",
            "tileset_piece_set",
            "world_import_tmj",
        ] {
            assert_eq!(kind(name), Kind::Write, "{name}");
        }
        // Like `export_map_part`: a write into `build/`, irreversible elsewhere.
        assert_eq!(kind("world_export_tmj"), Kind::Depends);
    }
}
