// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pressKey } from "./testUtils";
import { TreeRow } from "./TreeRow";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TreeRow context menu", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const item = () => container.querySelector<HTMLElement>("[role=treeitem]")!;
  const entries = () => [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
  const menu = () => document.querySelector("[role=menu]");

  it("context_menu_opens_on_right_click_and_shift_f10", () => {
    const onReset = vi.fn();
    const onToggle = vi.fn();
    const render = () =>
      root.render(
        <TreeRow
          label="Scriptdatei 0040"
          depth={1}
          expanded={false}
          onToggle={onToggle}
          selected={false}
          onSelect={() => {}}
          contextMenu={[{ label: "Auf Original zurücksetzen…", icon: "undo", onSelect: onReset }]}
        />,
      );
    act(render);
    // Still a tree item at its level, indented by its depth.
    expect(item().getAttribute("aria-level")).toBe("2");
    expect(item().getAttribute("aria-expanded")).toBe("false");
    expect(item().style.paddingLeft).toContain("var(--g4-raum-16)");
    act(() => void item().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2, clientX: 10, clientY: 10 })));
    expect(entries().map((e) => e.textContent)).toEqual(["Auf Original zurücksetzen…"]);
    act(() => entries()[0]!.click());
    expect(onReset).toHaveBeenCalledTimes(1);

    act(() => root.unmount());
    root = createRoot(container);
    act(render);
    expect(menu()).toBeNull();
    pressKey(item(), "F10", { shiftKey: true });
    expect(menu()).not.toBeNull();
    // Arrow right still opens the branch.
    pressKey(item(), "ArrowRight");
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("opened_by_shift_f10_it_focuses_the_first_entry_so_enter_picks_it", async () => {
    const onReset = vi.fn();
    act(() =>
      root.render(<TreeRow label="Scriptdatei 0040" depth={1} selected={false} onSelect={() => {}} contextMenu={[{ label: "Auf Original zurücksetzen…", icon: "undo", onSelect: onReset }]} />),
    );
    act(() => item().focus());
    pressKey(item(), "F10", { shiftKey: true });
    await act(async () => {});
    expect(document.activeElement).toBe(entries()[0]);
    pressKey(entries()[0]!, "Enter");
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("without_context_menu_nothing_changes", () => {
    act(() => root.render(<TreeRow label="Ort" depth={0} selected={false} onSelect={() => {}} />));
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2 });
    act(() => void item().dispatchEvent(event));
    expect(event.defaultPrevented).toBe(false);
    expect(pressKey(item(), "F10", { shiftKey: true }).defaultPrevented).toBe(false);
    expect(menu()).toBeNull();
    expect(container.firstElementChild?.getAttribute("role")).toBe("treeitem");
  });
});
