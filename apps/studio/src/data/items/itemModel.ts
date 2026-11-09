import { fieldLabel, fieldLabels } from "../../api";
import type { DataRowDto, FieldValueDto, ItemEntryDto, ItemRowExtra, ProjectInfo, RefDto } from "../../api";
import { t } from "../../i18n";
import type { SelectOption } from "../../ui/primitives";
import { pokemonTypes, typeName } from "../../ui/primitives/pokemonTypes";
import { POCKET_IDS, lockReason, pocketName, typeOption } from "../dataModel";

/** A field of an item as `DataPatch` `Item` names it. */
export type ItemField = keyof typeof fieldLabels.item;

/** The values of an item by field (`DataDocDto` `Item` lists them as `FieldValueDto`s). */
export type ItemValues = Partial<Record<ItemField, number>>;

export const itemValues = (fields: readonly FieldValueDto[]): ItemValues => Object.fromEntries(fields.map((f) => [f.name, f.value])) as ItemValues;

/** The data member of an item row: `extra.data_index` with the item table, the row itself without. */
export const dataMemberOf = (row: DataRowDto): number => (row.extra as ItemRowExtra | null)?.data_index ?? row.index;

/** Whether an item row has a bag icon to load: only with the item table (`entry`), and only where the icon archive has the icon. */
export const hasIcon = (row: DataRowDto, entry: ItemEntryDto | undefined): entry is ItemEntryDto => entry !== undefined && (row.extra as ItemRowExtra | null)?.icon === true;

/**
 * The fields of an item in the order of its 34 bytes (dspre-report §1.6, docs/formats/item.md): price, hold effect and parameter, pluck,
 * fling, natural gift, the word at 0x08 (natural gift type, prevent toss, can register, field pocket, battle pocket), the use functions,
 * party use, then the party parameters (the 52 named bits, the EV and friendship changes, HP and PP amounts).
 */
export const ITEM_FIELDS = Object.keys(fieldLabels.item) as ItemField[];

/** The field pockets (bits 7 to 10 of the word at 0x08) as the German bag names them. */
export const FIELD_POCKETS: readonly SelectOption<number>[] = POCKET_IDS.map((_, value) => ({
  value,
  // Read when asked for (in the language shown then), never when this module loads.
  get label() {
    return pocketName(value) ?? "";
  },
}));

/** One bit of the battle pocket (bits 11 to 15 of the word at 0x08): which pocket of the battle bag shows the item. */
export interface PocketBit {
  bit: number;
  mask: number;
  label: string;
}

/** The battle pockets bit by bit, as DSPRE `BattlePocket` orders them: Poké Balls, battle items, HP, status, PP. */
export const BATTLE_POCKETS: readonly PocketBit[] = (["balls", "battleItems", "healHp", "healStatus", "healPp"] as const).map((name, bit) => ({
  bit,
  mask: 1 << bit,
  get label() {
    return t(`data:itemModel.battlePockets.${name}`);
  },
}));

/** The battle pocket with `mask` set or cleared; every other bit stays. */
export const withBit = (word: number, mask: number, on: boolean): number => (on ? word | mask : word & ~mask) & 0x1f;

/** Whether the item shows in the battle bag (any battle pocket): what "Im Kampf nutzbar" means. */
export const usableInBattle = (values: ItemValues): boolean => (values.battle_pocket ?? 0) !== 0;

const options = (names: Readonly<Record<number, FunctionName>>): readonly SelectOption<number>[] =>
  Object.entries(names).map(([value, name]) => ({
    value: Number(value),
    get label() {
      return t(`data:itemModel.functions.${name}`);
    },
  }));

/** The field functions both families share (the use-function table of pokeplatinum and pokeheartgold), named after the items using them. */
type FunctionName =
  | "none" | "medicine" | "townMap" | "bicycle" | "journal" | "tmHm" | "mail" | "berry" | "poffinCase" | "palPad" | "poketchRadar" | "sprayduck" | "honey" | "vsSeeker"
  | "oldRod" | "goodRod" | "superRod" | "protectionAndCases" | "evolutionStone" | "escapeRope" | "azureFlute" | "explorerKit" | "mulch" | "vsRecorder" | "gracidea"
  | "apricornBox" | "berryPots" | "unownReport" | "itemFinder" | "gbSounds";

const SHARED_FUNCTIONS: Readonly<Record<number, FunctionName>> = {
  0: "none",
  1: "medicine",
  2: "townMap",
  4: "bicycle",
  5: "journal",
  6: "tmHm",
  7: "mail",
  8: "berry",
  9: "poffinCase",
  10: "palPad",
  11: "poketchRadar",
  12: "sprayduck",
  14: "honey",
  15: "vsSeeker",
  16: "oldRod",
  17: "goodRod",
  18: "superRod",
  19: "protectionAndCases",
  20: "evolutionStone",
  21: "escapeRope",
  22: "azureFlute",
};

/**
 * The use outside battle (byte 0x0A) per family, each value checked against the vanilla items of Platinum DE and HeartGold DE (the
 * names are the German item names that use it). Platinum has 0 to 24; HeartGold and SoulSilver drop 3 and 13 and go on to 29.
 */
export const FIELD_FUNCTIONS: Readonly<Record<"Pt" | "Hgss", readonly SelectOption<number>[]>> = {
  Pt: options({ ...SHARED_FUNCTIONS, 3: "explorerKit", 13: "mulch", 23: "vsRecorder", 24: "gracidea" }),
  Hgss: options({ ...SHARED_FUNCTIONS, 23: "apricornBox", 24: "berryPots", 25: "unownReport", 26: "itemFinder", 27: "gbSounds", 28: "gracidea", 29: "vsRecorder" }),
};

/** The field functions of the project's family (Diamond and Pearl, locked anyway, take Platinum's); a value they lack is "Funktion N". */
export function fieldFunctions(family: ProjectInfo["family"] | undefined, value: number): readonly SelectOption<number>[] {
  const known = family === "Hgss" ? FIELD_FUNCTIONS.Hgss : FIELD_FUNCTIONS.Pt;
  return known.some((o) => o.value === value) ? known : [...known, { value, label: t("data:itemModel.functionNumber", { value }) }];
}

/** The use in battle (byte 0x0B). 0 is no routine of its own: X items, flutes and berries have it and still show in the battle bag. */
export const BATTLE_FUNCTIONS: readonly SelectOption<number>[] = (["none", "ball", "medicine", "escape"] as const).map((name, value) => ({
  value,
  get label() {
    return t(`data:itemModel.battleFunctions.${name}`);
  },
}));

/** The battle functions; a value the games do not know is "Funktion N" (like the field functions). */
export function battleFunctions(value: number): readonly SelectOption<number>[] {
  return BATTLE_FUNCTIONS.some((o) => o.value === value) ? BATTLE_FUNCTIONS : [...BATTLE_FUNCTIONS, { value, label: t("data:itemModel.functionNumber", { value }) }];
}

/** No natural gift type (the value of the items that are no berries). */
export const NO_GIFT_TYPE = 31;
/** The natural gift types: the 18 types and 31 for none. */
export const GIFT_TYPES: readonly SelectOption<number>[] = [
  ...pokemonTypes.map((type) => typeOption(type.id)),
  {
    value: NO_GIFT_TYPE,
    get label() {
      return t("data:itemModel.noGiftType");
    },
  },
];

/** A group of fields of the hand: its title, its fields, and whether it shows only while tinkering (in the Rohdaten column). */
export interface FieldGroup {
  id: "values" | "effects" | "functions" | "party";
  readonly title: string;
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

/** The groups the hand shows, in order: the columns of Bauen, then what the Rohdaten column adds while tinkering. */
export const FIELD_GROUPS: readonly FieldGroup[] = [
  fieldGroup("values", VALUE_FIELDS, false),
  fieldGroup("effects", EFFECT_FIELDS, false),
  fieldGroup("functions", FUNCTION_FIELDS, false),
  fieldGroup("party", PARTY_FIELDS, true),
];

/** One group of fields; its `title` is read when asked for (in the language shown then), never when this module loads. */
function fieldGroup(id: FieldGroup["id"], fields: readonly ItemField[], tinkerOnly: boolean): FieldGroup {
  return {
    id,
    get title() {
      return t(`data:itemModel.groups.${id}`);
    },
    fields,
    tinkerOnly,
  };
}

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

const optionName = (list: readonly SelectOption<number>[], value: number) => list.find((o) => o.value === value)?.label ?? t("data:value", { value });

/** The battle pockets set in `word` in words ("KP heilen"; "keine"). */
export function battlePocketText(word: number): string {
  const set = BATTLE_POCKETS.filter((p) => (word & p.mask) !== 0).map((p) => p.label);
  const unknown = word & ~0x1f;
  if (unknown !== 0) set.push(t("data:value", { value: word }));
  return set.length === 0 ? t("data:itemModel.noBattlePocket") : set.join(", ");
}

/** Where a label is made: the project's family (for the field functions) and how many other item ids share the data. */
export interface LabelContext {
  family?: ProjectInfo["family"];
  shared?: number;
}

/** A value of an item in words for a label ("Medizin", "Keiner", "an"); numbers stay numbers. */
export function itemText(field: ItemField, value: number, family?: ProjectInfo["family"]): string {
  if (ONE_BIT_FIELDS.includes(field)) return value !== 0 ? t("data:itemModel.on") : t("data:itemModel.off");
  switch (field) {
    case "field_pocket":
      return optionName(FIELD_POCKETS, value);
    case "battle_pocket":
      return battlePocketText(value);
    case "natural_gift_type":
      return value === NO_GIFT_TYPE ? t("data:itemModel.noGiftType") : typeName(value);
    case "field_use_func":
      return optionName(fieldFunctions(family, value), value);
    case "battle_use_func":
      return optionName(battleFunctions(value), value);
    default:
      return String(value);
  }
}

/** „, gilt für 22 Gegenstände“ when the data is shared with `shared` other item ids; empty otherwise. */
const sharedSuffix = (shared = 0): string => (shared > 0 ? t("data:itemModel.sharedSuffix", { count: shared + 1 }) : "");

/** The undo label of a field: "Trank: Preis 300 → 250", with „, gilt für 22 Gegenstände“ when the data is shared. */
export const itemLabel = (name: string, field: ItemField, old: number, value: number, at: LabelContext = {}): string =>
  `${name}: ${fieldLabel("item", field)} ${itemText(field, old, at.family)} → ${itemText(field, value, at.family)}${sharedSuffix(at.shared)}`;

/** The undo label of one battle pocket bit: "Trank: Kampftasche KP heilen an → aus". */
export const pocketBitLabel = (name: string, pocket: PocketBit, on: boolean, at: LabelContext = {}): string =>
  `${t(on ? "data:itemModel.pocketOn" : "data:itemModel.pocketOff", { name, field: fieldLabel("item", "battle_pocket"), pocket: pocket.label })}${sharedSuffix(at.shared)}`;

/** The warning of the hand when the data is shared: „Preis und Werte gelten auch für 21 andere Gegenstände“. */
export const sharedWarning = (others: number): string => t("data:itemModel.sharedWarning", { count: others });

/** The note of the view: „Teilt Daten mit 21 Gegenständen“. */
export const sharedNote = (others: number): string => t("data:itemModel.sharedNote", { count: others });

/** The selling price: half the price, rounded down. */
export const sellPrice = (price: number): number => Math.floor(price / 2);

/**
 * The German sentence of an item's lock by the data member the kernel names ("item 0000: 36 bytes, expected 34" → "Daten 0000: 36 Bytes
 * statt 34"), not by the item id: several ids may share the member.
 */
export const lockText = (locked: string): string => lockReason(locked);

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

/** "2 Trainer-Items, 1 Team-Item, 3 wild getragen, 1 Entwicklung": every group of "Verwendet in". */
export const usesText = (uses: ItemUses): string =>
  t("data:itemModel.uses", {
    trainers: t("data:itemModel.usesTrainers", { count: uses.trainers.length }),
    parties: t("data:itemModel.usesParties", { count: uses.parties.length }),
    wild: t("data:itemModel.usesWild", { n: uses.wild.length }),
    evolutions: t("data:itemModel.usesEvolutions", { count: uses.evolutions.length }),
  });
