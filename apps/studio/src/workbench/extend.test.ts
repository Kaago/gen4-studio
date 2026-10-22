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
});

describe("extendSpec", () => {
  const source = { map: 3, clearBuildings: true };

  it("sets_header_cell_only_when_the_matrix_has_a_header_section", () => {
    expect(extendSpec(matrix(), 2, 0, source, 415)).toEqual({ matrix: 7, x: 2, y: 0, source_map: 3, clear_buildings: true, header_cell: null });
    expect(extendSpec(matrix({ headers: [0, 0] }), 2, 0, source, 415).header_cell).toBe(415);
  });
});
