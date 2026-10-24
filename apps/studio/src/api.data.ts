import type { AssetId } from "./api";
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

/** `species`, `learnset` and `evolution` share one document; `trainer` is only listed (its editor comes with 3b.3b). */
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

export interface TrainerRowExtra {
  class: number;
  party_count: number;
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

/** One encounter slot: `min === max` for grass; HeartGold grass adds the morning and night species (`species` is the day one). */
export interface SlotDto {
  index: number;
  species: number;
  min: number;
  max: number;
  species_morning: number | null;
  species_night: number | null;
}

export type EncounterMethod = "grass" | "surf" | "rock_smash" | "old_rod" | "good_rod" | "super_rod";

export interface MethodDto {
  method: EncounterMethod;
  /** 0 means never. */
  rate: number;
  slots: SlotDto[];
}

/** A replacement list (Platinum swarm, day, night, radar, dual_*; HeartGold radio, land_swarm, surf_swarm, night_fish, fish_swarm). */
export interface ExtraListDto {
  list: string;
  species: number[];
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
    }
  | { kind: "move"; fields: FieldValueDto[]; description: DescriptionDto | null; locked: string | null; raw_locked: number[] | null }
  | {
      kind: "item";
      fields: FieldValueDto[];
      description: DescriptionDto | null;
      /** The other item ids that use the same data member. */
      shared_with: number[];
      locked: string | null;
      raw_locked: number[] | null;
    }
  | { kind: "encounter"; methods: MethodDto[]; extras: ExtraListDto[]; locked: string | null; raw_locked: number[] | null };

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
  /** `method` is an `EncounterMethod` or a replacement list; `rate` takes no slot. Fields: rate, species, level, min, max, species_morning, species_night. */
  | { Encounter: { method: string; slot?: number | null; field: string; value: number } };

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
export type SpeciesTab = "values" | "abilities" | "learnset" | "evolution" | "machines" | "raw";

/** Where a reference leads: shapes of the frontend's `NavTarget`. `encounter` and `trainer` have no editor before 3b.3b (shown disabled). */
export type NavTargetDto =
  | { kind: "header"; id: number }
  | { kind: "event"; eventFile: number; eventKind: "object"; index: number }
  | { kind: "species"; index: number; tab?: SpeciesTab }
  | { kind: "encounter"; index: number }
  | { kind: "trainer"; index: number };

/** One reference: `field` (entry `slot`) of `source` points at the target; `label` says where in German. */
export interface RefDto {
  source: AssetId;
  field: string;
  slot: number | null;
  label: string;
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
  },
} as const satisfies Record<"species" | "move" | "item" | "encounter", Record<string, string>>;

/** The rows of a data list, optionally narrowed. */
export const dataList = (kind: DataKind, filter: DataFilter | null = null) => invoke<DataRowDto[]>("data_list", { kind, filter });
/** The document of one member (trainers reject with "kommt mit 3b.3b"). */
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

/** PNG bytes, or null for an empty body (no icon). */
const icon = (bytes: ArrayBuffer): ArrayBuffer | null => (bytes.byteLength > 0 ? bytes : null);
/** The bag icon of an item id as PNG, or null when it has none. */
export const getItemIcon = async (item: number) => icon(await invoke<ArrayBuffer>("get_item_icon", { item }));
/** The menu icon of a personal member as PNG, or null when it has none. */
export const getMonIcon = async (species: number) => icon(await invoke<ArrayBuffer>("get_mon_icon", { species }));
