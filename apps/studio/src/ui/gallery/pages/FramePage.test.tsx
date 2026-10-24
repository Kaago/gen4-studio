// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FramePage } from "./FramePage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("FramePage", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<FramePage />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const button = (label: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === label);

  it("shows_the_wizard_step_bar_in_the_hinge_and_steps_through_it", () => {
    const hinge = container.querySelector('[role="toolbar"][aria-label="Scharnier"]')!;
    const slot = hinge.querySelector<HTMLElement>("[data-wizard-bar]")!;
    expect(slot.hidden).toBe(false);
    expect(slot.querySelector("h2")?.textContent).toBe("Gebiet anlegen");
    const current = () => slot.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
    expect(current()).toBe("1. Größe");
    act(() => button("Weiter")!.click());
    expect(current()).toBe("2. Lage");
  });

  it("shows_start_cards_tabs_and_a_button_that_opens_the_palette", () => {
    expect(container.querySelector('[aria-label="Zuletzt geöffnete Projekte"]')?.querySelectorAll("li")).toHaveLength(2);
    expect([...container.querySelectorAll('[role="tab"]')].map((t) => t.textContent)).toEqual(expect.arrayContaining([expect.stringContaining("Probleme")]));
    // The button's text carries its shortcut after the label.
    act(() => [...container.querySelectorAll("button")].find((b) => b.textContent?.startsWith("Palette öffnen"))!.click());
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  });
});
