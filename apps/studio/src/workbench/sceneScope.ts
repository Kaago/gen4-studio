import { useMemo, useState } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import type { Area, World } from "./worldModel";

/** A rectangle of matrix cells: column `x`, row `y`, `width` columns by `height` rows. */
export interface CellRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** What the scene loads of a matrix and what its camera frames. */
export interface SceneScope {
  /** The cells to load, as cell index y * width + x. */
  cells: ReadonlySet<number>;
  /** The cells of the area itself, without the ring of neighbours. */
  focus: CellRect;
}

/** A matrix cell holds a map (an index past the maps holds none). */
const isFilled = (map: number | null | undefined): boolean => map !== null && map !== undefined;

/** The smallest rectangle around the cells `indices` of a matrix `width` columns wide; null for none. */
function boundingRect(indices: Iterable<number>, width: number): CellRect | null {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const i of indices) {
    const x = i % width;
    const y = Math.floor(i / width);
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  return x0 === Infinity ? null : { x: x0, y: y0, width: x1 - x0 + 1, height: y1 - y0 + 1 };
}

/**
 * The part of `matrix` the scene shows for an area with the headers `areaHeaders`: the filled cells carrying one of them and the
 * ring of their eight neighbours (whatever those hold, landscape too), inside the matrix. Null (the whole matrix) for a matrix without
 * a header section (a room, an area with its own map), without headers, or when no filled cell carries one of them.
 */
export function sceneScope(matrix: MatrixDto, areaHeaders: readonly number[] | null): SceneScope | null {
  const headers = matrix.headers;
  if (!headers || !areaHeaders?.length) return null;
  const wanted = new Set(areaHeaders);
  const { width, height } = matrix;
  const area: number[] = [];
  headers.forEach((id, i) => {
    if (wanted.has(id) && isFilled(matrix.maps[i])) area.push(i);
  });
  const focus = boundingRect(area, width);
  if (!focus) return null;
  const cells = new Set<number>();
  for (const i of area) {
    const x = i % width;
    const y = Math.floor(i / width);
    for (let ny = Math.max(0, y - 1); ny <= Math.min(height - 1, y + 1); ny++) {
      for (let nx = Math.max(0, x - 1); nx <= Math.min(width - 1, x + 1); nx++) cells.add(ny * width + nx);
    }
  }
  return { cells, focus };
}

/** `matrix` with the maps outside `scope` emptied (size, altitudes, headers and name kept); `matrix` itself for no scope. */
export function scopedMatrix(matrix: MatrixDto, scope: SceneScope | null): MatrixDto {
  if (!scope) return matrix;
  return { ...matrix, maps: matrix.maps.map((m, i) => (scope.cells.has(i) ? m : null)) };
}

/** The rectangle around the filled cells of `matrix`; null when none is filled. */
export function filledRect(matrix: MatrixDto): CellRect | null {
  return boundingRect(matrix.maps.flatMap((m, i) => (isFilled(m) ? [i] : [])), matrix.width);
}

function findArea(world: World, key: string): Area | undefined {
  for (const map of world.worldMaps) {
    const area = map.areas.find((a) => a.key === key);
    if (area) return area;
  }
  return world.moreAreas.find((a) => a.key === key);
}

/**
 * The headers whose cells the scene of `header` shows as its area: all headers of the area it lies in when that area is on the
 * header's matrix (the sections of a route), else the header alone; the header alone too before the world is read. Null without a header.
 */
export function areaHeadersOf(world: World | null, header: HeaderSummary | null): number[] | null {
  if (!header) return null;
  const key = world?.areaOf.get(header.id);
  const area = world && key !== undefined ? findArea(world, key) : undefined;
  if (!area || area.matrix !== header.matrix_id) return [header.id];
  return area.headers.map((h) => h.id);
}

/**
 * The matrix the scene loads for the selected `header` (its area and the ring around it, see `sceneScope`) and the cells its camera
 * frames (null: the scene frames the filled cells). The same objects while the matrix object and the area's headers stay the same,
 * so a reload of the models alone does not load the maps again.
 *
 * The header counts from the render the matrix object arrives in: a selection changes first and its matrix (a new object, even on
 * the same matrix) follows when fetched, so the scene keeps its scope meanwhile and loads once, with the new matrix. A failed
 * fetch keeps the scope too, as the scene keeps its matrix.
 */
export function useSceneScope(matrix: MatrixDto | null, header: HeaderSummary | null, world: World | null): { matrix: MatrixDto | null; focus: CellRect | null } {
  const [shown, setShown] = useState({ matrix, header });
  // Adjusted while rendering (React's pattern for state derived from props): the header that was selected when this matrix arrived.
  if (shown.matrix !== matrix) setShown({ matrix, header });
  const scopeHeader = shown.matrix === matrix ? shown.header : header;
  const key = areaHeadersOf(world, scopeHeader)?.join(",") ?? "";
  return useMemo(() => {
    if (!matrix) return { matrix: null, focus: null };
    const scope = sceneScope(matrix, key === "" ? null : key.split(",").map(Number));
    return { matrix: scopedMatrix(matrix, scope), focus: scope?.focus ?? null };
  }, [matrix, key]);
}
