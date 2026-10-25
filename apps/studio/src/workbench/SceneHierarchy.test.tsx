// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { EventsDto, HeaderSummary, MatrixDto } from "../api";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { visibleWords } from "../ui/visibleText";
import { SceneHierarchy } from "./SceneHierarchy";
import type { SceneHierarchyProps } from "./SceneHierarchy";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string, location_name: string, matrix_id: number): HeaderSummary => ({
  id,
  internal_name,
  location_name,
  location_index: id + 100,
  kanto: false,
  matrix_id,
  area_data_id: 0,
  event_file_id: 7,
  text_archive_id: 0,
  script_file_id: 42,
  level_script_id: 0,
});
const town = header(3, "C01", "Jubelstadt", 0);
const house = header(414, "T01R0201", "Zweiblattdorf", 200);
const matrix: MatrixDto = { index: 0, name: "Sinnoh", width: 2, height: 1, maps: [6, 7], headers: [3, 3] };
const events: EventsDto = {
  event_file_id: 7,
  locked: null,
  objects: [
    { index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, trainer_type: 0, sight: 0, x: 10, z: 20, y: 0 },
    { index: 1, local_id: 2, graphics_id: 4, movement_type: 0, script: 3, dir: 0, trainer_type: 0, sight: 0, x: 11, z: 21, y: 0 },
  ],
  warps: [
    { index: 0, x: 8, z: 9, dest_header_id: 414, dest_warp_id: 0, height: 0 },
    { index: 1, x: 9, z: 9, dest_header_id: 999, dest_warp_id: 0, height: 0 },
  ],
  coords: [{ index: 0, script: 5, x: 1, z: 2, width: 1, length: 1 }],
  bg: [{ index: 0, script: 12, event_type: 0, x: 4, z: 5, y: 0 }],
};

describe("SceneHierarchy", () => {
  let container: HTMLElement;
  let root: Root;
  const fns = {
    onSelect: vi.fn(),
    onFollow: vi.fn(),
    onOpenScript: vi.fn(),
    onTool: vi.fn(),
    onConnectRoom: vi.fn(),
    onBack: vi.fn(),
    onForward: vi.fn(),
  };
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of Object.values(fns)) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<SceneHierarchyProps> = {}) =>
    act(() =>
      root.render(
        <SceneHierarchy
          header={town}
          headers={[town, house]}
          matrix={matrix}
          events={events}
          buildings={[{ map: 6, models: [12, 13] }]}
          depth="bauen"
          worldName={(h) => (h.id === 414 ? "Haus von Lucas" : h.location_name)}
          selected={null}
          locked={null}
          loading={false}
          toolsBlocked={null}
          canBack={false}
          canForward={false}
          {...fns}
          {...over}
        />,
      ),
    );
  const tree = () => container.querySelector<HTMLElement>('[role=tree][aria-label="In dieser Szene"]')!;
  const items = () => [...container.querySelectorAll<HTMLElement>("[role=treeitem]")];
  const row = (text: string) => items().find((r) => r.textContent?.includes(text))!;
  const group = (g: string) => container.querySelector<HTMLElement>(`[data-scene-group="${g}"]`)!;
  const button = (label: string) => container.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
  const press = (el: HTMLElement) =>
    act(() => {
      el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      el.click();
    });
  const menuItems = () => [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];

  it("is_a_tree_called_In_dieser_Szene_with_groups", () => {
    show();
    expect(tree()).not.toBeNull();
    expect(container.querySelector("h2")?.textContent).toBe("In dieser Szene");
    expect([...container.querySelectorAll("[data-scene-group]")].map((g) => g.getAttribute("data-scene-group"))).toEqual(["header", "maps", "object", "warp", "coord", "bg", "scripts"]);
    expect(group("header").getAttribute("data-header-id")).toBe("3");
    expect(group("header").textContent).toContain("Jubelstadt");
    // Groups: the word and the count at the right end, no capitals.
    expect(group("object").textContent).toBe("Figuren2");
    expect(group("object").getAttribute("aria-label")).toBe("Figuren, 2");
    expect(group("object").getAttribute("aria-expanded")).toBe("true");
    expect(group("object").getAttribute("aria-level")).toBe("2");
    expect(container.querySelector('[data-event="object:1"]')?.textContent).toContain("x 11, z 21");
    expect(row("Warp 0").textContent).toContain("→ Haus von Lucas");
    expect(row("Warp 1").textContent).toContain("→ unbekanntes Ziel 0999");
    expect(container.querySelector('[data-script-file="42"]')?.textContent).toContain("Scriptdatei 0042");
    // The buildings show under their map once it is opened.
    expect(container.querySelector("[data-building]")).toBeNull();
    pressKey(row("Karte 0006"), "ArrowRight");
    expect(container.querySelector('[data-building="6:1"]')?.textContent).toContain("Gebäude 2 · Modell 13");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    show({ depth: "tuefteln" });
    expect(group("header").textContent).toContain("Header 0003 · Matrix 0000");
  });

  it("searches_by_title_and_clears_with_escape", () => {
    show();
    const field = container.querySelector<HTMLInputElement>('input[aria-label="In dieser Szene suchen"]')!;
    typeInto(field, "warp 1");
    expect(items().map((r) => r.textContent)).toEqual(["Jubelstadt", "Türen1", expect.stringContaining("Warp 1")]);
    pressKey(field, "Escape");
    expect(field.value).toBe("");
    expect(group("object")).not.toBeNull();
    typeInto(field, "zzz");
    expect(container.textContent).toContain("Nichts in dieser Szene passt zur Suche.");
  });

  it("selects_like_a_click_in_the_scene", () => {
    show();
    act(() => row("Objekt 1").click());
    expect(fns.onSelect).toHaveBeenLastCalledWith({ kind: "event", file: 7, eventKind: "object", index: 1 });
    act(() => row("Karte 0007").click());
    expect(fns.onSelect).toHaveBeenLastCalledWith({ kind: "matrixCell", matrix: 0, x: 1, y: 0 });
    act(() => group("header").click());
    expect(fns.onSelect).toHaveBeenLastCalledWith({ kind: "header", id: 3 });
    pressKey(row("Karte 0006"), "ArrowRight");
    act(() => row("Gebäude 1").click());
    expect(fns.onSelect).toHaveBeenLastCalledWith({ kind: "building", map: 6, index: 0 });
    // A script opens in the script tab; it is not a selection.
    fns.onSelect.mockReset();
    act(() => row("Scriptdatei 0042").click());
    expect(fns.onOpenScript).toHaveBeenCalledWith(42);
    expect(fns.onSelect).not.toHaveBeenCalled();
    // A group opens and closes on a click.
    act(() => group("object").click());
    expect(group("object").getAttribute("aria-expanded")).toBe("false");
    expect(fns.onSelect).not.toHaveBeenCalled();
    // The selection is marked: the same index in another class is another event.
    show({ selected: { kind: "event", file: 7, eventKind: "warp", index: 1 } });
    expect(row("Warp 1").getAttribute("aria-selected")).toBe("true");
    expect(items().filter((r) => r.getAttribute("aria-selected") === "true")).toHaveLength(1);
    show({ selected: { kind: "building", map: 6, index: 1 } });
    expect(row("Gebäude 2").getAttribute("aria-selected")).toBe("true");
    show({ selected: { kind: "header", id: 3 } });
    expect(group("header").getAttribute("aria-selected")).toBe("true");
  });

  it("follows_a_door_on_enter_and_double_click", () => {
    show();
    pressKey(row("Warp 0"), "Enter");
    expect(fns.onFollow).toHaveBeenCalledWith(414);
    // Enter selects the door as well, like the first click of a double click.
    expect(fns.onSelect).toHaveBeenCalledWith({ kind: "event", file: 7, eventKind: "warp", index: 0 });
    act(() => void row("Warp 0").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(fns.onFollow).toHaveBeenCalledTimes(2);
    expect(row("Warp 0").getAttribute("title")).toBe("Doppelklick oder Enter folgt der Tür zum Ziel");
    // An unknown target, another event, a building: nothing to follow.
    pressKey(row("Warp 1"), "Enter");
    act(() => void row("Warp 1").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    pressKey(row("Objekt 0"), "Enter");
    expect(fns.onFollow).toHaveBeenCalledTimes(2);
  });

  it("shows_the_lock_notice_and_badges", () => {
    show({ locked: "17 bytes read", marks: new Map([[3, "error"]]), faultyEvents: [{ file: 7, kind: "warp", index: 1 }] });
    const notice = container.querySelector('[role="status"]')!;
    expect(notice.textContent).toContain("Eventdatei gesperrt");
    expect(notice.textContent).toContain("schreibgeschützt: 17 bytes read");
    for (const g of ["object", "warp", "coord", "bg"]) expect(group(g).textContent).toContain("Gesperrt");
    for (const g of ["header", "maps", "scripts"]) expect(group(g).textContent).not.toContain("Gesperrt");
    expect(group("header").querySelector('[data-finding-mark="error"]')).not.toBeNull();
    expect(row("Warp 1").querySelector('[data-finding-mark="error"]')).not.toBeNull();
    expect(row("Warp 0").querySelector("[data-finding-mark]")).toBeNull();
    show();
    expect(container.querySelector('[role="status"]')).toBeNull();
    expect(container.textContent).not.toContain("Gesperrt");
  });

  it("says_what_to_do_when_empty", () => {
    show({ header: null, events: null, matrix: null, buildings: [] });
    expect(container.textContent).toContain("Noch keine Szene gezeigt.");
    expect(items()).toHaveLength(0);
    // A scene with nothing in it says how to add something.
    show({ events: { ...events, objects: [], warps: [], coords: [], bg: [] }, buildings: [] });
    expect(container.textContent).toContain("Hier ist noch nichts.");
    // While the events load it is not empty yet.
    show({ events: null, buildings: [], loading: true });
    expect(container.textContent).toContain("Lade …");
    expect(container.textContent).not.toContain("Hier ist noch nichts.");
  });

  it("adds_with_the_plus_menu_and_locks_the_event_tools", () => {
    show();
    press(button("Hinzufügen"));
    expect(menuItems().map((i) => i.textContent)).toEqual(["Figur", "Tür", "Auslöser", "Schild", "Gebäude", "Raum hier anbinden…"]);
    act(() => menuItems()[4]!.click());
    expect(fns.onTool).toHaveBeenCalledWith("building");
    press(button("Hinzufügen"));
    act(() => menuItems()[5]!.click());
    expect(fns.onConnectRoom).toHaveBeenCalledOnce();
    // Locked like the tools: the event tools say why, the building stays.
    show({ toolsBlocked: "schreibgeschützt: 17 bytes read" });
    press(button("Hinzufügen"));
    const [figure, , , , building] = menuItems();
    expect(figure!.getAttribute("aria-disabled")).toBe("true");
    expect(figure!.textContent).toBe("Figur, schreibgeschützt: 17 bytes read");
    expect(building!.getAttribute("aria-disabled")).toBeNull();
    act(() => figure!.click());
    expect(fns.onTool).toHaveBeenCalledOnce();
  });

  it("moves_with_arrows_home_end_and_opens_closes_with_right_left", () => {
    show();
    const first = items()[0]!;
    expect(first.tabIndex).toBe(0);
    act(() => first.focus());
    pressKey(first, "ArrowDown");
    expect(document.activeElement).toBe(group("maps"));
    pressKey(group("maps"), "End");
    expect(document.activeElement).toBe(items()[items().length - 1]);
    pressKey(document.activeElement as HTMLElement, "Home");
    expect(document.activeElement).toBe(first);
    pressKey(group("object"), "ArrowLeft");
    expect(group("object").getAttribute("aria-expanded")).toBe("false");
    expect(row("Objekt 0")).toBeUndefined();
    pressKey(group("object"), "ArrowRight");
    expect(row("Objekt 0")).toBeDefined();
    expect(items().filter((r) => r.tabIndex === 0)).toHaveLength(1);
  });

  it("focuses_a_group_when_asked", () => {
    show();
    pressKey(group("warp"), "ArrowLeft");
    expect(row("Warp 0")).toBeUndefined();
    show({ focusGroup: { group: "warp", token: 1 } });
    expect(group("warp").getAttribute("aria-expanded")).toBe("true");
    expect(row("Warp 0")).toBeDefined();
    expect(document.activeElement).toBe(group("warp"));
    // The same token again does nothing; a new one focuses again.
    act(() => first().focus());
    show({ focusGroup: { group: "warp", token: 1 } });
    expect(document.activeElement).toBe(first());
    show({ focusGroup: { group: "warp", token: 2 } });
    expect(document.activeElement).toBe(group("warp"));
    function first() {
      return items()[0]!;
    }
  });

  it("goes_back_and_forward", () => {
    show();
    expect(button("Zurück").disabled).toBe(true);
    expect(button("Vor").disabled).toBe(true);
    show({ canBack: true, canForward: true });
    act(() => button("Zurück").click());
    act(() => button("Vor").click());
    expect(fns.onBack).toHaveBeenCalledOnce();
    expect(fns.onForward).toHaveBeenCalledOnce();
    // Also without a scene (the history may hold one).
    show({ header: null, canBack: true });
    act(() => button("Zurück").click());
    expect(fns.onBack).toHaveBeenCalledTimes(2);
  });
});
