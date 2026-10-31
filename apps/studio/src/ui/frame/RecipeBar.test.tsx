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

  it("lists_five_recipes_two_runnable_three_locked_with_3d5", () => {
    show();
    expect(bar().getAttribute("aria-orientation")).toBe("vertical");
    expect(bar().getAttribute("data-anchor")).toBe("left");
    expect(keys().map((b) => b.getAttribute("data-recipe"))).toEqual(["create_area", "create_room", "new_figure", "connect_doors", "new_trainer_here"]);
    // One row like the toolbar: a runnable recipe shows its name, a locked one only its symbol and the lock.
    expect(keys().map((b) => b.textContent)).toEqual(["Gebiet anlegen", "Raum anlegen", "", "", ""]);
    expect(keys().map((b) => b.disabled)).toEqual([false, false, true, true, true]);
    expect(keys().map((b) => b.hasAttribute("data-icon-only"))).toEqual([false, false, true, true, true]);
    // A locked key keeps its name as the accessible name and its reason as the description.
    expect(keys().map((b) => b.getAttribute("aria-label"))).toEqual([null, null, "Neue Figur", "Tür verbinden", "Neuer Trainer hier"]);
    for (const id of ["new_figure", "connect_doors", "new_trainer_here"]) {
      expect(key(id).querySelectorAll("svg")).toHaveLength(2);
      expect(document.getElementById(key(id).getAttribute("aria-describedby")!)?.textContent).toBe("Kommt mit 3d.5");
    }
    // Flat at the top or the bottom, like the toolbar it stands in for.
    show("bottom");
    expect(bar().getAttribute("aria-orientation")).toBe("horizontal");
    expect(bar().getAttribute("data-anchor")).toBe("bottom");
  });

  it("click_runs_the_recipe", () => {
    show();
    act(() => key("create_room").click());
    expect(onRecipe).toHaveBeenCalledExactlyOnceWith("create_room");
    act(() => key("new_figure").click());
    expect(onRecipe).toHaveBeenCalledOnce();
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
