// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PANELS, LAYOUT_KEY, hasAllPanels, loadLayout, saveLayout } from "./layout";

describe("workbench layout persistence", () => {
  beforeEach(() => localStorage.clear());

  it("round-trips a layout through localStorage and tolerates garbage", () => {
    saveLayout({ a: 1 });
    expect(loadLayout()).toEqual({ a: 1 });
    localStorage.setItem(LAYOUT_KEY, "{not json");
    expect(loadLayout()).toBeNull();
  });

  it("returns null when nothing is stored or the value is not an object", () => {
    expect(loadLayout()).toBeNull();
    localStorage.setItem(LAYOUT_KEY, "42");
    expect(loadLayout()).toBeNull();
    localStorage.setItem(LAYOUT_KEY, "null");
    expect(loadLayout()).toBeNull();
  });

  it("default layout has the four panels once", () => {
    expect(DEFAULT_PANELS.map((p) => p.id).sort()).toEqual(["headers", "inspector", "status", "world"]);
  });

  it("accepts only stored layouts that contain all four panels", () => {
    const panels = Object.fromEntries(DEFAULT_PANELS.map((p) => [p.id, {}]));
    expect(hasAllPanels({ panels })).toBe(true);
    const { inspector: _dropped, ...three } = panels;
    expect(hasAllPanels({ panels: three })).toBe(false);
    expect(hasAllPanels({})).toBe(false);
    expect(hasAllPanels({ panels: null })).toBe(false);
  });
});
