import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { BuildingDto, MapDto, MatrixDto } from "../api";
import { BuildingLayer } from "./buildingLayer";

const building = (model_id: number, x = 0, z = 0, over: Partial<BuildingDto> = {}): BuildingDto => ({
  model_id,
  x,
  y: 0,
  z,
  sx: 1,
  sy: 1,
  sz: 1,
  rotation: ["00000000", "00000000", "00000000"],
  ...over,
});

/** Map 5 appears in matrix cells 0 and 2 of a 3 x 1 matrix; map 6 in cell 1. */
const matrix: MatrixDto = { index: 0, name: "m", width: 3, height: 1, maps: [5, 6, 5] };
const mapOf = (index: number, buildings: BuildingDto[]): { map: MapDto } => ({ map: { index, permissions: [], buildings } });
const loaded = () =>
  new Map([
    [5, mapOf(5, [building(7, 4, -2), building(8, 0, 0)])],
    [6, mapOf(6, [building(7, 1, 1)])],
  ]);

/** A model of one 16 x 16 x 16 unit box (a 1-tile cube at map scale). */
const model = () => new THREE.Group().add(new THREE.Mesh(new THREE.BoxGeometry(16, 16, 16), new THREE.MeshBasicMaterial()));

const positions = (layer: BuildingLayer) => layer.root.children[0].children.map((g) => g.position.toArray());

/** A downward ray at scene position (x, z). */
const ray = (x: number, z: number) => {
  const r = new THREE.Raycaster();
  r.set(new THREE.Vector3(x, 50, z), new THREE.Vector3(0, -1, 0));
  return r;
};

describe("BuildingLayer", () => {
  it("makes one group per building and matrix cell that shows its map", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    // Map 5 (2 buildings) in cells 0 and 2, map 6 (1 building) in cell 1.
    expect(positions(layer)).toEqual(expect.arrayContaining([[20, 0, 14], [16, 0, 16], [84, 0, 14], [80, 0, 16], [49, 0, 17]]));
    expect(layer.root.children[0].children).toHaveLength(5);
  });

  it("asks for each model once and shows a box until it arrives", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    expect(layer.missingModelIds().sort()).toEqual([7, 8]);
    const group = layer.root.children[0].children[0];
    expect(group.children).toHaveLength(1);
    layer.setTemplate(7, model());
    expect(layer.missingModelIds()).toEqual([8]);
    // The buildings with model 7 (three of them) swapped the box for a clone scaled by 1/16.
    const withModel = layer.root.children[0].children.filter((g) => g.children.length === 1 && g.children[0].type !== "Mesh");
    expect(withModel).toHaveLength(3);
    expect(withModel[0].children[0].scale.toArray()).toEqual([1 / 16, 1 / 16, 1 / 16]);
  });

  it("keeps the box of a model that failed and offers it again on retry", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.setFailed(8, "boom");
    expect(layer.failure(8)).toBe("boom");
    expect(layer.missingModelIds()).toEqual([7]);
    layer.retry(8);
    expect(layer.failure(8)).toBeUndefined();
    expect(layer.missingModelIds().sort()).toEqual([7, 8]);
  });

  it("returns every building to its box and forgets failures when the textures change", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.setTemplate(7, model());
    layer.setFailed(8, "boom");
    layer.resetModels();
    expect(layer.missingModelIds().sort()).toEqual([7, 8]);
    expect(layer.root.children[0].children.every((g) => g.children.length === 1 && g.children[0].type === "Mesh")).toBe(true);
  });

  it("picks the building under a ray, also through its placeholder box", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.root.updateMatrixWorld(true);
    // Building 0 of map 5 in cell 2 stands at (84, 14) with a unit box.
    const hit = layer.pick(ray(84, 14));
    expect(hit?.ref).toEqual({ map: 5, index: 0 });
    expect(hit?.cell).toEqual({ x: 64, z: 0 });
    expect(layer.pick(ray(70, 5))).toBeNull();
    layer.setTemplate(7, model());
    layer.root.updateMatrixWorld(true);
    expect(layer.pick(ray(49, 17))?.ref).toEqual({ map: 6, index: 0 });
  });

  it("moves a building in all its cells and outlines it while selected", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.select({ map: 5, index: 1 });
    const outlines = layer.root.children[1].children;
    expect(outlines).toHaveLength(2);
    layer.show({ map: 5, index: 1 }, building(8, 2.5, -1));
    expect(positions(layer)).toEqual(expect.arrayContaining([[18.5, 0, 15], [82.5, 0, 15]]));
    layer.select(null);
    expect(layer.root.children[1].children).toHaveLength(0);
  });

  it("swaps the model when the model number of a shown building changes", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.setTemplate(7, model());
    layer.show({ map: 6, index: 0 }, building(8, 1, 1));
    const group = layer.root.children[0].children.find((g) => g.position.x === 49) as THREE.Object3D;
    // Model 8 is not there yet: the building is back to its box.
    expect(group.children.map((c) => c.type)).toEqual(["Mesh"]);
    expect(layer.missingModelIds()).toEqual([8]);
  });

  it("rebuilds the slots of one map from new data and keeps the selection outline", () => {
    const layer = new BuildingLayer();
    layer.build(matrix, loaded());
    layer.setTemplate(7, model());
    layer.select({ map: 5, index: 1 });
    layer.syncMap(matrix, 5, [building(7, 4, -2), building(8, 3, 3), building(9)]);
    expect(layer.root.children[0].children).toHaveLength(1 + 6);
    expect(layer.has({ map: 5, index: 2 })).toBe(true);
    expect(layer.root.children[1].children).toHaveLength(2);
    expect(layer.missingModelIds().sort()).toEqual([8, 9]);
    // A selection whose building is gone has no slots; the caller notices through `has`.
    layer.syncMap(matrix, 5, [building(7)]);
    expect(layer.has({ map: 5, index: 1 })).toBe(false);
    expect(layer.root.children[1].children).toHaveLength(0);
  });
});
