#[cfg(feature = "dev-bridge")]
pub mod bridge;
mod commands;
mod errors;
mod lsp;
mod menu;
mod state;

use commands::{
    accept_baseline, accept_disk, add_building, add_event, add_script, adopt_scripts, apply_fix,
    apply_header_patch, build_rom, checks_status, create_area, create_encounter_file,
    create_event_file, create_matrix, create_room, create_text_archive, create_trainer, data_edit,
    data_get, data_list, data_raw, delete_building, delete_event, dirty, duplicate_event_file,
    duplicate_map, edit_text_archive, export_map_part, extend_area, force_quit, get_area_data,
    get_building_glb, get_events, get_header, get_item_icon, get_map, get_map_model_glb,
    get_matrix, get_mon_icon, get_object_sprite, get_text_archive, header_patch_status, history,
    ignore_finding, import_map_parts, inspect_map_import, item_table, list_headers, list_matrices,
    list_script_files, list_warps, machine_table, map_model_texture_status, member_counts,
    name_tables, new_script_file, open_project, plan_fix, read_script, redo, refs, reload_asset,
    rename_matrix, repair_header_patch, resize_matrix, resolve_script, run_checks, run_rom, save,
    save_level_script, save_script, scripts_status, search_scripts, search_text, set_area_data,
    set_check_enabled, set_header, set_matrix_cell, set_matrix_sections, set_permission,
    set_trainer_texts, split_encounter_file, tile_behaviors, undo, unignore_finding,
    update_building, update_event, validate_text, world_meta_get, world_meta_set,
};
use lsp::{lsp_send, lsp_start, lsp_stop};
use state::{AppState, QuitAction, quit_action};
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .menu(menu::app_menu)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == menu::QUIT_ID {
                request_quit(app);
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            list_headers,
            get_header,
            set_header,
            get_area_data,
            set_area_data,
            member_counts,
            get_matrix,
            set_matrix_cell,
            set_matrix_sections,
            resize_matrix,
            rename_matrix,
            create_matrix,
            list_matrices,
            list_warps,
            duplicate_map,
            create_event_file,
            duplicate_event_file,
            create_area,
            extend_area,
            create_room,
            header_patch_status,
            apply_header_patch,
            repair_header_patch,
            get_map,
            get_events,
            get_building_glb,
            add_building,
            update_building,
            delete_building,
            update_event,
            get_text_archive,
            edit_text_archive,
            create_text_archive,
            search_text,
            validate_text,
            scripts_status,
            adopt_scripts,
            list_script_files,
            read_script,
            save_script,
            save_level_script,
            new_script_file,
            add_script,
            resolve_script,
            search_scripts,
            lsp_start,
            lsp_send,
            lsp_stop,
            add_event,
            delete_event,
            get_map_model_glb,
            get_object_sprite,
            map_model_texture_status,
            tile_behaviors,
            set_permission,
            undo,
            redo,
            history,
            dirty,
            save,
            build_rom,
            run_rom,
            reload_asset,
            accept_disk,
            force_quit,
            data_list,
            data_get,
            data_edit,
            data_raw,
            name_tables,
            item_table,
            machine_table,
            refs,
            get_item_icon,
            get_mon_icon,
            create_trainer,
            set_trainer_texts,
            create_encounter_file,
            split_encounter_file,
            inspect_map_import,
            import_map_parts,
            export_map_part,
            run_checks,
            checks_status,
            ignore_finding,
            unignore_finding,
            accept_baseline,
            set_check_enabled,
            plan_fix,
            apply_fix,
            world_meta_get,
            world_meta_set
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                RunEvent::ExitRequested { api, .. } if exit_request_needs_confirmation(app) => {
                    api.prevent_exit();
                    // A failed emit only means the prompt cannot be shown; the app stays open.
                    let _ = app.emit("confirm-quit", ());
                }
                // The exit goes ahead (also the one after `force_quit`): end rotom-lsp. `Exit`
                // covers an exit that skipped the request.
                RunEvent::ExitRequested { .. } | RunEvent::Exit => lsp::stop_on_exit(app),
                _ => {}
            }
        });
}

/// Our Quit menu item (Cmd+Q): asks first when edits are unsaved, otherwise quits. Quitting
/// goes through `app.exit`, so the exit request below sees it again (and lets it pass).
fn request_quit(app: &tauri::AppHandle) {
    if exit_request_needs_confirmation(app) {
        // A failed emit only means the prompt cannot be shown; the app stays open.
        let _ = app.emit("confirm-quit", ());
    } else {
        app.exit(0);
    }
}

/// Any exit request while a window is open with unsaved edits asks the frontend first. Never
/// blocks when the exit follows `force_quit`, when no window is left (the close guard already
/// asked, and nothing could show the prompt) or when no project is open. A state lock a panic
/// poisoned is recovered, so its unsaved edits still count.
fn exit_request_needs_confirmation(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    let dirty = state
        .lock_session()
        .as_ref()
        .map_or(0, |session| session.store.dirty().len());
    let window_open = !app.webview_windows().is_empty();
    quit_action(window_open, dirty, state.force_quit.load(Ordering::SeqCst)) == QuitAction::Confirm
}
