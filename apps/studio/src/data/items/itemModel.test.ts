import { describe, expect, it } from "vitest";
import type { RefDto } from "../../api";
import {
  BATTLE_FUNCTIONS,
  battleFunctions,
  BATTLE_POCKETS,
  FIELD_FUNCTIONS,
  FIELD_GROUPS,
  FIELD_POCKETS,
  ITEM_FIELDS,
  ONE_BIT_FIELDS,
  PARTY_FIELDS,
  fieldFunctions,
  itemLabel,
  itemText,
  itemUses,
  lockText,
  pocketBitLabel,
  sellPrice,
  sharedNote,
  sharedWarning,
  usableInBattle,
  usesText,
  withBit,
} from "./itemModel";

describe("itemModel", () => {
  it("names_the_pockets_as_dspre_1_6_has_them", () => {
    // Field pocket: bits 7 to 10, numbers 0 to 7 (pokeplatinum POCKET_*).
    expect(FIELD_POCKETS.map((p) => [p.value, p.label])).toEqual([
      [0, "Items"],
      [1, "Medizin"],
      [2, "Bälle"],
      [3, "TM/VM"],
      [4, "Beeren"],
      [5, "Briefe"],
      [6, "Kampf-Items"],
      [7, "Basis-Items"],
    ]);
    // Battle pocket: bits 11 to 15, one bit per pocket of the battle bag.
    expect(BATTLE_POCKETS.map((p) => [p.mask, p.label])).toEqual([
      [0x01, "Pokébälle"],
      [0x02, "Kampf-Items"],
      [0x04, "KP heilen"],
      [0x08, "Statusprobleme heilen"],
      [0x10, "AP heilen"],
    ]);
  });

  it("lists_the_fields_in_the_order_of_dspre_1_6", () => {
    expect(ITEM_FIELDS.slice(0, 15)).toEqual([
      "price",
      "hold_effect",
      "hold_param",
      "pluck_effect",
      "fling_effect",
      "fling_power",
      "natural_gift_power",
      "natural_gift_type",
      "prevent_toss",
      "can_register",
      "field_pocket",
      "battle_pocket",
      "field_use_func",
      "battle_use_func",
      "party_use",
    ]);
    // The party parameters: 7 heals, guard spec, revive, revive all, level up, evolve, 7 stages, PP up and max, three restores,
    // 6 EV flags, 3 friendship flags, then 6 EVs, HP and PP amounts, 3 friendship values.
    expect(PARTY_FIELDS).toHaveLength(7 + 5 + 7 + 2 + 3 + 6 + 3 + 6 + 2 + 3);
    expect(PARTY_FIELDS[0]).toBe("slp_heal");
    expect(PARTY_FIELDS[PARTY_FIELDS.length - 1]).toBe("friendship_high");
  });

  it("groups_every_field_once_into_werte_effekte_funktionen_and_the_party_parameters_only_while_tinkering", () => {
    expect(FIELD_GROUPS.map((g) => [g.title, g.tinkerOnly])).toEqual([
      ["Werte", false],
      ["Effekte", false],
      ["Funktionen", false],
      ["Wirkung im Team", true],
    ]);
    const grouped = FIELD_GROUPS.flatMap((g) => g.fields);
    expect([...grouped].sort()).toEqual([...ITEM_FIELDS].sort());
    expect(new Set(grouped).size).toBe(grouped.length);
    expect(FIELD_GROUPS[0]!.fields).toEqual(["price", "field_pocket", "battle_pocket", "prevent_toss", "can_register"]);
    expect(FIELD_GROUPS[3]!.fields).toEqual(PARTY_FIELDS);
    expect(ONE_BIT_FIELDS.every((f) => ITEM_FIELDS.includes(f))).toBe(true);
  });

  it("sets_one_battle_pocket_bit_and_keeps_the_others", () => {
    expect(withBit(0x04, 0x01, true)).toBe(0x05);
    expect(withBit(0x05, 0x04, false)).toBe(0x01);
    expect(withBit(0x1f, 0x10, true)).toBe(0x1f);
  });

  it("words_values_and_labels", () => {
    expect(itemText("field_pocket", 1)).toBe("Medizin");
    expect(itemText("field_pocket", 9)).toBe("Wert 9");
    expect(itemText("battle_pocket", 0x05)).toBe("Pokébälle, KP heilen");
    expect(itemText("battle_pocket", 0)).toBe("keine");
    expect(itemText("natural_gift_type", 31)).toBe("Keiner");
    expect(itemText("natural_gift_type", 10)).toBe("Feuer");
    expect(itemText("field_use_func", 1)).toBe("Heilmittel");
    expect(itemText("battle_use_func", 2)).toBe("Heilmittel");
    expect(itemText("battle_use_func", 0)).toBe("Keine eigene Funktion");
    expect(itemText("prevent_toss", 1)).toBe("an");
    expect(itemText("hp_amount", 20)).toBe("20");
    expect(itemLabel("Trank", "price", 300, 250)).toBe("Trank: Preis 300 → 250");
    expect(itemLabel("Trank", "price", 300, 250, { shared: 21 })).toBe("Trank: Preis 300 → 250, gilt für 22 Gegenstände");
    expect(itemLabel("Trank", "field_pocket", 1, 2)).toBe("Trank: Tasche Medizin → Bälle");
    expect(itemLabel("Trank", "prevent_toss", 0, 1)).toBe("Trank: Nicht wegwerfbar aus → an");
    expect(pocketBitLabel("Trank", BATTLE_POCKETS[2]!, false)).toBe("Trank: Kampftasche KP heilen an → aus");
    expect(sellPrice(300)).toBe(150);
    expect(sellPrice(25)).toBe(12);
  });

  it("says_how_many_items_share_the_data", () => {
    expect(sharedWarning(21)).toBe("Preis und Werte gelten auch für 21 andere Gegenstände");
    expect(sharedWarning(1)).toBe("Preis und Werte gelten auch für 1 anderen Gegenstand");
    expect(sharedNote(21)).toBe("Teilt Daten mit 21 Gegenständen");
  });

  it("puts_the_lock_reason_into_german", () => {
    // The kernel names the data member, which several item ids may share.
    expect(lockText("item 0000: 36 bytes, expected 34")).toBe("Daten 0000: 36 Bytes statt 34");
    expect(lockText("odd")).toBe("odd");
  });

  it("counts_the_uses_by_kind_each_source_once", () => {
    const ref = (kind: RefDto["source"]["kind"], index: number, slot: number | null = null): RefDto => ({ source: { kind, index }, field: "item", slot, label: `${kind} ${index}`, source_name: `${kind} ${index}`, nav: { kind: "trainer", index } });
    const uses = itemUses([ref("TrainerProperties", 5, 0), ref("TrainerProperties", 5, 1), ref("TrainerParty", 7, 2), ref("Personal", 25), ref("Evolution", 133, 0)]);
    expect(uses.trainers.map((r) => r.source.index)).toEqual([5]);
    expect(uses.parties.map((r) => r.source.index)).toEqual([7]);
    expect(uses.wild.map((r) => r.source.index)).toEqual([25]);
    expect(uses.evolutions.map((r) => r.source.index)).toEqual([133]);
    expect(usesText(uses)).toBe("1 Trainer-Item, 1 Team-Item, 1 wild getragen, 1 Entwicklung");
  });

  it("names_the_field_functions_per_family_as_the_vanilla_items_use_them", () => {
    // Platinum DE: every value 0 to 24 is used, e.g. 2 Karte (member 420), 3 Forschersack (406), 13 the four mulches (95 to 98),
    // 22 Azurflöte (433), 23 Kampfkamera (443), 24 Gracidea (444).
    const pt = Object.fromEntries(FIELD_FUNCTIONS.Pt.map((o) => [o.value, o.label]));
    expect(Object.keys(pt).map(Number)).toEqual(Array.from({ length: 25 }, (_, i) => i));
    expect([pt[0], pt[1], pt[2], pt[3], pt[5], pt[9], pt[10], pt[11], pt[12], pt[13], pt[15], pt[19], pt[22], pt[23], pt[24]]).toEqual([
      "Keine",
      "Heilmittel",
      "Karte",
      "Forschersack",
      "Tagebuch",
      "Knurspbox",
      "Adressbuch",
      "Pokéradar",
      "Entonkanne",
      "Mulch",
      "Kampffahnder",
      "Schutz, Flöten, Koffer und Karten",
      "Azurflöte",
      "Kampfkamera",
      "Gracidea",
    ]);
    // HeartGold DE: no item uses 3 or 13; 23 to 29 are its own (Aprikokobox 445, Pflanzset 447, Icognitoheft 446, Itemradar 448,
    // GB-Player 479, Gracidea 443, Kampfkamera 442).
    const hg = Object.fromEntries(FIELD_FUNCTIONS.Hgss.map((o) => [o.value, o.label]));
    expect(hg[3]).toBeUndefined();
    expect(hg[13]).toBeUndefined();
    expect([hg[2], hg[5], hg[9], hg[11], hg[12], hg[15]]).toEqual(["Karte", "Tagebuch", "Knurspbox", "Pokéradar", "Entonkanne", "Kampffahnder"]);
    expect([23, 24, 25, 26, 27, 28, 29].map((v) => hg[v])).toEqual(["Aprikokobox", "Pflanzset", "Icognitoheft", "Itemradar", "GB-Player", "Gracidea", "Kampfkamera"]);
    expect(Object.values(pt).concat(Object.values(hg)).some((l) => l.startsWith("Ungenutzt"))).toBe(false);
  });

  it("names_a_field_function_the_family_does_not_know_by_its_number", () => {
    expect(fieldFunctions("Hgss", 13).slice(-1)[0]).toEqual({ value: 13, label: "Funktion 13" });
    expect(fieldFunctions("Pt", 25).slice(-1)[0]).toEqual({ value: 25, label: "Funktion 25" });
    expect(fieldFunctions("Pt", 24)).toBe(FIELD_FUNCTIONS.Pt);
    expect(itemText("field_use_func", 23, "Pt")).toBe("Kampfkamera");
    expect(itemText("field_use_func", 23, "Hgss")).toBe("Aprikokobox");
    expect(itemText("field_use_func", 13, "Hgss")).toBe("Funktion 13");
    expect(itemLabel("Honig", "field_use_func", 14, 23, { family: "Hgss" })).toBe("Honig: Feldfunktion Honig → Aprikokobox");
  });

  it("names_a_battle_function_the_games_do_not_know_by_its_number", () => {
    expect(battleFunctions(5).slice(-1)[0]).toEqual({ value: 5, label: "Funktion 5" });
    expect(battleFunctions(3)).toBe(BATTLE_FUNCTIONS);
    expect(itemText("battle_use_func", 5)).toBe("Funktion 5");
  });

  it("calls_an_item_usable_in_battle_by_its_battle_pocket_not_its_battle_function", () => {
    expect(BATTLE_FUNCTIONS[0]!.label).toBe("Keine eigene Funktion");
    // X-Angriff (member 57): battle pocket 2 (Kampf-Items), battle function 0.
    expect(usableInBattle({ battle_pocket: 2, battle_use_func: 0 })).toBe(true);
    expect(usableInBattle({ battle_pocket: 0, battle_use_func: 0 })).toBe(false);
  });
});
