// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixDto } from "../api";
import { areaHeadersOf, filledRect, sceneScope, scopedMatrix, useSceneScope } from "./sceneScope";
import type { World } from "./worldModel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// 6 x 5, header 0 is the landscape. Area "w0:1" has two sections (7 and 8); header 9 lies at the right and bottom edge.
// row 0: 0 0 0 0 0 0
// row 1: 0 7 7 0 0 0
// row 2: 0 7 8 0 0 0
// row 3: 0 0 0 0 0 9
// row 4: 0 0 0 0 0 9
const HEADERS = [0, 0, 0, 0, 0, 0, 0, 7, 7, 0, 0, 0, 0, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9];
const matrix: MatrixDto = {
  index: 0,
  name: "Welt",
  width: 6,
  height: 5,
  maps: HEADERS.map((_, i) => 100 + i),
  headers: HEADERS,
  altitudes: HEADERS.map((_, i) => i % 3),
};
const at = (x: number, y: number) => y * matrix.width + x;
const cellsOf = (indices: Iterable<number>) => [...indices].sort((a, b) => a - b).map((i) => [i % matrix.width, Math.floor(i / matrix.width)]);
const block = (x0: number, x1: number, y0: number, y1: number) => {
  const out: number[][] = [];
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) out.push([x, y]);
  return out;
};

const header = (id: number, matrix_id = 0): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "Route 204",
  location_index: 5,
  kanto: false,
  map_type: 2,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const world = (): World => ({
  worldMaps: [
    {
      key: "w0",
      matrix: 0,
      filter: null,
      name: "Sinnoh",
      areas: [{ key: "w0:1", name: "Route 204", placeKey: "Route 204", headers: [header(7), header(8)], rooms: [], matrix: 0 }],
    },
  ],
  moreAreas: [],
  unconnected: [],
  other: [],
  // Header 20 is a room assigned to the area; it lies on a matrix of its own.
  areaOf: new Map([
    [7, "w0:1"],
    [8, "w0:1"],
    [20, "w0:1"],
  ]),
  roomCount: 1,
  assignedRooms: 1,
});

describe("sceneScope", () => {
  it("loads_the_area_cells_and_the_ring_of_eight_neighbours", () => {
    const scope = sceneScope(matrix, [7])!;
    // The cells of header 7 and every cell touching one of them, corners included; (3, 3) touches none.
    expect(cellsOf(scope.cells)).toEqual([...block(0, 3, 0, 2), ...block(0, 2, 3, 3)]);
  });

  it("keeps_the_ring_inside_the_matrix_at_the_edges", () => {
    const scope = sceneScope(matrix, [9])!;
    expect(cellsOf(scope.cells)).toEqual(block(4, 5, 2, 4));
    // No index wrapped into the next row's first column, none past the end.
    for (const i of scope.cells) expect(i >= 0 && i < matrix.maps.length).toBe(true);
    expect(scope.cells.has(at(0, 3))).toBe(false);
    expect(scope.cells.has(at(0, 4))).toBe(false);
  });

  it("takes_both_sections_of_an_area", () => {
    const scope = sceneScope(matrix, [7, 8])!;
    expect(cellsOf(scope.cells)).toEqual(block(0, 3, 0, 3));
  });

  it("frames_only_the_area_cells_without_the_ring", () => {
    expect(sceneScope(matrix, [7])!.focus).toEqual({ x: 1, y: 1, width: 2, height: 2 });
    expect(sceneScope(matrix, [7, 8])!.focus).toEqual({ x: 1, y: 1, width: 2, height: 2 });
    expect(sceneScope(matrix, [9])!.focus).toEqual({ x: 5, y: 3, width: 1, height: 2 });
  });

  it("loads_and_frames_both_islands_of_an_area", () => {
    // Header 8 also holds the cells of header 9: one header, two islands.
    const islands: MatrixDto = { ...matrix, headers: HEADERS.map((h) => (h === 9 ? 8 : h)) };
    const scope = sceneScope(islands, [8])!;
    expect(cellsOf(scope.cells)).toEqual([...block(1, 3, 1, 1), ...block(1, 5, 2, 3), ...block(4, 5, 4, 4)]);
    expect(scope.focus).toEqual({ x: 2, y: 2, width: 4, height: 3 });
  });

  it("loads_the_whole_matrix_for_a_room_or_an_area_with_its_own_map", () => {
    const own: MatrixDto = { index: 3, name: "Haus", width: 2, height: 2, maps: [1, 2, 3, null] };
    expect(sceneScope(own, [7])).toBeNull();
    expect(sceneScope({ ...own, headers: null }, [7])).toBeNull();
    expect(sceneScope(matrix, null)).toBeNull();
    expect(sceneScope(matrix, [])).toBeNull();
  });

  it("loads_the_whole_matrix_when_no_cell_carries_the_area", () => {
    expect(sceneScope(matrix, [42])).toBeNull();
    // A cell that carries the header but has no map is no cell of the area.
    const empty = { ...matrix, maps: matrix.maps.map((m, i) => (HEADERS[i] === 9 ? null : m)) };
    expect(sceneScope(empty, [9])).toBeNull();
  });

  it("masks_the_maps_outside_the_scope_and_keeps_size_altitudes_and_headers", () => {
    const scope = sceneScope(matrix, [9])!;
    const masked = scopedMatrix(matrix, scope);
    expect(masked).not.toBe(matrix);
    expect(masked.index).toBe(matrix.index);
    expect(masked.name).toBe(matrix.name);
    expect(masked.width).toBe(6);
    expect(masked.height).toBe(5);
    expect(masked.headers).toEqual(matrix.headers);
    expect(masked.altitudes).toEqual(matrix.altitudes);
    expect(masked.maps).toHaveLength(30);
    masked.maps.forEach((m, i) => expect(m).toBe(scope.cells.has(i) ? matrix.maps[i] : null));
    expect(masked.maps.filter((m) => m !== null)).toHaveLength(6);
    // The input stays as it was.
    expect(matrix.maps.every((m) => m !== null)).toBe(true);
  });

  it("returns_the_same_matrix_object_for_no_scope", () => {
    expect(scopedMatrix(matrix, null)).toBe(matrix);
  });

  it("frames_the_filled_cells_of_a_whole_matrix", () => {
    const own: MatrixDto = { index: 3, name: "Haus", width: 4, height: 3, maps: [null, null, null, null, null, 1, 2, null, null, 3, null, null] };
    expect(filledRect(own)).toEqual({ x: 1, y: 1, width: 2, height: 2 });
    expect(filledRect({ ...own, maps: own.maps.map(() => null) })).toBeNull();
  });
});

describe("areaHeadersOf", () => {
  it("areaHeadersOf_takes_all_headers_of_the_area_of_the_selected_header", () => {
    expect(areaHeadersOf(world(), header(8))).toEqual([7, 8]);
    expect(areaHeadersOf(world(), header(7))).toEqual([7, 8]);
    // A room assigned to the area lies on another matrix: the area is not in its scene.
    expect(areaHeadersOf(world(), header(20, 4))).toEqual([20]);
    // A header in no area stands alone.
    expect(areaHeadersOf(world(), header(9))).toEqual([9]);
    expect(areaHeadersOf(world(), null)).toBeNull();
  });

  it("areaHeadersOf_is_the_header_alone_before_the_world_is_read", () => {
    expect(areaHeadersOf(null, header(8))).toEqual([8]);
    expect(areaHeadersOf(null, null)).toBeNull();
  });
});

describe("useSceneScope", () => {
  let container: HTMLElement;
  let root: Root;
  const seen: ReturnType<typeof useSceneScope>[] = [];
  const last = () => seen[seen.length - 1];
  function Probe({ m, h, w }: { m: MatrixDto | null; h: HeaderSummary | null; w: World | null }) {
    seen.push(useSceneScope(m, h, w));
    return null;
  }
  const show = (m: MatrixDto | null, h: HeaderSummary | null, w: World | null) => act(() => root.render(createElement(Probe, { m, h, w })));
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
    seen.length = 0;
  });
  afterEach(() => act(() => root.unmount()));

  it("keeps_the_masked_matrix_object_while_inputs_stay", () => {
    show(matrix, header(8), world());
    const first = last();
    expect(first.focus).toEqual({ x: 1, y: 1, width: 2, height: 2 });
    expect(first.matrix).not.toBe(matrix);
    expect(first.matrix!.maps.filter((m) => m !== null)).toHaveLength(16);
    // A world read again and another header object of the same area: the same objects.
    show(matrix, header(7), world());
    expect(last()).toBe(first);
    expect(last().matrix).toBe(first.matrix);
    expect(last().focus).toBe(first.focus);
    // New matrix data: a new mask.
    const grown = { ...matrix, maps: [...matrix.maps] };
    show(grown, header(7), world());
    expect(last().matrix).not.toBe(first.matrix);
    // Another header selected, its matrix not yet fetched: the scope stays.
    const kept = last();
    show(grown, header(9), world());
    expect(last()).toBe(kept);
    // Its matrix arrives (a new object): the header counts now; before the world is read, the header alone.
    const fetched = { ...matrix, maps: [...matrix.maps] };
    show(fetched, header(9), null);
    expect(last().focus).toEqual({ x: 5, y: 3, width: 1, height: 2 });
    // No scope: the matrix itself, and no focus.
    const none = { ...matrix };
    show(none, null, world());
    expect(last()).toEqual({ matrix: none, focus: null });
    expect(last().matrix).toBe(none);
    show(null, header(7), world());
    expect(last()).toEqual({ matrix: null, focus: null });
  });

  it("takes_the_area_of_the_shown_header_once_the_world_is_read", () => {
    show(matrix, header(8), null);
    expect(last().focus).toEqual({ x: 2, y: 2, width: 1, height: 1 });
    show(matrix, header(8), world());
    expect(last().focus).toEqual({ x: 1, y: 1, width: 2, height: 2 });
  });
});
