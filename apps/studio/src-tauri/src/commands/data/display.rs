//! Names as UI text (undo labels, reference labels): the name from the text archive, or a
//! fallback from the catalog where the kernel's name tables fall back to their own English
//! ("species 0520" becomes "Species 0520", in German "Art 0520").

use g4_kernel::data::names::NameTables;
use g4_kernel::store::Store;

/// `name` unless it is empty or the kernel's fallback `"<english> NNNN"`, else the catalog
/// text `key` (`%{index}`: NNNN).
fn or_fallback(name: String, english: &str, key: &str, index: u16) -> String {
    if name.trim().is_empty() || name == format!("{english} {index:04}") {
        t!(key, index = index : {:04}).into_owned()
    } else {
        name
    }
}

/// Personal member `index`: species or form name, else "Species 0520".
pub fn species(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.species_name(store, index);
    or_fallback(name, "species", "commands.data.species_fallback", index)
}

/// Move `index`: its name, else "Move 0469".
pub fn move_name(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.move_name(store, index);
    or_fallback(name, "move", "commands.data.move_fallback", index)
}

/// Item `index`: with the arm9 item table an item id ("Potion", else "Item 0099"), without it
/// a data member ("Data 0017").
pub fn item(names: &mut NameTables, store: &mut Store, index: u16, by_id: bool) -> String {
    if !by_id {
        return t!("commands.data.item_data", index = index : {:04}).into_owned();
    }
    let name = names.item_name(store, index);
    or_fallback(name, "item", "commands.data.item_fallback", index)
}

/// Trainer `index`: "Trainer 0005 Kai", or "Trainer 0005" without a name (the same in both
/// languages).
pub fn trainer(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.trainer_name(store, index);
    if name.trim().is_empty() || name == format!("trainer {index:04}") {
        t!("commands.data.trainer_fallback", index = index : {:04}).into_owned()
    } else {
        t!("commands.data.trainer_named", index = index : {:04}, name = name).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_fallbacks_and_empty_names_become_catalog_names() {
        let species = "commands.data.species_fallback";
        assert_eq!(
            or_fallback("species 0520".into(), "species", species, 520),
            "Species 0520"
        );
        assert_eq!(
            or_fallback(" ".into(), "move", "commands.data.move_fallback", 469),
            "Move 0469"
        );
        assert_eq!(
            or_fallback("Staravia".into(), "species", species, 397),
            "Staravia"
        );
        // Another index's fallback is a name like any other.
        assert_eq!(
            or_fallback("species 0001".into(), "species", species, 2),
            "species 0001"
        );
    }
}
