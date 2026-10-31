// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RecipeButton } from "./RecipeButton";
import type { RecipeButtonProps } from "./RecipeButton";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("RecipeButton", () => {
  let container: HTMLElement;
  let root: Root;
  const onClick = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onClick.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = (over: Partial<RecipeButtonProps> = {}) =>
    act(() => root.render(<RecipeButton id="create_room" label="Raum anlegen" icon="room" area="welt" hint="Kopiert einen Raum mit Tür nach draußen." onClick={onClick} {...over} />));
  const key = () => container.querySelector<HTMLButtonElement>("button[data-recipe]")!;

  it("has_name_icon_area_and_data_recipe", () => {
    show();
    expect(key().getAttribute("data-recipe")).toBe("create_room");
    // The name is visible (a recipe is a word, not a letter) and is the accessible name.
    expect(key().textContent).toBe("Raum anlegen");
    expect(key().getAttribute("data-area")).toBe("welt");
    expect(key().querySelector("svg")).not.toBeNull();
    expect(key().disabled).toBe(false);
    expect(key().hasAttribute("aria-pressed")).toBe(false);
    act(() => key().click());
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("locked_with_a_reason_is_disabled_and_says_why", () => {
    show({ id: "new_figure", label: "Neue Figur", icon: "figure", reason: "Kommt mit 3d.5" });
    expect(key().disabled).toBe(true);
    expect(key().getAttribute("data-locked")).toBe("");
    // Never colour alone: a second symbol, the lock, stands after the name.
    expect(key().querySelectorAll("svg")).toHaveLength(2);
    const described = key().getAttribute("aria-describedby")!;
    expect(document.getElementById(described)?.textContent).toBe("Kommt mit 3d.5");
    act(() => key().click());
    expect(onClick).not.toHaveBeenCalled();
    // A null reason is a recipe that runs.
    show({ reason: null });
    expect(key().disabled).toBe(false);
    expect(key().hasAttribute("aria-describedby")).toBe(false);
  });

  it("pressed_is_aria_pressed", () => {
    show({ pressed: true });
    expect(key().getAttribute("aria-pressed")).toBe("true");
    show({ pressed: false });
    expect(key().getAttribute("aria-pressed")).toBe("false");
  });

  it("icon_only_shows_the_symbol_and_keeps_the_name_as_accessible_name", () => {
    show({ id: "new_figure", label: "Neue Figur", icon: "figure", reason: "Kommt mit 3d.5", iconOnly: true });
    expect(key().getAttribute("data-icon-only")).toBe("");
    expect(key().textContent).toBe("");
    expect(key().getAttribute("aria-label")).toBe("Neue Figur");
    // The symbol and the lock stay; the reason is still the description.
    expect(key().querySelectorAll("svg")).toHaveLength(2);
    expect(document.getElementById(key().getAttribute("aria-describedby")!)?.textContent).toBe("Kommt mit 3d.5");
    // Without it the name is visible and no aria-label doubles it.
    show();
    expect(key().hasAttribute("data-icon-only")).toBe(false);
    expect(key().hasAttribute("aria-label")).toBe(false);
  });

  it("takes_the_small_size", () => {
    show({ size: "s" });
    expect(key().getAttribute("data-size")).toBe("s");
    show();
    expect(key().getAttribute("data-size")).toBe("m");
  });
});
