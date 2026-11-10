//! What the data commands send and receive; `apps/studio/src/api.data.ts` mirrors every type.

use std::collections::BTreeMap;

use g4_formats::data::evolution::EvoEntry;
use g4_formats::data::learnset::LearnEntry;
use g4_formats::data::personal::Personal;
use g4_kernel::checks::{MapCell, Subject};
use g4_kernel::data::battle_sprites::SpriteSlots;
use g4_kernel::data::egg_moves::EggMovesSource;
use g4_kernel::project::AssetId;
use serde::{Deserialize, Serialize};

use super::fields::{FieldSpanDto, FieldValueDto};
use crate::commands::ProblemDto;

/// The data a command works on. `species`, `learnset` and `evolution` share one document
/// (`DataDocDto::Species`).
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
/// cannot be edited (then `extra` is null), `deleted` whether the entry is a tombstone (3d.4:
/// trainers and encounter files of a text project; never for the other kinds).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DataRowDto {
    pub index: u16,
    pub name: String,
    pub tech: String,
    pub locked: Option<String>,
    pub extra: serde_json::Value,
    pub deleted: bool,
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

/// A replacement list entry that can take a slot's place: entry `entry` of list `list`
/// (the kernel's key, see [`ReplacementListDto`]) `when` (English, the kernel's words: "from
/// 10 to 19 o'clock", "during a swarm"). `in_file` is false for Trophy Garden and Great Marsh,
/// whose species are not in the encounter file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplacedByDto {
    pub list: &'static str,
    pub entry: u16,
    pub when: &'static str,
    pub in_file: bool,
}

/// A slot a replacement list entry takes the place of: slot `slot` of method `method`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotTargetDto {
    pub method: &'static str,
    pub slot: u16,
}

/// One slot of an encounter method, or one entry of a replacement list. `min == max` for
/// grass (one level per slot); HeartGold grass carries the morning and night species next to
/// `species` (the day species). `share` is the percent the game picks the slot with (for a
/// list entry: the shares of the slots it replaces in its first method added up);
/// `replaced_by` lists the replacement entries that can take a method slot's place (empty
/// for a list entry), `replaces` the slots a list entry replaces (empty for a method slot).
/// A list entry takes the level of the first slot it replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotDto {
    pub index: u16,
    pub species: u16,
    pub min: u8,
    pub max: u8,
    pub species_morning: Option<u16>,
    pub species_night: Option<u16>,
    pub share: u8,
    pub replaced_by: Vec<ReplacedByDto>,
    pub replaces: Vec<SlotTargetDto>,
}

/// One encounter method (`grass`, `surf`, `rock_smash` (HeartGold), `old_rod`, `good_rod`,
/// `super_rod`) with its rate (0 = never) and slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MethodDto {
    pub method: &'static str,
    pub rate: u32,
    pub slots: Vec<SlotDto>,
}

/// A replacement list stored in the encounter file, by the kernel's key (Platinum `swarm`,
/// `day`, `night`, `radar`, `dual_ruby` … `dual_leafgreen`; HeartGold `radio_hoenn`,
/// `radio_sinnoh`, `land_swarm`, `surf_swarm`, `night_fish`, `fish_swarm`), with its
/// `label` in the language of the process ("Day", "Hoenn Sound"; "Tag", "Radio Hoenn" in
/// German), when the game uses it (English) and its entries as slots.
/// `DataPatch::Encounter` takes `list` as `method` and the entry as `slot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplacementListDto {
    pub list: &'static str,
    pub label: String,
    pub when: &'static str,
    pub slots: Vec<SlotDto>,
}

/// The values of a Platinum encounter file that are no slot: the five form rates (Shellos
/// and Gastrodon) and the Unown table. `DataPatch::EncounterExtra` takes `form_rate_0` to
/// `form_rate_4` and `unown_table`. Both `None` in HeartGold.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EncounterExtrasDto {
    pub form_rates: Option<[u32; 5]>,
    pub unown_table: Option<u32>,
}

/// A rule the file breaks (shown, never enforced): the method key and the slot (0-based;
/// `None`: the method as a whole), the kernel's English text with 0-based numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleWarningDto {
    pub method: &'static str,
    pub slot: Option<u16>,
    pub text: String,
}

/// A header (place) and its name: the in-game location name, else the internal name, else
/// "Header NNNN".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceDto {
    pub header: u16,
    pub name: String,
}

/// A species and its share in percent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpeciesShareDto {
    pub species: u16,
    pub share: u8,
}

/// Who waits in the grass at one time of day (`morning`, `day`, `night`, the kernel's keys),
/// largest share first, without swarm, radar, radio and cartridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimeSharesDto {
    pub when: &'static str,
    pub species: Vec<SpeciesShareDto>,
}

/// Whether the party entries carry their own moves and a held item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyFormatDto {
    pub moves: bool,
    pub items: bool,
}

/// One Pokémon of a trainer party. `item` is `Some` exactly when the trainer's format has
/// items, `moves` when it has moves; `ball_capsule` is `Some` in Platinum and HeartGold.
/// `gender` (bits 0 to 3 of `byte1`) and `ability` (bits 4 to 7) are `Some` in HeartGold only;
/// there, a patch with both set writes `byte1` from them, else `byte1` is written as given.
/// `species` takes 10 bits, `form` 6.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyMonDto {
    pub iv_or_difficulty: u8,
    pub byte1: u8,
    pub level: u16,
    pub species: u16,
    pub form: u8,
    pub item: Option<u16>,
    pub moves: Option<[u16; 4]>,
    pub ball_capsule: Option<u16>,
    #[serde(default)]
    pub gender: Option<u8>,
    #[serde(default)]
    pub ability: Option<u8>,
}

/// One battle text of a trainer: its kind (0 before the battle, 1 lost, 2 after the battle,
/// …), where it lives (battle text archive, message) and the text in chatot form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrainerTextDto {
    pub kind: u16,
    pub archive: u16,
    pub message: u16,
    pub text: String,
}

/// Whether a species has battle images of its own in the female slots and in the male (or
/// only) slots (`battle_sprites::sprite_slots`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpriteSlotsDto {
    pub female: bool,
    pub male: bool,
}

impl From<SpriteSlots> for SpriteSlotsDto {
    fn from(slots: SpriteSlots) -> Self {
        SpriteSlotsDto {
            female: slots.female,
            male: slots.male,
        }
    }
}

/// How much of a fixed budget a table fills, in words (Platinum's egg moves in overlay 5:
/// marks, moves and the end all count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BudgetDto {
    pub used: u32,
    pub total: u32,
}

/// The egg moves of one species: its moves (empty without an entry), where the table is read
/// from (`narc`, `patch`, `overlay`), the budget of a table in overlay 5 (`None` for a NARC
/// member), and `locked`, the reason the table cannot be read or edited (then `moves` is empty
/// and must not be shown as the species' moves).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EggMovesDto {
    pub moves: Vec<u16>,
    pub source: EggMovesSource,
    pub budget: Option<BudgetDto>,
    pub locked: Option<String>,
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
        /// Which genders have battle images of their own (`get_battle_sprite` falls back to
        /// the other gender's silently); `None` when the game has no readable sprites.
        sprite_slots: Option<SpriteSlotsDto>,
        /// The species' egg moves; `None` where the game has no egg move table
        /// (Diamond/Pearl).
        egg_moves: Option<EggMovesDto>,
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
    /// An encounter file: its methods with shares and replacements, the replacement lists it
    /// stores, the Platinum extras, the rules it breaks (`warnings`, and the same as Problems
    /// rows of source `encounter` and file `encounters/NNNN` in `problems`, which replace the
    /// file's previous rows), the headers that use it and who waits in the grass per time of
    /// day.
    Encounter {
        methods: Vec<MethodDto>,
        replacements: Vec<ReplacementListDto>,
        extras: EncounterExtrasDto,
        warnings: Vec<RuleWarningDto>,
        problems: Vec<ProblemDto>,
        users: Vec<PlaceDto>,
        effective: Vec<TimeSharesDto>,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
    },
    /// A trainer: properties, party, name (without the `{TRAINER_NAME:…}` wrapper), battle
    /// texts and the object events that start its battle (references, as `refs` gives them).
    /// `placeholder` marks the locked placeholder of the sight script slot. `name_archive` and
    /// `text_archive` are the trainer name and battle text banks; `texts_locked` the reason
    /// the battle texts cannot be read or written (table and archive differ). `problems` are
    /// the Problems rows of source `trainer` and file `trainers/NNNN` (the text before the
    /// battle should end with `\r`).
    Trainer {
        class: u8,
        name: String,
        format: PartyFormatDto,
        ai: u32,
        battle_type: u32,
        items: [u16; 4],
        party: Vec<PartyMonDto>,
        texts: Vec<TrainerTextDto>,
        texts_locked: Option<String>,
        events: Vec<RefDto>,
        name_archive: Option<u16>,
        text_archive: Option<u16>,
        problems: Vec<ProblemDto>,
        placeholder: bool,
        locked: Option<String>,
        raw_locked: Option<Vec<u8>>,
    },
}

/// One battle text to write: its kind and the text in chatot form (empty: a kind the trainer
/// does not have yet is not added).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TrainerTextInDto {
    pub kind: u16,
    pub text: String,
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
    /// [`MethodDto`] and [`ReplacementListDto`]); the method's `rate` takes no slot. Levels
    /// are pulled along on input: a water `min` above `max` raises `max` to it, a `max`
    /// below `min` lowers `min` (one step).
    Encounter {
        method: String,
        slot: Option<u16>,
        field: String,
        value: i64,
    },
    /// Slots `a` and `b` (0-based) of `method` trade places: species (HeartGold grass: all
    /// three times of day) and levels, one step. Refused before writing when `a == b`, a slot
    /// lies outside the method or `method` is a replacement list.
    EncounterSwap {
        method: String,
        a: u8,
        b: u8,
    },
    /// A Platinum value that is no slot: `form_rate_0` to `form_rate_4`, `unown_table`.
    EncounterExtra {
        field: String,
        value: i64,
    },
    /// A field of the trainer properties: `class`, `ai`, `battle_type`, `item0` to `item3`.
    Trainer {
        field: String,
        value: i64,
    },
    /// The trainer's name (at most [`g4_kernel::data::trainer_rules::NAME_MAX_CHARS`]
    /// characters), written to the trainer name bank at index = trainer.
    TrainerName {
        name: String,
    },
    /// Own moves and held items on or off for the whole party (`None`: unchanged). Moves on
    /// fill each Pokémon with the moves the game would give it (the learnset of its species'
    /// base form up to its level); off drops them. Items on give every Pokémon item 0, off
    /// drops the items. One step.
    TrainerFormat {
        moves: Option<bool>,
        items: Option<bool>,
    },
    /// The whole party (at most six), written in the trainer's format.
    TrainerParty {
        party: Vec<PartyMonDto>,
    },
    /// Battle texts, as `set_trainer_texts` writes them.
    TrainerTexts {
        texts: Vec<TrainerTextInDto>,
    },
    /// The species' whole egg move list (at most [`EGG_MOVES_MAX`] moves, each a move of the
    /// game, not 0; empty removes the species' entry). On Platinum without DSPRE's patch the
    /// table must stay within its budget in overlay 5: adding needs a move removed elsewhere.
    EggMoves {
        moves: Vec<u16>,
    },
}

/// The most egg moves `DataPatch::EggMoves` takes for one species (the fields `move0` to
/// `move15` of `g4k check-data egg_moves`; vanilla species have at most 14).
pub const EGG_MOVES_MAX: usize = 16;

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
/// tables are the text archives' messages as they are; `sounds` are the music pieces of the
/// family's command database, sorted by id.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NameTablesDto {
    pub species: Vec<String>,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub types: Vec<String>,
    pub classes: Vec<String>,
    pub forms: Vec<String>,
    /// The text archives the tables are read from.
    pub archives: NameArchivesDto,
    pub sounds: Vec<SoundNameDto>,
}

/// One music piece: its id (`music_day_id`, `music_night_id`, the script's `PlayMusic`), its
/// constant name (`SEQ_TOWN01`) and where the game plays it, when the database says so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoundNameDto {
    pub id: u16,
    pub name: String,
    pub used_in: Option<String>,
}

/// The text archive of each name table (`None` where the game's banks do not name one): an
/// edit to one of these archives changes the names, and the rows that carry them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NameArchivesDto {
    pub species: Option<u16>,
    pub moves: Option<u16>,
    pub items: Option<u16>,
    pub abilities: Option<u16>,
    pub types: Option<u16>,
    pub classes: Option<u16>,
    pub forms: Option<u16>,
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
    Message { archive: u16, index: u16 },
}

/// Where a link leads: the shapes of the frontend's `NavTarget` (`workbench/navigation.ts`),
/// all of them (`api.data.ts` mirrors it and a type test keeps both equal). A warp is an
/// `event` with `event_kind` `warp`; `header` is a header known to show the target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NavTargetDto {
    Header {
        id: u16,
    },
    MatrixCell {
        matrix: u16,
        x: u8,
        y: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        header: Option<u16>,
    },
    Area {
        id: u16,
    },
    /// A building of a map (a reference of the map to its model).
    Building {
        map: u16,
        index: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        header: Option<u16>,
    },
    /// `event_kind`: `warp`, `object`, `coord` or `bg`.
    #[serde(rename_all = "camelCase")]
    Event {
        event_file: u16,
        event_kind: &'static str,
        index: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        header: Option<u16>,
    },
    /// A text archive, at one of its messages (none: the first).
    TextArchive {
        index: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<u16>,
    },
    /// A script file, at a 1-based line (none: where the cursor was).
    Script {
        file: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
    },
    Species {
        index: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<&'static str>,
    },
    // Mirrors `NavTarget`; no backend answer leads to a move yet.
    #[allow(dead_code)]
    Move {
        index: u16,
    },
    /// An item by item id. Mirrors `NavTarget`; no backend answer leads to an item yet.
    #[allow(dead_code)]
    Item {
        id: u16,
    },
    Trainer {
        index: u16,
    },
    Encounter {
        index: u16,
    },
    /// A flag or a variable in the list of flags and variables (Technik). Mirrors
    /// `NavTarget`; no backend answer leads to one yet.
    #[allow(dead_code)]
    #[serde(rename_all = "camelCase")]
    Flag {
        flag_kind: g4_kernel::flags::FlagKind,
        number: u16,
    },
    /// A story in Geschichten › Geschichten, by its number. Mirrors `NavTarget`; no backend
    /// answer leads to one yet.
    #[allow(dead_code)]
    Story {
        number: u16,
    },
    /// A thread in Geschichten › Fäden, by its variable. Mirrors `NavTarget`; no backend answer
    /// leads to one yet.
    #[allow(dead_code)]
    Thread {
        var: u16,
    },
}

/// Where a finding about `subject` leads (spec 3b.4 §5): the spot its `key` names when the
/// workbench opens one (`warp:2`, `sign:1` → that event, `message:12…` → that message,
/// `line:7:…` → that script line, `cell:3,26` → that matrix cell), else the subject as a whole.
/// A map leads to the first matrix cell that places it (`map_cells` of the report). `None`
/// where the workbench has nothing to open: the project, an event file or a matrix without a
/// spot, a map no matrix places.
pub fn subject_to_nav(
    subject: Subject,
    key: &str,
    map_cells: &BTreeMap<u16, MapCell>,
) -> Option<NavTargetDto> {
    let parts: Vec<&str> = key.split(':').collect();
    // The number after `name` in the key ("message:12:line:2" → 12 for "message").
    let number = |name: &str| -> Option<u32> {
        let at = parts.iter().position(|p| *p == name)?;
        parts.get(at + 1)?.parse().ok()
    };
    Some(match subject {
        Subject::Header(id) => NavTargetDto::Header { id },
        Subject::EventFile(event_file) => {
            let event_kind = match *parts.first()? {
                "warp" => "warp",
                "object" => "object",
                "sign" => "bg",
                "trigger" => "coord",
                _ => return None,
            };
            NavTargetDto::Event {
                event_file,
                event_kind,
                index: parts.get(1)?.parse().ok()?,
                header: None,
            }
        }
        Subject::ScriptFile(file) => NavTargetDto::Script {
            file,
            line: number("line"),
        },
        Subject::TextArchive(index) => NavTargetDto::TextArchive {
            index,
            message: number("message").and_then(|m| u16::try_from(m).ok()),
        },
        Subject::Trainer(index) => NavTargetDto::Trainer { index },
        Subject::Encounter(index) => NavTargetDto::Encounter { index },
        Subject::Matrix(matrix) => {
            let (x, y) = parts
                .get(1)
                .filter(|_| parts[0] == "cell")?
                .split_once(',')?;
            NavTargetDto::MatrixCell {
                matrix,
                x: x.parse().ok()?,
                y: y.parse().ok()?,
                header: None,
            }
        }
        Subject::Map(map) => {
            let cell = map_cells.get(&map)?;
            NavTargetDto::MatrixCell {
                matrix: cell.matrix,
                x: cell.x,
                y: cell.y,
                header: cell.header,
            }
        }
        Subject::AreaData(id) => NavTargetDto::Area { id },
        Subject::Project => return None,
    })
}

/// One reference: `field` (slot `slot`) of `source` points at the target. `label` says where
/// in words ("Encounters 0012, Grass slot 3"; "Begegnungen 0012, Gras Platz 3" in German),
/// `source_name` names the source on its own ("Encounters 0012", "Staravia", "Wanderer
/// Kai"), `nav` where a click leads, `access` what
/// the source does with the target (`read`, `write`, `read_write`, `unknown`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefDto {
    pub source: AssetId,
    pub field: &'static str,
    pub slot: Option<u16>,
    pub label: String,
    pub source_name: String,
    pub nav: NavTargetDto,
    pub access: g4_kernel::data::refs::Access,
}

/// What `set_trainer_texts` did: the assets it changed (empty: nothing) and the Problems rows
/// of source `trainer` for the trainer (they replace its previous rows).
#[derive(Debug, Clone, Serialize)]
pub struct TrainerTextsDto {
    pub changed: Vec<AssetId>,
    pub problems: Vec<ProblemDto>,
}

/// What `split_encounter_file` did: the new encounter file (written at once, not undoable)
/// and the assets the undoable step changed (the header table; empty when nothing changed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SplitDto {
    pub index: u16,
    pub changed: Vec<AssetId>,
}
