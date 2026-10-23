import { describe, expect, it } from "vitest";
import { cellLabel, parseCellField, rawCell, rawRow, resizePreview, sameLayout } from "./matrix";

describe("matrix helpers", () => {
  it("labels cells and counts lost cells on shrink", () => {
    expect(cellLabel(null, null)).toBe("—");
    expect(cellLabel(570, 415)).toBe("M0570 H0415");
    expect(resizePreview(1, 1, { index: 0, name: "", width: 2, height: 2, maps: [1, 2, null, 4] })).toEqual({ lostCells: 2 });
  });

  it("labels a map without a header and a header on an empty cell", () => {
    expect(cellLabel(5, null)).toBe("M0005");
    expect(cellLabel(null, 7)).toBe("—");
  });

  it("loses no cells when growing, and counts only the filled ones that go", () => {
    const m = { index: 0, name: "", width: 2, height: 2, maps: [1, 2, null, 4] };
    expect(resizePreview(3, 3, m)).toEqual({ lostCells: 0 });
    expect(resizePreview(2, 1, m)).toEqual({ lostCells: 1 });
    expect(resizePreview(2, 1, { ...m, maps: [1, 2, null, null] })).toEqual({ lostCells: 0 });
  });

  it("counts header and altitude values that would go with an empty-map cell", () => {
    const m = { index: 0, name: "", width: 2, height: 1, maps: [1, null], headers: [0, 9], altitudes: [0, 0] };
    expect(resizePreview(1, 1, m)).toEqual({ lostCells: 1 });
    expect(resizePreview(1, 1, { ...m, headers: [0, 0], altitudes: [0, 2] })).toEqual({ lostCells: 1 });
  });

  it("compares layouts by grid, maps and altitudes", () => {
    const m = { index: 1, name: "a", width: 2, height: 1, maps: [1, null] };
    expect(sameLayout(m, { ...m, name: "b", headers: [1, 2] })).toBe(true);
    expect(sameLayout(m, { ...m, altitudes: [0, 3] })).toBe(false);
    expect(sameLayout({ ...m, altitudes: [0, 3] }, { ...m, altitudes: [0, 4] })).toBe(false);
    expect(sameLayout({ ...m, altitudes: [0, 3] }, { ...m, name: "b", altitudes: [0, 3] })).toBe(true);
    // No altitude section and all-zero altitudes look the same in the scene.
    expect(sameLayout(m, { ...m, altitudes: [0, 0] })).toBe(true);
    expect(sameLayout({ ...m, altitudes: null }, m)).toBe(true);
    expect(sameLayout(m, { ...m, maps: [1, 2] })).toBe(false);
    expect(sameLayout(m, { ...m, width: 1, maps: [1] })).toBe(false);
    expect(sameLayout(m, { ...m, index: 2 })).toBe(false);
  });

  describe("parseCellField", () => {
    it("clamps a number into 0..max and rounds", () => {
      expect(parseCellField("12", 100, false)).toBe(12);
      expect(parseCellField("500", 100, false)).toBe(100);
      expect(parseCellField("-3", 100, false)).toBe(0);
      expect(parseCellField("2.6", 100, false)).toBe(3);
    });
    it("takes a blank as empty only where that is allowed, else rejects it", () => {
      expect(parseCellField("  ", 100, true)).toBe("empty");
      expect(parseCellField("", 100, false)).toBeNull();
      expect(parseCellField("abc", 100, true)).toBeNull();
    });
  });

  describe("raw values", () => {
    it("writes_a_cell_as_map_header_and_altitude_in_the_way_the_cells_do", () => {
      expect(rawCell(104, 411, 0)).toBe("0104 0411 0");
      expect(rawCell(521, null, 2)).toBe("0521 2");
      expect(rawCell(521, 415, null)).toBe("0521 0415");
    });

    it("reads_one_row_as_u16_hex_in_capitals_and_u8_altitudes", () => {
      const m = { index: 0, name: "", width: 3, height: 2, maps: [10, null, 600, 1, 2, 3], headers: [411, 0, 65535, 1, 2, 3], altitudes: [0, 255, 7, 1, 2, 3] };
      expect(rawRow(m, 0)).toEqual({ maps: ["000A", "FFFF", "0258"], headers: ["019B", "0000", "FFFF"], altitudes: [0, 255, 7] });
    });

    it("leaves_out_the_sections_the_matrix_does_not_have", () => {
      const m = { index: 0, name: "", width: 2, height: 1, maps: [1, 2], headers: null, altitudes: null };
      expect(rawRow(m, 0)).toEqual({ maps: ["0001", "0002"], headers: null, altitudes: null });
    });
  });
});
