// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Picker } from "./Picker";
import type { PickerItem } from "./Picker";
import { readCss, ruleBody } from "../cssSource";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const items: PickerItem[] = [
  { id: 410, label: "Jubelstadt", tech: "T01R0201", area: "welt", group: "Städte", count: "3 Räume" },
  { id: 411, label: "Zweiblattdorf", tech: "T01R0101", area: "welt", group: "Städte", preview: <i data-preview="411" /> },
  { id: 412, label: "Sandgemme", tech: "T01R0102", area: "welt", group: "Städte" },
  { id: 420, label: "Route 201", area: "welt", group: "Routen" },
];

describe("Picker", () => {
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

  const input = () => document.querySelector<HTMLInputElement>("input[role=combobox]")!;
  /**
   * What a user does: the search field has focus (the dialog moves it there on open) and receives the text as typed
   * input. Base UI highlights the first match only for typed input, so the event carries an `inputType`.
   */
  const type = (text: string) => {
    const el = input();
    act(() => {
      el.focus();
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(el, text);
      el.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: text }));
    });
  };
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  const render = (props: Partial<Parameters<typeof Picker>[0]> = {}) => {
    const onPick = vi.fn();
    const onOpenChange = vi.fn();
    act(() => root.render(<Picker open onOpenChange={onOpenChange} title="Header wählen" items={items} onPick={onPick} {...props} />));
    return { onPick, onOpenChange };
  };

  it("lists_every_item_under_its_visible_group_label", () => {
    render();
    expect(options()).toHaveLength(4);
    const dialog = document.querySelector("[role=dialog]")!;
    expect(dialog.textContent).toContain("Städte");
    expect(dialog.textContent).toContain("Routen");
    expect(dialog.querySelectorAll("[role=group]")).toHaveLength(2);
  });

  it("renders_the_preview_and_the_count_in_the_row", () => {
    render();
    expect(options()[1]!.querySelector("[data-preview='411']")).not.toBeNull();
    expect(options()[0]!.textContent).toContain("3 Räume");
    expect(options()[0]!.textContent).toContain("T01R0201");
  });

  it("the_count_title_is_the_title_of_the_count", () => {
    render({ items: [{ id: 1, label: "Eins", count: "3 · 1", countTitle: "gesetzt 3 · geprüft 1" }, { id: 2, label: "Zwei", count: "7" }] });
    const counts = [...document.querySelectorAll<HTMLElement>("[role=option] span")].filter((s) => s.textContent === "3 · 1" || s.textContent === "7");
    expect(counts[0]!.title).toBe("gesetzt 3 · geprüft 1");
    expect(counts[1]!.hasAttribute("title")).toBe(false);
  });

  it("the_cut_note_shows_only_while_the_limit_cuts_hits_and_gets_the_numbers", () => {
    const cutNote = vi.fn((shown: number, total: number) => `${shown} von ${total}`);
    render({ limitPerGroup: 2, cutNote });
    // 3 in „Städte“ cut to 2, 1 in „Routen“: 3 of 4 rows listed.
    expect(document.querySelector("[role=dialog]")!.textContent).toContain("3 von 4");
    type("t01r01");
    expect(document.querySelector("[role=dialog]")!.textContent).not.toContain("von");
    act(() => root.render(<Picker open onOpenChange={vi.fn()} title="Header wählen" items={items} onPick={vi.fn()} cutNote={cutNote} />));
    expect(document.querySelector("[role=dialog]")!.textContent).not.toContain("von");
  });

  it("typing_narrows_the_list_immediately_and_highlights_the_first_match", () => {
    render();
    type("T01R01");
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining("Zweiblattdorf"), expect.stringContaining("Sandgemme")]);
    expect(options()[0]!.hasAttribute("data-highlighted")).toBe(true);
    expect(options()[1]!.hasAttribute("data-highlighted")).toBe(false);
  });

  it("a_numeric_query_finds_the_item_by_id_with_leading_zeros", () => {
    render();
    type("0412");
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining("Sandgemme")]);
  });

  it("arrow_down_moves_the_highlight_and_enter_picks_it_and_closes", () => {
    const { onPick, onOpenChange } = render();
    type("T01R01");
    pressKey(input(), "ArrowDown");
    expect(options()[1]!.hasAttribute("data-highlighted")).toBe(true);
    pressKey(input(), "Enter");
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick.mock.calls[0]![0]).toMatchObject({ id: 412, label: "Sandgemme" });
    expect(onOpenChange.mock.calls.map((c) => c[0])).toContain(false);
  });

  it("enter_right_after_typing_picks_the_first_match", () => {
    const { onPick } = render();
    type("zwei");
    pressKey(input(), "Enter");
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick.mock.calls[0]![0]).toMatchObject({ id: 411 });
  });

  it("clicking_a_row_picks_it", () => {
    const { onPick } = render();
    act(() => options()[3]!.click());
    expect(onPick.mock.calls[0]![0]).toMatchObject({ id: 420 });
  });

  it("escape_closes_without_picking", () => {
    const { onPick, onOpenChange } = render();
    type("zwei");
    pressKey(input(), "Escape");
    expect(onPick).not.toHaveBeenCalled();
    expect(onOpenChange.mock.calls.map((c) => c[0])).toEqual([false]);
  });

  it("shows_the_empty_text_when_nothing_matches", () => {
    render({ emptyText: "Kein Header passt." });
    type("qqq");
    expect(options()).toHaveLength(0);
    expect(document.querySelector("[role=dialog]")!.textContent).toContain("Kein Header passt.");
  });

  it("does_not_show_the_empty_text_while_there_are_matches", () => {
    render({ emptyText: "Kein Header passt." });
    expect(document.querySelector("[role=dialog]")!.textContent).not.toContain("Kein Header passt.");
  });

  it("renders_the_footer_below_the_list", () => {
    render({ footer: <span>Neu anlegen im Header-Editor</span> });
    expect(document.querySelector("[role=dialog]")!.textContent).toContain("Neu anlegen im Header-Editor");
  });

  it("a_strong_item_shows_its_label_emphasised", () => {
    render({ items: [{ id: 1, label: "FLAG_HAT_MEDAILLE", strong: true, description: "FLAG_UNUSED_0x0094" }, { id: 2, label: "FLAG_UNUSED_0x0096" }] });
    expect(options()[0]!.querySelector("[class*=strong]")?.textContent).toBe("FLAG_HAT_MEDAILLE");
    expect(options()[1]!.querySelector("[class*=strong]")).toBeNull();
    // The emphasis is the token weight, not a colour.
    expect(ruleBody(readCss("./Picker.module.css", import.meta.url), ".strong")).toMatch(/font:\s*var\(--g4-font-text-s-betont\)/);
  });

  it("filters_are_a_segmented_choice_the_owner_applies", () => {
    function Host() {
      const [filter, setFilter] = useState("alle");
      const shown = filter === "alle" ? items : items.filter((i) => i.group === "Routen");
      return (
        <Picker
          open
          onOpenChange={() => {}}
          title="Header wählen"
          items={shown}
          onPick={() => {}}
          filters={[
            { value: "alle", label: "Alle" },
            { value: "routen", label: "Routen" },
          ]}
          filter={filter}
          onFilter={setFilter}
        />
      );
    }
    act(() => root.render(<Host />));
    expect(options()).toHaveLength(4);
    const routen = [...document.querySelectorAll<HTMLElement>("button")].find((b) => b.textContent === "Routen")!;
    act(() => routen.click());
    expect(options()).toHaveLength(1);
  });

  it("a_reopened_picker_starts_with_an_empty_search", () => {
    render();
    type("zwei");
    expect(options()).toHaveLength(1);
    act(() => root.render(<Picker open={false} onOpenChange={() => {}} title="Header wählen" items={items} onPick={() => {}} />));
    act(() => root.render(<Picker open onOpenChange={() => {}} title="Header wählen" items={items} onPick={() => {}} />));
    expect(input().value).toBe("");
    expect(options()).toHaveLength(4);
  });

  it("closed_renders_nothing", () => {
    render({ open: false });
    expect(document.querySelector("[role=dialog]")).toBeNull();
  });

  describe("palette variant", () => {
    const paletteItems: PickerItem[] = [
      { id: "cmd-open", label: "Projekt öffnen", group: "Befehle", icon: "open", shortcut: "⌘O" },
      { id: "cmd-save", label: "Speichern", group: "Befehle", icon: "save", shortcut: "⌘S", description: "Schreibt alle Änderungen" },
      { id: "cmd-build", label: "Bauen", group: "Befehle", icon: "build" },
      { id: 411, label: "Zweiblattdorf", tech: "T01R0101", group: "Orte", area: "welt" },
      { id: 512, label: "Sandgemme", tech: "T01R0102", group: "Orte", area: "welt" },
      { id: 513, label: "Jubelstadt", tech: "T01R0201", group: "Orte", area: "welt" },
      { id: "text-412", label: "Zweiblattdorf", tech: "0412", group: "Textarchive", area: "texte" },
    ];
    const renderPalette = (props: Partial<Parameters<typeof Picker>[0]> = {}) => render({ variant: "palette", items: paletteItems, title: "Befehlspalette", ...props });
    const labels = () => options().map((o) => o.textContent ?? "");
    const tick = () =>
      act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

    it("limit_per_group_shows_that_many_per_group_and_counts_all_hits_in_the_footer", () => {
      renderPalette({ limitPerGroup: 2 });
      expect(options()).toHaveLength(5);
      expect(labels().filter((t) => t.includes("Bauen"))).toHaveLength(0);
      expect(labels().filter((t) => t.includes("Jubelstadt"))).toHaveLength(0);
      expect(document.querySelector("[role=dialog]")!.textContent).toContain("7 Treffer");
    });

    it("the_limit_applies_after_filtering", () => {
      renderPalette({ limitPerGroup: 1 });
      type("t01r");
      // Three places match; one row is shown, the count is the hits before cutting.
      expect(options()).toHaveLength(1);
      expect(document.querySelector("[role=dialog]")!.textContent).toContain("3 Treffer");
    });

    it("empty_query_groups_show_only_those_groups_until_something_is_typed", () => {
      renderPalette({ emptyQueryGroups: ["Befehle"] });
      expect(labels().map((t) => t.includes("Zweiblattdorf"))).not.toContain(true);
      expect(options()).toHaveLength(3);
      type("sand");
      expect(labels()).toEqual([expect.stringContaining("Sandgemme")]);
    });

    it("group_order_sorts_the_groups_and_keeps_the_rest_after_them", () => {
      renderPalette({ groupOrder: ["Textarchive", "Orte"] });
      const text = [...document.querySelectorAll("[role=group]")].map((g) => g.textContent ?? "");
      expect(text).toHaveLength(3);
      expect(text[0]).toContain("Textarchive");
      expect(text[1]).toContain("Orte");
      expect(text[2]).toContain("Befehle");
    });

    it("a_number_finds_the_item_by_its_tech_prefix", () => {
      renderPalette();
      type("0412");
      expect(labels()).toEqual([expect.stringContaining("Zweiblattdorf")]);
      expect(options()[0]!.textContent).toContain("0412");
    });

    it("a_query_also_matches_the_description", () => {
      renderPalette();
      type("änderungen");
      expect(labels()).toEqual([expect.stringContaining("Speichern")]);
    });

    it("shows_the_shortcut_the_description_and_marks_the_match", () => {
      renderPalette();
      expect(options()[1]!.querySelector("kbd")!.textContent).toBe("⌘S");
      expect(options()[1]!.textContent).toContain("Schreibt alle Änderungen");
      type("speich");
      const mark = options()[0]!.querySelector("mark");
      expect(mark).not.toBeNull();
      expect(mark!.textContent!.toLowerCase()).toBe("speich");
    });

    it("hides_the_close_button_and_names_the_dialog_for_screen_readers", () => {
      renderPalette();
      expect(document.querySelector("[aria-label=Schließen]")).toBeNull();
      expect(document.querySelector("[role=dialog]")!.getAttribute("aria-labelledby")).not.toBeNull();
      expect(document.querySelector("[role=dialog]")!.textContent).toContain("Befehlspalette");
    });

    it("shows_the_key_hint_in_the_footer", () => {
      renderPalette();
      expect(document.querySelector("[role=dialog]")!.textContent).toContain("↑↓ wählen, ↵ ausführen");
    });

    it("on_pick_comes_after_the_close_request_on_the_next_tick", async () => {
      const order: string[] = [];
      const { onPick, onOpenChange } = renderPalette();
      onOpenChange.mockImplementation((open: boolean) => order.push(open ? "open" : "close"));
      onPick.mockImplementation(() => order.push("pick"));
      type("sand");
      pressKey(input(), "Enter");
      expect(onOpenChange).toHaveBeenCalledWith(false);
      expect(onPick).not.toHaveBeenCalled();
      await tick();
      expect(onPick).toHaveBeenCalledTimes(1);
      expect(onPick.mock.calls[0]![0]).toMatchObject({ id: 512 });
      // The owner may be asked to close more than once; what counts is that the pick comes last.
      expect(order[0]).toBe("close");
      expect(order[order.length - 1]).toBe("pick");
      expect(order.filter((o) => o === "pick")).toHaveLength(1);
    });

    it("two_enters_in_a_row_pick_only_once_per_opening", async () => {
      const { onPick } = renderPalette();
      type("sand");
      pressKey(input(), "Enter");
      pressKey(input(), "Enter");
      await tick();
      pressKey(input(), "Enter");
      await tick();
      expect(onPick).toHaveBeenCalledTimes(1);
    });

    it("a_reopened_palette_picks_again", async () => {
      const { onPick } = renderPalette();
      type("sand");
      pressKey(input(), "Enter");
      await tick();
      act(() => root.render(<Picker open={false} onOpenChange={() => {}} title="Befehlspalette" variant="palette" items={paletteItems} onPick={onPick} />));
      act(() => root.render(<Picker open onOpenChange={() => {}} title="Befehlspalette" variant="palette" items={paletteItems} onPick={onPick} />));
      type("sand");
      pressKey(input(), "Enter");
      await tick();
      expect(onPick).toHaveBeenCalledTimes(2);
    });

    it("a_palette_reopened_while_it_still_closes_starts_with_an_empty_search_and_picks_again", async () => {
      // Base UI keeps the popup (and the body in it) mounted until its exit animation has finished. An animation that never
      // finishes stands for reopening the palette within that time.
      const proto = Element.prototype;
      const own = Object.getOwnPropertyDescriptor(proto, "getAnimations");
      Object.defineProperty(proto, "getAnimations", { configurable: true, writable: true, value: () => [{ finished: new Promise(() => {}) }] });
      try {
        const { onPick } = renderPalette();
        type("sand");
        pressKey(input(), "Enter");
        await tick();
        expect(onPick).toHaveBeenCalledTimes(1);
        const before = input();
        act(() => root.render(<Picker open={false} onOpenChange={() => {}} title="Befehlspalette" variant="palette" items={paletteItems} onPick={onPick} />));
        // Still closing: the same search field is there.
        expect(input()).toBe(before);
        act(() => root.render(<Picker open onOpenChange={() => {}} title="Befehlspalette" variant="palette" items={paletteItems} onPick={onPick} />));
        expect(input().value).toBe("");
        expect(labels().filter((t) => t.includes("Sandgemme"))).toHaveLength(1);
        type("sand");
        pressKey(input(), "Enter");
        await tick();
        expect(onPick).toHaveBeenCalledTimes(2);
      } finally {
        if (own) Object.defineProperty(proto, "getAnimations", own);
        else Reflect.deleteProperty(proto, "getAnimations");
      }
    });

    it("a_disabled_item_is_listed_and_marked_but_never_picked", async () => {
      const locked: PickerItem[] = [
        { id: "cmd-area", label: "Gebiet anlegen…", group: "Befehle", icon: "plus", disabled: true, description: "Erst ein Projekt öffnen" },
        { id: "cmd-open", label: "Projekt öffnen", group: "Befehle", icon: "open" },
      ];
      const { onPick } = renderPalette({ items: locked });
      const lockedRow = () => options().find((o) => o.textContent?.includes("Gebiet anlegen…"))!;
      expect(lockedRow().getAttribute("aria-disabled")).toBe("true");
      expect(lockedRow().textContent).toContain("Erst ein Projekt öffnen");
      type("gebiet");
      pressKey(input(), "Enter");
      // The filtered list renders its rows anew: click the row that is shown now.
      const row = lockedRow();
      expect(row.getAttribute("aria-disabled")).toBe("true");
      act(() => row.click());
      await tick();
      expect(onPick).not.toHaveBeenCalled();
      type("projekt");
      pressKey(input(), "Enter");
      await tick();
      expect(onPick).toHaveBeenCalledTimes(1);
      expect(onPick.mock.calls[0]![0]).toMatchObject({ id: "cmd-open" });
    });

    it("the_default_dialog_variant_still_picks_synchronously", () => {
      const { onPick } = render();
      type("zwei");
      pressKey(input(), "Enter");
      expect(onPick).toHaveBeenCalledTimes(1);
    });
  });
});
