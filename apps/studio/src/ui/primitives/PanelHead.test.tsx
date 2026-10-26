// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PanelHead } from "./PanelHead";
import type { PanelHeadSwitcher } from "./PanelHead";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PanelHead", () => {
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

  const press = (el: HTMLElement) =>
    act(() => {
      el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      el.click();
    });
  const radios = () => [...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
  const switcher = (onChange: (id: string) => void): PanelHeadSwitcher => ({
    value: "pokedex",
    onChange,
    items: [
      { id: "pokedex", label: "Pokédex", count: 508, shortcut: "⌘1" },
      { id: "trainer", label: "Trainer", disabled: true, hint: "kommt mit Phase 3b.3b", shortcut: "⌘2" },
      { id: "moves", label: "Attacken", count: 471, shortcut: "⌘4" },
    ],
  });

  it("shows_the_title_as_a_heading_without_a_switcher", () => {
    act(() => root.render(<PanelHead title="Orte" subtitle="Platin" />));
    expect(container.querySelector("h2")?.textContent).toBe("Orte");
    expect(container.querySelector("button")).toBeNull();
  });

  it("shows_no_heading_without_a_title_but_keeps_subtitle_and_actions", () => {
    act(() => root.render(<PanelHead subtitle="2 Fehler" actions={<button type="button">Neu</button>} />));
    expect(container.querySelector("h2")).toBeNull();
    expect(container.querySelector("header")?.textContent).toBe("2 FehlerNeu");
  });

  it("with_a_switcher_the_heading_is_a_menu_button_naming_the_current_part", () => {
    act(() => root.render(<PanelHead title="Pokémon" switcher={switcher(() => {})} />));
    const button = container.querySelector<HTMLElement>("h2 button")!;
    expect(button.textContent).toBe("Pokédex");
    expect(button.getAttribute("aria-haspopup")).toBe("menu");
    press(button);
    expect(radios().map((r) => r.textContent)).toEqual(["Pokédex508⌘1", "Trainer, kommt mit Phase 3b.3b⌘2", "Attacken471⌘4"]);
    expect(radios().map((r) => r.getAttribute("aria-checked"))).toEqual(["true", "false", "false"]);
  });

  it("choosing_a_part_calls_on_change_and_a_locked_one_does_not", () => {
    const onChange = vi.fn();
    act(() => root.render(<PanelHead title="Pokémon" switcher={switcher(onChange)} />));
    press(container.querySelector<HTMLElement>("h2 button")!);
    const locked = radios()[1]!;
    expect(locked.getAttribute("aria-disabled") === "true" || locked.hasAttribute("data-disabled")).toBe(true);
    act(() => locked.click());
    expect(onChange).not.toHaveBeenCalled();
    act(() => radios()[2]!.click());
    expect(onChange).toHaveBeenCalledExactlyOnceWith("moves");
  });
});
