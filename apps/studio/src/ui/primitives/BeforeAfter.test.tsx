// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../cssSource";
import { BeforeAfter } from "./BeforeAfter";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("BeforeAfter", () => {
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
  const column = (name: string) => container.querySelector<HTMLElement>(`[role=group][aria-label="${name}"]`)!;
  const lines = (name: string) => [...column(name).querySelectorAll<HTMLElement>("[data-line]")].map((l) => [l.textContent, l.hasAttribute("data-changed")]);

  it("shows_both_sides_in_two_columns_and_marks_the_lines_that_changed", () => {
    act(() => root.render(<BeforeAfter label="Textarchiv 0433" before={"Hallo,\ndas ist eine viel zu lange Zeile\nTschüss"} after={"Hallo,\ndas ist eine viel zu\nlange Zeile\nTschüss"} />));
    expect(container.querySelector("[role=group][aria-label='Textarchiv 0433']")).not.toBeNull();
    expect(column("Vorher").textContent).toContain("Vorher");
    expect(lines("Vorher")).toEqual([
      ["Hallo,", false],
      ["das ist eine viel zu lange Zeile", true],
      ["Tschüss", false],
    ]);
    expect(lines("Nachher")).toEqual([
      ["Hallo,", false],
      ["das ist eine viel zu", true],
      ["lange Zeile", true],
      ["Tschüss", false],
    ]);
    // Never colour alone: what went is a deletion, what came an insertion, each with its sign.
    expect(column("Vorher").querySelectorAll("del")).toHaveLength(1);
    expect(column("Nachher").querySelectorAll("ins")).toHaveLength(2);
  });

  it("says_leer_for_an_empty_side", () => {
    act(() => root.render(<BeforeAfter before="" after={"script script_7 #7:\n\tEnd"} />));
    expect(column("Vorher").querySelectorAll("[data-line]")).toHaveLength(0);
    expect(column("Vorher").textContent).toContain("Leer");
    expect(lines("Nachher").every(([, changed]) => changed)).toBe(true);
    expect(container.querySelector("[role=group][aria-label='Vorher und Nachher']")).not.toBeNull();
  });

  it("never_wraps_a_line_so_a_line_break_shows_as_one_and_the_column_scrolls_sideways", () => {
    // A 60-character text line in a 215 px column: one line that scrolls, not four rows that look like four lines.
    const css = readCss("./BeforeAfter.module.css", import.meta.url);
    expect(ruleBody(css, ".line")).toMatch(/white-space:\s*pre;/);
    expect(ruleBody(css, ".line")).not.toMatch(/overflow-wrap|word-break/);
    // As wide as its longest line, at least the column: the changed surface reaches across while scrolling.
    expect(ruleBody(css, ".line")).toMatch(/width:\s*max-content/);
    expect(ruleBody(css, ".line")).toMatch(/min-width:\s*100%/);
    expect(ruleBody(css, ".lines")).toMatch(/overflow:\s*auto/);
  });

  it("wraps_long_lines_on_request_with_a_hanging_indent", () => {
    act(() => root.render(<BeforeAfter before="a" after="b" />));
    expect(container.querySelector("[data-wrap]")).toBeNull();
    act(() => root.render(<BeforeAfter before={"SPIELER: {STRVAR_1, 3, 0, 0} Studio"} after="SPIELER:" wrap />));
    expect(container.querySelector("[role=group][aria-label='Vorher und Nachher']")?.hasAttribute("data-wrap")).toBe(true);
    // A long line breaks inside its column (no sideways scrolling); its continuation rows hang in, so it still reads as one line.
    const css = readCss("./BeforeAfter.module.css", import.meta.url);
    const line = ruleBody(css, ".wrap .line");
    expect(line).toMatch(/white-space:\s*pre-wrap;/);
    expect(line).toMatch(/overflow-wrap:\s*anywhere;/);
    expect(line).toMatch(/width:\s*auto;/);
    expect(ruleBody(css, ".wrap .text")).toMatch(/text-indent:\s*calc\(-1 \* var\(--g4-raum-16\)\)/);
  });

  it("keeps_the_signs_in_view_while_a_column_scrolls_sideways", () => {
    const css = readCss("./BeforeAfter.module.css", import.meta.url);
    // The sign column stays at the left edge of its column; the text scrolls under it, so the gutter is opaque, in the line's surface.
    const sign = ruleBody(css, ".line::before");
    expect(sign).toMatch(/position:\s*sticky;/);
    expect(sign).toMatch(/left:\s*0;/);
    expect(sign).toMatch(/background:\s*var\(--g4-flaeche-blatt\)/);
    // The app's reset makes every ::before border-box: the sign keeps its 1ch beside the inset and the gap, else it sits on the text.
    expect(sign).toMatch(/box-sizing:\s*content-box;/);
    expect(ruleBody(css, ".changed::before")).toMatch(/background:\s*var\(--g4-auswahl-flaeche\)/);
  });

  it("highlights_changed_lines_in_the_selection_surface", () => {
    const css = readCss("./BeforeAfter.module.css", import.meta.url);
    expect(ruleBody(css, ".changed")).toMatch(/background:\s*var\(--g4-auswahl-flaeche\)/);
  });

  describe("scrolls_each_column_to_its_first_changed_line", () => {
    // jsdom has no layout: every line is 20 px high, a column shows 100 px of 50 lines.
    const text = (n: number, changedAt: number, word: string) => Array.from({ length: n }, (_, i) => (i === changedAt ? word : `line ${i}`)).join("\n");
    const proto = HTMLElement.prototype;
    const saved = ["offsetTop", "offsetHeight", "clientHeight"].map((k) => [k, Object.getOwnPropertyDescriptor(proto, k)] as const);
    const computed = window.getComputedStyle;
    beforeEach(() => {
      window.getComputedStyle = ((el: Element) => ({ ...computed(el), lineHeight: "18px" })) as typeof window.getComputedStyle;
      Object.defineProperty(proto, "offsetTop", {
        configurable: true,
        get(this: HTMLElement) {
          return this.hasAttribute("data-line") ? 4 + 20 * [...this.parentElement!.children].indexOf(this) : 0;
        },
      });
      Object.defineProperty(proto, "offsetHeight", { configurable: true, get: () => 20 });
      Object.defineProperty(proto, "clientHeight", { configurable: true, get: () => 100 });
    });
    afterEach(() => {
      window.getComputedStyle = computed;
      for (const [k, d] of saved) d ? Object.defineProperty(proto, k, d) : delete (proto as unknown as Record<string, unknown>)[k];
    });
    const top = (name: string) => column(name).querySelector<HTMLElement>("ol")!.scrollTop;

    it("the_offset_does_not_depend_on_the_height_of_the_line_before_a_wrapped_event", () => {
      // The line before the change is a tall wrapped line (220 px): the change still starts two lines below the top.
      const offsetTop = Object.getOwnPropertyDescriptor(proto, "offsetTop")!.get!;
      Object.defineProperty(proto, "offsetTop", {
        configurable: true,
        get(this: HTMLElement) {
          const i = this.hasAttribute("data-line") ? [...this.parentElement!.children].indexOf(this) : 0;
          return offsetTop.call(this) + (i >= 30 ? 200 : 0);
        },
      });
      act(() => root.render(<BeforeAfter before={text(50, 30, "old")} after={text(50, 30, "new")} wrap />));
      expect(top("Vorher")).toBe(604 + 200 - 36);
    });

    it("a_change_far_down_is_brought_into_view_two_visual_lines_below_the_top", () => {
      act(() => root.render(<BeforeAfter before={text(50, 30, "old")} after={text(50, 30, "new")} wrap />));
      // Line 30 sits at 4 + 600; it lands two visual lines (2 * 18 px, the computed line height) below the column's top.
      expect(top("Vorher")).toBe(604 - 36);
      expect(top("Nachher")).toBe(604 - 36);
    });

    it("a_change_on_top_leaves_the_column_at_0", () => {
      act(() => root.render(<BeforeAfter before={text(50, 1, "old")} after={text(50, 1, "new")} wrap />));
      expect(top("Vorher")).toBe(0);
      expect(top("Nachher")).toBe(0);
    });

    it("moves_again_when_the_text_changes_and_never_scrolls_the_page", () => {
      let scrolled = 0;
      const original = Element.prototype.scrollIntoView;
      Element.prototype.scrollIntoView = () => void scrolled++;
      act(() => root.render(<BeforeAfter before={text(50, 30, "old")} after={text(50, 30, "new")} />));
      act(() => root.render(<BeforeAfter before={text(50, 10, "old")} after={text(50, 10, "new")} />));
      expect(top("Vorher")).toBe(4 + 20 * 10 - 36);
      act(() => root.render(<BeforeAfter before={text(50, 0, "old")} after={text(50, 0, "new")} />));
      expect(top("Vorher")).toBe(0);
      expect(scrolled).toBe(0);
      Element.prototype.scrollIntoView = original;
    });

    describe("while_the_layout_settles", () => {
      type Observer = { callback: () => void; observed: Element[]; disconnected: boolean };
      let observers: Observer[];
      const original = globalThis.ResizeObserver;
      beforeEach(() => {
        observers = [];
        globalThis.ResizeObserver = class {
          o: Observer;
          constructor(callback: () => void) {
            this.o = { callback, observed: [], disconnected: false };
            observers.push(this.o);
          }
          observe(el: Element) {
            this.o.observed.push(el);
          }
          disconnect() {
            this.o.disconnected = true;
          }
          unobserve() {}
        } as unknown as typeof ResizeObserver;
      });
      afterEach(() => {
        globalThis.ResizeObserver = original;
      });
      // A later layout: every line from the 11th on is lower by `extra` px (the lines above wrapped after the dialog reached its width).
      const settle = (extra: number) => {
        const base = Object.getOwnPropertyDescriptor(proto, "offsetTop")!.get!;
        Object.defineProperty(proto, "offsetTop", {
          configurable: true,
          get(this: HTMLElement) {
            return base.call(this) + (this.hasAttribute("data-line") && [...this.parentElement!.children].indexOf(this) >= 10 ? extra : 0);
          },
        });
      };

      it("follows_a_later_resize_of_the_column", () => {
        act(() => root.render(<BeforeAfter before={text(50, 30, "old")} after={text(50, 30, "new")} wrap />));
        expect(top("Vorher")).toBe(568);
        expect(observers.length).toBeGreaterThan(0);
        settle(200);
        act(() => observers.forEach((o) => o.callback()));
        expect(top("Vorher")).toBe(768);
        expect(top("Nachher")).toBe(768);
      });

      it("stops_following_after_the_user_scrolls_and_disconnects_on_unmount", () => {
        act(() => root.render(<BeforeAfter before={text(50, 30, "old")} after={text(50, 30, "new")} wrap />));
        const vorher = column("Vorher").querySelector<HTMLElement>("ol")!;
        act(() => void vorher.dispatchEvent(new Event("wheel")));
        const mine = observers.filter((o) => o.observed.includes(vorher));
        expect(mine.every((o) => o.disconnected)).toBe(true);
        vorher.scrollTop = 100;
        settle(200);
        act(() => observers.filter((o) => o.observed.includes(vorher)).forEach((o) => o.disconnected || o.callback()));
        expect(vorher.scrollTop).toBe(100);
        // The other column was not touched by the user: it still follows until unmount.
        expect(observers.some((o) => !o.disconnected)).toBe(true);
        act(() => root.unmount());
        expect(observers.every((o) => o.disconnected)).toBe(true);
        root = createRoot(container);
      });
    });
  });
});
