import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../../api";
import { headerFieldSpecs } from "./fields";
import type { FieldSpec } from "./fields";
import { pickerItems, referenceFor } from "./referenceFor";

const header = (id: number, internal: string, location: string, event = 0): HeaderSummary => ({
  id,
  internal_name: internal,
  location_name: location,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: event,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const headers = [header(411, "T01", "Zweiblattdorf", 9), header(414, "T01R0201", "", 12)];
const spec = (key: string) => headerFieldSpecs("Pt").find((f) => f.key === key)!;

describe("referenceFor", () => {
  it("leads_a_matrix_to_its_cell_in_the_world_area", () => {
    expect(referenceFor(spec("matrix_id"), 2, headers)).toEqual({ area: "welt", label: "Matrix", tech: "0002", target: { kind: "matrixCell", matrix: 2, x: 0, y: 0 } });
  });

  it("leads_a_text_archive_to_the_texts_area", () => {
    expect(referenceFor(spec("text_archive_id"), 5, headers)).toEqual({ area: "texte", label: "Textarchiv", tech: "0005", target: { kind: "textArchive", index: 5 } });
  });

  it("leads_a_script_file_and_area_data_to_the_world_area", () => {
    expect(referenceFor(spec("script_file_id"), 40, headers)).toEqual({ area: "welt", label: "Scriptdatei", tech: "0040", target: { kind: "script", file: 40 } });
    expect(referenceFor(spec("area_data_id"), 1, headers)).toEqual({ area: "welt", label: "Area-Data", tech: "0001", target: { kind: "area", id: 1 } });
  });

  it("has_no_script_to_open_for_a_level_script_of_none", () => {
    expect(referenceFor(spec("level_script_id"), 0, headers)).toEqual({ area: "welt", label: "Level-Script", tech: "keins", target: null });
    expect(referenceFor(spec("level_script_id"), 41, headers).target).toEqual({ kind: "script", file: 41 });
  });

  it("leads_an_event_file_to_the_first_header_that_uses_it_and_nowhere_when_none_does", () => {
    expect(referenceFor(spec("event_file_id"), 12, headers)).toMatchObject({ area: "welt", label: "Eventdatei", target: { kind: "header", id: 414 } });
    expect(referenceFor(spec("event_file_id"), 99, headers).target).toBeNull();
  });

  it("names_a_destination_header_by_location_and_identifier", () => {
    const dest: FieldSpec = { key: "dest_header_id", label: "Zielheader", kind: "pick", pick: "header" };
    expect(referenceFor(dest, 411, headers)).toEqual({ area: "welt", label: "Zweiblattdorf", tech: "0411 · T01", target: { kind: "header", id: 411 } });
    // Without a location name the internal name is the label.
    expect(referenceFor(dest, 414, headers)).toMatchObject({ label: "T01R0201", tech: "0414 · T01R0201" });
    expect(referenceFor(dest, 9999, headers)).toEqual({ area: "welt", label: "unbekannt", tech: "9999", target: null });
  });

  it("has_no_place_to_open_for_a_tileset", () => {
    expect(referenceFor({ key: "map_tileset", label: "Karten-Tileset", kind: "pick", pick: "mapTexture" }, 3, headers).target).toBeNull();
  });
});

describe("pickerItems", () => {
  it("lists_every_index_up_to_the_limit_with_its_number_as_the_technical_part", () => {
    expect(pickerItems(spec("matrix_id"), 2, headers)).toEqual([
      { id: 0, label: "Matrix", tech: "0000", area: "welt" },
      { id: 1, label: "Matrix", tech: "0001", area: "welt" },
      { id: 2, label: "Matrix", tech: "0002", area: "welt" },
    ]);
  });
});
