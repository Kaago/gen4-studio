//! "Einstellungen › Agenten", the status bar menu and the question dialog: the commands of
//! the agent hub. Only the app's UI calls them; for an agent they are unavailable.

use std::path::PathBuf;

use tauri::State;

use crate::errors::CmdError;
use crate::mcp::hub::{AgentPoll, AgentSettingsDto, AgentStatus, GrantView, setup_line};
use crate::mcp::settings::{GrantLevel, ProjectKey};
use crate::state::AppState;

/// The root of the open project, `None` without one.
fn open_root(state: &AppState) -> Option<PathBuf> {
    state
        .lock_session()
        .as_ref()
        .map(|session| session.project.root.clone())
}

pub fn agent_status_impl(state: &AppState) -> Result<AgentStatus, CmdError> {
    Ok(state.agents.status())
}

pub fn agent_poll_impl(state: &AppState, since: u64) -> Result<AgentPoll, CmdError> {
    Ok(state.agents.poll(since))
}

/// The settings with the setup line and the grant of the open project (a project whose
/// folder cannot be resolved shows its path without a grant).
pub fn agent_settings_impl(state: &AppState) -> Result<AgentSettingsDto, CmdError> {
    let settings = state.agents.settings();
    let open_project = open_root(state).map(|root| match ProjectKey::of(&root) {
        Ok(key) => GrantView {
            level: settings.grant_for(&key),
            path: key.path,
        },
        Err(_) => GrantView {
            path: root.display().to_string(),
            level: None,
        },
    });
    Ok(AgentSettingsDto {
        settings,
        setup_line: setup_line(),
        open_project,
    })
}

pub fn agent_set_enabled_impl(state: &AppState, on: bool) -> Result<(), CmdError> {
    Ok(state.agents.set_enabled(on)?)
}

pub fn agent_set_expert_impl(state: &AppState, on: bool) -> Result<(), CmdError> {
    Ok(state.agents.set_expert(on)?)
}

/// Sets the grant of the open project.
pub fn agent_set_grant_impl(state: &AppState, level: GrantLevel) -> Result<(), CmdError> {
    let root = open_root(state).ok_or_else(|| CmdError::from("Kein Projekt offen".to_owned()))?;
    let key = ProjectKey::of(&root)?;
    Ok(state.agents.set_grant(key, level)?)
}

pub fn agent_remove_grant_impl(state: &AppState, path: &str) -> Result<(), CmdError> {
    Ok(state.agents.remove_grant(path)?)
}

pub fn agent_answer_impl(state: &AppState, question: u64, answer: &str) -> Result<(), CmdError> {
    Ok(state.agents.answer(question, answer)?)
}

pub fn agent_disconnect_impl(state: &AppState) -> Result<(), CmdError> {
    state.agents.disconnect_all();
    Ok(())
}

pub fn agent_set_read_only_impl(state: &AppState, on: bool) -> Result<(), CmdError> {
    state.agents.set_read_only(on);
    Ok(())
}

#[tauri::command]
pub fn agent_status(state: State<AppState>) -> Result<AgentStatus, CmdError> {
    agent_status_impl(&state)
}

#[tauri::command]
pub fn agent_poll(state: State<AppState>, since: u64) -> Result<AgentPoll, CmdError> {
    agent_poll_impl(&state, since)
}

#[tauri::command]
pub fn agent_settings(state: State<AppState>) -> Result<AgentSettingsDto, CmdError> {
    agent_settings_impl(&state)
}

#[tauri::command]
pub fn agent_set_enabled(state: State<AppState>, on: bool) -> Result<(), CmdError> {
    agent_set_enabled_impl(&state, on)
}

#[tauri::command]
pub fn agent_set_expert(state: State<AppState>, on: bool) -> Result<(), CmdError> {
    agent_set_expert_impl(&state, on)
}

#[tauri::command]
pub fn agent_set_grant(state: State<AppState>, level: GrantLevel) -> Result<(), CmdError> {
    agent_set_grant_impl(&state, level)
}

#[tauri::command]
pub fn agent_remove_grant(state: State<AppState>, path: String) -> Result<(), CmdError> {
    agent_remove_grant_impl(&state, &path)
}

#[tauri::command]
pub fn agent_answer(state: State<AppState>, question: u64, answer: String) -> Result<(), CmdError> {
    agent_answer_impl(&state, question, &answer)
}

#[tauri::command]
pub fn agent_disconnect(state: State<AppState>) -> Result<(), CmdError> {
    agent_disconnect_impl(&state)
}

#[tauri::command]
pub fn agent_set_read_only(state: State<AppState>, on: bool) -> Result<(), CmdError> {
    agent_set_read_only_impl(&state, on)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::open_synthetic;

    #[test]
    fn grant_needs_an_open_project() {
        let state = AppState::default();
        let e = agent_set_grant_impl(&state, GrantLevel::Read).unwrap_err();
        assert_eq!(e.message, "Kein Projekt offen");
        assert_eq!(agent_settings_impl(&state).unwrap().open_project, None);
    }

    #[test]
    fn settings_show_the_grant_of_the_open_project() {
        let (fixture, state) = open_synthetic();
        let path = std::fs::canonicalize(fixture.dir.path())
            .unwrap()
            .display()
            .to_string();
        let dto = agent_settings_impl(&state).unwrap();
        assert_eq!(
            dto.open_project,
            Some(GrantView {
                path: path.clone(),
                level: None
            })
        );
        assert!(dto.setup_line.starts_with("claude mcp add gen4-studio -- "));
        agent_set_grant_impl(&state, GrantLevel::Write).unwrap();
        let dto = agent_settings_impl(&state).unwrap();
        assert_eq!(dto.open_project.unwrap().level, Some(GrantLevel::Write));
        agent_remove_grant_impl(&state, &path).unwrap();
        assert!(
            agent_settings_impl(&state)
                .unwrap()
                .settings
                .grants
                .is_empty()
        );
    }
}
