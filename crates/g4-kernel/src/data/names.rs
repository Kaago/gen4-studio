//! Names and descriptions of species, moves, items, abilities, types, trainers and trainer
//! classes, read from the text archives of the project (`metadata/text_banks.yaml` says which
//! archive holds which bank). They are read through [`Store::text_archive`], so an unsaved
//! rename shows at once and the loader rule applies (`text::TextSync`: the JSON, or the
//! member when only it changed since the last recorded sync). The names of the music pieces
//! are no text bank: [`sound_names`] reads them from the bundled command database.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use g4_formats::game::{Family, Language};
use serde::Deserialize;

use crate::data::text_banks::text_banks_ref;
use crate::error::KernelError;
use crate::store::Store;

/// A name or text bank: one text archive per game family, the message index being the id of
/// the species, move, item, trainer, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bank {
    Species,
    Moves,
    MoveDescriptions,
    Items,
    ItemDescriptions,
    Abilities,
    Types,
    TrainerNames,
    TrainerClasses,
    BattleTexts,
    Forms,
}

impl Bank {
    /// The key of the bank in `text_banks.yaml`.
    fn key(self) -> &'static str {
        match self {
            Bank::Species => "species",
            Bank::Moves => "moves",
            Bank::MoveDescriptions => "move_descriptions",
            Bank::Items => "items",
            Bank::ItemDescriptions => "item_descriptions",
            Bank::Abilities => "abilities",
            Bank::Types => "types",
            Bank::TrainerNames => "trainer_names",
            Bank::TrainerClasses => "trainer_classes",
            Bank::BattleTexts => "battle_texts",
            Bank::Forms => "forms",
        }
    }
}

/// The text archive of `bank` in a game of `family` and `language`, or `None` when the
/// metadata has no number for it (Diamond/Pearl battle texts, a Japanese bank that was not
/// looked up: there is no Western fallback).
pub fn bank_index(family: Family, language: Language, bank: Bank) -> Option<u16> {
    text_banks_ref(family, language).get(bank.key())
}

/// Wrapper chatot puts around a trainer name: the message is `{TRAINER_NAME:Kai}`.
pub mod trainer_name {
    const OPEN: &str = "{TRAINER_NAME:";
    const CLOSE: &str = "}";

    /// The name inside the wrapper; a message without wrapper is returned as it is.
    pub fn unwrap(s: &str) -> String {
        s.strip_prefix(OPEN)
            .and_then(|rest| rest.strip_suffix(CLOSE))
            .unwrap_or(s)
            .to_owned()
    }

    /// `name` inside the wrapper, the form the text archive stores. `unwrap(wrap(n)) == n`
    /// for every `n`.
    pub fn wrap(s: &str) -> String {
        format!("{OPEN}{s}{CLOSE}")
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FormEntry {
    // Part of the file for readers of it; the name tables need the name only.
    #[allow(dead_code)]
    base: u16,
    name: String,
}

/// The form names of `metadata/species_forms.yaml`, by personal member.
fn builtin_forms() -> BTreeMap<u16, String> {
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    serde_yaml::from_str::<BTreeMap<u16, FormEntry>>(include_str!(
        "../../metadata/species_forms.yaml"
    ))
    .unwrap_or_default()
    .into_iter()
    .map(|(member, form)| (member, form.name))
    .collect()
}

/// One bank's messages as read, with what they were read from.
struct Cached {
    archive: u16,
    generation: u64,
    texts: Vec<String>,
}

/// The names of one project, cached per bank. A cache entry is read again when the text
/// archive's [`Store::text_generation`] changed (edit, undo, redo, reload, creation), so the
/// tables must be used with one store only.
pub struct NameTables {
    forms: BTreeMap<u16, String>,
    cache: HashMap<Bank, Cached>,
}

impl Default for NameTables {
    fn default() -> Self {
        NameTables::new()
    }
}

impl NameTables {
    /// Name tables with the form names of `species_forms.yaml`.
    pub fn new() -> NameTables {
        NameTables::with_forms(builtin_forms())
    }

    /// Name tables with `forms` (personal member to display name) in place of the form
    /// names of the metadata.
    pub fn with_forms(forms: BTreeMap<u16, String>) -> NameTables {
        NameTables {
            forms,
            cache: HashMap::new(),
        }
    }

    /// The messages of `bank` in message order (the index is the species, move, item, ...
    /// id); empty when the game has no such bank. An archive that does not decode is an error.
    pub fn names(&mut self, store: &mut Store, bank: Bank) -> Result<&[String], KernelError> {
        let project = store.project();
        let Some(archive) = bank_index(project.family, project.language, bank) else {
            return Ok(&[]);
        };
        let generation = store.text_generation(archive);
        let fresh = self
            .cache
            .get(&bank)
            .is_some_and(|c| c.archive == archive && c.generation == generation);
        if !fresh {
            let texts = store
                .text_archive(archive)?
                .messages
                .iter()
                .map(|m| m.text.clone())
                .collect();
            self.cache.insert(
                bank,
                Cached {
                    archive,
                    generation,
                    texts,
                },
            );
        }
        Ok(self.cache.get(&bank).map_or(&[], |c| c.texts.as_slice()))
    }

    /// Message `index` of `bank`; `None` when it does not exist or the bank cannot be read.
    /// An archive that does not decode is swallowed here (the name functions then show their
    /// fallback); [`NameTables::names`] reports that error.
    fn message(&mut self, store: &mut Store, bank: Bank, index: u16) -> Option<String> {
        self.names(store, bank)
            .ok()?
            .get(usize::from(index))
            .cloned()
    }

    /// Message `index` of `bank`, or `"<what> 0520"` when there is none (the UI translates).
    fn name_or(&mut self, store: &mut Store, bank: Bank, index: u16, what: &str) -> String {
        self.message(store, bank, index)
            .unwrap_or_else(|| format!("{what} {index:04}"))
    }

    /// Name of personal member `index`: the form name of members 496 to 507, else the
    /// species name; `"species 0520"` when neither exists.
    pub fn species_name(&mut self, store: &mut Store, index: u16) -> String {
        if let Some(name) = self.forms.get(&index) {
            return name.clone();
        }
        self.name_or(store, Bank::Species, index, "species")
    }

    pub fn move_name(&mut self, store: &mut Store, index: u16) -> String {
        self.name_or(store, Bank::Moves, index, "move")
    }

    /// Name of the item with id `index` (the item names are indexed by item id).
    pub fn item_name(&mut self, store: &mut Store, index: u16) -> String {
        self.name_or(store, Bank::Items, index, "item")
    }

    pub fn class_name(&mut self, store: &mut Store, index: u16) -> String {
        self.name_or(store, Bank::TrainerClasses, index, "class")
    }

    /// Name of trainer `index` without the `{TRAINER_NAME:...}` wrapper.
    pub fn trainer_name(&mut self, store: &mut Store, index: u16) -> String {
        match self.message(store, Bank::TrainerNames, index) {
            Some(name) => trainer_name::unwrap(&name),
            None => format!("trainer {index:04}"),
        }
    }

    /// Message `index` of `bank` (a description bank, but any bank reads); `None` when it
    /// does not exist or the archive does not decode (see [`NameTables::names`] for the error).
    pub fn description(&mut self, store: &mut Store, bank: Bank, index: u16) -> Option<String> {
        self.message(store, bank, index)
    }
}

/// One entry of `sounds` in the command database.
#[derive(Debug, Deserialize)]
struct RawSound {
    name: String,
    #[serde(default)]
    used_in: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSounds {
    #[serde(default)]
    sounds: HashMap<u16, RawSound>,
}

/// One music piece: id, constant name, where the game plays it.
pub type SoundName = (u16, String, Option<String>);

/// The music pieces of `family` from `sounds` of the bundled command database
/// (`scripts::scrcmd_database`), sorted by id: id, constant name (`SEQ_TOWN01`) and where the
/// game plays it when the database says so. Empty when the database does not parse (a test
/// parses each).
pub fn sound_names(family: Family) -> Vec<SoundName> {
    static TABLES: [OnceLock<Vec<SoundName>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    TABLES[slot]
        .get_or_init(|| {
            let json = crate::scripts::scrcmd_database(family).1;
            let mut sounds: Vec<_> = serde_json::from_str::<RawSounds>(json)
                .map(|db| db.sounds)
                .unwrap_or_default()
                .into_iter()
                .map(|(id, s)| (id, s.name, s.used_in))
                .collect();
            sounds.sort_by_key(|s| s.0);
            sounds
        })
        .clone()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sound_names_come_from_the_command_database_sorted_by_id() {
        let pt = sound_names(Family::Pt);
        assert_eq!(pt.len(), 1013);
        assert!(pt.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(pt[0], (1, "SEQ_PV001".to_owned(), None));
        let dummy = pt.iter().find(|s| s.0 == 1000).unwrap();
        assert_eq!(dummy.1, "SEQ_DUMMY");
        assert_eq!(
            dummy.2.as_deref(),
            Some("Beta version of Rt. 206 (Plays in mystery zone)")
        );
        let hgss = sound_names(Family::Hgss);
        assert_eq!(hgss.len(), 1372);
        assert_eq!(hgss.last().unwrap().0, 2378);
        assert_eq!(sound_names(Family::Dp).len(), 828);
    }
}
