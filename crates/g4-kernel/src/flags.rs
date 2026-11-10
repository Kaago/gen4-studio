//! Flags and variables: the pret names of the command databases, the ranges of the numbers and
//! where the save block holds them in RAM, as metadata of the three games the studio knows.
//!
//! A flag is a number `1..` (0 means "no flag"), a variable a number from `0x4000` (below
//! `0x4000` there is none; `0x8000` and up are local to a script). The names come from the
//! bundled command databases ([`names`]), the ranges from `metadata/flag_ranges.yaml`
//! ([`ranges`]), the layout of the save block from `metadata/save_layout.yaml`
//! ([`save_layout`]).

use serde::{Deserialize, Serialize};

use crate::textproject::symbols::SymbolKind;

mod entities;
mod names;
mod pool;
mod ranges;
mod save_layout;

pub use entities::{
    FlagEntity, NameReport, NameRequest, TakenBy, entity_rel, has_own_name, id_required,
    name_flag, name_flag_files, not_in_save_text, nothing_to_change, own_names, read_entity,
    taken_text,
};
pub use names::{NameTable, name_is_unused, names};
pub use pool::{
    Pool, free_flag, free_var, pool, pool_empty, pool_empty_text, pool_empty_var, reserved,
    reserved_by_empty_region,
};
pub use ranges::{
    Area, Range, Ranges, area_of, hidden_item_flag, hidden_items_base, in_save, pool_range, ranges,
    trainer_flag,
};
pub use save_layout::{SaveBlock, save_block};

/// Whether a number is a flag or a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    Flag,
    Var,
}

impl FlagKind {
    /// The flag kind of a [`SymbolKind`]: `Flag`, `Var`, `None` for every other kind.
    pub fn from_symbol(symbol: SymbolKind) -> Option<FlagKind> {
        match symbol {
            SymbolKind::Flag => Some(FlagKind::Flag),
            SymbolKind::Var => Some(FlagKind::Var),
            _ => None,
        }
    }

    /// The kind of id the number has: [`SymbolKind::Flag`] or [`SymbolKind::Var`].
    pub fn symbol(self) -> SymbolKind {
        match self {
            FlagKind::Flag => SymbolKind::Flag,
            FlagKind::Var => SymbolKind::Var,
        }
    }

    /// `FLAG`, `VAR`.
    pub fn prefix(self) -> &'static str {
        match self {
            FlagKind::Flag => "FLAG",
            FlagKind::Var => "VAR",
        }
    }

    /// `flags`, `vars`: the folder `data/<folder>/` and the register `data/ids/<folder>.json`.
    pub fn folder(self) -> &'static str {
        match self {
            FlagKind::Flag => "flags",
            FlagKind::Var => "vars",
        }
    }

    /// `Flag`, `Variable`: the word for the user interface.
    pub fn word(self) -> &'static str {
        match self {
            FlagKind::Flag => "Flag",
            FlagKind::Var => "Variable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_names_its_symbol_prefix_folder_and_word() {
        assert_eq!(FlagKind::Flag.symbol(), SymbolKind::Flag);
        assert_eq!(FlagKind::Var.symbol(), SymbolKind::Var);
        assert_eq!(FlagKind::Flag.prefix(), "FLAG");
        assert_eq!(FlagKind::Var.prefix(), "VAR");
        assert_eq!(FlagKind::Flag.folder(), "flags");
        assert_eq!(FlagKind::Var.folder(), "vars");
        assert_eq!(FlagKind::Flag.word(), "Flag");
        assert_eq!(FlagKind::Var.word(), "Variable");
        assert_eq!(FlagKind::from_symbol(SymbolKind::Flag), Some(FlagKind::Flag));
        assert_eq!(FlagKind::from_symbol(SymbolKind::Var), Some(FlagKind::Var));
        assert_eq!(FlagKind::from_symbol(SymbolKind::Trainer), None);
    }
}
