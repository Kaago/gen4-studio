// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { EventsDto, HeaderSummary } from "../api";
import { pressKey } from "../ui/primitives/testUtils";
import { ContextPanel } from "./ContextPanel";
import type { ContextPanelProps } from "./ContextPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number, internal_name: string): HeaderSummary => ({ id, internal_name, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
const events: EventsDto = {
  event_file_id: 7,
  locked: null,
  bg: [{ index: 0, script: 12, event_type: 0, x: 4, z: 5, y: 0 }],
  objects: [
    { index: 0, local_id: 1, graphics_id: 3, movement_type: 0, script: 2, dir: 1, x: 10, z: 20, y: 0 },
    { index: 1, local_id: 2, graphics_id: 4, movement_type: 0, script: 3, dir: 0, x: 11, z: 21, y: 0 },
  ],
  warps: [{ index: 0, x: 8, z: 9, dest_header_id: 414, dest_warp_id: 0, height: 0 }],
  coords: [],
};

describe("ContextPanel", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelectEvent = vi.fn();
  const onSelectBuilding = vi.fn();
  const onOpenHeader = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onSelectEvent, onSelectBuilding, onOpenHeader]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<ContextPanelProps> = {}) =>
    act(() =>
      root.render(
        <ContextPanel
          hasPlace
          events={events}
          headers={[header(414, "T01R0201")]}
          buildings={[{ map: 40, models: [21, 22] }]}
          selectedEvent={null}
          selectedBuilding={null}
          onSelectEvent={onSelectEvent}
          onSelectBuilding={onSelectBuilding}
          onOpenHeader={onOpenHeader}
          {...over}
        />,
      ),
    );
  const row = (text: string) => [...container.querySelectorAll<HTMLElement>("[role=option]")].find((r) => r.textContent?.includes(text))!;

  it("heads_the_panel_with_the_counts_and_groups_the_events_by_class_then_the_buildings", () => {
    show();
    expect(container.querySelector("h2")?.textContent).toBe("An diesem Ort");
    expect(container.textContent).toContain("2 Figuren, 1 Tür, 1 Schild, 2 Gebäude");
    const groups = [...container.querySelectorAll('[role="group"]')].map((g) => g.getAttribute("aria-label") ?? g.querySelector("div")?.textContent);
    expect(groups).toEqual(["Figuren2", "Türen1", "Schilder1", "Gebäude2", "Karte 0040"]);
    expect(row("Objekt 1 · ID 2").textContent).toContain("x 11, z 21");
    expect(row("Warp 0 → Header 0414 (T01R0201)")).toBeDefined();
    expect(row("Gebäude 2").textContent).toContain("Modell 22");
  });

  it("makes_the_list_reachable_by_keyboard_when_a_place_is_chosen_after_the_empty_state", () => {
    show({ hasPlace: false, events: null });
    expect(container.querySelectorAll("[role=option]")).toHaveLength(0);
    show();
    const options = [...container.querySelectorAll<HTMLElement>("[role=option]")];
    expect(options.filter((o) => o.tabIndex === 0)).toHaveLength(1);
    expect(options[0]!.tabIndex).toBe(0);
    pressKey(options[0]!, "ArrowDown");
    expect(options.filter((o) => o.tabIndex === 0)).toEqual([options[1]]);
  });

  it("selects_an_event_on_a_click_like_a_click_in_the_scene", () => {
    show();
    act(() => row("Objekt 1").click());
    expect(onSelectEvent).toHaveBeenCalledWith({ file: 7, kind: "object", index: 1 });
  });

  it("selects_a_building_on_a_click", () => {
    show();
    act(() => row("Gebäude 2").click());
    expect(onSelectBuilding).toHaveBeenCalledWith({ map: 40, index: 1 });
  });

  it("marks_the_selected_event_and_building", () => {
    show({ selectedEvent: { file: 7, kind: "object", index: 0 }, selectedBuilding: { map: 40, index: 1 } });
    expect(row("Objekt 0").getAttribute("aria-selected")).toBe("true");
    expect(row("Objekt 1").getAttribute("aria-selected")).toBe("false");
    expect(row("Gebäude 2").getAttribute("aria-selected")).toBe("true");
    expect(row("Gebäude 1").getAttribute("aria-selected")).toBe("false");
    // The same index in another class is another event.
    show({ selectedEvent: { file: 7, kind: "warp", index: 1 } });
    expect(container.querySelectorAll('[aria-selected="true"]').length).toBe(0);
  });

  it("follows_a_door_to_its_header_on_a_double_click_and_nothing_else_does", () => {
    show();
    const dbl = (el: HTMLElement) => act(() => void el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    dbl(row("Warp 0"));
    expect(onOpenHeader).toHaveBeenCalledWith(414);
    dbl(row("Objekt 0"));
    dbl(row("Gebäude 1"));
    expect(onOpenHeader).toHaveBeenCalledOnce();
  });

  it("follows_a_door_to_its_header_on_enter_and_nothing_else_does", () => {
    show();
    pressKey(row("Warp 0"), "Enter");
    expect(onOpenHeader).toHaveBeenCalledWith(414);
    // Enter selects the door as well, like the click of a double click.
    expect(onSelectEvent).toHaveBeenCalledWith({ file: 7, kind: "warp", index: 0 });
    pressKey(row("Objekt 0"), "Enter");
    pressKey(row("Gebäude 1"), "Enter");
    expect(onOpenHeader).toHaveBeenCalledOnce();
    show({ headers: [] });
    pressKey(row("Warp 0"), "Enter");
    expect(onOpenHeader).toHaveBeenCalledOnce();
  });

  it("says_it_is_loading_while_the_events_of_the_place_load_instead_of_calling_it_empty", () => {
    show({ events: null, buildings: [], loading: true });
    expect(container.textContent).toContain("Lade …");
    expect(container.textContent).not.toContain("Hier ist noch nichts.");
    // Read (or unreadable): the place is what it is.
    show({ events: null, buildings: [], loading: false });
    expect(container.textContent).not.toContain("Lade …");
    expect(container.textContent).toContain("Hier ist noch nichts.");
  });

  it("does_not_follow_a_door_to_a_header_that_does_not_exist", () => {
    show({ headers: [] });
    act(() => void row("Warp 0").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    expect(onOpenHeader).not.toHaveBeenCalled();
  });

  it("warns_when_the_event_file_is_locked", () => {
    show({ events: { ...events, locked: "17 bytes read" } });
    expect(container.querySelector('[role="status"]')?.textContent).toContain("schreibgeschützt: 17 bytes read");
    show();
    expect(container.querySelector('[role="status"]')).toBeNull();
  });

  it("asks_for_a_place_when_none_is_shown", () => {
    show({ hasPlace: false, events: null, buildings: [] });
    expect(container.textContent).toContain("Noch kein Ort gewählt.");
    expect(container.querySelector("[role=option]")).toBeNull();
  });

  it("says_so_for_a_place_without_events_and_buildings", () => {
    show({ events: { ...events, bg: [], objects: [], warps: [] }, buildings: [] });
    expect(container.textContent).toContain("Noch nichts");
    expect(container.textContent).toContain("Hier ist noch nichts.");
    expect(container.querySelector("[role=option]")).toBeNull();
  });

  it("lists_the_buildings_while_the_events_still_load", () => {
    show({ events: null });
    expect(row("Gebäude 1")).toBeDefined();
    expect(container.textContent).toContain("2 Gebäude");
  });
});
