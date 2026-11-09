import { MATRIX_EMPTY, type HeaderSummary, type MatrixDto } from "../api";
import { t } from "../i18n";
import { cellKey } from "./AreaGrid";
import type { CellRect } from "./sceneScope";
import type { Area, World, WorldMap } from "./worldModel";

/** What the world map panel shows: a world map (`w<matrix>`, Kanto `w<matrix>k`), both halves (`w<matrix>all`), `more` or `plan`. */
export type WorldViewKey = string;

export interface WorldViewItem {
  key: WorldViewKey;
  label: string;
  /** The matrix the view draws; null for "Weitere Gebiete" and the room plan (they draw other matrices). */
  matrix: number | null;
}

/** A matrix cell holds a map. */
export const isFilled = (map: number | null | undefined): boolean => map !== null && map !== undefined && map !== MATRIX_EMPTY;

/**
 * The views of the world map panel in the order of its switcher: the world maps, after the last half of a split matrix the view of
 * both halves, "Weitere Gebiete" while there are such areas, and "Raumplan" while `planShown` (a room or an area with its own map is shown).
 */
export function worldViews(world: World, planShown: boolean): WorldViewItem[] {
  const views: WorldViewItem[] = [];
  world.worldMaps.forEach((map, i) => {
    views.push({ key: map.key, label: map.name, matrix: map.matrix });
    const halves = world.worldMaps.filter((m) => m.matrix === map.matrix && m.filter !== null);
    if (map.filter !== null && halves.length > 1 && world.worldMaps[i + 1]?.matrix !== map.matrix) {
      views.push({ key: `w${map.matrix}all`, label: halves.map((m) => m.name).join(t("workbench:worldMapView.and")), matrix: map.matrix });
    }
  });
  if (world.moreAreas.length > 0) views.push({ key: "more", label: t("workbench:worldTree.groups.more"), matrix: null });
  if (planShown) views.push({ key: "plan", label: t("workbench:worldMapView.plan"), matrix: null });
  return views;
}

function findArea(world: World, key: string): { area: Area; map: WorldMap | null } | null {
  for (const map of world.worldMaps) {
    const area = map.areas.find((a) => a.key === key);
    if (area) return { area, map };
  }
  const area = world.moreAreas.find((a) => a.key === key);
  return area ? { area, map: null } : null;
}

/**
 * The view that shows `header`: the world map of the area it is a section of; the room plan for a room and for an area with a map of
 * its own; none for the landscape, a header outside the world model, or while the world is not read.
 */
export function viewFor(world: World | null, header: HeaderSummary | null): WorldViewKey | null {
  if (!world || !header) return null;
  const key = world.areaOf.get(header.id);
  const found = key === undefined ? null : findArea(world, key);
  if (!found) return world.unconnected.some((r) => r.header.id === header.id) ? "plan" : null;
  const section = found.area.headers.some((h) => h.id === header.id);
  return found.map && section ? found.map.key : "plan";
}

/** The cells of `matrix` (index y * width + x) with a header of the given set, filled ones only. */
export function cellsOfHeaders(matrix: MatrixDto, ids: ReadonlySet<number>): number[] {
  const headers = matrix.headers ?? [];
  const cells: number[] = [];
  headers.forEach((id, i) => {
    if (ids.has(id) && isFilled(matrix.maps[i])) cells.push(i);
  });
  return cells;
}

/**
 * The cells the view of `map` shows of `matrix`: the rectangle around the cells of its areas plus one cell of margin (inside the matrix),
 * and the `foreign` cells in it that belong to an area of the other half of a split matrix (drawn dimmed, not selectable as an area).
 * An unsplit world map shows the whole matrix.
 */
export function halfWindow(world: World, map: WorldMap, matrix: MatrixDto): { rect: CellRect; foreign: ReadonlySet<string> } {
  const whole: CellRect = { x: 0, y: 0, width: matrix.width, height: matrix.height };
  if (map.filter === null) return { rect: whole, foreign: new Set() };
  const idsOf = (maps: WorldMap[]) => new Set(maps.flatMap((m) => m.areas.flatMap((a) => a.headers.map((h) => h.id))));
  const own = cellsOfHeaders(matrix, idsOf([map]));
  if (own.length === 0) return { rect: whole, foreign: new Set() };
  const xs = own.map((i) => i % matrix.width);
  const ys = own.map((i) => Math.floor(i / matrix.width));
  const x0 = Math.max(0, Math.min(...xs) - 1);
  const y0 = Math.max(0, Math.min(...ys) - 1);
  const x1 = Math.min(matrix.width - 1, Math.max(...xs) + 1);
  const y1 = Math.min(matrix.height - 1, Math.max(...ys) + 1);
  const others = cellsOfHeaders(matrix, idsOf(world.worldMaps.filter((m) => m !== map && m.matrix === map.matrix)));
  return {
    rect: { x: x0, y: y0, width: x1 - x0 + 1, height: y1 - y0 + 1 },
    foreign: new Set(others.map((i) => cellKey(i % matrix.width, Math.floor(i / matrix.width)))),
  };
}

/** The area whose section the cell (x, y) of `matrix` is; null for landscape, a header without an area, an empty cell, or none there. */
export function areaAtCell(world: World, matrix: MatrixDto, x: number, y: number): Area | null {
  if (x < 0 || y < 0 || x >= matrix.width || y >= matrix.height || !matrix.headers) return null;
  const i = y * matrix.width + x;
  const header = matrix.headers[i];
  if (header === undefined || !isFilled(matrix.maps[i])) return null;
  const key = world.areaOf.get(header);
  const area = key === undefined ? null : (findArea(world, key)?.area ?? null);
  return area && area.matrix === matrix.index && area.headers.some((h) => h.id === header) ? area : null;
}

/** The `cellKey`s of the filled cells of `matrix` with a header of the given set (the cells of an area). */
export function cellKeysOfHeaders(matrix: MatrixDto, ids: ReadonlySet<number>): Set<string> {
  return new Set(cellsOfHeaders(matrix, ids).map((i) => cellKey(i % matrix.width, Math.floor(i / matrix.width))));
}

const NEIGHBOURS: readonly (readonly [number, number])[] = [
  [0, -1],
  [-1, 0],
  [1, 0],
  [0, 1],
];

/** The first of the 4 neighbours of (x, y) (above, left, right, below) that lies in the matrix and whose cell index satisfies `test`. */
export function firstNeighbour(matrix: MatrixDto, x: number, y: number, test: (i: number) => boolean): { x: number; y: number } | null {
  for (const [dx, dy] of NEIGHBOURS) {
    const nx = x + dx;
    const ny = y + dy;
    if (nx >= 0 && ny >= 0 && nx < matrix.width && ny < matrix.height && test(ny * matrix.width + nx)) return { x: nx, y: ny };
  }
  return null;
}

/** Whether a neighbour of (x, y) lies in the matrix and its cell index satisfies `test`. */
const neighbourIs = (matrix: MatrixDto, x: number, y: number, test: (i: number) => boolean): boolean => firstNeighbour(matrix, x, y, test) !== null;

/**
 * The cell a "+" at (x, y) of a room plan copies: a filled neighbour with a header other than 0 (the new map belongs to its room), else
 * any filled neighbour (a plan without header section).
 */
export function planNeighbour(matrix: MatrixDto, x: number, y: number): { x: number; y: number } | null {
  const headers = matrix.headers;
  const withHeader = headers ? firstNeighbour(matrix, x, y, (i) => isFilled(matrix.maps[i]) && (headers[i] ?? 0) !== 0) : null;
  return withHeader ?? firstNeighbour(matrix, x, y, (i) => isFilled(matrix.maps[i]));
}

/** The empty cells of `matrix` that touch (4 neighbours) a cell of the area with the headers `areaHeaders`: where "+" extends the area. */
export function worldExtendTargets(matrix: MatrixDto, areaHeaders: readonly number[]): Set<string> {
  const targets = new Set<string>();
  const headers = matrix.headers;
  if (!headers) return targets;
  const wanted = new Set(areaHeaders);
  const inArea = (i: number) => isFilled(matrix.maps[i]) && wanted.has(headers[i]);
  for (let y = 0; y < matrix.height; y++) {
    for (let x = 0; x < matrix.width; x++) {
      if (!isFilled(matrix.maps[y * matrix.width + x]) && neighbourIs(matrix, x, y, inArea)) targets.add(cellKey(x, y));
    }
  }
  return targets;
}

/**
 * Where "+" extends a room plan: the empty cells beside filled ones, and the column right of the matrix and the row below it beside
 * filled border cells (a matrix grows to the right and downwards only).
 */
export function planExtendTargets(matrix: MatrixDto): Set<string> {
  const targets = new Set<string>();
  const filled = (i: number) => isFilled(matrix.maps[i]);
  for (let y = 0; y < matrix.height; y++) {
    for (let x = 0; x < matrix.width; x++) {
      if (!filled(y * matrix.width + x) && neighbourIs(matrix, x, y, filled)) targets.add(cellKey(x, y));
    }
  }
  for (let y = 0; y < matrix.height; y++) if (filled(y * matrix.width + matrix.width - 1)) targets.add(cellKey(matrix.width, y));
  for (let x = 0; x < matrix.width; x++) if (filled((matrix.height - 1) * matrix.width + x)) targets.add(cellKey(x, matrix.height));
  return targets;
}

/**
 * The block of `width` × `height` cells with its upper left cell at `target` on the world map `matrix`: its `cellKey`s inside the matrix,
 * whether an area may go there (`ok`: the matrix has a header section, the block lies inside it and only on empty cells and landscape, a
 * map with header 0) and how many landscape cells it replaces. `cells` (relative to the corner) are the area's maps where it has holes:
 * only they replace landscape; default the whole block.
 */
export function targetBlock(
  matrix: MatrixDto,
  target: { x: number; y: number },
  width: number,
  height: number,
  cells?: readonly { x: number; y: number }[],
): { cells: Set<string>; ok: boolean; replaced: number } {
  const keys = new Set<string>();
  const headers = matrix.headers;
  let ok = headers != null && target.x + width <= matrix.width && target.y + height <= matrix.height;
  for (let y = target.y; y < Math.min(matrix.height, target.y + height); y++) {
    for (let x = target.x; x < Math.min(matrix.width, target.x + width); x++) {
      keys.add(cellKey(x, y));
      const i = y * matrix.width + x;
      if (isFilled(matrix.maps[i]) && (headers?.[i] ?? 0) !== 0) ok = false;
    }
  }
  const covered = cells ?? Array.from({ length: width * height }, (_, i) => ({ x: i % width, y: Math.floor(i / width) }));
  let replaced = 0;
  for (const c of covered) {
    const x = target.x + c.x;
    const y = target.y + c.y;
    const i = y * matrix.width + x;
    if (x < matrix.width && y < matrix.height && headers && isFilled(matrix.maps[i]) && headers[i] === 0) replaced++;
  }
  return { cells: keys, ok, replaced };
}
