import * as THREE from "three";
import type { MatrixDto } from "../../api";
import { effectiveTheme } from "../../ui/tokens/theme";
import { selectionColor } from "../colors";
import { MAP_SIZE, cellOrigin, cellY } from "../layout";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";

/** How wide the frame's ring is, in scene units (half a tile); its inner edge lies on the cell's border. */
export const FRAME_WIDTH = 0.5;

/**
 * The flat ring around matrix cell (`col`, `row`): four thin quads, `FRAME_WIDTH` wide, whose inner edge is the cell's border, just over
 * the cell's overlay plane. It is depth-tested (buildings cover it) and drawn after the tiles.
 */
export function cellFrame(matrix: MatrixDto, col: number, row: number, color: THREE.ColorRepresentation): THREE.Mesh {
  const { x, z } = cellOrigin(col, row);
  const y = cellY(matrix, col, row) + OVERLAY_Y * 2;
  const w = FRAME_WIDTH;
  const x0 = x - w;
  const x1 = x + MAP_SIZE + w;
  const z0 = z - w;
  const z1 = z + MAP_SIZE + w;
  // Each side a rectangle [xa, xb] × [za, zb]: north and south span the corners, west and east fill in between them.
  const sides = [
    [x0, x1, z0, z],
    [x0, x1, z + MAP_SIZE, z1],
    [x0, x, z, z + MAP_SIZE],
    [x + MAP_SIZE, x1, z, z + MAP_SIZE],
  ];
  const positions: number[] = [];
  const index: number[] = [];
  for (const [xa, xb, za, zb] of sides) {
    const i = positions.length / 3;
    positions.push(xa, y, za, xb, y, za, xb, y, zb, xa, y, zb);
    // Counter-clockwise seen from above, so the quads face up.
    index.push(i, i + 3, i + 2, i, i + 2, i + 1);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(index);
  const frame = new THREE.Mesh(geometry, new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide, depthWrite: false }));
  // After the tiles (renderOrder 10), so it lies over their overlay.
  frame.renderOrder = 11;
  return frame;
}

/**
 * Frames the selected cell (`h.selectedCell`) of the shown matrix in the selection colour of `theme`. No frame without either, outside the
 * matrix, or over a cell whose map the scene has not loaded (an empty cell, or one outside the loaded area of a masked matrix).
 */
export function showCellFrame(h: SceneHost, theme: "hell" | "dunkel" = effectiveTheme()) {
  const cell = h.selectedCell;
  const m = h.matrix;
  const map = cell && m && cell.x < m.width && cell.y < m.height ? m.maps[cell.y * m.width + cell.x] : null;
  h.setCellFrame(cell && m && map != null && h.loaded.has(map) ? cellFrame(m, cell.x, cell.y, selectionColor(theme)) : null);
}
