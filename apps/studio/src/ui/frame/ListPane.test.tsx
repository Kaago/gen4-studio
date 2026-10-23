// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ListPane } from "./ListPane";
import type { ListPaneProps } from "./ListPane";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ListPane", () => {
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

  const props = (over: Partial<ListPaneProps> = {}): ListPaneProps => ({ title: "Orte", open: true, onToggle: vi.fn(), children: <p>Inhalt</p>, ...over });
  const show = (p: ListPaneProps) => act(() => root.render(<ListPane {...p} />));
  const grip = () => container.querySelector<HTMLButtonElement>("button[aria-expanded]")!;
  const fire = (el: HTMLElement, type: string, detail: number) => act(() => void el.dispatchEvent(new MouseEvent(type, { bubbles: true, detail })));

  it("shows_the_head_with_title_subtitle_and_actions_above_the_content", () => {
    show(props({ subtitle: "Platin, DE", actions: <button type="button">Neu</button> }));
    expect(container.querySelector("h2")?.textContent).toBe("Orte");
    expect(container.querySelector("section")?.getAttribute("aria-label")).toBe("Orte");
    expect(container.textContent).toContain("Platin, DE");
    expect([...container.querySelectorAll("button")].map((b) => b.textContent)).toContain("Neu");
    expect(container.textContent).toContain("Inhalt");
  });

  it("folds_on_a_double_click_of_the_grip_and_not_on_a_single_click", () => {
    const p = props();
    show(p);
    expect(grip().getAttribute("aria-label")).toBe("Liste einklappen");
    expect(grip().getAttribute("aria-expanded")).toBe("true");
    fire(grip(), "mousedown", 1);
    fire(grip(), "click", 1);
    expect(p.onToggle).not.toHaveBeenCalled();
    fire(grip(), "dblclick", 2);
    expect(p.onToggle).toHaveBeenCalledTimes(1);
  });

  it("folds_from_the_keyboard_through_the_grip_button", () => {
    const p = props();
    show(p);
    // A click without a pointer (Enter or Space on the focused button) has no click count.
    act(() => grip().click());
    expect(p.onToggle).toHaveBeenCalledTimes(1);
  });

  it("keeps_the_content_mounted_but_hidden_while_folded_and_opens_on_one_click", () => {
    const p = props({ open: false });
    show(p);
    expect(grip().getAttribute("aria-label")).toBe("Liste ausklappen");
    expect(grip().getAttribute("aria-expanded")).toBe("false");
    const body = container.querySelector("section")!;
    expect(body.hidden).toBe(true);
    expect(body.textContent).toContain("Inhalt");
    fire(grip(), "mousedown", 1);
    fire(grip(), "click", 1);
    expect(p.onToggle).toHaveBeenCalledTimes(1);
  });

  it("does_not_fold_again_when_the_double_click_that_opened_the_list_ends", () => {
    const onToggle = vi.fn();
    show(props({ open: false, onToggle }));
    fire(grip(), "mousedown", 1);
    fire(grip(), "click", 1);
    // The owner opens the list on the first click; the second press, click and the double click follow at once.
    show(props({ open: true, onToggle }));
    fire(grip(), "mousedown", 2);
    fire(grip(), "click", 2);
    fire(grip(), "dblclick", 2);
    expect(onToggle).toHaveBeenCalledTimes(1);
  });
});
