import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { filterHeaders, headerLabel, showsMatrix } from "./headers";

const header = (id: number, internal_name: string, location_name: string): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const headers = [header(411, "T01", "Zweiblattdorf"), header(412, "T01R0101", "Zweiblattdorf"), header(413, "C01", "Jubelstadt"), header(0, "EVERYWHERE", "")];

describe("filterHeaders", () => {
  it("returns every header for an empty or blank query", () => {
    expect(filterHeaders(headers, "")).toBe(headers);
    expect(filterHeaders(headers, "  ")).toBe(headers);
  });

  it("matches the location name case-insensitively", () => {
    expect(filterHeaders(headers, "zweiblatt").map((h) => h.id)).toEqual([411, 412]);
  });

  it("matches the internal name case-insensitively", () => {
    expect(filterHeaders(headers, "r0101").map((h) => h.id)).toEqual([412]);
    expect(filterHeaders(headers, "everywhere").map((h) => h.id)).toEqual([0]);
  });

  it("returns nothing when neither name matches", () => {
    expect(filterHeaders(headers, "xyz")).toEqual([]);
  });
});

describe("headerLabel", () => {
  it("prefers the location name and falls back to the internal name", () => {
    expect(headerLabel(headers[0])).toBe("Zweiblattdorf");
    expect(headerLabel(headers[3])).toBe("EVERYWHERE");
  });
});

describe("showsMatrix", () => {
  const h = { ...header(411, "T01", "Zweiblattdorf"), matrix_id: 7 };

  it("marks a header whose matrix is the shown one", () => {
    expect(showsMatrix(h, 7)).toBe(true);
    expect(showsMatrix(h, 8)).toBe(false);
  });

  it("marks nothing while no matrix is shown", () => {
    expect(showsMatrix(h, null)).toBe(false);
  });
});
