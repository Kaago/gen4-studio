// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MapDto, MatrixDto } from "../../api";
import { createEditBus } from "../../bus/editBus";
import { EntityIdCache, TextProjectProvider } from "../../textproject/TextProjectContext";
import { NO_WORLD, WorkbenchProviders } from "../WorkbenchContext";
import type { MatrixSelection, WorkbenchData } from "../WorkbenchContext";
import { PaintProvider, usePaint } from "./PaintContext";
import type { PaintModel } from "./PaintContext";

const api = vi.hoisted(() => ({ worlds: new Set<number>(), getMap: vi.fn() }));
vi.mock(import("../../api"), async (original) => ({
  ...(await original()),
  getMap: api.getMap,
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** A 2 × 1 matrix: map 120 (a world map) at (0, 0), map 121 (binary) at (1, 0). */
const MATRIX = { index: 4, width: 2, height: 1, maps: [120, 121] } as unknown as MatrixDto;

describe("PaintContext", () => {
  let container: HTMLElement;
  let root: Root;
  let seen: PaintModel | null = null;
  const bus = createEditBus();
  function Probe() {
    seen = usePaint();
    return null;
  }
  beforeEach(() => {
    api.worlds = new Set([120]);
    api.getMap.mockReset();
    api.getMap.mockImplementation(async (index: number): Promise<MapDto> => ({ index, permissions: [], buildings: [], heights: [], world: api.worlds.has(index) }));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    seen = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (cell: MatrixSelection | null, textProject: boolean) => {
    const data = { matrix: MATRIX, bus } as unknown as WorkbenchData;
    await act(async () =>
      root.render(
        <TextProjectProvider active={textProject} ids={new EntityIdCache()} onRenamed={() => {}}>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: cell, message: null, world: NO_WORLD }}>
            <PaintProvider>
              <Probe />
            </PaintProvider>
          </WorkbenchProviders>
        </TextProjectProvider>,
      ),
    );
  };

  it("target_is_null_for_a_binary_map_and_in_the_dspre_layout", async () => {
    await show({ matrix: 4, x: 0, y: 0 }, true);
    expect(seen!.target).toBe(120);
    // A binary map has no target.
    await show({ matrix: 4, x: 1, y: 0 }, true);
    expect(seen!.target).toBeNull();
    // The DSPRE layout has no world maps to paint, not even for a map that says it is one.
    await show({ matrix: 4, x: 0, y: 0 }, false);
    expect(seen!.target).toBeNull();
    // No cell, or a cell of another matrix than the shown one: none either.
    await show(null, true);
    expect(seen!.target).toBeNull();
    await show({ matrix: 9, x: 0, y: 0 }, true);
    expect(seen!.target).toBeNull();
  });

  it("follows_a_map_that_becomes_a_world_map", async () => {
    await show({ matrix: 4, x: 1, y: 0 }, true);
    expect(seen!.target).toBeNull();
    api.worlds.add(121);
    await act(async () => bus.emit({ topic: "mapModelReload", map: 121 }));
    expect(seen!.target).toBe(121);
  });

  it("starts_with_the_brush_and_merges_a_patch", async () => {
    await show({ matrix: 4, x: 0, y: 0 }, true);
    expect(seen!.state.sub).toBe("brush");
    expect(seen!.state.ramp).toEqual({ dir: "e", steps: 1 });
    act(() => seen!.set({ sub: "ramp", ramp: { dir: "n", steps: 2 } }));
    expect(seen!.state.sub).toBe("ramp");
    expect(seen!.state.ramp).toEqual({ dir: "n", steps: 2 });
    expect(seen!.state.layer).toBe(0);
  });
});
