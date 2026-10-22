import type * as THREE from "three";
import type { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { EventsDto, MatrixDto, TileBehavior } from "../../api";
import type { BuildingLayer, BuildingRef } from "../buildingLayer";
import type { EventRef } from "../events";
import type { MarkerSet } from "../markers";
import type { Loaded } from "./types";

export interface SceneHandle {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  content: THREE.Group;
  overlay: THREE.Object3D | null;
  render: () => void;
  /** Matrix and loaded maps behind the current content (null until a matrix is loaded). */
  matrix: MatrixDto | null;
  /** The terrain model instances in `content` (one per matrix cell showing a map), for reloading them alone. */
  models: THREE.Object3D[];
  /** What the last full load could not read (names, maps), kept for the summary after a models-only reload. */
  loadProblems: { names: string | null; maps: string[] };
  loaded: Map<number, Loaded>;
  tiles: THREE.InstancedMesh | null;
  /** Map id to the first overlay instance of each matrix cell showing that map. */
  bases: Map<number, number[]>;
  /** Tile behaviour names by kind byte (empty until loaded). */
  names: Map<number, TileBehavior>;
  markers: MarkerSet | null;
  /** The events the markers show (null: none shown, or they could not be read). */
  events: EventsDto | null;
  /** The selected event, as the scene knows it (follows the `selectedEvent` prop). */
  selectedEvent: EventRef | null;
  /** The buildings of the shown matrix (a child of `content`; null until a matrix is loaded). */
  buildings: BuildingLayer | null;
  /** The selected building, as the scene knows it (follows the `selectedMap`/`selectedBuilding` props). */
  selection: BuildingRef | null;
  /** Re-resolves the tile under the pointer on the next frame (set by the setup effect). */
  refreshHover: (() => void) | null;
  /** A render is already queued for the next animation frame. */
  renderQueued: boolean;
  /** The component is gone; late results and queued renders are dropped. */
  disposed: boolean;
}

/** Renders on the next animation frame; bursts of arriving models cost one render. */
export function queueRender(h: SceneHandle) {
  if (h.renderQueued) return;
  h.renderQueued = true;
  requestAnimationFrame(() => {
    h.renderQueued = false;
    if (!h.disposed) h.render();
  });
}
