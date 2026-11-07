//! The product's command table: every Tauri command by name, over its `*_impl` function, for
//! callers that are not the app's own webview (the dev bridge, the MCP server).
//!
//! Arguments arrive as one JSON object keyed like Tauri's `invoke` does (the camelCase of the
//! Rust parameter names), so `api.ts` and these callers speak the same dialect. Every path a
//! command takes goes through the caller's [`Policy`] first.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::commands::policy::Policy;
use crate::commands::*;
use crate::errors::CmdError;
use crate::state::AppState;
use g4_kernel::checks::{FixId, Subject};
use g4_kernel::data::battle_sprites::Side;
use g4_kernel::map_import::{Part, Source};
use g4_kernel::model_import::ImportOptions;
use g4_kernel::original::ResetTarget;
use g4_kernel::preview::PreviewSlot;
use g4_kernel::project::AssetKind;
use g4_kernel::scripts::LevelScriptDoc;
use g4_kernel::store::MapHeader;

/// What a command answers: JSON, or raw bytes for the GLB and icon commands.
#[derive(Debug)]
pub enum Reply {
    Json(Value),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub enum DispatchError {
    /// No command of that name.
    Unknown(String),
    /// The command ran (or its arguments or paths were refused) and failed.
    Cmd(CmdError),
}

impl From<CmdError> for DispatchError {
    fn from(e: CmdError) -> Self {
        DispatchError::Cmd(e)
    }
}

/// The names registered with `generate_handler!` in `lib.rs`, in the same order (a test keeps
/// the two lists equal).
pub const COMMANDS: &[&str] = &[
    "open_project",
    "list_headers",
    "get_header",
    "set_header",
    "get_area_data",
    "set_area_data",
    "member_counts",
    "get_matrix",
    "set_matrix_cell",
    "set_matrix_sections",
    "resize_matrix",
    "rename_matrix",
    "create_matrix",
    "list_matrices",
    "list_warps",
    "duplicate_map",
    "create_event_file",
    "duplicate_event_file",
    "create_area",
    "extend_area",
    "create_room",
    "split_room",
    "header_patch_status",
    "apply_header_patch",
    "repair_header_patch",
    "get_map",
    "get_events",
    "get_building_glb",
    "add_building",
    "update_building",
    "delete_building",
    "update_event",
    "get_text_archive",
    "edit_text_archive",
    "create_text_archive",
    "search_text",
    "validate_text",
    "scripts_status",
    "adopt_scripts",
    "list_script_files",
    "read_script",
    "save_script",
    "save_level_script",
    "new_script_file",
    "add_script",
    "resolve_script",
    "search_scripts",
    "lsp_start",
    "lsp_send",
    "lsp_stop",
    "add_event",
    "delete_event",
    "get_map_model_glb",
    "get_object_sprite",
    "map_model_texture_status",
    "tile_behaviors",
    "set_permission",
    "undo",
    "redo",
    "history",
    "dirty",
    "save",
    "build_rom",
    "run_rom",
    "reload_asset",
    "accept_disk",
    "resolve_text_conflicts",
    "force_quit",
    "data_list",
    "data_get",
    "data_edit",
    "data_raw",
    "name_tables",
    "item_table",
    "machine_table",
    "refs",
    "get_item_icon",
    "get_mon_icon",
    "create_trainer",
    "set_trainer_texts",
    "create_encounter_file",
    "split_encounter_file",
    "inspect_map_import",
    "import_map_parts",
    "export_map_part",
    "run_checks",
    "checks_status",
    "ignore_finding",
    "unignore_finding",
    "accept_baseline",
    "set_check_enabled",
    "plan_fix",
    "apply_fix",
    "inspect_rom",
    "default_project_parent",
    "check_project_dest",
    "create_project",
    "plan_empty_region",
    "apply_empty_region",
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
    "world_meta_get",
    "world_meta_set",
    "place_slots",
    "create_place",
    "rename_place",
    "assign_place",
    "project_preview_get",
    "project_preview_set",
    "base_status",
    "check_base",
    "assign_base",
    "register_base_rom",
    "create_patch",
    "original_status",
    "plan_reset",
    "reset_to_original",
    "changed_from_original",
    "text_check",
    "text_export",
    "text_project_status",
    "adopt_text_project",
    "revert_text_project",
    "migrate_text_project",
    "external_sync",
    "external_changes",
    "conflict_diff",
    "resolve_external_conflict",
    "plan_rename_id",
    "rename_id",
    "entity_ids",
    "close_project",
    "git_status",
    "git_init",
    "diff_entity",
    "unsaved_changes",
    "restore_text_project",
    "get_battle_sprite",
    "get_trainer_sprite",
    "releases",
    "publish_patch",
    "reveal_path",
    "git_commit_all",
    "git_commit_prepare",
    "recipe_list",
    "recipe_plan",
    "recipe_apply",
    "list_flags",
    "flag_usage",
    "free_flag",
    "name_flag",
    "dialog_of",
    "split_message",
    "list_threads",
    "thread_graph",
    "story_list",
    "story_get",
    "story_create",
    "story_update",
    "live_status",
    "live_play",
    "live_connect",
    "live_stop",
    "live_resume",
    "live_set",
    "live_breakpoints",
    "live_breakpoint_set",
    "live_breakpoint_clear",
    "dialogs_of_header",
    "plan_delete",
    "delete_entity",
    "restore_entity",
    "story_delete",
    "world_create",
    "world_edit",
    "world_get",
    "tileset_get",
    "tileset_save",
    "get_world_mesh",
    "get_tileset_texture",
    "get_map_heights",
    "set_map_terrain",
    "get_map_budget",
    "bake_test",
    "world_layers",
];

/// Import files must be readable under the policy; a map of the project needs nothing.
fn confine_source(policy: &dyn Policy, source: Source) -> Result<Source, CmdError> {
    Ok(match source {
        Source::Files(paths) => Source::Files(
            paths
                .iter()
                .map(|p| policy.read_path(&p.to_string_lossy()))
                .collect::<Result<_, _>>()?,
        ),
        map => map,
    })
}

/// The project folder `path` of `project_preview_*` names, if it is there: one `open_project`
/// may open. `None` when nothing is there (a start list entry whose folder, a folder above it
/// or the target of its link is gone) and the nearest folder above it that is there may be
/// opened, so such an entry answers no preview instead of an error; nothing is read there.
/// A path outside the policy is refused either way, and so is a missing tail with `.` or `..`.
/// (`read_path` takes files only.)
fn preview_root(policy: &dyn Policy, path: &str) -> Result<Option<std::path::PathBuf>, CmdError> {
    use std::path::{Component, Path};
    let full = Path::new(path);
    if full.exists() {
        return policy.check_project(path).map(Some);
    }
    let outside = || CmdError::from(format!("Pfad liegt außerhalb der erlaubten Ordner: {path}"));
    let mut existing = full;
    loop {
        if !matches!(
            existing.components().next_back(),
            Some(Component::Normal(_))
        ) {
            return Err(outside());
        }
        existing = match existing.parent() {
            Some(parent) if parent.as_os_str().is_empty() => Path::new("."),
            Some(parent) => parent,
            None => return Err(outside()),
        };
        if existing.exists() {
            break;
        }
    }
    policy
        .check_project(&path_text(existing)?)
        .map_err(|_| outside())?;
    Ok(None)
}

/// The `.nds` of `check_base` and `assign_base`: the sibling `.nds` DSPRE unpacked the open
/// project from (`suggest_base`, never an original) even outside the folders the policy
/// reads, since without it an agent could never assign it; any other path as the policy reads
/// it. Relative paths are relative to the project folder.
fn base_nds(
    state: &AppState,
    policy: &dyn Policy,
    nds: &str,
) -> Result<std::path::PathBuf, CmdError> {
    match suggested_base_named(state, nds) {
        Some(suggested) => Ok(suggested),
        None => policy.read_path(nds),
    }
}

/// What `assign_base` with `args` would assign, checked as the command checks it (the `.nds`
/// as [`base_nds`] takes it, the base check, the mark) without writing: the `.nds` and its
/// check, or the error the command would answer.
pub fn check_assign_base(
    state: &AppState,
    policy: &dyn Policy,
    args: &Value,
) -> Result<(std::path::PathBuf, g4_kernel::start::BaseCheck), DispatchError> {
    #[derive(serde::Deserialize)]
    struct Args {
        nds: String,
    }
    let a: Args = parse("assign_base", args.clone())?;
    let nds = base_nds(state, policy, &a.nds)?;
    let check = check_assign_impl(state, &path_text(&nds)?)?;
    Ok((nds, check))
}

fn path_text(path: &std::path::Path) -> Result<String, CmdError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| CmdError::from(format!("Pfad ist kein gültiger Text: {}", path.display())))
}

fn parse<T: DeserializeOwned>(name: &str, args: Value) -> Result<T, DispatchError> {
    // A command without parameters may be called with `null` or with `{}`.
    let args = if args.is_null() {
        Value::Object(Default::default())
    } else {
        args
    };
    serde_json::from_value(args)
        .map_err(|e| CmdError::from(format!("Ungültige Argumente für {name}: {e}")).into())
}

fn json<T: Serialize>(result: Result<T, CmdError>) -> Result<Reply, DispatchError> {
    let value = serde_json::to_value(result?).map_err(|e| CmdError::from(e.to_string()))?;
    Ok(Reply::Json(value))
}

fn bytes(result: Result<Vec<u8>, CmdError>) -> Result<Reply, DispatchError> {
    Ok(Reply::Bytes(result?))
}

/// Parses `$args` into a struct of the listed fields (camelCase keys, `Option` stays
/// optional), runs `$call` on it and turns its result into a reply with `$reply`.
macro_rules! run {
    ($reply:ident, $name:literal, $args:expr, { $($(#[$meta:meta])* $field:ident : $ty:ty),* $(,)? }, |$a:ident| $call:expr) => {{
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            $($(#[$meta])* $field: $ty),*
        }
        let $a: Args = parse($name, $args)?;
        $reply($call)
    }};
}

/// Runs the command `name` against `state`. The paths of `open_project`, `build_rom`,
/// `run_rom`, the map part and the project preview commands are checked by `policy` first;
/// `force_quit` is a no-op and the language server is off.
pub fn dispatch(
    state: &AppState,
    policy: &dyn Policy,
    name: &str,
    args: Value,
) -> Result<Reply, DispatchError> {
    match name {
        "open_project" => run!(json, "open_project", args, { path: String }, |a| {
            let project = policy.check_project(&a.path)?;
            let project = project
                .to_str()
                .ok_or_else(|| CmdError::from(format!("Pfad ist kein gültiger Text: {}", a.path)))?;
            open_project_impl(state, project)
        }),
        "list_headers" => run!(json, "list_headers", args, {}, |_a| list_headers_impl(
            state
        )),
        "get_header" => {
            run!(json, "get_header", args, { id: u16 }, |a| get_header_impl(state, a.id))
        }
        "set_header" => run!(
            json, "set_header", args,
            { id: u16, header: MapHeader, label: Option<String> },
            |a| set_header_impl(state, a.id, a.header, a.label.as_deref().unwrap_or("edit header"))
        ),
        "get_area_data" => {
            run!(json, "get_area_data", args, { id: u16 }, |a| get_area_data_impl(state, a.id))
        }
        "set_area_data" => run!(
            json, "set_area_data", args,
            { id: u16, area: AreaDataDto, label: Option<String> },
            |a| set_area_data_impl(state, a.id, a.area, a.label.as_deref().unwrap_or("edit area data"))
        ),
        "member_counts" => run!(json, "member_counts", args, {}, |_a| member_counts_impl(
            state
        )),
        "get_matrix" => {
            run!(json, "get_matrix", args, { index: u16 }, |a| get_matrix_impl(state, a.index))
        }
        "set_matrix_cell" => run!(
            json, "set_matrix_cell", args,
            { index: u16, x: u8, y: u8, map: Option<u16>, header: Option<u16>, altitude: Option<u8> },
            |a| set_matrix_cell_impl(state, a.index, a.x, a.y, a.map, a.header, a.altitude)
        ),
        "set_matrix_sections" => run!(
            json, "set_matrix_sections", args,
            { index: u16, headers: bool, altitudes: bool },
            |a| set_matrix_sections_impl(state, a.index, a.headers, a.altitudes)
        ),
        "resize_matrix" => run!(
            json, "resize_matrix", args,
            { index: u16, width: u8, height: u8 },
            |a| resize_matrix_impl(state, a.index, a.width, a.height)
        ),
        "rename_matrix" => run!(
            json, "rename_matrix", args,
            { index: u16, name: String },
            |a| rename_matrix_impl(state, a.index, &a.name)
        ),
        "create_matrix" => {
            run!(json, "create_matrix", args, { name: String }, |a| create_matrix_impl(state, &a.name))
        }
        "list_matrices" => run!(json, "list_matrices", args, {}, |_a| list_matrices_impl(
            state
        )),
        "list_warps" => run!(json, "list_warps", args, {}, |_a| list_warps_impl(state)),
        "duplicate_map" => run!(
            json, "duplicate_map", args,
            { source: u16, clear_buildings: Option<bool> },
            |a| duplicate_map_impl(state, a.source, a.clear_buildings.unwrap_or(false))
        ),
        "create_event_file" => run!(json, "create_event_file", args, {}, |_a| {
            create_event_file_impl(state)
        }),
        "duplicate_event_file" => run!(
            json, "duplicate_event_file", args,
            { source: u16 },
            |a| duplicate_event_file_impl(state, a.source)
        ),
        "create_area" => {
            run!(json, "create_area", args, { spec: AreaSpec }, |a| create_area_impl(state, a.spec))
        }
        "extend_area" => {
            run!(json, "extend_area", args, { spec: ExtendSpec }, |a| extend_area_impl(state, a.spec))
        }
        "create_room" => {
            run!(json, "create_room", args, { spec: RoomSpec }, |a| create_room_impl(state, a.spec))
        }
        "split_room" => run!(
            json, "split_room", args,
            { header: u16, name: Option<String> },
            |a| split_room_impl(state, a.header, a.name)
        ),
        "header_patch_status" => run!(json, "header_patch_status", args, {}, |_a| {
            header_patch_status_impl(state)
        }),
        "apply_header_patch" => run!(json, "apply_header_patch", args, {}, |_a| {
            apply_header_patch_impl(state)
        }),
        "repair_header_patch" => run!(json, "repair_header_patch", args, {}, |_a| {
            repair_header_patch_impl(state)
        }),
        "get_map" => run!(json, "get_map", args, { index: u16 }, |a| get_map_impl(state, a.index)),
        "get_events" => {
            run!(json, "get_events", args, { event_file_id: u16 }, |a| get_events_impl(state, a.event_file_id))
        }
        "get_building_glb" => run!(
            bytes, "get_building_glb", args,
            { model_id: u32, area_data_id: u8 },
            |a| get_building_glb_impl(state, a.model_id, a.area_data_id)
        ),
        "add_building" => {
            run!(json, "add_building", args, { map: u16 }, |a| add_building_impl(state, a.map))
        }
        "update_building" => run!(
            json, "update_building", args,
            { map: u16, index: u16, patch: BuildingPatch },
            |a| update_building_impl(state, a.map, a.index, a.patch)
        ),
        "delete_building" => run!(
            json, "delete_building", args,
            { map: u16, index: u16 },
            |a| delete_building_impl(state, a.map, a.index)
        ),
        "update_event" => run!(
            json, "update_event", args,
            { event_file: u16, kind: EventKind, index: u16, patch: EventPatch },
            |a| update_event_impl(state, a.event_file, a.kind, a.index, a.patch)
        ),
        "get_text_archive" => run!(json, "get_text_archive", args, { index: u16 }, |a| {
            get_text_archive_impl(state, a.index)
        }),
        "edit_text_archive" => run!(
            json, "edit_text_archive", args,
            { index: u16, edit: TextEdit, label: Option<String> },
            |a| edit_text_archive_impl(state, a.index, a.edit, a.label)
        ),
        "create_text_archive" => run!(json, "create_text_archive", args, {}, |_a| {
            create_text_archive_impl(state)
        }),
        "search_text" => run!(
            json, "search_text", args,
            { query: String, limit: usize },
            |a| search_text_impl(state, a.query, a.limit)
        ),
        "validate_text" => run!(json, "validate_text", args, { text: String }, |a| {
            Ok::<_, CmdError>(validate_text_impl(a.text))
        }),
        "scripts_status" => run!(json, "scripts_status", args, {}, |_a| scripts_status_impl(
            state
        )),
        // Over HTTP there is no channel: the same adoption without progress events (the
        // frontend reports one final progress from the report).
        "adopt_scripts" => run!(json, "adopt_scripts", args, {}, |_a| {
            adopt_scripts_impl(state, &mut |_progress| {})
        }),
        "list_script_files" => run!(json, "list_script_files", args, {}, |_a| {
            list_script_files_impl(state)
        }),
        "read_script" => {
            run!(json, "read_script", args, { file: u16 }, |a| read_script_impl(state, a.file))
        }
        "save_script" => run!(
            json, "save_script", args,
            { file: u16, text: String },
            |a| save_script_impl(state, a.file, a.text)
        ),
        "save_level_script" => run!(
            json, "save_level_script", args,
            { file: u16, doc: LevelScriptDoc },
            |a| save_level_script_impl(state, a.file, a.doc)
        ),
        "new_script_file" => run!(json, "new_script_file", args, { kind: String }, |a| {
            new_script_file_impl(state, a.kind)
        }),
        "add_script" => {
            run!(json, "add_script", args, { file: u16 }, |a| add_script_impl(state, a.file))
        }
        "resolve_script" => run!(
            json, "resolve_script", args,
            { header: u16, n: u16 },
            |a| resolve_script_impl(state, a.header, a.n)
        ),
        "search_scripts" => run!(
            json, "search_scripts", args,
            { query: String, limit: usize },
            |a| search_scripts_impl(state, a.query, a.limit)
        ),
        // rotom-lsp talks over a Tauri channel, which HTTP has not: no language services in the
        // browser, exactly as when the binary is missing.
        "lsp_start" => run!(json, "lsp_start", args, {}, |_a| {
            Err::<(), _>(CmdError {
                code: "tool_missing",
                message: crate::lsp::MISSING.to_owned(),
                asset: None,
                problems: Vec::new(),
                references: Vec::new(),
            })
        }),
        "lsp_send" => run!(json, "lsp_send", args, { body: String }, |a| {
            let _ = a.body;
            Err::<(), _>(CmdError::from("rotom-lsp läuft nicht"))
        }),
        "lsp_stop" => run!(json, "lsp_stop", args, {}, |_a| Ok::<(), CmdError>(())),
        "add_event" => run!(
            json, "add_event", args,
            { event_file: u16, kind: EventKind, x: i32, z: i32, patch: Option<EventPatch>, label: Option<String> },
            |a| add_event_impl(state, a.event_file, a.kind, a.x, a.z, a.patch, a.label.as_deref())
        ),
        "delete_event" => run!(
            json, "delete_event", args,
            { event_file: u16, kind: EventKind, index: u16 },
            |a| delete_event_impl(state, a.event_file, a.kind, a.index)
        ),
        "get_map_model_glb" => run!(
            bytes, "get_map_model_glb", args,
            { index: u16, area_data_id: u8 },
            |a| get_map_model_glb_impl(state, a.index, a.area_data_id)
        ),
        "get_object_sprite" => run!(
            json, "get_object_sprite", args,
            { graphics_id: u16, facing: u8 },
            |a| get_object_sprite_impl(state, a.graphics_id, a.facing)
        ),
        "map_model_texture_status" => run!(
            json, "map_model_texture_status", args,
            { index: u16, area_data_id: u8 },
            |a| map_model_texture_status_impl(state, a.index, a.area_data_id)
        ),
        "tile_behaviors" => run!(json, "tile_behaviors", args, {}, |_a| tile_behaviors_impl(
            state
        )),
        "set_permission" => run!(
            json, "set_permission", args,
            { index: u16, x: u8, y: u8, kind: u8, collision: u8 },
            |a| set_permission_impl(state, a.index, a.x, a.y, a.kind, a.collision)
        ),
        "undo" => run!(json, "undo", args, {}, |_a| undo_impl(state)),
        "redo" => run!(json, "redo", args, {}, |_a| redo_impl(state)),
        "history" => run!(json, "history", args, {}, |_a| history_impl(state)),
        "dirty" => run!(json, "dirty", args, {}, |_a| dirty_impl(state)),
        "save" => run!(json, "save", args, {}, |_a| save_impl(state)),
        "build_rom" => run!(json, "build_rom", args, { out: String, fresh: Option<bool> }, |a| {
            let out = policy.check_output(&a.out)?;
            let out = out
                .to_str()
                .ok_or_else(|| CmdError::from(format!("Pfad ist kein gültiger Text: {}", a.out)))?;
            build_rom_impl(state, out, a.fresh.unwrap_or(false))
        }),
        "run_rom" => run!(
            json, "run_rom", args,
            { emulator: String, rom: String },
            |a| {
                let (emulator, rom) = policy.check_run(&a.emulator, &a.rom)?;
                let text = |path: &std::path::Path| {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        CmdError::from(format!("Pfad ist kein gültiger Text: {}", path.display()))
                    })
                };
                run_rom_impl(&text(&emulator)?, &text(&rom)?)
            }
        ),
        "reload_asset" => run!(
            json, "reload_asset", args,
            { kind: AssetKind, index: u16 },
            |a| reload_asset_impl(state, a.kind, a.index)
        ),
        "accept_disk" => run!(
            json, "accept_disk", args,
            { kind: AssetKind, index: u16 },
            |a| accept_disk_impl(state, a.kind, a.index)
        ),
        "resolve_text_conflicts" => run!(
            json, "resolve_text_conflicts", args,
            { choices: Vec<TextChoice> },
            |a| resolve_text_conflicts_impl(state, a.choices)
        ),
        "force_quit" => Ok(Reply::Json(Value::Null)),
        "data_list" => run!(
            json, "data_list", args,
            { kind: DataKind, filter: Option<DataFilter> },
            |a| data_list_impl(state, a.kind, a.filter)
        ),
        "data_get" => run!(
            json, "data_get", args,
            { kind: DataKind, index: u16 },
            |a| data_get_impl(state, a.kind, a.index)
        ),
        "data_edit" => run!(
            json, "data_edit", args,
            { kind: DataKind, index: u16, patch: DataPatch, label: Option<String> },
            |a| data_edit_impl(state, a.kind, a.index, a.patch, a.label)
        ),
        "data_raw" => run!(
            json, "data_raw", args,
            { kind: DataKind, index: u16 },
            |a| data_raw_impl(state, a.kind, a.index)
        ),
        "name_tables" => run!(json, "name_tables", args, {}, |_a| name_tables_impl(state)),
        "item_table" => run!(json, "item_table", args, {}, |_a| item_table_impl(state)),
        "machine_table" => run!(json, "machine_table", args, {}, |_a| machine_table_impl(
            state
        )),
        "refs" => run!(json, "refs", args, { target: TargetDto }, |a| refs_impl(state, a.target)),
        "get_item_icon" => run!(bytes, "get_item_icon", args, { item: u16 }, |a| {
            get_item_icon_impl(state, a.item)
        }),
        "get_mon_icon" => run!(bytes, "get_mon_icon", args, { species: u16 }, |a| {
            get_mon_icon_impl(state, a.species)
        }),
        "create_trainer" => run!(
            json, "create_trainer", args,
            { class: u8, name: String },
            |a| create_trainer_impl(state, a.class, &a.name)
        ),
        "set_trainer_texts" => run!(
            json, "set_trainer_texts", args,
            { trainer: u16, texts: Vec<g4_kernel::data::trainer_create::TrainerText> },
            |a| set_trainer_texts_impl(state, a.trainer, a.texts)
        ),
        "create_encounter_file" => run!(
            json, "create_encounter_file", args,
            { copy_of: Option<u16> },
            |a| create_encounter_file_impl(state, a.copy_of)
        ),
        "split_encounter_file" => run!(
            json, "split_encounter_file", args,
            { header: u16 },
            |a| split_encounter_file_impl(state, a.header)
        ),
        "inspect_map_import" => run!(
            json, "inspect_map_import", args,
            { map: u16, source: Source, options: Option<ImportOptions> },
            |a| {
                let source = confine_source(policy, a.source)?;
                inspect_map_import_impl(state, a.map, &source, &a.options.unwrap_or_default())
            }
        ),
        "import_map_parts" => run!(
            json, "import_map_parts", args,
            {
                map: u16,
                source: Source,
                parts: Vec<Part>,
                keep_buildings: Option<bool>,
                options: Option<ImportOptions>,
            },
            |a| {
                let source = confine_source(policy, a.source)?;
                import_map_parts_impl(
                    state,
                    a.map,
                    &source,
                    &a.parts,
                    a.keep_buildings.unwrap_or(false),
                    &a.options.unwrap_or_default(),
                )
            }
        ),
        "export_map_part" => run!(
            json, "export_map_part", args,
            { map: u16, part: Part, path: String },
            |a| {
                let path = policy.write_path(&a.path)?;
                export_map_part_impl(state, a.map, a.part, &path)
            }
        ),
        "run_checks" => run!(
            json, "run_checks", args,
            { scope: Option<CheckScope> },
            |a| run_checks_impl(state, a.scope.unwrap_or_default())
        ),
        "checks_status" => run!(json, "checks_status", args, {}, |_a| checks_status_impl(
            state
        )),
        "ignore_finding" => run!(
            json, "ignore_finding", args,
            { code: String, subject: Subject, key: String, fingerprint: String },
            |a| ignore_finding_impl(state, &a.code, a.subject, &a.key, &a.fingerprint)
        ),
        "unignore_finding" => run!(
            json, "unignore_finding", args,
            { code: String, subject: Subject, key: String, fingerprint: String },
            |a| unignore_finding_impl(state, &a.code, a.subject, &a.key, &a.fingerprint)
        ),
        "accept_baseline" => run!(json, "accept_baseline", args, {}, |_a| {
            accept_baseline_impl(state)
        }),
        "set_check_enabled" => run!(
            json, "set_check_enabled", args,
            { code: String, enabled: bool },
            |a| set_check_enabled_impl(state, &a.code, a.enabled)
        ),
        "plan_fix" => run!(
            json, "plan_fix", args,
            { finding: FindingArg, fix: FixId },
            |a| plan_fix_impl(state, &a.finding, a.fix)
        ),
        "apply_fix" => run!(
            json, "apply_fix", args,
            { finding: FindingArg, fix: FixId },
            |a| apply_fix_impl(state, &a.finding, a.fix)
        ),
        "inspect_rom" => run!(json, "inspect_rom", args, { path: String }, |a| {
            let rom = policy.read_path(&a.path)?;
            inspect_rom_impl(&path_text(&rom)?)
        }),
        "default_project_parent" => run!(json, "default_project_parent", args, {}, |_a| {
            // The bridge has no Documents folder and refuses the `_DSPRE_contents` ending.
            Ok::<_, CmdError>(ProjectParentDto {
                parent: path_text(&policy.new_project_parent()?)?,
                dspre_suffix: false,
            })
        }),
        "check_project_dest" => run!(
            json, "check_project_dest", args,
            { dest: String, rom_size: u64 },
            |a| {
                let dest = policy.check_new_project(&a.dest)?;
                Ok::<_, CmdError>(check_project_dest_impl(&path_text(&dest)?, a.rom_size))
            }
        ),
        "create_project" => run!(
            json, "create_project", args,
            { rom: String, dest: String, template: g4_kernel::start::Template, text_project: Option<bool>, git: Option<bool> },
            |a| {
                let rom = policy.read_path(&a.rom)?;
                let dest = policy.check_new_project(&a.dest)?;
                // No live progress over HTTP: the answer carries the time of each phase.
                create_project_impl(
                    state,
                    &path_text(&rom)?,
                    &path_text(&dest)?,
                    a.template,
                    a.text_project.unwrap_or(false),
                    a.git.unwrap_or(false),
                    &mut |_| {},
                )
            }
        ),
        "plan_empty_region" => run!(json, "plan_empty_region", args, {}, |_a| {
            plan_empty_region_impl(state)
        }),
        "apply_empty_region" => run!(json, "apply_empty_region", args, {}, |_a| {
            apply_empty_region_impl(state)
        }),
        "agent_status" => run!(json, "agent_status", args, {}, |_a| agent_status_impl(
            state
        )),
        "agent_poll" => {
            run!(json, "agent_poll", args, { since: u64 }, |a| agent_poll_impl(state, a.since))
        }
        "agent_settings" => run!(json, "agent_settings", args, {}, |_a| agent_settings_impl(
            state
        )),
        "agent_set_enabled" => run!(json, "agent_set_enabled", args, { on: bool }, |a| {
            agent_set_enabled_impl(state, a.on)
        }),
        "agent_set_expert" => run!(json, "agent_set_expert", args, { on: bool }, |a| {
            agent_set_expert_impl(state, a.on)
        }),
        "agent_set_grant" => run!(
            json, "agent_set_grant", args,
            { level: crate::mcp::settings::GrantLevel },
            |a| agent_set_grant_impl(state, a.level)
        ),
        // The path names a saved grant; nothing is read or written there.
        "agent_remove_grant" => run!(json, "agent_remove_grant", args, { path: String }, |a| {
            agent_remove_grant_impl(state, &a.path)
        }),
        "agent_answer" => run!(
            json, "agent_answer", args,
            { question: u64, answer: String },
            |a| agent_answer_impl(state, a.question, &a.answer)
        ),
        "agent_disconnect" => run!(json, "agent_disconnect", args, {}, |_a| {
            agent_disconnect_impl(state)
        }),
        "agent_set_read_only" => run!(json, "agent_set_read_only", args, { on: bool }, |a| {
            agent_set_read_only_impl(state, a.on)
        }),
        "world_meta_get" => run!(json, "world_meta_get", args, {}, |_a| world_meta_get_impl(
            state
        )),
        "world_meta_set" => run!(
            json, "world_meta_set", args,
            { meta: g4_kernel::world_meta::WorldMeta },
            |a| world_meta_set_impl(state, a.meta)
        ),
        "place_slots" => run!(json, "place_slots", args, {}, |_a| place_slots_impl(state)),
        "create_place" => run!(
            json, "create_place", args,
            { name: String, headers: Vec<u16> },
            |a| create_place_impl(state, &a.name, &a.headers)
        ),
        "rename_place" => run!(
            json, "rename_place", args,
            { index: u8, name: String },
            |a| rename_place_impl(state, a.index, &a.name)
        ),
        "assign_place" => run!(
            json, "assign_place", args,
            { headers: Vec<u16>, index: u8 },
            |a| assign_place_impl(state, &a.headers, a.index)
        ),
        "project_preview_get" => {
            run!(json, "project_preview_get", args, { path: Option<String> }, |a| {
                match a.path.map(|p| preview_root(policy, &p)).transpose()? {
                    // Gone: no preview, and nothing to read.
                    Some(None) => Ok(PreviewDto {
                        source: None,
                        png: None,
                        standard_map: None,
                    }),
                    Some(Some(root)) => project_preview_get_impl(state, Some(&root)),
                    None => project_preview_get_impl(state, None),
                }
            })
        }
        "project_preview_set" => run!(
            json, "project_preview_set", args,
            { path: Option<String>, slot: Option<PreviewSlot>, png: Option<String>, file: Option<String> },
            |a| {
                let root = match a.path {
                    Some(p) => Some(preview_root(policy, &p)?.ok_or_else(|| {
                        CmdError::from(format!("Kein Projektordner: {p}"))
                    })?),
                    None => None,
                };
                let file = a.file.map(|f| policy.read_path(&f)).transpose()?;
                project_preview_set_impl(
                    state,
                    root.as_deref(),
                    a.slot.unwrap_or_default(),
                    a.png.as_deref(),
                    file.as_deref(),
                )
            }
        ),
        "base_status" => run!(json, "base_status", args, {}, |_a| base_status_impl(state)),
        "check_base" => run!(json, "check_base", args, { nds: String }, |a| {
            let nds = base_nds(state, policy, &a.nds)?;
            check_base_impl(state, &path_text(&nds)?)
        }),
        "assign_base" => run!(json, "assign_base", args, { nds: String }, |a| {
            let nds = base_nds(state, policy, &a.nds)?;
            assign_base_impl(state, &path_text(&nds)?)
        }),
        "register_base_rom" => run!(json, "register_base_rom", args, { path: String }, |a| {
            let path = policy.read_path(&a.path)?;
            register_base_rom_impl(state, &path_text(&path)?)
        }),
        "create_patch" => run!(json, "create_patch", args, {}, |_a| {
            // The targets are fixed (build/<Name>.nds and .xdelta); the policy still has the
            // last word on them once the base ROM is found (and makes build/ when it may).
            create_patch_checked(state, false, &|targets| {
                policy.check_output(&path_text(&targets.rom)?)?;
                policy.check_output(&path_text(&targets.patch)?)?;
                Ok(())
            })
        }),
        "original_status" => run!(json, "original_status", args, {}, |_a| {
            original_status_impl(state)
        }),
        "plan_reset" => run!(json, "plan_reset", args, { target: ResetTarget }, |a| {
            plan_reset_impl(state, a.target)
        }),
        "reset_to_original" => run!(
            json, "reset_to_original", args,
            { target: ResetTarget, fields: Option<Vec<String>> },
            |a| reset_to_original_impl(state, a.target, a.fields)
        ),
        // No live progress over HTTP: the report carries the counts and the time.
        "changed_from_original" => run!(json, "changed_from_original", args, {}, |_a| {
            changed_from_original_impl(state, &mut |_progress| {})
        }),
        "text_check" => run!(json, "text_check", args, {}, |_a| text_check_impl(state)),
        "text_export" => run!(json, "text_export", args, {}, |_a| text_export_impl(state)),
        "text_project_status" => run!(json, "text_project_status", args, {}, |_a| {
            text_project_status_impl(state)
        }),
        // No live progress over HTTP: the report carries the counts and the time.
        "adopt_text_project" => run!(json, "adopt_text_project", args, {}, |_a| {
            adopt_text_project_impl(state, &mut |_progress| {})
        }),
        "revert_text_project" => run!(json, "revert_text_project", args, {}, |_a| {
            revert_text_project_impl(state)
        }),
        "migrate_text_project" => run!(json, "migrate_text_project", args, {}, |_a| {
            migrate_text_project_impl(state)
        }),
        "external_sync" => run!(json, "external_sync", args, {}, |_a| {
            external_sync_impl(state, &mut |_progress| {})
        }),
        "external_changes" => run!(json, "external_changes", args, { diff: Option<bool> }, |a| {
            external_changes_impl(state, a.diff.unwrap_or(false))
        }),
        "conflict_diff" => run!(json, "conflict_diff", args, { key: String }, |a| {
            conflict_diff_impl(state, a.key)
        }),
        "resolve_external_conflict" => run!(
            json, "resolve_external_conflict", args,
            { key: String, keep: g4_kernel::textproject::sync::Keep },
            |a| resolve_external_conflict_impl(state, a.key, a.keep)
        ),
        "plan_rename_id" => {
            run!(json, "plan_rename_id", args, { id: String, new_id: String }, |a| {
                plan_rename_id_impl(state, a.id, a.new_id)
            })
        }
        "rename_id" => run!(json, "rename_id", args, { id: String, new_id: String }, |a| {
            rename_id_impl(state, a.id, a.new_id)
        }),
        "entity_ids" => run!(
            json, "entity_ids", args,
            { kind: g4_kernel::textproject::symbols::SymbolKind },
            |a| entity_ids_impl(state, a.kind)
        ),
        "close_project" => run!(json, "close_project", args, {}, |_a| close_project_impl(
            state
        )),
        "git_status" => run!(json, "git_status", args, {}, |_a| git_status_impl(state)),
        // No live progress over HTTP: the report carries the time of each phase.
        "git_init" => run!(json, "git_init", args, { templates_only: Option<bool> }, |a| {
            let opts = g4_kernel::git::InitOptions {
                templates_only: a.templates_only.unwrap_or(false),
            };
            git_init_impl(state, opts, &mut |_progress| {})
        }),
        // The subject comes flat (`kind`, `index`, `folder`, `path`), as the MCP tool takes it.
        "diff_entity" => run!(
            json, "diff_entity", args,
            {
                kind: String,
                index: Option<u16>,
                folder: Option<String>,
                path: Option<String>,
                against: g4_kernel::diff::Against,
            },
            |a| {
                let subject = diff_subject(a.kind, a.index, a.folder, a.path)?;
                diff_entity_impl(state, subject, a.against)
            }
        ),
        "unsaved_changes" => run!(json, "unsaved_changes", args, {}, |_a| {
            unsaved_changes_impl(state)
        }),
        // The folder is one `open_project` may open, the base ROM one the policy reads; no
        // live progress over HTTP.
        "restore_text_project" => run!(
            json, "restore_text_project", args,
            { root: String, nds: Option<String> },
            |a| {
                let root = path_text(&policy.check_project(&a.root)?)?;
                let nds = match a.nds {
                    Some(nds) => Some(path_text(&policy.read_path(&nds)?)?),
                    None => None,
                };
                restore_text_project_impl(state, root, nds, &mut |_progress| {})
            }
        ),
        "get_battle_sprite" => run!(
            bytes, "get_battle_sprite", args,
            { species: u16, form: Option<u8>, side: Side, female: Option<bool>, shiny: Option<bool>, frame: Option<u8> },
            |a| get_battle_sprite_impl(state, a.species, a.form, a.side, a.female, a.shiny, a.frame)
        ),
        "get_trainer_sprite" => run!(bytes, "get_trainer_sprite", args, { class: u8 }, |a| {
            get_trainer_sprite_impl(state, a.class)
        }),
        "releases" => run!(json, "releases", args, {}, |_a| releases_impl(state)),
        // `dir` (relative to the project folder) is the place. The policy is asked about every
        // file the release writes there and about the built ROM before the place is looked at
        // on disk (`check_target`, which makes nothing), and once more before the build
        // (`check_output`, which makes build/ when it may). No live progress over HTTP: the
        // report carries the time of each phase.
        "publish_patch" => run!(
            json, "publish_patch", args,
            {
                version: String,
                notes: Option<String>,
                dir: Option<String>,
                file: Option<String>,
                readme: Option<bool>,
                overwrite: Option<bool>,
            },
            |a| {
                let opts = PublishArgs {
                    version: a.version,
                    notes: a.notes,
                    dir: a.dir,
                    file: a.file,
                    readme: a.readme,
                    overwrite: a.overwrite,
                }
                .options();
                publish_patch_checked(state, opts, false, Some(policy), &mut |_progress| {})
            }
        ),
        // The command takes only a release file of the list or its folder, nothing else.
        "reveal_path" => run!(json, "reveal_path", args, { path: String }, |a| {
            reveal_path_impl(state, &a.path)
        }),
        // No live progress over HTTP: the report carries the time of each phase.
        "git_commit_all" => run!(
            json, "git_commit_all", args,
            { message: Option<String>, notes: Option<String> },
            |a| {
                let opts = g4_kernel::git::CommitOptions {
                    message: a.message,
                    notes: a.notes,
                };
                git_commit_all_impl(state, opts, &mut |_progress| {})
            }
        ),
        "git_commit_prepare" => run!(json, "git_commit_prepare", args, {}, |_a| {
            git_commit_prepare_impl(state)
        }),
        "recipe_list" => run!(json, "recipe_list", args, {}, |_a| Ok::<_, CmdError>(
            recipe_list_impl()
        )),
        // `api.ts` sends `{ recipe, input }`; absent or `null` input is `{}`.
        "recipe_plan" => run!(
            json, "recipe_plan", args,
            { recipe: g4_kernel::recipes::RecipeId, input: Option<Value> },
            |a| recipe_plan_impl(state, recipe_call(a.recipe, a.input))
        ),
        "recipe_apply" => run!(
            json, "recipe_apply", args,
            { recipe: g4_kernel::recipes::RecipeId, input: Option<Value> },
            |a| recipe_apply_impl(state, recipe_call(a.recipe, a.input))
        ),
        "list_flags" => run!(json, "list_flags", args, { kind: g4_kernel::flags::FlagKind }, |a| {
            list_flags_impl(state, a.kind)
        }),
        "flag_usage" => run!(
            json, "flag_usage", args,
            { kind: g4_kernel::flags::FlagKind, number: u16 },
            |a| flag_usage_impl(state, a.kind, a.number)
        ),
        "free_flag" => run!(json, "free_flag", args, { kind: g4_kernel::flags::FlagKind }, |a| {
            free_flag_impl(state, a.kind)
        }),
        "name_flag" => run!(
            json, "name_flag", args,
            {
                kind: g4_kernel::flags::FlagKind,
                number: u16,
                id: Option<String>,
                description: Option<String>,
            },
            |a| name_flag_impl(
                state,
                g4_kernel::flags::NameRequest {
                    kind: a.kind,
                    number: a.number,
                    id: a.id,
                    description: a.description,
                }
            )
        ),
        "dialog_of" => run!(
            json, "dialog_of", args,
            { header: u16, event_file: u16, kind: g4_kernel::dialog::DialogKind, index: u16 },
            |a| dialog_of_impl(state, a.header, a.event_file, a.kind, a.index)
        ),
        "split_message" => run!(
            json, "split_message", args,
            { header: u16, event_file: u16, index: u16, archive: u16, message: u16 },
            |a| split_message_impl(state, a.header, a.event_file, a.index, a.archive, a.message)
        ),
        "list_threads" => run!(json, "list_threads", args, {}, |_a| list_threads_impl(
            state
        )),
        "thread_graph" => run!(
            json, "thread_graph", args,
            { var: u16, story: Option<String> },
            |a| thread_graph_impl(state, a.var, a.story)
        ),
        "story_list" => run!(json, "story_list", args, {}, |_a| story_list_impl(state)),
        "story_get" => run!(json, "story_get", args, { id: String }, |a| {
            story_get_impl(state, a.id)
        }),
        // A progress the symbolic arguments made a number becomes its id again.
        "story_create" => run!(json, "story_create", args, { req: Value }, |a| {
            story_create_value(state, a.req)
        }),
        "story_update" => run!(
            json, "story_update", args,
            { id: String, patch: Value },
            |a| story_update_value(state, a.id, a.patch)
        ),
        "live_status" => run!(json, "live_status", args, {}, |_a| live_status_impl(state)),
        // Starts an emulator: the policy decides, like `run_rom` (the bridge refuses it).
        "live_play" => run!(
            json, "live_play", args,
            { emulator: String, rom: String },
            |a| {
                let (emulator, rom) = policy.check_run(&a.emulator, &a.rom)?;
                let text = |path: &std::path::Path| {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        CmdError::from(format!("Pfad ist kein gültiger Text: {}", path.display()))
                    })
                };
                live_play_impl(state, &text(&emulator)?, &text(&rom)?)
            }
        ),
        "live_connect" => run!(
            json, "live_connect", args,
            { port: u16, host: Option<String> },
            |a| live_connect_impl(state, a.host, a.port)
        ),
        "live_stop" => run!(json, "live_stop", args, {}, |_a| live_stop_impl(state)),
        "live_resume" => run!(json, "live_resume", args, {}, |_a| live_resume_impl(state)),
        "live_set" => run!(
            json, "live_set", args,
            { kind: g4_kernel::flags::FlagKind, number: u16, value: u16 },
            |a| live_set_impl(state, a.kind, a.number, a.value)
        ),
        "live_breakpoints" => run!(json, "live_breakpoints", args, {}, |_a| {
            live_breakpoints_impl(state)
        }),
        "live_breakpoint_set" => run!(
            json, "live_breakpoint_set", args,
            { event_file: u16, kind: String, index: u16 },
            |a| live_breakpoint_set_impl(
                state,
                crate::live_hub::EventRef { event_file: a.event_file, kind: a.kind, index: a.index }
            )
        ),
        "live_breakpoint_clear" => run!(json, "live_breakpoint_clear", args, { id: u32 }, |a| {
            live_breakpoint_clear_impl(state, a.id)
        }),
        "dialogs_of_header" => run!(json, "dialogs_of_header", args, { header: u16 }, |a| {
            dialogs_of_header_impl(state, a.header)
        }),
        "plan_delete" => run!(
            json, "plan_delete", args,
            { target: g4_kernel::delete::DeleteTarget },
            |a| plan_delete_impl(state, a.target)
        ),
        "delete_entity" => run!(
            json, "delete_entity", args,
            {
                target: g4_kernel::delete::DeleteTarget,
                // `g4_call` passes the curated tool's snake_case key as it is.
                #[serde(alias = "free_number")]
                free_number: Option<bool>,
            },
            |a| delete_entity_impl(state, a.target, a.free_number.unwrap_or(false))
        ),
        "restore_entity" => run!(
            json, "restore_entity", args,
            { target: g4_kernel::delete::DeleteTarget },
            |a| restore_entity_impl(state, a.target)
        ),
        "story_delete" => run!(json, "story_delete", args, { id: String }, |a| {
            story_delete_impl(state, a.id)
        }),
        "world_create" => run!(
            json, "world_create", args,
            { req: g4_kernel::world::edit::WorldCreateRequest },
            |a| world_create_impl(state, a.req)
        ),
        "world_edit" => run!(
            json, "world_edit", args,
            { req: g4_kernel::world::edit::WorldEditRequest },
            |a| world_edit_impl(state, a.req)
        ),
        "world_get" => run!(json, "world_get", args, { map: u16 }, |a| world_get_impl(
            state, a.map
        )),
        "tileset_get" => run!(json, "tileset_get", args, { package: u16 }, |a| {
            tileset_get_impl(state, a.package)
        }),
        "tileset_save" => run!(json, "tileset_save", args, { package: u16 }, |a| {
            tileset_save_impl(state, a.package)
        }),
        "get_world_mesh" => run!(json, "get_world_mesh", args, { map: u16 }, |a| {
            get_world_mesh_impl(state, a.map)
        }),
        "get_tileset_texture" => run!(
            json, "get_tileset_texture", args,
            { package: u16, texture: String },
            |a| get_tileset_texture_impl(state, a.package, a.texture)
        ),
        "get_map_heights" => run!(json, "get_map_heights", args, { index: u16 }, |a| {
            get_map_heights_impl(state, a.index)
        }),
        "set_map_terrain" => run!(
            json, "set_map_terrain", args,
            {
                index: u16,
                grid: g4_kernel::terrain::GridRows,
                kinds: Vec<String>,
                overrides: Option<Vec<g4_kernel::terrain::CellOverride>>,
                bdhc: Option<g4_kernel::terrain::BdhcSource>,
                permissions: Option<g4_kernel::terrain::PermissionSource>,
            },
            |a| set_map_terrain_impl(
                state,
                a.index,
                terrain_input(a.grid, a.kinds, a.overrides, a.bdhc, a.permissions)
            )
        ),
        "get_map_budget" => run!(json, "get_map_budget", args, { index: u16 }, |a| {
            get_map_budget_impl(state, a.index)
        }),
        "bake_test" => run!(
            json, "bake_test", args,
            { map: u16, block: Option<[u8; 2]>, out_dir: Option<String> },
            |a| {
                let out_dir = a.out_dir.map(|dir| policy.write_path(&dir)).transpose()?;
                bake_test_impl(state, a.map, a.block, out_dir)
            }
        ),
        "world_layers" => run!(
            json, "world_layers", args,
            { req: g4_kernel::world::WorldLayersRequest },
            |a| world_layers_impl(state, a.req)
        ),
        _ => Err(DispatchError::Unknown(name.to_owned())),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::policy::UserPolicy;
    use crate::commands::{
        dirty_impl, get_header_impl, get_matrix_impl, history_impl, list_headers_impl,
        open_project_impl, open_synthetic, open_synthetic_for_models,
    };
    use crate::state::AppState;
    use g4_kernel::model_import::samples::CUBE_OBJ;
    use serde_json::{Value, json};
    use std::collections::BTreeSet;

    /// The names in `generate_handler![ … ]` of `lib.rs`.
    fn handler_names() -> Vec<String> {
        let lib = include_str!("../lib.rs");
        let start = lib.find("generate_handler![").unwrap() + "generate_handler![".len();
        let block = &lib[start..start + lib[start..].find(']').unwrap()];
        block
            .split(',')
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn policy_over(fixture: &g4_kernel::test_support::Synthetic) -> UserPolicy {
        UserPolicy::new(fixture.dir.path(), &[], None).unwrap()
    }

    /// The bridge's policy over the synthetic project's folder.
    #[cfg(feature = "dev-bridge")]
    fn dev_policy_over(
        fixture: &g4_kernel::test_support::Synthetic,
    ) -> crate::bridge::roots::DevPolicy {
        crate::bridge::roots::DevPolicy::new(&[fixture.dir.path().to_path_buf()]).unwrap()
    }

    #[cfg(feature = "dev-bridge")]
    use crate::bridge::roots::Roots;

    #[cfg(feature = "dev-bridge")]
    fn roots_over(fixture: &g4_kernel::test_support::Synthetic) -> Roots {
        dev_policy_over(fixture)
    }

    fn call(state: &AppState, policy: &dyn Policy, name: &str, args: Value) -> Reply {
        dispatch(state, policy, name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
    }

    fn json_of(reply: Reply) -> Value {
        match reply {
            Reply::Json(v) => v,
            Reply::Bytes(_) => panic!("expected JSON"),
        }
    }

    #[test]
    fn commands_match_the_tauri_handler() {
        let handler = handler_names();
        let handler_set: BTreeSet<&str> = handler.iter().map(String::as_str).collect();
        let table_set: BTreeSet<&str> = COMMANDS.iter().copied().collect();
        assert_eq!(handler.len(), handler_set.len(), "duplicate in the handler");
        assert_eq!(COMMANDS.len(), table_set.len(), "duplicate in COMMANDS");
        let missing: Vec<_> = handler_set.difference(&table_set).collect();
        let extra: Vec<_> = table_set.difference(&handler_set).collect();
        assert!(missing.is_empty(), "not in COMMANDS: {missing:?}");
        assert!(extra.is_empty(), "not in the handler: {extra:?}");
    }

    #[test]
    fn every_command_accepts_the_keys_api_ts_sends() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let dir = fixture.dir.path().to_str().unwrap().to_owned();
        let out = "build/out.nds".to_owned();
        let per_in = fixture.dir.path().join("in.per");
        std::fs::write(&per_in, [0u8; 2048]).unwrap();
        let per_in = per_in.to_str().unwrap().to_owned();
        let per_out = "build/out.per".to_owned();
        let header = call(&state, &policy, "get_header", json!({ "id": 0 }));
        let header = json_of(header);
        let area = json_of(call(&state, &policy, "get_area_data", json!({ "id": 0 })));

        // One call per command, with the keys `api.ts` passes to `invoke`.
        let cases: Vec<(&str, Value)> = vec![
            ("open_project", json!({ "path": dir })),
            ("list_headers", json!({})),
            ("get_header", json!({ "id": 0 })),
            (
                "set_header",
                json!({ "id": 0, "header": header, "label": "edit" }),
            ),
            ("get_area_data", json!({ "id": 0 })),
            ("set_area_data", json!({ "id": 0, "area": area })),
            ("member_counts", Value::Null),
            ("get_matrix", json!({ "index": 0 })),
            (
                "set_matrix_cell",
                json!({ "index": 0, "x": 0, "y": 0, "map": 1 }),
            ),
            (
                "set_matrix_sections",
                json!({ "index": 0, "headers": false, "altitudes": false }),
            ),
            (
                "resize_matrix",
                json!({ "index": 0, "width": 2, "height": 1 }),
            ),
            ("rename_matrix", json!({ "index": 0, "name": "renamed" })),
            ("create_matrix", json!({ "name": "new" })),
            ("list_matrices", json!({})),
            ("list_warps", json!({})),
            (
                "duplicate_map",
                json!({ "source": 0, "clearBuildings": true }),
            ),
            ("create_event_file", json!({})),
            ("duplicate_event_file", json!({ "source": 0 })),
            (
                "create_area",
                json!({ "spec": {
                    "name": "area", "width": 1, "height": 1,
                    "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": true }],
                    "with_headers": false, "header": null, "target": null,
                } }),
            ),
            (
                "extend_area",
                json!({ "spec": {
                    "matrix": 0, "x": 0, "y": 0, "source_map": 0,
                    "clear_buildings": false, "header_cell": null,
                } }),
            ),
            (
                "create_room",
                json!({ "spec": {
                    "name": "room", "source_map": 0, "clear_buildings": true,
                    "header": { "id": 0, "area_data_id": null, "event_file": "Keep",
                                "allow_patch": false, "place": { "Existing": 1 } },
                    "entrance": null,
                } }),
            ),
            ("split_room", json!({ "header": 1, "name": "Haus" })),
            ("header_patch_status", json!({})),
            ("apply_header_patch", json!({})),
            ("repair_header_patch", json!({})),
            ("get_map", json!({ "index": 0 })),
            ("get_events", json!({ "eventFileId": 1 })),
            ("get_building_glb", json!({ "modelId": 0, "areaDataId": 0 })),
            ("add_building", json!({ "map": 0 })),
            (
                "update_building",
                json!({ "map": 0, "index": 0, "patch": { "x": 1.0 } }),
            ),
            ("delete_building", json!({ "map": 0, "index": 0 })),
            (
                "update_event",
                json!({ "eventFile": 1, "kind": "warp", "index": 0, "patch": { "dest_header_id": 2 } }),
            ),
            ("get_text_archive", json!({ "index": 433 })),
            (
                "edit_text_archive",
                json!({ "index": 433, "edit": { "Set": { "index": 0, "text": "x" } }, "label": null }),
            ),
            ("create_text_archive", Value::Null),
            ("search_text", json!({ "query": "a", "limit": 5 })),
            ("validate_text", json!({ "text": "hello" })),
            ("scripts_status", json!({})),
            ("adopt_scripts", json!({})),
            ("list_script_files", json!({})),
            ("read_script", json!({ "file": 0 })),
            ("save_script", json!({ "file": 0, "text": "" })),
            ("save_level_script", json!({ "file": 0, "doc": {} })),
            ("new_script_file", json!({ "kind": "script" })),
            ("add_script", json!({ "file": 0 })),
            ("resolve_script", json!({ "header": 0, "n": 1 })),
            ("search_scripts", json!({ "query": "a", "limit": 5 })),
            ("lsp_start", json!({})),
            ("lsp_send", json!({ "body": "{}" })),
            ("lsp_stop", json!({})),
            (
                "add_event",
                json!({ "eventFile": 1, "kind": "object", "x": 1, "z": 1, "patch": { "script": 3929, "trainer_type": 1, "sight": 3 }, "label": "Trainer Jens platziert" }),
            ),
            (
                "delete_event",
                json!({ "eventFile": 1, "kind": "warp", "index": 0 }),
            ),
            ("get_map_model_glb", json!({ "index": 0, "areaDataId": 0 })),
            ("get_object_sprite", json!({ "graphicsId": 0, "facing": 0 })),
            (
                "map_model_texture_status",
                json!({ "index": 0, "areaDataId": 0 }),
            ),
            ("tile_behaviors", json!({})),
            (
                "set_permission",
                json!({ "index": 0, "x": 0, "y": 0, "kind": 1, "collision": 0 }),
            ),
            ("undo", json!({})),
            ("redo", json!({})),
            ("history", json!({})),
            ("dirty", json!({})),
            ("save", json!({})),
            ("build_rom", json!({ "out": out })),
            ("run_rom", json!({ "emulator": "emu", "rom": "rom.nds" })),
            ("reload_asset", json!({ "kind": "Event", "index": 0 })),
            ("accept_disk", json!({ "kind": "Event", "index": 0 })),
            (
                "resolve_text_conflicts",
                json!({ "choices": [{ "archive": 433, "keep": "json" }] }),
            ),
            ("force_quit", json!({})),
            (
                "data_list",
                json!({ "kind": "species", "filter": { "query": "a", "types": [1], "pocket": null, "category": null } }),
            ),
            ("data_get", json!({ "kind": "species", "index": 1 })),
            (
                "data_edit",
                json!({ "kind": "species", "index": 1, "patch": { "Personal": { "field": "attack", "value": 85 } }, "label": null }),
            ),
            ("data_raw", json!({ "kind": "move", "index": 1 })),
            ("name_tables", json!({})),
            ("item_table", json!({})),
            ("machine_table", json!({})),
            (
                "refs",
                json!({ "target": { "kind": "species", "index": 1 } }),
            ),
            ("get_item_icon", json!({ "item": 1 })),
            ("get_mon_icon", json!({ "species": 1 })),
            ("create_trainer", json!({ "class": 1, "name": "Jens" })),
            (
                "set_trainer_texts",
                json!({ "trainer": 1, "texts": [{ "kind": 0, "text": "Los!\\r" }] }),
            ),
            ("create_encounter_file", json!({ "copyOf": 0 })),
            ("split_encounter_file", json!({ "header": 0 })),
            (
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [per_in] }, "options": { "unit": 16, "colors": 16 } }),
            ),
            (
                "import_map_parts",
                json!({ "map": 0, "source": { "map": 2 }, "parts": ["permissions"], "keepBuildings": false,
                        "options": { "unit": 16, "colors": 16 } }),
            ),
            (
                "export_map_part",
                json!({ "map": 0, "part": "permissions", "path": per_out }),
            ),
            ("run_checks", json!({ "scope": "cheap" })),
            ("checks_status", json!({})),
            (
                "ignore_finding",
                json!({ "code": "header.text-split", "subject": { "kind": "scriptFile", "index": 0 },
                        "key": "text_archive", "fingerprint": "00000000000000ff" }),
            ),
            (
                "unignore_finding",
                json!({ "code": "header.text-split", "subject": { "kind": "scriptFile", "index": 0 },
                        "key": "text_archive", "fingerprint": "00000000000000ff" }),
            ),
            ("accept_baseline", json!({})),
            (
                "set_check_enabled",
                json!({ "code": "warp.never-fires", "enabled": false }),
            ),
            (
                "plan_fix",
                json!({ "finding": { "code": "text.too-wide", "subject": { "kind": "textArchive", "index": 433 },
                        "key": "message:1:line:1", "fingerprint": "00000000000000ff" }, "fix": "wrap" }),
            ),
            (
                "apply_fix",
                json!({ "finding": { "code": "text.too-wide", "subject": { "kind": "textArchive", "index": 433 },
                        "key": "message:1:line:1", "fingerprint": "00000000000000ff" }, "fix": "wrap" }),
            ),
            ("inspect_rom", json!({ "path": per_in })),
            ("default_project_parent", json!({})),
            (
                "check_project_dest",
                json!({ "dest": format!("{dir}/neu"), "romSize": 1000 }),
            ),
            (
                "create_project",
                json!({ "rom": per_in, "dest": format!("{dir}/neu"), "template": "original",
                        "textProject": true, "git": false }),
            ),
            ("plan_empty_region", json!({})),
            ("apply_empty_region", json!({})),
            ("agent_status", json!({})),
            ("agent_poll", json!({ "since": 0 })),
            ("agent_settings", json!({})),
            ("agent_set_enabled", json!({ "on": false })),
            ("agent_set_expert", json!({ "on": false })),
            ("agent_set_grant", json!({ "level": "read" })),
            ("agent_remove_grant", json!({ "path": "/nirgends" })),
            ("agent_answer", json!({ "question": 1, "answer": "deny" })),
            ("agent_disconnect", json!({})),
            ("agent_set_read_only", json!({ "on": false })),
            ("world_meta_get", json!({})),
            (
                "world_meta_set",
                json!({ "meta": { "roomNames": { "1": "Haus" } } }),
            ),
            ("place_slots", json!({})),
            (
                "create_place",
                json!({ "name": "Neuheim", "headers": [0, 1] }),
            ),
            (
                "rename_place",
                json!({ "index": 1, "name": "Zweiblattdorf" }),
            ),
            ("assign_place", json!({ "headers": [0], "index": 2 })),
            // `api.preview.ts` sends every key, `null` when it is not given.
            ("project_preview_get", json!({ "path": null })),
            (
                "project_preview_set",
                json!({ "path": null, "slot": null, "png": null, "file": null }),
            ),
            ("base_status", json!({})),
            ("check_base", json!({ "nds": per_in })),
            ("assign_base", json!({ "nds": per_in })),
            ("register_base_rom", json!({ "path": per_in })),
            ("create_patch", json!({})),
            ("original_status", json!({})),
            (
                "plan_reset",
                json!({ "target": { "kind": "text_message", "archive": 433, "message": 0 } }),
            ),
            (
                "reset_to_original",
                json!({ "target": { "kind": "header", "id": 0 }, "fields": ["music_day"] }),
            ),
            ("changed_from_original", json!({})),
            ("text_check", json!({})),
            ("text_export", json!({})),
            ("text_project_status", json!({})),
            ("adopt_text_project", json!({})),
            ("revert_text_project", json!({})),
            ("migrate_text_project", json!({})),
            ("external_sync", json!({})),
            ("external_changes", json!({ "diff": true })),
            ("conflict_diff", json!({ "key": "data/moves/0001.json" })),
            (
                "resolve_external_conflict",
                json!({ "key": "data/moves/0001.json", "keep": "studio" }),
            ),
            (
                "plan_rename_id",
                json!({ "id": "TRAINER_YOUNGSTER_KAI", "newId": "TRAINER_KID_KAI" }),
            ),
            (
                "rename_id",
                json!({ "id": "TRAINER_YOUNGSTER_KAI", "newId": "TRAINER_KID_KAI" }),
            ),
            ("entity_ids", json!({ "kind": "Species" })),
            ("git_status", json!({})),
            // Templates only: a test never runs the user's `git`.
            ("git_init", json!({ "templatesOnly": true })),
            (
                "diff_entity",
                json!({ "kind": "trainer", "index": 1, "against": "saved" }),
            ),
            ("unsaved_changes", json!({})),
            ("restore_text_project", json!({ "root": dir, "nds": null })),
            (
                "get_battle_sprite",
                json!({ "species": 1, "side": "front" }),
            ),
            ("get_trainer_sprite", json!({ "class": 1 })),
            ("releases", json!({})),
            (
                "publish_patch",
                json!({ "version": "0.1.0", "notes": "Erste Fassung" }),
            ),
            ("reveal_path", json!({ "path": "build/x.xdelta" })),
            // No repository: refused before git runs.
            ("git_commit_all", json!({ "message": "Stand" })),
            // No repository: refused before anything is written.
            ("git_commit_prepare", json!({})),
            ("recipe_list", json!({})),
            (
                "recipe_plan",
                json!({ "recipe": "extend_area", "input": {
                    "matrix": 0, "x": 3, "y": 0, "source_map": 0,
                    "clear_buildings": false, "header_cell": null,
                } }),
            ),
            (
                "recipe_apply",
                json!({ "recipe": "extend_area", "input": {
                    "matrix": 0, "x": 3, "y": 0, "source_map": 0,
                    "clear_buildings": false, "header_cell": null,
                } }),
            ),
            ("list_flags", json!({"kind": "flag"})),
            ("flag_usage", json!({"kind": "flag", "number": 148})),
            ("free_flag", json!({"kind": "var"})),
            (
                "name_flag",
                json!({"kind": "flag", "number": 148, "id": "FLAG_TEST", "description": null}),
            ),
            (
                "dialog_of",
                json!({"header": 1, "eventFile": 0, "kind": "object", "index": 0}),
            ),
            (
                "split_message",
                json!({"header": 1, "eventFile": 0, "index": 0, "archive": 3, "message": 5}),
            ),
            ("list_threads", json!({})),
            ("thread_graph", json!({"var": 16432, "story": null})),
            ("story_list", json!({})),
            ("story_get", json!({"id": "STORY_TEST"})),
            (
                "story_create",
                json!({"req": {"name": "Test", "progress": {"var": "VAR_UNUSED_0x406C"}}}),
            ),
            (
                "story_update",
                json!({"id": "STORY_TEST", "patch": {"notes": "x"}}),
            ),
            ("live_status", json!({})),
            ("live_play", json!({ "emulator": "emu", "rom": "rom.nds" })),
            ("live_connect", json!({ "port": free_port(), "host": null })),
            ("live_stop", json!({})),
            ("live_resume", json!({})),
            (
                "live_set",
                json!({ "kind": "flag", "number": 0x173, "value": 0 }),
            ),
            ("live_breakpoints", json!({})),
            (
                "live_breakpoint_set",
                json!({ "eventFile": 0, "kind": "warp", "index": 0 }),
            ),
            ("live_breakpoint_clear", json!({ "id": 1 })),
            ("dialogs_of_header", json!({"header": 1})),
            // No base ROM: refused before anything is planned or deleted.
            (
                "plan_delete",
                json!({ "target": { "kind": "trainer", "index": 1 } }),
            ),
            (
                "delete_entity",
                json!({ "target": { "kind": "flag", "number": 148 }, "freeNumber": false }),
            ),
            (
                "restore_entity",
                json!({ "target": { "kind": "header", "id": 0 } }),
            ),
            ("story_delete", json!({"id": "STORY_TEST"})),
            // The DSPRE layout: the writers refuse, the readers answer no world.
            ("world_create", json!({ "req": { "map": 0, "tileset": 0 } })),
            (
                "world_edit",
                json!({ "req": { "map": 0, "changes": [{ "cells": [[0, 0]], "tile": "0001" }] } }),
            ),
            ("world_get", json!({ "map": 0 })),
            ("tileset_get", json!({ "package": 0 })),
            ("tileset_save", json!({ "package": 0 })),
            ("get_world_mesh", json!({ "map": 0 })),
            (
                "get_tileset_texture",
                json!({ "package": 0, "texture": "syn_boden" }),
            ),
            ("get_map_heights", json!({ "index": 0 })),
            (
                "set_map_terrain",
                json!({
                    "index": 0,
                    "grid": { "heights": vec![vec!["0"; 32].join(" "); 32],
                              "ramps": vec![vec!["."; 32].join(" "); 32] },
                    "kinds": vec![vec!["."; 32].join(" "); 32],
                    "overrides": [{ "x": 1, "y": 2, "kind": 0, "collision": 128 }],
                    "bdhc": "grid",
                    "permissions": "derive",
                }),
            ),
            ("get_map_budget", json!({ "index": 0 })),
            // Pack 0 of the synthetic project is no NSBTX: refused before anything is written.
            ("bake_test", json!({ "map": 0 })),
            // Outside a text project: refused before anything is written.
            (
                "world_layers",
                json!({ "req": { "map": 0, "op": { "add": { "name": "Brücke" } } } }),
            ),
            // Last: it closes the project the cases above work on.
            ("close_project", json!({})),
        ];

        let tested: BTreeSet<&str> = cases.iter().map(|(name, _)| *name).collect();
        let table: BTreeSet<&str> = COMMANDS.iter().copied().collect();
        assert_eq!(cases.len(), tested.len(), "a command is tested twice");
        assert_eq!(tested, table, "every command needs a case");

        for (name, args) in cases {
            match dispatch(&state, &policy, name, args) {
                Ok(_) => {}
                Err(DispatchError::Unknown(n)) => panic!("{name}: the table does not know {n}"),
                Err(DispatchError::Cmd(e)) => assert!(
                    !e.message.starts_with("Ungültige Argumente"),
                    "{name}: {}",
                    e.message
                ),
            }
        }
    }

    /// `delete_entity` honours `free_number` as `g4_call` passes the curated tool's key, as it
    /// does `freeNumber`: a flag has no number to free.
    #[test]
    fn delete_entity_takes_free_number_in_snake_case() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        for key in ["freeNumber", "free_number"] {
            let mut args = json!({ "target": { "kind": "flag", "number": 148 } });
            args[key] = json!(true);
            let message = match dispatch(&state, &policy, "delete_entity", args) {
                Err(DispatchError::Cmd(e)) => e.message,
                other => panic!("{key}: {:?}", other.map(|_| ())),
            };
            assert_eq!(
                message, "Flag 0x0094 hat keine Nummer, die sich freigeben lässt.",
                "{key}"
            );
        }
    }

    /// `recipe_plan` answers the draft and changes nothing; `recipe_apply` is one undo step
    /// whose `changed` names the matrix; `recipe_list` names the four recipes.
    #[test]
    fn recipe_plan_writes_nothing_and_recipe_apply_is_one_step() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let list = json_of(call(&state, &policy, "recipe_list", json!({})));
        let ids: Vec<&str> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "create_area",
                "create_room",
                "extend_area",
                "empty_region",
                "new_figure",
                "connect_doors",
                "new_trainer_here",
                "new_story",
                "talk_advances",
                "figure_appears",
                "scene_on_enter",
                "reward_once"
            ]
        );
        let args = json!({ "recipe": "extend_area", "input": {
            "matrix": 0, "x": 2, "y": 0, "source_map": 2,
            "clear_buildings": false, "header_cell": null,
        } });
        let undo = history_impl(&state).unwrap().undo.len();
        let draft = json_of(call(&state, &policy, "recipe_plan", args.clone()));
        assert_eq!(
            draft["cells"],
            json!([{ "matrix": 0, "x": 2, "y": 0, "source_map": 2 }]),
            "{draft}"
        );
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert_eq!(history_impl(&state).unwrap().undo.len(), undo);

        let outcome = json_of(call(&state, &policy, "recipe_apply", args));
        assert_eq!(history_impl(&state).unwrap().undo.len(), undo + 1);
        let changed = outcome["changed"].as_array().unwrap();
        assert!(
            changed.contains(&json!({ "kind": "Matrix", "index": 0 })),
            "{outcome}"
        );
        assert!(outcome["change"].is_u64(), "{outcome}");
        assert_eq!(outcome["report"]["width"], 3, "{outcome}");
        assert_eq!(
            outcome["created"],
            json!([{ "kind": "Map", "index": outcome["report"]["map"] }]),
            "{outcome}"
        );
        // Without input the empty region reads `{}`: the kernel refuses, not the arguments.
        match dispatch(
            &state,
            &policy,
            "recipe_plan",
            json!({ "recipe": "empty_region" }),
        ) {
            Err(DispatchError::Cmd(e)) => {
                assert!(
                    !e.message.starts_with("Ungültige Argumente"),
                    "{}",
                    e.message
                )
            }
            Ok(_) => {}
            Err(other) => panic!("{other:?}"),
        }
    }

    /// `recipe_apply` stays generic: `recipe: new_story` with its input reaches the kernel
    /// recipe through the dispatch table (the story and its variable's name in the report, one
    /// undo step); in the DSPRE layout the kernel refuses it, not the arguments.
    #[test]
    fn recipe_apply_new_story_reaches_the_kernel() {
        let fx = g4_kernel::test_support::TextFixture::exported();
        let state = AppState::default();
        state.base_roms.set_explicit(Some(fx.rom().to_path_buf()));
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let free = json_of(call(&state, &policy, "free_flag", json!({ "kind": "var" })));
        let var = free["number"].as_u64().unwrap();
        let undo = history_impl(&state).unwrap().undo.len();
        let outcome = json_of(call(
            &state,
            &policy,
            "recipe_apply",
            json!({ "recipe": "new_story", "input": { "name": "Testfaden", "var": var } }),
        ));
        assert_eq!(outcome["report"]["story"], "STORY_TESTFADEN", "{outcome}");
        assert_eq!(outcome["report"]["var"], var, "{outcome}");
        assert_eq!(
            outcome["report"]["var_id"], "VAR_STORY_TESTFADEN",
            "{outcome}"
        );
        assert_eq!(history_impl(&state).unwrap().undo.len(), undo + 1);
        let list = json_of(call(&state, &policy, "story_list", json!({})));
        assert_eq!(list["stories"][0]["id"], "STORY_TESTFADEN", "{list}");

        let (synthetic, dspre) = open_synthetic();
        match dispatch(
            &dspre,
            &policy_over(&synthetic),
            "recipe_apply",
            json!({ "recipe": "new_story", "input": { "name": "Testfaden", "var": var } }),
        ) {
            Err(DispatchError::Cmd(e)) => assert_eq!(
                e.message,
                g4_kernel::textproject::rename::NOT_A_TEXT_PROJECT
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn assign_base_allows_the_suggested_sibling_and_refuses_other_outside_paths() {
        use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};
        // `<tmp>/X_DSPRE_contents` with `<tmp>/X.nds` next to it, outside the project.
        let s = synthetic_project();
        let (_rom_dir, rom) = synthetic_rom();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("X_DSPRE_contents");
        copy_dir(s.dir.path(), &root);
        let nds = tmp.path().join("X.nds");
        std::fs::copy(&rom, &nds).unwrap();
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(&root, &[], None).unwrap();
        let outside = |name: &str, args: Value| match dispatch(&state, &policy, name, args) {
            Err(DispatchError::Cmd(e)) => assert!(
                e.message.starts_with("Pfad liegt außerhalb"),
                "{name}: {}",
                e.message
            ),
            other => panic!("{name}: {other:?}"),
        };

        // Another ROM outside the project is not read, not even a copy of the sibling.
        let elsewhere = tempfile::tempdir().unwrap();
        let copy = elsewhere.path().join("X.nds");
        std::fs::copy(&nds, &copy).unwrap();
        for name in ["check_base", "assign_base"] {
            outside(name, json!({ "nds": copy.to_str().unwrap() }));
        }
        outside(
            "register_base_rom",
            json!({ "path": copy.to_str().unwrap() }),
        );
        // The sibling as an original under G4_ROMS_DIR is not read either.
        state
            .base_roms
            .configure_with(None, None, Some(tmp.path().to_path_buf()));
        outside("assign_base", json!({ "nds": nds.to_str().unwrap() }));
        assert!(!root.join(".g4").exists());
        state.base_roms.configure_with(None, None, None);

        // The sibling is read, absolute or relative to the project folder.
        let check = json_of(call(
            &state,
            &policy,
            "check_base",
            json!({ "nds": "../X.nds" }),
        ));
        assert_eq!(check["refused"], Value::Null, "{check}");
        let status = json_of(call(
            &state,
            &policy,
            "assign_base",
            json!({ "nds": nds.to_str().unwrap() }),
        ));
        assert_eq!(status["state"], "found", "{status}");
        assert!(root.join(".g4/base.json").is_file());
        // Base status reads; registering another path still goes through the policy.
        let status = json_of(call(&state, &policy, "base_status", json!({})));
        assert_eq!(status["state"], "found", "{status}");
        outside(
            "register_base_rom",
            json!({ "path": nds.to_str().unwrap() }),
        );
    }

    #[test]
    fn publish_patch_puts_its_place_through_the_policy_before_the_build() {
        use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};
        let s = synthetic_project();
        let (_rom_dir, rom) = synthetic_rom();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("X_DSPRE_contents");
        copy_dir(s.dir.path(), &root);
        let nds = tmp.path().join("X.nds");
        std::fs::copy(&rom, &nds).unwrap();
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        assign_base_impl(&state, nds.to_str().unwrap()).unwrap();
        let policy = UserPolicy::new(&root, &[], None).unwrap();

        // A place outside the allowed folders: refused, nothing built, no build/ made.
        let elsewhere = tempfile::tempdir().unwrap();
        let args = json!({ "version": "0.1.0", "dir": elsewhere.path().to_str().unwrap() });
        match dispatch(&state, &policy, "publish_patch", args) {
            Err(DispatchError::Cmd(e)) => assert!(
                e.message.starts_with("Pfad liegt außerhalb"),
                "{}",
                e.message
            ),
            other => panic!("{other:?}"),
        }
        assert!(!root.join("build").exists());
        assert!(
            std::fs::read_dir(elsewhere.path())
                .unwrap()
                .next()
                .is_none()
        );
        // An outside place that is a file, or holds the file already: the policy's sentence,
        // before the place is looked at (never `bad_dir` or `exists`).
        let file = elsewhere.path().join("datei");
        std::fs::write(&file, b"x").unwrap();
        std::fs::write(elsewhere.path().join("X-v0.1.0.xdelta"), b"alt").unwrap();
        for dir in [file.clone(), elsewhere.path().to_path_buf()] {
            let args = json!({ "version": "0.1.0", "dir": dir.to_str().unwrap() });
            match dispatch(&state, &policy, "publish_patch", args) {
                Err(DispatchError::Cmd(e)) => assert!(
                    e.code == "error" && e.message.starts_with("Pfad liegt außerhalb"),
                    "{dir:?}: {}: {}",
                    e.code,
                    e.message
                ),
                other => panic!("{other:?}"),
            }
        }
        assert!(!root.join("build").exists());

        // In build/ the policy makes the folder and the build runs (and fails on the
        // synthetic project as build_rom does).
        let err = match dispatch(
            &state,
            &policy,
            "publish_patch",
            json!({ "version": "0.1.0" }),
        ) {
            Err(DispatchError::Cmd(e)) => e,
            other => panic!("{other:?}"),
        };
        let project = g4_kernel::project::Project::open(&root).unwrap();
        let out = g4_kernel::patch::default_targets(&project).rom;
        let plain = build_rom_impl(&state, out.to_str().unwrap(), false).unwrap_err();
        assert_eq!((err.code, &err.message), (plain.code, &plain.message));
        assert!(!root.join(".g4/releases.json").exists());
    }

    #[test]
    fn create_patch_without_a_base_creates_nothing() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        match dispatch(&state, &policy, "create_patch", json!({})) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!((e.code, e.message.as_str()), ("base_missing", NO_MARK))
            }
            other => panic!("{other:?}"),
        }
        assert!(!fixture.dir.path().join("build").exists());
    }

    /// `<tmp>/_testkopien/X_DSPRE_contents` (the synthetic project, no `build/`) open in a
    /// fresh state with `X.nds` beside it assigned as its base ROM.
    fn copy_with_a_base() -> (tempfile::TempDir, std::path::PathBuf, AppState) {
        use g4_kernel::test_support::{copy_dir, synthetic_project, synthetic_rom};
        let s = synthetic_project();
        let (_rom_dir, rom) = synthetic_rom();
        let tmp = tempfile::tempdir().unwrap();
        let copies = tmp.path().join("_testkopien");
        let root = copies.join("X_DSPRE_contents");
        copy_dir(s.dir.path(), &root);
        let nds = copies.join("X.nds");
        std::fs::copy(&rom, &nds).unwrap();
        assert!(!root.join("build").exists());
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        crate::commands::base::assign_base_impl(&state, nds.to_str().unwrap()).unwrap();
        (tmp, root, state)
    }

    /// `create_patch` through `policy` on a project without `build/`: the policy makes the
    /// folder and the build runs (and fails on the synthetic project as `build_rom` does).
    fn create_patch_makes_build(policy: &dyn Policy, root: &std::path::Path, state: &AppState) {
        let err = match dispatch(state, policy, "create_patch", json!({})) {
            Err(DispatchError::Cmd(e)) => e,
            other => panic!("{other:?}"),
        };
        assert!(root.join("build").is_dir(), "{}", err.message);
        let targets = crate::commands::base::patch_targets(state).unwrap();
        let plain = build_rom_impl(state, targets.rom.to_str().unwrap(), false).unwrap_err();
        assert_eq!((err.code, &err.message), (plain.code, &plain.message));
    }

    #[test]
    fn create_patch_makes_build_through_the_user_policy() {
        let (_tmp, root, state) = copy_with_a_base();
        let policy = UserPolicy::new_with_roms(&root, &[], None, None).unwrap();
        create_patch_makes_build(&policy, &root, &state);
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn create_patch_makes_build_through_the_bridge() {
        let (tmp, root, state) = copy_with_a_base();
        let policy = Roots::new(&[tmp.path().join("_testkopien")]).unwrap();
        create_patch_makes_build(&policy, &root, &state);
    }

    #[test]
    fn create_area_takes_the_patch_a_new_place_and_no_internal_name() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let report = json_of(call(
            &state,
            &policy,
            "create_area",
            json!({ "spec": {
                "name": "Neuheim", "width": 1, "height": 1,
                "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": true }],
                "with_headers": true,
                "header": { "id": 2, "area_data_id": null, "event_file": "New",
                            "new_header": { "template": 1, "internal_name": null },
                            "allow_patch": true, "place": { "New": "Neuheim" } },
            } }),
        ));
        assert_eq!(
            (&report["header"], &report["patched"], &report["place"]),
            (&json!(2), &json!(true), &json!(3))
        );
    }

    #[test]
    fn create_area_takes_a_target_on_a_world_map() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let spec = json!({
            "name": "Neuheim", "width": 1, "height": 1,
            "cells": [{ "x": 0, "y": 0, "source_map": 0, "clear_buildings": true }],
            "with_headers": false, "header": null,
            "target": { "matrix": 0, "x": 2, "y": 0 },
        });
        // Matrix 0 has no header section yet: refused as the kernel words it.
        match dispatch(&state, &policy, "create_area", json!({ "spec": spec })) {
            Err(DispatchError::Cmd(e)) => assert!(e.message.contains("keine Weltkarte"), "{e:?}"),
            Err(e) => panic!("{e:?}"),
            Ok(_) => panic!("a matrix without a header section is no target"),
        }
        let sections = json!({ "index": 0, "headers": true, "altitudes": false });
        call(&state, &policy, "set_matrix_sections", sections);
        let size = json!({ "index": 0, "width": 3, "height": 1 });
        call(&state, &policy, "resize_matrix", size);
        let report = json_of(call(
            &state,
            &policy,
            "create_area",
            json!({ "spec": spec }),
        ));
        assert_eq!(
            (&report["matrix"], &report["replaced"]),
            (&json!(0), &json!(0))
        );
    }

    #[test]
    fn reads_match_the_direct_impl_call() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        fn direct<T: serde::Serialize>(v: T) -> Value {
            serde_json::to_value(v).unwrap()
        }
        assert_eq!(
            json_of(call(&state, &policy, "list_headers", json!({}))),
            direct(list_headers_impl(&state).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &policy, "get_header", json!({ "id": 0 }))),
            direct(get_header_impl(&state, 0).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &policy, "get_matrix", json!({ "index": 0 }))),
            direct(get_matrix_impl(&state, 0).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &policy, "dirty", Value::Null)),
            direct(dirty_impl(&state).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &policy, "history", Value::Null)),
            direct(history_impl(&state).unwrap())
        );
    }

    #[test]
    fn glb_commands_reply_with_bytes_or_tool_missing() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        for (name, args) in [
            ("get_map_model_glb", json!({ "index": 0, "areaDataId": 0 })),
            ("get_building_glb", json!({ "modelId": 0, "areaDataId": 0 })),
        ] {
            match dispatch(&state, &policy, name, args) {
                Ok(Reply::Bytes(_)) => {}
                Err(DispatchError::Cmd(e)) if e.code == "tool_missing" => {}
                // A debug build finds tools/apicula/bin/apicula on its own; it rejects the
                // synthetic placeholder model, which still proves the command was dispatched.
                Err(DispatchError::Cmd(e))
                    if e.code == "error" && e.message.contains("apicula") => {}
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[test]
    fn icon_commands_reply_with_bytes_and_an_empty_body_means_none() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        // The synthetic CPUE arm9 has no verified tables: no icon, no error.
        for (name, args) in [
            ("get_item_icon", json!({ "item": 17 })),
            ("get_mon_icon", json!({ "species": 1 })),
        ] {
            match call(&state, &policy, name, args) {
                Reply::Bytes(bytes) => assert!(bytes.is_empty(), "{name}"),
                Reply::Json(v) => panic!("{name}: {v}"),
            }
        }
        // The synthetic battle sprites are there: a PNG, and empty for a species without one.
        for (name, args, png) in [
            (
                "get_battle_sprite",
                json!({ "species": 1, "side": "front" }),
                true,
            ),
            (
                "get_battle_sprite",
                json!({ "species": 1, "form": 0, "side": "back", "female": true, "shiny": true, "frame": 1 }),
                true,
            ),
            (
                "get_battle_sprite",
                json!({ "species": 600, "side": "front" }),
                false,
            ),
            ("get_trainer_sprite", json!({ "class": 1 }), true),
            ("get_trainer_sprite", json!({ "class": 7 }), false),
        ] {
            match call(&state, &policy, name, args.clone()) {
                Reply::Bytes(bytes) => {
                    assert_eq!(bytes.starts_with(b"\x89PNG"), png, "{name} {args}");
                    assert_eq!(bytes.is_empty(), !png, "{name} {args}");
                }
                Reply::Json(v) => panic!("{name}: {v}"),
            }
        }
    }

    #[test]
    fn data_commands_answer_like_their_impl() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let rows = json_of(call(
            &state,
            &policy,
            "data_list",
            json!({ "kind": "species", "filter": { "query": "ivy" } }),
        ));
        assert_eq!(rows[0]["name"], "Ivysaur");
        let doc = json_of(call(
            &state,
            &policy,
            "data_get",
            json!({ "kind": "move", "index": 1 }),
        ));
        assert_eq!(doc["kind"], "move");
        assert_eq!(doc["description"]["text"], "Pounds the foe.");
        let changed = json_of(call(
            &state,
            &policy,
            "data_edit",
            json!({ "kind": "species", "index": 2, "patch": { "Personal": { "field": "attack", "value": 85 } } }),
        ));
        assert_eq!(changed, json!([{ "kind": "Personal", "index": 2 }]));
        assert_eq!(
            json_of(call(&state, &policy, "history", json!({})))["undo"],
            json!(["Ivysaur: Angriff 4 → 85"])
        );
        // The egg move patch deserializes; the CPUE fixture's table is locked (unverified
        // place), so it is refused as read-only with the reason.
        let egg = dispatch(
            &state,
            &policy,
            "data_edit",
            json!({ "kind": "species", "index": 1, "patch": { "EggMoves": { "moves": [1, 2] } } }),
        );
        match egg {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.code, "read_only");
                assert!(
                    e.message.contains("für CPUE nicht geprüft"),
                    "{}",
                    e.message
                );
            }
            other => panic!("{other:?}"),
        }
        let refs = json_of(call(
            &state,
            &policy,
            "refs",
            json!({ "target": { "kind": "item", "id": 1 } }),
        ));
        assert_eq!(refs, json!([]));
        assert_eq!(
            json_of(call(&state, &policy, "item_table", Value::Null)),
            Value::Null
        );
    }

    #[test]
    fn checks_answer_findings_at_the_top_level_and_ignore_through_the_bridge() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        // The drafts of 3b.4b call `{scope: full}`; it means all groups.
        let report = json_of(call(
            &state,
            &policy,
            "run_checks",
            json!({ "scope": "full" }),
        ));
        assert_eq!(
            report["groups"],
            json!(["cheap", "per_script", "per_archive"])
        );
        assert_eq!(report["running"], false);
        let findings = report["findings"].as_array().unwrap();
        assert!(
            findings
                .iter()
                .all(|f| f["source"] == "check" && f["code"].is_string())
        );
        let split: Vec<&Value> = findings
            .iter()
            .filter(|f| f["code"] == "header.text-split")
            .collect();
        assert_eq!(split.len(), 1, "{report}");
        let split = split[0].clone();
        // Without arguments: all groups as well.
        let again = json_of(call(&state, &policy, "run_checks", Value::Null));
        assert_eq!(again["findings"], report["findings"]);

        // The finding goes back as it came, with its own fields.
        let answer = json_of(call(
            &state,
            &policy,
            "ignore_finding",
            json!({ "code": split["code"], "subject": split["subject"], "key": split["key"],
                    "fingerprint": split["fingerprint"] }),
        ));
        assert_eq!(answer["ignored"], json!([split]));
        assert!(
            !answer["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["code"] == "header.text-split")
        );
        assert!(fixture.dir.path().join("g4-checks.json").is_file());
        assert_eq!(
            json_of(call(&state, &policy, "checks_status", json!({}))),
            answer
        );
        let back = json_of(call(
            &state,
            &policy,
            "unignore_finding",
            json!({ "code": split["code"], "subject": split["subject"], "key": split["key"],
                    "fingerprint": split["fingerprint"] }),
        ));
        assert_eq!(back["ignored"], json!([]));
        assert_eq!(back["findings"], report["findings"]);
    }

    /// The 3b.4 scenarios send a finding as `run_checks` answered it (`${r.findings.0}`).
    #[test]
    fn fixes_take_the_finding_as_run_checks_answers_it() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let long = "Welcome to Twinleaf Town, where a fresh breeze always blows and the grass smells sweet in the morning sun.";
        call(
            &state,
            &policy,
            "edit_text_archive",
            json!({ "index": 433, "edit": { "Set": { "index": 1, "text": long } } }),
        );
        let wide = |report: &Value| -> Vec<Value> {
            report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["code"] == "text.too-wide")
                .cloned()
                .collect()
        };
        let report = json_of(call(&state, &policy, "run_checks", json!({})));
        let found = wide(&report);
        assert_eq!(found.len(), 1, "{report}");
        // The whole row, with label, args, fixes and the rest the backend does not read.
        let args = json!({ "finding": found[0], "fix": "wrap" });

        let plan = json_of(call(&state, &policy, "plan_fix", args.clone()));
        assert_eq!(plan["irreversible"], false);
        assert_eq!(
            plan["changes"][0]["asset"],
            json!({ "kind": "TextArchive", "index": 433 })
        );
        let result = json_of(call(&state, &policy, "apply_fix", args.clone()));
        assert_eq!(
            result,
            json!({ "changed": [{ "kind": "TextArchive", "index": 433 }], "remaining": false })
        );
        let after = json_of(call(&state, &policy, "run_checks", json!({})));
        assert!(wide(&after).is_empty(), "{after}");

        // Applied once: the finding is gone, a second time is refused.
        match dispatch(&state, &policy, "apply_fix", args) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.code, "finding_stale");
                assert_eq!(e.message, "finding is no longer current");
            }
            other => panic!("expected finding_stale, got {other:?}"),
        }
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn open_project_and_build_rom_are_confined_to_the_roots() {
        let (fixture, state) = open_synthetic();
        let policy = dev_policy_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().to_str().unwrap();
        for (name, args) in [
            ("open_project", json!({ "path": outside })),
            ("build_rom", json!({ "out": format!("{outside}/out.nds") })),
        ] {
            match dispatch(&state, &policy, name, args) {
                Err(DispatchError::Cmd(e)) => {
                    assert!(
                        e.message.starts_with("Pfad liegt außerhalb"),
                        "{}",
                        e.message
                    )
                }
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn bridge_refuses_paths_outside_the_import_root() {
        let (fixture, state) = open_synthetic();
        let policy = dev_policy_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().join("a.per");
        std::fs::write(&outside, [0u8; 2048]).unwrap();
        let outside = outside.to_str().unwrap();
        let outside_obj = elsewhere.path().join("a.obj");
        std::fs::write(&outside_obj, b"v 0 0 0\n").unwrap();
        let outside_obj = outside_obj.to_str().unwrap();
        for (name, args) in [
            (
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [outside] } }),
            ),
            (
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [outside_obj] }, "options": { "unit": 16 } }),
            ),
            (
                "import_map_parts",
                json!({ "map": 0, "source": { "files": [outside] }, "parts": ["permissions"] }),
            ),
            (
                "export_map_part",
                json!({ "map": 0, "part": "permissions", "path": outside }),
            ),
        ] {
            match dispatch(&state, &policy, name, args) {
                Err(DispatchError::Cmd(e)) => assert!(
                    e.message.starts_with("Pfad liegt außerhalb"),
                    "{name}: {}",
                    e.message
                ),
                other => panic!("{name}: {other:?}"),
            }
        }
        // Inside the root: inspected, imported (a no-op here) and exported.
        let inside = fixture.dir.path().join("b.per");
        std::fs::write(&inside, [0x80u8; 2048]).unwrap();
        let inside = inside.to_str().unwrap();
        let report = json_of(call(
            &state,
            &policy,
            "inspect_map_import",
            json!({ "map": 0, "source": { "files": [inside] } }),
        ));
        assert_eq!(report[0]["part"], "permissions");
        assert_eq!(report[0]["state"], json!({ "kind": "ok" }));
        let changed = json_of(call(
            &state,
            &policy,
            "import_map_parts",
            json!({ "map": 0, "source": { "files": [inside] }, "parts": ["permissions"] }),
        ));
        assert_eq!(changed, json!([{ "kind": "Map", "index": 0 }]));
        assert_eq!(
            json_of(call(&state, &policy, "history", json!({})))["undo"],
            json!(["map 0000: permissions imported"])
        );
    }

    /// A port nothing listens on (bound and let go again).
    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn live_play_is_unavailable_and_live_connect_is_not() {
        let (fixture, state) = open_synthetic();
        let policy = dev_policy_over(&fixture);
        match dispatch(
            &state,
            &policy,
            "live_play",
            json!({ "emulator": "e", "rom": "r" }),
        ) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.code, "error");
                assert_eq!(
                    e.message,
                    "Im Browser-Modus nicht verfügbar, g4emu benutzen"
                );
            }
            other => panic!("{other:?}"),
        }
        let status = json_of(call(
            &state,
            &policy,
            "live_connect",
            json!({ "port": free_port() }),
        ));
        assert_eq!(status["state"], "connecting");
        call(&state, &policy, "live_stop", json!({}));
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn run_rom_is_unavailable() {
        let (fixture, state) = open_synthetic();
        let policy = dev_policy_over(&fixture);
        match dispatch(
            &state,
            &policy,
            "run_rom",
            json!({ "emulator": "e", "rom": "r" }),
        ) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.code, "error");
                assert_eq!(
                    e.message,
                    "Im Browser-Modus nicht verfügbar, g4emu benutzen"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn user_policy_confines_build_and_open() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().to_str().unwrap();
        let refused =
            |name: &str, args: Value, starts: &str| match dispatch(&state, &policy, name, args) {
                Err(DispatchError::Cmd(e)) => {
                    assert!(e.message.starts_with(starts), "{name}: {}", e.message)
                }
                other => panic!("{name}: {other:?}"),
            };
        // Another folder than the project does not open, the project itself does.
        refused(
            "open_project",
            json!({ "path": outside }),
            "Nur das offene Projekt: ",
        );
        call(
            &state,
            &policy,
            "open_project",
            json!({ "path": fixture.dir.path().to_str().unwrap() }),
        );
        // Builds go to build/ (relative to the project), nowhere else.
        for out in [
            format!("{outside}/out.nds"),
            "out.nds".to_owned(),
            "build/../out.nds".to_owned(),
        ] {
            refused("build_rom", json!({ "out": out }), "Pfad liegt außerhalb");
        }
        // Map parts: reads from the project, writes under build/.
        let stray = elsewhere.path().join("a.per");
        std::fs::write(&stray, [0u8; 2048]).unwrap();
        let stray = stray.to_str().unwrap();
        refused(
            "inspect_map_import",
            json!({ "map": 0, "source": { "files": [stray] } }),
            "Pfad liegt außerhalb",
        );
        // A model or an image outside is refused like any part file.
        for name in ["a.obj", "a.png"] {
            let stray = elsewhere.path().join(name);
            std::fs::write(&stray, b"v 0 0 0\n").unwrap();
            refused(
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [stray.to_str().unwrap()] },
                        "options": { "unit": 16, "colors": 4 } }),
                "Pfad liegt außerhalb",
            );
        }
        refused(
            "export_map_part",
            json!({ "map": 0, "part": "permissions", "path": "out.per" }),
            "Pfad liegt außerhalb",
        );
        call(
            &state,
            &policy,
            "export_map_part",
            json!({ "map": 0, "part": "permissions", "path": "build/out.per" }),
        );
        assert!(fixture.dir.path().join("build/out.per").is_file());
        // The emulator has to be configured at start.
        refused(
            "run_rom",
            json!({ "emulator": "", "rom": "build/out.per" }),
            "Kein Emulator freigegeben",
        );
    }

    #[test]
    fn inspect_over_the_bridge_converts_an_obj() {
        let (fixture, state) = open_synthetic_for_models();
        let policy = policy_over(&fixture);
        let obj = fixture.dir.path().join("import").join(CUBE_OBJ);
        let report = json_of(call(
            &state,
            &policy,
            "inspect_map_import",
            json!({ "map": 0, "source": { "files": [obj.to_str().unwrap()] },
                    "options": { "unit": 16, "colors": 16 } }),
        ));
        assert_eq!(report[0]["part"], "model");
        assert_eq!(report[0]["state"], json!({ "kind": "ok" }));
        let preview = &report[0]["preview"];
        assert_eq!(preview["model"]["stats"]["quads"], 6);
        assert_eq!(preview["model"]["format"], "obj");
        assert_eq!(preview["model"]["budget"]["max"], 0xF000);
        assert_eq!(preview["model"]["budget"]["state"], "calm");
        assert_eq!(preview["textures"][0]["name"], "g4_imp_cube");
        assert_eq!(preview["textures"][0]["new"], true);
        assert_eq!(preview["style"], "outdoor");
        // Without options the defaults apply (16 colours).
        let report = json_of(call(
            &state,
            &policy,
            "inspect_map_import",
            json!({ "map": 0, "source": { "files": [obj.to_str().unwrap()] } }),
        ));
        assert_eq!(report[0]["preview"]["textures"][0]["colors"], 16);
    }

    #[test]
    fn options_out_of_range_over_the_bridge() {
        let (fixture, state) = open_synthetic_for_models();
        let policy = policy_over(&fixture);
        let obj = fixture.dir.path().join("import").join(CUBE_OBJ);
        let obj = obj.to_str().unwrap();
        for (name, args) in [
            (
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [obj] }, "options": { "unit": 0 } }),
            ),
            (
                "import_map_parts",
                json!({ "map": 0, "source": { "files": [obj] }, "parts": ["model"],
                        "options": { "colors": 8 } }),
            ),
        ] {
            match dispatch(&state, &policy, name, args) {
                Err(DispatchError::Cmd(e)) => assert_eq!(e.code, "error", "{name}: {e:?}"),
                other => panic!("{name}: {other:?}"),
            }
        }
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn lsp_is_off_in_the_bridge() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        match dispatch(&state, &policy, "lsp_start", json!({})) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.code, "tool_missing");
                assert_eq!(e.message, "rotom-lsp fehlt — Completion und Diagnosen aus");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            dispatch(&state, &policy, "lsp_send", json!({ "body": "{}" })),
            Err(DispatchError::Cmd(_))
        ));
        assert!(matches!(
            call(&state, &policy, "lsp_stop", json!({})),
            Reply::Json(Value::Null)
        ));
    }

    #[test]
    fn adopt_scripts_runs_without_progress() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let status = json_of(call(&state, &policy, "scripts_status", json!({})));
        assert_eq!(status["adopted"], false);
        let report = json_of(call(&state, &policy, "adopt_scripts", json!({})));
        assert_eq!(report["decompiled"], 2, "{report}");
        assert_eq!(report["skipped_empty"], 1, "{report}");
        assert_eq!(report["failed"], json!([]), "{report}");
        let status = json_of(call(&state, &policy, "scripts_status", json!({})));
        assert_eq!(status["adopted"], true);
        // The sources landed inside the project, which lies under the allowed root.
        assert!(fixture.dir.path().join("rotom.toml").is_file());
    }

    #[test]
    fn force_quit_is_a_no_op() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let reply = call(&state, &policy, "force_quit", json!({}));
        assert!(matches!(reply, Reply::Json(Value::Null)));
        assert!(!state.force_quit.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn unknown_command_is_unknown() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        match dispatch(&state, &policy, "no_such_command", json!({})) {
            Err(DispatchError::Unknown(name)) => assert_eq!(name, "no_such_command"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_arguments_name_the_command() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        // snake_case instead of the camelCase key, and a missing key.
        for (name, args) in [
            ("get_events", json!({ "event_file_id": 1 })),
            ("get_matrix", json!({})),
        ] {
            match dispatch(&state, &policy, name, args) {
                Err(DispatchError::Cmd(e)) => {
                    assert_eq!(e.code, "error");
                    assert!(
                        e.message
                            .starts_with(&format!("Ungültige Argumente für {name}: ")),
                        "{}",
                        e.message
                    );
                }
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[test]
    fn add_event_takes_an_optional_patch_and_label_in_the_same_undo_step() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        // The old keys alone still work.
        call(
            &state,
            &policy,
            "add_event",
            json!({ "eventFile": 0, "kind": "object", "x": 1, "z": 1 }),
        );
        call(
            &state,
            &policy,
            "add_event",
            json!({ "eventFile": 0, "kind": "object", "x": 2, "z": 1,
                    "patch": { "script": 3929, "trainer_type": 1, "sight": 3 }, "label": "Trainer Jens platziert" }),
        );
        let o = &get_events_impl(&state, 0).unwrap().objects[1];
        assert_eq!((o.x, o.script, o.trainer_type, o.sight), (2, 3929, 1, 3));
        assert_eq!(
            history_impl(&state).unwrap().undo,
            ["add event", "Trainer Jens platziert"]
        );
    }

    #[test]
    fn null_and_empty_object_both_mean_no_arguments() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        for args in [Value::Null, json!({})] {
            call(&state, &policy, "list_headers", args);
        }
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn inspect_rom_reads_a_rom_under_the_root() {
        let (tmp, rom) = g4_kernel::test_support::synthetic_rom();
        let roots = Roots::new(&[tmp.path().to_path_buf()]).unwrap();
        let state = AppState::default();
        let info = json_of(call(
            &state,
            &roots,
            "inspect_rom",
            json!({ "path": rom.to_str().unwrap() }),
        ));
        assert_eq!(info["game_code"], "CPUE", "{info}");
        assert_eq!(info["support"]["kind"], "untested", "{info}");
        // A ROM outside the roots is not read.
        let other = tempfile::tempdir().unwrap();
        let outside = other.path().join("x.nds");
        std::fs::copy(&rom, &outside).unwrap();
        match dispatch(
            &state,
            &roots,
            "inspect_rom",
            json!({ "path": outside.to_str().unwrap() }),
        ) {
            Err(DispatchError::Cmd(e)) => assert!(e.message.starts_with("Pfad liegt außerhalb")),
            other => panic!("{other:?}"),
        }
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn create_project_with_a_header_only_rom_fails_and_leaves_nothing() {
        let (tmp, rom) = g4_kernel::test_support::synthetic_rom();
        let roots = Roots::new(&[tmp.path().to_path_buf()]).unwrap();
        let state = AppState::default();
        let dest = tmp.path().join("neu");
        // ds-rom cannot unpack a ROM that is only a header.
        match dispatch(
            &state,
            &roots,
            "create_project",
            json!({ "rom": rom.to_str().unwrap(), "dest": dest.to_str().unwrap(),
                    "template": "data_only" }),
        ) {
            Err(DispatchError::Cmd(e)) => assert_eq!(e.code, "error"),
            other => panic!("{other:?}"),
        }
        assert!(!dest.exists());
        assert!(!tmp.path().join(".neu.g4-new").exists());
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn create_project_refuses_the_dspre_suffix() {
        let (tmp, rom) = g4_kernel::test_support::synthetic_rom();
        let roots = Roots::new(&[tmp.path().to_path_buf()]).unwrap();
        let state = AppState::default();
        let dest = tmp.path().join("Platin_DSPRE_contents");
        for name in ["create_project", "check_project_dest"] {
            let args = if name == "create_project" {
                json!({ "rom": rom.to_str().unwrap(), "dest": dest.to_str().unwrap(),
                        "template": "original" })
            } else {
                json!({ "dest": dest.to_str().unwrap(), "romSize": 1000 })
            };
            match dispatch(&state, &roots, name, args) {
                Err(DispatchError::Cmd(e)) => {
                    assert!(
                        e.message
                            .starts_with("Originale (_DSPRE_contents) sind gesperrt: "),
                        "{name}: {}",
                        e.message
                    );
                    assert!(e.message.contains("ohne die Endung"), "{}", e.message);
                }
                other => panic!("{name}: {other:?}"),
            }
        }
        assert!(!dest.exists());
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn default_project_parent_is_the_first_root_without_suffix() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let got = json_of(call(&state, &roots, "default_project_parent", json!({})));
        assert_eq!(
            got,
            json!({ "parent": fixture.dir.path().canonicalize().unwrap().to_str().unwrap(),
                    "dspre_suffix": false })
        );
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn check_project_dest_answers_for_a_destination_under_the_root() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let dest = fixture.dir.path().join("neu");
        let got = json_of(call(
            &state,
            &roots,
            "check_project_dest",
            json!({ "dest": dest.to_str().unwrap(), "romSize": 1000 }),
        ));
        assert_eq!(got["state"], "missing", "{got}");
        assert_eq!(got["needed_bytes"], 2500, "{got}");
        assert_eq!(got["ok"], true, "{got}");
    }

    /// Needs `G4_TEST_ROM` (a copy of a ROM, read only): creates a project from it under a
    /// root, then opens it like the app does.
    #[cfg(feature = "dev-bridge")]
    #[test]
    fn project_start_bridge_creates_and_opens() {
        let Some(nds) = std::env::var_os("G4_TEST_ROM").map(std::path::PathBuf::from) else {
            eprintln!("skipped: G4_TEST_ROM not set");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        let rom = tmp.path().join("copy.nds");
        std::fs::copy(&nds, &rom).unwrap();
        let code = String::from_utf8(std::fs::read(&rom).unwrap()[0x0C..0x10].to_vec()).unwrap();
        let roots = Roots::new(&[tmp.path().to_path_buf()]).unwrap();
        let state = AppState::default();
        let dest = tmp.path().join("neu");
        let created = json_of(call(
            &state,
            &roots,
            "create_project",
            json!({ "rom": rom.to_str().unwrap(), "dest": dest.to_str().unwrap(),
                    "template": "original" }),
        ));
        assert_eq!(created["game_code"], code, "{created}");
        assert_eq!(created["phases"].as_array().unwrap().len(), 4, "{created}");
        let root = created["root"].as_str().unwrap().to_owned();
        assert!(dest.join(".g4/base.json").is_file());
        // The ROM is remembered as the base ROM of its SHA-1.
        let sha1 = created["sha1"].as_str().unwrap();
        assert!(state.base_roms.settings().find(sha1).is_some());
        let opened = json_of(call(
            &state,
            &roots,
            "open_project",
            json!({ "path": root }),
        ));
        assert_eq!(opened["game_code"], code, "{opened}");
    }

    /// A synthetic project whose matrix 0 has a start town (header 1 "T01", the cell (1, 0)) next
    /// to a cell of header 0, with an unsaved edit on top: the plan works, the apply must not.
    #[cfg(feature = "dev-bridge")]
    #[test]
    fn apply_empty_region_refuses_unsaved_changes() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let mut names = Vec::new();
        for name in ["E00", "T01"] {
            let mut record = [0u8; 16];
            record[..name.len()].copy_from_slice(name.as_bytes());
            names.extend_from_slice(&record);
        }
        std::fs::write(
            fixture
                .dir
                .path()
                .join("files/fielddata/maptable/mapname.bin"),
            names,
        )
        .unwrap();
        let state = AppState::default();
        crate::commands::open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        adopt_scripts_impl(&state, &mut |_| {}).unwrap();
        {
            let mut guard = crate::commands::lock(&state).unwrap();
            let session = crate::commands::session(&mut guard).unwrap();
            session
                .store
                .edit_matrix(0, "town", |m| {
                    m.headers = Some(vec![0, 1]);
                    m.altitudes = Some(vec![0, 0]);
                    Ok(())
                })
                .unwrap();
        }
        let roots = roots_over(&fixture);
        let plan = json_of(call(&state, &roots, "plan_empty_region", json!({})));
        assert_eq!(plan["start_header"], 1, "{plan}");
        assert_eq!(plan["town_cells"], json!([[1, 0]]), "{plan}");
        assert_eq!(plan["already_empty"], false, "{plan}");
        let dirty_before = dirty_impl(&state).unwrap();
        assert!(!dirty_before.is_empty());
        match dispatch(&state, &roots, "apply_empty_region", json!({})) {
            Err(DispatchError::Cmd(e)) => assert_eq!(e.message, "save first"),
            other => panic!("{other:?}"),
        }
        assert_eq!(dirty_impl(&state).unwrap(), dirty_before);
        // Saved, it applies: the answer names what to load again, and the world is empty now.
        call(&state, &roots, "save", json!({}));
        let report = json_of(call(&state, &roots, "apply_empty_region", json!({})));
        assert_eq!(report["plan"], plan, "{report}");
        let changed = report["changed"].as_array().unwrap();
        assert!(
            changed.contains(&json!({ "kind": "Matrix", "index": 0 })),
            "{report}"
        );
        assert!(report["change"].is_u64(), "{report}");
        let again = json_of(call(&state, &roots, "plan_empty_region", json!({})));
        assert_eq!(again["already_empty"], true, "{again}");
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn the_empty_region_commands_need_an_open_project() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let roots = roots_over(&fixture);
        let state = AppState::default();
        for name in ["plan_empty_region", "apply_empty_region"] {
            match dispatch(&state, &roots, name, json!({})) {
                Err(DispatchError::Cmd(e)) => assert_eq!(e.message, "no project open", "{name}"),
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    /// Needs `G4_TEST_ROM` (a copy of a ROM, read only): creates a project with the empty-region
    /// template (the kernel adopts the scripts, applies the recipe and saves), opens it like the
    /// app does, checks it, and shows that the world is empty now: the plan says so and a second
    /// apply is refused.
    #[cfg(feature = "dev-bridge")]
    #[test]
    fn project_start_bridge_empty_region() {
        let Some(nds) = std::env::var_os("G4_TEST_ROM").map(std::path::PathBuf::from) else {
            eprintln!("skipped: G4_TEST_ROM not set");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        let rom = tmp.path().join("copy.nds");
        std::fs::copy(&nds, &rom).unwrap();
        let roots = Roots::new(&[tmp.path().to_path_buf()]).unwrap();
        let state = AppState::default();
        let dest = tmp.path().join("neu");
        let created = json_of(call(
            &state,
            &roots,
            "create_project",
            json!({ "rom": rom.to_str().unwrap(), "dest": dest.to_str().unwrap(),
                    "template": "empty_region" }),
        ));
        assert_eq!(created["template"], "empty_region", "{created}");
        let root = created["root"].as_str().unwrap().to_owned();
        json_of(call(
            &state,
            &roots,
            "open_project",
            json!({ "path": root }),
        ));
        let plan = json_of(call(&state, &roots, "plan_empty_region", json!({})));
        assert_eq!(plan["already_empty"], true, "{plan}");
        match dispatch(&state, &roots, "apply_empty_region", json!({})) {
            Err(DispatchError::Cmd(e)) => {
                assert_eq!(e.message, "the world is already empty");
            }
            other => panic!("{other:?}"),
        }
        let report = json_of(call(
            &state,
            &roots,
            "run_checks",
            json!({ "scope": "full" }),
        ));
        assert_eq!(
            report["groups"],
            json!(["cheap", "per_script", "per_archive"]),
            "{report}"
        );
        let errors: Vec<&Value> = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == "error")
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
    }

    // The project preview (phase 4.0 §3.3).

    /// A 10 × 10 PNG, grey.
    fn small_png() -> Vec<u8> {
        g4_kernel::data::icons::png(&[0x80; 10 * 10 * 4], 10, 10).unwrap()
    }

    fn base64_of(bytes: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// Width and height of the PNG `png` (base64) answers, from its IHDR chunk.
    fn png_size(png: &Value) -> (u32, u32) {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png.as_str().unwrap())
            .unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&bytes[12..16], b"IHDR");
        let be = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (be(16), be(20))
    }

    /// The standard map of [`open_synthetic_with_town`].
    fn town() -> Value {
        json!({ "header": 0, "map": 1, "areaDataId": 0 })
    }

    /// [`open_synthetic`] with a header layer in matrix 0 (as the kernel's test of
    /// `standard_map` writes it): header 0, Platinum's start town `T01`, has cell (1, 0) with
    /// map 1, so the project has a standard map ([`town`]).
    fn open_synthetic_with_town() -> (g4_kernel::test_support::Synthetic, AppState) {
        use g4_formats::matrix::Matrix;
        use g4_formats::narc::Narc;
        let fixture = g4_kernel::test_support::synthetic_project();
        let matrix = Matrix {
            name: "m".into(),
            width: 2,
            height: 1,
            headers: Some(vec![1, 0]),
            altitudes: None,
            maps: vec![2, 1],
        };
        std::fs::write(
            fixture
                .dir
                .path()
                .join("files/fielddata/mapmatrix/map_matrix.narc"),
            Narc {
                members: vec![matrix.write()],
            }
            .write(),
        )
        .unwrap();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        (fixture, state)
    }

    /// The refusal message of a call that must fail.
    fn refused_with(state: &AppState, policy: &dyn Policy, name: &str, args: Value) -> String {
        match dispatch(state, policy, name, args) {
            Err(DispatchError::Cmd(e)) => e.message,
            other => panic!("{name}: {other:?}"),
        }
    }

    #[test]
    fn preview_get_on_open_project_has_standard_map_field() {
        let (fixture, state) = open_synthetic_with_town();
        let policy = policy_over(&fixture);
        let got = json_of(call(&state, &policy, "project_preview_get", json!({})));
        assert_eq!((&got["source"], &got["png"]), (&Value::Null, &Value::Null));
        assert_eq!(got["standardMap"], town(), "{got}");
        // It is the kernel's answer.
        let project = g4_kernel::project::Project::open(fixture.dir.path()).unwrap();
        assert_eq!(
            got["standardMap"],
            serde_json::to_value(g4_kernel::preview::standard_map(&project)).unwrap()
        );
        // `api.preview.ts` sends `path: null`: the same.
        assert_eq!(
            json_of(call(
                &state,
                &policy,
                "project_preview_get",
                json!({ "path": null })
            )),
            got
        );
    }

    #[test]
    fn preview_without_path_needs_an_open_project() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let policy = policy_over(&fixture);
        let state = AppState::default();
        for (name, args) in [
            ("project_preview_get", json!({})),
            (
                "project_preview_set",
                json!({ "png": base64_of(&small_png()) }),
            ),
        ] {
            assert_eq!(
                refused_with(&state, &policy, name, args),
                "no project open",
                "{name}"
            );
        }
        assert!(!fixture.dir.path().join(".g4/preview.png").exists());
    }

    #[test]
    fn preview_set_png_then_get_returns_eigen() {
        let (fixture, state) = open_synthetic_with_town();
        let policy = policy_over(&fixture);
        let set = json_of(call(
            &state,
            &policy,
            "project_preview_set",
            json!({ "png": base64_of(&small_png()) }),
        ));
        assert_eq!(set["source"], "eigen", "{set}");
        assert_eq!(png_size(&set["png"]), (256, 192));
        assert_eq!(set["standardMap"], town());
        assert!(fixture.dir.path().join(".g4/preview.png").is_file());
        let got = json_of(call(&state, &policy, "project_preview_get", json!({})));
        assert_eq!(got, set);
        // What is on disk is what the answer carries.
        let on_disk = std::fs::read(fixture.dir.path().join(".g4/preview.png")).unwrap();
        assert_eq!(got["png"], base64_of(&on_disk));
    }

    #[test]
    fn preview_set_remove_falls_back_to_standard() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let png = base64_of(&small_png());
        let set = |args: Value| json_of(call(&state, &policy, "project_preview_set", args));
        assert_eq!(
            set(json!({ "slot": "standard", "png": png }))["source"],
            "standard"
        );
        assert!(
            fixture
                .dir
                .path()
                .join(".g4/cache/preview-standard.png")
                .is_file()
        );
        assert_eq!(set(json!({ "png": png }))["source"], "eigen");
        // Neither png nor file: the slot's picture goes, the standard one shows.
        let removed = set(json!({}));
        assert_eq!(removed["source"], "standard", "{removed}");
        assert_eq!(png_size(&removed["png"]), (256, 192));
        assert!(!fixture.dir.path().join(".g4/preview.png").exists());
        // Removing again is no error; without the standard one there is none.
        assert_eq!(set(json!({ "slot": "eigen" }))["source"], "standard");
        let none = set(json!({ "slot": "standard" }));
        assert_eq!(
            (&none["source"], &none["png"]),
            (&Value::Null, &Value::Null)
        );
    }

    #[test]
    fn preview_set_png_and_file_is_refused() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let file = fixture.dir.path().join("bild.png");
        std::fs::write(&file, small_png()).unwrap();
        let message = refused_with(
            &state,
            &policy,
            "project_preview_set",
            json!({ "png": base64_of(&small_png()), "file": file.to_str().unwrap() }),
        );
        assert_eq!(
            message,
            "png und file schließen sich aus; nur eins von beiden angeben"
        );
        // Text that is no base64 is no picture.
        let message = refused_with(
            &state,
            &policy,
            "project_preview_set",
            json!({ "png": "kein Bild!" }),
        );
        assert!(message.starts_with("preview.format"), "{message}");
        // Base64 longer than the kernel's 32 MiB file would take is refused before it is
        // decoded: this text is no base64 at all, yet the answer is the size.
        let long = "!".repeat((32 * 1024 * 1024usize).div_ceil(3) * 4 + 4);
        let message = refused_with(
            &state,
            &policy,
            "project_preview_set",
            json!({ "png": long }),
        );
        assert!(message.starts_with("preview.too_large"), "{message}");
        assert!(!fixture.dir.path().join(".g4/preview.png").exists());
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn preview_set_file_under_root() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let file = fixture.dir.path().join("bilder/titel.png");
        std::fs::create_dir(fixture.dir.path().join("bilder")).unwrap();
        std::fs::write(&file, small_png()).unwrap();
        let set = json_of(call(
            &state,
            &roots,
            "project_preview_set",
            json!({ "file": file.to_str().unwrap() }),
        ));
        assert_eq!(set["source"], "eigen", "{set}");
        assert_eq!(png_size(&set["png"]), (256, 192));
        assert!(fixture.dir.path().join(".g4/preview.png").is_file());
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn preview_set_file_outside_roots_is_refused() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let file = elsewhere.path().join("titel.png");
        std::fs::write(&file, small_png()).unwrap();
        let message = refused_with(
            &state,
            &roots,
            "project_preview_set",
            json!({ "file": file.to_str().unwrap() }),
        );
        assert!(message.starts_with("Pfad liegt außerhalb"), "{message}");
        assert!(!fixture.dir.path().join(".g4").exists());
    }

    #[cfg(feature = "dev-bridge")]
    #[test]
    fn preview_get_path_outside_roots_is_refused() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let other = elsewhere.path().join("projekt");
        std::fs::create_dir_all(other.join(".g4")).unwrap();
        let png = g4_kernel::preview::fit_preview(&small_png()).unwrap();
        std::fs::write(other.join(".g4/preview.png"), png).unwrap();
        // An existing folder and missing ones outside the roots (also below a missing
        // folder, or leaving the roots through `..` after a missing one); read and write.
        for path in [
            other.clone(),
            elsewhere.path().join("fehlt"),
            elsewhere.path().join("gruppe/fehlt"),
            fixture.dir.path().join("weg/../../x"),
        ] {
            let path = path.to_str().unwrap();
            for (name, args) in [
                ("project_preview_get", json!({ "path": path })),
                (
                    "project_preview_set",
                    json!({ "path": path, "png": base64_of(&small_png()) }),
                ),
                ("project_preview_set", json!({ "path": path })),
            ] {
                let message = refused_with(&state, &roots, name, args);
                assert!(
                    message.starts_with("Pfad liegt außerhalb"),
                    "{name}: {message}"
                );
            }
        }
        assert!(other.join(".g4/preview.png").is_file());
    }

    #[cfg(feature = "dev-bridge")]
    #[cfg(unix)] // symlinks
    #[test]
    fn preview_get_path_without_g4_is_empty() {
        let (fixture, state) = open_synthetic_with_town();
        let roots = roots_over(&fixture);
        let get = |path: &std::path::Path| {
            json_of(call(
                &state,
                &roots,
                "project_preview_get",
                json!({ "path": path.to_str().unwrap() }),
            ))
        };
        let empty = json!({ "source": null, "png": null, "standardMap": null });
        // A folder without .g4/ and a folder that is gone (a stale start list entry).
        let other = fixture.dir.path().join("anderes");
        std::fs::create_dir(&other).unwrap();
        assert_eq!(get(&other), empty);
        assert_eq!(get(&fixture.dir.path().join("weg")), empty);
        assert!(!other.join(".g4").exists());
        // Gone with the group folder it was in, or a link that leads nowhere (also one that
        // would lead out of the roots): no preview either, and nothing is created.
        assert_eq!(get(&fixture.dir.path().join("gruppe/projekt")), empty);
        let dangling = fixture.dir.path().join("verweis");
        std::os::unix::fs::symlink(fixture.dir.path().join("nirgends"), &dangling).unwrap();
        assert_eq!(get(&dangling), empty);
        let elsewhere = tempfile::tempdir().unwrap();
        let out = fixture.dir.path().join("hinaus");
        std::os::unix::fs::symlink(elsewhere.path().join("nirgends"), &out).unwrap();
        assert_eq!(get(&out), empty);
        assert_eq!(get(&out.join("projekt")), empty);
        // Writing into a folder that is gone is refused, and creates nothing.
        for gone in [
            fixture.dir.path().join("gruppe/projekt"),
            dangling.clone(),
            out.clone(),
        ] {
            let message = refused_with(
                &state,
                &roots,
                "project_preview_set",
                json!({ "path": gone.to_str().unwrap(), "png": base64_of(&small_png()) }),
            );
            assert!(message.starts_with("Kein Projektordner"), "{message}");
        }
        assert!(!fixture.dir.path().join("gruppe").exists());
        assert!(!fixture.dir.path().join("nirgends").exists());
        assert!(!elsewhere.path().join("nirgends").exists());
        // Written by path, read by path: another project than the open one has no standard
        // map.
        let set = json_of(call(
            &state,
            &roots,
            "project_preview_set",
            json!({ "path": other.to_str().unwrap(), "png": base64_of(&small_png()) }),
        ));
        assert_eq!(
            (&set["source"], &set["standardMap"]),
            (&json!("eigen"), &Value::Null)
        );
        assert_eq!(get(&other), set);
        assert!(!fixture.dir.path().join(".g4").exists());
        assert_eq!(get(&other)["standardMap"], Value::Null);
        // The open project's own folder, also through a link: as without a path, with the
        // standard map.
        let own = json_of(call(&state, &roots, "project_preview_get", json!({})));
        assert_eq!(own["standardMap"], town(), "{own}");
        assert_eq!(get(fixture.dir.path()), own);
        let link = fixture.dir.path().join("anderes/zurück");
        std::os::unix::fs::symlink(fixture.dir.path(), &link).unwrap();
        assert_eq!(get(&link), own);
    }

    #[test]
    fn bake_test_out_dir_outside_the_roots_is_refused() {
        let (fixture, state) = open_synthetic();
        let policy = policy_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let away = elsewhere.path().join("bake");
        for out_dir in [away.to_str().unwrap(), "bake", "../bake"] {
            match dispatch(
                &state,
                &policy,
                "bake_test",
                json!({ "map": 0, "block": [3, 4], "outDir": out_dir }),
            ) {
                Err(DispatchError::Cmd(e)) => assert!(
                    e.message.starts_with("Pfad liegt außerhalb"),
                    "{out_dir}: {}",
                    e.message
                ),
                other => panic!("{out_dir}: {other:?}"),
            }
        }
        assert!(!away.exists());
        assert!(!fixture.dir.path().join("bake").exists());
        assert!(history_impl(&state).unwrap().undo.is_empty());

        // `build/bake` as the g4emu scenarios pass it: relative to the project, `build/` made.
        crate::commands::bake::tests::prepare(fixture.dir.path());
        assert!(!fixture.dir.path().join("build").exists());
        let reply = json_of(call(
            &state,
            &policy,
            "bake_test",
            json!({ "map": 0, "outDir": "build/bake" }),
        ));
        let build = fixture
            .dir
            .path()
            .canonicalize()
            .unwrap()
            .join("build/bake");
        let model_file = build.join("bake-0000.nsbmd");
        assert_eq!(reply["model_file"], json!(model_file.to_str().unwrap()));
        assert_eq!(reply["pack"], json!(0));
        assert_eq!(reply["appended"], json!(["g4_bake_a", "g4_bake_b"]));
        assert!(model_file.is_file());
        assert!(build.join("maptex-0000.nsbtx").is_file());
    }
}
