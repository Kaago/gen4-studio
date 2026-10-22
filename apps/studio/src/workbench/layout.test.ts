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

  it("default layout has the six panels once, the raster and the world map as tabs of the world", () => {
    expect(DEFAULT_PANELS.map((p) => p.id).sort()).toEqual(["headers", "inspector", "matrix", "status", "world", "worldmap"]);
    expect(DEFAULT_PANELS.find((p) => p.id === "matrix")?.position).toEqual({ direction: "within", referencePanel: "world" });
    expect(DEFAULT_PANELS.find((p) => p.id === "worldmap")?.position).toEqual({ direction: "within", referencePanel: "world" });
  });

  it("names the panels with the glossary's words: the explorer is the left panel, 300 px wide", () => {
    const title = (id: string) => DEFAULT_PANELS.find((p) => p.id === id)?.title;
    expect([title("world"), title("matrix"), title("worldmap"), title("headers")]).toEqual(["3D", "Raster", "Weltkarte", "Welt"]);
    expect(DEFAULT_PANELS.find((p) => p.id === "headers")).toMatchObject({ position: { direction: "left", referencePanel: "world" }, size: 300 });
  });

  it("does not read layouts of the five-panel version", () => {
    expect(LAYOUT_KEY).toBe("g4.layout.v3");
  });

  describe("isSaneLayout", () => {
    const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
    const layout = (leaves: object[], panelIds: string[] = ["headers", "world", "matrix", "worldmap", "inspector", "status"]) => ({
      grid: { root: { type: "branch", data: leaves }, width: 1000, height: 800, orientation: "HORIZONTAL" },
      panels: Object.fromEntries(panelIds.map((id) => [id, { id }])),
    });
    const defaults = () => layout([leaf("headers"), { type: "branch", data: [leaf("world", "matrix", "worldmap"), leaf("status")] }, leaf("inspector")]);

    it("accepts the default arrangement", () => {
      expect(isSaneLayout(defaults())).toBe(true);
    });

    it("accepts moved panels as long as status stays alone", () => {
      expect(isSaneLayout(layout([leaf("headers", "inspector"), leaf("world", "matrix"), leaf("status")]))).toBe(true);
    });

    it("rejects a layout where status shares a group", () => {
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "matrix", "status"), leaf("inspector")]))).toBe(false);
      expect(isSaneLayout(layout([leaf("status", "inspector"), leaf("headers"), leaf("world", "matrix")]))).toBe(false);
    });

    it("rejects a layout of the five-panel version, which has no world map", () => {
      const v2 = layout([leaf("headers"), leaf("world", "matrix"), leaf("status"), leaf("inspector")], ["headers", "world", "matrix", "inspector", "status"]);
      expect(isSaneLayout(v2)).toBe(false);
    });

    it("rejects a layout with a missing panel or no groups", () => {
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "matrix"), leaf("status")], ["headers", "world", "matrix", "worldmap", "status"]))).toBe(false);
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "inspector"), leaf("status")], ["headers", "world", "inspector", "worldmap", "status"]))).toBe(false);
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "matrix"), leaf("inspector")]))).toBe(false);
      expect(isSaneLayout({})).toBe(false);
      expect(isSaneLayout({ panels: null })).toBe(false);
    });
  });
});
