//! Kernel side of the data editors (Pokémon, moves, items, trainers, encounters): the
//! read-only arm9 tables and the metadata the editors name things with
//! (`metadata/text_banks.yaml`, `metadata/species_forms.yaml`).

pub mod arm9;

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

    const BANKS: [&str; 11] = [
        "species",
        "moves",
        "move_descriptions",
        "items",
        "item_descriptions",
        "abilities",
        "types",
        "trainer_names",
        "trainer_classes",
        "battle_texts",
        "forms",
    ];

    #[test]
    fn text_banks_yaml_names_every_bank_of_the_checked_families() {
        let all: BTreeMap<String, BTreeMap<String, serde_yaml::Value>> =
            serde_yaml::from_str(include_str!("../../metadata/text_banks.yaml")).unwrap();
        assert_eq!(all.keys().collect::<Vec<_>>(), ["dp", "hgss", "pt"]);
        let bank = |family: &str, name: &str| all[family].get(name).and_then(|v| v.as_u64());
        for family in ["pt", "hgss"] {
            assert_eq!(all[family].len(), BANKS.len(), "{family}");
            for name in BANKS {
                assert!(bank(family, name).is_some(), "{family} {name}");
            }
        }
        assert_eq!(bank("pt", "item_descriptions"), Some(391));
        assert_eq!(bank("hgss", "move_descriptions"), Some(749));
        assert_eq!(bank("hgss", "forms"), Some(802));
        // Diamond/Pearl is unverified and has no battle texts or forms.
        assert_eq!(all["dp"]["unverified"].as_bool(), Some(true));
        assert_eq!(bank("dp", "species"), Some(362));
        assert_eq!(bank("dp", "battle_texts"), None);
        assert_eq!(bank("dp", "forms"), None);
        for (name, value) in &all["dp"] {
            assert!(
                name == "unverified" || BANKS.contains(&name.as_str()),
                "dp {name}"
            );
            assert!(
                name == "unverified" || value.as_u64().is_some(),
                "dp {name}"
            );
        }
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
