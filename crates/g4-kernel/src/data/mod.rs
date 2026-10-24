//! Kernel side of the data editors (Pokémon, moves, items, trainers, encounters): the
//! read-only arm9 tables, the text banks per family and language, the form names
//! (`metadata/species_forms.yaml`, read by the name tables in `names`) and the reference
//! index (`refs`).

pub mod arm9;
pub mod names;
pub mod refs;
pub mod text_banks;

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
