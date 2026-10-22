import { beforeEach, describe, expect, it, vi } from "vitest";
import type { BuildingDto } from "../../api";
import { updateBuilding } from "../../api";
import type { BuildingRef } from "../buildingLayer";
import { createBuildingDrag } from "./buildingDrag";
import { fakePicker, fakeRefs, settle, surfaceKit } from "./dragTestKit.testkit";
import type { SceneHost } from "./SceneHost";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), updateBuilding: vi.fn() }));

const building: BuildingDto = { model_id: 1, x: 4, y: 0, z: 6, sx: 1, sy: 1, sz: 1, rotation: ["00000000", "00000000", "00000000"] };
const selected: BuildingRef = { map: 3, index: 0 };

/** A scene whose building layer "hits" the given building under the pointer. */
function scene(hit: { ref: BuildingRef } | null, cellHeight = 0) {
  const show = vi.fn();
  const h = {
    selection: selected,
    loaded: new Map([[3, { map: { index: 3, permissions: [], buildings: [building] } }]]),
    buildings: { pick: () => (hit ? { ref: hit.ref, cell: { x: 0, y: cellHeight, z: 0 }, building } : null), show },
    render: vi.fn(),
    refreshHover: vi.fn(),
  } as unknown as SceneHost;
  return { h, show };
}

function setup(hit: { ref: BuildingRef } | null, paintEnabled = true, cellHeight = 0) {
  const { h, show } = scene(hit, cellHeight);
  const { refs, callbacks } = fakeRefs({ paintEnabled });
  const picker = fakePicker();
  const planePoint = vi.spyOn(picker, "planePoint");
  const kit = surfaceKit();
  const drag = createBuildingDrag(h, picker, kit.surface, refs);
  drag.attach();
  return { h, show, callbacks, picker, drag, planePoint, ...kit };
}

beforeEach(() => {
  vi.mocked(updateBuilding).mockReset().mockResolvedValue(undefined as never);
});

describe("createBuildingDrag", () => {
  it("only begins on the selected building", () => {
    const other = setup({ ref: { map: 3, index: 1 } });
    other.press(100, 100);
    expect(other.drag.dragging).toBe(false);
    const none = setup(null);
    none.press(100, 100);
    expect(none.drag.dragging).toBe(false);
    const own = setup({ ref: selected });
    own.press(100, 100);
    expect(own.drag.dragging).toBe(true);
  });

  it("does not begin while painting is disabled or an event marker is over the building", () => {
    const off = setup({ ref: selected }, false);
    off.press(100, 100);
    expect(off.drag.dragging).toBe(false);
    const covered = setup({ ref: selected });
    vi.mocked(covered.picker.markerAt).mockReturnValue({ kind: "warp", data: { index: 0, x: 0, z: 0 } } as never);
    covered.press(100, 100);
    expect(covered.drag.dragging).toBe(false);
  });

  it("previews the move and writes it once when the pointer is released", async () => {
    const t = setup({ ref: selected });
    t.press(100, 100);
    t.moveTo(103, 100);
    t.moveTo(106, 100);
    expect(updateBuilding).not.toHaveBeenCalled();
    t.release(106, 100);
    // Centre of the map cell is (16, 16): the building's x/z are relative to it; 6 tiles right of x = 4.
    expect(updateBuilding).toHaveBeenCalledTimes(1);
    expect(updateBuilding).toHaveBeenCalledWith(3, 0, { x: 10, z: 6 });
    t.release(106, 100);
    expect(updateBuilding).toHaveBeenCalledTimes(1);
    await settle();
    expect(t.callbacks.onBuildingMoved).toHaveBeenCalledWith(3);
  });

  it("drags on the plane of its own cell's height", () => {
    const raised = setup({ ref: selected }, true, 1.5);
    raised.press(100, 100);
    // The building itself stands at y 0 over a cell raised by 1.5.
    expect(raised.planePoint.mock.calls[0][2].constant).toBeCloseTo(-1.5, 5);
    const flat = setup({ ref: selected });
    flat.press(100, 100);
    expect(flat.planePoint.mock.calls[0][2].constant).toBeCloseTo(-0, 5);
  });

  it("puts the building back on Escape and writes nothing", () => {
    const t = setup({ ref: selected });
    t.press(100, 100);
    t.moveTo(106, 100);
    t.escape();
    expect(updateBuilding).not.toHaveBeenCalled();
    expect(t.show).toHaveBeenLastCalledWith(selected, building);
    expect(t.drag.dragging).toBe(false);
  });

  it("restores the building and reports when the write is refused", async () => {
    vi.mocked(updateBuilding).mockRejectedValue(new Error("nope"));
    const t = setup({ ref: selected });
    t.press(100, 100);
    t.moveTo(106, 100);
    t.release(106, 100);
    await settle();
    expect(t.show).toHaveBeenLastCalledWith(selected, building);
    expect(t.callbacks.onError).toHaveBeenCalledTimes(1);
    expect(t.callbacks.onBuildingMoved).not.toHaveBeenCalled();
  });
});
