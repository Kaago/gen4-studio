import { describe, expect, it } from "vitest";
import { permissionColor } from "./colors";
import { legendEntries } from "./legend";

describe("legendEntries", () => {
  it("lists_the_seven_terrain_classes_with_their_names", () => {
    const { terrain } = legendEntries();
    expect(terrain.map((t) => t.label)).toEqual(["Blockiert", "Wasser", "Hohes Gras", "Vorsprung", "Eis", "Spezial", "Unbekannt"]);
  });

  it("lists_the_five_event_kinds_with_their_names", () => {
    const { events } = legendEntries();
    expect(events.map((e) => e.label)).toEqual(["Figur", "Tür", "Auslöser", "Schild", "Gegenstand"]);
  });

  it("gives_every_entry_a_distinct_colour_as_hash_rrggbb", () => {
    const { terrain, events } = legendEntries();
    for (const e of [...terrain, ...events]) expect(e.color).toMatch(/^#[0-9A-F]{6}$/);
    expect(new Set(terrain.map((t) => t.color)).size).toBe(terrain.length);
    expect(new Set(events.map((e) => e.color)).size).toBe(events.length);
  });

  it("shows_the_colours_the_overlay_paints", () => {
    const colorOf = (label: string) => legendEntries().terrain.find((t) => t.label === label)?.color;
    expect(colorOf("Blockiert")).toBe(permissionColor(0, 0x80, null));
    expect(colorOf("Wasser")).toBe(permissionColor(0, 0, "water"));
    expect(colorOf("Hohes Gras")).toBe(permissionColor(0, 0, "grass"));
    expect(colorOf("Vorsprung")).toBe(permissionColor(0, 0, "ledge"));
    expect(colorOf("Eis")).toBe(permissionColor(0, 0, "ice"));
    expect(colorOf("Spezial")).toBe(permissionColor(0, 0, "door"));
    expect(colorOf("Unbekannt")).toBe(permissionColor(0, 0, null));
  });

  it("terrain_entries_name_their_pattern", () => {
    const patternOf = (label: string) => legendEntries().terrain.find((t) => t.label === label)?.pattern;
    expect(legendEntries().terrain.map((t) => t.pattern)).toEqual(["hatch", "waves", "dots", "arrow", "crosshatch", "none", "none"]);
    expect(patternOf("Blockiert")).toBe("hatch");
    expect(patternOf("Vorsprung")).toBe("arrow");
  });

  it("gives_every_patterned_entry_the_shade_the_shader_draws", () => {
    for (const t of legendEntries().terrain) {
      if (t.pattern === "none") expect(t.patternColor).toBeNull();
      else expect(t.patternColor).toMatch(/^#[0-9A-F]{6}$/);
    }
    const blocked = legendEntries().terrain[0];
    // Cell colour × 0.55 (in the linear working space, like the shader): darker than the fill.
    const sum = (hex: string) => [1, 3, 5].reduce((a, i) => a + parseInt(hex.slice(i, i + 2), 16), 0);
    expect(sum(blocked.patternColor!)).toBeLessThan(sum(blocked.color));
  });
});
