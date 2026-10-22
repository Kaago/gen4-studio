import { describe, expect, it } from "vitest";
import type { EventsDto } from "../api";
import { MARKER_COLORS, clampToMatrix, eventAt, eventDeletePrompt, eventEditGate, eventDetails, eventReadOnlyReason, eventTitle, lockedText, markerPosition, warpDestinationLabel } from "./events";

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

describe("warpDestinationLabel", () => {
  const hs = [
    { id: 414, internal_name: "T01R0201", location_name: "Zweiblattdorf", matrix_id: 126, area_data_id: 20, event_file_id: 1 },
    { id: 7, internal_name: "D99", location_name: "", matrix_id: 1, area_data_id: 1, event_file_id: 2 },
  ];

  it("labels warp destinations and flags unknown headers", () => {
    expect(warpDestinationLabel(414, hs)).toEqual({ text: "0414 Zweiblattdorf (T01R0201)", known: true });
    expect(warpDestinationLabel(1234, hs)).toEqual({ text: "unbekannt (1234)", known: false });
  });

  it("shows the internal name once for a header without a location name", () => {
    expect(warpDestinationLabel(7, hs)).toEqual({ text: "0007 D99", known: true });
  });
});

describe("read-only events", () => {
  it("shows the reason of a locked file", () => {
    expect(lockedText("17 bytes read, 16 would be written")).toBe("schreibgeschützt: 17 bytes read, 16 would be written");
  });

  it("takes the reason out of a read_only error about an event file", () => {
    const err = { code: "read_only", message: "event file 0001 is read-only: warp height must be 0", asset: { kind: "Event", index: 1 } };
    expect(eventReadOnlyReason(err)).toBe("warp height must be 0");
  });

  it("leaves every other error alone", () => {
    expect(eventReadOnlyReason({ code: "read_only", message: "map 0001 is read-only: x", asset: { kind: "Map", index: 1 } })).toBeNull();
    expect(eventReadOnlyReason({ code: "error", message: "boom", asset: null })).toBeNull();
    expect(eventReadOnlyReason(new Error("x"))).toBeNull();
  });
});

describe("event identity helpers", () => {
  const events: EventsDto = {
    event_file_id: 1,
    locked: null,
    bg: [],
    objects: [],
    warps: [
      { index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 },
      { index: 1, x: 9, z: 4, dest_header_id: 2, dest_warp_id: 0, height: 0 },
    ],
    coords: [],
  };

  it("finds an event by class and index", () => {
    expect(eventAt(events, "warp", 1)?.x).toBe(9);
    expect(eventAt(events, "warp", 2)).toBeUndefined();
    expect(eventAt(events, "object", 0)).toBeUndefined();
  });

  it("names kind, index and position in the delete prompt", () => {
    expect(eventDeletePrompt("warp", 1, events.warps[1])).toBe("Warp 1 (x 9, z 4) löschen?");
    expect(eventDeletePrompt("coord", 3, undefined)).toBe("Trigger 3 löschen?");
  });

  it("keeps dragged tiles inside the matrix", () => {
    expect(clampToMatrix({ x: 70, z: -3 }, m)).toEqual({ x: 63, z: 0 });
    expect(clampToMatrix({ x: 5, z: 40 }, m)).toEqual({ x: 5, z: 31 });
    expect(clampToMatrix({ x: 12, z: 7 }, m)).toEqual({ x: 12, z: 7 });
  });
});

describe("eventEditGate", () => {
  it("allows editing a file that is not locked, or when nothing is known about it", () => {
    expect(eventEditGate(null, 1)).toEqual({ allowed: true });
    expect(eventEditGate({ file: 2, reason: "x" }, 1)).toEqual({ allowed: true });
  });

  it("refuses a locked file with the text the inspector shows", () => {
    expect(eventEditGate({ file: 1, reason: "17 bytes read, 16 would be written" }, 1)).toEqual({
      allowed: false,
      text: "schreibgeschützt: 17 bytes read, 16 would be written",
    });
  });
});
