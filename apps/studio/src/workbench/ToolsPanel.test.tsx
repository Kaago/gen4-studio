// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Tool } from "../ui/frame/shellState";
import type { Density } from "../ui/tokens/theme";
import { pressKey } from "../ui/primitives/testUtils";
import { ToolsPanel } from "./ToolsPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ToolsPanel", () => {
  let container: HTMLElement;
  let root: Root;
  const onTool = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onTool.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (tool: Tool = "select", eventsBlocked: string | null = null, density: Density = "komfort") =>
    act(() => root.render(<ToolsPanel tool={tool} onTool={onTool} eventsBlocked={eventsBlocked} density={density} />));
  const buttons = () => [...container.querySelectorAll<HTMLButtonElement>("button")];
  const tool = (name: string) => buttons().find((b) => b.getAttribute("aria-label") === name)!;

  it("shows_seven_keys_with_name_and_letter_and_exactly_the_active_one_pressed", () => {
    show("paint");
    expect(buttons().map((b) => b.getAttribute("aria-label"))).toEqual(["Auswählen", "Gelände malen", "Figur", "Tür", "Auslöser", "Schild", "Gebäude"]);
    expect(buttons().map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "true", "false", "false", "false", "false", "false"]);
    expect(tool("Figur").textContent).toBe("FigurF");
    expect(tool("Gelände malen").getAttribute("aria-keyshortcuts")).toBe("B");
    expect(container.querySelector('[role="toolbar"]')?.getAttribute("aria-label")).toBe("Werkzeugleiste");
  });

  it("switches_the_tool_on_a_click", () => {
    show();
    act(() => tool("Tür").click());
    expect(onTool).toHaveBeenCalledWith("door");
  });

  it("says_under_the_keys_what_the_active_tool_does", () => {
    show("select");
    expect(container.textContent).toContain("Malt nie.");
    show("paint");
    expect(container.textContent).toContain("Umschalt-Klick löscht sie.");
    expect(container.textContent).not.toContain("Malt nie.");
    show("building");
    expect(container.textContent).toContain("setzt dort ein Gebäude");
  });

  it("locks_the_four_event_tools_with_the_reason_and_leaves_the_others_alone", () => {
    const reason = "schreibgeschützt: 17 bytes read";
    show("select", reason);
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

  it("leaves_the_event_tools_open_when_nothing_blocks_them", () => {
    show("select", null);
    expect(buttons().some((b) => b.getAttribute("aria-disabled") === "true")).toBe(false);
  });

  it("shows_the_symbol_only_in_compact_density_and_keeps_the_name_as_the_accessible_name", () => {
    show("select", null, "kompakt");
    expect(tool("Figur").textContent).toBe("F");
    expect(tool("Figur").querySelector("svg")).not.toBeNull();
    expect(container.textContent).not.toContain("Gelände malen");
  });

  it("moves_between_the_keys_with_the_arrow_keys_and_keeps_one_in_the_tab_order", () => {
    show();
    const [first, second] = buttons();
    expect(buttons().filter((b) => b.tabIndex === 0)).toEqual([first]);
    act(() => first!.focus());
    pressKey(first!, "ArrowRight");
    expect(document.activeElement).toBe(second);
    expect(buttons().filter((b) => b.tabIndex === 0)).toEqual([second]);
  });
});
