import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { nextSelection } from "./navigation";
import type { NavState } from "./navigation";

const header = (id: number, matrix_id: number, event_file_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
  matrix_id,
  area_data_id: 0,
  event_file_id,
  text_archive_id: 0,
});

const headers = [header(3, 0, 10), header(4, 0, 11), header(415, 7, 12)];
const state: NavState = { selectedHeader: 3, matrix: 0, maps: [100, 101], eventFile: 10, headers };

describe("nextSelection", () => {
  it("matrix cell with a header navigates to that header", () => {
    const next = nextSelection({ selectedHeader: 3, matrix: 0 }, { kind: "matrixCell", matrix: 0, x: 1, y: 0, header: 415 });
    expect(next.selectedHeader).toBe(415);
  });

  it("matrix cell without a header pins the cell and shows the matrix panel", () => {
    expect(nextSelection(state, { kind: "matrixCell", matrix: 0, x: 1, y: 2 })).toEqual({
      selectedHeader: 3,
      pinned: { kind: "matrixCell", matrix: 0, x: 1, y: 2 },
      panel: "matrix",
    });
    // A header id the project does not have is no link.
    expect(nextSelection(state, { kind: "matrixCell", matrix: 0, x: 1, y: 2, header: 999 }).selectedHeader).toBe(3);
  });

  it("header selects and pins it", () => {
    expect(nextSelection(state, { kind: "header", id: 4 })).toEqual({ selectedHeader: 4, pinned: { kind: "header", id: 4 }, panel: "headers" });
  });

  it("a script opens in the script panel at its line and leaves selection and pin alone", () => {
    expect(nextSelection(state, { kind: "script", file: 42, line: 7 })).toEqual({
      selectedHeader: 3,
      pinned: null,
      panel: "script",
      script: { file: 42, line: 7 },
    });
    expect(nextSelection(state, { kind: "script", file: 42 })).toMatchObject({ panel: "script", script: { file: 42, line: null } });
  });

  it("an unknown header changes nothing", () => {
    expect(nextSelection(state, { kind: "header", id: 999 })).toEqual({ selectedHeader: 3, pinned: null, panel: null });
  });

  it("a text archive leads to the text panel and leaves the selection and the pin alone", () => {
    expect(nextSelection(state, { kind: "textArchive", index: 433, message: 2 })).toEqual({
      selectedHeader: 3,
      pinned: null,
      panel: "text",
      text: { index: 433, message: 2 },
    });
    expect(nextSelection(state, { kind: "textArchive", index: 5 })).toMatchObject({ panel: "text", text: { index: 5, message: null } });
  });

  it("area pins the area data and keeps the selected header", () => {
    expect(nextSelection(state, { kind: "area", id: 22 })).toEqual({ selectedHeader: 3, pinned: { kind: "area", id: 22 }, panel: "inspector" });
  });

  it("a building of a shown map is pinned without changing the header", () => {
    expect(nextSelection(state, { kind: "building", map: 101, index: 2, header: 415 })).toEqual({
      selectedHeader: 3,
      pinned: { kind: "building", map: 101, index: 2 },
      panel: "world",
    });
  });

  it("a building of a map that is not shown selects the header known to show it first", () => {
    const next = nextSelection(state, { kind: "building", map: 300, index: 0, header: 415 });
    expect(next.selectedHeader).toBe(415);
    expect(next.pinned).toEqual({ kind: "building", map: 300, index: 0 });
  });

  it("a building of a map that is not shown, with no known header, is pinned alone", () => {
    expect(nextSelection(state, { kind: "building", map: 300, index: 0 }).selectedHeader).toBe(3);
    expect(nextSelection(state, { kind: "building", map: 300, index: 0, header: 999 }).selectedHeader).toBe(3);
  });

  it("an event of the shown file is pinned without changing the header", () => {
    expect(nextSelection(state, { kind: "event", eventFile: 10, eventKind: "warp", index: 1, header: 415 })).toEqual({
      selectedHeader: 3,
      pinned: { kind: "event", eventFile: 10, eventKind: "warp", index: 1 },
      panel: "world",
    });
  });

  it("an event of another file selects the header that reads it", () => {
    expect(nextSelection(state, { kind: "event", eventFile: 12, eventKind: "object", index: 0, header: 415 }).selectedHeader).toBe(415);
    // The hinted header must read that file; otherwise the event is pinned alone.
    expect(nextSelection(state, { kind: "event", eventFile: 12, eventKind: "object", index: 0, header: 4 }).selectedHeader).toBe(3);
    expect(nextSelection(state, { kind: "event", eventFile: 12, eventKind: "object", index: 0 }).selectedHeader).toBe(3);
  });
});
