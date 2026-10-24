//! Encounter files for the data commands: the methods and replacement lists as DTOs, and one
//! field change with every index and value checked first (the codec's accessors panic on an
//! index outside their list).

use g4_formats::data::encounter::{
    DualSlot, Encounters, HgssEncounters, Method, PtEncounters, WaterSlot,
};

use super::dto::{ExtraListDto, MethodDto, SlotDto};

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

/// The methods of the file in editor order.
pub fn methods(e: &Encounters) -> Vec<MethodDto> {
    e.methods()
        .into_iter()
        .map(|m| MethodDto {
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
                })
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

/// The replacement lists of the file. Platinum's form rates and Unown table are only in the
/// raw data.
pub fn extras(e: &Encounters) -> Vec<ExtraListDto> {
    let list = |list: &'static str, species: Vec<u16>| ExtraListDto { list, species };
    match e {
        Encounters::Pt(p) => {
            let mut out = vec![
                list("swarm", (0..2).map(|i| p.swarm(i)).collect()),
                list("day", (0..2).map(|i| p.day(i)).collect()),
                list("night", (0..2).map(|i| p.night(i)).collect()),
                list("radar", (0..4).map(|i| p.radar(i)).collect()),
            ];
            out.extend(
                DUAL_LISTS
                    .iter()
                    .map(|&(key, game)| list(key, (0..2).map(|i| p.dual(game, i)).collect())),
            );
            out
        }
        Encounters::Hgss(h) => vec![
            list("radio", (0..4).map(|i| h.radio(i)).collect()),
            list("land_swarm", vec![h.land_swarm()]),
            list("surf_swarm", vec![h.surf_swarm()]),
            list("night_fish", vec![h.night_fish()]),
            list("fish_swarm", vec![h.fish_swarm()]),
        ],
    }
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

    /// A field of a water slot (min, max, species).
    fn water(&self, slot: &mut WaterSlot) -> Result<i64, String> {
        let old = match self.field {
            "min" => {
                let old = slot.min();
                slot.set_min(narrow(self.value, self.field)?);
                i64::from(old)
            }
            "max" => {
                let old = slot.max();
                slot.set_max(narrow(self.value, self.field)?);
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
        type Rate = (fn(&PtEncounters) -> u32, fn(&mut PtEncounters, u32));
        type Water = (
            fn(&PtEncounters, usize) -> WaterSlot,
            fn(&mut PtEncounters, usize, WaterSlot),
        );
        let water: Option<(Rate, Water)> = match self.method {
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
        };
        if let Some(((rate, set_rate), (get, set))) = water {
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
        type Rate = (fn(&HgssEncounters) -> u8, fn(&mut HgssEncounters, u8));
        type Water = (
            fn(&HgssEncounters, usize) -> WaterSlot,
            fn(&mut HgssEncounters, usize, WaterSlot),
        );
        let water: Option<(Rate, Water, usize)> = match self.method {
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
        };
        if let Some(((rate, set_rate), (get, set), count)) = water {
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
        let m = methods(&pt);
        assert_eq!((m[0].slots[2].species, m[0].slots[2].min), (396, 7));
        assert_eq!(
            (m[4].method, m[4].rate, m[4].slots[4].max),
            ("super_rod", 30, 50)
        );
        let x = extras(&pt);
        assert_eq!(
            x.iter().find(|l| l.list == "dual_emerald").unwrap().species,
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
        let m = methods(&hg);
        assert_eq!(m[0].slots[0].species_night, Some(5));
        assert_eq!((m[2].method, m[2].slots[1].species), ("rock_smash", 74));
        assert_eq!(extras(&hg)[1].species, [6]);
    }
}
