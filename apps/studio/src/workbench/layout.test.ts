// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { AREAS } from "../ui/frame/shellState";
import { DECK_PANELS, deckPanels, dropOldLayouts, isSaneLayout, layoutKey, loadLayout, saveLayout } from "./layout";

describe("deck layout persistence", () => {
  beforeEach(() => localStorage.clear());

  it("keys_each_area_and_deck_on_its_own_under_v8", () => {
    expect(layoutKey("welt", "schau")).toBe("g4.layout.v8.welt.schau");
    expect(layoutKey("welt", "hand")).toBe("g4.layout.v8.welt.hand");
    expect(layoutKey("texte", "schau")).toBe("g4.layout.v8.texte.schau");
    expect(layoutKey("texte", "hand")).toBe("g4.layout.v8.texte.hand");
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

  it("does_not_read_the_layouts_of_the_versions_before_the_text_area", () => {
    localStorage.setItem("g4.layout.v6.welt.hand", JSON.stringify({ a: 1 }));
    localStorage.setItem("g4.layout.v7.welt.hand", JSON.stringify({ a: 1 }));
    expect(loadLayout(layoutKey("welt", "hand"))).toBeNull();
  });

  it("drops_the_v7_layouts_and_nothing_else", () => {
    localStorage.setItem("g4.layout.v7.welt.schau", "{}");
    localStorage.setItem("g4.layout.v7.welt.hand", "{}");
    localStorage.setItem("g4.layout.v7.texte.hand", "{}");
    localStorage.setItem("g4.layout.v8.welt.hand", "{}");
    localStorage.setItem("g4.layout.v6.welt.hand", "{}");
    localStorage.setItem("g4.shell.v1", "{}");
    dropOldLayouts(localStorage);
    const left = Array.from({ length: localStorage.length }, (_, i) => localStorage.key(i)).sort();
    expect(left).toEqual(["g4.layout.v6.welt.hand", "g4.layout.v8.welt.hand", "g4.shell.v1"]);
  });

  it("drops_nothing_from_a_storage_that_cannot_be_read", () => {
    const broken = {
      get length(): number {
        throw new Error("denied");
      },
    } as unknown as Storage;
    expect(() => dropOldLayouts(broken)).not.toThrow();
  });

  it("splits_the_panels_of_the_world_between_the_docks_and_leaves_the_explorer_and_the_text_out", () => {
    expect(DECK_PANELS.welt.schau.map((p) => p.id)).toEqual(["world", "matrix", "worldmap"]);
    expect(DECK_PANELS.welt.hand.map((p) => p.id)).toEqual(["inspector", "script", "problems", "tools", "context"]);
    const all = [...DECK_PANELS.welt.schau, ...DECK_PANELS.welt.hand];
    expect(all.some((p) => (p.id as string) === "headers")).toBe(false);
    expect(all.some((p) => (p.id as string) === "text")).toBe(false);
    // The first panel anchors a dock, the others are tabs of its group, except the tools and the context of the hand beside the inspector.
    expect(DECK_PANELS.welt.schau.slice(1).every((p) => p.position.direction === "within" && p.position.referencePanel === "world")).toBe(true);
    const tabs = DECK_PANELS.welt.hand.filter((p) => p.id !== "inspector" && p.id !== "tools" && p.id !== "context");
    expect(tabs.map((p) => p.id)).toEqual(["script", "problems"]);
    expect(tabs.every((p) => p.position.direction === "within" && p.position.referencePanel === "inspector")).toBe(true);
    const title = (id: string) => all.find((p) => p.id === id)?.title;
    expect([title("world"), title("matrix"), title("worldmap"), title("inspector"), title("script"), title("problems"), title("tools"), title("context")]).toEqual([
      "3D",
      "Raster",
      "Weltkarte",
      "Inspector",
      "Script",
      "Probleme",
      "Werkzeuge",
      "An diesem Ort",
    ]);
  });

  it("gives_the_texts_the_view_in_the_schau_and_the_messages_with_the_problems_in_the_hand", () => {
    expect(DECK_PANELS.texte.schau.map((p) => p.id)).toEqual(["textView"]);
    expect(DECK_PANELS.texte.hand.map((p) => p.id)).toEqual(["textEdit", "problems"]);
    // The problems are a tab of the messages' group.
    expect(DECK_PANELS.texte.hand[1]?.position).toEqual({ direction: "within", referencePanel: "textEdit" });
    expect([...DECK_PANELS.texte.schau, ...DECK_PANELS.texte.hand].map((p) => p.title)).toEqual(["Vorschau", "Nachrichten", "Probleme"]);
  });

  it("puts_the_tools_left_and_the_context_right_of_the_inspector_with_their_widths", () => {
    const spec = (id: string) => DECK_PANELS.welt.hand.find((p) => p.id === id)!;
    expect(spec("tools").position).toEqual({ direction: "left", referencePanel: "inspector" });
    expect(spec("tools").width).toBe(180);
    expect(spec("context").position).toEqual({ direction: "right", referencePanel: "inspector" });
    expect(spec("context").width).toBe(260);
    // The inspector is the anchor and grows with the rest.
    expect(DECK_PANELS.welt.hand[0]?.id).toBe("inspector");
    expect(spec("inspector").width).toBeUndefined();
  });

  it("looks_up_the_panels_of_an_area_and_an_area_without_a_deck_has_none", () => {
    expect(deckPanels("welt", "schau")).toBe(DECK_PANELS.welt.schau);
    expect(deckPanels("texte", "hand")).toBe(DECK_PANELS.texte.hand);
    expect(deckPanels("start", "hand")).toEqual([]);
    expect(deckPanels("technik", "schau")).toEqual([]);
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
      expect(isSaneLayout(layout([leaf("tools"), leaf("inspector", "script"), leaf("problems"), leaf("context")], ["tools", "inspector", "script", "problems", "context"]), hand)).toBe(true);
      expect(isSaneLayout(layout([leaf("textEdit", "problems")], ["textEdit", "problems"]), DECK_PANELS.texte.hand)).toBe(true);
    });

    it("checks_against_the_list_it_is_given_so_one_docks_layout_is_not_the_others", () => {
      const handLayout = layout([leaf("tools"), leaf("inspector", "script", "problems"), leaf("context")], ["tools", "inspector", "script", "problems", "context"]);
      expect(isSaneLayout(handLayout, hand)).toBe(true);
      expect(isSaneLayout(handLayout, schau)).toBe(false);
      expect(isSaneLayout(handLayout, DECK_PANELS.texte.hand)).toBe(false);
    });

    it("rejects_a_world_hand_of_v7_that_still_has_the_text_panel", () => {
      const v7 = layout([leaf("tools"), leaf("inspector", "text", "script", "problems"), leaf("context")], ["tools", "inspector", "text", "script", "problems", "context"]);
      expect(isSaneLayout(v7, hand)).toBe(false);
    });

    it("rejects_a_layout_with_a_missing_or_an_unknown_panel_or_no_panels_at_all", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix")], ["world", "matrix"]), schau)).toBe(false);
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap", "status")], ["world", "matrix", "worldmap", "status"]), schau)).toBe(false);
      expect(isSaneLayout({}, schau)).toBe(false);
      expect(isSaneLayout({ panels: null }, schau)).toBe(false);
    });
  });
});
