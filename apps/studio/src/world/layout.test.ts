import { describe, it, expect } from "vitest";
import { ALTITUDE_STEP, cellOrigin, cellY, cellYAtTile } from "./layout";

const flat = { width: 2 };

describe("layout", () => {
  it("places matrix cells 32 units apart", () => {
    expect(cellOrigin(0, 0)).toEqual({ x: 0, z: 0 });
    expect(cellOrigin(2, 1)).toEqual({ x: 64, z: 32 });
  });

  it("raises a cell by half a tile per altitude step", () => {
    expect(ALTITUDE_STEP).toBe(0.5);
    expect(cellY({ ...flat, altitudes: [0, 2] }, 1, 0)).toBe(1);
    expect(cellY({ ...flat, altitudes: [3, 0] }, 0, 0)).toBe(1.5);
  });

  it("keeps everything at 0 without an altitude section", () => {
    expect(cellY({ ...flat, altitudes: null }, 1, 0)).toBe(0);
    expect(cellY(flat, 1, 0)).toBe(0);
    expect(cellY({ ...flat, altitudes: [1] }, 1, 0)).toBe(0);
  });

  it("finds the cell of an absolute tile", () => {
    const m = { width: 2, altitudes: [0, 2, 4, 6] };
    expect(cellYAtTile(m, 40, 3)).toBe(1);
    expect(cellYAtTile(m, 31, 3)).toBe(0);
    expect(cellYAtTile(m, 3, 32)).toBe(2);
    expect(cellYAtTile(m, 63, 63)).toBe(3);
  });

  it("is 0 for a tile outside the altitude grid", () => {
    expect(cellYAtTile({ width: 2, altitudes: [0, 2] }, 70, 3)).toBe(0);
    expect(cellYAtTile({ width: 2, altitudes: [0, 2] }, -1, 3)).toBe(0);
    expect(cellYAtTile({ width: 2, altitudes: [0, 2] }, 3, 40)).toBe(0);
  });
});
