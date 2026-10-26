// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import type { World } from "../worldModel";
import { PlaceOnWorldMap } from "./PlaceOnWorldMap";

// The grid asks the app-wide cache for map pictures; there is no renderer here.
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_index: 0, kanto: false, location_name: "", matrix_id: 1, area_data_id: 0, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const headers = [header(0), header(1)];
/** 3 × 2: the area of header 1 at (0, 0) and (1, 0), an empty cell at (2, 0), landscape below. */
const matrix: MatrixDto = { index: 1, name: "m1", width: 3, height: 2, maps: [10, 11, null, 13, 14, 15], headers: [1, 1, 0, 0, 0, 0] };
const world: World = { worldMaps: [{ key: "w1", matrix: 1, filter: null, name: "Sinnoh", areas: [] }], moreAreas: [], unconnected: [], other: [], areaOf: new Map(), roomCount: 0, assignedRooms: 0 };
const SENTENCE = "Der Block muss auf freie Zellen oder Landschaft fallen.";

describe("PlaceOnWorldMap", () => {
  let container: HTMLElement;
  let root: Root;
  const onChange = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onChange.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (value: { x: number; y: number } | null, depth: Depth = "bauen") =>
    act(() => root.render(<PlaceOnWorldMap world={world} matrix={matrix} headers={headers} width={2} height={1} value={value} onChange={onChange} depth={depth} focus={1} />));
  const cell = (x: number, y: number) => container.querySelector<HTMLElement>(`[data-cell="${x},${y}"]`);
  const marked = (attribute: string) => [...container.querySelectorAll(`[data-cell]${attribute}`)].map((e) => e.getAttribute("data-cell"));

  it("marks_the_block_and_blocks_next_on_a_taken_cell", () => {
    show({ x: 0, y: 1 });
    expect(marked("[aria-pressed=true]")).toEqual(["0,1", "1,1"]);
    expect(marked("[data-foreign]")).toEqual([]);
    expect(container.textContent).not.toContain(SENTENCE);
    // (1, 0) belongs to an area: the block is marked red and the sentence says why „Weiter“ is locked.
    show({ x: 1, y: 0 });
    expect(marked("[data-foreign]")).toEqual(["1,0", "2,0"]);
    expect(marked("[aria-pressed=true]")).toEqual([]);
    expect(container.querySelector("[role=alert]")?.textContent).toBe(SENTENCE);
    // Past the edge of the world map as well.
    show({ x: 2, y: 1 });
    expect(marked("[data-foreign]")).toEqual(["2,1"]);
    expect(container.textContent).toContain(SENTENCE);
  });

  it("picks_the_top_left_cell", () => {
    show(null);
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("Weltkarte, 2 Zeilen, 3 Spalten");
    expect(marked("[aria-pressed=true]")).toEqual([]);
    act(() => cell(1, 1)?.click());
    expect(onChange).toHaveBeenCalledWith({ x: 1, y: 1 });
    act(() => cell(2, 0)?.click());
    expect(onChange).toHaveBeenLastCalledWith({ x: 2, y: 0 });
    // A cell of an area is no place for the corner.
    onChange.mockClear();
    act(() => cell(0, 0)?.click());
    expect(onChange).not.toHaveBeenCalled();
    // Tüfteln keeps the grid's own name.
    show(null, "tuefteln");
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("Raster 1, 2 Zeilen, 3 Spalten");
  });
});
