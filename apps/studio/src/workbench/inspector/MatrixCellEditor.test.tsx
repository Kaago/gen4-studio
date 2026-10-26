// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MATRIX_EMPTY, duplicateMap, getMatrix, setMatrixCell } from "../../api";
import type { AssetId, HeaderSummary, MatrixDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { MatrixCellEditor } from "./MatrixCellEditor";
import type { MatrixCellContext } from "./MatrixCellEditor";
import { chooseAction, headAction, headActions } from "./headTesting";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn(), setMatrixCell: vi.fn(), duplicateMap: vi.fn(), inspectMapImport: vi.fn(async () => []) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const summary = (id: number): HeaderSummary => ({
  id,
  internal_name: `T${id}`,
  location_name: "Zweiblattdorf",
  location_index: 5,
  kanto: false,
  matrix_id: 3,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const matrix = (over: Partial<MatrixDto> = {}): MatrixDto => ({ index: 3, name: "m", width: 2, height: 1, maps: [10, 11], headers: [411, 412], altitudes: [0, 2], ...over });

describe("MatrixCellEditor", () => {
  let container: HTMLElement;
  let root: Root;
  const goTo = vi.fn();
  const onEdited = vi.fn();
  const onCountsChanged = vi.fn();
  const onError = vi.fn();
  const onMapImported = vi.fn();
  let dirty: AssetId[] = [];
  const context = (): MatrixCellContext => ({
    dirty,
    headers: [summary(411), summary(412)],
    identify: (h) => (h.id === 412 ? { kind: "room", name: "Haus von Lucas", place: "Zweiblattdorf" } : { kind: "area", name: "Zweiblattdorf", place: "Zweiblattdorf" }),
    headerCount: 500,
    goTo,
    counts: { Map: 600 },
    revision: 0,
    onEdited,
    onCountsChanged,
    onError,
    onMapImported,
    checks: null,
  });
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getMatrix).mockReset().mockImplementation(async () => matrix());
    vi.mocked(setMatrixCell).mockReset().mockResolvedValue(undefined);
    vi.mocked(duplicateMap).mockReset().mockResolvedValue(77);
    for (const fn of [goTo, onEdited, onCountsChanged, onError]) fn.mockReset();
    dirty = [];
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(x = 1, y = 0, depth: "bauen" | "tuefteln" = "bauen") {
    await act(async () => root.render(<MatrixCellEditor matrix={3} x={x} y={y} context={context()} depth={depth} />));
    await flush();
  }
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;
  const typeAndBlur = async (label: string, text: string) => {
    await act(async () => void input(label).focus());
    typeInto(input(label), text);
    await act(async () => void input(label).blur());
    await flush();
  };

  it("names_the_cell_and_the_matrix", async () => {
    await show();
    expect(container.querySelector("header[data-inspector-kind='map'] [data-eyebrow]")?.textContent).toBe("Karte");
    // Bauen: the map of the cell, and where it lies, in the words of the hierarchy (no matrix or header codes).
    expect(container.querySelector("h2")?.textContent).toBe("Karte 0011");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 1, 0 · Haus von Lucas");
    expect(input("Karte").value).toBe("11");
    expect(input("Gebiet").value).toBe("412");
    expect(input("Höhe").value).toBe("2");
  });

  it("says_Ungespeichert_while_the_matrix_has_unsaved_changes_and_not_for_another_matrix", async () => {
    dirty = [{ kind: "Matrix", index: 4 }, { kind: "Map", index: 3 }];
    await show();
    expect(container.textContent).not.toContain("Ungespeichert");
    dirty = [{ kind: "Matrix", index: 3 }];
    await show();
    expect(container.textContent).toContain("Ungespeichert");
  });

  it("writes_a_changed_map_and_an_emptied_map_as_the_empty_marker", async () => {
    await show();
    await typeAndBlur("Karte", "20");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, 20, null, null);
    await typeAndBlur("Karte", "");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, MATRIX_EMPTY, null, null);
    expect(onEdited).toHaveBeenCalledWith(3);
  });

  it("refuses_a_map_beyond_the_archive_with_the_limit_and_writes_nothing", async () => {
    await show();
    await typeAndBlur("Karte", "700");
    expect(container.textContent).toContain("höchstens 599");
    expect(setMatrixCell).not.toHaveBeenCalled();
  });

  it("drops_the_refusal_of_the_map_as_soon_as_the_user_types_again", async () => {
    await show();
    await typeAndBlur("Karte", "700");
    expect(container.textContent).toContain("höchstens 599");
    await act(async () => void input("Karte").focus());
    typeInto(input("Karte"), "7");
    expect(container.textContent).not.toContain("höchstens 599");
  });

  it("writes_a_header_and_an_altitude_and_refuses_beyond_their_limits", async () => {
    await show();
    await typeAndBlur("Gebiet", "413");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, 413, null);
    await typeAndBlur("Höhe", "9");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, null, 9);
    await typeAndBlur("Höhe", "300");
    expect(container.textContent).toContain("höchstens 255");
    await typeAndBlur("Gebiet", "600");
    expect(container.textContent).toContain("höchstens 499");
    expect(setMatrixCell).toHaveBeenCalledTimes(2);
  });

  it("leads_the_header_of_the_cell_to_its_editor", async () => {
    await show();
    await act(async () => void button("Gebiet0412").click());
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 412 });
  });

  it("says_which_sections_the_matrix_lacks", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show();
    expect(container.textContent).toContain("Die Weltkarte hat keinen Abschnitt für Gebiete.");
    expect(container.textContent).not.toContain("Höhenabschnitt");
    expect(input("Gebiet")).toBeNull();
  });

  it("shows_the_height_in_bauen_for_a_matrix_without_altitudes", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ altitudes: null }));
    await show();
    expect(input("Höhe").value).toBe("0");
    expect(container.textContent).not.toContain("Die Weltkarte hat keinen Höhenabschnitt.");
    await show(1, 0, "tuefteln");
    expect(input("Höhe").value).toBe("0");
    expect(container.textContent).not.toContain("Höhenabschnitt");
  });

  it("commits_a_height_without_altitudes_as_one_cell_edit", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ altitudes: null }));
    await show();
    await typeAndBlur("Höhe", "2");
    expect(setMatrixCell).toHaveBeenCalledTimes(1);
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, null, 2);
  });

  it("duplicates_the_map_into_the_cell_and_refreshes_the_counts", async () => {
    await show();
    chooseAction(container, "Duplizieren");
    await flush();
    expect(duplicateMap).toHaveBeenCalledWith(11);
    expect(setMatrixCell).toHaveBeenCalledWith(3, 1, 0, 77, null, null);
    expect(onCountsChanged).toHaveBeenCalledTimes(1);
  });

  it("says_a_cell_outside_the_matrix_is_outside", async () => {
    await show(5, 0);
    expect(container.textContent).toContain("außerhalb der Weltkarte (2 × 1)");
  });

  it("names_the_cell_with_the_codes_in_tuefteln", async () => {
    await show(1, 0, "tuefteln");
    expect(container.querySelector("h2")?.textContent).toBe("Zelle 1, 0");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Raster 0003 · M0011 H0412");
  });

  it("names_a_cell_without_a_map_or_a_known_header_by_the_cell_alone", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null], headers: [411, 900] }));
    await show(1, 0);
    expect(container.querySelector("h2")?.textContent).toBe("Leere Zelle");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 1, 0");
    await show(0, 0);
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 0, 0 · Zweiblattdorf");
    // Header 0 is the landscape: no place to name.
    act(() => root.render(null));
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: [0, 900] }));
    await show(0, 0);
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 0, 0");
    act(() => root.render(null));
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: [411, 900] }));
    await show(1, 0);
    expect(container.querySelector("h2")?.textContent).toBe("Karte 0011");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 1, 0");
  });

  it("says_no_raster_matrix_or_header_and_no_codes_in_building", async () => {
    await show();
    expect(visibleWords(container)).not.toMatch(/\bM\d{4}\b|\bH\d{4}\b/);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    await show(5, 0);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    // A matrix without the sections, read anew.
    act(() => root.render(null));
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show();
    expect(container.textContent).toContain("Die Weltkarte hat keinen Abschnitt für Gebiete.");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("keeps_the_technical_words_in_tinkering", async () => {
    await show(1, 0, "tuefteln");
    expect(container.textContent).toContain("Raster 0003");
    expect(input("Header").value).toBe("412");
    expect(button("Header0412")).toBeDefined();
    act(() => root.render(null));
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show(1, 0, "tuefteln");
    expect(container.textContent).toContain("Das Raster hat keinen Header-Abschnitt (in der Schau unter „Raster“ einschaltbar).");
    await show(5, 0, "tuefteln");
    expect(container.textContent).toContain("außerhalb des Rasters (2 × 1)");
  });

  it("shows_the_section_Technisch_and_the_Dez_Hex_switch_only_in_tuefteln", async () => {
    const technical = () => [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent === "Technisch");
    const hexSwitch = () => [...container.querySelectorAll("button")].some((b) => b.textContent === "Hex");
    await show(1, 0, "bauen");
    expect(technical()).toBeUndefined();
    expect(hexSwitch()).toBe(false);
    await show(1, 0, "tuefteln");
    expect(technical()?.getAttribute("aria-expanded")).toBe("false");
    expect(hexSwitch()).toBe(true);
    await act(async () => void technical()?.click());
    expect(container.textContent).toContain("Raster 0003, Zelle 1, 0");
  });

  it("shows_the_raw_row_of_the_cell_in_hex_in_tuefteln_and_marks_the_chosen_cell", async () => {
    await show(1, 0, "bauen");
    expect(container.textContent).not.toContain("Rohdaten");
    await show(1, 0, "tuefteln");
    const head = [...container.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find((b) => b.textContent === "Rohdaten, Zeile 0");
    expect(head?.getAttribute("aria-expanded")).toBe("true");
    const table = container.querySelector("table[aria-label='Rohdaten, Zeile 0']")!;
    const row = (name: string) => [...table.querySelectorAll("tbody tr")].find((r) => r.querySelector("th")?.textContent === name)!;
    const values = (name: string) => [...row(name).querySelectorAll("td")].map((c) => c.textContent);
    expect(values("Karte")).toEqual(["000A", "000B"]);
    expect(values("Header")).toEqual(["019B", "019C"]);
    expect(values("Höhe")).toEqual(["0", "2"]);
    // The chosen cell (x = 1) is marked in every line; the other one is not.
    expect([...table.querySelectorAll("[aria-current='true']")].map((c) => c.textContent)).toEqual(["000B", "019C", "2"]);
    expect(row("Karte").querySelectorAll("td")[0]?.hasAttribute("aria-current")).toBe(false);
  });

  it("leaves_out_the_raw_lines_of_sections_the_matrix_lacks", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show(0, 0, "tuefteln");
    const table = container.querySelector("table[aria-label='Rohdaten, Zeile 0']")!;
    expect([...table.querySelectorAll("tbody th")].map((h) => h.textContent)).toEqual(["Karte"]);
  });

  it("offers_import_export_and_duplicate_in_the_head_menu_not_in_the_field_row", async () => {
    await show();
    expect(headActions(container)).toEqual(["Teile importieren…", "Teile exportieren…", "Duplizieren"]);
    // The buttons below the field „Karte“ and at the end of the form are gone.
    for (const old of ["Teile importieren…", "Teile exportieren…", "Karte duplizieren und hier eintragen", "Duplizieren"]) expect(button(old), old).toBeUndefined();
    chooseAction(container, "Teile importieren…");
    await flush();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Teile in Karte 0011 importieren");
    expect(document.querySelector("[data-import-dialog='11']")).not.toBeNull();
    await act(async () => void [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Abbrechen")!.click());
    await flush();
    chooseAction(container, "Teile exportieren…");
    await flush();
    expect(document.querySelector("[data-export-dialog='11']")).not.toBeNull();
  });

  it("offers_no_parts_and_no_duplicate_for_a_cell_without_a_map", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null] }));
    await show();
    expect(headAction(container, "Teile importieren…")).toBeUndefined();
    expect(headAction(container, "Teile exportieren…")).toBeUndefined();
    const duplicate = headAction(container, "Duplizieren");
    expect(duplicate?.getAttribute("aria-disabled") === "true" || duplicate?.hasAttribute("data-disabled")).toBe(true);
  });
});
