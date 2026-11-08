// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PAINT_TOOLS } from "../../workbench/paintTools";
import { TOOLS } from "../../workbench/tools";
import type { PaintSub } from "../../world/paint/ops";
import { pressKey } from "../primitives/testUtils";
import type { Density } from "../tokens/theme";
import { FloatingToolbar, anchorAt, isLowView, shownAnchor, toolbarMinHeight } from "./FloatingToolbar";
import type { ToolbarAnchor, Tool } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("FloatingToolbar", () => {
  let container: HTMLElement;
  let root: Root;
  const onTool = vi.fn();
  const onAnchor = vi.fn();
  // The 3D view the toolbar floats in: 400 px high from the top of the page.
  const area = () => ({ top: 0, bottom: 400, height: 400, left: 0, right: 600, width: 600, x: 0, y: 0, toJSON: () => ({}) }) as DOMRect;
  beforeEach(() => {
    Element.prototype.setPointerCapture = vi.fn();
    Element.prototype.releasePointerCapture = vi.fn();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onTool.mockReset();
    onAnchor.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const onSub = vi.fn();
  const show = (opts: { tool?: Tool; eventsBlocked?: string | null; density?: Density; anchor?: ToolbarAnchor; chosen?: ToolbarAnchor; sub?: PaintSub | null; beside?: boolean } = {}) =>
    act(() =>
      root.render(
        <FloatingToolbar
          tools={TOOLS}
          tool={opts.tool ?? "select"}
          onTool={onTool}
          eventsBlocked={opts.eventsBlocked ?? null}
          density={opts.density ?? "komfort"}
          anchor={opts.anchor ?? "left"}
          chosen={opts.chosen ?? opts.anchor ?? "left"}
          onAnchor={onAnchor}
          area={area}
          paint={opts.sub ? { tools: PAINT_TOOLS, sub: opts.sub, onSub } : null}
          paintBeside={opts.beside}
        />,
      ),
    );
  const bar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Werkzeugleiste"]')!;
  const keys = () => [...container.querySelectorAll<HTMLButtonElement>("button[data-tool]")];
  const tool = (name: string) => keys().find((b) => b.getAttribute("aria-label") === name)!;
  const grip = () => container.querySelector<HTMLElement>("[data-toolbar-grip]")!;
  const pointer = (type: string, clientY: number) => {
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientX: 10, clientY });
    Object.defineProperty(event, "pointerId", { value: 1 });
    act(() => void grip().dispatchEvent(event));
  };

  it("shows_seven_keys_with_symbol_and_letter_and_exactly_the_active_one_pressed", () => {
    show({ tool: "paint" });
    expect(keys().map((b) => b.getAttribute("aria-label"))).toEqual(["Auswählen", "Gelände malen", "Figur", "Tür", "Auslöser", "Schild", "Gebäude"]);
    expect(keys().map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "true", "false", "false", "false", "false", "false"]);
    // No visible name: the symbol and the letter in the corner.
    expect(tool("Figur").textContent).toBe("F");
    expect(tool("Figur").querySelector("svg")).not.toBeNull();
    expect(bar().textContent).not.toContain("Gelände malen");
    expect(tool("Gelände malen").getAttribute("aria-keyshortcuts")).toBe("B");
    expect(tool("Gelände malen").getAttribute("data-tool")).toBe("paint");
  });

  it("paint_tools_show_only_for_a_world_map", () => {
    // A binary map (or another tool): no second group.
    show({ tool: "paint" });
    expect(container.querySelector("[data-paint-tools]")).toBeNull();
    show({ tool: "paint", sub: "fill" });
    const group = container.querySelector<HTMLElement>("[data-paint-tools]")!;
    expect(group.getAttribute("role")).toBe("group");
    expect(group.getAttribute("aria-label")).toBe("Unterwerkzeuge");
    const subs = [...group.querySelectorAll<HTMLButtonElement>("button[data-paint-tool]")];
    expect(subs.map((b) => b.getAttribute("aria-label"))).toEqual(["Pinsel", "Rechteck", "Füllen", "Radierer", "Pipette", "Höhe", "Rampe", "Terrain-Pinsel", "Klippenpinsel"]);
    expect(subs.map((b) => b.textContent)).toEqual(["B", "P", "G", "E", "I", "H", "R", "T", "K"]);
    // Exactly the active sub-tool is pressed; the main tool stays pressed too.
    expect(subs.map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "false", "true", "false", "false", "false", "false", "false", "false"]);
    expect(tool("Gelände malen").getAttribute("aria-pressed")).toBe("true");
    expect(subs[6]!.getAttribute("aria-keyshortcuts")).toBe("R");
    // The group follows the main tools, after a separator, inside the one roving list of the bar.
    expect(group.previousElementSibling?.getAttribute("role")).toBe("separator");
    expect(subs.every((b) => b.hasAttribute("data-roving"))).toBe(true);
    act(() => subs[5]!.click());
    expect(onSub).toHaveBeenCalledWith("height");
    expect(onTool).not.toHaveBeenCalled();
  });

  it("puts_the_sub_tools_beside_the_upright_bar_only_when_asked_and_upright", () => {
    show({ tool: "paint", sub: "brush" });
    expect(bar().hasAttribute("data-paint-beside")).toBe(false);
    show({ tool: "paint", sub: "brush", beside: true });
    expect(bar().hasAttribute("data-paint-beside")).toBe(true);
    // The group stays the bar's own (one roving list); the separator before it is marked so the column form can hide it.
    const group = container.querySelector<HTMLElement>("[data-paint-tools]")!;
    expect(group.parentElement).toBe(bar());
    expect(group.previousElementSibling?.hasAttribute("data-paint-separator")).toBe(true);
    // Flat bars wrap instead; without sub-tools there is nothing to put beside.
    show({ tool: "paint", sub: "brush", beside: true, anchor: "top" });
    expect(bar().hasAttribute("data-paint-beside")).toBe(false);
    show({ tool: "paint", sub: null, beside: true });
    expect(bar().hasAttribute("data-paint-beside")).toBe(false);
    const css = readFileSync("src/ui/frame/FloatingToolbar.module.css", "utf8");
    const column = /\.bar\[data-paint-beside\] > \.group \{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(column).toContain("position: absolute");
    expect(column).toContain("left: calc(100% + var(--g4-raum-4))");
    expect(css).toMatch(/\.bar\[data-paint-beside\] > \[data-paint-separator\] \{\s*display: none;/);
  });

  it("switches_the_tool_on_a_click", () => {
    show();
    act(() => tool("Tür").click());
    expect(onTool).toHaveBeenCalledWith("door");
  });

  it("separates_the_four_groups", () => {
    show();
    // Auswählen | Gelände malen | Figur, Tür, Auslöser, Schild | Gebäude
    const children = [...bar().querySelectorAll("button[data-tool], [role=separator]")];
    expect(children.map((c) => c.getAttribute("data-tool") ?? "|")).toEqual(["select", "|", "paint", "|", "figure", "door", "trigger", "sign", "|", "building"]);
    // Across the bar: a vertical bar has horizontal separators.
    expect(bar().querySelector("[role=separator]")?.getAttribute("aria-orientation")).toBe("horizontal");
  });

  it("locks_the_four_event_tools_with_the_reason", () => {
    const reason = "schreibgeschützt: 17 bytes read";
    show({ eventsBlocked: reason });
    for (const name of ["Figur", "Tür", "Auslöser", "Schild"]) {
      const button = tool(name);
      expect(button.getAttribute("aria-disabled")).toBe("true");
      // The reason is also the description, not only a tooltip.
      expect(document.getElementById(button.getAttribute("aria-describedby")!)?.textContent).toBe(reason);
      act(() => button.click());
    }
    expect(onTool).not.toHaveBeenCalled();
    for (const name of ["Auswählen", "Gelände malen", "Gebäude"]) {
      expect(tool(name).getAttribute("aria-disabled")).toBeNull();
      expect(tool(name).getAttribute("aria-describedby")).toBeNull();
    }
    act(() => tool("Gebäude").click());
    expect(onTool).toHaveBeenCalledWith("building");
  });

  it("is_vertical_on_the_left_and_horizontal_on_top_and_bottom", () => {
    show({ anchor: "left" });
    expect(bar().getAttribute("aria-orientation")).toBe("vertical");
    expect(bar().getAttribute("data-anchor")).toBe("left");
    show({ anchor: "top" });
    expect(bar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(bar().querySelector("[role=separator]")?.getAttribute("aria-orientation")).toBe("vertical");
    show({ anchor: "bottom" });
    expect(bar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(bar().getAttribute("data-anchor")).toBe("bottom");
  });

  it("moves_between_the_keys_with_the_arrows_of_its_orientation", () => {
    show({ anchor: "left" });
    const [first, second, third] = keys();
    expect(keys().filter((b) => b.tabIndex === 0)).toEqual([first]);
    act(() => first!.focus());
    pressKey(first!, "ArrowRight");
    expect(document.activeElement).toBe(first);
    pressKey(first!, "ArrowDown");
    expect(document.activeElement).toBe(second);
    show({ anchor: "top" });
    pressKey(second!, "ArrowDown");
    expect(document.activeElement).toBe(second);
    pressKey(second!, "ArrowRight");
    expect(document.activeElement).toBe(third);
    expect(keys().filter((b) => b.tabIndex === 0)).toEqual([third]);
  });

  it("moves_to_the_anchor_chosen_in_the_grip_menu", () => {
    show({ anchor: "left" });
    const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Lage der Werkzeugleiste"]')!;
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    const items = [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
    expect(items.map((i) => i.textContent)).toEqual(["Links", "Oben", "Unten"]);
    expect(items.map((i) => i.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
    act(() => items[2]!.click());
    expect(onAnchor).toHaveBeenCalledWith("bottom");
  });

  const openAnchorMenu = () => {
    const trigger = container.querySelector<HTMLButtonElement>('button[aria-label="Lage der Werkzeugleiste"]')!;
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    return [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
  };

  it("ticks_the_remembered_anchor_in_a_low_view_and_says_why_left_lies_flat", () => {
    // The view is low: the remembered left bar shows flat on top.
    show({ anchor: "top", chosen: "left" });
    const items = openAnchorMenu();
    expect(items.map((i) => i.textContent)).toEqual(["Links (bei dieser Höhe oben)", "Oben", "Unten"]);
    expect(items.map((i) => i.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
  });

  it("writes_the_entry_chosen_in_a_low_view_even_when_it_is_what_shows", () => {
    show({ anchor: "top", chosen: "left" });
    const items = openAnchorMenu();
    act(() => items[1]!.click());
    expect(onAnchor).toHaveBeenCalledExactlyOnceWith("top");
  });

  it("moves_to_the_nearest_anchor_after_a_drag", () => {
    show();
    // The grip is for the pointer only; the caret's menu is the way of the keyboard and of assistive tech.
    expect(grip().getAttribute("aria-hidden")).toBe("true");
    expect(grip().tabIndex).toBe(-1);
    for (const [y, anchor] of [
      [50, "top"],
      [350, "bottom"],
      [200, "left"],
    ] as const) {
      onAnchor.mockReset();
      pointer("pointerdown", 120);
      pointer("pointermove", y);
      pointer("pointerup", y);
      expect(onAnchor).toHaveBeenCalledExactlyOnceWith(anchor);
    }
    // A press without a move is no drag.
    onAnchor.mockReset();
    pointer("pointerdown", 50);
    pointer("pointerup", 51);
    expect(onAnchor).not.toHaveBeenCalled();
  });

  it("keeps_the_letter_and_the_accessible_name_in_compact_density", () => {
    show({ density: "kompakt" });
    expect(tool("Figur").textContent).toBe("F");
    expect(tool("Figur").getAttribute("aria-label")).toBe("Figur");
    expect(bar().getAttribute("data-dichte")).toBe("kompakt");
  });

  it("shownAnchor_falls_back_to_the_top_in_a_low_view_460_px_in_komfort_and_400_in_kompakt", () => {
    const komfort = toolbarMinHeight("komfort");
    const kompakt = toolbarMinHeight("kompakt");
    expect([komfort, kompakt]).toEqual([460, 400]);
    // The seven sub-tools of „Gelände malen“ lengthen the upright bar by a separator and a key each.
    expect([toolbarMinHeight("komfort", 7), toolbarMinHeight("kompakt", 7)]).toEqual([460 + 5 + 7 * 44, 400 + 5 + 7 * 36]);
    expect(toolbarMinHeight("komfort", 0)).toBe(460);
    expect(isLowView(459, komfort)).toBe(true);
    expect(isLowView(460, komfort)).toBe(false);
    expect(isLowView(399, kompakt)).toBe(true);
    expect(isLowView(400, kompakt)).toBe(false);
    // Not measured yet: not low.
    expect(isLowView(0, komfort)).toBe(false);
    expect(shownAnchor("left", true)).toBe("top");
    expect(shownAnchor("left", false)).toBe("left");
    expect(shownAnchor("bottom", true)).toBe("bottom");
    expect(shownAnchor("top", true)).toBe("top");
  });

  it("anchorAt_splits_the_view_in_quarters", () => {
    expect(anchorAt(0, 400)).toBe("top");
    expect(anchorAt(99, 400)).toBe("top");
    expect(anchorAt(100, 400)).toBe("left");
    expect(anchorAt(300, 400)).toBe("left");
    expect(anchorAt(301, 400)).toBe("bottom");
    expect(anchorAt(-20, 400)).toBe("top");
    expect(anchorAt(900, 400)).toBe("bottom");
  });
});
