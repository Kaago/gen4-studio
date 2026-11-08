import * as THREE from "three";
import { szene } from "../../ui/tokens/tokens";
import { MAP_SIZE, cellOrigin } from "../layout";
import type { Cell } from "../paint/ops";
import { disposeTree } from "../sceneUtil";
import { dashedFrame } from "./draft";
import { HEIGHT_LAYER_Y } from "./heightLayer";
import type { SceneHost } from "./SceneHost";

/**
 * How the plates of a stroke look: the tile's texture (brush, rectangle), a colour (the placeholder of a tile without a picture; height
 * and ramp in the colour of the target step, per cell when it is a function) or hatched like an unknown tile (eraser, removing a ramp).
 */
export type PreviewLook =
  | { texture: CanvasImageSource; rotate?: number; flip?: boolean }
  | { color: number | ((cell: Cell) => number) }
  | { erase: true };

/** The plates float this far over the layer „Höhen“ (`HEIGHT_LAYER_Y`), so they are seen over it and over the overlay. */
export const PREVIEW_LIFT = 0.01;
/** Drawn after the layer „Höhen“ (its plates 10, its isolines 11) and the draft frames (11). */
export const PREVIEW_ORDER = 12;
/** Opacity of the plates: what the stroke will paint, not painted yet. */
const PREVIEW_OPACITY = 0.85;
/** The frame of a rectangle floats this far over the plates. */
const FRAME_LIFT = 0.02;

const LIVE = "strokeLive";
const FRAME = "strokeFrame";
const HELD = "strokeHeld";

/** The matrix cells of `h` that show `map` (a map can stand in several cells). */
function cellsShowing(h: SceneHost, map: number): { col: number; row: number }[] {
  const matrix = h.matrix;
  if (!matrix) return [];
  const out: { col: number; row: number }[] = [];
  matrix.maps.forEach((id, i) => {
    if (id === map) out.push({ col: i % matrix.width, row: Math.floor(i / matrix.width) });
  });
  return out;
}

/** Removes the children of `h.paintPreview` named `name` and frees them. */
function drop(h: SceneHost, name: string, id?: number) {
  for (const child of [...h.paintPreview.children]) {
    if (child.name !== name || (id !== undefined && child.userData.previewId !== id)) continue;
    h.paintPreview.remove(child);
    disposeTree(child);
  }
}

/** A hatch of diagonal stripes in the colour of an unknown tile, transparent between them (made from pixels: no 2D canvas needed). */
function hatchTexture(): THREE.DataTexture {
  const size = 8;
  const color = new THREE.Color(szene.gelaende.unbekannt);
  const data = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4;
      const on = (x + y) % 4 < 2;
      data[i] = Math.round(color.r * 255);
      data[i + 1] = Math.round(color.g * 255);
      data[i + 2] = Math.round(color.b * 255);
      data[i + 3] = on ? 255 : 0;
    }
  }
  const texture = new THREE.DataTexture(data, size, size, THREE.RGBAFormat);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.needsUpdate = true;
  return texture;
}

/**
 * The world-bound UV of the floor point (x, z) of a tile turned `rotate` quarter turns and mirrored after the turn with `flip`: a port of
 * the kernel's `uv_of` (`g4_kernel::world::mesh`, table in docs/formats/world_map.md).
 */
export function uvOf(x: number, z: number, rotate: number, flip: boolean): [u: number, v: number] {
  const r = ((rotate % 4) + 4) % 4;
  const [u, v] = r === 0 ? [x, z] : r === 1 ? [z, -x] : r === 2 ? [-x, -z] : [-z, x];
  // `+ 0` turns −0 into 0.
  return [(flip ? -u : u) + 0, v + 0];
}

/** The corners of a plate in the order of its geometry (`PlaneGeometry` laid flat): north-west, north-east, south-west, south-east. */
const PLATE_CORNERS: Cell[] = [
  [0, 0],
  [1, 0],
  [0, 1],
  [1, 1],
];

/**
 * The UVs the world mesh gives the corners of one cell painted with a tile turned `rotate` and mirrored with `flip` (`uvOf` in tile units,
 * then `(u, −v)` as `meshGeometries` divides it), for a texture as large as the cell; corners as `PLATE_CORNERS`, flat `[u0, v0, u1, …]`.
 */
export function plateMeshUvs(rotate: number, flip: boolean): number[] {
  return PLATE_CORNERS.flatMap(([x, z]) => {
    const [u, v] = uvOf(x, z, rotate, flip);
    return [u, -v + 0];
  });
}

/**
 * The UVs of a plate showing a tile picture (an `ImageBitmap`, whose first row WebGL puts at v = 0 since it ignores flipY) the way the
 * mesh shows it from a texture with its first row at v = 1 (`toTexture`): v mirrored, then each axis moved into 0 to 1 (the mesh repeats
 * its textures, the plate clamps). Turned and mirrored like the mesh, so r1 shows the picture turned clockwise seen from above, north up.
 */
export function plateUvs(rotate: number, flip: boolean): number[] {
  const mesh = plateMeshUvs(rotate, flip);
  const out = mesh.map((value, i) => (i % 2 === 0 ? value : 1 - value));
  const min = [Math.min(out[0], out[2], out[4], out[6]), Math.min(out[1], out[3], out[5], out[7])];
  return out.map((value, i) => value - min[i % 2] + 0);
}

function previewMaterial(look: PreviewLook): THREE.MeshBasicMaterial {
  const material = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: PREVIEW_OPACITY,
    depthTest: false,
    depthWrite: false,
    // The token colours must come out as they are, not through tone mapping.
    toneMapped: false,
  });
  if ("texture" in look) {
    const texture = new THREE.Texture(look.texture as THREE.Texture["image"]);
    texture.magFilter = THREE.NearestFilter;
    texture.minFilter = THREE.NearestFilter;
    texture.generateMipmaps = false;
    texture.colorSpace = THREE.SRGBColorSpace;
    // WebGL ignores flipY for an ImageBitmap (its first row lands at v = 0); the plate's UVs (`plateUvs`) account for it.
    texture.flipY = false;
    texture.needsUpdate = true;
    material.map = texture;
  } else if ("erase" in look) {
    material.map = hatchTexture();
  }
  return material;
}

/**
 * Shows the plates of the stroke being drawn: one per cell of `cells` (map-local; cells outside 0 to 31 are left out) in every matrix
 * cell showing `map`, `PREVIEW_LIFT` over the layer „Höhen“ on the ground, looking as `look` says; in place of the plates before. Does not render.
 */
export function setStrokePreview(h: SceneHost, map: number, cells: Cell[], look: PreviewLook): void {
  drop(h, LIVE);
  const own = cells.filter(([x, z]) => x >= 0 && x < MAP_SIZE && z >= 0 && z < MAP_SIZE);
  const at = cellsShowing(h, map);
  if (own.length === 0 || at.length === 0) return;
  const plane = new THREE.PlaneGeometry(1, 1);
  plane.rotateX(-Math.PI / 2);
  if ("texture" in look) plane.setAttribute("uv", new THREE.Float32BufferAttribute(plateUvs(look.rotate ?? 0, look.flip ?? false), 2));
  const plates = new THREE.InstancedMesh(plane, previewMaterial(look), own.length * at.length);
  plates.name = LIVE;
  plates.renderOrder = PREVIEW_ORDER;
  plates.frustumCulled = false;
  const m4 = new THREE.Matrix4();
  const color = new THREE.Color();
  let n = 0;
  for (const { col, row } of at) {
    const origin = cellOrigin(col, row);
    for (const cell of own) {
      const x = origin.x + cell[0];
      const z = origin.z + cell[1];
      m4.setPosition(x + 0.5, HEIGHT_LAYER_Y + PREVIEW_LIFT + h.ground.y(x, z), z + 0.5);
      plates.setMatrixAt(n, m4);
      if ("color" in look) plates.setColorAt(n, color.setHex(typeof look.color === "function" ? look.color(cell) : look.color));
      n++;
    }
  }
  h.paintPreview.add(plates);
}

/** Shows the dashed frame of `draft.ts` around the rectangle from `a` to `b` (null: around `a` alone) in every cell showing `map`. Does not render. */
export function setStrokeRect(h: SceneHost, map: number, a: Cell, b: Cell | null): void {
  drop(h, FRAME);
  const to = b ?? a;
  const [x0, x1] = [Math.min(a[0], to[0]), Math.max(a[0], to[0]) + 1];
  const [z0, z1] = [Math.min(a[1], to[1]), Math.max(a[1], to[1]) + 1];
  for (const { col, row } of cellsShowing(h, map)) {
    const origin = cellOrigin(col, row);
    const y = HEIGHT_LAYER_Y + PREVIEW_LIFT + FRAME_LIFT + h.ground.y(origin.x + a[0], origin.z + a[1]);
    const frame = dashedFrame(origin.x + x0, origin.z + z0, origin.x + x1, origin.z + z1, y);
    frame.name = FRAME;
    frame.renderOrder = PREVIEW_ORDER + 1;
    h.paintPreview.add(frame);
  }
}

let nextId = 1;

/**
 * Keeps the plates of the stroke just drawn until its answer (the frame goes): returns the id `clearStrokePreview` takes them away by.
 * The next stroke draws new plates beside them.
 */
export function holdStrokePreview(h: SceneHost): number {
  drop(h, FRAME);
  const id = nextId++;
  for (const child of h.paintPreview.children) {
    if (child.name !== LIVE) continue;
    child.name = HELD;
    child.userData.previewId = id;
  }
  return id;
}

/**
 * Takes a preview away and frees its geometry, material and texture: without `id` the stroke being drawn (plates and frame), with `id`
 * the plates `holdStrokePreview` kept under it. Does not render.
 */
export function clearStrokePreview(h: SceneHost, id?: number): void {
  if (id === undefined) {
    drop(h, LIVE);
    drop(h, FRAME);
  } else {
    drop(h, HELD, id);
  }
}

/** Frees every preview of `h` (the scene goes). */
export function disposeStrokePreviews(h: SceneHost): void {
  for (const child of [...h.paintPreview.children]) {
    h.paintPreview.remove(child);
    disposeTree(child);
  }
}
