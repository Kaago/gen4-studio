//! The empty-region template as commands on the open project: look at what it would do, then
//! do it. `create_project` with the template `empty_region` applies it to a new project; these
//! commands apply it to an open one (all unsaved work saved first).

use g4_kernel::templates::{self, RegionPlan, RegionReport};
use tauri::Manager;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// What the recipe would do to the world as the store has it now (unsaved edits included);
/// edits nothing.
pub fn plan_empty_region_impl(state: &AppState) -> Result<RegionPlan, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(templates::plan_empty_region(&mut session.store)?)
}

/// Applies the recipe: the new members (border map, level script, start script, text) are
/// written at once, the rest is one undoable step. Refused with unsaved changes, without
/// adopted scripts and on a world that is empty already. The answer's `changed` lists every
/// asset the app has to load again.
pub fn apply_empty_region_impl(state: &AppState) -> Result<RegionReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = templates::apply_empty_region(&mut session.store);
    session.after_write();
    // The border map, the level script and the start script are written past the store
    // (also when a later step failed), so the checks' last report is of an older state.
    session.files_written(&[]);
    Ok(report?)
}

/// Reads maps, matrix 0 and event files that are not loaded yet, so it runs on a blocking
/// worker.
#[tauri::command]
pub async fn plan_empty_region(app: tauri::AppHandle) -> Result<RegionPlan, CmdError> {
    tauri::async_runtime::spawn_blocking(move || plan_empty_region_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// Writes members and compiles scripts (seconds), so it runs on a blocking worker.
#[tauri::command]
pub async fn apply_empty_region(app: tauri::AppHandle) -> Result<RegionReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || apply_empty_region_impl(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}
