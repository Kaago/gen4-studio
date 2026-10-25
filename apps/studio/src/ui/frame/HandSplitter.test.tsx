// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HandSplitter } from "./HandSplitter";
import { ShellProvider, useShell } from "./ShellContext";
import type { ShellApi } from "./ShellContext";
import { HAND_DEFAULT } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("HandSplitter", () => {
  let container: HTMLElement;
  let root: Root;
  let shell: ShellApi;

  function Probe() {
    shell = useShell();
    return null;
  }
  const show = (hidden?: boolean) =>
    act(() =>
      root.render(
        <ShellProvider initialArea="welt">
          <Probe />
          <div>
            <HandSplitter area="welt" controls="dock-welt-hand" hidden={hidden} />
          </div>
        </ShellProvider>,
      ),
    );
  const splitter = () => container.querySelector<HTMLElement>("[data-splitter=hand]")!;
  const height = () => shell.state.handHeight.welt;
  const collapsed = () => shell.state.handCollapsed.welt;
  /** A splitter whose lower edge is at y 600 inside a deck that ends at y 900: the hand is 300 px high. */
  function geometry() {
    const rect = (bottom: number) => ({ bottom, top: bottom - 8, height: 8, left: 0, right: 100, width: 100, x: 0, y: bottom - 8, toJSON: () => ({}) }) as DOMRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      return this.getAttribute("role") === "separator" ? rect(600) : rect(900);
    });
  }
  const pointer = (type: string, clientY: number) => {
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientY });
    Object.defineProperty(event, "pointerId", { value: 1 });
    act(() => void splitter().dispatchEvent(event));
  };
  const key = (name: string) => act(() => void splitter().dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })));

  beforeEach(() => {
    localStorage.clear();
    Element.prototype.setPointerCapture = vi.fn();
    Element.prototype.releasePointerCapture = vi.fn();
    Element.prototype.hasPointerCapture = vi.fn(() => true);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  it("is_a_separator_that_names_the_hand_it_controls_and_its_height", () => {
    show();
    const el = splitter();
    expect(el.getAttribute("role")).toBe("separator");
    expect(el.getAttribute("aria-orientation")).toBe("horizontal");
    expect(el.getAttribute("aria-label")).toBe("Höhe der Hand");
    expect(el.getAttribute("aria-controls")).toBe("dock-welt-hand");
    expect(el.getAttribute("aria-valuemin")).toBe("200");
    expect(el.getAttribute("aria-valuemax")).toBe("560");
    expect(el.getAttribute("aria-valuenow")).toBe(String(HAND_DEFAULT));
    expect(el.getAttribute("aria-valuenow")).toBe("286");
    expect(el.getAttribute("aria-valuetext")).toBe("286 px");
    expect(el.tabIndex).toBe(0);
    expect(el.title).toBe("Ziehen ändert die Höhe, Doppelklick klappt ein");
  });

  it("sets_the_hand_height_while_dragging_with_the_pointer_captured", () => {
    show();
    geometry();
    pointer("pointerdown", 600);
    expect(vi.mocked(Element.prototype.setPointerCapture).mock.contexts).toEqual([splitter()]);
    pointer("pointermove", 500);
    // The pointer moved 100 px up: the hand grows from 300 to 400.
    expect(height()).toBe(400);
    pointer("pointermove", 620);
    expect(height()).toBe(280);
    pointer("pointerup", 620);
    pointer("pointermove", 100);
    expect(height()).toBe(280);
  });

  it("keeps_the_height_between_200_and_560", () => {
    show();
    geometry();
    pointer("pointerdown", 600);
    pointer("pointermove", -2000);
    expect(height()).toBe(560);
    pointer("pointermove", 5000);
    expect(height()).toBe(200);
  });

  it("opens_a_collapsed_hand_when_dragged", () => {
    show();
    act(() => shell.toggleHandCollapsed("welt"));
    expect(collapsed()).toBe(true);
    geometry();
    pointer("pointerdown", 600);
    pointer("pointermove", 450);
    expect(collapsed()).toBe(false);
    expect(height()).toBe(450);
  });

  it("grows_and_shrinks_by_16_with_the_arrow_keys_and_jumps_with_home_and_end", () => {
    show();
    key("ArrowUp");
    expect(height()).toBe(302);
    key("ArrowDown");
    key("ArrowDown");
    expect(height()).toBe(270);
    key("Home");
    expect(height()).toBe(560);
    key("End");
    expect(height()).toBe(200);
    expect(splitter().getAttribute("aria-valuenow")).toBe("200");
  });

  it("collapses_and_expands_on_enter_and_on_a_double_click", () => {
    show();
    key("Enter");
    expect(collapsed()).toBe(true);
    expect(splitter().getAttribute("aria-valuenow")).toBe("0");
    expect(splitter().getAttribute("aria-valuetext")).toBe("eingeklappt");
    act(() => void splitter().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
    expect(collapsed()).toBe(false);
    expect(splitter().getAttribute("aria-valuenow")).toBe("286");
    act(() => void splitter().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
    expect(collapsed()).toBe(true);
    // The remembered height is not touched by collapsing.
    expect(height()).toBe(286);
  });

  it("renders_nothing_while_hidden", () => {
    show(true);
    expect(splitter()).toBeNull();
    expect(container.innerHTML).toBe("<div></div>");
    show(false);
    expect(splitter()).not.toBeNull();
  });
});
