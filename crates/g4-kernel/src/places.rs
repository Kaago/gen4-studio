//! Places: the in-game location names a map header selects with its `location_name`
//! index (phase 3f.2). The names live in one bank of the text archive; this module knows
//! which one and reads it from the store, so names the user added and has not saved yet
//! are already there.

use uxie::text_bank::game_strings::TextBankIds;

use crate::error::KernelError;
use crate::project::Project;
use crate::store::Store;

/// The text archive member holding the location names of `project`'s game and language
/// (the bank [`Project::location_names`] reads from disk).
pub fn location_bank(project: &Project) -> u16 {
    TextBankIds::for_game(project.uxie_family(), project.uxie_language()).locations
}

/// The location names as the store holds them, unsaved edits included, indexed by the
/// `location_name` of a map header.
pub fn location_names(store: &mut Store) -> Result<Vec<String>, KernelError> {
    let bank = location_bank(store.project());
    Ok(store
        .text_archive(bank)?
        .messages
        .iter()
        .map(|m| m.text.clone())
        .collect())
}
