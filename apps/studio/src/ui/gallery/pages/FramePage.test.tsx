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

  it("shows_the_inspector_head_and_the_scene_hierarchy", () => {
    // The heads: the kind as an eyebrow, the title, the ident and the badges; Tüfteln says „Header“ where Bauen says „Gebiet“.
    const heads = [...container.querySelectorAll<HTMLElement>('[data-frame="inspector-kopf"] header')];
    expect(heads.map((h) => h.getAttribute("data-inspector-kind"))).toEqual(["figure", "area", "area", "door", "map"]);
    expect(heads.map((h) => h.querySelector("[data-eyebrow]")?.textContent)).toEqual(["Figur", "Gebiet", "Header", "Tür", "Karte"]);
    expect(heads[0]!.querySelector("[data-ident]")?.textContent).toBe("Objekt 2 · Eventdatei 0390");
    expect(heads[0]!.textContent).toContain("Ungespeichert");
    expect(heads[3]!.textContent).toContain("Gesperrt");
    expect(heads[4]!.textContent).toContain("Nicht rückgängig");
    // Each head has the menu „Aktionen“; the locked event cannot be deleted.
    const menu = (head: HTMLElement) => head.querySelector<HTMLButtonElement>('button[aria-label="Aktionen"]')!;
    expect(heads.every((h) => menu(h) !== null)).toBe(true);
    act(() => menu(heads[3]!).click());
    const items = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
    expect(items.map((i) => i.textContent)).toEqual(["LöschenEntf"]);
    expect(items[0]!.getAttribute("aria-disabled")).toBe("true");
    act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));

    // The hierarchy: a tree with the groups of the sample scene, the figure chosen, a finding at the root and at the door without a target.
    const frame = container.querySelector<HTMLElement>('[data-frame="hierarchie"]')!;
    const tree = frame.querySelector('[role="tree"][aria-label="In dieser Szene"]')!;
    expect([...tree.querySelectorAll("[data-scene-group]")].map((g) => g.getAttribute("data-scene-group"))).toEqual(["header", "maps", "object", "warp", "coord", "bg", "scripts"]);
    expect(tree.querySelector('[data-scene-group="object"]')?.textContent).toBe("Figuren2");
    expect(tree.querySelector('[data-event="object:1"]')?.getAttribute("aria-selected")).toBe("true");
    expect(tree.querySelector('[data-event="warp:0"]')?.textContent).toContain("→ Haus 1");
    expect(tree.querySelector('[data-event="warp:1"]')?.textContent).toContain("→ unbekanntes Ziel 0999");
    expect(tree.querySelector('[data-scene-group="header"] [role="img"]')).not.toBeNull();
    expect(tree.querySelector('[data-event="warp:1"] [role="img"]')).not.toBeNull();
    // A click selects, and the one locked scene names its reason and marks the event groups.
    act(() => tree.querySelector<HTMLElement>('[data-event="coord:0"]')!.click());
    expect(tree.querySelector('[data-event="coord:0"]')?.getAttribute("aria-selected")).toBe("true");
    expect(tree.querySelector('[data-event="object:1"]')?.getAttribute("aria-selected")).toBe("false");
    const locked = container.querySelector<HTMLElement>('[data-frame="hierarchie-gesperrt"]')!;
    expect(locked.textContent).toContain("Eventdatei gesperrt");
    expect(locked.querySelector('[data-scene-group="warp"]')?.textContent).toContain("Gesperrt");
  });
});
