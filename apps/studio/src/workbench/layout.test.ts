// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { AREAS } from "../ui/frame/shellState";
import { DECK_PANELS, deckPanels, isSaneLayout, layoutKey, loadLayout, saveLayout } from "./layout";

describe("deck layout persistence", () => {
  beforeEach(() => localStorage.clear());

  it("keys_each_area_and_deck_on_its_own_under_v6", () => {
    expect(layoutKey("welt", "schau")).toBe("g4.layout.v6.welt.schau");
    expect(layoutKey("welt", "hand")).toBe("g4.layout.v6.welt.hand");
    expect(layoutKey("texte", "hand")).toBe("g4.layout.v6.texte.hand");
    const keys = AREAS.flatMap((a) => [layoutKey(a, "schau"), layoutKey(a, "hand")]);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("round_trips_a_layout_through_local_storage_and_tolerates_garbage", () => {
    const key = layoutKey("welt", "schau");
    saveLayout(key, { a: 1 });
    expect(loadLayout(key)).toEqual({ a: 1 });
    localStorage.setItem(key, "{not json");
    expect(loadLayout(key)).toBeNull();
  });

  it("returns_null_when_nothing_is_stored_or_the_value_is_not_an_object", () => {
    const key = layoutKey("welt", "hand");
    expect(loadLayout(key)).toBeNull();
    localStorage.setItem(key, "42");
    expect(loadLayout(key)).toBeNull();
    localStorage.setItem(key, "null");
    expect(loadLayout(key)).toBeNull();
  });

  it("does_not_read_the_layout_of_the_single_dock_version", () => {
    localStorage.setItem("g4.layout.v5", JSON.stringify({ a: 1 }));
    expect(loadLayout(layoutKey("welt", "schau"))).toBeNull();
    expect(loadLayout(layoutKey("welt", "hand"))).toBeNull();
  });

  it("splits_the_panels_between_the_docks_and_leaves_the_explorer_out", () => {
    expect(DECK_PANELS.welt.schau.map((p) => p.id)).toEqual(["world", "matrix", "worldmap"]);
    expect(DECK_PANELS.welt.hand.map((p) => p.id)).toEqual(["inspector", "text", "script", "problems"]);
    const all = [...DECK_PANELS.welt.schau, ...DECK_PANELS.welt.hand];
    expect(all.some((p) => (p.id as string) === "headers")).toBe(false);
    // The first panel anchors a dock, the others are tabs of its group.
    expect(DECK_PANELS.welt.schau.slice(1).every((p) => p.position.direction === "within" && p.position.referencePanel === "world")).toBe(true);
    expect(DECK_PANELS.welt.hand.slice(1).every((p) => p.position.direction === "within" && p.position.referencePanel === "inspector")).toBe(true);
    const title = (id: string) => all.find((p) => p.id === id)?.title;
    expect([title("world"), title("matrix"), title("worldmap"), title("inspector"), title("text"), title("script"), title("problems")]).toEqual(["3D", "Raster", "Weltkarte", "Inspector", "Text", "Script", "Probleme"]);
  });

  it("an_area_without_a_deck_has_no_panels", () => {
    expect(deckPanels("welt", "schau")).toBe(DECK_PANELS.welt.schau);
    expect(deckPanels("start", "hand")).toEqual([]);
  });

  describe("isSaneLayout", () => {
    const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
    const layout = (leaves: object[], panelIds: string[]) => ({
      grid: { root: { type: "branch", data: leaves }, width: 1000, height: 800, orientation: "HORIZONTAL" },
      panels: Object.fromEntries(panelIds.map((id) => [id, { id }])),
    });
    const schau = DECK_PANELS.welt.schau;
    const hand = DECK_PANELS.welt.hand;

    it("accepts_a_layout_with_exactly_the_panels_of_the_dock", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap")], ["world", "matrix", "worldmap"]), schau)).toBe(true);
      expect(isSaneLayout(layout([leaf("inspector", "text"), leaf("script", "problems")], ["inspector", "text", "script", "problems"]), hand)).toBe(true);
    });

    it("checks_against_the_list_it_is_given_so_one_docks_layout_is_not_the_others", () => {
      const handLayout = layout([leaf("inspector", "text", "script", "problems")], ["inspector", "text", "script", "problems"]);
      expect(isSaneLayout(handLayout, hand)).toBe(true);
      expect(isSaneLayout(handLayout, schau)).toBe(false);
    });

    it("rejects_a_layout_with_a_missing_or_an_unknown_panel_or_no_panels_at_all", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix")], ["world", "matrix"]), schau)).toBe(false);
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap", "status")], ["world", "matrix", "worldmap", "status"]), schau)).toBe(false);
      expect(isSaneLayout({}, schau)).toBe(false);
      expect(isSaneLayout({ panels: null }, schau)).toBe(false);
    });
  });
});
