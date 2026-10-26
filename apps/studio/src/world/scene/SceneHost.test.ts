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

import { dunkel, hell } from "../../ui/tokens/tokens";
import type { BuildingLayer } from "../buildingLayer";
import type { MarkerSet } from "../markers";
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
    expect(h.camera.aspect).toBe(2);
    expect(renders.count).toBe(rendered + 1);
    size(300, 600);
    observers[0]!.callback();
    expect(sizes[sizes.length - 1]).toEqual([300, 600]);
    expect(h.camera.aspect).toBe(0.5);
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
