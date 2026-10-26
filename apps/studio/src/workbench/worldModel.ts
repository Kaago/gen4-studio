import { type HeaderSummary, MATRIX_EMPTY, type MatrixDto, type MatrixSummary, type WarpLink, type WorldMeta } from "../api";
import { UNNAMED_KEY, UNNAMED_PLACE } from "./places";
import { suggestRoomNames } from "./roomNames";

export { roomName } from "./roomNames";

/** Location index 0 is no place ("Mysteriöser Ort"): it makes no area and names no room. */
export const NO_PLACE_INDEX = 0;
/** Header 0 (`EVERYWHERE`) fills the landscape cells of a world map; it belongs to no area. */
const LANDSCAPE = 0;

export interface WorldInput {
  family: string;
  headers: readonly HeaderSummary[];
  matrices: readonly MatrixSummary[];
  links: readonly WarpLink[];
  /** The cells of the matrices with a header section, by matrix index. */
  cells: ReadonlyMap<number, MatrixDto>;
  meta: WorldMeta;
}

export interface Room {
  header: HeaderSummary;
  /** The name of the project file, else `suggested`. */
  name: string;
  suggested: string;
  /** Location name as in `places.ts` (`UNNAMED_KEY` for no place), so `placeColor` stays the same. */
  placeKey: string;
  placeName: string;
  /** Assigned by the project file (`roomOwner`), not by its entrance. */
  manual: boolean;
  /** Other headers on the same matrix. */
  shared: number;
}

export interface Area {
  /** `w<matrix>:<index>` (Kanto `w<matrix>k:<index>`) on a world map, `m<matrix>` for an area with its own map. */
  key: string;
  name: string;
  placeKey: string;
  /** The headers of the area's cells (its sections), by id. */
  headers: HeaderSummary[];
  /** The rooms whose entrance lies in the area, by id. */
  rooms: Room[];
  matrix: number;
}

export interface WorldMap {
  /** `w<matrix>`, `w<matrix>k` for Kanto. */
  key: string;
  matrix: number;
  filter: "johto" | "kanto" | null;
  name: string;
  areas: Area[];
}

export interface World {
  worldMaps: WorldMap[];
  /** Areas with their own map ("Weitere Gebiete"), by first header id. */
  moreAreas: Area[];
  /** Rooms with neither an entrance nor an area of their place ("Nicht verbunden"), by id. */
  unconnected: Room[];
  /** Header 0, headers without a known matrix or without maps, nameless headers of an area matrix without a cell. */
  other: HeaderSummary[];
  /** Header id to area key, for the headers of the areas and the rooms assigned to one. */
  areaOf: ReadonlyMap<number, string>;
  roomCount: number;
  assignedRooms: number;
}

interface Draft {
  area: Area;
  /** The location index the area stands for. */
  place: number;
  /** The half of a split HeartGold/SoulSilver world map the area lies in; null elsewhere. */
  filter: Filter;
}

type Filter = WorldMap["filter"];

/** The place name of a header; empty for no place (location index 0). */
export const placeNameOf = (h: HeaderSummary): string => (h.location_index === NO_PLACE_INDEX ? "" : h.location_name.trim());
const placeKeyOf = (h: HeaderSummary): string => placeNameOf(h) || UNNAMED_KEY;

/** Filled cells per header of one matrix with a header section (landscape and unknown headers left out). */
function cellCounts(dto: MatrixDto, byId: ReadonlyMap<number, HeaderSummary>): Map<number, number> {
  const counts = new Map<number, number>();
  const ids = dto.headers ?? [];
  for (const [i, id] of ids.entries()) {
    const map = dto.maps[i];
    if (map === null || map === undefined || map === MATRIX_EMPTY) continue;
    if (id === LANDSCAPE || !byId.has(id)) continue;
    counts.set(id, (counts.get(id) ?? 0) + 1);
  }
  return counts;
}

/** The place with the most cells among `counts` (ties: the lower header id). */
function mostCells(counts: ReadonlyMap<number, number>, byId: ReadonlyMap<number, HeaderSummary>): HeaderSummary | undefined {
  const perPlace = new Map<number, { header: HeaderSummary; cells: number }>();
  for (const [id, n] of [...counts].sort((a, b) => a[0] - b[0])) {
    const header = byId.get(id);
    if (!header || header.location_index === NO_PLACE_INDEX) continue;
    const entry = perPlace.get(header.location_index);
    if (entry) entry.cells += n;
    else perPlace.set(header.location_index, { header, cells: n });
  }
  let best: { header: HeaderSummary; cells: number } | undefined;
  for (const entry of perPlace.values()) if (!best || entry.cells > best.cells) best = entry;
  return best?.header;
}

function defaultMapName(family: string, matrix: number, filter: Filter, counts: ReadonlyMap<number, number>, byId: ReadonlyMap<number, HeaderSummary>): string {
  if (matrix === 0) {
    if (family === "Hgss") return filter === "kanto" ? "Kanto" : "Johto";
    return "Sinnoh";
  }
  const header = mostCells(counts, byId);
  return header ? placeNameOf(header) || UNNAMED_PLACE : `Weltkarte ${matrix}`;
}

const newArea = (key: string, matrix: number, headers: HeaderSummary[], named: HeaderSummary, filter: Filter = null): Draft => ({
  area: { key, name: placeNameOf(named) || UNNAMED_PLACE, placeKey: placeKeyOf(named), headers, rooms: [], matrix },
  place: named.location_index,
  filter,
});

const byFirstHeader = (a: Draft, b: Draft) => a.area.headers[0].id - b.area.headers[0].id;

/**
 * The world as the tree shows it (Gutachten A §1.2): world maps (matrices with a header section whose cells carry at
 * least two places; HeartGold/SoulSilver splits one into Johto and Kanto by the headers' Kanto flag) with one area per
 * place, areas with their own map, rooms under the area of their entrance (breadth-first over the warp graph, through
 * other rooms), the rooms without one, and the headers that are none of these.
 */
export function buildWorld({ family, headers, matrices, links, cells, meta }: WorldInput): World {
  const sorted = [...headers].sort((a, b) => a.id - b.id);
  const byId = new Map(sorted.map((h) => [h.id, h]));
  const matrixByIndex = new Map(matrices.map((m) => [m.index, m]));
  const notWorldMaps = new Set(meta.notWorldMaps);

  const worldMaps: { map: WorldMap; drafts: Draft[] }[] = [];
  const more: Draft[] = [];
  /** Per matrix with known cells, the headers that have a cell there. */
  const placed = new Map<number, Map<number, number>>();
  /** Headers already in an area: a header lies in one area only (the first by matrix index). */
  const claimed = new Set<number>();
  const claim = (draft: Draft) => {
    for (const h of draft.area.headers) claimed.add(h.id);
    return draft;
  };

  for (const [index, dto] of [...cells].sort((a, b) => a[0] - b[0])) {
    if (!dto.headers) continue;
    const allCounts = cellCounts(dto, byId);
    placed.set(index, allCounts);
    const counts = new Map([...allCounts].filter(([id]) => !claimed.has(id)));
    // By id, so the first header of an area is its lowest id whatever the order of the cells.
    const cellHeaders = sorted.filter((h) => counts.has(h.id) && h.location_index !== NO_PLACE_INDEX);
    const places = new Set(cellHeaders.map((h) => h.location_index));
    if (places.size === 0) continue;
    if (places.size < 2 || notWorldMaps.has(index)) {
      more.push(claim(newArea(`m${index}`, index, cellHeaders, mostCells(counts, byId) as HeaderSummary)));
      continue;
    }
    const halves: [Filter, HeaderSummary[]][] = [];
    const kanto = cellHeaders.filter((h) => h.kanto);
    if (family === "Hgss" && kanto.length > 0 && kanto.length < cellHeaders.length) {
      halves.push(["johto", cellHeaders.filter((h) => !h.kanto)], ["kanto", kanto]);
    } else {
      halves.push([null, cellHeaders]);
    }
    for (const [filter, half] of halves) {
      const key = `w${index}${filter === "kanto" ? "k" : ""}`;
      const groups = new Map<number, HeaderSummary[]>();
      for (const h of half) {
        const group = groups.get(h.location_index);
        if (group) group.push(h);
        else groups.set(h.location_index, [h]);
      }
      const drafts = [...groups].map(([place, group]) => claim(newArea(`${key}:${place}`, index, group, group[0], filter))).sort(byFirstHeader);
      const halfCounts = new Map([...counts].filter(([id]) => half.some((h) => h.id === id)));
      const name = meta.worldMaps.find((w) => w.matrix === index && (w.filter ?? null) === filter)?.name.trim() || defaultMapName(family, index, filter, halfCounts, byId);
      worldMaps.push({ map: { key, matrix: index, filter, name, areas: drafts.map((d) => d.area) }, drafts });
    }
  }

  // Outside without a header section: a matrix of more than one map that a town or route header (type 1 or 2) uses.
  const onMatrix = new Map<number, HeaderSummary[]>();
  for (const h of sorted) {
    const list = onMatrix.get(h.matrix_id);
    if (list) list.push(h);
    else onMatrix.set(h.matrix_id, [h]);
  }
  for (const m of [...matrices].sort((a, b) => a.index - b.index)) {
    if (m.has_headers || placed.has(m.index) || m.filled <= 1) continue;
    const users = (onMatrix.get(m.index) ?? []).filter((h) => h.id !== LANDSCAPE && !claimed.has(h.id));
    if (!users.some((h) => h.map_type === 1 || h.map_type === 2)) continue;
    more.push(claim(newArea(`m${m.index}`, m.index, users, users.find((h) => h.location_index !== NO_PLACE_INDEX) ?? users[0])));
  }
  more.sort(byFirstHeader);

  const drafts = [...worldMaps.flatMap((w) => w.drafts), ...more];
  const draftByKey = new Map(drafts.map((d) => [d.area.key, d]));
  const draftOfHeader = new Map<number, Draft>();
  for (const d of drafts) for (const h of d.area.headers) draftOfHeader.set(h.id, d);

  // Sort the rest into rooms and other.
  const other: HeaderSummary[] = [];
  const rooms: HeaderSummary[] = [];
  for (const h of sorted) {
    if (draftOfHeader.has(h.id)) continue;
    const matrix = matrixByIndex.get(h.matrix_id);
    const orphan = h.location_index === NO_PLACE_INDEX && placed.has(h.matrix_id) && !placed.get(h.matrix_id)?.has(h.id);
    if (h.id === LANDSCAPE || !matrix || matrix.filled === 0 || orphan) other.push(h);
    else rooms.push(h);
  }

  const neighbours = warpGraph(sorted, links);
  const owners = new Map<number, { draft: Draft; manual: boolean }>();
  const byEntrance = new Map<number, Draft>();
  const open: HeaderSummary[] = [];
  for (const room of rooms) {
    const manual = ownerFromMeta(meta.roomOwner[String(room.id)], draftByKey, draftOfHeader);
    if (manual) {
      owners.set(room.id, { draft: manual, manual: true });
      continue;
    }
    const draft = entranceArea(room, neighbours, draftOfHeader);
    if (draft) byEntrance.set(room.id, draft);
    else open.push(room);
  }
  for (const [id, draft] of byEntrance) owners.set(id, { draft, manual: false });
  // Without an entrance: an area of the room's place, else the area most rooms of its place lie in by their entrance
  // (the levels of the Höhle der Umkehr the game moves the player through by script).
  for (const room of open) {
    const draft = pick(drafts.filter((d) => samePlace(d, room)), room) ?? siblingsArea(room, rooms, byEntrance);
    if (draft) owners.set(room.id, { draft, manual: false });
  }

  const sharing = new Map<number, number>();
  for (const h of sorted) sharing.set(h.matrix_id, (sharing.get(h.matrix_id) ?? 0) + 1);
  const roomsOf = (list: HeaderSummary[], areaPlaceName: string, manual: (id: number) => boolean): Room[] => {
    const suggestions = suggestRoomNames(
      list.map((header) => ({ header, placeName: placeNameOf(header) })),
      areaPlaceName,
    );
    return list.map((header, i) => {
      const placeName = placeNameOf(header);
      const suggested = suggestions[i]!;
      return {
        header,
        name: meta.roomNames[String(header.id)]?.trim() || suggested,
        suggested,
        placeKey: placeKeyOf(header),
        placeName: placeName || UNNAMED_PLACE,
        manual: manual(header.id),
        shared: (sharing.get(header.matrix_id) ?? 1) - 1,
      };
    });
  };

  const areaOf = new Map<number, string>();
  for (const d of drafts) {
    for (const h of d.area.headers) areaOf.set(h.id, d.area.key);
    const own = rooms.filter((r) => owners.get(r.id)?.draft === d);
    for (const r of own) areaOf.set(r.id, d.area.key);
    d.area.rooms = roomsOf(own, d.place === NO_PLACE_INDEX ? "" : placeNameOf(d.area.headers.find((h) => h.location_index === d.place) ?? d.area.headers[0]), (id) => owners.get(id)?.manual ?? false);
  }
  const unconnected = roomsOf(
    rooms.filter((r) => !owners.has(r.id)),
    "",
    () => false,
  );

  return {
    worldMaps: worldMaps.map((w) => w.map),
    moreAreas: more.map((d) => d.area),
    unconnected,
    other,
    areaOf,
    roomCount: rooms.length,
    assignedRooms: owners.size,
  };
}

/**
 * The undirected warp graph: a warp joins the header of its event file and its target. A file several headers share
 * (event file 0) makes no edges, else everything would hang on everything; the landscape header joins nothing.
 */
function warpGraph(headers: readonly HeaderSummary[], links: readonly WarpLink[]): Map<number, Set<number>> {
  const byFile = new Map<number, HeaderSummary[]>();
  for (const h of headers) {
    const list = byFile.get(h.event_file_id);
    if (list) list.push(h);
    else byFile.set(h.event_file_id, [h]);
  }
  const known = new Set(headers.map((h) => h.id));
  const neighbours = new Map<number, Set<number>>();
  const join = (a: number, b: number) => {
    let set = neighbours.get(a);
    if (!set) neighbours.set(a, (set = new Set()));
    set.add(b);
  };
  for (const link of links) {
    const users = byFile.get(link.event_file);
    if (!users || users.length !== 1) continue;
    const from = users[0].id;
    const to = link.dest_header_id;
    if (from === to || from === LANDSCAPE || to === LANDSCAPE || !known.has(to)) continue;
    join(from, to);
    join(to, from);
  }
  return neighbours;
}

/** The area of the nearest area header over the warp graph (through rooms); undefined when none is reachable. */
function entranceArea(room: HeaderSummary, neighbours: ReadonlyMap<number, ReadonlySet<number>>, draftOfHeader: ReadonlyMap<number, Draft>): Draft | undefined {
  const seen = new Set([room.id]);
  let frontier = [room.id];
  while (frontier.length > 0) {
    const next: number[] = [];
    const hits: Draft[] = [];
    for (const id of frontier) {
      for (const n of neighbours.get(id) ?? []) {
        if (seen.has(n)) continue;
        seen.add(n);
        const draft = draftOfHeader.get(n);
        if (draft) hits.push(draft);
        else next.push(n);
      }
    }
    if (hits.length > 0) return pick(hits, room);
    frontier = next;
  }
  return undefined;
}

/**
 * Of several areas the one of the room's place (of the room's half, Johto or Kanto, when the place lies in both), else
 * the one with the lowest first header id.
 */
function pick(candidates: readonly Draft[], room: HeaderSummary): Draft | undefined {
  const same = preferHalf(candidates.filter((d) => samePlace(d, room)), room);
  const pool = same.length > 0 ? same : candidates;
  return [...pool].sort(byFirstHeader)[0];
}

/** Whether an area lies in the room's half of a split world map (an area of no split map lies in both). */
const sameHalf = (d: Draft, room: HeaderSummary) => d.filter === null || (d.filter === "kanto") === room.kanto;

/** The areas of `drafts` in the room's half, or all of them when none is. */
function preferHalf(drafts: readonly Draft[], room: HeaderSummary): readonly Draft[] {
  const half = drafts.filter((d) => sameHalf(d, room));
  return half.length > 0 ? half : drafts;
}

/**
 * The area most rooms of `room`'s place lie in by their entrance, of the room's half when one lies there (ties: the lowest
 * first header id).
 */
function siblingsArea(room: HeaderSummary, rooms: readonly HeaderSummary[], byEntrance: ReadonlyMap<number, Draft>): Draft | undefined {
  if (room.location_index === NO_PLACE_INDEX) return undefined;
  const votes = new Map<Draft, number>();
  for (const other of rooms) {
    const draft = other.location_index === room.location_index ? byEntrance.get(other.id) : undefined;
    if (draft) votes.set(draft, (votes.get(draft) ?? 0) + 1);
  }
  const pool = new Set(preferHalf([...votes.keys()], room));
  let best: Draft | undefined;
  for (const [draft, n] of votes) {
    if (!pool.has(draft)) continue;
    const bestVotes = best ? (votes.get(best) ?? 0) : 0;
    if (!best || n > bestVotes || (n === bestVotes && byFirstHeader(draft, best) < 0)) best = draft;
  }
  return best;
}

const samePlace = (d: Draft, room: HeaderSummary) => room.location_index !== NO_PLACE_INDEX && d.place === room.location_index;

/** The area the project file assigns a room to: by area key, or by the id of one of the area's headers. */
function ownerFromMeta(value: string | undefined, byKey: ReadonlyMap<string, Draft>, ofHeader: ReadonlyMap<number, Draft>): Draft | undefined {
  if (value === undefined) return undefined;
  return byKey.get(value) ?? (/^\d+$/.test(value) ? ofHeader.get(Number(value)) : undefined);
}

/** The room of header `id` (under an area or unconnected); undefined for an area, another header or an unknown id. */
export function roomOf(world: World, id: number): Room | undefined {
  const areas = [...world.worldMaps.flatMap((w) => w.areas), ...world.moreAreas];
  for (const area of areas) {
    const room = area.rooms.find((r) => r.header.id === id);
    if (room) return room;
  }
  return world.unconnected.find((r) => r.header.id === id);
}

/** What the world model says a header is: an area (a section of one), a room, or one of the others; and how it is called. */
export interface WorldName {
  kind: "area" | "room" | "other";
  /** The area's name, the room's name (the project file's, else the suggested one), else the header's place or code. */
  name: string;
  /** The header's place name ("Ohne Ortsnamen" for none). */
  placeName: string;
}

export type WorldNames = ReadonlyMap<number, WorldName>;

/** Header id to what the world tree calls it: the breadcrumb, the inspector head and the door under the pointer say the same. */
export function worldNames(world: World): WorldNames {
  const names = new Map<number, WorldName>();
  const placeName = (h: HeaderSummary) => placeNameOf(h) || UNNAMED_PLACE;
  const areas = [...world.worldMaps.flatMap((w) => w.areas), ...world.moreAreas];
  for (const area of areas) {
    for (const h of area.headers) names.set(h.id, { kind: "area", name: area.name, placeName: placeName(h) });
    for (const room of area.rooms) names.set(room.header.id, { kind: "room", name: room.name, placeName: room.placeName });
  }
  for (const room of world.unconnected) names.set(room.header.id, { kind: "room", name: room.name, placeName: room.placeName });
  for (const h of world.other) names.set(h.id, { kind: "other", name: placeNameOf(h) || h.internal_name, placeName: placeName(h) });
  return names;
}
