import type { HeaderSummary, PlaceSlots } from "../../api";
import type { PickerItem } from "../../ui/primitives";
import { UNNAMED_KEY } from "../places";
import { NO_PLACE_INDEX, placeNameOf } from "../worldModel";
import type { Area, World } from "../worldModel";

/** Index 0 of the bank in the picker „Ort wählen“: the game's "Mysterious Zone", no place. */
export const NO_PLACE_LABEL = "Kein Ort (Mysteriöser Ort)";
/** Why Diamond and Pearl have no new places (the kernel's refusal, said before the click). */
export const DP_PLACES = "Eigene Orte gibt es nur für Platin und HeartGold/SoulSilver.";
/** The longest name of the vanilla games: a longer one may be cut off in the game. */
const NAME_LIMIT = 16;

/** „1 Gebiet oder Raum“, „3 Gebiete und Räume“: how many areas and rooms carry a place. */
export function usersText(count: number): string {
  if (count === 0) return "unbenutzt";
  return count === 1 ? "1 Gebiet oder Raum" : `${count} Gebiete und Räume`;
}

/**
 * How a header counts as a user of a place: an area's sections (the headers of its cells) count as one user, the area; a room, a header
 * the world model does not know and every header while there is no world count one each.
 */
function userKeyOf(world: World | null): (h: HeaderSummary) => string {
  const sections = new Set<number>();
  if (world) for (const area of [...world.worldMaps.flatMap((w) => w.areas), ...world.moreAreas]) for (const h of area.headers) sections.add(h.id);
  return (h) => {
    const area = world !== null && sections.has(h.id) ? world.areaOf.get(h.id) : undefined;
    return area === undefined ? `header:${h.id}` : `area:${area}`;
  };
}

/** How many areas and rooms `headers` stand for: the sections of one area are one. */
export function countUsers(headers: readonly HeaderSummary[], world: World | null): number {
  return new Set(headers.map(userKeyOf(world))).size;
}

/** The key `placeColor` takes for the name at `index` of the bank, as the world tree keys a place (no place and a blank name: `UNNAMED_KEY`). */
export function placeKeyOf(index: number, name: string): string {
  return index === NO_PLACE_INDEX ? UNNAMED_KEY : name.trim() || UNNAMED_KEY;
}

/**
 * The entries of the picker „Ort wählen“: every name of the location bank in its order, id the index, index 0 as „Kein Ort“; each with
 * how many areas and rooms carry it (`countUsers`).
 */
export function placeItems(headers: readonly HeaderSummary[], names: readonly string[], world: World | null = null): PickerItem[] {
  const users = new Map<number, HeaderSummary[]>();
  for (const h of headers) users.set(h.location_index, [...(users.get(h.location_index) ?? []), h]);
  return names.map((name, index) => ({
    id: index,
    label: index === NO_PLACE_INDEX ? NO_PLACE_LABEL : name,
    count: usersText(countUsers(users.get(index) ?? [], world)),
  }));
}

/** One box of „Mitnehmen“ in the dialog „Neuer Ort“: the area (all its sections) or one room, and whether it is ticked at first. */
export interface PlaceChoice {
  /** The first header of the entry (the area's first section, the room's header). */
  id: number;
  label: string;
  checked: boolean;
  /** The headers the entry moves to the new place. */
  headers: number[];
}

/** The location index an area stands for: that of the section named like the area, else of its first. */
function areaPlace(area: Area): number {
  return (area.headers.find((h) => (placeNameOf(h) || UNNAMED_KEY) === area.placeKey) ?? area.headers[0]!).location_index;
}

/**
 * What the dialog „Neuer Ort“ offers to take along from `header`: its area (named, all its sections) and each of the area's rooms by
 * clear name. Opened from the area, the area and the rooms of its present place are ticked; opened from a room, only that room. A
 * header in no area is the only choice.
 */
export function presetChoice(world: World, header: HeaderSummary): PlaceChoice[] {
  const key = world.areaOf.get(header.id);
  const area = key === undefined ? undefined : [...world.worldMaps.flatMap((w) => w.areas), ...world.moreAreas].find((a) => a.key === key);
  if (area === undefined) {
    const room = world.unconnected.find((r) => r.header.id === header.id);
    return [{ id: header.id, label: room?.name ?? (placeNameOf(header) || header.internal_name), checked: true, headers: [header.id] }];
  }
  const fromArea = area.headers.some((h) => h.id === header.id);
  const place = areaPlace(area);
  return [
    { id: area.headers[0]!.id, label: area.name, checked: fromArea, headers: area.headers.map((h) => h.id) },
    ...area.rooms.map((room) => ({
      id: room.header.id,
      label: room.name,
      checked: fromArea ? room.header.location_index === place : room.header.id === header.id,
      headers: [room.header.id],
    })),
  ];
}

/** The note for a name longer than the vanilla games' longest (a warning, no refusal); null for one that fits. */
export function nameWarning(name: string): string | null {
  return [...name].length > NAME_LIMIT ? "Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden." : null;
}

/**
 * The warning before a new place takes the slot of an unused Sinnoh name (a full HeartGold/SoulSilver bank: `createPlace` takes the
 * first of `reusable`); null while there is a free slot or nothing to reuse.
 */
export function reuseWarning(slots: PlaceSlots, names: readonly string[]): string | null {
  const index = slots.reusable[0];
  if (slots.free > 0 || index === undefined) return null;
  return `Alle freien Plätze sind belegt. Der neue Ort übernimmt den Platz von „${names[index] ?? ""}“, einem Sinnoh-Namen, den kein Gebiet nutzt. Pokémon, die über den Pal-Park kommen und dort gefangen wurden, zeigen dann den neuen Namen als Fundort.`;
}
