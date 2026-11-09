import type { FlagArea, FlagEntryDto, FlagKind, FlagRefDto } from "../api";
import { t } from "../i18n";
import { hex4 } from "./flagWords";

/**
 * What the list of flags and variables shows: the kind, one area (or all), only unused numbers, only free ones, only those with an own
 * name, and a search over own name, pret name, aliases and the number in hex and decimal.
 */
export interface FlagFilter {
  kind: FlagKind;
  area: FlagArea | "all";
  unusedOnly: boolean;
  freeOnly: boolean;
  ownOnly: boolean;
  query: string;
}

/** Flags, every area, no box ticked, no search. */
export const DEFAULT_FILTER: FlagFilter = { kind: "flag", area: "all", unusedOnly: false, freeOnly: false, ownOnly: false, query: "" };

const FLAG_AREAS: readonly FlagArea[] = ["map_local", "general", "hidden_items", "trainer", "system", "daily", "temp"];
const VAR_AREAS: readonly FlagArea[] = ["map_local", "object_gfx", "general", "script_local"];

/** The areas of a kind in the order the filter offers them (`temp` only HeartGold and SoulSilver have: the surface shows it when listed). */
export function areasOf(kind: FlagKind): FlagArea[] {
  return [...(kind === "flag" ? FLAG_AREAS : VAR_AREAS)];
}

/** Whether `entry` matches the search `query` (trimmed, case ignored); digits alone match the decimal number exactly. */
function matches(entry: FlagEntryDto, query: string): boolean {
  if (query === "") return true;
  if (/^\d+$/.test(query)) return String(entry.number) === query;
  const words = [entry.own_id, entry.name, ...entry.aliases, hex4(entry.number), `0x${entry.number.toString(16)}`];
  return words.some((w) => w !== null && w.toLowerCase().includes(query));
}

const unused = (e: FlagEntryDto) => e.usage.read === 0 && e.usage.write === 0 && e.usage.unknown === 0;

/** The entries the filter lets through, in their order (`kind` chooses the list; the entries are of one kind already). */
export function filterEntries(entries: readonly FlagEntryDto[], filter: FlagFilter): FlagEntryDto[] {
  const query = filter.query.trim().toLowerCase();
  return entries.filter(
    (e) =>
      (filter.area === "all" || e.area === filter.area) &&
      (!filter.unusedOnly || unused(e)) &&
      (!filter.freeOnly || e.free) &&
      (!filter.ownOnly || e.own_id !== null) &&
      matches(e, query),
  );
}

/** A header's number as four digits. */
const id4 = (n: number) => String(n).padStart(4, "0");

/**
 * The references grouped by the header that uses their source, in the order of their first reference: „Zweiblattdorf (0411)“, a header
 * without a place's name „Header 0007“, references of no header „Ohne Header“.
 */
export function groupRefsByHeader(refs: readonly FlagRefDto[]): { header: number | null; name: string; refs: FlagRefDto[] }[] {
  const groups = new Map<number | null, { header: number | null; name: string; refs: FlagRefDto[] }>();
  for (const ref of refs) {
    let group = groups.get(ref.header);
    if (group === undefined) {
      const name =
        ref.header === null
          ? t("flags:flagFilter.headerNone")
          : ref.header_name === null
            ? t("flags:flagFilter.headerUnnamed", { id: id4(ref.header) })
            : `${ref.header_name} (${id4(ref.header)})`;
      group = { header: ref.header, name, refs: [] };
      groups.set(ref.header, group);
    }
    group.refs.push(ref);
  }
  return [...groups.values()];
}
