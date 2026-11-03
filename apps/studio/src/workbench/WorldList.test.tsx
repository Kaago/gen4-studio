// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, WarpLink } from "../api";
import { ShellProvider } from "../ui/frame/ShellContext";
import { pressKey, typeInSearch } from "../ui/primitives/testUtils";
import { visibleWords } from "../ui/visibleText";
import { deleteSpy, withDelete } from "../delete/deleteTesting";
import { chooseContext, contextTexts } from "../original/resetTesting";
import { headerIdentity } from "./headerIdentity";
import { buildPlaces } from "./places";
import { buildWorld, worldNames } from "./worldModel";
import { WorldList } from "./WorldList";
import type { WorldListProps } from "./WorldList";

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
const world = buildWorld({ family: "Pt", headers, matrices, links, cells: new Map([[0, cells]]), meta: { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} } });
const names = worldNames(world);
const places = buildPlaces(headers, matrices);
const identify = (h: HeaderSummary) => headerIdentity(places, h, names);
const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "en", header_count: headers.length };

/** The height the list's column measures (jsdom has no layout). */
const HEIGHT = 400;

describe("WorldList", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  const onToggleList = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    localStorage.clear();
    onSelect.mockReset();
    onToggleList.mockReset();
    Element.prototype.scrollIntoView = () => {};
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      return { x: 0, y: 0, top: 0, left: 0, right: 272, bottom: HEIGHT, width: 272, height: this.hasAttribute("data-world-list") ? HEIGHT : 0, toJSON: () => ({}) } as DOMRect;
    });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  const props = (over: Partial<WorldListProps> = {}): WorldListProps => ({
    info,
    headers,
    world,
    identify,
    matricesLoaded: true,
    matricesFailed: false,
    selected: 3,
    shownMatrix: 20,
    revealToken: 0,
    depth: "bauen",
    onSelect,
    onOpenWizard: () => {},
    scriptsAdopted: true,
    onOpenScript: () => {},
    listOpen: true,
    onToggleList,
    sort: { tree: "game", list: "id" },
    onSort: () => {},
    onRename: () => {},
    onOwner: () => {},
    scene: {
      header: headers[3]!,
      headers,
      matrix: null,
      events: null,
      buildings: [],
      depth: "bauen",
      identify,
      selected: null,
      locked: null,
      loading: false,
      onSelect: () => {},
      onFollow: () => {},
      onOpenScript: () => {},
      onTool: () => {},
      toolsBlocked: null,
      onConnectRoom: () => {},
      onNewPlace: () => {},
      onBack: () => {},
      onForward: () => {},
      canBack: false,
      canForward: false,
    },
    ...over,
  });
  const show = (over: Partial<WorldListProps> = {}) =>
    act(() =>
      root.render(
        <ShellProvider>
          <WorldList {...props(over)} />
        </ShellProvider>,
      ),
    );
  const section = (label: string) => container.querySelector<HTMLElement>(`section[aria-label="${label}"]`);
  const splitter = () => container.querySelector<HTMLElement>('[role="separator"][data-splitter="world-list"]');
  const button = (label: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.getAttribute("aria-label") === label || b.textContent === label);
  const placesTree = () => container.querySelector<HTMLElement>('[role="tree"][aria-label="Orte"]')!;
  const stored = () => JSON.parse(localStorage.getItem("g4.shell.v1") ?? "{}") as { worldListSplit?: number; placesFolded?: boolean };
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const foldedRow = () => container.querySelector<HTMLButtonElement>("button[data-places-row]");

  it("has_two_sections_Orte_and_In_dieser_Szene_with_a_splitter", () => {
    show();
    // The list is called „Welt“ and names the game; its grip folds it.
    expect(section("Welt")).not.toBeNull();
    expect(section("Welt")!.querySelector("h2")?.textContent).toBe("Welt");
    expect(section("Welt")!.textContent).toContain("Platin, EN");
    act(() => button("Liste einklappen")!.click());
    expect(onToggleList).toHaveBeenCalledOnce();
    // Above the places, below the scene, the splitter between them.
    const places = section("Orte")!;
    const scene = section("In dieser Szene")!;
    const follows = (a: Element, b: Element) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    expect(follows(places, splitter()!)).toBe(true);
    expect(follows(splitter()!, scene)).toBe(true);
    expect([...section("Welt")!.querySelectorAll("h2")].map((h) => h.textContent)).toEqual(["Welt", "Orte", "In dieser Szene"]);
    expect(scene.querySelector('[role="tree"][aria-label="In dieser Szene"]')).not.toBeNull();
    expect(placesTree()).not.toBeNull();
    // Half and half by default, each part at least 96 px; the splitter sizes the places.
    expect(document.getElementById(splitter()!.getAttribute("aria-controls")!)?.contains(places)).toBe(true);
    expect(splitter()!.getAttribute("aria-label")).toBe("Höhe der Orte");
    expect(["aria-valuenow", "aria-valuemin", "aria-valuemax"].map((a) => splitter()!.getAttribute(a))).toEqual(["200", "96", "296"]);
    pressKey(splitter()!, "ArrowDown");
    expect(splitter()!.getAttribute("aria-valuenow")).toBe("216");
    expect(stored().worldListSplit).toBeCloseTo(0.54);
    pressKey(splitter()!, "Home");
    expect(splitter()!.getAttribute("aria-valuenow")).toBe("296");
    pressKey(splitter()!, "End");
    expect(splitter()!.getAttribute("aria-valuenow")).toBe("96");
    // A double click halves again.
    act(() => void splitter()!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(splitter()!.getAttribute("aria-valuenow")).toBe("200");
    expect(stored().worldListSplit).toBe(0.5);
    // Bauen names no Raster, Matrix or Header.
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("folds_the_places_to_a_row_that_opens_the_picker", () => {
    show();
    expect(foldedRow()).toBeNull();
    act(() => button("Orte einklappen")!.click());
    // The tree stays mounted (its search and open branches survive) but hidden; the scene gets the whole height.
    expect(placesTree().closest("[hidden]")).not.toBeNull();
    expect(splitter()).toBeNull();
    expect(button("Orte ausklappen")).toBeDefined();
    // One row names the shown place: the place and the room, with its symbol and an arrow.
    expect(foldedRow()!.textContent).toBe("Zweiblattdorf › Haus 1");
    expect(foldedRow()!.querySelectorAll("svg")).toHaveLength(2);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    act(() => foldedRow()!.click());
    expect(dialog()?.textContent).toContain("Ort oder Raum wählen");
    // The areas and the rooms of the world.
    const labels = options().map((o) => o.textContent);
    expect(labels).toEqual(expect.arrayContaining([expect.stringContaining("Zweiblattdorf"), expect.stringContaining("Haus 1"), expect.stringContaining("Haus 2"), expect.stringContaining("Route 201"), expect.stringContaining("Kampfzone")]));
    // An area alone names itself.
    act(() => button("Orte ausklappen")!.click());
    expect(placesTree().closest("[hidden]")).toBeNull();
    expect(splitter()).not.toBeNull();
    act(() => button("Orte einklappen")!.click());
    show({ selected: 1 });
    expect(foldedRow()!.textContent).toBe("Zweiblattdorf");
    show({ selected: null });
    expect(foldedRow()!.textContent).toBe("Ort oder Raum wählen");
  });

  it("picks_a_room_in_the_folded_row", () => {
    show();
    act(() => button("Orte einklappen")!.click());
    act(() => foldedRow()!.click());
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Haus 2");
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining("Haus 2")]);
    pressKey(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Enter");
    expect(onSelect).toHaveBeenCalledWith(headers[4]);
    // A number finds its header too, as in the tree.
    act(() => foldedRow()!.click());
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "0005");
    act(() => options()[0]!.click());
    expect(onSelect).toHaveBeenLastCalledWith(headers[5]);
  });

  it("touches_the_area_of_a_pick_in_the_folded_row_like_a_click_in_the_tree", () => {
    show();
    act(() => button("Orte einklappen")!.click());
    act(() => foldedRow()!.click());
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Haus 2");
    pressKey(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Enter");
    // The room's area is the one chosen last (the sort „Zuletzt gewählt“ reads it).
    const area = world.areaOf.get(4)!;
    expect(JSON.parse(localStorage.getItem("g4.world.recent./p")!)).toEqual([area]);
    // An area on a map of its own is touched by its own key.
    act(() => foldedRow()!.click());
    typeInSearch(document.querySelector<HTMLInputElement>("input[role=combobox]")!, "Kampfzone");
    act(() => options()[0]!.click());
    expect(JSON.parse(localStorage.getItem("g4.world.recent./p")!)).toEqual([world.areaOf.get(5)!, area]);
  });

  it("remembers_the_split_and_the_fold", () => {
    show();
    pressKey(splitter()!, "ArrowUp");
    expect(stored().worldListSplit).toBeCloseTo(0.46);
    act(() => button("Orte einklappen")!.click());
    expect(stored().placesFolded).toBe(true);
    // A new run reads both.
    act(() => root.unmount());
    root = createRoot(container);
    show();
    expect(foldedRow()).not.toBeNull();
    act(() => button("Orte ausklappen")!.click());
    expect(splitter()!.getAttribute("aria-valuenow")).toBe("184");
    expect(stored().placesFolded).toBe(false);
  });

  it("hides_deleted_until_show_deleted_and_offers_restore", () => {
    // A third house of Zweiblattdorf (6) is a tombstone.
    const withGone = [...headers, { ...header(6, "T01R0301", "Zweiblattdorf", 2, 22), deleted: true }];
    const gone = buildWorld({
      family: "Pt",
      headers: withGone,
      matrices: [...matrices, summary(22, 1, 1)],
      links: [...links, { event_file: 1, index: 6, x: 0, z: 0, dest_header_id: 6, dest_warp_id: 0 }],
      cells: new Map([[0, cells]]),
      meta: { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} },
    });
    const del = deleteSpy();
    act(() =>
      root.render(
        <ShellProvider>
          {withDelete(del, <WorldList {...props({ headers: withGone, world: gone })} />)}
        </ShellProvider>,
      ),
    );
    const places = () => section("Orte")!;
    const box = () => places().querySelector<HTMLElement>("[data-show-deleted]")!;
    const row = (id: number) => places().querySelector<HTMLElement>(`[data-header-id="${id}"]`);
    // The tree: off by default, the tombstone is not there; with the box it is, with its badge and „Wiederherstellen“.
    expect(box().getAttribute("data-show-deleted")).toBe("false");
    expect(box().textContent).toBe("Gelöschte zeigen");
    expect(row(3)).not.toBeNull();
    expect(row(6)).toBeNull();
    act(() => box().querySelector<HTMLElement>("[role=checkbox]")!.click());
    expect(box().getAttribute("data-show-deleted")).toBe("true");
    expect(row(6)?.getAttribute("data-deleted")).toBe("true");
    expect(row(6)?.querySelector("[data-entity-state]")?.textContent).toBe("gelöscht");
    expect(row(3)?.hasAttribute("data-deleted")).toBe(false);
    expect(contextTexts(row(6)!)[0]).toBe("Wiederherstellen");
    expect(contextTexts(row(3)!)).not.toContain("Wiederherstellen");
    chooseContext(row(6)!, "Wiederherstellen");
    expect(del.restore).toHaveBeenCalledWith({ kind: "header", id: 6 });
    // The flat list keeps the choice; off again, the tombstone goes.
    act(() => button("Liste")!.click());
    expect(places().querySelector('[role=listbox] [data-header-id="6"]')?.getAttribute("data-deleted")).toBe("true");
    expect(contextTexts(places().querySelector<HTMLElement>('[role=listbox] [data-header-id="6"]')!)[0]).toBe("Wiederherstellen");
    act(() => box().querySelector<HTMLElement>("[role=checkbox]")!.click());
    expect(places().querySelector('[role=listbox] [data-header-id="6"]')).toBeNull();
    expect(places().querySelector('[role=listbox] [data-header-id="3"]')).not.toBeNull();
  });
});
