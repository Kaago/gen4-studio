import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { TYPE_OPTIONS, filterText, lockReason, pocketLabel, rowLabel } from "./dataModel";
import { TARGETS, flagLabel, FLAGS, usesText } from "./moves/moveModel";
import { FIELD_GROUPS, fieldFunctions, sharedNote } from "./items/itemModel";
import { EGG_GROUPS, EVO_METHODS, GENDER, TABS, hatchText, lockText } from "./pokedex/pokedexModel";
import { TIME_COLUMNS, TIME_HOURS, methodTabs, replacedByText, shareText, usedByText, warningText } from "./encounters/encounterModel";
import { AI_LEVELS, TEXT_KINDS, bitLabel, partialTrainer, problemText } from "./trainer/trainerModel";

// The tables of the data areas hold their words as getters and the functions read the catalog when they are called: in English they are
// English, and German again after the switch back.
describe("the data areas in English", () => {
  beforeEach(async () => {
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    await i18next.changeLanguage("de");
  });

  it("names the lists, pockets and locks in English", () => {
    expect(rowLabel("move", { index: 469, name: "move 0469", tech: "0469", locked: null, extra: null }, true)).toBe("Move 0469");
    expect(pocketLabel(1)).toBe("Medicine pocket");
    expect(filterText({ types: [], pocket: 1, category: 2 })).toBe("Medicine pocket, Category Status");
    expect(lockReason("item 0000: 36 bytes, expected 34")).toBe("Data 0000: 36 bytes instead of 34");
    expect(lockReason("trainer 0012: party 20 bytes, expected 16")).toBe("Trainer 0012: party 20 bytes instead of 16");
  });

  it("words the Pokédex tables in English", () => {
    expect(GENDER.map((g) => g.label).slice(0, 2)).toEqual(["male only", "87.5% male"]);
    expect(EVO_METHODS[4]?.label).toBe("Level");
    expect(EVO_METHODS[13]?.label).toBe("Level, Ninjask branch");
    expect(EGG_GROUPS[3]?.label).toBe("Flying");
    expect(TABS.map((t) => t.label)).toEqual(["Values", "Abilities and types", "Learnset", "Egg moves", "Evolution", "TM/HM", "Raw data"]);
    expect(hatchText(20)).toBe("20 cycles, 5,120 steps");
    expect(lockText("personal 0001: 36 bytes, expected 44", { kind: "Personal", index: 1 })).toBe("Pokémon data 0001: 36 bytes instead of 44");
  });

  it("words the move, item, encounter and trainer tables in English", () => {
    expect(TARGETS[1]?.label).toBe("Depends on the move");
    expect(FLAGS[0]?.label).toBe("Makes contact");
    expect(flagLabel("Wing Attack", FLAGS[0]!, true)).toBe("Wing Attack: Makes contact off → on");
    expect(usesText({ learnsets: [{}, {}] as never, trainers: [{}] as never, evolutions: [] })).toBe("2 learnsets, 1 trainer");
    expect(FIELD_GROUPS.map((g) => g.title)).toEqual(["Values", "Effects", "Functions", "Effect on the party"]);
    expect(fieldFunctions("Pt", 4).find((o) => o.value === 4)?.label).toBe("Bicycle");
    expect(sharedNote(21)).toBe("Shares data with 21 items");
    expect(methodTabs("Pt").map((t) => t.label)).toEqual(["Grass", "Surf", "Old Rod", "Good Rod", "Super Rod", "Replacements"]);
    expect(TIME_COLUMNS.map((c) => c.label)).toEqual(["Morning", "Day", "Night"]);
    expect(replacedByText([{ list: "day", entry: 0, when: "from 10 to 19 o'clock", in_file: true }])).toBe("Day 1 (from 10:00 to 19:00)");
    expect(usedByText(3)).toBe("used by 3 places");
    expect(shareText(25)).toBe("25%");
    expect(TYPE_OPTIONS.map((o) => o.value)).toEqual(Array.from({ length: 18 }, (_, i) => i));
    expect(Object.values(TIME_HOURS)).toEqual(["4–9 AM", "10 AM–7 PM", "8 PM–3 AM"]);
    expect(warningText("grass rate 200 acts like 0: the method never runs", (id) => `Species ${id}`)).toContain("Grass rate 200 acts like 0");
    expect(AI_LEVELS.map((l) => l.label)).toEqual(["simple", "smart", "expert", "custom"]);
    expect(TEXT_KINDS.map((k) => k.label)).toEqual(["Before the battle", "Defeat", "After"]);
    expect(bitLabel(2)).toBe("Bit 2: Expert");
    expect(problemText({ message: "trainer 12: the text before the battle does not end with \\r, so its box closes at once" })).toBe(
      "The text before the battle should end with ⏎, or its box closes at once",
    );
    expect(partialTrainer("creating trainer 930: trainer 930 was written; writing the placeholder failed: disk full").text).toBe(
      "Trainer 930 was only partly created: Trainer 930 is written. Failed afterwards: Write the placeholder (disk full).",
    );
  });
});
