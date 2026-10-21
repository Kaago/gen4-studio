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
  it("formats map, coordinates, type and blocking", () => {
    const named = { kind: 0x5e, collision: 0x80, blocked: true, name: "WARP_STAIRS_EAST", class: "warp" };
    expect(tileSummary(187, 8, 4, named)).toBe("Karte 0187 · Kachel x 8, y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert");
    const unnamed = { kind: 0x3, collision: 0, blocked: false, name: null, class: null };
    expect(tileSummary(1, 0, 31, unnamed)).toBe("Karte 0001 · Kachel x 0, y 31 · Typ 0x03 · begehbar");
  });
});
