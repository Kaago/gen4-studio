import type { MatrixDto } from "../../api";
import type { CellRect } from "../../workbench/sceneScope";
import { MAP_SIZE, cellY } from "../layout";
import type { SceneHost } from "./SceneHost";

/**
 * Puts the camera above and south of the cells `focus` of `matrix`, looking down on their middle at the height of the middle cell,
 * as far off as the longer side of the rectangle asks for; the far plane is moved out when a wide rectangle needs it. Renders.
 */
export function frameCells(host: SceneHost, focus: CellRect, matrix: MatrixDto): void {
  const w = focus.width * MAP_SIZE;
  const d = focus.height * MAP_SIZE;
  const span = Math.max(w, d);
  const x = focus.x * MAP_SIZE + w / 2;
  const z = focus.y * MAP_SIZE + d / 2;
  const y = cellY(matrix, focus.x + Math.floor(focus.width / 2), focus.y + Math.floor(focus.height / 2));
  if (host.camera.far < span * 4) {
    host.camera.far = span * 4;
    host.camera.updateProjectionMatrix();
  }
  host.controls.target.set(x, y, z);
  host.camera.position.set(x, y + span * 0.9, z + span * 0.7);
  host.controls.update();
  host.render();
}
