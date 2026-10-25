// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, ScriptFileDto, ScriptHitDto, WarpLink } from "../api";
import { ShellProvider, useShell } from "../ui/frame/ShellContext";
import { pressKey } from "../ui/primitives/testUtils";
import { buildWorld } from "./worldModel";
import { WorldExplorer } from "./WorldExplorer";
import type { WorldExplorerProps } from "./WorldExplorer";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listScriptFiles: vi.fn(), searchScripts: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string, location_name: string, location_index: number, matrix_id: number, map_type = 0): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  location_index,
  kanto: false,
  map_type,
  matrix_id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});
const summary = (index: number, width: number, height: number, has_headers = false): MatrixSummary => ({ index, name: `m${index}`, width, height, has_headers, has_altitudes: false, filled: width * height });

// Sinnoh: Zweiblattdorf (1) with the houses 3 and 4, Route 201 (2); the Kampfzone (5) has a map of its own; 0 is the landscape.
const headers = [
  header(0, "EVERYWHERE", "", 0, 0),
  header(1, "T01", "Zweiblattdorf", 2, 0, 1),
  header(2, "R201", "Route 201", 3, 0, 2),
  header(3, "T01R0101", "Zweiblattdorf", 2, 20),
  header(4, "T01R0201", "Zweiblattdorf", 2, 21),
  header(5, "D32", "Kampfzone", 60, 5, 2),
];
const matrices = [summary(0, 3, 1, true), summary(5, 2, 2), summary(20, 1, 1), summary(21, 1, 1)];
const cells: MatrixDto = { index: 0, name: "m0", width: 3, height: 1, maps: [1, 2, 3], headers: [1, 2, 0] };
const links: WarpLink[] = [3, 4].map((dest) => ({ event_file: 1, index: dest, x: 0, z: 0, dest_header_id: dest, dest_warp_id: 0 }));
const worldOf = (list: HeaderSummary[] = headers) =>
  buildWorld({ family: "Pt", headers: list, matrices, links, cells: new Map([[0, cells]]), meta: { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} } });
const world = worldOf();
const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: headers.length };

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
    info,
    headers,
    world,
    matricesLoaded: true,
    matricesFailed: false,
    selected: null,
    shownMatrix: null,
    revealToken: 0,
    depth: "tuefteln",
    onSelect: () => {},
    onOpenWizard: () => {},
    scriptsAdopted: true,
    onOpenScript: () => {},
    listOpen: true,
    onToggleList: () => {},
    sort: { tree: "game", list: "id" },
    onSort: () => {},
    onRename: () => {},
    onOwner: () => {},
    ...over,
  });
  const render = (p: WorldExplorerProps) => act(() => root.render(<WorldExplorer {...p} />));
  const treeitems = () => [...container.querySelectorAll<HTMLElement>('[role="treeitem"]')];
  const areas = () => [...container.querySelectorAll<HTMLElement>('[role="treeitem"][data-area-key]')];
  const expanded = () => areas().map((p) => p.getAttribute("aria-expanded"));
  const row = (id: number) => container.querySelector<HTMLElement>(`[data-header-id="${id}"]:not([data-area-key])`) ?? container.querySelector<HTMLElement>(`[data-header-id="${id}"]`);
  const button = (name: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name || b.getAttribute("aria-label") === name);
  const search = () => container.querySelector<HTMLInputElement>(SEARCH)!;
  const type = (input: HTMLInputElement, value: string) =>
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  const caret = (el: HTMLElement) => click(el.querySelector("span[aria-hidden]"));
  const menuItems = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemradio"]')];

  it("counts_world_maps_areas_and_rooms", () => {
    render(props({ depth: "bauen" }));
    expect(container.textContent).toContain("1 Weltkarte, 3 Gebiete, 2 Räume");
    expect(container.textContent).not.toContain("Header");
    render(props({ depth: "tuefteln" }));
    expect(container.textContent).toContain("1 Weltkarte, 3 Gebiete, 2 Räume, 6 Header");
  });

  it("says_it_loads_until_the_world_is_read_and_lists_the_headers_meanwhile", () => {
    render(props({ world: null, matricesLoaded: false, selected: 3 }));
    expect(container.textContent).toContain("Lade Übersicht …");
    expect(row(3)?.textContent).toBe("Zweiblattdorf0003 · T01R0101");
    expect(scrolled).toEqual(["3"]);
    // The world arrives: the room opens in its area and is scrolled to again.
    render(props({ selected: 3 }));
    expect(container.textContent).not.toContain("Lade Übersicht");
    expect(row(3)?.textContent).toBe("Haus 10003 · T01R0101");
    expect(scrolled).toEqual(["3", "3"]);
  });

  it("says the overview is not available when its first listing failed, instead of loading for ever", () => {
    render(props({ world: null, matricesLoaded: false, matricesFailed: true }));
    expect(container.textContent).toContain("Übersicht nicht verfügbar");
    expect(container.textContent).not.toContain("Lade Übersicht");
  });

  it("switches_the_sort_in_the_head_menu_and_remembers_it", () => {
    function Pane() {
      const { state, setWorldSort } = useShell();
      return <WorldExplorer {...props({ sort: state.worldSort, onSort: setWorldSort })} />;
    }
    act(() =>
      root.render(
        <ShellProvider>
          <Pane />
        </ShellProvider>,
      ),
    );
    const order = () => areas().map((a) => a.getAttribute("data-area-key"));
    expect(order()).toEqual(["w0:2", "w0:3"]);
    click(button("Sortierung"));
    expect(menuItems().map((e) => [e.textContent, e.getAttribute("aria-checked")])).toEqual([
      ["Spielreihenfolge", "true"],
      ["Name A–Z", "false"],
      ["Zuletzt gewählt", "false"],
    ]);
    act(() => menuItems()[1].click());
    expect(order()).toEqual(["w0:3", "w0:2"]);
    expect(JSON.parse(localStorage.getItem("g4.shell.v1")!).worldSort).toEqual({ tree: "name", list: "id" });
    // The recent choice: the area clicked last comes first.
    click(button("Sortierung"));
    act(() => menuItems()[2].click());
    click(areas()[1]);
    expect(order()).toEqual(["w0:3", "w0:2"]);
    expect(JSON.parse(localStorage.getItem("g4.world.recent./p")!)).toEqual(["w0:3"]);
    // The list mode has its own sort.
    click(button("Liste"));
    click(button("Sortierung"));
    expect(menuItems().map((e) => e.textContent)).toEqual(["Nummer", "Name", "Kürzel"]);
    act(() => menuItems()[2].click());
    expect([...container.querySelectorAll('[role="option"]')].map((o) => o.getAttribute("data-header-id"))).toEqual(["5", "0", "2", "1", "3", "4"]);
    expect(JSON.parse(localStorage.getItem("g4.shell.v1")!).worldSort).toEqual({ tree: "recent", list: "code" });
  });

  it("marks a header that a check found an error or warning at, in the tree and in the list", () => {
    render(props({ selected: 3, marks: new Map([[3, "error"], [4, "warning"]]) }));
    expect(row(3)?.querySelector("[data-finding-mark]")?.getAttribute("data-finding-mark")).toBe("error");
    expect(row(4)?.querySelector("[data-finding-mark]")?.getAttribute("aria-label")).toBe("Warnung gefunden");
    // The area carries the worst mark of its sections, not of its rooms.
    expect(row(1)?.querySelector("[data-finding-mark]")).toBeNull();
    click(button("Liste"));
    expect(row(3)?.querySelector("[data-finding-mark]")?.getAttribute("data-finding-mark")).toBe("error");
  });

  it("marks the selected row, and the rows of the shown matrix with a text of their own", () => {
    render(props({ selected: 3, shownMatrix: 20 }));
    expect(row(3)?.getAttribute("aria-selected")).toBe("true");
    expect(row(4)?.getAttribute("aria-selected")).toBe("false");
    // Not by colour alone: the marked rows carry a name for assistive technology.
    expect(row(3)?.querySelector('[aria-label="Raster wird angezeigt"]')).not.toBeNull();
    expect(row(4)?.querySelector('[aria-label="Raster wird angezeigt"]')).toBeNull();
  });

  it("calls onSelect with the header of the clicked row, and with Enter on a focused row", () => {
    const onSelect = vi.fn();
    render(props({ selected: 3, onSelect }));
    click(row(4));
    expect(onSelect).toHaveBeenLastCalledWith(headers[4]);
    pressKey(row(3)!, "Enter");
    expect(onSelect).toHaveBeenLastCalledWith(headers[3]);
  });

  it("filters while typing, not only when the field is left", () => {
    render(props());
    act(() => search().focus());
    type(search(), "Rou");
    expect(areas()).toHaveLength(1);
    expect(document.activeElement).toBe(search());
  });

  it("clears a search that hides a newly selected header, then scrolls to it", () => {
    render(props());
    type(search(), "Route");
    expect(areas()).toHaveLength(1);
    render(props({ selected: 4 }));
    expect(search().value).toBe("");
    expect(expanded()).toEqual(["true", "false"]);
    expect(scrolled).toEqual(["4"]);
  });

  it("does not pull a selection back that a later search hides", () => {
    render(props({ selected: 3 }));
    expect(scrolled).toEqual(["3"]);
    type(search(), "Route");
    expect(areas()).toHaveLength(1);
    expect(search().value).toBe("Route");
    expect(scrolled).toEqual(["3"]);
  });

  it("opens an area a link reveals although the user closed it, and keeps an area the user opened", () => {
    render(props({ selected: 3 }));
    caret(areas()[0]);
    expect(expanded()[0]).toBe("false");
    render(props({ selected: 3, revealToken: 1 }));
    expect(expanded()[0]).toBe("true");
    expect(scrolled).toEqual(["3", "3"]);
    caret(areas()[1]);
    render(props({ selected: 4 }));
    expect(expanded()).toEqual(["true", "true"]);
  });

  describe("the head", () => {
    it("is titled Orte, names the game and folds the list through the grip", () => {
      const onToggleList = vi.fn();
      render(props({ onToggleList }));
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
      render(props({ selected: 3 }));
      type(search(), "Zwei");
      render(props({ selected: 3, listOpen: false }));
      expect(container.querySelector("section")?.hidden).toBe(true);
      render(props({ selected: 3, listOpen: true }));
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
      expect(areas()).toHaveLength(0);
      expect(container.querySelectorAll('[role="option"]')).toHaveLength(6);
      expect(localStorage.getItem("g4.explorer.mode")).toBe("list");
      act(() => root.unmount());
      root = createRoot(container);
      render(props());
      expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
      act(() => button("Baum")!.click());
      expect(areas()).toHaveLength(2);
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
        act(() => button("Baum")!.click());
        click(areas()[0]);
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
      render(props({ selected: 3 }));
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

    it("move between the rows of the tree and of the list, and open and close a branch", () => {
      render(props({ selected: 3 }));
      const first = treeitems()[0];
      pressKey(first, "ArrowDown");
      expect(document.activeElement).toBe(treeitems()[1]);
      pressKey(areas()[1], "ArrowRight");
      expect(expanded()).toEqual(["true", "true"]);
      pressKey(areas()[0], "ArrowLeft");
      expect(expanded()).toEqual(["false", "true"]);
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
    const countedWorld = worldOf(counted);
    const base = props({ headers: counted, world: countedWorld, selected: 3 });
    render(base);
    const first = reads;
    expect(first).toBeGreaterThan(0);
    render({ ...base });
    expect(reads).toBe(first);
    // Selection 3 -> 4 renders the two rooms whose selection changed, not every row.
    render({ ...base, selected: 4 });
    expect(reads - first).toBeGreaterThan(0);
    expect(reads - first).toBeLessThan(first);
  });

  describe("scripts", () => {
    const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
    const scriptsRoot = () => container.querySelector<HTMLElement>("[data-scripts-root]")!;
    const openScripts = () => click(scriptsRoot());
    /** The script files below the root node (level 2). */
    const fileRows = () => [...container.querySelectorAll<HTMLElement>('[data-script-file][aria-level="2"]')];
    /** The script entries of a room (level 4: world map, area, room, entry). */
    const roomChildren = () => [...container.querySelectorAll<HTMLElement>('[data-script-file][aria-level="4"]')];
    const scriptBox = () => container.querySelector<HTMLInputElement>('input[aria-label="Scripts durchsuchen"]')!;
    const file = (n: number, kind: ScriptFileDto["kind"] = "script", users: number[] = []): ScriptFileDto => ({ file: n, kind, size: 120, headers: users });
    const hit = (n: number, line: number, excerpt: string): ScriptHitDto => ({ file: n, line, excerpt });
    const hitRows = () => treeitems().filter((t) => t.textContent?.includes("Zeile"));
    const withScripts = headers.map((h) => (h.id === 3 ? { ...h, script_file_id: 7, level_script_id: 8 } : { ...h, script_file_id: 5, level_script_id: 0 }));
    const toggleHeader = (id: number) => pressKey(row(id)!, "ArrowRight");

    it("lists_the_scripts_node_first_in_the_tree", () => {
      render(props());
      expect(treeitems()[0]).toBe(scriptsRoot());
      expect(scriptsRoot().textContent).toBe("Scripts");
      expect(scriptsRoot().getAttribute("aria-expanded")).toBe("false");
    });

    it("shows_the_script_file_and_level_script_below_an_expanded_room_and_below_an_open_area", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, world: worldOf(withScripts), selected: 3, onOpenScript }));
      // The open area lists its own script file first.
      expect([...container.querySelectorAll<HTMLElement>('[data-script-file][aria-level="3"]')].map((b) => b.textContent)).toEqual(["Scriptdatei 0005"]);
      expect(roomChildren()).toHaveLength(0);
      expect(row(3)?.getAttribute("aria-expanded")).toBe("false");
      toggleHeader(3);
      expect(row(3)?.getAttribute("aria-expanded")).toBe("true");
      expect(roomChildren().map((b) => b.textContent)).toEqual(["Scriptdatei 0007", "Level-Script 0008"]);
      click(roomChildren()[0]);
      click(roomChildren()[1]);
      expect(onOpenScript.mock.calls).toEqual([[7], [8]]);
      pressKey(row(3)!, "ArrowLeft");
      expect(roomChildren()).toHaveLength(0);
    });

    it("disables_the_header_children_and_asks_for_the_adoption_while_scripts_are_not_adopted", () => {
      const onOpenScript = vi.fn();
      render(props({ headers: withScripts, world: worldOf(withScripts), selected: 3, scriptsAdopted: false, onOpenScript }));
      toggleHeader(3);
      const child = roomChildren()[0];
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
