// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../api";
import { extendArea, getMatrix } from "../api";
import { headerIdentity } from "./headerIdentity";
import { buildWorld } from "./worldModel";
import type { World } from "./worldModel";
import { mapsText } from "./places";
import { TERMS } from "./terms";
import { visibleWords } from "../ui/visibleText";
import { ThumbnailCache } from "../world/thumbnails";
import { WorldMapPanel } from "./WorldMapPanel";
import type { WorldMapContext } from "./WorldMapPanel";

// Counts how often a room card renders (it calls mapsText on every render).
vi.mock("./places", async (importOriginal) => {
  const original = await importOriginal<typeof import("./places")>();
  return { ...original, mapsText: vi.fn(original.mapsText) };
});
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), getMatrix: vi.fn(), extendArea: vi.fn() }));

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
    depth: "bauen",
    world: null,
    identify: (h) => headerIdentity([], h),
    pinnedCell: null,
    counts: { Map: 600 },
    onEdited: vi.fn(),
    onCountsChanged: vi.fn(),
    ...extra,
  });
  const show = async (ctx: WorldMapContext) => {
    act(() => root.render(<WorldMapPanel context={ctx} />));
    await flush();
  };

  it("says_there_is_no_region_and_still_lists_the_places", async () => {
    await show(context({ matrices: withoutRegion }));
    expect(container.textContent).toContain("Keine Region gefunden (keine Weltkarte mit Gebieten).");
    expect(container.querySelector("[data-cell]")).toBeNull();
    expect(container.querySelector('[data-place-key="Zweiblattdorf"]')).not.toBeNull();
    expect(container.querySelector('[data-place-key="Route 201"]')).not.toBeNull();
    expect(getMatrix).not.toHaveBeenCalled();
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    await show(context());
    for (const details of container.querySelectorAll("details")) details.open = true;
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    expect(container.querySelector('[data-header-id="0"]')?.textContent).toBe("Gebiet · 0000");
    expect(container.querySelector('[data-header-id="0"]')?.getAttribute("title")).toBe(TERMS.area.short);
    await show(context({ matrices: withoutRegion }));
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    // The views of a read world: the world map with its switcher, the room plan, "Weitere Gebiete".
    serveMatrices();
    await show(worldContext({ selected: 1 }));
    expect(container.querySelector("[data-world-view]")?.getAttribute("data-world-view")).toBe("w0");
    for (const details of container.querySelectorAll("details")) details.open = true;
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    await show(worldContext({ selected: 4 }));
    expect(container.querySelector("[data-world-view]")?.getAttribute("data-world-view")).toBe("plan");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    await show(worldContext({ selected: 1 }));
    await choose("Weitere Gebiete");
    expect(container.querySelector("[data-world-view]")?.getAttribute("data-world-view")).toBe("more");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("labels_the_grids_in_user_words_in_bauen_and_keeps_raster_in_tuefteln", async () => {
    const label = () => container.querySelector("[role=group]")?.getAttribute("aria-label");
    await show(context());
    expect(label()).toBe("Weltkarte, 1 Zeilen, 2 Spalten");
    await show(context({ depth: "tuefteln" }));
    expect(label()).toBe("Raster 0, 1 Zeilen, 2 Spalten");
  });

  it("keeps_the_technical_words_in_tinkering", async () => {
    await show(context({ depth: "tuefteln" }));
    expect(container.querySelector('[data-header-id="0"]')?.textContent).toBe("Header · 0000");
    expect(container.querySelector('[data-header-id="0"]')?.getAttribute("title")).toBe(TERMS.outside.short);
    await show(context({ matrices: withoutRegion, depth: "tuefteln" }));
    expect(container.textContent).toContain("Keine Region gefunden (kein Raster mit Header-Abschnitt).");
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

  // A read world (Sinnoh): matrix 0 (3 × 3) with Zweiblattdorf (1), Route 201 (2, 3), landscape (0) and two empty cells (2, 0) and (2, 2);
  // matrix 7 is an area with its own map (Großmoor, 10); header 4 is a room of Zweiblattdorf on matrix 20 (1 × 1, no header section).
  const wheader = (id: number, name: string, place: string, index: number, matrix: number, type = 2, kanto = false): HeaderSummary => ({
    ...header(id, place, matrix),
    internal_name: name,
    location_index: index,
    map_type: type,
    kanto,
  });
  const sinnohHeaders = [
    wheader(0, "EVERYWHERE", "Mysteriöser Ort", 0, 0),
    wheader(1, "T01", "Zweiblattdorf", 2, 0, 1),
    wheader(2, "R201A", "Route 201", 3, 0),
    wheader(3, "R201B", "Route 201", 3, 0),
    wheader(4, "T01R0101", "Zweiblattdorf", 2, 20, 4),
    wheader(10, "D06", "Großmoor", 70, 7),
  ];
  const sinnohSummaries = [summary(0, 3, 3, true), summary(7, 2, 2, true), summary(20, 1, 1)];
  const sinnohDtos = new Map<number, MatrixDto>([
    [0, { index: 0, name: "m0", width: 3, height: 3, maps: [1, 2, null, 4, 5, 6, 7, 8, null], headers: [1, 1, 0, 2, 3, 0, 0, 0, 0] }],
    [7, { index: 7, name: "m7", width: 2, height: 2, maps: [700, 701, 702, 703], headers: [10, 0, 0, 10] }],
    [20, { index: 20, name: "m20", width: 1, height: 1, maps: [50] }],
  ]);
  const meta = { version: 1 as const, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} };
  const sinnoh: World = buildWorld({ family: "Pt", headers: sinnohHeaders, matrices: sinnohSummaries, links: [], cells: sinnohDtos, meta });

  // HeartGold/SoulSilver: matrix 0 (5 × 4) holds Johto (Neuborkia 1 at 0,0 and 1,0; Route 29 2 at 0,1) and Kanto (Alabastia 3 at 2,1 and
  // 3,1; Route 29 of Kanto 4 at 2,2 and 3,2); Bergwerk (5) lies on its own map.
  const hgssHeaders = [
    wheader(0, "EVERYWHERE", "", 0, 0),
    wheader(1, "T20", "Neuborkia", 126, 0, 1),
    wheader(2, "R29", "Route 29", 127, 0),
    wheader(3, "T01", "Alabastia", 200, 0, 1, true),
    wheader(4, "R29K", "Route 29", 127, 0, 2, true),
    wheader(5, "D01", "Bergwerk", 9, 10, 3),
  ];
  const hgssSummaries = [summary(0, 5, 4, true), summary(10, 2, 1, true)];
  const hgssDtos = new Map<number, MatrixDto>([
    [0, { index: 0, name: "m0", width: 5, height: 4, maps: Array.from({ length: 20 }, (_, i) => i + 1), headers: [1, 1, 0, 0, 0, 2, 0, 3, 3, 0, 0, 0, 4, 4, 0, 0, 0, 0, 0, 0] }],
    [10, { index: 10, name: "m10", width: 2, height: 1, maps: [80, 81], headers: [5, 5] }],
  ]);
  const hgss: World = buildWorld({ family: "Hgss", headers: hgssHeaders, matrices: hgssSummaries, links: [], cells: hgssDtos, meta });

  function serveMatrices(dtos: ReadonlyMap<number, MatrixDto> = sinnohDtos) {
    vi.mocked(getMatrix).mockImplementation(async (index: number) => {
      const dto = dtos.get(index);
      if (!dto) throw new Error(`no matrix ${index}`);
      return dto;
    });
  }
  const worldContext = (extra: Partial<WorldMapContext> = {}) =>
    context({ headers: sinnohHeaders, matrices: sinnohSummaries, warpLinks: [], world: sinnoh, identify: (h) => ({ kind: "room", name: `Name ${h.id}`, place: "" }), ...extra });
  const hgssContext = (extra: Partial<WorldMapContext> = {}) => context({ headers: hgssHeaders, matrices: hgssSummaries, warpLinks: [], world: hgss, ...extra });

  const viewOf = () => container.querySelector("[data-world-view]")?.getAttribute("data-world-view");
  const cell = (key: string) => container.querySelector<HTMLElement>(`[data-cell="${key}"]`);
  /** Picks a view from the switcher of the head. */
  const trigger = () => container.querySelector<HTMLElement>("header h2 button")!;
  /** Waits (a few ms at a time, at most a second) until the switcher's menu is open or closed. */
  async function menuIs(open: boolean) {
    for (let waited = 0; (trigger().getAttribute("aria-expanded") === "true") !== open; waited += 5) {
      if (waited > 1000) throw new Error(`the menu did not ${open ? "open" : "close"}`);
      await act(() => new Promise<void>((resolve) => setTimeout(resolve, 5)));
    }
  }
  /** Opens the switcher, runs `inside` (a pick closes the menu), and closes it if it is still open. */
  async function withMenu<T>(inside: () => T): Promise<T> {
    act(() => trigger().click());
    await menuIs(true);
    const result = inside();
    if (trigger().getAttribute("aria-expanded") === "true") act(() => trigger().click());
    await menuIs(false);
    // The chosen view reads its matrix.
    await flush();
    return result;
  }
  const choose = (label: string) =>
    withMenu(() => {
      const item = [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")].find((r) => r.textContent === label);
      if (!item) throw new Error(`no view ${label}`);
      act(() => item.click());
    });
  /** The views the switcher offers. */
  const menuLabels = () => withMenu(() => [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")].map((r) => r.textContent));
  const altClick = (el: HTMLElement | null) => act(() => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, altKey: true })));
  const doubleClick = (el: HTMLElement | null) => act(() => void el?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));

  it("switches_between_world_maps_and_more_areas", async () => {
    serveMatrices();
    await show(worldContext({ selected: 1 }));
    expect(viewOf()).toBe("w0");
    expect(container.querySelector("header h2 button")?.textContent).toBe("Sinnoh");
    expect(await menuLabels()).toEqual(["Sinnoh", "Weitere Gebiete"]);
    await choose("Weitere Gebiete");
    expect(viewOf()).toBe("more");
    expect(container.querySelector('[data-more-area="m7"]')).not.toBeNull();
    expect(container.querySelector("[data-world-viewport]")).toBeNull();
    await choose("Sinnoh");
    expect(viewOf()).toBe("w0");
    expect(container.querySelector("[data-world-viewport]")).not.toBeNull();
    expect(cell("0,0")).not.toBeNull();
  });

  it("follows_the_selection_to_the_world_map_of_an_area_and_to_the_plan_of_a_room", async () => {
    serveMatrices();
    await show(worldContext({ selected: 1 }));
    expect(viewOf()).toBe("w0");
    // A choice in the menu holds until the next selection.
    await choose("Weitere Gebiete");
    await show(worldContext({ selected: 1 }));
    expect(viewOf()).toBe("more");
    await show(worldContext({ selected: 4 }));
    expect(viewOf()).toBe("plan");
    expect(await menuLabels()).toEqual(["Sinnoh", "Weitere Gebiete", "Raumplan"]);
    await show(worldContext({ selected: 2 }));
    expect(viewOf()).toBe("w0");
    await show(worldContext({ selected: 10 }));
    expect(viewOf()).toBe("plan");
    // The choice made while header 1 was selected does not come back with it.
    await show(worldContext({ selected: 1 }));
    expect(viewOf()).toBe("w0");
  });

  it("selects_the_area_on_a_click_and_the_map_on_alt_click_or_double_click", async () => {
    serveMatrices();
    const ctx = worldContext({ selected: 1 });
    await show(ctx);
    // Cell (1, 1) is the second section of Route 201: a click selects the area by its first header.
    act(() => cell("1,1")?.click());
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "header", id: 2 });
    altClick(cell("1,1"));
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 1, y: 1, header: 3, show: "map" });
    doubleClick(cell("0,0"));
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 0, y: 0, header: 1, show: "map" });
  });

  it("selects_the_map_on_a_landscape_cell", async () => {
    serveMatrices();
    const ctx = worldContext({ selected: 1 });
    await show(ctx);
    // Header 0 is no header: only the pin moves, the selection and the scene stay.
    act(() => cell("2,1")?.click());
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 2, y: 1, header: null, show: "map" });
    altClick(cell("2,1"));
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 2, y: 1, header: null, show: "map" });
    // The pinned cell is marked instead of the area's cells.
    await show(worldContext({ selected: 1, pinnedCell: { matrix: 0, x: 2, y: 1 } }));
    expect(cell("2,1")?.getAttribute("aria-pressed")).toBe("true");
    expect(cell("0,0")?.getAttribute("aria-pressed")).toBe("false");
    await show(worldContext({ selected: 1 }));
    expect(cell("0,0")?.getAttribute("aria-pressed")).toBe("true");
    expect(cell("1,0")?.getAttribute("aria-pressed")).toBe("true");
    expect(cell("2,1")?.getAttribute("aria-pressed")).toBe("false");
  });

  it("shows_plus_beside_the_selected_area_and_opens_the_extend_dialog", async () => {
    serveMatrices();
    vi.mocked(extendArea).mockReset();
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 3, height: 3 });
    const ctx = worldContext({ selected: 1 });
    await show(ctx);
    const plus = container.querySelector<HTMLElement>('[data-extend="2,0"]');
    expect(plus?.getAttribute("aria-label")).toBe("Karte bei x 2, y 0 anlegen (nicht rückgängig)");
    // (2, 2) is empty too, but touches no cell of Zweiblattdorf; no "+" column or row outside the world map.
    expect(container.querySelectorAll("[data-extend]").length).toBe(1);
    act(() => plus?.click());
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog.textContent).toContain("Karte bei x 2, y 0 anlegen?");
    expect(dialog.textContent).toContain("Nicht rückgängig");
    // Bauen names the area the new map belongs to, not its header.
    expect(dialog.textContent).toContain("Die neue Karte gehört zu „Zweiblattdorf“.");
    expect(visibleWords(dialog)).not.toMatch(/Raster|Matrix|Header/);
    await act(async () => [...dialog.querySelectorAll("button")].find((b) => b.textContent === "Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith(expect.objectContaining({ matrix: 0, x: 2, y: 0, source_map: 2, header_cell: 1 }));
    expect(ctx.onCountsChanged).toHaveBeenCalled();
    expect(ctx.onEdited).toHaveBeenCalledWith(0);
  });

  it("shows_the_room_plan_at_64_with_plus_at_the_edges", async () => {
    serveMatrices();
    await show(worldContext({ selected: 4 }));
    expect(viewOf()).toBe("plan");
    expect(container.querySelector("header h2 button")?.textContent).toBe("Raumplan");
    expect(container.querySelector("header")?.textContent).toContain("Name 4");
    const group = container.querySelector<HTMLElement>("[role=group]");
    expect(group?.getAttribute("aria-label")).toBe("Raumplan, 1 Zeilen, 1 Spalten");
    expect(group?.style.gridTemplateColumns).toBe("repeat(2, 64px)");
    expect([...container.querySelectorAll("[data-extend]")].map((e) => e.getAttribute("data-extend")).sort()).toEqual(["0,1", "1,0"]);
    expect(container.querySelector("[data-world-viewport]")).toBeNull();
    expect(container.querySelector("[data-place-key]")).toBeNull();
  });

  it("labels_the_room_plan_raster_in_tuefteln", async () => {
    serveMatrices();
    await show(worldContext({ selected: 4, depth: "tuefteln" }));
    expect(viewOf()).toBe("plan");
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toMatch(/^Raster \d+, 1 Zeilen, 1 Spalten$/);
  });

  it("selects_the_map_on_a_click_in_the_room_plan", async () => {
    serveMatrices();
    const ctx = worldContext({ selected: 4 });
    await show(ctx);
    act(() => cell("0,0")?.click());
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 20, x: 0, y: 0, header: 4, show: "map" });
    await show(worldContext({ selected: 4, pinnedCell: { matrix: 20, x: 0, y: 0 } }));
    expect(cell("0,0")?.getAttribute("aria-pressed")).toBe("true");
    // Großmoor's plan: cell (1, 0) carries header 0, so the link names the shown header and the plan and the scene stay.
    const moor = worldContext({ selected: 10 });
    await show(moor);
    expect(viewOf()).toBe("plan");
    act(() => cell("1,0")?.click());
    expect(moor.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 7, x: 1, y: 0, header: 10, show: "map" });
    await show(worldContext({ selected: 10, pinnedCell: { matrix: 7, x: 1, y: 0 } }));
    expect(viewOf()).toBe("plan");
    expect(cell("1,0")?.getAttribute("aria-pressed")).toBe("true");
  });

  it("proposes_the_nearest_cell_with_a_header_from_a_plus_of_the_room_plan", async () => {
    // Großmoor with an empty cell (1, 1): left of it lies header 0 (map 702), above it header 10 (map 701).
    const moor: MatrixDto = { index: 7, name: "m7", width: 2, height: 2, maps: [700, 701, 702, null], headers: [10, 10, 0, 0] };
    serveMatrices(new Map([...sinnohDtos, [7, moor]]));
    await show(worldContext({ selected: 10 }));
    act(() => container.querySelector<HTMLElement>('[data-extend="1,1"]')?.click());
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog.textContent).toContain("Kopie von Karte 0701 (Nachbarzelle)");
    expect(dialog.textContent).toContain("Die neue Karte gehört zu „Name 10“.");
    vi.mocked(extendArea).mockReset();
    vi.mocked(extendArea).mockResolvedValue({ map: 600, width: 2, height: 2 });
    await act(async () => [...dialog.querySelectorAll("button")].find((b) => b.textContent === "Karte anlegen")?.click());
    expect(extendArea).toHaveBeenCalledWith(expect.objectContaining({ matrix: 7, x: 1, y: 1, source_map: 701, header_cell: 10 }));
  });

  it("lists_more_areas_as_tiles_that_select_their_area", async () => {
    serveMatrices();
    const ctx = worldContext({ selected: 1 });
    await show(ctx);
    await choose("Weitere Gebiete");
    const tile = container.querySelector<HTMLElement>('[data-more-area="m7"]');
    expect(tile?.tagName).toBe("BUTTON");
    expect(tile?.textContent).toContain("Großmoor");
    expect(tile?.textContent).toContain("4 Karten");
    // The plan of its matrix in 32 px, drawn as pictures inside the button.
    expect(tile?.querySelectorAll("[data-cell]").length).toBe(4);
    expect(tile?.querySelector("button")).toBeNull();
    expect(getMatrix).toHaveBeenCalledWith(7);
    act(() => tile?.click());
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "header", id: 10 });
  });

  it("dims_the_other_half_and_windows_kanto", async () => {
    serveMatrices(hgssDtos);
    const ctx = hgssContext({ selected: 3 });
    await show(ctx);
    expect(viewOf()).toBe("w0k");
    expect(await menuLabels()).toEqual(["Johto", "Kanto", "Johto und Kanto", "Weitere Gebiete"]);
    // Kanto's cells lie in x 2..3, y 1..2: the window is x 1..4, y 0..3.
    expect(cell("0,0")).toBeNull();
    expect(cell("4,3")).not.toBeNull();
    expect(cell("1,0")?.hasAttribute("data-foreign")).toBe(true);
    expect(cell("2,1")?.hasAttribute("data-foreign")).toBe(false);
    act(() => cell("1,0")?.click());
    expect(ctx.goTo).not.toHaveBeenCalled();
    altClick(cell("1,0"));
    expect(ctx.goTo).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 1, y: 0, header: 1, show: "map" });
    await show(hgssContext({ selected: 1 }));
    expect(viewOf()).toBe("w0");
    expect(cell("4,0")).toBeNull();
    expect(cell("2,1")?.hasAttribute("data-foreign")).toBe(true);
    await choose("Johto und Kanto");
    expect(viewOf()).toBe("w0all");
    expect(cell("4,3")).not.toBeNull();
    expect(container.querySelector("[data-foreign]")).toBeNull();
  });
});
