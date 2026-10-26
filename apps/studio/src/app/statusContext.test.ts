import { describe, expect, it } from "vitest";
import type { HeaderSummary, ObjectDto, WarpDto } from "../api";
import type { SceneHover } from "../workbench/Inspector";
import { dataStatus, statusContext } from "./statusContext";

const header = (over: Partial<HeaderSummary> = {}): HeaderSummary => ({ id: 3, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 3, text_archive_id: 0, script_file_id: 0, level_script_id: 0, ...over });

describe("statusContext", () => {
  it("is_empty_without_a_selected_header", () => {
    expect(statusContext({ kind: "none" }, null, undefined, "bauen")).toBe("");
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    expect(statusContext({ kind: "header", id: 3 }, null, header(), "bauen")).toBe("Zweiblattdorf");
  });

  it("says_place_and_map_with_its_cell_in_bauen", () => {
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, null, header(), "bauen", 412)).toBe("Zweiblattdorf, Karte 0412 (3, 27)");
    // The map is not known: the cell alone.
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, null, header(), "bauen")).toBe("Zweiblattdorf, Karte (3, 27)");
  });

  it("says_an_empty_cell_in_bauen", () => {
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, null, header(), "bauen", null)).toBe("Zweiblattdorf, leere Zelle (3, 27)");
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    expect(statusContext({ kind: "header", id: 3 }, null, header(), "tuefteln")).toBe("Zweiblattdorf, Raster 0");
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, null, header(), "tuefteln", 412)).toBe("Zweiblattdorf, Raster 0, Zelle 3, 27");
  });

  it("falls_back_to_the_internal_name_and_adds_the_cell", () => {
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, null, header({ location_name: "" }), "tuefteln")).toBe("T01, Raster 0, Zelle 3, 27");
  });

  it("says_the_hovered_tile_or_marker_first_then_the_place", () => {
    const info = { kind: 3, collision: 0, blocked: false, name: null, class: null };
    const tile: SceneHover = { kind: "tile", map: 187, x: 8, y: 4, absX: 8, absZ: 4, info };
    const warp: WarpDto = { index: 1, x: 8, z: 4, dest_header_id: 414, dest_warp_id: 0, height: 0 };
    const door = (destName?: string): SceneHover => ({ kind: "eventHover", eventFile: 3, eventKind: "warp", index: 1, data: warp, destName });
    const figure: SceneHover = { kind: "eventHover", eventFile: 3, eventKind: "object", index: 2, data: { index: 2, local_id: 7 } as ObjectDto };
    const cell = { kind: "matrixCell", matrix: 0, x: 3, y: 27 } as const;
    // The hover wins over the selection ...
    expect(statusContext(cell, tile, header(), "bauen")).toBe("Kachel x 8, y 4 · Karte 0187 · begehbar");
    expect(statusContext(cell, tile, header(), "tuefteln")).toBe("Karte 0187 · Kachel x 8, y 4 · Typ 0x03 · begehbar");
    // A door has the number the inspector's title gives it („Warp 1 → …“).
    expect(statusContext(cell, door("Haus des Rivalen"), header(), "bauen")).toBe("Tür 1 → Haus des Rivalen");
    expect(statusContext(cell, door(), header(), "bauen")).toBe("Tür 1 → Ort 0414");
    expect(statusContext(cell, door(), header(), "tuefteln")).toBe("Tür 1 → Header 0414");
    expect(statusContext(cell, figure, header(), "bauen")).toBe("Objekt 2 · ID 7");
    // ... also without a selected header, and says no technical word in building.
    expect(statusContext({ kind: "none" }, door(), undefined, "bauen")).toBe("Tür 1 → Ort 0414");
    // ... and when the pointer leaves, the place is back.
    expect(statusContext(cell, null, header(), "bauen", 412)).toBe("Zweiblattdorf, Karte 0412 (3, 27)");
  });
});

describe("dataStatus", () => {
  it("names_the_selected_member_with_its_four_digit_number", () => {
    expect(dataStatus({ label: "Staravia", tech: "0397" })).toBe("Staravia, 0397");
  });

  it("is_empty_without_a_selection", () => {
    expect(dataStatus(null)).toBe("");
  });
});
