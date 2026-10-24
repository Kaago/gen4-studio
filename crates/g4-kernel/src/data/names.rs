//! Names and descriptions of species, moves, items, abilities, types, trainers and trainer
//! classes, read from the text archives of the project (`metadata/text_banks.yaml` says which
//! archive holds which bank). They are read through [`Store::text_archive`], so an unsaved
//! rename shows at once and the loader rule (newer member or JSON wins) applies.

use std::collections::{BTreeMap, HashMap};

use g4_formats::game::{Family, Language};
use serde::Deserialize;

use crate::data::text_banks::text_banks;
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
    text_banks(family, language).get(bank.key())
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
    /// does not exist.
    pub fn description(&mut self, store: &mut Store, bank: Bank, index: u16) -> Option<String> {
        self.message(store, bank, index)
    }
}
