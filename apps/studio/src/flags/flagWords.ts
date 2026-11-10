import type { FlagAccess, FlagArea, FlagKind, FlagListDto, FlagUsageCounts } from "../api";
import { t } from "../i18n";
import { filterItems } from "../ui/primitives";
import type { PickerItem } from "../ui/primitives";

/** Why „Freies Flag nehmen“ is locked while the scripts are not adopted (the backend's `scripts_not_adopted`), or its variable form. */
export function scriptsNotIndexedText(kind: FlagKind): string {
  return kind === "flag" ? t("flags:words.scriptsNotIndexed.flag") : t("flags:words.scriptsNotIndexed.var");
}

/** Why the pool of `kind` has nothing to give (the kernel's `pool_empty_text`). */
export function poolEmptyText(kind: FlagKind): string {
  return kind === "flag" ? t("flags:words.poolEmpty.flag") : t("flags:words.poolEmpty.var");
}

/** What a field shows for 0: no flag, no variable. */
export function noneText(): string {
  return t("flags:words.none");
}

/**
 * The first row of every flag or variable picker: 0, none. An ordinary row: a search lists it only when it matches „Keins (0)“, „kein“,
 * „none“ or the number 0 (`filterItems` finds a keyword that contains the query, so „0x0094“ or „148“ never find it). The keywords name
 * both languages: whoever types „kein“ or „none“ finds it whatever the app speaks.
 */
export function noneItem(): PickerItem {
  return { id: 0, label: t("flags:words.noneItem"), area: "geschichten", keywords: ["kein", "none"] };
}

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
  return kind === "flag" ? t("flags:words.kind.flag") : t("flags:words.kind.var");
}

/** The name of a number: its own name, else its pret name, else its kind and number („Flag 0x0094“). */
export function displayName(e: { name: string | null; own_id: string | null }, kind: FlagKind, n: number): string {
  return e.own_id ?? e.name ?? `${kindWord(kind)} ${hex4(n)}`;
}

/** The catalog key (`flags:words.area.<key>`) of each area's word. */
const AREA_KEYS = {
  map_local: "mapLocal",
  general: "general",
  hidden_items: "hiddenItems",
  trainer: "trainer",
  system: "system",
  daily: "daily",
  temp: "temp",
  object_gfx: "objectGfx",
  script_local: "scriptLocal",
  outside: "outside",
} as const satisfies Record<FlagArea, string>;

/** The word of an area in the app's language. */
export function areaWord(area: FlagArea): string {
  return t(`flags:words.area.${AREA_KEYS[area]}`);
}

/** „gesetzt 3 · geprüft 1“, with „ · unklar 2“ only when something is unclear; „unbenutzt“ when nothing uses the number. */
export function usageText(u: FlagUsageCounts): string {
  if (u.read === 0 && u.write === 0 && u.unknown === 0) return t("flags:words.usage.unused");
  return u.unknown > 0
    ? t("flags:words.usage.longUnclear", { write: u.write, read: u.read, unknown: u.unknown })
    : t("flags:words.usage.long", { write: u.write, read: u.read });
}

/** `usageText` in the short form of the picker row: „3 · 1“ (set · checked), „ · 2“ more when something is unclear; „unbenutzt“ when nothing uses the number. */
export function usageShort(u: FlagUsageCounts): string {
  if (u.read === 0 && u.write === 0 && u.unknown === 0) return t("flags:words.usage.unused");
  const known = `${u.write} · ${u.read}`;
  return u.unknown > 0 ? `${known} · ${u.unknown}` : known;
}

/** The catalog key (`flags:words.access.<key>`) of each access's verb. */
const ACCESS_KEYS = { read: "read", write: "write", read_write: "readWrite", unknown: "unknown" } as const satisfies Record<FlagAccess, string>;

/** What a reference does with its number, as a verb. */
export function accessWord(a: FlagAccess): string {
  return t(`flags:words.access.${ACCESS_KEYS[a]}`);
}

/**
 * The rows of a flag or variable picker: „Keins (0)“ first (a search lists it only when the query finds it) (0 is no flag and stays a valid value), then the numbers of `list` that match
 * `query` (own name, pret name, an alias, the number in hex or decimal; empty: all). A row names the own name, else the pret name, else
 * kind and number (an own name `strong`); its second line the pret name (under an own name) or the area; the number in hex as `tech`; the
 * usage as `count` (short, its long form the `countTitle`).
 */
export function pickerItems(list: FlagListDto, query: string): PickerItem[] {
  return filterItems([noneItem(), ...flagRows(list)], query);
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
        count: usageShort(e.usage),
        countTitle: usageText(e.usage),
        keywords: [...e.aliases, ...(e.own_id !== null && e.name !== null ? [e.name] : []), `0x${e.number.toString(16)}`, String(e.number)],
      }),
    );
}

/** The kernel's `NOT_A_TEXT_PROJECT`: why an own name needs a text project (the kernel says it in German; `flagWords.test.ts` holds the German text equal). */
export function notATextProject(): string {
  return t("flags:words.notATextProject");
}

/** Why naming is locked in the DSPRE layout, with the way there. */
export function namingNeedsTextProject(): string {
  return t("flags:words.namingNeedsTextProject", { reason: notATextProject() });
}

/** Why a number past the save cannot be named. */
export function outsideTheSave(): string {
  return t("flags:words.outsideTheSave");
}

/** What the cell „Live“ shows for a value of the snapshot: a flag „1“ or „0“, a variable its value in decimal; „–“ when the snapshot does not hold it. */
export function liveCellText(kind: FlagKind, value: number | null): string {
  if (value === null) return "–";
  return kind === "flag" ? (value !== 0 ? "1" : "0") : String(value);
}

/** The tooltip of a cell „Live“ with a value: a flag says what a click does, a variable where it is changed. */
export function liveCellTip(kind: FlagKind, value: number): string {
  if (kind === "var") return t("flags:words.liveTip.var", { value });
  return value !== 0 ? t("flags:words.liveTip.flagSet") : t("flags:words.liveTip.flagClear");
}
