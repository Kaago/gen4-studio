import { describe, expect, it } from "vitest";
import { FLAGS, flagEntry, flagList } from "../flags/flagTesting";
import {
  fallbackNumber,
  flagIdOf,
  flagNumberOf,
  freeVarsText,
  placesText,
  progressLabel,
  recipeTitle,
  stepKindWord,
  storiesCount,
  threadName,
  useText,
  valuesText,
} from "./storyWords";
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

  it("ids_and_numbers_of_flags_and_variables", () => {
    // The id a story keeps: the own name, the pret name, else the fallback id (four digits at least, decimal).
    expect(flagIdOf("var", 0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN" })).toBe("VAR_RIVALIN");
    expect(flagIdOf("var", 0x406c, { name: "VAR_UNUSED_0x406C", own_id: null })).toBe("VAR_UNUSED_0x406C");
    expect(flagIdOf("var", 0x4071, null)).toBe("VAR_16497");
    expect(flagIdOf("flag", 0x94, undefined)).toBe("FLAG_0148");
    // And back: by own name, pret name and alias of the list, else a fallback id; null where neither knows it.
    expect(flagNumberOf("flag", "FLAG_HAT_MEDAILLE", FLAGS)).toBe(0x94);
    expect(flagNumberOf("flag", "FLAG_UNUSED_0x0094", FLAGS)).toBe(0x94);
    expect(flagNumberOf("flag", "FLAG_ALT_0x96", FLAGS)).toBe(0x96);
    expect(flagNumberOf("flag", "FLAG_0371", null)).toBe(0x173);
    expect(flagNumberOf("var", "VAR_16448", flagList("var", [flagEntry(0x4031)]))).toBe(0x4040);
    expect(flagNumberOf("flag", "VAR_16448", null)).toBeNull();
    expect(flagNumberOf("var", "VAR_WEG", null)).toBeNull();
    expect(fallbackNumber("TRAINER", "TRAINER_0042")).toBe(42);
    expect(fallbackNumber("TRAINER", "TRAINER_CLASS_0042")).toBeNull();
    expect(fallbackNumber("TRAINER", "TRAINER_")).toBeNull();
  });

  it("uses_and_counts", () => {
    expect(useText({ trainer: "TRAINER_0042" })).toBe("TRAINER_0042");
    expect(useText({ map: "MAP_ZWEIBLATTDORF" })).toBe("MAP_ZWEIBLATTDORF");
    expect(useText({ script: { file: 101, slot: 2 } })).toBe("Scriptdatei 0101 · Script 2");
    expect(useText({ message: { archive: 12, index: 4 } })).toBe("Textarchiv 0012 · Text 4");
    expect(useText({ levelScript: { file: 12, condition: 1 } })).toBe("Level-Script 0012 · Bedingung 1");
    expect(useText({ object: { file: 394, local_id: 3 } })).toBe("Objekt 3 · Eventdatei 0394");
    expect(storiesCount(0)).toBe("Keine Geschichten");
    expect(storiesCount(1)).toBe("1 Geschichte");
    expect(storiesCount(1200)).toBe("1.200 Geschichten");
  });

  it("recipe_titles_and_the_pool", () => {
    expect(recipeTitle("new_story")).toBe("Neue Geschichte");
    expect(recipeTitle("talk_advances")).toBe("Gespräch schaltet weiter");
    expect(recipeTitle("new_figure")).toBe("Neue Figur");
    // A recipe of a newer studio as it is; nothing of Object's own.
    expect(recipeTitle("cutscene")).toBe("cutscene");
    expect(recipeTitle("toString")).toBe("toString");
    expect(freeVarsText(20)).toBe("20 Variablen frei");
    expect(freeVarsText(1)).toBe("1 Variable frei");
    expect(freeVarsText(0)).toBe("Keine Variable frei");
  });
});
