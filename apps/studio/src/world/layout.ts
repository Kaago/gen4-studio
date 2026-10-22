import type { MatrixDto } from "../api";

/** Tiles per map cell edge. The scene uses tile units. */
export const MAP_SIZE = 32;

/** apicula GLB units per tile: a 32-tile map model spans 512 units (measured on vanilla Pt/HGSS maps). Building models use the same scale. */
export const MODEL_UNITS_PER_TILE = 16;
export const MODEL_SCALE = 1 / MODEL_UNITS_PER_TILE;

/** Scene origin of matrix cell (col, row). */
export function cellOrigin(col: number, row: number): { x: number; z: number } {
  return { x: col * MAP_SIZE, z: row * MAP_SIZE };
}

/** Scene height of one altitude step: half a tile (pokeplatinum `LandDataManager_CalculateRenderingPosition`: altitude x tile size / 2). */
export const ALTITUDE_STEP = 0.5;

/** Scene height of matrix cell (`col`, `row`): its altitude steps times `ALTITUDE_STEP`; 0 without an altitude section. */
export function cellY(matrix: Pick<MatrixDto, "width" | "altitudes">, col: number, row: number): number {
  return (matrix.altitudes?.[row * matrix.width + col] ?? 0) * ALTITUDE_STEP;
}

/** Scene height of the cell holding absolute matrix tile (`x`, `z`); 0 outside the altitude grid. */
export function cellYAtTile(matrix: Pick<MatrixDto, "width" | "altitudes">, x: number, z: number): number {
  const col = Math.floor(x / MAP_SIZE);
  if (col < 0 || col >= matrix.width) return 0;
  return cellY(matrix, col, Math.floor(z / MAP_SIZE));
}
