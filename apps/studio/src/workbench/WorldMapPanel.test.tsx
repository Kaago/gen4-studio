// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../api";
import { getMatrix } from "../api";
import { mapsText } from "./places";
import { ThumbnailCache } from "../world/thumbnails";
import { WorldMapPanel } from "./WorldMapPanel";
import type { WorldMapContext } from "./WorldMapPanel";

// Counts how often a room card renders (it calls mapsText on every render).
vi.mock("./places", async (importOriginal) => {
  const original = await importOriginal<typeof import("./places")>();
  return { ...original, mapsText: vi.fn(original.mapsText) };
});
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMatrix: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, location_name: string, matrix_id: number, event_file_id = 10 + id): HeaderSummary => ({
  id,
  internal_name: `H${id}`, location_index: 0, kanto: false,
  location_name,
  matrix_id,
  area_data_id: 0,
  event_file_id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const summary = (index: number, width: number, height: number, has_headers = false): MatrixSummary => ({
  index,
  name: `m${index}`,
  width,
  height,
  has_headers,
  has_altitudes: false,
  filled: width * height,
});

const headers = [header(0, "Zweiblattdorf", 0), header(1, "Zweiblattdorf", 7), header(2, "Zweiblattdorf", 8), header(3, "Route 201", 0)];
const withRegion = [summary(0, 2, 1, true), summary(7, 1, 1), summary(8, 2, 2)];
const withoutRegion = [summary(0, 2, 1), summary(7, 1, 1), summary(8, 2, 2)];
const region: MatrixDto = { index: 0, name: "r", width: 2, height: 1, maps: [100, 101], headers: [0, 3] };
/** Header 1 leaves to the door of header 0 at tile (40, 5), i.e. cell (1, 0); header 2 has no exit. */
const links: WarpLink[] = [
  { event_file: 11, index: 0, x: 3, z: 3, dest_header_id: 0, dest_warp_id: 2 },
  { event_file: 10, index: 2, x: 40, z: 5, dest_header_id: 1, dest_warp_id: 0 },
];

describe("WorldMapPanel", () => {
  let container: HTMLElement;
  let root: Root;
  let cache: ThumbnailCache;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    cache = new ThumbnailCache({ glb: async () => new ArrayBuffer(2) }, async () => "data:image/png;base64,AAAA");
    vi.mocked(getMatrix).mockReset();
    vi.mocked(getMatrix).mockResolvedValue(region);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const context = (extra: Partial<WorldMapContext> = {}): WorldMapContext => ({
    headers,
    matrices: withRegion,
    warpLinks: links,
    matricesLoaded: true,
    matricesFailed: false,
    selected: 0,
    areaDataId: 1,
    revision: 0,
    goTo: vi.fn(),
    onError: vi.fn(),
    onLoadStatus: vi.fn(),
    cache,
    ...extra,
  });
  const show = async (ctx: WorldMapContext) => {
    act(() => root.render(<WorldMapPanel context={ctx} />));
    await flush();
  };

  it("says_there_is_no_region_and_still_lists_the_places", async () => {
    await show(context({ matrices: withoutRegion }));
    expect(container.textContent).toContain("Keine Region gefunden (kein Raster mit Header-Abschnitt).");
    expect(container.querySelector("[data-cell]")).toBeNull();
    expect(container.querySelector('[data-place-key="Zweiblattdorf"]')).not.toBeNull();
    expect(container.querySelector('[data-place-key="Route 201"]')).not.toBeNull();
    expect(getMatrix).not.toHaveBeenCalled();
  });

  it("says_the_overview_is_loading_instead_of_claiming_there_is_no_region", async () => {
    await show(context({ matrices: [], matricesLoaded: false }));
    expect(container.textContent).not.toContain("Keine Region gefunden");
    expect(container.textContent).toContain("Lade Übersicht");
  });

  it("clicking_a_region_cell_opens_its_header", async () => {
    const ctx = context();
    await show(ctx);
    expect(getMatrix).toHaveBeenCalledWith(0);
    act(() => container.querySelector<HTMLElement>('[data-cell="1,0"]')?.click());
    expect(ctx.goTo).toHaveBeenCalledWith({ kind: "header", id: 3 });
  });

  it("clicking_a_room_card_opens_the_room", async () => {
    const ctx = context();
    await show(ctx);
    act(() => container.querySelector<HTMLElement>('button[data-room-id="1"]')?.click());
    expect(ctx.goTo).toHaveBeenCalledWith({ kind: "header", id: 1 });
  });

  it("opens_the_place_of_the_selected_header_and_follows_a_new_selection", async () => {
    await show(context({ selected: 0 }));
    const open = (key: string) => container.querySelector<HTMLDetailsElement>(`[data-place-key="${key}"]`)?.open;
    expect(open("Zweiblattdorf")).toBe(true);
    expect(open("Route 201")).toBe(false);
    await show(context({ selected: 3 }));
    expect(open("Route 201")).toBe(true);
  });

  it("draws_a_line_only_for_rooms_whose_entrance_is_known_and_greys_the_others", async () => {
    await show(context());
    expect(container.querySelector('line[data-room-id="1"]')).not.toBeNull();
    expect(container.querySelector('line[data-room-id="2"]')).toBeNull();
    const grey = container.querySelector<HTMLElement>('button[data-room-id="2"]');
    expect(grey?.textContent).toContain("ohne Eingang");
    expect(grey?.textContent).toContain("kein Ausgang");
    expect(grey?.title).toContain("kein Ausgang");
    expect(container.querySelector('button[data-room-id="1"]')?.textContent).not.toContain("ohne Eingang");
  });

  it("draws_lines_only_for_the_place_of_the_selected_header", async () => {
    const more = [...headers, header(5, "Route 201", 7)];
    // Room 5 leaves to the door of header 3 (event file 13) at tile (5, 5), i.e. cell (0, 0).
    const warps = [...links, { event_file: 15, index: 0, x: 1, z: 1, dest_header_id: 3, dest_warp_id: 1 }, { event_file: 13, index: 1, x: 5, z: 5, dest_header_id: 5, dest_warp_id: 0 }];
    const lineIds = () => [...container.querySelectorAll("line[data-room-id]")].map((l) => l.getAttribute("data-room-id"));
    await show(context({ headers: more, warpLinks: warps, selected: 3 }));
    expect(lineIds()).toEqual(["5"]);
    // The place of header 3 stays open, but the selection moved to the other one.
    await show(context({ headers: more, warpLinks: warps, selected: 0 }));
    expect(container.querySelector<HTMLDetailsElement>('[data-place-key="Route 201"]')?.open).toBe(true);
    expect(lineIds()).toEqual(["1"]);
  });

  it("measures_the_lines_again_when_a_place_above_the_selected_one_opens_or_closes", async () => {
    await show(context({ selected: 3 }));
    const measured = vi.spyOn(Element.prototype, "getBoundingClientRect");
    try {
      const other = container.querySelector<HTMLDetailsElement>('[data-place-key="Zweiblattdorf"]');
      expect(other?.open).toBe(false);
      measured.mockClear();
      // The user opens the other place: the selected place's cards move, nothing else changes.
      act(() => {
        if (other) other.open = true;
        other?.dispatchEvent(new Event("toggle"));
      });
      expect(other?.open).toBe(true);
      expect(measured).toHaveBeenCalled();
    } finally {
      measured.mockRestore();
    }
  });

  it("panning_and_zooming_do_not_render_the_place_list_again", async () => {
    await show(context());
    const renders = vi.mocked(mapsText).mock.calls.length;
    expect(renders).toBeGreaterThan(0);
    const viewport = container.querySelector<HTMLElement>("[data-world-viewport]");
    act(() => void viewport?.dispatchEvent(new WheelEvent("wheel", { deltaY: -200, bubbles: true, cancelable: true })));
    act(() => void viewport?.dispatchEvent(new WheelEvent("wheel", { deltaY: -200, bubbles: true, cancelable: true })));
    expect(vi.mocked(mapsText).mock.calls.length).toBe(renders);
  });

  it("zooms_with_the_wheel_within_its_limits", async () => {
    await show(context());
    const viewport = container.querySelector<HTMLElement>("[data-world-viewport]");
    const scaleOf = () => Number(/scale\(([\d.]+)\)/.exec(container.querySelector<HTMLElement>("[data-world-grid]")?.style.transform ?? "")?.[1]);
    expect(scaleOf()).toBe(1);
    act(() => void viewport?.dispatchEvent(new WheelEvent("wheel", { deltaY: -200, bubbles: true, cancelable: true })));
    expect(scaleOf()).toBeGreaterThan(1);
    for (let i = 0; i < 40; i++) act(() => void viewport?.dispatchEvent(new WheelEvent("wheel", { deltaY: -500, bubbles: true, cancelable: true })));
    expect(scaleOf()).toBe(3);
    for (let i = 0; i < 80; i++) act(() => void viewport?.dispatchEvent(new WheelEvent("wheel", { deltaY: 500, bubbles: true, cancelable: true })));
    expect(scaleOf()).toBe(0.5);
  });

  it("reports_the_picture_progress_only_beyond_fifty_pending_and_clears_it_at_the_end", async () => {
    const big: MatrixDto = { index: 0, name: "r", width: 10, height: 6, maps: Array.from({ length: 60 }, (_, i) => 100 + i), headers: new Array<number>(60).fill(0) };
    vi.mocked(getMatrix).mockResolvedValue(big);
    // The renders stay open until released, so the queue is long while the panel polls.
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => (release = resolve));
    cache = new ThumbnailCache({ glb: async () => (await gate, new ArrayBuffer(2)) }, async () => "data:image/png;base64,AAAA");
    const ctx = context({ matrices: [summary(0, 10, 6, true)], cache });
    await show(ctx);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 350)));
    const texts = vi.mocked(ctx.onLoadStatus).mock.calls.map(([m]) => m?.text ?? null);
    expect(texts).toContain("Kartenbilder 0/60");
    release();
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 350)));
    const after = vi.mocked(ctx.onLoadStatus).mock.calls.map(([m]) => m?.text ?? null);
    expect(after[after.length - 1]).toBeNull();
  });

  it("stays_silent_for_a_short_queue", async () => {
    const ctx = context();
    await show(ctx);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 350)));
    expect(ctx.onLoadStatus).not.toHaveBeenCalled();
  });

  it("clears_its_line_on_unmount", async () => {
    const big: MatrixDto = { index: 0, name: "r", width: 10, height: 6, maps: Array.from({ length: 60 }, (_, i) => 100 + i), headers: new Array<number>(60).fill(0) };
    vi.mocked(getMatrix).mockResolvedValue(big);
    const stuck = new ThumbnailCache({ glb: () => new Promise<ArrayBuffer>(() => {}) }, async () => "");
    const ctx = context({ matrices: [summary(0, 10, 6, true)], cache: stuck });
    await show(ctx);
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 350)));
    expect(ctx.onLoadStatus).toHaveBeenCalled();
    act(() => root.unmount());
    const calls = vi.mocked(ctx.onLoadStatus).mock.calls;
    expect(calls[calls.length - 1]).toEqual([null]);
    root = createRoot(container);
  });
});
