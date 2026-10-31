import type { FlagEntryDto, FlagKind, FlagListDto } from "../api";

/** One entry of the list as the backend sends it: unused, unnamed, in the general area, unless `over` says otherwise. */
export function flagEntry(number: number, over: Partial<FlagEntryDto> = {}): FlagEntryDto {
  return {
    number,
    name: null,
    aliases: [],
    own_id: null,
    description: null,
    area: "general",
    usage: { read: 0, write: 0, unknown: 0, sources: 0 },
    free: false,
    reserved: null,
    places: 0,
    ...over,
  };
}

/** A list of `kind` with `entries`; the scripts are indexed and the pool has room unless `over` says otherwise. */
export function flagList(kind: FlagKind, entries: FlagEntryDto[], over: Partial<FlagListDto> = {}): FlagListDto {
  return {
    kind,
    entries,
    pool_free: 3,
    pool_total: 10,
    counts: { script_local: 0, dynamic: 0 },
    scripts_indexed: true,
    text_project: false,
    family: "Pt",
    ms: 4,
    ...over,
  };
}

/** Platinum-like flags: an own name over a pret name, a pret name with an alias, an unnamed number, a map-local one. */
export const FLAGS: FlagListDto = flagList("flag", [
  flagEntry(0x20, { name: "FLAG_MAP_LOCAL_0x20", area: "map_local", usage: { read: 40, write: 14, unknown: 0, sources: 54 }, places: 54 }),
  flagEntry(0x94, { name: "FLAG_UNUSED_0x0094", own_id: "FLAG_HAT_MEDAILLE", description: "Die Medaille ist abgeholt.", usage: { read: 1, write: 3, unknown: 0, sources: 2 } }),
  flagEntry(0x96, { name: "FLAG_UNUSED_0x0096", aliases: ["FLAG_ALT_0x96"], usage: { read: 0, write: 0, unknown: 2, sources: 1 } }),
  flagEntry(0x173, { free: true }),
  flagEntry(0x550, { name: "FLAG_TRAINER_0", area: "trainer", usage: { read: 1, write: 1, unknown: 0, sources: 1 } }),
]);
