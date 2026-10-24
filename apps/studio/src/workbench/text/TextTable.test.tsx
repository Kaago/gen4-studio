// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TextTable } from "./TextTable";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const COLUMNS = [{ title: "Nr." }, { title: "Text" }, { title: "Längste Zeile" }] as const;

describe("TextTable", () => {
  let container: HTMLElement;
  let root: Root;
  const onEnter = vi.fn();

  /** A table of `count` rows that selects what it is told to. */
  function Table({ count, initial = null }: { count: number; initial?: number | null }) {
    const [selected, setSelected] = useState<number | null>(initial);
    return (
      <TextTable
        label="Nachrichten"
        kind="messages"
        columns={COLUMNS}
        count={count}
        selected={selected}
        rowAttrs={(i) => ({ "data-message": i })}
        cells={(i) => [String(i).padStart(3, "0"), `Text ${i}`, "3 von 38"]}
        onSelect={setSelected}
        onEnter={onEnter}
      />
    );
  }

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onEnter.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const rows = () => [...container.querySelectorAll('[role="row"]')];
  const row = (i: number) => container.querySelector<HTMLElement>(`[data-message="${i}"]`);
  const scroller = () => container.querySelector<HTMLElement>("[data-text-table-body]")!;
  const key = (el: Element, k: string) => act(() => void el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })));

  it("scrolls a selection it mounts with into view with the sizes as laid out", () => {
    // jsdom has no layout: the body is 240 px high, a row 24 px (the compact density), as a browser would measure them.
    const restore: (() => void)[] = [];
    const stub = (name: "clientHeight" | "offsetHeight", get: (el: HTMLElement) => number) => {
      const before = Object.getOwnPropertyDescriptor(HTMLElement.prototype, name);
      Object.defineProperty(HTMLElement.prototype, name, { configurable: true, get(this: HTMLElement) { return get(this); } });
      restore.push(() => (before ? Object.defineProperty(HTMLElement.prototype, name, before) : delete (HTMLElement.prototype as unknown as Record<string, unknown>)[name]));
    };
    stub("clientHeight", (el) => (el.hasAttribute("data-text-table-body") ? 240 : 0));
    stub("offsetHeight", (el) => (el.hasAttribute("data-row") ? 24 : 0));
    try {
      act(() => root.render(<Table count={800} initial={400} />));
      // Row 400 ends at 401 × 24 px; the body shows 240 px of it.
      expect(scroller().scrollTop).toBe(401 * 24 - 240);
      expect(row(400)).not.toBeNull();
    } finally {
      while (restore.length > 0) restore.pop()!();
    }
  });

  it("is a grid with a header row and named columns", () => {
    act(() => root.render(<Table count={3} />));
    const grid = container.querySelector('[role="grid"]');
    expect(grid?.getAttribute("aria-label")).toBe("Nachrichten");
    expect(grid?.getAttribute("aria-rowcount")).toBe("4");
    expect([...container.querySelectorAll('[role="columnheader"]')].map((c) => c.textContent)).toEqual(["Nr.", "Text", "Längste Zeile"]);
    expect(rows()).toHaveLength(4);
  });

  it("renders only the rows in view and a buffer, at most 60 of 800", () => {
    act(() => root.render(<Table count={800} />));
    expect(rows().length).toBeLessThanOrEqual(60);
    expect(row(0)).not.toBeNull();
    expect(row(799)).toBeNull();
    // Scrolled down, the window follows.
    act(() => {
      scroller().scrollTop = 400 * 28;
      scroller().dispatchEvent(new Event("scroll"));
    });
    expect(rows().length).toBeLessThanOrEqual(60);
    expect(row(0)).toBeNull();
    expect(row(400)).not.toBeNull();
    expect(row(400)?.getAttribute("aria-rowindex")).toBe("402");
  });

  it("selects by click, moves the selection and the focus with the arrow keys and gives only that row the tab stop", () => {
    act(() => root.render(<Table count={5} />));
    // Nothing selected: the first row is the tab stop.
    expect(row(0)?.tabIndex).toBe(0);
    act(() => row(1)!.click());
    expect(row(1)?.getAttribute("aria-selected")).toBe("true");
    expect(row(1)?.tabIndex).toBe(0);
    expect(row(0)?.tabIndex).toBe(-1);
    key(row(1)!, "ArrowDown");
    expect(row(2)?.getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(row(2));
    key(row(2)!, "End");
    expect(document.activeElement).toBe(row(4));
    key(row(4)!, "Home");
    expect(row(0)?.getAttribute("aria-selected")).toBe("true");
    key(row(0)!, "Enter");
    expect(onEnter).toHaveBeenCalledWith(0);
  });

  it("selects a row that is not selected yet before Enter asks for the editor", () => {
    act(() => root.render(<Table count={5} initial={1} />));
    key(row(3)!, "Enter");
    expect(row(3)?.getAttribute("aria-selected")).toBe("true");
    expect(row(1)?.getAttribute("aria-selected")).toBe("false");
    expect(onEnter).toHaveBeenCalledWith(3);
  });

  it("measures the row height again when it changes (another density), so the window stays right", () => {
    let rowHeight = 32;
    const descriptor = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      get(this: HTMLElement) {
        return this.hasAttribute("data-row") ? rowHeight : 0;
      },
    });
    try {
      act(() => root.render(<Table count={800} />));
      act(() => {
        scroller().scrollTop = 400 * 32;
        scroller().dispatchEvent(new Event("scroll"));
      });
      expect(row(400)).not.toBeNull();
      // Compact rows: the same scroll position now lies further down the list.
      rowHeight = 24;
      act(() => root.render(<Table count={801} />));
      expect(row(533)).not.toBeNull();
      expect(row(400)).toBeNull();
    } finally {
      if (descriptor) Object.defineProperty(HTMLElement.prototype, "offsetHeight", descriptor);
    }
  });

  it("brings a selection far down into view and keeps it reachable with the keys", () => {
    act(() => root.render(<Table count={800} initial={700} />));
    expect(row(700)).not.toBeNull();
    expect(row(700)?.getAttribute("aria-selected")).toBe("true");
    key(row(700)!, "ArrowDown");
    expect(document.activeElement).toBe(row(701));
  });
});
