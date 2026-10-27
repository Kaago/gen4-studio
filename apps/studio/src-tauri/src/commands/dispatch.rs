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
use g4_kernel::map_import::{Part, Source};
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
    ($reply:ident, $name:literal, $args:expr, { $($field:ident : $ty:ty),* $(,)? }, |$a:ident| $call:expr) => {{
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            $($field: $ty),*
        }
        let $a: Args = parse($name, $args)?;
        $reply($call)
    }};
}

/// Runs the command `name` against `state`. The paths of `open_project`, `build_rom`,
/// `run_rom` and the map part commands are checked by `policy` first; `force_quit` is a
/// no-op and the language server is off.
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
            { index: u16, edit: TextEdit },
            |a| edit_text_archive_impl(state, a.index, a.edit)
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
            { map: u16, source: Source },
            |a| {
                let source = confine_source(policy, a.source)?;
                inspect_map_import_impl(state, a.map, &source)
            }
        ),
        "import_map_parts" => run!(
            json, "import_map_parts", args,
            { map: u16, source: Source, parts: Vec<Part>, keep_buildings: Option<bool> },
            |a| {
                let source = confine_source(policy, a.source)?;
                import_map_parts_impl(state, a.map, &source, &a.parts, a.keep_buildings.unwrap_or(false))
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
            { rom: String, dest: String, template: g4_kernel::start::Template },
            |a| {
                let rom = policy.read_path(&a.rom)?;
                let dest = policy.check_new_project(&a.dest)?;
                // No live progress over HTTP: the answer carries the time of each phase.
                create_project_impl(&path_text(&rom)?, &path_text(&dest)?, a.template, &mut |_| {})
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
        open_synthetic,
    };
    use crate::state::AppState;
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
                json!({ "index": 433, "edit": { "Set": { "index": 0, "text": "x" } } }),
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
                json!({ "map": 0, "source": { "files": [per_in] } }),
            ),
            (
                "import_map_parts",
                json!({ "map": 0, "source": { "map": 2 }, "parts": ["permissions"], "keepBuildings": false }),
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
                json!({ "rom": per_in, "dest": format!("{dir}/neu"), "template": "original" }),
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
        for (name, args) in [
            (
                "inspect_map_import",
                json!({ "map": 0, "source": { "files": [outside] } }),
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
}
