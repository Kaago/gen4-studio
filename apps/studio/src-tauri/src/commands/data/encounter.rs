//! Encounter files for the data commands: the methods, replacement lists, extras, warnings
//! and grass shares as DTOs (the rules from `g4_kernel::data::encounter_rules`), one field
//! change and the swap of two slots, with every index and value checked first (the codec's
//! accessors panic on an index outside their list).

use std::path::PathBuf;

use g4_formats::data::encounter::{
    DualSlot, Encounters, HgssEncounters, Method, PtEncounters, WaterSlot,
};
use g4_formats::game::Family;
use g4_kernel::data::encounter_rules::{self, Replacement};
use g4_kernel::text::{Problem, Severity};

use super::dto::{
    EncounterExtrasDto, MethodDto, ReplacedByDto, ReplacementListDto, RuleWarningDto, SlotDto,
    SlotTargetDto, SpeciesShareDto, TimeSharesDto,
};
use super::labels;
use crate::commands::ProblemDto;

/// The key of a method in DTOs, patches and `fieldLabels`.
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

/// The methods of the file in editor order, each slot with its share and the replacement
/// entries that can take its place.
pub fn methods(e: &Encounters, family: Family) -> Vec<MethodDto> {
    let reps = encounter_rules::replacements(family);
    e.methods()
        .into_iter()
        .map(|m| {
            let shares = encounter_rules::shares(m.method, family);
            MethodDto {
                method: method_key(m.method),
                rate: m.rate,
                slots: m
                    .slots
                    .iter()
                    .map(|s| SlotDto {
                        // At most 12 slots.
                        index: s.index as u16,
                        species: s.species,
                        min: s.min,
                        max: s.max,
                        species_morning: s.alt_species.map(|[morning, _]| morning),
                        species_night: s.alt_species.map(|[_, night]| night),
                        share: shares.get(s.index).copied().unwrap_or(0),
                        replaced_by: reps
                            .iter()
                            .filter(|r| r.method == m.method && r.slots.contains(&s.index))
                            .map(|r| ReplacedByDto {
                                list: r.list,
                                // Lists hold at most four entries.
                                entry: r.entry as u16,
                                when: r.when,
                                in_file: r.in_file,
                            })
                            .collect(),
                        replaces: Vec::new(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The German label of a replacement list key.
pub fn list_label(list: &str) -> &'static str {
    labels::label(labels::ENCOUNTER, list).unwrap_or("Ersatzliste")
}

/// The replacement lists the file stores, in the kernel's order, each entry as a slot with
/// the slots it replaces (Trophy Garden and Great Marsh are not in the file and not listed;
/// they show in `replaced_by` of the slots).
pub fn replacements(e: &Encounters, family: Family) -> Vec<ReplacementListDto> {
    let methods = e.methods();
    let mut out: Vec<ReplacementListDto> = Vec::new();
    let mut entries: Vec<(&'static str, usize, Vec<Replacement>)> = Vec::new();
    for r in encounter_rules::replacements(family)
        .into_iter()
        .filter(|r| r.in_file)
    {
        match entries
            .iter_mut()
            .find(|(list, entry, _)| *list == r.list && *entry == r.entry)
        {
            Some((_, _, targets)) => targets.push(r),
            None => entries.push((r.list, r.entry, vec![r])),
        }
    }
    for (list, entry, targets) in entries {
        let Some(species) = encounter_rules::list_species(e, list, entry) else {
            continue;
        };
        let first = targets[0];
        let shares = encounter_rules::shares(first.method, family);
        let share = first
            .slots
            .iter()
            .map(|&s| shares.get(s).copied().unwrap_or(0))
            .sum();
        let level = methods
            .iter()
            .find(|m| m.method == first.method)
            .and_then(|m| first.slots.first().and_then(|&s| m.slots.get(s)))
            .map_or((0, 0), |s| (s.min, s.max));
        let slot = SlotDto {
            // Lists hold at most four entries.
            index: entry as u16,
            species,
            min: level.0,
            max: level.1,
            species_morning: None,
            species_night: None,
            share,
            replaced_by: Vec::new(),
            replaces: targets
                .iter()
                .flat_map(|r| {
                    r.slots.iter().map(|&slot| SlotTargetDto {
                        method: method_key(r.method),
                        // At most 12 slots.
                        slot: slot as u16,
                    })
                })
                .collect(),
        };
        match out.iter_mut().find(|l| l.list == list) {
            Some(l) => l.slots.push(slot),
            None => out.push(ReplacementListDto {
                list,
                label: list_label(list),
                when: first.when,
                slots: vec![slot],
            }),
        }
    }
    out
}

/// The Platinum form rates and Unown table; nothing in HeartGold.
pub fn extras(e: &Encounters) -> EncounterExtrasDto {
    match e {
        Encounters::Pt(p) => EncounterExtrasDto {
            form_rates: Some(p.form_rates()),
            unown_table: Some(p.unown_table()),
        },
        Encounters::Hgss(_) => EncounterExtrasDto::default(),
    }
}

/// The rules the file breaks.
pub fn warnings(e: &Encounters, family: Family) -> Vec<RuleWarningDto> {
    encounter_rules::warnings(e, family)
        .into_iter()
        .map(|w| RuleWarningDto {
            method: method_key(w.method),
            // At most 12 slots.
            slot: w.slot.map(|s| s as u16),
            text: w.text,
        })
        .collect()
}

/// The file of encounter file `index` in the Problems list.
pub fn problem_file(index: u16) -> String {
    format!("encounters/{index:04}")
}

/// The warnings as Problems rows of source `encounter` (line = slot + 1, 1 for the method).
pub fn problems(index: u16, warnings: &[RuleWarningDto]) -> Vec<ProblemDto> {
    let file = problem_file(index);
    warnings
        .iter()
        .map(|w| {
            let problem = Problem {
                severity: Severity::Warning,
                file: PathBuf::from(&file),
                line: w.slot.map_or(1, |s| u32::from(s) + 1),
                col: 1,
                message: w.text.clone(),
            };
            ProblemDto::from_kernel(&problem, std::path::Path::new(""), "encounter")
        })
        .collect()
}

/// Who waits in the grass per time of day.
pub fn effective(e: &Encounters, family: Family) -> Vec<TimeSharesDto> {
    encounter_rules::effective_shares(e, family)
        .into_iter()
        .map(|t| TimeSharesDto {
            when: t.when,
            species: t
                .species
                .into_iter()
                .map(|(species, share)| SpeciesShareDto { species, share })
                .collect(),
        })
        .collect()
}

const DUAL_LISTS: [(&str, DualSlot); 5] = [
    ("dual_ruby", DualSlot::Ruby),
    ("dual_sapphire", DualSlot::Sapphire),
    ("dual_emerald", DualSlot::Emerald),
    ("dual_firered", DualSlot::FireRed),
    ("dual_leafgreen", DualSlot::LeafGreen),
];

type PtRate = (fn(&PtEncounters) -> u32, fn(&mut PtEncounters, u32));
type PtWater = (
    fn(&PtEncounters, usize) -> WaterSlot,
    fn(&mut PtEncounters, usize, WaterSlot),
);
type HgssRate = (fn(&HgssEncounters) -> u8, fn(&mut HgssEncounters, u8));
type HgssWater = (
    fn(&HgssEncounters, usize) -> WaterSlot,
    fn(&mut HgssEncounters, usize, WaterSlot),
);

/// The rate and slot accessors of a Platinum water method (five slots each).
fn pt_water(method: &str) -> Option<(PtRate, PtWater)> {
    match method {
        "surf" => Some((
            (PtEncounters::surf_rate, PtEncounters::set_surf_rate),
            (PtEncounters::surf, PtEncounters::set_surf),
        )),
        "old_rod" => Some((
            (PtEncounters::old_rod_rate, PtEncounters::set_old_rod_rate),
            (PtEncounters::old_rod, PtEncounters::set_old_rod),
        )),
        "good_rod" => Some((
            (PtEncounters::good_rod_rate, PtEncounters::set_good_rod_rate),
            (PtEncounters::good_rod, PtEncounters::set_good_rod),
        )),
        "super_rod" => Some((
            (
                PtEncounters::super_rod_rate,
                PtEncounters::set_super_rod_rate,
            ),
            (PtEncounters::super_rod, PtEncounters::set_super_rod),
        )),
        _ => None,
    }
}

/// The rate and slot accessors of a HeartGold water or rock method, with its slot count.
fn hgss_water(method: &str) -> Option<(HgssRate, HgssWater, usize)> {
    match method {
        "surf" => Some((
            (HgssEncounters::surf_rate, HgssEncounters::set_surf_rate),
            (HgssEncounters::surf, HgssEncounters::set_surf),
            5,
        )),
        "rock_smash" => Some((
            (HgssEncounters::rock_rate, HgssEncounters::set_rock_rate),
            (HgssEncounters::rock, HgssEncounters::set_rock),
            2,
        )),
        "old_rod" => Some((
            (
                HgssEncounters::old_rod_rate,
                HgssEncounters::set_old_rod_rate,
            ),
            (HgssEncounters::old_rod, HgssEncounters::set_old_rod),
            5,
        )),
        "good_rod" => Some((
            (
                HgssEncounters::good_rod_rate,
                HgssEncounters::set_good_rod_rate,
            ),
            (HgssEncounters::good_rod, HgssEncounters::set_good_rod),
            5,
        )),
        "super_rod" => Some((
            (
                HgssEncounters::super_rod_rate,
                HgssEncounters::set_super_rod_rate,
            ),
            (HgssEncounters::super_rod, HgssEncounters::set_super_rod),
            5,
        )),
        _ => None,
    }
}

/// Exchanges the values `get` reads at `a` and `b` of `record`.
fn exchange<R, T>(
    record: &mut R,
    a: usize,
    b: usize,
    get: impl Fn(&R, usize) -> T,
    set: impl Fn(&mut R, usize, T),
) {
    let (at_a, at_b) = (get(record, a), get(record, b));
    set(record, a, at_b);
    set(record, b, at_a);
}

/// Slots `a` and `b` of `method` (a method of the file, see [`method_key`]) trade places:
/// species and levels, a HeartGold grass slot with all three times of day. The bytes the
/// views do not show (Platinum padding, the high species word) stay with their place. Refused,
/// leaving `e` as it was, when `a == b`, a slot lies outside the method or `method` is no
/// method of the file (a replacement list: its entries take the place of slots, they have none
/// of their own). An empty slot (species 0) swaps like any other.
pub fn swap(e: &mut Encounters, method: &str, a: u16, b: u16) -> Result<(), String> {
    let methods = e.methods();
    let Some(view) = methods.iter().find(|m| method_key(m.method) == method) else {
        let keys: Vec<&str> = methods.iter().map(|m| method_key(m.method)).collect();
        return Err(format!(
            "{method} is no method of this file; slots swap within {}",
            keys.join(", ")
        ));
    };
    if a == b {
        return Err(format!("{method} slot {a} cannot swap with itself"));
    }
    let count = view.slots.len();
    if let Some(bad) = [a, b].into_iter().find(|&s| usize::from(s) >= count) {
        return Err(format!("{method} has {count} slots, none at {bad}"));
    }
    let (a, b) = (usize::from(a), usize::from(b));
    match e {
        Encounters::Pt(p) => match pt_water(method) {
            Some((_, (get, set))) => exchange(p, a, b, get, set),
            None => exchange(p, a, b, PtEncounters::grass, PtEncounters::set_grass),
        },
        Encounters::Hgss(h) => match hgss_water(method) {
            Some((_, (get, set), _)) => exchange(h, a, b, get, set),
            None => {
                type Column = (
                    fn(&HgssEncounters, usize) -> u16,
                    fn(&mut HgssEncounters, usize, u16),
                );
                let columns: [Column; 3] = [
                    (
                        HgssEncounters::grass_morning,
                        HgssEncounters::set_grass_morning,
                    ),
                    (HgssEncounters::grass_day, HgssEncounters::set_grass_day),
                    (HgssEncounters::grass_night, HgssEncounters::set_grass_night),
                ];
                for (get, set) in columns {
                    exchange(h, a, b, get, set);
                }
                exchange(
                    h,
                    a,
                    b,
                    HgssEncounters::grass_level,
                    HgssEncounters::set_grass_level,
                );
            }
        },
    }
    Ok(())
}

/// One encounter change as `DataPatch::Encounter` names it.
pub struct EncounterEdit<'a> {
    pub method: &'a str,
    pub slot: Option<u16>,
    pub field: &'a str,
    pub value: i64,
}

fn narrow<T: TryFrom<i64>>(value: i64, field: &str) -> Result<T, String> {
    T::try_from(value).map_err(|_| format!("{value} does not fit {field}"))
}

impl EncounterEdit<'_> {
    /// The slot index, checked against the `count` slots of the list.
    fn slot(&self, count: usize) -> Result<usize, String> {
        let slot = self
            .slot
            .ok_or_else(|| format!("{} {} needs a slot", self.method, self.field))?;
        let slot = usize::from(slot);
        if slot < count {
            Ok(slot)
        } else {
            Err(format!("{} has {count} slots, none at {slot}", self.method))
        }
    }

    fn unknown_field(&self) -> String {
        format!("{} has no field {}", self.method, self.field)
    }

    /// A field of a water slot (min, max, species). A `min` above the slot's `max` raises
    /// `max` to it, a `max` below its `min` lowers `min`: the levels are pulled along on
    /// input (the bytes may hold `min > max`, which is only warned about).
    fn water(&self, slot: &mut WaterSlot) -> Result<i64, String> {
        let old = match self.field {
            "min" => {
                let old = slot.min();
                let min = narrow(self.value, self.field)?;
                slot.set_min(min);
                if min > slot.max() {
                    slot.set_max(min);
                }
                i64::from(old)
            }
            "max" => {
                let old = slot.max();
                let max = narrow(self.value, self.field)?;
                slot.set_max(max);
                if max < slot.min() {
                    slot.set_min(max);
                }
                i64::from(old)
            }
            "species" => {
                let old = slot.species();
                slot.set_species(narrow(self.value, self.field)?);
                i64::from(old)
            }
            _ => return Err(self.unknown_field()),
        };
        Ok(old)
    }

    /// Entry `slot` of a species list of `count` entries, read by `get` and written by `set`.
    fn species_entry<R>(
        &self,
        record: &mut R,
        count: usize,
        get: impl Fn(&R, usize) -> u16,
        set: impl Fn(&mut R, usize, u16),
    ) -> Result<i64, String> {
        if self.field != "species" {
            return Err(self.unknown_field());
        }
        // A list of one entry (HeartGold swarms) may be named without a slot.
        let slot = if count == 1 && self.slot.is_none() {
            0
        } else {
            self.slot(count)?
        };
        let old = get(record, slot);
        set(record, slot, narrow(self.value, self.field)?);
        Ok(i64::from(old))
    }

    /// Applies the change to `e` and returns the old value; an unknown method, field or slot
    /// or a value out of range is an error and leaves `e` as it was.
    pub fn apply(&self, e: &mut Encounters) -> Result<i64, String> {
        let mut copy = e.clone();
        let old = match &mut copy {
            Encounters::Pt(p) => self.apply_pt(p)?,
            Encounters::Hgss(h) => self.apply_hgss(h)?,
        };
        *e = copy;
        Ok(old)
    }

    fn apply_pt(&self, p: &mut PtEncounters) -> Result<i64, String> {
        if let Some(((rate, set_rate), (get, set))) = pt_water(self.method) {
            if self.field == "rate" {
                let old = rate(p);
                set_rate(p, narrow(self.value, self.field)?);
                return Ok(i64::from(old));
            }
            let slot = self.slot(5)?;
            let mut s = get(p, slot);
            let old = self.water(&mut s)?;
            set(p, slot, s);
            return Ok(old);
        }
        if let Some(&(_, game)) = DUAL_LISTS.iter().find(|(key, _)| *key == self.method) {
            return self.species_entry(
                p,
                2,
                |p, i| p.dual(game, i),
                |p, i, v| p.set_dual(game, i, v),
            );
        }
        match self.method {
            "grass" => match self.field {
                "rate" => {
                    let old = p.grass_rate();
                    p.set_grass_rate(narrow(self.value, self.field)?);
                    Ok(i64::from(old))
                }
                "species" | "level" | "min" | "max" => {
                    let slot = self.slot(12)?;
                    let mut s = p.grass(slot);
                    let old = if self.field == "species" {
                        let old = s.species;
                        s.species = narrow(self.value, self.field)?;
                        i64::from(old)
                    } else {
                        let old = s.level;
                        s.level = narrow(self.value, self.field)?;
                        i64::from(old)
                    };
                    p.set_grass(slot, s);
                    Ok(old)
                }
                _ => Err(self.unknown_field()),
            },
            "swarm" => self.species_entry(p, 2, PtEncounters::swarm, PtEncounters::set_swarm),
            "day" => self.species_entry(p, 2, PtEncounters::day, PtEncounters::set_day),
            "night" => self.species_entry(p, 2, PtEncounters::night, PtEncounters::set_night),
            "radar" => self.species_entry(p, 4, PtEncounters::radar, PtEncounters::set_radar),
            other => Err(format!("Platinum encounter files have no method {other}")),
        }
    }

    fn apply_hgss(&self, h: &mut HgssEncounters) -> Result<i64, String> {
        if let Some(((rate, set_rate), (get, set), count)) = hgss_water(self.method) {
            if self.field == "rate" {
                let old = rate(h);
                set_rate(h, narrow(self.value, self.field)?);
                return Ok(i64::from(old));
            }
            let slot = self.slot(count)?;
            let mut s = get(h, slot);
            let old = self.water(&mut s)?;
            set(h, slot, s);
            return Ok(old);
        }
        match self.method {
            "grass" => {
                if self.field == "rate" {
                    let old = h.grass_rate();
                    h.set_grass_rate(narrow(self.value, self.field)?);
                    return Ok(i64::from(old));
                }
                let slot = self.slot(12)?;
                let old = match self.field {
                    "level" | "min" | "max" => {
                        let old = h.grass_level(slot);
                        h.set_grass_level(slot, narrow(self.value, self.field)?);
                        old.into()
                    }
                    "species" => {
                        let old = h.grass_day(slot);
                        h.set_grass_day(slot, narrow(self.value, self.field)?);
                        old.into()
                    }
                    "species_morning" => {
                        let old = h.grass_morning(slot);
                        h.set_grass_morning(slot, narrow(self.value, self.field)?);
                        old.into()
                    }
                    "species_night" => {
                        let old = h.grass_night(slot);
                        h.set_grass_night(slot, narrow(self.value, self.field)?);
                        old.into()
                    }
                    _ => return Err(self.unknown_field()),
                };
                Ok(old)
            }
            "radio" => self.species_entry(h, 4, HgssEncounters::radio, HgssEncounters::set_radio),
            // The kernel's names of the two radio lists: entries 0, 1 and 2, 3 of `radio`.
            "radio_hoenn" => {
                self.species_entry(h, 2, HgssEncounters::radio, HgssEncounters::set_radio)
            }
            "radio_sinnoh" => {
                self.species_entry(h, 2, |h, i| h.radio(2 + i), |h, i, v| h.set_radio(2 + i, v))
            }
            "land_swarm" => {
                self.species_entry(h, 1, |h, _| h.land_swarm(), |h, _, v| h.set_land_swarm(v))
            }
            "surf_swarm" => {
                self.species_entry(h, 1, |h, _| h.surf_swarm(), |h, _, v| h.set_surf_swarm(v))
            }
            "night_fish" => {
                self.species_entry(h, 1, |h, _| h.night_fish(), |h, _, v| h.set_night_fish(v))
            }
            "fish_swarm" => {
                self.species_entry(h, 1, |h, _| h.fish_swarm(), |h, _, v| h.set_fish_swarm(v))
            }
            other => Err(format!("HeartGold encounter files have no method {other}")),
        }
    }
}

/// Changes a Platinum value that is no slot (`form_rate_0` to `form_rate_4`,
/// `unown_table`) and returns the old value; HeartGold has none.
pub fn apply_extra(e: &mut Encounters, field: &str, value: i64) -> Result<i64, String> {
    let Encounters::Pt(p) = e else {
        return Err(format!("HeartGold encounter files have no field {field}"));
    };
    let value: u32 = narrow(value, field)?;
    if field == "unown_table" {
        let old = p.unown_table();
        p.set_unown_table(value);
        return Ok(i64::from(old));
    }
    let n = field
        .strip_prefix("form_rate_")
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|&n| n < 5)
        .ok_or_else(|| format!("Platinum encounter files have no field {field}"))?;
    let mut rates = p.form_rates();
    let old = rates[n];
    rates[n] = value;
    p.set_form_rates(rates);
    Ok(i64::from(old))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_formats::game::Family;

    fn edit<'a>(
        method: &'a str,
        slot: Option<u16>,
        field: &'a str,
        value: i64,
    ) -> EncounterEdit<'a> {
        EncounterEdit {
            method,
            slot,
            field,
            value,
        }
    }

    #[test]
    fn out_of_range_slots_methods_and_values_are_errors_not_panics() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        let before = pt.clone();
        for bad in [
            edit("grass", Some(12), "species", 1),
            edit("surf", Some(5), "min", 1),
            edit("radar", Some(4), "species", 1),
            edit("grass", None, "species", 1),
            edit("rock_smash", Some(0), "species", 1),
            edit("grass", Some(0), "level", 256),
            edit("grass", Some(0), "species", -1),
            edit("surf", Some(0), "colour", 1),
            edit("radio", Some(0), "species", 1),
        ] {
            assert!(
                bad.apply(&mut pt).is_err(),
                "{} {:?} {}",
                bad.method,
                bad.slot,
                bad.field
            );
        }
        assert_eq!(pt, before);
        let mut hg = Encounters::read(&[0; 196], Family::Hgss).unwrap();
        assert!(
            edit("rock_smash", Some(2), "species", 1)
                .apply(&mut hg)
                .is_err()
        );
        assert!(edit("grass", Some(0), "rate", 256).apply(&mut hg).is_err());
        assert!(
            edit("land_swarm", Some(1), "species", 1)
                .apply(&mut hg)
                .is_err()
        );
    }

    #[test]
    fn changes_land_where_the_view_reads_them() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        assert_eq!(edit("grass", Some(2), "level", 7).apply(&mut pt), Ok(0));
        assert_eq!(edit("grass", Some(2), "species", 396).apply(&mut pt), Ok(0));
        assert_eq!(edit("super_rod", Some(4), "max", 50).apply(&mut pt), Ok(0));
        assert_eq!(edit("super_rod", None, "rate", 30).apply(&mut pt), Ok(0));
        assert_eq!(
            edit("dual_emerald", Some(1), "species", 9).apply(&mut pt),
            Ok(0)
        );
        let m = methods(&pt, Family::Pt);
        assert_eq!((m[0].slots[2].species, m[0].slots[2].min), (396, 7));
        assert_eq!(
            (m[4].method, m[4].rate, m[4].slots[4].max),
            ("super_rod", 30, 50)
        );
        let x = replacements(&pt, Family::Pt);
        let species =
            |list: &ReplacementListDto| list.slots.iter().map(|s| s.species).collect::<Vec<_>>();
        assert_eq!(
            species(x.iter().find(|l| l.list == "dual_emerald").unwrap()),
            [0, 9]
        );

        let mut hg = Encounters::read(&[0; 196], Family::Hgss).unwrap();
        assert_eq!(
            edit("grass", Some(0), "species_night", 5).apply(&mut hg),
            Ok(0)
        );
        assert_eq!(edit("land_swarm", None, "species", 6).apply(&mut hg), Ok(0));
        assert_eq!(
            edit("rock_smash", Some(1), "species", 74).apply(&mut hg),
            Ok(0)
        );
        assert_eq!(
            edit("radio_sinnoh", Some(1), "species", 7).apply(&mut hg),
            Ok(0)
        );
        let m = methods(&hg, Family::Hgss);
        assert_eq!(m[0].slots[0].species_night, Some(5));
        assert_eq!((m[2].method, m[2].slots[1].species), ("rock_smash", 74));
        let x = replacements(&hg, Family::Hgss);
        let land = x.iter().find(|l| l.list == "land_swarm").unwrap();
        assert_eq!(species(land), [6]);
        let sinnoh = x.iter().find(|l| l.list == "radio_sinnoh").unwrap();
        assert_eq!(species(sinnoh), [0, 7]);
        assert_eq!(sinnoh.label, "Radio Sinnoh");
        let SlotDto { replaces, .. } = &sinnoh.slots[1];
        let targets: Vec<_> = replaces.iter().map(|t| (t.method, t.slot)).collect();
        assert_eq!(targets, [("grass", 4), ("grass", 5)]);
    }

    #[test]
    fn slots_carry_shares_and_their_replacements() {
        let pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        let m = methods(&pt, Family::Pt);
        let shares: Vec<u8> = m[0].slots.iter().map(|s| s.share).collect();
        assert_eq!(shares, [20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1]);
        let by: Vec<_> = m[0].slots[2]
            .replaced_by
            .iter()
            .map(|r| (r.list, r.entry))
            .collect();
        assert_eq!(by, [("day", 0), ("night", 0)]);
        // Trophy Garden is not in the file: it replaces slot 6 but is no list of the file.
        assert!(
            m[0].slots[6]
                .replaced_by
                .iter()
                .any(|r| r.list == "trophy_garden" && !r.in_file)
        );
        let lists = replacements(&pt, Family::Pt);
        assert!(lists.iter().all(|l| l.list != "trophy_garden"));
        let radar = lists.iter().find(|l| l.list == "radar").unwrap();
        assert_eq!(
            (radar.label, radar.slots.len(), radar.slots[2].share),
            ("Poké-Radar", 4, 1)
        );
        assert_eq!(extras(&pt).form_rates, Some([0; 5]));
    }

    #[test]
    fn water_levels_are_pulled_along_on_input() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        let slot = |e: &Encounters| {
            let m = methods(e, Family::Pt);
            (m[1].slots[0].min, m[1].slots[0].max)
        };
        edit("surf", Some(0), "max", 10).apply(&mut pt).unwrap();
        assert_eq!(slot(&pt), (0, 10));
        // A min above max takes max along.
        assert_eq!(edit("surf", Some(0), "min", 20).apply(&mut pt), Ok(0));
        assert_eq!(slot(&pt), (20, 20));
        // A max below min takes min along.
        assert_eq!(edit("surf", Some(0), "max", 5).apply(&mut pt), Ok(20));
        assert_eq!(slot(&pt), (5, 5));
    }

    #[test]
    fn extras_are_platinum_only() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        assert_eq!(apply_extra(&mut pt, "form_rate_4", 3), Ok(0));
        assert_eq!(apply_extra(&mut pt, "unown_table", 2), Ok(0));
        assert_eq!(extras(&pt).form_rates, Some([0, 0, 0, 0, 3]));
        assert_eq!(extras(&pt).unown_table, Some(2));
        assert!(apply_extra(&mut pt, "form_rate_5", 1).is_err());
        assert!(apply_extra(&mut pt, "unown_table", -1).is_err());
        let mut hg = Encounters::read(&[0; 196], Family::Hgss).unwrap();
        assert!(apply_extra(&mut hg, "unown_table", 1).is_err());
        assert_eq!(extras(&hg), EncounterExtrasDto::default());
    }

    #[test]
    fn swap_with_itself_is_refused_before_writing() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        edit("grass", Some(2), "species", 396)
            .apply(&mut pt)
            .unwrap();
        let before = pt.clone();
        for (method, a, b) in [
            ("grass", 2, 2),
            ("grass", 2, 12),
            ("surf", 5, 0),
            ("rock_smash", 0, 1),
            ("day", 0, 1),
            ("swarm", 0, 1),
            ("radar", 0, 3),
            ("dual_ruby", 0, 1),
        ] {
            assert!(swap(&mut pt, method, a, b).is_err(), "{method} {a} {b}");
        }
        assert_eq!(pt, before);
        let err = swap(&mut pt, "grass", 2, 2).unwrap_err();
        assert!(err.contains("itself"), "{err}");
        let err = swap(&mut pt, "swarm", 0, 1).unwrap_err();
        assert!(
            err.contains("grass, surf, old_rod, good_rod, super_rod"),
            "{err}"
        );
        let mut hg = Encounters::read(&[0; 196], Family::Hgss).unwrap();
        let before = hg.clone();
        for (method, a, b) in [
            ("rock_smash", 0, 2),
            ("radio", 0, 1),
            ("radio_hoenn", 0, 1),
            ("land_swarm", 0, 1),
            ("night_fish", 0, 1),
        ] {
            assert!(swap(&mut hg, method, a, b).is_err(), "{method} {a} {b}");
        }
        assert_eq!(hg, before);
    }

    #[test]
    fn swap_of_slot_2_and_7_moves_species_and_levels_hgss_keeps_three_columns() {
        let mut pt = Encounters::read(&[0; 424], Family::Pt).unwrap();
        for (slot, species, level) in [(2, 396, 3), (7, 399, 12)] {
            edit("grass", Some(slot), "species", species)
                .apply(&mut pt)
                .unwrap();
            edit("grass", Some(slot), "level", level)
                .apply(&mut pt)
                .unwrap();
        }
        swap(&mut pt, "grass", 2, 7).unwrap();
        let grass = |e: &Encounters, slot: usize| {
            let s = methods(e, family_of(e))[0].slots[slot].clone();
            (s.species, s.min, s.max, s.species_morning, s.species_night)
        };
        assert_eq!(grass(&pt, 2), (399, 12, 12, None, None));
        assert_eq!(grass(&pt, 7), (396, 3, 3, None, None));
        // An empty target slot is allowed: the species moves, the empty place takes its spot.
        swap(&mut pt, "grass", 7, 11).unwrap();
        assert_eq!(grass(&pt, 11), (396, 3, 3, None, None));
        assert_eq!(grass(&pt, 7), (0, 0, 0, None, None));
        // Water: species, min and max move together.
        edit("surf", Some(0), "max", 20).apply(&mut pt).unwrap();
        edit("surf", Some(0), "min", 10).apply(&mut pt).unwrap();
        edit("surf", Some(0), "species", 54).apply(&mut pt).unwrap();
        swap(&mut pt, "surf", 4, 0).unwrap();
        let surf = |e: &Encounters, slot: usize| {
            let s = &methods(e, Family::Pt)[1].slots[slot];
            (s.species, s.min, s.max)
        };
        assert_eq!((surf(&pt, 0), surf(&pt, 4)), ((0, 0, 0), (54, 10, 20)));

        let mut hg = Encounters::read(&[0; 196], Family::Hgss).unwrap();
        for (slot, base, level) in [(2, 10, 5), (7, 20, 9)] {
            for (i, field) in ["species_morning", "species", "species_night"]
                .into_iter()
                .enumerate()
            {
                edit("grass", Some(slot), field, base + i as i64)
                    .apply(&mut hg)
                    .unwrap();
            }
            edit("grass", Some(slot), "level", level)
                .apply(&mut hg)
                .unwrap();
        }
        swap(&mut hg, "grass", 2, 7).unwrap();
        assert_eq!(grass(&hg, 2), (21, 9, 9, Some(20), Some(22)));
        assert_eq!(grass(&hg, 7), (11, 5, 5, Some(10), Some(12)));
        edit("rock_smash", Some(1), "species", 74)
            .apply(&mut hg)
            .unwrap();
        swap(&mut hg, "rock_smash", 0, 1).unwrap();
        let rock = &methods(&hg, Family::Hgss)[2];
        assert_eq!((rock.slots[0].species, rock.slots[1].species), (74, 0));
    }

    fn family_of(e: &Encounters) -> Family {
        match e {
            Encounters::Pt(_) => Family::Pt,
            Encounters::Hgss(_) => Family::Hgss,
        }
    }
}
