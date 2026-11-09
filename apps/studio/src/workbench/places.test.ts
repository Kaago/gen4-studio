import { describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixSummary, WarpLink } from "../api";
import { UNNAMED_KEY, buildPlaces, filterPlaces, placeColor, placeOf, scriptChildren, unnamedPlace } from "./places";

const header = (id: number, location_name: string, matrix_id: number, internal_name = `H${id}`): HeaderSummary => ({
  id,
  internal_name, location_index: 0, kanto: false,
  location_name,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const matrix = (index: number, width: number, height: number, has_headers = false): MatrixSummary => ({
  index,
  name: `m${index}`,
  width,
  height,
  has_headers,
  has_altitudes: false,
  filled: width * height,
});

const headers = [
  header(0, "Zweiblattdorf", 0, "T01"),
  header(1, "Zweiblattdorf", 7, "T01R0101"),
  header(2, "Zweiblattdorf", 8, "T01R0102"),
  header(3, "Route 201", 0, "T01R0201"),
  header(4, "", 9, "D01"),
];
const matrices = [matrix(0, 30, 30, true), matrix(7, 1, 1), matrix(8, 2, 2), matrix(9, 1, 1)];

const link = (dest_header_id: number): WarpLink => ({ event_file: 0, index: 0, x: 0, z: 0, dest_header_id, dest_warp_id: 0 });

describe("buildPlaces", () => {
  it("groups_headers_by_location_outside_first_then_rooms", () => {
    const places = buildPlaces(headers, matrices);
    expect(places.map((p) => p.name)).toEqual(["Zweiblattdorf", "Route 201", unnamedPlace()]);
    const first = places[0];
    expect(first.headers.map((h) => h.header.id)).toEqual([0, 1, 2]);
    expect(first.headers.map((h) => h.kind)).toEqual(["outside", "inside", "inside"]);
    expect(first.headers[2].size).toBe("2 × 2");
    expect(first.headers[1].size).toBeNull();
    expect(first.headers[0].mapCount).toBe(900);
    expect(first.headers[2].mapCount).toBe(4);
  });

  it("orders_rooms_by_id_after_outside_headers_even_when_they_come_first", () => {
    const places = buildPlaces([header(1, "Ort", 7), header(2, "Ort", 0), header(3, "Ort", 8)], matrices);
    expect(places[0].headers.map((h) => h.header.id)).toEqual([2, 1, 3]);
  });

  it("headers_without_name_or_matrix_summary_are_still_listed", () => {
    const places = buildPlaces([...headers, header(5, "Ort", 42)], matrices);
    const unnamed = places.find((p) => p.key === UNNAMED_KEY);
    expect(unnamed?.name).toBe(unnamedPlace());
    expect(unnamed?.headers.map((h) => h.header.id)).toEqual([4]);
    const orphan = places.find((p) => p.name === "Ort")?.headers[0];
    expect(orphan).toMatchObject({ kind: "inside", mapCount: 0, size: null });
  });

  it("counts_incoming_warps", () => {
    const places = buildPlaces(headers, matrices, [link(1), link(1)]);
    expect(places.flatMap((p) => p.headers).map((h) => [h.header.id, h.warpsIn])).toEqual([
      [0, 0],
      [1, 2],
      [2, 0],
      [3, 0],
      [4, 0],
    ]);
  });
});

describe("filterPlaces", () => {
  const places = buildPlaces(headers, matrices);

  it("filter_matches_place_names_internal_names_and_ids", () => {
    const byName = filterPlaces(places, "zweibl");
    expect(byName).toHaveLength(1);
    expect(byName[0].headers).toHaveLength(3);

    const byCode = filterPlaces(places, "T01R0201");
    expect(byCode.map((p) => p.name)).toEqual(["Route 201"]);
    expect(byCode[0].headers.map((h) => h.header.id)).toEqual([3]);

    const byId = filterPlaces(places, "0003");
    expect(byId.flatMap((p) => p.headers).map((h) => h.header.id)).toEqual([3]);

    expect(filterPlaces(places, "")).toBe(places);
    expect(filterPlaces(places, "nichts davon")).toEqual([]);
  });
});

describe("placeOf", () => {
  it("finds_the_place_of_a_header", () => {
    const places = buildPlaces(headers, matrices);
    expect(placeOf(places, 2)?.name).toBe("Zweiblattdorf");
    expect(placeOf(places, 99)).toBeUndefined();
  });
});

describe("placeColor", () => {
  it("place_colour_is_stable", () => {
    expect(placeColor("a")).toBe(placeColor("a"));
    expect(placeColor("a")).not.toBe(placeColor("b"));
    expect(placeColor("a")).toMatch(/^hsl\(/);
  });
});

describe("scriptChildren", () => {
  it("lists_the_script_file_always_and_the_level_script_only_when_there_is_one", () => {
    expect(scriptChildren({ script_file_id: 0, level_script_id: 1 })).toEqual([
      { kind: "script", file: 0, label: "Scriptdatei 0000" },
      { kind: "levelScript", file: 1, label: "Level-Script 0001" },
    ]);
    expect(scriptChildren({ script_file_id: 412, level_script_id: 0 })).toEqual([{ kind: "script", file: 412, label: "Scriptdatei 0412" }]);
  });
});
