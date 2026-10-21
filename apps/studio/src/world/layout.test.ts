import { describe, it, expect } from "vitest";
import { cellOrigin, buildingBox } from "./layout";

describe("layout", () => {
  it("places matrix cells 32 units apart", () => {
    expect(cellOrigin(0, 0)).toEqual({ x: 0, z: 0 });
    expect(cellOrigin(2, 1)).toEqual({ x: 64, z: 32 });
  });
  it("centers a building relative to the cell centre and uses the scale as size", () => {
    const box = buildingBox({ model_id: 1, x: -4, y: 0, z: 6, sx: 2, sy: 1, sz: 3 }, { x: 64, z: 32 });
    expect(box).toEqual({ cx: 76, cy: 0.5, cz: 54, sx: 2, sy: 1, sz: 3 });
  });
});
