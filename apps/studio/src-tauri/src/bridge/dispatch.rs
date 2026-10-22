//! The bridge's command table: every Tauri command by name, over its `*_impl` function.
//!
//! Arguments arrive as one JSON object keyed like Tauri's `invoke` does (the camelCase of the
//! Rust parameter names), so `api.ts` and the bridge speak the same dialect.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::roots::Roots;
use crate::commands::*;
use crate::errors::CmdError;
use crate::state::AppState;
use g4_kernel::project::AssetKind;
use g4_kernel::store::MapHeader;

/// What a command answers: JSON, or raw bytes for the GLB commands.
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
    "add_event",
    "delete_event",
    "get_map_model_glb",
    "get_object_sprite",
    "map_model_texture_status",
    "tile_behaviors",
    "set_permission",
    "undo",
    "redo",
    "dirty",
    "save",
    "build_rom",
    "run_rom",
    "reload_asset",
    "accept_disk",
    "force_quit",
];

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

/// Runs the command `name` against `state`. `open_project` and `build_rom` are confined to
/// `roots` first; `run_rom` and `force_quit` are not available in the browser.
pub fn dispatch(
    state: &AppState,
    roots: &Roots,
    name: &str,
    args: Value,
) -> Result<Reply, DispatchError> {
    match name {
        "open_project" => run!(json, "open_project", args, { path: String }, |a| {
            let project = roots.check_project(&a.path)?;
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
        "add_event" => run!(
            json, "add_event", args,
            { event_file: u16, kind: EventKind, x: i32, z: i32 },
            |a| add_event_impl(state, a.event_file, a.kind, a.x, a.z)
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
        "dirty" => run!(json, "dirty", args, {}, |_a| dirty_impl(state)),
        "save" => run!(json, "save", args, {}, |_a| save_impl(state)),
        "build_rom" => run!(json, "build_rom", args, { out: String }, |a| {
            let out = roots.check_output(&a.out)?;
            let out = out
                .to_str()
                .ok_or_else(|| CmdError::from(format!("Pfad ist kein gültiger Text: {}", a.out)))?;
            build_rom_impl(state, out)
        }),
        "run_rom" => run!(
            json, "run_rom", args,
            { emulator: String, rom: String },
            |a| {
                // The arguments are checked like any other command's, then refused.
                let _ = (a.emulator, a.rom);
                Err::<u32, _>(CmdError::from("Im Browser-Modus nicht verfügbar, g4emu benutzen"))
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
        "force_quit" => Ok(Reply::Json(Value::Null)),
        _ => Err(DispatchError::Unknown(name.to_owned())),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::{
        dirty_impl, get_header_impl, get_matrix_impl, list_headers_impl, open_synthetic,
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

    fn roots_over(fixture: &g4_kernel::test_support::Synthetic) -> Roots {
        Roots::new(&[fixture.dir.path().to_path_buf()]).unwrap()
    }

    fn call(state: &AppState, roots: &Roots, name: &str, args: Value) -> Reply {
        dispatch(state, roots, name, args).unwrap_or_else(|e| panic!("{name}: {e:?}"))
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
        let roots = roots_over(&fixture);
        let dir = fixture.dir.path().to_str().unwrap().to_owned();
        let out = fixture
            .dir
            .path()
            .join("out.nds")
            .to_str()
            .unwrap()
            .to_owned();
        let header = call(&state, &roots, "get_header", json!({ "id": 0 }));
        let header = json_of(header);
        let area = json_of(call(&state, &roots, "get_area_data", json!({ "id": 0 })));

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
                    "with_headers": false, "header": null,
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
                    "header": { "id": 0, "area_data_id": null, "event_file": "Keep" },
                    "entrance": null,
                } }),
            ),
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
            (
                "add_event",
                json!({ "eventFile": 1, "kind": "warp", "x": 1, "z": 1 }),
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
            ("dirty", json!({})),
            ("save", json!({})),
            ("build_rom", json!({ "out": out })),
            ("run_rom", json!({ "emulator": "emu", "rom": "rom.nds" })),
            ("reload_asset", json!({ "kind": "Event", "index": 0 })),
            ("accept_disk", json!({ "kind": "Event", "index": 0 })),
            ("force_quit", json!({})),
        ];

        let tested: BTreeSet<&str> = cases.iter().map(|(name, _)| *name).collect();
        let table: BTreeSet<&str> = COMMANDS.iter().copied().collect();
        assert_eq!(cases.len(), tested.len(), "a command is tested twice");
        assert_eq!(tested, table, "every command needs a case");

        for (name, args) in cases {
            match dispatch(&state, &roots, name, args) {
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
    fn reads_match_the_direct_impl_call() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        fn direct<T: serde::Serialize>(v: T) -> Value {
            serde_json::to_value(v).unwrap()
        }
        assert_eq!(
            json_of(call(&state, &roots, "list_headers", json!({}))),
            direct(list_headers_impl(&state).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &roots, "get_header", json!({ "id": 0 }))),
            direct(get_header_impl(&state, 0).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &roots, "get_matrix", json!({ "index": 0 }))),
            direct(get_matrix_impl(&state, 0).unwrap())
        );
        assert_eq!(
            json_of(call(&state, &roots, "dirty", Value::Null)),
            direct(dirty_impl(&state).unwrap())
        );
    }

    #[test]
    fn glb_commands_reply_with_bytes_or_tool_missing() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        for (name, args) in [
            ("get_map_model_glb", json!({ "index": 0, "areaDataId": 0 })),
            ("get_building_glb", json!({ "modelId": 0, "areaDataId": 0 })),
        ] {
            match dispatch(&state, &roots, name, args) {
                Ok(Reply::Bytes(_)) => {}
                Err(DispatchError::Cmd(e)) if e.code == "tool_missing" => {}
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[test]
    fn open_project_and_build_rom_are_confined_to_the_roots() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().to_str().unwrap();
        for (name, args) in [
            ("open_project", json!({ "path": outside })),
            ("build_rom", json!({ "out": format!("{outside}/out.nds") })),
        ] {
            match dispatch(&state, &roots, name, args) {
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

    #[test]
    fn run_rom_is_unavailable() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        match dispatch(
            &state,
            &roots,
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
    fn force_quit_is_a_no_op() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        let reply = call(&state, &roots, "force_quit", json!({}));
        assert!(matches!(reply, Reply::Json(Value::Null)));
        assert!(!state.force_quit.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn unknown_command_is_unknown() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        match dispatch(&state, &roots, "no_such_command", json!({})) {
            Err(DispatchError::Unknown(name)) => assert_eq!(name, "no_such_command"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_arguments_name_the_command() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        // snake_case instead of the camelCase key, and a missing key.
        for (name, args) in [
            ("get_events", json!({ "event_file_id": 1 })),
            ("get_matrix", json!({})),
        ] {
            match dispatch(&state, &roots, name, args) {
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
    fn null_and_empty_object_both_mean_no_arguments() {
        let (fixture, state) = open_synthetic();
        let roots = roots_over(&fixture);
        for args in [Value::Null, json!({})] {
            call(&state, &roots, "list_headers", args);
        }
    }
}
