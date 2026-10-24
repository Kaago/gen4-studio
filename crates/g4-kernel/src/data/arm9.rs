//! The read-only tables in `arm9/arm9.bin` (item table, TM/HM list, menu icon palettes),
//! located by `metadata/arm9_tables.yaml`. A project reads them once, on first use. A table
//! is "not available" (`None`) when the game code has no offset or only an unverified one,
//! when `arm9.bin` is too short, or when the table fails its plausibility check (a patched
//! arm9 can move tables, so a wrong offset must not produce wrong names). The arm9 is never
//! written.

use std::collections::HashMap;
use std::sync::OnceLock;

use g4_formats::data::arm9_tables::{self, ItemTable, MachineTable, MonIconPalettes};
use serde::Deserialize;

use crate::project::{AssetKind, Project};

const ARM9_TABLES_YAML: &str = include_str!("../../metadata/arm9_tables.yaml");

/// Where the arm9 tables of one game lie, and the trainer limits of the game (one entry of
/// `arm9_tables.yaml`). Offsets are file offsets in `arm9/arm9.bin`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arm9Layout {
    /// Taken from DSPRE without a ROM to check against: the kernel reads no table.
    #[serde(default)]
    pub unverified: bool,
    pub item_table: Option<u64>,
    /// Number of item ids in the item table.
    pub item_count: Option<usize>,
    pub machine_table: Option<u64>,
    pub mon_icon_palettes: Option<u64>,
    /// Number of menu icons in the palette table.
    pub mon_icon_palettes_len: Option<usize>,
    pub trainer_name_max: Option<u8>,
    /// Event flag of trainer 0 ("defeated").
    pub trainer_flag_start: Option<u16>,
    /// Number of trainers that have a flag.
    pub trainer_flag_capacity: Option<u16>,
}

/// Every entry of `arm9_tables.yaml`, keyed by game code.
/// The parsed `arm9_tables.yaml`, read once per process.
fn layouts() -> &'static HashMap<String, Arm9Layout> {
    static LAYOUTS: OnceLock<HashMap<String, Arm9Layout>> = OnceLock::new();
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    LAYOUTS.get_or_init(|| serde_yaml::from_str(ARM9_TABLES_YAML).unwrap_or_default())
}

/// The `arm9_tables.yaml` entry of `game_code`, if there is one.
pub fn arm9_layout(game_code: &str) -> Option<Arm9Layout> {
    layouts().get(game_code).cloned()
}

/// The tables of one project, read on first use. By design a table that is not available
/// then (no offset, short arm9, implausible) stays `None` for the life of the `Project`: the
/// studio never edits these tables (only the dynamic headers patch writes the arm9, elsewhere),
/// and a session sees one consistent answer.
#[derive(Debug, Default)]
pub(crate) struct Arm9Cache(OnceLock<Arm9Tables>);

#[derive(Debug, Default)]
struct Arm9Tables {
    /// Item and machine table, available together: their plausibility is checked together.
    items: Option<(ItemTable, MachineTable)>,
    palettes: Option<MonIconPalettes>,
}

impl Arm9Tables {
    fn read(project: &Project) -> Arm9Tables {
        let Some(layout) = arm9_layout(&project.game_code).filter(|l| !l.unverified) else {
            return Arm9Tables::default();
        };
        let Ok(arm9) = std::fs::read(project.arm9_path()) else {
            return Arm9Tables::default();
        };
        let offset = |o: Option<u64>| o.and_then(|o| usize::try_from(o).ok());
        let items = (|| {
            let item = ItemTable::read(&arm9, offset(layout.item_table)?, layout.item_count?)?;
            let machine = MachineTable::read(&arm9, offset(layout.machine_table)?)?;
            let members = project.member_count(AssetKind::Item).ok()?;
            arm9_tables::plausible(&item, &machine, members).ok()?;
            Some((item, machine))
        })();
        let palettes = (|| {
            let palettes = MonIconPalettes::read(
                &arm9,
                offset(layout.mon_icon_palettes)?,
                layout.mon_icon_palettes_len?,
            )?;
            palettes.plausible().ok()?;
            Some(palettes)
        })();
        Arm9Tables { items, palettes }
    }
}

impl Project {
    fn arm9_tables(&self) -> &Arm9Tables {
        self.arm9_tables.0.get_or_init(|| Arm9Tables::read(self))
    }

    /// The item table (item id → data member, icon, palette, GBA item), or `None` when it is
    /// not available.
    pub fn item_table(&self) -> Option<&ItemTable> {
        self.arm9_tables().items.as_ref().map(|(item, _)| item)
    }

    /// The moves of TM01..TM92 and HM01..HM08, or `None` when the table is not available.
    pub fn machine_table(&self) -> Option<&MachineTable> {
        self.arm9_tables()
            .items
            .as_ref()
            .map(|(_, machine)| machine)
    }

    /// The palette of every menu icon, or `None` when the table is not available.
    pub fn mon_icon_palettes(&self) -> Option<&MonIconPalettes> {
        self.arm9_tables().palettes.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const HEADER_TABLES_YAML: &str = include_str!("../../metadata/header_tables.yaml");

    #[test]
    fn arm9_tables_yaml_has_every_checked_code() {
        let all = layouts();
        assert_eq!(
            all["CPUD"],
            Arm9Layout {
                unverified: false,
                item_table: Some(0xF0D1C),
                item_count: Some(468),
                machine_table: Some(0xF0C54),
                mon_icon_palettes: Some(0xF07D8),
                mon_icon_palettes_len: Some(540),
                trainer_name_max: Some(7),
                trainer_flag_start: Some(0x550),
                trainer_flag_capacity: Some(1040),
            }
        );
        assert_eq!(
            all["IPKD"],
            Arm9Layout {
                unverified: false,
                item_table: Some(0x100148),
                item_count: Some(537),
                machine_table: Some(0x100080),
                mon_icon_palettes: Some(0xFFBC4),
                mon_icon_palettes_len: Some(544),
                trainer_name_max: Some(7),
                trainer_flag_start: Some(0x550),
                trainer_flag_capacity: Some(1040),
            }
        );
        assert_eq!(
            all["IPKE"],
            Arm9Layout {
                item_table: Some(0x100194),
                machine_table: Some(0x1000CC),
                mon_icon_palettes: Some(0xFFC10),
                ..all["IPKD"].clone()
            }
        );
        // Every game the kernel knows has an entry; only the three checked ones are verified.
        let codes: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML).unwrap();
        assert_eq!(all.len(), codes.len());
        for code in codes.keys() {
            let layout = &all[code];
            let checked = ["CPUD", "IPKD", "IPKE"].contains(&code.as_str());
            assert_eq!(layout.unverified, !checked, "{code}");
            assert!(
                layout.item_table.is_some() && layout.machine_table.is_some(),
                "{code}"
            );
        }
        assert_eq!(
            arm9_layout("CPUE").map(|l| l.item_table),
            Some(Some(0xF0CC4))
        );
        assert_eq!(arm9_layout("XXXX"), None);
    }

    /// A project folder with `header.yaml`, `item_members` itemData members and an
    /// `arm9.bin` holding vanilla-shaped CPUD tables (item `i` uses data member `i % 10`).
    fn project(dir: &Path, code: &str, item_members: usize) {
        std::fs::write(dir.join("header.yaml"), format!("gamecode: {code}\n")).unwrap();
        let items = dir.join("unpacked/itemData");
        std::fs::create_dir_all(&items).unwrap();
        for i in 0..item_members {
            std::fs::write(items.join(format!("{i:04}")), [0u8; 34]).unwrap();
        }
        let mut arm9 = vec![0u8; 0xF0D1C + 468 * 8];
        for i in 0..468 {
            let at = 0xF0D1C + 8 * i;
            arm9[at..at + 2].copy_from_slice(&((i % 10) as u16).to_le_bytes());
        }
        arm9[0xF0C54..0xF0C56].copy_from_slice(&264u16.to_le_bytes());
        arm9[0xF0C54 + 192..0xF0C56 + 192].copy_from_slice(&432u16.to_le_bytes());
        for i in 0..540 {
            arm9[0xF07D8 + i] = (i % 3) as u8;
        }
        std::fs::create_dir_all(dir.join("arm9")).unwrap();
        std::fs::write(dir.join("arm9/arm9.bin"), arm9).unwrap();
    }

    #[test]
    fn tables_of_a_checked_game_are_read_once() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path(), "CPUD", 10);
        let p = Project::open(dir.path()).unwrap();
        assert_eq!(p.item_table().unwrap().entries.len(), 468);
        assert_eq!(p.item_table().unwrap().entries[17].data, 7);
        assert_eq!(p.machine_table().unwrap().moves[0], 264);
        assert_eq!(p.mon_icon_palettes().unwrap().values.len(), 540);
        // Cached: a later change on disk does not show.
        std::fs::remove_file(dir.path().join("arm9/arm9.bin")).unwrap();
        assert!(p.item_table().is_some() && p.mon_icon_palettes().is_some());
        // A fresh project without the arm9 has none.
        let p = Project::open(dir.path()).unwrap();
        assert!(p.item_table().is_none() && p.machine_table().is_none());
        assert!(p.mon_icon_palettes().is_none());
    }

    #[test]
    fn implausible_tables_are_not_available() {
        let dir = tempfile::tempdir().unwrap();
        // Item ids point at data member 9, but the item data has nine members.
        project(dir.path(), "CPUD", 9);
        let p = Project::open(dir.path()).unwrap();
        assert!(p.item_table().is_none() && p.machine_table().is_none());
        assert!(p.mon_icon_palettes().is_some());
        // A palette value of 3 makes the palette table implausible.
        let mut arm9 = std::fs::read(dir.path().join("arm9/arm9.bin")).unwrap();
        arm9[0xF07D8 + 100] = 3;
        std::fs::write(dir.path().join("arm9/arm9.bin"), &arm9).unwrap();
        assert!(
            Project::open(dir.path())
                .unwrap()
                .mon_icon_palettes()
                .is_none()
        );
    }

    #[test]
    fn unverified_or_unlisted_codes_and_short_arm9_have_no_tables() {
        let dir = tempfile::tempdir().unwrap();
        // The same bytes as CPUD, but CPUE's offsets are unverified.
        project(dir.path(), "CPUE", 10);
        let p = Project::open(dir.path()).unwrap();
        assert!(p.item_table().is_none() && p.mon_icon_palettes().is_none());
        project(dir.path(), "CPUD", 10);
        let arm9 = std::fs::read(dir.path().join("arm9/arm9.bin")).unwrap();
        std::fs::write(dir.path().join("arm9/arm9.bin"), &arm9[..arm9.len() - 1]).unwrap();
        let p = Project::open(dir.path()).unwrap();
        assert!(p.item_table().is_none() && p.machine_table().is_none());
        assert!(p.mon_icon_palettes().is_some());
    }
}
