// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ThreadGraph from "./ThreadGraph";
import { sampleThreadGraph } from "./sampleGraph";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ThreadGraph", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  const onOpenPlace = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onSelect.mockReset();
    onOpenPlace.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (selected: string | null) =>
    act(() => root.render(<ThreadGraph graph={sampleThreadGraph} selected={selected} onSelect={onSelect} onOpenPlace={onOpenPlace} />));
  const graph = () => container.querySelector<HTMLElement>("[data-thread-graph]")!;
  const storyNode = (id: string) => container.querySelector<HTMLElement>(`[data-story-node][data-node-id="${id}"]`)!;

  it("renders_one_story_node_per_graph_node", () => {
    show(null);
    expect(graph().getAttribute("data-node-count")).toBe(String(sampleThreadGraph.nodes.length));
    expect(graph().getAttribute("data-edge-count")).toBe(String(sampleThreadGraph.edges.length));
    expect(container.querySelectorAll("[data-story-node]")).toHaveLength(sampleThreadGraph.nodes.length);
    expect(storyNode("v:1").getAttribute("data-kind")).toBe("dialog");
    expect(storyNode("v:3").hasAttribute("data-noted-only")).toBe(true);
    expect(storyNode("p:script:0200").hasAttribute("data-loose")).toBe(true);
    // No minimap; nothing to drag in 3d.6.
    expect(container.querySelector(".react-flow__minimap")).toBeNull();
    expect(container.querySelector(".react-flow__node.draggable")).toBeNull();
  });

  it("selecting_a_place_calls_onSelect", () => {
    show(null);
    act(() => storyNode("p:script:0101").click());
    expect(onSelect).toHaveBeenCalledWith("p:script:0101");
    expect(onOpenPlace).not.toHaveBeenCalled();
    // Only the selected place has the key „Öffnen“.
    expect(container.querySelector("[data-open-place]")).toBeNull();
  });

  it("the_selected_place_opens_with_the_key_oeffnen", () => {
    show("p:event:0394");
    expect(storyNode("p:event:0394").hasAttribute("data-selected")).toBe(true);
    const open = container.querySelector<HTMLButtonElement>("[data-open-place] button")!;
    expect(open.textContent).toBe("Öffnen");
    act(() => open.click());
    expect(onOpenPlace).toHaveBeenCalledWith({ kind: "event", eventFile: 394, eventKind: "coord", index: 0, header: 412 });

    // A selected value has nothing to open.
    show("v:1");
    expect(storyNode("v:1").hasAttribute("data-selected")).toBe(true);
    expect(container.querySelector("[data-open-place]")).toBeNull();
  });

  it("a_selection_change_keeps_the_measured_nodes_visible", async () => {
    // jsdom has no layout: nodes are 192 × 64, the viewport is unscaled, and the observer reports an element once, when it is first
    // observed (as a browser does on observe). A node whose size React Flow forgets is observed anew and hidden until the report comes.
    const sizes = (["offsetWidth", "offsetHeight"] as const).map((key) => [key, Object.getOwnPropertyDescriptor(HTMLElement.prototype, key)] as const);
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 192 });
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 64 });
    const observed = new WeakSet<Element>();
    let observeCalls = 0;
    class OnceObserver {
      constructor(private readonly callback: ResizeObserverCallback) {}
      observe(target: Element): void {
        observeCalls += 1;
        if (observed.has(target)) return;
        observed.add(target);
        queueMicrotask(() => act(() => this.callback([{ target } as unknown as ResizeObserverEntry], this as unknown as ResizeObserver)));
      }
      unobserve(): void {}
      disconnect(): void {}
    }
    vi.stubGlobal("ResizeObserver", OnceObserver);
    vi.stubGlobal("DOMMatrixReadOnly", class { m22 = 1; });
    // Edge labels measure their text.
    const svg = SVGElement.prototype as unknown as { getBBox?: () => DOMRect };
    const hadBBox = svg.getBBox;
    svg.getBBox = () => ({ x: 0, y: 0, width: 80, height: 12 }) as DOMRect;
    try {
      const allVisible = () => {
        const wrappers = [...container.querySelectorAll<HTMLElement>(".react-flow__node")];
        return wrappers.length === sampleThreadGraph.nodes.length && wrappers.every((w) => w.style.visibility !== "hidden");
      };
      show(null);
      await act(async () => {
        for (let i = 0; i < 5; i++) await Promise.resolve();
      });
      expect(allVisible()).toBe(true);
      const before = observeCalls;

      // Selecting a node and clearing it again leaves every node measured: none is hidden, none is observed anew.
      act(() => storyNode("p:script:0101").click());
      show("p:script:0101");
      expect(allVisible()).toBe(true);
      show(null);
      expect(allVisible()).toBe(true);
      expect(observeCalls).toBe(before);
    } finally {
      vi.unstubAllGlobals();
      if (hadBBox) svg.getBBox = hadBBox;
      else delete svg.getBBox;
      for (const [key, descriptor] of sizes) {
        if (descriptor) Object.defineProperty(HTMLElement.prototype, key, descriptor);
        else delete (HTMLElement.prototype as unknown as Record<string, unknown>)[key];
      }
    }
  });
});
