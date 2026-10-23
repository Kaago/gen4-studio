// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, MatrixSummary, ProjectInfo, ScriptFileDto, ScriptHitDto } from "../api";
import { placeColor } from "./places";
import { TERMS } from "./terms";
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
    onSelect: () => {},
    onOpenWizard: () => {},
    scriptsAdopted: true,
    onOpenScript: () => {},
    ...over,
  });
  const render = (p: WorldExplorerProps) => act(() => root.render(<WorldExplorer {...p} />));
  const places = () => [...container.querySelectorAll<HTMLDetailsElement>("details[data-place-key]")];
  const row = (id: number) => container.querySelector<HTMLButtonElement>(`[data-header-id="${id}"]`);
  const button = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name);
  const search = () => container.querySelector<HTMLInputElement>(SEARCH)!;
  const type = (input: HTMLInputElement, value: string) =>
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  /** What the browser does when the user clicks a summary: the open state changes and a toggle event follows. */
  const toggle = (details: HTMLDetailsElement, open: boolean) =>
    act(() => {
      details.open = open;
      details.dispatchEvent(new Event("toggle"));
    });

  it("renders one details per place with its colour chip and header count; only the selected header's place is open", () => {
    render(props({ selected: 1 }));
    expect(places()).toHaveLength(3);
    expect(places().map((d) => d.querySelector("summary")?.textContent)).toEqual(["Zweiblattdorf3", "Route 2011", "Ohne Ortsnamen1"]);
    expect(places().map((d) => d.open)).toEqual([true, false, false]);
    // The chip has the place's colour (the browser may write it in another notation, so both go through an element).
    const expected = document.createElement("i");
    expected.style.background = placeColor("Zweiblattdorf");
    expect(places()[0].querySelector<HTMLElement>("summary [data-place-chip]")?.style.background).toBe(expected.style.background);
    expect(expected.style.background).not.toBe("");
  });

  it("names each place for screen readers without running the count into the name", () => {
    render(props());
    expect(places().map((d) => d.querySelector("summary")?.getAttribute("aria-label"))).toEqual(["Zweiblattdorf, 3 Header", "Route 201, 1 Header", "Ohne Ortsnamen, 1 Header"]);
    expect(places()[0].querySelector('summary [aria-hidden="true"]')?.textContent).toBe("3");
  });

  it("starts with every place closed when nothing is selected", () => {
    render(props());
    expect(places().map((d) => d.open)).toEqual([false, false, false]);
  });

  it("writes outside headers by their place name, rooms by their internal name, and grey: id, code and the rooms' maps", () => {
    render(props({ selected: 1 }));
    expect(row(0)?.textContent).toBe("Zweiblattdorf0000 · T01");
    expect(row(1)?.textContent).toBe("T01R01010001 · 1 Karte");
    expect(row(2)?.textContent).toBe("T01R01020002 · 4 Karten (2 × 2)");
    expect(row(4)?.textContent).toBe("D010004 · 1 Karte");
  });

  it("shows 'Lade Übersicht …' and no room details until the overview is loaded", () => {
    render(props({ selected: 1, matricesLoaded: false, info: { root: "/p", game_code: "CPUE", family: "HGSS", language: "de", header_count: 5 } }));
    expect(container.textContent).toContain("Lade Übersicht …");
    // The tree is there at once, with the places and the headers' ids and codes.
    expect(places()).toHaveLength(3);
    expect(row(1)?.textContent).toBe("Zweiblattdorf0001 · T01R0101");
    render(props({ selected: 1 }));
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

  it("marks the selected row and the rows of the shown matrix", () => {
    render(props({ selected: 3, shownMatrix: 0 }));
    expect(row(3)?.getAttribute("aria-current")).toBe("true");
    expect(row(0)?.getAttribute("aria-current")).toBeNull();
    expect(row(0)?.style.boxShadow).toContain("inset");
    expect(row(1)?.style.boxShadow).toBe("none");
  });

  it("calls onSelect with the header of the clicked row", () => {
    const onSelect = vi.fn();
    render(props({ selected: 1, onSelect }));
    act(() => row(3)?.click());
    expect(onSelect).toHaveBeenCalledWith(headers[3]);
  });

  it("a search keeps the places that match, opened", () => {
    render(props());
    type(search(), "Route");
    expect(places()).toHaveLength(1);
    expect(places()[0].querySelector("summary")?.textContent).toContain("Route 201");
    expect(places()[0].open).toBe(true);
    // A code searches inside the places.
    type(search(), "r0102");
    expect(places().map((d) => d.querySelector("summary")?.textContent)).toEqual(["Zweiblattdorf1"]);
    expect(row(2)).not.toBeNull();
    expect(row(1)).toBeNull();
  });

  it("clears a search that hides a newly selected header, then scrolls to it", () => {
    render(props());
    type(search(), "Route");
    expect(places()).toHaveLength(1);
    render(props({ selected: 4 }));
    expect(places()).toHaveLength(3);
    expect(search().value).toBe("");
    expect(places().map((d) => d.open)).toEqual([false, false, true]);
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
    expect(places().map((d) => d.open)).toEqual([true, true, false]);
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
    toggle(places()[0], false);
    expect(places()[0].open).toBe(false);
    render(props({ selected: 1, revealToken: 1 }));
    expect(places()[0].open).toBe(true);
    expect(scrolled).toEqual(["1", "1"]);
  });

  it("keeps a place the user opened open", () => {
    render(props());
    toggle(places()[1], true);
    render(props({ selected: 0 }));
    expect(places().map((d) => d.open)).toEqual([true, true, false]);
  });

  describe("+ Neu", () => {
    it("opens a menu whose entries ask for the wizards", () => {
      const onOpenWizard = vi.fn();
      render(props({ onOpenWizard }));
      const neu = button("+ Neu")!;
      expect(neu.getAttribute("aria-haspopup")).toBe("menu");
      expect(container.querySelector('[role="menu"]')).toBeNull();
      act(() => neu.click());
      expect(neu.getAttribute("aria-expanded")).toBe("true");
      expect([...container.querySelectorAll('[role="menuitem"]')].map((e) => e.textContent)).toEqual(["Gebiet anlegen…", "Raum anlegen…"]);
      act(() => button("Gebiet anlegen…")?.click());
      expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "area" });
      expect(container.querySelector('[role="menu"]')).toBeNull();
      act(() => button("+ Neu")!.click());
      act(() => button("Raum anlegen…")?.click());
      expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "room" });
    });

    it("is used with the keyboard: focus on the first entry, arrows wrap, Escape gives the focus back", () => {
      render(props());
      const neu = button("+ Neu")!;
      neu.focus();
      act(() => neu.click());
      const items = () => [...container.querySelectorAll<HTMLElement>('[role="menuitem"]')];
      expect(document.activeElement).toBe(items()[0]);
      const press = (key: string) => act(() => void document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
      press("ArrowDown");
      expect(document.activeElement).toBe(items()[1]);
      press("ArrowDown");
      expect(document.activeElement).toBe(items()[0]);
      press("ArrowUp");
      expect(document.activeElement).toBe(items()[1]);
      press("Home");
      expect(document.activeElement).toBe(items()[0]);
      press("Escape");
      expect(container.querySelector('[role="menu"]')).toBeNull();
      expect(document.activeElement).toBe(neu);
    });

    it("closes on Escape and on a click elsewhere", () => {
      render(props());
      act(() => button("+ Neu")!.click());
      act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(container.querySelector('[role="menu"]')).toBeNull();
      act(() => button("+ Neu")!.click());
      act(() => void document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
      expect(container.querySelector('[role="menu"]')).toBeNull();
    });
  });

  describe("Begriffe", () => {
    it("lists every term with its technical name and explanation, and closes on Escape", () => {
      render(props());
      expect(container.querySelector('[role="dialog"]')).toBeNull();
      act(() => button("Begriffe")!.click());
      // The box is rendered into the page body: the dock's panels would cut it off.
      const box = document.querySelector('[role="dialog"][aria-label="Begriffe"]');
      expect(container.contains(box)).toBe(false);
      const text = box?.textContent ?? "";
      for (const term of Object.values(TERMS)) {
        expect(text).toContain(term.label);
        expect(text).toContain(term.short);
        if (term.tech) expect(text).toContain(term.tech);
      }
      // A press inside the box keeps it open; Escape closes it.
      act(() => void box?.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
      expect(document.querySelector('[role="dialog"][aria-label="Begriffe"]')).not.toBeNull();
      act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(document.querySelector('[role="dialog"][aria-label="Begriffe"]')).toBeNull();
    });
  });

  describe("tree / list switch", () => {
    it("shows the old header list in the list mode and remembers the mode", () => {
      render(props({ selected: 1 }));
      expect(container.querySelector('input[aria-label="Header suchen"]')).toBeNull();
      act(() => button("Liste")!.click());
      expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
      expect(container.querySelector(SEARCH)).toBeNull();
      expect(places()).toHaveLength(0);
      expect(container.querySelectorAll("li")).toHaveLength(5);
      expect(localStorage.getItem("g4.explorer.mode")).toBe("list");
      // The menu stays in both modes.
      expect(button("+ Neu")).toBeDefined();
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
    // Selection 0 -> 3 renders the two outside rows whose mark changes, not the five of the tree.
    render({ ...base, selected: 3 });
    expect(reads - first).toBeGreaterThan(0);
    expect(reads - first).toBeLessThan(first);
  });

  describe("scripts", () => {
    const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
    const scriptsRoot = () => container.querySelector<HTMLDetailsElement>("details[data-scripts-root]")!;
    const file = (n: number, kind: ScriptFileDto["kind"] = "script", headers: number[] = []): ScriptFileDto => ({ file: n, kind, size: 120, headers });
    const hit = (n: number, line: number, excerpt: string): ScriptHitDto => ({ file: n, line, excerpt });
    const withScripts = headers.map((h) => (h.id === 0 ? { ...h, script_file_id: 0, level_script_id: 1 } : { ...h, script_file_id: 5, level_script_id: 0 }));
    const toggleHeader = (id: number) => click(container.querySelector(`[data-header-id="${id}"]`)!.previousElementSibling);

    it("shows_the_script_file_and_level_script_below_an_expanded_header_and_opens_them", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, selected: 0, onOpenScript }));
      expect(container.querySelector("[data-script-file]")).toBeNull();
      toggleHeader(0);
      const children = [...row(0)!.closest("li")!.querySelectorAll<HTMLButtonElement>("[data-script-file]")];
      expect(children.map((b) => b.textContent)).toEqual(["Scriptdatei 0000", "Level-Script 0001"]);
      click(children[0]);
      click(children[1]);
      expect(onOpenScript.mock.calls).toEqual([[0], [1]]);
      // A header without a level script has only its script file; collapsing hides the children again.
      toggleHeader(3);
      expect([...row(3)!.closest("li")!.querySelectorAll("[data-script-file]")].map((b) => b.textContent)).toEqual(["Scriptdatei 0005"]);
      toggleHeader(0);
      expect(row(0)!.closest("li")!.querySelector("[data-script-file]")).toBeNull();
    });

    it("disables_the_header_children_and_asks_for_the_adoption_while_scripts_are_not_adopted", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, selected: 0, scriptsAdopted: false, onOpenScript }));
      toggleHeader(0);
      const child = container.querySelector<HTMLButtonElement>("[data-script-file]")!;
      expect(child.disabled).toBe(true);
      expect(child.title).toBe("Erst Scripts übernehmen (Panel „Script“)");
      click(child);
      expect(onOpenScript).not.toHaveBeenCalled();
    });

    it("reads_the_script_list_when_the_root_node_is_first_opened_not_before", async () => {
      vi.mocked(listScriptFiles).mockResolvedValue([file(0, "script", [0, 4]), file(1, "levelScript", [0]), file(2, "binary")]);
      const onOpenScript = vi.fn();
      render(props({ onOpenScript }));
      expect(container.querySelector("summary")?.textContent).toBe("Scripts");
      await flush();
      expect(listScriptFiles).not.toHaveBeenCalled();
      toggle(scriptsRoot(), true);
      await flush();
      expect(listScriptFiles).toHaveBeenCalledTimes(1);
      const rows = [...scriptsRoot().querySelectorAll<HTMLButtonElement>("[data-script-file]")];
      expect(rows.map((r) => r.textContent)).toEqual([
        "Scriptdatei 0000120 Byte · Header 0000, 0004",
        "Level-Script 0001120 Byte · Header 0000",
        "Scriptdatei 0002binär · 120 Byte · ohne Header",
      ]);
      expect(scriptsRoot().querySelector("summary")?.textContent).toBe("Scripts3");
      click(rows[1]);
      expect(onOpenScript).toHaveBeenCalledWith(1);
    });

    it("says_to_adopt_first_and_calls_no_command_while_scripts_are_not_adopted", async () => {
      render(props({ scriptsAdopted: false }));
      toggle(scriptsRoot(), true);
      await flush();
      expect(scriptsRoot().textContent).toContain("Erst Scripts übernehmen (Panel „Script“)");
      expect(scriptsRoot().querySelector("input")).toBeNull();
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
        toggle(scriptsRoot(), true);
        const box = scriptsRoot().querySelector<HTMLInputElement>('input[aria-label="Scripts durchsuchen"]')!;
        type(box, "Ma");
        await act(async () => void vi.advanceTimersByTime(200));
        type(box, "Main");
        await act(async () => void vi.advanceTimersByTime(299));
        expect(searchScripts).not.toHaveBeenCalled();
        await act(async () => void vi.advanceTimersByTime(1));
        // Only the last text was searched, once.
        expect(searchScripts).toHaveBeenCalledTimes(1);
        expect(searchScripts).toHaveBeenCalledWith("Main", 100);
        const hits = [...scriptsRoot().querySelectorAll<HTMLButtonElement>("li button")];
        expect(hits.map((h) => h.textContent)).toEqual(["Scriptdatei 0042 · Zeile 7 · script Main #1", "Scriptdatei 0043 · Zeile 2 · Message 3"]);
        click(hits[0]);
        expect(onOpenScript).toHaveBeenCalledWith(42, 7);
        // Clearing the search shows the list again.
        type(box, "");
        expect(scriptsRoot().querySelector("[data-script-file]")?.textContent).toContain("Scriptdatei 0000");
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
        toggle(scriptsRoot(), true);
        type(scriptsRoot().querySelector("input")!, "xyz");
        expect(scriptsRoot().textContent).toContain("Suche …");
        await act(async () => void vi.advanceTimersByTime(300));
        expect(scriptsRoot().textContent).toContain("Keine Treffer.");
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
      toggle(scriptsRoot(), true);
      render(props({ info: b }));
      await flush();
      expect(scriptsRoot().textContent).toContain("Scriptdatei 0009");
      await act(async () => late([file(1)]));
      expect(scriptsRoot().textContent).toContain("Scriptdatei 0009");
      expect(scriptsRoot().textContent).not.toContain("Scriptdatei 0001");
    });
  });
});
