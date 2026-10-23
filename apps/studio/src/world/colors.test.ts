import { describe, it, expect } from "vitest";
import { dunkel, hell, szene } from "../ui/tokens/tokens";
import { eventColor, hexOf, permissionColor, sceneBackground, selectionColor, terrainColor } from "./colors";

describe("hexOf", () => {
  it("writes_a_token_number_as_six_upper_case_hex_digits", () => {
    expect(hexOf(0xd65a00)).toBe("#D65A00");
    expect(hexOf(0x0073b5)).toBe("#0073B5");
    expect(hexOf(0x000000)).toBe("#000000");
  });
});

describe("permissionColor", () => {
  it("marks_blocked_cells_regardless_of_kind_and_class", () => {
    const blocked = hexOf(szene.gelaende.blockiert);
    expect(permissionColor(0, 0x80, null)).toBe(blocked);
    expect(permissionColor(5, 0x80, "grass")).toBe(blocked);
    expect(permissionColor(5, 0x81, "water")).toBe(blocked);
  });

  it("maps_every_behaviour_class_to_its_terrain_token", () => {
    const expected: Record<string, keyof typeof szene.gelaende> = {
      block: "blockiert",
      water: "wasser",
      grass: "hohes-gras",
      ledge: "vorsprung",
      ice: "eis",
      sand: "spezial",
      slide: "spezial",
      warp: "spezial",
      door: "spezial",
      bridge: "spezial",
      furniture: "spezial",
    };
    for (const [cls, key] of Object.entries(expected)) {
      expect(permissionColor(0x02, 0, cls), cls).toBe(hexOf(szene.gelaende[key]));
    }
  });

  it("leaves_unnamed_kinds_and_other_unknown", () => {
    const unknown = hexOf(szene.gelaende.unbekannt);
    expect(permissionColor(0, 0, null)).toBe(unknown);
    expect(permissionColor(0x33, 0, null)).toBe(unknown);
    expect(permissionColor(0x08, 0, "other")).toBe(unknown);
    expect(permissionColor(0x08, 0, "something-new")).toBe(unknown);
  });

  it("gives_the_colours_as_hash_rrggbb", () => {
    expect(permissionColor(0, 0, "water")).toMatch(/^#[0-9A-F]{6}$/);
    expect(permissionColor(0, 0, "water")).toBe("#0073B5");
  });
});

describe("scene colours", () => {
  it("builds_terrain_and_event_colours_from_the_tokens", () => {
    expect(terrainColor("eis")).toBe("#52B5E7");
    expect(eventColor("warp")).toBe("#AD3173");
  });

  it("picks_the_selection_outline_by_theme_so_it_stays_visible_on_the_dark_surface", () => {
    expect(selectionColor("hell")).toBe(hexOf(szene.auswahl.halo));
    expect(selectionColor("dunkel")).toBe(hexOf(szene.auswahl.kern));
    expect(selectionColor("hell")).not.toBe(selectionColor("dunkel"));
  });

  it("takes_the_background_of_the_scene_from_the_theme", () => {
    expect(sceneBackground("hell")).toBe(hell["flaeche/welt"]);
    expect(sceneBackground("dunkel")).toBe(dunkel["flaeche/welt"]);
    expect(sceneBackground("hell")).not.toBe(sceneBackground("dunkel"));
  });
});
