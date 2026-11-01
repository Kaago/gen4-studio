import { describe, expect, it } from "vitest";
import { placesText, progressLabel, stepKindWord, threadName, valuesText } from "./storyWords";
import { threadOf } from "./storiesTesting";

describe("storyWords", () => {
  it("values_and_places_texts", () => {
    expect(valuesText([0, 1, 2])).toBe("3 Werte: 0, 1, 2");
    expect(valuesText([5])).toBe("1 Wert: 5");
    expect(valuesText([])).toBe("Keine Werte");
    expect(valuesText([0, 1, 2, 3, 4, 5, 6, 7])).toBe("8 Werte: 0, 1, 2, 3, 4, 5, 6, 7");
    // From nine values the first and the last.
    expect(valuesText(Array.from({ length: 16 }, (_, i) => i))).toBe("16 Werte: 0 … 15");
    // Three places, two of them with a header: the names, then „…“ for the rest.
    expect(placesText(threadOf(0x4031))).toBe("3 Orte: Zweiblattdorf, Route 201, …");
    // As many places as names: no rest.
    expect(placesText(threadOf(0x4031, { places: 2 }))).toBe("2 Orte: Zweiblattdorf, Route 201");
    // At most three names, then how many more.
    expect(placesText(threadOf(0x4031, { places: 7, headers: [1, 2, 3, 4, 5], header_names: ["Herzhofen", "Route 205", "Flori", "Ewigenau", "Route 210"] }))).toBe(
      "7 Orte: Herzhofen, Route 205, Flori + 4",
    );
    // A name two headers share is named once.
    expect(placesText(threadOf(0x4031, { places: 2, headers: [3, 4], header_names: ["Flori", "Flori"] }))).toBe("2 Orte: Flori, …");
    // No header among the places: the count alone.
    expect(placesText(threadOf(0x4031, { places: 2, headers: [], header_names: [] }))).toBe("2 Orte");
    expect(placesText(threadOf(0x4031, { places: 1, headers: [9], header_names: ["Sandgemme"] }))).toBe("1 Ort: Sandgemme");
  });

  it("thread_name_prefers_the_own_id", () => {
    expect(threadName(threadOf(0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN" }))).toBe("VAR_RIVALIN");
    expect(threadName(threadOf(0x4040, { name: "VAR_UNK_0x4040" }))).toBe("VAR_UNK_0x4040");
    expect(threadName(threadOf(0x4030))).toBe("Variable 0x4030");
  });

  it("step_kinds_and_progress", () => {
    expect(stepKindWord("battle")).toBe("Kampf");
    expect(stepKindWord("appearance")).toBe("Erscheinen/Verschwinden");
    expect(progressLabel({ var: "VAR_RIVALIN" })).toEqual({ kind: "var", id: "VAR_RIVALIN" });
    expect(progressLabel({ flag: "FLAG_HAT_MEDAILLE" })).toEqual({ kind: "flag", id: "FLAG_HAT_MEDAILLE" });
  });
});
