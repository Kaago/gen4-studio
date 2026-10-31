import { describe, expect, it } from "vitest";
import type { AreaReport, AreaSpec, EventsDto, HeaderSummary, MatrixDto, RoomReport, RoomSpec } from "../../api";
import { areaLayout, areaPlace, areaResult, areaSpec, areaSummary, buildingSources, emptyAreaForm, internalName, mapUnderTile, partialText, previewArea, emptyRoomForm, isLateFailure, parseMapList, rectSelection, placeForm, planSpec, roomPlace, roomResult, roomSpec, roomSummary, sourceHeader, specialHeader, templateWarp, toggleSelection } from "./wizardSpecs";
import type { AreaForm, RoomForm } from "./wizardSpecs";
import type { Area, World } from "../worldModel";

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
    expect(areaSpec(form(), 0, "tuefteln")).toEqual({ error: "Mindestens eine Karte wählen." });
  });

  it("refuses_more_than_255_columns_or_rows", () => {
    expect(areaSpec(form({ selected: [cell(0, 0), cell(255, 0)] }), 0, "tuefteln")).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect(areaSpec(form({ selected: [cell(0, 0), cell(0, 255)] }), 0, "tuefteln")).toEqual({ error: "Höchstens 255 Spalten/Zeilen." });
    expect("spec" in areaSpec(form({ selected: [cell(0, 0), cell(254, 0)] }), 0, "tuefteln")).toBe(true);
  });

  it("refuses_a_blank_name", () => {
    expect(areaSpec(form({ selected: [cell(0, 0)], name: "   " }), 0, "tuefteln")).toEqual({ error: "Name darf nicht leer sein." });
  });

  it("builds_the_backend_request_with_the_assigned_header", () => {
    const result = areaSpec(form({ selected: [cell(2, 1, 521), cell(3, 1, 522)], name: " Route 230 ", header: 1, areaDataId: 7, eventFile: { Copy: 9 } }), 0, "tuefteln");
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
    const result = areaSpec(form({ selected: [cell(0, 0)], withHeaders: false }), 0, "tuefteln");
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
    const result = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: "ROUTE_230" }, areaDataId: 7 }), 470, "tuefteln");
    expect(result).toMatchObject({ spec: { header: { id: 470, area_data_id: 7, event_file: "New", new_header: { template: 342, internal_name: "ROUTE_230" } } } });
  });

  it("derives_the_internal_name_from_the_area_name_until_one_is_typed", () => {
    const derived = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: null } }), 470, "tuefteln");
    expect(derived).toMatchObject({ spec: { header: { new_header: { internal_name: "ROUTE_230" } } } });
    const typed = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: "Mein Weg" } }), 470, "tuefteln");
    expect(typed).toMatchObject({ spec: { header: { new_header: { internal_name: "MEIN_WEG" } } } });
  });

  it("ignores_a_chosen_existing_header", () => {
    const result = areaSpec(form({ ...picked, header: 3, newHeader: { template: 342, internalName: null } }), 470, "tuefteln");
    expect(result).toMatchObject({ spec: { header: { id: 470 } } });
  });

  it("sends_no_new_header_for_an_existing_one", () => {
    const result = areaSpec(form({ ...picked, header: 3 }), 470, "tuefteln");
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
    const report: AreaReport = { matrix: 131, maps: [[0, 0, 571], [1, 0, 572]], event_file: 55, header: 1, patched: false, snapshot: null, place: null, replaced: 0 };
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
    const { lines } = areaResult({ matrix: 3, maps: [[0, 0, 9]], event_file: null, header: null, patched: false, snapshot: null, place: null, replaced: 0 }, spec);
    expect(lines).toEqual(["Raster 0003 „A“ (1 × 1) angelegt.", "Karte 0009 kopiert."]);
  });

  it("names_where_the_patch_put_the_old_header_folder", () => {
    const spec: AreaSpec = { name: "A", width: 1, height: 1, cells: [], with_headers: false, header: null };
    const report: AreaReport = { matrix: 3, maps: [[0, 0, 9]], event_file: null, header: null, patched: true, snapshot: "/p/.g4/backup/dynamicHeaders-20261026-120000", place: null, replaced: 0 };
    const line = "Sicherung des alten Ordners: /p/.g4/backup/dynamicHeaders-20261026-120000";
    expect(areaResult(report, spec).lines).toEqual(["Raster 0003 „A“ (1 × 1) angelegt.", "Karte 0009 kopiert.", line]);
    // Bauen names the backup folder only: the path names the header folder.
    expect(areaResult(report, spec, "bauen").lines).toEqual(["Gebiet „A“ angelegt: 1 Karte.", "Die alten Dateien sind unter .g4/backup gesichert."]);
    // No snapshot (or no patch in the call): no line.
    expect(areaResult({ ...report, snapshot: null }, spec, "bauen").lines).toEqual(["Gebiet „A“ angelegt: 1 Karte."]);
    expect(areaResult({ ...report, patched: false }, spec).lines.join(" ")).not.toContain("Sicherung");
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
    expect(roomSpec(roomForm({ template: null }), 0, "tuefteln")).toEqual({ error: "Vorlage wählen." });
    expect(roomSpec(roomForm({ templateMap: null }), 0, "tuefteln")).toEqual({ error: "Vorlage wählen." });
  });

  it("asks_for_the_header_and_for_the_outside_header_only_when_connecting", () => {
    expect(roomSpec(roomForm({ header: null }), 0, "tuefteln")).toEqual({ error: "Header wählen." });
    expect(roomSpec(roomForm({ outsideHeader: null }), 0, "tuefteln")).toEqual({ error: "Außen-Header wählen." });
    expect("spec" in roomSpec(roomForm({ outsideHeader: null, connect: false }), 0, "tuefteln")).toBe(true);
  });

  it("builds_the_request_with_the_entrance", () => {
    const built = roomSpec(roomForm({ name: " Haus ", areaDataId: 3, eventFile: { Copy: 9 }, clearBuildings: true, insideX: 5, insideZ: 6 }), 0, "tuefteln");
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
    expect("spec" in roomSpec(roomForm({ x: 12, z: 7 }), 0, "tuefteln", { outsideMatrix: grid })).toBe(true);
    expect(roomSpec(roomForm({ x: 40, z: 7 }), 0, "tuefteln", { outsideMatrix: grid })).toEqual({ error: "Kachel x 40, z 7 liegt in Zelle (1, 0) ohne Karte." });
    expect(roomSpec(roomForm({ x: 70, z: 7 }), 0, "tuefteln", { outsideMatrix: grid })).toMatchObject({ error: expect.stringContaining("von Header 0004, nicht von 0000") });
    // Without a header section every cell is the outside header's.
    expect("spec" in roomSpec(roomForm({ x: 70, z: 7 }), 0, "tuefteln", { outsideMatrix: { ...grid, headers: null } })).toBe(true);
  });

  it("refuses_door_tiles_that_already_have_a_warp_the_game_would_take_first", () => {
    const warp = (index: number, x: number, z: number) => ({ index, x, z, dest_header_id: 4, dest_warp_id: 0, height: 0 });
    expect(roomSpec(roomForm(), 0, "tuefteln", { outsideWarps: [warp(3, 12, 7)] })).toMatchObject({ error: expect.stringContaining("außen liegt schon Warp 3") });
    expect("spec" in roomSpec(roomForm(), 0, "tuefteln", { outsideWarps: [warp(3, 12, 8)] })).toBe(true);
    // Inside only the file the room keeps counts: a copy gets its warp repointed, a new file has none.
    const kept = { file: 9, warps: [warp(0, 5, 6)] };
    expect(roomSpec(roomForm({ eventFile: "Keep", insideX: 5, insideZ: 6 }), 0, "tuefteln", { keptWarps: kept })).toMatchObject({ error: expect.stringContaining("Warp 0 der Eventdatei 0009") });
    expect("spec" in roomSpec(roomForm({ eventFile: { Copy: 9 }, insideX: 5, insideZ: 6 }), 0, "tuefteln", { keptWarps: kept })).toBe(true);
  });

  it("makes_no_entrance_when_connecting_later", () => {
    const built = roomSpec(roomForm({ connect: false }), 0, "tuefteln");
    expect("spec" in built && built.spec.entrance).toBeNull();
  });

  it("refuses_an_inside_tile_off_the_map_but_only_when_connecting", () => {
    expect(roomSpec(roomForm({ insideX: 40 }), 0, "tuefteln")).toEqual({ error: "Kachel liegt außerhalb der Karte (0 bis 31)." });
    expect(roomSpec(roomForm({ insideZ: -1 }), 0, "tuefteln")).toEqual({ error: "Kachel liegt außerhalb der Karte (0 bis 31)." });
    expect("spec" in roomSpec(roomForm({ insideX: 40, connect: false }), 0, "tuefteln")).toBe(true);
  });

  it("asks_for_the_door_outside_when_it_is_not_entered_yet", () => {
    expect("error" in roomSpec(roomForm({ x: null }), 0, "tuefteln")).toBe(true);
    expect("error" in roomSpec(roomForm({ z: null }), 0, "tuefteln")).toBe(true);
    expect("spec" in roomSpec(roomForm({ x: null, z: null, connect: false }), 0, "tuefteln")).toBe(true);
  });

  it("refuses_a_door_outside_the_outside_headers_matrix", () => {
    const size = { width: 2, height: 1 };
    expect("spec" in roomSpec(roomForm({ x: 63, z: 31 }), 0, "tuefteln", size)).toBe(true);
    expect(roomSpec(roomForm({ x: 64, z: 7 }), 0, "tuefteln", size)).toEqual({ error: "Kachel x 64, z 7 liegt außerhalb des Rasters (x 0 bis 63, z 0 bis 31)." });
    expect("error" in roomSpec(roomForm({ x: 3, z: 32 }), 0, "tuefteln", size)).toBe(true);
    expect("error" in roomSpec(roomForm({ x: -1 }), 0, "tuefteln", size)).toBe(true);
    // An unknown matrix is not checked.
    expect("spec" in roomSpec(roomForm({ x: 9999 }), 0, "tuefteln", null)).toBe(true);
  });

  it("refuses_the_rooms_own_header_as_the_header_outside", () => {
    expect("error" in roomSpec(roomForm({ outsideHeader: 1 }), 0, "tuefteln")).toBe(true);
  });

  it("refuses_an_empty_name", () => {
    expect(roomSpec(roomForm({ name: "  " }), 0, "tuefteln")).toEqual({ error: "Name darf nicht leer sein." });
  });
});

describe("roomSpec with a new header", () => {
  it("takes_the_next_number_and_needs_no_chosen_header", () => {
    const built = roomSpec(roomForm({ header: null, template: 342, newHeader: { template: 342, internalName: null }, name: "Haus" }), 470, "tuefteln");
    expect(built).toMatchObject({ spec: { header: { id: 470, event_file: "New", new_header: { template: 342, internal_name: "HAUS" } } } });
  });

  it("refuses_a_new_header_as_the_header_outside", () => {
    expect("error" in roomSpec(roomForm({ header: null, newHeader: { template: 7, internalName: null }, outsideHeader: 470 }), 470, "tuefteln")).toBe(true);
  });

  it("still_asks_for_a_header_when_neither_is_chosen", () => {
    expect(roomSpec(roomForm({ header: null }), 470, "tuefteln")).toEqual({ error: "Header wählen." });
  });
});

describe("the header in Bauen", () => {
  const picked = { selected: [cell(0, 0, 521)], name: " Neuheim " };
  // What Tüfteln chose stays out of Bauen: the header, its area data and event file, a typed internal name.
  const tinkered = { header: 3, areaDataId: 9, eventFile: "Keep" as const };

  it("builds_a_new_header_with_place_and_patch_in_bauen", () => {
    const area = areaSpec(form({ ...picked, ...tinkered, newHeader: { template: 342, internalName: "TYPED" }, lookAreaData: 7, patchConsent: true }), 470, "bauen");
    expect("spec" in area && area.spec.header).toEqual({ id: 470, area_data_id: 7, event_file: "New", new_header: { template: 342, internal_name: null }, place: { New: "Neuheim" }, allow_patch: true });
    // The backend names the header: the summary quotes no name.
    expect("spec" in area && areaSummary(area.spec, headers).join(" ")).not.toContain("null");
    const existing = areaSpec(form({ ...picked, newHeader: { template: 342, internalName: null }, place: { Existing: 4 } }), 470, "bauen");
    expect("spec" in existing && existing.spec.header).toMatchObject({ area_data_id: null, place: { Existing: 4 }, allow_patch: false });
    expect(areaSpec(form({ ...picked, newHeader: { template: 342, internalName: null }, place: { New: "  " } }), 470, "bauen")).toEqual({ error: "Name des neuen Orts darf nicht leer sein." });

    const room = roomSpec(roomForm({ ...tinkered, eventFile: { Copy: 9 }, template: 342, place: { Existing: 4 }, patchConsent: true }), 470, "bauen");
    expect("spec" in room && room.spec.header).toEqual({ id: 470, area_data_id: null, event_file: "New", new_header: { template: 342, internal_name: null }, place: { Existing: 4 }, allow_patch: true });
    expect(roomSpec(roomForm({ place: null }), 470, "bauen")).toEqual({ error: "Ort wählen." });
  });

  it("keeps_the_header_choice_in_tuefteln", () => {
    const bauen = { place: { New: "Neuheim" }, lookAreaData: 7, patchConsent: true };
    const area = areaSpec(form({ ...picked, header: 3, ...bauen }), 470, "tuefteln");
    expect("spec" in area && area.spec.header).toEqual({ id: 3, area_data_id: null, event_file: "New" });
    const room = roomSpec(roomForm({ ...bauen, areaDataId: 3 }), 470, "tuefteln");
    expect("spec" in room && room.spec.header).toEqual({ id: 1, area_data_id: 3, event_file: "New" });
  });

  it("defaults_the_place_to_a_new_one_for_an_area_and_the_entrance_for_a_room", () => {
    expect(areaPlace({ place: null, name: " Neuheim " })).toEqual({ New: "Neuheim" });
    expect(areaPlace({ place: { Existing: 2 }, name: "Neuheim" })).toEqual({ Existing: 2 });
    const outside = { ...header(5, "OUT"), location_index: 12 };
    const template = { ...header(6, "TPL"), location_index: 30 };
    expect(roomPlace(null, outside, template)).toEqual({ Existing: 12 });
    expect(roomPlace(null, null, template)).toEqual({ Existing: 30 });
    expect(roomPlace(null, null, null)).toBeNull();
    expect(placeForm({ New: "Neuheim" }, { New: "Neuheim" })).toBeNull();
    expect(placeForm({ New: "Altheim" }, { New: "Neuheim" })).toEqual({ New: "Altheim" });
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
  const report: RoomReport = { matrix: 131, map: 571, event_file: 20, outside_warp: 3, inside_warp: 0, inside_warp_reused: false, door_tiles: [], patched: false, snapshot: null, place: null };

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

  it("names_where_the_patch_put_the_old_header_folder", () => {
    const patched = { ...report, patched: true, snapshot: "/p/.g4/backup/dynamicHeaders-20261026-120000" };
    const line = "Sicherung des alten Ordners: /p/.g4/backup/dynamicHeaders-20261026-120000";
    expect(roomResult(patched, spec).lines.slice(-1)).toEqual([line]);
    expect(roomResult(patched, spec, "bauen").lines.slice(-1)).toEqual(["Die alten Dateien sind unter .g4/backup gesichert."]);
    expect(roomResult({ ...patched, snapshot: null }, spec, "bauen").lines.join(" ")).not.toContain("Sicherung");
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

describe("the words in Bauen", () => {
  const FORBIDDEN = /Raster|Matrix|Header/;
  // 0342 is the template (Zweiblattdorf, look 3); 0400 carries look 5 at Sandgemme; 0411 is the village outside the room.
  const known: HeaderSummary[] = [
    { ...header(342, "T01", 0, 3), location_index: 1, location_name: "Zweiblattdorf" },
    { ...header(400, "T02", 0, 5), location_index: 2, location_name: "Sandgemme" },
    { ...header(411, "T03", 0, 3), location_index: 1, location_name: "Zweiblattdorf" },
    { ...header(7, "R01", 9, 4), location_index: 3, location_name: "Spielerhaus" },
  ];
  const picked = { selected: [cell(0, 0, 521), cell(1, 0, 522)], name: "Neuheim", clearBuildings: true, newHeader: { template: 342, internalName: null } };
  const areaOf = (change: Partial<AreaForm> = {}): AreaSpec => {
    const built = areaSpec(form({ ...picked, ...change }), 470, "bauen");
    if (!("spec" in built)) throw new Error(built.error);
    return built.spec;
  };
  const roomOf = (change: Partial<RoomForm> = {}): RoomSpec => {
    const built = roomSpec(roomForm({ name: "Haus", template: 7, outsideHeader: 411, place: { Existing: 1 }, ...change }), 470, "bauen");
    if (!("spec" in built)) throw new Error(built.error);
    return built.spec;
  };

  it("summarises_an_area_in_user_words_in_bauen", () => {
    expect(areaSummary(areaOf(), known, [], "bauen")).toEqual([
      "Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an.",
      "Kopiert Karten 0521, 0522, ohne Gebäude.",
      "Neuer Ort „Neuheim“.",
      "Aussehen wie Zweiblattdorf.",
      "Eine neue, leere Liste der Figuren und Türen.",
      "Anlegen lässt sich nicht rückgängig machen.",
    ]);
    const existing = areaSummary(areaOf({ place: { Existing: 2 }, lookAreaData: 5, clearBuildings: false }), known, [], "bauen");
    expect(existing).toContain("Ort: Sandgemme.");
    expect(existing).toContain("Aussehen wie Sandgemme.");
    expect(existing[1]).toBe("Kopiert Karten 0521, 0522, mit Gebäuden.");
    // A place no header carries yet is named by its number; a look without a place name says so.
    const unnamed = areaSummary(areaOf({ place: { Existing: 12 }, lookAreaData: 6 }), [...known, { ...header(9, "X", 0, 6) }], [], "bauen");
    expect(unnamed).toContain("Ort: Nummer 0012.");
    expect(unnamed).toContain("Aussehen: Ohne Ortsnamen.");
    expect(areaSummary(areaOf(), known, [], "bauen").join(" ")).not.toMatch(FORBIDDEN);
  });

  it("always_gives_a_new_area_its_cells_in_bauen", () => {
    expect(areaOf({ withHeaders: false }).with_headers).toBe(true);
    const tinkered = areaSpec(form({ ...picked, withHeaders: false, header: 3 }), 470, "tuefteln");
    expect("spec" in tinkered && tinkered.spec.with_headers).toBe(false);
  });

  it("says_what_was_made_in_user_words_in_bauen", () => {
    const report: AreaReport = { matrix: 131, maps: [[0, 0, 571], [1, 0, 572]], event_file: 55, header: 470, patched: true, snapshot: null, place: 12, replaced: 0 };
    const { lines, next } = areaResult(report, areaOf(), "bauen");
    expect(lines).toEqual(["Gebiet „Neuheim“ angelegt: 2 Karten."]);
    expect(areaResult({ ...report, maps: [[0, 0, 571]] }, areaOf(), "bauen").lines).toEqual(["Gebiet „Neuheim“ angelegt: 1 Karte."]);
    expect([...lines, ...next].join(" ")).not.toMatch(FORBIDDEN);
  });

  it("names_the_templates_own_place_for_its_look_in_bauen", () => {
    // Template 342 is Route 201; its area data is Jubelstadt's too, and Jubelstadt comes first in header order.
    const shared: HeaderSummary[] = [
      { ...header(100, "C01", 0, 3), location_index: 4, location_name: "Jubelstadt" },
      { ...header(342, "R201", 0, 3), location_index: 5, location_name: "Route 201" },
      { ...header(400, "T02", 0, 5), location_index: 2, location_name: "Sandgemme" },
    ];
    expect(areaSummary(areaOf(), shared, [], "bauen")).toContain("Aussehen wie Route 201.");
    // A look chosen in the step stays named by its first place.
    expect(areaSummary(areaOf({ lookAreaData: 5 }), shared, [], "bauen")).toContain("Aussehen wie Sandgemme.");
  });

  it("summarises_a_room_in_user_words_in_bauen", () => {
    const lines = roomSummary(roomOf(), known, undefined, [], {}, "bauen");
    expect(lines).toEqual([
      "Legt den Raum „Haus“ mit einer Kopie der Karte 0321 an, mit Gebäuden.",
      "Ort: Zweiblattdorf.",
      "Aussehen wie Spielerhaus.",
      "Eine neue, leere Liste der Figuren und Türen.",
      "Verbindet Zweiblattdorf (Kachel x 12, z 7) mit dem neuen Raum.",
      "Der Rückweg liegt im neuen Raum auf Kachel x 16, z 30.",
      "Anlegen lässt sich nicht rückgängig machen.",
    ]);
    const later = roomSummary(roomOf({ connect: false, place: { New: "Neues Haus" } }), known, "Eventdatei 0005 ist schreibgeschützt", [], {}, "bauen");
    expect(later).toContain("Neuer Ort „Neues Haus“.");
    expect(later).toContain("Legt keinen Eingang an (Eventdatei 0005 ist schreibgeschützt): später mit Werkzeug „Tür“ oder „+“ › „Tür“ in „In dieser Szene“ setzen.");
    expect([...lines, ...later].join(" ")).not.toMatch(FORBIDDEN);
    const reused = { index: 2, x: 16, z: 30, dest_header_id: 411, dest_warp_id: 0, height: 0 };
    expect(roomSummary(roomOf(), known, undefined, [], { reused }, "bauen").join(" ")).not.toMatch(FORBIDDEN);
  });

  it("says_what_was_made_for_a_room_in_user_words_in_bauen", () => {
    const report: RoomReport = { matrix: 131, map: 571, event_file: 20, outside_warp: 3, inside_warp: 0, inside_warp_reused: false, door_tiles: [{ map: 571, x: 8, y: 4 }], patched: false, snapshot: null, place: null };
    const { lines, next } = roomResult(report, roomOf(), "bauen");
    expect(lines[0]).toBe("Raum „Haus“ angelegt: Karte 0571.");
    expect(lines[1]).toBe("Eingang: Warp 3 außen, Rückweg: Warp 0 im neuen Raum.");
    expect([...lines, ...next].join(" ")).not.toMatch(FORBIDDEN);
    const alone = roomResult({ ...report, outside_warp: null, inside_warp: null }, roomOf({ connect: false }), "bauen");
    expect([...alone.lines, ...alone.next].join(" ")).not.toMatch(FORBIDDEN);
  });

  it("refuses_doors_in_user_words_in_bauen", () => {
    const refuse = (change: Partial<RoomForm>, checks: Parameters<typeof roomSpec>[3]) => {
      const built = roomSpec(roomForm({ template: 7, place: { Existing: 1 }, ...change }), 470, "bauen", checks);
      return "error" in built ? built.error : "";
    };
    expect(refuse({ outsideHeader: null }, null)).toBe("Gebiet am Eingang wählen.");
    expect(refuse({ x: 99 }, { width: 2, height: 1 })).toBe("Kachel x 99, z 7 liegt außerhalb des Gebiets am Eingang (x 0 bis 63, z 0 bis 31).");
    expect(refuse({ x: 40 }, { outsideMatrix: { width: 2, height: 1, maps: [1, 2], headers: [0, 5] } })).toBe(
      "Kachel x 40, z 7 liegt in Zelle (1, 0) eines anderen Gebiets: dort nimmt das Spiel die Warps des Gebiets am Eingang nicht.",
    );
    expect(refuse({}, { outsideWarps: [{ index: 4, x: 12, z: 7, dest_header_id: 3, dest_warp_id: 0, height: 0 }] })).toBe("Auf Kachel x 12, z 7 außen liegt schon Warp 4: eine andere Kachel wählen.");
  });

  it("says_a_late_failure_in_user_words_in_bauen", () => {
    // The area and its arrangement without their numbers; the maps keep theirs.
    expect(partialText("header 0470, matrix 0131, maps 0571, 0572 and event file 0055 were created; header edit failed: boom", "bauen", "area")).toBe(
      "Angelegt: das Gebiet, die Anordnung, Karten 0571, 0572, Eventdatei 0055. Danach fehlgeschlagen: Gebiet einrichten (boom).",
    );
    expect(partialText("headers 0470, 0471 were created; matrix creation failed: x", "bauen", "room")).toBe("Angelegt: die Räume. Danach fehlgeschlagen: Anordnung anlegen (x).");
    expect(partialText("header 0470 was created; header creation failed: x", "bauen", "room")).toBe("Angelegt: der Raum. Danach fehlgeschlagen: Raum anlegen (x).");
  });

  it("keeps_the_patch_that_stays_in_a_late_failure", () => {
    // The backend's texts (`created_text` in wizards.rs): members after the patch, or the patch alone.
    const withMembers = "the Dynamic Headers patch was applied and stays, header 0002 was created; map copy failed: X";
    const patchOnly = "the Dynamic Headers patch was applied and stays; header creation failed: X";
    expect(partialText(withMembers, "bauen", "area")).toBe("Spielcode für neue Orte angepasst (bleibt). Angelegt: das Gebiet. Danach fehlgeschlagen: Karte kopieren (X).");
    expect(partialText(patchOnly, "bauen", "area")).toBe("Spielcode für neue Orte angepasst (bleibt). Danach fehlgeschlagen: Gebiet anlegen (X).");
    expect(partialText(patchOnly, "bauen", "room")).toBe("Spielcode für neue Orte angepasst (bleibt). Danach fehlgeschlagen: Raum anlegen (X).");
    expect(partialText(withMembers, "tuefteln")).toBe("Patch „Dynamische Header“ angewendet (bleibt). Angelegt: Header 0002. Danach fehlgeschlagen: Karte kopieren (X).");
    expect(partialText(patchOnly, "tuefteln")).toBe("Patch „Dynamische Header“ angewendet (bleibt). Danach fehlgeschlagen: Header anlegen (X).");
    expect(partialText("the Dynamic Headers patch was applied and stays, header 0006, matrix 0131 and map 0571 were created; warp edit failed: boom", "bauen", "room")).toBe(
      "Spielcode für neue Orte angepasst (bleibt). Angelegt: der Raum, die Anordnung, Karte 0571. Danach fehlgeschlagen: Warp setzen (boom).",
    );
  });

  it("keeps_the_tuefteln_words", () => {
    const spec: AreaSpec = { name: "Route 230", width: 1, height: 1, cells: [{ x: 0, y: 0, source_map: 521, clear_buildings: false }], with_headers: true, header: { id: 1, area_data_id: null, event_file: "New" } };
    expect(areaSummary(spec, headers, [], "tuefteln")[0]).toBe("Legt Raster „Route 230“ (1 × 1) mit Header-Abschnitt an.");
    expect(partialText("header 0593 was created; map copy failed: x", "tuefteln")).toBe("Angelegt: Header 0593. Danach fehlgeschlagen: Karte kopieren (x).");
    const room: RoomSpec = { name: "Haus", source_map: 321, clear_buildings: false, header: { id: 1, area_data_id: null, event_file: "New" }, entrance: null };
    expect(roomResult({ matrix: 131, map: 571, event_file: null, outside_warp: null, inside_warp: null, inside_warp_reused: false, door_tiles: [], patched: false, snapshot: null, place: null }, room, "tuefteln").lines[0]).toBe(
      "Raum „Haus“ angelegt: Raster 0131, Karte 0571.",
    );
    expect(roomSpec(roomForm({ outsideHeader: null }), 470, "tuefteln")).toEqual({ error: "Außen-Header wählen." });
  });
});

describe("buildingSources", () => {
  const room = { ...header(30, "R01", 9), location_name: "Spielerhaus" };
  const area = (matrix: number, name: string, rooms: World["unconnected"] = []): Area => ({ key: `m${matrix}`, name, placeKey: name, headers: [], rooms, matrix });
  const world: World = {
    // HeartGold/SoulSilver: Johto and Kanto are halves of one world map.
    worldMaps: [
      { key: "w0", matrix: 0, filter: "johto", name: "Johto", areas: [area(0, "Neuborkia", [{ header: room, name: "Haus des Spielers", suggested: "", placeKey: "", placeName: "", manual: false, shared: 0 }])] },
      { key: "w0k", matrix: 0, filter: "kanto", name: "Kanto", areas: [] },
    ],
    moreAreas: [area(5, "Knofensa-Turm")],
    unconnected: [],
    other: [],
    areaOf: new Map(),
    roomCount: 1,
    assignedRooms: 0,
  };

  it("names_the_world_maps_and_the_areas_with_a_map_of_their_own", () => {
    expect(buildingSources(world, [], 0)).toEqual([
      { value: 0, label: "Johto / Kanto" },
      { value: 5, label: "Knofensa-Turm" },
    ]);
    expect(buildingSources(null, [], null)).toEqual([]);
  });

  it("adds_the_raster_the_wizard_was_opened_on_by_its_room_or_place", () => {
    const last = (sources: { value: number; label: string }[]) => sources[sources.length - 1];
    expect(last(buildingSources(world, [room], 9))).toEqual({ value: 9, label: "Haus des Spielers" });
    expect(last(buildingSources(world, [{ ...header(31, "X", 12), location_name: "Seeufer" }], 12))).toEqual({ value: 12, label: "Seeufer" });
    expect(last(buildingSources(world, [], 13))).toEqual({ value: 13, label: "Ohne Ortsnamen" });
  });
});

describe("an area on a world map", () => {
  const picked = { selected: [cell(0, 0, 521), cell(1, 0, 522)], name: "Neuheim", newHeader: { template: 1, internalName: null } };
  const specOf = (change: Partial<AreaForm>, depth: "bauen" | "tuefteln" = "bauen"): AreaSpec => {
    const built = areaSpec(form({ ...picked, ...change }), 470, depth);
    if (!("spec" in built)) throw new Error(built.error);
    return built.spec;
  };

  it("sends_the_target_only_on_a_world_map", () => {
    // The target is kept while „Eigene Karte“ is chosen, but not sent.
    expect(specOf({ target: { matrix: 1, x: 2, y: 3 } })).not.toHaveProperty("target");
    expect(specOf({ worldMap: 1, target: { matrix: 1, x: 2, y: 3 } }).target).toEqual({ matrix: 1, x: 2, y: 3 });
    // No cell chosen yet, or one of another world map.
    const error = { error: "Die Zelle wählen, an der das Gebiet auf der Weltkarte liegen soll." };
    expect(areaSpec(form({ ...picked, worldMap: 1 }), 470, "bauen")).toEqual(error);
    expect(areaSpec(form({ ...picked, worldMap: 1, target: { matrix: 2, x: 0, y: 0 } }), 470, "tuefteln")).toEqual(error);
  });

  it("says_where_the_area_lies_in_both_depths", () => {
    const spec = specOf({ worldMap: 1, target: { matrix: 1, x: 4, y: 7 } });
    const bauen = areaSummary(spec, headers, [], "bauen", { name: "Sinnoh", replaced: 2 });
    expect(bauen[1]).toBe("Liegt auf der Weltkarte Sinnoh bei x 4, y 7 (nahtlos begehbar); ersetzt 2 Landschaftskarten.");
    expect(bauen.join(" ")).not.toMatch(/Raster|Matrix|Header/);
    expect(areaSummary(spec, headers, [], "bauen", { name: "Sinnoh", replaced: 1 })[1]).toBe("Liegt auf der Weltkarte Sinnoh bei x 4, y 7 (nahtlos begehbar); ersetzt 1 Landschaftskarte.");
    expect(areaSummary(spec, headers, [], "bauen", { name: "Johto und Kanto", replaced: 0 })[1]).toBe("Liegt auf der Weltkarte Johto und Kanto bei x 4, y 7 (nahtlos begehbar).");
    const tinkered = specOf({ worldMap: 1, target: { matrix: 1, x: 4, y: 7 }, header: 2, newHeader: null }, "tuefteln");
    const tuefteln = areaSummary(tinkered, headers, [], "tuefteln", { name: "Sinnoh", replaced: 2 });
    // No new raster: the area goes into the world map's.
    expect(tuefteln[0]).toBe("Legt das Gebiet „Neuheim“ (2 × 1) ohne neues Raster an.");
    expect(tuefteln).toContain("Ziel: Raster 0001, Zelle 4, 7; 2 Landschaftszellen werden ersetzt.");
    expect(tuefteln).toContain("Stellt Header 0002 „TWO“ auf Raster 0001 um, Eventdatei: neu, leer.");
    expect(areaSummary(tinkered, headers, [], "tuefteln", { name: "Sinnoh", replaced: 1 })).toContain("Ziel: Raster 0001, Zelle 4, 7; 1 Landschaftszelle wird ersetzt.");
  });

  it("names_the_world_map_the_area_went_into_in_the_result", () => {
    const spec = specOf({ worldMap: 1, target: { matrix: 1, x: 4, y: 7 } }, "tuefteln");
    const report: AreaReport = { matrix: 1, maps: [[0, 0, 571], [1, 0, 572]], event_file: null, header: null, patched: false, snapshot: null, place: null, replaced: 2 };
    expect(areaResult(report, spec).lines[0]).toBe("Gebiet „Neuheim“ (2 × 1) in Raster 0001 angelegt.");
  });

  it("names_walking_over_instead_of_a_door_as_the_next_step", () => {
    const report: AreaReport = { matrix: 1, maps: [[0, 0, 571], [1, 0, 572]], event_file: null, header: 470, patched: false, snapshot: null, place: null, replaced: 2 };
    const bauen = areaResult(report, specOf({ worldMap: 1, target: { matrix: 1, x: 4, y: 7 } }), "bauen").next;
    expect(bauen[0]).toBe("Nahtlos begehbar: vom Nachbargebiet aus hinüberlaufen.");
    expect(bauen.join(" ")).not.toContain("Tür");
    const tuefteln = areaResult(report, specOf({ worldMap: 1, target: { matrix: 1, x: 4, y: 7 } }, "tuefteln")).next;
    expect(tuefteln[0]).toBe("Nahtlos begehbar: vom Nachbargebiet in Raster 0001 aus hinüberlaufen.");
    expect(tuefteln.join(" ")).not.toContain("Tür");
    // An area with a map of its own still needs a door.
    expect(areaResult(report, specOf({}), "bauen").next[0]).toContain("Mit der Welt verbinden: ");
  });
});

describe("planSpec", () => {
  const header = { id: 6, area_data_id: null, event_file: "New" as const, new_header: { template: 2, internal_name: null }, allow_patch: false };
  const spec = { header } as unknown as RoomSpec;

  it("assumes_the_patch_for_a_new_header_while_it_is_missing", () => {
    expect(planSpec(spec, true).header.allow_patch).toBe(true);
    // The spec „Anlegen“ sends keeps the user's consent.
    expect(spec.header.allow_patch).toBe(false);
  });

  it("keeps_the_spec_with_the_patch_in_the_game_without_a_new_header_or_header", () => {
    expect(planSpec(spec, false)).toBe(spec);
    const existing = { header: { ...header, new_header: null } } as unknown as RoomSpec;
    expect(planSpec(existing, true)).toBe(existing);
    const none = { header: null } as unknown as AreaSpec;
    expect(planSpec(none, true)).toBe(none);
  });
});
