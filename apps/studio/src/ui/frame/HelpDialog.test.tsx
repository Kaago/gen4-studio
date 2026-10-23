// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TERMS } from "../../workbench/terms";
import { pressKey } from "../primitives/testUtils";
import { HelpDialog } from "./HelpDialog";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("HelpDialog", () => {
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

  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");

  it("renders_nothing_while_closed", () => {
    act(() => root.render(<HelpDialog open={false} onOpenChange={() => {}} />));
    expect(dialog()).toBeNull();
  });

  it("lists_every_term_with_its_technical_name_and_explanation", () => {
    act(() => root.render(<HelpDialog open onOpenChange={() => {}} />));
    const box = dialog()!;
    // The dialog is rendered into the page body, outside the frame that opened it.
    expect(container.contains(box)).toBe(false);
    expect(box.getAttribute("aria-labelledby")).not.toBeNull();
    const text = box.textContent ?? "";
    for (const term of Object.values(TERMS)) {
      expect(text).toContain(term.label);
      expect(text).toContain(term.short);
      if (term.tech) expect(text).toContain(term.tech);
    }
    expect(box.querySelectorAll("dt")).toHaveLength(Object.keys(TERMS).length);
  });

  it("asks_its_owner_to_close_on_escape", () => {
    const onOpenChange = vi.fn();
    act(() => root.render(<HelpDialog open onOpenChange={onOpenChange} />));
    pressKey(dialog()!, "Escape");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
