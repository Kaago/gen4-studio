import { MAP_SIZE } from "./layout";

/** Collision byte for a blocked tile. */
export const BLOCKED = 0x80;

/**
 * Maps a world hit (tile units) to the tile coordinates 0..31 inside the matrix cell whose
 * origin is `cell`. The far edge belongs to the neighbouring cell; outside the cell is null.
 */
export function pickCell(
  hit: { x: number; z: number },
  cell: { x: number; z: number },
): { x: number; y: number } | null {
  const x = Math.floor(hit.x - cell.x);
  const y = Math.floor(hit.z - cell.z);
  if (x < 0 || x >= MAP_SIZE || y < 0 || y >= MAP_SIZE) return null;
  return { x, y };
}

/** Matrix column/row containing a world hit, or null outside the matrix. */
export function matrixCellAt(
  hit: { x: number; z: number },
  width: number,
  height: number,
): { col: number; row: number } | null {
  const col = Math.floor(hit.x / MAP_SIZE);
  const row = Math.floor(hit.z / MAP_SIZE);
  if (col < 0 || col >= width || row < 0 || row >= height) return null;
  return { col, row };
}

/** True if pointer movement between down and up is small enough to count as a click (not an orbit drag). */
export function isClick(down: { x: number; y: number }, up: { x: number; y: number }): boolean {
  return Math.hypot(up.x - down.x, up.y - down.y) < 4;
}
