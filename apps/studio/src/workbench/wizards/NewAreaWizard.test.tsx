// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createArea, getMatrix } from "../../api";
import type { AreaReport, HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../../api";
import { NewAreaWizard } from "./NewAreaWizard";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), createArea: vi.fn(), getMatrix: vi.fn() }));
// The grids ask the app-wide cache for map pictures; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const header = (id: number, internal: string, matrix = 0, area = 0, event = 0): HeaderSummary => ({ id, internal_name: internal, location_name: "", matrix_id: matrix, area_data_id: area, event_file_id: event });
const headers = [header(0, "ZERO"), header(1, "NOTHING", 0, 3, 8)];
const summary = (index: number, width: number, height: number, hasHeaders = false): MatrixSummary => ({ index, name: `m${index}`, width, height, has_headers: hasHeaders, has_altitudes: false, filled: width * height });
const matrices = [summary(0, 2, 1), summary(1, 3, 2, true)];
const links: WarpLink[] = [{ event_file: 8, index: 0, x: 1, z: 1, dest_header_id: 1, dest_warp_id: 0 }];
const matrixOf = (index: number): MatrixDto =>
  index === 0
    ? { index: 0, name: "m0", width: 2, height: 1, maps: [0, 1] }
    : { index: 1, name: "m1", width: 3, height: 2, maps: [10, 11, null, 13, 14, 15], headers: [1, 1, 0, 0, 0, 0] };
const report: AreaReport = { matrix: 131, maps: [[0, 0, 571], [1, 0, 572]], event_file: null, header: null };

describe("NewAreaWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onCreated = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMatrix).mockReset().mockImplementation(async (index) => matrixOf(index));
    vi.mocked(createArea).mockReset().mockResolvedValue(report);
    onClose.mockReset();
    onCreated.mockReset().mockResolvedValue(undefined);
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const wizard = (prefill: { matrix?: number } | undefined, listed: MatrixSummary[] = matrices) => (
    <NewAreaWizard headers={headers} matrices={listed} links={links} counts={{ Map: 600, AreaData: 20 }} prefill={prefill} onClose={onClose} onCreated={onCreated} onError={onError} />
  );
  async function show(prefill: { matrix?: number } | undefined = { matrix: 0 }) {
    await act(async () => root.render(wizard(prefill)));
    await flush();
  }
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);
  const cell = (x: number, y: number) => container.querySelector<HTMLElement>(`[data-cell="${x},${y}"]`);
  const click = (el: Element | null | undefined, init: MouseEventInit = {}) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true, ...init })));
  const next = () => click(button("Weiter"));
  const selected = () => [...container.querySelectorAll("[data-cell][aria-pressed=true]")].map((e) => e.getAttribute("data-cell"));

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
    await click(button("Im Studio zeigen"));
    expect(onCreated).toHaveBeenCalledWith(report, spec);
  });

  it("walks_the_steps_in_order_and_gates_weiter_on_a_selection", async () => {
    await show();
    const current = () => container.querySelector('[aria-current="step"]')?.textContent;
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
    expect(container.querySelector<HTMLSelectElement>("select")?.value).toBe("1");
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(6);
    await click(cell(0, 0));
    await act(async () => {
      const select = container.querySelector<HTMLSelectElement>("select")!;
      select.value = "0";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await flush();
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
    expect(selected()).toEqual([]);
  });

  describe("the source grid", () => {
    const placed = [{ ...header(0, "ZERO"), location_name: "Sandgem" }, { ...header(1, "NOTHING", 1, 3, 8), location_name: "Route 201" }];
    const caption = () => container.querySelector('[aria-live="polite"]')?.textContent;
    const hover = (x: number, y: number) => act(async () => void cell(x, y)?.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })));

    async function open(prefill: { matrix?: number; header?: number }) {
      await act(async () =>
        root.render(<NewAreaWizard headers={placed} matrices={matrices} links={links} counts={{ Map: 600, AreaData: 20 }} prefill={prefill} onClose={onClose} onCreated={onCreated} onError={onError} />),
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
    await click([...container.querySelectorAll("button")].find((b) => b.textContent?.startsWith("0001")));
    await next();
    expect(container.textContent).toContain("Eventdatei: neu, leer.");
    await click(button("Anlegen"));
    expect(vi.mocked(createArea).mock.calls[0][0].header?.event_file).toBe("New");
  });

  it("falls_back_to_the_first_matrix_once_the_overview_arrives_when_nothing_is_prefilled", async () => {
    await act(async () => root.render(wizard(undefined, [])));
    await flush();
    expect(getMatrix).not.toHaveBeenCalled();
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(0);
    await act(async () => root.render(wizard(undefined)));
    await flush();
    expect(getMatrix).toHaveBeenCalledWith(0);
    expect(container.querySelector<HTMLSelectElement>("select")?.value).toBe("0");
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
  });

  it("keeps_a_prefilled_matrix_when_the_overview_arrives_later", async () => {
    await act(async () => root.render(wizard({ matrix: 1 }, [])));
    await act(async () => root.render(wizard({ matrix: 1 })));
    await flush();
    expect(container.querySelector<HTMLSelectElement>("select")?.value).toBe("1");
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
    // Header 0001 is picked from the list; the warp that leads to it is counted.
    await click([...container.querySelectorAll("button")].find((b) => b.textContent?.startsWith("0001")));
    expect(container.textContent).toContain("Warps hierher: 1");
    expect(container.textContent).not.toContain("nichts führt hierher");
    await click([...container.querySelectorAll("input[type=radio]")].find((r) => r.parentElement?.textContent === "Kopie von 0008"));
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
    const row = (id: string) => [...container.querySelectorAll("li")].find((li) => li.textContent?.startsWith(id))?.textContent;
    expect(row("0000")).toContain("Warps hierher: 0, wirkt unbenutzt");
    expect(row("0001")).toContain("Warps hierher: 1");
    expect(row("0001")).not.toContain("wirkt unbenutzt");
    await click([...container.querySelectorAll("button")].find((b) => b.textContent?.startsWith("0000")));
    expect(container.textContent).toContain("nichts führt hierher");
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
    expect(container.querySelector('[aria-current="step"]')?.textContent).toBe("4. Zusammenfassung");
    expect(button("Anlegen")?.disabled).toBe(false);
    expect(button("Im Studio zeigen")).toBeUndefined();
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
    vi.mocked(createArea).mockRejectedValue({ code: "error", message: "matrix 0131 and maps 0571, 0572 were created; header edit failed: boom", asset: null });
    await show();
    await click(cell(0, 0));
    await next();
    await next();
    await next();
    await click(button("Anlegen"));
    expect(onError).not.toHaveBeenCalled();
    expect(container.textContent).toContain("matrix 0131 and maps 0571, 0572 were created; header edit failed: boom");
    expect(container.textContent).toContain("Raster-Tab");
    expect(container.textContent).toContain("Header-Editor");
    expect(button("Im Studio zeigen")).toBeUndefined();
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  it("closes_with_escape_and_reports_that_nothing_was_written", async () => {
    await show();
    await act(async () => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledWith(false);
    expect(createArea).not.toHaveBeenCalled();
  });
});
