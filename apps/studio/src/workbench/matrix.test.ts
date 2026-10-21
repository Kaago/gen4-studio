import { describe, expect, it } from "vitest";
import { cellLabel, parseCellField, resizePreview, sameLayout } from "./matrix";

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

  it("compares layouts by grid and maps only", () => {
    const m = { index: 1, name: "a", width: 2, height: 1, maps: [1, null] };
    expect(sameLayout(m, { ...m, name: "b", headers: [1, 2], altitudes: [0, 3] })).toBe(true);
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
});
