//! German names of the editable fields, per data kind. They name a field in the undo label
//! ("Staravia: Angriff 75 → 85") and are mirrored one to one in `fieldLabels` of
//! `apps/studio/src/api.data.ts` (a test keeps both equal). A field missing here cannot be
//! changed through `data_edit`: the tables are also the list of editable fields.

/// Personal data (`DataPatch::Personal`), in the order the editor lists them.
pub const SPECIES: &[(&str, &str)] = &[
    ("hp", "KP"),
    ("attack", "Angriff"),
    ("defense", "Verteidigung"),
    ("speed", "Initiative"),
    ("sp_attack", "Spezial-Angriff"),
    ("sp_defense", "Spezial-Verteidigung"),
    ("type1", "Typ 1"),
    ("type2", "Typ 2"),
    ("catch_rate", "Fangrate"),
    ("base_exp", "Basis-EP"),
    ("ev_yield_hp", "EV-Ertrag KP"),
    ("ev_yield_attack", "EV-Ertrag Angriff"),
    ("ev_yield_defense", "EV-Ertrag Verteidigung"),
    ("ev_yield_speed", "EV-Ertrag Initiative"),
    ("ev_yield_sp_attack", "EV-Ertrag Spezial-Angriff"),
    ("ev_yield_sp_defense", "EV-Ertrag Spezial-Verteidigung"),
    ("item1", "Getragener Gegenstand 1"),
    ("item2", "Getragener Gegenstand 2"),
    ("gender_ratio", "Geschlecht"),
    ("egg_cycles", "Schlüpfzyklen"),
    ("base_friendship", "Basis-Freundschaft"),
    ("growth_rate", "Wachstum"),
    ("egg_group1", "Ei-Gruppe 1"),
    ("egg_group2", "Ei-Gruppe 2"),
    ("ability1", "Fähigkeit 1"),
    ("ability2", "Fähigkeit 2"),
    ("flee_rate", "Fluchtrate"),
    ("color", "Farbe"),
    ("flip", "Gespiegelt"),
];

/// Species fields that are no personal data, each changed by its own patch:
/// `egg_moves` by `DataPatch::EggMoves`. `fieldLabels.species` carries them too (the patches
/// name their label constant directly, so the table is only the test's).
#[cfg(test)]
pub const SPECIES_PARTS: &[(&str, &str)] = &[("egg_moves", EGG_MOVES)];

/// The egg moves of a species (`DataPatch::EggMoves`, undo label „Bisasam: Ei-Attacken 7 →
/// 8“).
pub const EGG_MOVES: &str = "Ei-Attacken";

/// Moves (`DataPatch::Move`).
pub const MOVE: &[(&str, &str)] = &[
    ("effect", "Effekt"),
    ("category", "Kategorie"),
    ("power", "Stärke"),
    ("move_type", "Typ"),
    ("accuracy", "Genauigkeit"),
    ("pp", "AP"),
    ("effect_chance", "Effektchance"),
    ("target", "Ziel"),
    ("priority", "Priorität"),
    ("flags", "Merkmale"),
    ("contest", "Wettbewerbseffekt"),
    ("contest_type", "Wettbewerbstyp"),
];

/// Items (`DataPatch::Item`): the named fields and bits of `docs/formats/item.md`, without
/// padding and without the containers of the bits.
pub const ITEM: &[(&str, &str)] = &[
    ("price", "Preis"),
    ("hold_effect", "Trage-Effekt"),
    ("hold_param", "Trage-Parameter"),
    ("pluck_effect", "Pflücker-Effekt"),
    ("fling_effect", "Schleuder-Effekt"),
    ("fling_power", "Schleuder-Stärke"),
    ("natural_gift_power", "Beerenkräfte-Stärke"),
    ("natural_gift_type", "Beerenkräfte-Typ"),
    ("prevent_toss", "Nicht wegwerfbar"),
    ("can_register", "Registrierbar"),
    ("field_pocket", "Tasche"),
    ("battle_pocket", "Kampftasche"),
    ("field_use_func", "Feldfunktion"),
    ("battle_use_func", "Kampffunktion"),
    ("party_use", "Wirkung im Team"),
    ("slp_heal", "Heilt Schlaf"),
    ("psn_heal", "Heilt Vergiftung"),
    ("brn_heal", "Heilt Verbrennung"),
    ("frz_heal", "Heilt Einfrieren"),
    ("prlz_heal", "Heilt Paralyse"),
    ("cfs_heal", "Heilt Verwirrung"),
    ("inf_heal", "Heilt Verliebtheit"),
    ("guard_spec", "Megaschutz"),
    ("revive", "Belebt"),
    ("revive_all", "Belebt alle"),
    ("level_up", "Level-Aufstieg"),
    ("evolve", "Entwicklung"),
    ("attack_stages", "Angriff-Stufen"),
    ("defense_stages", "Verteidigung-Stufen"),
    ("sp_attack_stages", "Spezial-Angriff-Stufen"),
    ("sp_defense_stages", "Spezial-Verteidigung-Stufen"),
    ("speed_stages", "Initiative-Stufen"),
    ("accuracy_stages", "Genauigkeit-Stufen"),
    ("crit_stages", "Volltreffer-Stufen"),
    ("pp_up", "AP-Plus"),
    ("pp_max", "AP-Top"),
    ("pp_restore", "Füllt AP auf"),
    ("pp_restore_all", "Füllt alle AP auf"),
    ("hp_restore", "Füllt KP auf"),
    ("hp_ev_flag", "Ändert EV KP"),
    ("attack_ev_flag", "Ändert EV Angriff"),
    ("defense_ev_flag", "Ändert EV Verteidigung"),
    ("speed_ev_flag", "Ändert EV Initiative"),
    ("sp_attack_ev_flag", "Ändert EV Spezial-Angriff"),
    ("sp_defense_ev_flag", "Ändert EV Spezial-Verteidigung"),
    ("friendship_low_flag", "Ändert Freundschaft niedrig"),
    ("friendship_mid_flag", "Ändert Freundschaft mittel"),
    ("friendship_high_flag", "Ändert Freundschaft hoch"),
    ("hp_ev", "EV-Änderung KP"),
    ("attack_ev", "EV-Änderung Angriff"),
    ("defense_ev", "EV-Änderung Verteidigung"),
    ("speed_ev", "EV-Änderung Initiative"),
    ("sp_attack_ev", "EV-Änderung Spezial-Angriff"),
    ("sp_defense_ev", "EV-Änderung Spezial-Verteidigung"),
    ("hp_amount", "KP-Menge"),
    ("pp_amount", "AP-Menge"),
    ("friendship_low", "Freundschaft niedrig"),
    ("friendship_mid", "Freundschaft mittel"),
    ("friendship_high", "Freundschaft hoch"),
];

/// Encounter files (`DataPatch::Encounter`): the methods and replacement lists (`method`)
/// and the fields of a slot (`field`).
pub const ENCOUNTER: &[(&str, &str)] = &[
    ("grass", "Gras"),
    ("surf", "Surfen"),
    ("rock_smash", "Zertrümmerer"),
    ("old_rod", "Angel"),
    ("good_rod", "Profiangel"),
    ("super_rod", "Superangel"),
    ("swarm", "Schwarm"),
    ("day", "Tag"),
    ("night", "Nacht"),
    ("radar", "Poké-Radar"),
    ("dual_ruby", "Dual-Slot Rubin"),
    ("dual_sapphire", "Dual-Slot Saphir"),
    ("dual_emerald", "Dual-Slot Smaragd"),
    ("dual_firered", "Dual-Slot Feuerrot"),
    ("dual_leafgreen", "Dual-Slot Blattgrün"),
    ("radio", "Radio"),
    ("radio_hoenn", "Radio Hoenn"),
    ("radio_sinnoh", "Radio Sinnoh"),
    ("land_swarm", "Schwarm im Gras"),
    ("surf_swarm", "Schwarm beim Surfen"),
    ("night_fish", "Angeln nachts"),
    ("fish_swarm", "Schwarm beim Angeln"),
    ("rate", "Rate"),
    ("species", "Pokémon"),
    ("level", "Level"),
    ("min", "Mindestlevel"),
    ("max", "Höchstlevel"),
    ("species_morning", "Pokémon morgens"),
    ("species_night", "Pokémon nachts"),
    ("form_rate_0", "Formrate 1"),
    ("form_rate_1", "Formrate 2"),
    ("form_rate_2", "Formrate 3"),
    ("form_rate_3", "Formrate 4"),
    ("form_rate_4", "Formrate 5"),
    ("unown_table", "Icognito-Tabelle"),
];

/// Trainer properties (`DataPatch::Trainer`).
pub const TRAINER: &[(&str, &str)] = &[
    ("class", "Klasse"),
    ("ai", "KI"),
    ("battle_type", "Kampfart"),
    ("item0", "Gegenstand 1"),
    ("item1", "Gegenstand 2"),
    ("item2", "Gegenstand 3"),
    ("item3", "Gegenstand 4"),
];

/// Every table by the key `fieldLabels` uses for it.
#[cfg(test)]
pub const ALL: &[(&str, &[(&str, &str)])] = &[
    ("species", SPECIES),
    ("move", MOVE),
    ("item", ITEM),
    ("encounter", ENCOUNTER),
    ("trainer", TRAINER),
];

/// The German name of `field` in `table`.
pub fn label(table: &[(&str, &'static str)], field: &str) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == field).map(|(_, v)| *v)
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

    #[test]
    fn field_labels_in_api_data_ts_mirror_the_command_tables() {
        let mut rust: BTreeMap<String, BTreeMap<String, String>> = ALL
            .iter()
            .map(|(kind, table)| {
                let fields = table
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect();
                ((*kind).to_owned(), fields)
            })
            .collect();
        // The species parts that are no personal data share `fieldLabels.species`.
        let species = rust.get_mut("species").unwrap();
        for (k, v) in SPECIES_PARTS {
            assert!(
                species.insert((*k).to_owned(), (*v).to_owned()).is_none(),
                "{k}"
            );
        }
        assert_eq!(ts_field_labels(), rust);
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
            assert!(table.iter().all(|(_, v)| !v.contains('.')), "abbreviation");
        }
    }
}
