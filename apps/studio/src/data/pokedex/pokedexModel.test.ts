import { describe, expect, it } from "vitest";
import type { DataRowDto } from "../../api";
import {
  EGG_GROUPS,
  EVO_METHODS,
  EV_YIELD,
  GENDER,
  GROWTH_RATES,
  evYieldFromWord,
  evYieldWord,
  genderText,
  hatchText,
  lockText,
  machineLabel,
  paramKind,
  sortLearnset,
  speciesCount,
  statAverages,
  statSum,
} from "./pokedexModel";

const row = (index: number, stats: [number, number, number, number, number, number] | null): DataRowDto => ({
  index,
  name: `Art ${index}`,
  tech: String(index).padStart(4, "0"),
  locked: stats === null ? "personal 0001: 36 bytes, expected 44" : null,
  extra: stats === null ? null : { types: [0, 0], number: index, stats },
});

describe("pokedexModel", () => {
  it("averages_the_stats_of_every_species_without_member_0_eggs_forms_and_locked_rows", () => {
    const rows = [
      row(0, [99, 99, 99, 99, 99, 99]),
      row(1, [45, 49, 49, 45, 65, 65]),
      row(397, [55, 75, 50, 80, 40, 40]),
      row(494, [10, 10, 10, 10, 10, 10]),
      row(503, [50, 65, 107, 86, 105, 107]),
      row(507, [50, 65, 107, 86, 105, 107]),
      row(398, null),
    ];
    // Row stats are HP, Attack, Defense, Speed, Special Attack, Special Defense.
    expect(statAverages(rows)).toEqual({ hp: 50, attack: 62, defense: 49.5, speed: 62.5, sp_attack: 52.5, sp_defense: 52.5 });
    expect(statAverages([])).toEqual({ hp: 0, attack: 0, defense: 0, speed: 0, sp_attack: 0, sp_defense: 0 });
  });

  it("sums_the_six_base_stats", () => {
    expect(statSum({ hp: 55, attack: 85, defense: 50, speed: 80, sp_attack: 40, sp_defense: 40 })).toBe(350);
  });

  it("counts_species_and_forms_without_member_0_and_the_eggs", () => {
    const rows = Array.from({ length: 508 }, (_, i) => row(i, [1, 1, 1, 1, 1, 1]));
    expect(speciesCount(rows)).toBe("493 Arten, 12 Formen");
    expect(speciesCount(rows.slice(0, 2))).toBe("1 Art");
    expect(speciesCount([...rows.slice(0, 2), row(500, [1, 1, 1, 1, 1, 1])])).toBe("1 Art, 1 Form");
  });

  it("maps_the_ev_yield_to_its_two_bit_fields_and_back", () => {
    expect(EV_YIELD.map((e) => [e.field, e.mask])).toEqual([
      ["ev_yield_hp", 0x0003],
      ["ev_yield_attack", 0x000c],
      ["ev_yield_defense", 0x0030],
      ["ev_yield_speed", 0x00c0],
      ["ev_yield_sp_attack", 0x0300],
      ["ev_yield_sp_defense", 0x0c00],
    ]);
    // Staravia: 2 Attack.
    expect(evYieldFromWord(0x0008)).toEqual([0, 2, 0, 0, 0, 0]);
    expect(evYieldWord([0, 2, 0, 0, 0, 0])).toBe(0x0008);
    expect(evYieldFromWord(0x0fff)).toEqual([3, 3, 3, 3, 3, 3]);
    // Bits 12 to 15 are not part of the yield; a value beyond two bits keeps its two bits.
    expect(evYieldFromWord(0xf004)).toEqual([0, 1, 0, 0, 0, 0]);
    expect(evYieldWord([1, 0, 0, 0, 0, 7])).toBe(0x0c01);
  });

  it("words_the_gender_steps", () => {
    expect(Object.fromEntries(GENDER.map((g) => [g.value, g.label]))).toEqual({
      0: "nur männlich",
      31: "87,5 % männlich",
      63: "75 % männlich",
      127: "50 % männlich",
      191: "75 % weiblich",
      225: "87,5 % weiblich",
      254: "nur weiblich",
      255: "geschlechtslos",
    });
    expect(genderText(127)).toBe("50 % männlich");
    expect(genderText(100)).toBe("Wert 100");
  });

  it("counts_the_hatch_steps_as_cycles_times_256", () => {
    expect(hatchText(15)).toBe("15 Zyklen, 3.840 Schritte");
    expect(hatchText(1)).toBe("1 Zyklus, 256 Schritte");
    expect(hatchText(120)).toBe("120 Zyklen, 30.720 Schritte");
  });

  it("names_six_growth_rates_and_fifteen_egg_groups", () => {
    expect(GROWTH_RATES.map((g) => g.label)).toEqual(["Mittelschnell", "Unregelmäßig", "Schwankend", "Mittellangsam", "Schnell", "Langsam"]);
    expect(GROWTH_RATES.map((g) => g.value)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(EGG_GROUPS).toHaveLength(15);
    expect(EGG_GROUPS[0]).toEqual({ value: 1, label: "Monster" });
    expect(EGG_GROUPS[3]).toEqual({ value: 4, label: "Flug" });
    expect(EGG_GROUPS[14]).toEqual({ value: 15, label: "Unbekannt" });
  });

  it("knows_the_kind_of_parameter_of_every_evolution_method", () => {
    expect(EVO_METHODS.map((m) => m.value)).toEqual(Array.from({ length: 27 }, (_, i) => i));
    const kinds = Object.fromEntries(EVO_METHODS.map((m) => [m.value, paramKind(m.value)]));
    for (const level of [4, 8, 9, 10, 11, 12, 13, 14, 22, 23]) expect(kinds[level]).toBe("level");
    for (const item of [6, 7, 16, 17, 18, 19]) expect(kinds[item]).toBe("item");
    expect(kinds[20]).toBe("move");
    expect(kinds[21]).toBe("species");
    expect(kinds[15]).toBe("beauty");
    for (const none of [0, 1, 2, 3, 5, 24, 25, 26]) expect(kinds[none]).toBe("none");
    // A method the games do not know keeps its parameter as a plain number.
    expect(paramKind(40)).toBe("number");
    expect(EVO_METHODS[4]?.label).toBe("Level");
    expect(EVO_METHODS[0]?.label).toBe("Keine");
  });

  it("words_a_lock_of_a_species_part_in_german", () => {
    expect(lockText("personal 0397: 36 bytes, expected 44", { kind: "Personal", index: 397 })).toBe("Pokémon-Daten 0397: 36 Bytes statt 44");
    expect(lockText("learnset 0012: 7 bytes, expected 44", { kind: "Learnset", index: 12 })).toBe("Lernset 0012: 7 Bytes statt 44");
    expect(lockText("evolution 0001: 40 bytes, expected 44", { kind: "Evolution", index: 1 })).toBe("Entwicklungen 0001: 40 Bytes statt 44");
    // A reason in another shape stays as the kernel says it, after the part.
    expect(lockText("something else", { kind: "Personal", index: 2 })).toBe("Pokémon-Daten 0002: something else");
  });

  it("names_the_machines_tm01_to_tm92_then_vm01_to_vm08", () => {
    expect(machineLabel(0)).toBe("TM01");
    expect(machineLabel(39)).toBe("TM40");
    expect(machineLabel(91)).toBe("TM92");
    expect(machineLabel(92)).toBe("VM01");
    expect(machineLabel(99)).toBe("VM08");
  });

  it("sorts_a_learnset_by_level_and_keeps_the_order_of_equal_levels", () => {
    const entries = [
      { move_id: 17, level: 9 },
      { move_id: 33, level: 1 },
      { move_id: 45, level: 1 },
      { move_id: 98, level: 5 },
    ];
    expect(sortLearnset(entries)).toEqual([
      { move_id: 33, level: 1 },
      { move_id: 45, level: 1 },
      { move_id: 98, level: 5 },
      { move_id: 17, level: 9 },
    ]);
  });
});
