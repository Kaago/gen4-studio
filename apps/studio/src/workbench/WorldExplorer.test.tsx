// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, MatrixSummary, ProjectInfo, ScriptFileDto, ScriptHitDto } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { placeColor } from "./places";
import { WorldExplorer } from "./WorldExplorer";
import type { WorldExplorerProps } from "./WorldExplorer";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listScriptFiles: vi.fn(), searchScripts: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, location_name: string, matrix_id: number, internal_name = `H${id}`): HeaderSummary => ({
  id,
  internal_name,
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

const SEARCH = 'input[aria-label="Welt durchsuchen"]';

describe("WorldExplorer", () => {
  let container: HTMLElement;
  let root: Root;
  const scrolled: string[] = [];
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    scrolled.length = 0;
    localStorage.clear();
    vi.mocked(listScriptFiles).mockReset();
    vi.mocked(searchScripts).mockReset();
    // jsdom has no layout, hence no scrollIntoView.
    Element.prototype.scrollIntoView = function (this: Element) {
      scrolled.push(this.getAttribute("data-header-id") ?? "?");
    };
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const props = (over: Partial<WorldExplorerProps> = {}): WorldExplorerProps => ({
    info: null,
    headers,
    matrices,
    matricesLoaded: true,
    matricesFailed: false,
    selected: null,
    shownMatrix: null,
    revealToken: 0,
    // The existing assertions read the rows with their identifiers: Tüfteln.
    depth: "tuefteln",
    onSelect: () => {},
    onOpenWizard: () => {},
    scriptsAdopted: true,
    onOpenScript: () => {},
    listOpen: true,
    onToggleList: () => {},
    ...over,
  });
  const render = (p: WorldExplorerProps) => act(() => root.render(<WorldExplorer {...p} />));
  const treeitems = () => [...container.querySelectorAll<HTMLElement>('[role="treeitem"]')];
  const places = () => [...container.querySelectorAll<HTMLElement>('[role="treeitem"][data-place-key]')];
  const expanded = () => places().map((p) => p.getAttribute("aria-expanded"));
  const row = (id: number) => container.querySelector<HTMLElement>(`[data-header-id="${id}"]`);
  const button = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name || b.getAttribute("aria-label") === name);
  const search = () => container.querySelector<HTMLInputElement>(SEARCH)!;
  const type = (input: HTMLInputElement, value: string) =>
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  /** What a click on a place does: it opens or closes it. */
  const toggle = (place: HTMLElement) => click(place);

  it("renders one treeitem per place with its colour chip and header count; only the selected header's place is open", () => {
    render(props({ selected: 1 }));
    expect(places()).toHaveLength(3);
    expect(places().map((p) => p.textContent)).toEqual(["Zweiblattdorf3", "Route 2011", "Ohne Ortsnamen1"]);
    expect(places().map((p) => p.getAttribute("aria-level"))).toEqual(["1", "1", "1"]);
    expect(expanded()).toEqual(["true", "false", "false"]);
    // The chip has the place's colour (the browser may write it in another notation, so both go through an element).
    const expectedColor = document.createElement("i");
    expectedColor.style.background = placeColor("Zweiblattdorf");
    expect(places()[0].querySelector<HTMLElement>("[data-place-chip]")?.style.background).toBe(expectedColor.style.background);
    expect(expectedColor.style.background).not.toBe("");
  });

  it("marks a header that a check found an error or warning at, in the tree and in the list", () => {
    render(props({ selected: 1, marks: new Map([[1, "error"], [2, "warning"]]) }));
    expect(row(1)?.querySelector("[data-finding-mark]")?.getAttribute("data-finding-mark")).toBe("error");
    expect(row(2)?.querySelector("[data-finding-mark]")?.getAttribute("aria-label")).toBe("Warnung gefunden");
    expect(row(0)?.querySelector("[data-finding-mark]")).toBeNull();
    click(button("Liste"));
    expect(row(1)?.querySelector("[data-finding-mark]")?.getAttribute("data-finding-mark")).toBe("error");
  });

  it("names each place for screen readers without running the count into the name", () => {
    render(props());
    expect(places().map((p) => p.getAttribute("aria-label"))).toEqual(["Zweiblattdorf, 3 Header", "Route 201, 1 Header", "Ohne Ortsnamen, 1 Header"]);
  });

  it("is a tree whose places and headers are treeitems on two levels", () => {
    render(props({ selected: 1 }));
    expect(container.querySelector('[role="tree"]')).not.toBeNull();
    expect(row(1)?.getAttribute("role")).toBe("treeitem");
    expect(row(1)?.getAttribute("aria-level")).toBe("2");
  });

  it("starts with every place closed when nothing is selected", () => {
    render(props());
    expect(expanded()).toEqual(["false", "false", "false"]);
    expect(row(0)).toBeNull();
  });

  it("opens and closes a place on a click and with the arrow keys", () => {
    render(props());
    toggle(places()[1]);
    expect(expanded()).toEqual(["false", "true", "false"]);
    expect(row(3)).not.toBeNull();
    pressKey(places()[1], "ArrowLeft");
    expect(expanded()).toEqual(["false", "false", "false"]);
    pressKey(places()[0], "ArrowRight");
    expect(expanded()).toEqual(["true", "false", "false"]);
  });

  it("writes outside headers by their place name, rooms by their internal name, and grey: id, code and the rooms' maps", () => {
    render(props({ selected: 1 }));
    toggle(places()[2]);
    expect(row(0)?.textContent).toBe("Zweiblattdorf0000 · T01");
    expect(row(1)?.textContent).toBe("T01R01010001 · 1 Karte");
    expect(row(2)?.textContent).toBe("T01R01020002 · 4 Karten (2 × 2)");
    expect(row(4)?.textContent).toBe("D010004 · 1 Karte");
  });

  it("leaves_the_id_and_the_code_to_the_tooltip_in_bauen_and_keeps_the_rooms_maps", () => {
    render(props({ selected: 1, depth: "bauen" }));
    toggle(places()[2]);
    expect(row(0)?.textContent).toBe("Zweiblattdorf");
    expect(row(1)?.textContent).toBe("T01R01011 Karte");
    expect(row(4)?.textContent).toBe("D011 Karte");
    expect(row(0)?.getAttribute("title")).toContain("Header 0000 T01");
  });

  it("shows_the_id_and_the_code_in_every_row_when_the_depth_switches_to_tuefteln", () => {
    render(props({ selected: 1, depth: "bauen" }));
    expect(row(0)?.textContent).not.toContain("0000");
    render(props({ selected: 1, depth: "tuefteln" }));
    expect(row(0)?.textContent).toBe("Zweiblattdorf0000 · T01");
  });

  it("shows 'Lade Übersicht …' and no room details until the overview is loaded", () => {
    render(props({ selected: 1, matricesLoaded: false, info: { root: "/p", game_code: "CPUE", family: "HGSS", language: "de", header_count: 5 } }));
    expect(container.textContent).toContain("Lade Übersicht …");
    // The tree is there at once, with the places and the headers' ids and codes.
    expect(places()).toHaveLength(3);
    expect(row(1)?.textContent).toBe("Zweiblattdorf0001 · T01R0101");
    render(props({ selected: 1, info: { root: "/p", game_code: "CPUE", family: "HGSS", language: "de", header_count: 5 } }));
    expect(container.textContent).not.toContain("Lade Übersicht");
    expect(container.textContent).toContain("3 Orte, 5 Header");
    expect(row(1)?.textContent).toBe("T01R01010001 · 1 Karte");
  });

  it("says the overview is not available when its first listing failed, instead of loading for ever", () => {
    render(props({ matricesLoaded: false, matricesFailed: true }));
    expect(container.textContent).toContain("Übersicht nicht verfügbar");
    expect(container.textContent).not.toContain("Lade Übersicht");
    expect(places()).toHaveLength(3);
    // Loaded wins: a later failing read does not hide what is known.
    render(props({ matricesFailed: true }));
    expect(container.textContent).toContain("3 Orte, 5 Header");
  });

  it("does not guess outside and rooms before the overview is known: every row has the place's name, in id order", () => {
    // With the overview, header 2 (matrix 0, outside) would come first and header 1 would be named by its code.
    const listed = [header(1, "Ort", 7, "R1"), header(2, "Ort", 0, "T")];
    render(props({ headers: listed, matricesLoaded: false, selected: 1 }));
    expect([...container.querySelectorAll("[data-header-id]")].map((e) => e.textContent)).toEqual(["Ort0001 · R1", "Ort0002 · T"]);
    render(props({ headers: listed, selected: 1 }));
    expect([...container.querySelectorAll("[data-header-id]")].map((e) => e.textContent)).toEqual(["Ort0002 · T", "R10001 · 1 Karte"]);
  });

  it("marks the selected row, and the rows of the shown matrix with a text of their own", () => {
    render(props({ selected: 3, shownMatrix: 0 }));
    toggle(places()[0]);
    expect(row(3)?.getAttribute("aria-selected")).toBe("true");
    expect(row(0)?.getAttribute("aria-selected")).toBe("false");
    // Not by colour alone: the marked rows carry a name for assistive technology.
    expect(row(0)?.querySelector('[aria-label="Raster wird angezeigt"]')).not.toBeNull();
    expect(row(3)?.querySelector('[aria-label="Raster wird angezeigt"]')).not.toBeNull();
    expect(row(1)?.querySelector('[aria-label="Raster wird angezeigt"]')).toBeNull();
  });

  it("calls onSelect with the header of the clicked row", () => {
    const onSelect = vi.fn();
    render(props({ selected: 1, onSelect }));
    click(row(2));
    expect(onSelect).toHaveBeenCalledWith(headers[2]);
  });

  it("selects with Enter on a focused row", () => {
    const onSelect = vi.fn();
    render(props({ selected: 1, onSelect }));
    pressKey(row(2)!, "Enter");
    expect(onSelect).toHaveBeenCalledWith(headers[2]);
  });

  it("a search keeps the places that match, opened", () => {
    render(props());
    type(search(), "Route");
    expect(places()).toHaveLength(1);
    expect(places()[0].textContent).toContain("Route 201");
    expect(expanded()).toEqual(["true"]);
    // A code searches inside the places.
    type(search(), "r0102");
    expect(places().map((p) => p.textContent)).toEqual(["Zweiblattdorf1"]);
    expect(row(2)).not.toBeNull();
    expect(row(1)).toBeNull();
  });

  it("filters while typing, not only when the field is left", () => {
    render(props());
    act(() => search().focus());
    type(search(), "Rou");
    expect(places()).toHaveLength(1);
    expect(document.activeElement).toBe(search());
  });

  it("clears a search that hides a newly selected header, then scrolls to it", () => {
    render(props());
    type(search(), "Route");
    expect(places()).toHaveLength(1);
    render(props({ selected: 4 }));
    expect(places()).toHaveLength(3);
    expect(search().value).toBe("");
    expect(expanded()).toEqual(["false", "false", "true"]);
    expect(scrolled).toEqual(["4"]);
  });

  it("does not pull a selection back that a later search hides", () => {
    render(props({ selected: 1 }));
    expect(scrolled).toEqual(["1"]);
    type(search(), "Route");
    expect(places()).toHaveLength(1);
    expect(search().value).toBe("Route");
    expect(scrolled).toEqual(["1"]);
  });

  it("opens the place of a newly selected header, keeps the others, and scrolls to the row once it is visible", () => {
    render(props({ selected: 1 }));
    expect(scrolled).toEqual(["1"]);
    render(props({ selected: 3 }));
    expect(expanded()).toEqual(["true", "true", "false"]);
    expect(scrolled).toEqual(["1", "3"]);
  });

  it("scrolls again for a new reveal token, also for the header that is selected already", () => {
    render(props({ selected: 2 }));
    expect(scrolled).toEqual(["2"]);
    render(props({ selected: 2 }));
    expect(scrolled).toEqual(["2"]);
    render(props({ selected: 2, revealToken: 1 }));
    expect(scrolled).toEqual(["2", "2"]);
  });

  it("opens a place a link reveals although the user closed it", () => {
    render(props({ selected: 1 }));
    toggle(places()[0]);
    expect(expanded()[0]).toBe("false");
    render(props({ selected: 1, revealToken: 1 }));
    expect(expanded()[0]).toBe("true");
    expect(scrolled).toEqual(["1", "1"]);
  });

  it("keeps a place the user opened open", () => {
    render(props());
    toggle(places()[1]);
    render(props({ selected: 0 }));
    expect(expanded()).toEqual(["true", "true", "false"]);
  });

  describe("the head", () => {
    it("is titled Orte, names the game and folds the list through the grip", () => {
      const onToggleList = vi.fn();
      render(props({ onToggleList, info: { root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: 5 } }));
      expect(container.querySelector("h2")?.textContent).toBe("Orte");
      expect(container.textContent).toContain("Platin, EN");
      // The view switch is symbols only (so the game name has room); the options keep their names.
      const view = container.querySelector('[role="group"][aria-label="Ansicht"]')!;
      expect([...view.querySelectorAll("button")].map((b) => [b.textContent, b.getAttribute("aria-label")])).toEqual([
        ["", "Baum"],
        ["", "Liste"],
      ]);
      act(() => button("Liste einklappen")!.click());
      expect(onToggleList).toHaveBeenCalledTimes(1);
    });

    it("keeps the tree and its state while the list is folded", () => {
      render(props({ selected: 1 }));
      type(search(), "Zwei");
      render(props({ selected: 1, listOpen: false }));
      expect(container.querySelector("section")?.hidden).toBe(true);
      render(props({ selected: 1, listOpen: true }));
      expect(search().value).toBe("Zwei");
    });
  });

  describe("the + menu", () => {
    const trigger = () => button("Neu anlegen")!;
    const press = (el: HTMLElement) =>
      act(() => {
        el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
        el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
        el.click();
      });
    const items = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];

    it("opens a menu whose entries ask for the wizards", () => {
      const onOpenWizard = vi.fn();
      render(props({ onOpenWizard }));
      expect(trigger().getAttribute("aria-haspopup")).toBe("menu");
      expect(document.querySelector('[role="menu"]')).toBeNull();
      press(trigger());
      expect(document.querySelector('[role="menu"]')).not.toBeNull();
      expect(items().map((e) => e.textContent)).toEqual(["Gebiet anlegen…", "Raum anlegen…"]);
      act(() => items()[0].click());
      expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "area" });
      expect(document.querySelector('[role="menu"]')).toBeNull();
      press(trigger());
      act(() => items()[1].click());
      expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "room" });
    });

    it("closes on Escape", () => {
      render(props());
      press(trigger());
      expect(document.querySelector('[role="menu"]')).not.toBeNull();
      pressKey(document.querySelector<HTMLElement>('[role="menu"]')!, "Escape");
      expect(document.querySelector('[role="menu"]')).toBeNull();
    });

    it("stays in the list mode too", () => {
      render(props());
      act(() => button("Liste")!.click());
      expect(trigger()).toBeDefined();
    });
  });

  describe("tree / list switch", () => {
    it("shows the header list in the list mode and remembers the mode", () => {
      render(props({ selected: 1 }));
      expect(container.querySelector('input[aria-label="Header suchen"]')).toBeNull();
      act(() => button("Liste")!.click());
      expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
      expect(container.querySelector(SEARCH)).toBeNull();
      expect(places()).toHaveLength(0);
      expect(container.querySelectorAll('[role="option"]')).toHaveLength(5);
      expect(localStorage.getItem("g4.explorer.mode")).toBe("list");
      act(() => root.unmount());
      root = createRoot(container);
      render(props());
      expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
      act(() => button("Baum")!.click());
      expect(places()).toHaveLength(3);
      expect(localStorage.getItem("g4.explorer.mode")).toBe("tree");
    });

    it("works without a usable localStorage", () => {
      const original = Object.getOwnPropertyDescriptor(window, "localStorage")!;
      Object.defineProperty(window, "localStorage", {
        configurable: true,
        get: () => {
          throw new Error("denied");
        },
      });
      try {
        render(props());
        act(() => button("Liste")!.click());
        expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
      } finally {
        Object.defineProperty(window, "localStorage", original);
      }
    });

    it("selects a header in the list mode with a click and marks it", () => {
      const onSelect = vi.fn();
      localStorage.setItem("g4.explorer.mode", "list");
      render(props({ selected: 3, onSelect }));
      expect(row(3)?.getAttribute("aria-selected")).toBe("true");
      click(row(1));
      expect(onSelect).toHaveBeenCalledWith(headers[1]);
    });
  });

  describe("the arrow keys", () => {
    const tabStops = (selector: string) => [...container.querySelectorAll<HTMLElement>(selector)].filter((el) => el.tabIndex === 0);

    // The roving list repairs its tab stop in a mutation observer callback, a microtask after the rows changed.
    const settle = () => act(async () => {});

    it("leave exactly one tab stop in the tree, after a search and after switching to the list and back", async () => {
      render(props({ selected: 1 }));
      expect(tabStops('[role="treeitem"]')).toHaveLength(1);
      type(search(), "Route");
      await settle();
      expect(tabStops('[role="treeitem"]')).toHaveLength(1);
      act(() => button("Liste")!.click());
      expect(tabStops('[role="option"]')).toHaveLength(1);
      type(container.querySelector<HTMLInputElement>('input[aria-label="Header suchen"]')!, "Route");
      await settle();
      expect(tabStops('[role="option"]')).toHaveLength(1);
      act(() => button("Baum")!.click());
      expect(tabStops('[role="treeitem"]')).toHaveLength(1);
    });

    it("move between the rows of the tree and of the list", () => {
      render(props({ selected: 1 }));
      const first = treeitems()[0];
      pressKey(first, "ArrowDown");
      expect(document.activeElement).toBe(treeitems()[1]);
      act(() => button("Liste")!.click());
      const options = [...container.querySelectorAll<HTMLElement>('[role="option"]')];
      pressKey(options[0], "ArrowDown");
      expect(document.activeElement).toBe(options[1]);
    });
  });

  it("does not render its rows again for the same props, and only the changed rows for a new selection", () => {
    let reads = 0;
    const counted = headers.map((h) => {
      const c = { ...h };
      Object.defineProperty(c, "internal_name", { enumerable: true, get: () => (reads++, h.internal_name) });
      return c;
    });
    const base = props({ headers: counted, selected: 0 });
    render(base);
    const first = reads;
    expect(first).toBeGreaterThan(0);
    render({ ...base });
    expect(reads).toBe(first);
    // Selection 0 -> 3 renders the two outside rows whose mark changes, not the three of the open place.
    render({ ...base, selected: 3 });
    expect(reads - first).toBeGreaterThan(0);
    expect(reads - first).toBeLessThan(first);
  });

  describe("scripts", () => {
    const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const scriptsRoot = () => container.querySelector<HTMLElement>("[data-scripts-root]")!;
    const openScripts = () => click(scriptsRoot());
    /** The script files below the root node (level 2); the children of a header are level 3. */
    const fileRows = () => [...container.querySelectorAll<HTMLElement>('[data-script-file][aria-level="2"]')];
    const headerChildren = () => [...container.querySelectorAll<HTMLElement>('[data-script-file][aria-level="3"]')];
    const scriptBox = () => container.querySelector<HTMLInputElement>('input[aria-label="Scripts durchsuchen"]')!;
    const file = (n: number, kind: ScriptFileDto["kind"] = "script", headers: number[] = []): ScriptFileDto => ({ file: n, kind, size: 120, headers });
    const hit = (n: number, line: number, excerpt: string): ScriptHitDto => ({ file: n, line, excerpt });
    const hitRows = () => treeitems().filter((t) => t.textContent?.includes("Zeile"));
    const withScripts = headers.map((h) => (h.id === 0 ? { ...h, script_file_id: 0, level_script_id: 1 } : { ...h, script_file_id: 5, level_script_id: 0 }));
    const toggleHeader = (id: number) => pressKey(row(id)!, "ArrowRight");

    it("lists_the_scripts_node_first_in_the_tree", () => {
      render(props());
      expect(treeitems()[0]).toBe(scriptsRoot());
      expect(scriptsRoot().textContent).toBe("Scripts");
      expect(scriptsRoot().getAttribute("aria-expanded")).toBe("false");
    });

    it("shows_the_script_file_and_level_script_below_an_expanded_header_and_opens_them", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, selected: 0, onOpenScript }));
      expect(headerChildren()).toHaveLength(0);
      expect(row(0)?.getAttribute("aria-expanded")).toBe("false");
      toggleHeader(0);
      expect(row(0)?.getAttribute("aria-expanded")).toBe("true");
      expect(headerChildren().map((b) => b.textContent)).toEqual(["Scriptdatei 0000", "Level-Script 0001"]);
      click(headerChildren()[0]);
      click(headerChildren()[1]);
      expect(onOpenScript.mock.calls).toEqual([[0], [1]]);
      // A header without a level script has only its script file; collapsing hides the children again.
      toggle(places()[1]);
      toggleHeader(3);
      expect(headerChildren().map((b) => b.textContent)).toEqual(["Scriptdatei 0000", "Level-Script 0001", "Scriptdatei 0005"]);
      pressKey(row(0)!, "ArrowLeft");
      expect(headerChildren().map((b) => b.textContent)).toEqual(["Scriptdatei 0005"]);
    });

    it("disables_the_header_children_and_asks_for_the_adoption_while_scripts_are_not_adopted", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, selected: 0, scriptsAdopted: false, onOpenScript }));
      toggleHeader(0);
      const child = headerChildren()[0];
      expect(child.getAttribute("aria-disabled")).toBe("true");
      expect(child.title).toBe("Erst Scripts übernehmen (Panel „Script“)");
      click(child);
      pressKey(child, "Enter");
      expect(onOpenScript).not.toHaveBeenCalled();
    });

    it("reads_the_script_list_when_the_root_node_is_first_opened_not_before", async () => {
      vi.mocked(listScriptFiles).mockResolvedValue([file(0, "script", [0, 4]), file(1, "levelScript", [0]), file(2, "binary")]);
      const onOpenScript = vi.fn();
      render(props({ onOpenScript }));
      await flush();
      expect(listScriptFiles).not.toHaveBeenCalled();
      openScripts();
      await flush();
      expect(listScriptFiles).toHaveBeenCalledTimes(1);
      expect(scriptsRoot().getAttribute("aria-expanded")).toBe("true");
      expect(fileRows().map((r) => r.textContent)).toEqual([
        "Scriptdatei 0000120 Byte · Header 0000, 0004",
        "Level-Script 0001120 Byte · Header 0000",
        "Scriptdatei 0002binär · 120 Byte · ohne Header",
      ]);
      expect(scriptsRoot().textContent).toBe("Scripts3");
      click(fileRows()[1]);
      expect(onOpenScript).toHaveBeenCalledWith(1);
    });

    it("says_to_adopt_first_and_calls_no_command_while_scripts_are_not_adopted", async () => {
      render(props({ scriptsAdopted: false }));
      openScripts();
      await flush();
      expect(container.textContent).toContain("Erst Scripts übernehmen (Panel „Script“)");
      expect(scriptBox()).toBeNull();
      expect(listScriptFiles).not.toHaveBeenCalled();
      expect(searchScripts).not.toHaveBeenCalled();
    });

    it("searches_after_a_pause_in_typing_and_opens_a_hit_at_its_line", async () => {
      vi.useFakeTimers();
      try {
        vi.mocked(listScriptFiles).mockResolvedValue([file(0)]);
        vi.mocked(searchScripts).mockResolvedValue([hit(42, 7, "script Main #1"), hit(43, 2, "Message 3")]);
        const onOpenScript = vi.fn();
        render(props({ onOpenScript }));
        openScripts();
        const box = scriptBox();
        type(box, "Ma");
        await act(async () => void vi.advanceTimersByTime(200));
        type(box, "Main");
        await act(async () => void vi.advanceTimersByTime(299));
        expect(searchScripts).not.toHaveBeenCalled();
        await act(async () => void vi.advanceTimersByTime(1));
        // Only the last text was searched, once.
        expect(searchScripts).toHaveBeenCalledTimes(1);
        expect(searchScripts).toHaveBeenCalledWith("Main", 100);
        expect(hitRows().map((h) => h.textContent)).toEqual(["Scriptdatei 0042 · Zeile 7 · script Main #1", "Scriptdatei 0043 · Zeile 2 · Message 3"]);
        click(hitRows()[0]);
        expect(onOpenScript).toHaveBeenCalledWith(42, 7);
        // Clearing the search shows the list again.
        type(box, "");
        expect(fileRows()[0]?.textContent).toContain("Scriptdatei 0000");
      } finally {
        vi.useRealTimers();
      }
    });

    it("says_so_when_nothing_matches", async () => {
      vi.useFakeTimers();
      try {
        vi.mocked(listScriptFiles).mockResolvedValue([]);
        vi.mocked(searchScripts).mockResolvedValue([]);
        render(props());
        openScripts();
        type(scriptBox(), "xyz");
        expect(container.textContent).toContain("Suche …");
        await act(async () => void vi.advanceTimersByTime(300));
        expect(container.textContent).toContain("Keine Treffer.");
      } finally {
        vi.useRealTimers();
      }
    });

    it("reports_a_failed_search_as_an_alert", async () => {
      vi.useFakeTimers();
      try {
        vi.mocked(listScriptFiles).mockResolvedValue([]);
        vi.mocked(searchScripts).mockRejectedValue("Suche abgebrochen");
        render(props());
        openScripts();
        type(scriptBox(), "xyz");
        await act(async () => void vi.advanceTimersByTime(300));
        expect(container.querySelector('[role="alert"]')?.textContent).toContain("Suche abgebrochen");
      } finally {
        vi.useRealTimers();
      }
    });

    it("does_not_show_a_list_that_arrives_for_another_project", async () => {
      const a: ProjectInfo = { root: "/a", game_code: "CPUE", family: "Pt", language: "en", header_count: 5 };
      const b: ProjectInfo = { ...a, root: "/b" };
      let late!: (files: ScriptFileDto[]) => void;
      vi.mocked(listScriptFiles)
        .mockImplementationOnce(() => new Promise<ScriptFileDto[]>((resolve) => (late = resolve)))
        .mockResolvedValueOnce([file(9)]);
      render(props({ info: a }));
      openScripts();
      render(props({ info: b }));
      await flush();
      expect(container.textContent).toContain("Scriptdatei 0009");
      await act(async () => late([file(1)]));
      expect(container.textContent).toContain("Scriptdatei 0009");
      expect(container.textContent).not.toContain("Scriptdatei 0001");
    });
  });
});
