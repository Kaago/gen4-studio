import i18next from "i18next";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";

/** The words of the world explorer: every panel takes label and explanation from here (spec §3). */
export type TermKey =
  | "worldMap"
  | "area"
  | "room"
  | "place"
  | "landscape"
  | "header"
  | "outside"
  | "matrix"
  | "map"
  | "warp"
  | "eventFile"
  | "areaData"
  | "patch"
  | "textArchive"
  | "message"
  | "scriptFile"
  | "script"
  | "levelScript"
  | "problem";

/**
 * The technical name of each term (the ROM's word, shown in Tüfteln and the glossary), or `null` where the game's word is the only one.
 * The words themselves (`label`, `plural`, `short`) live in the `terms` catalogs; `termLabel`, `termPlural` and `termShort` read them at
 * call time, in the language that is shown.
 */
export const TERMS: Record<TermKey, { tech: string | null }> = {
  worldMap: { tech: null },
  area: { tech: null },
  room: { tech: null },
  place: { tech: null },
  landscape: { tech: null },
  header: { tech: "MapHeader" },
  outside: { tech: null },
  matrix: { tech: "Matrix" },
  map: { tech: "MapFile" },
  warp: { tech: "Warp" },
  eventFile: { tech: "EventFile" },
  areaData: { tech: "AreaData" },
  textArchive: { tech: "TextArchive" },
  message: { tech: null },
  scriptFile: { tech: "ScriptFile" },
  script: { tech: null },
  levelScript: { tech: null },
  problem: { tech: null },
  patch: { tech: null },
};

/** The word of a term in the language that is shown, singular ("Gebiet", "Area"). Resolved at call time: i18next is not ready at module scope. */
export const termLabel = (key: TermKey): string => t(`terms:${key}.label`);

/** The plural of a term ("Gebiete", "Areas"). */
export const termPlural = (key: TermKey): string => t(`terms:${key}.plural`);

/** The explanation of a term, one or two sentences. */
export const termShort = (key: TermKey): string => t(`terms:${key}.short`);

/**
 * A label inside a sentence of the language shown: English lower-cases its first letter ("Change event file…"), an abbreviation stays
 * ("HP"); German keeps its nouns capitalized („Eventdatei ändern…“).
 */
export function inSentence(label: string): string {
  if (i18next.language === "de") return label;
  return /^\p{Lu}\p{Ll}/u.test(label) ? `${label.charAt(0).toLowerCase()}${label.slice(1)}` : label;
}

/** An id as the Gate documents and the lists write it: four digits. */
export const id4 = (n: number) => String(n).padStart(4, "0");

/** The number of a member as a card writes it, spelled out (no "Nr."): "Nummer 0017", "Number 0017". */
export const numberText = (n: number): string => t("workbench:terms.number", { id: id4(n) });

/** How an id is written next to a member; the header stays "Header" (the word the Gate documents use), the rest follows the technical name. */
const ID_NAME: Partial<Record<TermKey, string>> = { header: "Header" };

/** The technical name of a member with its id: "Matrix 0131", "Header 0415". */
export function techLabel(key: TermKey, id: number): string {
  const term = TERMS[key];
  return `${ID_NAME[key] ?? term.tech ?? termLabel(key)} ${id4(id)}`;
}

/**
 * The word for a header or a matrix at a depth: Bauen speaks of the game (an area outside, a room inside, a place when that is
 * not known; a world map), Tüfteln of the ROM ("Header", "Matrix").
 */
export function termFor(key: "header" | "matrix", depth: Depth, kind?: "area" | "room"): string {
  if (depthAtLeast(depth, "tuefteln")) return key === "header" ? "Header" : "Matrix";
  if (key === "matrix") return termLabel("worldMap");
  return termLabel(kind ?? "place");
}
