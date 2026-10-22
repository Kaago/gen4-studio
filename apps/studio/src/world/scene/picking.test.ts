import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { OVERLAY_Y } from "./content";
import { createPicker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

const SIZE = 800;

/** A 2 x 1 matrix: cell 0 on the ground, cell 1 at altitude 2 (one tile up). */
const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2], headers: null, altitudes: [0, 2] };
const loadedMap = (index: number): Loaded => ({ map: { index, permissions: [], buildings: [] } });

function host(m: MatrixDto | null = matrix) {
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
  // Looks at the border of the two cells from the side, so that rays are slanted.
  camera.position.set(45, 30, 20);
  camera.lookAt(32, 0, 5);
  camera.updateMatrixWorld(true);
  camera.updateProjectionMatrix();
  const canvas = { getBoundingClientRect: () => ({ left: 0, top: 0, width: SIZE, height: SIZE }) };
  const h = { canvas, camera, matrix: m, loaded: new Map([[1, loadedMap(1)], [2, loadedMap(2)]]) } as unknown as SceneHost;
  /** Client position at which the view ray passes through scene point (x, y, z). */
  const through = (x: number, y: number, z: number) => {
    const p = new THREE.Vector3(x, y, z).project(camera);
    return { x: ((p.x + 1) / 2) * SIZE, y: ((1 - p.y) / 2) * SIZE };
  };
  return { h, picker: createPicker(h), through };
}

describe("pickTile with altitudes", () => {
  it("hits the raised surface of a cell above the ground plane", () => {
    const { picker, through } = host();
    const at = through(37.5, OVERLAY_Y + 1, 5.5);
    const hit = picker.pickTile(at.x, at.y);
    expect(hit).toMatchObject({ mapId: 2, col: 1, row: 0, x: 5, y: 5 });
  });

  it("falls back to the ground plane where the raised plane is hit outside its cell", () => {
    const { picker, through } = host();
    // This ray crosses the plane of altitude 2 in cell 0 (which is on the ground), so only the ground plane counts.
    const at = through(30.5, OVERLAY_Y, 5.5);
    const hit = picker.pickTile(at.x, at.y);
    expect(hit).toMatchObject({ mapId: 1, col: 0, row: 0, x: 30, y: 5 });
  });

  it("still hits the ground plane of a matrix without altitudes", () => {
    const { picker, through } = host({ ...matrix, altitudes: null });
    const at = through(37.5, OVERLAY_Y, 5.5);
    expect(picker.pickTile(at.x, at.y)).toMatchObject({ mapId: 2, col: 1, x: 5, y: 5 });
  });

  it("returns null over nothing, and without a matrix", () => {
    const { picker, through } = host();
    const off = through(200, OVERLAY_Y, 5.5);
    expect(picker.pickTile(off.x, off.y)).toBeNull();
    expect(host(null).picker.pickTile(10, 10)).toBeNull();
  });
});
