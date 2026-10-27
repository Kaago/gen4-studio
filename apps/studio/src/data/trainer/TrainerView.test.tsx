// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents } from "../../api";
import { resetIcons } from "../icons";
import { PLACEHOLDER_DOC, trainerDoc, trainerHarness, wireApi } from "./trainerTesting";
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
  getEvents: vi.fn(),
  createTrainer: vi.fn(),
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
    expect(actionTexts(h.view())).toEqual(["Auf Original zurücksetzen…"]);
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
    expect(actionTexts(h.view())).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
    expect(contextTexts(h.list().querySelector<HTMLElement>('[data-trainer="12"]')!)).toEqual(["Auf Original zurücksetzen…Basis-ROM fehlt"]);
  });
});
