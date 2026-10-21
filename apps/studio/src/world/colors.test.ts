import { describe, it, expect } from "vitest";
import { permissionColor } from "./colors";

describe("permissionColor", () => {
  it("marks blocked cells red regardless of kind", () => {
    expect(permissionColor(0, 0x80)).toBe("#d64545");
    expect(permissionColor(5, 0x80)).toBe("#d64545");
  });
  it("colors walkable by kind family", () => {
    expect(permissionColor(0, 0)).toBe("#6b7280");
    expect(permissionColor(0x02, 0)).toBe("#4caf50"); // Gras
    expect(permissionColor(0x10, 0)).toBe("#3b82f6"); // Wasser
  });
});
