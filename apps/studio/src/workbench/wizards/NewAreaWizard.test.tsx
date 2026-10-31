// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyHeaderPatch, createArea, dirty, getMatrix, getTextArchive, headerPatchStatus, placeSlots, recipePlan } from "../../api";
import type { AreaReport, AreaSpec, DraftDto, HeaderPatchDto, HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { openSelect, pressKey } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import type { World } from "../worldModel";
import { NewAreaWizard } from "./NewAreaWizard";
import { DRAFT_DEBOUNCE_MS } from "./useDraftPlan";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), createArea: vi.fn(), getMatrix: vi.fn(), headerPatchStatus: vi.fn(), applyHeaderPatch: vi.fn(), dirty: vi.fn(), placeSlots: vi.fn(), getTextArchive: vi.fn(), recipePlan: vi.fn() }));
// The grids ask the app-wide cache for map pictures; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, matrix = 0, area = 0, event = 0): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: "", matrix_id: matrix, area_data_id: area, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(0, "ZERO"), header(1, "NOTHING", 0, 3, 8)];
const summary = (index: number, width: number, height: number, hasHeaders = false): MatrixSummary => ({ index, name: `m${index}`, width, height, has_headers: hasHeaders, has_altitudes: false, filled: width * height });
const matrices = [summary(0, 2, 1), summary(1, 3, 2, true)];
const links: WarpLink[] = [{ event_file: 8, index: 0, x: 1, z: 1, dest_header_id: 1, dest_warp_id: 0 }];
const matrixOf = (index: number): MatrixDto =>
  index === 0
    ? { index: 0, name: "m0", width: 2, height: 1, maps: [0, 1] }
    : { index: 1, name: "m1", width: 3, height: 2, maps: [10, 11, null, 13, 14, 15], headers: [1, 1, 0, 0, 0, 0] };
const patchState = (change: Partial<HeaderPatchDto> = {}): HeaderPatchDto => ({ supported: true, applied: true, repairable: false, narc: "unpacked/dynamicHeaders", ...change });
/** Matrix 1 is the world map Sinnoh, matrix 0 the area „Erzelingen-Mine“ with a map of its own. */
const world: World = {
  worldMaps: [{ key: "w1", matrix: 1, filter: null, name: "Sinnoh", areas: [] }],
  moreAreas: [{ key: "m0", name: "Erzelingen-Mine", placeKey: "Erzelingen-Mine", headers: [], rooms: [], matrix: 0 }],
  unconnected: [],
  other: [],
  areaOf: new Map(),
  roomCount: 0,
  assignedRooms: 0,
};
const FORBIDDEN = /Raster|Matrix|Header/;
// The plan of the area as the kernel answers it: two cells of a matrix that is still to come.
const planned: DraftDto = {
  cells: [
    { matrix: null, x: 0, y: 0, source_map: 0 },
    { matrix: null, x: 1, y: 0, source_map: 1 },
  ],
  tiles: [],
  events: [],
  messages: [],
  notes: ["Legt Gebiet „Neues Gebiet“ mit 2 Karten an."],
  irreversible: ["2 Karten werden kopiert"],
};
// A plan in the kernel's own words, as `recipes/area.rs` writes them (Tüfteln's words).
const kernelWords: DraftDto = {
  ...planned,
  notes: ["Legt Gebiet „Neues Gebiet“ mit 1 Karten an.", "Header 3 zeigt auf die neue Weltkarte."],
  irreversible: ["Karte 0010 wird kopiert", "Eventdatei 12 wird kopiert"],
};
const report: AreaReport = { matrix: 131, maps: [[0, 0, 571], [1, 0, 572]], event_file: null, header: null, patched: false, snapshot: null, place: null, replaced: 0 };

describe("NewAreaWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onCreated = vi.fn();
  const onError = vi.fn();
  const onHeaderPatch = vi.fn();
  const onDraft = vi.fn<(draft: DraftDto | null) => void>();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMatrix).mockReset().mockImplementation(async (index) => matrixOf(index));
    vi.mocked(createArea).mockReset().mockResolvedValue(report);
    vi.mocked(recipePlan).mockReset().mockResolvedValue(planned);
    onDraft.mockReset();
    onClose.mockReset();
    onCreated.mockReset().mockResolvedValue(undefined);
    onError.mockReset();
    onHeaderPatch.mockReset().mockResolvedValue(undefined);
    vi.mocked(headerPatchStatus).mockReset().mockResolvedValue(patchState());
    vi.mocked(applyHeaderPatch).mockReset().mockResolvedValue({ ...patchState(), snapshot: null });
    vi.mocked(dirty).mockReset().mockResolvedValue([]);
    vi.mocked(placeSlots).mockReset().mockResolvedValue({ bank: 433, count: 3, free: 253, reusable: [] });
    vi.mocked(getTextArchive)
      .mockReset()
      .mockResolvedValue({ index: 433, key: 0, lang: "de", messages: ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme"].map((text, i) => ({ id: `msg_${i}`, text })), users: [] });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  const wizard = (prefill: { matrix?: number } | undefined, listed: MatrixSummary[] = matrices, overview: { loaded: boolean; failed: boolean } = { loaded: true, failed: false }, depth: Depth = "tuefteln") => (
    <NewAreaWizard depth={depth} world={world} headers={headers} matrices={listed} matricesLoaded={overview.loaded} matricesFailed={overview.failed} links={links} counts={{ Map: 600, AreaData: 20 }} prefill={prefill} onClose={onClose} onCreated={onCreated} onHeaderPatch={onHeaderPatch} onError={onError} onDraft={onDraft} />
  );
  async function show(prefill: { matrix?: number } | undefined = { matrix: 0 }, depth: Depth = "tuefteln") {
    await act(async () => root.render(wizard(prefill, matrices, { loaded: true, failed: false }, depth)));
    await flush();
  }
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const cell = (x: number, y: number) => container.querySelector<HTMLElement>(`[data-cell="${x},${y}"]`);
  const click = (el: Element | null | undefined, init: MouseEventInit = {}) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true, ...init })));
  const next = () => click(button("Weiter"));
  const selected = () => [...container.querySelectorAll("[data-cell][aria-pressed=true]")].map((e) => e.getAttribute("data-cell"));
  /** The source select (a Base UI select: a button with the role combobox). */
  const source = () => container.querySelector<HTMLElement>('[role="combobox"][aria-label="Raster der Quelle"]');
  /** The rows of the header picker (a dialog outside the container). */
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  async function openPicker() {
    await click(button("Header wählen…"));
    await flush();
  }
  /** Picks the header with this number in the header picker. */
  async function pickHeader(id: string) {
    await openPicker();
    await click(options().find((o) => o.textContent?.includes(id)));
    await flush();
  }

  it("creates_the_area_from_two_clicked_cells_and_shows_the_result", async () => {
    await show();
    await click(cell(0, 0));
    await click(cell(1, 0));
    await next();
    await next();
    await next();
    expect(container.textContent).toContain("Kopiert Karte 0000");
    expect(container.textContent).toContain("Anlegen lässt sich nicht rückgängig machen.");
    await click(button("Anlegen"));
    expect(createArea).toHaveBeenCalledOnce();
    const spec = vi.mocked(createArea).mock.calls[0][0];
    expect(spec.cells).toHaveLength(2);
    expect(spec.with_headers).toBe(true);
    expect(container.textContent).toContain("Raster 0131");
    expect(container.textContent).toContain("Karten 0571, 0572");
    expect(button("Weiter")).toBeUndefined();
    await click(button("Zeigen"));
    expect(onCreated).toHaveBeenCalledWith(report, spec);
  });

  it("walks_the_steps_in_order_and_gates_weiter_on_a_selection", async () => {
    await show();
    const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
    expect(current()).toBe("1. Quelle");
    expect(button("Weiter")?.disabled).toBe(true);
    await click(cell(0, 0));
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    expect(current()).toBe("2. Anordnung");
    await next();
    expect(current()).toBe("3. Header");
    await next();
    expect(current()).toBe("4. Zusammenfassung");
    await click(button("Zurück"));
    expect(current()).toBe("3. Header");
  });

  it("selects_a_rectangle_with_shift_and_toggles_single_cells", async () => {
    await show({ matrix: 1 });
    await click(cell(0, 0));
    expect(selected()).toEqual(["0,0"]);
    await click(cell(2, 1), { shiftKey: true });
    // (2, 0) is an empty cell and never picked.
    expect(selected()).toEqual(["0,0", "1,0", "0,1", "1,1", "2,1"]);
    await click(cell(1, 0));
    expect(selected()).toEqual(["0,0", "0,1", "1,1", "2,1"]);
  });

  it("starts_from_the_prefilled_matrix_and_switches_the_source_with_the_select", async () => {
    await show({ matrix: 1 });
    expect(source()?.textContent).toContain("Raster 0001 m1 (3 × 2)");
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(6);
    await click(cell(0, 0));
    openSelect(source()!);
    await act(async () => options().find((o) => o.textContent?.startsWith("Raster 0000"))!.click());
    await flush();
    expect(source()?.textContent).toContain("Raster 0000 m0 (2 × 1)");
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
    expect(selected()).toEqual([]);
  });

  describe("the source grid", () => {
    const placed = [{ ...header(0, "ZERO"), location_name: "Sandgem" }, { ...header(1, "NOTHING", 1, 3, 8), location_name: "Route 201" }];
    const caption = () => container.querySelector('[data-caption]')?.textContent;
    const hover = (x: number, y: number) => act(async () => void cell(x, y)?.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })));

    async function open(prefill: { matrix?: number; header?: number }) {
      await act(async () =>
        root.render(<NewAreaWizard depth="tuefteln" world={world} headers={placed} matrices={matrices} matricesLoaded matricesFailed={false} links={links} counts={{ Map: 600, AreaData: 20 }} prefill={prefill} onClose={onClose} onCreated={onCreated} onHeaderPatch={onHeaderPatch} onError={onError} onDraft={onDraft} />),
      );
      await flush();
    }

    it("selects_the_cells_of_the_header_the_wizard_was_opened_for", async () => {
      await open({ matrix: 1, header: 1 });
      expect(selected()).toEqual(["0,0", "1,0"]);
      // The selection is the user's from here on: a click deselects.
      await click(cell(0, 0));
      expect(selected()).toEqual(["1,0"]);
      expect(button("Weiter")?.disabled).toBe(false);
    });

    it("takes_the_area_data_of_the_preselected_header_and_skips_empty_cells", async () => {
      await open({ matrix: 1, header: 0 });
      // (2, 0) holds header 0 but no map: it cannot be copied.
      expect(selected()).toEqual(["0,1", "1,1", "2,1"]);
      await next();
      await next();
      expect(container.querySelector<HTMLInputElement>('input[aria-label^="Area-Data"]')?.value).toBe("0");
    });

    it("selects_nothing_for_a_matrix_without_header_section", async () => {
      await open({ matrix: 0, header: 1 });
      expect(selected()).toEqual([]);
      expect(button("Weiter")?.disabled).toBe(true);
    });

    it("selects_nothing_when_the_header_has_no_cell_in_the_matrix", async () => {
      await open({ matrix: 1, header: 5 });
      expect(selected()).toEqual([]);
    });

    it("selects_nothing_when_the_wizard_has_no_header_to_start_from", async () => {
      await open({ matrix: 1 });
      expect(selected()).toEqual([]);
    });

    it("tints_the_cells_by_place", async () => {
      await open({ matrix: 1 });
      const tint = (x: number, y: number) => cell(x, y)?.style.boxShadow;
      expect(tint(0, 0)).not.toBe("");
      expect(tint(0, 1)).not.toBe("");
      expect(tint(0, 0)).not.toBe(tint(0, 1));
      expect(tint(1, 0)).toBe(tint(0, 0));
    });

    it("names_the_hovered_cell_else_the_one_clicked_last", async () => {
      await open({ matrix: 1 });
      expect(caption()).toBe("");
      await hover(1, 0);
      expect(caption()).toBe("Route 201 · Header 0001 · Karte 0011");
      await hover(0, 1);
      expect(caption()).toBe("Sandgem · Header 0000 · Karte 0013");
      await act(async () => void cell(0, 1)?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true })));
      expect(caption()).toBe("");
      await click(cell(1, 0));
      expect(caption()).toBe("Route 201 · Header 0001 · Karte 0011");
    });

    it("names_only_the_map_for_a_matrix_without_header_section", async () => {
      await open({ matrix: 0 });
      await hover(1, 0);
      expect(caption()).toBe("Karte 0001");
    });

    it("names_an_empty_cell_as_empty", async () => {
      await open({ matrix: 1 });
      await hover(2, 0);
      expect(caption()).toBe("Keine Karte bei x 2, y 0");
    });
  });

  it("gives_the_new_header_an_empty_event_file_unless_another_is_chosen", async () => {
    await show({ matrix: 1 });
    await click(cell(0, 0));
    await next();
    await next();
    await pickHeader("0001");
    await next();
    expect(container.textContent).toContain("Eventdatei: neu, leer.");
    await click(button("Anlegen"));
    expect(vi.mocked(createArea).mock.calls[0][0].header?.event_file).toBe("New");
  });

  it("says_the_overview_is_loading_or_unavailable_instead_of_no_raster", async () => {
    await act(async () => root.render(wizard(undefined, [], { loaded: false, failed: false })));
    await flush();
    expect(container.textContent).toContain("Lade Übersicht …");
    expect(container.textContent).not.toContain("Kein Raster vorhanden.");
    await act(async () => root.render(wizard(undefined, [], { loaded: false, failed: true })));
    expect(container.textContent).toContain("Übersicht nicht verfügbar");
    expect(container.textContent).not.toContain("Kein Raster vorhanden.");
    // Loaded and really empty: then there is no raster.
    await act(async () => root.render(wizard(undefined, [], { loaded: true, failed: false })));
    expect(container.textContent).toContain("Kein Raster vorhanden.");
  });

  it("falls_back_to_the_first_matrix_once_the_overview_arrives_when_nothing_is_prefilled", async () => {
    await act(async () => root.render(wizard(undefined, [], { loaded: false, failed: false })));
    await flush();
    expect(getMatrix).not.toHaveBeenCalled();
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(0);
    await act(async () => root.render(wizard(undefined)));
    await flush();
    expect(getMatrix).toHaveBeenCalledWith(0);
    expect(source()?.textContent).toContain("Raster 0000");
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
  });

  it("keeps_a_prefilled_matrix_when_the_overview_arrives_later", async () => {
    await act(async () => root.render(wizard({ matrix: 1 }, [])));
    await act(async () => root.render(wizard({ matrix: 1 })));
    await flush();
    expect(source()?.textContent).toContain("Raster 0001");
    expect(getMatrix).toHaveBeenCalledWith(1);
    expect(getMatrix).not.toHaveBeenCalledWith(0);
  });

  it("adds_maps_typed_by_number_and_reports_the_ones_it_cannot_use", async () => {
    await show();
    const field = container.querySelector<HTMLInputElement>('input[aria-label="Karten per Nummer"]')!;
    await act(async () => field.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(field, "7, 8, 9999, x");
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => field.blur());
    expect(container.textContent).toContain("9999, x");
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    await next();
    await next();
    await click(button("Anlegen"));
    expect(vi.mocked(createArea).mock.calls[0][0].cells.map((c) => [c.x, c.y, c.source_map])).toEqual([
      [0, 0, 7],
      [1, 0, 8],
    ]);
  });

  it("takes_area_data_and_event_file_from_the_header_of_the_first_source_cell", async () => {
    await show({ matrix: 1 });
    await click(cell(0, 0));
    await next();
    await next();
    expect(container.querySelector<HTMLInputElement>('input[aria-label^="Area-Data"]')?.value).toBe("3");
    // Header 0001 is picked in the picker; the warp that leads to it is counted.
    await pickHeader("0001");
    expect(container.querySelector('[data-header="1"]')?.textContent).toContain("NOTHING");
    expect(container.textContent).toContain("Warps hierher: 1");
    expect(container.textContent).not.toContain("nichts führt hierher");
    await click(button("Kopie von 0008"));
    await next();
    expect(container.textContent).toContain("Stellt Header 0001 „NOTHING“ auf das neue Raster um, Area-Data 0003, Eventdatei: Kopie von 0008.");
    await click(button("Anlegen"));
    expect(vi.mocked(createArea).mock.calls[0][0].header).toEqual({ id: 1, area_data_id: 3, event_file: { Copy: 8 } });
  });

  it("counts_the_warps_that_lead_to_each_header_in_the_picker_rows_and_marks_the_unused", async () => {
    await show();
    await click(cell(0, 0));
    await next();
    await next();
    await openPicker();
    const row = (id: string) => options().find((o) => o.textContent?.includes(id))?.textContent;
    // Header 0 is the default of every cell without one: no warp leads there, and still it is not free.
    expect(row("0000")).toContain("Warps hierher: 0, Sonder-Header, nicht frei");
    expect(row("0000")).not.toContain("wirkt unbenutzt");
    expect(row("0001")).toContain("Warps hierher: 1");
    expect(row("0001")).not.toContain("wirkt unbenutzt");
    await click(options().find((o) => o.textContent?.includes("0000")));
    await flush();
    expect(container.textContent).toContain("Sonder-Header: Vorgabe-Header aller Zellen ohne eigenen");
    expect(container.textContent).not.toContain("nichts führt hierher");
  });

  it("stays_on_the_summary_and_shows_a_refusal_that_created_nothing_inside_the_dialog", async () => {
    vi.mocked(createArea).mockRejectedValue({ code: "error", message: "map 9999 does not exist (the game has 600)", asset: null });
    await show();
    await click(cell(0, 0));
    await next();
    await next();
    await next();
    await click(button("Anlegen"));
    // The refusal is shown inside the dialog (the status strip is hidden behind its backdrop).
    expect(onError).not.toHaveBeenCalled();
    expect(container.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe("map 9999 does not exist (the game has 600)");
    expect(container.querySelector('[aria-current="step"]')?.getAttribute("aria-label")).toBe("4. Zusammenfassung");
    expect(button("Anlegen")?.disabled).toBe(false);
    expect(button("Zeigen")).toBeUndefined();
  });

  it("drops_the_refusal_when_the_user_goes_back_to_change_something", async () => {
    vi.mocked(createArea).mockRejectedValue({ code: "error", message: "map 9999 does not exist (the game has 600)", asset: null });
    await show();
    await click(cell(0, 0));
    await next();
    await next();
    await next();
    await click(button("Anlegen"));
    expect(container.querySelector('[role="alert"]')).not.toBeNull();
    await click(button("Zurück"));
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });

  it("shows_a_late_failure_in_the_result_step_with_the_way_to_finish_by_hand", async () => {
    vi.mocked(createArea).mockRejectedValue({ code: "partial", message: "matrix 0131 and maps 0571, 0572 were created; header edit failed: boom", asset: null });
    await show();
    await click(cell(0, 0));
    await next();
    await next();
    await next();
    await click(button("Anlegen"));
    expect(onError).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Angelegt: Raster 0131, Karten 0571, 0572. Danach fehlgeschlagen: Header umstellen (boom).");
    expect(container.textContent).toContain("Raster-Tab");
    expect(container.textContent).toContain("Header-Editor");
    expect(button("Zeigen")).toBeUndefined();
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  describe("in Bauen", () => {
    const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
    const consent = () => container.querySelector<HTMLElement>("[data-patch-consent] [role=checkbox]");
    /** Cell (0, 0) of matrix 1 belongs to header 0001 (area data 3): the new header is a copy of it. */
    async function toSummary(patch: Partial<HeaderPatchDto> = {}) {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState(patch));
      await show({ matrix: 1 }, "bauen");
      await click(cell(0, 0));
      await next();
      await next();
      await flush();
      await next();
    }

    it("shows_the_step_place_in_bauen_and_header_in_tuefteln", async () => {
      await show({ matrix: 1 }, "bauen");
      await click(cell(0, 0));
      await next();
      await next();
      expect(current()).toBe("3. Ort");
      expect(container.querySelector("[aria-label=Ort]")).not.toBeNull();
      expect(container.querySelector<HTMLInputElement>('input[aria-label="Name des neuen Orts"]')?.value).toBe("Neues Gebiet");
      // The headers here carry no place name: the look says so.
      expect(container.querySelector("button[data-look-picker]")?.textContent).toBe("Aussehen: Ohne Ortsnamen");
      act(() => root.unmount());
      root = createRoot(container);
      await show({ matrix: 1 }, "tuefteln");
      await click(cell(0, 0));
      await next();
      await next();
      expect(current()).toBe("3. Header");
      expect(button("Neuer Header")).toBeDefined();
    });

    it("cannot_create_without_the_consent_when_the_patch_is_missing", async () => {
      await toSummary({ applied: false });
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.querySelector("[data-patch-consent]")?.textContent).toContain("Nicht rückgängig");
      expect(button("Anlegen")?.disabled).toBe(true);
      await click(consent());
      expect(button("Anlegen")?.disabled).toBe(false);
      act(() => root.unmount());
      root = createRoot(container);
      // A broken patch is repaired first.
      await toSummary({ applied: true, repairable: true });
      expect(container.textContent).toContain("Die Anpassung des Spielcodes für neue Orte ist fehlerhaft. Reparieren in Technik › Patches.");
      expect(button("Anlegen")?.disabled).toBe(true);
    });

    it("sends_allow_patch_a_new_place_and_no_internal_name", async () => {
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header).toEqual({
        id: 2,
        area_data_id: null,
        event_file: "New",
        new_header: { template: 1, internal_name: null },
        place: { New: "Neues Gebiet" },
        allow_patch: true,
      });
      act(() => root.unmount());
      root = createRoot(container);
      vi.mocked(createArea).mockClear();
      // With the patch in place there is no line and no patch in the call.
      await toSummary({ applied: true });
      expect(container.querySelector("[data-patch-consent]")).toBeNull();
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header).toMatchObject({ allow_patch: false });
    });

    it("calls_on_header_patch_after_a_patching_create", async () => {
      vi.mocked(createArea).mockResolvedValue({ ...report, header: 2, patched: true });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).toHaveBeenCalledOnce();
      act(() => root.unmount());
      root = createRoot(container);
      onHeaderPatch.mockClear();
      vi.mocked(createArea).mockResolvedValue({ ...report, header: 2, patched: false });
      await toSummary({ applied: true });
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).not.toHaveBeenCalled();
    });

    it("reads_the_patch_again_after_a_late_failure_of_a_patching_create", async () => {
      vi.mocked(createArea).mockRejectedValue({ code: "partial", message: "the Dynamic Headers patch was applied and stays, header 0002 and matrix 0131 were created; map copy failed: boom", asset: null });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).toHaveBeenCalledOnce();
      expect(container.textContent).toContain("Spielcode für neue Orte angepasst (bleibt). Angelegt: das Gebiet, die Anordnung. Danach fehlgeschlagen: Karte kopieren (boom).");
      expect(visibleWords(document.body)).not.toMatch(FORBIDDEN);
      act(() => root.unmount());
      root = createRoot(container);
      // Without the patch in the call there is nothing to read again.
      onHeaderPatch.mockClear();
      await toSummary({ applied: true });
      await click(button("Anlegen"));
      await flush();
      expect(onHeaderPatch).not.toHaveBeenCalled();
    });

    it("names_where_the_patch_put_the_old_header_folder", async () => {
      vi.mocked(createArea).mockResolvedValue({ ...report, header: 2, patched: true, snapshot: "/p/.g4/backup/dynamicHeaders-20261026-120000" });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(container.textContent).toContain("Gebiet „Neues Gebiet“ angelegt: 2 Karten.");
      expect(container.textContent).toContain("Die alten Dateien sind unter .g4/backup gesichert.");
      expect(container.textContent).not.toContain("dynamicHeaders");
      expect(visibleWords(document.body)).not.toMatch(FORBIDDEN);
    });

    it("hands_a_failed_read_after_a_patching_create_to_on_error_and_keeps_the_result", async () => {
      const failure = { code: "error", message: "read failed", asset: null };
      onHeaderPatch.mockRejectedValue(failure);
      vi.mocked(createArea).mockResolvedValue({ ...report, header: 2, patched: true });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onError).toHaveBeenCalledWith(failure);
      expect(container.textContent).toContain("Gebiet „Neues Gebiet“ angelegt: 2 Karten.");
      expect(container.querySelector('[role="dialog"] [role="alert"]')).toBeNull();
      act(() => root.unmount());
      root = createRoot(container);
      // After a late failure the same: the partial result stays, the read failure goes to the app.
      onError.mockClear();
      vi.mocked(createArea).mockRejectedValue({ code: "partial", message: "the Dynamic Headers patch was applied and stays; header creation failed: boom", asset: null });
      await toSummary({ applied: false });
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(onError).toHaveBeenCalledWith(failure);
      expect(container.textContent).toContain("Spielcode für neue Orte angepasst (bleibt). Danach fehlgeschlagen: Gebiet anlegen (boom).");
    });

    it("plans_with_the_patch_assumed_while_it_is_missing_and_creates_with_the_users_consent", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
      // As the kernel: a new header without the patch and without allow_patch is refused.
      vi.mocked(recipePlan).mockImplementation(async (_recipe, input) =>
        (input as AreaSpec).header?.allow_patch === true ? planned : Promise.reject({ code: "refused", message: "new headers need the dynamic headers patch, apply the patch first", asset: null }),
      );
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      await show({ matrix: 1 }, "bauen");
      await click(cell(0, 0));
      await settle();
      // The draft shows from the first cell on, before any consent: the plan writes nothing.
      expect(vi.mocked(recipePlan).mock.lastCall?.[0]).toBe("create_area");
      expect((vi.mocked(recipePlan).mock.lastCall?.[1] as AreaSpec).header?.allow_patch).toBe(true);
      expect(onDraft).toHaveBeenLastCalledWith(planned);
      await next();
      await next();
      await flush();
      await next();
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.textContent).not.toContain("Vorschau nicht möglich");
      expect(container.textContent).not.toContain("dynamic headers");
      // „Anlegen“ still waits for the user's consent and sends it.
      expect(button("Anlegen")?.disabled).toBe(true);
      await click(consent());
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header?.allow_patch).toBe(true);
      // With the patch in the game nothing is assumed: plan and create send no consent.
      act(() => root.unmount());
      root = createRoot(container);
      vi.mocked(recipePlan).mockClear();
      vi.mocked(createArea).mockClear();
      await toSummary({ applied: true });
      await settle();
      expect((vi.mocked(recipePlan).mock.lastCall?.[1] as AreaSpec).header?.allow_patch).toBe(false);
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header?.allow_patch).toBe(false);
    });

    it("keeps_the_kernels_words_out_of_the_summary_in_bauen", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      vi.mocked(recipePlan).mockResolvedValue(kernelWords);
      await show({ matrix: 1 }, "bauen");
      await click(cell(0, 0));
      await next();
      await next();
      await flush();
      await next();
      await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
      expect(current()).toBe("4. Zusammenfassung");
      // The draft still reaches the scene; only the sentences stay out of the wizard.
      expect(onDraft).toHaveBeenLastCalledWith(kernelWords);
      expect(visibleWords(document.body)).not.toMatch(FORBIDDEN);
      expect(container.textContent).not.toContain("zeigt auf die neue Weltkarte");
      expect(container.textContent).not.toContain("Eventdatei 12 wird kopiert");
      expect(container.querySelector("[data-draft-notes]")).toBeNull();
    });

    it("says_no_raster_matrix_or_header_in_building", async () => {
      const words = () => visibleWords(document.body);
      const quelle = () => container.querySelector<HTMLElement>('[role="combobox"][aria-label="Quelle"]');
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      vi.mocked(createArea).mockResolvedValue({ ...report, header: 2, patched: true });
      await show({ matrix: 1 }, "bauen");
      expect(container.textContent).toContain("Karten aus einer Weltkarte oder einem Gebiet");
      expect(quelle()?.textContent).toContain("Sinnoh");
      openSelect(quelle()!);
      // The world maps and the areas with a map of their own, by name.
      expect(options().map((o) => o.textContent)).toEqual(["Sinnoh", "Erzelingen-Mine"]);
      expect(words()).not.toMatch(FORBIDDEN);
      await act(async () => options().find((o) => o.textContent === "Sinnoh")!.click());
      await flush();
      await click(cell(0, 0));
      // The caption names the place and the map of the clicked cell.
      expect(container.querySelector("[data-caption]")?.textContent).toBe("Ohne Ortsnamen · Karte 0010");
      // The grids' accessible names say no raster either.
      const gridNames = () => [...container.querySelectorAll("[aria-label]")].map((e) => e.getAttribute("aria-label") ?? "").filter((n) => n.includes("Zeilen"));
      expect(gridNames()).toEqual(["Karten der Quelle, 2 Zeilen, 3 Spalten"]);
      expect(words()).not.toMatch(FORBIDDEN);
      await next();
      expect(current()).toBe("2. Anordnung");
      expect([...container.querySelectorAll("label")].some((l) => l.textContent?.includes("Abschnitt"))).toBe(false);
      expect(gridNames()).toEqual(["Vorschau des neuen Gebiets, 1 Zeilen, 1 Spalten"]);
      expect(words()).not.toMatch(FORBIDDEN);
      await next();
      await flush();
      expect(current()).toBe("3. Ort");
      expect(words()).not.toMatch(FORBIDDEN);
      await next();
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.textContent).toContain("Legt das Gebiet „Neues Gebiet“ mit 1 × 1 Karten an.");
      expect(container.textContent).toContain("Neuer Ort „Neues Gebiet“.");
      expect(container.textContent).toContain("Eine neue, leere Liste der Figuren und Türen.");
      expect(words()).not.toMatch(FORBIDDEN);
      await click(consent());
      await click(button("Anlegen"));
      await flush();
      expect(vi.mocked(createArea).mock.calls[0][0].with_headers).toBe(true);
      expect(container.textContent).toContain("Gebiet „Neues Gebiet“ angelegt: 2 Karten.");
      expect(words()).not.toMatch(FORBIDDEN);
    });

    it("always_sends_the_cells_of_the_new_area_in_bauen", async () => {
      // Tüfteln's switch „Header-Abschnitt anlegen“ off does not reach Bauen.
      await show({ matrix: 1 }, "tuefteln");
      await click(cell(0, 0));
      await next();
      await click([...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((c) => c.closest("label")?.textContent === "Header-Abschnitt anlegen"));
      await act(async () => root.render(wizard({ matrix: 1 }, matrices, { loaded: true, failed: false }, "bauen")));
      await flush();
      expect(current()).toBe("2. Anordnung");
      await next();
      await flush();
      await next();
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].with_headers).toBe(true);
    });

    it("takes_the_source_by_name_in_bauen", async () => {
      const quelle = () => container.querySelector<HTMLElement>('[role="combobox"][aria-label="Quelle"]');
      await show({ matrix: 1 }, "bauen");
      expect(container.querySelectorAll("[data-cell]")).toHaveLength(6);
      openSelect(quelle()!);
      await act(async () => options().find((o) => o.textContent === "Erzelingen-Mine")!.click());
      await flush();
      expect(quelle()?.textContent).toContain("Erzelingen-Mine");
      expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
    });

    it("starts_on_an_existing_place_when_new_places_are_locked", async () => {
      // Diamond/Pearl: no patch, no new places.
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ supported: false, applied: false, narc: null }));
      await show({ matrix: 1 }, "bauen");
      await click(cell(0, 0));
      await next();
      await next();
      await flush();
      const toggle = (label: string) => [...container.querySelectorAll<HTMLButtonElement>("[aria-label=Ort] button")].find((b) => b.textContent === label);
      expect(toggle("Vorhandener Ort")?.getAttribute("aria-pressed")).toBe("true");
      expect(toggle("Neuer Ort")?.disabled).toBe(true);
      await next();
      expect(container.textContent).toContain("Neue Gebiete und Räume legt Bauen nur für Platin und HeartGold/SoulSilver an. In Tüfteln lässt sich ein vorhandenes Gebiet umstellen.");
      expect(button("Anlegen")?.disabled).toBe(true);
    });
  });

  describe("a new header", () => {
    /** Header 0001 is the source of cell (0, 0) of matrix 1, so the new header (number 0002) is a copy of it. */
    async function toHeaderStep() {
      await show({ matrix: 1 });
      await click(cell(0, 0));
      await next();
      await next();
    }
    /** An option of a `Segmented` by its text. */
    const radio = (label: string) => button(label) as HTMLButtonElement | undefined;
    const nameField = () => container.querySelector<HTMLInputElement>('input[aria-label="Interner Name"]');
    async function typeInto(input: HTMLInputElement, text: string) {
      await act(async () => input.focus());
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, text);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await act(async () => input.blur());
    }

    it("sends_the_new_header_with_the_next_number_and_the_source_header_as_template", async () => {
      await toHeaderStep();
      expect(headerPatchStatus).toHaveBeenCalledOnce();
      await click(radio("Neuer Header"));
      expect(container.textContent).toContain("Kopie von Header 0001 „NOTHING“");
      expect(nameField()?.value).toBe("NEUES_GEBIET");
      await next();
      expect(container.textContent).toContain("legt Header 0002 „NEUES_GEBIET“ neu an (Kopie von 0001) und stellt ihn auf das neue Raster um, Area-Data 0003, Eventdatei: neu, leer.");
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header).toEqual({ id: 2, area_data_id: 3, event_file: "New", new_header: { template: 1, internal_name: "NEUES_GEBIET" } });
    });

    it("follows_the_area_name_until_an_internal_name_is_typed", async () => {
      const rename = async (name: string) => {
        await click(button("Zurück"));
        await typeInto(container.querySelector<HTMLInputElement>('input[aria-label="Name des neuen Gebiets"]')!, name);
        await next();
      };
      await toHeaderStep();
      await click(radio("Neuer Header"));
      await rename("Route 230");
      expect(nameField()?.value).toBe("ROUTE_230");
      await typeInto(nameField()!, "mein weg");
      expect(nameField()?.value).toBe("MEIN_WEG");
      await rename("Route 231");
      // A typed name stays.
      expect(nameField()?.value).toBe("MEIN_WEG");
    });

    it("says_that_keeping_the_event_file_shares_the_templates", async () => {
      await toHeaderStep();
      await click(radio("Neuer Header"));
      await click(radio("Behalten"));
      await next();
      expect(container.textContent).toContain("Eventdatei: behalten (geteilt mit Header 0001)");
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].header?.event_file).toBe("Keep");
    });

    it("applies_the_patch_on_demand_and_then_offers_the_new_header", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      await toHeaderStep();
      expect(radio("Neuer Header")?.disabled).toBe(true);
      expect(container.textContent).toContain("arm9.bin.backup");
      await click(button("Patch anwenden"));
      expect(applyHeaderPatch).toHaveBeenCalledOnce();
      // The app reads the header table and list again.
      expect(onHeaderPatch).toHaveBeenCalledOnce();
      expect(button("Patch anwenden")).toBeUndefined();
      expect(radio("Neuer Header")?.disabled).toBe(false);
    });

    it("applies_the_patch_while_the_header_table_is_unsaved", async () => {
      // The kernel patches with the table from disk and keeps unsaved header edits: nothing asks to save first.
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      vi.mocked(dirty).mockResolvedValue([{ kind: "HeaderTable", index: 0 }]);
      await toHeaderStep();
      await click(button("Patch anwenden"));
      expect(applyHeaderPatch).toHaveBeenCalledOnce();
      expect(onHeaderPatch).toHaveBeenCalledOnce();
      expect(container.textContent).not.toContain("ungespeicherte Änderungen");
      expect(radio("Neuer Header")?.disabled).toBe(false);
    });

    it("shows_the_backends_refusal_in_the_step_and_keeps_the_button", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ applied: false }));
      vi.mocked(applyHeaderPatch).mockRejectedValue({ code: "error", message: "the header table changed on disk", asset: null });
      await toHeaderStep();
      await click(button("Patch anwenden"));
      expect(container.textContent).toContain("the header table changed on disk");
      expect(onHeaderPatch).not.toHaveBeenCalled();
      expect(onError).not.toHaveBeenCalled();
      expect(radio("Neuer Header")?.disabled).toBe(true);
      expect(button("Patch anwenden")?.disabled).toBe(false);
    });

    it("offers_no_new_header_for_a_game_without_the_patch_or_when_the_status_fails", async () => {
      vi.mocked(headerPatchStatus).mockResolvedValue(patchState({ supported: false, applied: false, narc: null }));
      await toHeaderStep();
      expect(radio("Neuer Header")?.disabled).toBe(true);
      expect(container.textContent).toContain("Neue Header gibt es nur für Platin und HeartGold/SoulSilver");
      act(() => root.unmount());
      root = createRoot(container);
      vi.mocked(headerPatchStatus).mockRejectedValue({ code: "error", message: "unknown game code XXXX", asset: null });
      await toHeaderStep();
      expect(radio("Neuer Header")?.disabled).toBe(true);
      expect(container.textContent).toContain("unknown game code XXXX");
      expect(onError).not.toHaveBeenCalled();
    });
  });

  describe("where the area lies", () => {
    const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
    const lage = () => [...container.querySelectorAll<HTMLButtonElement>("[aria-label=Lage] button")];
    const choose = (label: string) => click(lage().find((b) => b.textContent === label));
    const pressed = () => lage().find((b) => b.getAttribute("aria-pressed") === "true")?.textContent;
    const SINNOH = "Auf der Weltkarte Sinnoh (nahtlos begehbar)";
    const OWN = "Eigene Karte (über Türen verbunden)";
    /** A cell of the world map grid (the preview has cells too). */
    const worldCell = (x: number, y: number) => container.querySelector<HTMLElement>(`[role=group][aria-label$="2 Zeilen, 3 Spalten"] [data-cell="${x},${y}"]`);
    /** Picks the area header 0001 at (0, 0) and (1, 0) of the world map Sinnoh (matrix 1) and goes to „Anordnung“. */
    async function toLayout(depth: Depth) {
      await show({ matrix: 1 }, depth);
      await click(cell(0, 0));
      await click(cell(1, 0));
      await next();
      expect(current()).toBe("2. Anordnung");
    }

    it("offers_own_map_and_each_world_map", async () => {
      await toLayout("bauen");
      expect(lage().map((b) => b.textContent)).toEqual([OWN, SINNOH]);
      expect(pressed()).toBe(OWN);
      act(() => root.unmount());
      root = createRoot(container);
      // HeartGold/SoulSilver: Johto and Kanto are halves of one world map, offered once.
      const hgss: World = {
        ...world,
        worldMaps: [
          { key: "w1", matrix: 1, filter: "johto", name: "Johto", areas: [] },
          { key: "w1k", matrix: 1, filter: "kanto", name: "Kanto", areas: [] },
        ],
      };
      await act(async () =>
        root.render(<NewAreaWizard depth="tuefteln" world={hgss} headers={headers} matrices={matrices} matricesLoaded matricesFailed={false} links={links} counts={{ Map: 600, AreaData: 20 }} prefill={{ matrix: 1 }} onClose={onClose} onCreated={onCreated} onHeaderPatch={onHeaderPatch} onError={onError} onDraft={onDraft} />),
      );
      await flush();
      await click(cell(0, 0));
      await next();
      expect(lage().map((b) => b.textContent)).toEqual([OWN, "Auf der Weltkarte Johto und Kanto (nahtlos begehbar)"]);
    });

    it("sends_the_target_and_says_it_in_the_summary_in_bauen", async () => {
      const words = () => visibleWords(document.body);
      await toLayout("bauen");
      await choose(SINNOH);
      await flush();
      // No cell chosen yet: „Weiter“ waits for one, with the grid's hint and no error.
      expect(container.querySelector('[role=group][aria-label="Weltkarte, 2 Zeilen, 3 Spalten"]')).not.toBeNull();
      expect(button("Weiter")?.disabled).toBe(true);
      expect(container.textContent).toContain("Die Zelle anklicken, an der die linke obere Karte des neuen Gebiets liegen soll.");
      expect(container.textContent).not.toContain("Die Zelle wählen");
      expect(container.querySelector("[role=alert]")).toBeNull();
      expect(words()).not.toMatch(FORBIDDEN);
      // (2, 0) is empty, but the 2 × 1 block reaches past the edge.
      await click(worldCell(2, 0));
      expect(worldCell(2, 0)?.hasAttribute("data-foreign")).toBe(true);
      expect(container.textContent).toContain("Der Block muss auf freie Zellen oder Landschaft fallen.");
      expect(button("Weiter")?.disabled).toBe(true);
      // (1, 1) and (2, 1) are landscape.
      await click(worldCell(1, 1));
      expect(container.textContent).not.toContain("Der Block muss auf freie Zellen oder Landschaft fallen.");
      expect(button("Weiter")?.disabled).toBe(false);
      await next();
      await flush();
      expect(current()).toBe("3. Ort");
      await next();
      expect(current()).toBe("4. Zusammenfassung");
      expect(container.textContent).toContain("Liegt auf der Weltkarte Sinnoh bei x 1, y 1 (nahtlos begehbar); ersetzt 2 Landschaftskarten.");
      expect(words()).not.toMatch(FORBIDDEN);
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].target).toEqual({ matrix: 1, x: 1, y: 1 });
    });

    it("keeps_the_target_when_switching_back", async () => {
      await toLayout("tuefteln");
      await choose(SINNOH);
      await flush();
      // A world map ignores the switch for the header section.
      expect([...container.querySelectorAll("label")].some((l) => l.textContent === "Header-Abschnitt anlegen")).toBe(false);
      await click(worldCell(0, 1));
      expect(worldCell(0, 1)?.getAttribute("aria-pressed")).toBe("true");
      await choose(OWN);
      expect(worldCell(0, 1)).toBeNull();
      expect(button("Weiter")?.disabled).toBe(false);
      await choose(SINNOH);
      await flush();
      expect(pressed()).toBe(SINNOH);
      expect(worldCell(0, 1)?.getAttribute("aria-pressed")).toBe("true");
      expect(worldCell(1, 1)?.getAttribute("aria-pressed")).toBe("true");
      await next();
      await next();
      expect(container.textContent).toContain("Ziel: Raster 0001, Zelle 0, 1; 2 Landschaftszellen werden ersetzt.");
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0].target).toEqual({ matrix: 1, x: 0, y: 1 });
      act(() => root.unmount());
      root = createRoot(container);
      vi.mocked(createArea).mockClear();
      // Back on „Eigene Karte“ the target stays unsent.
      await toLayout("tuefteln");
      await choose(SINNOH);
      await flush();
      await click(worldCell(0, 1));
      await choose(OWN);
      await next();
      await next();
      await click(button("Anlegen"));
      expect(vi.mocked(createArea).mock.calls[0][0]).not.toHaveProperty("target");
    });
  });

  it("closes_with_escape_from_its_first_control_the_source_select", async () => {
    // 3c.2 review of Task 5a: the native select took the first Escape.
    await show();
    expect(document.activeElement).toBe(source());
    pressKey(source()!, "Escape");
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it("closes_with_escape_and_reports_that_nothing_was_written", async () => {
    await show();
    await act(async () => void container.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(createArea).not.toHaveBeenCalled();
  });

  it("shows_the_draft_while_cells_are_chosen_and_clears_it_on_cancel", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    // The host takes the wizard away when it closes.
    onClose.mockImplementation(() => root.render(null));
    await show();
    // No cell chosen: nothing to plan.
    await settle();
    expect(recipePlan).not.toHaveBeenCalled();
    await click(cell(0, 0));
    await click(cell(1, 0));
    await settle();
    expect(recipePlan).toHaveBeenCalledOnce();
    const [recipe, input] = vi.mocked(recipePlan).mock.calls[0];
    expect(recipe).toBe("create_area");
    expect((input as { cells: unknown[] }).cells).toHaveLength(2);
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await next();
    await next();
    await next();
    const notes = container.querySelector("[data-draft-notes]");
    expect(notes?.textContent).toContain("Legt Gebiet „Neues Gebiet“ mit 2 Karten an.");
    expect(notes?.textContent).toContain("2 Karten werden kopiert");
    await act(async () => void container.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });

  it("plans_the_same_spec_it_creates_and_clears_the_draft_after", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await show();
    await click(cell(0, 0));
    await click(cell(1, 0));
    await next();
    await next();
    await next();
    await act(async () => void (await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)));
    expect(onDraft).toHaveBeenLastCalledWith(planned);
    await click(button("Anlegen"));
    // „Anlegen“ sends the spec the draft was planned from.
    expect(vi.mocked(recipePlan).mock.lastCall?.[1]).toEqual(vi.mocked(createArea).mock.calls[0][0]);
    expect(onDraft).toHaveBeenLastCalledWith(null);
  });
});
