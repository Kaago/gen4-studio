import { describe, expect, it } from "vitest";
import type { TileBehavior } from "../api";
import { DOOR, DOOR_COLLISION, warpTile } from "./warpTile";

const names = new Map<number, TileBehavior>([
  [0x5e, { value: 0x5e, name: "WARP_STAIRS_EAST", class: "warp" }],
  [0x69, { value: 0x69, name: "DOOR", class: "door" }],
  [0xa9, { value: 0xa9, name: "SNOW_WITH_SHADOWS", class: "other" }],
]);

describe("warpTile", () => {
  it("fires_on_warp_tiles_and_on_blocking_doors_only", () => {
    // Gate 3b.1 (A) 9: the first return warp stood on snow and was never taken.
    expect(warpTile(0x5e, 0, names).fires).toBe(true);
    expect(warpTile(DOOR, DOOR_COLLISION, names).fires).toBe(true);
    expect(warpTile(DOOR, 0, names)).toEqual({ fires: false, text: "Verhalten 0x69 DOOR, Kollision 0x00: ein Warp löst hier im Spiel nicht aus (eine Tür löst nur mit Kollision 0x80 aus)." });
    expect(warpTile(0xa9, 0, names)).toEqual({ fires: false, text: "Verhalten 0xA9 SNOW_WITH_SHADOWS, Kollision 0x00: ein Warp löst hier im Spiel nicht aus (keine Tür- oder Warp-Kachel)." });
    expect(warpTile(0x00, 0x80, names).fires).toBe(false);
  });
});
