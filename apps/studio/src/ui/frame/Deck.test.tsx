// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import type { DockviewApi, DockviewGroupPanel, IDockviewPanelProps } from "dockview-react";
import i18next from "i18next";
import { act } from "react";
import type { FunctionComponent } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AREA_PANELS, arrangementKey, layoutKey } from "../../workbench/layout";
import type { PanelId } from "../../workbench/layout";
import { scriptDirtyStore } from "../../workbench/script/scriptDirtyStore";
import { Deck, resetLayout } from "./Deck";
import { floatPanel } from "./dockRules";
import type { DeckPreview } from "./Deck";
import { ShellProvider, useShell } from "./ShellContext";
import type { ShellApi } from "./ShellContext";
import { HAND_DEFAULT } from "./shellState";
import type { Depth } from "./shellState";

/** The deck's stylesheet as written (jsdom applies no CSS Modules), read from the package root vitest runs in. */
const deckCss = readFileSync("src/ui/frame/Deck.module.css", "utf8");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
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
/** The grid as nested lists: a group as its panel ids, a split as the list of its parts; a split of one part is that part. */
const shape = (node: GridNode): unknown => {
  if (node.type === "leaf") return (node.data as { views: string[] }).views;
  const parts = node.data as GridNode[];
  return parts.length === 1 ? shape(parts[0]!) : parts.map(shape);
};
const layoutOf = (api: DockviewApi) => shape(api.toJSON().grid.root as unknown as GridNode);
/** The Werkbank: the view above script and problems, the inspector right of both. */
const WERKBANK = [
  [
    ["world", "matrix", "worldmap"],
    ["script", "problems"],
  ],
  ["inspector"],
];
/** Lets dockview announce what it changed: its layout event comes a tick after the change. */
const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));

const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
/** A serialized dockview layout with these groups side by side. */
const stored = (...grouped: string[][]) => ({
  grid: { root: { type: "branch", data: grouped.map((g) => leaf(...g)) }, width: 1000, height: 400, orientation: "HORIZONTAL" },
  panels: Object.fromEntries(grouped.flat().map((id) => [id, { id, contentComponent: id, title: id }])),
});

const Empty: FunctionComponent<IDockviewPanelProps> = () => null;
const components = (ids: PanelId[]) => Object.fromEntries(ids.map((id) => [id, Empty]));
const WELT = components(["world", "matrix", "worldmap", "inspector", "script", "problems"]);
const TEXTE = components(["textView", "textEdit", "problems"]);
const POKEMON = components(["pokemonView", "pokemonEdit"]);

describe("Deck", () => {
  let container: HTMLElement;
  let root: Root;
  let shell: ShellApi;
  const dockRef = { current: null as DockviewApi | null };
  /** What App gives the world's deck for „Als Vorschaubild“ (undefined: nothing, as for the other decks). */
  let preview: DeckPreview | undefined;

  function Probe() {
    shell = useShell();
    return null;
  }
  /** Renders the deck (again: the dock stays); `wizard` as App gives it to the world's deck; Tüfteln unless `depth` says otherwise. */
  const render = async (area: "welt" | "texte", wizard?: boolean, depth: Depth = "tuefteln") => {
    await act(async () =>
      root.render(
        <ShellProvider initialArea={area}>
          <Probe />
          <Deck area={area} depth={depth} components={area === "welt" ? WELT : TEXTE} dockRef={dockRef} hinge={<div role="toolbar" aria-label="Scharnier" />} wizard={wizard} preview={preview} />
        </ShellProvider>,
      ),
    );
  };
  /** Mounts a fresh deck (a deck already there is unmounted first). */
  const mount = async (area: "welt" | "texte" = "welt", wizard?: boolean, depth: Depth = "tuefteln") => {
    act(() => root.unmount());
    root = createRoot(container);
    dockRef.current = null;
    await render(area, wizard, depth);
  };
  const api = () => dockRef.current!;
  const dock = () => container.querySelector<HTMLElement>("[data-dock]")!;
  const tab = (title: string) => [...dock().querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === title)!;
  /** Opens the context menu of a tab the way a right click does and returns its items. */
  const menu = async (title: string) => {
    await act(async () => void tab(title).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })));
    return [...document.querySelectorAll<HTMLElement>('.dv-context-menu [role="menuitem"]')];
  };
  const choose = async (title: string, label: string) => {
    const item = (await menu(title)).find((i) => i.textContent === label)!;
    await act(async () => item.click());
    await settle();
  };
  const groupOf = (id: string) => api().getPanel(id)!.group;
  /** Fires what dockview fires before it shows a drop overlay (a drag over a group); true when the deck refused it. */
  const overlay = (group: DockviewGroupPanel, kind: string, position: string) => {
    const event = { group, kind, position, preventDefault: vi.fn(), get defaultPrevented() { return event.preventDefault.mock.calls.length > 0; } };
    (api() as unknown as { component: { _onWillShowOverlay: { fire: (e: unknown) => void } } }).component._onWillShowOverlay.fire(event);
    return event.defaultPrevented;
  };

  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    preview = undefined;
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    scriptDirtyStore.set(0);
  });

  it("renders_one_dock_per_area_with_the_hinge_above", async () => {
    await mount();
    expect(container.querySelectorAll("[data-dock]")).toHaveLength(1);
    expect(dock().id).toBe("dock-welt");
    expect(dock().getAttribute("data-dock")).toBe("welt");
    // Every panel of the area is in the one dock; the deck has no splitter of its own (the sash between groups is dockview's).
    expect(api().panels.map((p) => p.id).sort()).toEqual(AREA_PANELS.welt.map((p) => p.id).sort());
    expect(container.querySelector("[data-splitter]")).toBeNull();
    const hinge = container.querySelector('[role="toolbar"]')!;
    expect(Boolean(hinge.compareDocumentPosition(dock()) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    await mount("texte");
    expect(dock().id).toBe("dock-texte");
    expect(dock().getAttribute("data-dock")).toBe("texte");
  });

  it("builds_the_default_arrangement_and_remembers_it_under_v11", async () => {
    await mount();
    expect(layoutOf(api())).toEqual(WERKBANK);
    // 3D in front, the view group active; each other group opens with its first panel.
    expect(api().activePanel?.id).toBe("world");
    expect(api().getPanel("script")!.api.isVisible).toBe(true);
    expect(api().getPanel("inspector")!.api.isVisible).toBe(true);
    // The inspector beside the view keeps between 280 and 480 px.
    expect([groupOf("inspector").minimumWidth, groupOf("inspector").maximumWidth]).toEqual([280, 480]);
    await settle();
    const saved = JSON.parse(localStorage.getItem(layoutKey("welt"))!);
    expect(Object.keys(saved.panels).sort()).toEqual(["inspector", "matrix", "problems", "script", "world", "worldmap"]);
    expect(shape(saved.grid.root)).toEqual(WERKBANK);
    expect(Object.keys(localStorage).filter((k) => k.startsWith("g4.layout.")).every((k) => k === layoutKey("welt") || k === arrangementKey("welt"))).toBe(true);
    await mount("texte");
    expect(layoutOf(api())).toEqual([["textView"], ["textEdit", "problems"]]);
    expect(api().activePanel?.id).toBe("textView");
    expect(api().getPanel("textEdit")!.api.isVisible).toBe(true);
    expect(Object.keys(JSON.parse(localStorage.getItem(layoutKey("texte"))!).panels).sort()).toEqual(["problems", "textEdit", "textView"]);
  });

  it("restores_a_stored_layout", async () => {
    localStorage.setItem(layoutKey("welt"), JSON.stringify(stored(["world", "matrix"], ["worldmap"], ["inspector", "script", "problems"])));
    await mount();
    expect(layoutOf(api())).toEqual([["world", "matrix"], ["worldmap"], ["inspector", "script", "problems"]]);
  });

  it("retitles_the_tabs_of_a_restored_layout_and_after_a_switch_of_language", async () => {
    // A layout stored in English keeps no title of its own: the tabs say what the language shown says.
    const english = stored(["world", "matrix", "worldmap"], ["inspector", "script", "problems"]);
    english.panels.worldmap.title = "World map";
    english.panels.problems.title = "Problems";
    localStorage.setItem(layoutKey("welt"), JSON.stringify(english));
    await mount();
    const titles = () => [...dock().querySelectorAll<HTMLElement>("[data-dock-tab]")].map((t) => t.textContent);
    expect(titles()).toEqual(expect.arrayContaining(["3D", "Raster", "Weltkarte", "Inspector", "Script", "Probleme"]));
    try {
      await act(async () => void (await i18next.changeLanguage("en")));
      expect(titles()).toEqual(expect.arrayContaining(["3D", "Matrix", "World map", "Inspector", "Script", "Problems"]));
      expect(titles()).not.toContain("Weltkarte");
    } finally {
      // The suite runs in German (vitest.setup.ts).
      await act(async () => void (await i18next.changeLanguage("de")));
    }
    expect(titles()).toEqual(expect.arrayContaining(["Weltkarte", "Probleme"]));
    await settle();
    const saved = JSON.parse(localStorage.getItem(layoutKey("welt"))!);
    expect([saved.panels.worldmap.title, saved.panels.problems.title]).toEqual(["Weltkarte", "Probleme"]);
  });

  it("restores_the_limits_of_the_inspector_beside_the_view", async () => {
    await mount();
    await settle();
    const saved = localStorage.getItem(layoutKey("welt"))!;
    await mount();
    localStorage.setItem(layoutKey("welt"), saved);
    await mount();
    expect(layoutOf(api())).toEqual(WERKBANK);
    expect([groupOf("inspector").minimumWidth, groupOf("inspector").maximumWidth]).toEqual([280, 480]);
  });

  it("falls_back_to_the_default_when_the_stored_layout_is_unusable_or_a_view_panel_floats", async () => {
    const floatingWorld = {
      ...stored(["matrix", "worldmap"], ["inspector", "script", "problems"]),
      panels: Object.fromEntries(AREA_PANELS.welt.map((p) => [p.id, { id: p.id, contentComponent: p.id, title: p.title }])),
      floatingGroups: [{ data: { views: ["world"], activeView: "world", id: "f" }, position: { left: 10, top: 10, width: 300, height: 300 } }],
    };
    const cases = [
      // A panel is missing (a layout of another panel set).
      JSON.stringify(stored(["world", "matrix"], ["inspector", "script", "problems"])),
      // A panel of an older version is in it.
      JSON.stringify(stored(["world", "matrix", "worldmap"], ["inspector", "script", "problems"], ["context"])),
      // Not JSON.
      "{not json",
      // The right panels, but a grid dockview refuses.
      JSON.stringify({ ...stored(["world", "matrix", "worldmap", "inspector", "script", "problems"]), grid: { root: { type: "nonsense" } } }),
      // 3D floats (stored by hand): the view stays docked.
      JSON.stringify(floatingWorld),
    ];
    for (const value of cases) {
      localStorage.setItem(layoutKey("welt"), value);
      await mount();
      expect(layoutOf(api())).toEqual(WERKBANK);
      expect(api().getPanel("world")!.api.location.type).toBe("grid");
      expect(api().getPanel("context")).toBeUndefined();
    }
  });

  it("shows_the_tab_bar_of_every_group", async () => {
    await mount();
    // Also the inspector alone in its group shows its tab.
    for (const group of api().groups) expect(group.header.hidden).toBe(false);
    expect(groupOf("inspector").element.querySelectorAll('[role="tab"]')).toHaveLength(1);
    expect([...dock().querySelectorAll('[role="tab"]')].map((t) => t.textContent).sort()).toEqual(["3D", "Inspector", "Probleme", "Raster", "Script", "Weltkarte"]);
    // Panels cannot be closed: no tab has a close button.
    expect(dock().querySelector(".dv-default-tab-action")).toBeNull();
  });

  it("marks_groups_and_panels_with_data_attributes", async () => {
    await mount();
    const marked = [...dock().querySelectorAll<HTMLElement>("[data-dock-group]")];
    expect(marked.map((g) => g.getAttribute("data-dock-group")).sort()).toEqual(api().groups.map((g) => g.id).sort());
    expect(dock().querySelectorAll('[data-dock-group-kind="schau"]')).toHaveLength(1);
    expect(groupOf("world").element.getAttribute("data-dock-group-kind")).toBe("schau");
    expect(groupOf("inspector").element.hasAttribute("data-dock-group-kind")).toBe(false);
    // Each panel is wrapped in its own element (dockview keeps the content of every panel rendered, over its group).
    for (const p of AREA_PANELS.welt) expect(dock().querySelectorAll(`[data-dock-panel="${p.id}"]`)).toHaveLength(1);
    // A group made later (the inspector floats) is marked too.
    await choose("Inspector", "Schweben lassen");
    expect(groupOf("inspector").element.getAttribute("data-dock-group")).toBe(groupOf("inspector").id);
  });

  it("marks_each_tab_with_its_panel_so_the_f6_round_finds_the_content_of_the_active_one", async () => {
    await mount();
    for (const p of AREA_PANELS.welt) expect(tab(p.title).querySelector("[data-dock-tab]")?.getAttribute("data-dock-tab")).toBe(p.id);
    // The view group's active tab names the panel whose content shows over it.
    const active = groupOf("world").element.querySelector<HTMLElement>(".dv-active-tab [data-dock-tab]")!;
    expect(active.getAttribute("data-dock-tab")).toBe(groupOf("world").activePanel!.id);
  });

  it("does_not_let_a_view_panel_be_dragged", async () => {
    await mount();
    const drag = (title: string) => {
      const event = new Event("dragstart", { bubbles: true, cancelable: true });
      tab(title).dispatchEvent(event);
      return event.defaultPrevented;
    };
    expect(drag("3D")).toBe(true);
    expect(drag("Weltkarte")).toBe(true);
    expect(drag("Inspector")).toBe(false);
  });

  it("keeps_other_panels_out_of_the_view_group_as_tabs", async () => {
    await mount();
    expect(overlay(groupOf("world"), "tab", "center")).toBe(true);
    expect(overlay(groupOf("world"), "content", "center")).toBe(true);
    expect(overlay(groupOf("world"), "content", "right")).toBe(false);
    expect(overlay(groupOf("script"), "content", "center")).toBe(false);
  });

  it("keeps_the_view_docked_on_a_shift_click_on_its_tab_or_its_header", async () => {
    await mount();
    // dockview floats a tab, or a whole group from its empty header, on Shift+pointerdown without asking onWillDragPanel.
    const shiftDown = (target: Element) => {
      const event = new MouseEvent("pointerdown", { bubbles: true, cancelable: true, shiftKey: true, button: 0 });
      act(() => void target.dispatchEvent(event));
      return event.defaultPrevented;
    };
    shiftDown(tab("3D"));
    shiftDown(tab("Raster"));
    shiftDown(groupOf("world").element.querySelector(".dv-void-container")!);
    expect(api().getPanel("world")!.api.location.type).toBe("grid");
    expect(api().getPanel("matrix")!.api.location.type).toBe("grid");
    expect(api().groups.filter((g) => g.api.location.type === "floating")).toHaveLength(0);
    // The inspector still floats that way (dockview's own gesture).
    shiftDown(tab("Inspector"));
    expect(api().getPanel("inspector")!.api.location.type).toBe("floating");
  });

  it("brings_a_floating_view_back_with_the_reset_of_the_arrangement", async () => {
    // A view that floats all the same (an older build let Shift+click do it) is not stuck: the reset docks it again.
    await mount();
    const world = api().getPanel("world");
    act(() => api().addFloatingGroup(groupOf("world")));
    expect(world!.api.location.type).toBe("floating");
    await choose("Inspector", "Anordnung zurücksetzen");
    expect(world!.api.location.type).toBe("grid");
    expect(layoutOf(api())).toEqual(WERKBANK);
    expect(api().activePanel?.id).toBe("world");
  });

  it("maximizes_and_restores_on_a_tab_double_click", async () => {
    await mount();
    const twice = (title: string) => act(async () => void tab(title).querySelector("span")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    await twice("Script");
    expect(api().getPanel("script")!.api.isMaximized()).toBe(true);
    await twice("Script");
    expect(api().hasMaximizedGroup()).toBe(false);
    await twice("3D");
    expect(api().getPanel("world")!.api.isMaximized()).toBe(true);
    await twice("3D");
    expect(api().hasMaximizedGroup()).toBe(false);
  });

  it("fills_the_dock_with_a_maximized_group_whatever_its_width_limits", async () => {
    await mount();
    act(() => api().layout(1000, 600));
    expect(groupOf("inspector").width).toBeLessThanOrEqual(480);
    // The inspector beside the view keeps at most 480 px; maximized it fills the dock, not 480 px of it.
    act(() => api().getPanel("inspector")!.api.maximize());
    expect(groupOf("inspector").width).toBe(1000);
    act(() => api().exitMaximizedGroup());
    expect([groupOf("inspector").minimumWidth, groupOf("inspector").maximumWidth]).toEqual([280, 480]);
    expect(groupOf("inspector").width).toBeLessThanOrEqual(480);
  });

  it("remembers_the_arrangement_it_builds_on_the_first_start", async () => {
    await mount();
    expect(localStorage.getItem(arrangementKey("welt"))).toBe("werkbank");
    await mount("texte");
    expect(localStorage.getItem(arrangementKey("texte"))).toBe("doppeldeck");
  });

  it("works_the_tab_menu_with_the_keyboard", async () => {
    await mount();
    const script = tab("Script");
    script.focus();
    await act(async () => void script.dispatchEvent(new KeyboardEvent("keydown", { key: "F10", shiftKey: true, bubbles: true, cancelable: true })));
    const items = () => [...document.querySelectorAll<HTMLElement>('.dv-context-menu [role="menuitem"]')];
    // The menu takes the focus on its first item; the arrows walk the items, round.
    expect(document.activeElement).toBe(items()[0]);
    const key = (k: string) => act(async () => void document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })));
    await key("ArrowDown");
    expect(document.activeElement?.textContent).toBe("Schweben lassen");
    await key("ArrowUp");
    await key("ArrowUp");
    expect(document.activeElement?.textContent).toBe("Anordnung zurücksetzen");
    // Escape closes it and gives the focus back to the tab.
    await key("Escape");
    expect(items()).toHaveLength(0);
    expect(document.activeElement).toBe(script);
    // Enter runs the item.
    await act(async () => void script.dispatchEvent(new KeyboardEvent("keydown", { key: "F10", shiftKey: true, bubbles: true, cancelable: true })));
    await key("Enter");
    await settle();
    expect(api().getPanel("script")!.api.isMaximized()).toBe(true);
    expect(items()).toHaveLength(0);
  });

  it("walks_onto_a_locked_line_by_keyboard_but_never_runs_it", async () => {
    const set = vi.fn();
    preview = { set, locked: (id) => (id === "worldmap" ? "Erst eine Karte wählen" : null) };
    await mount();
    const map = tab("Weltkarte");
    map.focus();
    await act(async () => void map.dispatchEvent(new KeyboardEvent("keydown", { key: "F10", shiftKey: true, bubbles: true, cancelable: true })));
    const items = () => [...document.querySelectorAll<HTMLElement>('.dv-context-menu [role="menuitem"]')];
    const key = (k: string) => act(async () => void document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })));
    // The focus starts on the first line that works; the arrows also reach the locked one, so its reason is read out.
    expect(document.activeElement?.textContent).toBe("Maximieren");
    await key("ArrowDown");
    expect(document.activeElement?.textContent).toBe("Als Vorschaubild");
    expect(document.activeElement?.getAttribute("aria-disabled")).toBe("true");
    // Enter and Space on it do nothing: the menu stays open, the focus stays.
    await key("Enter");
    await key(" ");
    expect(set).not.toHaveBeenCalled();
    expect(items()).toHaveLength(2);
    expect(document.activeElement?.textContent).toBe("Als Vorschaubild");
    await key("ArrowDown");
    expect(document.activeElement?.textContent).toBe("Maximieren");
  });

  it("offers_the_tab_menu_in_german_also_with_shift_f10", async () => {
    await mount();
    expect((await menu("Inspector")).map((i) => i.textContent)).toEqual(["Maximieren", "Schweben lassen", "Anordnung zurücksetzen"]);
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    expect((await menu("3D")).map((i) => i.textContent)).toEqual(["Maximieren"]);
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    await act(async () => void tab("Script").dispatchEvent(new KeyboardEvent("keydown", { key: "F10", shiftKey: true, bubbles: true, cancelable: true })));
    expect([...document.querySelectorAll('.dv-context-menu [role="menuitem"]')].map((i) => i.textContent)).toContain("Schweben lassen");
  });

  it("offers_als_vorschaubild_on_3d_and_weltkarte_and_says_why_it_is_locked", async () => {
    const set = vi.fn();
    let reason: string | null = "Erst eine Karte wählen";
    let placeReason: string | null = "Erst einen Ort zeigen";
    preview = { set, locked: (id) => (id === "worldmap" ? reason : id === "world" ? placeReason : null) };
    await mount();
    const closeMenus = () => document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    // Without a chosen map the world map's item is locked; its tooltip says why, and the keyboard passes it by.
    const locked = await menu("Weltkarte");
    expect(locked.map((i) => [i.textContent, i.getAttribute("aria-disabled"), i.getAttribute("title")])).toEqual([
      ["Maximieren", null, null],
      ["Als Vorschaubild", "true", "Erst eine Karte wählen"],
    ]);
    expect(document.querySelectorAll('.dv-context-menu [role="separator"]')).toHaveLength(1);
    // Built like dockview's own lines (so a change of dockview's markup shows here): its item class with the locked modifier.
    expect(locked[0]!.className).toBe("dv-context-menu-item");
    expect(locked[1]!.className).toBe("dv-context-menu-item dv-context-menu-item--disabled");
    // The reason also reaches a screen reader: the line is described by a hidden text in the menu.
    const described = document.getElementById(locked[1]!.getAttribute("aria-describedby") ?? "");
    expect(described?.textContent).toBe("Erst eine Karte wählen");
    expect(described?.hidden).toBe(true);
    expect(described?.closest(".dv-context-menu")).not.toBeNull();
    await act(async () => locked[1]!.click());
    expect(set).not.toHaveBeenCalled();
    closeMenus();
    // Without a place the 3D view has no scene: its item is locked as well, with its own reason.
    const noPlace = await menu("3D");
    expect(noPlace.map((i) => [i.textContent, i.getAttribute("aria-disabled"), i.getAttribute("title")])).toEqual([
      ["Maximieren", null, null],
      ["Als Vorschaubild", "true", "Erst einen Ort zeigen"],
    ]);
    await act(async () => noPlace[1]!.click());
    expect(set).not.toHaveBeenCalled();
    closeMenus();
    placeReason = null;
    await choose("3D", "Als Vorschaubild");
    expect(set).toHaveBeenCalledWith("world");
    closeMenus();
    reason = null;
    await choose("Weltkarte", "Als Vorschaubild");
    expect(set).toHaveBeenLastCalledWith("worldmap");
    closeMenus();
    // The Raster has none.
    expect((await menu("Raster")).map((i) => i.textContent)).toEqual(["Maximieren"]);
  });

  it("floats_and_docks_the_inspector", async () => {
    await mount();
    await choose("Inspector", "Schweben lassen");
    expect(api().getPanel("inspector")!.api.location.type).toBe("floating");
    // Said in German; the group stays inside the window, and is moved by its own title bar.
    expect(dock().querySelector(".dv-live-region")?.textContent).toBe("Inspector schwebt");
    expect((api() as unknown as { component: { options: object } }).component.options).toMatchObject({ floatingGroupBounds: "boundedWithinViewport", floatingGroupDragHandle: "titlebar" });
    expect(layoutOf(api())).toEqual([
      ["world", "matrix", "worldmap"],
      ["script", "problems"],
    ]);
    // The floating layout is remembered (a floating inspector is a sane layout).
    expect(JSON.parse(localStorage.getItem(layoutKey("welt"))!).floatingGroups).toHaveLength(1);
    // dockview's own locked line: the classes the deck's locked lines copy.
    expect((await menu("Inspector"))[0]!.className).toBe("dv-context-menu-item dv-context-menu-item--disabled");
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    expect((await menu("Inspector")).map((i) => [i.textContent, i.getAttribute("aria-disabled")])).toEqual([
      ["Maximieren", "true"],
      ["Schweben lassen", "true"],
      ["Zurück ins Dock", null],
      ["Anordnung zurücksetzen", null],
    ]);
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    await choose("Inspector", "Zurück ins Dock");
    expect(dock().querySelector(".dv-live-region")?.textContent).toBe("Inspector angedockt");
    expect(api().getPanel("inspector")!.api.location.type).toBe("grid");
    expect(layoutOf(api())).toEqual(WERKBANK);
  });

  it("themes_a_floating_group_and_the_tab_menu_like_the_docked_groups", async () => {
    await mount();
    await choose("Inspector", "Schweben lassen");
    // dockview puts floating groups and its menus beside its grid, into its shell: the theme sits on the shell, so it reaches them too, and
    // nothing of dockview's own dark theme is left.
    const floating = dock().querySelector<HTMLElement>(".dv-floating-overlay-host > .dv-resize-container")!;
    expect(floating).not.toBeNull();
    expect(floating.closest(".dockview-theme-g4")).not.toBeNull();
    expect(dock().querySelector(".dv-grid-view")!.closest(".dockview-theme-g4")).not.toBeNull();
    const item = (await menu("Inspector"))[0]!;
    expect(item.closest(".dv-context-menu")!.closest(".dockview-theme-g4")).not.toBeNull();
    expect(dock().querySelector('[class*="dockview-theme-"]:not(.dockview-theme-g4)')).toBeNull();
    document.querySelectorAll(".dv-context-menu").forEach((m) => m.remove());
    // Without dockview's theme the G4 theme brings the variables dockview needs (a floating group's layer above the grid), and a floating
    // group is a sheet (`flaeche/blatt`) with the floating edge and shadow.
    const theme = readFileSync("src/ui/tokens/dockview-theme.css", "utf8");
    expect(theme).toMatch(/\.dockview-theme-g4 \{[^}]*--dv-overlay-z-index: 999;/);
    expect(theme).toMatch(/\.dockview-theme-g4 \{[^}]*--dv-floating-titlebar-height: /);
    expect(theme).toMatch(/\.dockview-theme-g4 \.dv-resize-container \{[^}]*--dv-group-view-background-color: var\(--g4-flaeche-blatt\)/);
  });

  it("hides_a_drop_anchor_that_changes_its_container_as_dockviews_own_themes_do", () => {
    // Without the rule the drop anchor of a drag that moves between the grid and a floating group glides over from where it was.
    const theme = readFileSync("src/ui/tokens/dockview-theme.css", "utf8");
    expect(theme).toContain(".dockview-theme-g4 .dv-drop-target-container .dv-drop-target-anchor.dv-drop-target-anchor-container-changed { opacity: 0; transition: none; }");
  });

  it("resets_to_the_arrangement_without_rebuilding_the_panels_and_remembers_it", async () => {
    await mount();
    const world = api().getPanel("world");
    await choose("Inspector", "Schweben lassen");
    act(() => resetLayout("welt", dockRef, "doppeldeck", "tuefteln"));
    await settle();
    expect(layoutOf(api())).toEqual([["world", "matrix", "worldmap"], ["inspector", "script", "problems"]]);
    expect(localStorage.getItem(arrangementKey("welt"))).toBe("doppeldeck");
    expect(shape(JSON.parse(localStorage.getItem(layoutKey("welt"))!).grid.root)).toEqual([["world", "matrix", "worldmap"], ["inspector", "script", "problems"]]);
    // „Anordnung zurücksetzen“ builds the arrangement chosen last; „Layout zurücksetzen“ (no arrangement given) the area's default.
    await choose("Script", "Schweben lassen");
    await choose("Inspector", "Anordnung zurücksetzen");
    expect(layoutOf(api())).toEqual([["world", "matrix", "worldmap"], ["inspector", "script", "problems"]]);
    act(() => resetLayout("welt", dockRef, undefined, "tuefteln"));
    await settle();
    expect(layoutOf(api())).toEqual(WERKBANK);
    expect(localStorage.getItem(arrangementKey("welt"))).toBe("werkbank");
    // The panels were moved, never added again: the scene (and what listens to its panel) stays.
    expect(api().getPanel("world")).toBe(world);
    expect(api().activePanel?.id).toBe("world");
  });

  it("builds_bauen_without_the_raster_on_a_reset", async () => {
    await mount("welt", undefined, "bauen");
    const noRaster = [
      [
        ["world", "worldmap"],
        ["script", "problems"],
      ],
      ["inspector"],
    ];
    expect(layoutOf(api())).toEqual(noRaster);
    expect(tab("Raster")).toBeUndefined();
    const world = api().getPanel("world");
    act(() => resetLayout("welt", dockRef, "doppeldeck", "bauen"));
    await settle();
    expect(layoutOf(api())).toEqual([["world", "worldmap"], ["inspector", "script", "problems"]]);
    act(() => resetLayout("welt", dockRef, undefined, "bauen"));
    await settle();
    expect(layoutOf(api())).toEqual(noRaster);
    expect(api().getPanel("world")).toBe(world);
    // Remembered without the Raster, and still a layout the dock restores (in Tüfteln with the Raster back as a tab).
    const saved = JSON.parse(localStorage.getItem(layoutKey("welt"))!);
    expect(Object.keys(saved.panels).sort()).toEqual(["inspector", "problems", "script", "world", "worldmap"]);
    await mount("welt", undefined, "tuefteln");
    expect(layoutOf(api())).toEqual(WERKBANK);
    expect(api().getPanel("world")!.api.isVisible).toBe(true);
  });

  it("restores_a_layout_with_the_raster_in_bauen_without_it", async () => {
    localStorage.setItem(layoutKey("welt"), JSON.stringify(stored(["world", "matrix", "worldmap"], ["inspector", "script", "problems"])));
    await mount("welt", undefined, "bauen");
    expect(layoutOf(api())).toEqual([["world", "worldmap"], ["inspector", "script", "problems"]]);
    await settle();
    expect(Object.keys(JSON.parse(localStorage.getItem(layoutKey("welt"))!).panels)).not.toContain("matrix");
  });

  it("follows_the_depth_with_the_raster_tab", async () => {
    await mount();
    const world = api().getPanel("world");
    act(() => api().getPanel("matrix")!.api.setActive());
    await render("welt", undefined, "bauen");
    expect(api().getPanel("matrix")).toBeUndefined();
    expect(api().getPanel("world")!.api.isVisible).toBe(true);
    expect([...dock().querySelectorAll('[role="tab"]')].map((t) => t.textContent)).not.toContain("Raster");
    await render("welt", undefined, "tuefteln");
    expect(layoutOf(api())).toEqual(WERKBANK);
    expect(api().getPanel("matrix")!.api.isVisible).toBe(false);
    expect(api().getPanel("world")!.api.isVisible).toBe(true);
    // The dock was not built anew, and its content panel is there again.
    expect(api().getPanel("world")).toBe(world);
    expect(dock().querySelectorAll('[data-dock-panel="matrix"]')).toHaveLength(1);
  });

  it("forgets_the_stored_layout_of_a_dock_that_is_not_built_so_it_starts_from_the_arrangement", () => {
    // "Layout zurücksetzen" on Start, Technik or Einstellungen works on the world, which may never have been built in this session.
    localStorage.setItem(layoutKey("texte"), JSON.stringify({ panels: { textView: {} } }));
    localStorage.setItem(layoutKey("welt"), JSON.stringify({ panels: { tools: {} } }));
    expect(() => resetLayout("texte", { current: null }, undefined, "bauen")).not.toThrow();
    expect(localStorage.getItem(layoutKey("texte"))).toBeNull();
    // Another area's layout stays.
    expect(localStorage.getItem(layoutKey("welt"))).not.toBeNull();
  });

  describe("per sub-area", () => {
    /** The Pokémon deck showing `sub`, as App gives it the shell's sub-area. */
    const pokemonDeck = (sub: string) => (
      <ShellProvider initialArea="pokemon">
        <Deck area="pokemon" sub={sub} depth="bauen" components={POKEMON} dockRef={dockRef} hinge={<div role="toolbar" aria-label="Scharnier" />} />
      </ShellProvider>
    );
    /** Renders the Pokémon deck showing `sub` (again: the dock stays). */
    const renderPokemon = async (sub: string) => act(async () => root.render(pokemonDeck(sub)));
    const storedOf = (sub: string) => JSON.parse(localStorage.getItem(layoutKey("pokemon", sub)) ?? "null") as { panels: object; grid: { root: GridNode }; floatingGroups?: unknown[] } | null;
    const floats = (id: string) => api().getPanel(id)!.api.location.type === "floating";
    const DOPPELDECK = [["pokemonView"], ["pokemonEdit"]];

    it("switching the sub swaps the stored layouts and keeps both", async () => {
      // The layout stored for „Attacken“ is unusable (a panel of another dock): that sub-area falls back to its arrangement.
      localStorage.setItem(layoutKey("pokemon", "moves"), JSON.stringify(stored(["pokemonView"], ["pokemonEdit"], ["world"])));
      act(() => root.unmount());
      root = createRoot(container);
      dockRef.current = null;
      await renderPokemon("pokedex");
      await settle();
      const dockview = api();
      expect(layoutOf(api())).toEqual(DOPPELDECK);
      expect(storedOf("pokedex")?.floatingGroups ?? []).toHaveLength(0);
      // „Bearbeiten“ floats in the Pokédex, and the sub-area changes in the same step, before dockview announces the change (its layout
      // event waits for a microtask): the switch itself stores the layout it leaves.
      act(() => {
        floatPanel(api(), "pokemonEdit");
        root.render(pokemonDeck("trainer"));
      });
      // The Pokédex's layout was stored with its floating group first; the trainers start from the arrangement, in the same dock.
      expect(storedOf("pokedex")?.floatingGroups).toHaveLength(1);
      expect(dockRef.current).toBe(dockview);
      expect(floats("pokemonEdit")).toBe(false);
      expect(layoutOf(api())).toEqual(DOPPELDECK);
      await settle();
      expect(storedOf("trainer")?.floatingGroups ?? []).toHaveLength(0);
      expect(storedOf("pokedex")?.floatingGroups).toHaveLength(1);
      // The unusable layout of „Attacken“: the arrangement, no foreign panel, and the others keep theirs.
      await renderPokemon("moves");
      await settle();
      expect(layoutOf(api())).toEqual(DOPPELDECK);
      expect(api().getPanel("world")).toBeUndefined();
      expect(Object.keys(storedOf("moves")!.panels).sort()).toEqual(["pokemonEdit", "pokemonView"]);
      expect(storedOf("pokedex")?.floatingGroups).toHaveLength(1);
      expect(storedOf("trainer")).not.toBeNull();
      // Back in the Pokédex its floating „Bearbeiten“ comes back; the dock is still the same.
      await renderPokemon("pokedex");
      await settle();
      expect(floats("pokemonEdit")).toBe(true);
      expect(dockRef.current).toBe(dockview);
      // The tab menu's reset works on the shown sub-area only.
      await choose("Bearbeiten", "Anordnung zurücksetzen");
      expect(floats("pokemonEdit")).toBe(false);
      expect(storedOf("pokedex")?.floatingGroups ?? []).toHaveLength(0);
      // Nothing is kept under the area's own key: every layout of the Pokémon dock belongs to a sub-area.
      expect(localStorage.getItem(layoutKey("pokemon"))).toBeNull();
      expect(localStorage.getItem(arrangementKey("pokemon"))).toBeNull();
      expect(localStorage.getItem(arrangementKey("pokemon", "pokedex"))).toBe("doppeldeck");
    });

    it("resets_the_layout_of_the_given_sub_area_only", async () => {
      localStorage.setItem(layoutKey("pokemon", "trainer"), JSON.stringify(stored(["pokemonView", "pokemonEdit"])));
      localStorage.setItem(layoutKey("pokemon", "moves"), JSON.stringify(stored(["pokemonView", "pokemonEdit"])));
      // A dock that is not built forgets the layout of that sub-area only.
      resetLayout("pokemon", { current: null }, undefined, "bauen", "trainer");
      expect(localStorage.getItem(layoutKey("pokemon", "trainer"))).toBeNull();
      expect(localStorage.getItem(layoutKey("pokemon", "moves"))).not.toBeNull();
      // A built dock rebuilds the arrangement and stores it under its sub-area.
      act(() => root.unmount());
      root = createRoot(container);
      dockRef.current = null;
      await renderPokemon("moves");
      expect(layoutOf(api())).toEqual(["pokemonView", "pokemonEdit"]);
      act(() => resetLayout("pokemon", dockRef, undefined, "bauen", "moves"));
      await settle();
      expect(layoutOf(api())).toEqual(DOPPELDECK);
      expect(shape(storedOf("moves")!.grid.root)).toEqual(DOPPELDECK);
    });
  });

  it("does_not_rebuild_the_dock_when_the_frame_state_changes", async () => {
    await mount();
    const before = api();
    act(() => shell.setTool("paint"));
    act(() => shell.toggleTheme());
    expect(dockRef.current).toBe(before);
  });

  it("keeps_the_wizard_strip_under_the_dock_without_unmounting_it", async () => {
    await mount("texte");
    expect(container.querySelector("[data-wizard-hand]")).toBeNull();
    await mount("welt", false);
    const before = api();
    const strip = () => container.querySelector<HTMLElement>("[data-wizard-hand]")!;
    expect(strip().hidden).toBe(true);
    await render("welt", true);
    expect(strip().hidden).toBe(false);
    // Its height is the stylesheet's, the default height of the old hand.
    expect(strip().getAttribute("style")).toBeNull();
    expect(deckCss).toMatch(new RegExp(`\\.wizard \\{[^}]*height: ${HAND_DEFAULT}px`));
    // Under the dock, which stays as it is (the view can still be used).
    expect(Boolean(dock().compareDocumentPosition(strip()) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
    expect(dock().hidden).toBe(false);
    expect(dockRef.current).toBe(before);
    await render("welt", false);
    expect(strip().hidden).toBe(true);
    expect(dockRef.current).toBe(before);
  });

  it("marks_the_script_tab_while_a_script_is_unsaved", async () => {
    await mount();
    const dotIn = (title: string) => tab(title)?.querySelector('[aria-label="ungespeichert"]') ?? null;
    expect(dotIn("Script")).toBeNull();
    act(() => scriptDirtyStore.set(2));
    expect(dotIn("Script")?.getAttribute("role")).toBe("img");
    // Only the script tab carries it; the title is not touched.
    expect(dotIn("Inspector")).toBeNull();
    expect(dotIn("Probleme")).toBeNull();
    expect(api().getPanel("script")!.title).toBe("Script");
    act(() => scriptDirtyStore.set(0));
    expect(dotIn("Script")).toBeNull();
  });
});
