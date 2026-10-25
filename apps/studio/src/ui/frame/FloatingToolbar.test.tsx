// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TOOLS } from "../../workbench/tools";
import { pressKey } from "../primitives/testUtils";
import type { Density } from "../tokens/theme";
import { FloatingToolbar, TOOLBAR_MIN_HEIGHT, anchorAt, shownAnchor } from "./FloatingToolbar";
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

  const show = (opts: { tool?: Tool; eventsBlocked?: string | null; density?: Density; anchor?: ToolbarAnchor } = {}) =>
    act(() =>
      root.render(
        <FloatingToolbar
          tools={TOOLS}
          tool={opts.tool ?? "select"}
          onTool={onTool}
          eventsBlocked={opts.eventsBlocked ?? null}
          density={opts.density ?? "komfort"}
          anchor={opts.anchor ?? "left"}
          onAnchor={onAnchor}
          area={area}
        />,
      ),
    );
  const bar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Werkzeugleiste"]')!;
  const keys = () => [...container.querySelectorAll<HTMLButtonElement>("button[data-tool]")];
  const tool = (name: string) => keys().find((b) => b.getAttribute("aria-label") === name)!;
  const grip = () => container.querySelector<HTMLButtonElement>("button[data-toolbar-grip]")!;
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

  it("moves_to_the_nearest_anchor_after_a_drag", () => {
    show();
    expect(grip().getAttribute("aria-label")).toBe("Werkzeugleiste verschieben");
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

  it("shownAnchor_falls_back_to_the_top_below_400_px", () => {
    expect(TOOLBAR_MIN_HEIGHT).toBe(400);
    expect(shownAnchor("left", 399)).toBe("top");
    expect(shownAnchor("left", 400)).toBe("left");
    // Not measured yet: no fallback.
    expect(shownAnchor("left", 0)).toBe("left");
    expect(shownAnchor("bottom", 300)).toBe("bottom");
    expect(shownAnchor("top", 300)).toBe("top");
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
