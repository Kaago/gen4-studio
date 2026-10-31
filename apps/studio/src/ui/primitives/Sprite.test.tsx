// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Sprite } from "./Sprite";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Sprite", () => {
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

  it("shows_the_picture_at_its_size_named_by_the_label", () => {
    act(() => root.render(<Sprite url="blob:icon-17" label="Trank" size={32} area="gegenstaende" />));
    const img = container.querySelector("img")!;
    expect(img.getAttribute("src")).toBe("blob:icon-17");
    expect(img.getAttribute("alt")).toBe("Trank");
    expect(img.getAttribute("width")).toBe("32");
    expect(img.getAttribute("height")).toBe("32");
    expect(container.querySelector("[data-sprite]")?.getAttribute("data-sprite")).toBe("picture");
  });

  it("without_a_picture_shows_the_first_letter_as_a_named_placeholder", () => {
    act(() => root.render(<Sprite url={null} label="staravia" size={24} area="pokemon" />));
    expect(container.querySelector("img")).toBeNull();
    const placeholder = container.querySelector<HTMLElement>("[role=img]")!;
    expect(placeholder.getAttribute("aria-label")).toBe("staravia");
    expect(placeholder.textContent).toBe("S");
    expect(placeholder.getAttribute("data-sprite")).toBe("placeholder");
    expect(placeholder.getAttribute("data-area")).toBe("pokemon");
  });

  it("a_decorative_sprite_is_hidden_from_assistive_technology", () => {
    act(() => root.render(<Sprite url="blob:icon-17" label="Trank" size={24} area="gegenstaende" decorative />));
    expect(container.querySelector("img")!.getAttribute("alt")).toBe("");
    act(() => root.render(<Sprite url={null} label="Trank" size={24} area="gegenstaende" decorative />));
    const placeholder = container.querySelector<HTMLElement>("[data-sprite=placeholder]")!;
    expect(placeholder.getAttribute("role")).toBeNull();
    expect(placeholder.getAttribute("aria-label")).toBeNull();
    expect(placeholder.getAttribute("aria-hidden")).toBe("true");
    // The letter still shows.
    expect(placeholder.textContent).toBe("T");
  });

  it("draws_the_80_pixel_size_of_a_battle_picture", () => {
    act(() => root.render(<Sprite url="blob:battle-397" label="Staravia" size={80} area="pokemon" />));
    const img = container.querySelector("img")!;
    expect(img.getAttribute("width")).toBe("80");
    expect(img.getAttribute("height")).toBe("80");
    act(() => root.render(<Sprite url={null} label="Staravia" size={80} area="pokemon" />));
    expect(container.querySelector("[data-sprite=placeholder]")?.textContent).toBe("S");
  });
});
