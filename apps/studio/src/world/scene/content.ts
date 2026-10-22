import * as THREE from "three";
import type { EventsDto, MatrixDto, TileBehavior } from "../../api";
import { BuildingLayer } from "../buildingLayer";
import { permissionColor } from "../colors";
import { MAP_SIZE, MODEL_SCALE, cellOrigin } from "../layout";
import { buildMarkers, disposeMarkers, selectMarker } from "../markers";
import { cloneModel, disposeTree } from "../sceneUtil";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

/** Height of the overlay plane; clicks are intersected with the same plane. */
export const OVERLAY_Y = 0.05;

const colorCache = new Map<string, THREE.Color>();
export function overlayColor(kind: number, collision: number, names: Map<number, TileBehavior>): THREE.Color {
  const hex = permissionColor(kind, collision, names.get(kind)?.class ?? null);
  let color = colorCache.get(hex);
  if (!color) colorCache.set(hex, (color = new THREE.Color(hex)));
  return color;
}

/** Hands the model of map `mapId` to every matrix cell showing that map (clones share geometry and materials). */
export function addModel(h: SceneHost, matrix: MatrixDto, mapId: number, model: THREE.Object3D) {
  let used = false;
  matrix.maps.forEach((id, i) => {
    if (id !== mapId) return;
    const origin = cellOrigin(i % matrix.width, Math.floor(i / matrix.width));
    const instance = cloneModel(model);
    // The model is centred on the map centre.
    instance.scale.setScalar(MODEL_SCALE);
    instance.position.set(origin.x + MAP_SIZE / 2, 0, origin.z + MAP_SIZE / 2);
    h.content.add(instance);
    h.models.push(instance);
    used = true;
  });
  // The clones in `content` are disposed with it; a model without a cell would leak.
  if (!used) disposeTree(model);
}

/** Takes every terrain model out of the scene (the overlay, buildings and markers stay). */
export function removeModels(h: SceneHost) {
  for (const instance of h.models) {
    h.content.remove(instance);
    disposeTree(instance);
  }
  h.models = [];
}

/** Builds the overlay and the building boxes once all maps are known; models are added later. */
export function buildContent(matrix: MatrixDto, loaded: Map<number, Loaded>, names: Map<number, TileBehavior>) {
  const content = new THREE.Group();
  const cells: { col: number; row: number; data: Loaded }[] = [];
  matrix.maps.forEach((id, i) => {
    const data = id === null ? undefined : loaded.get(id);
    if (data) cells.push({ col: i % matrix.width, row: Math.floor(i / matrix.width), data });
  });

  const overlay = new THREE.Group();
  const perCell = MAP_SIZE * MAP_SIZE;
  const plane = new THREE.PlaneGeometry(1, 1);
  plane.rotateX(-Math.PI / 2);
  const planeMaterial = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: 0.45,
    depthTest: false,
    depthWrite: false,
  });
  const tiles = new THREE.InstancedMesh(plane, planeMaterial, cells.length * perCell);
  tiles.renderOrder = 10;
  tiles.frustumCulled = false;
  const m4 = new THREE.Matrix4();
  const bases = new Map<number, number[]>();
  let n = 0;
  for (const { col, row, data } of cells) {
    const origin = cellOrigin(col, row);
    const list = bases.get(data.map.index) ?? [];
    list.push(n);
    bases.set(data.map.index, list);
    for (let i = 0; i < perCell; i++) {
      const [kind, collision] = data.map.permissions[i] ?? [0, 0];
      m4.setPosition(origin.x + (i % MAP_SIZE) + 0.5, OVERLAY_Y, origin.z + Math.floor(i / MAP_SIZE) + 0.5);
      tiles.setMatrixAt(n, m4);
      tiles.setColorAt(n, overlayColor(kind, collision, names));
      n++;
    }
  }
  overlay.add(tiles);
  content.add(overlay);

  const buildings = new BuildingLayer();
  buildings.build(matrix, loaded);
  content.add(buildings.root);
  return { content, overlay, tiles, bases, buildings };
}

/** Replaces the scene's markers (`events` null: show none); the selected event keeps its outline. */
export function setMarkers(h: SceneHost, events: EventsDto | null, visible: boolean) {
  if (h.markers) {
    h.scene.remove(h.markers.group);
    disposeMarkers(h.markers);
    h.markers = null;
  }
  h.events = events;
  if (!h.matrix || !events) return;
  h.markers = buildMarkers(h.matrix, events);
  h.markers.group.visible = visible;
  const selected = h.selectedEvent;
  if (selected && selected.file === events.event_file_id) selectMarker(h.markers, selected);
  h.scene.add(h.markers.group);
}

/** Recolours every overlay instance of `mapId` from the map's current permissions. */
export function recolourMap(h: SceneHost, mapId: number) {
  const data = h.loaded.get(mapId);
  const bases = h.bases.get(mapId);
  if (!data || !bases || !h.tiles) return;
  for (const base of bases) {
    data.map.permissions.forEach(([kind, collision], i) => {
      h.tiles?.setColorAt(base + i, overlayColor(kind, collision, h.names));
    });
  }
  if (h.tiles.instanceColor) h.tiles.instanceColor.needsUpdate = true;
}
