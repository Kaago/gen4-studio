import * as THREE from "three";
import type { MatrixDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import type { Ground } from "../ground";
import { MAP_SIZE, MODEL_UNITS_PER_TILE, cellOrigin } from "../layout";
import { disposeTree } from "../sceneUtil";
import { OVERLAY_OPACITY } from "./content";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

/** Height of the layer's plates over their ground (`Ground.y`), just over the overlay (`OVERLAY_Y`); the isolines lie on the same height. */
export const HEIGHT_LAYER_Y = 0.06;

/** World units per height step (half a tile, one altitude step of the matrix). */
const UNITS_PER_STEP = 8;

/** The design system's cividis steps; the eighth step starts over at the first. */
const STEPS = 7;

/** The height step of a height in world units: one step per half tile, rounded. */
export function heightStep(units: number): number {
  return Math.round(units / UNITS_PER_STEP);
}

/** The colour of a height in world units: `szene.hoehe` of its step, cycling through the seven (below 0 backwards). */
export function heightColor(units: number): number {
  const index = ((heightStep(units) % STEPS) + STEPS) % STEPS;
  return szene.hoehe[String(index) as keyof typeof szene.hoehe];
}

/** The scene height of the walking surface at tile (x, z) in world units, the cell's altitude included (what the layer's steps count). */
const unitsAt = (ground: Ground, x: number, z: number) => ground.y(x, z) * MODEL_UNITS_PER_TILE;

/** The tiles the layer shows: those of matrix cells whose map is loaded. */
function shownTile(matrix: MatrixDto, loaded: ReadonlyMap<number, Loaded>) {
  return (x: number, z: number) => {
    const col = Math.floor(x / MAP_SIZE);
    const row = Math.floor(z / MAP_SIZE);
    if (col < 0 || row < 0 || col >= matrix.width || row >= matrix.height) return false;
    const id = matrix.maps[row * matrix.width + col];
    return id !== null && id !== undefined && loaded.has(id);
  };
}

/** The matrix cells whose map is loaded, in matrix order. */
function shownCells(matrix: MatrixDto, loaded: ReadonlyMap<number, Loaded>) {
  const cells: { col: number; row: number }[] = [];
  matrix.maps.forEach((id, i) => {
    if (id !== null && loaded.has(id)) cells.push({ col: i % matrix.width, row: Math.floor(i / matrix.width) });
  });
  return cells;
}

/**
 * The isolines of the shown tiles: every edge between two shown tiles of different steps (across cell borders too), as one pair of
 * xyz points per edge, on the height of the higher of the two tiles (plus `HEIGHT_LAYER_Y`). Edges towards an empty cell, a map that is
 * not loaded or beside the matrix have no line.
 */
export function isolineSegments(matrix: MatrixDto, loaded: ReadonlyMap<number, Loaded>, ground: Ground): Float32Array {
  const shown = shownTile(matrix, loaded);
  const out: number[] = [];
  const edge = (x: number, z: number, nx: number, nz: number, from: [number, number], to: [number, number]) => {
    if (!shown(nx, nz)) return;
    const here = unitsAt(ground, x, z);
    const there = unitsAt(ground, nx, nz);
    if (heightStep(here) === heightStep(there)) return;
    const y = HEIGHT_LAYER_Y + Math.max(here, there) / MODEL_UNITS_PER_TILE;
    out.push(from[0], y, from[1], to[0], y, to[1]);
  };
  for (const { col, row } of shownCells(matrix, loaded)) {
    const origin = cellOrigin(col, row);
    for (let i = 0; i < MAP_SIZE * MAP_SIZE; i++) {
      const x = origin.x + (i % MAP_SIZE);
      const z = origin.z + Math.floor(i / MAP_SIZE);
      // Each edge once: the east and the south edge of every tile.
      edge(x, z, x + 1, z, [x + 1, z], [x + 1, z + 1]);
      edge(x, z, x, z + 1, [x, z + 1], [x + 1, z + 1]);
    }
  }
  return new Float32Array(out);
}

const colorCache = new Map<number, THREE.Color>();
function stepColor(units: number): THREE.Color {
  const rgb = heightColor(units);
  let color = colorCache.get(rgb);
  if (!color) colorCache.set(rgb, (color = new THREE.Color(rgb)));
  return color;
}

/**
 * The layer „Höhen“ of the shown tiles: `tiles`, a plate per tile on its ground in the colour of its step (with the overlay's opacity),
 * and `isolines`, the lines of the step changes in the first height colour. Both draw over the terrain models like the overlay.
 */
export function buildHeightLayer(matrix: MatrixDto, loaded: ReadonlyMap<number, Loaded>, ground: Ground): THREE.Group {
  const layer = new THREE.Group();
  layer.name = "heights";
  const cells = shownCells(matrix, loaded);
  const plane = new THREE.PlaneGeometry(1, 1);
  plane.rotateX(-Math.PI / 2);
  const plateMaterial = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: OVERLAY_OPACITY,
    depthTest: false,
    depthWrite: false,
    // The token colours must come out as they are, not through tone mapping.
    toneMapped: false,
  });
  const tiles = new THREE.InstancedMesh(plane, plateMaterial, cells.length * MAP_SIZE * MAP_SIZE);
  tiles.name = "tiles";
  tiles.renderOrder = 10;
  tiles.frustumCulled = false;
  const m4 = new THREE.Matrix4();
  let n = 0;
  for (const { col, row } of cells) {
    const origin = cellOrigin(col, row);
    for (let i = 0; i < MAP_SIZE * MAP_SIZE; i++) {
      const x = origin.x + (i % MAP_SIZE);
      const z = origin.z + Math.floor(i / MAP_SIZE);
      m4.setPosition(x + 0.5, HEIGHT_LAYER_Y + ground.y(x, z), z + 0.5);
      tiles.setMatrixAt(n, m4);
      tiles.setColorAt(n, stepColor(unitsAt(ground, x, z)));
      n++;
    }
  }
  layer.add(tiles);

  const lines = new THREE.BufferGeometry();
  lines.setAttribute("position", new THREE.BufferAttribute(isolineSegments(matrix, loaded, ground), 3));
  const lineMaterial = new THREE.LineBasicMaterial({ color: szene.hoehe["0"], depthTest: false, depthWrite: false, toneMapped: false });
  const isolines = new THREE.LineSegments(lines, lineMaterial);
  isolines.name = "isolines";
  isolines.renderOrder = 11;
  isolines.frustumCulled = false;
  layer.add(isolines);
  return layer;
}

/** Says on the canvas whether the layer „Höhen“ is shown (`data-heights-shown`; hidden or not built: the attribute goes). */
export function reportHeights(h: SceneHost) {
  if (h.heights?.visible) h.canvas.dataset.heightsShown = "true";
  else delete h.canvas.dataset.heightsShown;
}

/**
 * Builds the layer „Höhen“ of `h.matrix` on `h.ground` from the maps loaded now and puts it into `h.content` in place of the one before
 * (which is freed), shown or hidden as `visible` says. Does not render.
 */
export function showHeightLayer(h: SceneHost, visible: boolean) {
  const old = h.heights;
  if (old) {
    h.content.remove(old);
    disposeTree(old);
  }
  h.heights = null;
  if (h.matrix) {
    const layer = buildHeightLayer(h.matrix, h.loaded, h.ground);
    layer.visible = visible;
    h.content.add(layer);
    h.heights = layer;
  }
  reportHeights(h);
}

/**
 * Shows or hides the layer „Höhen“ of the shown matrix. It is built lazily: the first switch-on builds it (the scene opens without it),
 * later switches only show or hide it; without a matrix nothing is built. Reports `data-heights-shown`. Does not render.
 */
export function setHeightsShown(h: SceneHost, on: boolean) {
  if (h.heights) {
    h.heights.visible = on;
    reportHeights(h);
  } else if (on && h.matrix) {
    showHeightLayer(h, true);
  } else {
    reportHeights(h);
  }
}
