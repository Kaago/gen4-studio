//! Kernel side of the data editors (Pokémon, moves, items, trainers, encounters): the
//! read-only arm9 tables, the text banks per family and language, the form names
//! (`metadata/species_forms.yaml`, read by the name tables in `names`) and the reference
//! index (`refs`), the item and menu icons as PNG (`icons`), the Pokémon and trainer battle
//! sprites as PNG (`battle_sprites`), the game rules the encounter and trainer editors show
//! (`encounter_rules`, `trainer_rules`), creating trainers and encounter files
//! (`trainer_create`, `encounter_create`), the tables inside overlays (`overlay_tables`) and
//! where the egg move table lives (`egg_moves`).

pub mod arm9;
pub mod battle_sprites;
pub mod egg_moves;
pub mod encounter_create;
pub mod encounter_rules;
pub mod icons;
pub mod names;
pub mod overlay_tables;
pub mod refs;
pub mod text_banks;
pub mod trainer_create;
pub mod trainer_rules;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Form {
        base: u16,
        name: String,
    }

    #[test]
    fn species_forms_yaml_names_personal_members_496_to_507() {
        let forms: BTreeMap<u16, Form> =
            serde_yaml::from_str(include_str!("../../metadata/species_forms.yaml")).unwrap();
        assert_eq!(
            forms.keys().copied().collect::<Vec<_>>(),
            (496..=507).collect::<Vec<_>>()
        );
        assert_eq!(forms[&496].base, 386);
        assert_eq!(forms[&496].name, "Deoxys, Angriffsform");
        assert_eq!(forms[&500].name, "Burmadame, Lumpenumhang");
        assert_eq!(forms[&503].name, "Hitze-Rotom");
        assert!(forms.range(503..=507).all(|(_, f)| f.base == 479));
    }
}
