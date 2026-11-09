import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SHORTCUTS, deletePrompt } from "./buildings";
import { KIND_LABELS, eventDeletePrompt, lockedText } from "./events";
import { heightWords } from "./ground";
import { tileSummary } from "./hover";
import { legendEntries } from "./legend";
import { textureNote } from "./load";
import { strokeLabel } from "./paint/ops";

// The world's texts come from functions that read the catalog when they are called; in English they are English, and in German again
// after the switch back (the tables built at module scope, `SHORTCUTS` and `KIND_LABELS`, read it on access).
describe("the world in English", () => {
  beforeEach(async () => {
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    await i18next.changeLanguage("de");
  });

  it("says tiles and heights in English, with a decimal point", () => {
    const grass = { kind: 0x2, collision: 0, blocked: false, name: "TALL_GRASS", class: "grass" };
    expect(tileSummary(190, 9, 13, grass, "bauen", 4)).toBe("Tile x 9, y 13 · Map 0190 · tall grass · Height 0.5 steps");
    expect(tileSummary(190, 9, 13, grass, "tuefteln", 8)).toBe("Map 0190 · Tile x 9, y 13 · Type 0x02 TALL_GRASS · walkable · Height 8 units (1 step)");
    expect(heightWords(null, "bauen")).toBe("no height");
  });

  it("names the event classes, shortcuts and legend in English", () => {
    expect(KIND_LABELS.object).toBe("Object");
    expect(SHORTCUTS.trainer.label).toBe("Trainer (in the Pokémon area)");
    expect(legendEntries().events.map((e) => e.label)).toEqual(["Character", "Door", "Trigger", "Sign", "Item"]);
    expect(legendEntries().terrain[2]!.label).toBe("Tall grass");
  });

  it("words the prompts, notes and history texts in English", () => {
    expect(deletePrompt(12, 3, { model_id: 7, x: -5.5, y: 1, z: 4 })).toBe("Delete building 4 of map 0012 (model 7, x -5.5, y 1, z 4)?");
    expect(eventDeletePrompt("coord", 3, undefined)).toBe("Delete Trigger 3?");
    expect(lockedText("17 bytes read")).toBe("read-only: 17 bytes read");
    expect(strokeLabel("brush", 123, 1)).toBe("Painted: Map 0123 (1 cell)");
    expect(strokeLabel("cliff", 123, 3, true)).toBe("Cliff lowered: Map 0123 (3 cells)");
    expect(textureNote([{ tileset: 12, reason: "x" }] as never)).toBe("Tileset 0012 not convertible – maps untextured");
  });
});
