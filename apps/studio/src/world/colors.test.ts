import { describe, it, expect } from "vitest";
import { permissionColor } from "./colors";

describe("permissionColor", () => {
  it("marks blocked cells red regardless of kind and class", () => {
    expect(permissionColor(0, 0x80, null)).toBe("#d64545");
    expect(permissionColor(5, 0x80, "grass")).toBe("#d64545");
  });
  it("colors walkable cells by behaviour class", () => {
    expect(permissionColor(0, 0, null)).toBe("#6b7280");
    expect(permissionColor(0x02, 0, "grass")).toBe("#4caf50");
    expect(permissionColor(0x10, 0, "water")).toBe("#3b82f6");
    expect(permissionColor(0x5e, 0, "warp")).toBe("#f59e0b");
  });
  it("leaves classes without a colour and unknown kinds neutral", () => {
    expect(permissionColor(0x08, 0, "other")).toBe("#6b7280");
    expect(permissionColor(0x33, 0, null)).toBe("#6b7280");
  });
});
