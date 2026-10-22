import { describe, it, expect } from "vitest";
import { cellOrigin } from "./layout";

describe("layout", () => {
  it("places matrix cells 32 units apart", () => {
    expect(cellOrigin(0, 0)).toEqual({ x: 0, z: 0 });
    expect(cellOrigin(2, 1)).toEqual({ x: 64, z: 32 });
  });
});
