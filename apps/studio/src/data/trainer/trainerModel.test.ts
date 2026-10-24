import { describe, expect, it } from "vitest";
import type { DataRowDto, PartyMonDto, ProblemDto } from "../../api";
import {
  AI_BITS,
  AI_LEVELS,
  ABILITY_OPTIONS,
  GENDER_OPTIONS,
  TEXT_KINDS,
  abilityOf,
  aiLevel,
  aiText,
  applyTrainer,
  bitLabel,
  className,
  defaultMoves,
  formCount,
  genderOf,
  groupRows,
  isDouble,
  nameCount,
  nameError,
  newMon,
  nextTrainer,
  partialTrainer,
  problemText,
  TRAINER_SCRIPTS,
  strengthField,
  strengthOf,
  trainerNameArchive,
  trainerRowLabel,
  trainerTitle,
  withBit,
  withDouble,
  withStrength,
} from "./trainerModel";
import { JENS, trainerDoc, trainerRow } from "./trainerTesting";

const mon = (over: Partial<PartyMonDto> = {}): PartyMonDto => ({ ...JENS.party[0]!, ...over });

describe("trainerModel", () => {
  it("has_the_ai_presets_einfach_klug_experte_and_eigene_for_any_other_value", () => {
    expect(AI_LEVELS.map((l) => [l.label, l.ai])).toEqual([
      ["einfach", 0x1],
      ["klug", 0x7],
      ["Experte", 0xf],
      ["eigene", null],
    ]);
    expect(aiLevel(1)).toBe("simple");
    expect(aiLevel(7)).toBe("smart");
    expect(aiLevel(15)).toBe("expert");
    expect(aiLevel(519)).toBe("custom");
    expect(aiLevel(0)).toBe("custom");
    expect(aiText(7)).toBe("klug");
    expect(aiText(519)).toBe("eigene 0x207");
  });

  it("names_eleven_ai_bits_after_dspre_and_keeps_all_32", () => {
    expect(AI_BITS).toHaveLength(11);
    expect(bitLabel(0)).toBe("Bit 0: Grundlagen");
    expect(bitLabel(2)).toBe("Bit 2: Experte");
    expect(bitLabel(10)).toBe("Bit 10: Schikane");
    expect(bitLabel(31)).toBe("Bit 31");
    // Bit 31 as an unsigned value, the others untouched.
    expect(withBit(1, 31, true)).toBe(0x80000001);
    expect(withBit(0x80000207, 9, false)).toBe(0x80000007);
  });

  it("reads_double_battles_from_bit_1_and_keeps_the_other_bits", () => {
    expect(isDouble(0)).toBe(false);
    expect(isDouble(2)).toBe(true);
    expect(withDouble(0, true)).toBe(2);
    expect(withDouble(2, false)).toBe(0);
    expect(withDouble(0x11, true)).toBe(0x13);
  });

  it("reads_gender_and_ability_in_heartgold_from_byte_1", () => {
    expect(GENDER_OPTIONS.map((o) => o.label)).toEqual(["Auto", "männlich", "weiblich"]);
    expect(ABILITY_OPTIONS.map((o) => o.label)).toEqual(["Auto", "Fähigkeit 1", "Fähigkeit 2"]);
    // The DTO's fields first, else the nibbles of byte 1.
    expect(genderOf(mon({ byte1: 0x21, gender: null, ability: null }))).toBe(1);
    expect(abilityOf(mon({ byte1: 0x21, gender: null, ability: null }))).toBe(2);
    expect(genderOf(mon({ byte1: 0x21, gender: 2, ability: 1 }))).toBe(2);
    expect(abilityOf(mon({ byte1: 0x21, gender: 2, ability: 1 }))).toBe(1);
  });

  it("shows_the_strength_of_platinum_as_the_iv_scale_over_both_bytes_and_of_heartgold_as_one_byte", () => {
    expect(strengthField("Pt")).toEqual({ label: "Stärke (IV-Skala)", bits: 16, hint: "IV = Wert × 31 / 255; Byte 1 ist das obere Byte" });
    expect(strengthField("Hgss")).toEqual({ label: "Stärke", bits: 8, hint: "IV = Wert × 31 / 255" });
    expect(strengthOf(mon({ iv_or_difficulty: 50, byte1: 1 }), "Pt")).toBe(306);
    expect(strengthOf(mon({ iv_or_difficulty: 50, byte1: 0x21 }), "Hgss")).toBe(50);
    expect(withStrength(mon({ iv_or_difficulty: 0, byte1: 0 }), "Pt", 306)).toMatchObject({ iv_or_difficulty: 50, byte1: 1 });
    // HeartGold keeps byte 1 (gender and ability).
    expect(withStrength(mon({ iv_or_difficulty: 0, byte1: 0x21 }), "Hgss", 200)).toMatchObject({ iv_or_difficulty: 200, byte1: 0x21 });
  });

  it("names_trainers_by_class_and_name_and_a_nameless_one_by_its_number", () => {
    const classes = ["", "Ass-Trainer", "class 0002"];
    expect(className(classes, 1)).toBe("Ass-Trainer");
    expect(className(classes, 2)).toBe("Klasse 2");
    expect(className(classes, 130)).toBe("Klasse 130");
    expect(trainerTitle("Wanderer", "Jens", 12)).toBe("Wanderer Jens");
    expect(trainerTitle("Wanderer", "", 12)).toBe("Trainer 0012");
    expect(trainerRowLabel(trainerRow(12, "Jens", 6, "Wanderer", 12, [{ header: 5, name: "Route 201" }]))).toBe("Wanderer Jens");
    expect(trainerRowLabel(trainerRow(5, "trainer 0005", 6, "Wanderer", 0, []))).toBe("Trainer 0005");
    const locked: DataRowDto = { index: 929, name: "", tech: "0929", locked: "placeholder for the sight script slot", extra: null };
    expect(trainerRowLabel(locked)).toBe("Trainer 0929");
  });

  it("allows_seven_characters_for_a_name", () => {
    expect(nameError("Jensine")).toBeNull();
    expect(nameError("Jensinea")).toBe("Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch");
    expect(nameCount("Jens")).toBe("4 von 7");
  });

  it("groups_rows_by_place_with_ohne_ort_last_or_by_class", () => {
    const rows = [
      trainerRow(12, "Jens", 6, "Wanderer", 12, [{ header: 5, name: "Route 201" }]),
      trainerRow(13, "Mia", 7, "Göre", 9, [{ header: 5, name: "Route 201" }, { header: 9, name: "Zweiblattdorf" }]),
      trainerRow(40, "Kai", 6, "Wanderer", 20, []),
    ];
    const byPlace = groupRows(rows, "place");
    expect(byPlace.map((g) => [g.label, g.rows.map((r) => r.index)])).toEqual([
      ["Route 201", [12, 13]],
      ["Zweiblattdorf", [13]],
      ["Ohne Ort", [40]],
    ]);
    const byClass = groupRows(rows, "class");
    expect(byClass.map((g) => [g.label, g.rows.map((r) => r.index)])).toEqual([
      ["Göre", [13]],
      ["Wanderer", [12, 40]],
    ]);
  });

  it("fills_moves_by_the_game_rule_and_makes_a_new_pokemon_in_the_format", () => {
    const learnset = [
      { move_id: 33, level: 1 },
      { move_id: 45, level: 1 },
      { move_id: 98, level: 5 },
      { move_id: 17, level: 9 },
      { move_id: 33, level: 10 },
      { move_id: 332, level: 13 },
    ];
    expect(defaultMoves(learnset, 4)).toEqual([33, 45, 0, 0]);
    expect(defaultMoves(learnset, 12)).toEqual([33, 45, 98, 17]);
    // The fifth move pushes the first one out.
    expect(defaultMoves(learnset, 13)).toEqual([45, 98, 17, 332]);
    expect(newMon(396, 5, { moves: true, items: true }, "Pt", [33, 45, 0, 0])).toEqual({
      iv_or_difficulty: 0,
      byte1: 0,
      level: 5,
      species: 396,
      form: 0,
      item: 0,
      moves: [33, 45, 0, 0],
      ball_capsule: 0,
      gender: null,
      ability: null,
    });
    expect(newMon(396, 5, { moves: false, items: false }, "Hgss")).toMatchObject({ item: null, moves: null, gender: 0, ability: 0 });
  });

  it("knows_the_species_with_forms", () => {
    expect(formCount(201)).toBe(28);
    expect(formCount(479)).toBe(6);
    expect(formCount(396)).toBe(1);
  });

  it("says_the_kernel_warning_in_german_with_a_verb", () => {
    const row: ProblemDto = { severity: "warning", file: "trainers/0012", line: 1, col: 1, message: "trainer 12: the text before the battle does not end with \\r, so its box closes at once", source: "trainer" };
    expect(problemText(row)).toBe("Der Text vor dem Kampf sollte auf ⏎ enden, sonst schließt sein Kasten sofort");
    expect(problemText({ ...row, message: "something else" })).toBe("something else");
    expect(TEXT_KINDS.map((k) => k.label)).toEqual(["Vor dem Kampf", "Niederlage", "Danach"]);
  });

  it("knows_the_next_id_and_skips_the_seen_slot_of_the_battle_script", () => {
    const rows = (n: number) => Array.from({ length: n }, (_, i) => trainerRow(i, "", 0, "x", 0, []));
    expect(nextTrainer(rows(928), "Pt")).toEqual({ id: 928, placeholder: null });
    expect(nextTrainer(rows(929), "Pt")).toEqual({ id: 930, placeholder: 929 });
    expect(nextTrainer(rows(931), "Pt")).toEqual({ id: 931, placeholder: null });
    expect(nextTrainer(rows(740), "Hgss")).toEqual({ id: 741, placeholder: 740 });
    expect(TRAINER_SCRIPTS.Pt).toEqual({ battleFile: 1114, seenSlot: 929 });
    expect(TRAINER_SCRIPTS.Hgss).toEqual({ battleFile: 953, seenSlot: 740 });
  });

  it("words_a_partial_creation_in_german_and_names_what_stays", () => {
    const message = "creating trainer 930: battle script file 1114 has the slots up to trainer 930, trainer 929 was written, trainer 930 was written; setting its name and text offset failed: disk full";
    expect(partialTrainer(message)).toEqual({
      text: "Trainer 930 nur zum Teil angelegt: Kampf-Script 1114 hat die Plätze bis Trainer 930, Trainer 929 und 930 sind geschrieben. Danach fehlgeschlagen: Name und Text-Offset setzen (disk full).",
      written: [929, 930],
      extended: true,
    });
    expect(partialTrainer("creating trainer 930: battle script file 1114 has the slots up to trainer 930; writing the placeholder failed: x")).toEqual({
      text: "Trainer 930 nur zum Teil angelegt: Kampf-Script 1114 hat die Plätze bis Trainer 930. Danach fehlgeschlagen: Platzhalter schreiben (x).",
      written: [],
      extended: true,
    });
    expect(partialTrainer("something else")).toEqual({ text: "something else", written: [], extended: false });
  });

  it("knows_the_trainer_name_bank_of_the_western_games", () => {
    expect(trainerNameArchive({ root: "/pt", game_code: "CPUD", family: "Pt", language: "de", header_count: 3 })).toBe(618);
    expect(trainerNameArchive({ root: "/hg", game_code: "IPKD", family: "Hgss", language: "de", header_count: 3 })).toBe(729);
    expect(trainerNameArchive({ root: "/hg", game_code: "IPKJ", family: "Hgss", language: "ja", header_count: 3 })).toBeNull();
  });

  it("lays_patches_over_the_document", () => {
    const doc = trainerDoc();
    expect(applyTrainer(doc, { Trainer: { field: "ai", value: 7 } }).ai).toBe(7);
    expect(applyTrainer(doc, { Trainer: { field: "item2", value: 17 } }).items).toEqual([0, 0, 17, 0]);
    expect(applyTrainer(doc, { TrainerName: { name: "Jensi" } }).name).toBe("Jensi");
    expect(applyTrainer(doc, { TrainerParty: { party: [] } }).party).toEqual([]);
    const off = applyTrainer({ ...doc, format: { moves: true, items: true }, party: [mon({ moves: [1, 2, 3, 4], item: 17 })] }, { TrainerFormat: { moves: false, items: null } });
    expect(off.format).toEqual({ moves: false, items: true });
    expect(off.party[0]?.moves).toBeNull();
    expect(off.party[0]?.item).toBe(17);
    const texts = applyTrainer(doc, { TrainerTexts: { texts: [{ kind: 1, text: "Mist!" }] } }).texts;
    expect(texts.find((t) => t.kind === 1)?.text).toBe("Mist!");
  });
});
