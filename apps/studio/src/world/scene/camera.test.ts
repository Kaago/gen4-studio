import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { frameCells } from "./camera";
import type { SceneHost } from "./SceneHost";

/** The parts of a scene host the camera touches; counts the control updates and the renders. */
const fakeHost = () => ({
  camera: new THREE.PerspectiveCamera(50, 1, 0.5, 8000),
  controls: {
    target: new THREE.Vector3(),
    updates: 0,
    update() {
      this.updates++;
    },
  },
  renders: 0,
  render() {
    this.renders++;
  },
});
// 6 x 5; the cell (2, 1) lies 4 steps up (2 scene units).
const matrix: MatrixDto = { index: 0, name: "m", width: 6, height: 5, maps: new Array(30).fill(1), altitudes: Array.from({ length: 30 }, (_, i) => (i === 1 * 6 + 2 ? 4 : 0)) };

describe("frameCells", () => {
  it("frames_a_rect_from_above_and_south_like_before", () => {
    const host = fakeHost();
    // Cells 1..2 of row 1: 64 x 32 tiles, centre (64, 48); the centre cell is (2, 1).
    frameCells(host as unknown as SceneHost, { x: 1, y: 1, width: 2, height: 1 }, matrix);
    expect(host.controls.target.toArray()).toEqual([64, 2, 48]);
    const [x, y, z] = host.camera.position.toArray();
    expect(x).toBe(64);
    expect(y).toBeCloseTo(2 + 64 * 0.9);
    expect(z).toBeCloseTo(48 + 64 * 0.7);
    expect(host.controls.updates).toBe(1);
    expect(host.renders).toBe(1);
    // A small rect keeps the far plane.
    expect(host.camera.far).toBe(8000);
  });

  it("raises_the_far_plane_for_a_wide_rect", () => {
    const host = fakeHost();
    const before = host.camera.projectionMatrix.clone();
    const wide: MatrixDto = { index: 0, name: "m", width: 100, height: 1, maps: new Array(100).fill(1) };
    frameCells(host as unknown as SceneHost, { x: 0, y: 0, width: 100, height: 1 }, wide);
    expect(host.camera.far).toBeGreaterThanOrEqual(100 * 32 * 4);
    // The projection follows the new far plane.
    expect(host.camera.projectionMatrix.equals(before)).toBe(false);
    expect(host.controls.target.toArray()).toEqual([1600, 0, 16]);
  });
});
