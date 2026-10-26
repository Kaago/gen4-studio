// @vitest-environment jsdom
import * as THREE from "three";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: string[] = [];
const renders = { count: 0 };
const clearColors: string[] = [];
const sizes: [number, number][] = [];

vi.mock("three", async (importOriginal) => {
  const three = await importOriginal<typeof import("three")>();
  class FakeRenderer {
    domElement = document.createElement("canvas");
    constructor() {
      const remove = this.domElement.remove.bind(this.domElement);
      this.domElement.remove = () => {
        calls.push("canvas");
        remove();
      };
    }
    setPixelRatio() {}
    setClearColor(color: string) {
      clearColors.push(color);
    }
    setSize(w: number, h: number) {
      sizes.push([w, h]);
    }
    render() {
      renders.count++;
    }
    dispose() {
      calls.push("renderer");
    }
  }
  return { ...three, WebGLRenderer: FakeRenderer };
});
vi.mock("../markers", () => ({ disposeMarkers: () => calls.push("markers") }));
vi.mock("../sceneUtil", () => ({ disposeTree: () => calls.push("content") }));

import type { MatrixDto } from "../../api";
import { dunkel, hell } from "../../ui/tokens/tokens";
import type { BuildingLayer } from "../buildingLayer";
import type { MarkerSet } from "../markers";
import { frameCells } from "./camera";
import { SceneHost } from "./SceneHost";

/** The observers made, with the callback each was given and the elements it watches. */
const observers: FakeObserver[] = [];
class FakeObserver {
  watched: Element[] = [];
  constructor(readonly callback: () => void) {
    observers.push(this);
  }
  observe(el: Element) {
    this.watched.push(el);
  }
  disconnect() {
    calls.push("observer");
  }
}

beforeEach(() => {
  calls.length = 0;
  renders.count = 0;
  clearColors.length = 0;
  sizes.length = 0;
  observers.length = 0;
  vi.stubGlobal("ResizeObserver", FakeObserver);
});
afterEach(() => vi.unstubAllGlobals());

function host() {
  const el = document.createElement("div");
  const h = new SceneHost(el);
  return { el, h };
}

/** Gives the element a size and lets the observer hear of it, as the dock does when a panel is maximized, restored or floated. */
function resize(el: HTMLElement, w: number, h: number) {
  Object.defineProperty(el, "clientWidth", { value: w, configurable: true });
  Object.defineProperty(el, "clientHeight", { value: h, configurable: true });
  observers[0]!.callback();
}

/** Width, depth and ratio of what an orthographic camera shows, in scene units (its zoom included). */
function shown(camera: THREE.OrthographicCamera) {
  const w = (camera.right - camera.left) / camera.zoom;
  const d = (camera.top - camera.bottom) / camera.zoom;
  return { w, d, ratio: w / d };
}

// 6 x 5, all cells at height 0; the focus is cells 1..2 of row 1 (64 x 32 tiles around (64, 48)).
const matrix: MatrixDto = { index: 0, name: "m", width: 6, height: 5, maps: new Array(30).fill(1) };
const focus = { x: 1, y: 1, width: 2, height: 1 };

describe("SceneHost", () => {
  it("clears_with_the_world_surface_of_the_shown_theme", () => {
    document.documentElement.dataset.theme = "dunkel";
    try {
      host();
      expect(clearColors).toEqual([dunkel["flaeche/welt"]]);
      clearColors.length = 0;
      document.documentElement.dataset.theme = "hell";
      host();
      expect(clearColors).toEqual([hell["flaeche/welt"]]);
    } finally {
      delete document.documentElement.dataset.theme;
    }
  });

  it("set_background_changes_the_clear_colour_and_leaves_models_and_content_alone", () => {
    const { h } = host();
    const content = h.content;
    const models = h.models;
    const model = new THREE.Group();
    models.push(model);
    content.add(model);
    clearColors.length = 0;
    h.setBackground("#101821");
    expect(clearColors).toEqual(["#101821"]);
    expect(h.content).toBe(content);
    expect(h.models).toBe(models);
    expect(h.models).toEqual([model]);
    expect(content.children).toEqual([model]);
  });

  it("puts its canvas into the element and renders once for the first size", () => {
    const { el, h } = host();
    expect(el.contains(h.canvas)).toBe(true);
    expect(renders.count).toBe(1);
  });

  it("resizes_the_renderer_and_the_camera_when_its_element_changes_size", () => {
    // The dock maximizes, restores or floats a group: the panel's element changes size and the scene must fill it without distortion.
    const { el, h } = host();
    const size = (w: number, h: number) => {
      Object.defineProperty(el, "clientWidth", { value: w, configurable: true });
      Object.defineProperty(el, "clientHeight", { value: h, configurable: true });
    };
    expect(observers.flatMap((o) => o.watched)).toEqual([el]);
    const rendered = renders.count;
    size(1200, 600);
    observers[0]!.callback();
    expect(sizes[sizes.length - 1]).toEqual([1200, 600]);
    expect(h.slanted.aspect).toBe(2);
    expect(renders.count).toBe(rendered + 1);
    size(300, 600);
    observers[0]!.callback();
    expect(sizes[sizes.length - 1]).toEqual([300, 600]);
    expect(h.slanted.aspect).toBe(0.5);
  });

  it("switches_to_an_orthographic_camera_looking_straight_down", () => {
    const { el, h } = host();
    resize(el, 800, 400);
    const before = h.controls;
    const disposed = vi.spyOn(before, "dispose");
    h.setTopDown(true, focus, matrix);
    expect(h.topDown).toBe(true);
    const camera = h.camera as THREE.OrthographicCamera;
    expect(camera).toBeInstanceOf(THREE.OrthographicCamera);
    // North stays up on the screen, the camera looks straight down onto the middle of the focus.
    expect(camera.up.toArray()).toEqual([0, 0, -1]);
    const dir = camera.getWorldDirection(new THREE.Vector3());
    expect([dir.x, dir.y, dir.z].map((v) => Math.round(v * 1e6) / 1e6 + 0)).toEqual([0, -1, 0]);
    expect(h.controls.target.x).toBeCloseTo(64);
    expect(h.controls.target.z).toBeCloseTo(48);
    expect(camera.position.x).toBeCloseTo(64);
    expect(camera.position.z).toBeCloseTo(48);
    // The focus fits, in the panel's shape.
    const s = shown(camera);
    expect(s.ratio).toBeCloseTo(2);
    expect(s.w).toBeGreaterThanOrEqual(64);
    expect(s.d).toBeGreaterThanOrEqual(32);
    // New controls drive the new camera: they pan and zoom but never turn it; the old ones are gone and render no more.
    expect(h.controls).not.toBe(before);
    expect(disposed).toHaveBeenCalled();
    expect(h.controls.object).toBe(camera);
    expect(h.controls.enableRotate).toBe(false);
    // The left button and one finger pan from above (rotating is off, so they would do nothing otherwise).
    expect(h.controls.mouseButtons.LEFT).toBe(THREE.MOUSE.PAN);
    expect(h.controls.touches.ONE).toBe(THREE.TOUCH.PAN);
    const rendered = renders.count;
    before.dispatchEvent({ type: "change" });
    expect(renders.count).toBe(rendered);
    h.controls.dispatchEvent({ type: "change" });
    expect(renders.count).toBe(rendered + 1);
  });

  it("keeps_the_orthographic_view_undistorted_on_resize", () => {
    const { el, h } = host();
    resize(el, 800, 400);
    h.setTopDown(true, focus, matrix);
    const camera = h.camera as THREE.OrthographicCamera;
    for (const [w, d] of [
      [1200, 600],
      [300, 600],
      [900, 300],
    ]) {
      const rendered = renders.count;
      resize(el, w, d);
      expect(h.camera).toBe(camera);
      const s = shown(camera);
      expect(s.ratio).toBeCloseTo(w / d);
      expect(s.w).toBeGreaterThanOrEqual(64);
      expect(s.d).toBeGreaterThanOrEqual(32);
      expect(renders.count).toBe(rendered + 1);
    }
  });

  it("goes_back_to_the_last_slanted_view", () => {
    const { el, h } = host();
    const slanted = h.camera as THREE.PerspectiveCamera;
    // The user has turned the slanted view somewhere.
    h.controls.target.set(10, 0, 20);
    slanted.position.set(10, 50, 80);
    h.controls.update();
    h.setTopDown(true, focus, matrix);
    // Panning the top-down view and resizing the panel meanwhile do not move the slanted view.
    h.controls.target.set(99, 0, 99);
    resize(el, 300, 600);
    h.setTopDown(false, focus, matrix);
    expect(h.topDown).toBe(false);
    expect(h.camera).toBe(slanted);
    expect(slanted.position.x).toBeCloseTo(10);
    expect(slanted.position.y).toBeCloseTo(50);
    expect(slanted.position.z).toBeCloseTo(80);
    expect(h.controls.target.toArray()).toEqual([10, 0, 20]);
    expect(h.controls.object).toBe(slanted);
    expect(h.controls.enableRotate).toBe(true);
    // The slanted view turns with the left button again (the controls' defaults).
    expect(h.controls.mouseButtons.LEFT).toBe(THREE.MOUSE.ROTATE);
    expect(h.controls.touches.ONE).toBe(THREE.TOUCH.ROTATE);
    // It has the panel's shape of now.
    expect(slanted.aspect).toBe(0.5);
  });

  it("frames_a_new_focus_in_the_top_down_view_and_keeps_the_slanted_view_for_it", () => {
    // A full load while the top-down view is on frames its focus there; going back shows the slanted view of that focus.
    const { el, h } = host();
    resize(el, 800, 400);
    h.setTopDown(true, focus, matrix);
    const camera = h.camera as THREE.OrthographicCamera;
    camera.zoom = 3;
    frameCells(h, { x: 4, y: 3, width: 1, height: 1 }, matrix);
    expect(h.camera).toBe(camera);
    expect(camera.zoom).toBe(1);
    expect(h.controls.target.x).toBeCloseTo(144);
    expect(h.controls.target.z).toBeCloseTo(112);
    expect(camera.position.x).toBeCloseTo(144);
    expect(camera.position.z).toBeCloseTo(112);
    h.setTopDown(false, focus, matrix);
    const rounded = (v: THREE.Vector3) => v.toArray().map((n) => Math.round(n * 10) / 10 + 0);
    expect(rounded(h.controls.target)).toEqual([144, 0, 112]);
    expect(rounded(h.camera.position)).toEqual([144, 28.8, 134.4]);
  });

  it("does_nothing_when_the_view_is_already_as_asked", () => {
    const { h } = host();
    const controls = h.controls;
    const rendered = renders.count;
    h.setTopDown(false, focus, matrix);
    expect(h.controls).toBe(controls);
    expect(renders.count).toBe(rendered);
    h.setTopDown(true, focus, matrix);
    const camera = h.camera;
    h.setTopDown(true, focus, matrix);
    expect(h.camera).toBe(camera);
  });

  it("disposes markers, buildings, observer and controls, content, then the renderer", () => {
    const { h } = host();
    h.markers = {} as MarkerSet;
    h.buildings = { dispose: () => calls.push("buildings") } as unknown as BuildingLayer;
    vi.spyOn(h.controls, "dispose").mockImplementation(() => void calls.push("controls"));
    h.dispose();
    expect(calls).toEqual(["markers", "buildings", "observer", "controls", "content", "renderer", "canvas"]);
    expect(h.disposed).toBe(true);
  });

  it("disposes without markers and buildings", () => {
    const { h } = host();
    h.dispose();
    expect(calls).toEqual(["observer", "content", "renderer", "canvas"]);
  });

  it("stops rendering on control changes once disposed", () => {
    const { h } = host();
    h.dispose();
    const before = renders.count;
    h.controls.dispatchEvent({ type: "change" });
    expect(renders.count).toBe(before);
  });

  it("renders a burst of queued renders once, and nothing after dispose", () => {
    const frames: (() => void)[] = [];
    vi.stubGlobal("requestAnimationFrame", (cb: () => void) => frames.push(cb));
    const { h } = host();
    const before = renders.count;
    h.queueRender();
    h.queueRender();
    h.queueRender();
    expect(frames).toHaveLength(1);
    frames[0]();
    expect(renders.count).toBe(before + 1);
    h.queueRender();
    h.dispose();
    frames[1]();
    expect(renders.count).toBe(before + 1);
  });
});
