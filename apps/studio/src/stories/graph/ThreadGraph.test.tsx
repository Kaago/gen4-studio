// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ThreadGraph, { FIT_MIN_ZOOM } from "./ThreadGraph";
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

  /**
   * React Flow measured in jsdom: nodes 192 × 64, an unscaled viewport, sizes reported once; `run` sees the measured graph. The box of
   * the graph (`.react-flow`) is `pane` when given, else as small as a node.
   */
  async function measured(run: () => Promise<void>, pane?: { width: number; height: number }) {
    const sizes = (["offsetWidth", "offsetHeight"] as const).map((key) => [key, Object.getOwnPropertyDescriptor(HTMLElement.prototype, key)] as const);
    const isPane = (el: HTMLElement) => pane !== undefined && el.classList.contains("react-flow__renderer");
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get(this: HTMLElement) { return isPane(this) ? pane!.width : 192; } });
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get(this: HTMLElement) { return isPane(this) ? pane!.height : 64; } });
    const observed = new WeakSet<Element>();
    // What one observer is given in one task it reports together, as a browser does (React Flow fits once its nodes are measured).
    class OnceObserver {
      private pending: Element[] = [];
      constructor(private readonly callback: ResizeObserverCallback) {}
      observe(target: Element): void {
        if (observed.has(target)) return;
        observed.add(target);
        this.pending.push(target);
        if (this.pending.length > 1) return;
        queueMicrotask(() => {
          const entries = this.pending.map((t) => ({ target: t }) as unknown as ResizeObserverEntry);
          this.pending = [];
          act(() => this.callback(entries, this as unknown as ResizeObserver));
        });
      }
      unobserve(): void {}
      disconnect(): void {}
    }
    vi.stubGlobal("ResizeObserver", OnceObserver);
    vi.stubGlobal("DOMMatrixReadOnly", class { m22 = 1; });
    const svg = SVGElement.prototype as unknown as { getBBox?: () => DOMRect };
    const hadBBox = svg.getBBox;
    svg.getBBox = () => ({ x: 0, y: 0, width: 80, height: 12 }) as DOMRect;
    try {
      await run();
    } finally {
      vi.unstubAllGlobals();
      if (hadBBox) svg.getBBox = hadBBox;
      else delete svg.getBBox;
      for (const [key, descriptor] of sizes) {
        if (descriptor) Object.defineProperty(HTMLElement.prototype, key, descriptor);
        else delete (HTMLElement.prototype as unknown as Record<string, unknown>)[key];
      }
    }
  }
  const settle = () =>
    act(async () => {
      for (let i = 0; i < 5; i++) await Promise.resolve();
    });
  const scale = () => Number(/scale\(([\d.]+)\)/.exec(container.querySelector<HTMLElement>(".react-flow__viewport")!.style.transform)![1]);

  it("a_graph_larger_than_its_box_is_fitted_no_smaller_than_readable", async () => {
    // HeartGold's VAR_UNOWN_REPORT_LEVEL (21 nodes) was fitted to nodes 50 px wide (gate 3d.6, U3): fitting stops at FIT_MIN_ZOOM, where
    // the node's text stays readable, and the rest is panned to. Here the box is as small as one node.
    await measured(async () => {
      show(null);
      await settle();
      expect(scale()).toBe(FIT_MIN_ZOOM);
    });
  });

  it("a_small_graph_in_a_large_box_is_fitted_as_before", async () => {
    await measured(
      async () => {
        show(null);
        await settle();
        // Never larger than 1:1, as before.
        expect(scale()).toBe(1);
      },
      { width: 2400, height: 1200 },
    );
  });

  const placed = { ...sampleThreadGraph, layout: { nodes: { s1: { x: 10, y: 20 }, gone: { x: 1, y: 2 } } } };

  it("with_onLayout_a_drag_end_hands_over_the_layout", async () => {
    const onLayout = vi.fn();
    await measured(async () => {
      // Without onLayout (a thread without a story) nothing moves; with it every node does.
      show(null);
      expect(container.querySelector(".react-flow__node.draggable")).toBeNull();
      act(() => root.render(<ThreadGraph graph={placed} selected={null} onSelect={onSelect} onOpenPlace={onOpenPlace} onLayout={onLayout} />));
      await settle();
      const wrapper = container.querySelector<HTMLElement>('.react-flow__node[data-id="v:1"]')!;
      expect(wrapper.classList.contains("draggable")).toBe(true);
      // A drag of the value node with step s1 by (30, 15): mouse down on it, moves on the window, up.
      // d3-drag follows the mouse on the event's view (the window).
      const at = (type: string, x: number, y: number, target: EventTarget) => {
        const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0 });
        Object.defineProperty(event, "view", { value: window });
        target.dispatchEvent(event);
      };
      await act(async () => {
        at("mousedown", 100, 100, storyNode("v:1"));
        at("mousemove", 110, 105, window);
        at("mousemove", 130, 115, window);
        at("mouseup", 130, 115, window);
        for (let i = 0; i < 5; i++) await Promise.resolve();
      });
      expect(onLayout).toHaveBeenCalledOnce();
      const layout = onLayout.mock.calls[0]![0] as { nodes: Record<string, { x: number; y: number }> };
      // By the step's key on a value with a step, by the node's id otherwise; a key the graph no longer shows stays.
      // The node lies where the drag left it (right of and below its place in the side file), and the layout says so.
      const shown = /translate\(([-\d.]+)px, ?([-\d.]+)px\)/.exec(wrapper.style.transform)!;
      expect(layout.nodes.s1).toEqual({ x: Math.round(Number(shown[1])), y: Math.round(Number(shown[2])) });
      expect(layout.nodes.s1!.x).toBeGreaterThan(10);
      expect(layout.nodes.s1!.y).toBeGreaterThan(20);
      expect(Object.keys(layout.nodes)).toEqual(expect.arrayContaining(["s1", "s2", "s3", "v:4", "p:script:0101", "p:event:0394", "gone"]));
      expect(layout.nodes.gone).toEqual({ x: 1, y: 2 });
    });
  });

  it("an_arrow_key_on_the_selected_node_hands_over_the_layout_too", async () => {
    const onLayout = vi.fn();
    await measured(async () => {
      act(() => root.render(<ThreadGraph graph={placed} selected="v:1" onSelect={onSelect} onOpenPlace={onOpenPlace} onLayout={onLayout} />));
      await settle();
      const wrapper = container.querySelector<HTMLElement>('.react-flow__node[data-id="v:1"]')!;
      // The description says that the keys move a node of a story's graph.
      expect(document.getElementById(wrapper.getAttribute("aria-describedby") ?? "")?.textContent).toContain("Pfeiltasten verschieben");
      // React Flow moves a selected node 5 px per arrow key: the move is saved like a drag's end.
      await act(async () => {
        wrapper.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }));
        for (let i = 0; i < 5; i++) await Promise.resolve();
      });
      expect(onLayout).toHaveBeenCalledOnce();
      const layout = onLayout.mock.calls[0]![0] as { nodes: Record<string, { x: number; y: number }> };
      expect(layout.nodes.s1).toEqual({ x: 15, y: 20 });
      expect(layout.nodes.gone).toEqual({ x: 1, y: 2 });
    });
    // Without a story nothing moves and the description says nothing of the keys.
    act(() => root.render(<ThreadGraph graph={sampleThreadGraph} selected="v:1" onSelect={onSelect} onOpenPlace={onOpenPlace} />));
    const wrapper = container.querySelector<HTMLElement>('.react-flow__node[data-id="v:1"]');
    expect(document.getElementById(wrapper?.getAttribute("aria-describedby") ?? "")?.textContent ?? "").not.toContain("Pfeiltasten");
  });
});
