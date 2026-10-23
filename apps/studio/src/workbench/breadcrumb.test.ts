import { describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixSummary } from "../api";
import { crumbsFor } from "./breadcrumb";
import type { Crumb } from "./breadcrumb";
import { buildPlaces } from "./places";
import type { InspectorModel } from "./Inspector";

const header = (id: number, location_name: string, matrix_id: number, internal_name = `H${id}`): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  matrix_id,
  area_data_id: 0,
  event_file_id: id + 100,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const matrix = (index: number, has_headers: boolean): MatrixSummary => ({ index, name: `m${index}`, width: 1, height: 1, has_headers, has_altitudes: false, filled: 1 });

const headers = [header(0, "Zweiblattdorf", 0), header(414, "Zweiblattdorf", 7), header(9, "", 9)];
const places = buildPlaces(headers, [matrix(0, true), matrix(7, false), matrix(9, false)]);
const pin = (pinned: InspectorModel, selected: number | null = 414, extra: { matrixName?: string; goTo?: () => void } = {}) => crumbsFor({ places, headers, selected, pinned, ...extra });

describe("crumbsFor", () => {
  it("lists_place_header_and_the_title_of_the_selected_event", () => {
    const crumbs = pin({ kind: "event", eventFile: 514, eventKind: "object", index: 2 });
    expect(crumbs.map((c) => [c.label, c.tech])).toEqual([
      ["Zweiblattdorf", undefined],
      ["Zweiblattdorf", "0414"],
      ["Objekt 2", "Eventdatei 0514"],
    ]);
  });

  it("names_a_header_without_a_location_name_Raum_and_gives_its_id", () => {
    const crumbs = pin({ kind: "header", id: 9 }, 9);
    expect(crumbs.map((c) => [c.label, c.tech])).toEqual([
      ["Ohne Ortsnamen", undefined],
      ["Raum", "0009"],
    ]);
  });

  it("has_two_crumbs_when_nothing_but_the_header_is_selected", () => {
    expect(pin({ kind: "header", id: 414 })).toHaveLength(2);
    expect(pin({ kind: "none" })).toHaveLength(2);
    // A hover over a tile is not a selection.
    expect(pin({ kind: "tile", map: 1, x: 0, y: 0, absX: 0, absZ: 0, info: { kind: 0, collision: 0, blocked: false, name: null, class: null } })).toHaveLength(2);
  });

  it("writes_a_building_a_cell_and_area_data_as_the_last_crumb", () => {
    const last = (crumbs: Crumb[]) => crumbs[crumbs.length - 1];
    expect(last(pin({ kind: "building", map: 3, index: 3 }))).toMatchObject({ label: "Gebäude 3" });
    expect(last(pin({ kind: "matrixCell", matrix: 0, x: 7, y: 25 }, 0, { matrixName: "Hauptraster" }))).toMatchObject({ label: "Zelle 7, 25", tech: "Hauptraster" });
    expect(last(pin({ kind: "area", id: 12 }))).toMatchObject({ label: "Area-Data 12" });
  });

  it("never_has_more_than_three_crumbs", () => {
    expect(pin({ kind: "event", eventFile: 1, eventKind: "warp", index: 0 }).length).toBeLessThanOrEqual(3);
  });

  it("says_Kein_Projekt_without_headers_and_Kein_Ort_gewaehlt_without_a_selected_header", () => {
    expect(crumbsFor({ places: [], headers: [], selected: null, pinned: { kind: "none" } })).toEqual([{ label: "Kein Projekt" }]);
    expect(pin({ kind: "none" }, null)).toEqual([{ label: "Kein Ort gewählt" }]);
  });

  it("makes_each_crumb_a_link_to_what_its_label_says", () => {
    const goTo = vi.fn();
    const crumbs = crumbsFor({ places, headers, selected: 414, pinned: { kind: "event", eventFile: 514, eventKind: "warp", index: 1 }, goTo });
    crumbs.forEach((c) => c.onClick?.());
    expect(goTo.mock.calls.map((c) => c[0])).toEqual([
      { kind: "header", id: 0 },
      { kind: "header", id: 414 },
      { kind: "event", eventFile: 514, eventKind: "warp", index: 1 },
    ]);
  });

  it("has_no_links_without_a_goTo", () => {
    expect(pin({ kind: "header", id: 414 }).every((c) => c.onClick === undefined)).toBe(true);
  });
});
