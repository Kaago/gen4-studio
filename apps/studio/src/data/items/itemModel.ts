import { fieldLabels } from "../../api";
import type { FieldValueDto, RefDto } from "../../api";
import type { SelectOption } from "../../ui/primitives";
import { pokemonTypes, typeName } from "../../ui/primitives/pokemonTypes";
import { POCKET_NAMES } from "../dataModel";

/** A field of an item as `DataPatch` `Item` names it. */
export type ItemField = keyof typeof fieldLabels.item;

/** The values of an item by field (`DataDocDto` `Item` lists them as `FieldValueDto`s). */
export type ItemValues = Partial<Record<ItemField, number>>;

export const itemValues = (fields: readonly FieldValueDto[]): ItemValues => Object.fromEntries(fields.map((f) => [f.name, f.value])) as ItemValues;

export const id4 = (n: number): string => String(n).padStart(4, "0");

/**
 * The fields of an item in the order of its 34 bytes (dspre-report §1.6, docs/formats/item.md): price, hold effect and parameter, pluck,
 * fling, natural gift, the word at 0x08 (natural gift type, prevent toss, can register, field pocket, battle pocket), the use functions,
 * party use, then the party parameters (the 52 named bits, the EV and friendship changes, HP and PP amounts).
 */
export const ITEM_FIELDS = Object.keys(fieldLabels.item) as ItemField[];

/** The field pockets (bits 7 to 10 of the word at 0x08) as the German bag names them. */
export const FIELD_POCKETS: readonly SelectOption<number>[] = POCKET_NAMES.map((label, value) => ({ value, label }));

/** One bit of the battle pocket (bits 11 to 15 of the word at 0x08): which pocket of the battle bag shows the item. */
export interface PocketBit {
  bit: number;
  mask: number;
  label: string;
}

/** The battle pockets bit by bit, as DSPRE `BattlePocket` orders them: Poké Balls, battle items, HP, status, PP. */
export const BATTLE_POCKETS: readonly PocketBit[] = ["Pokébälle", "Kampf-Items", "KP heilen", "Statusprobleme heilen", "AP heilen"].map((label, bit) => ({ bit, mask: 1 << bit, label }));

/** The battle pocket with `mask` set or cleared; every other bit stays. */
export const withBit = (word: number, mask: number, on: boolean): number => (on ? word | mask : word & ~mask) & 0x1f;

/** The use outside battle (byte 0x0A), DSPRE `FieldUseFunc`; values 23 to 29 are HeartGold and SoulSilver's. */
export const FIELD_FUNCTIONS: readonly SelectOption<number>[] = [
  "Allgemein",
  "Heilmittel",
  "Ungenutzt 2",
  "Ungenutzt 3",
  "Fahrrad",
  "Ungenutzt 5",
  "TM/VM",
  "Brief",
  "Beere",
  "Ungenutzt 9",
  "Freundebuch",
  "Ungenutzt 11",
  "Ungenutzt 12",
  "Ungenutzt 13",
  "Honig",
  "Ungenutzt 15",
  "Angel",
  "Profiangel",
  "Superangel",
  "Allgemein 2",
  "Entwicklungsstein",
  "Fluchtseil",
  "Ungenutzt 22",
  "Aprikokobox",
  "Beerentöpfe",
  "Icognito-Bericht",
  "Itemradar",
  "GB-Player",
  "Gracidea",
  "Kampf-Recorder",
].map((label, value) => ({ value, label }));

/** The use in battle (byte 0x0B), DSPRE `BattleUseFunc`. */
export const BATTLE_FUNCTIONS: readonly SelectOption<number>[] = ["Nicht im Kampf", "Ball", "Heilmittel", "Flucht"].map((label, value) => ({ value, label }));

/** No natural gift type (the value of the items that are no berries). */
export const NO_GIFT_TYPE = 31;
/** The natural gift types: the 18 types and 31 for none. */
export const GIFT_TYPES: readonly SelectOption<number>[] = [...pokemonTypes.map((t) => ({ value: t.id, label: t.name })), { value: NO_GIFT_TYPE, label: "Keiner" }];

/** A group of fields of the hand: its title, its fields, and whether it shows only while tinkering. */
export interface FieldGroup {
  title: string;
  fields: readonly ItemField[];
  tinkerOnly: boolean;
}

/** Werte: price, pockets and the two switches of the bag. */
export const VALUE_FIELDS: readonly ItemField[] = ["price", "field_pocket", "battle_pocket", "prevent_toss", "can_register"];
/** Effekte: holding, pluck, fling, natural gift. */
export const EFFECT_FIELDS: readonly ItemField[] = ["hold_effect", "hold_param", "pluck_effect", "fling_effect", "fling_power", "natural_gift_power", "natural_gift_type"];
/** Funktionen: the use outside and in battle, and whether the party parameters count. */
export const FUNCTION_FIELDS: readonly ItemField[] = ["field_use_func", "battle_use_func", "party_use"];
/** The party parameters (0x0E to 0x1F): only while tinkering. */
export const PARTY_FIELDS: readonly ItemField[] = ITEM_FIELDS.filter((f) => !VALUE_FIELDS.includes(f) && !EFFECT_FIELDS.includes(f) && !FUNCTION_FIELDS.includes(f));

export const FIELD_GROUPS: readonly FieldGroup[] = [
  { title: "Werte", fields: VALUE_FIELDS, tinkerOnly: false },
  { title: "Effekte", fields: EFFECT_FIELDS, tinkerOnly: false },
  { title: "Funktionen", fields: FUNCTION_FIELDS, tinkerOnly: false },
  { title: "Wirkung im Team", fields: PARTY_FIELDS, tinkerOnly: true },
];

/** The fields of one bit: a switch or a checkbox, "an" or "aus" in a label. */
export const ONE_BIT_FIELDS: readonly ItemField[] = [
  "prevent_toss",
  "can_register",
  "party_use",
  "slp_heal",
  "psn_heal",
  "brn_heal",
  "frz_heal",
  "prlz_heal",
  "cfs_heal",
  "inf_heal",
  "guard_spec",
  "revive",
  "revive_all",
  "level_up",
  "evolve",
  "pp_up",
  "pp_max",
  "pp_restore",
  "pp_restore_all",
  "hp_restore",
  "hp_ev_flag",
  "attack_ev_flag",
  "defense_ev_flag",
  "speed_ev_flag",
  "sp_attack_ev_flag",
  "sp_defense_ev_flag",
  "friendship_low_flag",
  "friendship_mid_flag",
  "friendship_high_flag",
];

const optionName = (options: readonly SelectOption<number>[], value: number) => options.find((o) => o.value === value)?.label ?? `Wert ${value}`;

/** The battle pockets set in `word` in words ("KP heilen"; "keine"). */
export function battlePocketText(word: number): string {
  const set = BATTLE_POCKETS.filter((p) => (word & p.mask) !== 0).map((p) => p.label);
  const unknown = word & ~0x1f;
  if (unknown !== 0) set.push(`Wert ${word}`);
  return set.length === 0 ? "keine" : set.join(", ");
}

/** A value of an item in words for a label ("Medizin", "Keiner", "an"); numbers stay numbers. */
export function itemText(field: ItemField, value: number): string {
  if (ONE_BIT_FIELDS.includes(field)) return value !== 0 ? "an" : "aus";
  switch (field) {
    case "field_pocket":
      return optionName(FIELD_POCKETS, value);
    case "battle_pocket":
      return battlePocketText(value);
    case "natural_gift_type":
      return value === NO_GIFT_TYPE ? "Keiner" : typeName(value);
    case "field_use_func":
      return optionName(FIELD_FUNCTIONS, value);
    case "battle_use_func":
      return optionName(BATTLE_FUNCTIONS, value);
    default:
      return String(value);
  }
}

/** ", gilt für 22 Gegenstände" when the data is shared with `shared` other item ids; empty otherwise. */
const sharedSuffix = (shared: number): string => (shared > 0 ? `, gilt für ${shared + 1} Gegenstände` : "");

/** The undo label of a field: "Trank: Preis 300 → 250", with ", gilt für 22 Gegenstände" when the data is shared. */
export const itemLabel = (name: string, field: ItemField, old: number, value: number, shared = 0): string =>
  `${name}: ${fieldLabels.item[field]} ${itemText(field, old)} → ${itemText(field, value)}${sharedSuffix(shared)}`;

/** The undo label of one battle pocket bit: "Trank: Kampftasche KP heilen an → aus". */
export const pocketBitLabel = (name: string, pocket: PocketBit, on: boolean, shared = 0): string =>
  `${name}: ${fieldLabels.item.battle_pocket} ${pocket.label} ${on ? "aus → an" : "an → aus"}${sharedSuffix(shared)}`;

/** The warning of the hand when the data is shared: "Preis und Werte gelten auch für 21 andere Gegenstände". */
export const sharedWarning = (others: number): string =>
  others === 1 ? "Preis und Werte gelten auch für 1 anderen Gegenstand" : `Preis und Werte gelten auch für ${others} andere Gegenstände`;

/** The note of the view: "Teilt Daten mit 21 Gegenständen". */
export const sharedNote = (others: number): string => (others === 1 ? "Teilt Daten mit 1 Gegenstand" : `Teilt Daten mit ${others} Gegenständen`);

/** The selling price: half the price, rounded down. */
export const sellPrice = (price: number): number => Math.floor(price / 2);

/**
 * The German sentence of a lock: the kernel says "item 0017: 36 bytes, expected 34", the editor "Gegenstand 0017: 36 Bytes statt 34". A
 * reason in another shape follows the item as the kernel says it.
 */
export function lockText(locked: string, index: number): string {
  const lengths = /: (\d+) bytes, expected (\d+)$/.exec(locked);
  return lengths ? `Gegenstand ${id4(index)}: ${lengths[1]} Bytes statt ${lengths[2]}` : `Gegenstand ${id4(index)}: ${locked}`;
}

/** The references to an item by kind, each source member once. */
export interface ItemUses {
  /** Trainers that carry it in their four item slots (Trainer-Items). */
  trainers: RefDto[];
  /** Trainer teams whose Pokémon hold it (Team-Items). */
  parties: RefDto[];
  /** Species that hold it in the wild (personal item 1 or 2). */
  wild: RefDto[];
  /** Evolutions that use it. */
  evolutions: RefDto[];
}

function firstPer(list: readonly RefDto[]): RefDto[] {
  const seen = new Set<number>();
  return list.filter((r) => {
    if (seen.has(r.source.index)) return false;
    seen.add(r.source.index);
    return true;
  });
}

export function itemUses(refs: readonly RefDto[]): ItemUses {
  return {
    trainers: firstPer(refs.filter((r) => r.source.kind === "TrainerProperties")),
    parties: firstPer(refs.filter((r) => r.source.kind === "TrainerParty")),
    wild: firstPer(refs.filter((r) => r.source.kind === "Personal")),
    evolutions: firstPer(refs.filter((r) => r.source.kind === "Evolution")),
  };
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** "2 Trainer-Items, 1 Team-Item, 3 wild getragen". */
export const usesText = (uses: ItemUses): string =>
  `${plural(uses.trainers.length, "Trainer-Item", "Trainer-Items")}, ${plural(uses.parties.length, "Team-Item", "Team-Items")}, ${uses.wild.length} wild getragen`;
