import { describe, expect, it } from "vitest";
import { HG_FILE, ROUTE_201 } from "./encounterTesting";
import { RARITY_SLOTS, rarityOf, slotRarity, swapTarget, timeSlot } from "./rarity";
import type { EncounterDoc } from "./encounterModel";

const grass = ROUTE_201.methods[0]!.slots;

/** Route 201 with Sheinux as „Tag“ entry 1: by day it waits on grass slot 3 only, never in the grass list itself. */
const DAY_SHEINUX: EncounterDoc = {
  ...ROUTE_201,
  replacements: ROUTE_201.replacements.map((l) => (l.list !== "day" ? l : { ...l, slots: l.slots.map((s) => (s.index === 0 ? { ...s, species: 403 } : s)) })),
};

describe("rarity", () => {
  it("rarity_of_a_share_is_haeufig_from_20_gewoehnlich_from_10_else_selten", () => {
    expect([100, 20, 19, 10, 9, 1].map(rarityOf)).toEqual(["haeufig", "haeufig", "gewoehnlich", "gewoehnlich", "selten", "selten"]);
  });

  it("the_classes_are_slots_0_1_then_2_to_5_then_6_to_11", () => {
    expect(RARITY_SLOTS).toEqual({ haeufig: [0, 1], gewoehnlich: [2, 3, 4, 5], selten: [6, 7, 8, 9, 10, 11] });
    expect([0, 1, 2, 5, 6, 11, 12].map(slotRarity)).toEqual(["haeufig", "haeufig", "gewoehnlich", "gewoehnlich", "selten", "selten", null]);
  });

  it("swaps_with_the_first_slot_of_the_target_class_and_not_within_its_own", () => {
    expect(swapTarget(grass, 1, "selten")).toBe(6);
    expect(swapTarget(grass, 7, "haeufig")).toBe(0);
    expect(swapTarget(grass, 9, "gewoehnlich")).toBe(2);
    // Already in that class: nothing to swap.
    expect(swapTarget(grass, 1, "haeufig")).toBeNull();
    expect(swapTarget(grass, 4, "gewoehnlich")).toBeNull();
    // A method without that place.
    expect(swapTarget(grass.slice(0, 4), 1, "selten")).toBeNull();
  });

  it("finds_the_lowest_grass_slot_of_a_species_at_a_time_and_the_field_its_picker_writes", () => {
    expect(timeSlot(ROUTE_201, "Pt", "morning", 399)).toEqual({ method: "grass", slot: 1, field: "species", grass: 1 });
    expect(timeSlot(ROUTE_201, "Pt", "night", 396)).toEqual({ method: "grass", slot: 0, field: "species", grass: 0 });
    expect(timeSlot(ROUTE_201, "Pt", "day", 403)).toBeNull();
    // Platinum by day: slots 2 and 3 come from the list „Tag“.
    expect(timeSlot(DAY_SHEINUX, "Pt", "day", 403)).toEqual({ method: "day", slot: 0, field: "species", grass: 2 });
    expect(timeSlot(DAY_SHEINUX, "Pt", "morning", 403)).toBeNull();
    expect(timeSlot(DAY_SHEINUX, "Pt", "night", 403)).toBeNull();
    // HeartGold: one species column per time of day.
    expect(timeSlot(HG_FILE, "Hgss", "morning", 16)).toEqual({ method: "grass", slot: 0, field: "species_morning", grass: 0 });
    expect(timeSlot(HG_FILE, "Hgss", "day", 19)).toEqual({ method: "grass", slot: 0, field: "species", grass: 0 });
    expect(timeSlot(HG_FILE, "Hgss", "night", 163)).toEqual({ method: "grass", slot: 0, field: "species_night", grass: 0 });
    expect(timeSlot(HG_FILE, "Hgss", "night", 16)).toBeNull();
  });
});
