import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../api";
import { extendDefaults, extendSpec } from "./extend";

const matrix = (over: Partial<MatrixDto> = {}): MatrixDto => ({ index: 7, name: "m", width: 2, height: 1, maps: [0, 1], ...over });

describe("extendDefaults", () => {
  it("takes_the_nearest_filled_cell_to_the_left_and_grows_the_width", () => {
    expect(extendDefaults(matrix(), 2, 0)).toEqual({ neighbour: { x: 1, y: 0, map: 1, header: null }, width: 3, height: 1 });
  });

  it("falls_back_to_the_cell_above_and_grows_the_height", () => {
    expect(extendDefaults(matrix(), 0, 1)).toEqual({ neighbour: { x: 0, y: 0, map: 0, header: null }, width: 2, height: 2 });
  });

  it("skips_empty_cells_on_the_way_left", () => {
    const m = matrix({ width: 3, maps: [5, null, null] });
    expect(extendDefaults(m, 2, 0).neighbour).toEqual({ x: 0, y: 0, map: 5, header: null });
  });

  it("has_no_neighbour_on_an_empty_matrix_and_keeps_the_size_inside_the_grid", () => {
    expect(extendDefaults(matrix({ width: 1, maps: [null] }), 0, 0)).toEqual({ neighbour: null, width: 1, height: 1 });
  });

  it("reports_the_header_of_the_neighbour", () => {
    expect(extendDefaults(matrix({ headers: [415, 416] }), 2, 0).neighbour?.header).toBe(416);
  });

  it("proposes_the_preferred_cell_first_and_falls_back_when_it_is_empty", () => {
    // Left of (1, 1) lies landscape (0, 1); the area's cell above is preferred.
    const m = matrix({ width: 2, height: 2, maps: [5, 6, 7, null], headers: [0, 416, 0, 0] });
    expect(extendDefaults(m, 1, 1, { x: 1, y: 0 }).neighbour).toEqual({ x: 1, y: 0, map: 6, header: 416 });
    expect(extendDefaults(m, 1, 1, { x: 1, y: 1 }).neighbour).toEqual({ x: 0, y: 1, map: 7, header: 0 });
  });
});

describe("extendSpec", () => {
  const source = { map: 3, clearBuildings: true };

  it("sets_header_cell_only_when_the_matrix_has_a_header_section", () => {
    expect(extendSpec(matrix(), 2, 0, source, 415)).toEqual({ matrix: 7, x: 2, y: 0, source_map: 3, clear_buildings: true, header_cell: null });
    expect(extendSpec(matrix({ headers: [0, 0] }), 2, 0, source, 415).header_cell).toBe(415);
  });
});
