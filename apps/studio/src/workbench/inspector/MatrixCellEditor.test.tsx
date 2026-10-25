// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MATRIX_EMPTY, duplicateMap, getMatrix, setMatrixCell } from "../../api";
import type { AssetId, MatrixDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { MatrixCellEditor } from "./MatrixCellEditor";
import type { MatrixCellContext } from "./MatrixCellEditor";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn(), setMatrixCell: vi.fn(), duplicateMap: vi.fn(), inspectMapImport: vi.fn(async () => []) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
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
  const context = (): MatrixCellContext => ({ dirty, headerCount: 500, goTo, counts: { Map: 600 }, revision: 0, onEdited, onCountsChanged, onError, onMapImported, checks: null });
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
    expect(container.querySelector("h2")?.textContent).toBe("Zelle 1, 0");
    expect(container.textContent).toContain("Raster 0003");
    expect(input("Karte").value).toBe("11");
    expect(input("Header").value).toBe("412");
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
    await typeAndBlur("Header", "413");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, 413, null);
    await typeAndBlur("Höhe", "9");
    expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, null, 9);
    await typeAndBlur("Höhe", "300");
    expect(container.textContent).toContain("höchstens 255");
    await typeAndBlur("Header", "600");
    expect(container.textContent).toContain("höchstens 499");
    expect(setMatrixCell).toHaveBeenCalledTimes(2);
  });

  it("leads_the_header_of_the_cell_to_its_editor", async () => {
    await show();
    await act(async () => void button("Header0412").click());
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 412 });
  });

  it("says_which_sections_the_matrix_lacks", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show();
    expect(container.textContent).toContain("Das Raster hat keinen Header-Abschnitt");
    expect(container.textContent).toContain("Das Raster hat keinen Höhenabschnitt");
    expect(input("Header")).toBeNull();
  });

  it("duplicates_the_map_into_the_cell_and_refreshes_the_counts", async () => {
    await show();
    await act(async () => void button("Karte duplizieren und hier eintragen").click());
    await flush();
    expect(duplicateMap).toHaveBeenCalledWith(11);
    expect(setMatrixCell).toHaveBeenCalledWith(3, 1, 0, 77, null, null);
    expect(onCountsChanged).toHaveBeenCalledTimes(1);
  });

  it("says_a_cell_outside_the_matrix_is_outside", async () => {
    await show(5, 0);
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

  it("opens_the_import_and_export_dialogs_of_the_cell_map_from_the_field_Karte", async () => {
    await show();
    await act(async () => void button("Teile importieren…").click());
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Teile in Karte 0011 importieren");
    expect(document.querySelector("[data-import-dialog='11']")).not.toBeNull();
    await act(async () => void [...document.querySelectorAll<HTMLButtonElement>("[role=dialog] button")].find((b) => b.textContent === "Abbrechen")!.click());
    await flush();
    await act(async () => void button("Teile exportieren…").click());
    expect(document.querySelector("[data-export-dialog='11']")).not.toBeNull();
  });

  it("offers_no_import_for_a_cell_without_a_map", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null] }));
    await show();
    expect(button("Teile importieren…")).toBeUndefined();
  });
});
