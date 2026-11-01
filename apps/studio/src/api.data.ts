import type { AssetId, EventKind, FlagKind, ProblemDto } from "./api";
import { invoke } from "./platform/transport";
import type { HexFieldSpan } from "./ui/primitives/HexBlock";

/**
 * The data commands (Pokémon, moves, items, encounters, trainers), mirroring
 * `apps/studio/src-tauri/src/commands/data/`. `api.ts` re-exports everything.
 *
 * Items are keyed by item id when `itemTable()` is not null: rows and `index` are item ids
 * (468 in Platinum, 537 in HeartGold), `extra.data_index` is the data member, and several ids
 * can share one member (`shared_with`). Without the table they are keyed by data member and
 * have no names (the UI shows "Daten 0017").
 *
 * Names are the kernel's: where a text archive has none they are English fallbacks such as
 * "species 0520" or "move 0469", which the UI words in German ("Art 0520", "Attacke 0469").
 *
 * Diamond and Pearl are locked: every `data*` command rejects with code "read_only" and the
 * message "Diamant/Perl: ohne Testprojekt nicht freigegeben".
 */

/** `species`, `learnset` and `evolution` share one document. */
export type DataKind = "species" | "learnset" | "evolution" | "move" | "item" | "encounter" | "trainer";

/** Narrows `dataList`: `query` matches the name (any case) or the number; `types` keeps species and moves of one of the types; `pocket` items of that pocket; `category` moves of that category. */
export interface DataFilter {
  query?: string | null;
  types?: number[];
  pocket?: number | null;
  category?: number | null;
}

/** `extra` of a species row (learnset rows carry `{ count }`, evolution rows `{ targets }`). */
export interface SpeciesRowExtra {
  types: [number, number];
  /** National dex number for 1 to 493; forms keep their member index. */
  number: number;
  /** HP, Attack, Defense, Speed, Special Attack, Special Defense. */
  stats: [number, number, number, number, number, number];
}

export interface MoveRowExtra {
  type: number;
  category: number;
  power: number;
  accuracy: number;
  pp: number;
}

export interface ItemRowExtra {
  price: number;
  field_pocket: number;
  /** Whether the item table names an icon the icon archive has. */
  icon: boolean;
  data_index: number;
}

export interface EncounterRowExtra {
  /** The headers whose `wild_pokemon` is this file. */
  users: number[];
}

/** A header (place) and its name: the in-game location name, else the internal name, else "Header 0012". */
export interface PlaceDto {
  header: number;
  name: string;
}

export interface TrainerRowExtra {
  class: number;
  /** The class name from the class bank, else "Klasse 130". */
  class_name: string;
  /** Highest level in the party (0 without Pokémon). */
  level_max: number;
  party_count: number;
  /** The headers whose object events start this trainer's battle; empty: "Ohne Ort". */
  places: PlaceDto[];
}

/** One row of a data list. `tech` is the four-digit index; `locked` the reason a member cannot be edited (then `extra` is null). */
export interface DataRowDto {
  index: number;
  name: string;
  tech: string;
  locked: string | null;
  extra: SpeciesRowExtra | MoveRowExtra | ItemRowExtra | EncounterRowExtra | TrainerRowExtra | { count: number } | { targets: number[] } | null;
}

/** The personal data of one species or form; `DataPatch` `Personal` takes these names (`flip` as 0 or 1). */
export interface PersonalDto {
  hp: number;
  attack: number;
  defense: number;
  speed: number;
  sp_attack: number;
  sp_defense: number;
  type1: number;
  type2: number;
  catch_rate: number;
  base_exp: number;
  ev_yield_hp: number;
  ev_yield_attack: number;
  ev_yield_defense: number;
  ev_yield_speed: number;
  ev_yield_sp_attack: number;
  ev_yield_sp_defense: number;
  item1: number;
  item2: number;
  gender_ratio: number;
  egg_cycles: number;
  base_friendship: number;
  growth_rate: number;
  egg_group1: number;
  egg_group2: number;
  ability1: number;
  ability2: number;
  flee_rate: number;
  color: number;
  flip: boolean;
}

export type PersonalField = keyof PersonalDto;

export interface LearnEntryDto {
  move_id: number;
  level: number;
}

/** One of the seven evolution entries (method 0: empty). */
export interface EvoEntryDto {
  method: number;
  param: number;
  target: number;
}

/** A field of a move or item with its width (what a `NumberField` allows). */
export interface FieldValueDto {
  name: string;
  value: number;
  bits: number;
  signed: boolean;
}

/** A description and where it lives, for "Beschreibung bearbeiten" (`goTo({ kind: "textArchive", index: archive, message })`). */
export interface DescriptionDto {
  archive: number;
  message: number;
  text: string;
}

/** A replacement entry that can take a slot's place: entry `entry` of list `list`, `when` in the kernel's English words. `in_file` is false for Trophy Garden and Great Marsh (not stored in the file). */
export interface ReplacedByDto {
  list: string;
  entry: number;
  when: string;
  in_file: boolean;
}

/** A slot a replacement entry takes the place of. */
export interface SlotTargetDto {
  method: EncounterMethod;
  slot: number;
}

/**
 * One encounter slot, or one entry of a replacement list. `min === max` for grass; HeartGold grass adds the morning and night species
 * (`species` is the day one). `share` is the percent the game picks the slot with (a list entry: the shares of the slots it replaces in its
 * first method); `replaced_by` the replacement entries that can take a method slot's place, `replaces` the slots a list entry replaces. A list
 * entry shows the level of the first slot it replaces.
 */
export interface SlotDto {
  index: number;
  species: number;
  min: number;
  max: number;
  species_morning: number | null;
  species_night: number | null;
  share: number;
  replaced_by: ReplacedByDto[];
  replaces: SlotTargetDto[];
}

export type EncounterMethod = "grass" | "surf" | "rock_smash" | "old_rod" | "good_rod" | "super_rod";

export interface MethodDto {
  method: EncounterMethod;
  /** 0 means never. */
  rate: number;
  slots: SlotDto[];
}

/**
 * A replacement list stored in the file, by the kernel's key (Platinum swarm, day, night, radar, dual_*; HeartGold radio_hoenn, radio_sinnoh,
 * land_swarm, surf_swarm, night_fish, fish_swarm), with its German `label` and when the game uses it (English). Its entries are slots: the
 * `Encounter` patch takes `list` as `method` and the entry as `slot`.
 */
export interface ReplacementListDto {
  list: string;
  label: string;
  when: string;
  slots: SlotDto[];
}

/** Platinum values that are no slot (Shellos/Gastrodon form rates, Unown table); null in HeartGold. `EncounterExtra` takes `form_rate_0` to `form_rate_4` and `unown_table`. */
export interface EncounterExtrasDto {
  form_rates: [number, number, number, number, number] | null;
  unown_table: number | null;
}

/** A rule the file breaks (shown, never enforced): method key, 0-based slot (null: the method), the kernel's English text with 0-based numbers. */
export interface RuleWarningDto {
  method: EncounterMethod;
  slot: number | null;
  text: string;
}

export interface SpeciesShareDto {
  species: number;
  share: number;
}

/** Who waits in the grass at one time of day (kernel keys `morning`, `day`, `night`), largest share first. */
export interface TimeSharesDto {
  when: "morning" | "day" | "night";
  species: SpeciesShareDto[];
}

/** Whether the party entries carry own moves and a held item. */
export interface PartyFormatDto {
  moves: boolean;
  items: boolean;
}

/**
 * One Pokémon of a trainer party. `item` is set exactly when the format has items, `moves` when it has moves; `ball_capsule` in Platinum and
 * HeartGold. `gender` (bits 0–3 of `byte1`) and `ability` (bits 4–7) only in HeartGold: there a patch with both set writes `byte1` from them.
 */
export interface PartyMonDto {
  iv_or_difficulty: number;
  byte1: number;
  level: number;
  species: number;
  form: number;
  item: number | null;
  moves: [number, number, number, number] | null;
  ball_capsule: number | null;
  gender: number | null;
  ability: number | null;
}

/** One battle text: kind (0 before the battle, 1 lost, 2 after …), where it lives (battle text archive, message), the text in chatot form. */
export interface TrainerTextDto {
  kind: number;
  archive: number;
  message: number;
  text: string;
}

/** A battle text to write (an empty text of a kind the trainer lacks adds nothing). */
export interface TrainerTextInDto {
  kind: number;
  text: string;
}

/** Whether a species has battle images of its own in the female slots and in the male (or only) slots. */
export interface SpriteSlotsDto {
  female: boolean;
  male: boolean;
}

/** How much of a fixed budget a table fills, in words (Platinum's egg moves in overlay 5: marks, moves and the end all count). */
export interface BudgetDto {
  used: number;
  total: number;
}

/** Where the egg move table is read from: the game's NARC (HeartGold/SoulSilver), DSPRE's patch NARC or overlay 5 (Platinum). */
export type EggMovesSource = "narc" | "patch" | "overlay";

/**
 * The egg moves of one species (empty without an entry). `budget` only for the table in overlay 5; `locked` is why the table cannot be
 * read or edited (then `moves` is empty and must not be shown as the species' moves).
 */
export interface EggMovesDto {
  moves: number[];
  source: EggMovesSource;
  budget: BudgetDto | null;
  locked: string | null;
}

/**
 * The document of one member, by `kind`. A locked member (wrong length, e.g. a 36-byte item DSPRE wrote) has `locked`
 * and `raw_locked`; its decoded fields are then empty (species: zeros) and must not be shown as values.
 */
export type DataDocDto =
  | {
      kind: "species";
      personal: PersonalDto;
      /** The personal data as last saved ("vorher"). */
      saved: PersonalDto;
      learnset: LearnEntryDto[];
      evolution: EvoEntryDto[];
      /** TM01..TM92, then HM01..HM08 (all false while the personal data is locked). */
      tm: boolean[];
      locked: string | null;
      raw_locked: number[] | null;
      /** The part `locked` and `raw_locked` are about: Personal, Learnset or Evolution (the first locked one). */
      locked_asset: AssetId | null;
      /** Which genders have battle images of their own (`getBattleSprite` falls back to the other one silently); null without readable sprites. */
      sprite_slots: SpriteSlotsDto | null;
      /** The species' egg moves; null where the game has no egg move table (Diamond/Pearl). */
      egg_moves: EggMovesDto | null;
    }
  | {
      kind: "move";
      fields: FieldValueDto[];
      description: DescriptionDto | null;
      /** The text archive of the move names (message = move index); null where the game's banks do not name it. */
      name_archive: number | null;
      locked: string | null;
      raw_locked: number[] | null;
    }
  | {
      kind: "item";
      fields: FieldValueDto[];
      description: DescriptionDto | null;
      /** The text archive of the item names (message = item id); null without the item table. */
      name_archive: number | null;
      /** The other item ids that use the same data member. */
      shared_with: number[];
      locked: string | null;
      raw_locked: number[] | null;
    }
  | {
      kind: "encounter";
      methods: MethodDto[];
      replacements: ReplacementListDto[];
      extras: EncounterExtrasDto;
      warnings: RuleWarningDto[];
      /** The warnings as Problems rows (source "encounter", file "encounters/0012"): they replace the file's previous rows. */
      problems: ProblemDto[];
      /** The headers that use the file. */
      users: PlaceDto[];
      effective: TimeSharesDto[];
      locked: string | null;
      raw_locked: number[] | null;
    }
  | {
      kind: "trainer";
      class: number;
      /** Without the `{TRAINER_NAME:…}` wrapper; empty when the bank has none. */
      name: string;
      format: PartyFormatDto;
      ai: number;
      battle_type: number;
      items: [number, number, number, number];
      party: PartyMonDto[];
      texts: TrainerTextDto[];
      /** Why the battle texts cannot be read or written (table and archive differ); null when they can. */
      texts_locked: string | null;
      /** The object events that start the battle (as `refs` gives them). */
      events: RefDto[];
      /** The trainer name bank (message = trainer) and the battle text bank. */
      name_archive: number | null;
      text_archive: number | null;
      /** Problems rows (source "trainer", file "trainers/0012"): the text before the battle should end with `\r`. They replace the trainer's previous rows. */
      problems: ProblemDto[];
      /** The locked placeholder of the sight script slot (never edited). */
      placeholder: boolean;
      locked: string | null;
      raw_locked: number[] | null;
    };

/** One change, externally tagged like the backend's enum. Without a label the backend writes "<Name>: <Feld> <alt> → <neu>". */
export type DataPatch =
  | { Personal: { field: PersonalField; value: number } }
  /** TM/HM `n`: 0 is TM01, 92 is HM01. */
  | { Tm: { n: number; on: boolean } }
  | { Learnset: { entries: LearnEntryDto[] } }
  | { Evolution: { slot: number; method: number; param: number; target: number } }
  | { Move: { field: string; value: number } }
  | { Item: { field: string; value: number } }
  /** TM/HM bits and learnset of species `from`, one undo step. */
  | { CopyMoves: { from: number } }
  /**
   * `method` is an `EncounterMethod` or a replacement list; `rate` takes no slot. Fields: rate, species, level, min, max, species_morning,
   * species_night. A water `min` above `max` raises `max` along, a `max` below `min` lowers `min` (one step).
   */
  | { Encounter: { method: string; slot?: number | null; field: string; value: number } }
  /** Platinum only: `form_rate_0` to `form_rate_4`, `unown_table`. */
  | { EncounterExtra: { field: string; value: number } }
  /** A trainer property: `class`, `ai`, `battle_type`, `item0` to `item3`. */
  | { Trainer: { field: TrainerField; value: number } }
  /** At most 7 characters, else refused with "Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch". */
  | { TrainerName: { name: string } }
  /** Own moves / held items for the whole party (null: unchanged). Moves on fill each Pokémon with the moves the game gives it; off drops them. One step. */
  | { TrainerFormat: { moves: boolean | null; items: boolean | null } }
  /** The whole party (at most 6), written in the trainer's format. */
  | { TrainerParty: { party: PartyMonDto[] } }
  /** Battle texts as `setTrainerTexts` writes them, one step with the patch label. */
  | { TrainerTexts: { texts: TrainerTextInDto[] } }
  /**
   * The species' whole egg move list (at most 16 moves of the game, not 0; empty removes the entry). Platinum without DSPRE's patch: within
   * the budget, else refused with "Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen" (adding needs a move removed elsewhere).
   */
  | { EggMoves: { moves: number[] } };

/** The trainer properties `Trainer` patches take. */
export type TrainerField = keyof (typeof fieldLabels)["trainer"];

/** A field span, the same shape the `HexBlock` primitive draws. */
export type FieldSpanDto = HexFieldSpan;

/** The bytes of a member as the store holds them, its field map (empty when locked) and its archive. */
export interface RawDto {
  bytes: number[];
  fields: FieldSpanDto[];
  /** The packed archive, relative to the project ("files/poketool/waza/pl_waza_tbl.narc"). */
  path: string;
  member: number;
}

/** `species` and `moves` per data member (with forms and fallbacks), `items` per item id, the rest as the text archives hold them. */
export interface NameTablesDto {
  species: string[];
  moves: string[];
  items: string[];
  abilities: string[];
  types: string[];
  classes: string[];
  forms: string[];
  /** The text archives the tables are read from: an edit there changes the names (and the rows that carry them). */
  archives: NameArchivesDto;
}

/** The text archive of each name table; null where the game's banks do not name one. */
export interface NameArchivesDto {
  species: number | null;
  moves: number | null;
  items: number | null;
  abilities: number | null;
  types: number | null;
  classes: number | null;
  forms: number | null;
}

/** One item id of the arm9 item table: data member, icon and palette member of the icon archive, GBA item id. */
export interface ItemEntryDto {
  data: number;
  icon: number;
  palette: number;
  gba: number;
}

/** What `refs` looks for; items by item id. */
export type TargetDto =
  | { kind: "species"; index: number }
  | { kind: "move"; index: number }
  | { kind: "item"; id: number }
  | { kind: "trainer"; index: number }
  | { kind: "trainer_class"; index: number }
  | { kind: "encounter"; index: number };

/** The tabs of the Pokédex hand a reference can open. */
export type SpeciesTab = "values" | "abilities" | "learnset" | "eggMoves" | "evolution" | "machines" | "raw";

/**
 * Where a link leads: the shapes of the frontend's `NavTarget` (`workbench/navigation.ts`), all of them (a type test keeps both equal).
 * `show` on a matrix cell is the frontend's (the world map's links); the backend does not send it.
 */
export type NavTargetDto =
  | { kind: "header"; id: number }
  | { kind: "matrixCell"; matrix: number; x: number; y: number; header?: number | null; show?: "header" | "map" }
  | { kind: "area"; id: number }
  | { kind: "building"; map: number; index: number; header?: number | null }
  | { kind: "event"; eventFile: number; eventKind: EventKind; index: number; header?: number | null }
  | { kind: "textArchive"; index: number; message?: number }
  | { kind: "script"; file: number; line?: number }
  | { kind: "species"; index: number; tab?: SpeciesTab }
  | { kind: "move"; index: number }
  | { kind: "item"; id: number }
  | { kind: "trainer"; index: number }
  | { kind: "encounter"; index: number }
  | { kind: "flag"; flagKind: FlagKind; number: number };

/**
 * One reference: `field` (entry `slot`) of `source` points at the target; `label` says where in German ("Staravia, Lernset Platz 4"),
 * `source_name` names the source on its own ("Staravia", "Wanderer Kai", "Begegnungen 0012").
 */
export interface RefDto {
  source: AssetId;
  field: string;
  slot: number | null;
  label: string;
  source_name: string;
  nav: NavTargetDto;
}

/** German field names per kind, the same table the backend labels its undo steps with (a backend test keeps both equal). */
export const fieldLabels = {
  species: {
    hp: "KP",
    attack: "Angriff",
    defense: "Verteidigung",
    speed: "Initiative",
    sp_attack: "Spezial-Angriff",
    sp_defense: "Spezial-Verteidigung",
    type1: "Typ 1",
    type2: "Typ 2",
    catch_rate: "Fangrate",
    base_exp: "Basis-EP",
    ev_yield_hp: "EV-Ertrag KP",
    ev_yield_attack: "EV-Ertrag Angriff",
    ev_yield_defense: "EV-Ertrag Verteidigung",
    ev_yield_speed: "EV-Ertrag Initiative",
    ev_yield_sp_attack: "EV-Ertrag Spezial-Angriff",
    ev_yield_sp_defense: "EV-Ertrag Spezial-Verteidigung",
    item1: "Getragener Gegenstand 1",
    item2: "Getragener Gegenstand 2",
    gender_ratio: "Geschlecht",
    egg_cycles: "Schlüpfzyklen",
    base_friendship: "Basis-Freundschaft",
    growth_rate: "Wachstum",
    egg_group1: "Ei-Gruppe 1",
    egg_group2: "Ei-Gruppe 2",
    ability1: "Fähigkeit 1",
    ability2: "Fähigkeit 2",
    flee_rate: "Fluchtrate",
    color: "Farbe",
    flip: "Gespiegelt",
    egg_moves: "Ei-Attacken",
  },
  move: {
    effect: "Effekt",
    category: "Kategorie",
    power: "Stärke",
    move_type: "Typ",
    accuracy: "Genauigkeit",
    pp: "AP",
    effect_chance: "Effektchance",
    target: "Ziel",
    priority: "Priorität",
    flags: "Merkmale",
    contest: "Wettbewerbseffekt",
    contest_type: "Wettbewerbstyp",
  },
  item: {
    price: "Preis",
    hold_effect: "Trage-Effekt",
    hold_param: "Trage-Parameter",
    pluck_effect: "Pflücker-Effekt",
    fling_effect: "Schleuder-Effekt",
    fling_power: "Schleuder-Stärke",
    natural_gift_power: "Beerenkräfte-Stärke",
    natural_gift_type: "Beerenkräfte-Typ",
    prevent_toss: "Nicht wegwerfbar",
    can_register: "Registrierbar",
    field_pocket: "Tasche",
    battle_pocket: "Kampftasche",
    field_use_func: "Feldfunktion",
    battle_use_func: "Kampffunktion",
    party_use: "Wirkung im Team",
    slp_heal: "Heilt Schlaf",
    psn_heal: "Heilt Vergiftung",
    brn_heal: "Heilt Verbrennung",
    frz_heal: "Heilt Einfrieren",
    prlz_heal: "Heilt Paralyse",
    cfs_heal: "Heilt Verwirrung",
    inf_heal: "Heilt Verliebtheit",
    guard_spec: "Megaschutz",
    revive: "Belebt",
    revive_all: "Belebt alle",
    level_up: "Level-Aufstieg",
    evolve: "Entwicklung",
    attack_stages: "Angriff-Stufen",
    defense_stages: "Verteidigung-Stufen",
    sp_attack_stages: "Spezial-Angriff-Stufen",
    sp_defense_stages: "Spezial-Verteidigung-Stufen",
    speed_stages: "Initiative-Stufen",
    accuracy_stages: "Genauigkeit-Stufen",
    crit_stages: "Volltreffer-Stufen",
    pp_up: "AP-Plus",
    pp_max: "AP-Top",
    pp_restore: "Füllt AP auf",
    pp_restore_all: "Füllt alle AP auf",
    hp_restore: "Füllt KP auf",
    hp_ev_flag: "Ändert EV KP",
    attack_ev_flag: "Ändert EV Angriff",
    defense_ev_flag: "Ändert EV Verteidigung",
    speed_ev_flag: "Ändert EV Initiative",
    sp_attack_ev_flag: "Ändert EV Spezial-Angriff",
    sp_defense_ev_flag: "Ändert EV Spezial-Verteidigung",
    friendship_low_flag: "Ändert Freundschaft niedrig",
    friendship_mid_flag: "Ändert Freundschaft mittel",
    friendship_high_flag: "Ändert Freundschaft hoch",
    hp_ev: "EV-Änderung KP",
    attack_ev: "EV-Änderung Angriff",
    defense_ev: "EV-Änderung Verteidigung",
    speed_ev: "EV-Änderung Initiative",
    sp_attack_ev: "EV-Änderung Spezial-Angriff",
    sp_defense_ev: "EV-Änderung Spezial-Verteidigung",
    hp_amount: "KP-Menge",
    pp_amount: "AP-Menge",
    friendship_low: "Freundschaft niedrig",
    friendship_mid: "Freundschaft mittel",
    friendship_high: "Freundschaft hoch",
  },
  encounter: {
    grass: "Gras",
    surf: "Surfen",
    rock_smash: "Zertrümmerer",
    old_rod: "Angel",
    good_rod: "Profiangel",
    super_rod: "Superangel",
    swarm: "Schwarm",
    day: "Tag",
    night: "Nacht",
    radar: "Poké-Radar",
    dual_ruby: "Dual-Slot Rubin",
    dual_sapphire: "Dual-Slot Saphir",
    dual_emerald: "Dual-Slot Smaragd",
    dual_firered: "Dual-Slot Feuerrot",
    dual_leafgreen: "Dual-Slot Blattgrün",
    radio: "Radio",
    radio_hoenn: "Radio Hoenn",
    radio_sinnoh: "Radio Sinnoh",
    land_swarm: "Schwarm im Gras",
    surf_swarm: "Schwarm beim Surfen",
    night_fish: "Angeln nachts",
    fish_swarm: "Schwarm beim Angeln",
    rate: "Rate",
    species: "Pokémon",
    level: "Level",
    min: "Mindestlevel",
    max: "Höchstlevel",
    species_morning: "Pokémon morgens",
    species_night: "Pokémon nachts",
    form_rate_0: "Formrate 1",
    form_rate_1: "Formrate 2",
    form_rate_2: "Formrate 3",
    form_rate_3: "Formrate 4",
    form_rate_4: "Formrate 5",
    unown_table: "Icognito-Tabelle",
  },
  trainer: {
    class: "Klasse",
    ai: "KI",
    battle_type: "Kampfart",
    item0: "Gegenstand 1",
    item1: "Gegenstand 2",
    item2: "Gegenstand 3",
    item3: "Gegenstand 4",
  },
} as const satisfies Record<"species" | "move" | "item" | "encounter" | "trainer", Record<string, string>>;

/** The rows of a data list, optionally narrowed. */
export const dataList = (kind: DataKind, filter: DataFilter | null = null) => invoke<DataRowDto[]>("data_list", { kind, filter });
/** The document of one member. */
export const dataGet = (kind: DataKind, index: number) => invoke<DataDocDto>("data_get", { kind, index });
/** One patch as one undo step; resolves to the assets it changed (empty: nothing changed). */
export const dataEdit = (kind: DataKind, index: number, patch: DataPatch, label: string | null = null) =>
  invoke<AssetId[]>("data_edit", { kind, index, patch, label });
/** Raw bytes and field map of a member (Tüfteln); `species` means its personal data. */
export const dataRaw = (kind: DataKind, index: number) => invoke<RawDto>("data_raw", { kind, index });
export const nameTables = () => invoke<NameTablesDto>("name_tables");
/** The arm9 item table, or null when it is not available (unverified offset, implausible values). */
export const itemTable = () => invoke<ItemEntryDto[] | null>("item_table");
/** The moves of TM01..TM92 and HM01..HM08, or null when the table is not available. */
export const machineTable = () => invoke<number[] | null>("machine_table");
/** Every reference to `target` (without scripts until 3b.4). */
export const refs = (target: TargetDto) => invoke<RefDto[]>("refs", { target });

/** What `createTrainer` did: the new trainer, the placeholder it wrote at the sight slot before it (null: none), whether the battle script got new slots. */
export interface CreateTrainerReport {
  id: number;
  placeholder: number | null;
  script_extended: boolean;
}

/** What `setTrainerTexts` did: the assets it changed (empty: nothing) and the trainer's Problems rows (source "trainer"). */
export interface TrainerTextsDto {
  changed: AssetId[];
  problems: ProblemDto[];
}

/** What `splitEncounterFile` did: the new encounter file (not undoable) and the assets of the undoable step (the header table). */
export interface SplitDto {
  index: number;
  changed: AssetId[];
}

/**
 * Creates a trainer of class `cls` named `name` at the next ID; written at once, not undoable: its history entry (name and text offset) is
 * an undo barrier, `undo` there rejects with "Trainer <id> anlegen lässt sich nicht rückgängig machen".
 * Rejects with the kernel's words: "Erst Scripts übernehmen (Bereich Welt, Panel Script)", a name over 7 characters, the flag capacity; code
 * "partial" when the battle script was extended before a later step failed, or a failed extension could not put the old script back.
 */
export const createTrainer = (cls: number, name: string) => invoke<CreateTrainerReport>("create_trainer", { class: cls, name });
/** Sets battle texts of `trainer` as one undo step (`TrainerTexts` patch through `dataEdit` does the same with a label). */
export const setTrainerTexts = (trainer: number, texts: TrainerTextInDto[]) => invoke<TrainerTextsDto>("set_trainer_texts", { trainer, texts });
/** Appends an encounter file, a copy of `copyOf` or empty; resolves to its index. Written at once, not undoable. HeartGold refuses from 255 on. */
export const createEncounterFile = (copyOf: number | null = null) => invoke<number>("create_encounter_file", { copyOf });
/** Gives `header` an encounter file of its own: a copy (not undoable) and the header pointed at it (one undo step). */
export const splitEncounterFile = (header: number) => invoke<SplitDto>("split_encounter_file", { header });

/** Problems rows of the data editors: the source for the Problems list and the file they belong to. */
export const encounterProblemFile = (index: number) => `encounters/${String(index).padStart(4, "0")}`;
export const trainerProblemFile = (index: number) => `trainers/${String(index).padStart(4, "0")}`;

/** PNG bytes, or null for an empty body (no icon; a transport may also hand an empty body over as null). */
const icon = (bytes: ArrayBuffer | null | undefined): ArrayBuffer | null => (bytes?.byteLength ? bytes : null);
/** The bag icon of an item id as PNG, or null when it has none. */
export const getItemIcon = async (item: number) => icon(await invoke<ArrayBuffer | null>("get_item_icon", { item }));
/** The menu icon of a personal member as PNG, or null when it has none. */
export const getMonIcon = async (species: number) => icon(await invoke<ArrayBuffer | null>("get_mon_icon", { species }));

/**
 * One battle sprite: `form` is the game's form index (0 the base form; a personal member 494 to 507 as `species` names its form
 * itself), `frame` 0 the left, 1 the right half of the image. Left out: form 0, male, not shiny, frame 0.
 */
export type BattleSpriteRequest = { species: number; form?: number; side: "front" | "back"; female?: boolean; shiny?: boolean; frame?: 0 | 1 };
/** An 80 x 80 battle sprite as PNG, or null when the species has none (an empty female slot shows the male image). */
export const getBattleSprite = async ({ species, form, side, female, shiny, frame }: BattleSpriteRequest) =>
  icon(await invoke<ArrayBuffer | null>("get_battle_sprite", { species, form, side, female, shiny, frame }));
/** The 80 x 80 battle image of trainer class `cls` as PNG, or null when it has none. */
export const getTrainerSprite = async (cls: number) => icon(await invoke<ArrayBuffer | null>("get_trainer_sprite", { class: cls }));
