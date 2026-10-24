// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FilterChips } from "./FilterChips";
import type { FilterChipOption } from "./FilterChips";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const typeOptions: FilterChipOption[] = ["Normal", "Kampf", "Flug", "Gift", "Boden", "Gestein", "Käfer", "Geist", "Stahl"].map((label, type) => ({
  value: String(type),
  label,
  type,
}));

describe("FilterChips", () => {
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

  const chips = (scope: ParentNode = container) => [...scope.querySelectorAll<HTMLButtonElement>("button[aria-pressed]")];
  const chip = (label: string, scope: ParentNode = container) => chips(scope).find((b) => b.textContent?.startsWith(label))!;
  const button = (text: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text);
  const press = (el: HTMLElement) =>
    act(() => {
      el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      el.click();
    });

  it("shows_the_first_chips_as_toggle_buttons_with_counts", () => {
    const options: FilterChipOption[] = [
      { value: "items", label: "Gegenstände", count: 162 },
      { value: "medicine", label: "Medizin", count: 38 },
    ];
    act(() => root.render(<FilterChips ariaLabel="Taschen" options={options} value={new Set(["medicine"])} onChange={() => {}} />));
    expect(container.querySelector("[role=group]")?.getAttribute("aria-label")).toBe("Taschen");
    expect(chips().map((c) => c.textContent)).toEqual(["Gegenstände162", "Medizin38"]);
    expect(chip("Gegenstände").getAttribute("aria-pressed")).toBe("false");
    expect(chip("Medizin").getAttribute("aria-pressed")).toBe("true");
  });

  it("a_click_adds_or_removes_the_option", () => {
    const onChange = vi.fn();
    act(() => root.render(<FilterChips ariaLabel="Typ" options={typeOptions} value={new Set(["2"])} onChange={onChange} />));
    act(() => chip("Normal").click());
    expect([...onChange.mock.lastCall![0]].sort()).toEqual(["0", "2"]);
    act(() => chip("Flug").click());
    expect([...onChange.mock.lastCall![0]]).toEqual([]);
  });

  it("puts_the_rest_behind_n_more_in_a_popover", () => {
    const onChange = vi.fn();
    act(() => root.render(<FilterChips ariaLabel="Typ" options={typeOptions} value={new Set()} onChange={onChange} />));
    expect(chips().map((c) => c.textContent)).toEqual(["Normal", "Kampf", "Flug", "Gift", "Boden", "Gestein"]);
    const more = button("3 weitere")!;
    expect(more).toBeDefined();
    press(more);
    const popup = document.querySelector<HTMLElement>("[role=dialog]")!;
    expect(chips(popup).map((c) => c.textContent)).toEqual(["Käfer", "Geist", "Stahl"]);
    act(() => chip("Geist", popup).click());
    expect([...onChange.mock.lastCall![0]]).toEqual(["7"]);
  });

  it("keeps_an_active_option_from_the_rest_in_the_row", () => {
    act(() => root.render(<FilterChips ariaLabel="Typ" options={typeOptions} value={new Set(["8"])} onChange={() => {}} visible={2} />));
    expect(chips().map((c) => c.textContent)).toEqual(["Normal", "Kampf", "Stahl"]);
    expect(button("6 weitere")).toBeDefined();
  });

  it("filter_loesen_clears_the_selection_and_shows_only_when_something_is_chosen", () => {
    const onChange = vi.fn();
    act(() => root.render(<FilterChips ariaLabel="Typ" options={typeOptions} value={new Set()} onChange={onChange} />));
    expect(button("Filter lösen")).toBeUndefined();
    act(() => root.render(<FilterChips ariaLabel="Typ" options={typeOptions} value={new Set(["2", "0"])} onChange={onChange} />));
    act(() => button("Filter lösen")!.click());
    expect([...onChange.mock.lastCall![0]]).toEqual([]);
  });
});
