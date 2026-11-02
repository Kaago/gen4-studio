// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataRaw } from "../../api";
import { openSelect, pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { namedClassOptions } from "./trainerModel";
import { JENS, PLACEHOLDER_DOC, TRAINER_NAMES, trainerDoc, trainerHarness, wireApi } from "./trainerTesting";

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
  validateText: vi.fn(async () => []),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TrainerHandNarrate", () => {
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
  const textArea = (label: string) => h.hand().querySelector<HTMLTextAreaElement>(`textarea[aria-label="${label}"]`);
  /** Types into a message field and leaves it, as a user does: the field commits on blur. */
  const write = async (el: HTMLTextAreaElement | null, text: string) => {
    if (!el) throw new Error("no field");
    await act(async () => {
      el.focus();
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(el, text);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => el.blur());
    await flush();
  };

  it("no AI segment, no raw tab, team and texts editable", async () => {
    await h.open(12, "erzaehlen");
    expect(h.hand().querySelector("[data-trainer-narrate]")).not.toBeNull();
    expect(tabNames()).toEqual(["Team1", "Trainer"]);
    expect(dataRaw).not.toHaveBeenCalled();

    // Team as in Bauen: six cards, a changed level is one step with a speaking label.
    expect(cards()).toHaveLength(6);
    await h.click(cards()[0]);
    await enter("Level", "14");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerParty: { party: [{ ...JENS.party[0], level: 14 }] } }, "Wanderer Jens: Platz 1 Level 12 → 14");

    // Trainer: class, name, the double battle switch and the battle texts; no AI, no items of the trainer, no team format, no AI bits.
    await h.showTab("Trainer");
    expect(h.hand().querySelector('[aria-label="KI"]')).toBeNull();
    expect(h.hand().querySelector("[data-ai-bit]")).toBeNull();
    expect(h.hand().querySelector('[aria-label="Kampfart"]')).toBeNull();
    expect(h.field("Gegenstand 1")).toBeNull();
    expect(button("Gegenstand 1")).toBeUndefined();
    expect(h.hand().textContent).not.toContain("Items des Trainers");
    expect(h.toggle("Eigene Attacken")).toBeNull();
    expect(h.toggle("Items tragen")).toBeNull();
    expect(h.field("Name")?.value).toBe("Jens");
    expect(textArea("Vor dem Kampf")?.value).toBe("Los geht's!\\r");

    await enter("Name", "Jensi");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerName: { name: "Jensi" } }, "Wanderer Jens: Name Jens → Jensi");
    await h.click(h.toggle("Doppelkampf"));
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "battle_type", value: 2 } }, "Wanderer Jens: Kampfart Einzel → Doppel");
    openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Klasse"]')!);
    await flush();
    await h.click(h.selectOptions().find((o) => o.textContent === "Göre"));
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "class", value: 7 } }, "Wanderer Jens: Klasse Wanderer → Göre");
    await write(textArea("Niederlage"), "Mist!");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 1, text: "Mist!" }] } }, "Wanderer Jens: Text Niederlage geändert");
  });

  it("writes_the_battle_texts_in_the_message_editor_without_raw_codes", async () => {
    const warning = { severity: "warning" as const, file: "trainers/0012", line: 1, col: 1, message: "trainer 12: the text before the battle does not end with \\r, so its box closes at once", source: "trainer" as const };
    wireApi(() => trainerDoc({ texts: [{ kind: 0, archive: 617, message: 100, text: "Los!" }], problems: [warning] }));
    await h.open(12, "erzaehlen");
    await h.showTab("Trainer");
    const texts = h.hand().querySelector<HTMLElement>('section[aria-label="Kampftexte"]')!;
    // No plain text field with the raw codes: the message editor, as in the wizard „Neuer Trainer hier“.
    expect(h.field("Vor dem Kampf")).toBeNull();
    expect(textArea("Vor dem Kampf")?.value).toBe("Los!");
    expect(texts.querySelector("h3")?.textContent).not.toContain("\\r");
    expect(texts.querySelector("h3")?.textContent).toContain("Seitenende (Umschalt+Enter)");

    // A missing kind is made in the message editor too; the write is one step with the label of Bauen.
    await h.click(button("Text Niederlage anlegen"));
    await write(textArea("Niederlage"), "Mist!");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 1, text: "Mist!" }] } }, "Wanderer Jens: Text Niederlage angelegt");
    // The warning keeps its action.
    await h.click(button("⏎ anhängen"));
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 0, text: "Los!\\r" }] } }, "Wanderer Jens: Text vor dem Kampf geändert");
  });

  it("drops_a_refused_battle_text_for_the_stored_one", async () => {
    await h.open(12, "erzaehlen");
    await h.showTab("Trainer");
    vi.mocked(dataEdit).mockRejectedValueOnce(new Error("too long"));
    await write(textArea("Niederlage"), "Viel zu lang");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 1, text: "Viel zu lang" }] } }, "Wanderer Jens: Text Niederlage geändert");
    await flush();
    expect(textArea("Niederlage")?.value).toBe("Verloren!");
  });

  it("points_the_team_notes_to_bauen", async () => {
    await h.open(12, "erzaehlen");
    await h.click(cards()[0]);
    // The switches for items and own moves live in the Trainer tab of Bauen only.
    expect(h.hand().textContent).toContain("Items tragen ist aus (in Bauen, Reiter Trainer)");
    expect(h.hand().textContent).toContain("Eigene Attacken sind aus (in Bauen, Reiter Trainer)");
  });

  it("stays_without_raw_data_while_the_hand_of_bauen_has_them_in_tuefteln", async () => {
    // Erzählen never shows Rohdaten; Tüfteln is the hand of Bauen with them.
    await h.open(12, "tuefteln");
    expect(h.hand().querySelector("[data-trainer-narrate]")).toBeNull();
    expect(tabNames()).toEqual(["Team1", "Trainer", "Rohdaten"]);
  });

  it("locks_every_field_of_the_placeholder", async () => {
    wireApi(() => PLACEHOLDER_DOC);
    await h.open(929, "erzaehlen");
    expect(h.hand().textContent).toContain("Alle Felder sind gesperrt");
    await h.showTab("Trainer");
    expect(h.field("Name")?.disabled).toBe(true);
  });

  it("offers_the_classes_by_name_only", () => {
    const classes = ["Pkmn-Trainer", "class 1", "-----", "", "Wanderer"];
    expect(namedClassOptions(classes, 4).map((o) => o.label)).toEqual(["Pkmn-Trainer", "Wanderer"]);
    // The trainer's own class stays offered even without a name, so the field shows it.
    expect(namedClassOptions(classes, 1).map((o) => o.label)).toEqual(["Pkmn-Trainer", "Klasse 1", "Wanderer"]);
    // The test bank names three classes; the others read "Klasse 8" and are left out.
    expect(namedClassOptions(TRAINER_NAMES.classes, 6).map((o) => o.label)).toEqual(["Pkmn-Trainer", "Wanderer", "Göre"]);
  });
});
