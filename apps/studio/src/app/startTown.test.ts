import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import type { HeaderIdentity } from "../workbench/headerIdentity";
import { UNNAMED_PLACE } from "../workbench/places";
import { NO_PLACE_INDEX } from "../workbench/worldModel";
import { startTownName } from "./startTown";

const header = (location_name: string, location_index = 1): HeaderSummary => ({
  id: 411,
  internal_name: "T01",
  location_index,
  kanto: false,
  location_name,
  matrix_id: 0,
  area_data_id: 7,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const identity = (kind: HeaderIdentity["kind"], name: string): HeaderIdentity => ({ kind, name, place: name });

describe("startTownName", () => {
  it("names_the_area_or_the_place_of_an_outside_header", () => {
    expect(startTownName(header("Zweiblattdorf"), identity("area", "Zweiblattdorf"))).toBe("Zweiblattdorf");
    expect(startTownName(header("Zweiblattdorf"), identity("other", "Zweiblattdorf"))).toBe("Zweiblattdorf");
  });

  it("gives_no_name_for_a_room", () => {
    expect(startTownName(header("Zweiblattdorf"), identity("room", "Haus 1"))).toBeNull();
  });

  it("gives_no_name_for_a_header_without_a_place_name_also_as_an_area", () => {
    // The world names a nameless area „Ohne Ortsnamen“: the hint then says „der Startstadt“, not „von Ohne Ortsnamen“.
    expect(startTownName(header(""), identity("area", UNNAMED_PLACE))).toBeNull();
    expect(startTownName(header("   "), identity("area", UNNAMED_PLACE))).toBeNull();
    expect(startTownName(header("Zweiblattdorf", NO_PLACE_INDEX), identity("area", UNNAMED_PLACE))).toBeNull();
    expect(startTownName(header(""), identity("other", "T01"))).toBeNull();
    // A place name the world model does not know yet still falls back to „Ohne Ortsnamen“: no name either.
    expect(startTownName(header("Zweiblattdorf"), identity("area", UNNAMED_PLACE))).toBeNull();
  });
});
