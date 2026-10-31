// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { NextStep } from "../../api";
import { DoneResult } from "./WizardResult";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DoneResult", () => {
  let container: HTMLElement;
  let root: Root;
  const onStep = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onStep.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const STEPS: NextStep[] = [
    { id: "create_area", label: "Gebiet anlegen", available: true, reason: null },
    { id: "new_figure", label: "Neue Figur hier", available: false, reason: "Kommt mit 3d.5" },
  ];
  const keys = () => [...container.querySelectorAll<HTMLButtonElement>("button[data-recipe]")];

  it("steps_render_as_recipe_buttons_locked_with_reason_and_click_calls_on_step", () => {
    act(() => root.render(<DoneResult title="Welt geleert" lines={["Randkarte 0666"]} next={["Mit „Hier spielen“ ausprobieren."]} steps={STEPS} onStep={onStep} />));
    expect(keys().map((b) => [b.getAttribute("data-recipe"), b.textContent, b.disabled])).toEqual([
      ["create_area", "Gebiet anlegen", false],
      ["new_figure", "Neue Figur hier", true],
    ]);
    expect(document.getElementById(keys()[1]!.getAttribute("aria-describedby")!)?.textContent).toBe("Kommt mit 3d.5");
    // Under „Als Nächstes“ the buttons come first, the lines of text after them.
    const next = container.querySelector("[data-next-steps]")!;
    expect(next.previousElementSibling?.textContent).toBe("Als Nächstes");
    expect(container.textContent).toContain("Mit „Hier spielen“ ausprobieren.");
    expect(next.compareDocumentPosition(container.querySelector("ul")!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    act(() => keys()[0]!.click());
    expect(onStep).toHaveBeenCalledExactlyOnceWith("create_area");
    act(() => keys()[1]!.click());
    expect(onStep).toHaveBeenCalledOnce();
  });

  it("without_steps_shows_the_lines_only", () => {
    act(() => root.render(<DoneResult title="Gebiet angelegt" lines={["A"]} next={["B"]} />));
    expect(keys()).toEqual([]);
    expect(container.querySelector("[data-next-steps]")).toBeNull();
    expect(container.textContent).toContain("Als Nächstes");
  });
});
