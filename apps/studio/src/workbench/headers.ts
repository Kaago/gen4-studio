import type { HeaderSummary } from "../api";

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
