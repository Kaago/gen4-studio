// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
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
  const getPanel = vi.fn((id: string) => (id === "headers" ? { focus: focusHeaders } : undefined));
  const dock = { current: { getPanel } };
  const docks = { welt: { schau: dock, hand: dock } } as unknown as AreaDocks;
  const setArea = vi.fn();

  function Probe() {
    scene = useSceneSync({
      bus: createEditBus(),
      setMessage: vi.fn(),
      fail: vi.fn(),
      refreshDirty: async () => {},
      setPinned: vi.fn(),
      docks,
      setArea,
      revealHand: vi.fn(),
      onOpenText: vi.fn(),
      onOpenScript: vi.fn(),
      onOpenData: vi.fn(),
    });
    return null;
  }

  beforeEach(async () => {
    for (const fn of [focusHeaders, getPanel, setArea]) fn.mockClear();
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
});
