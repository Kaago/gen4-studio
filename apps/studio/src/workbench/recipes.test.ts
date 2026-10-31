import { describe, expect, it } from "vitest";
import { RECIPE_BAR, nextStepsFor, recipeRequest } from "./recipes";

describe("recipes", () => {
  it("the_bar_runs_area_and_room_and_locks_the_rest_until_3d5", () => {
    expect(RECIPE_BAR.map((r) => [r.id, r.reason ?? null])).toEqual([
      ["create_area", null],
      ["create_room", null],
      ["new_figure", "Kommt mit 3d.5"],
      ["connect_doors", "Kommt mit 3d.5"],
      ["new_trainer_here", "Kommt mit 3d.5"],
    ]);
    // Every entry says what it makes, in the tooltip's second line.
    for (const r of RECIPE_BAR) expect(r.hint).not.toBe("");
  });

  it("next_steps_mirror_the_kernels_table", () => {
    const short = (recipe: Parameters<typeof nextStepsFor>[0]) => nextStepsFor(recipe).map((s) => [s.id, s.label, s.available, s.reason]);
    expect(short("create_area")).toEqual([
      ["connect_doors", "Tür verbinden", false, "Kommt mit 3d.5"],
      ["new_figure", "Neue Figur hier", false, "Kommt mit 3d.5"],
    ]);
    expect(short("create_room")).toEqual([["new_figure", "Neue Figur hier", false, "Kommt mit 3d.5"]]);
    expect(short("extend_area")).toEqual([["adjust_terrain", "Gelände anpassen", false, "Kommt mit Phase 6"]]);
    expect(short("empty_region")).toEqual([["create_area", "Gebiet anlegen", true, null]]);
  });

  it("a_recipe_opens_its_wizard_and_the_room_gets_its_door_outside_in_the_given_header", () => {
    expect(recipeRequest("create_area", 7)).toEqual({ kind: "area" });
    expect(recipeRequest("create_room", 7)).toEqual({ kind: "room", prefill: { outsideHeader: 7 } });
    expect(recipeRequest("create_room", null)).toEqual({ kind: "room" });
    // A recipe without a wizard (yet) opens nothing.
    expect(recipeRequest("new_figure", 7)).toBeNull();
    expect(recipeRequest("adjust_terrain", 7)).toBeNull();
  });
});
