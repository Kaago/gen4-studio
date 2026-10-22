// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../api";
import { createEditBus } from "../bus/editBus";
import type { InspectorModel } from "../workbench/Inspector";
import { useWorldActions } from "./useWorldActions";
import type { WorldActions } from "./useWorldActions";

vi.mock("../platform/dialogs", () => ({ ask: vi.fn() }));

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
