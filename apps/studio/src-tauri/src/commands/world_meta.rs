//! The project file `g4-world.json` (names of world maps and rooms, rooms assigned by hand; see
//! `g4_kernel::world_meta`). `world_meta_get` reads it, `world_meta_set` writes it at once (no
//! undo: it is editor knowledge, not part of the ROM). Like the ignore file it is read
//! robustly (ruling R15): a broken file does not keep the project closed, `world_meta_get`
//! reports why, and `world_meta_set` refuses to overwrite it until it reads again. Its headers
//! are an id or a number in the file (a text project writes ids), numbers here.

use g4_kernel::world_meta::{self, WORLD_FILE, WorldMeta};
use serde::Serialize;
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// What `world_meta_get` answers: the meta (empty when the file is missing or broken) and why
/// the file could not be read, if it could not.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldMetaDto {
    pub meta: WorldMeta,
    pub broken: Option<String>,
}

pub fn world_meta_get_impl(state: &AppState) -> Result<WorldMetaDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let root = session.project.root.clone();
    let (meta, broken) = match session.ids() {
        Ok(ids) => world_meta::load(&root, ids),
        Err(e) => (WorldMeta::default(), Some(e.message)),
    };
    Ok(WorldMetaDto { meta, broken })
}

/// Writes `meta` to the world file. The file is read again first: a broken one is refused
/// (nothing is written), a file that reads now (repaired, or removed) is replaced.
pub fn world_meta_set_impl(state: &AppState, meta: WorldMeta) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let root = session.project.root.clone();
    let root = &root;
    let ids = session.ids()?;
    if let (_, Some(why)) = world_meta::load(root, ids) {
        return Err(format!(
            "{WORLD_FILE} ist beschädigt: {why}. Repariere oder lösche die Datei."
        )
        .into());
    }
    world_meta::save(root, &meta, ids)?;
    Ok(())
}

#[tauri::command]
pub fn world_meta_get(state: State<AppState>) -> Result<WorldMetaDto, CmdError> {
    world_meta_get_impl(&state)
}

#[tauri::command]
pub fn world_meta_set(state: State<AppState>, meta: WorldMeta) -> Result<(), CmdError> {
    world_meta_set_impl(&state, meta)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::open_synthetic;

    fn named(room: u16, name: &str) -> WorldMeta {
        let mut meta = WorldMeta::default();
        meta.room_names.insert(room, name.to_owned());
        meta
    }

    #[test]
    fn world_meta_is_empty_without_a_file_and_set_writes_it_to_the_project_root() {
        let (fixture, state) = open_synthetic();
        let answer = world_meta_get_impl(&state).unwrap();
        assert_eq!(answer.meta, WorldMeta::default());
        assert_eq!(answer.broken, None);
        world_meta_set_impl(&state, named(1, "Haus")).unwrap();
        assert!(fixture.dir.path().join(WORLD_FILE).is_file());
        assert_eq!(world_meta_get_impl(&state).unwrap().meta, named(1, "Haus"));
    }

    #[test]
    fn a_broken_world_file_is_reported_and_set_refuses_to_overwrite_it_until_it_reads_again() {
        let (fixture, state) = open_synthetic();
        let path = fixture.dir.path().join(WORLD_FILE);
        std::fs::write(&path, "{ kaputt").unwrap();
        let answer = world_meta_get_impl(&state).unwrap();
        assert_eq!(answer.meta, WorldMeta::default());
        assert!(answer.broken.is_some());
        let err = world_meta_set_impl(&state, named(1, "Haus")).unwrap_err();
        assert!(
            err.message.starts_with("g4-world.json ist beschädigt: "),
            "{err:?}"
        );
        assert!(
            err.message.ends_with("Repariere oder lösche die Datei."),
            "{err:?}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ kaputt");
        // Repaired by hand: the next write goes through.
        std::fs::write(&path, "{}").unwrap();
        assert_eq!(world_meta_get_impl(&state).unwrap().broken, None);
        world_meta_set_impl(&state, named(1, "Haus")).unwrap();
        assert_eq!(world_meta_get_impl(&state).unwrap().meta, named(1, "Haus"));
    }

    #[test]
    fn a_world_file_with_map_ids_reads_and_a_dspre_folder_writes_numbers() {
        let (fixture, state) = open_synthetic();
        let path = fixture.dir.path().join(WORLD_FILE);
        // Header 1 of the synthetic project is `T02` in `mapname.bin`.
        std::fs::write(&path, r#"{"roomNames": {"MAP_T02": "Haus"}}"#).unwrap();
        assert_eq!(world_meta_get_impl(&state).unwrap().meta, named(1, "Haus"));
        world_meta_set_impl(&state, named(1, "Laden")).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"1\": \"Laden\""), "{text}");

        // An unknown id: the file reads broken and is not overwritten.
        std::fs::write(&path, r#"{"roomNames": {"MAP_X": "Haus"}}"#).unwrap();
        let answer = world_meta_get_impl(&state).unwrap();
        assert_eq!(
            answer.broken.as_deref(),
            Some("unbekannte ID MAP_X in roomNames")
        );
        assert!(world_meta_set_impl(&state, named(1, "Laden")).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"roomNames": {"MAP_X": "Haus"}}"#
        );
    }

    #[test]
    fn the_commands_need_an_open_project() {
        let state = AppState::default();
        assert!(world_meta_get_impl(&state).is_err());
        assert!(world_meta_set_impl(&state, WorldMeta::default()).is_err());
    }
}
