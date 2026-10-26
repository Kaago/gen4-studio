import * as THREE from "three";
import type { MatrixDto } from "../../api";
import { effectiveTheme } from "../../ui/tokens/theme";
import { selectionColor } from "../colors";
import { MAP_SIZE, cellOrigin, cellY } from "../layout";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";

/** The square around matrix cell (`col`, `row`), one map wide, just over the cell's overlay plane; drawn over everything. */
export function cellFrame(matrix: MatrixDto, col: number, row: number, color: THREE.ColorRepresentation): THREE.LineLoop {
  const { x, z } = cellOrigin(col, row);
  const y = cellY(matrix, col, row) + OVERLAY_Y * 2;
  const corners = [x, y, z, x + MAP_SIZE, y, z, x + MAP_SIZE, y, z + MAP_SIZE, x, y, z + MAP_SIZE];
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(corners, 3));
  const frame = new THREE.LineLoop(geometry, new THREE.LineBasicMaterial({ color, depthTest: false }));
  frame.renderOrder = 10;
  return frame;
}

/** Frames the selected cell (`h.selectedCell`) of the shown matrix in the selection colour of `theme`; no frame without either, or outside the matrix. */
export function showCellFrame(h: SceneHost, theme: "hell" | "dunkel" = effectiveTheme()) {
  const cell = h.selectedCell;
  const m = h.matrix;
  h.setCellFrame(cell && m && cell.x < m.width && cell.y < m.height ? cellFrame(m, cell.x, cell.y, selectionColor(theme)) : null);
}
