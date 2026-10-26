import { describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, WarpLink } from "../api";
import { buildPlaces, placeColor } from "./places";
import { cellBounds, centredPan, lineBetween, placeCells, regionMatrix, roomLinkIndex, roomLinks, wheelPixels } from "./worldMap";

const summary = (index: number, width: number, height: number, has_headers: boolean, filled: number): MatrixSummary => ({
  index,
  name: `m${index}`,
  width,
  height,
  has_headers,
  has_altitudes: false,
  filled,
});

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

const warp = (event_file: number, index: number, dest_header_id: number, dest_warp_id: number, x = 0, z = 0): WarpLink => ({ event_file, index, x, z, dest_header_id, dest_warp_id });

describe("regionMatrix", () => {
  it("takes_the_header_matrix_with_the_most_filled_cells", () => {
    const list = [summary(0, 1, 1, false, 1), summary(1, 30, 30, true, 500), summary(2, 4, 4, true, 10)];
    expect(regionMatrix(list)?.index).toBe(1);
  });

  it("breaks_a_tie_by_the_smallest_index", () => {
    const list = [summary(5, 3, 3, true, 9), summary(2, 3, 3, true, 9), summary(7, 3, 3, true, 9)];
    expect(regionMatrix(list)?.index).toBe(2);
  });

  it("is_null_without_a_header_section", () => {
    expect(regionMatrix([summary(0, 30, 30, false, 900), summary(1, 2, 2, false, 4)])).toBeNull();
    expect(regionMatrix([])).toBeNull();
  });
});

const headers = [header(0, "Zweiblattdorf", 0), header(1, "Zweiblattdorf", 7, 11), header(3, "Route 201", 0), header(4, "", 9)];
const matrices = [summary(0, 30, 30, true, 900), summary(7, 1, 1, false, 1), summary(9, 1, 1, false, 1)];
const places = buildPlaces(headers, matrices);

const region = (maps: (number | null)[], cellHeaders: number[], width = maps.length, height = 1): MatrixDto => ({ index: 0, name: "r", width, height, maps, headers: cellHeaders });

describe("placeCells", () => {
  it("collects_the_cells_of_each_place_with_their_centre_and_colour", () => {
    const cells = placeCells(region([100, 101], [0, 3]), places);
    expect(cells.map((c) => c.name)).toEqual(["Zweiblattdorf", "Route 201"]);
    expect(cells[0]).toEqual({ key: "Zweiblattdorf", name: "Zweiblattdorf", color: placeColor("Zweiblattdorf"), cells: [{ x: 0, y: 0 }], center: { x: 0, y: 0 } });
    expect(cells[1].cells).toEqual([{ x: 1, y: 0 }]);
    expect(cells[1].center).toEqual({ x: 1, y: 0 });
  });

  it("centres_a_place_on_the_mean_of_its_cells", () => {
    const [dorf] = placeCells(region([1, 2, 3, 4], [0, 0, 0, 0], 2, 2), places);
    expect(dorf.cells).toHaveLength(4);
    expect(dorf.center).toEqual({ x: 0.5, y: 0.5 });
  });

  it("leaves_out_cells_without_a_map_and_headers_without_a_place", () => {
    const cells = placeCells(region([100, null, 102], [0, 3, 99]), places);
    expect(cells).toHaveLength(1);
    expect(cells[0].cells).toEqual([{ x: 0, y: 0 }]);
  });

  it("treats_header_zero_like_any_other_header_but_only_with_a_map", () => {
    expect(placeCells(region([null], [0]), places)).toEqual([]);
  });

  it("is_empty_without_a_header_section", () => {
    expect(placeCells({ ...region([1], [0]), headers: null }, places)).toEqual([]);
  });
});

describe("roomLinks", () => {
  const dorf = places[0];
  const outside = warp(10, 2, 1, 0, 40, 5);
  const exit = warp(11, 0, 0, 2);

  it("leads_a_room_to_the_cell_of_its_entrance", () => {
    const [link] = roomLinks(dorf, headers, [exit, outside], region([1, 2], [0, 0]));
    expect(link).toEqual({ header: 1, cell: { x: 1, y: 0 }, reason: null });
  });

  it("skips_the_outside_headers_of_the_place", () => {
    expect(roomLinks(dorf, headers, [exit, outside], region([1, 2], [0, 0])).map((l) => l.header)).toEqual([1]);
  });

  it("says_when_the_room_has_no_exit", () => {
    const [link] = roomLinks(dorf, headers, [outside], region([1, 2], [0, 0]));
    expect(link).toEqual({ header: 1, cell: null, reason: "kein Ausgang" });
  });

  it("says_when_the_target_warp_is_missing", () => {
    const [link] = roomLinks(dorf, headers, [exit], region([1, 2], [0, 0]));
    expect(link).toEqual({ header: 1, cell: null, reason: "Ziel-Warp fehlt" });
  });

  it("says_when_the_target_is_not_an_outside_header", () => {
    const toRoom = warp(11, 0, 4, 0);
    const [link] = roomLinks(dorf, headers, [toRoom], region([1, 2], [0, 0]));
    expect(link).toEqual({ header: 1, cell: null, reason: "Ziel liegt nicht auf der Weltkarte" });
    const unknown = warp(11, 0, 99, 0);
    expect(roomLinks(dorf, headers, [unknown], region([1, 2], [0, 0]))[0].reason).toBe("Ziel liegt nicht auf der Weltkarte");
  });

  it("says_when_the_entrance_lies_beyond_the_region", () => {
    const far = warp(10, 2, 1, 0, 40 + 32 * 5, 5);
    const [link] = roomLinks(dorf, headers, [exit, far], region([1, 2], [0, 0]));
    expect(link).toEqual({ header: 1, cell: null, reason: "Ziel außerhalb der Region" });
  });

  it("gives_rooms_that_share_an_event_file_their_own_entrance", () => {
    // Rooms 1 and 2 share event file 11: exits 1 (to the door pair 10/0) and 3 (to the door pair 10/1) are both in it.
    const doors = [warp(10, 0, 1, 1, 40, 5), warp(10, 1, 2, 3, 70, 5)];
    const exits = [warp(11, 1, 0, 0), warp(11, 3, 0, 1)];
    const shared = [header(0, "Zweiblattdorf", 0), header(1, "Zweiblattdorf", 7, 11), header(2, "Zweiblattdorf", 7, 11)];
    const town = buildPlaces(shared, matrices)[0];
    const result = roomLinks(town, shared, [...doors, ...exits], region([1, 2, 3], [0, 0, 0]));
    expect(result.map((l) => [l.header, l.cell])).toEqual([
      [1, { x: 1, y: 0 }],
      [2, { x: 2, y: 0 }],
    ]);
  });

  it("falls_back_to_the_first_resolving_exit_when_none_points_back", () => {
    const door = warp(10, 2, 3, 0, 40, 5);
    const [link] = roomLinks(dorf, headers, [exit, door], region([1, 2], [0, 0]));
    expect(link.cell).toEqual({ x: 1, y: 0 });
  });

  it("prefers_an_exit_that_resolves_over_one_that_does_not", () => {
    const toRoom = warp(11, 0, 4, 0);
    const good = warp(11, 1, 0, 2);
    const [link] = roomLinks(dorf, headers, [toRoom, good, outside], region([1, 2], [0, 0]));
    expect(link.cell).toEqual({ x: 1, y: 0 });
  });
});

describe("lineBetween", () => {
  const card = { left: 500, top: 100, width: 120, height: 40 } as DOMRect;

  it("runs_from_the_left_edge_of_the_card_to_the_centre_of_the_cell", () => {
    // Cell (1, 0): pitch 34, centre at 34 + 16 = 50 inside the grid; the grid starts at (10, 20) on screen.
    expect(lineBetween(card, { x: 1, y: 0 }, 32, { x: 10, y: 20 }, 1)).toEqual({ x1: 500, y1: 120, x2: 60, y2: 36 });
  });

  it("scales_the_cell_position_with_the_zoom", () => {
    expect(lineBetween(card, { x: 1, y: 2 }, 32, { x: 10, y: 20 }, 2)).toEqual({ x1: 500, y1: 120, x2: 10 + 50 * 2, y2: 20 + (2 * 34 + 16) * 2 });
  });
});

describe("wheelPixels", () => {
  it("turns_lines_and_pages_into_pixels", () => {
    expect(wheelPixels({ deltaY: 120, deltaMode: 0 })).toBe(120);
    expect(wheelPixels({ deltaY: 3, deltaMode: 1 })).toBe(48);
    expect(wheelPixels({ deltaY: 1, deltaMode: 2 }, 600)).toBe(600);
  });
});

describe("roomLinkIndex", () => {
  it("groups_warps_by_file_in_index_order_and_finds_one_by_file_and_index", () => {
    const w = (event_file: number, index: number) => ({ event_file, index, x: 0, z: 0, dest_header_id: 0, dest_warp_id: 0 });
    const index = roomLinkIndex([], [w(5, 2), w(5, 0), w(7, 1)]);
    expect(index.byFile.get(5)?.map((l) => l.index)).toEqual([0, 2]);
    expect(index.byWarp.get("7:1")).toEqual(w(7, 1));
    expect(index.byWarp.get("7:0")).toBeUndefined();
  });
});

describe("cellBounds", () => {
  it("frames_the_cells_of_a_matrix_and_nothing_for_none", () => {
    // Width 10: cells (3, 2), (7, 1) and (5, 4).
    expect(cellBounds(10, [23, 17, 45])).toEqual({ x: 3, y: 1, width: 5, height: 4 });
    expect(cellBounds(10, [])).toBeNull();
  });
});

describe("centredPan", () => {
  const size = { width: 400, height: 300 };
  it("centres_cells_out_of_view_at_the_same_zoom", () => {
    // Cell (3, 27) at 32 px plus a 2 px gap spans x 102..134, y 918..950: its middle (118, 934) goes to (200, 150).
    expect(centredPan({ x: 3, y: 27, width: 1, height: 1 }, { x: 0, y: 0 }, 32, size, { scale: 1, x: 12, y: 12 })).toEqual({ x: 82, y: -784 });
    expect(centredPan({ x: 3, y: 27, width: 1, height: 1 }, { x: 0, y: 0 }, 32, size, { scale: 2, x: 12, y: 12 })).toEqual({ x: 200 - 236, y: 150 - 1868 });
  });

  it("counts_from_the_window_of_a_half", () => {
    // The window starts at cell (20, 25): cell (23, 27) is drawn where (3, 2) would be without it.
    expect(centredPan({ x: 23, y: 27, width: 1, height: 1 }, { x: 20, y: 25 }, 32, size, { scale: 1, x: 300, y: 300 })).toEqual({ x: 82, y: 66 });
  });

  it("leaves_cells_in_view_alone", () => {
    expect(centredPan({ x: 1, y: 1, width: 2, height: 1 }, { x: 0, y: 0 }, 32, size, { scale: 1, x: 12, y: 12 })).toBeNull();
  });
});
