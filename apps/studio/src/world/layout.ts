import type { BuildingDto } from "../api";

/** Tiles per map cell edge. The scene uses tile units. */
export const MAP_SIZE = 32;

/** Scene origin of matrix cell (col, row). */
export function cellOrigin(col: number, row: number): { x: number; z: number } {
  return { x: col * MAP_SIZE, z: row * MAP_SIZE };
}

/** Box for a building; sizes are the scale factors (placeholder volume until real models, Phase 2). */
export function buildingBox(b: BuildingDto, cell: { x: number; z: number }) {
  return {
    cx: cell.x + MAP_SIZE / 2 + b.x,
    cy: b.y + b.sy / 2,
    cz: cell.z + MAP_SIZE / 2 + b.z,
    sx: b.sx,
    sy: b.sy,
    sz: b.sz,
  };
}
