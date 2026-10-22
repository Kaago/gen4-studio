import * as THREE from "three";
import { cellOrigin } from "../layout";
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
  /** The point of `plane` under a pointer position, or null when the view ray never reaches it. */
  planePoint(clientX: number, clientY: number, plane: THREE.Plane): { x: number; z: number } | null;
}

export function createPicker(h: SceneHost): Picker {
  const canvas = h.canvas;
  const raycaster = new THREE.Raycaster();
  const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -OVERLAY_Y);
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
    const hit = raycaster.ray.intersectPlane(plane, new THREE.Vector3());
    if (!hit) return null;
    const m = h.matrix;
    const at = matrixCellAt(hit, m.width, m.height);
    if (!at) return null;
    const mapId = m.maps[at.row * m.width + at.col];
    const tile = mapId === null || mapId === undefined ? null : pickCell(hit, cellOrigin(at.col, at.row));
    if (mapId === null || mapId === undefined || !tile) return null;
    const data = h.loaded.get(mapId);
    if (!data) return null;
    return { mapId, data, col: at.col, row: at.row, x: tile.x, y: tile.y };
  };
  const planePoint = (clientX: number, clientY: number, onPlane: THREE.Plane) => {
    aim(clientX, clientY);
    const hit = raycaster.ray.intersectPlane(onPlane, new THREE.Vector3());
    return hit ? { x: hit.x, z: hit.z } : null;
  };
  return { raycaster, aim, markerAt, pickTile, planePoint };
}
