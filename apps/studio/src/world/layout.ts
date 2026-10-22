/** Tiles per map cell edge. The scene uses tile units. */
export const MAP_SIZE = 32;

/** apicula GLB units per tile: a 32-tile map model spans 512 units (measured on vanilla Pt/HGSS maps). Building models use the same scale. */
export const MODEL_UNITS_PER_TILE = 16;
export const MODEL_SCALE = 1 / MODEL_UNITS_PER_TILE;

/** Scene origin of matrix cell (col, row). */
export function cellOrigin(col: number, row: number): { x: number; z: number } {
  return { x: col * MAP_SIZE, z: row * MAP_SIZE };
}
