import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import type { CellRect } from "../../workbench/sceneScope";
import { frameCells, topDownFrame } from "./camera";
import type { SceneHost } from "./SceneHost";

/** The parts of a scene host the camera touches; counts the control updates and the renders. */
const fakeHost = () => ({
  camera: new THREE.PerspectiveCamera(50, 1, 0.5, 8000),
  get slanted() {
    return this.camera;
  },
  topDown: false,
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

describe("topDownFrame", () => {
  /** The extent of a frame in scene units: width, depth, and the ratio of the two. */
  const extent = (f: { left: number; right: number; top: number; bottom: number }) => {
    const w = f.right - f.left;
    const d = f.top - f.bottom;
    return { w, d, ratio: w / d };
  };

  it("fits_the_focus_into_the_top_down_view_for_wide_and_tall_panels", () => {
    // A row of three cells (96 x 32 tiles) and a column of three (32 x 96), each in a wide (2:1) and a tall (1:2) panel.
    const row: CellRect = { x: 1, y: 2, width: 3, height: 1 };
    const column: CellRect = { x: 1, y: 2, width: 1, height: 3 };
    for (const [focus, w, d] of [
      [row, 96, 32],
      [column, 32, 96],
    ] as const) {
      for (const aspect of [2, 0.5]) {
        const f = topDownFrame(focus, aspect);
        const e = extent(f);
        // The view has the panel's shape (no distortion), holds the whole focus, and is tight on one side (a small margin only).
        expect(e.ratio).toBeCloseTo(aspect);
        expect(e.w).toBeGreaterThanOrEqual(w);
        expect(e.d).toBeGreaterThanOrEqual(d);
        expect(Math.min(e.w / w, e.d / d)).toBeLessThanOrEqual(1.15);
        // Symmetric around the camera, which stands over the middle of the focus.
        expect(f.left).toBeCloseTo(-f.right);
        expect(f.bottom).toBeCloseTo(-f.top);
        expect(f.target.toArray()).toEqual([32 + w / 2, 0, 64 + d / 2]);
      }
    }
  });
});
