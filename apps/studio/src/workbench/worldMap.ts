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
export function roomLinks(place: Place, headers: readonly HeaderSummary[], links: readonly WarpLink[], region: MatrixDto): RoomLink[] {
  const byId = new Map(headers.map((h) => [h.id, h]));
  const result: RoomLink[] = [];
  for (const { header, kind } of place.headers) {
    if (kind !== "inside") continue;
    const exits = links.filter((l) => l.event_file === header.event_file_id).sort((a, b) => a.index - b.index);
    let reason = "kein Ausgang";
    let cell: RoomLink["cell"] = null;
    for (const [n, exit] of exits.entries()) {
      const outcome = entranceCell(exit, byId, links, region);
      if (typeof outcome !== "string") {
        cell = outcome;
        break;
      }
      if (n === 0) reason = outcome;
    }
    result.push({ header: header.id, cell, reason: cell === null ? reason : null });
  }
  return result;
}

/** The cell the paired warp of `exit` lies in, or the reason there is none. */
function entranceCell(exit: WarpLink, byId: ReadonlyMap<number, HeaderSummary>, links: readonly WarpLink[], region: MatrixDto): { x: number; y: number } | string {
  const target = byId.get(exit.dest_header_id);
  if (!target || target.matrix_id !== region.index) return "Ziel ist kein Außen-Header";
  const paired = links.find((l) => l.event_file === target.event_file_id && l.index === exit.dest_warp_id);
  if (!paired) return "Ziel-Warp fehlt";
  const cell = { x: Math.floor(paired.x / MAP_TILES), y: Math.floor(paired.z / MAP_TILES) };
  if (cell.x < 0 || cell.y < 0 || cell.x >= region.width || cell.y >= region.height) return "Ziel außerhalb der Region";
  return cell;
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
