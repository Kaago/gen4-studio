// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMatrix, getMatrix, renameMatrix, resizeMatrix, setMatrixSections } from "../api";
import type { HeaderSummary, MatrixDto, ProjectInfo } from "../api";
import { ask } from "../platform/dialogs";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { MatrixPanel } from "./MatrixPanel";
import type { MatrixPanelContext } from "./MatrixPanel";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  getMatrix: vi.fn(),
  createMatrix: vi.fn(),
  renameMatrix: vi.fn(),
  resizeMatrix: vi.fn(),
  setMatrixSections: vi.fn(),
}));
vi.mock("../platform/dialogs", () => ({ ask: vi.fn() }));
// The pictures are not under test: every cell shows its number plate.
vi.mock("../world/useThumbnail", () => ({ useThumbnail: () => null }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const info = { root: "/p", family: "Platinum" } as unknown as ProjectInfo;
const headers: HeaderSummary[] = [];
const matrix: MatrixDto = { index: 7, name: "Dorf", width: 2, height: 1, maps: [10, 11], headers: [415, 416], altitudes: [0, 0] };

describe("MatrixPanel", () => {
  let container: HTMLElement;
  let root: Root;
  const goTo = vi.fn();
  const onEdited = vi.fn();
  const onCountsChanged = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [getMatrix, createMatrix, renameMatrix, resizeMatrix, setMatrixSections, ask]) vi.mocked(f).mockReset();
    for (const f of [goTo, onEdited, onCountsChanged, onError]) f.mockReset();
    vi.mocked(getMatrix).mockResolvedValue(matrix);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const context = (over: Partial<MatrixPanelContext> = {}): MatrixPanelContext => ({
    info,
    headers,
    areaDataId: 0,
    counts: { Map: 600 },
    sceneMatrix: 7,
    revision: 0,
    selection: null,
    depth: "bauen",
    goTo,
    onEdited,
    onCountsChanged,
    onDraft: vi.fn(),
    onError,
    ...over,
  });
  const show = async (over: Partial<MatrixPanelContext> = {}) => {
    act(() => root.render(<MatrixPanel context={context(over)} />));
    await flush();
  };
  const field = (name: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${name}"]`)!;
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name)!;
  const commit = async (input: HTMLInputElement, text: string) => {
    await act(async () => input.focus());
    typeInto(input, text);
    await act(async () => input.blur());
    await flush();
  };

  it("says_when_no_project_is_open", async () => {
    await show({ info: null });
    expect(container.textContent).toContain("Kein Projekt geöffnet.");
  });

  it("asks_for_a_place_but_still_offers_a_new_matrix_while_no_header_is_selected", async () => {
    await show({ sceneMatrix: null });
    expect(container.textContent).toContain("Noch kein Ort gewählt.");
    expect(container.querySelector("input")).toBeNull();
    expect(button("Neue Matrix")).toBeDefined();
  });

  it("has_no_head_title_that_repeats_the_tab", async () => {
    await show();
    // The dock tab says "Raster"; the head keeps the matrix's number and its action.
    expect(container.querySelector("header h2")).toBeNull();
    expect(container.querySelector("header")?.textContent).toContain("0007 · Matrix");
    expect(container.querySelector("header")?.textContent).toContain("Neue Matrix");
  });

  it("heads_the_panel_with_the_matrix_and_fills_the_fields_from_it", async () => {
    await show();
    expect(container.querySelector("header")?.textContent).toContain("0007");
    expect(field("Name der Matrix").value).toBe("Dorf");
    expect(field("Breite").value).toBe("2");
    expect(field("Höhe").value).toBe("1");
    expect(container.querySelector('[role=switch][aria-checked="true"]')).not.toBeNull();
    expect(container.querySelectorAll("[role=switch]")).toHaveLength(2);
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
  });

  it("shows_the_raw_values_in_the_cells_only_in_tuefteln", async () => {
    await show();
    expect(container.querySelectorAll("[data-cell]")).toHaveLength(2);
    expect(container.querySelectorAll("[data-raw]")).toHaveLength(0);
    await show({ depth: "tuefteln" });
    expect([...container.querySelectorAll("[data-raw]")].map((e) => e.getAttribute("data-raw"))).toEqual(["0010 0415 0", "0011 0416 0"]);
  });

  it("renames_on_commit_and_reloads_through_the_owner", async () => {
    vi.mocked(renameMatrix).mockResolvedValue(undefined as never);
    await show();
    await commit(field("Name der Matrix"), "Neuer Name");
    expect(renameMatrix).toHaveBeenCalledWith(7, "Neuer Name");
    expect(onEdited).toHaveBeenCalledWith(7);
  });

  it("resizes_on_commit_and_refuses_a_width_beyond_255_naming_the_maximum", async () => {
    vi.mocked(resizeMatrix).mockResolvedValue(undefined as never);
    await show();
    await commit(field("Breite"), "3");
    expect(resizeMatrix).toHaveBeenCalledWith(7, 3, 1);
    vi.mocked(resizeMatrix).mockClear();
    await commit(field("Breite"), "300");
    expect(resizeMatrix).not.toHaveBeenCalled();
    expect(container.textContent).toContain("255");
  });

  it("asks_before_a_resize_that_loses_cells", async () => {
    vi.mocked(ask).mockResolvedValue(false);
    await show();
    await commit(field("Breite"), "1");
    expect(ask).toHaveBeenCalledOnce();
    expect(vi.mocked(ask).mock.calls[0][0]).toContain("außerhalb von 1 × 1");
    expect(resizeMatrix).not.toHaveBeenCalled();
  });

  it("asks_before_a_section_with_values_is_removed_and_keeps_it_when_refused", async () => {
    vi.mocked(ask).mockResolvedValue(false);
    await show();
    const header = [...container.querySelectorAll<HTMLElement>("[role=switch]")].find((s) => s.closest("label")?.textContent === "Header")!;
    await act(async () => header.click());
    await flush();
    expect(ask).toHaveBeenCalledOnce();
    expect(setMatrixSections).not.toHaveBeenCalled();
  });

  it("removes_an_empty_section_without_asking", async () => {
    vi.mocked(setMatrixSections).mockResolvedValue(undefined as never);
    await show();
    const heights = [...container.querySelectorAll<HTMLElement>("[role=switch]")].find((s) => s.closest("label")?.textContent === "Höhen")!;
    await act(async () => heights.click());
    await flush();
    expect(ask).not.toHaveBeenCalled();
    expect(setMatrixSections).toHaveBeenCalledWith(7, true, false);
  });

  it("shows_the_matrix_of_a_cell_chosen_by_a_link_not_the_scenes", async () => {
    // Gate 3d.4 B12 (b): „Matrix 0290, Zelle 0, 0“ followed while a room of matrix 0007 was selected: the grid showed 0007.
    vi.mocked(getMatrix).mockImplementation(async (i) => ({ ...matrix, index: i }));
    await show({ selection: { matrix: 290, x: 0, y: 0 } });
    expect(getMatrix).toHaveBeenLastCalledWith(290);
    expect(container.querySelector("header")?.textContent).toContain("0290 · Matrix");
    expect(container.querySelector('[data-cell="0,0"]')?.getAttribute("aria-pressed")).toBe("true");
    // A cell of the scene's matrix, or none: the scene's.
    await show({ selection: null });
    expect(container.querySelector("header")?.textContent).toContain("0007 · Matrix");
  });

  it("creates_a_matrix_and_shows_it_with_a_note_that_no_header_uses_it", async () => {
    vi.mocked(createMatrix).mockResolvedValue(9);
    vi.mocked(getMatrix).mockImplementation(async (i) => ({ ...matrix, index: i }));
    await show();
    await act(async () => button("Neue Matrix").click());
    await flush();
    expect(createMatrix).toHaveBeenCalledWith("neu");
    expect(goTo).toHaveBeenCalledWith({ kind: "matrixCell", matrix: 9, x: 0, y: 0 });
    expect(container.textContent).toContain("gehört noch zu keinem Header");
  });

  it("opens_the_cell_in_the_inspector_on_a_click_and_the_extend_dialog_from_a_plus_cell", async () => {
    await show();
    await act(async () => container.querySelector<HTMLElement>('[data-cell="1,0"]')!.click());
    expect(goTo).toHaveBeenCalledWith({ kind: "matrixCell", matrix: 7, x: 1, y: 0 });
    expect(document.querySelector("[role=dialog]")).toBeNull();
    await act(async () => container.querySelector<HTMLElement>('[data-extend="2,0"]')!.click());
    const dialog = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog).not.toBeNull();
    expect(dialog.textContent).toContain("Karte bei x 2, y 0 anlegen?");
    pressKey((document.activeElement as HTMLElement) ?? dialog, "Escape");
    await flush();
    expect(document.querySelector("[role=dialog]")).toBeNull();
  });
});
