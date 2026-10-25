import { describe, expect, it } from "vitest";
import type { AreaReport, AreaSpec, EventsDto, HeaderSummary, MatrixDto, RoomReport, RoomSpec } from "../../api";
import { areaLayout, areaResult, areaSpec, areaSummary, emptyAreaForm, internalName, mapUnderTile, partialText, previewArea, emptyRoomForm, isLateFailure, parseMapList, rectSelection, roomResult, roomSpec, roomSummary, sourceHeader, specialHeader, templateWarp, toggleSelection } from "./wizardSpecs";
import type { AreaForm, RoomForm } from "./wizardSpecs";

const header = (id: number, internal: string, matrix = 0, area = 0, event = 0): HeaderSummary => ({ id, internal_name: internal, location_index: 0, kanto: false, location_name: "", matrix_id: matrix, area_data_id: area, event_file_id: event, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
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

describe("emptyAreaForm", () => {
  it("gives_the_new_header_an_empty_event_file_instead_of_sharing_the_old_one", () => {
    expect(emptyAreaForm.eventFile).toBe("New");
  });
});

describe("areaSpec", () => {
  it("refuses_an_empty_selection", () => {
    expect(areaSpec(form(), 0)).toEqual({ error: "Mindestens eine Karte wählen." });
  });

  it("refuses_more_than_255_columns_or_rows", () => {
    expect(areaSpec(form({ selected: [cell(0, 0), cell(255, 0)] }), 0)).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect(areaSpec(form({ selected: [cell(0, 0), cell(0, 255)] }), 0)).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect("spec" in areaSpec(form({ selected: [cell(0, 0), cell(254, 0)] }), 0)).toBe(true);
  });

  it("refuses_a_blank_name", () => {
    expect(areaSpec(form({ selected: [cell(0, 0)], name: "   " }), 0)).toEqual({ error: "Name darf nicht leer sein." });
  });

  it("builds_the_backend_request_with_the_assigned_header", () => {
    const result = areaSpec(form({ selected: [cell(2, 1, 521), cell(3, 1, 522)], name: " Route 230 ", header: 1, areaDataId: 7, eventFile: { Copy: 9 } }), 0);
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
    const result = areaSpec(form({ selected: [cell(0, 0)], withHeaders: false }), 0);
    expect(result).toMatchObject({ spec: { with_headers: false, header: null } });
  });
});

describe("internalName", () => {
  it("makes_upper_case_words_joined_by_underscores", () => {
    expect(internalName("Route 230")).toBe("ROUTE_230");
    expect(internalName("  Neues   Gebiet ")).toBe("NEUES_GEBIET");
    expect(internalName("route_230")).toBe("ROUTE_230");
  });

  it("drops_everything_that_is_not_ascii_and_never_expands_it", () => {
    expect(internalName("Straße")).toBe("STRAE");
    expect(internalName("Café-Bar")).toBe("CAFBAR");
  });

  it("falls_back_to_newmap_when_nothing_is_left", () => {
    expect(internalName("")).toBe("NEWMAP");
    expect(internalName("   ")).toBe("NEWMAP");
    expect(internalName("ß!?")).toBe("NEWMAP");
  });

  it("keeps_at_most_16_bytes", () => {
    expect(internalName("x".repeat(20))).toHaveLength(16);
    expect(internalName("Route 230 der Weg zum Gipfel")).toBe("ROUTE_230_DER_WE");
  });
});

describe("areaSpec with a new header", () => {
  const picked = { selected: [cell(0, 0, 521)], name: "Route 230" };

  it("takes_the_next_number_and_names_the_template_and_the_internal_name", () => {
    const result = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: "ROUTE_230" }, areaDataId: 7 }), 470);
    expect(result).toMatchObject({ spec: { header: { id: 470, area_data_id: 7, event_file: "New", new_header: { template: 342, internal_name: "ROUTE_230" } } } });
  });

  it("derives_the_internal_name_from_the_area_name_until_one_is_typed", () => {
    const derived = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: null } }), 470);
    expect(derived).toMatchObject({ spec: { header: { new_header: { internal_name: "ROUTE_230" } } } });
    const typed = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: "Mein Weg" } }), 470);
    expect(typed).toMatchObject({ spec: { header: { new_header: { internal_name: "MEIN_WEG" } } } });
  });

  it("ignores_a_chosen_existing_header", () => {
    const result = areaSpec(form({ ...picked, header: 3, newHeader: { template: 342, internalName: null } }), 470);
    expect(result).toMatchObject({ spec: { header: { id: 470 } } });
  });

  it("sends_no_new_header_for_an_existing_one", () => {
    const result = areaSpec(form({ ...picked, header: 3 }), 470);
    expect("spec" in result && result.spec.header).toEqual({ id: 3, area_data_id: null, event_file: "New" });
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

  it("warns_when_the_header_is_in_use", () => {
    const link = (dest: number, index: number) => ({ event_file: 5, index, x: 0, z: 0, dest_header_id: dest, dest_warp_id: 0 });
    const used = areaSummary(spec, headers, [link(1, 0), link(1, 1), link(2, 2)]);
    expect(used).toContain(`Achtung: Header 0001 wird benutzt (2 Warps führen hierher); er verliert sein bisheriges Raster ${String(headers[1].matrix_id).padStart(4, "0")}.`);
    expect(used[used.length - 1]).toBe("Anlegen lässt sich nicht rückgängig machen.");
    expect(areaSummary(spec, headers, [link(2, 0)]).some((l) => l.startsWith("Achtung"))).toBe(false);
    expect(areaSummary({ ...spec, header: null }, headers, [link(1, 0)]).some((l) => l.startsWith("Achtung"))).toBe(false);
    expect(roomSummary({ name: "Haus", source_map: 1, clear_buildings: false, header: { id: 2, area_data_id: null, event_file: "New" }, entrance: null }, headers, undefined, [link(2, 0)])).toContain(
      `Achtung: Header 0002 wird benutzt (1 Warp führt hierher); er verliert sein bisheriges Raster ${String(headers[2].matrix_id).padStart(4, "0")}.`,
    );
  });
});

describe("areaSummary with a new header", () => {
  const spec: AreaSpec = {
    name: "Route 230",
    width: 1,
    height: 1,
    cells: [{ x: 0, y: 0, source_map: 521, clear_buildings: false }],
    with_headers: true,
    header: { id: 470, area_data_id: 3, event_file: "New", new_header: { template: 342, internal_name: "ROUTE_230" } },
  };

  it("says_that_the_header_is_made_and_whose_copy_it_is", () => {
    expect(areaSummary(spec, headers)[2]).toBe("Der Assistent legt Header 0470 „ROUTE_230“ neu an (Kopie von 0342) und stellt ihn auf das neue Raster um, Area-Data 0003, Eventdatei: neu, leer.");
  });

  it("says_that_keeping_the_event_file_shares_the_templates", () => {
    const kept = areaSummary({ ...spec, header: { ...spec.header!, event_file: "Keep" } }, headers)[2];
    expect(kept).toContain("Eventdatei: behalten (geteilt mit Header 0342)");
  });

  it("does_not_warn_about_use_for_a_header_that_does_not_exist_yet", () => {
    const link = { event_file: 5, index: 0, x: 0, z: 0, dest_header_id: 470, dest_warp_id: 0 };
    expect(areaSummary(spec, headers, [link]).some((l) => l.startsWith("Achtung"))).toBe(false);
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
    const created = { ...spec, header: { id: 593, area_data_id: null, event_file: "New" as const, new_header: { template: 342, internal_name: "R231" } } };
    expect(areaResult({ ...report, header: 593 }, created).lines).toContain("Neuer Header 0593 „R231“ (Kopie von 0342) zeigt jetzt auf Raster 0131, Eventdatei 0055.");
    expect(next).toEqual(["Mit der Welt verbinden: Werkzeug „Tür“ oder „+“ › „Tür“ in „In dieser Szene“ (Ziel im Event-Editor wählen)", "Karten prüfen: Raster-Tab"]);
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

  it("passes_over_the_filler_header_0_and_prefers_the_header_the_wizard_was_opened_for", () => {
    // Gate 3b.1 (A) 16: the forest cells around the town belong to header 0, and the first selected cell was one.
    const town: MatrixDto = { index: 0, name: "m", width: 3, height: 2, maps: [10, 11, 12, 13, 14, 15], headers: [0, 0, 0, 0, 2, 1] };
    const picked = [cell(0, 0, 10), cell(1, 1, 14), cell(2, 1, 15)];
    expect(sourceHeader(town, headers, picked)?.id).toBe(2);
    expect(sourceHeader(town, headers, picked, 1)?.id).toBe(1);
    // A preferred header no selected cell has does not count; only filler left: header 0 after all.
    expect(sourceHeader(town, headers, picked, 7)?.id).toBe(2);
    expect(sourceHeader(town, headers, [cell(0, 0, 10)])?.id).toBe(0);
  });
});

describe("specialHeader", () => {
  const h = (id: number, internal_name: string, map_type?: number) => ({ id, internal_name, map_type });
  it("names_the_default_header_the_underground_and_the_union_room", () => {
    // Gate 3a (A) 12: header 0002 "UG" kept the Underground's map type, and a room on it stayed black.
    expect(specialHeader(h(0, "EVERYWHERE", 2))).toContain("Vorgabe-Header");
    expect(specialHeader(h(2, "UG", 6))).toContain("Untergrund");
    expect(specialHeader(h(3, "UG"))).toContain("Untergrund");
    expect(specialHeader(h(9, "MY_UG_COPY", 6))).toContain("Untergrund");
    expect(specialHeader(h(2, "UNION", 4))).toContain("Union-Raum");
    expect(specialHeader(h(1, "NOTHING", 0))).toBeNull();
    expect(specialHeader(h(415, "T01R0202", 4))).toBeNull();
  });

  it("warns_in_the_summary_when_a_special_header_is_given_away", () => {
    const ug = { ...header(2, "UG"), map_type: 6 };
    const spec: AreaSpec = { name: "A", width: 1, height: 1, cells: [{ x: 0, y: 0, source_map: 1, clear_buildings: false }], with_headers: true, header: { id: 2, area_data_id: null, event_file: "New" } };
    expect(areaSummary(spec, [ug])).toContain("Achtung: Header 0002 ist ein Sonder-Header (Untergrund: das Spiel startet dort den Untergrund, ein Raum bleibt schwarz).");
    expect(areaSummary(spec, [header(2, "TWO")]).some((l) => l.includes("Sonder-Header"))).toBe(false);
  });
});

describe("partialText", () => {
  it("says_in_german_what_was_created_and_which_step_failed", () => {
    expect(partialText("header 0593 was created; map copy failed: unpacked/maps/0670: Permission denied (os error 13)")).toBe(
      "Angelegt: Header 0593. Danach fehlgeschlagen: Karte kopieren (unpacked/maps/0670: Permission denied (os error 13)).",
    );
    expect(partialText("header 0470, matrix 0001, maps 0003, 0004 and event file 0002 were created; warp edit failed: boom")).toBe(
      "Angelegt: Header 0470, Raster 0001, Karten 0003, 0004, Eventdatei 0002. Danach fehlgeschlagen: Warp setzen (boom).",
    );
    expect(partialText("map 0003 was created; door tile failed: x")).toBe("Angelegt: Karte 0003. Danach fehlgeschlagen: Tür-Kachel setzen (x).");
    // Anything else stays as it came.
    expect(partialText("something else")).toBe("something else");
  });
});

describe("parseMapList", () => {
  it("reads_a_comma_or_space_separated_list_and_reports_what_it_cannot_use", () => {
    expect(parseMapList("7, 8 9", 600)).toEqual({ maps: [7, 8, 9], invalid: [] });
    expect(parseMapList("7, x, 700, 2.5", 600)).toEqual({ maps: [7], invalid: ["x", "700", "2.5"] });
    expect(parseMapList("  ", 600)).toEqual({ maps: [], invalid: [] });
  });
});

const roomForm = (change: Partial<RoomForm> = {}): RoomForm => ({ ...emptyRoomForm, template: 7, templateMap: 321, header: 1, outsideHeader: 0, x: 12, z: 7, ...change });
const place = (id: number, location: string, internal: string): HeaderSummary => ({ ...header(id, internal), location_name: location });

describe("roomSpec", () => {
  it("asks_for_a_template_first", () => {
    expect(roomSpec(roomForm({ template: null }), 0)).toEqual({ error: "Vorlage wählen." });
    expect(roomSpec(roomForm({ templateMap: null }), 0)).toEqual({ error: "Vorlage wählen." });
  });

  it("asks_for_the_header_and_for_the_outside_header_only_when_connecting", () => {
    expect(roomSpec(roomForm({ header: null }), 0)).toEqual({ error: "Header wählen." });
    expect(roomSpec(roomForm({ outsideHeader: null }), 0)).toEqual({ error: "Außen-Header wählen." });
    expect("spec" in roomSpec(roomForm({ outsideHeader: null, connect: false }), 0)).toBe(true);
  });

  it("builds_the_request_with_the_entrance", () => {
    const built = roomSpec(roomForm({ name: " Haus ", areaDataId: 3, eventFile: { Copy: 9 }, clearBuildings: true, insideX: 5, insideZ: 6 }), 0);
    expect(built).toEqual({
      spec: {
        name: "Haus",
        source_map: 321,
        clear_buildings: true,
        header: { id: 1, area_data_id: 3, event_file: { Copy: 9 } },
        entrance: { outside_header: 0, x: 12, z: 7, inside_x: 5, inside_z: 6, door_tiles: true },
      },
    });
  });

  it("refuses_a_door_outside_in_an_empty_cell_or_a_cell_of_another_header", () => {
    const grid = { width: 3, height: 1, maps: [10, null, 12], headers: [0, 0, 4] };
    expect("spec" in roomSpec(roomForm({ x: 12, z: 7 }), 0, { outsideMatrix: grid })).toBe(true);
    expect(roomSpec(roomForm({ x: 40, z: 7 }), 0, { outsideMatrix: grid })).toEqual({ error: "Kachel x 40, z 7 liegt in Zelle (1, 0) ohne Karte." });
    expect(roomSpec(roomForm({ x: 70, z: 7 }), 0, { outsideMatrix: grid })).toMatchObject({ error: expect.stringContaining("von Header 0004, nicht von 0000") });
    // Without a header section every cell is the outside header's.
    expect("spec" in roomSpec(roomForm({ x: 70, z: 7 }), 0, { outsideMatrix: { ...grid, headers: null } })).toBe(true);
  });

  it("refuses_door_tiles_that_already_have_a_warp_the_game_would_take_first", () => {
    const warp = (index: number, x: number, z: number) => ({ index, x, z, dest_header_id: 4, dest_warp_id: 0, height: 0 });
    expect(roomSpec(roomForm(), 0, { outsideWarps: [warp(3, 12, 7)] })).toMatchObject({ error: expect.stringContaining("außen liegt schon Warp 3") });
    expect("spec" in roomSpec(roomForm(), 0, { outsideWarps: [warp(3, 12, 8)] })).toBe(true);
    // Inside only the file the room keeps counts: a copy gets its warp repointed, a new file has none.
    const kept = { file: 9, warps: [warp(0, 5, 6)] };
    expect(roomSpec(roomForm({ eventFile: "Keep", insideX: 5, insideZ: 6 }), 0, { keptWarps: kept })).toMatchObject({ error: expect.stringContaining("Warp 0 der Eventdatei 0009") });
    expect("spec" in roomSpec(roomForm({ eventFile: { Copy: 9 }, insideX: 5, insideZ: 6 }), 0, { keptWarps: kept })).toBe(true);
  });

  it("makes_no_entrance_when_connecting_later", () => {
    const built = roomSpec(roomForm({ connect: false }), 0);
    expect("spec" in built && built.spec.entrance).toBeNull();
  });

  it("refuses_an_inside_tile_off_the_map_but_only_when_connecting", () => {
    expect(roomSpec(roomForm({ insideX: 40 }), 0)).toEqual({ error: "Kachel liegt außerhalb der Karte (0 bis 31)." });
    expect(roomSpec(roomForm({ insideZ: -1 }), 0)).toEqual({ error: "Kachel liegt außerhalb der Karte (0 bis 31)." });
    expect("spec" in roomSpec(roomForm({ insideX: 40, connect: false }), 0)).toBe(true);
  });

  it("asks_for_the_door_outside_when_it_is_not_entered_yet", () => {
    expect("error" in roomSpec(roomForm({ x: null }), 0)).toBe(true);
    expect("error" in roomSpec(roomForm({ z: null }), 0)).toBe(true);
    expect("spec" in roomSpec(roomForm({ x: null, z: null, connect: false }), 0)).toBe(true);
  });

  it("refuses_a_door_outside_the_outside_headers_matrix", () => {
    const size = { width: 2, height: 1 };
    expect("spec" in roomSpec(roomForm({ x: 63, z: 31 }), 0, size)).toBe(true);
    expect(roomSpec(roomForm({ x: 64, z: 7 }), 0, size)).toEqual({ error: "Kachel x 64, z 7 liegt außerhalb des Rasters (x 0 bis 63, z 0 bis 31)." });
    expect("error" in roomSpec(roomForm({ x: 3, z: 32 }), 0, size)).toBe(true);
    expect("error" in roomSpec(roomForm({ x: -1 }), 0, size)).toBe(true);
    // An unknown matrix is not checked.
    expect("spec" in roomSpec(roomForm({ x: 9999 }), 0, null)).toBe(true);
  });

  it("refuses_the_rooms_own_header_as_the_header_outside", () => {
    expect("error" in roomSpec(roomForm({ outsideHeader: 1 }), 0)).toBe(true);
  });

  it("refuses_an_empty_name", () => {
    expect(roomSpec(roomForm({ name: "  " }), 0)).toEqual({ error: "Name darf nicht leer sein." });
  });
});

describe("roomSpec with a new header", () => {
  it("takes_the_next_number_and_needs_no_chosen_header", () => {
    const built = roomSpec(roomForm({ header: null, template: 342, newHeader: { template: 342, internalName: null }, name: "Haus" }), 470);
    expect(built).toMatchObject({ spec: { header: { id: 470, event_file: "New", new_header: { template: 342, internal_name: "HAUS" } } } });
  });

  it("refuses_a_new_header_as_the_header_outside", () => {
    expect("error" in roomSpec(roomForm({ header: null, newHeader: { template: 7, internalName: null }, outsideHeader: 470 }), 470)).toBe(true);
  });

  it("still_asks_for_a_header_when_neither_is_chosen", () => {
    expect(roomSpec(roomForm({ header: null }), 470)).toEqual({ error: "Header wählen." });
  });
});

describe("roomSummary", () => {
  const spec: RoomSpec = { name: "Haus", source_map: 321, clear_buildings: false, header: { id: 1, area_data_id: 3, event_file: "New" }, entrance: { outside_header: 411, x: 12, z: 7, inside_x: 16, inside_z: 30 } };
  const known = [header(1, "NOTHING"), place(411, "Zweiblattdorf", "T01")];

  it("names_the_connection_and_the_header_it_changes", () => {
    const lines = roomSummary(spec, known);
    expect(lines).toContain("Verbindet 0411 Zweiblattdorf (Kachel x 12, z 7) mit dem neuen Raum.");
    expect(lines).toContain("Stellt Header 0001 „NOTHING“ auf das neue Raster um, Area-Data 0003, Eventdatei: neu, leer.");
    expect(lines[0]).toContain("Karte 0321");
    expect(lines[lines.length - 1]).toBe("Anlegen lässt sich nicht rückgängig machen.");
  });

  it("names_why_no_entrance_is_made", () => {
    const lines = roomSummary({ ...spec, entrance: null }, known, "Eventdatei 0005 ist schreibgeschützt");
    expect(lines.some((l) => l.startsWith("Legt keinen Eingang an (Eventdatei 0005 ist schreibgeschützt)"))).toBe(true);
  });

  it("says_that_nothing_is_connected_without_an_entrance", () => {
    const lines = roomSummary({ ...spec, entrance: null }, known);
    expect(lines.some((l) => l.startsWith("Verbindet"))).toBe(false);
    expect(lines.some((l) => l.startsWith("Legt keinen Eingang an"))).toBe(true);
  });
});

describe("mapUnderTile", () => {
  it("finds_the_map_of_the_cell_and_nothing_off_the_grid", () => {
    const grid = { width: 2, height: 1, maps: [10, null] };
    expect(mapUnderTile(grid, 31, 31)).toBe(10);
    expect(mapUnderTile(grid, 32, 0)).toBeNull();
    expect(mapUnderTile(grid, 64, 0)).toBeNull();
    expect(mapUnderTile(grid, -1, 0)).toBeNull();
    expect(mapUnderTile(null, 0, 0)).toBeNull();
  });
});

describe("roomResult", () => {
  const spec: RoomSpec = { name: "Haus", source_map: 321, clear_buildings: false, header: { id: 1, area_data_id: null, event_file: "New" }, entrance: { outside_header: 411, x: 12, z: 7, inside_x: 16, inside_z: 30 } };
  const report: RoomReport = { matrix: 131, map: 571, event_file: 20, outside_warp: 3, inside_warp: 0, inside_warp_reused: false, door_tiles: [] };

  it("lists_what_was_made_and_the_warps", () => {
    const { lines, next } = roomResult(report, spec);
    expect(lines[0]).toBe("Raum „Haus“ angelegt: Raster 0131, Karte 0571.");
    expect(lines[1]).toBe("Header 0001 zeigt jetzt auf Raster 0131, Eventdatei 0020.");
    expect(lines[2]).toBe("Eingang: Warp 3 in Header 0411, Rückweg: Warp 0 im neuen Raum.");
    expect(next[0]).toContain("Emulator");
    // Figures and triggers come from the tools and the „+“ of „In dieser Szene“ since 3e.3, not from a header editor.
    expect(next[1]).toBe("Innenraum einrichten: Figuren, Auslöser und Schilder mit den Werkzeugen oder „+“ in „In dieser Szene“");
  });

  it("calls_a_created_header_new_and_names_the_door_tiles_and_the_reused_warp", () => {
    const created: RoomSpec = { ...spec, header: { id: 593, area_data_id: null, event_file: { Copy: 394 }, new_header: { template: 415, internal_name: "TESTZIMMER" } } };
    const { lines } = roomResult({ ...report, inside_warp_reused: true, door_tiles: [{ map: 187, x: 8, y: 4 }] }, created);
    expect(lines[1]).toBe("Neuer Header 0593 „TESTZIMMER“ (Kopie von 0415) zeigt jetzt auf Raster 0131, Eventdatei 0020.");
    expect(lines[2]).toBe("Eingang: Warp 3 in Header 0411, Rückweg: Warp 0 (der kopierte Warp auf der Kachel, umgestellt) im neuen Raum.");
    expect(lines[3]).toBe("Tür-Kachel gesetzt: Karte 0187, Kachel x 8, z 4 innerhalb der Karte.");
  });

  it("points_to_the_door_tool_when_no_entrance_was_made", () => {
    const { lines, next } = roomResult({ ...report, outside_warp: null, inside_warp: null }, { ...spec, entrance: null });
    expect(lines).toHaveLength(2);
    expect(next[0]).toBe("Eingang verbinden: Werkzeug „Tür“ oder „+“ › „Tür“ in „In dieser Szene“");
    expect(next[0]).not.toMatch(/Ausgänge|Warp hinzufügen/);
  });
});

describe("templateWarp", () => {
  const events = (warps: { x: number; z: number }[]): EventsDto => ({
    event_file_id: 1,
    locked: null,
    bg: [],
    objects: [],
    coords: [],
    warps: warps.map((w, index) => ({ index, ...w, dest_header_id: 0, dest_warp_id: 0, height: 0 })),
  });

  it("is_the_position_of_the_first_warp", () => {
    expect(templateWarp(events([{ x: 3, z: 9 }, { x: 5, z: 5 }]))).toEqual({ x: 3, z: 9 });
  });

  it("is_null_without_warps_or_events", () => {
    expect(templateWarp(events([]))).toBeNull();
    expect(templateWarp(null)).toBeNull();
  });
});

describe("isLateFailure", () => {
  it("tells_a_failure_after_creating_from_one_before_by_the_code", () => {
    expect(isLateFailure({ code: "partial", message: "map 0571 and matrix 0131 were created; header edit failed: boom", asset: null })).toBe(true);
    // The text alone no longer decides: a plain error with the same words is a refusal.
    expect(isLateFailure({ code: "error", message: "map 0571 was created; header edit failed: boom", asset: null })).toBe(false);
    expect(isLateFailure({ code: "error", message: "nothing was created; x failed: y", asset: null })).toBe(false);
    expect(isLateFailure("map 0571 was created; header edit failed: boom")).toBe(false);
  });
});

describe("previewArea", () => {
  const h = (id: number, area_data_id: number) => ({ id, internal_name: `H${id}`, location_index: 0, kanto: false, location_name: "", matrix_id: 0, area_data_id, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
  const headers = [h(1, 4), h(2, 9)];
  it("prefers_the_forms_area_then_the_chosen_header_then_the_source", () => {
    expect(previewArea({ areaDataId: 7, header: 1 }, headers, h(2, 9))).toBe(7);
    expect(previewArea({ areaDataId: null, header: 1 }, headers, h(2, 9))).toBe(4);
    expect(previewArea({ areaDataId: null, header: null }, headers, h(2, 9))).toBe(9);
    expect(previewArea({ areaDataId: null, header: 99 }, headers, h(2, 9))).toBe(9);
    expect(previewArea({ areaDataId: null, header: null }, headers, null)).toBe(0);
  });
});
