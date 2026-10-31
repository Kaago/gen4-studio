//! The recipes as commands on the open project (phase 3d.2): list them, plan one without
//! writing (the draft the app shows dashed), apply one as one undo step. The recipes
//! themselves are the kernel's ([`g4_kernel::recipes`]); `recipe_apply` answers the whole
//! outcome (report, created members, change, changed assets, next steps), the wizard commands
//! keep answering the report alone.

use g4_kernel::recipes::{self, Draft, OutcomeDto, RecipeCall, RecipeId, RecipeInfo};
use serde_json::Value;
use tauri::Manager;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The call of `recipe` with `input` as `api.ts` and the dispatcher send them (`{recipe,
/// input}`); absent or `null` input reads as `{}`.
pub fn recipe_call(recipe: RecipeId, input: Option<Value>) -> RecipeCall {
    RecipeCall {
        recipe,
        input: input
            .filter(|v| !v.is_null())
            .unwrap_or_else(|| Value::Object(Default::default())),
    }
}

/// Every recipe, with its title and whether applying it always cannot be undone completely.
pub fn recipe_list_impl() -> Vec<RecipeInfo> {
    recipes::list()
}

/// What applying `call` would do to the store as it is now (unsaved edits included); edits
/// nothing.
pub fn recipe_plan_impl(state: &AppState, call: RecipeCall) -> Result<Draft, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(recipes::plan(&mut session.store, &call)?)
}

/// Applies `call`: its new members first (not undoable), then one undo step. Members may have
/// been created and scripts or archives written past the store also when a later step failed,
/// so the reference index and the checks' last report are refreshed either way.
pub fn recipe_apply_impl(state: &AppState, call: RecipeCall) -> Result<OutcomeDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let outcome = recipes::apply(&mut session.store, &call);
    session.after_write();
    session.files_written(&[]);
    Ok(outcome?)
}

#[tauri::command]
pub fn recipe_list() -> Vec<RecipeInfo> {
    recipe_list_impl()
}

/// Reads maps, matrices and event files that are not loaded yet, so it runs on a blocking
/// worker.
#[tauri::command]
pub async fn recipe_plan(
    app: tauri::AppHandle,
    recipe: RecipeId,
    input: Option<Value>,
) -> Result<Draft, CmdError> {
    let call = recipe_call(recipe, input);
    tauri::async_runtime::spawn_blocking(move || recipe_plan_impl(&app.state::<AppState>(), call))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}

/// Writes members and may compile scripts (seconds), so it runs on a blocking worker.
#[tauri::command]
pub async fn recipe_apply(
    app: tauri::AppHandle,
    recipe: RecipeId,
    input: Option<Value>,
) -> Result<OutcomeDto, CmdError> {
    let call = recipe_call(recipe, input);
    tauri::async_runtime::spawn_blocking(move || recipe_apply_impl(&app.state::<AppState>(), call))
        .await
        .map_err(|e| CmdError::from(e.to_string()))?
}
