// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet } from "../../api";
import type { DataDocDto } from "../../api";
import { pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { HG_FILE, ROUTE_201, encounterDoc, encounterHarness, projectHg, wireApi } from "./encounterTesting";

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

describe("EncounterHand", () => {
  let h: ReturnType<typeof encounterHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = encounterHarness();
  });
  afterEach(() => h.unmount());

  const tabNames = () => h.tabs().map((t) => t.textContent);
  const enter = async (label: string, text: string) => {
    const input = h.field(label)!;
    typeInto(input, text);
    pressKey(input, "Enter");
    await flush();
  };
  const table = () => h.hand().querySelector<HTMLElement>("table")!;
  const headers = () => [...table().querySelectorAll("th")].map((c) => c.textContent);
  const row = (slot: number) => table().querySelector<HTMLElement>(`tr[data-slot="${slot}"]`)!;

  it("has_a_tab_per_method_with_the_count_of_its_used_slots_and_ersatz", async () => {
    await h.open(140);
    expect(dataGet).toHaveBeenCalledWith("encounter", 140);
    expect(tabNames()).toEqual(["Gras11", "Surfen1", "Alte Angel0", "Profi-Angel0", "Super-Angel0", "Ersatz5"]);
    expect(headers()).toEqual(["Platz", "Pokémon", "Level", "Anteil", "Ersetzt durch"]);
    expect(row(0).textContent).toContain("Platz 1");
    expect(row(0).textContent).toContain("20 %");
    expect(row(2).textContent).toContain("Tag 1 (von 10 bis 19 Uhr), Nacht 1 (von 20 bis 3 Uhr)");
    // An empty slot is valid and muted.
    expect(row(11).hasAttribute("data-empty")).toBe(true);
    expect(row(0).hasAttribute("data-empty")).toBe(false);
  });

  it("changes_the_rate_with_a_speaking_label_and_refuses_more_than_a_byte_in_platinum", async () => {
    await h.open(140);
    expect(h.hand().textContent).toContain("0 = nie");
    expect(h.hand().textContent).toContain("Das Spiel liest nur das untere Byte");
    await enter("Gras-Rate", "100");
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: null, field: "rate", value: 100 } }, "Route 201: Gras-Rate 30 → 100");
    vi.mocked(dataEdit).mockClear();
    await enter("Gras-Rate", "300");
    expect(dataEdit).not.toHaveBeenCalled();
  });

  it("changes_the_species_of_a_slot_through_a_picker", async () => {
    await h.open(140);
    await h.click(h.button("Pokémon von Platz 1 wählen, jetzt Staralili"));
    await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Bidiza")));
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 0, field: "species", value: 399 } }, "Route 201: Gras Platz 1 Staralili → Bidiza");
  });

  it("writes_a_min_above_max_and_shows_max_pulled_along_as_the_backend_answers", async () => {
    let doc: DataDocDto = ROUTE_201;
    wireApi(() => doc);
    await h.open(140);
    await h.showTab("Surfen");
    expect(h.field("Mindestlevel von Platz 1")?.value).toBe("20");
    expect(h.field("Höchstlevel von Platz 1")?.value).toBe("30");
    vi.mocked(dataEdit).mockImplementation(async () => {
      const surf = ROUTE_201.methods[1]!;
      doc = encounterDoc({ methods: ROUTE_201.methods.map((m) => (m === surf ? { ...surf, slots: surf.slots.map((s) => (s.index === 0 ? { ...s, min: 40, max: 40 } : s)) } : m)) });
      return [{ kind: "Encounter", index: 140 }];
    });
    await enter("Mindestlevel von Platz 1", "40");
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "surf", slot: 0, field: "min", value: 40 } }, "Route 201: Surfen Platz 1 Mindestlevel 20 → 40");
    await flush();
    expect(h.field("Höchstlevel von Platz 1")?.value).toBe("40");
  });

  it("shows_the_warning_of_a_slot_inline", async () => {
    await h.open(140);
    expect(h.hand().querySelector("[data-warning]")?.textContent).toContain("Nacht Eintrag 1 ist leer");
  });

  it("edits_the_replacement_lists_in_the_ersatz_tab", async () => {
    await h.open(140);
    await h.showTab("Ersatz");
    expect([...h.hand().querySelectorAll("table")].map((t) => t.getAttribute("aria-label"))).toEqual(["Schwarm", "Tag", "Nacht"]);
    expect(h.hand().textContent).toContain("Gras Platz 3");
    await h.click(h.button("Tag Eintrag 1 wählen, jetzt Staralili"));
    await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Sheinux")));
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "day", slot: 0, field: "species", value: 403 } }, "Route 201: Tag Eintrag 1 Staralili → Sheinux");
  });

  it("switches_gastrodon_to_east_by_writing_100_and_offers_the_unown_tables", async () => {
    await h.open(140);
    await h.showTab("Ersatz");
    const east = [...h.hand().querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === "Gastrodon Ost")!.querySelector<HTMLElement>("[role=switch]")!;
    await h.click(east);
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { EncounterExtra: { field: "form_rate_1", value: 100 } }, "Route 201: Gastrodon West → Ost");
    expect(h.hand().querySelector('[aria-label="Icognito-Tabelle"]')?.textContent).toContain("Fast alle Formen (0)");
  });

  it("shows_heartgold_grass_with_three_species_columns_and_one_level", async () => {
    wireApi(() => HG_FILE);
    await h.open(140, "bauen", { project: projectHg });
    expect(tabNames()).toEqual(["Gras12", "Surfen0", "Fels0", "Alte Angel0", "Profi-Angel0", "Super-Angel0", "Ersatz0"]);
    expect(headers()).toEqual(["Platz", "Morgens", "Tags", "Nachts", "Level", "Anteil", "Ersetzt durch"]);
    expect(h.hand().textContent).not.toContain("Das Spiel liest nur das untere Byte");
    expect(h.button("Morgens von Platz 1 wählen, jetzt Taubsi")).toBeDefined();
    expect(h.button("Tags von Platz 1 wählen, jetzt Rattfratz")).toBeDefined();
    await h.click(h.button("Nachts von Platz 1 wählen, jetzt Hoothoot"));
    await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Taubsi")));
    expect(dataEdit).toHaveBeenCalledWith("encounter", 140, { Encounter: { method: "grass", slot: 0, field: "species_night", value: 16 } }, "Route 201: Gras Platz 1 nachts Hoothoot → Taubsi");
  });

  it("shows_the_raw_bytes_while_tinkering", async () => {
    await h.open(140, "tuefteln");
    expect(tabNames()).toContain("Rohdaten");
    await h.showTab("Rohdaten");
    expect(h.hand().textContent).toContain("pl_enc_data.narc");
  });
});
