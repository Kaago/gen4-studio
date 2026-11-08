import * as THREE from "three";
import type { EventsDto, MatrixDto, TileBehavior } from "../../api";
import { BuildingLayer } from "../buildingLayer";
import { permissionColor } from "../colors";
import { createGround } from "../ground";
import type { Ground } from "../ground";
import { MAP_SIZE, MODEL_SCALE, cellOrigin, cellY } from "../layout";
import { MARKER_Y, applySprite, buildMarkers, disposeMarkers, markFaulty, moveMarker, selectMarker } from "../markers";
import type { MarkerSet } from "../markers";
import { cloneModel, disposeTree } from "../sceneUtil";
import { peekSprite, spriteTexture } from "../sprites";
import { showHeightLayer } from "./heightLayer";
import { countPatterns, patternOf, setPatternAt, withPatterns } from "./patterns";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

/** Height of the overlay plate of a tile over its ground (`Ground.y`); clicks are intersected with the same plates. */
export const OVERLAY_Y = 0.05;

/** Opacity of the overlay fill (spec §8.1: 42 %). */
export const OVERLAY_OPACITY = 0.42;

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
    const col = i % matrix.width;
    const row = Math.floor(i / matrix.width);
    const origin = cellOrigin(col, row);
    const instance = cloneModel(model);
    instance.userData.mapId = mapId;
    // The model is centred on the map centre.
    instance.scale.setScalar(MODEL_SCALE);
    instance.position.set(origin.x + MAP_SIZE / 2, cellY(matrix, col, row), origin.z + MAP_SIZE / 2);
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

/** Takes every terrain instance of map `mapId` out of the scene and disposes it; the other maps' instances stay. */
export function removeMapModel(h: SceneHost, mapId: number) {
  const kept: THREE.Object3D[] = [];
  for (const instance of h.models) {
    if (instance.userData.mapId !== mapId) {
      kept.push(instance);
      continue;
    }
    h.content.remove(instance);
    disposeTree(instance);
  }
  h.models = kept;
}

/** Builds the overlay (its tiles on `ground`) and the building boxes once all maps are known; models are added later. */
export function buildContent(matrix: MatrixDto, loaded: Map<number, Loaded>, names: Map<number, TileBehavior>, ground: Ground = createGround(matrix, loaded)) {
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
    opacity: OVERLAY_OPACITY,
    depthTest: false,
    depthWrite: false,
    // The token colours must come out as they are, not through tone mapping.
    toneMapped: false,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
  const tiles = new THREE.InstancedMesh(plane, planeMaterial, cells.length * perCell);
  tiles.renderOrder = 10;
  tiles.frustumCulled = false;
  // Each tile carries its terrain pattern (rule „Gelände-Muster“); a tile without one looks as before.
  withPatterns(tiles);
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
      const x = origin.x + (i % MAP_SIZE);
      const z = origin.z + Math.floor(i / MAP_SIZE);
      m4.setPosition(x + 0.5, OVERLAY_Y + ground.y(x, z), z + 0.5);
      tiles.setMatrixAt(n, m4);
      tiles.setColorAt(n, overlayColor(kind, collision, names));
      setPatternAt(tiles, n, patternOf(kind, collision, names.get(kind)));
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

/** Replaces the scene's markers (`events` null: show none); the selected event keeps its outline, a faulty one its error colour. */
export function setMarkers(h: SceneHost, events: EventsDto | null, visible: boolean) {
  if (h.markers) {
    h.scene.remove(h.markers.group);
    disposeMarkers(h.markers);
    h.markers = null;
  }
  h.events = events;
  if (!h.matrix || !events) {
    reportSelectedMarker(h);
    return;
  }
  h.markers = buildMarkers(h.matrix, events, h.ground);
  h.markers.group.visible = visible;
  const selected = h.selectedEvent;
  if (selected && selected.file === events.event_file_id) selectMarker(h.markers, selected);
  reportSelectedMarker(h);
  markFaulty(h.markers, h.faulty.filter((ref) => ref.file === events.event_file_id));
  h.scene.add(h.markers.group);
  loadSprites(h, h.markers, events);
}

/**
 * Gives the object markers of `set` their sprites. Sprites that are already loaded are applied at once; the others
 * are asked for (once per graphics id and facing, see `spriteTexture`) and drawn when they arrive. A result for a
 * set that has been replaced meanwhile is dropped (the new set asks for itself); an object without a sprite keeps
 * its placeholder square.
 */
function loadSprites(h: SceneHost, set: MarkerSet, events: EventsDto) {
  for (const object of events.objects) {
    const ready = peekSprite(object.graphics_id, object.dir);
    if (ready !== undefined) {
      if (ready) applySprite(set, object.index, ready);
      continue;
    }
    void spriteTexture(object.graphics_id, object.dir).then((look) => {
      if (!look || h.markers !== set) return;
      if (applySprite(set, object.index, look)) h.queueRender();
    });
  }
}

/** Recolours every overlay instance of `mapId` from the map's current permissions, its pattern with it; the canvas counts the patterns anew. */
export function recolourMap(h: SceneHost, mapId: number) {
  const data = h.loaded.get(mapId);
  const bases = h.bases.get(mapId);
  const tiles = h.tiles;
  if (!data || !bases || !tiles) return;
  for (const base of bases) {
    data.map.permissions.forEach(([kind, collision], i) => {
      tiles.setColorAt(base + i, overlayColor(kind, collision, h.names));
      setPatternAt(tiles, base + i, patternOf(kind, collision, h.names.get(kind)));
    });
  }
  if (tiles.instanceColor) tiles.instanceColor.needsUpdate = true;
  tiles.geometry.getAttribute("aPattern").needsUpdate = true;
  reportTerrainPatterns(h);
}

/**
 * Says on the canvas how many overlay cells carry a terrain pattern (`data-terrain-patterns`) while the layer „Gelände“ is visible;
 * hidden (or nothing loaded yet): the attribute goes.
 */
export function reportTerrainPatterns(h: SceneHost) {
  if (h.tiles && h.overlay?.visible) h.canvas.dataset.terrainPatterns = String(countPatterns(h.tiles));
  else delete h.canvas.dataset.terrainPatterns;
}

/**
 * Puts what stands on the ground of map `mapId` onto `h.ground` again (its heights changed: `set_map_terrain`, its undo): the overlay
 * plates of every cell showing the map and the markers on its tiles; the other maps stay as they are. A layer „Höhen“ that was built is
 * built anew on `h.ground` (its isolines run across map borders), shown or hidden as before; one never switched on stays unbuilt. Does
 * not render.
 */
export function moveMapGround(h: SceneHost, mapId: number) {
  const matrix = h.matrix;
  if (!matrix) return;
  const tiles = h.tiles;
  if (tiles) {
    // Each plate keeps its place and takes the height of the ground of its tile.
    const m4 = new THREE.Matrix4();
    const at = new THREE.Vector3();
    for (const base of h.bases.get(mapId) ?? []) {
      for (let i = base; i < base + MAP_SIZE * MAP_SIZE; i++) {
        tiles.getMatrixAt(i, m4);
        at.setFromMatrixPosition(m4);
        m4.setPosition(at.x, OVERLAY_Y + h.ground.y(Math.floor(at.x), Math.floor(at.z)), at.z);
        tiles.setMatrixAt(i, m4);
      }
    }
    tiles.instanceMatrix.needsUpdate = true;
  }
  const set = h.markers;
  if (set) {
    set.ground = h.ground;
    const onMap = (x: number, z: number) => {
      const col = Math.floor(x / MAP_SIZE);
      const row = Math.floor(z / MAP_SIZE);
      return col >= 0 && col < matrix.width && row >= 0 && row < matrix.height && matrix.maps[row * matrix.width + col] === mapId;
    };
    for (const { kind, data } of set.entries.values()) {
      for (const event of data) if (onMap(event.x, event.z)) moveMarker(set, { kind, index: event.index }, { x: event.x, z: event.z });
    }
  }
  if (h.heights) showHeightLayer(h, h.heights.visible);
  reportSelectedMarker(h);
}

/** Says on the canvas how high the selected marker stands (`data-selected-marker-y`: `MARKER_Y` over its ground, 3 decimals); none selected: the attribute goes. */
export function reportSelectedMarker(h: SceneHost) {
  const set = h.markers;
  const ref = set?.selected ?? null;
  // Each object is an entry of its own: look through every entry of the kind.
  const found = set && ref ? [...set.entries.values()].flatMap((e) => (e.kind === ref.kind ? e.data : [])).find((d) => d.index === ref.index) : undefined;
  if (!set || !found) delete h.canvas.dataset.selectedMarkerY;
  else h.canvas.dataset.selectedMarkerY = (MARKER_Y + set.ground.y(found.x, found.z)).toFixed(3);
}
