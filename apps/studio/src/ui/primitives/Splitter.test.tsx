// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Splitter } from "./Splitter";
import type { SplitterProps } from "./Splitter";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Splitter", () => {
  let container: HTMLElement;
  let root: Root;
  const onValue = vi.fn();
  const base = { orientation: "horizontal", grows: "up", label: "Höhe", value: 300, min: 200, max: 560, onValue, measure: (e: PointerEvent) => 1000 - e.clientY } as SplitterProps;
  const show = (props: Partial<SplitterProps> = {}) => act(() => root.render(<Splitter {...({ ...base, ...props } as SplitterProps)} />));
  const el = () => container.querySelector<HTMLElement>("[role=separator]")!;
  const pointer = (type: string, clientX: number, clientY: number, init: PointerEventInit = {}) => {
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientX, clientY, ...init });
    Object.defineProperty(event, "pointerId", { value: 1 });
    act(() => void el().dispatchEvent(event));
  };

  beforeEach(() => {
    onValue.mockReset();
    Element.prototype.setPointerCapture = vi.fn();
    Element.prototype.releasePointerCapture = vi.fn();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  it("is_a_separator_with_value_range_label_and_controls", () => {
    show({ controls: "dock", title: "Ziehen ändert die Höhe", dataAttrs: { "data-splitter": "demo" } });
    expect(el().getAttribute("aria-orientation")).toBe("horizontal");
    expect(el().getAttribute("aria-label")).toBe("Höhe");
    expect(el().getAttribute("aria-controls")).toBe("dock");
    expect(el().getAttribute("aria-valuemin")).toBe("200");
    expect(el().getAttribute("aria-valuemax")).toBe("560");
    expect(el().getAttribute("aria-valuenow")).toBe("300");
    expect(el().getAttribute("aria-valuetext")).toBe("300 px");
    expect(el().tabIndex).toBe(0);
    expect(el().title).toBe("Ziehen ändert die Höhe");
    expect(el().getAttribute("data-splitter")).toBe("demo");
    show({ orientation: "vertical", grows: "right", valueText: "schmal" });
    expect(el().getAttribute("aria-orientation")).toBe("vertical");
    expect(el().getAttribute("aria-valuetext")).toBe("schmal");
    expect(el().hasAttribute("aria-controls")).toBe(false);
  });

  it("steps_with_the_arrows_in_the_direction_it_grows", () => {
    show({ grows: "up" });
    pressKey(el(), "ArrowUp");
    pressKey(el(), "ArrowDown");
    // The second step starts from the first (nothing rendered between them).
    expect(onValue.mock.calls).toEqual([[316], [300]]);
    onValue.mockReset();
    show({ grows: "down", step: 10 });
    pressKey(el(), "ArrowDown");
    pressKey(el(), "ArrowUp");
    // The arrows of the other axis do nothing.
    const left = pressKey(el(), "ArrowLeft");
    expect(onValue.mock.calls).toEqual([[310], [300]]);
    expect(left.defaultPrevented).toBe(false);
    onValue.mockReset();
    show({ orientation: "vertical", grows: "left" });
    pressKey(el(), "ArrowLeft");
    pressKey(el(), "ArrowRight");
    const up = pressKey(el(), "ArrowUp");
    expect(onValue.mock.calls).toEqual([[316], [300]]);
    expect(up.defaultPrevented).toBe(false);
    onValue.mockReset();
    show({ orientation: "vertical", grows: "right" });
    pressKey(el(), "ArrowRight");
    expect(onValue.mock.calls).toEqual([[316]]);
  });

  it("keeps_a_step_inside_min_and_max", () => {
    show({ value: 555 });
    pressKey(el(), "ArrowUp");
    expect(onValue.mock.calls).toEqual([[560]]);
  });

  it("jumps_to_the_ends_with_home_and_end", () => {
    show();
    const home = pressKey(el(), "Home");
    pressKey(el(), "End");
    expect(onValue.mock.calls).toEqual([[560], [200]]);
    expect(home.defaultPrevented).toBe(true);
  });

  it("drags_with_the_pointer_captured", () => {
    show();
    pointer("pointerdown", 0, 700);
    expect(vi.mocked(Element.prototype.setPointerCapture).mock.contexts).toEqual([el()]);
    pointer("pointermove", 0, 600);
    // The pointer went 100 px up: the value grows by what `measure` reports, from where the drag began.
    expect(onValue.mock.calls).toEqual([[400]]);
    pointer("pointermove", 0, 750);
    expect(onValue).toHaveBeenLastCalledWith(250);
    pointer("pointermove", 0, -2000);
    expect(onValue).toHaveBeenLastCalledWith(560);
    pointer("pointerup", 0, 750);
    expect(vi.mocked(Element.prototype.releasePointerCapture).mock.contexts).toEqual([el()]);
    onValue.mockReset();
    pointer("pointermove", 0, 100);
    expect(onValue).not.toHaveBeenCalled();
  });

  it("ignores_other_buttons_and_another_pointer_and_lets_go_on_cancel", () => {
    show();
    pointer("pointerdown", 0, 700, { button: 2 });
    pointer("pointermove", 0, 600);
    expect(onValue).not.toHaveBeenCalled();
    pointer("pointerdown", 0, 700);
    const other = new MouseEvent("pointermove", { bubbles: true, cancelable: true, clientY: 600 });
    Object.defineProperty(other, "pointerId", { value: 2 });
    act(() => void el().dispatchEvent(other));
    expect(onValue).not.toHaveBeenCalled();
    pointer("pointercancel", 0, 700);
    pointer("pointermove", 0, 600);
    expect(onValue).not.toHaveBeenCalled();
  });

  it("calls_double_click_and_enter", () => {
    const onDoubleClick = vi.fn();
    const onEnter = vi.fn();
    show({ onDoubleClick, onEnter });
    act(() => void el().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
    const enter = pressKey(el(), "Enter");
    expect(onDoubleClick).toHaveBeenCalledOnce();
    expect(onEnter).toHaveBeenCalledOnce();
    expect(enter.defaultPrevented).toBe(true);
    // Without handlers they do nothing and Enter is not swallowed.
    show({ onDoubleClick: undefined, onEnter: undefined });
    expect(pressKey(el(), "Enter").defaultPrevented).toBe(false);
  });

  it("does_not_drag_before_4_px", () => {
    show();
    pointer("pointerdown", 100, 700);
    pointer("pointermove", 101, 701);
    pointer("pointermove", 103, 702);
    expect(onValue).not.toHaveBeenCalled();
    // Sideways counts too; past the slop the drag starts and measures from where the pointer was pressed.
    pointer("pointermove", 100, 695);
    expect(onValue.mock.calls).toEqual([[305]]);
    // Once begun, a move back inside the slop still moves.
    pointer("pointermove", 100, 699);
    expect(onValue).toHaveBeenLastCalledWith(301);
    pointer("pointerup", 100, 699);
  });

  it("counts_two_key_presses_before_a_render_twice", () => {
    show();
    act(() => {
      el().dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true }));
      el().dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true }));
    });
    expect(onValue.mock.calls).toEqual([[316], [332]]);
  });

  it("forgets_the_pending_steps_when_its_owner_did_not_take_them", async () => {
    show();
    pressKey(el(), "ArrowDown");
    await Promise.resolve();
    pressKey(el(), "ArrowUp");
    // The value stayed 300 (nothing rendered): the second press starts from it, not from 284.
    expect(onValue.mock.calls).toEqual([[284], [316]]);
  });

  it("takes_the_value_of_a_render_as_the_new_start", () => {
    show();
    pressKey(el(), "ArrowUp");
    show({ value: 316 });
    pressKey(el(), "ArrowUp");
    expect(onValue.mock.calls).toEqual([[316], [332]]);
  });
});
