import * as THREE from "three";
import { ALTITUDE_STEP, cellOrigin } from "../layout";
import { pickMarker } from "../markers";
import type { MarkerHit } from "../markers";
import { matrixCellAt, pickCell } from "../paint";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

/** The distinct altitudes of a matrix, highest first (`[0]` without an altitude section). */
export function altitudeLevels(altitudes: readonly number[] | null | undefined): number[] {
  return [...new Set(altitudes?.length ? altitudes : [0])].sort((a, b) => b - a);
}

/** The tile under a pointer position. */
export interface PickedTile {
  mapId: number;
  data: Loaded;
  col: number;
  row: number;
  x: number;
  y: number;
}

/** What a pointer position hits in the scene; all of it shares one raycaster. */
export interface Picker {
  /** The raycaster the last `aim` (or `markerAt`, `pickTile`, `planePoint`) pointed along the view ray. */
  readonly raycaster: THREE.Raycaster;
  /** Aims the shared raycaster along the view ray through a pointer position. */
  aim(clientX: number, clientY: number): void;
  /** The nearest event marker under a pointer position, or null (also while markers are hidden). */
  markerAt(clientX: number, clientY: number): MarkerHit | null;
  /** The tile under a pointer position, or null outside every loaded map cell. */
  pickTile(clientX: number, clientY: number): PickedTile | null;
  /** The point of `plane` under a pointer position, or null when the view ray never reaches it. */
  planePoint(clientX: number, clientY: number, plane: THREE.Plane): { x: number; z: number } | null;
}

export function createPicker(h: SceneHost): Picker {
  const canvas = h.canvas;
  const raycaster = new THREE.Raycaster();
  // The picking planes of the shown matrix, highest first; rebuilt only when the altitudes change.
  let levelsOf: readonly number[] | null | undefined;
  let levels: { level: number; plane: THREE.Plane }[] = [];
  const levelPlanes = (altitudes: readonly number[] | null | undefined) => {
    if (!levels.length || altitudes !== levelsOf) {
      levelsOf = altitudes;
      levels = altitudeLevels(altitudes).map((level) => ({ level, plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -(OVERLAY_Y + level * ALTITUDE_STEP)) }));
    }
    return levels;
  };
  const aim = (clientX: number, clientY: number) => {
    const rect = canvas.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
    );
    raycaster.setFromCamera(ndc, h.camera);
  };
  const markerAt = (clientX: number, clientY: number) => {
    if (!h.markers?.group.visible) return null;
    aim(clientX, clientY);
    return pickMarker(h.markers, raycaster);
  };
  const pickTile = (clientX: number, clientY: number) => {
    if (!h.matrix) return null;
    aim(clientX, clientY);
    const m = h.matrix;
    const hit = new THREE.Vector3();
    // From the highest plane down: the first plane that is hit inside a cell of exactly its height is the surface under the pointer.
    for (const { level, plane } of levelPlanes(m.altitudes)) {
      if (!raycaster.ray.intersectPlane(plane, hit)) continue;
      const at = matrixCellAt(hit, m.width, m.height);
      if (!at) continue;
      const index = at.row * m.width + at.col;
      if ((m.altitudes?.[index] ?? 0) !== level) continue;
      const mapId = m.maps[index];
      const tile = pickCell(hit, cellOrigin(at.col, at.row));
      const data = mapId === null || mapId === undefined ? undefined : h.loaded.get(mapId);
      if (!tile || mapId === null || mapId === undefined || !data) continue;
      return { mapId, data, col: at.col, row: at.row, x: tile.x, y: tile.y };
    }
    return null;
  };
  const planePoint = (clientX: number, clientY: number, onPlane: THREE.Plane) => {
    aim(clientX, clientY);
    const hit = raycaster.ray.intersectPlane(onPlane, new THREE.Vector3());
    return hit ? { x: hit.x, z: hit.z } : null;
  };
  return { raycaster, aim, markerAt, pickTile, planePoint };
}
