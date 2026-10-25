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
    const hinge = container.querySelector('[data-frame="assistent"] [role="toolbar"][aria-label="Scharnier"]')!;
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

  it("shows_the_hinge_above_the_view_with_the_splitter_and_the_floating_toolbar", () => {
    // One splitter between the view and the hand, and the toolbar upright and flat.
    expect(container.querySelectorAll('[role="separator"][data-splitter]')).toHaveLength(1);
    const bars = [...container.querySelectorAll('[aria-label="Werkzeugleiste"]')];
    expect(bars.map((b) => b.getAttribute("aria-orientation"))).toEqual(expect.arrayContaining(["vertical", "horizontal"]));
    // The hinge heads the first frame of the section, before the view and the splitter.
    const frame = container.querySelector('[data-frame="scharnier-oben"]')!;
    const order = [...frame.querySelectorAll('[aria-label="Scharnier"], [data-dock="schau"], [data-splitter], [data-dock="hand"]')].map(
      (el) => el.getAttribute("data-dock") ?? (el.hasAttribute("data-splitter") ? "teiler" : "scharnier"),
    );
    expect(order).toEqual(["scharnier", "schau", "teiler", "hand"]);
  });
});
