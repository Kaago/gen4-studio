// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet } from "../../api";
import { pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import type { EncounterDoc } from "./encounterModel";
import { HG_FILE, ROUTE_201, encounterHarness, projectHg, wireApi } from "./encounterTesting";

/** The HeartGold file with Habitak (21) in the morning on slot 2 only. */
const HG_MORNING: EncounterDoc = {
  ...HG_FILE,
  methods: HG_FILE.methods.map((m) => (m.method !== "grass" ? m : { ...m, slots: m.slots.map((s) => (s.index === 1 ? { ...s, species_morning: 21 } : s)) })),
  effective: [{ when: "morning", species: [{ species: 16, share: 80 }, { species: 21, share: 20 }] }, ...HG_FILE.effective.slice(1)],
};

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  createEncounterFile: vi.fn(),
  splitEncounterFile: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Route 201 with Sheinux as „Tag“ entry 1 (by day on grass slot 3 only) and on grass slot 5 (gewöhnlich, mornings and nights 10 %). */
const SHEINUX: EncounterDoc = {
  ...ROUTE_201,
  methods: ROUTE_201.methods.map((m) => (m.method !== "grass" ? m : { ...m, slots: m.slots.map((s) => (s.index === 4 ? { ...s, species: 403 } : s)) })),
  replacements: ROUTE_201.replacements.map((l) => (l.list !== "day" ? l : { ...l, slots: l.slots.map((s) => (s.index === 0 ? { ...s, species: 403 } : s)) })),
  effective: [
    { when: "morning", species: [{ species: 396, share: 65 }, { species: 399, share: 24 }, { species: 403, share: 10 }] },
    { when: "day", species: [{ species: 396, share: 55 }, { species: 399, share: 24 }, { species: 403, share: 20 }] },
    { when: "night", species: [{ species: 396, share: 55 }, { species: 399, share: 24 }, { species: 403, share: 10 }] },
  ],
};

describe("EncounterViewNarrate", () => {
  let h: ReturnType<typeof encounterHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = encounterHarness();
  });
  afterEach(() => h.unmount());

  const rows = (when: string) => [...h.view().querySelectorAll<HTMLElement>(`li[data-waiting="${when}"]`)];
  const row = (when: string, species: number) => h.view().querySelector<HTMLElement>(`li[data-waiting="${when}"][data-species="${species}"]`)!;
  const inRow = (r: HTMLElement, label: string) => [...r.querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent ?? "").trim() === label);
  const rarity = (r: HTMLElement) => r.querySelector<HTMLElement>("[data-rarity]")!;
  const rarityButton = (r: HTMLElement, word: string) => [...rarity(r).querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === word)!;
  const pick = async (r: HTMLElement, label: string, species: string) => {
    await h.click(inRow(r, label));
    await h.click(h.pickerOptions().find((o) => o.textContent?.includes(species)));
  };

  it("three columns with the effective species", async () => {
    await h.open(140, "erzaehlen");
    const columns = [...h.view().querySelectorAll<HTMLElement>("[data-time]")];
    expect(columns.map((c) => c.dataset.time)).toEqual(["morning", "day", "night"]);
    expect(columns.map((c) => c.querySelector("h3")?.textContent)).toEqual([expect.stringContaining("Morgens"), expect.stringContaining("Tags"), expect.stringContaining("Nachts")]);
    for (const when of ["morning", "day", "night"]) expect(rows(when).map((r) => Number(r.dataset.species))).toEqual([396, 399]);
    const staralili = row("night", 396);
    expect(staralili.textContent).toContain("Staralili");
    expect(staralili.querySelector("[data-sprite]")).not.toBeNull();
    expect(staralili.querySelector("[data-share]")?.textContent).toBe("65 % · häufig");
    // The class of its first slot (Platz 1).
    expect(rarity(staralili).dataset.rarity).toBe("haeufig");
    expect(inRow(staralili, "Pokémon nachts wählen, jetzt Staralili")).toBeDefined();
    expect(staralili.querySelector<HTMLInputElement>('input[aria-label="Level Staralili Nachts"]')?.value).toBe("2");
    expect(staralili.textContent).toContain("Level");
    expect(rarity(staralili).querySelector("[role=group]")?.getAttribute("aria-label")).toBe("Wie häufig Staralili Nachts");
    // Surfen and Angeln stay in Bauen.
    expect(h.view().textContent).not.toContain("Enton");
    // The hand: only how often, and who uses the file.
    expect(h.hand().querySelector("[role=tab]")).toBeNull();
    expect(h.hand().querySelector<HTMLInputElement>('input[aria-label="Wie oft (0 = nie)"]')?.value).toBe("30");
    expect(h.hand().textContent).toContain("Verwendet von 1 Ort");
    expect(h.hand().querySelector('[data-header="342"]')).not.toBeNull();
  });

  it("the hand writes the grass rate", async () => {
    await h.open(140, "erzaehlen");
    const input = h.field("Wie oft (0 = nie)")!;
    typeInto(input, "50");
    pressKey(input, "Enter");
    await flush();
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: null, field: "rate", value: 50 } }, "Route 201: Gras-Rate 30 → 50");
  });

  it("picking a species writes the slot of that time", async () => {
    wireApi(() => HG_FILE);
    await h.open(140, "erzaehlen", { project: projectHg });
    expect(rows("night").map((r) => Number(r.dataset.species))).toEqual([163]);
    await pick(row("night", 163), "Pokémon nachts wählen, jetzt Hoothoot", "Taubsi");
    expect(dataEdit).toHaveBeenCalledTimes(1);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 0, field: "species_night", value: 16 } }, "Route 201: Gras Platz 1 nachts Hoothoot → Taubsi");
    vi.mocked(dataEdit).mockClear();
    // The level column is shared by all three times.
    const level = row("morning", 16).querySelector<HTMLInputElement>('input[aria-label="Level Taubsi Morgens"]')!;
    typeInto(level, "9");
    pressKey(level, "Enter");
    await flush();
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 0, field: "level", value: 9 } }, "Route 201: Gras Platz 1 Level 2 → 9");
  });

  it("picking a species in platinum writes the grass slot", async () => {
    await h.open(140, "erzaehlen");
    await pick(row("morning", 399), "Pokémon morgens wählen, jetzt Bidiza", "Sheinux");
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 1, field: "species", value: 403 } }, "Route 201: Gras Platz 2 Bidiza → Sheinux");
  });

  it("a pick finds the place in the file as it is when the edit runs", async () => {
    await h.open(140, "erzaehlen");
    // Behind the view, Bidiza has left slot 2: its lowest slot is now 4.
    const moved: EncounterDoc = { ...ROUTE_201, methods: ROUTE_201.methods.map((m) => (m.method !== "grass" ? m : { ...m, slots: m.slots.map((s) => (s.index === 1 ? { ...s, species: 396 } : s)) })) };
    vi.mocked(dataGet).mockResolvedValue(moved);
    await pick(row("morning", 399), "Pokémon morgens wählen, jetzt Bidiza", "Sheinux");
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 3, field: "species", value: 403 } }, "Route 201: Gras Platz 4 Bidiza → Sheinux");
  });

  it("a_day_only_species_patches_the_day_list", async () => {
    wireApi(() => SHEINUX);
    await h.open(140, "erzaehlen");
    const sheinux = row("day", 403);
    expect(sheinux.querySelector("[data-share]")?.textContent).toBe("20 % · häufig");
    // Its place is grass slot 3, taken by „Tag“ entry 1: a swap cannot move a list entry.
    expect(rarity(sheinux).dataset.rarity).toBe("gewoehnlich");
    expect(rarity(sheinux).title).toContain("Gras Platz 3");
    expect([...rarity(sheinux).querySelectorAll("button")].every((b) => b.disabled)).toBe(true);
    await pick(sheinux, "Pokémon tags wählen, jetzt Sheinux", "Bidiza");
    expect(dataEdit).toHaveBeenCalledTimes(1);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "day", slot: 0, field: "species", value: 399 } }, "Route 201: Tag Eintrag 1 Sheinux → Bidiza");
    // Mornings it is the grass slot itself.
    expect([...rarity(row("morning", 403)).querySelectorAll("button")].some((b) => b.disabled)).toBe(false);
  });

  it("rarity swaps with the first slot of the class in one edit", async () => {
    await h.open(140, "erzaehlen");
    const bidiza = row("morning", 399);
    expect(rarity(bidiza).dataset.rarity).toBe("haeufig");
    await h.click(rarityButton(bidiza, "selten"));
    expect(dataEdit).toHaveBeenCalledTimes(1);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { EncounterSwap: { method: "grass", a: 1, b: 6 } }, "Route 201: Gras Platz 2 ↔ 7");
  });

  it("a platinum day row on a grass slot swaps past the slots the day list takes", async () => {
    await h.open(140, "erzaehlen");
    const bidiza = row("day", 399);
    expect([...rarity(bidiza).querySelectorAll("button")].every((b) => !b.disabled)).toBe(true);
    expect(rarity(bidiza).title).toBe("");
    await h.click(rarityButton(bidiza, "gewöhnlich"));
    expect(dataEdit).toHaveBeenCalledTimes(1);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { EncounterSwap: { method: "grass", a: 1, b: 4 } }, "Route 201: Gras Platz 2 ↔ 5");
  });

  it("a class without a fitting slot is no edit", async () => {
    await h.open(140, "erzaehlen");
    // By day slots 3 and 4 are the „Tag“ entries and Staralili already waits on 5 and 6.
    await h.click(rarityButton(row("day", 396), "gewöhnlich"));
    expect(dataEdit).not.toHaveBeenCalled();
    expect(h.calls.onEdited).not.toHaveBeenCalled();
  });

  it("a heartgold rarity swap trades the slot of that time's column", async () => {
    wireApi(() => HG_MORNING);
    await h.open(140, "erzaehlen", { project: projectHg });
    const habitak = row("morning", 21);
    expect(rarity(habitak).dataset.rarity).toBe("haeufig");
    await h.click(rarityButton(habitak, "selten"));
    expect(dataEdit).toHaveBeenCalledTimes(1);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { EncounterSwap: { method: "grass", a: 1, b: 6 } }, "Route 201: Gras Platz 2 ↔ 7");
  });

  it("the same rarity is no edit", async () => {
    wireApi(() => SHEINUX);
    await h.open(140, "erzaehlen");
    const sheinux = row("morning", 403);
    expect(rarity(sheinux).dataset.rarity).toBe("gewoehnlich");
    await h.click(rarityButton(sheinux, "gewöhnlich"));
    expect(dataEdit).not.toHaveBeenCalled();
    expect(h.calls.onEdited).not.toHaveBeenCalled();
  });
});
