// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getMatrix, worldMetaGet, worldMetaSet } from "../api";
import type { HeaderSummary, MatrixDto, MatrixSummary, ProjectInfo, WarpLink, WorldMeta } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { placeColor } from "./places";
import { useWorldActions } from "./RoomActions";
import { useWorldModel } from "./useWorldModel";
import { buildWorld } from "./worldModel";
import { WorldTree } from "./WorldTree";
import type { WorldTreeProps } from "./WorldTree";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  getMatrix: vi.fn(),
  worldMetaGet: vi.fn(),
  worldMetaSet: vi.fn(),
  listScriptFiles: vi.fn(),
  searchScripts: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string, place: string, index: number, matrix_id: number, map_type = 0): HeaderSummary => ({
  id,
  internal_name,
  location_name: place,
  location_index: index,
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

// Sinnoh (matrix 0): Zweiblattdorf (1) with the house 3 (warp from 1), Route 201 (2) with the mine level 4 of the place
// Bergwerk (warp to 2). Kampfzone (5) has a map of its own, the Villa (6) no entrance; 0 (landscape) and 7 (unknown
// matrix) are other.
const headers = [
  header(0, "EVERYWHERE", "Mysteriöser Ort", 0, 0),
  header(1, "T01", "Zweiblattdorf", 2, 0, 1),
  header(2, "R201", "Route 201", 3, 0, 2),
  header(3, "T01R0101", "Zweiblattdorf", 2, 20),
  header(4, "D01R0101", "Bergwerk", 9, 21),
  header(5, "D32", "Kampfzone", 60, 5, 2),
  header(6, "D99", "Villa", 40, 23),
  header(7, "LOST", "Zweiblattdorf", 2, 99),
];
const matrices = [summary(0, 3, 1, true), summary(5, 2, 2), summary(20, 1, 1), summary(21, 1, 1), summary(23, 1, 1)];
const cells: MatrixDto = { index: 0, name: "m0", width: 3, height: 1, maps: [1, 2, 3], headers: [1, 2, 0] };
const links: WarpLink[] = [
  { event_file: 1, index: 0, x: 0, z: 0, dest_header_id: 3, dest_warp_id: 0 },
  { event_file: 4, index: 0, x: 0, z: 0, dest_header_id: 2, dest_warp_id: 0 },
];
const emptyMeta = (patch: Partial<WorldMeta> = {}): WorldMeta => ({ version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {}, ...patch });
const info: ProjectInfo = { root: "/p", game_code: "CPUE", family: "Pt", language: "de", header_count: headers.length };
const world = buildWorld({ family: "Pt", headers, matrices, links, cells: new Map([[0, cells]]), meta: emptyMeta() });

describe("WorldTree", () => {
  let container: HTMLElement;
  let root: Root;
  const scrolled: string[] = [];
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    scrolled.length = 0;
    onError.mockReset();
    vi.mocked(getMatrix).mockReset().mockResolvedValue(cells);
    vi.mocked(worldMetaGet).mockReset().mockResolvedValue({ meta: emptyMeta(), broken: null });
    vi.mocked(worldMetaSet).mockReset().mockResolvedValue(undefined);
    Element.prototype.scrollIntoView = function (this: Element) {
      scrolled.push(this.getAttribute("data-header-id") ?? "?");
    };
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const props = (over: Partial<WorldTreeProps> = {}): WorldTreeProps => ({
    info,
    headers,
    world,
    matricesLoaded: true,
    matricesFailed: false,
    selected: null,
    shownMatrix: null,
    revealToken: 0,
    depth: "bauen",
    onSelect: () => {},
    scriptsAdopted: true,
    onOpenScript: () => {},
    sort: "game",
    recent: [],
    onTouch: () => {},
    onRename: () => {},
    onOwner: () => {},
    ...over,
  });
  const render = (over: Partial<WorldTreeProps> = {}) => act(() => root.render(<WorldTree {...props(over)} />));
  const row = (id: number) => container.querySelector<HTMLElement>(`[data-header-id="${id}"][role="treeitem"]:not([data-area-key])`) ?? container.querySelector<HTMLElement>(`[data-header-id="${id}"]`);
  const area = (key: string) => container.querySelector<HTMLElement>(`[data-area-key="${key}"]`);
  const mapRow = (key: string) => container.querySelector<HTMLElement>(`[data-world-map="${key}"]`);
  const group = (key: string) => container.querySelector<HTMLElement>(`[data-world-group="${key}"]`);
  const rootRows = () => [...container.querySelectorAll<HTMLElement>('[role="treeitem"][aria-level="1"]')].map((r) => r.textContent);
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  const caret = (el: HTMLElement) => click(el.querySelector("span[aria-hidden]"));
  const search = () => container.querySelector<HTMLInputElement>('input[aria-label="Welt durchsuchen"]')!;
  const type = (input: HTMLInputElement, value: string) =>
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  // A click alone: the trigger opens on it. (A mousedown would queue a second opening for the next frame, which the
  // awaited writes below would run.)
  const press = (el: HTMLElement) => act(() => el.click());
  const menuItem = (label: string) => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => e.textContent === label);
  const dialogButton = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('[role="dialog"] button')].find((b) => b.textContent === label)!;
  const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

  it("shows_world_maps_areas_and_rooms_as_a_tree", () => {
    render({ selected: 3 });
    expect(container.querySelector('[role="tree"]')?.getAttribute("aria-label")).toBe("Orte");
    expect(mapRow("w0")?.getAttribute("aria-level")).toBe("1");
    expect(mapRow("w0")?.textContent).toBe("Sinnoh2");
    expect(mapRow("w0")?.getAttribute("aria-label")).toBe("Sinnoh, 2 Gebiete");
    const town = area("w0:2")!;
    expect(town.getAttribute("role")).toBe("treeitem");
    expect(town.getAttribute("aria-level")).toBe("2");
    expect(town.getAttribute("data-place-key")).toBe("Zweiblattdorf");
    expect(town.getAttribute("data-header-id")).toBe("1");
    expect(town.getAttribute("aria-label")).toBe("Zweiblattdorf, 1 Raum");
    expect(town.getAttribute("aria-expanded")).toBe("true");
    expect(area("w0:3")?.getAttribute("aria-expanded")).toBe("false");
    const house = row(3)!;
    expect(house.getAttribute("aria-level")).toBe("3");
    expect(house.hasAttribute("data-room")).toBe(true);
    expect(house.textContent).toBe("Haus 1");
    expect(house.getAttribute("aria-selected")).toBe("true");
    // Building speaks of the game: no ids, no technical words.
    expect(container.textContent).not.toMatch(/Header|Matrix|Raster|0003/);
    expect([...container.querySelectorAll("[title]")].map((e) => e.getAttribute("title")).join(" ")).not.toMatch(/Header|Matrix|Raster/);
  });

  it("marks_a_room_of_another_place_with_its_place_label", () => {
    render({ selected: 4 });
    const label = row(4)!.querySelector<HTMLElement>("[data-place-label]")!;
    expect(label.textContent).toBe("Bergwerk");
    const dot = label.querySelector<HTMLElement>("[data-place-dot]")!;
    const expected = document.createElement("i");
    expected.style.background = placeColor("Bergwerk");
    expect(dot.style.background).toBe(expected.style.background);
    // Not by colour alone: the name is in the row's accessible name too.
    expect(row(4)!.getAttribute("aria-label")).toBe("Ebene 1, Bergwerk");
    render({ selected: 3 });
    expect(row(3)!.querySelector("[data-place-label]")).toBeNull();
  });

  it("opens_the_branch_of_the_selected_header_and_scrolls_to_it", () => {
    render();
    // The world maps start open, their areas closed.
    expect(mapRow("w0")?.getAttribute("aria-expanded")).toBe("true");
    expect(area("w0:3")?.getAttribute("aria-expanded")).toBe("false");
    expect(row(4)).toBeNull();
    render({ selected: 4 });
    expect(area("w0:3")?.getAttribute("aria-expanded")).toBe("true");
    expect(scrolled).toEqual(["4"]);
    // A closed world map opens too, and a new reveal token scrolls again.
    caret(mapRow("w0")!);
    expect(area("w0:3")).toBeNull();
    render({ selected: 4, revealToken: 1 });
    expect(mapRow("w0")?.getAttribute("aria-expanded")).toBe("true");
    expect(scrolled).toEqual(["4", "4"]);
    // The area of the selected header is selected itself.
    render({ selected: 5, revealToken: 2 });
    expect(group("more")?.getAttribute("aria-expanded")).toBe("true");
    expect(area("m5")?.getAttribute("aria-selected")).toBe("true");
  });

  it("makes_an_area_without_rooms_a_leaf", () => {
    render();
    click(group("more"));
    // The Kampfzone has no rooms: no caret, no open state; an area with rooms keeps both.
    const leaf = area("m5")!;
    expect(leaf.hasAttribute("aria-expanded")).toBe(false);
    expect(leaf.querySelector("span[aria-hidden]")!.childElementCount).toBe(0);
    expect(area("w0:2")!.hasAttribute("aria-expanded")).toBe(true);
    // Tüfteln lists sections as children only for an area of several headers; this one has one.
    render({ depth: "tuefteln" });
    expect(area("m5")!.hasAttribute("aria-expanded")).toBe(false);
  });

  it("shows_more_areas_and_unconnected_after_the_world_maps", () => {
    render();
    expect(rootRows()).toEqual(["Scripts", "Sinnoh2", "Weitere Gebiete1", "Nicht verbunden1"]);
    click(group("more"));
    expect(area("m5")?.getAttribute("aria-level")).toBe("2");
    expect(area("m5")?.textContent).toBe("Kampfzone0");
    click(group("unconnected"));
    expect(row(6)?.getAttribute("aria-level")).toBe("2");
    // Named by its place already: no place label besides.
    expect(row(6)?.textContent).toBe("Villa");
    expect(row(6)?.querySelector("[data-place-label]")).toBeNull();
  });

  it("shows_other_only_in_tinkering_with_ids_and_codes", () => {
    render({ depth: "tuefteln", selected: 3 });
    expect(rootRows()).toEqual(["Scripts", "SinnohMatrix 00002", "Weitere Gebiete1", "Nicht verbunden1", "Sonstiges2"]);
    expect(row(3)?.textContent).toBe("Haus 10003 · T01R0101");
    expect(area("w0:2")?.textContent).toBe("Zweiblattdorf0001 · T011");
    expect(mapRow("w0")?.textContent).toBe("SinnohMatrix 00002");
    click(group("other"));
    expect(row(7)?.textContent).toBe("Zweiblattdorf0007 · LOST");
    expect(row(0)?.textContent).toBe("Mysteriöser Ort0000 · EVERYWHERE");
    render({ depth: "bauen", selected: 3 });
    expect(group("other")).toBeNull();
  });

  it("filters_by_name_place_code_and_number", () => {
    render();
    type(search(), "Berg");
    expect(area("w0:2")).toBeNull();
    expect(area("w0:3")?.getAttribute("aria-expanded")).toBe("true");
    expect(row(4)?.textContent).toContain("Ebene 1");
    type(search(), "haus");
    expect(row(3)).not.toBeNull();
    expect(row(4)).toBeNull();
    type(search(), "t01r0101");
    expect(row(3)).not.toBeNull();
    type(search(), "0005");
    expect(rootRows()).toEqual(["Scripts", "Weitere Gebiete1"]);
    expect(area("m5")).not.toBeNull();
    // A world map's name keeps it whole.
    type(search(), "Sinn");
    expect(area("w0:2")).not.toBeNull();
    expect(area("w0:3")).not.toBeNull();
    type(search(), "xyz");
    expect(container.textContent).toContain("Kein Ort passt zur Suche.");
  });

  it("selects_the_first_header_of_an_area_and_touches_it_as_recent", () => {
    const onSelect = vi.fn();
    const onTouch = vi.fn();
    render({ onSelect, onTouch });
    click(area("w0:3"));
    expect(onSelect).toHaveBeenLastCalledWith(headers[2]);
    expect(onTouch).toHaveBeenLastCalledWith("w0:3");
    // The caret opens the area without selecting it.
    caret(area("w0:2")!);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(area("w0:2")?.getAttribute("aria-expanded")).toBe("true");
    click(row(3));
    expect(onSelect).toHaveBeenLastCalledWith(headers[3]);
    expect(onTouch).toHaveBeenLastCalledWith("w0:2");
    // A world map only opens and closes.
    click(mapRow("w0"));
    expect(onSelect).toHaveBeenCalledTimes(2);
    expect(mapRow("w0")?.getAttribute("aria-expanded")).toBe("false");
    pressKey(mapRow("w0")!, "ArrowRight");
    expect(mapRow("w0")?.getAttribute("aria-expanded")).toBe("true");
  });

  it("sorts_the_areas_by_name_and_by_the_recent_choice", () => {
    const names = () => [...container.querySelectorAll<HTMLElement>("[data-area-key]")].map((a) => a.getAttribute("data-area-key"));
    render();
    expect(names()).toEqual(["w0:2", "w0:3"]);
    render({ sort: "name" });
    expect(names()).toEqual(["w0:3", "w0:2"]);
    render({ sort: "recent", recent: ["w0:3"] });
    expect(names()).toEqual(["w0:3", "w0:2"]);
  });

  describe("with the project file", () => {
    function Harness({ selected }: { selected: number }) {
      const model = useWorldModel({ info, headers, matrices, links, loaded: true, revision: 0, onError });
      const { onRename, onOwner } = useWorldActions(model.meta, model.setMeta);
      return <WorldTree {...props({ selected, world: model.world, onRename, onOwner })} />;
    }
    const mount = async (selected: number) => {
      act(() => root.render(<Harness selected={selected} />));
      await flush();
    };
    const actions = (name: string) => container.querySelector<HTMLElement>(`[aria-label="Weitere Aktionen für ${name}"]`)!;
    const rename = async (name: string, to: string) => {
      press(actions(name));
      act(() => menuItem("Umbenennen…")!.click());
      expect(document.querySelector('[role="dialog"]')?.textContent).toContain("Raum umbenennen");
      const field = document.querySelector<HTMLInputElement>('[role="dialog"] input[aria-label="Name"]')!;
      type(field, to);
      await act(async () => dialogButton("Umbenennen").click());
      await flush();
    };

    it("renames_a_room_and_writes_the_project_file", async () => {
      await mount(3);
      // Only the selected room has the menu.
      expect(row(4)).toBeNull();
      await rename("Haus 1", "Elternhaus");
      expect(worldMetaSet).toHaveBeenLastCalledWith(emptyMeta({ roomNames: { "3": "Elternhaus" } }));
      expect(row(3)?.textContent).toBe("Elternhaus");
      expect(document.querySelector('[role="dialog"]')).toBeNull();
      // An empty name gives the suggestion back.
      await rename("Elternhaus", "");
      expect(worldMetaSet).toHaveBeenLastCalledWith(emptyMeta());
      expect(row(3)?.textContent).toBe("Haus 1");
      expect(onError).not.toHaveBeenCalled();
    });

    it("renames_a_world_map", async () => {
      await mount(3);
      press(actions("Sinnoh"));
      act(() => menuItem("Umbenennen…")!.click());
      expect(document.querySelector('[role="dialog"]')?.textContent).toContain("Weltkarte umbenennen");
      type(document.querySelector<HTMLInputElement>('[role="dialog"] input[aria-label="Name"]')!, "Sinnoh-Region");
      await act(async () => dialogButton("Umbenennen").click());
      await flush();
      expect(worldMetaSet).toHaveBeenLastCalledWith(emptyMeta({ worldMaps: [{ matrix: 0, name: "Sinnoh-Region" }] }));
      expect(mapRow("w0")?.textContent).toBe("Sinnoh-Region2");
    });

    it("assigns_a_room_to_another_area_and_resets_it", async () => {
      await mount(3);
      press(actions("Haus 1"));
      expect(menuItem("Zuordnung zurücksetzen")).toBeUndefined();
      act(() => menuItem("Gehört zu Gebiet…")!.click());
      const options = [...document.querySelectorAll<HTMLElement>('[role="dialog"] [role="option"]')];
      // Every area but the room's own, with its world map.
      expect(options.map((o) => o.textContent)).toEqual(["Route 201Sinnoh", "KampfzoneWeitere Gebiete"]);
      await act(async () => options[1].click());
      await flush();
      expect(worldMetaSet).toHaveBeenLastCalledWith(emptyMeta({ roomOwner: { "3": "m5" } }));
      // The room moved under the Kampfzone, which opens to show it.
      expect(row(3)?.getAttribute("aria-level")).toBe("3");
      expect(group("more")?.getAttribute("aria-expanded")).toBe("true");
      expect(area("w0:2")?.getAttribute("aria-label")).toBe("Zweiblattdorf, 0 Räume");
      press(actions("Haus 1"));
      await act(async () => menuItem("Zuordnung zurücksetzen")!.click());
      await flush();
      expect(worldMetaSet).toHaveBeenLastCalledWith(emptyMeta());
      expect(area("w0:2")?.getAttribute("aria-label")).toBe("Zweiblattdorf, 1 Raum");
    });

    it("refuses_to_rename_while_the_project_file_is_broken", async () => {
      vi.mocked(worldMetaGet).mockResolvedValue({ meta: emptyMeta(), broken: "expected value at line 1 column 1" });
      await mount(3);
      await rename("Haus 1", "Elternhaus");
      expect(worldMetaSet).not.toHaveBeenCalled();
      expect(String(onError.mock.calls[0][0])).toContain("g4-world.json ist beschädigt: expected value at line 1 column 1. Repariere oder lösche die Datei.");
      expect(row(3)?.textContent).toBe("Haus 1");
    });
  });
});
