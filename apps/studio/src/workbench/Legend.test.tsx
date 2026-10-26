// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { visibleWords } from "../ui/visibleText";
import { legendEntries } from "../world/legend";
import { Legend } from "./Legend";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Legend", () => {
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

  it("is_a_button_named_legende_that_opens_the_names_of_all_colours", async () => {
    act(() => root.render(<Legend />));
    const button = [...container.querySelectorAll("button")].find((b) => b.textContent === "Legende")!;
    expect(button).toBeDefined();
    await act(async () => button.click());
    const popup = document.body.querySelector<HTMLElement>("[role=dialog]")!;
    expect(popup).not.toBeNull();
    const names = [...popup.querySelectorAll("li")].map((li) => li.textContent);
    expect(names).toEqual(["Blockiert", "Wasser", "Hohes Gras", "Vorsprung", "Eis", "Spezial", "Unbekannt", "Figur", "Tür", "Auslöser", "Schild", "Gegenstand"]);
  });

  it("says_no_raster_matrix_or_header_in_building", async () => {
    act(() => root.render(<Legend />));
    await act(async () => container.querySelector("button")!.click());
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    expect(visibleWords(document.body.querySelector("[role=dialog]")!)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("shows_a_swatch_in_the_colour_the_scene_uses_next_to_every_name", async () => {
    act(() => root.render(<Legend />));
    await act(async () => container.querySelector("button")!.click());
    const items = [...document.body.querySelectorAll<HTMLElement>("[role=dialog] li")];
    const { terrain, events } = legendEntries();
    const colours = [...terrain, ...events].map((e) => e.color.toLowerCase());
    const swatches = items.map((li) => li.querySelector<HTMLElement>("span[aria-hidden=true]")!.style.background.toLowerCase());
    // jsdom normalises `#rrggbb` to `rgb(r, g, b)`; compare both ways.
    const rgb = (hex: string) => `rgb(${[1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(", ")})`;
    expect(swatches).toEqual(colours.map(rgb));
  });
});
