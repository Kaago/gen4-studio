// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, splitEncounterFile } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { readShellState } from "../../ui/frame/shellState";
import { ProblemsStore } from "../../workbench/problems/problems";
import { resetIcons } from "../icons";
import { FILE_ROWS, ROUTE_201, SHARED_FILE, encounterHarness, fileRow, wireApi } from "./encounterTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  createEncounterFile: vi.fn(),
  splitEncounterFile: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("EncounterView", () => {
  let h: ReturnType<typeof encounterHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi((index) => (index === 141 ? SHARED_FILE : ROUTE_201));
    Element.prototype.scrollIntoView = () => {};
    h = encounterHarness();
  });
  afterEach(() => h.unmount());

  const waiting = () => [...h.view().querySelectorAll<HTMLElement>("[data-species]")];

  it("shows_who_waits_in_the_grass_with_the_share_per_time_of_day", async () => {
    await h.open(140);
    expect(h.view().querySelector("h2")?.textContent).toBe("Route 201");
    expect(h.view().textContent).toContain("Wer hier wartet");
    expect(waiting().map((w) => Number(w.dataset.species))).toEqual([396, 399]);
    const staralili = waiting()[0]!;
    expect(staralili.textContent).toContain("Staralili");
    expect(staralili.textContent).toContain("Morgens 75 %");
    expect(staralili.textContent).toContain("Tags 75 %");
    expect(staralili.textContent).toContain("Nachts 65 %");
    // Its types from the Pokédex and its picture.
    expect([...staralili.querySelectorAll("[data-type]")].map((t) => t.getAttribute("data-type"))).toEqual(["0", "2"]);
    expect(staralili.querySelector("[data-sprite]")).not.toBeNull();
  });

  it("lists_the_places_that_use_the_file_as_references_into_the_world_without_a_split_for_one", async () => {
    await h.open(140);
    expect(h.view().textContent).toContain("Verwendet von 1 Ort");
    const place = h.view().querySelector<HTMLElement>('[data-header="342"]')!;
    expect(place.textContent).toContain("Route 201");
    await h.click(place);
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "header", id: 342 });
    expect(h.button("Route 201: für diesen Ort abspalten…")).toBeUndefined();
  });

  it("tells_where_a_file_no_place_uses_gets_bound_in_the_words_of_the_world", async () => {
    // Since 3e.3 the header editor is the inspector of an area or a room; „Header-Editor“ names nothing the user sees.
    wireApi(() => ({ ...ROUTE_201, users: [] }));
    await h.open(140);
    expect(h.view().textContent).toContain("Von keinem Ort verwendet");
    expect(h.view().textContent).toContain("Binde die Datei im Inspector eines Gebiets oder Raums an (Feld „Begegnungen“), dann gilt sie dort.");
    expect(h.view().textContent).not.toContain("Header-Editor");
  });

  it("splits_a_file_for_one_of_its_places_after_a_question_and_selects_the_copy", async () => {
    await h.open(141);
    expect(h.view().textContent).toContain("Verwendet von 2 Orten");
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    await h.click(h.button("Route 203: für diesen Ort abspalten…"));
    const dialog = h.dialog()!;
    expect(dialog.textContent).toContain("Route 203 eine eigene Begegnungsdatei geben?");
    expect(dialog.textContent).toContain("Nicht rückgängig");
    expect(dialog.textContent).toContain("Das Umhängen des Headers ist rückgängig");
    await h.click(h.button("Abspalten"));
    expect(splitEncounterFile).toHaveBeenCalledWith(401);
    expect(readShellState().selected.encounters).toBe(183);
    expect(heard).toContainEqual({ topic: "headerTable" });
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "encounter", index: null });
    expect(h.calls.onEdited).toHaveBeenCalled();
    expect(h.dialog()).toBeNull();
  });

  it("treats_a_partial_split_like_one_that_went_through_and_says_what_stays", async () => {
    await h.open(141);
    const heard: EditEvent[] = [];
    h.bus.subscribe(() => true, (e) => heard.push(e));
    // As the app does: the data store reads again what the bus announces.
    h.bus.subscribe(() => true, (e) => h.store.onEvent(e));
    // The copy was written, switching the header failed; the list read again has the copy.
    vi.mocked(splitEncounterFile).mockRejectedValue({
      code: "partial",
      message: "encounter file 183 was written as a copy of 141; switching header 401 to it failed: disk full",
      asset: null,
      problems: [],
    });
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "encounter" ? [...FILE_ROWS, fileRow(183, [])] : []));
    await h.click(h.button("Route 203: für diesen Ort abspalten…"));
    await h.click(h.button("Abspalten"));
    expect(h.dialog()).toBeNull();
    expect(readShellState().selected.encounters).toBe(183);
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "encounter", index: null });
    expect(heard).toContainEqual({ topic: "headerTable" });
    expect(h.calls.onEdited).toHaveBeenCalled();
    expect(h.view().textContent).toContain(
      "Begegnungsdatei 183 ist als Kopie von 141 angelegt und bleibt. Danach fehlgeschlagen: Header 401 auf sie umhängen (disk full).",
    );
  });

  it("shows_the_warnings_in_german_and_leaves_the_problems_list_to_the_checks", async () => {
    const problems = new ProblemsStore();
    await h.open(140, "bauen", { problems });
    const notices = [...h.view().querySelectorAll<HTMLElement>("[data-warning]")];
    expect(notices.map((n) => n.textContent)).toEqual([expect.stringContaining("Nacht Eintrag 1 ist leer: von 20 bis 3 Uhr erscheint auf Gras Platz 3 ein Glitch-Pokémon")]);
    // The rules of the file are the checks `encounter.*` now: they reach the Problems list from `run_checks`, not from the document.
    expect(problems.list()).toEqual([]);
  });
});
