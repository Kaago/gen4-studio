//! Read-only tables in `arm9/arm9.bin`: the item table (item id → data
//! member, icon, palette, GBA item), the TM/HM move list and the palette index
//! of every Pokémon menu icon. Offsets come from the kernel metadata
//! (`arm9_tables.yaml`); nothing here ever writes the arm9.

/// One entry of the item table: what item id `i` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEntry {
    /// `itemData` member with the item's data (several ids share members).
    pub data: u16,
    /// `itemIcons` member with the icon graphic (NCGR).
    pub icon: u16,
    /// `itemIcons` member with the icon palette (NCLR).
    pub palette: u16,
    /// Item id in the GBA games (Pal Park and migration).
    pub gba: u16,
}

/// The item table, indexed by item id (8 bytes per id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTable {
    pub entries: Vec<ItemEntry>,
}

/// Length of one item table entry.
const ITEM_ENTRY_LEN: usize = 8;

/// `len` bytes of `arm9` starting at `offset`, or `None` when the arm9 is too short.
fn slice(arm9: &[u8], offset: usize, len: usize) -> Option<&[u8]> {
    arm9.get(offset..offset.checked_add(len)?)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

impl ItemTable {
    /// Reads `count` entries at `offset`; `None` when the arm9 is too short.
    pub fn read(arm9: &[u8], offset: usize, count: usize) -> Option<ItemTable> {
        let bytes = slice(arm9, offset, count.checked_mul(ITEM_ENTRY_LEN)?)?;
        let entries = bytes
            .as_chunks::<ITEM_ENTRY_LEN>()
            .0
            .iter()
            .map(|e| ItemEntry {
                data: u16_at(e, 0),
                icon: u16_at(e, 2),
                palette: u16_at(e, 4),
                gba: u16_at(e, 6),
            })
            .collect();
        Some(ItemTable { entries })
    }
}

/// The moves taught by TM01..TM92 (indices 0..92) and HM01..HM08 (92..100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineTable {
    pub moves: Vec<u16>,
}

impl MachineTable {
    /// Number of machines: 92 TMs and 8 HMs.
    pub const LEN: usize = 100;

    /// Reads the 100 move ids at `offset`; `None` when the arm9 is too short.
    pub fn read(arm9: &[u8], offset: usize) -> Option<MachineTable> {
        let bytes = slice(arm9, offset, 2 * Self::LEN)?;
        let moves = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|w| u16::from_le_bytes(*w))
            .collect();
        Some(MachineTable { moves })
    }
}

/// Palette number (0, 1 or 2 of the icon NCLR) of every Pokémon menu icon,
/// indexed by icon index (`monIcons` NCGR member − 7), not by personal member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonIconPalettes {
    pub values: Vec<u8>,
}

impl MonIconPalettes {
    /// Number of palettes the icons use.
    pub const PALETTES: u8 = 3;

    /// Reads `len` bytes at `offset`; `None` when the arm9 is too short.
    pub fn read(arm9: &[u8], offset: usize, len: usize) -> Option<MonIconPalettes> {
        slice(arm9, offset, len).map(|values| MonIconPalettes {
            values: values.to_vec(),
        })
    }

    /// Every value names one of the three icon palettes; anything else means
    /// the offset does not point at the table (a patched or foreign arm9).
    pub fn plausible(&self) -> Result<(), String> {
        match self.values.iter().position(|&v| v >= Self::PALETTES) {
            Some(i) => Err(format!(
                "icon {i} uses palette {}, only 0 to {} exist",
                self.values[i],
                Self::PALETTES - 1
            )),
            None => Ok(()),
        }
    }
}

/// TM01 teaches Focus Punch in every game.
const TM01_MOVE: u16 = 264;
/// HM05 teaches Defog (Diamond, Pearl, Platinum) or Whirlpool (HeartGold, SoulSilver).
const HM05_MOVES: [u16; 2] = [432, 250];
/// Index of HM05 in the machine table.
const HM05: usize = 96;

/// Whether the two tables look like the vanilla ones: TM01 is Focus Punch,
/// HM05 is Defog or Whirlpool, and every item id points at an existing
/// `itemData` member. The error names the first check that failed.
pub fn plausible(
    item: &ItemTable,
    machine: &MachineTable,
    item_members: usize,
) -> Result<(), String> {
    match machine.moves.first() {
        Some(&TM01_MOVE) => {}
        other => return Err(format!("TM01 teaches {other:?}, expected {TM01_MOVE}")),
    }
    match machine.moves.get(HM05) {
        Some(m) if HM05_MOVES.contains(m) => {}
        other => {
            return Err(format!(
                "HM05 teaches {other:?}, expected one of {HM05_MOVES:?}"
            ));
        }
    }
    if let Some((id, e)) = item
        .entries
        .iter()
        .enumerate()
        .find(|(_, e)| usize::from(e.data) >= item_members)
    {
        return Err(format!(
            "item {id} uses data member {}, the item data has {item_members}",
            e.data
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An arm9 image with a two-entry item table at 0x10, a machine table at
    /// 0x20 and three palette bytes at 0x200.
    fn arm9() -> Vec<u8> {
        let mut a = vec![0u8; 0x204];
        // Item 0: data 0, icon 1, palette 2, GBA 0; item 1: data 17, icon 24, palette 25, GBA 13.
        for (i, w) in [0u16, 1, 2, 0, 17, 24, 25, 13].iter().enumerate() {
            a[0x10 + 2 * i..0x12 + 2 * i].copy_from_slice(&w.to_le_bytes());
        }
        let mut machines = [0u16; 100];
        machines[0] = 264;
        machines[96] = 432;
        for (i, m) in machines.iter().enumerate() {
            a[0x20 + 2 * i..0x22 + 2 * i].copy_from_slice(&m.to_le_bytes());
        }
        a[0x200..0x203].copy_from_slice(&[1, 2, 0]);
        a
    }

    #[test]
    fn tables_read_little_endian_entries() {
        let a = arm9();
        let items = ItemTable::read(&a, 0x10, 2).unwrap();
        assert_eq!(
            items.entries[1],
            ItemEntry {
                data: 17,
                icon: 24,
                palette: 25,
                gba: 13
            }
        );
        let machines = MachineTable::read(&a, 0x20).unwrap();
        assert_eq!(machines.moves.len(), MachineTable::LEN);
        assert_eq!((machines.moves[0], machines.moves[96]), (264, 432));
        let palettes = MonIconPalettes::read(&a, 0x200, 3).unwrap();
        assert_eq!(palettes.values, [1, 2, 0]);
        assert_eq!(palettes.plausible(), Ok(()));
        assert_eq!(plausible(&items, &machines, 18), Ok(()));
    }

    #[test]
    fn item_table_is_none_when_arm9_is_short() {
        let a = arm9();
        // Exactly fitting is fine, one entry more is not.
        assert!(ItemTable::read(&a, a.len() - 16, 2).is_some());
        assert_eq!(ItemTable::read(&a, a.len() - 16, 3), None);
        assert_eq!(ItemTable::read(&a, usize::MAX, 1), None);
        assert_eq!(ItemTable::read(&a, 0, usize::MAX), None);
        assert_eq!(MachineTable::read(&a, a.len() - 199), None);
        assert_eq!(MonIconPalettes::read(&a, a.len() - 2, 3), None);
    }

    #[test]
    fn plausible_rejects_a_data_index_beyond_the_members() {
        let a = arm9();
        let items = ItemTable::read(&a, 0x10, 2).unwrap();
        let machines = MachineTable::read(&a, 0x20).unwrap();
        // Item 1 uses data member 17: 17 members are too few.
        let err = plausible(&items, &machines, 17).unwrap_err();
        assert!(err.contains("item 1") && err.contains("17"), "{err}");
    }

    #[test]
    fn plausible_checks_tm01_and_hm05() {
        let a = arm9();
        let items = ItemTable::read(&a, 0x10, 2).unwrap();
        let mut machines = MachineTable::read(&a, 0x20).unwrap();
        machines.moves[96] = 250;
        assert_eq!(plausible(&items, &machines, 18), Ok(()));
        machines.moves[96] = 57;
        assert!(
            plausible(&items, &machines, 18)
                .unwrap_err()
                .contains("HM05")
        );
        machines.moves[96] = 432;
        machines.moves[0] = 5;
        assert!(
            plausible(&items, &machines, 18)
                .unwrap_err()
                .contains("TM01")
        );
        let empty = MachineTable { moves: vec![] };
        assert!(plausible(&items, &empty, 18).is_err());
    }

    #[test]
    fn palettes_outside_the_three_are_implausible() {
        let p = MonIconPalettes {
            values: vec![0, 1, 2, 3],
        };
        assert!(p.plausible().unwrap_err().contains("icon 3"));
    }
}
