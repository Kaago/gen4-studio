// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto } from "../api";
import { ThumbnailCache } from "../world/thumbnails";
import { AreaGrid } from "./AreaGrid";
import type { AreaGridProps } from "./AreaGrid";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const matrix: MatrixDto = { index: 3, name: "m", width: 3, height: 2, maps: [521, 522, null, 523, null, 524], headers: [415, 416, 0, 417, 0, 0], altitudes: [0, 0, 0, 0, 0, 2] };
const header = (id: number, area: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_name: "", matrix_id: 3, area_data_id: area, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(415, 7), header(416, 7), header(417, 8)];

describe("AreaGrid", () => {
  let container: HTMLElement;
  let root: Root;
  let glb: ReturnType<typeof vi.fn<(map: number, area: number) => Promise<ArrayBuffer>>>;
  let cache: ThumbnailCache;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    glb = vi.fn(async () => new ArrayBuffer(2));
    cache = new ThumbnailCache({ glb }, async () => "data:image/png;base64,AAAA");
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (props: Partial<AreaGridProps> = {}) =>
    act(() => root.render(<AreaGrid matrix={matrix} headers={headers} areaFallback={1} cellSize={64} cache={cache} {...props} />));
  const cell = (x: number, y: number) => container.querySelector<HTMLElement>(`[data-cell="${x},${y}"]`);

  it("names_the_grid_with_the_word_of_the_ui_vocabulary", () => {
    show();
    const name = container.querySelector('[role="group"]')?.getAttribute("aria-label");
    expect(name).toMatch(/^Raster \d+, \d+ Zeilen, \d+ Spalten$/);
  });

  it("renders_one_cell_per_grid_position", () => {
    show();
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(6);
    expect(container.querySelectorAll("[data-extend]")).toHaveLength(0);
  });

  it("adds_a_plus_column_row_and_corner_and_reports_the_target_cell", () => {
    const onExtend = vi.fn();
    show({ onExtend });
    const plus = [...container.querySelectorAll("[data-extend]")].map((e) => e.getAttribute("data-extend"));
    expect(plus).toHaveLength(matrix.width + matrix.height + 1);
    expect(plus).toEqual(expect.arrayContaining(["3,0", "3,1", "0,2", "1,2", "2,2", "3,2"]));
    act(() => container.querySelector<HTMLElement>('[data-extend="3,0"]')?.click());
    expect(onExtend).toHaveBeenCalledWith(3, 0);
  });

  it("reports_clicks_with_the_shift_modifier_and_double_clicks", () => {
    const onCellClick = vi.fn();
    const onCellDoubleClick = vi.fn();
    show({ onCellClick, onCellDoubleClick });
    act(() => cell(1, 0)?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    act(() => cell(1, 0)?.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true })));
    act(() => cell(1, 0)?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(onCellClick.mock.calls).toEqual([
      [1, 0, { shift: false }],
      [1, 0, { shift: true }],
    ]);
    expect(onCellDoubleClick).toHaveBeenCalledWith(1, 0);
  });

  it("marks_selected_cells_as_pressed", () => {
    show({ selected: new Set(["1,0", "2,1"]) });
    const pressed = [...container.querySelectorAll('[data-cell][aria-pressed="true"]')].map((e) => e.getAttribute("data-cell"));
    expect(pressed.sort()).toEqual(["1,0", "2,1"]);
  });

  it("shows_the_thumbnail_once_it_is_ready_and_asks_the_source_once_per_filled_cell", async () => {
    show();
    expect((cell(0, 0)?.getAttribute("style") ?? "")).not.toContain('url("data:');
    await flush();
    expect((cell(0, 0)?.getAttribute("style") ?? "")).toContain('url("data:');
    expect(glb).toHaveBeenCalledTimes(4);
  });

  it("draws_each_cell_with_the_tileset_of_its_header_and_the_fallback_without_one", async () => {
    show({ matrix: { ...matrix, headers: [415, 416, 0, 417, 0, 0] } });
    await flush();
    const calls = glb.mock.calls.map(([map, area]) => `${map}:${area}`).sort();
    // 521 and 522 use areas 7, 523 uses 8, 524 has header 0 which no header row lists: the fallback 1.
    expect(calls).toEqual(["521:7", "522:7", "523:8", "524:1"]);
  });

  it("uses_the_fallback_for_a_grid_without_a_header_section", async () => {
    show({ matrix: { ...matrix, headers: undefined } });
    await flush();
    expect(glb.mock.calls.every(([, area]) => area === 1)).toBe(true);
  });

  it("titles_cells_with_their_parts_and_dashes_empty_ones", () => {
    show();
    expect(cell(1, 0)?.getAttribute("title")).toBe("x 1, y 0 · Karte 0522 · Header 0416 · Höhe 0");
    expect(cell(2, 1)?.getAttribute("title")).toBe("x 2, y 1 · Karte 0524 · Header 0000 · Höhe 2");
    expect(cell(2, 0)?.getAttribute("title")).toBe("x 2, y 0 · Höhe 0");
    expect(cell(2, 0)?.textContent).toBe("—");
  });

  it("shows_map_and_header_ids_only_at_size_64", () => {
    show({ cellSize: 64 });
    expect(cell(0, 0)?.textContent).toContain("0521");
    expect(cell(0, 0)?.textContent).toContain("0415");
    show({ cellSize: 32 });
    expect(cell(0, 0)?.textContent).not.toContain("0415");
  });

  it("writes_no_raw_values_into_the_cells_in_bauen", () => {
    show();
    expect(container.querySelectorAll("[data-raw]")).toHaveLength(0);
  });

  it("writes_map_header_and_altitude_into_every_filled_cell_in_tuefteln", () => {
    show({ tinker: true });
    expect(cell(0, 0)?.getAttribute("data-raw")).toBe("0521 0415 0");
    expect(cell(2, 1)?.getAttribute("data-raw")).toBe("0524 0000 2");
    // An empty cell has no map to describe.
    expect(cell(2, 0)?.hasAttribute("data-raw")).toBe(false);
    expect(container.querySelectorAll("[data-raw]")).toHaveLength(4);
    const lines = [...(cell(2, 1)?.querySelectorAll("span span") ?? [])].map((e) => e.textContent);
    expect(lines).toEqual(["0524", "0000", "2"]);
  });

  it("leaves_the_header_line_out_for_a_grid_without_a_header_section_in_tuefteln", () => {
    show({ tinker: true, matrix: { ...matrix, headers: undefined } });
    expect(cell(0, 0)?.getAttribute("data-raw")).toBe("0521 0");
  });

  it("tints_cells_by_the_colour_of_their_header", () => {
    show({ colorOf: (h) => (h === 415 ? "rgb(1, 2, 3)" : null) });
    expect((cell(0, 0)?.getAttribute("style") ?? "")).toContain("rgb(1, 2, 3)");
    expect((cell(1, 0)?.getAttribute("style") ?? "")).not.toContain("rgb(1, 2, 3)");
  });

  describe("with an IntersectionObserver", () => {
    interface Fake {
      element: Element;
      callback: IntersectionObserverCallback;
      disconnected: boolean;
    }
    let observers: Fake[];
    beforeEach(() => {
      observers = [];
      class FakeObserver {
        private readonly fake: Fake;
        constructor(callback: IntersectionObserverCallback) {
          this.fake = { element: document.body, callback, disconnected: false };
          observers.push(this.fake);
        }
        observe(element: Element) {
          this.fake.element = element;
        }
        disconnect() {
          this.fake.disconnected = true;
        }
      }
      vi.stubGlobal("IntersectionObserver", FakeObserver);
    });
    afterEach(() => vi.unstubAllGlobals());

    const setVisible = (x: number, y: number, isIntersecting: boolean) => {
      const fake = observers.find((o) => o.element === cell(x, y) && !o.disconnected);
      if (!fake) throw new Error(`no observer for cell ${x},${y}`);
      act(() => fake.callback([{ isIntersecting } as IntersectionObserverEntry], fake as unknown as IntersectionObserver));
    };

    it("does_not_request_pictures_of_cells_out_of_view_and_requests_when_they_scroll_in", async () => {
      show();
      await flush();
      expect(glb).not.toHaveBeenCalled();
      expect((cell(0, 0)?.getAttribute("style") ?? "")).not.toContain('url("data:');

      setVisible(1, 0, true);
      await flush();
      expect(glb.mock.calls).toEqual([[522, 7]]);
      expect((cell(1, 0)?.getAttribute("style") ?? "")).toContain('url("data:');
      expect((cell(0, 0)?.getAttribute("style") ?? "")).not.toContain('url("data:');
    });

    it("observes_only_filled_cells", () => {
      show();
      expect(observers).toHaveLength(4);
    });

    it("withdraws_the_queued_request_of_a_cell_that_leaves_the_view", () => {
      // Concurrency 1 keeps the second request queued behind the first, which never settles.
      cache = new ThumbnailCache({ glb: vi.fn(() => new Promise<ArrayBuffer>(() => {})) }, async () => "data:x", { concurrency: 1 });
      show();
      setVisible(0, 0, true);
      setVisible(1, 0, true);
      expect(cache.pending()).toBe(2);
      setVisible(1, 0, false);
      expect(cache.pending()).toBe(1);
    });
  });

  it("renders_again_only_the_cells_whose_selection_changed", () => {
    const onCellClick = () => {};
    show({ onCellClick, selected: new Set(["0,0"]) });
    const reads = vi.spyOn(cache, "get");
    show({ onCellClick, selected: new Set(["1,0"]) });
    // Each rendered cell reads its picture once or twice (React may read a store snapshot twice): the two cells whose
    // selection changed, not all four filled cells.
    const keys = new Set(reads.mock.calls.map(([map, area]) => `${map}:${area}`));
    expect([...keys].sort()).toEqual(["521:7", "522:7"]);
  });
});
