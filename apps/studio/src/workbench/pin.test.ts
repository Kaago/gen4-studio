import { describe, expect, it } from "vitest";
import { pinAfterHistory, shownInspector } from "./pin";
import type { InspectorModel } from "./Inspector";

describe("pinAfterHistory", () => {
  it("lets go of a pinned building, whatever its index", () => {
    expect(pinAfterHistory({ kind: "building", map: 4, index: 1 }, 415)).toEqual({ kind: "header", id: 415 });
    expect(pinAfterHistory({ kind: "building", map: 4, index: 1 }, null)).toEqual({ kind: "none" });
  });

  it("lets go of a pinned event too, as its index may have shifted", () => {
    const event = { kind: "event", eventFile: 1, eventKind: "warp", index: 2 } as const;
    expect(pinAfterHistory(event, 415)).toEqual({ kind: "header", id: 415 });
    expect(pinAfterHistory(event, null)).toEqual({ kind: "none" });
  });

  it("keeps every other pin", () => {
    const area = { kind: "area", id: 3 } as const;
    expect(pinAfterHistory(area, 415)).toBe(area);
    const header = { kind: "header", id: 7 } as const;
    expect(pinAfterHistory(header, 7)).toBe(header);
    const cell = { kind: "matrixCell", matrix: 1, x: 0, y: 0 } as const;
    expect(pinAfterHistory(cell, 7)).toBe(cell);
  });
});

describe("shownInspector", () => {
  const tile: InspectorModel = { kind: "tile", map: 1, x: 2, y: 3, absX: 2, absZ: 3, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } };
  const marker = (index: number, eventKind: "warp" | "object" = "warp", eventFile = 1): InspectorModel => ({
    kind: "eventHover",
    eventFile,
    eventKind,
    index,
    data: { index, x: 1, z: 1, dest_header_id: 0, dest_warp_id: 0, height: 0 },
  });
  const header: InspectorModel = { kind: "header", id: 5 };
  const building: InspectorModel = { kind: "building", map: 1, index: 0 };
  const event: InspectorModel = { kind: "event", eventFile: 1, eventKind: "warp", index: 2 };

  it("shows the pin when nothing is hovered", () => {
    expect(shownInspector(event, null)).toBe(event);
  });

  it("lets the hover win over an ordinary pin", () => {
    expect(shownInspector(header, tile)).toBe(tile);
    expect(shownInspector(header, marker(0))).toEqual(marker(0));
  });

  it("keeps a pinned building over tiles, but not over markers", () => {
    expect(shownInspector(building, tile)).toBe(building);
    expect(shownInspector(building, marker(0))).toEqual(marker(0));
  });

  it("keeps a pinned event over tiles and over its own marker only", () => {
    expect(shownInspector(event, tile)).toBe(event);
    expect(shownInspector(event, marker(2))).toBe(event);
    expect(shownInspector(event, marker(3))).toEqual(marker(3));
    expect(shownInspector(event, marker(2, "object"))).toEqual(marker(2, "object"));
    expect(shownInspector(event, marker(2, "warp", 9))).toEqual(marker(2, "warp", 9));
  });
});
