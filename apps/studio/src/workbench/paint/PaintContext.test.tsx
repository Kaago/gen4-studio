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
import { PaintProvider, frameStillWanted, usePaint } from "./PaintContext";
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

  const show = async (cell: MatrixSelection | null, textProject: boolean, tool = "select") => {
    const data = { matrix: MATRIX, bus, tool } as unknown as WorkbenchData;
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

  it("a_frame_request_waits_for_its_cell_and_goes_when_the_user_leaves_it", async () => {
    // Asked before the link and the tool arrive: nothing cancels it yet.
    await show({ matrix: 4, x: 1, y: 0 }, true);
    act(() => seen!.requestFrame!({ matrix: 4, x: 0, y: 0 }));
    expect(seen!.frame).toEqual({ matrix: 4, x: 0, y: 0 });
    // Its cell selected with „Gelände malen“: met, kept until the scene frames it.
    await show({ matrix: 4, x: 0, y: 0 }, true, "paint");
    expect(seen!.frame).toEqual({ matrix: 4, x: 0, y: 0 });
    // Another tool afterwards: dropped, the camera never jumps later.
    await show({ matrix: 4, x: 0, y: 0 }, true, "select");
    expect(seen!.frame).toBeNull();
    // Met, then another cell: dropped as well.
    act(() => seen!.requestFrame!({ matrix: 4, x: 0, y: 0 }));
    await show({ matrix: 4, x: 0, y: 0 }, true, "paint");
    await show({ matrix: 4, x: 1, y: 0 }, true, "paint");
    expect(seen!.frame).toBeNull();
  });

  it("frameStillWanted_cancels_only_after_the_request_was_met", () => {
    const frame = { matrix: 0, x: 4, y: 27 };
    expect(frameStillWanted(frame, false, null, "select")).toEqual({ wanted: true, met: false });
    expect(frameStillWanted(frame, false, { matrix: 0, x: 4, y: 27 }, "paint")).toEqual({ wanted: true, met: true });
    expect(frameStillWanted(frame, true, { matrix: 0, x: 4, y: 27 }, "select")).toEqual({ wanted: false, met: true });
    expect(frameStillWanted(frame, true, { matrix: 1, x: 4, y: 27 }, "paint")).toEqual({ wanted: false, met: true });
    expect(frameStillWanted(frame, true, null, "paint")).toEqual({ wanted: false, met: true });
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

  it("edge_set_and_rise_default_to_none_and_two", async () => {
    await show({ matrix: 4, x: 0, y: 0 }, true);
    expect(seen!.state.edgeSet).toBeNull();
    expect(seen!.state.cliffRise).toBe(2);
    act(() => seen!.set({ sub: "cliff", edgeSet: "Weg", cliffRise: 4 }));
    expect(seen!.state).toMatchObject({ sub: "cliff", edgeSet: "Weg", cliffRise: 4 });
  });

  it("removing_a_layer_moves_hidden_marks_and_the_active_layer_down", async () => {
    await show({ matrix: 4, x: 0, y: 0 }, true);
    const layer = (name: string) => ({ name, role: "overlay" as const, tiles: [] });
    const world = { version: 1, tileset: 3, layers: [{ ...layer("Boden"), role: "ground" as const }, layer("A"), layer("B"), layer("C")], permissions: { derive: true } };
    act(() => seen!.publishWorld!({ map: 120, world, faces: 0, textures: 0, estimate: 0, share: 0, bake: null }));
    act(() => seen!.set({ layer: 3 }));
    act(() => seen!.toggleHidden!(1));
    act(() => seen!.toggleHidden!(2));
    expect([...seen!.hidden!]).toEqual([1, 2]);
    // „A“ (1) goes: „B“ moves to 1 and stays hidden, „C“ (active) moves to 2.
    act(() => seen!.layerRemoved!(1));
    expect([...seen!.hidden!]).toEqual([1]);
    expect(seen!.state.layer).toBe(2);
    // The active layer itself goes: the one below it becomes active.
    act(() => seen!.layerRemoved!(2));
    expect(seen!.state.layer).toBe(1);
    expect([...seen!.hidden!]).toEqual([1]);
    // Another target forgets the hidden layers, coming back shows all.
    await show({ matrix: 4, x: 1, y: 0 }, true);
    await show({ matrix: 4, x: 0, y: 0 }, true);
    expect(seen!.hidden!.size).toBe(0);
  });
});
