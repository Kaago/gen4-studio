import type { FlagAccess, FlagArea, FlagKind, FlagListDto, FlagUsageCounts } from "../api";
import { filterItems } from "../ui/primitives";
import type { PickerItem } from "../ui/primitives";

/** Why „Freies Flag nehmen“ is locked while the scripts are not adopted (the backend's `scripts_not_adopted`). */
export const SCRIPTS_NOT_INDEXED = "Erst Scripts übernehmen: ohne Script-Quellen weiß der Index nicht, welche Flags Scripts benutzen.";

/** Why the pool of `kind` has nothing to give (the kernel's `pool_empty_text`). */
export function poolEmptyText(kind: FlagKind): string {
  return kind === "flag" ? "Kein freies Flag im allgemeinen Bereich." : "Keine freie Variable im allgemeinen Bereich.";
}

/** What a field shows for 0: no flag, no variable. */
export const NONE = "Keins";

/**
 * The first row of every flag or variable picker: 0, none. An ordinary row: a search lists it only when it matches „Keins (0)“, „kein“,
 * „none“ or the number 0 (`filterItems` finds a keyword that contains the query, so „0x0094“ or „148“ never find it).
 */
export const NONE_ITEM: PickerItem = { id: 0, label: `${NONE} (0)`, area: "geschichten", keywords: ["kein", "none"] };

/** The number as `0x` plus four capital hex digits: „0x0094“. */
export function hex4(n: number): string {
  return `0x${n.toString(16).toUpperCase().padStart(4, "0")}`;
}

/** The number in hex and decimal: „0x0094 · 148“. */
export function numberText(n: number): string {
  return `${hex4(n)} · ${n}`;
}

/** „Flag“ or „Variable“. */
export function kindWord(kind: FlagKind): string {
  return kind === "flag" ? "Flag" : "Variable";
}

/** The name of a number: its own name, else its pret name, else its kind and number („Flag 0x0094“). */
export function displayName(e: { name: string | null; own_id: string | null }, kind: FlagKind, n: number): string {
  return e.own_id ?? e.name ?? `${kindWord(kind)} ${hex4(n)}`;
}

const AREA_WORDS: Record<FlagArea, string> = {
  map_local: "kartenlokal",
  general: "allgemein",
  hidden_items: "versteckte Items",
  trainer: "Trainer",
  system: "System",
  daily: "Tagesflags",
  temp: "temporär",
  object_gfx: "Objektgrafik",
  script_local: "Script-lokal",
  outside: "außerhalb",
};

/** The German word of an area. */
export function areaWord(area: FlagArea): string {
  return AREA_WORDS[area];
}

/** „gesetzt 3 · geprüft 1“, with „ · unklar 2“ only when something is unclear; „unbenutzt“ when nothing uses the number. */
export function usageText(u: FlagUsageCounts): string {
  if (u.read === 0 && u.write === 0 && u.unknown === 0) return "unbenutzt";
  const known = `gesetzt ${u.write} · geprüft ${u.read}`;
  return u.unknown > 0 ? `${known} · unklar ${u.unknown}` : known;
}

const ACCESS_WORDS: Record<FlagAccess, string> = { read: "prüft", write: "setzt", read_write: "setzt und prüft", unknown: "unklar" };

/** What a reference does with its number, as a verb. */
export function accessWord(a: FlagAccess): string {
  return ACCESS_WORDS[a];
}

/**
 * The rows of a flag or variable picker: „Keins (0)“ first (a search lists it only when the query finds it) (0 is no flag and stays a valid value), then the numbers of `list` that match
 * `query` (own name, pret name, an alias, the number in hex or decimal; empty: all). A row names the own name, else the pret name, else
 * kind and number (an own name `strong`); its second line the pret name (under an own name) or the area; the number in hex as `tech`; the
 * usage as `count`.
 */
export function pickerItems(list: FlagListDto, query: string): PickerItem[] {
  return filterItems([NONE_ITEM, ...flagRows(list)], query);
}

/** The rows of the numbers of `list` as `pickerItems` lists them after „Keins (0)“ (a number 0 of the list is that row, not one of these). */
export function flagRows(list: FlagListDto): PickerItem[] {
  return list.entries
    .filter((e) => e.number !== 0)
    .map(
      (e): PickerItem => ({
        id: e.number,
        label: displayName(e, list.kind, e.number),
        description: e.own_id !== null && e.name !== null ? e.name : areaWord(e.area),
        tech: hex4(e.number),
        area: "geschichten",
        strong: e.own_id !== null,
        count: usageText(e.usage),
        keywords: [...e.aliases, ...(e.own_id !== null && e.name !== null ? [e.name] : []), `0x${e.number.toString(16)}`, String(e.number)],
      }),
    );
}

/** The kernel's `NOT_A_TEXT_PROJECT`: why an own name needs a text project. */
export const NOT_A_TEXT_PROJECT = "Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.";

/** Why naming is locked in the DSPRE layout, with the way there. */
export const NAMING_NEEDS_TEXT_PROJECT = `${NOT_A_TEXT_PROJECT} (Technik › Textprojekt)`;

/** Why a number past the save cannot be named. */
export const OUTSIDE_THE_SAVE = "Liegt außerhalb des Spielstands.";
