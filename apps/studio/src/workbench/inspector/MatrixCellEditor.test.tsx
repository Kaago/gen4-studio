// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MATRIX_EMPTY, duplicateMap, getMap, getMapBudget, getMatrix, setMatrixCell, worldBake } from "../../api";
import type { MapDto } from "../../api";
import { EntityIdCache, TextProjectProvider } from "../../textproject/TextProjectContext";
import type { AssetId, HeaderSummary, MatrixDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { visibleWords } from "../../ui/visibleText";
import { MatrixCellEditor } from "./MatrixCellEditor";
import type { MatrixCellContext } from "./MatrixCellEditor";
import type { Area, World } from "../worldModel";
import { chooseAction, headAction, headActions } from "./headTesting";
import { MISSING, closeMenu, isDisabled, resetSpy, withReset } from "../../original/resetTesting";
import { deleteSpy, withDelete } from "../../delete/deleteTesting";
import type { TilesetDto, WorldGetDto } from "../../api.world";
import { INITIAL_PAINT, PaintContext } from "../paint/PaintContext";
import type { PaintModel } from "../paint/PaintContext";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getMatrix: vi.fn(), setMatrixCell: vi.fn(), duplicateMap: vi.fn(), inspectMapImport: vi.fn(async () => []), getMapBudget: vi.fn(), getMap: vi.fn(), worldBake: vi.fn() }));
// The palette's pictures: none (the role colours), no kernel asked.
vi.mock("../../world/paint/paletteImages", () => ({ tileImage: async () => null, peekTileImage: () => undefined, resetTileImages: () => {}, tileImagesGeneration: () => 0, onTileImagesReset: () => () => {} }));
// The world model the editor names areas and world maps from (Bauen); null until a test sets one.
const shownWorld: { world: World | null } = { world: null };
vi.mock("../WorkbenchContext", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../WorkbenchContext")>();
  return { ...actual, useWorld: () => ({ ...actual.NO_WORLD, world: shownWorld.world }) };
});

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
  let reset = resetSpy();
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
    vi.mocked(getMapBudget).mockReset().mockImplementation(async (map) => ({
      map,
      model: { used: 21452, max: 61440, state: "calm" },
      bdhc: { used: 300, max: 36864, state: "calm" },
      buildings: { used: 3, max: 32, state: "calm" },
      textures: null,
      textures_missing: "Kein Gebiet zeigt diese Karte.",
    }));
    for (const fn of [goTo, onEdited, onCountsChanged, onError]) fn.mockReset();
    dirty = [];
    reset = resetSpy();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  async function show(x = 1, y = 0, depth: "erzaehlen" | "bauen" | "tuefteln" = "bauen") {
    await act(async () => root.render(withReset(reset, <MatrixCellEditor matrix={3} x={x} y={y} context={context()} depth={depth} />)));
    await flush();
  }
  /** Shows the editor anew, so it reads its matrix again. */
  async function remount() {
    await act(async () => root.render(null));
    await show();
  }
  const input = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label='${label}']`)!;
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;
  const typeAndBlur = async (label: string, text: string) => {
    await act(async () => void input(label).focus());
    typeInto(input(label), text);
    await act(async () => void input(label).blur());
    await flush();
  };

  it("the_source_section_stands_above_the_budget_and_reads_the_world_in_a_text_project", async () => {
    // Outside a text project every map is binary: nothing is read, adoption is locked.
    await show();
    const source = container.querySelector<HTMLElement>("[data-world-source]");
    expect(source?.getAttribute("data-world-source")).toBe("binary");
    expect(source?.compareDocumentPosition(container.querySelector("[data-map-budget]")!) ?? 0).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(container.querySelector<HTMLButtonElement>("[data-world-adopt]")?.disabled).toBe(true);
    expect(getMap).not.toHaveBeenCalled();
    // In a text project the map's flag decides.
    vi.mocked(getMap).mockReset().mockResolvedValue({ world: true } as MapDto);
    vi.mocked(worldBake).mockReset().mockResolvedValue(null);
    await act(async () =>
      root.render(
        <TextProjectProvider active ids={new EntityIdCache()} onRenamed={() => {}}>
          {withReset(reset, <MatrixCellEditor matrix={3} x={1} y={0} context={context()} depth="bauen" />)}
        </TextProjectProvider>,
      ),
    );
    await flush();
    expect(getMap).toHaveBeenCalledExactlyOnceWith(11);
    expect(container.querySelector("[data-world-source]")?.getAttribute("data-world-source")).toBe("world");
    expect(worldBake).toHaveBeenCalledWith(11);
  });

  describe("painting", () => {
    const world: WorldGetDto = {
      map: 11,
      world: {
        version: 1,
        tileset: 3,
        layers: [
          { name: "Boden", role: "ground", tiles: [] },
          { name: "Büsche", role: "overlay", tiles: [] },
        ],
        permissions: { derive: true },
      },
      faces: 1024,
      textures: 2,
      estimate: 30000,
      share: 0.5,
      bake: null,
    };
    const tileset: TilesetDto = {
      package: 3,
      stored: true,
      tileset: {
        version: 1,
        package: 3,
        tiles: [{ texture: "weg", texture_index: 0, size: [32, 32], role: "ground", kind: 0, collision: 0, height: "flat", origin: "vanilla" }],
      },
    };
    const model = (): PaintModel => ({ state: INITIAL_PAINT, set: vi.fn(), target: 11, world, tileset, hidden: new Set(), toggleHidden: vi.fn() });
    async function showPainting(painting: boolean) {
      await act(async () =>
        root.render(
          <PaintContext.Provider value={model()}>
            {withReset(reset, <MatrixCellEditor matrix={3} x={1} y={0} context={{ ...context(), painting }} depth="bauen" />)}
          </PaintContext.Provider>,
        ),
      );
      await flush();
    }

    it("paint_sections_show_for_a_world_map_while_painting", async () => {
      await showPainting(true);
      const palette = container.querySelector("[data-paint-palette]");
      const layers = container.querySelector("[data-paint-layers]");
      expect(palette?.querySelector('[data-palette-tile="0000"]')?.getAttribute("aria-pressed")).toBe("true");
      expect(layers?.querySelector('[data-layer-row="1"]')?.textContent).toContain("Büsche · Überlagerung");
      // Palette, then layers, both before „Quelle“ and „Budget“.
      expect(palette!.compareDocumentPosition(layers!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
      expect(layers!.compareDocumentPosition(container.querySelector("[data-world-source]")!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
      expect(layers!.compareDocumentPosition(container.querySelector("[data-map-budget]")!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    });

    it("paint_sections_hide_for_another_tool", async () => {
      await showPainting(false);
      expect(container.querySelector("[data-paint-palette]")).toBeNull();
      expect(container.querySelector("[data-paint-layers]")).toBeNull();
      expect(container.querySelector("[data-map-budget]")).not.toBeNull();
      // Another cell than the painted map shows none either.
      await act(async () =>
        root.render(
          <PaintContext.Provider value={model()}>
            {withReset(reset, <MatrixCellEditor matrix={3} x={0} y={0} context={{ ...context(), painting: true }} depth="bauen" />)}
          </PaintContext.Provider>,
        ),
      );
      await flush();
      expect(container.querySelector("[data-paint-palette]")).toBeNull();
    });
  });

  it("a_cell_with_a_map_shows_its_budget", async () => {
    await show();
    expect(container.querySelector("[data-map-budget]")?.textContent).toContain("Grenzen des Spiels; Speichern geht immer.");
    expect(container.querySelector('[data-budget="model"]')?.textContent).toContain("21.452 von 61.440 B");
    expect(getMapBudget).toHaveBeenLastCalledWith(11);
    // The map changed behind the editor (its terrain, model or buildings): the budget reads again.
    const reads = vi.mocked(getMapBudget).mock.calls.length;
    await act(async () => root.render(withReset(reset, <MatrixCellEditor matrix={3} x={1} y={0} context={{ ...context(), mapRevision: 1 }} depth="bauen" />)));
    await flush();
    expect(vi.mocked(getMapBudget).mock.calls.length).toBe(reads + 1);
    // Tüfteln shows it too.
    await show(1, 0, "tuefteln");
    expect(container.querySelector("[data-map-budget]")).not.toBeNull();
    // Not in Erzählen.
    vi.mocked(getMapBudget).mockClear();
    await show(1, 0, "erzaehlen");
    expect(container.querySelector("[data-map-budget]")).toBeNull();
    expect(getMapBudget).not.toHaveBeenCalled();
    // Not for a cell without a map.
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null] }));
    await act(async () => root.render(null));
    await show(1, 0);
    expect(container.querySelector("h2")?.textContent).toBe("Leere Zelle");
    expect(container.querySelector("[data-map-budget]")).toBeNull();
    expect(getMapBudget).not.toHaveBeenCalled();
  });

  it("names_the_cell_and_the_matrix", async () => {
    await show();
    expect(container.querySelector("header[data-inspector-kind='map'] [data-eyebrow]")?.textContent).toBe("Karte");
    // Bauen: the map of the cell, and where it lies, in the words of the hierarchy (no matrix or header codes).
    expect(container.querySelector("h2")?.textContent).toBe("Karte 0011");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zelle 1, 0 · Haus von Lucas");
    expect(input("Karte").value).toBe("11");
    // Bauen: the area the cell belongs to by its name, no number to type.
    expect(input("Gebiet")).toBeNull();
    expect(button("Haus von Lucas")).toBeDefined();
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
    await show(1, 0, "tuefteln");
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

  it("leads_the_area_of_the_cell_to_its_editor", async () => {
    await show();
    await act(async () => void button("Haus von Lucas").click());
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 412 });
  });

  it("picks_the_area_of_a_cell_by_name_in_bauen_with_the_landscape_and_each_section", async () => {
    const area = (key: string, name: string, ids: number[]): Area => ({ key, name, placeKey: name, headers: ids.map(summary), rooms: [], matrix: 3 });
    shownWorld.world = {
      worldMaps: [{ key: "w3", matrix: 3, filter: null, name: "Sinnoh", areas: [area("w3:0", "Zweiblattdorf", [411]), area("w3:1", "Route 204", [413, 414])] }],
      moreAreas: [area("m9", "Großmoor", [420])].map((a) => ({ ...a, matrix: 9 })),
      unconnected: [],
      other: [],
      areaOf: new Map(),
      roomCount: 0,
      assignedRooms: 0,
    };
    try {
      await show();
      expect(container.textContent).toContain("Gehört zu");
      await act(async () => void container.querySelector<HTMLButtonElement>("button[data-cell-owner]")!.click());
      await flush();
      const options = [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
      expect(options.map((o) => o.textContent)).toEqual([
        expect.stringContaining("Landschaft"),
        "Zweiblattdorf",
        "Route 204, Abschnitt 1",
        "Route 204, Abschnitt 2",
      ]);
      await act(async () => void options[3].click());
      await flush();
      expect(setMatrixCell).toHaveBeenLastCalledWith(3, 1, 0, null, 414, null);
      // A cell outside is named by its world map, not by a number.
      await show(5, 0);
      expect(container.querySelector("[data-ident]")?.textContent).toBe("Sinnoh");
    } finally {
      shownWorld.world = null;
    }
  });

  it("calls_a_landscape_cell_landscape_in_bauen", async () => {
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: [411, 0] }));
    await show();
    expect(container.textContent).toContain("Gehört zuLandschaft");
    expect(button("Landschaft")).toBeUndefined();
  });

  it("says_nothing_of_a_missing_area_section_in_bauen", async () => {
    // A room's cells have no area section; Bauen does not speak of the world map there, it just leaves the field out.
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show();
    expect(container.textContent).not.toContain("Weltkarte");
    expect(container.textContent).not.toContain("Abschnitt");
    expect(container.textContent).not.toContain("Höhenabschnitt");
    expect(input("Gebiet")).toBeNull();
    expect(container.textContent).not.toContain("Gehört zu");
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
    expect(container.querySelector("p")?.textContent).toBe("Die Zelle liegt außerhalb der Karten (2 × 1).");
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
    // No header number either: the area is named.
    expect(visibleWords(container)).not.toMatch(/0412/);
    await show(5, 0);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    // Outside, without a world model: the matrix is named by the room that uses it, not by its number.
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Zweiblattdorf");
    // A matrix without the sections, read anew.
    act(() => root.render(null));
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ headers: null, altitudes: null }));
    await show();
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header|Weltkarte/);
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
    expect(headActions(container)).toEqual(["Teile importieren…", "Teile exportieren…", "Duplizieren", "Löschen…", "Auf Original zurücksetzen…"]);
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

  it("head_offers_loeschen_for_the_map_and_shows_its_state", async () => {
    const del = deleteSpy({ "map:11": "new" });
    await act(async () => root.render(withReset(reset, withDelete(del, <MatrixCellEditor matrix={3} x={1} y={0} context={context()} depth="bauen" />))));
    await flush();
    // The cell holds map 11: „Löschen…“ after „Duplizieren“ deletes that map; the head names how it stands against the base.
    chooseAction(container, "Löschen…");
    expect(del.open).toHaveBeenCalledWith({ kind: "map", index: 11 });
    expect(container.querySelector("header[data-inspector-kind] [data-entity-state]")?.textContent).toBe("neu");
    // An empty cell has no map to delete.
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null] }));
    await act(async () => root.render(null));
    await act(async () => root.render(withReset(reset, withDelete(del, <MatrixCellEditor matrix={3} x={1} y={0} context={context()} depth="bauen" />))));
    await flush();
    expect(headAction(container, "Löschen…")).toBeUndefined();
    expect(container.querySelector("[data-entity-state]")).toBeNull();
  });

  it("head_offers_reset_of_the_map", async () => {
    await show();
    // The cell holds map 11: the entry resets that map, as one undo step after the question.
    chooseAction(container, "Auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenCalledWith({ kind: "map", index: 11 });
    // A map the base does not have is new: locked, with the reason.
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, 650] }));
    await remount();
    const item = headAction(container, "Auf Original zurücksetzen…");
    expect(isDisabled(item)).toBe(true);
    expect(item?.textContent).toBe("Auf Original zurücksetzen…Kein Original");
    closeMenu();
    // Without a base ROM the entry says so.
    reset = resetSpy(MISSING);
    vi.mocked(getMatrix).mockImplementation(async () => matrix());
    await remount();
    expect(headActions(container)).toContain("Auf Original zurücksetzen…Basis-ROM fehlt");
    // An empty cell has no map to reset.
    vi.mocked(getMatrix).mockImplementation(async () => matrix({ maps: [10, null] }));
    await remount();
    expect(headAction(container, "Auf Original zurücksetzen…")).toBeUndefined();
  });
});
