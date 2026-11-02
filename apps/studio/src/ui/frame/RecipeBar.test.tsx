// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RECIPE_BAR } from "../../workbench/recipes";
import { pressKey } from "../primitives/testUtils";
import { RecipeBar } from "./RecipeBar";
import type { ToolbarAnchor } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("RecipeBar", () => {
  let container: HTMLElement;
  let root: Root;
  const onRecipe = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onRecipe.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (anchor: ToolbarAnchor = "left") => act(() => root.render(<RecipeBar entries={RECIPE_BAR} onRecipe={onRecipe} density="komfort" anchor={anchor} />));
  const bar = () => container.querySelector<HTMLElement>('[role="toolbar"][aria-label="Rezepte"]')!;
  const keys = () => [...container.querySelectorAll<HTMLButtonElement>("button[data-recipe]")];
  const key = (id: string) => container.querySelector<HTMLButtonElement>(`button[data-recipe="${id}"]`)!;

  it("lists_five_recipes_all_runnable", () => {
    show();
    expect(bar().getAttribute("aria-orientation")).toBe("vertical");
    expect(bar().getAttribute("data-anchor")).toBe("left");
    expect(keys().map((b) => b.getAttribute("data-recipe"))).toEqual(["create_area", "create_room", "new_figure", "connect_doors", "new_trainer_here"]);
    // Since 3d.5 every recipe runs: each key shows its name, none is locked or carries a reason.
    expect(keys().map((b) => b.textContent)).toEqual(["Gebiet anlegen", "Raum anlegen", "Neue Figur", "Tür verbinden", "Neuer Trainer hier"]);
    expect(keys().map((b) => b.disabled)).toEqual([false, false, false, false, false]);
    expect(keys().map((b) => b.hasAttribute("data-icon-only"))).toEqual([false, false, false, false, false]);
    for (const b of keys()) expect(b.querySelectorAll("svg")).toHaveLength(1);
    // Flat at the top or the bottom, like the toolbar it stands in for.
    show("bottom");
    expect(bar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(bar().getAttribute("data-anchor")).toBe("bottom");
  });

  it("flat_shows_only_the_symbols_with_the_names_as_accessible_names", () => {
    // UI test 3d.5 (U3): with a wizard open the bar lies flat at the top beside the layer bar; five names ran 24 px past the view in
    // Komfort at 1440 × 900. Flat, every key is its symbol: the name is the accessible name and heads the tooltip.
    const names = ["Gebiet anlegen", "Raum anlegen", "Neue Figur", "Tür verbinden", "Neuer Trainer hier"];
    for (const anchor of ["top", "bottom"] as const) {
      show(anchor);
      expect(keys().map((b) => b.hasAttribute("data-icon-only"))).toEqual([true, true, true, true, true]);
      expect(keys().map((b) => b.textContent)).toEqual(["", "", "", "", ""]);
      expect(keys().map((b) => b.getAttribute("aria-label"))).toEqual(names);
      expect(keys().map((b) => b.disabled)).toEqual([false, false, false, false, false]);
    }
    // Upright the names are back.
    show("left");
    expect(keys().map((b) => b.textContent)).toEqual(names);
    expect(keys().some((b) => b.hasAttribute("aria-label"))).toBe(false);
  });

  it("a_locked_entry_shows_its_symbol_and_lock_and_is_skipped_by_the_arrows", () => {
    // No recipe of the bar is locked since 3d.5; a locked entry still behaves as before (a later recipe may need it).
    const entries = RECIPE_BAR.map((r) => (r.id === "create_room" ? { ...r, reason: "Kommt später" } : r));
    act(() => root.render(<RecipeBar entries={entries} onRecipe={onRecipe} density="komfort" anchor="left" />));
    expect(key("create_room").disabled).toBe(true);
    expect(key("create_room").textContent).toBe("");
    expect(key("create_room").getAttribute("aria-label")).toBe("Raum anlegen");
    expect(key("create_room").querySelectorAll("svg")).toHaveLength(2);
    expect(document.getElementById(key("create_room").getAttribute("aria-describedby")!)?.textContent).toBe("Kommt später");
    act(() => key("create_area").focus());
    pressKey(key("create_area"), "ArrowDown");
    expect(document.activeElement).toBe(key("new_figure"));
  });

  it("click_runs_the_recipe", () => {
    show();
    act(() => key("create_room").click());
    expect(onRecipe).toHaveBeenCalledExactlyOnceWith("create_room");
    act(() => key("new_figure").click());
    expect(onRecipe).toHaveBeenLastCalledWith("new_figure");
    expect(onRecipe).toHaveBeenCalledTimes(2);
  });

  it("moves_between_the_runnable_recipes_with_the_arrows", () => {
    show();
    // One tab stop: the first runnable recipe (a locked one is disabled, out of the tab order); the arrows of the orientation move it.
    expect(keys().filter((b) => !b.disabled && b.tabIndex === 0)).toEqual([key("create_area")]);
    expect(key("create_room").tabIndex).toBe(-1);
    act(() => key("create_area").focus());
    pressKey(key("create_area"), "ArrowDown");
    expect(document.activeElement).toBe(key("create_room"));
  });
});
