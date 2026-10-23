// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { IconButton } from "./IconButton";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("IconButton", () => {
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
    vi.useRealTimers();
  });

  const button = () => container.querySelector("button")!;

  it("names_itself_by_the_label_and_shows_it_as_tooltip_on_hover", async () => {
    vi.useFakeTimers();
    act(() => root.render(<IconButton label="Speichern" icon="save" shortcut="⌘S" onClick={() => {}} />));
    expect(button().getAttribute("aria-label")).toBe("Speichern");
    expect(document.querySelector("[role=tooltip]")).toBeNull();

    await act(async () => {
      button().dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      button().dispatchEvent(new MouseEvent("mouseenter"));
      button().dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
      await vi.advanceTimersByTimeAsync(500);
    });
    const tip = document.querySelector("[role=tooltip]");
    expect(tip).not.toBeNull();
    expect(tip!.textContent).toBe("Speichern ⌘S");
  });

  it("does_not_open_the_tooltip_before_its_delay", async () => {
    vi.useFakeTimers();
    act(() => root.render(<IconButton label="Speichern" icon="save" onClick={() => {}} />));
    await act(async () => {
      button().dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      button().dispatchEvent(new MouseEvent("mouseenter"));
      button().dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
      await vi.advanceTimersByTimeAsync(300);
    });
    expect(document.querySelector("[role=tooltip]")).toBeNull();
  });

  it("reports_pressed_state", () => {
    act(() => root.render(<IconButton label="Gelände" icon="paint" pressed onClick={() => {}} />));
    expect(button().getAttribute("aria-pressed")).toBe("true");
    act(() => root.render(<IconButton label="Gelände" icon="paint" pressed={false} onClick={() => {}} />));
    expect(button().getAttribute("aria-pressed")).toBe("false");
  });

  it("has_no_pressed_state_when_not_a_toggle_and_calls_onClick", () => {
    const onClick = vi.fn();
    act(() => root.render(<IconButton label="Speichern" icon="save" onClick={onClick} />));
    expect(button().hasAttribute("aria-pressed")).toBe(false);
    act(() => button().click());
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("calls_onClick_on_a_toggle_too", () => {
    const onClick = vi.fn();
    act(() => root.render(<IconButton label="Gelände" icon="paint" pressed={false} onClick={onClick} />));
    act(() => button().click());
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
