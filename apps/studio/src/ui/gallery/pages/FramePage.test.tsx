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

  it("shows_the_splitter_horizontally_and_vertically_and_sizes_the_first_part", () => {
    const frame = (name: string) => container.querySelector<HTMLElement>(`[data-frame="${name}"]`)!;
    const horizontal = frame("teiler-waagrecht").querySelector<HTMLElement>('[role="separator"]')!;
    const vertical = frame("teiler-senkrecht").querySelector<HTMLElement>('[role="separator"]')!;
    expect(horizontal.getAttribute("aria-orientation")).toBe("horizontal");
    expect(vertical.getAttribute("aria-orientation")).toBe("vertical");
    const press = (el: HTMLElement, key: string) => act(() => void el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
    press(horizontal, "ArrowDown");
    expect(horizontal.getAttribute("aria-valuenow")).toBe("136");
    expect(frame("teiler-waagrecht").firstElementChild?.getAttribute("style")).toContain("136px");
    press(vertical, "ArrowRight");
    expect(vertical.getAttribute("aria-valuenow")).toBe("136");
    press(vertical, "ArrowDown");
    expect(vertical.getAttribute("aria-valuenow")).toBe("136");
  });
});
