import { describe, expect, it } from "vitest";
import { NBSP, count } from "./words";

describe("words", () => {
  it("binds_a_count_to_what_it_counts", () => {
    // UI-test fix round 1, M7: the shared helper of every count+unit message (no-break space, U+00A0).
    expect(NBSP).toBe(" ");
    expect(count(1, "Datei", "Dateien")).toBe("1 Datei");
    expect(count(0, "Datei", "Dateien")).toBe("0 Dateien");
    expect(count(12, "Datei", "Dateien")).toBe("12 Dateien");
  });
});
