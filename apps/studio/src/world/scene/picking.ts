import * as THREE from "three";
import type { Ground } from "../ground";
import { MAP_SIZE, cellOrigin } from "../layout";
import { pickMarker } from "../markers";
import type { MarkerHit } from "../markers";
import { matrixCellAt, pickCell } from "../paint";
import { OVERLAY_Y } from "./content";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

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
  /** The scene point of the surface under a pointer position (the plate of the tile it is over, on the ground), or null over no matrix cell. */
  surfacePoint(clientX: number, clientY: number): { x: number; z: number } | null;
  /** The point of `plane` under a pointer position, or null when the view ray never reaches it. */
  planePoint(clientX: number, clientY: number, plane: THREE.Plane): { x: number; z: number } | null;
}

export function createPicker(h: SceneHost): Picker {
  const canvas = h.canvas;
  const raycaster = new THREE.Raycaster();
  // The picking planes of the shown ground, highest first; rebuilt only when the scene has another ground.
  let levelsOf: Ground | null = null;
  let levels: { level: number; plane: THREE.Plane }[] = [];
  const levelPlanes = (ground: Ground) => {
    if (ground !== levelsOf) {
      levelsOf = ground;
      levels = ground.levels().map((level) => ({ level, plane: new THREE.Plane(new THREE.Vector3(0, 1, 0), -(OVERLAY_Y + level)) }));
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
  /**
   * The point of the surface under the pointer: planes from the highest ground down, the first one hit
   * on a tile of exactly its height (and whose cell index `accept`s) wins; overlay and picking share these plates.
   */
  const surfaceHit = (clientX: number, clientY: number, accept: (index: number) => boolean) => {
    const m = h.matrix;
    if (!m) return null;
    aim(clientX, clientY);
    const hit = new THREE.Vector3();
    for (const { level, plane } of levelPlanes(h.ground)) {
      if (!raycaster.ray.intersectPlane(plane, hit)) continue;
      const at = matrixCellAt(hit, m.width, m.height);
      const tile = at && pickCell(hit, cellOrigin(at.col, at.row));
      if (!at || !tile) continue;
      const index = at.row * m.width + at.col;
      if (Math.abs(h.ground.y(at.col * MAP_SIZE + tile.x, at.row * MAP_SIZE + tile.y) - level) >= 1e-6 || !accept(index)) continue;
      return { hit, at, index, tile };
    }
    return null;
  };
  const surfacePoint = (clientX: number, clientY: number) => {
    const found = surfaceHit(clientX, clientY, () => true);
    return found ? { x: found.hit.x, z: found.hit.z } : null;
  };
  const pickTile = (clientX: number, clientY: number) => {
    const m = h.matrix;
    if (!m) return null;
    const found = surfaceHit(clientX, clientY, (index) => {
      const id = m.maps[index];
      return id !== null && id !== undefined && h.loaded.has(id);
    });
    if (!found) return null;
    const mapId = m.maps[found.index] as number;
    const data = h.loaded.get(mapId);
    if (!data) return null;
    return { mapId, data, col: found.at.col, row: found.at.row, x: found.tile.x, y: found.tile.y };
  };
  const planePoint = (clientX: number, clientY: number, onPlane: THREE.Plane) => {
    aim(clientX, clientY);
    const hit = raycaster.ray.intersectPlane(onPlane, new THREE.Vector3());
    return hit ? { x: hit.x, z: hit.z } : null;
  };
  return { raycaster, aim, markerAt, pickTile, surfacePoint, planePoint };
}
