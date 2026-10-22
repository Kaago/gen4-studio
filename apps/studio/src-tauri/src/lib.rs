mod commands;
mod errors;
mod menu;
mod state;

use commands::{
    accept_disk, add_building, add_event, apply_header_patch, build_rom, create_area,
    create_event_file, create_matrix, create_room, delete_building, delete_event, dirty,
    duplicate_event_file, duplicate_map, extend_area, force_quit, get_area_data, get_building_glb,
    get_events, get_header, get_map, get_map_model_glb, get_matrix, get_object_sprite,
    header_patch_status, list_headers, list_matrices, list_warps, map_model_texture_status,
    member_counts, open_project, redo, reload_asset, rename_matrix, repair_header_patch,
    resize_matrix, run_rom, save, set_area_data, set_header, set_matrix_cell, set_matrix_sections,
    set_permission, tile_behaviors, undo, update_building, update_event,
};
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
            add_event,
            delete_event,
            get_map_model_glb,
            get_object_sprite,
            map_model_texture_status,
            tile_behaviors,
            set_permission,
            undo,
            redo,
            dirty,
            save,
            build_rom,
            run_rom,
            reload_asset,
            accept_disk,
            force_quit
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event
                && exit_request_needs_confirmation(app)
            {
                api.prevent_exit();
                // A failed emit only means the prompt cannot be shown; the app stays open.
                let _ = app.emit("confirm-quit", ());
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
/// asked, and nothing could show the prompt), when no project is open, or when the state lock
/// is poisoned.
fn exit_request_needs_confirmation(app: &tauri::AppHandle) -> bool {
    let state = app.state::<AppState>();
    let dirty = match state.session.lock() {
        Ok(guard) => guard
            .as_ref()
            .map_or(0, |session| session.store.dirty().len()),
        Err(_) => 0,
    };
    let window_open = !app.webview_windows().is_empty();
    quit_action(window_open, dirty, state.force_quit.load(Ordering::SeqCst)) == QuitAction::Confirm
}
