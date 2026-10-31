// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataRaw, getTrainerSprite } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { ProblemsStore } from "../../workbench/problems/problems";
import { openSelect, pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { JENS, PLACEHOLDER_DOC, trainerDoc, trainerHarness, wireApi } from "./trainerTesting";

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
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const WARNING = { severity: "warning" as const, file: "trainers/0012", line: 1, col: 1, message: "trainer 12: the text before the battle does not end with \\r, so its box closes at once", source: "trainer" as const };
const projectHg = { root: "/hg", game_code: "IPKD", family: "Hgss", language: "de", header_count: 3 };

describe("TrainerHand", () => {
  let h: ReturnType<typeof trainerHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = trainerHarness();
  });
  afterEach(() => h.unmount());

  const cards = () => [...h.hand().querySelectorAll<HTMLElement>("[data-slot]")];
  const tabNames = () => h.tabs().map((t) => t.textContent);
  const enter = async (label: string, text: string) => {
    const input = h.field(label)!;
    typeInto(input, text);
    pressKey(input, "Enter");
    await flush();
  };
  const button = (label: string) => [...h.hand().querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === label);
  const segment = (label: string) => [...h.hand().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label);

  it("the_head_shows_the_class_sprite_small", async () => {
    vi.mocked(getTrainerSprite).mockResolvedValue(new ArrayBuffer(4));
    await h.open();
    expect(getTrainerSprite).toHaveBeenCalledWith(6);
    const img = h.hand().querySelector<HTMLImageElement>("[data-trainer-sprite] img");
    expect(img).not.toBeNull();
    expect(img!.getAttribute("width")).toBe("48");
    expect(img!.getAttribute("alt")).toBe("");
    // The picture stands beside the title of the hand.
    expect(h.hand().querySelector("[data-trainer-sprite]")?.parentElement?.textContent).toContain("Wanderer Jens");
  });

  it("has_the_tabs_team_and_trainer_and_the_raw_data_while_tinkering", async () => {
    await h.open(12, "bauen");
    expect(dataGet).toHaveBeenCalledWith("trainer", 12);
    expect(tabNames()).toEqual(["Team1", "Trainer"]);
    h.unmount();
    h = trainerHarness();
    await h.open(12, "tuefteln");
    expect(tabNames()).toEqual(["Team1", "Trainer", "Rohdaten"]);
  });

  describe("team", () => {
    it("shows_six_cards_and_the_empty_places", async () => {
      await h.open();
      expect(cards()).toHaveLength(6);
      expect(cards()[0]?.getAttribute("aria-label")).toBe("Platz 1: Staralili, Level 12");
      expect(cards()[1]?.textContent).toBe("Platz 2 leer");
      expect(cards()[5]?.textContent).toBe("Platz 6 leer");
    });

    it("commits_a_changed_level_as_the_whole_party_with_a_speaking_label", async () => {
      await h.open();
      const heard: EditEvent[] = [];
      h.bus.subscribe("dataChanged", (e) => heard.push(e));
      await h.click(cards()[0]);
      expect(cards()[0]?.getAttribute("aria-pressed")).toBe("true");
      expect(h.field("Level")?.value).toBe("12");
      await enter("Level", "14");
      expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { TrainerParty: { party: [{ ...JENS.party[0], level: 14 }] } }, "Wanderer Jens: Platz 1 Level 12 → 14");
      expect(heard).toEqual([{ topic: "dataChanged", kind: "trainer", index: 12 }]);
      expect(h.calls.onEdited).toHaveBeenCalled();
    });

    it("refuses_a_level_above_100", async () => {
      await h.open();
      await enter("Level", "101");
      expect(dataEdit).not.toHaveBeenCalled();
    });

    it("adds_a_pokemon_on_an_empty_place_with_the_moves_the_game_would_give_it", async () => {
      wireApi(() => trainerDoc({ format: { moves: true, items: false }, party: [{ ...JENS.party[0]!, moves: [33, 45, 0, 0] }] }));
      await h.open();
      await h.click(cards()[1]);
      expect(document.body.textContent).toContain("Pokémon für Platz 2 wählen");
      await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Staravia")));
      expect(dataGet).toHaveBeenCalledWith("species", 397);
      const added = { iv_or_difficulty: 0, byte1: 0, level: 12, species: 397, form: 0, item: null, moves: [33, 45, 98, 0], ball_capsule: 0, gender: null, ability: null };
      expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { TrainerParty: { party: [{ ...JENS.party[0], moves: [33, 45, 0, 0] }, added] } }, "Wanderer Jens: Platz 2 Staravia hinzugefügt");
    });

    it("changes_the_species_and_removes_a_pokemon", async () => {
      await h.open();
      await h.click(button("Art von Platz 1 wählen, jetzt Staralili"));
      await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Staravia")));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerParty: { party: [{ ...JENS.party[0], species: 397 }] } }, "Wanderer Jens: Platz 1 Staralili → Staravia");
      await h.click(button("Pokémon entfernen"));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerParty: { party: [] } }, "Wanderer Jens: Platz 1 Staralili entfernt");
    });

    it("shows_moves_only_with_own_moves_and_the_item_only_with_items", async () => {
      await h.open();
      expect(button("Attacke 1 von Platz 1 wählen, jetzt Tackle")).toBeUndefined();
      expect(h.hand().textContent).toContain("Eigene Attacken sind aus");
      h.unmount();
      wireApi(() => trainerDoc({ format: { moves: true, items: true }, party: [{ ...JENS.party[0]!, moves: [33, 45, 0, 0], item: 17 }] }));
      h = trainerHarness();
      await h.open();
      await h.click(button("Attacke 2 von Platz 1 wählen, jetzt Heuler"));
      await h.click(h.pickerOptions().find((o) => o.textContent?.includes("Ruckzuckhieb")));
      expect(dataEdit).toHaveBeenLastCalledWith(
        "trainer",
        12,
        { TrainerParty: { party: [{ ...JENS.party[0], moves: [33, 98, 0, 0], item: 17 }] } },
        "Wanderer Jens: Platz 1 Attacke 2 Heuler → Ruckzuckhieb",
      );
      expect(button("Gegenstand von Platz 1 wählen, jetzt Trank")).toBeDefined();
    });

    it("keeps_both_bytes_of_the_platinum_strength", async () => {
      wireApi(() => trainerDoc({ party: [{ ...JENS.party[0]!, iv_or_difficulty: 50, byte1: 1 }] }));
      await h.open();
      expect(h.field("Stärke (IV-Skala)")?.value).toBe("306");
      await enter("Stärke (IV-Skala)", "255");
      expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { TrainerParty: { party: [{ ...JENS.party[0], iv_or_difficulty: 255, byte1: 0 }] } }, "Wanderer Jens: Platz 1 Stärke 306 → 255");
    });

    it("edits_gender_and_ability_in_heartgold_and_the_difficulty_as_one_byte", async () => {
      wireApi(() => trainerDoc({ party: [{ ...JENS.party[0]!, iv_or_difficulty: 100, byte1: 0x20, gender: 0, ability: 2 }] }));
      await h.open(12, "bauen", { project: projectHg });
      expect(h.field("Stärke")?.value).toBe("100");
      expect(h.hand().textContent).toContain("IV = Wert × 31 / 255");
      expect(h.hand().querySelector('[aria-label="Fähigkeit"]')?.textContent).toContain("Fähigkeit 2");
      openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Geschlecht"]')!);
      await flush();
      await h.click(h.selectOptions().find((o) => o.textContent?.includes("männlich")));
      expect(dataEdit).toHaveBeenCalledWith(
        "trainer",
        12,
        { TrainerParty: { party: [{ ...JENS.party[0], iv_or_difficulty: 100, byte1: 0x20, gender: 1, ability: 2 }] } },
        "Wanderer Jens: Platz 1 Geschlecht Auto → männlich",
      );
    });
  });

  describe("trainer", () => {
    it("fills_the_moves_with_the_switch_and_says_so_in_the_label", async () => {
      await h.open();
      await h.showTab("Trainer");
      expect(h.hand().textContent).toContain("Einschalten gibt jedem Pokémon die Attacken, die das Spiel ihm gäbe");
      await h.click(h.toggle("Eigene Attacken"));
      expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { TrainerFormat: { moves: true, items: null } }, "Wanderer Jens: eigene Attacken an");
      await h.click(h.toggle("Items tragen"));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerFormat: { moves: null, items: true } }, "Wanderer Jens: Items tragen an");
    });

    it("refuses_a_name_of_eight_characters_and_says_why", async () => {
      await h.open();
      await h.showTab("Trainer");
      expect(h.field("Name")?.value).toBe("Jens");
      expect(h.hand().textContent).toContain("4 von 7");
      await enter("Name", "Jensinea");
      expect(dataEdit).not.toHaveBeenCalled();
      expect(h.hand().textContent).toContain("Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch");
      await enter("Name", "Jensi");
      expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { TrainerName: { name: "Jensi" } }, "Wanderer Jens: Name Jens → Jensi");
    });

    it("sets_ai_battle_type_and_class", async () => {
      await h.open();
      await h.showTab("Trainer");
      await h.click(segment("klug"));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "ai", value: 7 } }, "Wanderer Jens: KI einfach → klug");
      await h.click(segment("Doppel"));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "battle_type", value: 2 } }, "Wanderer Jens: Kampfart Einzel → Doppel");
      openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Klasse"]')!);
      await flush();
      await h.click(h.selectOptions().find((o) => o.textContent?.includes("Göre")));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "class", value: 7 } }, "Wanderer Jens: Klasse Wanderer → Göre");
    });

    it("names_an_unknown_class_as_the_backend_does_and_offers_the_mark_only_for_its_warning", async () => {
      wireApi(() => trainerDoc({ class: 130, texts: [{ kind: 0, archive: 617, message: 100, text: "Los!" }], problems: [{ ...WARNING, message: "trainer 12: something else" }] }));
      await h.open();
      await h.showTab("Trainer");
      expect(h.hand().querySelector('[aria-label="Klasse"]')?.textContent).toContain("Klasse 130");
      expect(h.hand().textContent).toContain("trainer 12: something else");
      expect(button("⏎ anhängen")).toBeUndefined();
    });

    it("writes_a_battle_text_and_shows_the_warning_of_the_text_before_the_battle", async () => {
      const problems = new ProblemsStore();
      wireApi(() => trainerDoc({ texts: [{ kind: 0, archive: 617, message: 100, text: "Los!" }], problems: [WARNING] }));
      await h.open(12, "bauen", { problems });
      await h.showTab("Trainer");
      expect(h.hand().textContent).toContain("Der Text vor dem Kampf sollte auf ⏎ enden");
      // The Problems list holds the trainer's row.
      expect(problems.list().map((r) => r.file)).toEqual(["trainers/0012"]);
      expect(h.field("Vor dem Kampf")?.value).toBe("Los!");
      // Missing kinds are offered to be made.
      expect(h.field("Niederlage")).toBeNull();
      await h.click(button("Text Niederlage anlegen"));
      await enter("Niederlage", "Mist!");
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 1, text: "Mist!" }] } }, "Wanderer Jens: Text Niederlage angelegt");
      await h.click(button("⏎ anhängen"));
      expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 0, text: "Los!\\r" }] } }, "Wanderer Jens: Text vor dem Kampf geändert");
    });
  });

  it("locks_every_field_of_the_placeholder", async () => {
    wireApi(() => PLACEHOLDER_DOC);
    await h.open(929);
    expect(h.hand().textContent).toContain("Platzhalter für das Sicht-Script");
    expect(cards().every((c) => c.getAttribute("aria-disabled") === "true")).toBe(true);
    await h.showTab("Trainer");
    expect(h.field("Name")?.disabled).toBe(true);
    expect(h.toggle("Eigene Attacken")?.hasAttribute("data-disabled")).toBe(true);
    await h.click(segment("klug"));
    expect(dataEdit).not.toHaveBeenCalled();
  });

  it("shows_both_members_and_the_32_ai_bits_while_tinkering", async () => {
    await h.open(12, "tuefteln");
    await h.showTab("Rohdaten");
    expect(dataRaw).toHaveBeenCalledWith("trainer", 12);
    expect(h.hand().textContent).toContain("files/poketool/trainer/trdata.narc");
    const bits = [...h.hand().querySelectorAll<HTMLElement>("[data-ai-bit] [role=checkbox]")];
    expect(bits).toHaveLength(32);
    expect(bits[0]?.getAttribute("aria-checked")).toBe("true");
    await h.click(h.toggle("Bit 1: Angriffe bewerten"));
    expect(dataEdit).toHaveBeenCalledWith("trainer", 12, { Trainer: { field: "ai", value: 3 } }, "Wanderer Jens: KI einfach → eigene 0x3");
  });
});
