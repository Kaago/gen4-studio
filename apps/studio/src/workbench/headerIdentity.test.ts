import { describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixSummary } from "../api";
import { headerIdentity } from "./headerIdentity";
import { buildPlaces } from "./places";
import type { WorldNames } from "./worldModel";

const header = (id: number, internal_name: string, location_name: string, matrix_id: number, location_index = 2): HeaderSummary => ({
  id,
  internal_name,
  location_index,
  kanto: false,
  location_name,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const matrix = (index: number, has_headers: boolean): MatrixSummary => ({ index, name: `m${index}`, width: 1, height: 1, has_headers, has_altitudes: false, filled: 1 });

const town = header(411, "T01", "Zweiblattdorf", 0);
const house = header(412, "T01R0101", "Zweiblattdorf", 3);
const shed = header(413, "T01X", "Zweiblattdorf", 4);
const headers = [town, house, shed];
const places = buildPlaces(headers, [matrix(0, true), matrix(3, false), matrix(4, false)]);

describe("headerIdentity", () => {
  it("takes_kind_and_name_from_the_world_once_it_is_read", () => {
    const names: WorldNames = new Map([
      [412, { kind: "room", name: "Omas Haus", placeName: "Zweiblattdorf" }],
      [413, { kind: "other", name: "Zweiblattdorf", placeName: "Zweiblattdorf" }],
    ]);
    expect(headerIdentity(places, house, names)).toEqual({ kind: "room", name: "Omas Haus", place: "Zweiblattdorf" });
    expect(headerIdentity(places, shed, names)).toEqual({ kind: "other", name: "Zweiblattdorf", place: "Zweiblattdorf" });
    // A header the world does not know yet: the places tell.
    expect(headerIdentity(places, town, names)).toEqual({ kind: "area", name: "Zweiblattdorf", place: "Zweiblattdorf" });
  });

  it("names_rooms_as_the_world_model_does_until_it_is_read", () => {
    expect(headerIdentity(places, town)).toEqual({ kind: "area", name: "Zweiblattdorf", place: "Zweiblattdorf" });
    expect(headerIdentity(places, house)).toEqual({ kind: "room", name: "Haus 1", place: "Zweiblattdorf" });
    // No name in its code: „Raum“ and its number among the numbered rooms of the place.
    expect(headerIdentity(places, shed)).toEqual({ kind: "room", name: "Raum 1", place: "Zweiblattdorf" });
    // A header the places do not list (yet).
    expect(headerIdentity([], header(9, "D05R0103", "Bergwerk", 9))).toEqual({ kind: "room", name: "Ebene 3", place: "Bergwerk" });
  });
});
