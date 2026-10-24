//! What the data commands send and receive; `apps/studio/src/api.data.ts` mirrors every type.

use g4_formats::data::evolution::EvoEntry;
use g4_formats::data::learnset::LearnEntry;
use g4_formats::data::personal::Personal;
use g4_kernel::project::AssetId;
use serde::{Deserialize, Serialize};

use super::fields::{FieldSpanDto, FieldValueDto};

/// The data a command works on. `species`, `learnset` and `evolution` share one document
/// (`DataDocDto::Species`); `trainer` is listed only (its editor comes with 3b.3b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataKind {
    Species,
    Learnset,
    Evolution,
    Move,
    Item,
    Encounter,
    Trainer,
}

/// Narrows `data_list`: `query` matches the name (any case) or the number, `types` keeps
/// species and moves of one of these types (empty: all), `pocket` items of that field pocket,
/// `category` moves of that category.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DataFilter {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub types: Vec<u8>,
    #[serde(default)]
    pub pocket: Option<u8>,
    #[serde(default)]
    pub category: Option<u8>,
}

/// One row of a data list. `name` is the kernel's name (English fallback such as
/// "species 0520" when the text archive has none; empty for an item without the arm9 item
/// table and for encounter files), `tech` the four-digit index, `locked` the reason a member
/// cannot be edited (then `extra` is null).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DataRowDto {
    pub index: u16,
    pub name: String,
    pub tech: String,
    pub locked: Option<String>,
    pub extra: serde_json::Value,
}

/// The personal data of one species or form, field by field (the names of `Personal::FIELDS`;
/// `DataPatch::Personal` takes the same names).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PersonalDto {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub sp_attack: u8,
    pub sp_defense: u8,
    pub type1: u8,
    pub type2: u8,
    pub catch_rate: u8,
    pub base_exp: u8,
    pub ev_yield_hp: u8,
    pub ev_yield_attack: u8,
    pub ev_yield_defense: u8,
    pub ev_yield_speed: u8,
    pub ev_yield_sp_attack: u8,
    pub ev_yield_sp_defense: u8,
    pub item1: u16,
    pub item2: u16,
    pub gender_ratio: u8,
    pub egg_cycles: u8,
    pub base_friendship: u8,
    pub growth_rate: u8,
    pub egg_group1: u8,
    pub egg_group2: u8,
    pub ability1: u8,
    pub ability2: u8,
    pub flee_rate: u8,
    pub color: u8,
    pub flip: bool,
}

impl From<&Personal> for PersonalDto {
    fn from(p: &Personal) -> Self {
        PersonalDto {
            hp: p.hp(),
            attack: p.attack(),
            defense: p.defense(),
            speed: p.speed(),
            sp_attack: p.sp_attack(),
            sp_defense: p.sp_defense(),
            type1: p.type1(),
            type2: p.type2(),
            catch_rate: p.catch_rate(),
            base_exp: p.base_exp(),
            ev_yield_hp: p.ev_yield(0),
            ev_yield_attack: p.ev_yield(1),
            ev_yield_defense: p.ev_yield(2),
            ev_yield_speed: p.ev_yield(3),
            ev_yield_sp_attack: p.ev_yield(4),
            ev_yield_sp_defense: p.ev_yield(5),
            item1: p.item1(),
            item2: p.item2(),
            gender_ratio: p.gender_ratio(),
            egg_cycles: p.egg_cycles(),
            base_friendship: p.base_friendship(),
            growth_rate: p.growth_rate(),
            egg_group1: p.egg_group1(),
            egg_group2: p.egg_group2(),
            ability1: p.ability1(),
            ability2: p.ability2(),
            flee_rate: p.flee_rate(),
            color: p.color(),
            flip: p.flip(),
        }
    }
}

/// One level-up move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnEntryDto {
    pub move_id: u16,
    pub level: u8,
}

impl From<&LearnEntry> for LearnEntryDto {
    fn from(e: &LearnEntry) -> Self {
        LearnEntryDto {
            move_id: e.move_id,
            level: e.level,
        }
    }
}

/// One of the seven evolution entries (method 0: empty).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct EvoEntryDto {
    pub method: u16,
    pub param: u16,
    pub target: u16,
}

impl From<EvoEntry> for EvoEntryDto {
    fn from(e: EvoEntry) -> Self {
        EvoEntryDto {
            method: e.method,
            param: e.param,
            target: e.target,
        }
    }
}

/// A description and where it lives (text archive and message), for "Beschreibung bearbeiten".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptionDto {
    pub archive: u16,
    pub message: u16,
    pub text: String,
}

/// One slot of an encounter method. `min == max` for grass (one level per slot); HeartGold
/// grass carries the morning and night species next to `species` (the day species).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotDto {
    pub index: u16,
    pub species: u16,
    pub min: u8,
    pub max: u8,
    pub species_morning: Option<u16>,
    pub species_night: Option<u16>,
}

/// One encounter method (`grass`, `surf`, `rock_smash` (HeartGold), `old_rod`, `good_rod`,
/// `super_rod`) with its rate (0 = never) and slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MethodDto {
    pub method: &'static str,
    pub rate: u32,
    pub slots: Vec<SlotDto>,
}

/// A replacement list of an encounter file (Platinum `swarm`, `day`, `night`, `radar`,
/// `dual_*`; HeartGold `radio`, `land_swarm`, `surf_swarm`, `night_fish`, `fish_swarm`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtraListDto {
    pub list: &'static str,
    pub species: Vec<u16>,
}

/// The document of one data member, tagged by `kind`. `locked` is the reason a member cannot
/// be edited and `raw_locked` its bytes as read; the decoded fields of a locked part are then
/// empty (Species: zeros) and must not be shown as values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DataDocDto {
    /// Personal data, learnset, evolutions and TM/HM bits of one species or form. `saved` is
    /// the personal data as last saved ("vorher").
    Species {
        personal: PersonalDto,
        saved: PersonalDto,
        learnset: Vec<LearnEntryDto>,
        evolution: [EvoEntryDto; 7],
        /// TM01..TM92, HM01..HM08.
        tm: Vec<bool>,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
        /// The part `locked` and `raw_locked` are about (personal data, learnset or
        /// evolutions; the first locked one).
        locked_asset: Option<AssetId>,
    },
    /// A move. `name_archive` is the text archive of the move names (message = move index);
    /// `None` where the game's banks do not name it.
    Move {
        fields: Vec<FieldValueDto>,
        description: Option<DescriptionDto>,
        name_archive: Option<u16>,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
    },
    /// An item: by item id with the arm9 item table, else by data member. `shared_with` lists
    /// the other item ids that use the same data member. `name_archive` is the text archive of
    /// the item names (message = item id); `None` without the item table or a known bank.
    Item {
        fields: Vec<FieldValueDto>,
        description: Option<DescriptionDto>,
        name_archive: Option<u16>,
        shared_with: Vec<u16>,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
    },
    Encounter {
        methods: Vec<MethodDto>,
        extras: Vec<ExtraListDto>,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
    },
}

/// One change of a data member (externally tagged, like `TextEdit`). Each names the part it
/// changes; `CopyMoves` changes two in one undo step.
#[derive(Debug, Clone, Deserialize)]
pub enum DataPatch {
    /// A field of the personal data, by its name in `PersonalDto`.
    Personal {
        field: String,
        value: u32,
    },
    /// TM/HM `n` (0 is TM01, 92 is HM01) on or off.
    Tm {
        n: u8,
        on: bool,
    },
    /// The whole learnset.
    Learnset {
        entries: Vec<LearnEntryDto>,
    },
    /// Evolution entry `slot` (0 to 6).
    Evolution {
        slot: u8,
        method: u16,
        param: u16,
        target: u16,
    },
    Move {
        field: String,
        value: i64,
    },
    Item {
        field: String,
        value: i64,
    },
    /// The TM/HM bits and the learnset of species `from`.
    CopyMoves {
        from: u16,
    },
    /// `field` of slot `slot` of `method` (a method or a replacement list, see
    /// [`MethodDto`] and [`ExtraListDto`]); the method's `rate` takes no slot.
    Encounter {
        method: String,
        slot: Option<u16>,
        field: String,
        value: i64,
    },
}

/// The raw bytes of a member as the store holds them, the field map that describes them
/// (empty for a locked member) and where they come from (packed archive, member).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RawDto {
    pub bytes: Vec<u8>,
    pub fields: Vec<FieldSpanDto>,
    pub path: String,
    pub member: u16,
}

/// The name tables of the project. `species` and `moves` hold one name per data member
/// (forms 496 to 507 from `species_forms.yaml`, English fallbacks "species 0520" and
/// "move 0469" where the text archive has none); `items` is indexed by item id; the other
/// tables are the text archives' messages as they are.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NameTablesDto {
    pub species: Vec<String>,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub types: Vec<String>,
    pub classes: Vec<String>,
    pub forms: Vec<String>,
}

/// One entry of the arm9 item table: data member, icon (NCGR) and palette (NCLR) member of
/// `itemIcons`, GBA item id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ItemEntryDto {
    pub data: u16,
    pub icon: u16,
    pub palette: u16,
    pub gba: u16,
}

/// What `refs` looks for (`item` by item id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetDto {
    Species { index: u16 },
    Move { index: u16 },
    Item { id: u16 },
    Trainer { index: u16 },
    TrainerClass { index: u8 },
    Encounter { index: u16 },
}

/// Where a reference leads: the shapes of the frontend's `NavTarget`. `encounter` and
/// `trainer` have no editor before 3b.3b; the UI shows them disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NavTargetDto {
    Header {
        id: u16,
    },
    #[serde(rename_all = "camelCase")]
    Event {
        event_file: u16,
        event_kind: &'static str,
        index: u16,
    },
    Species {
        index: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<&'static str>,
    },
    Encounter {
        index: u16,
    },
    Trainer {
        index: u16,
    },
}

/// One reference: `field` (slot `slot`) of `source` points at the target. `label` says where
/// in German ("Begegnungen 0012, Gras Platz 3"), `nav` where a click leads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefDto {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub label: String,
    pub nav: NavTargetDto,
}
