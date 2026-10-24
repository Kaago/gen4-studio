// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataRaw } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { pressKey, typeInSearch, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { LEARNSET, STARAVIA, pokedexHarness, staraviaDoc, wireApi } from "./pokedexTesting";

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

describe("PokedexHand", () => {
  let h: ReturnType<typeof pokedexHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = pokedexHarness();
  });
  afterEach(() => h.unmount());

  const tabNames = () => h.tabs().map((t) => t.textContent?.replace(/\d+$/, ""));

  it("has_the_tabs_of_the_editor_and_the_raw_data_only_while_tinkering", async () => {
    await h.open(397, "bauen");
    expect(tabNames()).toEqual(["Werte", "Fähigkeiten und Typen", "Lernset", "Entwicklung", "TM/VM"]);
    expect(dataGet).toHaveBeenCalledWith("species", 397);
    h.unmount();
    h = pokedexHarness();
    await h.open(397, "tuefteln");
    expect(tabNames()).toEqual(["Werte", "Fähigkeiten und Typen", "Lernset", "Entwicklung", "TM/VM", "Rohdaten"]);
  });

  it("commits_a_changed_attack_as_one_labelled_edit_and_tells_the_others", async () => {
    await h.open();
    const heard: EditEvent[] = [];
    h.bus.subscribe("dataChanged", (e) => heard.push(e));
    vi.mocked(dataGet).mockClear();
    const attack = h.field("Angriff")!;
    expect(attack.value).toBe("75");
    typeInto(attack, "85");
    pressKey(attack, "Enter");
    await flush();
    expect(dataEdit).toHaveBeenCalledWith("species", 397, { Personal: { field: "attack", value: 85 } }, "Staravia: Angriff 75 → 85");
    expect(heard).toEqual([{ topic: "dataChanged", kind: "species", index: 397 }]);
    expect(h.calls.onEdited).toHaveBeenCalled();
    // The editor reads the species again.
    expect(dataGet).toHaveBeenCalledWith("species", 397);
  });

  it("offers_the_number_base_only_while_tinkering", async () => {
    await h.open(397, "bauen");
    expect(h.hand().querySelector('[aria-label="Zahlenbasis von Angriff"]')).toBeNull();
    h.unmount();
    h = pokedexHarness();
    await h.open(397, "tuefteln");
    expect(h.hand().querySelector('[aria-label="Zahlenbasis von Angriff"]')).not.toBeNull();
  });

  it("shows_the_values_with_their_words", async () => {
    await h.open();
    expect(h.field("EV-Ertrag Angriff")?.value).toBe("2");
    expect(h.hand().textContent).toContain("höher ist leichter");
    expect(h.hand().textContent).toContain("15 Zyklen, 3.840 Schritte");
    const select = (label: string) => h.hand().querySelector<HTMLElement>(`[aria-label="${label}"]`)?.textContent;
    expect(select("Wachstum")).toContain("Mittellangsam");
    expect(select("Geschlecht")).toContain("50 % männlich");
    expect(select("Ei-Gruppe 1")).toContain("Flug");
    // Flee rate and colour only while tinkering.
    expect(h.field("Fluchtrate")).toBeNull();
  });

  it("shows_an_error_of_the_backend_as_a_notice", async () => {
    vi.mocked(dataEdit).mockRejectedValue({ code: "error", message: "no editable field attack", asset: null });
    await h.open();
    const attack = h.field("Angriff")!;
    typeInto(attack, "85");
    pressKey(attack, "Enter");
    await flush();
    expect(h.hand().querySelector("[role=alert]")?.textContent).toContain("no editable field attack");
  });

  it("locks_the_fields_of_a_species_whose_personal_data_has_the_wrong_length", async () => {
    vi.mocked(dataGet).mockResolvedValue(
      staraviaDoc({ locked: "personal 0397: 36 bytes, expected 44", raw_locked: Array.from({ length: 36 }, () => 0), locked_asset: { kind: "Personal", index: 397 } }),
    );
    await h.open();
    expect(h.hand().textContent).toContain("Pokémon-Daten 0397: 36 Bytes statt 44");
    expect(h.field("Angriff")?.disabled).toBe(true);
    await h.showTab("TM/VM");
    expect(h.hand().querySelector<HTMLButtonElement>("[data-copy-moves]")?.disabled).toBe(true);
  });

  it("marks_the_tabs_of_unsaved_parts", async () => {
    await h.open(397, "bauen", { dirty: [{ kind: "Learnset", index: 397 }] });
    const dotted = h.tabs().filter((t) => t.parentElement?.querySelector('[aria-label="ungespeichert"]')).map((t) => t.textContent?.replace(/\d+$/, ""));
    expect(dotted).toEqual(["Lernset"]);
  });

  describe("abilities and types", () => {
    it("picks_an_ability_from_all_124", async () => {
      await h.open();
      await h.showTab("Fähigkeiten und Typen");
      const button = h.hand().querySelector<HTMLButtonElement>('button[aria-label="Fähigkeit 1 wählen, jetzt Bedroher"]');
      await h.click(button);
      expect(h.options()).toHaveLength(124);
      await h.click(h.options().find((o) => o.textContent?.includes("Adlerauge")));
      expect(dataEdit).toHaveBeenCalledWith("species", 397, { Personal: { field: "ability1", value: 51 } }, "Staravia: Fähigkeit 1 Bedroher → Adlerauge");
    });
  });

  describe("learnset", () => {
    it("lists_the_moves_and_commits_a_changed_level_sorted_by_level", async () => {
      await h.open();
      await h.showTab("Lernset");
      const rows = [...h.hand().querySelectorAll("[data-learn]")];
      expect(rows.map((r) => r.getAttribute("data-move"))).toEqual(["33", "45", "98", "17"]);
      expect(h.hand().textContent).toContain("4 von 20");
      const level = h.field("Level von Ruckzuckhieb")!;
      typeInto(level, "12");
      pressKey(level, "Enter");
      await flush();
      expect(dataEdit).toHaveBeenCalledWith(
        "species",
        397,
        { Learnset: { entries: [LEARNSET[0], LEARNSET[1], LEARNSET[3], { move_id: 98, level: 12 }] } },
        "Staravia: Ruckzuckhieb Level 5 → 12",
      );
    });

    it("refuses_a_level_over_100", async () => {
      await h.open();
      await h.showTab("Lernset");
      const level = h.field("Level von Tackle")!;
      typeInto(level, "101");
      pressKey(level, "Enter");
      await flush();
      expect(dataEdit).not.toHaveBeenCalled();
    });

    it("deletes_a_row", async () => {
      await h.open();
      await h.showTab("Lernset");
      await h.click(h.hand().querySelector<HTMLElement>('button[aria-label="Heuler auf Level 1 entfernen"]'));
      expect(dataEdit).toHaveBeenCalledWith("species", 397, { Learnset: { entries: [LEARNSET[0], LEARNSET[2], LEARNSET[3]] } }, "Staravia: Lernset 4 → 3 Attacken");
    });

    it("adds_a_row_until_the_twenty_the_game_reads", async () => {
      await h.open();
      await h.showTab("Lernset");
      await h.click(h.hand().querySelector<HTMLElement>("[data-add-learn]"));
      typeInSearch(document.querySelector<HTMLInputElement>('input[aria-label="Suchen"]')!, "Flügel");
      await h.click(h.options()[0]);
      expect(dataEdit).toHaveBeenCalledWith(
        "species",
        397,
        { Learnset: { entries: [LEARNSET[0], LEARNSET[1], { move_id: 17, level: 1 }, LEARNSET[2], LEARNSET[3]] } },
        "Staravia: Lernset 4 → 5 Attacken",
      );
    });

    it("locks_adding_at_twenty_rows", async () => {
      const full = Array.from({ length: 20 }, (_, i) => ({ move_id: 33, level: i + 1 }));
      vi.mocked(dataGet).mockResolvedValue(staraviaDoc({ learnset: full }));
      await h.open();
      await h.showTab("Lernset");
      expect(h.hand().querySelector<HTMLButtonElement>("[data-add-learn]")?.disabled).toBe(true);
      expect(h.hand().textContent).toContain("20 von 20");
    });
  });

  describe("evolution", () => {
    it("shows_seven_rows_with_a_field_for_the_parameter_of_the_method_and_a_jump_to_the_target", async () => {
      await h.open();
      await h.showTab("Entwicklung");
      expect(h.hand().querySelectorAll("[data-evo]")).toHaveLength(7);
      const first = h.hand().querySelector<HTMLElement>('[data-evo="0"]')!;
      expect(first.querySelector('[aria-label="Methode, Entwicklung 1"]')?.textContent).toContain("Level");
      expect(first.querySelector<HTMLInputElement>('input[aria-label="Level, Entwicklung 1"]')?.value).toBe("34");
      const jump = first.querySelector<HTMLButtonElement>("button[data-species-ref]")!;
      expect(jump.textContent).toContain("Staraptor");
      await h.click(jump);
      expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "species", index: 398 });
      const level = first.querySelector<HTMLInputElement>('input[aria-label="Level, Entwicklung 1"]')!;
      typeInto(level, "30");
      pressKey(level, "Enter");
      await flush();
      expect(dataEdit).toHaveBeenCalledWith("species", 397, { Evolution: { slot: 0, method: 4, param: 30, target: 398 } }, "Staravia: Entwicklung 1 Level 34 → 30");
    });
  });

  describe("machines", () => {
    it("toggles_a_machine_named_by_its_move", async () => {
      await h.open();
      await h.showTab("TM/VM");
      const grid = h.hand().querySelector<HTMLElement>("[role=group][aria-label='TM und VM']")!;
      expect(grid.querySelectorAll("[role=checkbox]")).toHaveLength(100);
      expect(grid.querySelector('[aria-label="TM40 Aero-Ass"]')?.getAttribute("aria-checked")).toBe("true");
      expect(h.hand().textContent).toContain("2 von 100");
      await h.click(grid.querySelector<HTMLElement>('[aria-label="TM40 Aero-Ass"]'));
      expect(dataEdit).toHaveBeenCalledWith("species", 397, { Tm: { n: 39, on: false } }, "Staravia: TM40 an → aus");
    });

    it("copies_machines_and_learnset_of_another_species_in_one_step", async () => {
      vi.mocked(dataEdit).mockResolvedValue([
        { kind: "Personal", index: 397 },
        { kind: "Learnset", index: 397 },
      ]);
      await h.open();
      const heard: EditEvent[] = [];
      h.bus.subscribe("dataChanged", (e) => heard.push(e));
      await h.showTab("TM/VM");
      await h.click(h.hand().querySelector<HTMLElement>("[data-copy-moves]"));
      expect(document.body.textContent).toContain("Ersetzt TM/VM und Lernset von Staravia");
      await h.click(h.options().find((o) => o.textContent?.includes("Staralili")));
      expect(dataEdit).toHaveBeenCalledWith("species", 397, { CopyMoves: { from: 396 } }, "Staravia: TM/VM und Lernset wie Staralili");
      expect(heard).toEqual([
        { topic: "dataChanged", kind: "species", index: 397 },
        { topic: "dataChanged", kind: "learnset", index: 397 },
      ]);
    });
  });

  describe("raw data", () => {
    it("shows_the_bytes_of_each_part_and_marks_the_field_focused_last", async () => {
      await h.open(397, "tuefteln");
      act(() => h.field("Angriff")!.focus());
      await h.showTab("Rohdaten");
      expect(dataRaw).toHaveBeenCalledWith("species", 397);
      expect(dataRaw).toHaveBeenCalledWith("learnset", 397);
      expect(dataRaw).toHaveBeenCalledWith("evolution", 397);
      expect(h.hand().textContent).toContain("files/poketool/personal/pl_personal.narc");
      expect([...h.hand().querySelectorAll("[data-marked]")].map((c) => c.getAttribute("data-offset"))).toContain("1");
      expect(h.hand().textContent).toContain("Angriff");
    });
  });

  it("keeps_the_values_of_the_saved_state_apart", async () => {
    // A guard for the fixtures: the document's own values show, not the saved ones.
    vi.mocked(dataGet).mockResolvedValue(staraviaDoc({ personal: { ...STARAVIA, attack: 85 } }));
    await h.open();
    expect(h.field("Angriff")?.value).toBe("85");
  });
});
