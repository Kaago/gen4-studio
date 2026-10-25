import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { dataStatus, statusContext } from "./statusContext";

const header = (over: Partial<HeaderSummary> = {}): HeaderSummary => ({ id: 3, internal_name: "T01", location_index: 0, kanto: false, location_name: "Zweiblattdorf", matrix_id: 0, area_data_id: 0, event_file_id: 3, text_archive_id: 0, script_file_id: 0, level_script_id: 0, ...over });

describe("statusContext", () => {
  it("is_empty_without_a_selected_header", () => {
    expect(statusContext({ kind: "none" }, undefined, "bauen")).toBe("");
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    expect(statusContext({ kind: "header", id: 3 }, header(), "bauen")).toBe("Zweiblattdorf");
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, header(), "bauen")).toBe("Zweiblattdorf, Zelle 3, 27");
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    expect(statusContext({ kind: "header", id: 3 }, header(), "tuefteln")).toBe("Zweiblattdorf, Raster 0");
  });

  it("falls_back_to_the_internal_name_and_adds_the_cell", () => {
    expect(statusContext({ kind: "matrixCell", matrix: 0, x: 3, y: 27 }, header({ location_name: "" }), "tuefteln")).toBe("T01, Raster 0, Zelle 3, 27");
  });

  it("describes_the_tile_under_the_pointer", () => {
    const info = { kind: 3, collision: 0, blocked: false, name: null, class: null };
    expect(statusContext({ kind: "tile", map: 187, x: 8, y: 4, absX: 8, absZ: 4, info }, header(), "bauen")).toBe("Karte 0187 · Kachel x 8, y 4 · Typ 0x03 · begehbar");
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
