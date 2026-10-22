// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixSummary } from "../api";
import { placeColor } from "./places";
import { TERMS } from "./terms";
import { WorldExplorer } from "./WorldExplorer";
import type { WorldExplorerProps } from "./WorldExplorer";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, location_name: string, matrix_id: number, internal_name = `H${id}`): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
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
    ...over,
  });
  const render = (p: WorldExplorerProps) => act(() => root.render(<WorldExplorer {...p} />));
  const places = () => [...container.querySelectorAll<HTMLDetailsElement>("details")];
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
      const text = container.querySelector('[role="dialog"]')?.textContent ?? "";
      for (const term of Object.values(TERMS)) {
        expect(text).toContain(term.label);
        expect(text).toContain(term.short);
        if (term.tech) expect(text).toContain(term.tech);
      }
      act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(container.querySelector('[role="dialog"]')).toBeNull();
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
});
