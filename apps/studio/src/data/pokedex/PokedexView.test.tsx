// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, refs } from "../../api";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { STARAVIA, pokedexHarness, staraviaDoc, wireApi } from "./pokedexTesting";
import { MISSING, actionTexts, chooseItem, chooseContext, contextTexts } from "../../original/resetTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  machineTable: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PokedexView", () => {
  let h: ReturnType<typeof pokedexHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi(() => staraviaDoc({ personal: { ...STARAVIA, attack: 85 } }));
    Element.prototype.scrollIntoView = () => {};
    h = pokedexHarness();
  });
  afterEach(() => h.unmount());

  const meters = () => [...h.view().querySelectorAll<HTMLElement>("[role=meter]")];
  const labelOf = (meter: HTMLElement) => document.getElementById(meter.getAttribute("aria-labelledby") ?? "")?.textContent;

  it("heads_with_name_types_and_abilities", async () => {
    await h.open();
    const head = h.view().querySelector("h2")!;
    expect(head.textContent).toBe("Staravia");
    expect([...h.view().querySelectorAll("[data-type]")].map((b) => b.textContent)).toEqual(["Normal", "Flug"]);
    expect(h.view().textContent).toContain("Bedroher");
  });

  it("draws_six_stat_bars_against_the_average_of_all_species_and_names_the_saved_sum", async () => {
    await h.open();
    expect(meters().map((m) => [labelOf(m), m.getAttribute("aria-valuenow")])).toEqual([
      ["KP", "55"],
      ["Angriff weicht ab", "85"],
      ["Verteidigung", "50"],
      ["Spezial-Angriff", "40"],
      ["Spezial-Verteidigung", "40"],
      ["Initiative", "80"],
    ]);
    // Average attack of 1, 396, 397, 398 and 520 (not 0 and the form 503): (50 + 55 + 75 + 50 + 50) / 5 = 56.
    expect(meters()[1]?.getAttribute("aria-valuetext")).toBe("85, über Schnitt");
    expect(h.view().textContent).toContain("Summe 350, vorher 340");
  });

  it("names_no_saved_sum_while_nothing_changed", async () => {
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc());
    await h.open();
    expect(h.view().textContent).toContain("Summe 340");
    expect(h.view().textContent).not.toContain("vorher");
  });

  it("lists_the_places_as_world_references_that_lead_to_the_header", async () => {
    await h.open();
    expect(refs).toHaveBeenCalledWith({ kind: "species", index: 397 });
    const places = [...h.view().querySelectorAll<HTMLButtonElement>("[data-place]")];
    // Two slots of one encounter file are one place.
    expect(places.map((p) => p.textContent)).toEqual(["Route 2090012"]);
    await act(async () => places[0]!.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "header", id: 5 });
  });

  it("opens_a_trainer_whose_team_holds_the_species", async () => {
    await h.open();
    const trainer = h.view().querySelector<HTMLButtonElement>("[data-trainer]")!;
    expect(trainer.textContent).toContain("Wanderer Kai");
    expect(trainer.disabled).toBe(false);
    await act(async () => trainer.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "trainer", index: 44 });
  });

  it("lists_the_other_uses_counted_without_scripts", async () => {
    await h.open();
    expect(h.view().textContent).toContain("ohne Scripts gezählt");
    const use = h.view().querySelector<HTMLButtonElement>("[data-use]")!;
    expect(use.textContent).toContain("Staralili, Entwicklung 1 Ziel");
    await act(async () => use.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "species", index: 396, tab: "evolution" });
  });

  it("follows_an_edit_of_the_hand", async () => {
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc());
    await h.open();
    expect(meters()[1]?.getAttribute("aria-valuenow")).toBe("75");
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc({ personal: { ...STARAVIA, attack: 90 } }));
    act(() => h.bus.emit({ topic: "dataChanged", kind: "species", index: 397 }));
    await flush();
    expect(meters()[1]?.getAttribute("aria-valuenow")).toBe("90");
  });

  it("says_why_a_locked_species_has_no_values", async () => {
    vi.mocked(dataGet).mockResolvedValue(
      staraviaDoc({ locked: "personal 0397: 36 bytes, expected 44", raw_locked: [], locked_asset: { kind: "Personal", index: 397 } }),
    );
    await h.open();
    expect(h.view().textContent).toContain("Pokémon-Daten 0397: 36 Bytes statt 44");
    expect(meters()).toHaveLength(0);
  });

  it("takes_a_lock_without_its_part_as_one_of_the_personal_data_like_the_hand", async () => {
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc({ locked: "personal 0397: 36 bytes, expected 44", raw_locked: [], locked_asset: null }));
    await h.open();
    expect(meters()).toHaveLength(0);
    expect(h.hand().querySelector<HTMLInputElement>('input[aria-label="Angriff"]')?.disabled).toBe(true);
  });

  it("keeps_the_values_when_only_the_learnset_is_locked", async () => {
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc({ locked: "learnset 0397: 7 bytes, expected 44", raw_locked: [], locked_asset: { kind: "Learnset", index: 397 } }));
    await h.open();
    expect(meters()).toHaveLength(6);
  });

  it("offers_reset_in_head_and_list", async () => {
    await h.open();
    // The head of the view: the species' data, learnset and evolution in one step.
    expect(actionTexts(h.view())).toEqual(["Auf Original zurücksetzen…"]);
    chooseItem(h.view(), "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenLastCalledWith({ kind: "species", index: 397 });
    const row = h.q<HTMLElement>('[data-species="396"]')!;
    expect(contextTexts(row)).toEqual(["Auf Original zurücksetzen…"]);
    chooseContext(row, "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenLastCalledWith({ kind: "species", index: 396 });
  });

  it("without_a_base_the_entry_says_basis_rom_fehlt", async () => {
    h.reset.status = MISSING;
    await h.open();
    expect(actionTexts(h.view())).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
    expect(contextTexts(h.q<HTMLElement>('[data-species="397"]')!)).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
  });
});
