// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, refs } from "../../api";
import type { RefDto } from "../../api";
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
    for (const part of ["Physisch", "Nummer 0017", "60 Stärke", "100 % Genauigkeit", "35 AP", "Macht Kontakt", "Schutzschild wirkt", "Spiegeltrick wirkt", "King-Stein wirkt", "Priorität 0", "Ein gewähltes Pokémon"]) {
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

  it("leads_from_the_name_to_the_name_archive_the_backend_names", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { name_archive: 750 }));
    await h.open();
    const name = h.view().querySelector<HTMLButtonElement>("h2 button")!;
    expect(name.textContent).toBe("FlügelschlagArchiv 0750");
    await h.click(name);
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 750, message: 17 });
  });

  it("names_the_move_without_a_link_when_no_archive_names_it", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { name_archive: null }));
    await h.open();
    expect(h.view().querySelector("h2")?.textContent).toBe("Flügelschlag");
    expect(h.view().querySelector("h2 button")).toBeNull();
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
    expect(trainer.textContent).toContain("Wanderer Kai");
    await act(async () => trainer.click());
    expect(h.calls.goTo).toHaveBeenLastCalledWith({ kind: "trainer", index: 44 });
  });

  it("shows_all_learnsets_of_one_move_and_starts_short_again_for_the_next", async () => {
    const learnset = (index: number): RefDto => ({ source: { kind: "Learnset", index }, field: "move", slot: 0, label: `Art ${index}, Lernset Platz 1`, source_name: `Art ${index}`, nav: { kind: "species", index, tab: "learnset" } });
    vi.mocked(refs).mockResolvedValue(Array.from({ length: 15 }, (_, i) => learnset(i + 1)));
    await h.open();
    const chips = () => h.view().querySelectorAll("[data-learnset]").length;
    expect(chips()).toBe(12);
    await h.click(chip("Alle zeigen, 3 weitere"));
    expect(chips()).toBe(15);
    await h.click(h.q<HTMLElement>('[data-move="33"]'));
    expect(chips()).toBe(12);
  });

  it("shows_a_description_edited_in_the_texts_and_ignores_other_archives", async () => {
    await h.open();
    const reads = () => vi.mocked(dataGet).mock.calls.length;
    const before = reads();
    // Another archive (a script's texts): nothing to read again.
    act(() => h.bus.emit({ topic: "textData", archive: 100 }));
    await flush();
    expect(reads()).toBe(before);
    // The description archive, edited in the texts ("Beschreibung bearbeiten"): the view shows the new text.
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { description: { archive: 646, message: 17, text: "Neue Beschreibung." } }));
    act(() => h.bus.emit({ topic: "textData", archive: 646 }));
    await flush();
    const screen = h.view().querySelector('[role=group][aria-label^="DS-Bildschirm"]');
    expect(screen?.textContent).toContain("Neue Beschreibung.");
    // The name archive too, and any archive (a reload): each read as often as the description's (view and hand both show the move).
    const perEvent = reads() - before;
    expect(perEvent).toBeGreaterThan(0);
    act(() => h.bus.emit({ topic: "textData", archive: 647 }));
    await flush();
    act(() => h.bus.emit({ topic: "textData", archive: null }));
    await flush();
    expect(reads()).toBe(before + 3 * perEvent);
  });

  it("names_sources_by_their_own_name_also_when_it_holds_a_comma", async () => {
    vi.mocked(refs).mockResolvedValue([
      { source: { kind: "Learnset", index: 496 }, field: "move", slot: 0, label: "Deoxys, Angriffsform, Lernset Platz 1", source_name: "Deoxys, Angriffsform", nav: { kind: "species", index: 496, tab: "learnset" } },
    ]);
    await h.open();
    const chip = h.view().querySelector<HTMLButtonElement>('[data-learnset="496"]')!;
    expect(chip.textContent).toBe("Deoxys, Angriffsform0496");
  });

  it("follows_an_edit_of_the_move", async () => {
    await h.open();
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({ power: 65 }));
    act(() => h.bus.emit({ topic: "dataChanged", kind: "move", index: 17 }));
    await flush();
    expect(card().textContent).toContain("65 Stärke");
  });
});
