//! Rules of the wild encounter files the editor shows next to the bytes:
//! slot shares, replacement lists, warnings and the species per time of day.
//! Source: `docs/research/2026-10-23-3b3/encounter-mechanics-3b3b.md` §1–§3, §6.
//!
//! Texts are English with numbers (the UI words them); slot and entry numbers
//! are the 0-based indexes of the codec. Nothing here refuses anything: the
//! editor shows warnings, never locks.

pub use g4_formats::data::encounter::Method;
use g4_formats::data::encounter::{DualSlot, Encounters, HgssEncounters, PtEncounters, WaterSlot};
use g4_formats::game::Family;

const GRASS: &[u8] = &[20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1];
const WATER: &[u8] = &[60, 30, 5, 4, 1];
const PT_GOOD_ROD: &[u8] = &[40, 40, 15, 4, 1];
const HGSS_ROD: &[u8] = &[40, 30, 15, 10, 5];
const HGSS_ROCK: &[u8] = &[80, 20];

/// The share in percent of each slot of `method`, in slot order (the game
/// picks a slot with one `rand(100)` against fixed bounds). Empty for a method
/// the family does not use (Rock Smash in Platinum).
pub fn shares(method: Method, family: Family) -> &'static [u8] {
    match (method, family) {
        (Method::Grass, _) => GRASS,
        (Method::Surf, _) => WATER,
        (Method::RockSmash, Family::Hgss) => HGSS_ROCK,
        (Method::RockSmash, _) => &[],
        (Method::OldRod | Method::GoodRod | Method::SuperRod, Family::Hgss) => HGSS_ROD,
        (Method::OldRod, _) => WATER,
        (Method::GoodRod | Method::SuperRod, _) => PT_GOOD_ROD,
    }
}

/// One entry of a replacement list and the slots of `method` it replaces
/// (only the species; the level stays the slot's own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replacement {
    /// Name of the list: for lists in the file the stem of the codec field
    /// (`day`, `night`, `swarm`, `radar`, `dual_ruby` …, HeartGold `radio_hoenn`,
    /// `radio_sinnoh`, `land_swarm`, `surf_swarm`, `night_fish`, `fish_swarm`);
    /// `trophy_garden` and `great_marsh` come from elsewhere.
    pub list: &'static str,
    /// The entry of the list (0-based).
    pub entry: usize,
    pub method: Method,
    /// The slots of `method` the entry replaces.
    pub slots: &'static [usize],
    /// When the game uses the entry, English.
    pub when: &'static str,
    /// Whether the list is stored in the encounter file (false: Trophy Garden,
    /// Great Marsh).
    pub in_file: bool,
}

const fn rep(
    list: &'static str,
    entry: usize,
    method: Method,
    slots: &'static [usize],
    when: &'static str,
) -> Replacement {
    Replacement {
        list,
        entry,
        method,
        slots,
        when,
        in_file: true,
    }
}

const PT_DAY: &str = "from 10 to 19 o'clock";
const PT_NIGHT: &str = "from 20 to 3 o'clock";
const SWARM: &str = "during a swarm";
const PT_DUAL: [(DualSlot, &str, &str); 5] = [
    (
        DualSlot::Ruby,
        "dual_ruby",
        "with Ruby in the Game Boy Advance slot",
    ),
    (
        DualSlot::Sapphire,
        "dual_sapphire",
        "with Sapphire in the Game Boy Advance slot",
    ),
    (
        DualSlot::Emerald,
        "dual_emerald",
        "with Emerald in the Game Boy Advance slot",
    ),
    (
        DualSlot::FireRed,
        "dual_firered",
        "with FireRed in the Game Boy Advance slot",
    ),
    (
        DualSlot::LeafGreen,
        "dual_leafgreen",
        "with LeafGreen in the Game Boy Advance slot",
    ),
];
const HGSS_SWARM: &str = "during a swarm on one of the 20 swarm headers";

/// The replacement lists of `family`, entry by entry, in the order the editor
/// lists them (research §3.1, §3.2). Platinum (and Diamond/Pearl): `day` and
/// `night` replace grass slots 2 and 3, `swarm` 0 and 1, `radar` 4, 5, 10 and
/// 11, the dual-slot lists 8 and 9, Trophy Garden and Great Marsh 6 and 7.
/// HeartGold: the radio lists replace grass 2, 3 (entry 0) and 4, 5 (entry 1),
/// the swarm lists grass 0, 1, Surf 0 and rod slots; `night_fish` replaces a
/// rod slot at night. The HeartGold morning, day and night grass columns are
/// no replacement but the grass list itself (see [`effective_species`]).
pub fn replacements(family: Family) -> Vec<Replacement> {
    use Method::*;
    match family {
        Family::Dp | Family::Pt => {
            let mut out = vec![
                rep("day", 0, Grass, &[2], PT_DAY),
                rep("day", 1, Grass, &[3], PT_DAY),
                rep("night", 0, Grass, &[2], PT_NIGHT),
                rep("night", 1, Grass, &[3], PT_NIGHT),
                rep("swarm", 0, Grass, &[0], SWARM),
                rep("swarm", 1, Grass, &[1], SWARM),
                rep("radar", 0, Grass, &[4], "with the Poké Radar"),
                rep("radar", 1, Grass, &[5], "with the Poké Radar"),
                rep("radar", 2, Grass, &[10], "with the Poké Radar"),
                rep("radar", 3, Grass, &[11], "with the Poké Radar"),
            ];
            for (_, list, when) in PT_DUAL {
                out.push(rep(list, 0, Grass, &[8], when));
                out.push(rep(list, 1, Grass, &[9], when));
            }
            for (list, when) in [
                ("trophy_garden", "in the Trophy Garden"),
                ("great_marsh", "in the Great Marsh during a Safari Game"),
            ] {
                for (entry, slots) in [(0, &[6][..]), (1, &[7][..])] {
                    out.push(Replacement {
                        in_file: false,
                        ..rep(list, entry, Grass, slots, when)
                    });
                }
            }
            out
        }
        Family::Hgss => {
            let hoenn = "with the Hoenn sound on the radio";
            let sinnoh = "with the Sinnoh sound on the radio";
            vec![
                rep("radio_hoenn", 0, Grass, &[2, 3], hoenn),
                rep("radio_hoenn", 1, Grass, &[4, 5], hoenn),
                rep("radio_sinnoh", 0, Grass, &[2, 3], sinnoh),
                rep("radio_sinnoh", 1, Grass, &[4, 5], sinnoh),
                rep("land_swarm", 0, Grass, &[0, 1], HGSS_SWARM),
                rep("surf_swarm", 0, Surf, &[0], HGSS_SWARM),
                rep("night_fish", 0, GoodRod, &[3], "at night"),
                rep("night_fish", 0, SuperRod, &[1], "at night"),
                rep("fish_swarm", 0, OldRod, &[2], HGSS_SWARM),
                rep("fish_swarm", 0, GoodRod, &[0, 2, 3], HGSS_SWARM),
                rep("fish_swarm", 0, SuperRod, &[0, 1, 2, 3, 4], HGSS_SWARM),
            ]
        }
    }
}

/// The species of entry `entry` of replacement list `list` in `enc`, or
/// `None` for a list the file does not store (or of the other family).
pub fn list_species(enc: &Encounters, list: &str, entry: usize) -> Option<u16> {
    match enc {
        Encounters::Pt(e) => pt_list_species(e, list, entry),
        Encounters::Hgss(e) => hgss_list_species(e, list, entry),
    }
}

fn pt_list_species(e: &PtEncounters, list: &str, entry: usize) -> Option<u16> {
    let two = entry < 2;
    match list {
        "day" if two => Some(e.day(entry)),
        "night" if two => Some(e.night(entry)),
        "swarm" if two => Some(e.swarm(entry)),
        "radar" if entry < 4 => Some(e.radar(entry)),
        _ if two => PT_DUAL
            .iter()
            .find(|(_, name, _)| *name == list)
            .map(|&(game, _, _)| e.dual(game, entry)),
        _ => None,
    }
}

fn hgss_list_species(e: &HgssEncounters, list: &str, entry: usize) -> Option<u16> {
    match (list, entry) {
        ("radio_hoenn", 0 | 1) => Some(e.radio(entry)),
        ("radio_sinnoh", 0 | 1) => Some(e.radio(2 + entry)),
        ("land_swarm", 0) => Some(e.land_swarm()),
        ("surf_swarm", 0) => Some(e.surf_swarm()),
        ("night_fish", 0) => Some(e.night_fish()),
        ("fish_swarm", 0) => Some(e.fish_swarm()),
        _ => None,
    }
}

/// Check code of an empty species where the game reads one: a HeartGold grass time column,
/// or an empty replacement list entry over an occupied slot (a glitch Pokémon).
pub const EMPTY_SLOT: &str = "encounter.empty-slot";
/// Check code of a species at level 0.
pub const LEVEL_ZERO: &str = "encounter.level-zero";
/// Check code of a level above 100.
pub const LEVEL_HIGH: &str = "encounter.level-high";
/// Check code of a water slot whose min level is above its max (the game swaps them).
pub const MIN_MAX: &str = "encounter.min-max";
/// Check code of a Platinum rate above 255 (the game reads the low byte).
pub const RATE_BYTE: &str = "encounter.rate-byte";

/// A rule the file breaks; shown next to the slot (or the method for
/// `slot: None`), never enforced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleWarning {
    pub method: Method,
    pub slot: Option<usize>,
    pub text: String,
    /// The check code ([`EMPTY_SLOT`], [`LEVEL_ZERO`], [`LEVEL_HIGH`], [`MIN_MAX`],
    /// [`RATE_BYTE`]).
    pub code: &'static str,
    /// Names the spot within the file: `slot:<method>:<slot>` (with `:<column>` or
    /// `:<list>:<entry>` for an empty slot), `rate:<method>` for a rate.
    pub key: String,
    /// The numbers of the text, by code: level-zero `[method, slot, species]`, level-high
    /// `[method, slot, level]`, min-max `[method, slot, min, max]`, rate-byte
    /// `[method, rate, low byte]`, empty-slot `[method, slot, column or list, entry]` (entry
    /// empty for a HeartGold grass column). `method` is [`method_key`].
    pub args: Vec<String>,
}

/// The stable name of `method` in keys and arguments (`grass`, `surf`, `rock_smash`,
/// `old_rod`, `good_rod`, `super_rod`).
pub fn method_key(method: Method) -> &'static str {
    match method {
        Method::Grass => "grass",
        Method::Surf => "surf",
        Method::RockSmash => "rock_smash",
        Method::OldRod => "old_rod",
        Method::GoodRod => "good_rod",
        Method::SuperRod => "super_rod",
    }
}

fn method_name(method: Method) -> &'static str {
    match method {
        Method::Grass => "grass",
        Method::Surf => "surf",
        Method::RockSmash => "rock smash",
        Method::OldRod => "old rod",
        Method::GoodRod => "good rod",
        Method::SuperRod => "super rod",
    }
}

/// One slot of any method as the rules see it: grass has `min == max`; a
/// HeartGold grass slot lists its morning, day and night species.
struct RuleSlot {
    species: Vec<(&'static str, u16)>,
    min: u8,
    max: u8,
}

impl RuleSlot {
    fn occupied(&self) -> bool {
        self.min != 0 || self.max != 0 || self.species.iter().any(|&(_, s)| s != 0)
    }

    fn first_species(&self) -> u16 {
        self.species
            .iter()
            .map(|&(_, s)| s)
            .find(|&s| s != 0)
            .unwrap_or(0)
    }
}

fn water_slots(slots: impl Iterator<Item = WaterSlot>) -> Vec<RuleSlot> {
    slots
        .map(|s| RuleSlot {
            species: vec![("", s.species())],
            min: s.min(),
            max: s.max(),
        })
        .collect()
}

/// Each method of the file with its stored rate and its slots.
fn rule_methods(enc: &Encounters) -> Vec<(Method, u32, Vec<RuleSlot>)> {
    match enc {
        Encounters::Pt(e) => {
            let grass = (0..12)
                .map(|i| {
                    let s = e.grass(i);
                    RuleSlot {
                        species: vec![("", s.species)],
                        min: s.level,
                        max: s.level,
                    }
                })
                .collect();
            vec![
                (Method::Grass, e.grass_rate(), grass),
                (
                    Method::Surf,
                    e.surf_rate(),
                    water_slots((0..5).map(|i| e.surf(i))),
                ),
                (
                    Method::OldRod,
                    e.old_rod_rate(),
                    water_slots((0..5).map(|i| e.old_rod(i))),
                ),
                (
                    Method::GoodRod,
                    e.good_rod_rate(),
                    water_slots((0..5).map(|i| e.good_rod(i))),
                ),
                (
                    Method::SuperRod,
                    e.super_rod_rate(),
                    water_slots((0..5).map(|i| e.super_rod(i))),
                ),
            ]
        }
        Encounters::Hgss(e) => {
            let grass = (0..12)
                .map(|i| RuleSlot {
                    species: vec![
                        ("morning", e.grass_morning(i)),
                        ("day", e.grass_day(i)),
                        ("night", e.grass_night(i)),
                    ],
                    min: e.grass_level(i),
                    max: e.grass_level(i),
                })
                .collect();
            let rate = |r: u8| u32::from(r);
            vec![
                (Method::Grass, rate(e.grass_rate()), grass),
                (
                    Method::Surf,
                    rate(e.surf_rate()),
                    water_slots((0..5).map(|i| e.surf(i))),
                ),
                (
                    Method::RockSmash,
                    rate(e.rock_rate()),
                    water_slots((0..2).map(|i| e.rock(i))),
                ),
                (
                    Method::OldRod,
                    rate(e.old_rod_rate()),
                    water_slots((0..5).map(|i| e.old_rod(i))),
                ),
                (
                    Method::GoodRod,
                    rate(e.good_rod_rate()),
                    water_slots((0..5).map(|i| e.good_rod(i))),
                ),
                (
                    Method::SuperRod,
                    rate(e.super_rod_rate()),
                    water_slots((0..5).map(|i| e.super_rod(i))),
                ),
            ]
        }
    }
}

/// The hours of a HeartGold grass column.
fn column_hours(when: &str) -> &'static str {
    match when {
        "morning" => "from 4 to 9 o'clock",
        "day" => "from 10 to 19 o'clock",
        _ => "from 20 to 3 o'clock",
    }
}

/// The rules `enc` breaks (spec §3b.3 encounters, research §1–§3, §6). Only
/// methods whose rate is above 0 are checked (Platinum: the rate the game
/// reads, the low byte). Per slot: a species at level 0, a level above 100,
/// water min above max (the game swaps them). An empty slot (species and
/// levels 0) is valid. An empty replacement entry is flagged when the slot it
/// replaces is not empty: Platinum day, night, swarm, radar and dual-slot
/// lists; HeartGold each of the morning, day and night columns, radio and
/// swarm lists. Platinum rates above 255 act like their low byte.
pub fn warnings(enc: &Encounters, family: Family) -> Vec<RuleWarning> {
    let methods = rule_methods(enc);
    let mut out = Vec::new();
    let effective_rate = |rate: u32| match enc {
        Encounters::Pt(_) => rate & 0xFF,
        Encounters::Hgss(_) => rate,
    };
    for (method, rate, slots) in &methods {
        let name = method_name(*method);
        let mkey = method_key(*method);
        let warn = |slot: Option<usize>, code: &'static str, text: String, args: Vec<String>| {
            let key = match slot {
                Some(i) => format!("slot:{mkey}:{i}"),
                None => format!("rate:{mkey}"),
            };
            RuleWarning {
                method: *method,
                slot,
                text,
                code,
                key,
                args: [mkey.to_owned()].into_iter().chain(args).collect(),
            }
        };
        if matches!(enc, Encounters::Pt(_)) && *rate > 0xFF {
            let low = rate & 0xFF;
            let text = if low == 0 {
                format!("{name} rate {rate} acts like 0: the method never runs")
            } else {
                format!("{name} rate {rate} acts like {low}")
            };
            out.push(warn(
                None,
                RATE_BYTE,
                text,
                vec![rate.to_string(), low.to_string()],
            ));
        }
        if effective_rate(*rate) == 0 {
            continue;
        }
        for (i, s) in slots.iter().enumerate() {
            if !s.occupied() {
                continue;
            }
            let species = s.first_species();
            let (lo, hi) = (s.min.min(s.max), s.min.max(s.max));
            if species != 0 && lo == 0 {
                out.push(warn(
                    Some(i),
                    LEVEL_ZERO,
                    format!("{name} slot {i}: species {species} at level 0"),
                    vec![i.to_string(), species.to_string()],
                ));
            }
            if hi > 100 {
                out.push(warn(
                    Some(i),
                    LEVEL_HIGH,
                    format!("{name} slot {i}: level {hi} is above 100"),
                    vec![i.to_string(), hi.to_string()],
                ));
            }
            if s.min > s.max {
                out.push(warn(
                    Some(i),
                    MIN_MAX,
                    format!(
                        "{name} slot {i}: min level {} is above max level {}, the game swaps them",
                        s.min, s.max
                    ),
                    vec![i.to_string(), s.min.to_string(), s.max.to_string()],
                ));
            }
            if s.species.len() > 1 {
                for &(when, sp) in &s.species {
                    if sp == 0 {
                        let mut w = warn(
                            Some(i),
                            EMPTY_SLOT,
                            format!(
                                "{name} slot {i}: the {when} species is empty: {} it gives a glitch Pokémon",
                                column_hours(when)
                            ),
                            vec![i.to_string(), when.to_owned(), String::new()],
                        );
                        w.key = format!("{}:{when}", w.key);
                        out.push(w);
                    }
                }
            }
        }
    }
    for r in replacements(family) {
        let Some(species) = list_species(enc, r.list, r.entry) else {
            continue;
        };
        let Some((_, rate, slots)) = methods.iter().find(|(m, _, _)| *m == r.method) else {
            continue;
        };
        if species != 0 || effective_rate(*rate) == 0 {
            continue;
        }
        for &slot in r.slots {
            if slots.get(slot).is_some_and(RuleSlot::occupied) {
                let mkey = method_key(r.method);
                out.push(RuleWarning {
                    method: r.method,
                    slot: Some(slot),
                    text: format!(
                        "{} entry {} is empty: {}, {} slot {slot} gives a glitch Pokémon",
                        r.list,
                        r.entry,
                        r.when,
                        method_name(r.method)
                    ),
                    code: EMPTY_SLOT,
                    key: format!("slot:{mkey}:{slot}:{}:{}", r.list, r.entry),
                    args: vec![
                        mkey.to_owned(),
                        slot.to_string(),
                        r.list.to_owned(),
                        r.entry.to_string(),
                    ],
                });
            }
        }
    }
    out
}

/// The times of day the game tells apart for grass, with their hours: 4–9
/// morning, 10–19 day (with dusk), 20–3 night.
pub const TIMES: [&str; 3] = ["morning", "day", "night"];

/// The species grass slot `slot` gives at each time of day without swarm,
/// radar, radio or cartridge: `[("morning", …), ("day", …), ("night", …)]`.
/// Platinum takes slots 2 and 3 from the `day` and `night` lists, every other
/// slot from the grass list; HeartGold reads its three columns. Empty for a
/// slot outside 0..12.
pub fn effective_species(
    enc: &Encounters,
    family: Family,
    slot: usize,
) -> Vec<(&'static str, u16)> {
    // The variant decides the layout; the family only names it.
    let _ = family;
    if slot >= 12 {
        return Vec::new();
    }
    match enc {
        Encounters::Pt(e) => {
            let base = e.grass(slot).species;
            let (day, night) = match slot {
                2 | 3 => (e.day(slot - 2), e.night(slot - 2)),
                _ => (base, base),
            };
            vec![("morning", base), ("day", day), ("night", night)]
        }
        Encounters::Hgss(e) => vec![
            ("morning", e.grass_morning(slot)),
            ("day", e.grass_day(slot)),
            ("night", e.grass_night(slot)),
        ],
    }
}

/// Who waits in the grass at one time of day: each species with the sum of
/// the shares of its slots, largest share first (then by species).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeShares {
    pub when: &'static str,
    pub species: Vec<(u16, u8)>,
}

/// [`effective_species`] of all twelve grass slots added up per time of day
/// ([`TIMES`] order) with the grass shares; empty slots (species 0) are left
/// out. Independent of the rate.
pub fn effective_shares(enc: &Encounters, family: Family) -> Vec<TimeShares> {
    let shares = shares(Method::Grass, family);
    let per_slot: Vec<Vec<(&'static str, u16)>> = (0..12)
        .map(|slot| effective_species(enc, family, slot))
        .collect();
    TIMES
        .iter()
        .enumerate()
        .map(|(t, &when)| {
            let mut species: Vec<(u16, u8)> = Vec::new();
            for (slot, times) in per_slot.iter().enumerate() {
                let sp = times[t].1;
                if sp == 0 {
                    continue;
                }
                match species.iter_mut().find(|(s, _)| *s == sp) {
                    Some((_, share)) => *share += shares[slot],
                    None => species.push((sp, shares[slot])),
                }
            }
            species.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            TimeShares { when, species }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use g4_formats::data::encounter::Slot;
    use g4_formats::data::record::RawRecord;

    fn pt() -> PtEncounters {
        PtEncounters::read(&[0; 424]).unwrap()
    }

    fn hgss() -> HgssEncounters {
        HgssEncounters::read(&[0; 196]).unwrap()
    }

    /// A Platinum file whose grass list and every replacement list hold species 1 at level 5.
    fn pt_full_grass() -> PtEncounters {
        let mut e = pt();
        e.set_grass_rate(30);
        for i in 0..12 {
            e.set_grass(
                i,
                Slot {
                    level: 5,
                    species: 1,
                },
            );
        }
        for i in 0..2 {
            e.set_swarm(i, 1);
            e.set_day(i, 1);
            e.set_night(i, 1);
            for game in DualSlot::ALL {
                e.set_dual(game, i, 1);
            }
        }
        for i in 0..4 {
            e.set_radar(i, 1);
        }
        e
    }

    /// A HeartGold file whose three grass columns, radio and swarm lists hold species 1.
    fn hgss_full_grass() -> HgssEncounters {
        let mut e = hgss();
        e.set_grass_rate(30);
        for i in 0..12 {
            e.set_grass_level(i, 5);
            e.set_grass_morning(i, 1);
            e.set_grass_day(i, 1);
            e.set_grass_night(i, 1);
        }
        for i in 0..4 {
            e.set_radio(i, 1);
        }
        e.set_land_swarm(1);
        e.set_surf_swarm(1);
        e.set_night_fish(1);
        e.set_fish_swarm(1);
        e
    }

    fn texts(w: &[RuleWarning]) -> Vec<&str> {
        w.iter().map(|w| w.text.as_str()).collect()
    }

    #[test]
    fn grass_shares_sum_to_100_in_both_families() {
        for family in [Family::Pt, Family::Hgss] {
            let grass = shares(Method::Grass, family);
            assert_eq!(grass, [20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1]);
            assert_eq!(grass.iter().map(|&s| u32::from(s)).sum::<u32>(), 100);
        }
    }

    #[test]
    fn water_shares_follow_the_family() {
        assert_eq!(shares(Method::Surf, Family::Pt), [60, 30, 5, 4, 1]);
        assert_eq!(shares(Method::OldRod, Family::Pt), [60, 30, 5, 4, 1]);
        assert_eq!(shares(Method::GoodRod, Family::Pt), [40, 40, 15, 4, 1]);
        assert_eq!(shares(Method::SuperRod, Family::Pt), [40, 40, 15, 4, 1]);
        assert!(shares(Method::RockSmash, Family::Pt).is_empty());
        assert_eq!(shares(Method::Surf, Family::Hgss), [60, 30, 5, 4, 1]);
        for rod in [Method::OldRod, Method::GoodRod, Method::SuperRod] {
            assert_eq!(shares(rod, Family::Hgss), [40, 30, 15, 10, 5]);
        }
        assert_eq!(shares(Method::RockSmash, Family::Hgss), [80, 20]);
        // Diamond and Pearl share the Platinum code.
        assert_eq!(
            shares(Method::GoodRod, Family::Dp),
            shares(Method::GoodRod, Family::Pt)
        );
        for family in [Family::Pt, Family::Hgss] {
            for method in [
                Method::Surf,
                Method::RockSmash,
                Method::OldRod,
                Method::GoodRod,
                Method::SuperRod,
            ] {
                let s = shares(method, family);
                assert!(s.is_empty() || s.iter().map(|&v| u32::from(v)).sum::<u32>() == 100);
            }
        }
    }

    #[test]
    fn replacements_name_the_slots_each_list_entry_replaces() {
        let slots = |family, list: &str, entry: usize| -> Vec<(Method, &'static [usize])> {
            replacements(family)
                .into_iter()
                .filter(|r| r.list == list && r.entry == entry)
                .map(|r| (r.method, r.slots))
                .collect()
        };
        let grass = |s: &'static [usize]| vec![(Method::Grass, s)];
        assert_eq!(slots(Family::Pt, "day", 0), grass(&[2]));
        assert_eq!(slots(Family::Pt, "day", 1), grass(&[3]));
        assert_eq!(slots(Family::Pt, "night", 1), grass(&[3]));
        assert_eq!(slots(Family::Pt, "swarm", 0), grass(&[0]));
        assert_eq!(slots(Family::Pt, "radar", 2), grass(&[10]));
        assert_eq!(slots(Family::Pt, "radar", 3), grass(&[11]));
        assert_eq!(slots(Family::Pt, "dual_emerald", 1), grass(&[9]));
        assert_eq!(slots(Family::Pt, "trophy_garden", 0), grass(&[6]));
        assert_eq!(slots(Family::Hgss, "radio_hoenn", 0), grass(&[2, 3]));
        assert_eq!(slots(Family::Hgss, "radio_sinnoh", 1), grass(&[4, 5]));
        assert_eq!(slots(Family::Hgss, "land_swarm", 0), grass(&[0, 1]));
        assert_eq!(
            slots(Family::Hgss, "night_fish", 0),
            vec![(Method::GoodRod, &[3][..]), (Method::SuperRod, &[1][..])]
        );
        assert!(slots(Family::Hgss, "day", 0).is_empty());
        // The time words name the hours.
        let day = replacements(Family::Pt)
            .into_iter()
            .find(|r| r.list == "day")
            .unwrap();
        assert_eq!(day.when, "from 10 to 19 o'clock");
        assert!(day.in_file);
        let garden = replacements(Family::Pt)
            .into_iter()
            .find(|r| r.list == "trophy_garden")
            .unwrap();
        assert!(!garden.in_file);
    }

    #[test]
    fn warnings_flag_level_zero_species_and_swapped_levels_only_when_rate_is_positive() {
        let mut e = pt_full_grass();
        e.set_grass(
            4,
            Slot {
                level: 0,
                species: 25,
            },
        );
        e.set_grass(
            5,
            Slot {
                level: 101,
                species: 25,
            },
        );
        e.set_surf_rate(10);
        let mut s = e.surf(1);
        s.set_species(54);
        s.set_min(30);
        s.set_max(20);
        e.set_surf(1, s);
        let enc = Encounters::Pt(e.clone());
        let w = warnings(&enc, Family::Pt);
        let key: Vec<(Method, Option<usize>)> = w.iter().map(|w| (w.method, w.slot)).collect();
        // Surf slots 0, 2, 3, 4 are fully empty: valid, no warning.
        assert_eq!(
            key,
            [
                (Method::Grass, Some(4)),
                (Method::Grass, Some(5)),
                (Method::Surf, Some(1))
            ]
        );
        assert_eq!(
            texts(&w),
            [
                "grass slot 4: species 25 at level 0",
                "grass slot 5: level 101 is above 100",
                "surf slot 1: min level 30 is above max level 20, the game swaps them",
            ]
        );

        // Rate 0: the method never runs, nothing is flagged.
        e.set_grass_rate(0);
        e.set_surf_rate(0);
        assert!(warnings(&Encounters::Pt(e), Family::Pt).is_empty());

        // The same for HeartGold: a water level 0 and a grass level 0.
        let mut h = hgss_full_grass();
        h.set_grass_level(7, 0);
        h.set_old_rod_rate(25);
        let mut s = h.old_rod(0);
        s.set_species(129);
        s.set_min(0);
        s.set_max(10);
        h.set_old_rod(0, s);
        let w = warnings(&Encounters::Hgss(h.clone()), Family::Hgss);
        assert_eq!(
            texts(&w),
            [
                "grass slot 7: species 1 at level 0",
                "old rod slot 0: species 129 at level 0",
            ]
        );
        h.set_grass_rate(0);
        h.set_old_rod_rate(0);
        assert!(warnings(&Encounters::Hgss(h), Family::Hgss).is_empty());
    }

    #[test]
    fn empty_replacement_entries_are_flagged_in_every_list_the_game_uses() {
        // A fully set file has no warnings.
        assert!(warnings(&Encounters::Pt(pt_full_grass()), Family::Pt).is_empty());
        assert!(warnings(&Encounters::Hgss(hgss_full_grass()), Family::Hgss).is_empty());

        let mut e = pt_full_grass();
        e.set_swarm(1, 0);
        e.set_night(0, 0);
        let w = warnings(&Encounters::Pt(e.clone()), Family::Pt);
        assert_eq!(
            texts(&w),
            [
                "night entry 0 is empty: from 20 to 3 o'clock, grass slot 2 gives a glitch Pokémon",
                "swarm entry 1 is empty: during a swarm, grass slot 1 gives a glitch Pokémon",
            ]
        );
        assert_eq!(w[0].slot, Some(2));
        assert_eq!(w[1].slot, Some(1));
        // Without a grass rate nothing replaces anything.
        e.set_grass_rate(0);
        assert!(warnings(&Encounters::Pt(e), Family::Pt).is_empty());

        // HeartGold: each of the three time columns, and the radio list.
        let mut h = hgss_full_grass();
        h.set_grass_morning(3, 0);
        h.set_grass_night(11, 0);
        h.set_radio(1, 0);
        let w = warnings(&Encounters::Hgss(h), Family::Hgss);
        assert_eq!(
            texts(&w),
            [
                "grass slot 3: the morning species is empty: from 4 to 9 o'clock it gives a glitch Pokémon",
                "grass slot 11: the night species is empty: from 20 to 3 o'clock it gives a glitch Pokémon",
                "radio_hoenn entry 1 is empty: with the Hoenn sound on the radio, grass slot 4 gives a glitch Pokémon",
                "radio_hoenn entry 1 is empty: with the Hoenn sound on the radio, grass slot 5 gives a glitch Pokémon",
            ]
        );

        // A new empty file with a rate but nothing else: empty slots are valid, no warning.
        let mut new = pt();
        new.set_grass_rate(100);
        assert!(warnings(&Encounters::Pt(new), Family::Pt).is_empty());
    }

    #[test]
    fn warnings_carry_their_check_code_key_and_args() {
        let mut e = pt_full_grass();
        e.set_grass(
            4,
            Slot {
                level: 0,
                species: 25,
            },
        );
        e.set_grass(
            5,
            Slot {
                level: 101,
                species: 25,
            },
        );
        e.set_swarm(1, 0);
        e.set_surf_rate(10);
        let mut s = e.surf(1);
        s.set_species(54);
        s.set_min(30);
        s.set_max(20);
        e.set_surf(1, s);
        e.set_good_rod_rate(300);
        let w = warnings(&Encounters::Pt(e), Family::Pt);
        let got: Vec<(&str, &str, Vec<&str>)> = w
            .iter()
            .map(|w| {
                (
                    w.code,
                    w.key.as_str(),
                    w.args.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (
                    "encounter.level-zero",
                    "slot:grass:4",
                    vec!["grass", "4", "25"]
                ),
                (
                    "encounter.level-high",
                    "slot:grass:5",
                    vec!["grass", "5", "101"]
                ),
                (
                    "encounter.min-max",
                    "slot:surf:1",
                    vec!["surf", "1", "30", "20"]
                ),
                (
                    "encounter.rate-byte",
                    "rate:good_rod",
                    vec!["good_rod", "300", "44"]
                ),
                (
                    "encounter.empty-slot",
                    "slot:grass:1:swarm:1",
                    vec!["grass", "1", "swarm", "1"]
                ),
            ]
        );

        // HeartGold: an empty time column is an empty slot too.
        let mut h = hgss_full_grass();
        h.set_grass_morning(3, 0);
        let w = warnings(&Encounters::Hgss(h), Family::Hgss);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].code, "encounter.empty-slot");
        assert_eq!(w[0].key, "slot:grass:3:morning");
        assert_eq!(w[0].args, ["grass", "3", "morning", ""]);
    }

    #[test]
    fn platinum_rates_above_255_wrap() {
        let mut e = pt_full_grass();
        e.set_grass_rate(300);
        e.set_surf_rate(256);
        let w = warnings(&Encounters::Pt(e), Family::Pt);
        assert_eq!(
            texts(&w),
            [
                "grass rate 300 acts like 44",
                "surf rate 256 acts like 0: the method never runs",
            ]
        );
        assert_eq!((w[0].method, w[0].slot), (Method::Grass, None));
    }

    #[test]
    fn effective_species_per_time_of_day() {
        let mut e = pt_full_grass();
        e.set_grass(
            2,
            Slot {
                level: 5,
                species: 10,
            },
        );
        e.set_day(0, 11);
        e.set_night(0, 12);
        let enc = Encounters::Pt(e);
        assert_eq!(
            effective_species(&enc, Family::Pt, 2),
            [("morning", 10), ("day", 11), ("night", 12)]
        );
        assert_eq!(
            effective_species(&enc, Family::Pt, 0),
            [("morning", 1), ("day", 1), ("night", 1)]
        );
        assert!(effective_species(&enc, Family::Pt, 12).is_empty());

        let mut h = hgss_full_grass();
        h.set_grass_morning(5, 20);
        h.set_grass_night(5, 21);
        let enc = Encounters::Hgss(h);
        assert_eq!(
            effective_species(&enc, Family::Hgss, 5),
            [("morning", 20), ("day", 1), ("night", 21)]
        );
    }

    #[test]
    fn effective_shares_add_up_the_grass_slots_per_time_of_day() {
        let mut e = pt_full_grass();
        e.set_grass(
            0,
            Slot {
                level: 5,
                species: 7,
            },
        );
        e.set_day(0, 7);
        let enc = Encounters::Pt(e);
        let all = effective_shares(&enc, Family::Pt);
        let when: Vec<&str> = all.iter().map(|t| t.when).collect();
        assert_eq!(when, ["morning", "day", "night"]);
        // Morning: slot 0 (20) is species 7, the rest species 1.
        assert_eq!(all[0].species, [(1, 80), (7, 20)]);
        // Day: slot 2 (10) is species 7 too.
        assert_eq!(all[1].species, [(1, 70), (7, 30)]);
        for t in &all {
            assert_eq!(
                t.species.iter().map(|&(_, s)| u32::from(s)).sum::<u32>(),
                100
            );
        }
        // Species 0 (empty slots) is left out.
        let empty = effective_shares(&Encounters::Pt(pt()), Family::Pt);
        assert!(empty.iter().all(|t| t.species.is_empty()));
    }

    /// Gated (`G4_TEST_PROJECT`): vanilla files break no rule, so every
    /// warning the editor shows points at an edit.
    #[test]
    fn vanilla_encounter_files_raise_no_warnings() {
        let Some(root) = crate::test_support::vanilla_project() else {
            return;
        };
        let dir = root.join("unpacked/encounters");
        let mut files = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            let family = match bytes.len() {
                424 => Family::Pt,
                196 => Family::Hgss,
                _ => continue,
            };
            let enc = Encounters::read(&bytes, family).unwrap();
            assert_eq!(warnings(&enc, family), [], "file {files}");
            files += 1;
        }
        assert!(files > 100, "{files} encounter files in {}", dir.display());
    }

    #[test]
    fn hgss_water_levels_are_checked_in_their_byte_order() {
        let mut e = hgss();
        e.set_surf_rate(5);
        e.set_surf_swarm(9);
        e.set_surf(0, WaterSlot::from_hgss_bytes([5, 3, 9, 0]));
        let w = warnings(&Encounters::Hgss(e), Family::Hgss);
        assert_eq!(
            texts(&w),
            ["surf slot 0: min level 5 is above max level 3, the game swaps them"]
        );
    }
}
