//! Which text archive holds which name or text bank (`metadata/text_banks.yaml`), per game
//! family and language. Western ROMs share one set per family; Japanese and Korean ROMs have
//! their own (unverified) sets, which never fall back to the Western numbers.

use std::collections::{BTreeMap, HashMap};

use g4_formats::game::{Family, Language};
use serde::Deserialize;

const TEXT_BANKS_YAML: &str = include_str!("../../metadata/text_banks.yaml");

/// The bank numbers of one family and language, keyed by bank name (`species`, `moves`,
/// `move_descriptions`, `items`, `item_descriptions`, `abilities`, `types`, `trainer_names`,
/// `trainer_classes`, `battle_texts`, `forms`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct TextBanks {
    /// Not checked against a project (uxie or DSPRE values).
    #[serde(default)]
    pub unverified: bool,
    #[serde(flatten)]
    pub banks: BTreeMap<String, u16>,
}

impl TextBanks {
    /// Text archive of bank `name`, or `None` when it is unknown for this game.
    pub fn get(&self, name: &str) -> Option<u16> {
        self.banks.get(name).copied()
    }
}

#[derive(Debug, Deserialize)]
struct FamilyBanks {
    #[serde(default)]
    by_language: HashMap<String, TextBanks>,
    #[serde(flatten)]
    western: TextBanks,
}

fn family_key(family: Family) -> &'static str {
    match family {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    }
}

/// `by_language` key of the languages with their own bank numbers.
fn language_key(language: Language) -> Option<&'static str> {
    match language {
        Language::Japanese => Some("ja"),
        Language::Korean => Some("ko"),
        Language::English
        | Language::French
        | Language::German
        | Language::Italian
        | Language::Spanish => None,
    }
}

/// The text banks of `family` and `language`; empty when the metadata has none for them.
pub fn text_banks(family: Family, language: Language) -> TextBanks {
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    let mut all: HashMap<String, FamilyBanks> =
        serde_yaml::from_str(TEXT_BANKS_YAML).unwrap_or_default();
    let Some(mut family) = all.remove(family_key(family)) else {
        return TextBanks::default();
    };
    match language_key(language) {
        Some(key) => family.by_language.remove(key).unwrap_or_default(),
        None => family.western,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    const WESTERN: [Language; 5] = [
        Language::English,
        Language::French,
        Language::German,
        Language::Italian,
        Language::Spanish,
    ];

    #[test]
    fn text_banks_yaml_parses_into_known_banks_only() {
        let all: HashMap<String, FamilyBanks> = serde_yaml::from_str(TEXT_BANKS_YAML).unwrap();
        let mut families: Vec<_> = all.keys().map(String::as_str).collect();
        families.sort();
        assert_eq!(families, ["dp", "hgss", "pt"]);
        for (family, banks) in &all {
            let mut langs: Vec<_> = banks.by_language.keys().map(String::as_str).collect();
            langs.sort();
            assert_eq!(langs, ["ja", "ko"], "{family}");
            for set in std::iter::once(&banks.western).chain(banks.by_language.values()) {
                for name in set.banks.keys() {
                    assert!(
                        BANKS.contains(&name.as_str()),
                        "{family}: unknown bank {name}"
                    );
                }
            }
            assert!(banks.by_language.values().all(|s| s.unverified), "{family}");
        }
    }

    #[test]
    fn western_platinum_and_hgss_have_every_checked_bank() {
        for language in WESTERN {
            for family in [Family::Pt, Family::Hgss] {
                let banks = text_banks(family, language);
                assert!(!banks.unverified, "{family:?} {language:?}");
                for name in BANKS {
                    assert!(banks.get(name).is_some(), "{family:?} {language:?} {name}");
                }
            }
            let pt = text_banks(Family::Pt, language);
            assert_eq!(
                (
                    pt.get("species"),
                    pt.get("item_descriptions"),
                    pt.get("forms")
                ),
                (Some(412), Some(391), Some(697))
            );
            let hgss = text_banks(Family::Hgss, language);
            assert_eq!(
                (hgss.get("move_descriptions"), hgss.get("battle_texts")),
                (Some(749), Some(728))
            );
        }
    }

    #[test]
    fn japanese_and_korean_resolve_their_own_unverified_banks() {
        let ja = text_banks(Family::Hgss, Language::Japanese);
        assert!(ja.unverified);
        assert_eq!((ja.get("species"), ja.get("moves")), (Some(232), Some(739)));
        // No Western fallback for banks the language set does not list.
        assert_eq!(ja.get("battle_texts"), None);
        assert_eq!(ja.get("move_descriptions"), None);
        let ko = text_banks(Family::Pt, Language::Korean);
        assert_eq!(
            (ko.get("moves"), ko.get("abilities")),
            (Some(637), Some(605))
        );
        let dp_ja = text_banks(Family::Dp, Language::Japanese);
        assert_eq!(dp_ja.get("trainer_names"), Some(550));
    }

    #[test]
    fn diamond_pearl_is_unverified_without_battle_texts_or_forms() {
        let dp = text_banks(Family::Dp, Language::English);
        assert!(dp.unverified);
        assert_eq!(dp.get("species"), Some(362));
        assert_eq!(dp.get("battle_texts"), None);
        assert_eq!(dp.get("forms"), None);
    }
}
