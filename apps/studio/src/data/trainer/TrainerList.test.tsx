// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createTrainer, dataList } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { readShellState } from "../../ui/frame/shellState";
import { openSelect, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { trainerHarness, wireApi } from "./trainerTesting";

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

describe("TrainerList", () => {
  let h: ReturnType<typeof trainerHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = trainerHarness();
  });
  afterEach(() => h.unmount());

  const tree = () => h.list().querySelector<HTMLElement>("[role=tree]")!;
  const groups = () => [...tree().querySelectorAll<HTMLElement>("[data-group]")].map((g) => g.textContent);
  const trainers = () => [...tree().querySelectorAll<HTMLElement>("[data-trainer]")].map((t) => Number(t.dataset.trainer));
  const search = () => h.list().querySelector<HTMLInputElement>('input[aria-label="Trainer durchsuchen"]')!;
  const group = (label: string) => [...tree().querySelectorAll<HTMLElement>("[data-group]")].find((g) => g.textContent?.startsWith(label));

  it("shows_the_trainers_as_a_tree_by_place_with_ohne_ort_and_opens_the_place_of_the_selection", async () => {
    await h.open(12);
    expect(dataList).toHaveBeenCalledWith("trainer");
    expect(search().getAttribute("placeholder")).toBe("Name, Klasse oder Nummer");
    // Route 201 holds the selection and is open; "Ohne Ort" is closed.
    expect(groups()).toEqual(["Route 2012", "Ohne Ort3"]);
    expect(trainers()).toEqual([12, 13]);
    const jens = tree().querySelector<HTMLElement>('[data-trainer="12"]')!;
    expect(jens.getAttribute("role")).toBe("treeitem");
    expect(jens.getAttribute("aria-level")).toBe("2");
    expect(jens.getAttribute("aria-selected")).toBe("true");
    expect(jens.textContent).toContain("Wanderer Jens");
    expect(jens.textContent).toContain("0012");
    expect(jens.textContent).toContain("Lv. 12");
    await h.click(group("Ohne Ort"));
    expect(trainers()).toEqual([12, 13, 0, 40, 929]);
    // The list head counts the trainers.
    expect(h.list().textContent).toContain("5 Trainer");
  });

  it("lists_the_placeholder_locked", async () => {
    await h.open(12);
    await h.click(group("Ohne Ort"));
    const placeholder = tree().querySelector<HTMLElement>('[data-trainer="929"]')!;
    expect(placeholder.textContent).toContain("Trainer 0929");
    expect(placeholder.textContent).toContain("Gesperrt");
    expect(placeholder.getAttribute("aria-label")).toBe("Trainer 0929, 0929, gesperrt: Platzhalter für das Sicht-Script");
  });

  it("finds_trainers_by_name_class_or_number_in_every_place", async () => {
    await h.open(null);
    typeInto(search(), "Wanderer");
    await flush();
    expect(trainers()).toEqual([12, 40]);
    typeInto(search(), "göre");
    await flush();
    expect(trainers()).toEqual([13]);
    typeInto(search(), "40");
    await flush();
    expect(trainers()).toEqual([40]);
    typeInto(search(), "Nobody");
    await flush();
    expect(h.list().textContent).toContain("Nichts gefunden für „Nobody“");
  });

  it("groups_by_class_on_request", async () => {
    await h.open(null);
    const byClass = [...h.list().querySelectorAll<HTMLElement>("button")].find((b) => b.textContent === "Nach Klasse")!;
    await h.click(byClass);
    expect(groups()).toEqual(["Göre1", "Klasse 01", "Wanderer2", "Ohne Klasse1"]);
  });

  it("selects_a_trainer_and_names_it_in_the_breadcrumb", async () => {
    await h.open(null);
    await h.click(group("Route 201"));
    await h.click(tree().querySelector<HTMLElement>('[data-trainer="13"]'));
    expect(readShellState().selected.trainer).toBe(13);
    expect(h.trail()).toEqual(["Pokémon", "Trainer", "Göre Mia"]);
    h.lastTrail()[2]?.onClick?.();
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "trainer", index: 13 });
  });

  it("creates_a_trainer_through_a_question_and_selects_it", async () => {
    await h.open(12);
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    await h.click(h.list().querySelector<HTMLElement>('button[aria-label="Trainer anlegen…"]'));
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog.textContent).toContain("Trainer 930 anlegen?");
    expect(dialog.textContent).toContain("Nicht rückgängig");
    expect(dialog.textContent).toContain("Höchstens 7 Zeichen");
    openSelect(dialog.querySelector<HTMLElement>('[aria-label="Klasse"]')!);
    await flush();
    await h.click([...document.querySelectorAll<HTMLElement>("[role=option]")].find((o) => o.textContent?.includes("Göre")));
    typeInto(dialog.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Mia");
    await h.click([...dialog.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Anlegen"));
    expect(createTrainer).toHaveBeenCalledWith(7, "Mia");
    expect(readShellState().selected.trainer).toBe(930);
    expect(document.querySelector("[role=dialog]")).toBeNull();
    // The list and the name bank are read again; the history catches up (the name is an undo step).
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "trainer", index: null });
    expect(heard).toContainEqual({ topic: "textData", archive: 618 });
    expect(h.calls.onEdited).toHaveBeenCalled();
    // The placeholder written before it is named.
    expect(h.list().textContent).toContain("Trainer 930 angelegt; 929 bleibt Platzhalter für das Sicht-Script.");
  });

  it("refuses_a_long_name_and_shows_the_backends_refusal_in_the_dialog", async () => {
    await h.open(12);
    await h.click(h.list().querySelector<HTMLElement>('button[aria-label="Trainer anlegen…"]'));
    const dialog = () => document.querySelector<HTMLElement>("[role=dialog]")!;
    const create = () => [...dialog().querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Anlegen")!;
    typeInto(dialog().querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jensinea");
    await h.click(create());
    expect(createTrainer).not.toHaveBeenCalled();
    expect(dialog().textContent).toContain("Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch");
    vi.mocked(createTrainer).mockRejectedValue({ code: "error", message: "Erst Scripts übernehmen (Bereich Welt, Panel Script)", asset: null });
    typeInto(dialog().querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Jens");
    await act(async () => create().click());
    await flush();
    expect(createTrainer).toHaveBeenCalledWith(6, "Jens");
    expect(dialog().textContent).toContain("Erst Scripts übernehmen (Bereich Welt, Panel Script)");
    expect(readShellState().selected.trainer).toBe(12);
  });
});
