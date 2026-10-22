// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: string[] = [];
const renders = { count: 0 };

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
    setClearColor() {}
    setSize() {}
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

import type { BuildingLayer } from "../buildingLayer";
import type { MarkerSet } from "../markers";
import { SceneHost } from "./SceneHost";

class FakeObserver {
  observe() {}
  disconnect() {
    calls.push("observer");
  }
}

beforeEach(() => {
  calls.length = 0;
  renders.count = 0;
  vi.stubGlobal("ResizeObserver", FakeObserver);
});
afterEach(() => vi.unstubAllGlobals());

function host() {
  const el = document.createElement("div");
  const h = new SceneHost(el);
  return { el, h };
}

describe("SceneHost", () => {
  it("puts its canvas into the element and renders once for the first size", () => {
    const { el, h } = host();
    expect(el.contains(h.canvas)).toBe(true);
    expect(renders.count).toBe(1);
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
