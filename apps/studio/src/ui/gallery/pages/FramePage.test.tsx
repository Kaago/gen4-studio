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

  it("shows_the_hinge_above_the_dock_and_the_floating_toolbar", () => {
    // The deck has no splitter of its own (dockview's sash sizes the groups); the toolbar upright and flat.
    expect(container.querySelector('[data-frame="scharnier-oben"] [data-splitter]')).toBeNull();
    const bars = [...container.querySelectorAll('[aria-label="Werkzeugleiste"]')];
    expect(bars.map((b) => b.getAttribute("aria-orientation"))).toEqual(expect.arrayContaining(["vertical", "horizontal"]));
    // The hinge heads the first frame of the section, before the dock with the view above the edit group.
    const frame = container.querySelector('[data-frame="scharnier-oben"]')!;
    const order = [...frame.querySelectorAll('[aria-label="Scharnier"], [data-dock], [data-dock-group]')].map((el) => el.getAttribute("data-dock-group") ?? el.getAttribute("data-dock") ?? "scharnier");
    expect(order).toEqual(["scharnier", "welt", "schau", "bearbeiten"]);
  });

  it("speaks_of_the_app_as_it_is_now_without_markdown_in_the_notes", () => {
    // Token names are quoted in the notes, not written as Markdown code (the page shows no backtick).
    expect(container.textContent).not.toContain("`");
    // The hinge above the dock: the place as the app names it, the dock with the view and the edit group (no „Hand“ any more).
    const frame = container.querySelector<HTMLElement>('[data-frame="scharnier-oben"]')!;
    expect(frame.textContent).toContain("Gebiet 0411");
    expect(frame.textContent).not.toContain("Hand");
    expect(frame.textContent).not.toContain("Außen");
  });

  it("shows_the_dock_tabs_and_a_floating_group", () => {
    const frame = (name: string) => container.querySelector<HTMLElement>(`[data-frame="${name}"]`)!;
    const tabs = (group: Element) => [...group.querySelectorAll<HTMLElement>(".dv-tab")];
    const menu = (root: Element, id: string) =>
      [...root.querySelectorAll<HTMLElement>(`[data-tab-menu="${id}"] > li`)].map((li) =>
        li.classList.contains("dv-context-menu-separator") ? "—" : `${li.textContent}${li.getAttribute("aria-disabled") === "true" ? " (gesperrt)" : ""}`,
      );

    // The tabs in dockview's classes inside the dock theme: one visible tab per group, the view group has the focus.
    const docked = frame("dock-reiter");
    expect(docked.classList.contains("dockview-theme-g4")).toBe(true);
    const view = docked.querySelector('[data-dock-group="schau"]')!;
    const below = docked.querySelector('[data-dock-group="unten"]')!;
    expect(view.classList.contains("dv-active-group")).toBe(true);
    expect(below.classList.contains("dv-inactive-group")).toBe(true);
    expect(tabs(view).map((t) => t.textContent)).toEqual(["3D", "Raster", "Weltkarte"]);
    expect(tabs(view).map((t) => t.classList.contains("dv-active-tab"))).toEqual([true, false, false]);
    // The unsaved dot on the script's tab, and no close button anywhere.
    expect(below.querySelector('[data-dock-tab="script"] [role="img"][aria-label="ungespeichert"]')).not.toBeNull();
    expect(docked.querySelector(".dv-default-tab-action")).toBeNull();
    // The menus come from the app's own `tabMenuItems`: a view tab only maximizes, a docked one also floats and resets.
    expect(menu(docked, "world")).toEqual(["Maximieren", "—", "Als Vorschaubild"]);
    // „Als Vorschaubild“ on the world map, locked without a chosen map, with its reason as the tooltip.
    expect(menu(docked, "worldmap")).toEqual(["Maximieren", "—", "Als Vorschaubild (gesperrt)"]);
    expect(docked.querySelector('[data-tab-menu="worldmap"] [aria-disabled="true"]')?.getAttribute("title")).toBe("Erst eine Karte wählen");
    expect(menu(docked, "script")).toEqual(["Maximieren", "Schweben lassen", "—", "Anordnung zurücksetzen"]);

    // The floating group: dockview's floating container with the title bar as its grip, the inspector's tab and head in it.
    const floating = frame("schwebende-gruppe");
    const group = floating.querySelector(".dv-resize-container.dv-resize-container-with-titlebar")!;
    expect(group.firstElementChild?.classList.contains("dv-floating-titlebar")).toBe(true);
    expect(tabs(group).map((t) => t.textContent)).toEqual(["Inspector"]);
    expect(group.querySelector('header[data-inspector-kind="figure"]')).not.toBeNull();
    // The view stays docked under it; the floating panel's menu cannot maximize or float again, but docks back.
    expect(floating.querySelector('[data-dock-group="schau"]')?.closest(".dv-resize-container")).toBeNull();
    expect(menu(floating, "inspector")).toEqual(["Maximieren (gesperrt)", "Schweben lassen (gesperrt)", "Zurück ins Dock", "—", "Anordnung zurücksetzen"]);

    // The arrangements, and the hinge's „Schau maximieren“, pressed while the view fills the dock.
    expect([...frame("anordnung").querySelectorAll("[data-arrangement]")].map((a) => a.getAttribute("data-arrangement"))).toEqual(["werkbank", "doppeldeck"]);
    const maximize = frame("scharnier-oben").querySelector<HTMLButtonElement>('button[aria-label="Schau maximieren"]')!;
    expect(maximize.getAttribute("aria-pressed")).toBe("false");
    act(() => maximize.click());
    expect(maximize.getAttribute("aria-pressed")).toBe("true");
    expect(frame("scharnier-oben").querySelector<HTMLElement>('[data-dock-group="bearbeiten"]')!.hidden).toBe(true);
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
    expect(heads.map((h) => h.getAttribute("data-inspector-kind"))).toEqual(["figure", "area", "area", "door", "map", "room"]);
    expect(heads.map((h) => h.querySelector("[data-eyebrow]")?.textContent)).toEqual(["Figur", "Gebiet", "Header", "Tür", "Karte", "Raum"]);
    // The state against the base or as a tombstone, before the other badges.
    expect(heads.map((h) => h.querySelector("[data-entity-state]")?.textContent ?? null)).toEqual([null, null, "geändert", null, "neu", "gelöscht"]);
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

  it("shows_the_recipe_button_at_rest_pressed_and_locked_and_the_recipe_bar", () => {
    const keys = [...container.querySelectorAll<HTMLButtonElement>('[data-gallery="rezept-taste"] button[data-recipe]')];
    expect(keys.slice(0, 4).map((b) => [b.textContent, b.getAttribute("aria-pressed"), b.disabled])).toEqual([
      ["Gebiet anlegen", null, false],
      ["Raum anlegen", null, false],
      ["Raum anlegen", "true", false],
      ["Zu einer Geschichte", null, true],
    ]);
    // A locked key as only its symbol, the way the bar shows it.
    expect(keys[4]?.getAttribute("aria-label")).toBe("Hier spielen");
    expect(keys[4]?.textContent).toBe("");
    // The bar upright at the left and flat at the top, one row: since 3d.5 every recipe is free; upright named, flat only its symbol
    // with the name as the accessible name (U3).
    const names = ["Gebiet anlegen", "Raum anlegen", "Neue Figur", "Tür verbinden", "Neuer Trainer hier"];
    const bars = [...container.querySelectorAll<HTMLElement>('[role="toolbar"][aria-label="Rezepte"]')];
    expect(bars.map((b) => b.getAttribute("aria-orientation"))).toEqual(["vertical", "horizontal"]);
    const recipes = (bar: HTMLElement) => [...bar.querySelectorAll("button[data-recipe]")];
    expect(recipes(bars[0]!).map((b) => b.textContent)).toEqual(names);
    expect(recipes(bars[1]!).map((b) => b.getAttribute("aria-label"))).toEqual(names);
  });
});
