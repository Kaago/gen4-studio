import { fieldLabels } from "../../api";
import type { FieldValueDto, ProjectInfo, RefDto } from "../../api";
import type { SelectOption } from "../../ui/primitives";
import { typeName } from "../../ui/primitives/pokemonTypes";
import { CATEGORY_NAMES } from "../dataModel";

/** A field of a move as `DataPatch` `Move` names it. */
export type MoveField = keyof typeof fieldLabels.move;

/** The values of a move by field (`DataDocDto` `Move` lists them as `FieldValueDto`s). */
export type MoveValues = Partial<Record<MoveField, number>>;

export const moveValues = (fields: readonly FieldValueDto[]): MoveValues => Object.fromEntries(fields.map((f) => [f.name, f.value])) as MoveValues;

export const id4 = (n: number): string => String(n).padStart(4, "0");

/** The categories by their number in the move data (byte 0x02). */
export const CATEGORIES: Readonly<Record<number, string>> = Object.fromEntries(CATEGORY_NAMES.map((name, i) => [i, name]));

/** A category in words; a number the games do not know reads "Wert 3". */
export const categoryName = (category: number): string => CATEGORIES[category] ?? `Wert ${category}`;

/**
 * Who a move hits (the u16 at 0x08): 0 or one bit of 0 to 10 (dspre-report §1.5, pret `RANGE_*`: single target, depends on the move,
 * random opponent, both opponents, all but the user, user, user's side, field, opponents' side, ally, user or ally, Me First).
 */
export const TARGETS: readonly SelectOption<number>[] = [
  { value: 0, label: "Ein gewähltes Pokémon" },
  { value: 1 << 0, label: "Je nach Attacke" },
  { value: 1 << 1, label: "Zufälliger Gegner" },
  { value: 1 << 2, label: "Beide Gegner" },
  { value: 1 << 3, label: "Alle außer dem Anwender" },
  { value: 1 << 4, label: "Anwender" },
  { value: 1 << 5, label: "Eigene Seite" },
  { value: 1 << 6, label: "Ganzes Feld" },
  { value: 1 << 7, label: "Gegnerische Seite" },
  { value: 1 << 8, label: "Partner" },
  { value: 1 << 9, label: "Anwender oder Partner" },
  { value: 1 << 10, label: "Ein Gegner, wie bei Ich zuerst" },
];

/** A target in words; another value reads "Wert 3". */
export const targetName = (target: number): string => TARGETS.find((t) => t.value === target)?.label ?? `Wert ${target}`;

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
export const FLAGS: readonly MoveFlag[] = [
  { label: "Macht Kontakt", named: true },
  { label: "Schutz wirkt", named: true },
  { label: "Magiemantel wirkt", named: true },
  { label: "Übernahme wirkt", named: true },
  { label: "Spiegeltrick wirkt", named: true },
  { label: "King-Stein wirkt", named: true },
  { label: "Bit 6", named: false },
  { label: "Bit 7", named: false },
].map((f, bit) => ({ ...f, bit, mask: 1 << bit }));

export const NAMED_FLAGS: readonly MoveFlag[] = FLAGS.filter((f) => f.named);

/** The flags byte with `mask` set or cleared; every other bit (named or not) stays as it is. */
export const withFlag = (flags: number, mask: number, on: boolean): number => (on ? flags | mask : flags & ~mask) & 0xff;

export const hasFlag = (flags: number, mask: number): boolean => (flags & mask) !== 0;

/** "0x73". */
export const hex2 = (n: number): string => `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;

/** The most PP the move reaches with PP-Ups (three of them add a fifth each): 35 → 56. */
export const ppMax = (pp: number): number => Math.floor((pp * 8) / 5);

/** The priority in words: "Priorität 0", "Priorität +1", "Priorität −1" (typographic minus). */
export const priorityText = (priority: number): string => `Priorität ${priority > 0 ? `+${priority}` : priority < 0 ? `−${-priority}` : "0"}`;

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
    default:
      return String(value);
  }
}

/** The undo label of a field: "Flügelschlag: Stärke 60 → 65", "Flügelschlag: Typ Flug → Feuer". */
export const moveLabel = (name: string, field: MoveField, old: number, value: number): string =>
  `${name}: ${fieldLabels.move[field]} ${moveText(field, old)} → ${moveText(field, value)}`;

/** The undo label of one flag bit: "Flügelschlag: Macht Kontakt an → aus". */
export const flagLabel = (name: string, flag: MoveFlag, on: boolean): string => `${name}: ${flag.label} ${on ? "aus → an" : "an → aus"}`;

/**
 * The German sentence of a lock: the kernel says "move 0017: 18 bytes, expected 16", the editor "Attacke 0017: 18 Bytes statt 16". A
 * reason in another shape follows the move as the kernel says it.
 */
export function lockText(locked: string, index: number): string {
  const lengths = /: (\d+) bytes, expected (\d+)$/.exec(locked);
  return lengths ? `Attacke ${id4(index)}: ${lengths[1]} Bytes statt ${lengths[2]}` : `Attacke ${id4(index)}: ${locked}`;
}

/**
 * The text archive of the move names (`text_banks.yaml` `moves`): Platinum 647, HeartGold and SoulSilver 750, with the Japanese and
 * Korean sets of their own; null where it is not known (Diamond and Pearl are locked anyway).
 */
const NAME_BANKS: Record<string, { western: number; Japanese: number; Korean: number }> = {
  Pt: { western: 647, Japanese: 636, Korean: 637 },
  Hgss: { western: 750, Japanese: 739, Korean: 743 },
};

export function nameBank(project: Pick<ProjectInfo, "family" | "language"> | null): number | null {
  const banks = project === null ? undefined : NAME_BANKS[project.family];
  if (banks === undefined || project === null) return null;
  return project.language === "Japanese" ? banks.Japanese : project.language === "Korean" ? banks.Korean : banks.western;
}

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

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** "14 Lernsets, 3 Trainer". */
export const usesText = (uses: MoveUses): string => `${plural(uses.learnsets.length, "Lernset", "Lernsets")}, ${uses.trainers.length} Trainer`;
