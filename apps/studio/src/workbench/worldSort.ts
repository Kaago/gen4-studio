import type { HeaderSummary } from "../api";
import type { ListSort, TreeSort } from "../ui/frame/shellState";
import type { Area, Room, World } from "./worldModel";

export type { ListSort, TreeSort };

/** Names as a person sorts them: „Route 2“ before „Route 10“, „Äther“ between „Achtung“ and „Bach“. */
export const compareNames = (a: string, b: string) => a.localeCompare(b, "de", { numeric: true, sensitivity: "base" });

const MAX_RECENT_AREAS = 50;
const recentKey = (project: string) => `g4.world.recent.${project}`;

/** The area keys chosen last in a project, newest first; a missing, broken or unreadable value is an empty list. */
export function readRecent(project: string): string[] {
  try {
    const raw = localStorage.getItem(recentKey(project));
    const parsed: unknown = raw === null ? [] : JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((k): k is string => typeof k === "string").slice(0, MAX_RECENT_AREAS) : [];
  } catch {
    return [];
  }
}

/** Puts an area first in the project's list (an earlier entry dropped, at most 50 kept) and stores it; returns the new list. */
export function touchRecent(project: string, areaKey: string): string[] {
  const next = [areaKey, ...readRecent(project).filter((k) => k !== areaKey)].slice(0, MAX_RECENT_AREAS);
  try {
    localStorage.setItem(recentKey(project), JSON.stringify(next));
  } catch {
    /* the choice is simply not remembered */
  }
  return next;
}

const byName = (a: { name: string }, b: { name: string }) => compareNames(a.name, b.name);

function sortAreas(areas: readonly Area[], sort: TreeSort, recent: readonly string[]): Area[] {
  const rooms = (a: Area): Area => (sort === "name" ? { ...a, rooms: [...a.rooms].sort(byName) } : { ...a, rooms: [...a.rooms] });
  const copy = areas.map(rooms);
  if (sort === "name") return copy.sort(byName);
  if (sort === "game") return copy;
  // Recent first (newest first), the rest keeps its game order; keys of areas that are gone are skipped.
  const rank = (a: Area) => {
    const i = recent.indexOf(a.key);
    return i < 0 ? recent.length : i;
  };
  return copy.sort((a, b) => rank(a) - rank(b));
}

/**
 * The world with its areas sorted inside each world map and under "Weitere Gebiete", and its rooms inside each area (by name
 * in the name sort, else in game order). World maps keep the order of the matrices; the input is not changed.
 */
export function sortWorld(world: World, sort: TreeSort, recent: readonly string[]): World {
  const unconnected: Room[] = sort === "name" ? [...world.unconnected].sort(byName) : [...world.unconnected];
  return {
    ...world,
    worldMaps: world.worldMaps.map((m) => ({ ...m, areas: sortAreas(m.areas, sort, recent) })),
    moreAreas: sortAreas(world.moreAreas, sort, recent),
    unconnected,
  };
}

const shownName = (h: HeaderSummary) => h.location_name || h.internal_name;

/** The header list by number, by the name the list shows (place, else code) or by code; equal entries keep their number order. */
export function sortHeaders(headers: readonly HeaderSummary[], sort: ListSort): HeaderSummary[] {
  const compare =
    sort === "name"
      ? (a: HeaderSummary, b: HeaderSummary) => compareNames(shownName(a), shownName(b))
      : sort === "code"
        ? (a: HeaderSummary, b: HeaderSummary) => compareNames(a.internal_name, b.internal_name)
        : () => 0;
  return [...headers].sort((a, b) => compare(a, b) || a.id - b.id);
}
