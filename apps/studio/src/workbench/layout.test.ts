// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { AREAS } from "../ui/frame/shellState";
import { AREA_PANELS, DECK_AREAS, DEFAULT_ARRANGEMENT, arrangementKey, arrangementSteps, dropOldLayouts, isDeckArea, isSaneLayout, isSchauPanel, layoutKey, loadArrangement, loadLayout, panelsFor, saveArrangement, saveLayout } from "./layout";
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

  it("remembers_the_arrangement_of_each_area_and_falls_back_to_its_default", () => {
    expect(arrangementKey("welt")).toBe("g4.layout.v11.welt.arrangement");
    expect(loadArrangement("welt")).toBe("werkbank");
    expect(loadArrangement("texte")).toBe("doppeldeck");
    saveArrangement("welt", "doppeldeck");
    expect(loadArrangement("welt")).toBe("doppeldeck");
    expect(loadArrangement("texte")).toBe("doppeldeck");
    localStorage.setItem(arrangementKey("welt"), "anything");
    expect(loadArrangement("welt")).toBe("werkbank");
  });

  it("round_trips_a_layout_through_local_storage_and_tolerates_garbage", () => {
    const key = layoutKey("welt");
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
    localStorage.setItem("g4.layout.v11.welt.arrangement", "werkbank");
    localStorage.setItem("g4.layout.v12.welt", "{}");
    localStorage.setItem("g4.shell.v1", "{}");
    // An interim build of v11 stored the two docks of a deck under their own keys; one dock per area does not read them.
    localStorage.setItem("g4.layout.v11.welt.schau", "{}");
    localStorage.setItem("g4.layout.v11.texte.hand", "{}");
    dropOldLayouts(localStorage);
    const left = Array.from({ length: localStorage.length }, (_, i) => localStorage.key(i)).sort();
    expect(left).toEqual(["g4.layout.v11.welt", "g4.layout.v11.welt.arrangement", "g4.layout.v12.welt", "g4.shell.v1"]);
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
      // Every arrangement of the area places exactly the area's panels of the depth, each once.
      for (const arrangement of ["werkbank", "doppeldeck"] as const) {
        for (const depth of ["bauen", "tuefteln"] as const) {
          expect(arrangementSteps(area, arrangement, depth).map((st) => st.id).sort()).toEqual(panelsFor(area, depth).map((p) => p.id as string).sort());
        }
      }
      expect(panelsFor(area, "tuefteln")).toEqual(AREA_PANELS[area]);
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
      const steps = arrangementSteps("welt", "werkbank", "tuefteln");
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
      const world = arrangementSteps("welt", "doppeldeck", "tuefteln");
      expect(world.map((st) => st.id)).toEqual(["world", "matrix", "worldmap", "inspector", "script", "problems"]);
      expect(step(world, "inspector")).toEqual({ id: "inspector", position: { referencePanel: "world", direction: "below" }, height: 286 });
      expect(step(world, "script")?.position).toEqual({ referencePanel: "inspector", direction: "within" });
      expect(step(world, "problems")?.position).toEqual({ referencePanel: "inspector", direction: "within" });
      const texte = arrangementSteps("texte", "doppeldeck", "tuefteln");
      expect(texte).toEqual([
        { id: "textView" },
        { id: "textEdit", position: { referencePanel: "textView", direction: "below" }, height: 286 },
        { id: "problems", position: { referencePanel: "textEdit", direction: "within" } },
      ]);
      expect(arrangementSteps("pokemon", "doppeldeck", "tuefteln")).toEqual([{ id: "pokemonView" }, { id: "pokemonEdit", position: { referencePanel: "pokemonView", direction: "below" }, height: 286 }]);
      expect(arrangementSteps("gegenstaende", "doppeldeck", "tuefteln")).toEqual([{ id: "itemView" }, { id: "itemEdit", position: { referencePanel: "itemView", direction: "below" }, height: 286 }]);
    });

    it("refers_only_to_panels_an_earlier_step_added", () => {
      for (const area of DECK_AREAS) {
        for (const arrangement of ["werkbank", "doppeldeck"] as const) {
          for (const depth of ["bauen", "tuefteln"] as const) {
            const placed = new Set<string>();
            for (const st of arrangementSteps(area, arrangement, depth)) {
              if (st.position) expect(placed.has(st.position.referencePanel)).toBe(true);
              else expect(placed.size).toBe(0);
              placed.add(st.id);
            }
          }
        }
      }
    });

    it("leaves_the_raster_out_of_both_world_arrangements_in_bauen", () => {
      expect(AREA_PANELS.welt.filter((p) => p.tinkerOnly).map((p) => p.id)).toEqual(["matrix"]);
      expect(panelsFor("welt", "bauen").map((p) => p.id)).toEqual(["world", "worldmap", "inspector", "script", "problems"]);
      for (const arrangement of ["werkbank", "doppeldeck"] as const) {
        const bauen = arrangementSteps("welt", arrangement, "bauen");
        const tuefteln = arrangementSteps("welt", arrangement, "tuefteln");
        expect(bauen.map((st) => st.id)).toEqual(["world", "worldmap", "inspector", "script", "problems"]);
        // The other steps are the same as in Tüfteln.
        expect(bauen).toEqual(tuefteln.filter((st) => st.id !== "matrix"));
      }
      // The other areas have no panel only for Tüfteln.
      for (const area of ["texte", "pokemon", "gegenstaende"] as const) expect(arrangementSteps(area, "doppeldeck", "bauen")).toEqual(arrangementSteps(area, "doppeldeck", "tuefteln"));
    });

    it("keeps_the_raster_in_the_view_group_in_tuefteln", () => {
      for (const arrangement of ["werkbank", "doppeldeck"] as const) {
        const steps = arrangementSteps("welt", arrangement, "tuefteln");
        expect(steps.slice(0, 3).map((st) => st.id)).toEqual(["world", "matrix", "worldmap"]);
        expect(step(steps, "matrix")).toEqual({ id: "matrix", position: { referencePanel: "world", direction: "within" } });
      }
    });

    it("gives_the_areas_without_a_choice_one_plan_whatever_is_asked", () => {
      expect(arrangementSteps("texte", "werkbank", "tuefteln")).toEqual(arrangementSteps("texte", "doppeldeck", "tuefteln"));
      expect(arrangementSteps("pokemon", "werkbank", "tuefteln")).toEqual(arrangementSteps("pokemon", "doppeldeck", "tuefteln"));
    });
  });

  it("defaults_the_world_to_werkbank_and_the_others_to_doppeldeck", () => {
    expect(DEFAULT_ARRANGEMENT).toEqual({ welt: "werkbank", texte: "doppeldeck", pokemon: "doppeldeck", gegenstaende: "doppeldeck" });
  });

  it("knows_the_areas_with_a_deck", () => {
    expect(DECK_AREAS).toEqual(["welt", "texte", "pokemon", "gegenstaende"]);
    expect(isDeckArea("pokemon") && isDeckArea("gegenstaende") && !isDeckArea("geschichten") && !isDeckArea("start")).toBe(true);
  });

  describe("isSaneLayout", () => {
    const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
    const layout = (leaves: object[], panelIds: string[]) => ({
      grid: { root: { type: "branch", data: leaves }, width: 1000, height: 800, orientation: "HORIZONTAL" },
      panels: Object.fromEntries(panelIds.map((id) => [id, { id }])),
    });
    // Panel lists of their own (a part of an area's panels), so one list's layout is checked against another.
    const schau = AREA_PANELS.welt.filter((p) => p.schau);
    const hand = AREA_PANELS.welt.filter((p) => !p.schau);
    const textHand = AREA_PANELS.texte.filter((p) => !p.schau);

    it("accepts_a_layout_with_exactly_the_panels_it_is_given", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap")], ["world", "matrix", "worldmap"]), schau)).toBe(true);
      expect(isSaneLayout(layout([leaf("inspector", "script"), leaf("problems")], ["inspector", "script", "problems"]), hand)).toBe(true);
      expect(isSaneLayout(layout([leaf("textEdit", "problems")], ["textEdit", "problems"]), textHand)).toBe(true);
    });

    it("checks_against_the_list_it_is_given", () => {
      const handLayout = layout([leaf("inspector", "script", "problems")], ["inspector", "script", "problems"]);
      expect(isSaneLayout(handLayout, hand)).toBe(true);
      expect(isSaneLayout(handLayout, schau)).toBe(false);
      expect(isSaneLayout(handLayout, textHand)).toBe(false);
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

    it("accepts_a_stored_layout_with_or_without_the_raster", () => {
      const all = AREA_PANELS.welt;
      const withRaster = layout([leaf("world", "matrix", "worldmap"), leaf("inspector", "script", "problems")], ["world", "matrix", "worldmap", "inspector", "script", "problems"]);
      const withoutRaster = layout([leaf("world", "worldmap"), leaf("inspector", "script", "problems")], ["world", "worldmap", "inspector", "script", "problems"]);
      expect(isSaneLayout(withRaster, all)).toBe(true);
      expect(isSaneLayout(withoutRaster, all)).toBe(true);
      // Only the Raster may be missing: without 3D or the inspector the layout is of another panel set.
      expect(isSaneLayout(layout([leaf("matrix", "worldmap"), leaf("inspector", "script", "problems")], ["matrix", "worldmap", "inspector", "script", "problems"]), all)).toBe(false);
      expect(isSaneLayout(layout([leaf("world", "worldmap"), leaf("script", "problems")], ["world", "worldmap", "script", "problems"]), all)).toBe(false);
      // The list of a depth is checked the same way.
      expect(isSaneLayout(withoutRaster, panelsFor("welt", "bauen"))).toBe(true);
    });

    it("rejects_a_layout_with_a_missing_or_an_unknown_panel_or_no_panels_at_all", () => {
      expect(isSaneLayout(layout([leaf("world", "matrix")], ["world", "matrix"]), schau)).toBe(false);
      expect(isSaneLayout(layout([leaf("world", "matrix", "worldmap", "status")], ["world", "matrix", "worldmap", "status"]), schau)).toBe(false);
      expect(isSaneLayout({}, schau)).toBe(false);
      expect(isSaneLayout({ panels: null }, schau)).toBe(false);
    });
  });
});
