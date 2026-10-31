// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DSPokedexProfile } from "./DSPokedexProfile";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DSPokedexProfile", () => {
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

  const group = () => container.querySelector<HTMLElement>("[role=group]");
  const texts = (text: string) => [...container.querySelectorAll("*")].filter((e) => e.children.length === 0 && e.textContent === text);

  it("names_the_profile_and_shows_the_three_digit_number_and_the_name", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} />));
    expect(group()?.getAttribute("aria-label")).toBe("Pokédex-Profil Staravia");
    expect(container.querySelector("[data-ds-profile]")).not.toBeNull();
    expect(texts("Nr. 397")).toHaveLength(1);
    expect(texts("Staravia")).toHaveLength(1);
    act(() => root.render(<DSPokedexProfile number={7} name="Schiggy" types={[11]} spriteUrl={null} scale={1} />));
    expect(texts("Nr. 007")).toHaveLength(1);
  });

  it("shows_a_type_badge_per_type", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} />));
    expect([...container.querySelectorAll("[data-type]")].map((e) => e.getAttribute("data-type"))).toEqual(["0", "2"]);
    act(() => root.render(<DSPokedexProfile number={393} name="Plinfa" types={[11]} spriteUrl={null} scale={1} />));
    expect([...container.querySelectorAll("[data-type]")].map((e) => e.getAttribute("data-type"))).toEqual(["11"]);
    // The game stores a single type twice: one badge.
    act(() => root.render(<DSPokedexProfile number={393} name="Plinfa" types={[11, 11]} spriteUrl={null} scale={1} />));
    expect([...container.querySelectorAll("[data-type]")].map((e) => e.getAttribute("data-type"))).toEqual(["11"]);
  });

  it("shows_folgt_for_missing_category_height_weight_and_text", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} />));
    expect(texts("folgt")).toHaveLength(3);
    expect(texts("Pokédex-Text folgt")).toHaveLength(1);
    expect(group()?.textContent).toContain("Kategorie");
    expect(group()?.textContent).toContain("Größe");
    expect(group()?.textContent).toContain("Gewicht");
  });

  it("shows_given_category_height_weight_and_text", () => {
    act(() =>
      root.render(
        <DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} category="Star" height="0,6 m" weight="15,5 kg" text="Lebt in Wäldern." scale={1} />,
      ),
    );
    expect(texts("folgt")).toHaveLength(0);
    expect(texts("Star")).toHaveLength(1);
    expect(texts("0,6 m")).toHaveLength(1);
    expect(texts("15,5 kg")).toHaveLength(1);
    expect(texts("Lebt in Wäldern.")).toHaveLength(1);
    expect(texts("Pokédex-Text folgt")).toHaveLength(0);
  });

  it("without_a_sprite_shows_the_placeholder_and_with_one_the_picture", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} />));
    const placeholder = container.querySelector("[data-sprite=placeholder]");
    expect(placeholder?.textContent).toBe("S");
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl="blob:battle-397" scale={1} />));
    const img = container.querySelector("img")!;
    expect(img.getAttribute("src")).toBe("blob:battle-397");
    expect(img.getAttribute("width")).toBe("80");
  });

  it("carries_the_full_name_in_title_on_a_name_line_that_ends_with_an_ellipsis", () => {
    const name = "Burmadame, Lumpenumhang";
    act(() => root.render(<DSPokedexProfile number={500} name={name} types={[0]} spriteUrl={null} scale={1} decorative />));
    const el = texts(name)[0] as HTMLElement;
    expect(el.getAttribute("title")).toBe(name);
    expect(el.className).toMatch(/name/);
  });

  it("draws_at_the_given_scale", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={2} />));
    expect(group()?.style.getPropertyValue("--ds-scale")).toBe("2");
  });

  it("a_decorative_profile_hides_the_group", () => {
    act(() => root.render(<DSPokedexProfile number={397} name="Staravia" types={[0, 2]} spriteUrl={null} scale={1} decorative />));
    expect(group()).toBeNull();
    expect(container.querySelector("[aria-label^='Pokédex-Profil']")).toBeNull();
    expect(container.firstElementChild?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector("[data-ds-profile]")).not.toBeNull();
  });
});
