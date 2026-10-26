// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { createEditBus } from "../bus/editBus";
import { useSceneSync } from "./useSceneSync";
import type { AreaDocks, SceneSync } from "./useSceneSync";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
  location_index: 0,
  kanto: false,
  matrix_id: id,
  area_data_id: id,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  listHeaders: vi.fn(async () => ({ headers: [header(1), header(2)] })),
  getMatrix: vi.fn(async (index: number) => ({ index, width: 1, height: 1, maps: [null], altitudes: [0] })),
  getAreaData: vi.fn(async () => ({})),
}));

describe("useSceneSync goTo", () => {
  let container: HTMLElement;
  let root: Root;
  let scene: SceneSync;
  const focusHeaders = vi.fn();
  const focusScript = vi.fn();
  // The world's one dock: a maximized group (the view's, `maximized`) hides every panel that is not in it.
  let maximized = false;
  const exitMaximizedGroup = vi.fn(() => void (maximized = false));
  const script = { focus: focusScript, api: { isMaximized: () => false } };
  const getPanel = vi.fn((id: string) => (id === "headers" ? { focus: focusHeaders, api: { isMaximized: () => false } } : id === "script" ? script : undefined));
  const dock = { current: { getPanel, hasMaximizedGroup: () => maximized, exitMaximizedGroup } };
  const docks = { welt: dock } as unknown as AreaDocks;
  const setArea = vi.fn();
  const setPinned = vi.fn();
  let depth: Depth | undefined;

  function Probe() {
    scene = useSceneSync({
      bus: createEditBus(),
      setMessage: vi.fn(),
      fail: vi.fn(),
      refreshDirty: async () => {},
      setPinned,
      docks,
      setArea,
      onOpenText: vi.fn(),
      onOpenScript: vi.fn(),
      onOpenData: vi.fn(),
      depth,
    });
    return null;
  }

  beforeEach(async () => {
    for (const fn of [focusHeaders, focusScript, getPanel, setArea, setPinned, exitMaximizedGroup]) fn.mockClear();
    maximized = false;
    depth = undefined;
    container = document.createElement("div");
    root = createRoot(container);
    act(() => root.render(<Probe />));
    await act(() => scene.syncHeaders());
  });
  afterEach(() => act(() => root.unmount()));

  it("focuses_the_panel_of_a_header_link", async () => {
    await act(async () => void (await scene.goTo({ kind: "header", id: 2 })));
    expect(scene.selected).toBe(2);
    expect(focusHeaders).toHaveBeenCalledOnce();
  });

  it("selects_without_taking_the_focus_when_asked", async () => {
    await act(async () => void (await scene.goTo({ kind: "header", id: 2 }, { focus: false })));
    expect(scene.selected).toBe(2);
    expect(setArea).toHaveBeenCalledWith("welt");
    expect(getPanel).not.toHaveBeenCalled();
    expect(focusHeaders).not.toHaveBeenCalled();
  });

  it("shows_the_world_for_a_cell_in_bauen_without_changing_a_tab", async () => {
    depth = "bauen";
    act(() => root.render(<Probe />));
    await act(async () => void (await scene.goTo({ kind: "matrixCell", matrix: 1, x: 0, y: 0 })));
    expect(setPinned).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 1, x: 0, y: 0 });
    expect(setArea).toHaveBeenCalledWith("welt");
    // No tab of the dock is looked for (Bauen has no Raster).
    expect(getPanel).not.toHaveBeenCalled();
  });

  it("focuses_a_panel_in_the_one_dock_and_ends_a_maximization_that_hides_it", async () => {
    // A link to a script while the view is maximized: the maximization ends, the script comes forward.
    maximized = true;
    await act(async () => void (await scene.goTo({ kind: "script", file: 4, line: 2 })));
    expect(setArea).toHaveBeenCalledWith("welt");
    expect(exitMaximizedGroup).toHaveBeenCalledOnce();
    expect(maximized).toBe(false);
    expect(focusScript).toHaveBeenCalledOnce();
    // Without a maximization nothing is restored; a panel in the maximized group stays maximized.
    await act(async () => void (await scene.goTo({ kind: "script", file: 5 })));
    expect(exitMaximizedGroup).toHaveBeenCalledOnce();
    maximized = true;
    script.api.isMaximized = () => true;
    await act(async () => void (await scene.goTo({ kind: "script", file: 6 })));
    script.api.isMaximized = () => false;
    expect(exitMaximizedGroup).toHaveBeenCalledOnce();
    expect(maximized).toBe(true);
    expect(focusScript).toHaveBeenCalledTimes(3);
  });
});
