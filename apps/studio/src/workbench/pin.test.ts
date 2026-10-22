import { describe, expect, it } from "vitest";
import { pinAfterHistory } from "./pin";

describe("pinAfterHistory", () => {
  it("lets go of a pinned building, whatever its index", () => {
    expect(pinAfterHistory({ kind: "building", map: 4, index: 1 }, 415)).toEqual({ kind: "header", id: 415 });
    expect(pinAfterHistory({ kind: "building", map: 4, index: 1 }, null)).toEqual({ kind: "none" });
  });

  it("keeps every other pin", () => {
    const area = { kind: "area", id: 3 } as const;
    expect(pinAfterHistory(area, 415)).toBe(area);
    const header = { kind: "header", id: 7 } as const;
    expect(pinAfterHistory(header, 7)).toBe(header);
  });
});
