// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Kbd } from "./Kbd";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Kbd", () => {
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

  it("renders_a_kbd_with_the_keys_and_no_button", () => {
    act(() => root.render(<Kbd keys="⌘K" />));
    const kbd = container.querySelector("kbd")!;
    expect(kbd.textContent).toBe("⌘K");
    expect(container.querySelector("button")).toBeNull();
  });

  it("renders_the_same_element_on_the_hinge_and_stays_not_interactive", () => {
    act(() => root.render(<Kbd keys="⇧⌘P" onHinge />));
    const kbd = container.querySelector("kbd")!;
    expect(kbd.textContent).toBe("⇧⌘P");
    expect(container.querySelector("button")).toBeNull();
    expect(container.firstElementChild).toBe(kbd);
  });
});
