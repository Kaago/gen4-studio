// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PANELS, LAYOUT_KEY, isSaneLayout, loadLayout, saveLayout } from "./layout";

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

  describe("isSaneLayout", () => {
    const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
    const layout = (leaves: object[], panelIds: string[] = ["headers", "world", "inspector", "status"]) => ({
      grid: { root: { type: "branch", data: leaves }, width: 1000, height: 800, orientation: "HORIZONTAL" },
      panels: Object.fromEntries(panelIds.map((id) => [id, { id }])),
    });
    const defaults = () => layout([leaf("headers"), { type: "branch", data: [leaf("world"), leaf("status")] }, leaf("inspector")]);

    it("accepts the default arrangement", () => {
      expect(isSaneLayout(defaults())).toBe(true);
    });

    it("accepts moved panels as long as status stays alone", () => {
      expect(isSaneLayout(layout([leaf("headers", "inspector"), leaf("world"), leaf("status")]))).toBe(true);
    });

    it("rejects a layout where status shares a group", () => {
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "status"), leaf("inspector")]))).toBe(false);
      expect(isSaneLayout(layout([leaf("status", "inspector"), leaf("headers"), leaf("world")]))).toBe(false);
    });

    it("rejects a layout with a missing panel or no groups", () => {
      expect(isSaneLayout(layout([leaf("headers"), leaf("world"), leaf("status")], ["headers", "world", "status"]))).toBe(false);
      expect(isSaneLayout(layout([leaf("headers"), leaf("world"), leaf("inspector")]))).toBe(false);
      expect(isSaneLayout({})).toBe(false);
      expect(isSaneLayout({ panels: null })).toBe(false);
    });
  });
});
