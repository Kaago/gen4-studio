import { beforeEach, describe, expect, it, vi } from "vitest";
import { setPermission } from "../../api";
import type { Tool } from "../../ui/frame/shellState";
import { BLOCKED } from "../paint";
import { attachCanvasClicks } from "./canvasClicks";
import { fakePicker, fakeRefs, settle, surfaceKit } from "./dragTestKit.testkit";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { SceneCallbacks } from "./types";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), setPermission: vi.fn() }));
vi.mock("./content", () => ({ recolourMap: vi.fn() }));

const MAP = 5;
const data = { map: { index: MAP, permissions: Array.from({ length: 1024 }, () => [0x02, 0x00] as [number, number]), buildings: [] } };
/** The tile (3, 4) of the map in matrix cell (column 1, row 2). */
const tile = { mapId: MAP, data, col: 1, row: 2, x: 3, y: 4 };
const MARKER = { kind: "object", data: { index: 6 } } as never;
const BUILDING = { ref: { map: MAP, index: 2 } };

function setup(tool: Tool, over: { picked?: boolean; marker?: boolean; building?: boolean; overlay?: boolean; callbacks?: Partial<SceneCallbacks> } = {}) {
  const h = {
    canvas: undefined as unknown,
    events: { event_file_id: 9 },
    buildings: { pick: vi.fn(() => (over.building ? BUILDING : null)) },
    overlay: { visible: over.overlay ?? true },
    loaded: new Map([[MAP, data]]),
    render: vi.fn(),
    refreshHover: vi.fn(),
  } as unknown as SceneHost;
  const kit = surfaceKit();
  (h as unknown as { canvas: unknown }).canvas = kit.canvas;
  const picker: Picker = fakePicker();
  vi.mocked(picker.markerAt).mockReturnValue(over.marker ? MARKER : null);
  vi.mocked(picker.pickTile).mockReturnValue(over.picked === false ? null : (tile as never));
  const { refs, callbacks } = fakeRefs({ tool, ...over.callbacks });
  const detach = attachCanvasClicks(h, picker, refs);
  const click = (shift = false) => {
    const down = { clientX: 100, clientY: 100, button: 0, shiftKey: shift };
    kit.canvas.emit("pointerdown", down);
    kit.canvas.emit("pointerup", down);
  };
  return { h, callbacks, picker, click, detach, kit };
}

beforeEach(() => {
  vi.mocked(setPermission).mockReset().mockResolvedValue(undefined as never);
});

describe("attachCanvasClicks by tool", () => {
  it("select_never_paints_and_adds_nothing_on_a_free_tile", async () => {
    const { callbacks, click } = setup("select");
    click();
    await settle();
    expect(setPermission).not.toHaveBeenCalled();
    expect(callbacks.onTileClick).not.toHaveBeenCalled();
    expect(callbacks.onSelectEvent).not.toHaveBeenCalled();
    expect(callbacks.onSelectBuilding).not.toHaveBeenCalled();
    expect(callbacks.onEdited).not.toHaveBeenCalled();
  });

  it("select_chooses_the_marker_under_the_click_before_a_building_below_it", () => {
    const { callbacks, click } = setup("select", { marker: true, building: true });
    click();
    expect(callbacks.onSelectEvent).toHaveBeenCalledWith({ file: 9, kind: "object", index: 6 });
    expect(callbacks.onSelectBuilding).not.toHaveBeenCalled();
    expect(setPermission).not.toHaveBeenCalled();
  });

  it("select_chooses_a_building_when_no_marker_is_there", () => {
    const { callbacks, click } = setup("select", { building: true });
    click();
    expect(callbacks.onSelectBuilding).toHaveBeenCalledWith(BUILDING.ref);
    expect(callbacks.onSelectEvent).not.toHaveBeenCalled();
    expect(setPermission).not.toHaveBeenCalled();
  });

  it("paint_sets_collision_on_the_tile_and_shift_clears_it", async () => {
    const { callbacks, click } = setup("paint");
    click();
    expect(setPermission).toHaveBeenLastCalledWith(MAP, 3, 4, 0x02, BLOCKED);
    await settle();
    expect(callbacks.onEdited).toHaveBeenCalledOnce();
    expect(data.map.permissions[4 * 32 + 3]).toEqual([0x02, BLOCKED]);
    click(true);
    expect(setPermission).toHaveBeenLastCalledWith(MAP, 3, 4, 0x02, 0x00);
    expect(callbacks.onTileClick).not.toHaveBeenCalled();
  });

  it("paint_still_selects_a_marker_or_building_under_the_click_instead_of_painting", () => {
    const { callbacks, click } = setup("paint", { marker: true });
    click();
    expect(callbacks.onSelectEvent).toHaveBeenCalledOnce();
    expect(setPermission).not.toHaveBeenCalled();
  });

  it("paint_does_nothing_while_edits_are_off_or_the_terrain_layer_is_hidden", () => {
    setup("paint", { callbacks: { paintEnabled: false } }).click();
    setup("paint", { overlay: false }).click();
    setup("paint", { picked: false }).click();
    expect(setPermission).not.toHaveBeenCalled();
  });

  it.each<Tool>(["figure", "door", "trigger", "sign", "building"])("%s_reports_the_tile_and_neither_paints_nor_selects", (tool) => {
    const { callbacks, click } = setup(tool, { marker: true, building: true });
    click();
    expect(callbacks.onTileClick).toHaveBeenCalledOnce();
    expect(callbacks.onTileClick).toHaveBeenCalledWith({ mapId: MAP, x: 3, y: 4, absX: 35, absZ: 68 });
    expect(callbacks.onSelectEvent).not.toHaveBeenCalled();
    expect(callbacks.onSelectBuilding).not.toHaveBeenCalled();
    expect(setPermission).not.toHaveBeenCalled();
  });

  it("an_adding_tool_ignores_a_click_off_the_map_while_edits_are_off_and_a_drag", () => {
    const off = setup("door", { picked: false });
    off.click();
    const busy = setup("door", { callbacks: { paintEnabled: false } });
    busy.click();
    expect(off.callbacks.onTileClick).not.toHaveBeenCalled();
    expect(busy.callbacks.onTileClick).not.toHaveBeenCalled();
    const drag = setup("door");
    drag.kit.canvas.emit("pointerdown", { clientX: 100, clientY: 100, button: 0 });
    drag.kit.canvas.emit("pointerup", { clientX: 140, clientY: 100, button: 0 });
    expect(drag.callbacks.onTileClick).not.toHaveBeenCalled();
  });

  it("an_adding_tool_works_with_the_terrain_layer_hidden", () => {
    const { callbacks, click } = setup("figure", { overlay: false });
    click();
    expect(callbacks.onTileClick).toHaveBeenCalledOnce();
  });
});
