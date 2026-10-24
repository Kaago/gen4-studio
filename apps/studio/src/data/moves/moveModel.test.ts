import { describe, expect, it } from "vitest";
import type { RefDto } from "../../api";
import { CATEGORIES, FLAGS, NAMED_FLAGS, TARGETS, categoryName, flagLabel, lockText, moveLabel, moveUses, moveValues, ppMax, priorityText, targetName, usesText, withFlag } from "./moveModel";

describe("moveModel", () => {
  it("names_the_three_categories_and_keeps_an_unknown_one_as_a_value", () => {
    expect(CATEGORIES).toEqual({ 0: "Physisch", 1: "Speziell", 2: "Status" });
    expect(categoryName(3)).toBe("Wert 3");
  });

  it("names_the_targets_of_dspre_1_5_as_zero_and_one_bit_each", () => {
    expect(TARGETS.map((t) => t.value)).toEqual([0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024]);
    expect(targetName(0)).toBe("Ein gewähltes Pokémon");
    expect(targetName(4)).toBe("Beide Gegner");
    expect(targetName(3)).toBe("Wert 3");
  });

  it("names_six_flag_bits_and_leaves_bits_6_and_7_to_tinkering", () => {
    expect(NAMED_FLAGS.map((f) => [f.mask, f.label])).toEqual([
      [0x01, "Macht Kontakt"],
      [0x02, "Schutzschild wirkt"],
      [0x04, "Magiemantel wirkt"],
      [0x08, "Übernahme wirkt"],
      [0x10, "Spiegeltrick wirkt"],
      [0x20, "King-Stein wirkt"],
    ]);
    expect(FLAGS.filter((f) => !f.named).map((f) => f.mask)).toEqual([0x40, 0x80]);
  });

  it("sets_and_clears_one_flag_bit_and_keeps_the_unknown_ones", () => {
    expect(withFlag(0x40, 0x01, true)).toBe(0x41);
    expect(withFlag(0x73, 0x01, false)).toBe(0x72);
    expect(withFlag(0xc0, 0x20, true)).toBe(0xe0);
  });

  it("labels_an_edit_with_the_old_and_new_value_in_words", () => {
    expect(moveLabel("Flügelschlag", "power", 60, 65)).toBe("Flügelschlag: Stärke 60 → 65");
    expect(moveLabel("Flügelschlag", "move_type", 2, 10)).toBe("Flügelschlag: Typ Flug → Feuer");
    expect(moveLabel("Flügelschlag", "category", 0, 1)).toBe("Flügelschlag: Kategorie Physisch → Speziell");
    expect(moveLabel("Flügelschlag", "priority", 0, -1)).toBe("Flügelschlag: Priorität 0 → \u22121");
    expect(moveLabel("Flügelschlag", "priority", -1, 1)).toBe("Flügelschlag: Priorität \u22121 → +1");
    expect(flagLabel("Flügelschlag", FLAGS[0]!, false)).toBe("Flügelschlag: Macht Kontakt an → aus");
  });

  it("words_numbers_of_the_card", () => {
    expect(ppMax(35)).toBe(56);
    expect(priorityText(0)).toBe("Priorität 0");
    expect(priorityText(1)).toBe("Priorität +1");
    expect(priorityText(-7)).toBe("Priorität −7");
  });

  it("says_a_lock_in_german", () => {
    expect(lockText("move 0017: 18 bytes, expected 16", 17)).toBe("Attacke 0017: 18 Bytes statt 16");
  });

  it("reads_the_values_by_field", () => {
    expect(moveValues([{ name: "power", value: 60, bits: 8, signed: false }])).toEqual({ power: 60 });
  });

  it("counts_learnsets_and_trainers_once_per_member", () => {
    const ref = (kind: "Learnset" | "TrainerParty" | "Evolution", index: number): RefDto => ({ source: { kind, index }, field: "move", slot: 0, label: "", nav: { kind: "species", index } });
    const uses = moveUses([ref("Learnset", 396), ref("Learnset", 397), ref("TrainerParty", 4), ref("TrainerParty", 4), ref("Evolution", 1)]);
    expect(usesText(uses)).toBe("2 Lernsets, 1 Trainer");
    expect(uses.evolutions).toHaveLength(1);
  });
});
