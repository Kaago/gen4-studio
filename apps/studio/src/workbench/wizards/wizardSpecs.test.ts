import { describe, expect, it } from "vitest";
import type { AreaReport, AreaSpec, HeaderSummary, MatrixDto } from "../../api";
import { areaLayout, areaResult, areaSpec, areaSummary, emptyAreaForm, parseMapList, rectSelection, sourceHeader, toggleSelection } from "./wizardSpecs";
import type { AreaForm } from "./wizardSpecs";

const header = (id: number, internal: string, matrix = 0, area = 0, event = 0): HeaderSummary => ({ id, internal_name: internal, location_name: "", matrix_id: matrix, area_data_id: area, event_file_id: event });
const headers = [header(0, "ZERO"), header(1, "NOTHING"), header(2, "TWO", 5, 7, 9)];

const form = (change: Partial<AreaForm> = {}): AreaForm => ({ ...emptyAreaForm, sourceMatrix: 0, ...change });
const cell = (x: number, y: number, map = 0) => ({ x, y, map });

describe("areaLayout", () => {
  it("takes_the_hull_of_the_selection_and_places_the_cells_relative_to_it", () => {
    const layout = areaLayout(form({ selected: [cell(4, 2, 11), cell(3, 2, 10), cell(3, 3, 12)], clearBuildings: true }));
    expect(layout.width).toBe(2);
    expect(layout.height).toBe(2);
    expect(layout.cells).toEqual([
      { x: 0, y: 0, source_map: 10, clear_buildings: true },
      { x: 1, y: 0, source_map: 11, clear_buildings: true },
      { x: 0, y: 1, source_map: 12, clear_buildings: true },
    ]);
  });

  it("appends_maps_picked_by_number_in_a_row_behind_the_hull", () => {
    const layout = areaLayout(form({ selected: [cell(3, 2, 10), cell(4, 2, 11), cell(3, 3, 12)], manualMaps: [7] }));
    expect(layout.cells[layout.cells.length - 1]).toEqual({ x: 0, y: 2, source_map: 7, clear_buildings: false });
    expect([layout.width, layout.height]).toEqual([2, 3]);
  });

  it("widens_the_grid_when_the_manual_row_is_longer_than_the_hull", () => {
    const layout = areaLayout(form({ selected: [cell(1, 1, 10)], manualMaps: [7, 8, 9] }));
    expect([layout.width, layout.height]).toEqual([3, 2]);
    expect(layout.cells.map((c) => [c.x, c.y, c.source_map])).toEqual([
      [0, 0, 10],
      [0, 1, 7],
      [1, 1, 8],
      [2, 1, 9],
    ]);
  });

  it("puts_manual_maps_into_the_first_row_when_nothing_is_selected", () => {
    const layout = areaLayout(form({ manualMaps: [7, 8] }));
    expect([layout.width, layout.height]).toEqual([2, 1]);
    expect(layout.cells.map((c) => [c.x, c.y])).toEqual([
      [0, 0],
      [1, 0],
    ]);
  });
});

describe("areaSpec", () => {
  it("refuses_an_empty_selection", () => {
    expect(areaSpec(form())).toEqual({ error: "Mindestens eine Karte wählen." });
  });

  it("refuses_more_than_255_columns_or_rows", () => {
    expect(areaSpec(form({ selected: [cell(0, 0), cell(255, 0)] }))).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect(areaSpec(form({ selected: [cell(0, 0), cell(0, 255)] }))).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect("spec" in areaSpec(form({ selected: [cell(0, 0), cell(254, 0)] }))).toBe(true);
  });

  it("refuses_a_blank_name", () => {
    expect(areaSpec(form({ selected: [cell(0, 0)], name: "   " }))).toEqual({ error: "Name darf nicht leer sein." });
  });

  it("builds_the_backend_request_with_the_assigned_header", () => {
    const result = areaSpec(form({ selected: [cell(2, 1, 521), cell(3, 1, 522)], name: " Route 230 ", header: 1, areaDataId: 7, eventFile: { Copy: 9 } }));
    expect(result).toEqual({
      spec: {
        name: "Route 230",
        width: 2,
        height: 1,
        cells: [
          { x: 0, y: 0, source_map: 521, clear_buildings: false },
          { x: 1, y: 0, source_map: 522, clear_buildings: false },
        ],
        with_headers: true,
        header: { id: 1, area_data_id: 7, event_file: { Copy: 9 } },
      },
    });
  });

  it("sends_no_header_assignment_when_none_is_chosen", () => {
    const result = areaSpec(form({ selected: [cell(0, 0)], withHeaders: false }));
    expect(result).toMatchObject({ spec: { with_headers: false, header: null } });
  });
});

describe("areaSummary", () => {
  const spec: AreaSpec = {
    name: "Route 230",
    width: 2,
    height: 1,
    cells: [
      { x: 0, y: 0, source_map: 521, clear_buildings: true },
      { x: 1, y: 0, source_map: 522, clear_buildings: true },
    ],
    with_headers: true,
    header: { id: 1, area_data_id: null, event_file: "New" },
  };

  it("says_what_will_be_created_and_that_it_cannot_be_undone", () => {
    expect(areaSummary(spec, headers)).toEqual([
      "Legt Raster „Route 230“ (2 × 1) mit Header-Abschnitt an.",
      "Kopiert Karte 0521 → Zelle (0, 0), Karte 0522 → Zelle (1, 0), ohne Gebäude.",
      "Stellt Header 0001 „NOTHING“ auf das neue Raster um, Eventdatei: neu, leer.",
      "Anlegen lässt sich nicht rückgängig machen.",
    ]);
  });

  it("names_area_data_and_the_other_event_file_choices_and_a_missing_header", () => {
    const kept = areaSummary({ ...spec, with_headers: false, cells: [{ x: 0, y: 0, source_map: 1, clear_buildings: false }], header: { id: 2, area_data_id: 3, event_file: { Copy: 9 } } }, headers);
    expect(kept[0]).toBe("Legt Raster „Route 230“ (2 × 1) an.");
    expect(kept[1]).toBe("Kopiert Karte 0001 → Zelle (0, 0), mit Gebäuden.");
    expect(kept[2]).toBe("Stellt Header 0002 „TWO“ auf das neue Raster um, Area-Data 0003, Eventdatei: Kopie von 0009.");
    expect(areaSummary({ ...spec, header: { id: 2, area_data_id: null, event_file: "Keep" } }, headers)[2]).toContain("Eventdatei: behalten");
    expect(areaSummary({ ...spec, header: null }, headers)[2]).toBe("Stellt keinen Header um.");
  });
});

describe("areaResult", () => {
  it("names_the_new_raster_and_its_maps_and_what_comes_next", () => {
    const spec: AreaSpec = { name: "Route 230", width: 2, height: 1, cells: [], with_headers: true, header: null };
    const report: AreaReport = { matrix: 131, maps: [[0, 0, 571], [1, 0, 572]], event_file: 55, header: 1 };
    const { lines, next } = areaResult(report, spec);
    expect(lines[0]).toBe("Raster 0131 „Route 230“ (2 × 1) angelegt.");
    expect(lines.join("\n")).toContain("Karten 0571, 0572");
    expect(lines.join("\n")).toContain("Header 0001 zeigt jetzt auf Raster 0131, Eventdatei 0055");
    expect(next).toEqual(["Eingang verbinden: Raum anlegen… oder Inspector › Ausgänge", "Karten prüfen: Raster-Tab"]);
  });

  it("uses_the_singular_and_leaves_out_a_header_that_was_not_assigned", () => {
    const spec: AreaSpec = { name: "A", width: 1, height: 1, cells: [], with_headers: false, header: null };
    const { lines } = areaResult({ matrix: 3, maps: [[0, 0, 9]], event_file: null, header: null }, spec);
    expect(lines).toEqual(["Raster 0003 „A“ (1 × 1) angelegt.", "Karte 0009 kopiert."]);
  });
});

describe("selection helpers", () => {
  const matrix: MatrixDto = { index: 0, name: "m", width: 3, height: 2, maps: [10, 11, null, 13, 14, 15], headers: [1, 1, 0, 2, 2, 2] };

  it("toggles_a_filled_cell_and_ignores_an_empty_one", () => {
    const one = toggleSelection([], matrix, 1, 0);
    expect(one).toEqual([cell(1, 0, 11)]);
    expect(toggleSelection(one, matrix, 1, 0)).toEqual([]);
    expect(toggleSelection(one, matrix, 2, 0)).toBe(one);
  });

  it("adds_the_filled_cells_of_the_rectangle_between_two_cells", () => {
    const picked = rectSelection([cell(0, 0, 10)], matrix, { x: 0, y: 0 }, { x: 2, y: 1 });
    expect(picked.map((c) => [c.x, c.y, c.map])).toEqual([
      [0, 0, 10],
      [1, 0, 11],
      [0, 1, 13],
      [1, 1, 14],
      [2, 1, 15],
    ]);
  });

  it("finds_the_header_of_the_first_selected_cell_that_has_one_in_row_order", () => {
    expect(sourceHeader(matrix, headers, [cell(0, 1, 13), cell(1, 0, 11)])?.id).toBe(1);
    expect(sourceHeader({ ...matrix, headers: null }, headers, [cell(0, 0, 10)])).toBeNull();
    expect(sourceHeader(matrix, headers, [])).toBeNull();
  });
});

describe("parseMapList", () => {
  it("reads_a_comma_or_space_separated_list_and_reports_what_it_cannot_use", () => {
    expect(parseMapList("7, 8 9", 600)).toEqual({ maps: [7, 8, 9], invalid: [] });
    expect(parseMapList("7, x, 700, 2.5", 600)).toEqual({ maps: [7], invalid: ["x", "700", "2.5"] });
    expect(parseMapList("  ", 600)).toEqual({ maps: [], invalid: [] });
  });
});
