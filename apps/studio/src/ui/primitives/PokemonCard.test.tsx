// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PokemonCard } from "./PokemonCard";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PokemonCard", () => {
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
  const card = () => container.querySelector<HTMLElement>("[role=button]")!;

  it("is_a_button_with_icon_name_level_and_item_named_by_its_label", () => {
    const onClick = vi.fn();
    act(() =>
      root.render(<PokemonCard state="idle" species={396} name="Staralili" level={12} item="Trank" iconUrl="blob:mon" onClick={onClick} ariaLabel="Platz 1: Staralili, Level 12" slot={1} dataAttrs={{ "data-slot": 0 }} />),
    );
    expect(card().getAttribute("aria-label")).toBe("Platz 1: Staralili, Level 12");
    expect(card().getAttribute("aria-pressed")).toBe("false");
    expect(card().getAttribute("data-state")).toBe("idle");
    expect(card().getAttribute("data-species")).toBe("396");
    expect(card().getAttribute("data-slot")).toBe("0");
    expect(card().tabIndex).toBe(0);
    expect(card().textContent).toContain("Staralili");
    expect(card().textContent).toContain("Lv. 12");
    expect(card().textContent).toContain("Trank");
    expect(card().querySelector("img")?.getAttribute("src")).toBe("blob:mon");
    act(() => card().click());
    expect(onClick).toHaveBeenCalledTimes(1);
    pressKey(card(), "Enter");
    pressKey(card(), " ");
    expect(onClick).toHaveBeenCalledTimes(3);
  });

  it("marks_the_selected_card_as_pressed", () => {
    act(() => root.render(<PokemonCard state="selected" species={396} name="Staralili" level={12} onClick={() => {}} ariaLabel="Platz 1: Staralili" />));
    expect(card().getAttribute("aria-pressed")).toBe("true");
    expect(card().getAttribute("data-state")).toBe("selected");
    // Without an item, no item line.
    expect(card().textContent).not.toContain("Trank");
  });

  it("says_which_place_is_empty", () => {
    act(() => root.render(<PokemonCard state="empty" slot={4} onClick={() => {}} ariaLabel="Platz 4 leer, Pokémon hinzufügen" />));
    expect(card().textContent).toBe("Platz 4 leer");
    expect(card().getAttribute("data-state")).toBe("empty");
    expect(card().querySelector("img, [data-sprite]")).toBeNull();
  });

  it("does_nothing_when_disabled", () => {
    const onClick = vi.fn();
    act(() => root.render(<PokemonCard state="idle" name="Staralili" level={5} onClick={onClick} ariaLabel="Platz 1" disabled />));
    expect(card().getAttribute("aria-disabled")).toBe("true");
    act(() => card().click());
    pressKey(card(), "Enter");
    expect(onClick).not.toHaveBeenCalled();
  });
});
