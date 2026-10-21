import { describe, it, expect } from "vitest";
import { isClick, matrixCellAt, pickCell } from "./paint";

describe("pickCell", () => {
  it("maps a world hit to the cell's tile coordinates", () => {
    expect(pickCell({ x: 64.5, z: 33.2 }, { x: 64, z: 32 })).toEqual({ x: 0, y: 1 });
    expect(pickCell({ x: 95.9, z: 63.9 }, { x: 64, z: 32 })).toEqual({ x: 31, y: 31 });
  });
  it("returns null outside the cell", () => {
    expect(pickCell({ x: 96.1, z: 40 }, { x: 64, z: 32 })).toBeNull();
    expect(pickCell({ x: 63.9, z: 40 }, { x: 64, z: 32 })).toBeNull();
    expect(pickCell({ x: 70, z: 64.1 }, { x: 64, z: 32 })).toBeNull();
    expect(pickCell({ x: 70, z: 31.9 }, { x: 64, z: 32 })).toBeNull();
  });
  it("treats the far edge as outside", () => {
    expect(pickCell({ x: 96, z: 40 }, { x: 64, z: 32 })).toBeNull();
  });
});

describe("matrixCellAt", () => {
  it("finds the matrix cell or null outside", () => {
    expect(matrixCellAt({ x: 40, z: 5 }, 2, 1)).toEqual({ col: 1, row: 0 });
    expect(matrixCellAt({ x: 64, z: 5 }, 2, 1)).toBeNull();
    expect(matrixCellAt({ x: -0.1, z: 5 }, 2, 1)).toBeNull();
    expect(matrixCellAt({ x: 5, z: 32 }, 2, 1)).toBeNull();
  });
});

describe("isClick", () => {
  it("accepts under 4 px and rejects drags", () => {
    expect(isClick({ x: 10, y: 10 }, { x: 12, y: 11 })).toBe(true);
    expect(isClick({ x: 10, y: 10 }, { x: 14, y: 10 })).toBe(false);
  });
});
