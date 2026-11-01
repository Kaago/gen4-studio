import { describe, expect, it } from "vitest";
import { DEFAULT_FILTER, areasOf, filterEntries, groupRefsByHeader } from "./flagFilter";
import { flagEntry, flagRef } from "./flagTesting";

const ENTRIES = [
  flagEntry(0x20, { name: "FLAG_MAP_LOCAL_0x20", area: "map_local", usage: { read: 40, write: 14, unknown: 0, sources: 54 }, places: 54 }),
  flagEntry(0x94, { name: "FLAG_UNUSED_0x0094", own_id: "FLAG_HAT_MEDAILLE", usage: { read: 1, write: 3, unknown: 0, sources: 2 } }),
  flagEntry(0x173, { free: true }),
  flagEntry(0x2da, { name: "FLAG_OBTAINED_HIDDEN_0", area: "hidden_items" }),
  flagEntry(0x4000, { name: "VAR_MAP_LOCAL_0x00", aliases: ["VARS_START"], area: "map_local" }),
];
const numbers = (filter: Partial<typeof DEFAULT_FILTER>) => filterEntries(ENTRIES, { ...DEFAULT_FILTER, ...filter }).map((e) => e.number);

const ref = flagRef;

describe("flagFilter", () => {
  it("filters_by_kind_area_unused_free_own_and_query", () => {
    expect(DEFAULT_FILTER).toEqual({ kind: "flag", area: "all", unusedOnly: false, freeOnly: false, ownOnly: false, query: "" });
    expect(areasOf("flag")).toEqual(["map_local", "general", "hidden_items", "trainer", "system", "daily", "temp"]);
    expect(areasOf("var")).toEqual(["map_local", "object_gfx", "general", "script_local"]);
    expect(numbers({})).toEqual([0x20, 0x94, 0x173, 0x2da, 0x4000]);
    // The narrowest area of an entry is the one the filter matches.
    expect(numbers({ area: "general" })).toEqual([0x94, 0x173]);
    expect(numbers({ area: "hidden_items" })).toEqual([0x2da]);
    expect(numbers({ area: "map_local" })).toEqual([0x20, 0x4000]);
    expect(numbers({ unusedOnly: true })).toEqual([0x173, 0x2da, 0x4000]);
    expect(numbers({ freeOnly: true })).toEqual([0x173]);
    expect(numbers({ ownOnly: true })).toEqual([0x94]);
    expect(numbers({ unusedOnly: true, area: "general" })).toEqual([0x173]);
    // The search: own name, pret name, an alias, the number in hex (four digits or short) and in decimal; case does not matter.
    expect(numbers({ query: "medaille" })).toEqual([0x94]);
    expect(numbers({ query: "FLAG_UNUSED" })).toEqual([0x94]);
    expect(numbers({ query: "VARS_START" })).toEqual([0x4000]);
    expect(numbers({ query: "0x0094" })).toEqual([0x94]);
    expect(numbers({ query: "0x94" })).toEqual([0x94]);
    expect(numbers({ query: "148" })).toEqual([0x94]);
    expect(numbers({ query: " 371 " })).toEqual([0x173]);
    // Digits find the decimal number exactly, not every number that contains them.
    expect(numbers({ query: "14" })).toEqual([]);
    expect(numbers({ query: "nichts davon" })).toEqual([]);
  });

  it("groups_refs_by_header", () => {
    const refs = [
      ref({ label: "Objekt 3 · Sichtbarkeits-Flag" }),
      ref({ header: 412, header_name: "Zweiblattdorf", label: "Objekt 1 · Sichtbarkeits-Flag" }),
      ref({ label: "Objekt 5 · Sichtbarkeits-Flag" }),
      ref({ header: null, header_name: null, label: "Script 2" }),
      ref({ header: 7, header_name: null, label: "Objekt 0" }),
    ];
    const groups = groupRefsByHeader(refs);
    // In the order of the first reference; the name of a place with its header number, a header without a name by its number.
    expect(groups.map((g) => [g.header, g.name, g.refs.map((r) => r.label)])).toEqual([
      [411, "Zweiblattdorf (0411)", ["Objekt 3 · Sichtbarkeits-Flag", "Objekt 5 · Sichtbarkeits-Flag"]],
      [412, "Zweiblattdorf (0412)", ["Objekt 1 · Sichtbarkeits-Flag"]],
      [null, "Ohne Header", ["Script 2"]],
      [7, "Header 0007", ["Objekt 0"]],
    ]);
  });
});
