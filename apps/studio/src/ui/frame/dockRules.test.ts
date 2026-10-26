// @vitest-environment jsdom
import { DockviewComponent } from "dockview-react";
import type { DockviewApi, DockviewGroupPanel, DockviewWillShowOverlayLocationEvent, GroupDragEvent, TabDragEvent } from "dockview-react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DeckArea } from "../../workbench/layout";
import { applyArrangement, dockPanel, floatPanel, keepViewGroupClosed, keepViewGroupInPlace, keepViewPanelsInPlace, toggleSchauMaximized } from "./dockRules";

// jsdom has no ResizeObserver; dockview watches its container with one.
globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

interface GridNode {
  type: "leaf" | "branch";
  data: GridNode[] | { views: string[] };
}
/**
 * The grid as nested lists: a group as its panel ids, a split as the list of its parts (directions alternate down the tree). A split of one
 * part is that part: dockview may wrap the root once more without changing the layout.
 */
const shape = (node: GridNode): unknown => {
  if (node.type === "leaf") return (node.data as { views: string[] }).views;
  const parts = node.data as GridNode[];
  return parts.length === 1 ? shape(parts[0]!) : parts.map(shape);
};
const grid = (api: DockviewApi) => shape(api.toJSON().grid.root as unknown as GridNode);
/** Whether the group of `id` takes the full height of the dock (1200 × 800 here). */
const fullHeight = (api: DockviewApi, id: string) => api.getPanel(id)!.group.api.height === api.height;
/** The Werkbank: the view above script and problems on the left, the inspector right of both in full height. */
const WERKBANK = [
  [
    ["world", "matrix", "worldmap"],
    ["script", "problems"],
  ],
  ["inspector"],
];
const floating = (api: DockviewApi) => api.groups.filter((g) => g.api.location.type === "floating").map((g) => g.panels.map((p) => p.id));

describe("dock rules", () => {
  let element: HTMLElement;
  let dock: DockviewComponent;
  /** What the dock said (its announcements go here instead of its live region). */
  let said: string[];
  const api = () => dock.api;

  beforeEach(() => {
    localStorage.clear();
    element = document.createElement("div");
    document.body.append(element);
    said = [];
    dock = new DockviewComponent(element, {
      createComponent: () => ({ element: document.createElement("div"), init: () => {} }),
      announcer: (event) => void said.push(event.message),
    });
    dock.layout(1200, 800);
  });
  afterEach(() => {
    dock.dispose();
    element.remove();
  });
  const build = (area: DeckArea = "welt", arrangement: "werkbank" | "doppeldeck" = "werkbank") => applyArrangement(api(), area, arrangement);

  it("builds_the_werkbank_with_3d_in_front_and_the_inspector_limited_to_280_to_480", () => {
    build();
    expect(grid(api())).toEqual(WERKBANK);
    expect(fullHeight(api(), "inspector")).toBe(true);
    expect(fullHeight(api(), "world")).toBe(false);
    // dockview activates what was added last; the view and each group open with their first panel.
    expect(api().getPanel("world")!.api.isVisible).toBe(true);
    expect(api().getPanel("script")!.api.isVisible).toBe(true);
    expect(api().activePanel?.id).toBe("world");
    const inspector = api().getPanel("inspector")!.group;
    expect([inspector.minimumWidth, inspector.maximumWidth]).toEqual([280, 480]);
  });

  it("builds_the_doppeldeck_and_switches_between_the_two_keeping_the_panels", () => {
    build("welt", "doppeldeck");
    expect(grid(api())).toEqual([["world", "matrix", "worldmap"], ["inspector", "script", "problems"]]);
    expect(api().getPanel("inspector")!.api.isVisible).toBe(true);
    const world = api().getPanel("world");
    build("welt", "werkbank");
    expect(grid(api())).toEqual(WERKBANK);
    // The panels are moved, not rebuilt: the scene and the editors keep what they hold.
    expect(api().getPanel("world")).toBe(world);
    expect(api().activePanel?.id).toBe("world");
    build("welt", "doppeldeck");
    expect(grid(api())).toEqual([["world", "matrix", "worldmap"], ["inspector", "script", "problems"]]);
    // The limits of the inspector beside the view do not follow it into the edit group below.
    expect(api().getPanel("inspector")!.group.maximumWidth).not.toBe(480);
  });

  it("builds_the_other_areas_with_the_view_above_the_edit_group", () => {
    build("texte", "doppeldeck");
    expect(grid(api())).toEqual([["textView"], ["textEdit", "problems"]]);
    expect(api().getPanel("textEdit")!.api.isVisible).toBe(true);
    expect(api().activePanel?.id).toBe("textView");
  });

  it("rebuilds_from_a_floating_inspector_and_a_maximized_group", () => {
    build();
    floatPanel(api(), "inspector");
    api().maximizeGroup(api().getPanel("script")!);
    build();
    expect(api().hasMaximizedGroup()).toBe(false);
    expect(floating(api())).toEqual([]);
    expect(grid(api())).toEqual(WERKBANK);
  });

  it("floats_a_panel_beside_the_view_but_never_a_view_panel", () => {
    build();
    expect(floatPanel(api(), "world")).toBe(false);
    expect(floatPanel(api(), "nothing")).toBe(false);
    expect(floatPanel(api(), "inspector")).toBe(true);
    expect(floating(api())).toEqual([["inspector"]]);
    expect(api().getPanel("inspector")!.api.location.type).toBe("floating");
    // Already floating: nothing to do.
    expect(floatPanel(api(), "inspector")).toBe(false);
  });

  it("anchors_a_floating_panel_to_the_right_edge_of_the_view", () => {
    build();
    const float = vi.spyOn(api(), "addFloatingGroup");
    floatPanel(api(), "inspector");
    // Anchored right (not at a left measured while the inspector still stood beside the view), so it stays there when the dock resizes.
    // (jsdom has no layout, so the box dockview clamps it to is not looked at.)
    expect(float).toHaveBeenCalledWith(api().getPanel("inspector"), { position: { right: 16, top: 16 }, width: 340, height: 480 });
    expect(api().toJSON().floatingGroups?.[0]?.position).toHaveProperty("right");
  });

  it("docks_a_floating_panel_where_the_arrangement_puts_it", () => {
    build();
    floatPanel(api(), "inspector");
    said = [];
    expect(dockPanel(api(), "welt", "inspector")).toBe(true);
    // Said once, without the closing and opening of the move.
    expect(said).toEqual(["Inspector angedockt"]);
    // Werkbank: right of the view, in full height, with its limits again.
    expect(grid(api())).toEqual(WERKBANK);
    expect(fullHeight(api(), "inspector")).toBe(true);
    expect(api().getPanel("inspector")!.group.maximumWidth).toBe(480);
    expect(dockPanel(api(), "welt", "inspector")).toBe(false);
    // Doppeldeck: back into the edit group below the view.
    build("welt", "doppeldeck");
    floatPanel(api(), "inspector");
    expect(dockPanel(api(), "welt", "inspector", "doppeldeck")).toBe(true);
    expect(grid(api())).toEqual([["world", "matrix", "worldmap"], ["script", "problems", "inspector"]]);
    // A panel whose group-mates float too goes where its own step puts it.
    floatPanel(api(), "script");
    expect(dockPanel(api(), "welt", "script", "werkbank")).toBe(true);
    expect(api().getPanel("script")!.api.location.type).toBe("grid");
  });

  it("maximizes_the_view_group_and_restores_it", () => {
    build();
    expect(toggleSchauMaximized(api())).toBe(true);
    expect(api().getPanel("world")!.api.isMaximized()).toBe(true);
    expect(toggleSchauMaximized(api())).toBe(false);
    expect(api().hasMaximizedGroup()).toBe(false);
    // Another group maximized: the button maximizes the view instead.
    api().maximizeGroup(api().getPanel("inspector")!);
    expect(toggleSchauMaximized(api())).toBe(true);
    expect(api().getPanel("world")!.api.isMaximized()).toBe(true);
  });

  it("keeps_view_panels_from_being_dragged", () => {
    build();
    const drag = (id: string) => {
      const nativeEvent = new Event("dragstart", { cancelable: true });
      keepViewPanelsInPlace({ nativeEvent, panel: api().getPanel(id)! } as unknown as TabDragEvent);
      return nativeEvent.defaultPrevented;
    };
    expect(drag("world")).toBe(true);
    expect(drag("matrix")).toBe(true);
    expect(drag("inspector")).toBe(false);
    expect(drag("script")).toBe(false);
    // Nor the view group as a whole (by its header), which would move or float the view.
    const dragGroup = (id: string) => {
      const nativeEvent = new Event("dragstart", { cancelable: true });
      keepViewGroupInPlace({ nativeEvent, group: api().getPanel(id)!.group } as unknown as GroupDragEvent);
      return nativeEvent.defaultPrevented;
    };
    expect(dragGroup("world")).toBe(true);
    expect(dragGroup("inspector")).toBe(false);
  });

  it("keeps_other_panels_out_of_the_view_group_but_lets_them_split_beside_it", () => {
    build();
    const view = api().getPanel("world")!.group;
    const edit = api().getPanel("script")!.group;
    const overlay = (group: DockviewGroupPanel, kind: string, position: string) => {
      const event = { group, kind, position, preventDefault: vi.fn() };
      keepViewGroupClosed(event as unknown as DockviewWillShowOverlayLocationEvent);
      return event.preventDefault.mock.calls.length > 0;
    };
    expect(overlay(view, "tab", "center")).toBe(true);
    expect(overlay(view, "header_space", "center")).toBe(true);
    expect(overlay(view, "content", "center")).toBe(true);
    expect(overlay(view, "content", "right")).toBe(false);
    expect(overlay(view, "content", "bottom")).toBe(false);
    expect(overlay(edit, "tab", "center")).toBe(false);
    expect(overlay(edit, "content", "center")).toBe(false);
  });

  it("refuses_every_drop_of_a_view_panel_or_the_view_group_whichever_drag_brought_it", () => {
    // A touch or pen drag does not ask onWillDragPanel; its drop overlays are refused instead.
    build();
    const edit = api().getPanel("script")!.group;
    const view = api().getPanel("world")!.group;
    const overlay = (data: { panelId: string | null; groupId: string } | undefined, position = "right") => {
      const event = { group: edit, kind: "content", position, api: api(), getData: () => data, preventDefault: vi.fn() };
      keepViewGroupClosed(event as unknown as DockviewWillShowOverlayLocationEvent);
      return event.preventDefault.mock.calls.length > 0;
    };
    expect(overlay({ panelId: "world", groupId: view.id })).toBe(true);
    expect(overlay({ panelId: null, groupId: view.id })).toBe(true);
    expect(overlay({ panelId: "inspector", groupId: api().getPanel("inspector")!.group.id })).toBe(false);
    expect(overlay(undefined)).toBe(false);
  });

  it("brings_a_floating_view_group_back_into_the_grid_when_it_rebuilds", () => {
    build();
    const world = api().getPanel("world");
    // What a Shift+click on the view's header did before the deck stopped it.
    api().addFloatingGroup(world!.group);
    expect(world!.api.location.type).toBe("floating");
    build();
    expect(floating(api())).toEqual([]);
    expect(grid(api())).toEqual(WERKBANK);
    expect(api().getPanel("world")).toBe(world);
    expect(api().activePanel?.id).toBe("world");
  });
});
