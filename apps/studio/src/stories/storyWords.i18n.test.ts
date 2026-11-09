import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { threadOf } from "./storiesTesting";
import { freeVarsText, placesText, recipeTitle, storiesCount, useText, valuesText } from "./storyWords";
import { threadsCount } from "./ThreadsSurface";

describe("story words in English", () => {
  beforeEach(async () => {
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    await i18next.changeLanguage("de");
  });

  it("counts with the English plural, the zero form and the thousands separator", () => {
    expect([0, 1, 2, 1500].map(storiesCount)).toEqual(["No stories", "1 story", "2 stories", "1,500 stories"]);
    expect([0, 1, 20].map(threadsCount)).toEqual(["No threads", "1 thread", "20 threads"]);
    expect([0, 1, 20].map(freeVarsText)).toEqual(["No free variables", "1 free variable", "20 free variables"]);
  });

  it("names values and places in English", () => {
    expect(valuesText([])).toBe("No values");
    expect(valuesText([5])).toBe("1 value: 5");
    expect(valuesText([0, 1, 2])).toBe("3 values: 0, 1, 2");
    expect(placesText(threadOf(0x4031))).toBe("3 places: Zweiblattdorf, Route 201, …");
  });

  it("names where a step's use is in English", () => {
    expect(useText({ script: { file: 101, slot: 2 } })).toBe("Script file 0101 · Script 2");
    expect(useText({ message: { archive: 12, index: 4 } })).toBe("Text archive 0012 · Text 4");
    expect(useText({ levelScript: { file: 5, condition: 1 } })).toBe("Level script 0005 · Condition 1");
    expect(useText({ object: { local_id: 3, file: 101 } })).toBe("Object 3 · Event file 0101");
  });

  it("titles the recipes in English and leaves an unknown one as it is", () => {
    expect(recipeTitle("talk_advances")).toBe("Conversation advances");
    expect(recipeTitle("a_newer_recipe")).toBe("a_newer_recipe");
  });
});
