import { describe, expect, it } from "vitest";
import type { DataRowDto, HeaderSummary } from "../../api";
import {
  applyEncounter,
  encounterRowLabel,
  groupFiles,
  isEmptySlot,
  methodTabs,
  partialSplit,
  rateLabel,
  replacedByText,
  replacesText,
  slotColumns,
  slotText,
  usedByText,
  warningText,
  whenText,
} from "./encounterModel";
import { ROUTE_201, slot } from "./encounterTesting";

const header = (id: number, location_name: string, internal_name = `H${id}`): HeaderSummary => ({ id, internal_name, location_index: 0, kanto: false, location_name, matrix_id: 0, area_data_id: 0, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const fileRow = (index: number, users: number[] | null): DataRowDto => ({ index, name: "", tech: String(index).padStart(4, "0"), locked: users === null ? "encounter 0001: 12 bytes, expected 424" : null, extra: users === null ? null : { users } });

describe("encounterModel", () => {
  it("has_the_method_tabs_of_each_family_in_game_order_with_ersatz_last", () => {
    expect(methodTabs("Pt").map((t) => t.label)).toEqual(["Gras", "Surfen", "Alte Angel", "Profi-Angel", "Super-Angel", "Ersatz"]);
    expect(methodTabs("Hgss").map((t) => t.label)).toEqual(["Gras", "Surfen", "Fels", "Alte Angel", "Profi-Angel", "Super-Angel", "Ersatz"]);
    expect(methodTabs("Hgss").map((t) => t.id)).toEqual(["grass", "surf", "rock_smash", "old_rod", "good_rod", "super_rod", "replacements"]);
  });

  it("has_three_species_columns_and_one_level_column_for_heartgold_grass", () => {
    expect(slotColumns("Hgss", "grass").map((c) => c.label)).toEqual(["Platz", "Morgens", "Tags", "Nachts", "Level", "Anteil", "Ersetzt durch"]);
    expect(slotColumns("Hgss", "grass").map((c) => c.key)).toEqual(["slot", "species_morning", "species", "species_night", "level", "share", "replaced"]);
    expect(slotColumns("Pt", "grass").map((c) => c.label)).toEqual(["Platz", "Pokémon", "Level", "Anteil", "Ersetzt durch"]);
    expect(slotColumns("Hgss", "surf").map((c) => c.label)).toEqual(["Platz", "Pokémon", "Level (min–max)", "Anteil", "Ersetzt durch"]);
  });

  it("counts_places_from_one_and_words_replacements_in_german", () => {
    expect(slotText(0)).toBe("Platz 1");
    expect(replacedByText([{ list: "day", entry: 0, when: "from 10 to 19 o'clock", in_file: true }, { list: "night", entry: 0, when: "from 20 to 3 o'clock", in_file: true }])).toBe(
      "Tag 1 (von 10 bis 19 Uhr), Nacht 1 (von 20 bis 3 Uhr)",
    );
    expect(replacedByText([])).toBe("");
    expect(whenText("with Emerald in the Game Boy Advance slot")).toBe("mit Smaragd im GBA-Slot");
    expect(whenText("in the Trophy Garden")).toBe("im Trophäengarten");
    expect(whenText("something new")).toBe("something new");
    expect(replacesText([{ method: "grass", slot: 2 }, { method: "grass", slot: 3 }])).toBe("Gras Platz 3, 4");
    expect(rateLabel("Route 201", "grass", 30, 100)).toBe("Route 201: Gras-Rate 30 → 100");
    expect(usedByText(1)).toBe("verwendet von 1 Ort");
    expect(usedByText(3)).toBe("verwendet von 3 Orten");
  });

  it("words_a_partial_split_in_german_and_names_the_copy_that_stays", () => {
    expect(partialSplit("encounter file 183 was written as a copy of 141; switching header 401 to it failed: disk full")).toEqual({
      copy: 183,
      text: "Begegnungsdatei 183 ist als Kopie von 141 angelegt und bleibt. Danach fehlgeschlagen: Header 401 auf sie umhängen (disk full).",
    });
    expect(partialSplit("something else")).toBeNull();
  });

  it("words_the_kernel_warnings_in_german_with_places_from_one", () => {
    const names = (id: number) => (id === 396 ? "Staralili" : `Art ${id}`);
    expect(warningText("grass slot 2: species 396 at level 0", names)).toBe("Gras Platz 3: Staralili auf Level 0");
    expect(warningText("surf slot 0: min level 30 is above max level 20, the game swaps them")).toBe("Surfen Platz 1: Mindestlevel 30 liegt über Höchstlevel 20, das Spiel tauscht sie");
    expect(warningText("good rod slot 4: level 120 is above 100")).toBe("Profi-Angel Platz 5: Level 120 liegt über 100");
    expect(warningText("grass rate 300 acts like 44")).toBe("Gras-Rate 300 wirkt wie 44 (das Spiel liest nur das untere Byte)");
    expect(warningText("grass rate 256 acts like 0: the method never runs")).toContain("Gras-Rate 256 wirkt wie 0");
    expect(warningText("grass slot 1: the night species is empty: from 20 to 3 o'clock it gives a glitch Pokémon")).toBe(
      "Gras Platz 2: Die Art für nachts ist leer, von 20 bis 3 Uhr erscheint dort ein Glitch-Pokémon",
    );
    expect(warningText("swarm entry 0 is empty: during a swarm, grass slot 0 gives a glitch Pokémon")).toBe("Schwarm Eintrag 1 ist leer: bei einem Schwarm erscheint auf Gras Platz 1 ein Glitch-Pokémon");
    expect(warningText("unknown words")).toBe("unknown words");
  });

  it("names_a_file_by_the_places_that_use_it_and_groups_the_others_ohne_ort", () => {
    const headers = [header(342, "Route 201"), header(343, "Route 201"), header(400, "Route 202"), header(401, ""), header(402, "Zweiblattdorf"), header(403, "Sandgemme")];
    expect(encounterRowLabel(fileRow(140, [342, 343]), headers)).toBe("Route 201");
    expect(encounterRowLabel(fileRow(141, [400, 401]), headers)).toBe("Route 202, H401");
    expect(encounterRowLabel(fileRow(142, [342, 400, 401, 402, 403]), headers)).toBe("Route 201, Route 202, H401 +2");
    expect(encounterRowLabel(fileRow(12, []), headers)).toBe("Begegnungen 0012");
    expect(encounterRowLabel(fileRow(13, [999]), headers)).toBe("Header 0999");
    const { placed, unplaced } = groupFiles([fileRow(0, []), fileRow(140, [342]), fileRow(1, null)]);
    expect(placed.map((r) => r.index)).toEqual([140]);
    expect(unplaced.map((r) => r.index)).toEqual([0, 1]);
  });

  it("lays_a_written_value_over_the_document_and_pulls_water_levels_like_the_backend", () => {
    const rate = applyEncounter(ROUTE_201, { Encounter: { method: "grass", slot: null, field: "rate", value: 100 } });
    expect(rate.methods[0]?.rate).toBe(100);
    const surf = applyEncounter(ROUTE_201, { Encounter: { method: "surf", slot: 0, field: "min", value: 40 } });
    expect(surf.methods[1]?.slots[0]).toMatchObject({ min: 40, max: 40 });
    const level = applyEncounter(ROUTE_201, { Encounter: { method: "grass", slot: 1, field: "level", value: 7 } });
    expect(level.methods[0]?.slots[1]).toMatchObject({ min: 7, max: 7 });
    const day = applyEncounter(ROUTE_201, { Encounter: { method: "day", slot: 1, field: "species", value: 9 } });
    expect(day.replacements.find((l) => l.list === "day")?.slots[1]?.species).toBe(9);
    const east = applyEncounter(ROUTE_201, { EncounterExtra: { field: "form_rate_1", value: 100 } });
    expect(east.extras.form_rates).toEqual([0, 100, 0, 0, 0]);
    // Shares and warnings stay as read.
    expect(rate.warnings).toBe(ROUTE_201.warnings);
  });

  it("takes_a_slot_without_species_and_levels_as_empty", () => {
    expect(isEmptySlot(slot(3, 0, 0))).toBe(true);
    expect(isEmptySlot(slot(3, 0, 5))).toBe(false);
    expect(isEmptySlot({ ...slot(3, 0, 0), species_night: 4 })).toBe(false);
  });
});
