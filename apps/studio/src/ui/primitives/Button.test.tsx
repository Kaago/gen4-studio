// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Button } from "./Button";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Button", () => {
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

  it("renders_the_label_with_shortcut_and_forwards_disabled", () => {
    act(() =>
      root.render(
        <Button kind="primary" shortcut="⌘S" disabled>
          Speichern
        </Button>,
      ),
    );
    const b = container.querySelector("button")!;
    expect(b.textContent).toContain("Speichern");
    expect(b.disabled).toBe(true);
    expect(b.textContent).toContain("⌘S");
  });

  it("forwards_native_props_and_clicks_and_defaults_to_type_button", () => {
    const onClick = vi.fn();
    act(() =>
      root.render(
        <Button kind="secondary" icon="save" aria-label="Sichern" onClick={onClick}>
          Sichern
        </Button>,
      ),
    );
    const b = container.querySelector("button")!;
    expect(b.getAttribute("type")).toBe("button");
    expect(b.getAttribute("aria-label")).toBe("Sichern");
    expect(b.querySelector("svg")).not.toBeNull();
    act(() => b.click());
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
