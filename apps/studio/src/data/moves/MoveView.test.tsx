// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, refs } from "../../api";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { movesHarness, wingAttackDoc, wireApi } from "./movesTesting";

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
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("MoveView", () => {
  let h: ReturnType<typeof movesHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = movesHarness();
  });
  afterEach(() => h.unmount());

  const card = () => h.view().querySelector<HTMLElement>('[aria-label="Karte"]')!;
  const chip = (label: string) => [...h.view().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.startsWith(label));

  it("shows_the_card_with_type_category_numbers_and_traits", async () => {
    await h.open();
    expect(dataGet).toHaveBeenCalledWith("move", 17);
    expect(h.view().querySelector("h2")?.textContent).toContain("Flügelschlag");
    expect(card().querySelector("[data-type]")?.textContent).toBe("Flug");
    const text = card().textContent ?? "";
    for (const part of ["Physisch", "Nr. 0017", "60 Stärke", "100 % Genauigkeit", "35 AP", "Macht Kontakt", "Schutz wirkt", "Spiegeltrick wirkt", "King-Stein wirkt", "Priorität 0", "Ein gewähltes Pokémon"]) {
      expect(text).toContain(part);
    }
    // Bit 6 has no name: the card does not show it; magic coat and snatch are not set.
    expect(text).not.toContain("Bit 6");
    expect(text).not.toContain("Magiemantel");
  });

  it("names_a_move_that_always_hits_and_one_without_power", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ power: 0, accuracy: 0, category: 2 }));
    await h.open();
    expect(card().textContent).toContain("trifft immer");
    expect(card().textContent).toContain("keine Stärke");
    expect(card().textContent).toContain("Status");
  });

  it("leads_from_the_name_to_its_text_archive", async () => {
    await h.open();
    const name = h.view().querySelector<HTMLButtonElement>("h2 button")!;
    await h.click(name);
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 647, message: 17 });
  });

  it("shows_the_description_on_the_ds_screen_and_leads_to_its_text", async () => {
    await h.open();
    const screen = h.view().querySelector('[role=group][aria-label^="DS-Bildschirm"]');
    expect(screen?.textContent).toContain("Trifft den Gegner mit weit");
    expect(screen?.textContent).toContain("ausgebreiteten Flügeln.");
    await h.click(chip("Beschreibung bearbeiten"));
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 646, message: 17 });
  });

  it("says_when_the_description_is_missing", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { description: null }));
    await h.open();
    expect(h.view().textContent).toContain("Beschreibung nicht gefunden");
    expect(chip("Beschreibung bearbeiten")).toBeUndefined();
  });

  it("counts_where_the_move_is_used_without_scripts", async () => {
    await h.open();
    expect(refs).toHaveBeenCalledWith({ kind: "move", index: 17 });
    const uses = h.view().querySelector<HTMLElement>('section[aria-label="Verwendet in"]')!;
    expect(uses.textContent).toContain("2 Lernsets, 1 Trainer");
    expect(uses.textContent).toContain("ohne Scripts gezählt");
    const learnsets = [...uses.querySelectorAll<HTMLButtonElement>("[data-learnset]")];
    expect(learnsets.map((b) => b.textContent)).toEqual(["Staralili0396", "Staravia0397"]);
    await act(async () => learnsets[1]!.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "species", index: 397, tab: "learnset" });
    const trainer = uses.querySelector<HTMLButtonElement>("[data-trainer]")!;
    expect(trainer.disabled).toBe(true);
    expect(trainer.textContent).toContain("Wanderer Kai");
  });

  it("follows_an_edit_of_the_move", async () => {
    await h.open();
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ power: 65 }));
    act(() => h.bus.emit({ topic: "dataChanged", kind: "move", index: 17 }));
    await flush();
    expect(card().textContent).toContain("65 Stärke");
  });
});
