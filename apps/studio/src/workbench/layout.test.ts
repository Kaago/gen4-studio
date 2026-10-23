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

  it("default layout has the eight panels once, the raster, the world map, the text and the script as tabs of the world", () => {
    expect(DEFAULT_PANELS.map((p) => p.id).sort()).toEqual(["headers", "inspector", "matrix", "problems", "script", "text", "world", "worldmap"]);
    expect(DEFAULT_PANELS.find((p) => p.id === "script")).toMatchObject({ title: "Script", position: { direction: "within", referencePanel: "world" } });
    // The problems list is a group of its own below the world's tabs.
    expect(DEFAULT_PANELS.find((p) => p.id === "problems")).toMatchObject({ title: "Probleme", position: { direction: "below", referencePanel: "world" } });
    expect(DEFAULT_PANELS.find((p) => p.id === "text")).toMatchObject({ title: "Text", position: { direction: "within", referencePanel: "world" } });
    expect(DEFAULT_PANELS.find((p) => p.id === "matrix")?.position).toEqual({ direction: "within", referencePanel: "world" });
    expect(DEFAULT_PANELS.find((p) => p.id === "worldmap")?.position).toEqual({ direction: "within", referencePanel: "world" });
  });

  it("names the panels with the glossary's words: the explorer is the left panel, 300 px wide", () => {
    const title = (id: string) => DEFAULT_PANELS.find((p) => p.id === id)?.title;
    expect([title("world"), title("matrix"), title("worldmap"), title("headers")]).toEqual(["3D", "Raster", "Weltkarte", "Welt"]);
    expect(DEFAULT_PANELS.find((p) => p.id === "headers")).toMatchObject({ position: { direction: "left", referencePanel: "world" }, size: 300 });
  });

  it("does not read layouts of the seven-panel version", () => {
    expect(LAYOUT_KEY).toBe("g4.layout.v5");
    // A layout stored under an old key is simply not there for this version.
    localStorage.setItem("g4.layout.v3", JSON.stringify({ a: 1 }));
    localStorage.setItem("g4.layout.v4", JSON.stringify({ a: 1 }));
    expect(loadLayout()).toBeNull();
  });

  describe("isSaneLayout", () => {
    const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
    const layout = (leaves: object[], panelIds: string[] = ["headers", "world", "matrix", "worldmap", "text", "script", "problems", "inspector"]) => ({
      grid: { root: { type: "branch", data: leaves }, width: 1000, height: 800, orientation: "HORIZONTAL" },
      panels: Object.fromEntries(panelIds.map((id) => [id, { id }])),
    });
    const defaults = () => layout([leaf("headers"), { type: "branch", data: [leaf("world", "matrix", "worldmap", "text", "script"), leaf("problems")] }, leaf("inspector")]);

    it("accepts the default arrangement", () => {
      expect(isSaneLayout(defaults())).toBe(true);
    });

    it("accepts moved panels", () => {
      expect(isSaneLayout(layout([leaf("headers", "inspector"), leaf("world", "matrix", "worldmap", "text", "script", "problems")]))).toBe(true);
    });

    it("rejects a layout that still holds the removed status panel (the key stays v5, so such layouts exist)", () => {
      const ids = ["headers", "world", "matrix", "worldmap", "text", "script", "problems", "inspector", "status"];
      const saved = layout([leaf("headers"), { type: "branch", data: [leaf("world", "matrix", "worldmap", "text", "script"), leaf("problems"), leaf("status")] }, leaf("inspector")], ids);
      expect(isSaneLayout(saved)).toBe(false);
    });

    it("rejects a layout of the five-panel version, which has no world map", () => {
      const v2 = layout([leaf("headers"), leaf("world", "matrix"), leaf("inspector")], ["headers", "world", "matrix", "inspector"]);
      expect(isSaneLayout(v2)).toBe(false);
    });

    it("rejects a layout of the six-panel version, which has no text panel", () => {
      const v3 = layout([leaf("headers"), leaf("world", "matrix", "worldmap"), leaf("inspector")], ["headers", "world", "matrix", "worldmap", "inspector"]);
      expect(isSaneLayout(v3)).toBe(false);
    });

    it("rejects a layout of the seven-panel version, which has no script and problems panels", () => {
      const v4 = layout([leaf("headers"), leaf("world", "matrix", "worldmap", "text"), leaf("inspector")], ["headers", "world", "matrix", "worldmap", "text", "inspector"]);
      expect(isSaneLayout(v4)).toBe(false);
    });

    it("rejects a layout with a missing panel or no groups", () => {
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "matrix")], ["headers", "world", "matrix", "worldmap", "text"]))).toBe(false);
      expect(isSaneLayout(layout([leaf("headers"), leaf("world", "inspector")], ["headers", "world", "inspector", "worldmap", "text"]))).toBe(false);
      expect(isSaneLayout({})).toBe(false);
      expect(isSaneLayout({ panels: null })).toBe(false);
    });
  });
});
