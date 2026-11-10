//! The ranges of the flag and variable numbers per family (`metadata/flag_ranges.yaml`):
//! which area a number lies in and which numbers the save block holds.

use std::collections::HashMap;
use std::sync::OnceLock;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::FlagKind;

const RANGES_YAML: &str = include_str!("../../metadata/flag_ranges.yaml");

/// What a range of numbers is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    MapLocal,
    General,
    HiddenItems,
    Trainer,
    System,
    Daily,
    Temp,
    ObjectGfx,
    ScriptLocal,
    /// In no range: flag 0, numbers past the save block, variables below `0x4000`.
    Outside,
}

impl Area {
    /// The word for the user interface, in the current language.
    pub fn word(self) -> String {
        let key = match self {
            Area::MapLocal => "flags.area_map_local",
            Area::General => "flags.area_general",
            Area::HiddenItems => "flags.area_hidden_items",
            Area::Trainer => "flags.area_trainer",
            Area::System => "flags.area_system",
            Area::Daily => "flags.area_daily",
            Area::Temp => "flags.area_temp",
            Area::ObjectGfx => "flags.area_object_gfx",
            Area::ScriptLocal => "flags.area_script_local",
            Area::Outside => "flags.area_outside",
        };
        t!(key).into_owned()
    }
}

/// A range of numbers, both ends inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub area: Area,
    pub start: u16,
    pub end: u16,
}

impl Range {
    fn contains(&self, n: u16) -> bool {
        (self.start..=self.end).contains(&n)
    }
}

/// The ranges of one family.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Ranges {
    pub flags: Vec<Range>,
    /// One past the last flag of the save block.
    pub flags_end: u16,
    pub vars: Vec<Range>,
    /// One past the last variable of the save block.
    pub vars_end: u16,
}

/// One entry of the file: its own ranges, or the ranges of another family.
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Like { like: String },
    Own(Ranges),
}

/// The parsed file per family (Dp, Pt, Hgss), read once per process; `dp` is `like: pt`.
fn table() -> &'static [Ranges; 3] {
    static TABLE: OnceLock<[Ranges; 3]> = OnceLock::new();
    TABLE.get_or_init(|| {
        // The file is embedded and parsed by the tests, so a parse error cannot reach a user.
        let file: HashMap<String, Entry> = serde_yaml::from_str(RANGES_YAML).unwrap_or_default();
        let empty = Ranges {
            flags: Vec::new(),
            flags_end: 0,
            vars: Vec::new(),
            vars_end: 0,
        };
        let own = |key: &str| match file.get(key) {
            Some(Entry::Own(r)) => r.clone(),
            _ => empty.clone(),
        };
        let pt = own("pt");
        let hgss = own("hgss");
        let dp = match file.get("dp") {
            Some(Entry::Like { like }) if like == "hgss" => hgss.clone(),
            Some(Entry::Like { .. }) => pt.clone(),
            _ => empty.clone(),
        };
        [dp, pt, hgss]
    })
}

/// The ranges of `family`; Diamond and Pearl are those of Platinum (not checked on a ROM).
pub fn ranges(family: Family) -> &'static Ranges {
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    &table()[slot]
}

fn of_kind(family: Family, kind: FlagKind) -> &'static [Range] {
    let r = ranges(family);
    match kind {
        FlagKind::Flag => &r.flags,
        FlagKind::Var => &r.vars,
    }
}

/// The area of `n`: the narrowest range that holds it, so the hidden items win over the general
/// flags they lie in; [`Area::Outside`] when no range does.
pub fn area_of(family: Family, kind: FlagKind, n: u16) -> Area {
    of_kind(family, kind)
        .iter()
        .filter(|r| r.contains(n))
        .min_by_key(|r| r.end - r.start)
        .map_or(Area::Outside, |r| r.area)
}

/// Whether `n` is one the save block holds: flags `1..flags_end`, variables `0x4000..vars_end`.
pub fn in_save(family: Family, kind: FlagKind, n: u16) -> bool {
    let r = ranges(family);
    match kind {
        FlagKind::Flag => (1..r.flags_end).contains(&n),
        FlagKind::Var => (0x4000..r.vars_end).contains(&n),
    }
}

/// The flag of the first hidden item (`hidden_items.start`); hidden item `n` has the flag
/// `base + n`.
pub fn hidden_items_base(family: Family) -> Option<u16> {
    of_kind(family, FlagKind::Flag)
        .iter()
        .find(|r| r.area == Area::HiddenItems)
        .map(|r| r.start)
}

/// The flag of hidden item `item` (an overworld background event with script `8000 + item`):
/// `hidden_items.start + item`, while it is in the range.
pub fn hidden_item_flag(family: Family, item: u16) -> Option<u16> {
    let range = of_kind(family, FlagKind::Flag)
        .iter()
        .find(|r| r.area == Area::HiddenItems)?;
    range
        .start
        .checked_add(item)
        .filter(|flag| *flag <= range.end)
}

/// The "defeated" flag of `trainer`: `trainer.start + trainer`, while it is in the range.
pub fn trainer_flag(family: Family, trainer: u16) -> Option<u16> {
    let range = of_kind(family, FlagKind::Flag)
        .iter()
        .find(|r| r.area == Area::Trainer)?;
    range
        .start
        .checked_add(trainer)
        .filter(|flag| *flag <= range.end)
}

/// The range new numbers are taken from: the [`Area::General`] range of `kind`.
pub fn pool_range(family: Family, kind: FlagKind) -> Option<Range> {
    of_kind(family, kind)
        .iter()
        .find(|r| r.area == Area::General)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::arm9::arm9_layout;
    use crate::flags::FlagKind::{Flag, Var};
    use g4_formats::game::Family::{Dp, Hgss, Pt};

    #[test]
    fn ranges_follow_the_decisions() {
        for (n, area) in [
            (0x20, Area::MapLocal),
            (0x41, Area::General),
            (0x2DA, Area::HiddenItems),
            (0x3F5, Area::HiddenItems),
            (0x3F6, Area::General),
            (0x550, Area::Trainer),
            (0x95F, Area::Trainer),
            (0x960, Area::System),
            (0xAA0, Area::Daily),
            (0xB5F, Area::Daily),
            (0xB60, Area::Outside),
            (0, Area::Outside),
        ] {
            assert_eq!(area_of(Pt, Flag, n), area, "pt flag {n:#X}");
        }
        for (n, area) in [
            (0x4030, Area::General),
            (0x411F, Area::General),
            (0x4120, Area::Outside),
            (0x8000, Area::ScriptLocal),
            (0x4000, Area::MapLocal),
            (0x4025, Area::ObjectGfx),
            (0x3FFF, Area::Outside),
        ] {
            assert_eq!(area_of(Pt, Var, n), area, "pt var {n:#X}");
        }
        assert_eq!(area_of(Hgss, Var, 0x4120), Area::General);
        assert_eq!(area_of(Hgss, Var, 0x416F), Area::General);
        assert_eq!(area_of(Hgss, Var, 0x4170), Area::Outside);
        assert_eq!(area_of(Hgss, Var, 0x800D), Area::ScriptLocal);
        assert_eq!(area_of(Hgss, Var, 0x800E), Area::Outside);
        assert_eq!(area_of(Hgss, Flag, 0x4000), Area::Temp);
        assert_eq!(area_of(Hgss, Flag, 0x3F), Area::MapLocal);
        assert_eq!(area_of(Hgss, Flag, 0x40), Area::General);
        assert_eq!(area_of(Hgss, Flag, 0x320), Area::HiddenItems);
        assert_eq!(area_of(Hgss, Flag, 0x406), Area::HiddenItems);
        assert_eq!(area_of(Hgss, Flag, 0x407), Area::General);
        // Diamond and Pearl are taken as Platinum.
        assert_eq!(area_of(Dp, Flag, 0x2DA), Area::HiddenItems);
        assert_eq!(ranges(Dp), ranges(Pt));
        assert_eq!(Area::HiddenItems.word(), "Hidden items");
    }

    #[test]
    fn in_save_matches_the_block_sizes() {
        assert!(!in_save(Pt, Flag, 0));
        assert!(in_save(Pt, Flag, 1));
        assert!(in_save(Pt, Flag, 0xB5F));
        assert!(!in_save(Pt, Flag, 0xB60));
        assert_eq!(ranges(Pt).flags_end as u32 - 1, 2911);
        // 2 912 flags in 364 bytes.
        assert_eq!(0xB60 / 8, 364);
        assert!(!in_save(Pt, Var, 0x3FFF));
        assert!(in_save(Pt, Var, 0x4000));
        assert!(in_save(Pt, Var, 0x411F));
        assert!(!in_save(Pt, Var, 0x4120));
        assert_eq!(ranges(Pt).vars_end - 0x4000, 288);
        assert!(in_save(Hgss, Var, 0x416F));
        assert!(!in_save(Hgss, Var, 0x4170));
        assert_eq!(ranges(Hgss).vars_end - 0x4000, 368);
        assert!(!in_save(Hgss, Flag, 0x4000));
        assert!(!in_save(Pt, Var, 0x8000));
    }

    #[test]
    fn trainer_flags_match_the_arm9_tables() {
        assert_eq!(trainer_flag(Pt, 0), Some(0x550));
        assert_eq!(trainer_flag(Pt, 1039), Some(0x95F));
        assert_eq!(trainer_flag(Pt, 1040), None);
        assert_eq!(trainer_flag(Hgss, 5), Some(0x555));
        for code in ["CPUD", "IPKD", "IPKE"] {
            let layout = arm9_layout(code).unwrap();
            let family = if code == "CPUD" { Pt } else { Hgss };
            let start = layout.trainer_flag_start.unwrap();
            let capacity = layout.trainer_flag_capacity.unwrap();
            assert_eq!(trainer_flag(family, 0), Some(start), "{code}");
            assert_eq!(
                trainer_flag(family, capacity - 1),
                Some(start + capacity - 1)
            );
            assert_eq!(trainer_flag(family, capacity), None, "{code}");
        }
    }

    #[test]
    fn pool_range_is_general() {
        let pt = pool_range(Pt, Flag).unwrap();
        assert_eq!((pt.area, pt.start, pt.end), (Area::General, 0x41, 0x54F));
        let hgss = pool_range(Hgss, Var).unwrap();
        assert_eq!((hgss.start, hgss.end), (0x4030, 0x416F));
        assert_eq!(hidden_items_base(Pt), Some(0x2DA));
        assert_eq!(hidden_items_base(Hgss), Some(0x320));
        assert_eq!(hidden_item_flag(Pt, 3), Some(0x2DD));
        assert_eq!(hidden_item_flag(Pt, 283), Some(0x3F5));
        assert_eq!(hidden_item_flag(Pt, 284), None);
        assert_eq!(hidden_item_flag(Hgss, 230), Some(0x406));
        assert_eq!(hidden_item_flag(Hgss, 231), None);
    }

    #[test]
    fn the_file_parses_and_every_range_is_sound() {
        for family in [Pt, Hgss] {
            let r = ranges(family);
            assert!(!r.flags.is_empty() && !r.vars.is_empty(), "{family:?}");
            for range in r.flags.iter().chain(&r.vars) {
                assert!(range.start <= range.end, "{range:?}");
            }
            assert_eq!(
                r.flags.iter().filter(|x| x.area == Area::General).count(),
                1
            );
            assert_eq!(r.vars.iter().filter(|x| x.area == Area::General).count(), 1);
        }
    }
}
