import { describe, expect, it } from "vitest";
import { MARKER_COLORS, eventDetails, eventTitle, markerPosition } from "./events";

const m = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2] };

describe("markerPosition", () => {
  it("places markers in world space and rejects coordinates outside the matrix", () => {
    expect(markerPosition(8, 4, m)).toEqual({ wx: 8.5, wz: 4.5 });
    expect(markerPosition(40, 4, m)).toEqual({ wx: 40.5, wz: 4.5 });
    expect(markerPosition(64, 4, m)).toBeNull();
    expect(markerPosition(8, 32, m)).toBeNull();
    expect(markerPosition(-1, 0, m)).toBeNull();
    expect(markerPosition(0, -1, m)).toBeNull();
  });

  it("accepts the last tile of the matrix", () => {
    expect(markerPosition(63, 31, m)).toEqual({ wx: 63.5, wz: 31.5 });
  });
});

describe("MARKER_COLORS", () => {
  it("has one colour per event class", () => {
    expect(MARKER_COLORS).toEqual({ warp: "#2563eb", object: "#eab308", coord: "#22c55e", bg: "#a855f7" });
  });
});

describe("event texts", () => {
  const warp = { index: 0, x: 8, z: 4, dest_header_id: 414, dest_warp_id: 1, height: 0 };

  it("names a warp with its destination header and internal name", () => {
    expect(eventTitle("warp", warp, "T01R0201")).toBe("Warp 0 → Header 0414 (T01R0201)");
    expect(eventTitle("warp", warp)).toBe("Warp 0 → Header 0414");
    expect(eventDetails("warp", warp)).toBe("Position x 8, z 4 · Ziel-Warp 1 · Höhe 0");
  });

  it("describes objects, triggers and background events", () => {
    const object = { index: 2, local_id: 7, graphics_id: 9, movement_type: 3, script: 2001, x: 12, z: 13, y: -1 };
    expect(eventTitle("object", object)).toBe("Objekt 2 · ID 7");
    expect(eventDetails("object", object)).toBe("Position x 12, z 13, y -1 · Grafik 9 · Bewegung 3 · Skript 2001");
    const coord = { index: 1, script: 9000, x: 3, z: 5, width: 2, length: 1 };
    expect(eventTitle("coord", coord)).toBe("Trigger 1 · Skript 9000");
    expect(eventDetails("coord", coord)).toBe("Position x 3, z 5 · Größe 2 × 1");
    const bg = { index: 0, script: 100, event_type: 1, x: 20, z: 21, y: 0 };
    expect(eventTitle("bg", bg)).toBe("Hintergrund-Event 0 · Skript 100");
    expect(eventDetails("bg", bg)).toBe("Position x 20, z 21, y 0 · Typ 1");
  });
});
