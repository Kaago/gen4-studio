mod commands;
mod state;

use commands::{get_map, get_map_model_glb, get_matrix, list_headers, open_project};
use state::AppState;

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
            get_map_model_glb
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
