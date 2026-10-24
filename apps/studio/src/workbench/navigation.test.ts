import { describe, expect, it } from "vitest";
import type { HeaderSummary } from "../api";
import { nextSelection, withHeaderHint } from "./navigation";
import type { NavState } from "./navigation";

const header = (id: number, matrix_id: number, event_file_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
  matrix_id,
  area_data_id: 0,
  event_file_id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
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
      area: "welt",
      selectedHeader: 3,
      pinned: { kind: "matrixCell", matrix: 0, x: 1, y: 2 },
      panel: "matrix",
    });
    // A header id the project does not have is no link.
    expect(nextSelection(state, { kind: "matrixCell", matrix: 0, x: 1, y: 2, header: 999 }).selectedHeader).toBe(3);
  });

  it("header selects and pins it", () => {
    expect(nextSelection(state, { kind: "header", id: 4 })).toEqual({ area: "welt", selectedHeader: 4, pinned: { kind: "header", id: 4 }, panel: "headers" });
  });

  it("a script opens in the script panel at its line and leaves selection and pin alone", () => {
    expect(nextSelection(state, { kind: "script", file: 42, line: 7 })).toEqual({
      area: "welt",
      selectedHeader: 3,
      pinned: null,
      panel: "script",
      script: { file: 42, line: 7 },
    });
    expect(nextSelection(state, { kind: "script", file: 42 })).toMatchObject({ panel: "script", script: { file: 42, line: null } });
  });

  it("an unknown header changes nothing", () => {
    expect(nextSelection(state, { kind: "header", id: 999 })).toEqual({ area: "welt", selectedHeader: 3, pinned: null, panel: null });
  });

  it("a text archive leads to the messages of the text area and leaves the selection and the pin alone", () => {
    expect(nextSelection(state, { kind: "textArchive", index: 433, message: 2 })).toEqual({
      area: "texte",
      selectedHeader: 3,
      pinned: null,
      panel: "textEdit",
      text: { index: 433, message: 2 },
    });
    expect(nextSelection(state, { kind: "textArchive", index: 23 })).toMatchObject({ area: "texte", panel: "textEdit", text: { index: 23, message: null } });
  });

  it("a species, a move and an item lead to their data areas and leave the selection and the pin alone", () => {
    expect(nextSelection(state, { kind: "species", index: 397 })).toEqual({
      area: "pokemon",
      selectedHeader: 3,
      pinned: null,
      panel: "pokemonView",
      data: { sub: "pokedex", index: 397 },
    });
    // A tab of the Pokédex hand opens the hand.
    expect(nextSelection(state, { kind: "species", index: 397, tab: "learnset" })).toMatchObject({ area: "pokemon", panel: "pokemonEdit", data: { sub: "pokedex", index: 397, tab: "learnset" } });
    expect(nextSelection(state, { kind: "move", index: 17 })).toMatchObject({ area: "pokemon", pinned: null, panel: "pokemonView", data: { sub: "moves", index: 17 } });
    expect(nextSelection(state, { kind: "trainer", index: 12 })).toEqual({ area: "pokemon", selectedHeader: 3, pinned: null, panel: "pokemonView", data: { sub: "trainer", index: 12 } });
    // An encounter file opens Pokémon › Begegnungen.
    expect(nextSelection(state, { kind: "encounter", index: 140 })).toEqual({ area: "pokemon", selectedHeader: 3, pinned: null, panel: "pokemonView", data: { sub: "encounters", index: 140 } });
    expect(nextSelection(state, { kind: "item", id: 17 })).toEqual({
      area: "gegenstaende",
      selectedHeader: 3,
      pinned: null,
      panel: "itemView",
      data: { sub: "items", index: 17 },
    });
  });

  it("every other target is in the world", () => {
    const targets = [
      { kind: "header", id: 4 },
      { kind: "header", id: 999 },
      { kind: "matrixCell", matrix: 0, x: 1, y: 2 },
      { kind: "area", id: 22 },
      { kind: "building", map: 101, index: 2 },
      { kind: "event", eventFile: 10, eventKind: "warp", index: 1 },
      { kind: "script", file: 42 },
    ] as const;
    for (const target of targets) expect(nextSelection(state, target).area).toBe("welt");
  });

  it("area pins the area data and keeps the selected header", () => {
    expect(nextSelection(state, { kind: "area", id: 22 })).toEqual({ area: "welt", selectedHeader: 3, pinned: { kind: "area", id: 22 }, panel: "inspector" });
  });

  it("a building of a shown map is pinned without changing the header", () => {
    expect(nextSelection(state, { kind: "building", map: 101, index: 2, header: 415 })).toEqual({
      area: "welt",
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
      area: "welt",
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

describe("withHeaderHint", () => {
  const h = (id: number, event_file_id: number) => ({ id, event_file_id }) as HeaderSummary;
  const headers = [h(411, 390), h(415, 394), h(416, 394)];

  it("gives_an_event_without_a_header_the_first_header_that_reads_its_file", () => {
    expect(withHeaderHint({ kind: "event", eventFile: 394, eventKind: "warp", index: 0 }, headers)).toEqual({ kind: "event", eventFile: 394, eventKind: "warp", index: 0, header: 415 });
    // So the place is shown first: the scene holds the event.
    expect(nextSelection({ selectedHeader: 411, matrix: 0, eventFile: 390, headers }, withHeaderHint({ kind: "event", eventFile: 394, eventKind: "warp", index: 0 }, headers)).selectedHeader).toBe(415);
  });

  it("leaves_a_given_header_an_unknown_file_and_every_other_target_alone", () => {
    const hinted = { kind: "event", eventFile: 394, eventKind: "warp", index: 0, header: 416 } as const;
    expect(withHeaderHint(hinted, headers)).toBe(hinted);
    const orphan = { kind: "event", eventFile: 999, eventKind: "object", index: 1 } as const;
    expect(withHeaderHint(orphan, headers)).toBe(orphan);
    const script = { kind: "script", file: 5 } as const;
    expect(withHeaderHint(script, headers)).toBe(script);
  });
});
