// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createTrainer, dataList } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { readShellState } from "../../ui/frame/shellState";
import { openSelect, typeInto } from "../../ui/primitives/testUtils";
import { finding } from "../../checks/checksTesting";
import { ProblemsStore } from "../../workbench/problems/problems";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { trainerHarness, trainerRow, wireApi } from "./trainerTesting";
import { chooseContext, readyWith, contextTexts } from "../../original/resetTesting";

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

  it("marks_a_trainer_that_a_check_found_an_error_at", async () => {
    const problems = new ProblemsStore();
    problems.replaceChecks([finding("trainer.no-party", "error", { subject: { kind: "trainer", index: 13 }, key: "party" })]);
    await h.open(12, "bauen", { problems });
    const mark = tree().querySelector('[data-trainer="13"] [data-finding-mark]');
    expect(mark?.getAttribute("data-finding-mark")).toBe("error");
    expect(mark?.getAttribute("aria-label")).toBe("Fehler gefunden");
    expect(tree().querySelector('[data-trainer="12"] [data-finding-mark]')).toBeNull();
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
    // The list and the name bank are read again; the history catches up (its entry is an undo barrier).
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "trainer", index: null });
    expect(heard).toContainEqual({ topic: "textData", archive: 618 });
    expect(h.calls.onEdited).toHaveBeenCalled();
    // The placeholder written before it is named.
    expect(h.list().textContent).toContain("Trainer 930 angelegt; 929 bleibt Platzhalter für das Sicht-Script.");
    // The extended battle script is named by its file, as the Script panel names it.
    expect(h.list().textContent).toContain("Die Kampf-Scriptdatei 1114 hat neue Plätze bekommen.");
  });

  const openCreate = async () => {
    await h.click(h.list().querySelector<HTMLElement>('button[aria-label="Trainer anlegen…"]'));
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    typeInto(dialog.querySelector<HTMLInputElement>('input[aria-label="Name"]')!, "Lars");
    return dialog;
  };
  const create = (dialog: HTMLElement) => h.click([...dialog.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Anlegen"));

  it("asks_about_the_id_after_the_placeholder_when_the_next_one_is_the_sight_slot", async () => {
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "trainer" ? [trainerRow(0, "", 0, "Klasse 0", 0, []), trainerRow(928, "Jens", 6, "Wanderer", 5, [])] : []));
    await h.open(928);
    const dialog = await openCreate();
    expect(dialog.textContent).toContain("Trainer 930 anlegen?");
    expect(dialog.textContent).toContain("929 wird Platzhalter für das Sicht-Script.");
  });

  it("saves_the_open_battle_script_before_and_shows_its_extension_after_creating", async () => {
    const scriptFiles = { saveIfDirty: vi.fn(async () => {}), reload: vi.fn(async () => {}) };
    await h.open(12, "bauen", { scriptFiles });
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    await create(await openCreate());
    expect(scriptFiles.saveIfDirty).toHaveBeenCalledWith(1114);
    expect(scriptFiles.saveIfDirty.mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(createTrainer).mock.invocationCallOrder[0]!);
    expect(scriptFiles.reload).toHaveBeenCalledWith(1114);
    // The compile may have appended to text archives.
    expect(heard).toContainEqual({ topic: "textData", archive: null });
  });

  it("does_not_create_when_the_battle_script_cannot_be_saved", async () => {
    const scriptFiles = { saveIfDirty: vi.fn(async () => Promise.reject({ code: "error", message: "Textarchiv 0199 erst speichern", asset: null })), reload: vi.fn(async () => {}) };
    await h.open(12, "bauen", { scriptFiles });
    const dialog = await openCreate();
    await create(dialog);
    expect(createTrainer).not.toHaveBeenCalled();
    expect(dialog.textContent).toContain("Textarchiv 0199 erst speichern");
  });

  it("treats_a_partial_creation_like_one_that_went_through_and_says_what_stays", async () => {
    const scriptFiles = { saveIfDirty: vi.fn(async () => {}), reload: vi.fn(async () => {}) };
    const message = "creating trainer 930: battle script file 1114 has the slots up to trainer 930, trainer 929 was written, trainer 930 was written; setting its name and text offset failed: disk full";
    vi.mocked(createTrainer).mockRejectedValue({ code: "partial", message, asset: null });
    await h.open(12, "bauen", { scriptFiles });
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    await create(await openCreate());
    expect(document.querySelector("[role=dialog]")).toBeNull();
    expect(h.list().textContent).toContain("Trainer 930 nur zum Teil angelegt");
    expect(h.list().textContent).toContain("Danach fehlgeschlagen: Name und Text-Offset setzen (disk full).");
    expect(readShellState().selected.trainer).toBe(930);
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "trainer", index: null });
    expect(h.calls.onEdited).toHaveBeenCalled();
    expect(scriptFiles.reload).toHaveBeenCalledWith(1114);
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

  it("rows_offer_reset_and_lock_a_new_trainer", async () => {
    // The base has 929 trainers (0 to 928): the placeholder 929 is offered (the kernel refuses it with its reason), a trainer from 930 on is new.
    h.reset.status = readyWith({ trainer: 930 });
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "trainer" ? [trainerRow(12, "Jens", 6, "Wanderer", 12, [{ header: 5, name: "Route 201" }]), trainerRow(930, "Neu", 6, "Wanderer", 5, [{ header: 5, name: "Route 201" }])] : []));
    await h.open(12);
    const row = (index: number) => tree().querySelector<HTMLElement>(`[data-trainer="${index}"]`)!;
    chooseContext(row(12), "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenCalledWith({ kind: "trainer", index: 12 });
    expect(contextTexts(row(930))).toEqual(["Auf Original zurücksetzen…Kein Original"]);
    // The right click selects nothing.
    expect(row(930).getAttribute("aria-selected")).toBe("false");
  });
});
