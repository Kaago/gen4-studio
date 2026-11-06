import * as THREE from "three";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../../api";
import { createGround } from "../ground";
import { MAP_SIZE } from "../layout";
import { OVERLAY_Y } from "./content";
import { fakeRefs, viewThrough } from "./dragTestKit.testkit";
import { createHover } from "./pointerHover";
import type { SceneHost } from "./SceneHost";

/** One cell, one map; columns from 16 on lie 32 E (two tiles) up. */
const one: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1], headers: null, altitudes: null };
const slope = { 1: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => (i % MAP_SIZE >= 16 ? 32 : 0)) };

function view() {
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
  camera.position.set(20.5, 22, 30.5);
  camera.lookAt(20.5, 2, 10.5);
  const v = viewThrough(camera, one, slope);
  Object.assign(v.h, { names: new Map(), markers: null, events: null });
  const { refs, callbacks } = fakeRefs();
  const hover = createHover(v.h, v.picker, refs, () => false);
  const detach = hover.attach();
  const point = (x: number, y: number, z: number) => {
    const at = v.through(x, y, z);
    v.h.canvas.dispatchEvent(Object.assign(new Event("pointermove"), { clientX: at.x, clientY: at.y }));
    // The next frame comes.
    for (const run of frames.splice(0)) run(0);
  };
  return { h: v.h as SceneHost, callbacks, point, detach };
}

/** The frames asked for and not yet run. */
const frames: FrameRequestCallback[] = [];

beforeEach(() => {
  frames.length = 0;
  vi.stubGlobal("requestAnimationFrame", (run: FrameRequestCallback) => frames.push(run));
  vi.stubGlobal("cancelAnimationFrame", () => undefined);
});
afterEach(() => vi.unstubAllGlobals());

describe("createHover", () => {
  it("hover_writes_tile_and_ground_to_the_canvas", () => {
    const { h, callbacks, point, detach } = view();
    point(20.5, OVERLAY_Y + 2, 10.5);
    expect(h.canvas.dataset.hoverTile).toBe("20,10");
    expect(h.canvas.dataset.hoverGround).toBe("2.000");
    expect(callbacks.onInspect).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "tile", x: 20, y: 10, absX: 20, absZ: 10, units: 32 }));
    point(5.5, OVERLAY_Y, 10.5);
    expect(h.canvas.dataset.hoverTile).toBe("5,10");
    expect(h.canvas.dataset.hoverGround).toBe("0.000");
    expect(callbacks.onInspect).toHaveBeenLastCalledWith(expect.objectContaining({ x: 5, units: 0 }));
    // Off the matrix: nothing is hovered, the canvas says so.
    point(200, OVERLAY_Y, 10.5);
    expect(h.canvas.dataset.hoverTile).toBeUndefined();
    expect(h.canvas.dataset.hoverGround).toBeUndefined();
    detach();
  });

  it("hover_reports_the_new_height_when_only_the_ground_changed_under_a_resting_pointer", () => {
    // Straight down onto tile (20, 10): the same client point picks that tile at any height.
    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
    camera.up.set(0, 0, -1);
    camera.position.set(20.5, 40, 10.5);
    camera.lookAt(20.5, 0, 10.5);
    const v = viewThrough(camera, one, slope);
    Object.assign(v.h, { names: new Map(), markers: null, events: null });
    const { refs, callbacks } = fakeRefs();
    const detach = createHover(v.h, v.picker, refs, () => false).attach();
    const at = v.through(20.5, 2, 10.5);
    const rest = () => {
      v.h.canvas.dispatchEvent(Object.assign(new Event("pointermove"), { clientX: at.x, clientY: at.y }));
      for (const run of frames.splice(0)) run(0);
    };
    rest();
    expect(callbacks.onInspect).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "tile", absX: 20, absZ: 10, units: 32 }));
    // An undo or set_map_terrain changes only the heights (the collision stays): the map comes back, the scene gets a new ground.
    const data = v.h.loaded.get(1)!;
    data.map = { ...data.map, heights: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => (i % MAP_SIZE >= 16 ? 16 : 0)) };
    v.h.ground = createGround(one, v.h.loaded);
    rest();
    expect(v.h.canvas.dataset.hoverTile).toBe("20,10");
    expect(v.h.canvas.dataset.hoverGround).toBe("1.000");
    expect(callbacks.onInspect).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "tile", absX: 20, absZ: 10, units: 16 }));
    detach();
  });
});
