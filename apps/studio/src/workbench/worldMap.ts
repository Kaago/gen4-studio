import type { HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../api";
import { placeColor } from "./places";
import type { Place } from "./places";

/** Pixels between two cells of an `AreaGrid` (the grid's `gap`). */
export const GRID_GAP = 2;
/** Tiles along one side of a map: a warp at tile (x, z) of the region lies in cell (⌊x / 32⌋, ⌊z / 32⌋). */
const MAP_TILES = 32;

/** The matrix the world map draws: the one with a header section and the most filled cells (a tie goes to the smaller index). */
export function regionMatrix(matrices: readonly MatrixSummary[]): MatrixSummary | null {
  let best: MatrixSummary | null = null;
  for (const m of matrices) {
    if (!m.has_headers) continue;
    if (best === null || m.filled > best.filled || (m.filled === best.filled && m.index < best.index)) best = m;
  }
  return best;
}

export interface PlaceCells {
  key: string;
  name: string;
  color: string;
  cells: { x: number; y: number }[];
  /** Mean of the cells, in cell units (not necessarily a cell of the place). */
  center: { x: number; y: number };
}

/** The region's cells grouped by place (places keep their order). Cells without a map belong to no place; places without a cell are left out. */
export function placeCells(region: MatrixDto, places: readonly Place[]): PlaceCells[] {
  const headerCells = region.headers;
  if (!headerCells) return [];
  const placeOfHeader = new Map<number, number>();
  places.forEach((place, i) => place.headers.forEach(({ header }) => placeOfHeader.set(header.id, i)));
  const cells = places.map(() => [] as { x: number; y: number }[]);
  for (let i = 0; i < region.maps.length; i++) {
    if (region.maps[i] === null) continue;
    const place = placeOfHeader.get(headerCells[i]);
    if (place !== undefined) cells[place].push({ x: i % region.width, y: Math.floor(i / region.width) });
  }
  const result: PlaceCells[] = [];
  places.forEach((place, i) => {
    const own = cells[i];
    if (own.length === 0) return;
    const center = { x: own.reduce((sum, c) => sum + c.x, 0) / own.length, y: own.reduce((sum, c) => sum + c.y, 0) / own.length };
    result.push({ key: place.key, name: place.name, color: placeColor(place.key), cells: own, center });
  });
  return result;
}

export interface RoomLink {
  header: number;
  /** The region cell of the room's entrance; null when it cannot be told (`reason` says why). */
  cell: { x: number; y: number } | null;
  reason: string | null;
}

/**
 * For each room of `place` the region cell of its entrance: a warp of the room that leads to a header of the
 * region, and the warp of that header it is paired with (its tile gives the cell). A room with several exits takes
 * the first one that resolves, else the first reason.
 */
/** The lookups `roomLinks` needs, built once for all places (headers by id, warps by event file and by file and index). */
export interface RoomLinkIndex {
  byId: ReadonlyMap<number, HeaderSummary>;
  /** The warps of each event file, by index. */
  byFile: ReadonlyMap<number, readonly WarpLink[]>;
  byWarp: ReadonlyMap<string, WarpLink>;
}

const warpKey = (file: number, index: number) => `${file}:${index}`;

export function roomLinkIndex(headers: readonly HeaderSummary[], links: readonly WarpLink[]): RoomLinkIndex {
  const byFile = new Map<number, WarpLink[]>();
  const byWarp = new Map<string, WarpLink>();
  for (const link of links) {
    const list = byFile.get(link.event_file);
    if (list) list.push(link);
    else byFile.set(link.event_file, [link]);
    byWarp.set(warpKey(link.event_file, link.index), link);
  }
  for (const list of byFile.values()) list.sort((a, b) => a.index - b.index);
  return { byId: new Map(headers.map((h) => [h.id, h])), byFile, byWarp };
}

/** `index` is built from `headers` and `links` when not given (the world map passes one for all its places). */
export function roomLinks(place: Place, headers: readonly HeaderSummary[], links: readonly WarpLink[], region: MatrixDto, index: RoomLinkIndex = roomLinkIndex(headers, links)): RoomLink[] {
  const result: RoomLink[] = [];
  for (const { header, kind } of place.headers) {
    if (kind !== "inside") continue;
    const exits = index.byFile.get(header.event_file_id) ?? [];
    let reason = "kein Ausgang";
    // Rooms may share an event file (the wizard can keep it), so the exits of the file are not all this room's: the
    // one whose paired outside warp leads back to this header is; else the first that resolves.
    let cell: RoomLink["cell"] = null;
    let fallback: RoomLink["cell"] = null;
    for (const [n, exit] of exits.entries()) {
      const outcome = entrance(exit, index, region);
      if (typeof outcome === "string") {
        if (n === 0) reason = outcome;
        continue;
      }
      if (outcome.paired.dest_header_id === header.id) {
        cell = outcome.cell;
        break;
      }
      fallback ??= outcome.cell;
    }
    cell ??= fallback;
    result.push({ header: header.id, cell, reason: cell === null ? reason : null });
  }
  return result;
}

/** The paired warp of `exit` and the cell it lies in, or the reason there is none. */
function entrance(exit: WarpLink, index: RoomLinkIndex, region: MatrixDto): { cell: { x: number; y: number }; paired: WarpLink } | string {
  const target = index.byId.get(exit.dest_header_id);
  if (!target || target.matrix_id !== region.index) return "Ziel liegt nicht auf der Weltkarte";
  const paired = index.byWarp.get(warpKey(target.event_file_id, exit.dest_warp_id));
  if (!paired) return "Ziel-Warp fehlt";
  const cell = { x: Math.floor(paired.x / MAP_TILES), y: Math.floor(paired.z / MAP_TILES) };
  if (cell.x < 0 || cell.y < 0 || cell.x >= region.width || cell.y >= region.height) return "Ziel außerhalb der Region";
  return { cell, paired };
}

/**
 * The line from the left edge of a room card to the centre of a region cell. `card` and the result share one frame
 * (client pixels); `origin` is where the grid's top-left corner lies in that frame (after pan and zoom), `scale` the zoom.
 */
export function lineBetween(card: DOMRect, cell: { x: number; y: number }, cellSize: number, origin: { x: number; y: number }, scale: number): { x1: number; y1: number; x2: number; y2: number } {
  const pitch = cellSize + GRID_GAP;
  return {
    x1: card.left,
    y1: card.top + card.height / 2,
    x2: origin.x + (cell.x * pitch + cellSize / 2) * scale,
    y2: origin.y + (cell.y * pitch + cellSize / 2) * scale,
  };
}

/** Lines and pages of a wheel event in pixels (`WheelEvent.deltaMode` 1 and 2), so line-mode wheels zoom like the others. */
export function wheelPixels(e: Pick<WheelEvent, "deltaY" | "deltaMode">, pageHeight = 800): number {
  if (e.deltaMode === 1) return e.deltaY * 16;
  if (e.deltaMode === 2) return e.deltaY * pageHeight;
  return e.deltaY;
}
