// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ShellProvider } from "../ui/frame/ShellContext";
import { PaintContext } from "./paint/PaintContext";
import type { PaintModel } from "./paint/PaintContext";
import { SceneOverlays } from "./SceneOverlays";
import { NO_WORLD, WorkbenchProviders } from "./WorkbenchContext";
import type { WorkbenchData } from "./WorkbenchContext";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const KEY = "g4.shell.v1";

/** The overlays' stylesheet as written (jsdom lays nothing out and applies no CSS Modules), read from the package root vitest runs in. */
const overlaysCss = readFileSync("src/workbench/SceneOverlays.module.css", "utf8");
const toolbarCss = readFileSync("src/ui/frame/FloatingToolbar.module.css", "utf8");

describe("SceneOverlays", () => {
  let container: HTMLElement;
  let root: Root;
  const onTool = vi.fn();
  const onShowPermissions = vi.fn();
  const onShowHeights = vi.fn();
  const onShowEvents = vi.fn();
  const onOpenWizard = vi.fn();
  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const f of [onTool, onShowPermissions, onShowHeights, onShowEvents, onOpenWizard]) f.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  const show = (over: Partial<WorkbenchData> = {}, paint?: PaintModel) => {
    const data = { tool: "select", onTool, eventFileId: 3, eventLock: null, selected: 7, onOpenWizard, ...over } as unknown as WorkbenchData;
    const layers = { showPermissions: true, showHeights: false, showEvents: false, onShowPermissions, onShowHeights, onShowEvents };
    act(() =>
      root.render(
        <ShellProvider>
          <WorkbenchProviders data={data} live={{ inspector: { kind: "none" }, matrixSelection: null, message: null, world: NO_WORLD }}>
            {paint ? (
              <PaintContext.Provider value={paint}>
                <SceneOverlays layers={layers} />
              </PaintContext.Provider>
            ) : (
              <SceneOverlays layers={layers} />
            )}
          </WorkbenchProviders>
        </ShellProvider>,
      ),
    );
  };
  const overlays = () => container.querySelector<HTMLElement>("[data-scene-overlays]")!;
  const toolbar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Werkzeugleiste"]')!;
  const layerBar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Ebenen der Schau"]')!;
  const tool = (name: string) => container.querySelector<HTMLButtonElement>(`button[data-tool][aria-label="${name}"]`)!;
  const chooseAnchor = (label: string) => {
    const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Lage der Werkzeugleiste"]')!;
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    const item = [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")].find((i) => i.textContent === label)!;
    act(() => item.click());
  };

  it("puts_the_layers_and_the_toolbar_into_one_corner", () => {
    show({ tool: "paint" });
    // The layer bar first, the toolbar under it: one column at the top left of the view.
    expect([...overlays().children]).toEqual([layerBar(), toolbar()]);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    expect(toolbar().getAttribute("aria-orientation")).toBe("vertical");
    expect(tool("Gelände malen").getAttribute("aria-pressed")).toBe("true");
    act(() => tool("Figur").click());
    expect(onTool).toHaveBeenCalledWith("figure");
    act(() => layerBar().querySelector<HTMLButtonElement>('button[aria-label="Ereignisse"]')!.click());
    expect(onShowEvents).toHaveBeenCalledWith(true);
  });

  it("shows_the_paint_sub_tools_while_painting_a_world_map", () => {
    const set = vi.fn();
    const model = (target: number | null): PaintModel => ({ state: { sub: "rect", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2 }, set, target });
    show({ tool: "paint" }, model(120));
    const group = container.querySelector<HTMLElement>("[data-paint-tools]")!;
    expect(group.querySelector('[data-paint-tool="rect"]')!.getAttribute("aria-pressed")).toBe("true");
    act(() => group.querySelector<HTMLButtonElement>('[data-paint-tool="erase"]')!.click());
    expect(set).toHaveBeenCalledWith({ sub: "erase" });
    // Another tool, or a binary map: no sub-tools.
    show({ tool: "select" }, model(120));
    expect(container.querySelector("[data-paint-tools]")).toBeNull();
    show({ tool: "paint" }, model(null));
    expect(container.querySelector("[data-paint-tools]")).toBeNull();
  });

  it("height_and_ramp_show_the_height_layer", () => {
    const model = (sub: PaintModel["state"]["sub"]): PaintModel => ({ state: { sub, layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2 }, set: vi.fn(), target: 120 });
    show({ tool: "paint" }, model("brush"));
    expect(onShowHeights).not.toHaveBeenCalled();
    show({ tool: "paint" }, model("height"));
    expect(onShowHeights).toHaveBeenCalledWith(true);
    onShowHeights.mockReset();
    show({ tool: "paint" }, model("ramp"));
    expect(onShowHeights).toHaveBeenCalledWith(true);
  });

  it("takes_the_anchor_from_the_shell_and_writes_it_back", () => {
    localStorage.setItem(KEY, JSON.stringify({ toolbar: "top" }));
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    expect(toolbar().getAttribute("aria-orientation")).toBe("horizontal");
    chooseAnchor("Unten");
    expect(overlays().getAttribute("data-anchor")).toBe("bottom");
    expect(toolbar().getAttribute("data-anchor")).toBe("bottom");
    expect(JSON.parse(localStorage.getItem(KEY)!).toolbar).toBe("bottom");
  });

  const viewHeight = (height: number) =>
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const h = this.hasAttribute("data-scene-overlays") ? height : 0;
      return { top: 0, bottom: h, height: h, left: 0, right: 600, width: 600, x: 0, y: 0, toJSON: () => ({}) } as DOMRect;
    });

  it("keeps_a_flat_toolbar_that_wraps_under_the_layer_bar_at_the_top_of_a_narrow_view", () => {
    // 1100 × 720 with the inspector beside the view: a view about 460 px wide and low, so the toolbar lies flat beside the layer bar and
    // wraps to a second line. jsdom lays nothing out; the row is the anchor's, and its stylesheet packs the lines at the top (with the gap)
    // instead of spreading them over the view's height.
    viewHeight(300);
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    const top = /\.overlays\[data-anchor="top"\] \{([^}]*)\}/.exec(overlaysCss)?.[1] ?? "";
    expect(top).toContain("flex-wrap: wrap");
    expect(top).toContain("align-content: flex-start");
    // The upright column (anchor left) is not a wrapping row.
    const column = /\.overlays \{([^}]*)\}/.exec(overlaysCss)?.[1] ?? "";
    expect(column).toContain("flex-direction: column");
    expect(column).not.toContain("flex-wrap");
  });

  it("lays_the_toolbar_flat_on_top_in_a_low_view_and_keeps_the_anchor", () => {
    viewHeight(300);
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    expect(toolbar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(JSON.parse(localStorage.getItem(KEY)!).toolbar).toBe("left");
  });

  it("ticks_the_remembered_anchor_in_a_low_view_and_writes_the_entry_chosen", () => {
    viewHeight(300);
    show();
    const openMenu = () => {
      const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Lage der Werkzeugleiste"]')!;
      act(() => {
        trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
        trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
        trigger.click();
      });
      return [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
    };
    // The remembered left shows flat on top; the menu ticks "Links" and says why.
    const items = openMenu();
    expect(items.map((i) => [i.textContent, i.getAttribute("aria-checked")])).toEqual([
      ["Links (bei dieser Höhe oben)", "true"],
      ["Oben", "false"],
      ["Unten", "false"],
    ]);
    act(() => items[1]!.click());
    expect(JSON.parse(localStorage.getItem(KEY)!).toolbar).toBe("top");
  });

  it("calls_a_view_low_below_460_px_in_komfort_and_below_400_in_kompakt", () => {
    viewHeight(430);
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    act(() => root.unmount());
    root = createRoot(container);
    localStorage.setItem("g4.density", "kompakt");
    show();
    expect(overlays().getAttribute("data-anchor")).toBe("left");
  });

  it("keeps_the_paint_sub_tools_inside_the_flat_toolbar_of_a_narrow_view", () => {
    // Browser run 6.4a: a view about 340 px wide, low, so the toolbar lies flat; the sub-tools ran past the bar's edge behind the
    // inspector. They are a group of the toolbar itself, whose width stops at the view and whose flat row (and the group) wraps.
    viewHeight(300);
    const model: PaintModel = { state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2 }, set: vi.fn(), target: 120 };
    show({ tool: "paint" }, model);
    const group = container.querySelector<HTMLElement>("[data-paint-tools]")!;
    expect(group.parentElement).toBe(toolbar());
    expect(toolbar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(/\.overlays > \* \{([^}]*)\}/.exec(overlaysCss)?.[1]).toContain("max-width: 100%");
    const flat = /\.bar\[aria-orientation="horizontal"\] \{([^}]*)\}/.exec(toolbarCss)?.[1] ?? "";
    expect(flat).toContain("flex-wrap: wrap");
    const flatGroup = /\.bar\[aria-orientation="horizontal"\] \.group \{([^}]*)\}/.exec(toolbarCss)?.[1] ?? "";
    expect(flatGroup).toContain("flex-wrap: wrap");
    expect(flatGroup).toContain("min-width: 0");
  });

  it("keeps_the_upright_bar_and_puts_the_sub_tools_beside_it_where_they_do_not_fit_under_it", () => {
    // 600 px is high enough for the upright tools alone (460), not with the nine sub-tools under them (460 + 5 + 9 × 44 = 861 in
    // komfort): the bar stays upright, the sub-tools stand beside it.
    const model: PaintModel = { state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2 }, set: vi.fn(), target: 120 };
    const spy = viewHeight(600);
    show({ tool: "select" }, model);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    show({ tool: "paint" }, model);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    expect(toolbar().getAttribute("aria-orientation")).toBe("vertical");
    expect(toolbar().hasAttribute("data-paint-beside")).toBe(true);
    // High enough for both under each other: one column.
    spy.mockRestore();
    act(() => root.unmount());
    root = createRoot(container);
    viewHeight(900);
    show({ tool: "paint" }, model);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    expect(toolbar().hasAttribute("data-paint-beside")).toBe(false);
  });

  it("lays_the_toolbar_flat_with_sub_tools_only_below_the_old_threshold", () => {
    const model: PaintModel = { state: { sub: "brush", layer: 0, tile: "0041", ramp: { dir: "e", steps: 1 }, lower: false, edgeSet: null, cliffRise: 2 }, set: vi.fn(), target: 120 };
    viewHeight(461);
    show({ tool: "paint" }, model);
    expect(overlays().getAttribute("data-anchor")).toBe("left");
    act(() => root.unmount());
    root = createRoot(container);
    viewHeight(459);
    show({ tool: "paint" }, model);
    expect(overlays().getAttribute("data-anchor")).toBe("top");
    expect(toolbar().hasAttribute("data-paint-beside")).toBe(false);
  });

  it("locks_the_event_tools_with_the_locked_text_of_the_event_file", () => {
    show({ eventLock: { file: 103, reason: "17 bytes read" } });
    const figure = tool("Figur");
    expect(figure.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(figure.getAttribute("aria-describedby")!)?.textContent).toBe("schreibgeschützt: 17 bytes read");
    expect(tool("Gebäude").getAttribute("aria-disabled")).toBeNull();
  });

  it("locks_the_event_tools_while_no_place_is_shown", () => {
    show({ eventFileId: null });
    const door = tool("Tür");
    expect(door.getAttribute("aria-disabled")).toBe("true");
    expect(document.getElementById(door.getAttribute("aria-describedby")!)?.textContent).toBe("Erst einen Ort wählen.");
  });

  it("does_nothing_on_a_click_at_a_locked_tool", () => {
    show({ eventLock: { file: 103, reason: "17 bytes read" } });
    act(() => tool("Figur").click());
    act(() => tool("Schild").click());
    expect(onTool).not.toHaveBeenCalled();
  });

  it("erzaehlen_shows_the_recipe_bar_instead_of_the_tools_and_bauen_the_tools", () => {
    const recipes = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Rezepte"]');
    const recipe = (id: string) => container.querySelector<HTMLButtonElement>(`button[data-recipe="${id}"]`)!;
    localStorage.setItem(KEY, JSON.stringify({ area: "welt", depth: "erzaehlen", toolbar: "bottom" }));
    show();
    // The recipes take the toolbar's place: the same anchor, beside the same layer bar.
    expect(container.querySelector('[role="toolbar"][aria-label="Werkzeugleiste"]')).toBeNull();
    expect([...overlays().children]).toEqual([layerBar(), recipes()]);
    expect(recipes()!.getAttribute("data-anchor")).toBe("bottom");
    act(() => recipe("create_area").click());
    expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "area" });
    // „Raum anlegen“ puts the door outside into the selected area or room.
    act(() => recipe("create_room").click());
    expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "room", prefill: { outsideHeader: 7 } });
    // The recipes of 3d.5 start in the selected place (the host adds the tile last chosen in the scene).
    act(() => recipe("new_figure").click());
    expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "new_figure", prefill: { header: 7 } });
    act(() => recipe("connect_doors").click());
    expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "connect_doors", prefill: { a: { header: 7 } } });
    act(() => recipe("new_trainer_here").click());
    expect(onOpenWizard).toHaveBeenLastCalledWith({ kind: "new_trainer_here", prefill: { header: 7 } });
    expect(onOpenWizard).toHaveBeenCalledTimes(5);
    act(() => root.unmount());
    root = createRoot(container);
    localStorage.setItem(KEY, JSON.stringify({ area: "welt", depth: "bauen" }));
    show();
    expect(recipes()).toBeNull();
    expect(toolbar()).not.toBeNull();
  });
});
