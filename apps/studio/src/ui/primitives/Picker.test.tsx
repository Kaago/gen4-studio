// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Picker } from "./Picker";
import type { PickerItem } from "./Picker";
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
});
