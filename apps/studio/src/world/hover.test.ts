import { describe, expect, it } from "vitest";
import { describeTile, tileSummary } from "./hover";

describe("describeTile", () => {
  it("describes a stairs tile and an unknown tile", () => {
    const names = new Map([[0x5e, { value: 0x5e, name: "WARP_STAIRS_EAST", class: "warp" }]]);
    expect(describeTile([0x5e, 0x80], names)).toEqual({ kind: 0x5e, collision: 0x80, blocked: true, name: "WARP_STAIRS_EAST", class: "warp" });
    expect(describeTile([0x33, 0], names)).toEqual({ kind: 0x33, collision: 0, blocked: false, name: null, class: null });
  });
});

describe("tileSummary", () => {
  const named = { kind: 0x5e, collision: 0x80, blocked: true, name: "WARP_STAIRS_EAST", class: "warp" };
  const unnamed = { kind: 0x3, collision: 0, blocked: false, name: null, class: null };
  const grass = { kind: 0x2, collision: 0, blocked: false, name: "TALL_GRASS", class: "grass" };

  it("formats map, coordinates, type and blocking in tinkering", () => {
    expect(tileSummary(187, 8, 4, named, "tuefteln")).toBe("Karte 0187 · Kachel x 8, y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert");
    expect(tileSummary(1, 0, 31, unnamed, "tuefteln")).toBe("Karte 0001 · Kachel x 0, y 31 · Typ 0x03 · begehbar");
    expect(tileSummary(1, 0, 31, grass, "tuefteln")).toBe("Karte 0001 · Kachel x 0, y 31 · Typ 0x02 TALL_GRASS · begehbar");
  });

  it("says the tile first and no type or constant in building", () => {
    expect(tileSummary(190, 9, 13, unnamed, "bauen")).toBe("Kachel x 9, y 13 · Karte 0190 · begehbar");
    expect(tileSummary(190, 9, 13, grass, "bauen")).toBe("Kachel x 9, y 13 · Karte 0190 · hohes Gras");
    // A walkable warp tile has no plain word of its own; a blocked tile is blocked whatever it is.
    expect(tileSummary(190, 9, 13, { ...named, blocked: false }, "bauen")).toBe("Kachel x 9, y 13 · Karte 0190 · begehbar");
    expect(tileSummary(187, 8, 4, named, "bauen")).toBe("Kachel x 8, y 4 · Karte 0187 · blockiert");
    expect(tileSummary(190, 9, 13, { ...grass, name: "TALL_GRASS" }, "bauen")).not.toMatch(/0x|TALL_GRASS/);
  });
});
