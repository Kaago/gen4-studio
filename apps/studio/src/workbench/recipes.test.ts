import { describe, expect, it } from "vitest";
import { RECIPE_BAR, nextStepsFor, recipeRequest } from "./recipes";

describe("recipes", () => {
  it("all_bar_recipes_run_and_next_knows_the_new_ones", () => {
    // Since 3d.5 every recipe of the bar runs: no entry carries a reason.
    expect(RECIPE_BAR.map((r) => [r.id, r.reason ?? null])).toEqual([
      ["create_area", null],
      ["create_room", null],
      ["new_figure", null],
      ["connect_doors", null],
      ["new_trainer_here", null],
    ]);
    // Every entry says what it makes, in the tooltip's second line.
    for (const r of RECIPE_BAR) expect(r.hint).not.toBe("");
    const short = (recipe: Parameters<typeof nextStepsFor>[0]) => nextStepsFor(recipe).map((s) => [s.id, s.label, s.available, s.reason]);
    // Area and room name the recipes of 3d.5, now free.
    expect(short("create_area")).toEqual([
      ["connect_doors", "Tür verbinden", true, null],
      ["new_figure", "Neue Figur hier", true, null],
    ]);
    expect(short("create_room")).toEqual([["new_figure", "Neue Figur hier", true, null]]);
    expect(short("extend_area")).toEqual([["adjust_terrain", "Gelände anpassen", true, null]]);
    expect(short("empty_region")).toEqual([["create_area", "Gebiet anlegen", true, null]]);
    expect(short("connect_doors")).toEqual([["play_here", "Hier spielen", false, "Kommt mit 3d.9"]]);
    expect(short("new_trainer_here")).toEqual([["play_here", "Hier spielen", false, "Kommt mit 3d.9"]]);
  });

  it("next_steps_after_3d7_free_story_join", () => {
    const short = (recipe: Parameters<typeof nextStepsFor>[0]) => nextStepsFor(recipe).map((s) => [s.id, s.label, s.available, s.reason]);
    // The dialog editor and „Zu einer Geschichte“ („Gespräch schaltet weiter“ with the figure set) are always there.
    expect(short("new_figure")).toEqual([
      ["edit_dialog", "Dialog bearbeiten", true, null],
      ["story_join", "Zu einer Geschichte", true, null],
    ]);
    // The kernel's `next` of the five story recipes: their steps run, playing comes with 3d.9.
    expect(short("new_story")).toEqual([
      ["talk_advances", "Gespräch schaltet weiter", true, null],
      ["figure_appears", "Figur erscheint ab Schritt", true, null],
    ]);
    expect(short("talk_advances")).toEqual([
      ["figure_appears", "Figur erscheint ab Schritt", true, null],
      ["scene_on_enter", "Szene beim Betreten", true, null],
    ]);
    for (const recipe of ["figure_appears", "scene_on_enter", "reward_once"] as const) {
      expect(short(recipe)).toEqual([["play_here", "Hier spielen", false, "Kommt mit 3d.9"]]);
    }
  });

  it("adjust_terrain_is_no_longer_locked", () => {
    // Since 6.4a the app runs the step (`startPainting`): free after „Gebiet erweitern“ as after „Neue Karte malen“.
    for (const recipe of ["extend_area", "new_painted_map"] as const) {
      expect(nextStepsFor(recipe).find((s) => s.id === "adjust_terrain")).toEqual({ id: "adjust_terrain", label: "Gelände anpassen", available: true, reason: null });
    }
  });

  it("new_painted_map_leads_to_adjust_terrain", () => {
    expect(nextStepsFor("new_painted_map").map((s) => [s.id, s.label, s.available, s.reason])).toEqual([["adjust_terrain", "Gelände anpassen", true, null]]);
    // No wizard: the app starts painting itself.
    expect(recipeRequest("new_painted_map", 7)).toBeNull();
  });

  it("a_recipe_opens_its_wizard_and_the_room_gets_its_door_outside_in_the_given_header", () => {
    expect(recipeRequest("create_area", 7)).toEqual({ kind: "area" });
    expect(recipeRequest("create_room", 7)).toEqual({ kind: "room", prefill: { outsideHeader: 7 } });
    expect(recipeRequest("create_room", null)).toEqual({ kind: "room" });
    // A recipe without a wizard (yet) opens nothing.
    expect(recipeRequest("adjust_terrain", 7)).toBeNull();
    expect(recipeRequest("edit_dialog", 7)).toBeNull();
  });

  it("recipe_request_opens_the_three_wizards_with_prefill", () => {
    // The figure and the trainer stand in the given header, on the given tile when there is one.
    expect(recipeRequest("new_figure", 7)).toEqual({ kind: "new_figure", prefill: { header: 7 } });
    expect(recipeRequest("new_figure", 7, { x: 40, z: 33 })).toEqual({ kind: "new_figure", prefill: { header: 7, tile: { x: 40, z: 33 } } });
    expect(recipeRequest("new_figure", null)).toEqual({ kind: "new_figure" });
    expect(recipeRequest("new_trainer_here", 7, { x: 1, z: 2 })).toEqual({ kind: "new_trainer_here", prefill: { header: 7, tile: { x: 1, z: 2 } } });
    expect(recipeRequest("new_trainer_here", null)).toEqual({ kind: "new_trainer_here" });
    // Door A of „Tür verbinden“ lies in the given header (a tile there when one is given).
    expect(recipeRequest("connect_doors", 7)).toEqual({ kind: "connect_doors", prefill: { a: { header: 7 } } });
    expect(recipeRequest("connect_doors", 7, { x: 3, z: 4 })).toEqual({ kind: "connect_doors", prefill: { a: { header: 7, tile: { x: 3, z: 4 } } } });
    expect(recipeRequest("connect_doors", null)).toEqual({ kind: "connect_doors" });
  });
});
