// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataRaw } from "../../api";
import { openSelect, pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { namedClassOptions } from "./trainerModel";
import { JENS, PLACEHOLDER_DOC, TRAINER_NAMES, trainerHarness, wireApi } from "./trainerTesting";

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
    expect(h.field("Vor dem Kampf")?.value).toBe("Los geht's!\\r");

    await enter("Name", "Jensi");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerName: { name: "Jensi" } }, "Wanderer Jens: Name Jens → Jensi");
    await h.click(h.toggle("Doppelkampf"));
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "battle_type", value: 2 } }, "Wanderer Jens: Kampfart Einzel → Doppel");
    openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Klasse"]')!);
    await flush();
    await h.click(h.selectOptions().find((o) => o.textContent === "Göre"));
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { Trainer: { field: "class", value: 7 } }, "Wanderer Jens: Klasse Wanderer → Göre");
    await enter("Niederlage", "Mist!");
    expect(dataEdit).toHaveBeenLastCalledWith("trainer", 12, { TrainerTexts: { texts: [{ kind: 1, text: "Mist!" }] } }, "Wanderer Jens: Text Niederlage geändert");
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
