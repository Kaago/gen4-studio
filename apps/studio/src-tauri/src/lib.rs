mod commands;
mod errors;
mod state;

use commands::{
    accept_disk, build_rom, dirty, force_quit, get_map, get_map_model_glb, get_matrix,
    list_headers, open_project, redo, reload_asset, run_rom, save, set_permission, undo,
};
use state::{AppState, exit_is_blocked};
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_project,
            list_headers,
            get_matrix,
            get_map,
            get_map_model_glb,
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

/// Cmd+Q (and any other exit request while a window is open) with unsaved edits asks the
/// frontend first. Never blocks when the exit follows `force_quit`, when no window is left
/// (the close guard already asked, and nothing could show the prompt), when no project is
/// open, or when the state lock is poisoned.
fn exit_request_needs_confirmation(app: &tauri::AppHandle) -> bool {
    if app.webview_windows().is_empty() {
        return false;
    }
    let state = app.state::<AppState>();
    let dirty = match state.session.lock() {
        Ok(guard) => guard
            .as_ref()
            .map_or(0, |session| session.store.dirty().len()),
        Err(_) => 0,
    };
    exit_is_blocked(dirty, state.force_quit.load(Ordering::SeqCst))
}
