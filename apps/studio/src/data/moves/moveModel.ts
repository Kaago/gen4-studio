import { fieldLabels } from "../../api";
import type { FieldValueDto, RefDto } from "../../api";
import { t } from "../../i18n";
import type { SelectOption } from "../../ui/primitives";
import { typeName } from "../../ui/primitives/pokemonTypes";
import { categoryNameOf, lockSentence } from "../dataModel";

/** A field of a move as `DataPatch` `Move` names it. */
export type MoveField = keyof typeof fieldLabels.move;

/** The values of a move by field (`DataDocDto` `Move` lists them as `FieldValueDto`s). */
export type MoveValues = Partial<Record<MoveField, number>>;

export const moveValues = (fields: readonly FieldValueDto[]): MoveValues => Object.fromEntries(fields.map((f) => [f.name, f.value])) as MoveValues;

/** The categories by their number in the move data (byte 0x02). */
export const CATEGORIES: Readonly<Record<number, string>> = {
  // Read when asked for (in the language shown then), never when this module loads.
  get 0() {
    return categoryNameOf(0)!;
  },
  get 1() {
    return categoryNameOf(1)!;
  },
  get 2() {
    return categoryNameOf(2)!;
  },
};

/** A category in words; a number the games do not know reads "Wert 3". */
export const categoryName = (category: number): string => CATEGORIES[category] ?? t("data:value", { value: category });

/**
 * Who a move hits (the u16 at 0x08): 0 or one bit of 0 to 10 (dspre-report §1.5, pret `RANGE_*`: single target, depends on the move,
 * random opponent, both opponents, all but the user, user, user's side, field, opponents' side, ally, user or ally, Me First).
 */
export const TARGETS: readonly SelectOption<number>[] = [
  target(0, "chosen"),
  target(1 << 0, "byMove"),
  target(1 << 1, "randomOpponent"),
  target(1 << 2, "bothOpponents"),
  target(1 << 3, "allButUser"),
  target(1 << 4, "user"),
  target(1 << 5, "ownSide"),
  target(1 << 6, "wholeField"),
  target(1 << 7, "opponentSide"),
  target(1 << 8, "ally"),
  target(1 << 9, "userOrAlly"),
  target(1 << 10, "meFirst"),
];

type TargetName = "chosen" | "byMove" | "randomOpponent" | "bothOpponents" | "allButUser" | "user" | "ownSide" | "wholeField" | "opponentSide" | "ally" | "userOrAlly" | "meFirst";

/** One target; its `label` is read when asked for (in the language shown then), never when this module loads. */
function target(value: number, name: TargetName): SelectOption<number> {
  return {
    value,
    get label() {
      return t(`data:moveModel.targets.${name}`);
    },
  };
}

/** A target in words; another value reads "Wert 3". */
export const targetName = (target: number): string => TARGETS.find((o) => o.value === target)?.label ?? t("data:value", { value: target });

/** One bit of the flags byte (0x0B). `named`: shown in Bauen; the others only while tinkering. */
export interface MoveFlag {
  bit: number;
  mask: number;
  label: string;
  named: boolean;
}

/**
 * The flags byte, bit by bit (dspre-report §1.5: contact, protect, magic coat, snatch, mirror move, king's rock, then two bits of the
 * battle display that the editors do not name). German move and item names as the games spell them.
 */
export const FLAGS: readonly MoveFlag[] = [flag(0, "contact"), flag(1, "protect"), flag(2, "magicCoat"), flag(3, "snatch"), flag(4, "mirrorMove"), flag(5, "kingsRock"), flag(6, null), flag(7, null)];

/** One flag; its `label` is read when asked for (in the language shown then), never when this module loads. */
function flag(bit: number, name: "contact" | "protect" | "magicCoat" | "snatch" | "mirrorMove" | "kingsRock" | null): MoveFlag {
  return {
    bit,
    mask: 1 << bit,
    get label() {
      return name === null ? t("data:moveModel.flags.bit", { bit }) : t(`data:moveModel.flags.${name}`);
    },
    named: name !== null,
  };
}

export const NAMED_FLAGS: readonly MoveFlag[] = FLAGS.filter((f) => f.named);

/** The flags byte with `mask` set or cleared; every other bit (named or not) stays as it is. */
export const withFlag = (flags: number, mask: number, on: boolean): number => (on ? flags | mask : flags & ~mask) & 0xff;

export const hasFlag = (flags: number, mask: number): boolean => (flags & mask) !== 0;

/** "0x73". */
export const hex2 = (n: number): string => `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;

/** The most PP the move reaches with PP-Ups (three of them add a fifth each): 35 → 56. */
export const ppMax = (pp: number): number => Math.floor((pp * 8) / 5);

/** A signed number with its sign: "+1", "0", "−1" (typographic minus). */
const signed = (n: number): string => (n > 0 ? `+${n}` : n < 0 ? `\u2212${-n}` : "0");

/** The priority in words: „Priorität 0“, „Priorität +1“, „Priorität −1“. */
export const priorityText = (priority: number): string => t("data:moveModel.priority", { value: signed(priority) });

/** A value of a move in words for an undo label ("Flug", "Physisch", "Beide Gegner"); numbers stay numbers. */
export function moveText(field: MoveField, value: number): string {
  switch (field) {
    case "move_type":
      return typeName(value);
    case "category":
      return categoryName(value);
    case "target":
      return targetName(value);
    case "flags":
      return hex2(value);
    case "priority":
      return signed(value);
    default:
      return String(value);
  }
}

/** The undo label of a field: „Flügelschlag: Stärke 60 → 65“, „Flügelschlag: Typ Flug → Feuer“. */
export const moveLabel = (name: string, field: MoveField, old: number, value: number): string =>
  `${name}: ${fieldLabels.move[field]} ${moveText(field, old)} → ${moveText(field, value)}`;

/** The undo label of one flag bit: „Flügelschlag: Macht Kontakt an → aus“. */
export const flagLabel = (name: string, flag: MoveFlag, on: boolean): string => t(on ? "data:moveModel.flagOn" : "data:moveModel.flagOff", { name, flag: flag.label });

/** The German sentence of a move's lock: "Attacke 0017: 18 Bytes statt 16" (`lockSentence`). */
export const lockText = (locked: string, index: number): string => lockSentence(t("data:dataModel.lockParts.move"), index, locked);

/** The references to a move by kind, each source member once: the learnsets that teach it, the trainers whose teams use it, the evolutions it triggers. */
export interface MoveUses {
  learnsets: RefDto[];
  trainers: RefDto[];
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

export function moveUses(refs: readonly RefDto[]): MoveUses {
  return {
    learnsets: firstPer(refs.filter((r) => r.source.kind === "Learnset")),
    trainers: firstPer(refs.filter((r) => r.source.kind === "TrainerParty")),
    evolutions: firstPer(refs.filter((r) => r.source.kind === "Evolution")),
  };
}

/** "14 Lernsets, 3 Trainer". */
export const usesText = (uses: MoveUses): string => t("data:moveModel.uses", { learnsets: t("data:moveModel.learnsets", { count: uses.learnsets.length }), trainers: t("data:moveModel.trainers", { count: uses.trainers.length }) });
