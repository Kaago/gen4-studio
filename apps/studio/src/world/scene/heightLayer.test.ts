import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { createGround } from "../ground";
import { MAP_SIZE } from "../layout";
import { HEIGHT_LAYER_Y, buildHeightLayer, heightColor, heightStep, isolineSegments, setHeightsShown } from "./heightLayer";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

/** A map whose tile heights (in world units) are `at(x, y)`. */
const mapWith = (index: number, at: (x: number, y: number) => number | null): Loaded => ({
  map: {
    index,
    permissions: [],
    buildings: [],
    heights: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => at(i % MAP_SIZE, Math.floor(i / MAP_SIZE))),
  },
});

/** The edges of `segments` as „x1,z1-x2,z2“ (the y of both ends is the same). */
const edges = (segments: Float32Array) => {
  const out: string[] = [];
  for (let i = 0; i < segments.length; i += 6) out.push(`${segments[i]},${segments[i + 2]}-${segments[i + 3]},${segments[i + 5]}`);
  return out;
};

describe("heightLayer", () => {
  it("steps_cycle_through_seven_cividis_colours", () => {
    expect(heightStep(0)).toBe(0);
    expect(heightStep(8)).toBe(1);
    expect(heightStep(11.9)).toBe(1);
    expect(heightStep(12)).toBe(2);
    expect(heightColor(0)).toBe(szene.hoehe["0"]);
    expect(heightColor(8)).toBe(szene.hoehe["1"]);
    expect(heightColor(48)).toBe(szene.hoehe["6"]);
    // Seven steps up the cycle starts again; below 0 it runs backwards.
    expect(heightColor(56)).toBe(szene.hoehe["0"]);
    expect(heightColor(-8)).toBe(szene.hoehe["6"]);
  });

  it("isolines_mark_every_step_change", () => {
    // One map at step 0 with a 3 x 3 block at step 2 (16 E) from tile (4, 6) to (6, 8).
    const one: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1], headers: null, altitudes: null };
    const block = new Map([[1, mapWith(1, (x, y) => (x >= 4 && x <= 6 && y >= 6 && y <= 8 ? 16 : 0))]]);
    const segments = isolineSegments(one, block, createGround(one, block));
    // Twelve pieces round the block, each a pair of xyz points.
    expect(segments.length).toBe(12 * 6);
    const found = edges(segments);
    expect(found).toContain("4,6-5,6");
    expect(found).toContain("4,6-4,7");
    expect(found).toContain("7,8-7,9");
    expect(found).toContain("6,9-7,9");
    // On the higher side: the block's height (1 tile) over the layer.
    for (let i = 1; i < segments.length; i += 3) expect(segments[i]).toBeCloseTo(HEIGHT_LAYER_Y + 1, 5);

    // Two loaded maps side by side at the same height (the right one by its altitude): no line along their border.
    const two: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2], headers: null, altitudes: [0, 2] };
    const level = new Map([
      [1, mapWith(1, () => 16)],
      [2, mapWith(2, () => 0)],
    ]);
    expect(isolineSegments(two, level, createGround(two, level)).length).toBe(0);
    // Without the altitude the right map is a step lower: one line along the whole border, on the left (higher) side.
    const low: MatrixDto = { ...two, altitudes: null };
    const border = isolineSegments(low, level, createGround(low, level));
    expect(border.length).toBe(MAP_SIZE * 6);
    expect(edges(border).every((e) => e.startsWith(`${MAP_SIZE},`))).toBe(true);
    // A cell whose map is not loaded draws no lines towards it.
    const half = new Map([[1, mapWith(1, () => 16)]]);
    expect(isolineSegments(low, half, createGround(low, half)).length).toBe(0);
  });

  it("tiles_lie_on_the_ground", () => {
    const two: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, null], headers: null, altitudes: [2, 0] };
    const loaded = new Map([[1, mapWith(1, (x, y) => (x === 3 && y === 4 ? 32 : 0))]]);
    const ground = createGround(two, loaded);
    const layer = buildHeightLayer(two, loaded, ground);
    const tiles = layer.getObjectByName("tiles") as THREE.InstancedMesh;
    const isolines = layer.getObjectByName("isolines") as THREE.LineSegments;
    // Only the loaded cell has tiles; the empty one has none.
    expect(tiles.count).toBe(MAP_SIZE * MAP_SIZE);
    const at = (i: number) => {
      const m = new THREE.Matrix4();
      tiles.getMatrixAt(i, m);
      return new THREE.Vector3().setFromMatrixPosition(m);
    };
    const raised = at(4 * MAP_SIZE + 3);
    expect(raised.x).toBeCloseTo(3.5, 5);
    expect(raised.z).toBeCloseTo(4.5, 5);
    // Altitude 2 (one tile) plus 32 E (two tiles).
    expect(raised.y).toBeCloseTo(HEIGHT_LAYER_Y + ground.y(3, 4), 5);
    expect(raised.y).toBeCloseTo(HEIGHT_LAYER_Y + 3, 5);
    expect(at(0).y).toBeCloseTo(HEIGHT_LAYER_Y + 1, 5);
    // The colour of a tile is its step: 16 E of altitude plus 32 E is step 6.
    const colour = new THREE.Color();
    tiles.getColorAt(4 * MAP_SIZE + 3, colour);
    expect(colour.getHex()).toBe(new THREE.Color(szene.hoehe["6"]).getHex());
    tiles.getColorAt(0, colour);
    expect(colour.getHex()).toBe(new THREE.Color(szene.hoehe["2"]).getHex());
    // The raised tile has four edges of a step change; the lines take the first height colour.
    expect(isolines.geometry.getAttribute("position").count).toBe(4 * 2);
    expect((isolines.material as THREE.LineBasicMaterial).color.getHex()).toBe(new THREE.Color(szene.hoehe["0"]).getHex());
  });

  it("the_layer_is_not_built_while_off_and_built_on_its_first_switch_on", () => {
    const one: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1], headers: null, altitudes: null };
    const loaded = new Map([[1, mapWith(1, () => 0)]]);
    // The parts of a scene handle the layer touches; `canvas` only needs a dataset.
    const h = { matrix: null, loaded, ground: createGround(one, loaded), content: new THREE.Group(), heights: null, canvas: { dataset: {} as DOMStringMap } } as unknown as SceneHost;
    // Without a matrix nothing is built, even when switched on.
    setHeightsShown(h, true);
    expect(h.heights).toBeNull();
    h.matrix = one;
    setHeightsShown(h, false);
    expect(h.heights).toBeNull();
    expect(h.content.children).toHaveLength(0);
    expect(h.canvas.dataset.heightsShown).toBeUndefined();
    // The first switch-on builds it into the content.
    setHeightsShown(h, true);
    const layer = h.heights!;
    expect(layer).not.toBeNull();
    expect(h.content.children).toEqual([layer]);
    expect(layer.visible).toBe(true);
    expect(h.canvas.dataset.heightsShown).toBe("true");
    // Later switches only hide and show the same layer.
    setHeightsShown(h, false);
    expect(h.heights).toBe(layer);
    expect(layer.visible).toBe(false);
    expect(h.canvas.dataset.heightsShown).toBeUndefined();
    setHeightsShown(h, true);
    expect(h.heights).toBe(layer);
    expect(h.content.children).toHaveLength(1);
  });
});
