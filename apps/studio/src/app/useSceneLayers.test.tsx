// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useSceneLayers } from "./useSceneLayers";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Layers = ReturnType<typeof useSceneLayers>;

describe("useSceneLayers", () => {
  let container: HTMLElement;
  let root: Root;
  const seen: Layers[] = [];
  function Probe() {
    seen.push(useSceneLayers());
    return null;
  }
  beforeEach(() => {
    seen.length = 0;
    container = document.createElement("div");
    root = createRoot(container);
    act(() => root.render(<Probe />));
  });
  afterEach(() => act(() => root.unmount()));

  const now = () => seen[seen.length - 1];
  const state = () => ({ permissions: now().showPermissions, heights: now().showHeights, events: now().showEvents });

  it("heights_and_terrain_exclude_each_other", () => {
    // The scene opens with terrain and events, without heights.
    expect(state()).toEqual({ permissions: true, heights: false, events: true });
    // Heights on: terrain goes.
    act(() => now().setShowHeights(true));
    expect(state()).toEqual({ permissions: false, heights: true, events: true });
    // Terrain on: heights go.
    act(() => now().setShowPermissions(true));
    expect(state()).toEqual({ permissions: true, heights: false, events: true });
    // Both off is allowed; events are on their own.
    act(() => now().setShowPermissions(false));
    expect(state()).toEqual({ permissions: false, heights: false, events: true });
    act(() => now().setShowHeights(true));
    act(() => now().setShowHeights(false));
    expect(state()).toEqual({ permissions: false, heights: false, events: true });
    act(() => now().setShowEvents(false));
    expect(state()).toEqual({ permissions: false, heights: false, events: false });
  });

  it("keeps_its_setters_across_renders", () => {
    const first = now();
    act(() => now().setShowHeights(true));
    expect(now().setShowHeights).toBe(first.setShowHeights);
    expect(now().setShowPermissions).toBe(first.setShowPermissions);
    expect(now().setShowEvents).toBe(first.setShowEvents);
  });
});
