import type { HeaderSummary, MatrixDto } from "../api";

/** In-game location name, or the internal name for headers without one. */
export function headerLabel(h: HeaderSummary): string {
  return h.location_name || h.internal_name;
}

/** Headers whose location name or internal name contains `query` (case-insensitive). */
export function filterHeaders(headers: HeaderSummary[], query: string): HeaderSummary[] {
  const q = query.trim().toLowerCase();
  if (q === "") return headers;
  return headers.filter((h) => h.location_name.toLowerCase().includes(q) || h.internal_name.toLowerCase().includes(q));
}

/** True when the header uses the matrix the scene shows right now (`shown`: its index, null: none). */
export function showsMatrix(h: HeaderSummary, shown: number | null): boolean {
  return shown !== null && h.matrix_id === shown;
}

/**
 * The map that stands for `header` in `matrix`: the first cell whose header section names it, else the first map of the
 * matrix (a room's matrix has one map and no header section). Null when the matrix places no map.
 */
export function mapOfHeader(matrix: MatrixDto, header: number): number | null {
  const cell = matrix.headers ? matrix.headers.findIndex((h, i) => h === header && matrix.maps[i] != null) : -1;
  if (cell >= 0) return matrix.maps[cell] ?? null;
  return matrix.maps.find((m): m is number => m != null) ?? null;
}
