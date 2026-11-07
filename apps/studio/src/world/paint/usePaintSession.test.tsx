// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorldEditRequest, WorldGetDto, WorldJson, WorldReport } from "../../api.world";
import { INITIAL_PAINT, PaintContext } from "../../workbench/paint/PaintContext";
import type { PaintModel, PaintState } from "../../workbench/paint/PaintContext";
import type { SceneHost } from "../scene/SceneHost";
import type { Stroke } from "../scene/strokes";
import { usePaintSession } from "./usePaintSession";

const api = vi.hoisted(() => ({ worldEdit: vi.fn(), worldGet: vi.fn(), tilesetGet: vi.fn() }));
vi.mock("../../api.world", async (original) => ({ ...(await original<typeof import("../../api.world")>()), ...api }));
const preview = vi.hoisted(() => {
  let id = 100;
  return { setStrokePreview: vi.fn(), holdStrokePreview: vi.fn(() => ++id), clearStrokePreview: vi.fn() };
});
vi.mock("../scene/strokePreview", () => preview);

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MAP = 5;
const row = (tokens: string[]) => tokens.join(" ");
/** A world whose ground is `0000` everywhere except `0041` on the first three cells of row 0 (and `0041` once more at (5, 0)). */
function world(first = "0041"): WorldJson {
  const rows = Array.from({ length: 32 }, () => Array.from({ length: 32 }, () => "0000"));
  rows[0][0] = rows[0][1] = rows[0][2] = rows[0][5] = first;
  return {
    version: 1,
    tileset: 6,
    layers: [{ name: "Boden", role: "ground", tiles: rows.map(row), heights: Array.from({ length: 32 }, () => row(Array.from({ length: 32 }, () => "0"))) }],
    permissions: { derive: true },
  };
}
const dto = (w: WorldJson): WorldGetDto => ({ map: MAP, world: w, faces: 0, textures: 0, estimate: 0, share: 0 });
const report = (req: WorldEditRequest): WorldReport => ({ map: req.map, tileset: 6, cells: 1, change: 1 });
const tick = () => act(async () => new Promise((r) => setTimeout(r, 0)));

describe("usePaintSession", () => {
  let container: HTMLElement;
  let root: Root;
  let session: ReturnType<typeof usePaintSession> | null = null;
  const host = {} as SceneHost;
  const refresh = vi.fn();
  const onError = vi.fn();
  const set = vi.fn();
  let state: PaintState = INITIAL_PAINT;

  function Probe({ revision }: { revision: number }) {
    session = usePaintSession(MAP, revision, refresh, onError, { scene: () => host });
    return null;
  }
  const show = async (revision = 0) => {
    const model: PaintModel = { state, set, target: MAP };
    await act(async () =>
      root.render(
        <PaintContext.Provider value={model}>
          <Probe revision={revision} />
        </PaintContext.Provider>,
      ),
    );
    await tick();
  };
  const stroke = (over: Partial<Stroke>): Stroke => ({ map: MAP, sub: "brush", cells: [[0, 0]], lower: false, preview: 1, ...over });

  beforeEach(() => {
    vi.clearAllMocks();
    state = { ...INITIAL_PAINT, tile: "0099" };
    api.worldGet.mockResolvedValue(dto(world()));
    api.tilesetGet.mockResolvedValue({ package: 6, stored: false, tileset: { version: 1, package: 6, tiles: [] } });
    api.worldEdit.mockImplementation(async (req: WorldEditRequest) => report(req));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    session = null;
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("one_stroke_is_one_world_edit_with_its_label", async () => {
    await show();
    act(() => session?.onStroke(stroke({ cells: [[3, 3], [4, 3]] })));
    await tick();
    expect(api.worldEdit).toHaveBeenCalledOnce();
    expect(api.worldEdit).toHaveBeenCalledWith({ map: MAP, label: "Gemalt: Karte 0005 (2 Zellen)", changes: [{ layer: 0, cells: [[3, 3], [4, 3]], tile: "0099" }] });
  });

  it("a_second_stroke_waits_for_the_first", async () => {
    const answers: (() => void)[] = [];
    api.worldEdit.mockImplementation((req: WorldEditRequest) => new Promise<WorldReport>((ok) => answers.push(() => ok(report(req)))));
    await show();
    act(() => {
      session?.onStroke(stroke({ cells: [[3, 3]], preview: 1 }));
      session?.onStroke(stroke({ cells: [[4, 4]], preview: 2 }));
    });
    await tick();
    expect(api.worldEdit).toHaveBeenCalledOnce();
    expect(session?.pending).toBe(2);
    await act(async () => answers[0]());
    await tick();
    expect(api.worldEdit).toHaveBeenCalledTimes(2);
    expect(api.worldEdit.mock.calls[1][0].changes[0].cells).toEqual([[4, 4]]);
    // The first stroke's preview went with its answer, the second one's stays until its own.
    expect(preview.clearStrokePreview).toHaveBeenCalledWith(host, 1);
    expect(preview.clearStrokePreview).not.toHaveBeenCalledWith(host, 2);
    expect(session?.pending).toBe(1);
    await act(async () => answers[1]());
    await tick();
    expect(preview.clearStrokePreview).toHaveBeenCalledWith(host, 2);
    expect(session?.pending).toBe(0);
  });

  it("a_refused_stroke_clears_its_preview", async () => {
    const refusal = { code: "WORLD_TILE_UNKNOWN", message: "Kachel 0099 gibt es im Tileset 0006 nicht." };
    api.worldEdit.mockRejectedValue(refusal);
    await show();
    expect(api.worldGet).toHaveBeenCalledTimes(1);
    act(() => session?.onStroke(stroke({ preview: 7 })));
    await tick();
    expect(preview.clearStrokePreview).toHaveBeenCalledWith(host, 7);
    expect(onError).toHaveBeenCalledWith(refusal);
    expect(api.worldGet).toHaveBeenCalledTimes(2);
    expect(refresh).not.toHaveBeenCalled();
  });

  it("fill_uses_the_world_read_last", async () => {
    await show();
    api.worldGet.mockResolvedValue(dto(world("0042")));
    await show(1);
    act(() => session?.onStroke(stroke({ sub: "fill", cells: [[1, 0]] })));
    await tick();
    // The 4-connected run of `0042` from (1, 0): (0, 0) to (2, 0), not the lone one at (5, 0).
    const req = api.worldEdit.mock.calls[0][0] as WorldEditRequest;
    expect(req.label).toBe("Gefüllt: Karte 0005 (3 Zellen)");
    expect([...req.changes[0].cells].sort()).toEqual([[0, 0], [1, 0], [2, 0]]);
    expect(preview.setStrokePreview).toHaveBeenCalledWith(host, MAP, req.changes[0].cells, expect.anything());
  });

  it("after_the_answer_mesh_heights_and_world_are_read_again", async () => {
    await show();
    const order: string[] = [];
    refresh.mockImplementation(() => order.push("refresh"));
    api.worldGet.mockImplementation(async () => {
      order.push("world_get");
      return dto(world());
    });
    preview.clearStrokePreview.mockImplementation(() => order.push("clear"));
    act(() => session?.onStroke(stroke({})));
    await tick();
    expect(refresh).toHaveBeenCalledWith(MAP);
    expect(order).toEqual(["refresh", "world_get", "clear"]);
    preview.clearStrokePreview.mockReset();
  });

  it("the_world_is_read_again_on_every_revision", async () => {
    await show(0);
    expect(api.worldGet).toHaveBeenCalledTimes(1);
    // ⌘Z or a world_edit over MCP: a new revision without a stroke.
    api.worldGet.mockResolvedValue(dto(world("0077")));
    await show(1);
    expect(api.worldGet).toHaveBeenCalledTimes(2);
    expect(session?.world?.world?.layers[0].tiles[0].startsWith("0077")).toBe(true);
    act(() => session?.onStroke(stroke({ sub: "fill", cells: [[0, 0]] })));
    await tick();
    expect((api.worldEdit.mock.calls[0][0] as WorldEditRequest).changes[0].cells).toHaveLength(3);
  });

  it("a_height_stroke_at_the_limit_says_so", async () => {
    const top = world();
    top.layers[0].heights = Array.from({ length: 32 }, () => row(Array.from({ length: 32 }, () => "62")));
    api.worldGet.mockResolvedValue(dto(top));
    const notices: string[] = [];
    function Notices() {
      session = usePaintSession(MAP, 0, refresh, onError, { scene: () => host, onNotice: (t) => notices.push(t) });
      return null;
    }
    await act(async () => root.render(<Notices />));
    await tick();
    act(() => session?.onStroke(stroke({ sub: "height", cells: [[0, 0], [1, 0]] })));
    await tick();
    expect(api.worldEdit).not.toHaveBeenCalled();
    expect(notices).toEqual(["2 Zellen schon an der Grenze (−16 bis 62)."]);
    expect(preview.clearStrokePreview).toHaveBeenCalledWith(host, 1);
  });

  it("pick_takes_the_tile_and_goes_back_to_the_brush", async () => {
    state = { ...INITIAL_PAINT, sub: "pick" };
    await show();
    act(() => session?.onPick(MAP, [1, 0]));
    const update = set.mock.calls[0][0] as (s: PaintState) => Partial<PaintState>;
    expect(update({ ...INITIAL_PAINT, sub: "pick" })).toEqual({ tile: "0041", sub: "brush" });
    expect(update({ ...INITIAL_PAINT, sub: "height" })).toEqual({ tile: "0041" });
  });
});
