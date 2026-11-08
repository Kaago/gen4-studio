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
import { placeholderColor } from "../scene/worldMesh";
import { usePaintSession } from "./usePaintSession";

const api = vi.hoisted(() => ({ worldEdit: vi.fn(), worldGet: vi.fn(), tilesetGet: vi.fn() }));
vi.mock("../../api.world", async (original) => ({ ...(await original<typeof import("../../api.world")>()), ...api }));
const preview = vi.hoisted(() => {
  let id = 100;
  return { setStrokePreview: vi.fn(), holdStrokePreview: vi.fn(() => ++id), clearStrokePreview: vi.fn() };
});
vi.mock("../scene/strokePreview", () => preview);
const images = vi.hoisted(() => ({ tileImage: vi.fn(async () => null), peekTileImage: vi.fn((): ImageBitmap | null | undefined => undefined), resetTileImages: vi.fn() }));
vi.mock("./paletteImages", () => images);

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
const dto = (w: WorldJson): WorldGetDto => ({ map: MAP, world: w, faces: 0, textures: 0, estimate: 0, share: 0, bake: null });
const report = (req: WorldEditRequest): WorldReport => ({ map: req.map, tileset: 6, cells: 1, change: 1, bake: null });
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

  it("the_tileset_is_read_again_on_every_revision", async () => {
    await show(0);
    expect(api.tilesetGet).toHaveBeenCalledExactlyOnceWith(6);
    expect(session?.tileset?.stored).toBe(false);
    // ⌘Z of a tileset step (or tileset_save over MCP): a new revision, the same package, a new answer.
    api.tilesetGet.mockResolvedValue({ package: 6, stored: true, tileset: { version: 1, package: 6, tiles: [] } });
    await show(1);
    expect(api.tilesetGet).toHaveBeenCalledTimes(2);
    expect(session?.tileset?.stored).toBe(true);
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

  it("a_second_plus_after_the_answer_builds_on_it", async () => {
    await show();
    // After the answer the world is read again, slowly: until then the answered stroke still counts.
    let release: (d: WorldGetDto) => void = () => {};
    api.worldGet.mockImplementation(() => new Promise<WorldGetDto>((ok) => (release = ok)));
    act(() => session?.onStroke(stroke({ sub: "height", cells: [[0, 0]] })));
    await tick();
    expect(api.worldEdit.mock.calls[0][0].changes).toEqual([{ cells: [[0, 0]], height: 1 }]);
    act(() => session?.onStroke(stroke({ sub: "height", cells: [[0, 0]], preview: 2 })));
    await tick();
    expect(api.worldEdit.mock.calls[1][0].changes).toEqual([{ cells: [[0, 0]], height: 2 }]);
    const raised = world();
    raised.layers[0].heights = Array.from({ length: 32 }, (_, z) => row(Array.from({ length: 32 }, (_, x) => (x === 0 && z === 0 ? "2" : "0"))));
    await act(async () => release(dto(raised)));
    await tick();
  });

  it("pending_counts_until_the_world_is_read_again", async () => {
    await show();
    let release: (d: WorldGetDto) => void = () => {};
    api.worldGet.mockImplementation(() => new Promise<WorldGetDto>((ok) => (release = ok)));
    act(() => session?.onStroke(stroke({ preview: 9 })));
    await tick();
    // worldEdit has answered, the world is still being read: the stroke is not done yet.
    expect(api.worldEdit).toHaveBeenCalledOnce();
    expect(refresh).toHaveBeenCalledWith(MAP);
    expect(session?.pending).toBe(1);
    expect(preview.clearStrokePreview).not.toHaveBeenCalledWith(host, 9);
    await act(async () => release(dto(world())));
    await tick();
    expect(preview.clearStrokePreview).toHaveBeenCalledWith(host, 9);
    expect(session?.pending).toBe(0);
  });

  it("the_brush_preview_shows_the_texture_of_the_tile", async () => {
    const tile = (texture: string) => ({ texture, texture_index: 0, size: [32, 32] as [number, number], role: "grass" as const, kind: 2, collision: 0, height: "flat" as const, origin: "vanilla" as const });
    api.tilesetGet.mockResolvedValue({ package: 6, stored: true, tileset: { version: 1, package: 6, tiles: [tile("weg"), tile("gras_a")] } });
    state = { ...INITIAL_PAINT, tile: "0001r1" };
    await show();
    // The chosen tile's picture is asked for ahead.
    expect(images.tileImage).toHaveBeenCalledWith(6, "gras_a");
    // Not there yet: the colour of the role.
    expect(session!.look("brush", false)).toEqual({ color: placeholderColor("grass") });
    const picture = {} as ImageBitmap;
    images.peekTileImage.mockImplementation(() => picture);
    // Turned (and mirrored) as the token says, like the mesh will show it.
    expect(session!.look("brush", false)).toEqual({ texture: picture, rotate: 1, flip: false });
    expect(session!.look("rect", false)).toEqual({ texture: picture, rotate: 1, flip: false });
    expect(images.peekTileImage).toHaveBeenCalledWith(6, "gras_a");
    state = { ...INITIAL_PAINT, tile: "0001r3f" };
    await show();
    expect(session!.look("brush", false)).toEqual({ texture: picture, rotate: 3, flip: true });
    // The eraser stays hatched.
    expect(session!.look("erase", false)).toEqual({ erase: true });
    images.peekTileImage.mockImplementation(() => undefined);
  });
  it("a_throwing_clear_still_lets_the_stroke_settle", async () => {
    await show();
    preview.clearStrokePreview.mockImplementation(() => {
      throw new Error("clear");
    });
    act(() => session?.onStroke(stroke({ preview: 9 })));
    await tick();
    await tick();
    // The answered stroke left the requests on their way although clearing its preview threw.
    expect(api.worldEdit).toHaveBeenCalledOnce();
    expect(session?.pending).toBe(0);
    preview.clearStrokePreview.mockReset();
  });

  it("a_throwing_on_error_still_lets_a_refused_stroke_settle", async () => {
    onError.mockImplementation(() => {
      throw new Error("onError");
    });
    await show();
    api.worldEdit.mockRejectedValue(new Error("refused"));
    act(() => session?.onStroke(stroke({ preview: 9 })));
    await tick();
    await tick();
    expect(onError).toHaveBeenCalled();
    expect(session?.pending).toBe(0);
    api.worldEdit.mockImplementation(async (req: WorldEditRequest) => report(req));
    onError.mockReset();
  });
});
