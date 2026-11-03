// @vitest-environment jsdom
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MenuEntry } from "./Menu";
import { pressKey, withoutFrames } from "./testUtils";
import { TreeRow } from "./TreeRow";
import { useRovingList } from "./useRovingList";

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

  describe("in a tree whose frames are late or never come", () => {
    // A busy page, or a hidden one (a background tab): Base UI moves the focus into the menu only on an animation frame.
    let frames = () => {};
    beforeEach(() => void (frames = withoutFrames()));
    afterEach(() => frames());

    /** A tree as the hosts build it (`WorldTree`, `SceneHierarchy`): a roving list whose arrows move between the rows. */
    function Tree({ menu, tick = 0 }: { menu: MenuEntry[]; tick?: number }) {
      const host = useRef<HTMLDivElement>(null);
      useRovingList(host);
      return (
        <div ref={host} role="tree" aria-label="Orte" data-tick={tick}>
          {["Jubelstadt 0003 · C01", "Zweiblattdorf 0412"].map((label) => (
            <TreeRow key={label} label={label} depth={0} selected={false} onSelect={() => onSelect(label)} contextMenu={menu} />
          ))}
        </div>
      );
    }
    const onSelect = vi.fn();
    const items = () => [...container.querySelectorAll<HTMLElement>("[role=treeitem]")];
    const twoEntries = (onReset: () => void): MenuEntry[] => [
      { label: "Header auf Original zurücksetzen…", onSelect: onReset },
      { label: "Zweiter", onSelect: () => {} },
    ];

    it("shift_f10_puts_the_focus_on_the_first_entry_so_the_keys_work_the_menu_not_the_tree", async () => {
      onSelect.mockReset();
      const onReset = vi.fn();
      act(() => root.render(<Tree menu={twoEntries(onReset)} />));
      act(() => items()[0]!.focus());
      pressKey(items()[0]!, "F10", { shiftKey: true });
      await act(async () => {});
      expect(document.activeElement).toBe(entries()[0]);
      // Arrow down walks the menu; the tree's tab stop stays on the row the menu belongs to.
      pressKey(entries()[0]!, "ArrowDown");
      expect(document.activeElement).toBe(entries()[1]);
      expect(items()[0]!.tabIndex).toBe(0);
      pressKey(entries()[1]!, "ArrowUp");
      pressKey(entries()[0]!, "Enter");
      expect(onReset).toHaveBeenCalledTimes(1);
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("a_render_of_the_tree_while_the_menu_is_open_leaves_the_focus_on_the_entry", async () => {
      act(() => root.render(<Tree menu={twoEntries(() => {})} />));
      act(() => items()[1]!.focus());
      pressKey(items()[1]!, "ContextMenu");
      await act(async () => {});
      pressKey(entries()[0]!, "ArrowDown");
      expect(document.activeElement).toBe(entries()[1]);
      // The hosts render their rows anew (a new menu array each time, a status that arrives): the focus stays where the keys put it.
      for (const tick of [1, 2]) act(() => root.render(<Tree menu={twoEntries(() => {})} tick={tick} />));
      await act(async () => {});
      expect(document.activeElement).toBe(entries()[1]);
    });

    it("with_every_entry_locked_the_first_one_holds_the_focus_and_enter_does_not_reach_the_row", async () => {
      onSelect.mockReset();
      const onReset = vi.fn();
      act(() => root.render(<Tree menu={[{ label: "Header auf Original zurücksetzen…", onSelect: onReset, disabled: true, detail: "Kein Original" }]} />));
      act(() => items()[0]!.focus());
      pressKey(items()[0]!, "F10", { shiftKey: true });
      await act(async () => {});
      expect(document.activeElement).toBe(entries()[0]);
      expect(entries()[0]!.getAttribute("aria-disabled")).toBe("true");
      pressKey(entries()[0]!, "Enter");
      expect(onReset).not.toHaveBeenCalled();
      expect(onSelect).not.toHaveBeenCalled();
    });

    it("a_right_click_puts_the_focus_on_the_menu_named_by_its_row", async () => {
      act(() => root.render(<Tree menu={twoEntries(() => {})} />));
      act(() => items()[0]!.focus());
      act(() => void items()[1]!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2, clientX: 10, clientY: 10 })));
      await act(async () => {});
      expect(document.activeElement).toBe(menu());
      expect(menu()?.getAttribute("aria-label")).toBe("Zweiblattdorf 0412");
      pressKey(menu() as HTMLElement, "ArrowDown");
      expect(document.activeElement).toBe(entries()[0]);
      expect(items()[0]!.tabIndex).toBe(0);
    });
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

describe("TreeRow tech", () => {
  it("keep_tech_makes_the_number_the_part_that_does_not_shrink", () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    const tech = () => [...container.querySelectorAll("span")].find((e) => e.textContent === "Trainer 12")!;
    act(() => root.render(<TreeRow label="Wanderer Jens" tech="Trainer 12" depth={1} selected={false} onSelect={() => {}} />));
    expect(tech().className).not.toMatch(/techKept/);
    act(() => root.render(<TreeRow label="Wanderer Jens" tech="Trainer 12" keepTech depth={1} selected={false} onSelect={() => {}} />));
    expect(tech().className).toMatch(/techKept/);
    act(() => root.unmount());
    container.remove();
  });
});
