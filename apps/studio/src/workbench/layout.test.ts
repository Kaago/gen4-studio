// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { AREAS } from "../ui/frame/shellState";
import { AREA_PANELS, DECK_AREAS, DECK_PANELS, DEFAULT_ARRANGEMENT, arrangementSteps, deckLayoutKey, deckPanels, dropOldLayouts, isDeckArea, isSaneLayout, isSchauPanel, layoutKey, loadLayout, saveLayout } from "./layout";
import type { ArrangementStep } from "./layout";

describe("deck layout persistence", () => {
  beforeEach(() => localStorage.clear());

  it("keys_each_area_under_v11", () => {
    expect(layoutKey("welt")).toBe("g4.layout.v11.welt");
    expect(layoutKey("texte")).toBe("g4.layout.v11.texte");
    expect(layoutKey("pokemon")).toBe("g4.layout.v11.pokemon");
    expect(layoutKey("gegenstaende")).toBe("g4.layout.v11.gegenstaende");
    const keys = AREAS.map((a) => layoutKey(a));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("keys_the_two_docks_of_the_deck_until_the_deck_is_rebuilt_under_v11_and_the_world_hand_is_one_group", () => {
    expect(deckLayoutKey("welt", "schau")).toBe("g4.layout.v11.welt.schau");
    expect(deckLayoutKey("welt", "hand")).toBe("g4.layout.v11.welt.hand");
    expect(deckLayoutKey("texte", "schau")).toBe("g4.layout.v11.texte.schau");
    expect(deckLayoutKey("texte", "hand")).toBe("g4.layout.v11.texte.hand");
    const keys = AREAS.flatMap((a) => [deckLayoutKey(a, "schau"), deckLayoutKey(a, "hand")]);
    expect(new Set(keys).size).toBe(keys.length);
    // „An diesem Ort“ is gone (the list „Welt“ shows the scene): the inspector anchors one group, script and problems are its tabs.
    expect(DECK_PANELS.welt.hand.map((p) => p.id)).toEqual(["inspector", "script", "problems"]);
    expect(DECK_PANELS.welt.hand.slice(1).every((p) => p.position.direction === "within" && p.position.referencePanel === "inspector")).toBe(true);
    expect(DECK_PANELS.welt.hand.some((p) => p.title === "An diesem Ort")).toBe(false);
  });

  it("round_trips_a_layout_through_local_storage_and_tolerates_garbage", () => {
    const key = deckLayoutKey("welt", "schau");
    saveLayout(key, { a: 1 });
    expect(loadLayout(key)).toEqual({ a: 1 });
    localStorage.setItem(key, "{not json");
    expect(loadLayout(key)).toBeNull();
  });

  it("returns_null_when_nothing_is_stored_or_the_value_is_not_an_object", () => {
    const key = layoutKey("welt");
    expect(loadLayout(key)).toBeNull();
    localStorage.setItem(key, "42");
    expect(loadLayout(key)).toBeNull();
    localStorage.setItem(key, "null");
    expect(loadLayout(key)).toBeNull();
  });

  it("does_not_read_the_layout_of_the_single_dock_version", () => {
    localStorage.setItem("g4.layout.v5", JSON.stringify({ a: 1 }));
    expect(loadLayout(layoutKey("welt"))).toBeNull();
  });

  it("does_not_read_the_layouts_of_the_versions_before_the_text_area", () => {
    localStorage.setItem("g4.layout.v6.welt.hand", JSON.stringify({ a: 1 }));
    localStorage.setItem("g4.layout.v7.welt.hand", JSON.stringify({ a: 1 }));
    expect(loadLayout(layoutKey("welt"))).toBeNull();
  });

  it("drops_every_layout_before_v11", () => {
    localStorage.setItem("g4.layout.v7.welt.hand", "{}");
    localStorage.setItem("g4.layout.v8.welt.schau", "{}");
    localStorage.setItem("g4.layout.v9.welt.schau", "{}");
    localStorage.setItem("g4.layout.v10.welt.schau", "{}");
    localStorage.setItem("g4.layout.v10.texte.hand", "{}");
    localStorage.setItem("g4.layout.v11.welt", "{}");
    localStorage.setItem("g4.layout.v12.welt", "{}");
    localStorage.setItem("g4.shell.v1", "{}");
    dropOldLayouts(localStorage);
    const left = Array.from({ length: localStorage.length }, (_, i) => localStorage.key(i)).sort();
    expect(left).toEqual(["g4.layout.v11.welt", "g4.layout.v12.welt", "g4.shell.v1"]);
  });

  it("drops_nothing_from_a_storage_that_cannot_be_read", () => {
    const broken = {
      get length(): number {
        throw new Error("denied");
      },
    } as unknown as Storage;
    expect(() => dropOldLayouts(broken)).not.toThrow();
  });

  it("lists_the_panels_of_each_area_once", () => {
    expect(AREA_PANELS.welt.map((p) => p.id)).toEqual(["world", "matrix", "worldmap", "inspector", "script", "problems"]);
    expect(AREA_PANELS.texte.map((p) => p.id)).toEqual(["textView", "textEdit", "problems"]);
    expect(AREA_PANELS.pokemon.map((p) => p.id)).toEqual(["pokemonView", "pokemonEdit"]);
    expect(AREA_PANELS.gegenstaende.map((p) => p.id)).toEqual(["itemView", "itemEdit"]);
    for (const area of DECK_AREAS) {
      const ids = AREA_PANELS[area].map((p) => p.id as string);
      expect(new Set(ids).size).toBe(ids.length);
      expect(ids.includes("headers")).toBe(false);
      // Every arrangement of the area places exactly the area's panels, each once.
      for (const arrangement of ["werkbank", "doppeldeck"] as const) {
        expect(arrangementSteps(area, arrangement).map((st) => st.id).sort()).toEqual([...ids].sort());
      }
    }
    expect(AREA_PANELS.welt.map((p) => p.title)).toEqual(["3D", "Raster", "Weltkarte", "Inspector", "Script", "Probleme"]);
    expect(AREA_PANELS.texte.map((p) => p.title)).toEqual(["Vorschau", "Nachrichten", "Probleme"]);
    // The view panels are marked, nothing else is.
    expect(DECK_AREAS.flatMap((a) => AREA_PANELS[a].filter((p) => p.schau).map((p) => p.id))).toEqual(["world", "matrix", "worldmap", "textView", "pokemonView", "itemView"]);
    expect(["world", "matrix", "worldmap", "textView", "pokemonView", "itemView"].every(isSchauPanel)).toBe(true);
    expect(["inspector", "script", "problems", "textEdit", "pokemonEdit", "itemEdit", "nothing"].some(isSchauPanel)).toBe(false);
  });

  describe("arrangementSteps", () => {
    const step = (steps: ArrangementStep[], id: string) => steps.find((st) => st.id === id);

    it("builds_the_werkbank_with_the_inspector_right_at_340_and_script_below_at_240", () => {
      const steps = arrangementSteps("welt", "werkbank");
      expect(steps.map((st) => st.id)).toEqual(["world", "matrix", "worldmap", "inspector", "script", "problems"]);
      expect(steps[0]).toEqual({ id: "world" });
      expect(step(steps, "matrix")?.position).toEqual({ referencePanel: "world", direction: "within" });
      expect(step(steps, "worldmap")?.position).toEqual({ referencePanel: "world", direction: "within" });
      expect(step(steps, "inspector")).toEqual({
        id: "inspector",
        position: { referencePanel: "world", direction: "right" },
        width: 340,
        minimumWidth: 280,
        maximumWidth: 480,
      });
      expect(step(steps, "script")).toEqual({ id: "script", position: { referencePanel: "world", direction: "below" }, height: 240 });
      expect(step(steps, "problems")?.position).toEqual({ referencePanel: "script", direction: "within" });
    });

    it("builds_the_doppeldeck_with_the_edit_group_below_at_286", () => {
      const world = arrangementSteps("welt", "doppeldeck");
      expect(world.map((st) => st.id)).toEqual(["world", "matrix", "worldmap", "inspector", "script", "problems"]);
      expect(step(world, "inspector")).toEqual({ id: "inspector", position: { referencePanel: "world", direction: "below" }, height: 286 });
      expect(step(world, "script")?.position).toEqual({ referencePanel: "inspector", direction: "within" });
      expect(step(world, "problems")?.position).toEqual({ referencePanel: "inspector", direction: "within" });
      const texte = arrangementSteps("texte", "doppeldeck");
      expect(texte).toEqual([
        { id: "textView" },
        { id: "textEdit", position: { referencePanel: "textView", direction: "below" }, height: 286 },
        { id: "problems", position: { referencePanel: "textEdit", direction: "within" } },
      ]);
      expect(arrangementSteps("pokemon", "doppeldeck")).toEqual([{ id: "pokemonView" }, { id: "pokemonEdit", position: { referencePanel: "pokemonView", direction: "below" }, height: 286 }]);
      expect(arrangementSteps("gegenstaende", "doppeldeck")).toEqual([{ id: "itemView" }, { id: "itemEdit", position: { referencePanel: "itemView", direction: "below" }, height: 286 }]);
    });

    it("refers_only_to_panels_an_earlier_step_added", () => {
      for (const area of DECK_AREAS) {
        for (const arrangement of ["werkbank", "doppeldeck"] as const) {
          const placed = new Set<string>();
          for (const st of arrangementSteps(area, arrangement)) {
            if (st.position) expect(placed.has(st.position.referencePanel)).toBe(true);
            else expect(placed.size).toBe(0);
            placed.add(st.id);
          }
        }
      }
    });

    it("gives_the_areas_without_a_choice_one_plan_whatever_is_asked", () => {
      expect(arrangementSteps("texte", "werkbank")).toEqual(arrangementSteps("texte", "doppeldeck"));
      expect(arrangementSteps("pokemon", "werkbank")).toEqual(arrangementSteps("pokemon", "doppeldeck"));
    });
  });

  it("defaults_the_world_to_werkbank_and_the_others_to_doppeldeck", () => {
    expect(DEFAULT_ARRANGEMENT).toEqual({ welt: "werkbank", texte: "doppeldeck", pokemon: "doppeldeck", gegenstaende: "doppeldeck" });
  });

  it("splits_the_panels_of_the_world_between_the_docks_and_leaves_the_explorer_and_the_text_out", () => {
    expect(DECK_PANELS.welt.schau.map((p) => p.id)).toEqual(["world", "matrix", "worldmap"]);
    expect(DECK_PANELS.welt.hand.map((p) => p.id)).toEqual(["inspector", "script", "problems"]);
    const all = [...DECK_PANELS.welt.schau, ...DECK_PANELS.welt.hand];
    expect(all.some((p) => (p.id as string) === "headers")).toBe(false);
    expect(all.some((p) => (p.id as string) === "text")).toBe(false);
    // The first panel anchors a dock, the others are tabs of its group.
    expect(DECK_PANELS.welt.schau.slice(1).every((p) => p.position.direction === "within" && p.position.referencePanel === "world")).toBe(true);
    const tabs = DECK_PANELS.welt.hand.filter((p) => p.id !== "inspector");
    expect(tabs.map((p) => p.id)).toEqual(["script", "problems"]);
    expect(tabs.every((p) => p.position.direction === "within" && p.position.referencePanel === "inspector")).toBe(true);
    const title = (id: string) => all.find((p) => p.id === id)?.title;
    expect([title("world"), title("matrix"), title("worldmap"), title("inspector"), title("script"), title("problems")]).toEqual(["3D", "Raster", "Weltkarte", "Inspector", "Script", "Probleme"]);
  });

  it("gives_the_texts_the_view_in_the_schau_and_the_messages_with_the_problems_in_the_hand", () => {
    expect(DECK_PANELS.texte.schau.map((p) => p.id)).toEqual(["textView"]);
    expect(DECK_PANELS.texte.hand.map((p) => p.id)).toEqual(["textEdit", "problems"]);
    // The problems are a tab of the messages' group.
    expect(DECK_PANELS.texte.hand[1]?.position).toEqual({ direction: "within", referencePanel: "textEdit" });
    expect([...DECK_PANELS.texte.schau, ...DECK_PANELS.texte.hand].map((p) => p.title)).toEqual(["Vorschau", "Nachrichten", "Probleme"]);
  });

  it("gives_the_pokemon_and_the_items_a_view_and_a_hand_each_without_the_problems", () => {
    expect(DECK_PANELS.pokemon.schau.map((p) => p.id)).toEqual(["pokemonView"]);
    expect(DECK_PANELS.pokemon.hand.map((p) => p.id)).toEqual(["pokemonEdit"]);
    expect(DECK_PANELS.gegenstaende.schau.map((p) => p.id)).toEqual(["itemView"]);
    expect(DECK_PANELS.gegenstaende.hand.map((p) => p.id)).toEqual(["itemEdit"]);
    expect(DECK_AREAS).toEqual(["welt", "texte", "pokemon", "gegenstaende"]);
    expect(isDeckArea("pokemon") && isDeckArea("gegenstaende") && !isDeckArea("geschichten")).toBe(true);
    expect(deckLayoutKey("pokemon", "hand")).toBe("g4.layout.v11.pokemon.hand");
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
      expect(isSaneLayout(layout([leaf("inspector", "script"), leaf("problems")], ["inspector", "script", "problems"]), hand)).toBe(true);
      expect(isSaneLayout(layout([leaf("textEdit", "problems")], ["textEdit", "problems"]), DECK_PANELS.texte.hand)).toBe(true);
    });

    it("checks_against_the_list_it_is_given_so_one_docks_layout_is_not_the_others", () => {
      const handLayout = layout([leaf("inspector", "script", "problems")], ["inspector", "script", "problems"]);
      expect(isSaneLayout(handLayout, hand)).toBe(true);
      expect(isSaneLayout(handLayout, schau)).toBe(false);
      expect(isSaneLayout(handLayout, DECK_PANELS.texte.hand)).toBe(false);
    });

    it("rejects_a_world_hand_of_v7_that_still_has_the_text_panel", () => {
      const v7 = layout([leaf("inspector", "text", "script", "problems"), leaf("context")], ["inspector", "text", "script", "problems", "context"]);
      expect(isSaneLayout(v7, hand)).toBe(false);
    });

    it("rejects_a_world_hand_of_v8_that_still_has_the_tools", () => {
      const v8 = layout([leaf("tools"), leaf("inspector", "script", "problems"), leaf("context")], ["tools", "inspector", "script", "problems", "context"]);
      expect(isSaneLayout(v8, hand)).toBe(false);
    });

    it("rejects_a_world_hand_of_v9_that_still_has_the_context", () => {
      const v9 = layout([leaf("inspector", "script", "problems"), leaf("context")], ["inspector", "script", "problems", "context"]);
      expect(isSaneLayout(v9, hand)).toBe(false);
    });

    it("rejects_a_layout_in_which_a_view_panel_floats", () => {
      const all = AREA_PANELS.welt;
      const ids = all.map((p) => p.id);
      const floating = (...views: string[]) => ({ data: { views, activeView: views[0], id: "float" }, position: { left: 10, top: 10, width: 340, height: 480 } });
      const base = layout([leaf("world", "matrix", "worldmap"), leaf("inspector", "script", "problems")], ids);
      expect(isSaneLayout(base, all)).toBe(true);
      expect(isSaneLayout({ ...base, floatingGroups: [] }, all)).toBe(true);
      // The inspector may float; a view panel never.
      expect(isSaneLayout({ ...layout([leaf("world", "matrix", "worldmap"), leaf("script", "problems")], ids), floatingGroups: [floating("inspector")] }, all)).toBe(true);
      expect(isSaneLayout({ ...layout([leaf("matrix", "worldmap"), leaf("inspector", "script", "problems")], ids), floatingGroups: [floating("world")] }, all)).toBe(false);
      expect(isSaneLayout({ ...layout([leaf("world", "worldmap"), leaf("script", "problems")], ids), floatingGroups: [floating("inspector", "matrix")] }, all)).toBe(false);
      expect(isSaneLayout({ ...base, floatingGroups: [null, { data: {} }, floating("textView")] }, all)).toBe(false);
    });

    it("rejects_a_layout_with_a_missing_or_an_unknown_panel_or_no_panels_at_all", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix")], ["world", "matrix"]), schau)).toBe(false);
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap", "status")], ["world", "matrix", "worldmap", "status"]), schau)).toBe(false);
      expect(isSaneLayout({}, schau)).toBe(false);
      expect(isSaneLayout({ panels: null }, schau)).toBe(false);
    });
  });
});
