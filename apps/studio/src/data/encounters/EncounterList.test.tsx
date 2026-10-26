// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createEncounterFile, dataList } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { readShellState } from "../../ui/frame/shellState";
import { typeInto } from "../../ui/primitives/testUtils";
import { finding } from "../../checks/checksTesting";
import { ProblemsStore } from "../../workbench/problems/problems";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { encounterHarness, wireApi } from "./encounterTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  createEncounterFile: vi.fn(),
  splitEncounterFile: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("EncounterList", () => {
  let h: ReturnType<typeof encounterHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = encounterHarness();
  });
  afterEach(() => h.unmount());

  const files = () => [...h.list().querySelectorAll<HTMLElement>("[data-encounter]")];
  const file = (index: number) => h.list().querySelector<HTMLElement>(`[data-encounter="${index}"]`);
  const search = () => h.list().querySelector<HTMLInputElement>('input[aria-label="Begegnungen durchsuchen"]')!;

  it("names_the_files_by_the_places_that_use_them_and_lists_the_others_ohne_ort", async () => {
    await h.open(140);
    expect(dataList).toHaveBeenCalledWith("encounter");
    expect(files().map((f) => Number(f.dataset.encounter))).toEqual([140, 141, 0, 182]);
    expect(file(140)?.textContent).toContain("Route 201");
    expect(file(140)?.textContent).toContain("0140");
    expect(file(140)?.textContent).toContain("verwendet von 1 Ort");
    expect(file(140)?.getAttribute("aria-selected")).toBe("true");
    expect(file(141)?.textContent).toContain("Route 202, Route 203");
    expect(file(141)?.textContent).toContain("verwendet von 2 Orten");
    const ohne = h.list().querySelector<HTMLElement>('[role=group][aria-label="Ohne Ort"]')!;
    expect(ohne.querySelector('[data-encounter="0"]')?.textContent).toContain("Begegnungen 0000");
    expect(ohne.querySelector('[data-encounter="182"]')).not.toBeNull();
    expect(h.list().textContent).toContain("4 Begegnungsdateien");
  });

  it("marks_a_file_that_a_check_found_an_error_or_warning_in", async () => {
    const problems = new ProblemsStore();
    problems.replaceChecks([
      finding("encounter.level-zero", "warning", { subject: { kind: "encounter", index: 141 }, key: "slot:grass:0" }),
      finding("encounter.min-max", "info", { subject: { kind: "encounter", index: 140 }, key: "slot:surf:0" }),
    ]);
    await h.open(140, "bauen", { problems });
    expect(file(141)?.querySelector("[data-finding-mark]")?.getAttribute("data-finding-mark")).toBe("warning");
    expect(file(140)?.querySelector("[data-finding-mark]")).toBeNull();
    await act(async () => problems.replaceChecks([]));
    expect(file(141)?.querySelector("[data-finding-mark]")).toBeNull();
  });

  it("selects_a_file_and_names_it_by_its_place_in_the_breadcrumb", async () => {
    await h.open(null);
    await h.click(file(141));
    expect(readShellState().selected.encounters).toBe(141);
    expect(h.trail()).toEqual(["Pokémon", "Begegnungen", "Route 202, Route 203"]);
    h.lastTrail()[2]?.onClick?.();
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "encounter", index: 141 });
  });

  it("finds_files_by_place_or_number", async () => {
    await h.open(null);
    typeInto(search(), "Route 203");
    await flush();
    expect(files().map((f) => Number(f.dataset.encounter))).toEqual([141]);
    typeInto(search(), "182");
    await flush();
    expect(files().map((f) => Number(f.dataset.encounter))).toEqual([182]);
  });

  it("creates_an_empty_file_through_a_question_and_selects_it", async () => {
    await h.open(140);
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    await h.click(h.button("Begegnungsdatei anlegen…"));
    const dialog = h.dialog()!;
    expect(dialog.textContent).toContain("Begegnungsdatei 0183 anlegen?");
    expect(dialog.textContent).toContain("Nicht rückgängig");
    expect(dialog.textContent).toContain("anbinden kann man sie im Inspector eines Gebiets oder Raums (Feld „Begegnungen“)");
    expect(dialog.textContent).not.toContain("Header-Editor");
    await h.click(h.button("Anlegen"));
    expect(createEncounterFile).toHaveBeenCalledWith(null);
    expect(readShellState().selected.encounters).toBe(183);
    expect(h.dialog()).toBeNull();
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "encounter", index: null });
    expect(h.calls.onEdited).toHaveBeenCalled();
  });

  it("creates_a_copy_of_a_file_chosen_in_a_picker", async () => {
    await h.open(140);
    await h.click(h.button("Begegnungsdatei anlegen…"));
    await h.click(h.button("Kopie von …"));
    // Without a chosen file there is nothing to copy yet.
    await h.click(h.button("Vorlage wählen, jetzt keine"));
    const route = h.pickerOptions().find((o) => o.textContent?.includes("Route 202, Route 203"));
    await h.click(route);
    expect(h.button("Vorlage wählen, jetzt Route 202, Route 203")).toBeDefined();
    await h.click(h.button("Anlegen"));
    expect(createEncounterFile).toHaveBeenCalledWith(141);
    expect(readShellState().selected.encounters).toBe(183);
  });

  it("shows_the_backends_refusal_in_the_dialog_and_selects_nothing", async () => {
    vi.mocked(createEncounterFile).mockRejectedValue({ code: "error", message: "HeartGold kann höchstens 255 Begegnungsdateien ansprechen", asset: null });
    await h.open(140);
    await h.click(h.button("Begegnungsdatei anlegen…"));
    await act(async () => h.button("Anlegen")!.click());
    await flush();
    expect(h.dialog()?.textContent).toContain("HeartGold kann höchstens 255 Begegnungsdateien ansprechen");
    expect(readShellState().selected.encounters).toBe(140);
  });
});
