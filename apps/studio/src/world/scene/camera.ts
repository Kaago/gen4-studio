import * as THREE from "three";
import type { MatrixDto } from "../../api";
import type { CellRect } from "../../workbench/sceneScope";
import { MAP_SIZE, cellY } from "../layout";
import type { SceneHost } from "./SceneHost";

/** How much room the top-down view leaves around its focus, as a share of the focus on its tighter side. */
const TOP_DOWN_MARGIN = 1.1;

/** The scene height of the middle cell of `focus` in `matrix`: the height both views look at. */
export function focusHeight(focus: CellRect, matrix: MatrixDto): number {
  return cellY(matrix, focus.x + Math.floor(focus.width / 2), focus.y + Math.floor(focus.height / 2));
}

/**
 * What an orthographic camera straight above the middle of the cells `focus` shows in a panel of `aspect` (width by height): a frame
 * symmetric around the camera, in the panel's shape (no distortion), holding the whole focus with a small margin on its tighter side.
 * `target` is the middle of the focus at height 0.
 */
export function topDownFrame(focus: CellRect, aspect: number): { left: number; right: number; top: number; bottom: number; target: THREE.Vector3 } {
  const w = focus.width * MAP_SIZE;
  const d = focus.height * MAP_SIZE;
  const halfD = (Math.max(d, w / aspect) * TOP_DOWN_MARGIN) / 2;
  const halfW = halfD * aspect;
  return { left: -halfW, right: halfW, top: halfD, bottom: -halfD, target: new THREE.Vector3(focus.x * MAP_SIZE + w / 2, 0, focus.y * MAP_SIZE + d / 2) };
}

/**
 * Puts the slanted camera above and south of the cells `focus` of `matrix`, looking down on their middle at the height of the middle cell,
 * as far off as the longer side of the rectangle asks for; the far plane is moved out when a wide rectangle needs it. While the top-down
 * view is on, it frames `focus` there and the slanted camera waits at that place for the way back. Renders.
 */
export function frameCells(host: SceneHost, focus: CellRect, matrix: MatrixDto): void {
  const w = focus.width * MAP_SIZE;
  const d = focus.height * MAP_SIZE;
  const span = Math.max(w, d);
  const x = focus.x * MAP_SIZE + w / 2;
  const z = focus.y * MAP_SIZE + d / 2;
  const y = focusHeight(focus, matrix);
  const camera = host.slanted;
  if (camera.far < span * 4) {
    camera.far = span * 4;
    camera.updateProjectionMatrix();
  }
  camera.position.set(x, y + span * 0.9, z + span * 0.7);
  if (host.topDown) {
    host.slantedTarget.set(x, y, z);
    host.frameTopDown(focus, matrix);
    return;
  }
  host.controls.target.set(x, y, z);
  host.controls.update();
  host.render();
}
