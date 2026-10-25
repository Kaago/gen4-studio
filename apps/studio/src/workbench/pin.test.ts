import { describe, expect, it } from "vitest";
import { pinAfterHistory } from "./pin";

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
