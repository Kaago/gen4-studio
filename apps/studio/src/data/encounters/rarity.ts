import type { SlotDto } from "../../api";
import { t } from "../../i18n";
import type { EncounterDoc, TimeKey } from "./encounterModel";

/**
 * How often a species waits in the grass, in the words of Erzählen. The game's shares belong to the places (Spec 3b.3 §10): a species gets
 * more or less frequent only by moving to another grass slot, so a class is a range of places.
 */
export type Rarity = "haeufig" | "gewoehnlich" | "selten";

/** The grass slots of each class (0-based): häufig 20 % each, gewöhnlich 10 % each, selten the rest. */
export const RARITY_SLOTS: Readonly<Record<Rarity, readonly number[]>> = {
  haeufig: [0, 1],
  gewoehnlich: [2, 3, 4, 5],
  selten: [6, 7, 8, 9, 10, 11],
};

/** The classes in the order the switch shows them. */
export const RARITIES: readonly { value: Rarity; readonly label: string }[] = [rarityOption("haeufig"), rarityOption("gewoehnlich"), rarityOption("selten")];

/** One class; its `label` is read when asked for (in the language shown then), never when this module loads. */
function rarityOption(value: Rarity) {
  return {
    value,
    get label() {
      return t(`data:rarity.${value}`);
    },
  };
}

/** „häufig“ for `haeufig`. */
export const rarityWord = (rarity: Rarity): string => RARITIES.find((r) => r.value === rarity)!.label;

/** The class of a species' summed share at one time: häufig from 20 %, gewöhnlich from 10 %, else selten. */
export function rarityOf(share: number): Rarity {
  if (share >= 20) return "haeufig";
  if (share >= 10) return "gewoehnlich";
  return "selten";
}

/** The class of grass slot `slot`; null outside the twelve slots. */
export function slotRarity(slot: number): Rarity | null {
  return (Object.keys(RARITY_SLOTS) as Rarity[]).find((r) => RARITY_SLOTS[r].includes(slot)) ?? null;
}

/** The species field of a slot as the `Encounter` patch names it. */
export type SpeciesField = "species" | "species_morning" | "species_night";

/**
 * Where a species waits at one time of day: `grass` is its lowest grass slot then (its level and its class); `method`, `slot` and `field`
 * is what its picker writes. Platinum by day and by night takes some grass slots from the lists `day` and `night` (the entries whose
 * `replaces` names the slot): there it is that list's entry. HeartGold has one species column per time of day.
 */
export interface TimeSlot {
  method: string;
  slot: number;
  field: SpeciesField;
  grass: number;
}

const HGSS_FIELD: Readonly<Record<TimeKey, SpeciesField>> = { morning: "species_morning", day: "species", night: "species_night" };

/** What grass slot `slot` gives at `when`: the species and where it is written. */
function atTime(doc: EncounterDoc, family: string, when: TimeKey, slot: SlotDto): { species: number; at: TimeSlot } {
  if (family === "Hgss") {
    const field = HGSS_FIELD[when];
    const species = field === "species" ? slot.species : (slot[field] ?? 0);
    return { species, at: { method: "grass", slot: slot.index, field, grass: slot.index } };
  }
  // Platinum: the list named like the time (`day`, `night`) replaces the slots its entries name.
  const entry = doc.replacements.find((l) => l.list === when)?.slots.find((e) => e.replaces.some((r) => r.method === "grass" && r.slot === slot.index));
  if (entry !== undefined) return { species: entry.species, at: { method: when, slot: entry.index, field: "species", grass: slot.index } };
  return { species: slot.species, at: { method: "grass", slot: slot.index, field: "species", grass: slot.index } };
}

/** Where `species` waits at `when` (its lowest grass slot); null when no grass slot gives it then. */
export function timeSlot(doc: EncounterDoc, family: string, when: TimeKey, species: number): TimeSlot | null {
  const grass = doc.methods.find((m) => m.method === "grass");
  if (grass === undefined || species === 0) return null;
  for (const slot of [...grass.slots].sort((a, b) => a.index - b.index)) {
    const here = atTime(doc, family, when, slot);
    if (here.species === species) return here.at;
  }
  return null;
}

/**
 * The grass slot that slot `from` trades with so that the species it gives at `when` becomes `rarity` (ruling R6b): the first slot of that
 * class that the time does not take from a replacement list (Platinum by day and by night: grass slots 2 and 3) and that does not already
 * give the same species then. Null when `from` is already of that class or no slot of it qualifies: nothing is written.
 */
export function swapTarget(doc: EncounterDoc, family: string, when: TimeKey, from: number, rarity: Rarity): number | null {
  const slots = doc.methods.find((m) => m.method === "grass")?.slots ?? [];
  const own = slots.find((s) => s.index === from);
  if (own === undefined || slotRarity(from) === rarity) return null;
  const species = atTime(doc, family, when, own).species;
  for (const target of RARITY_SLOTS[rarity]) {
    const slot = slots.find((s) => s.index === target);
    if (slot === undefined) continue;
    const there = atTime(doc, family, when, slot);
    if (there.at.method === "grass" && there.species !== species) return target;
  }
  return null;
}
