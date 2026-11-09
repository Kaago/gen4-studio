//! The editable fields, per data kind, with the catalog key of their name (`labels.<field>`
//! in `locales/{en,de}.yml`; the two trainer item slots that share an id with the species'
//! held items are `labels.trainer_item<n>`). The name is looked up when it is used, so it
//! follows the language of the process: it names a field in the undo label ("Staravia:
//! Angriff 75 → 85" in German) and the German names are mirrored one to one in `fieldLabels`
//! of `apps/studio/src/api.data.ts` (a test keeps both equal). A field missing here cannot be
//! changed through `data_edit`: the tables are also the list of editable fields.

/// Personal data (`DataPatch::Personal`), in the order the editor lists them.
pub const SPECIES: &[(&str, &str)] = &[
    ("hp", "labels.hp"),
    ("attack", "labels.attack"),
    ("defense", "labels.defense"),
    ("speed", "labels.speed"),
    ("sp_attack", "labels.sp_attack"),
    ("sp_defense", "labels.sp_defense"),
    ("type1", "labels.type1"),
    ("type2", "labels.type2"),
    ("catch_rate", "labels.catch_rate"),
    ("base_exp", "labels.base_exp"),
    ("ev_yield_hp", "labels.ev_yield_hp"),
    ("ev_yield_attack", "labels.ev_yield_attack"),
    ("ev_yield_defense", "labels.ev_yield_defense"),
    ("ev_yield_speed", "labels.ev_yield_speed"),
    ("ev_yield_sp_attack", "labels.ev_yield_sp_attack"),
    ("ev_yield_sp_defense", "labels.ev_yield_sp_defense"),
    ("item1", "labels.item1"),
    ("item2", "labels.item2"),
    ("gender_ratio", "labels.gender_ratio"),
    ("egg_cycles", "labels.egg_cycles"),
    ("base_friendship", "labels.base_friendship"),
    ("growth_rate", "labels.growth_rate"),
    ("egg_group1", "labels.egg_group1"),
    ("egg_group2", "labels.egg_group2"),
    ("ability1", "labels.ability1"),
    ("ability2", "labels.ability2"),
    ("flee_rate", "labels.flee_rate"),
    ("color", "labels.color"),
    ("flip", "labels.flip"),
];

/// Species fields that are no personal data, each changed by its own patch:
/// `egg_moves` by `DataPatch::EggMoves`. `fieldLabels.species` carries them too (the patches
/// name their label directly, so the table is only [`field_label`]'s and the test's).
pub const SPECIES_PARTS: &[(&str, &str)] = &[("egg_moves", EGG_MOVES)];

/// The catalog key of the name of the egg moves of a species (`DataPatch::EggMoves`, undo
/// label „Bisasam: Ei-Attacken 7 → 8“ in German).
pub const EGG_MOVES: &str = "labels.egg_moves";

/// Moves (`DataPatch::Move`).
pub const MOVE: &[(&str, &str)] = &[
    ("effect", "labels.effect"),
    ("category", "labels.category"),
    ("power", "labels.power"),
    ("move_type", "labels.move_type"),
    ("accuracy", "labels.accuracy"),
    ("pp", "labels.pp"),
    ("effect_chance", "labels.effect_chance"),
    ("target", "labels.target"),
    ("priority", "labels.priority"),
    ("flags", "labels.flags"),
    ("contest", "labels.contest"),
    ("contest_type", "labels.contest_type"),
];

/// Items (`DataPatch::Item`): the named fields and bits of `docs/formats/item.md`, without
/// padding and without the containers of the bits.
pub const ITEM: &[(&str, &str)] = &[
    ("price", "labels.price"),
    ("hold_effect", "labels.hold_effect"),
    ("hold_param", "labels.hold_param"),
    ("pluck_effect", "labels.pluck_effect"),
    ("fling_effect", "labels.fling_effect"),
    ("fling_power", "labels.fling_power"),
    ("natural_gift_power", "labels.natural_gift_power"),
    ("natural_gift_type", "labels.natural_gift_type"),
    ("prevent_toss", "labels.prevent_toss"),
    ("can_register", "labels.can_register"),
    ("field_pocket", "labels.field_pocket"),
    ("battle_pocket", "labels.battle_pocket"),
    ("field_use_func", "labels.field_use_func"),
    ("battle_use_func", "labels.battle_use_func"),
    ("party_use", "labels.party_use"),
    ("slp_heal", "labels.slp_heal"),
    ("psn_heal", "labels.psn_heal"),
    ("brn_heal", "labels.brn_heal"),
    ("frz_heal", "labels.frz_heal"),
    ("prlz_heal", "labels.prlz_heal"),
    ("cfs_heal", "labels.cfs_heal"),
    ("inf_heal", "labels.inf_heal"),
    ("guard_spec", "labels.guard_spec"),
    ("revive", "labels.revive"),
    ("revive_all", "labels.revive_all"),
    ("level_up", "labels.level_up"),
    ("evolve", "labels.evolve"),
    ("attack_stages", "labels.attack_stages"),
    ("defense_stages", "labels.defense_stages"),
    ("sp_attack_stages", "labels.sp_attack_stages"),
    ("sp_defense_stages", "labels.sp_defense_stages"),
    ("speed_stages", "labels.speed_stages"),
    ("accuracy_stages", "labels.accuracy_stages"),
    ("crit_stages", "labels.crit_stages"),
    ("pp_up", "labels.pp_up"),
    ("pp_max", "labels.pp_max"),
    ("pp_restore", "labels.pp_restore"),
    ("pp_restore_all", "labels.pp_restore_all"),
    ("hp_restore", "labels.hp_restore"),
    ("hp_ev_flag", "labels.hp_ev_flag"),
    ("attack_ev_flag", "labels.attack_ev_flag"),
    ("defense_ev_flag", "labels.defense_ev_flag"),
    ("speed_ev_flag", "labels.speed_ev_flag"),
    ("sp_attack_ev_flag", "labels.sp_attack_ev_flag"),
    ("sp_defense_ev_flag", "labels.sp_defense_ev_flag"),
    ("friendship_low_flag", "labels.friendship_low_flag"),
    ("friendship_mid_flag", "labels.friendship_mid_flag"),
    ("friendship_high_flag", "labels.friendship_high_flag"),
    ("hp_ev", "labels.hp_ev"),
    ("attack_ev", "labels.attack_ev"),
    ("defense_ev", "labels.defense_ev"),
    ("speed_ev", "labels.speed_ev"),
    ("sp_attack_ev", "labels.sp_attack_ev"),
    ("sp_defense_ev", "labels.sp_defense_ev"),
    ("hp_amount", "labels.hp_amount"),
    ("pp_amount", "labels.pp_amount"),
    ("friendship_low", "labels.friendship_low"),
    ("friendship_mid", "labels.friendship_mid"),
    ("friendship_high", "labels.friendship_high"),
];

/// Encounter files (`DataPatch::Encounter`): the methods and replacement lists (`method`)
/// and the fields of a slot (`field`).
pub const ENCOUNTER: &[(&str, &str)] = &[
    ("grass", "labels.grass"),
    ("surf", "labels.surf"),
    ("rock_smash", "labels.rock_smash"),
    ("old_rod", "labels.old_rod"),
    ("good_rod", "labels.good_rod"),
    ("super_rod", "labels.super_rod"),
    ("swarm", "labels.swarm"),
    ("day", "labels.day"),
    ("night", "labels.night"),
    ("radar", "labels.radar"),
    ("dual_ruby", "labels.dual_ruby"),
    ("dual_sapphire", "labels.dual_sapphire"),
    ("dual_emerald", "labels.dual_emerald"),
    ("dual_firered", "labels.dual_firered"),
    ("dual_leafgreen", "labels.dual_leafgreen"),
    ("radio", "labels.radio"),
    ("radio_hoenn", "labels.radio_hoenn"),
    ("radio_sinnoh", "labels.radio_sinnoh"),
    ("land_swarm", "labels.land_swarm"),
    ("surf_swarm", "labels.surf_swarm"),
    ("night_fish", "labels.night_fish"),
    ("fish_swarm", "labels.fish_swarm"),
    ("rate", "labels.rate"),
    ("species", "labels.species"),
    ("level", "labels.level"),
    ("min", "labels.min"),
    ("max", "labels.max"),
    ("species_morning", "labels.species_morning"),
    ("species_night", "labels.species_night"),
    ("form_rate_0", "labels.form_rate_0"),
    ("form_rate_1", "labels.form_rate_1"),
    ("form_rate_2", "labels.form_rate_2"),
    ("form_rate_3", "labels.form_rate_3"),
    ("form_rate_4", "labels.form_rate_4"),
    ("unown_table", "labels.unown_table"),
];

/// Trainer properties (`DataPatch::Trainer`).
pub const TRAINER: &[(&str, &str)] = &[
    ("class", "labels.class"),
    ("ai", "labels.ai"),
    ("battle_type", "labels.battle_type"),
    ("item0", "labels.trainer_item0"),
    ("item1", "labels.trainer_item1"),
    ("item2", "labels.trainer_item2"),
    ("item3", "labels.trainer_item3"),
];

/// Every table by the key `fieldLabels` uses for it.
pub const ALL: &[(&str, &[(&str, &str)])] = &[
    ("species", SPECIES),
    ("move", MOVE),
    ("item", ITEM),
    ("encounter", ENCOUNTER),
    ("trainer", TRAINER),
];

/// The name of `field` in `table`, in the language of the process.
pub fn label(table: &[(&str, &str)], field: &str) -> Option<String> {
    let (_, key) = table.iter().find(|(k, _)| *k == field)?;
    Some(name(key))
}

/// The name behind the catalog key `key`, in the language of the process.
pub fn name(key: &str) -> String {
    t!(key).into_owned()
}

/// The name of the egg moves of a species ([`EGG_MOVES`]), in the language of the process.
pub fn egg_moves() -> String {
    name(EGG_MOVES)
}

/// The name of `field` of the data kind `kind` (`species`, `move`, `item`, `encounter` or
/// `trainer`, the keys of `fieldLabels`), in the language of the process; `None` for a field
/// the kind does not list.
pub fn field_label(kind: &str, field: &str) -> Option<String> {
    let (_, table) = ALL.iter().find(|(k, _)| *k == kind)?;
    label(table, field).or_else(|| {
        if kind != "species" {
            return None;
        }
        let (_, key) = SPECIES_PARTS.iter().find(|(k, _)| *k == field)?;
        Some(name(key))
    })
}

/// The field names of `table`, in table order.
pub fn names(table: &[(&'static str, &str)]) -> Vec<&'static str> {
    table.iter().map(|(k, _)| *k).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use g4_formats::data::item::Item;
    use g4_formats::data::moves::Move;
    use g4_formats::data::personal::Personal;
    use g4_formats::data::record::RawRecord;

    use super::*;
    use crate::commands::data::fields::numeric_span;

    /// The name behind catalog key `key` in `locale`, whatever the language of the process.
    fn in_locale(key: &str, locale: &str) -> String {
        let text = t!(key, locale = locale).into_owned();
        assert_ne!(text, key, "{key} is missing in {locale}");
        text
    }

    /// `fieldLabels` of api.data.ts as kind → field → label: the lines `key: "Label",` below
    /// a line `kind: {`, up to the object's closing line (a `}` without indentation).
    fn ts_field_labels() -> BTreeMap<String, BTreeMap<String, String>> {
        let ts = include_str!("../../../../src/api.data.ts");
        let start = ts.find("export const fieldLabels").unwrap();
        let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut kind = None;
        for line in ts[start..].lines().skip(1) {
            if line.starts_with('}') {
                break;
            }
            let line = line.trim();
            if let Some(k) = line.strip_suffix(": {") {
                kind = Some(k.to_owned());
                continue;
            }
            if line.starts_with('}') {
                kind = None;
                continue;
            }
            let Some(kind) = &kind else { continue };
            let (key, value) = line.split_once(": ").unwrap();
            let value = value.trim_end_matches(',').trim_matches('"');
            out.entry(kind.clone())
                .or_default()
                .insert(key.to_owned(), value.to_owned());
        }
        out
    }

    /// The German names of the catalog, as `fieldLabels` of api.data.ts lists them.
    #[test]
    fn field_labels_in_api_data_ts_mirror_the_command_tables() {
        let mut rust: BTreeMap<String, BTreeMap<String, String>> = ALL
            .iter()
            .map(|(kind, table)| {
                let fields = table
                    .iter()
                    .map(|(k, key)| ((*k).to_owned(), in_locale(key, "de")))
                    .collect();
                ((*kind).to_owned(), fields)
            })
            .collect();
        // The species parts that are no personal data share `fieldLabels.species`.
        let species = rust.get_mut("species").unwrap();
        for (k, key) in SPECIES_PARTS {
            assert!(
                species
                    .insert((*k).to_owned(), in_locale(key, "de"))
                    .is_none(),
                "{k}"
            );
        }
        assert_eq!(ts_field_labels(), rust);
    }

    #[test]
    fn every_field_has_a_name_in_both_languages() {
        for (kind, table) in ALL {
            for (field, key) in *table {
                let english = in_locale(key, "en");
                assert!(!english.contains('.'), "{kind}.{field}: abbreviation");
                assert!(
                    !in_locale(key, "de").contains('.'),
                    "{kind}.{field}: abbreviation"
                );
            }
        }
        assert_eq!(in_locale(EGG_MOVES, "en"), "Egg moves");
        assert_eq!(in_locale(EGG_MOVES, "de"), "Ei-Attacken");
    }

    #[test]
    fn every_labelled_record_field_exists_and_is_numeric() {
        for (name, _) in SPECIES {
            assert!(numeric_span(Personal::FIELDS, name).is_some(), "{name}");
        }
        for (name, _) in MOVE {
            assert!(numeric_span(Move::FIELDS, name).is_some(), "{name}");
        }
        for (name, _) in ITEM {
            assert!(numeric_span(Item::FIELDS, name).is_some(), "{name}");
        }
        for table in [SPECIES, MOVE, ITEM, ENCOUNTER, TRAINER] {
            let mut keys = names(table);
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), table.len(), "duplicate key");
        }
    }
}
