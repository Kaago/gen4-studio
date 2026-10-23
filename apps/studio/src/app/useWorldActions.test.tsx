// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { addEvent, getMap, setPermission, tileBehaviors } from "../api";
import type { HeaderSummary, MatrixDto } from "../api";
import { ask } from "../platform/dialogs";
import { resetTileNames } from "../world/scene/tileNames";
import { createEditBus } from "../bus/editBus";
import type { InspectorModel } from "../workbench/Inspector";
import { useWorldActions } from "./useWorldActions";
import type { WorldActions } from "./useWorldActions";

vi.mock("../platform/dialogs", () => ({ ask: vi.fn() }));
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), addEvent: vi.fn(), getMap: vi.fn(), setPermission: vi.fn(), tileBehaviors: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const matrixOf = (index: number): MatrixDto => ({ index, name: "m", width: 1, height: 1, maps: [index] });
const tile = (absX: number, absZ: number): InspectorModel => ({ kind: "tile", map: 1, x: 0, y: 0, absX, absZ, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } });

describe("useWorldActions.lastMatrixTile", () => {
  let container: HTMLElement;
  let root: Root;
  let actions: WorldActions;
  const bus = createEditBus();
  const pins = { pinned: { kind: "none" }, setPinned: vi.fn(), setHover: vi.fn() } as never;
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => act(() => root.unmount()));

  function Probe({ matrix }: { matrix: MatrixDto | null }) {
    actions = useWorldActions({
      bus,
      pins,
      matrix,
      matrixRef: { current: matrix },
      headersRef: { current: [] },
      selectedRef: { current: null },
      setMessage: vi.fn(),
      fail: vi.fn(),
      refreshDirty: async () => {},
    });
    return null;
  }

  it("reports_the_tile_the_pointer_was_last_over_and_forgets_it_with_the_matrix", () => {
    act(() => root.render(<Probe matrix={matrixOf(0)} />));
    expect(actions.lastMatrixTile()).toBeNull();
    const reader = actions.lastMatrixTile;
    act(() => actions.onInspect(tile(12, 7)));
    expect(actions.lastMatrixTile()).toEqual({ x: 12, z: 7 });
    // Leaving the scene keeps the last tile: the wizard is opened with the pointer elsewhere.
    act(() => actions.onInspect({ kind: "none" }));
    expect(actions.lastMatrixTile()).toEqual({ x: 12, z: 7 });
    act(() => root.render(<Probe matrix={matrixOf(1)} />));
    expect(actions.lastMatrixTile()).toBeNull();
    expect(actions.lastMatrixTile).toBe(reader);
  });
});

describe("useWorldActions.onAddEvent", () => {
  let container: HTMLElement;
  let root: Root;
  let actions: WorldActions;
  const bus = createEditBus();
  const pins = { pinned: { kind: "none" }, setPinned: vi.fn(), setHover: vi.fn() } as never;
  const setMessage = vi.fn();
  const fail = vi.fn();
  const header: HeaderSummary = { id: 3, internal_name: "T01", location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: 7, text_archive_id: 0, script_file_id: 0, level_script_id: 0 };
  // Map 40 under the first cell: a plain tile at (12, 7), a door mat at (3, 4).
  const permissions = Array.from({ length: 1024 }, (_, i): [number, number] => (i === 4 * 32 + 3 ? [0x65, 0] : [0, 0]));
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
    resetTileNames();
    vi.mocked(tileBehaviors).mockReset().mockResolvedValue([{ value: 0x65, name: "WARP_ENTRANCE_SOUTH", class: "warp" }]);
    vi.mocked(getMap).mockReset().mockResolvedValue({ index: 40, permissions, buildings: [] });
    vi.mocked(addEvent).mockReset().mockResolvedValue(2);
    vi.mocked(setPermission).mockReset().mockResolvedValue(undefined);
    vi.mocked(ask).mockReset();
    setMessage.mockReset();
    fail.mockReset();
  });
  afterEach(() => act(() => root.unmount()));

  function Probe() {
    const matrix: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [40] };
    actions = useWorldActions({ bus, pins, matrix, matrixRef: { current: matrix }, headersRef: { current: [header] }, selectedRef: { current: 3 }, setMessage, fail, refreshDirty: async () => {} });
    return null;
  }
  const at = (x: number, z: number): InspectorModel => ({ kind: "tile", map: 40, x, y: z, absX: x, absZ: z, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } });

  it("offers_to_make_a_plain_tile_under_a_new_warp_a_door", async () => {
    // Gate 3b.1 (A) 9: a warp added on a snow tile was never taken in the game.
    vi.mocked(ask).mockResolvedValue(true);
    act(() => root.render(<Probe />));
    act(() => actions.onInspect(at(12, 7)));
    await act(async () => actions.onAddEvent("warp"));
    expect(addEvent).toHaveBeenCalledWith(7, "warp", 12, 7);
    expect(vi.mocked(ask).mock.calls[0][0]).toContain("ein Warp löst hier im Spiel nicht aus");
    expect(setPermission).toHaveBeenCalledWith(40, 12, 7, 0x69, 0x80);
    expect(fail).not.toHaveBeenCalled();
  });

  it("says_so_when_the_tile_is_left_as_it_is_and_asks_nothing_on_a_warp_tile", async () => {
    vi.mocked(ask).mockResolvedValue(false);
    act(() => root.render(<Probe />));
    act(() => actions.onInspect(at(12, 7)));
    await act(async () => actions.onAddEvent("warp"));
    expect(setPermission).not.toHaveBeenCalled();
    expect(setMessage).toHaveBeenCalledWith(expect.objectContaining({ kind: "info", text: expect.stringContaining("löst im Spiel nicht aus") }));
    vi.mocked(ask).mockClear();
    act(() => actions.onInspect(at(3, 4)));
    await act(async () => actions.onAddEvent("warp"));
    expect(ask).not.toHaveBeenCalled();
    // Other events never ask.
    act(() => actions.onInspect(at(12, 7)));
    await act(async () => actions.onAddEvent("object"));
    expect(ask).not.toHaveBeenCalled();
  });
});
