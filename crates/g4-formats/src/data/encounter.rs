//! Wild encounters (`encounters/NNNN`): Platinum (424 bytes, also used for
//! Diamond and Pearl) and HeartGold/SoulSilver (196 bytes). Layouts:
//! `docs/formats/encounter.md`.
//!
//! Both records keep their original bytes, so unnamed bytes (slot padding, the
//! unused block at 0xF8, species high words) survive every setter. A member of
//! any other length is rejected by `read`, never truncated.
//!
//! The byte order of a water slot differs between the families (Platinum
//! stores max before min, HeartGold min before max); [`WaterSlot`] hides that.

use super::record::{FieldKind::*, RawRecord};
use super::span;
use crate::error::FormatError;
use crate::game::Family;

crate::fixed_record!(
    /// One Platinum (and Diamond/Pearl) encounter file, kept as its original 424 bytes.
    pub PtEncounters,
    424,
    [
        span("grass_rate", 0x000, 4, U32),
        span("grass_00_level", 0x004, 4, U32),
        span("grass_00_species", 0x008, 4, U32),
        span("grass_01_level", 0x00C, 4, U32),
        span("grass_01_species", 0x010, 4, U32),
        span("grass_02_level", 0x014, 4, U32),
        span("grass_02_species", 0x018, 4, U32),
        span("grass_03_level", 0x01C, 4, U32),
        span("grass_03_species", 0x020, 4, U32),
        span("grass_04_level", 0x024, 4, U32),
        span("grass_04_species", 0x028, 4, U32),
        span("grass_05_level", 0x02C, 4, U32),
        span("grass_05_species", 0x030, 4, U32),
        span("grass_06_level", 0x034, 4, U32),
        span("grass_06_species", 0x038, 4, U32),
        span("grass_07_level", 0x03C, 4, U32),
        span("grass_07_species", 0x040, 4, U32),
        span("grass_08_level", 0x044, 4, U32),
        span("grass_08_species", 0x048, 4, U32),
        span("grass_09_level", 0x04C, 4, U32),
        span("grass_09_species", 0x050, 4, U32),
        span("grass_10_level", 0x054, 4, U32),
        span("grass_10_species", 0x058, 4, U32),
        span("grass_11_level", 0x05C, 4, U32),
        span("grass_11_species", 0x060, 4, U32),
        span("swarm_0", 0x064, 4, U32),
        span("swarm_1", 0x068, 4, U32),
        span("day_0", 0x06C, 4, U32),
        span("day_1", 0x070, 4, U32),
        span("night_0", 0x074, 4, U32),
        span("night_1", 0x078, 4, U32),
        span("radar_0", 0x07C, 4, U32),
        span("radar_1", 0x080, 4, U32),
        span("radar_2", 0x084, 4, U32),
        span("radar_3", 0x088, 4, U32),
        span("form_rate_0", 0x08C, 4, U32),
        span("form_rate_1", 0x090, 4, U32),
        span("form_rate_2", 0x094, 4, U32),
        span("form_rate_3", 0x098, 4, U32),
        span("form_rate_4", 0x09C, 4, U32),
        span("unown_table", 0x0A0, 4, U32),
        span("dual_ruby_0", 0x0A4, 4, U32),
        span("dual_ruby_1", 0x0A8, 4, U32),
        span("dual_sapphire_0", 0x0AC, 4, U32),
        span("dual_sapphire_1", 0x0B0, 4, U32),
        span("dual_emerald_0", 0x0B4, 4, U32),
        span("dual_emerald_1", 0x0B8, 4, U32),
        span("dual_firered_0", 0x0BC, 4, U32),
        span("dual_firered_1", 0x0C0, 4, U32),
        span("dual_leafgreen_0", 0x0C4, 4, U32),
        span("dual_leafgreen_1", 0x0C8, 4, U32),
        span("surf_rate", 0x0CC, 4, U32),
        span("surf_0_max", 0x0D0, 1, U8),
        span("surf_0_min", 0x0D1, 1, U8),
        span("surf_0_pad", 0x0D2, 2, Bytes),
        span("surf_0_species", 0x0D4, 4, U32),
        span("surf_1_max", 0x0D8, 1, U8),
        span("surf_1_min", 0x0D9, 1, U8),
        span("surf_1_pad", 0x0DA, 2, Bytes),
        span("surf_1_species", 0x0DC, 4, U32),
        span("surf_2_max", 0x0E0, 1, U8),
        span("surf_2_min", 0x0E1, 1, U8),
        span("surf_2_pad", 0x0E2, 2, Bytes),
        span("surf_2_species", 0x0E4, 4, U32),
        span("surf_3_max", 0x0E8, 1, U8),
        span("surf_3_min", 0x0E9, 1, U8),
        span("surf_3_pad", 0x0EA, 2, Bytes),
        span("surf_3_species", 0x0EC, 4, U32),
        span("surf_4_max", 0x0F0, 1, U8),
        span("surf_4_min", 0x0F1, 1, U8),
        span("surf_4_pad", 0x0F2, 2, Bytes),
        span("surf_4_species", 0x0F4, 4, U32),
        span("pad_f8", 0x0F8, 44, Bytes),
        span("old_rod_rate", 0x124, 4, U32),
        span("old_rod_0_max", 0x128, 1, U8),
        span("old_rod_0_min", 0x129, 1, U8),
        span("old_rod_0_pad", 0x12A, 2, Bytes),
        span("old_rod_0_species", 0x12C, 4, U32),
        span("old_rod_1_max", 0x130, 1, U8),
        span("old_rod_1_min", 0x131, 1, U8),
        span("old_rod_1_pad", 0x132, 2, Bytes),
        span("old_rod_1_species", 0x134, 4, U32),
        span("old_rod_2_max", 0x138, 1, U8),
        span("old_rod_2_min", 0x139, 1, U8),
        span("old_rod_2_pad", 0x13A, 2, Bytes),
        span("old_rod_2_species", 0x13C, 4, U32),
        span("old_rod_3_max", 0x140, 1, U8),
        span("old_rod_3_min", 0x141, 1, U8),
        span("old_rod_3_pad", 0x142, 2, Bytes),
        span("old_rod_3_species", 0x144, 4, U32),
        span("old_rod_4_max", 0x148, 1, U8),
        span("old_rod_4_min", 0x149, 1, U8),
        span("old_rod_4_pad", 0x14A, 2, Bytes),
        span("old_rod_4_species", 0x14C, 4, U32),
        span("good_rod_rate", 0x150, 4, U32),
        span("good_rod_0_max", 0x154, 1, U8),
        span("good_rod_0_min", 0x155, 1, U8),
        span("good_rod_0_pad", 0x156, 2, Bytes),
        span("good_rod_0_species", 0x158, 4, U32),
        span("good_rod_1_max", 0x15C, 1, U8),
        span("good_rod_1_min", 0x15D, 1, U8),
        span("good_rod_1_pad", 0x15E, 2, Bytes),
        span("good_rod_1_species", 0x160, 4, U32),
        span("good_rod_2_max", 0x164, 1, U8),
        span("good_rod_2_min", 0x165, 1, U8),
        span("good_rod_2_pad", 0x166, 2, Bytes),
        span("good_rod_2_species", 0x168, 4, U32),
        span("good_rod_3_max", 0x16C, 1, U8),
        span("good_rod_3_min", 0x16D, 1, U8),
        span("good_rod_3_pad", 0x16E, 2, Bytes),
        span("good_rod_3_species", 0x170, 4, U32),
        span("good_rod_4_max", 0x174, 1, U8),
        span("good_rod_4_min", 0x175, 1, U8),
        span("good_rod_4_pad", 0x176, 2, Bytes),
        span("good_rod_4_species", 0x178, 4, U32),
        span("super_rod_rate", 0x17C, 4, U32),
        span("super_rod_0_max", 0x180, 1, U8),
        span("super_rod_0_min", 0x181, 1, U8),
        span("super_rod_0_pad", 0x182, 2, Bytes),
        span("super_rod_0_species", 0x184, 4, U32),
        span("super_rod_1_max", 0x188, 1, U8),
        span("super_rod_1_min", 0x189, 1, U8),
        span("super_rod_1_pad", 0x18A, 2, Bytes),
        span("super_rod_1_species", 0x18C, 4, U32),
        span("super_rod_2_max", 0x190, 1, U8),
        span("super_rod_2_min", 0x191, 1, U8),
        span("super_rod_2_pad", 0x192, 2, Bytes),
        span("super_rod_2_species", 0x194, 4, U32),
        span("super_rod_3_max", 0x198, 1, U8),
        span("super_rod_3_min", 0x199, 1, U8),
        span("super_rod_3_pad", 0x19A, 2, Bytes),
        span("super_rod_3_species", 0x19C, 4, U32),
        span("super_rod_4_max", 0x1A0, 1, U8),
        span("super_rod_4_min", 0x1A1, 1, U8),
        span("super_rod_4_pad", 0x1A2, 2, Bytes),
        span("super_rod_4_species", 0x1A4, 4, U32),
    ]
);

crate::fixed_record!(
    /// One HeartGold/SoulSilver encounter file, kept as its original 196 bytes.
    pub HgssEncounters,
    196,
    [
        span("grass_rate", 0x000, 1, U8),
        span("surf_rate", 0x001, 1, U8),
        span("rock_rate", 0x002, 1, U8),
        span("old_rod_rate", 0x003, 1, U8),
        span("good_rod_rate", 0x004, 1, U8),
        span("super_rod_rate", 0x005, 1, U8),
        span("pad_06", 0x006, 2, Bytes),
        span("grass_level_00", 0x008, 1, U8),
        span("grass_level_01", 0x009, 1, U8),
        span("grass_level_02", 0x00A, 1, U8),
        span("grass_level_03", 0x00B, 1, U8),
        span("grass_level_04", 0x00C, 1, U8),
        span("grass_level_05", 0x00D, 1, U8),
        span("grass_level_06", 0x00E, 1, U8),
        span("grass_level_07", 0x00F, 1, U8),
        span("grass_level_08", 0x010, 1, U8),
        span("grass_level_09", 0x011, 1, U8),
        span("grass_level_10", 0x012, 1, U8),
        span("grass_level_11", 0x013, 1, U8),
        span("grass_morning_00", 0x014, 2, U16),
        span("grass_morning_01", 0x016, 2, U16),
        span("grass_morning_02", 0x018, 2, U16),
        span("grass_morning_03", 0x01A, 2, U16),
        span("grass_morning_04", 0x01C, 2, U16),
        span("grass_morning_05", 0x01E, 2, U16),
        span("grass_morning_06", 0x020, 2, U16),
        span("grass_morning_07", 0x022, 2, U16),
        span("grass_morning_08", 0x024, 2, U16),
        span("grass_morning_09", 0x026, 2, U16),
        span("grass_morning_10", 0x028, 2, U16),
        span("grass_morning_11", 0x02A, 2, U16),
        span("grass_day_00", 0x02C, 2, U16),
        span("grass_day_01", 0x02E, 2, U16),
        span("grass_day_02", 0x030, 2, U16),
        span("grass_day_03", 0x032, 2, U16),
        span("grass_day_04", 0x034, 2, U16),
        span("grass_day_05", 0x036, 2, U16),
        span("grass_day_06", 0x038, 2, U16),
        span("grass_day_07", 0x03A, 2, U16),
        span("grass_day_08", 0x03C, 2, U16),
        span("grass_day_09", 0x03E, 2, U16),
        span("grass_day_10", 0x040, 2, U16),
        span("grass_day_11", 0x042, 2, U16),
        span("grass_night_00", 0x044, 2, U16),
        span("grass_night_01", 0x046, 2, U16),
        span("grass_night_02", 0x048, 2, U16),
        span("grass_night_03", 0x04A, 2, U16),
        span("grass_night_04", 0x04C, 2, U16),
        span("grass_night_05", 0x04E, 2, U16),
        span("grass_night_06", 0x050, 2, U16),
        span("grass_night_07", 0x052, 2, U16),
        span("grass_night_08", 0x054, 2, U16),
        span("grass_night_09", 0x056, 2, U16),
        span("grass_night_10", 0x058, 2, U16),
        span("grass_night_11", 0x05A, 2, U16),
        span("radio_0", 0x05C, 2, U16),
        span("radio_1", 0x05E, 2, U16),
        span("radio_2", 0x060, 2, U16),
        span("radio_3", 0x062, 2, U16),
        span("surf_0_min", 0x064, 1, U8),
        span("surf_0_max", 0x065, 1, U8),
        span("surf_0_species", 0x066, 2, U16),
        span("surf_1_min", 0x068, 1, U8),
        span("surf_1_max", 0x069, 1, U8),
        span("surf_1_species", 0x06A, 2, U16),
        span("surf_2_min", 0x06C, 1, U8),
        span("surf_2_max", 0x06D, 1, U8),
        span("surf_2_species", 0x06E, 2, U16),
        span("surf_3_min", 0x070, 1, U8),
        span("surf_3_max", 0x071, 1, U8),
        span("surf_3_species", 0x072, 2, U16),
        span("surf_4_min", 0x074, 1, U8),
        span("surf_4_max", 0x075, 1, U8),
        span("surf_4_species", 0x076, 2, U16),
        span("rock_0_min", 0x078, 1, U8),
        span("rock_0_max", 0x079, 1, U8),
        span("rock_0_species", 0x07A, 2, U16),
        span("rock_1_min", 0x07C, 1, U8),
        span("rock_1_max", 0x07D, 1, U8),
        span("rock_1_species", 0x07E, 2, U16),
        span("old_rod_0_min", 0x080, 1, U8),
        span("old_rod_0_max", 0x081, 1, U8),
        span("old_rod_0_species", 0x082, 2, U16),
        span("old_rod_1_min", 0x084, 1, U8),
        span("old_rod_1_max", 0x085, 1, U8),
        span("old_rod_1_species", 0x086, 2, U16),
        span("old_rod_2_min", 0x088, 1, U8),
        span("old_rod_2_max", 0x089, 1, U8),
        span("old_rod_2_species", 0x08A, 2, U16),
        span("old_rod_3_min", 0x08C, 1, U8),
        span("old_rod_3_max", 0x08D, 1, U8),
        span("old_rod_3_species", 0x08E, 2, U16),
        span("old_rod_4_min", 0x090, 1, U8),
        span("old_rod_4_max", 0x091, 1, U8),
        span("old_rod_4_species", 0x092, 2, U16),
        span("good_rod_0_min", 0x094, 1, U8),
        span("good_rod_0_max", 0x095, 1, U8),
        span("good_rod_0_species", 0x096, 2, U16),
        span("good_rod_1_min", 0x098, 1, U8),
        span("good_rod_1_max", 0x099, 1, U8),
        span("good_rod_1_species", 0x09A, 2, U16),
        span("good_rod_2_min", 0x09C, 1, U8),
        span("good_rod_2_max", 0x09D, 1, U8),
        span("good_rod_2_species", 0x09E, 2, U16),
        span("good_rod_3_min", 0x0A0, 1, U8),
        span("good_rod_3_max", 0x0A1, 1, U8),
        span("good_rod_3_species", 0x0A2, 2, U16),
        span("good_rod_4_min", 0x0A4, 1, U8),
        span("good_rod_4_max", 0x0A5, 1, U8),
        span("good_rod_4_species", 0x0A6, 2, U16),
        span("super_rod_0_min", 0x0A8, 1, U8),
        span("super_rod_0_max", 0x0A9, 1, U8),
        span("super_rod_0_species", 0x0AA, 2, U16),
        span("super_rod_1_min", 0x0AC, 1, U8),
        span("super_rod_1_max", 0x0AD, 1, U8),
        span("super_rod_1_species", 0x0AE, 2, U16),
        span("super_rod_2_min", 0x0B0, 1, U8),
        span("super_rod_2_max", 0x0B1, 1, U8),
        span("super_rod_2_species", 0x0B2, 2, U16),
        span("super_rod_3_min", 0x0B4, 1, U8),
        span("super_rod_3_max", 0x0B5, 1, U8),
        span("super_rod_3_species", 0x0B6, 2, U16),
        span("super_rod_4_min", 0x0B8, 1, U8),
        span("super_rod_4_max", 0x0B9, 1, U8),
        span("super_rod_4_species", 0x0BA, 2, U16),
        span("land_swarm", 0x0BC, 2, U16),
        span("surf_swarm", 0x0BE, 2, U16),
        span("night_fish", 0x0C0, 2, U16),
        span("fish_swarm", 0x0C2, 2, U16),
    ]
);

/// Checks a slot index against the number of slots of its list; a wrong index
/// would silently hit a neighbouring list. Offsets come from the layout, the
/// index from callers, so this is an assertion like the record accessors.
fn slot_index(i: usize, count: usize) -> usize {
    assert!(i < count, "slot index {i} out of range 0..{count}");
    i
}

/// `u32` getter/setter pairs on a [`RawRecord`] implementor.
macro_rules! u32_fields {
    ($($get:ident / $set:ident @ $off:expr;)*) => {
        $(
            pub fn $get(&self) -> u32 {
                self.get_u32($off)
            }
            pub fn $set(&mut self, v: u32) {
                self.set_u32($off, v);
            }
        )*
    };
}

/// Lists of species words (`u32`, species in the low 16 bits) of a Platinum
/// record: `get(i)` / `set(i, v)`; `set` leaves the upper 16 bits alone.
macro_rules! pt_species_lists {
    ($($get:ident / $set:ident @ $base:expr, $count:expr;)*) => {
        $(
            pub fn $get(&self, i: usize) -> u16 {
                self.get_u16($base + 4 * slot_index(i, $count))
            }
            pub fn $set(&mut self, i: usize, species: u16) {
                self.set_u16($base + 4 * slot_index(i, $count), species);
            }
        )*
    };
}

/// A water method of a Platinum record: rate word at `$base`, five slots after it.
macro_rules! pt_water {
    ($($rate:ident / $set_rate:ident, $get:ident / $set:ident @ $base:expr;)*) => {
        $(
            pub fn $rate(&self) -> u32 {
                self.get_u32($base)
            }
            pub fn $set_rate(&mut self, v: u32) {
                self.set_u32($base, v);
            }
            pub fn $get(&self, i: usize) -> WaterSlot {
                self.water($base + 4, i)
            }
            pub fn $set(&mut self, i: usize, slot: WaterSlot) {
                self.set_water($base + 4, i, slot);
            }
        )*
    };
}

/// A list of water slots of a HeartGold record (the rate is a plain `u8`).
macro_rules! hgss_water {
    ($($get:ident / $set:ident @ $base:expr, $count:expr;)*) => {
        $(
            pub fn $get(&self, i: usize) -> WaterSlot {
                self.water($base, i, $count)
            }
            pub fn $set(&mut self, i: usize, slot: WaterSlot) {
                self.set_water($base, i, $count, slot);
            }
        )*
    };
}

/// One Platinum grass slot: the level word (only its low byte is the level)
/// and the species word (the game reads its low 16 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub level: u8,
    pub species: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaterLayout {
    /// `{ u8 max, u8 min, u16 pad, u32 species }`, 8 bytes.
    Pt,
    /// `{ u8 min, u8 max, u16 species }`, 4 bytes.
    Hgss,
}

/// One water slot (Surf, Rock Smash, rods) with min and max level and the
/// species. It remembers its family's byte order, which `min()` and `max()`
/// hide: Platinum stores max before min, HeartGold min before max. Bytes not
/// covered by the three values (Platinum padding, species high word) are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterSlot {
    layout: WaterLayout,
    raw: [u8; 8],
}

impl WaterSlot {
    /// A Platinum slot from its 8 bytes.
    pub fn from_pt_bytes(bytes: [u8; 8]) -> WaterSlot {
        WaterSlot {
            layout: WaterLayout::Pt,
            raw: bytes,
        }
    }

    /// A HeartGold slot from its 4 bytes.
    pub fn from_hgss_bytes(bytes: [u8; 4]) -> WaterSlot {
        let mut raw = [0; 8];
        raw[..4].copy_from_slice(&bytes);
        WaterSlot {
            layout: WaterLayout::Hgss,
            raw,
        }
    }

    fn byte_len(&self) -> usize {
        match self.layout {
            WaterLayout::Pt => 8,
            WaterLayout::Hgss => 4,
        }
    }

    fn min_at(&self) -> usize {
        match self.layout {
            WaterLayout::Pt => 1,
            WaterLayout::Hgss => 0,
        }
    }

    fn max_at(&self) -> usize {
        1 - self.min_at()
    }

    fn species_at(&self) -> usize {
        match self.layout {
            WaterLayout::Pt => 4,
            WaterLayout::Hgss => 2,
        }
    }

    pub fn min(&self) -> u8 {
        self.raw[self.min_at()]
    }

    pub fn max(&self) -> u8 {
        self.raw[self.max_at()]
    }

    /// The species the game uses (the low 16 bits of the species word).
    pub fn species(&self) -> u16 {
        let at = self.species_at();
        u16::from_le_bytes([self.raw[at], self.raw[at + 1]])
    }

    pub fn set_min(&mut self, v: u8) {
        self.raw[self.min_at()] = v;
    }

    pub fn set_max(&mut self, v: u8) {
        self.raw[self.max_at()] = v;
    }

    /// Sets the low 16 bits of the species word, the rest stays.
    pub fn set_species(&mut self, v: u16) {
        let at = self.species_at();
        self.raw[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
}

/// The five Game Boy Advance cartridges whose replacement lists ("dual slot")
/// follow the Poké Radar lists in a Platinum record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSlot {
    Ruby = 0,
    Sapphire = 1,
    Emerald = 2,
    FireRed = 3,
    LeafGreen = 4,
}

impl DualSlot {
    pub const ALL: [DualSlot; 5] = [
        DualSlot::Ruby,
        DualSlot::Sapphire,
        DualSlot::Emerald,
        DualSlot::FireRed,
        DualSlot::LeafGreen,
    ];
}

impl PtEncounters {
    const GRASS: usize = 0x04;
    const GRASS_SLOTS: usize = 12;
    const SWARM: usize = 0x64;
    const DAY: usize = 0x6C;
    const NIGHT: usize = 0x74;
    const RADAR: usize = 0x7C;
    const FORM_RATES: usize = 0x8C;
    const DUAL: usize = 0xA4;
    const UNUSED: usize = 0xF8;
    const UNUSED_LEN: usize = 44;

    u32_fields! {
        grass_rate / set_grass_rate @ 0x00;
        unown_table / set_unown_table @ 0xA0;
    }

    pt_species_lists! {
        swarm / set_swarm @ Self::SWARM, 2;
        day / set_day @ Self::DAY, 2;
        night / set_night @ Self::NIGHT, 2;
        radar / set_radar @ Self::RADAR, 4;
    }

    pt_water! {
        surf_rate / set_surf_rate, surf / set_surf @ 0xCC;
        old_rod_rate / set_old_rod_rate, old_rod / set_old_rod @ 0x124;
        good_rod_rate / set_good_rod_rate, good_rod / set_good_rod @ 0x150;
        super_rod_rate / set_super_rod_rate, super_rod / set_super_rod @ 0x17C;
    }

    /// Grass slot `i` (0..12).
    pub fn grass(&self, i: usize) -> Slot {
        let at = Self::GRASS + 8 * slot_index(i, Self::GRASS_SLOTS);
        Slot {
            level: self.get_u8(at),
            species: self.get_u16(at + 4),
        }
    }

    /// Writes the level byte and the low 16 bits of the species word; the
    /// other bytes of both words stay.
    pub fn set_grass(&mut self, i: usize, slot: Slot) {
        let at = Self::GRASS + 8 * slot_index(i, Self::GRASS_SLOTS);
        self.set_u8(at, slot.level);
        self.set_u16(at + 4, slot.species);
    }

    /// The five form-rate words: `[0]` Shellos, `[1]` Gastrodon (0 = West,
    /// anything else East), `[2..5]` unused by the game.
    pub fn form_rates(&self) -> [u32; 5] {
        std::array::from_fn(|i| self.get_u32(Self::FORM_RATES + 4 * i))
    }

    pub fn set_form_rates(&mut self, rates: [u32; 5]) {
        for (i, rate) in rates.into_iter().enumerate() {
            self.set_u32(Self::FORM_RATES + 4 * i, rate);
        }
    }

    /// Entry `i` (0..2) of the dual-slot list of `game`.
    pub fn dual(&self, game: DualSlot, i: usize) -> u16 {
        self.get_u16(Self::dual_at(game, i))
    }

    pub fn set_dual(&mut self, game: DualSlot, i: usize, species: u16) {
        self.set_u16(Self::dual_at(game, i), species);
    }

    fn dual_at(game: DualSlot, i: usize) -> usize {
        Self::DUAL + 8 * game as usize + 4 * slot_index(i, 2)
    }

    /// The 44 bytes at 0xF8 the game does not use (Rock Smash, commented out
    /// in the original code); zero in vanilla, kept as read.
    pub fn pad_f8(&self) -> &[u8; Self::UNUSED_LEN] {
        self.raw[Self::UNUSED..Self::UNUSED + Self::UNUSED_LEN]
            .try_into()
            .expect("44 bytes")
    }

    pub fn set_pad_f8(&mut self, bytes: &[u8; Self::UNUSED_LEN]) {
        self.raw[Self::UNUSED..Self::UNUSED + Self::UNUSED_LEN].copy_from_slice(bytes);
    }

    /// Slot `i` (0..5) of the list that starts at `first`.
    fn water(&self, first: usize, i: usize) -> WaterSlot {
        let at = first + 8 * slot_index(i, 5);
        WaterSlot::from_pt_bytes(self.raw[at..at + 8].try_into().expect("8 bytes"))
    }

    /// Writes min, max and the low 16 species bits; padding and the species
    /// high word stay, whichever family `slot` was read from.
    fn set_water(&mut self, first: usize, i: usize, slot: WaterSlot) {
        let at = first + 8 * slot_index(i, 5);
        let mut current = self.water(first, i);
        current.set_min(slot.min());
        current.set_max(slot.max());
        current.set_species(slot.species());
        self.raw[at..at + current.byte_len()].copy_from_slice(&current.raw);
    }
}

impl HgssEncounters {
    const LEVELS: usize = 0x08;
    const MORNING: usize = 0x14;
    const DAY: usize = 0x2C;
    const NIGHT: usize = 0x44;
    const RADIO: usize = 0x5C;

    accessors! {
        u8 grass_rate / set_grass_rate @ 0x00;
        u8 surf_rate / set_surf_rate @ 0x01;
        u8 rock_rate / set_rock_rate @ 0x02;
        u8 old_rod_rate / set_old_rod_rate @ 0x03;
        u8 good_rod_rate / set_good_rod_rate @ 0x04;
        u8 super_rod_rate / set_super_rod_rate @ 0x05;
        u16 land_swarm / set_land_swarm @ 0xBC;
        u16 surf_swarm / set_surf_swarm @ 0xBE;
        // Not a swarm: the night slot of the fishing lists.
        u16 night_fish / set_night_fish @ 0xC0;
        u16 fish_swarm / set_fish_swarm @ 0xC2;
    }

    hgss_water! {
        surf / set_surf @ 0x64, 5;
        rock / set_rock @ 0x78, 2;
        old_rod / set_old_rod @ 0x80, 5;
        good_rod / set_good_rod @ 0x94, 5;
        super_rod / set_super_rod @ 0xA8, 5;
    }

    /// Level of grass slot `i` (0..12), shared by morning, day and night.
    pub fn grass_level(&self, i: usize) -> u8 {
        self.get_u8(Self::LEVELS + slot_index(i, 12))
    }

    pub fn set_grass_level(&mut self, i: usize, level: u8) {
        self.set_u8(Self::LEVELS + slot_index(i, 12), level);
    }

    pub fn grass_morning(&self, i: usize) -> u16 {
        self.get_u16(Self::MORNING + 2 * slot_index(i, 12))
    }

    pub fn set_grass_morning(&mut self, i: usize, species: u16) {
        self.set_u16(Self::MORNING + 2 * slot_index(i, 12), species);
    }

    pub fn grass_day(&self, i: usize) -> u16 {
        self.get_u16(Self::DAY + 2 * slot_index(i, 12))
    }

    pub fn set_grass_day(&mut self, i: usize, species: u16) {
        self.set_u16(Self::DAY + 2 * slot_index(i, 12), species);
    }

    pub fn grass_night(&self, i: usize) -> u16 {
        self.get_u16(Self::NIGHT + 2 * slot_index(i, 12))
    }

    pub fn set_grass_night(&mut self, i: usize, species: u16) {
        self.set_u16(Self::NIGHT + 2 * slot_index(i, 12), species);
    }

    /// Radio species: `0..2` Hoenn sound, `2..4` Sinnoh sound.
    pub fn radio(&self, i: usize) -> u16 {
        self.get_u16(Self::RADIO + 2 * slot_index(i, 4))
    }

    pub fn set_radio(&mut self, i: usize, species: u16) {
        self.set_u16(Self::RADIO + 2 * slot_index(i, 4), species);
    }

    /// The two padding bytes at 0x06 (zero in vanilla), kept as read.
    pub fn pad_06(&self) -> [u8; 2] {
        [self.raw[6], self.raw[7]]
    }

    pub fn set_pad_06(&mut self, bytes: [u8; 2]) {
        self.raw[6..8].copy_from_slice(&bytes);
    }

    fn water(&self, first: usize, i: usize, count: usize) -> WaterSlot {
        let at = first + 4 * slot_index(i, count);
        WaterSlot::from_hgss_bytes(self.raw[at..at + 4].try_into().expect("4 bytes"))
    }

    fn set_water(&mut self, first: usize, i: usize, count: usize, slot: WaterSlot) {
        let at = first + 4 * slot_index(i, count);
        let mut current = self.water(first, i, count);
        current.set_min(slot.min());
        current.set_max(slot.max());
        current.set_species(slot.species());
        self.raw[at..at + current.byte_len()].copy_from_slice(&current.raw[..current.byte_len()]);
    }
}

/// An encounter method (one tab of the editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Grass,
    Surf,
    RockSmash,
    OldRod,
    GoodRod,
    SuperRod,
}

/// One slot of a [`MethodView`]: `min == max` for grass (one level per slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotView {
    pub index: usize,
    pub species: u16,
    pub min: u8,
    pub max: u8,
    /// HeartGold grass only: the (morning, night) species next to `species`
    /// (the day species); `None` for Platinum and for every water and rock slot.
    pub alt_species: Option<[u16; 2]>,
}

/// One method of an encounter file as the editor sees it, the same for both
/// families: the rate (percent-like value, 0 = never) and its slots in file
/// order. Replacement lists (swarm, radar, radio, ...) are not part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodView {
    pub method: Method,
    pub rate: u32,
    pub slots: Vec<SlotView>,
}

fn water_view(method: Method, rate: u32, slots: impl Iterator<Item = WaterSlot>) -> MethodView {
    MethodView {
        method,
        rate,
        slots: slots
            .enumerate()
            .map(|(index, s)| SlotView {
                index,
                species: s.species(),
                min: s.min(),
                max: s.max(),
                alt_species: None,
            })
            .collect(),
    }
}

/// An encounter file of either family.
// Both variants are plain record buffers held by value (the planned API); a
// few hundred bytes per file do not justify a Box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encounters {
    Pt(PtEncounters),
    Hgss(HgssEncounters),
}

impl Encounters {
    /// Reads a member of `family`; Diamond and Pearl share Platinum's layout.
    /// Only a wrong length is an error.
    pub fn read(bytes: &[u8], family: Family) -> Result<Encounters, FormatError> {
        match family {
            Family::Dp | Family::Pt => PtEncounters::read(bytes).map(Encounters::Pt),
            Family::Hgss => HgssEncounters::read(bytes).map(Encounters::Hgss),
        }
    }

    /// The raw bytes, unchanged unless a setter ran.
    pub fn write(&self) -> Vec<u8> {
        match self {
            Encounters::Pt(e) => e.write(),
            Encounters::Hgss(e) => e.write(),
        }
    }

    /// The methods of the file in editor order: Platinum Grass, Surf and the
    /// three rods; HeartGold adds Rock Smash after Surf. A HeartGold grass slot
    /// carries the day species in `species` and (morning, night) in
    /// `alt_species`.
    pub fn methods(&self) -> Vec<MethodView> {
        match self {
            Encounters::Pt(e) => {
                let grass = MethodView {
                    method: Method::Grass,
                    rate: e.grass_rate(),
                    slots: (0..12)
                        .map(|index| {
                            let s = e.grass(index);
                            SlotView {
                                index,
                                species: s.species,
                                min: s.level,
                                max: s.level,
                                alt_species: None,
                            }
                        })
                        .collect(),
                };
                vec![
                    grass,
                    water_view(Method::Surf, e.surf_rate(), (0..5).map(|i| e.surf(i))),
                    water_view(
                        Method::OldRod,
                        e.old_rod_rate(),
                        (0..5).map(|i| e.old_rod(i)),
                    ),
                    water_view(
                        Method::GoodRod,
                        e.good_rod_rate(),
                        (0..5).map(|i| e.good_rod(i)),
                    ),
                    water_view(
                        Method::SuperRod,
                        e.super_rod_rate(),
                        (0..5).map(|i| e.super_rod(i)),
                    ),
                ]
            }
            Encounters::Hgss(e) => {
                let grass = MethodView {
                    method: Method::Grass,
                    rate: u32::from(e.grass_rate()),
                    slots: (0..12)
                        .map(|index| SlotView {
                            index,
                            species: e.grass_day(index),
                            min: e.grass_level(index),
                            max: e.grass_level(index),
                            alt_species: Some([e.grass_morning(index), e.grass_night(index)]),
                        })
                        .collect(),
                };
                vec![
                    grass,
                    water_view(
                        Method::Surf,
                        u32::from(e.surf_rate()),
                        (0..5).map(|i| e.surf(i)),
                    ),
                    water_view(
                        Method::RockSmash,
                        u32::from(e.rock_rate()),
                        (0..2).map(|i| e.rock(i)),
                    ),
                    water_view(
                        Method::OldRod,
                        u32::from(e.old_rod_rate()),
                        (0..5).map(|i| e.old_rod(i)),
                    ),
                    water_view(
                        Method::GoodRod,
                        u32::from(e.good_rod_rate()),
                        (0..5).map(|i| e.good_rod(i)),
                    ),
                    water_view(
                        Method::SuperRod,
                        u32::from(e.super_rod_rate()),
                        (0..5).map(|i| e.super_rod(i)),
                    ),
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::record::fields_cover_exactly;
    use proptest::prelude::*;

    #[test]
    fn pt_fields_cover_424_bytes_once() {
        assert_eq!(
            fields_cover_exactly(PtEncounters::FIELDS, PtEncounters::LEN),
            Ok(())
        );
        assert_eq!(PtEncounters::LEN, 424);
    }

    #[test]
    fn hgss_fields_cover_196_bytes_once() {
        assert_eq!(
            fields_cover_exactly(HgssEncounters::FIELDS, HgssEncounters::LEN),
            Ok(())
        );
        assert_eq!(HgssEncounters::LEN, 196);
    }

    #[test]
    fn pad_fields_follow_the_hex_scheme() {
        for fields in [PtEncounters::FIELDS, HgssEncounters::FIELDS] {
            for f in fields.iter().filter(|f| f.name.starts_with("pad_")) {
                assert_eq!(f.name, format!("pad_{:02x}", f.offset));
            }
        }
    }

    #[test]
    fn pt_water_slot_min_max_order_is_hidden() {
        let slot = WaterSlot::from_pt_bytes([0x1E, 0x14, 0, 0, 0x2A, 0, 0, 0]);
        assert_eq!(slot.min(), 20);
        assert_eq!(slot.max(), 30);
        assert_eq!(slot.species(), 42);

        let mut slot = slot;
        slot.set_min(25);
        assert_eq!(slot.raw[..2], [0x1E, 25], "min is byte 1");
        slot.set_max(40);
        assert_eq!(slot.raw[..2], [40, 25], "max is byte 0");

        // The same through the record: Surf slot 0 starts at 0xD0.
        let mut rec = PtEncounters::read(&[0; 424]).unwrap();
        rec.raw_mut()[0xD0..0xD2].copy_from_slice(&[0x1E, 0x14]);
        let mut s = rec.surf(0);
        assert_eq!((s.min(), s.max()), (20, 30));
        s.set_min(25);
        rec.set_surf(0, s);
        assert_eq!(rec.raw()[0xD0..0xD2], [0x1E, 25]);
    }

    #[test]
    fn hgss_water_slot_min_before_max() {
        let slot = WaterSlot::from_hgss_bytes([0x14, 0x1E, 0x2A, 0x01]);
        assert_eq!(slot.min(), 20);
        assert_eq!(slot.max(), 30);
        assert_eq!(slot.species(), 0x012A);

        let mut rec = HgssEncounters::read(&[0; 196]).unwrap();
        rec.raw_mut()[0x64..0x68].copy_from_slice(&[0x14, 0x1E, 0x2A, 0x01]);
        let mut s = rec.surf(0);
        assert_eq!((s.min(), s.max(), s.species()), (20, 30, 0x012A));
        s.set_max(35);
        rec.set_surf(0, s);
        assert_eq!(rec.raw()[0x64..0x68], [0x14, 35, 0x2A, 0x01]);
    }

    #[test]
    fn water_slot_setters_keep_padding_and_species_high_bits() {
        let mut rec = PtEncounters::read(&[0xFF; 424]).unwrap();
        let mut s = rec.good_rod(2); // 0x150 + 4 + 16
        s.set_max(2);
        s.set_min(1);
        s.set_species(3);
        rec.set_good_rod(2, s);
        assert_eq!(
            rec.raw()[0x164..0x16C],
            [2, 1, 0xFF, 0xFF, 3, 0, 0xFF, 0xFF]
        );

        // Writing a slot read from the other family only transfers min, max and species.
        let hg = WaterSlot::from_hgss_bytes([7, 9, 0x2C, 0x01]);
        rec.set_good_rod(2, hg);
        assert_eq!(
            rec.raw()[0x164..0x16C],
            [9, 7, 0xFF, 0xFF, 0x2C, 0x01, 0xFF, 0xFF]
        );
    }

    #[test]
    fn pt_grass_slot_keeps_the_raw_words() {
        let mut rec = PtEncounters::read(&[0xFF; 424]).unwrap();
        assert_eq!(
            rec.grass(0),
            Slot {
                level: 0xFF,
                species: 0xFFFF
            }
        );
        rec.set_grass(
            11,
            Slot {
                level: 7,
                species: 0x0102,
            },
        );
        // Slot 11 = 0x04 + 88: level word, species word.
        assert_eq!(
            rec.raw()[0x5C..0x64],
            [7, 0xFF, 0xFF, 0xFF, 0x02, 0x01, 0xFF, 0xFF]
        );
        assert_eq!(
            rec.grass(11),
            Slot {
                level: 7,
                species: 0x0102
            }
        );
        assert_eq!(rec.grass(10).level, 0xFF, "neighbour untouched");
    }

    #[test]
    fn pt_fields_sit_at_the_documented_offsets() {
        let mut rec = PtEncounters::read(&[0; 424]).unwrap();
        rec.set_grass_rate(0x0102_0304);
        assert_eq!(rec.raw()[..4], [4, 3, 2, 1]);
        assert_eq!(rec.grass_rate(), 0x0102_0304);

        rec.set_swarm(1, 11);
        rec.set_day(0, 12);
        rec.set_night(1, 13);
        rec.set_radar(3, 14);
        assert_eq!(rec.raw()[0x68], 11);
        assert_eq!(rec.raw()[0x6C], 12);
        assert_eq!(rec.raw()[0x78], 13);
        assert_eq!(rec.raw()[0x88], 14);
        assert_eq!(
            (rec.swarm(1), rec.day(0), rec.night(1), rec.radar(3)),
            (11, 12, 13, 14)
        );

        rec.set_form_rates([1, 2, 3, 4, 5]);
        assert_eq!(rec.raw()[0x8C], 1);
        assert_eq!(rec.raw()[0x9C], 5);
        assert_eq!(rec.form_rates(), [1, 2, 3, 4, 5]);
        rec.set_unown_table(8);
        assert_eq!(rec.raw()[0xA0], 8);
        assert_eq!(rec.unown_table(), 8);

        for (k, g) in DualSlot::ALL.into_iter().enumerate() {
            for i in 0..2 {
                rec.set_dual(g, i, 100 + (2 * k + i) as u16);
            }
        }
        assert_eq!(rec.raw()[0xA4], 100, "Ruby 0");
        assert_eq!(rec.raw()[0xAC], 102, "Sapphire 0");
        assert_eq!(rec.raw()[0xC8], 109, "LeafGreen 1");
        assert_eq!(rec.dual(DualSlot::Emerald, 1), 105);

        rec.set_surf_rate(1);
        rec.set_old_rod_rate(2);
        rec.set_good_rod_rate(3);
        rec.set_super_rod_rate(4);
        assert_eq!(
            [
                rec.raw()[0xCC],
                rec.raw()[0x124],
                rec.raw()[0x150],
                rec.raw()[0x17C]
            ],
            [1, 2, 3, 4]
        );
        assert_eq!(
            (
                rec.surf_rate(),
                rec.old_rod_rate(),
                rec.good_rod_rate(),
                rec.super_rod_rate()
            ),
            (1, 2, 3, 4)
        );
    }

    #[test]
    fn pt_water_slots_sit_at_the_documented_offsets() {
        let mut rec = PtEncounters::read(&[0; 424]).unwrap();
        let slot = |n: u8| {
            let mut s = WaterSlot::from_pt_bytes([0; 8]);
            s.set_max(n + 1);
            s.set_min(n);
            s.set_species(u16::from(n) + 300);
            s
        };
        rec.set_surf(4, slot(10)); // 0xCC + 4 + 32
        rec.set_old_rod(0, slot(20)); // 0x124 + 4
        rec.set_good_rod(1, slot(30)); // 0x150 + 12
        rec.set_super_rod(4, slot(40)); // 0x17C + 36
        assert_eq!(rec.raw()[0xF0..0xF2], [11, 10]);
        assert_eq!(rec.raw()[0x128..0x12A], [21, 20]);
        assert_eq!(rec.raw()[0x15C..0x15E], [31, 30]);
        assert_eq!(rec.raw()[0x1A0..0x1A2], [41, 40]);
        assert_eq!(rec.raw()[0x1A4..0x1A6], 340u16.to_le_bytes());
        assert_eq!(rec.super_rod(4).species(), 340);
        assert_eq!(rec.surf(4).min(), 10);
        assert_eq!(rec.old_rod(0).max(), 21);
        assert_eq!(rec.good_rod(1).min(), 30);
        assert!(rec.pad_f8().iter().all(|b| *b == 0));
    }

    #[test]
    fn dual_slot_order_matches_the_file_order() {
        for (k, g) in DualSlot::ALL.into_iter().enumerate() {
            assert_eq!(g as usize, k);
        }
    }

    #[test]
    fn pt_views_carry_no_alternative_species() {
        let views = Encounters::Pt(pt_with_one_of_everything()).methods();
        assert!(
            views
                .iter()
                .flat_map(|v| &v.slots)
                .all(|s| s.alt_species.is_none())
        );
    }

    #[test]
    fn pad_f8_is_the_44_byte_block() {
        let mut rec = PtEncounters::read(&[0x5A; 424]).unwrap();
        assert_eq!(rec.pad_f8().len(), 44);
        rec.set_pad_f8(&[1; 44]);
        assert_eq!(rec.raw()[0xF7], 0x5A);
        assert!(rec.raw()[0xF8..0x124].iter().all(|b| *b == 1));
        assert_eq!(rec.raw()[0x124], 0x5A);
    }

    #[test]
    fn hgss_fields_sit_at_the_documented_offsets() {
        let mut rec = HgssEncounters::read(&[0; 196]).unwrap();
        rec.set_grass_rate(1);
        rec.set_surf_rate(2);
        rec.set_rock_rate(3);
        rec.set_old_rod_rate(4);
        rec.set_good_rod_rate(5);
        rec.set_super_rod_rate(6);
        assert_eq!(rec.raw()[..6], [1, 2, 3, 4, 5, 6]);
        assert_eq!(
            (
                rec.grass_rate(),
                rec.surf_rate(),
                rec.rock_rate(),
                rec.old_rod_rate(),
                rec.good_rod_rate(),
                rec.super_rod_rate()
            ),
            (1, 2, 3, 4, 5, 6)
        );

        rec.set_grass_level(11, 50);
        rec.set_grass_morning(11, 0x0101);
        rec.set_grass_day(0, 0x0202);
        rec.set_grass_night(11, 0x0303);
        assert_eq!(rec.raw()[0x13], 50);
        assert_eq!(rec.raw()[0x2A..0x2C], 0x0101u16.to_le_bytes());
        assert_eq!(rec.raw()[0x2C..0x2E], 0x0202u16.to_le_bytes());
        assert_eq!(rec.raw()[0x5A..0x5C], 0x0303u16.to_le_bytes());
        assert_eq!(rec.grass_level(11), 50);
        assert_eq!(rec.grass_morning(11), 0x0101);
        assert_eq!(rec.grass_day(0), 0x0202);
        assert_eq!(rec.grass_night(11), 0x0303);

        rec.set_radio(0, 1);
        rec.set_radio(2, 3);
        rec.set_radio(3, 4);
        assert_eq!(rec.raw()[0x5C], 1);
        assert_eq!(rec.raw()[0x60], 3);
        assert_eq!(rec.raw()[0x62], 4);
        assert_eq!(rec.radio(3), 4);

        rec.set_land_swarm(1);
        rec.set_surf_swarm(2);
        rec.set_night_fish(3);
        rec.set_fish_swarm(4);
        assert_eq!(rec.raw()[0xBC..0xC4], [1, 0, 2, 0, 3, 0, 4, 0]);
        assert_eq!(
            (
                rec.land_swarm(),
                rec.surf_swarm(),
                rec.night_fish(),
                rec.fish_swarm()
            ),
            (1, 2, 3, 4)
        );
    }

    #[test]
    fn hgss_water_slots_sit_at_the_documented_offsets() {
        let mut rec = HgssEncounters::read(&[0; 196]).unwrap();
        let slot = |n: u8| {
            let mut s = WaterSlot::from_hgss_bytes([0; 4]);
            s.set_min(n);
            s.set_max(n + 1);
            s.set_species(u16::from(n) + 300);
            s
        };
        rec.set_surf(4, slot(10)); // 0x64 + 16
        rec.set_rock(1, slot(20)); // 0x78 + 4
        rec.set_old_rod(0, slot(30)); // 0x80
        rec.set_good_rod(4, slot(40)); // 0x94 + 16
        rec.set_super_rod(4, slot(50)); // 0xA8 + 16
        assert_eq!(rec.raw()[0x74..0x76], [10, 11]);
        assert_eq!(rec.raw()[0x7C..0x7E], [20, 21]);
        assert_eq!(rec.raw()[0x80..0x82], [30, 31]);
        assert_eq!(rec.raw()[0xA4..0xA6], [40, 41]);
        assert_eq!(rec.raw()[0xB8..0xBA], [50, 51]);
        assert_eq!(rec.raw()[0xBA..0xBC], 350u16.to_le_bytes());
        assert_eq!(rec.rock(1).min(), 20);
        assert_eq!(rec.super_rod(4).species(), 350);
        assert_eq!(rec.pad_06(), [0, 0]);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn an_out_of_range_slot_index_panics_instead_of_hitting_a_neighbour() {
        let rec = PtEncounters::read(&[0; 424]).unwrap();
        let _ = rec.grass(12);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn a_rock_smash_index_past_two_panics() {
        let rec = HgssEncounters::read(&[0; 196]).unwrap();
        let _ = rec.rock(2);
    }

    fn pt_with_one_of_everything() -> PtEncounters {
        let mut rec = PtEncounters::read(&[0; 424]).unwrap();
        rec.set_grass_rate(10);
        for i in 0..12 {
            rec.set_grass(
                i,
                Slot {
                    level: 2 + i as u8,
                    species: 100 + i as u16,
                },
            );
        }
        rec.set_surf_rate(5);
        rec.set_old_rod_rate(6);
        rec.set_good_rod_rate(7);
        rec.set_super_rod_rate(8);
        for i in 0..5 {
            for (k, set) in [
                PtEncounters::set_surf as fn(&mut PtEncounters, usize, WaterSlot),
                PtEncounters::set_old_rod,
                PtEncounters::set_good_rod,
                PtEncounters::set_super_rod,
            ]
            .into_iter()
            .enumerate()
            {
                let mut s = WaterSlot::from_pt_bytes([0; 8]);
                s.set_min(10 * k as u8 + i as u8);
                s.set_max(10 * k as u8 + i as u8 + 5);
                s.set_species(200 + 10 * k as u16 + i as u16);
                set(&mut rec, i, s);
            }
        }
        rec
    }

    #[test]
    fn methods_view_lists_grass_surf_and_rods_with_slot_counts() {
        let pt = Encounters::Pt(pt_with_one_of_everything());
        let views = pt.methods();
        let shape: Vec<(Method, usize)> = views.iter().map(|v| (v.method, v.slots.len())).collect();
        assert_eq!(
            shape,
            [
                (Method::Grass, 12),
                (Method::Surf, 5),
                (Method::OldRod, 5),
                (Method::GoodRod, 5),
                (Method::SuperRod, 5),
            ]
        );
        assert_eq!(
            views.iter().map(|v| v.rate).collect::<Vec<_>>(),
            [10, 5, 6, 7, 8]
        );
        // Platinum grass has one level: min == max.
        assert_eq!(
            views[0].slots[3],
            SlotView {
                index: 3,
                species: 103,
                min: 5,
                max: 5,
                alt_species: None
            }
        );
        // Water: min and max come out in the right order whatever the bytes say.
        assert_eq!(
            views[1].slots[2],
            SlotView {
                index: 2,
                species: 202,
                min: 2,
                max: 7,
                alt_species: None
            }
        );
        assert_eq!(
            views[4].slots[4],
            SlotView {
                index: 4,
                species: 234,
                min: 34,
                max: 39,
                alt_species: None
            }
        );

        let hgss = Encounters::Hgss(HgssEncounters::read(&[0; 196]).unwrap());
        let shape: Vec<(Method, usize)> = hgss
            .methods()
            .iter()
            .map(|v| (v.method, v.slots.len()))
            .collect();
        assert_eq!(
            shape,
            [
                (Method::Grass, 12),
                (Method::Surf, 5),
                (Method::RockSmash, 2),
                (Method::OldRod, 5),
                (Method::GoodRod, 5),
                (Method::SuperRod, 5),
            ]
        );
    }

    #[test]
    fn hgss_methods_view_reads_rates_levels_and_the_day_list() {
        let mut rec = HgssEncounters::read(&[0; 196]).unwrap();
        rec.set_grass_rate(30);
        rec.set_rock_rate(20);
        rec.set_grass_level(1, 9);
        rec.set_grass_morning(1, 10);
        rec.set_grass_day(1, 11);
        rec.set_grass_night(1, 12);
        let mut s = rec.rock(1);
        s.set_min(3);
        s.set_max(5);
        s.set_species(77);
        rec.set_rock(1, s);

        let views = Encounters::Hgss(rec).methods();
        assert_eq!(views[0].rate, 30);
        assert_eq!(
            views[0].slots[1],
            SlotView {
                index: 1,
                species: 11,
                min: 9,
                max: 9,
                alt_species: Some([10, 12])
            },
            "species is the day list, alt_species is (morning, night)"
        );
        assert!(
            views[1..]
                .iter()
                .flat_map(|v| &v.slots)
                .all(|s| s.alt_species.is_none()),
            "water and rock slots carry no alternative species"
        );
        assert_eq!(views[2].method, Method::RockSmash);
        assert_eq!(views[2].rate, 20);
        assert_eq!(
            views[2].slots[1],
            SlotView {
                index: 1,
                species: 77,
                min: 3,
                max: 5,
                alt_species: None
            }
        );
    }

    #[test]
    fn encounters_read_picks_the_record_by_family() {
        let pt = vec![0u8; 424];
        let hg = vec![0u8; 196];
        assert!(matches!(
            Encounters::read(&pt, Family::Pt),
            Ok(Encounters::Pt(_))
        ));
        assert!(
            matches!(Encounters::read(&pt, Family::Dp), Ok(Encounters::Pt(_))),
            "Diamond and Pearl read like Platinum"
        );
        assert!(matches!(
            Encounters::read(&hg, Family::Hgss),
            Ok(Encounters::Hgss(_))
        ));
        assert!(matches!(
            Encounters::read(&hg, Family::Pt),
            Err(FormatError::BadSize {
                expected: 424,
                found: 196
            })
        ));
        assert!(matches!(
            Encounters::read(&pt, Family::Hgss),
            Err(FormatError::BadSize {
                expected: 196,
                found: 424
            })
        ));
    }

    proptest! {
        #[test]
        fn round_trip_random_bytes_both_families(
            pt in proptest::collection::vec(any::<u8>(), 424),
            hg in proptest::collection::vec(any::<u8>(), 196),
        ) {
            for family in [Family::Dp, Family::Pt] {
                let enc = Encounters::read(&pt, family).unwrap();
                prop_assert_eq!(enc.write(), pt.clone());
            }
            let enc = Encounters::read(&hg, Family::Hgss).unwrap();
            prop_assert_eq!(enc.write(), hg.clone());
        }

        #[test]
        fn views_of_random_bytes_never_panic(
            pt in proptest::collection::vec(any::<u8>(), 424),
            hg in proptest::collection::vec(any::<u8>(), 196),
        ) {
            prop_assert_eq!(Encounters::read(&pt, Family::Pt).unwrap().methods().len(), 5);
            prop_assert_eq!(Encounters::read(&hg, Family::Hgss).unwrap().methods().len(), 6);
        }

        #[test]
        fn only_the_exact_lengths_are_accepted(len in 0usize..600) {
            let bytes = vec![0x11; len];
            prop_assert_eq!(Encounters::read(&bytes, Family::Pt).is_ok(), len == 424);
            prop_assert_eq!(Encounters::read(&bytes, Family::Dp).is_ok(), len == 424);
            prop_assert_eq!(Encounters::read(&bytes, Family::Hgss).is_ok(), len == 196);
        }

        #[test]
        fn a_water_setter_changes_only_its_own_bytes(
            bytes in proptest::collection::vec(any::<u8>(), 424),
            i in 0usize..5,
            min in any::<u8>(),
            max in any::<u8>(),
            species in any::<u16>(),
        ) {
            let mut rec = PtEncounters::read(&bytes).unwrap();
            let mut s = rec.super_rod(i);
            s.set_min(min);
            s.set_max(max);
            s.set_species(species);
            rec.set_super_rod(i, s);
            let out = rec.write();
            let base = 0x17C + 4 + 8 * i;
            prop_assert_eq!(&out[..base], &bytes[..base]);
            prop_assert_eq!(&out[base + 2..base + 4], &bytes[base + 2..base + 4]);
            prop_assert_eq!(&out[base + 6..], &bytes[base + 6..]);
            prop_assert_eq!(out[base], max);
            prop_assert_eq!(out[base + 1], min);
            prop_assert_eq!(rec.super_rod(i).species(), species);
        }
    }
}
