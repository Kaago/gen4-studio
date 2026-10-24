//! Names as German UI text (undo labels, reference labels): the name from the text archive,
//! or a German fallback where the kernel's name tables fall back to English ("species 0520"
//! becomes "Art 0520").

use g4_kernel::data::names::NameTables;
use g4_kernel::store::Store;

/// `name` unless it is empty or the kernel's fallback `"<english> NNNN"`, else
/// `"<german> NNNN"`.
fn or_german(name: String, english: &str, german: &str, index: u16) -> String {
    if name.trim().is_empty() || name == format!("{english} {index:04}") {
        format!("{german} {index:04}")
    } else {
        name
    }
}

/// Personal member `index`: species or form name, else "Art 0520".
pub fn species(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.species_name(store, index);
    or_german(name, "species", "Art", index)
}

/// Move `index`: its name, else "Attacke 0469".
pub fn move_name(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.move_name(store, index);
    or_german(name, "move", "Attacke", index)
}

/// Item `index`: with the arm9 item table an item id ("Trank", else "Gegenstand 0099"),
/// without it a data member ("Daten 0017").
pub fn item(names: &mut NameTables, store: &mut Store, index: u16, by_id: bool) -> String {
    if !by_id {
        return format!("Daten {index:04}");
    }
    let name = names.item_name(store, index);
    or_german(name, "item", "Gegenstand", index)
}

/// Trainer `index`: "Trainer 0005 Kai", or "Trainer 0005" without a name.
pub fn trainer(names: &mut NameTables, store: &mut Store, index: u16) -> String {
    let name = names.trainer_name(store, index);
    if name.trim().is_empty() || name == format!("trainer {index:04}") {
        format!("Trainer {index:04}")
    } else {
        format!("Trainer {index:04} {name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_fallbacks_and_empty_names_become_german() {
        assert_eq!(
            or_german("species 0520".into(), "species", "Art", 520),
            "Art 0520"
        );
        assert_eq!(
            or_german(" ".into(), "move", "Attacke", 469),
            "Attacke 0469"
        );
        assert_eq!(
            or_german("Staravia".into(), "species", "Art", 397),
            "Staravia"
        );
        // Another index's fallback is a name like any other.
        assert_eq!(
            or_german("species 0001".into(), "species", "Art", 2),
            "species 0001"
        );
    }
}
