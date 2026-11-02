// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, dataList, entityIds, getBattleSprite, getEvents, getTrainerSprite } from "../../api";
import { resetIcons } from "../icons";
import { JENS, PLACEHOLDER_DOC, SPECIES_DOC, TRAINER_ROWS, trainerDoc, trainerHarness, wireApi } from "./trainerTesting";
import { MISSING, actionTexts, chooseItem, chooseContext, contextTexts } from "../../original/resetTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  getTrainerSprite: vi.fn(),
  getBattleSprite: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  getEvents: vi.fn(),
  createTrainer: vi.fn(),
  entityIds: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TrainerView", () => {
  let h: ReturnType<typeof trainerHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = trainerHarness();
  });
  afterEach(() => h.unmount());

  const button = (text: string) => [...h.view().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.includes(text));

  it("shows_class_name_and_the_team_as_cards", async () => {
    await h.open();
    const view = h.view();
    expect(view.querySelector("h2")?.textContent).toBe("Wanderer Jens");
    expect(view.textContent).toContain("0012");
    const cards = [...view.querySelectorAll<HTMLElement>("[data-slot]")];
    expect(cards).toHaveLength(1);
    expect(cards[0]?.textContent).toContain("Staralili");
    expect(cards[0]?.textContent).toContain("Lv. 12");
    expect(view.textContent).not.toContain("Doppelkampf");
    expect(view.textContent).not.toContain("Ohne Pokémon stürzt der Kampf ab");
  });

  it("names_where_the_trainer_stands_and_opens_the_event", async () => {
    await h.open();
    expect(getEvents).toHaveBeenCalledWith(8);
    const place = h.view().querySelector<HTMLButtonElement>("[data-event]")!;
    expect(place.textContent).toContain("Route 201");
    expect(place.textContent).toContain("Kachel 12, 7");
    place.click();
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "event", eventFile: 8, eventKind: "object", index: 3, header: 5 });
  });

  it("warns_of_an_empty_team_offers_to_place_an_unplaced_trainer_and_marks_a_double_battle", async () => {
    const onPlace = vi.fn();
    wireApi(() => trainerDoc({ party: [], events: [], battle_type: 2 }));
    await h.open(12, "bauen", { onPlaceTrainer: onPlace });
    expect(h.view().textContent).toContain("Ohne Pokémon stürzt der Kampf ab");
    expect(h.view().textContent).toContain("Noch nicht platziert");
    expect(h.view().textContent).toContain("Doppelkampf");
    button("In der Welt platzieren")!.click();
    expect(onPlace).toHaveBeenCalledWith(12);
  });

  it("offers_placing_disabled_while_nobody_takes_it", async () => {
    wireApi(() => trainerDoc({ events: [] }));
    await h.open();
    expect(button("In der Welt platzieren")?.disabled).toBe(true);
  });

  it("names_the_placeholder_and_does_not_warn_of_its_empty_team", async () => {
    wireApi(() => PLACEHOLDER_DOC);
    await h.open(929);
    expect(h.view().textContent).toContain("Platzhalter für das Sicht-Script");
    expect(h.view().textContent).not.toContain("Ohne Pokémon stürzt der Kampf ab");
    expect(button("In der Welt platzieren")).toBeUndefined();
  });

  it("offers_reset_in_head_and_list", async () => {
    await h.open();
    // The head of the view: the trainer's properties and team (its name and texts stay, they are text archives).
    expect(actionTexts(h.view())).toEqual(["Löschen…", "Auf Original zurücksetzen…"]);
    chooseItem(h.view(), "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenLastCalledWith({ kind: "trainer", index: 12 });
    // The row in the list: the same.
    const row = h.list().querySelector<HTMLElement>('[data-trainer="13"]')!;
    expect(contextTexts(row)).toEqual(["Auf Original zurücksetzen…"]);
    chooseContext(row, "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenLastCalledWith({ kind: "trainer", index: 13 });
  });

  it("without_a_base_the_entry_says_basis_rom_fehlt", async () => {
    h.reset.status = MISSING;
    await h.open();
    expect(actionTexts(h.view())).toEqual(["Löschen…", "Auf Original zurücksetzen…Basis-ROM fehlt"]);
    expect(contextTexts(h.list().querySelector<HTMLElement>('[data-trainer="12"]')!)).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
  });

  it("text_project_shows_the_id_and_offers_rename", async () => {
    vi.mocked(entityIds).mockReset().mockResolvedValue({ 12: "TRAINER_WANDERER_JENS" });
    h.text.active = true;
    await h.open();
    expect(entityIds).toHaveBeenCalledWith("Trainer");
    const id = h.view().querySelector<HTMLElement>("header [data-entity-id]");
    expect(id?.textContent).toBe("TRAINER_WANDERER_JENS");
    expect(h.view().querySelector('button[aria-label="ID kopieren"]')).not.toBeNull();
    expect(actionTexts(h.view())).toEqual(["Löschen…", "Auf Original zurücksetzen…", "ID umbenennen…"]);
    chooseItem(h.view(), "ID umbenennen…");
    const dialog = document.querySelector<HTMLElement>("[role=dialog]");
    expect(dialog?.querySelector("h2")?.textContent).toBe("ID umbenennen");
    expect(dialog?.querySelector<HTMLInputElement>('input[aria-label="Neue ID"]')?.value).toBe("TRAINER_WANDERER_JENS");
  });

  it("dspre_layout_shows_neither", async () => {
    vi.mocked(entityIds).mockReset().mockResolvedValue({});
    await h.open();
    expect(entityIds).not.toHaveBeenCalled();
    expect(h.view().querySelector("[data-entity-id]")).toBeNull();
    expect(actionTexts(h.view())).toEqual(["Löschen…", "Auf Original zurücksetzen…"]);
  });

  it("the_head_has_loeschen_and_the_state_badge", async () => {
    h.del.states["trainer:12"] = "changed";
    h.del.states["trainer:13"] = "changed";
    // Trainer 13 is a tombstone in the store: the live row wins over the saved state.
    const rows = TRAINER_ROWS.map((r) => (r.index === 13 ? Object.assign({}, r, { deleted: true }) : r));
    const before = vi.mocked(dataList).getMockImplementation()!;
    vi.mocked(dataList).mockImplementation(async (kind, ...rest) => (kind === "trainer" ? rows : before(kind, ...rest)));
    await h.open(12);
    chooseItem(h.view(), "Löschen…");
    expect(h.del.open).toHaveBeenLastCalledWith({ kind: "trainer", index: 12 });
    // Beside the menu „Aktionen“.
    const state = h.view().querySelector<HTMLElement>("header [data-entity-state]");
    expect(state?.textContent).toBe("geändert");
    expect(state?.parentElement?.querySelector("button[aria-label='Aktionen']")).not.toBeNull();
    h.unmount();
    h = trainerHarness();
    h.del.states["trainer:13"] = "changed";
    await h.open(13);
    expect(h.view().querySelector("header [data-entity-state]")?.textContent).toBe("gelöscht");
    h.unmount();
    h = trainerHarness();
    await h.open(40);
    expect(h.view().querySelector("[data-entity-state]")).toBeNull();
  });

  describe("class sprite and battle preview", () => {
    const section = () => h.view().querySelector<HTMLElement>("section[data-ds-battle-section]")!;
    const choose = async (label: string) => {
      const segment = [...section().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label);
      await h.click(segment);
    };

    it("the_head_shows_the_class_sprite", async () => {
      vi.mocked(getTrainerSprite).mockResolvedValue(new ArrayBuffer(4));
      await h.open();
      expect(getTrainerSprite).toHaveBeenCalledWith(6);
      const img = h.view().querySelector<HTMLImageElement>("header [data-trainer-sprite] img");
      expect(img).not.toBeNull();
      // The head names the trainer already: the picture is decoration.
      expect(img!.getAttribute("alt")).toBe("");
    });

    it("the_head_shows_the_placeholder_without_a_class_sprite", async () => {
      await h.open();
      expect(h.view().querySelector("header [data-trainer-sprite] [data-sprite=placeholder]")).not.toBeNull();
    });

    it("the_battle_preview_switches_between_trainer_and_first_pokemon", async () => {
      await h.open();
      expect([...h.view().querySelectorAll("section[aria-label]")].map((s) => s.getAttribute("aria-label"))).toEqual(["Übersicht Wanderer Jens", "Team", "Kampfvorschau", "Steht in der Welt"]);
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
      // The trainer before the battle: the class and name in the box, no level, no own side.
      const foe = section().querySelector('[data-ds-status="foe"]')!;
      expect(foe.textContent).toContain("Wanderer Jens");
      expect(foe.textContent).not.toContain("Lv.");
      expect(section().querySelector('[data-ds-status="own"]')).toBeNull();
      expect(getBattleSprite).not.toHaveBeenCalled();

      await choose("Erstes Pokémon");
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("pokemon");
      const mon = section().querySelector('[data-ds-status="foe"]')!;
      expect(mon.textContent).toContain("Staralili");
      expect(mon.textContent).toContain("Lv. 12");
      expect(getBattleSprite).toHaveBeenCalledWith(expect.objectContaining({ species: 396, side: "front" }));
      expect(section().querySelector('[data-ds-status="own"]')).toBeNull();

      await choose("Trainer");
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
      expect(section().querySelector('[data-ds-status="foe"]')?.textContent).not.toContain("Lv.");
    });

    it("the_first_pokemon_with_a_form_asks_for_that_form", async () => {
      wireApi(() => trainerDoc({ party: [{ ...JENS.party[0]!, species: 201, form: 5 }] }));
      await h.open();
      await choose("Erstes Pokémon");
      expect(getBattleSprite).toHaveBeenCalledWith({ species: 201, form: 5, side: "front" });
    });

    it("an_explicitly_female_first_pokemon_asks_for_the_female_picture_and_an_unset_gender_does_not", async () => {
      wireApi(() => trainerDoc({ party: [{ ...JENS.party[0]!, gender: 2 }] }));
      await h.open();
      await choose("Erstes Pokémon");
      expect(getBattleSprite).toHaveBeenCalledWith({ species: 396, side: "front", female: true });
      h.unmount();
      for (const gender of [null, 0, 1]) {
        vi.mocked(getBattleSprite).mockClear();
        resetIcons();
        wireApi(() => trainerDoc({ party: [{ ...JENS.party[0]!, gender }] }));
        h = trainerHarness();
        await h.open();
        await choose("Erstes Pokémon");
        expect(getBattleSprite).toHaveBeenCalledWith({ species: 396, side: "front" });
        expect(getBattleSprite).not.toHaveBeenCalledWith(expect.objectContaining({ female: true }));
        h.unmount();
      }
      h = trainerHarness();
    });

    it("without_a_team_only_the_trainer_is_offered", async () => {
      wireApi(() => trainerDoc({ party: [], events: [] }));
      await h.open(12, "bauen");
      expect(section()).not.toBeNull();
      expect([...section().querySelectorAll<HTMLButtonElement>("button")].map((b) => b.textContent)).toEqual(["Trainer"]);
      expect(section().querySelector('[data-ds-status="foe"]')?.textContent).toContain("Wanderer Jens");
    });

    it("the_choice_survives_a_change_of_trainer_and_falls_back_without_a_team", async () => {
      // Trainer 40 (Kai) has no team: the view stays mounted and shows the trainer, and Jens's choice comes back.
      wireApi(() => trainerDoc());
      vi.mocked(dataGet).mockImplementation(async (kind, index) => (kind === "trainer" && index === 40 ? trainerDoc({ name: "Kai", party: [], events: [] }) : kind === "trainer" ? trainerDoc() : SPECIES_DOC));
      await h.open();
      await choose("Erstes Pokémon");
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("pokemon");
      const tree = () => h.list().querySelector<HTMLElement>("[role=tree]")!;
      await h.click([...tree().querySelectorAll<HTMLElement>("[data-group]")].find((g) => g.textContent?.startsWith("Ohne Ort")));
      await h.click(tree().querySelector<HTMLElement>('[data-trainer="40"]'));
      expect(h.view().querySelector("[aria-label^='Übersicht Wanderer Kai']")).not.toBeNull();
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("trainer");
      // Route 201 closed with the selection of Kai: open it to choose Jens again.
      await h.click([...tree().querySelectorAll<HTMLElement>("[data-group]")].find((g) => g.textContent?.startsWith("Route 201")));
      await h.click(tree().querySelector<HTMLElement>('[data-trainer="12"]'));
      expect(section().querySelector("[data-foe]")?.getAttribute("data-foe")).toBe("pokemon");
    });

    it("the_placeholder_has_neither_sprite_nor_preview", async () => {
      wireApi(() => PLACEHOLDER_DOC);
      await h.open(929);
      expect(h.view().querySelector("section[data-ds-battle-section]")).toBeNull();
      expect(getTrainerSprite).not.toHaveBeenCalled();
    });
  });
});
