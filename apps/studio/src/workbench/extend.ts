import type { ExtendSpec, MatrixDto } from "../api";

export interface ExtendDefaults {
  /** The nearest filled cell to the left in the target's row, else the nearest above it in the target's column. */
  neighbour: { x: number; y: number; map: number; header: number | null } | null;
  /** The size the area has after the extension. */
  width: number;
  height: number;
}

function cellAt(matrix: MatrixDto, x: number, y: number): ExtendDefaults["neighbour"] {
  if (x < 0 || y < 0 || x >= matrix.width || y >= matrix.height) return null;
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  if (map === null) return null;
  return { x, y, map, header: matrix.headers ? (matrix.headers[i] ?? null) : null };
}

/**
 * What the extend dialog proposes for the empty or new cell `(x, y)`: the cell to copy and the size afterwards. `prefer` names the cell
 * to propose when it holds a map (the area's cell beside a "+" of the world map), before the search to the left and upwards.
 */
export function extendDefaults(matrix: MatrixDto, x: number, y: number, prefer: { x: number; y: number } | null = null): ExtendDefaults {
  let neighbour: ExtendDefaults["neighbour"] = prefer ? cellAt(matrix, prefer.x, prefer.y) : null;
  for (let nx = Math.min(x, matrix.width) - 1; nx >= 0 && neighbour === null; nx--) neighbour = cellAt(matrix, nx, y);
  for (let ny = Math.min(y, matrix.height) - 1; ny >= 0 && neighbour === null; ny--) neighbour = cellAt(matrix, x, ny);
  return { neighbour, width: Math.max(matrix.width, x + 1), height: Math.max(matrix.height, y + 1) };
}

/** The backend request; `headerCell` is only sent when the matrix has a header section. */
export function extendSpec(
  matrix: MatrixDto,
  x: number,
  y: number,
  source: { map: number; clearBuildings: boolean },
  headerCell: number | null,
): ExtendSpec {
  return {
    matrix: matrix.index,
    x,
    y,
    source_map: source.map,
    clear_buildings: source.clearBuildings,
    header_cell: matrix.headers ? headerCell : null,
  };
}
