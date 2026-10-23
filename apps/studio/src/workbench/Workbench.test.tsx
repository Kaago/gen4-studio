// @vitest-environment jsdom
import type { DockviewApi } from "dockview-react";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { isSaneLayout } from "./layout";

vi.mock("./panels", () => {
  const Empty = () => null;
  return { HeadersPanel: Empty, InspectorPanel: Empty, MatrixTab: Empty, StatusPanel: Empty, TextTab: Empty, WorldMapTab: Empty, WorldPanel: Empty, ScriptTab: Empty, ProblemsTab: Empty };
});

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
// jsdom has no ResizeObserver; dockview watches its container with one.
globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

import Workbench from "./Workbench";

interface Node {
  type: "leaf" | "branch";
  data: Node[] | { views: string[] };
}
/** The groups of a serialized grid in layout order, each as its list of panel ids. */
const groups = (node: Node): string[][] => (node.type === "leaf" ? [(node.data as { views: string[] }).views] : (node.data as Node[]).flatMap(groups));

describe("Workbench default layout", () => {
  let container: HTMLElement;
  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
  });
  afterEach(() => container.remove());

  it("builds_a_sane_layout_with_the_script_tab_in_the_world_group_and_problems_above_the_status_strip", async () => {
    const apiRef = { current: null as DockviewApi | null };
    const root = createRoot(container);
    await act(async () => root.render(<Workbench data={{} as never} live={{} as never} apiRef={apiRef} />));
    const json = apiRef.current!.toJSON();
    expect(groups(json.grid.root as unknown as Node)).toEqual([["headers"], ["world", "matrix", "worldmap", "text", "script"], ["problems"], ["status"], ["inspector"]]);
    expect(isSaneLayout(json)).toBe(true);
    act(() => root.unmount());
  });
});
