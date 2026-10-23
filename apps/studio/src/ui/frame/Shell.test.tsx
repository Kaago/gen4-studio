// @vitest-environment jsdom
import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Shell } from "./Shell";
import type { ShellProps } from "./Shell";
import type { AreaId } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Shell", () => {
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

  const areas: ShellProps["areas"] = {
    welt: { list: <p data-probe="welt-list">Explorer</p>, deck: <p data-probe="welt-deck">Weltdeck</p> },
    texte: { list: <p data-probe="texte-list">Textarchive</p>, deck: <p data-probe="texte-deck">Textdeck</p> },
  };
  const render = (area: AreaId, pane?: ReactNode, given: ShellProps["areas"] = areas) =>
    act(() => root.render(<Shell title="Titel" atlas="Atlas" area={area} areas={given} pane={pane} status="Status" />));
  /** The column of `area` that holds the list or the deck. */
  const column = (area: AreaId, part: "list" | "deck") => container.querySelector<HTMLElement>(`[data-area="${area}"][data-part="${part}"]`);
  const probe = (name: string) => container.querySelector<HTMLElement>(`[data-probe="${name}"]`);

  it("renders_every_area_it_is_given_and_hides_all_but_the_active_one", () => {
    render("welt");
    for (const name of ["welt-list", "welt-deck", "texte-list", "texte-deck"]) expect(probe(name)).not.toBeNull();
    expect(column("welt", "list")!.hidden).toBe(false);
    expect(column("welt", "deck")!.hidden).toBe(false);
    expect(column("texte", "list")!.hidden).toBe(true);
    expect(column("texte", "deck")!.hidden).toBe(true);
    // The deck of an area is the main region; the list sits before it.
    expect(column("welt", "deck")!.tagName).toBe("MAIN");
  });

  it("keeps_an_area_mounted_while_another_one_is_active", () => {
    render("welt");
    const deck = probe("welt-deck");
    render("texte");
    expect(column("texte", "list")!.hidden).toBe(false);
    expect(column("texte", "deck")!.hidden).toBe(false);
    expect(column("welt", "list")!.hidden).toBe(true);
    expect(column("welt", "deck")!.hidden).toBe(true);
    render("welt");
    expect(probe("welt-deck")).toBe(deck);
  });

  it("covers_every_area_with_a_pane_and_keeps_them_mounted", () => {
    render("start", <p data-probe="pane">Willkommen</p>);
    expect(probe("pane")).not.toBeNull();
    for (const area of ["welt", "texte"] as const) {
      expect(column(area, "list")!.hidden).toBe(true);
      expect(column(area, "deck")!.hidden).toBe(true);
    }
    expect(probe("welt-deck")).not.toBeNull();
  });

  it("an_area_without_a_list_has_only_its_deck_and_an_area_not_given_is_not_there", () => {
    render("texte", undefined, { texte: { deck: <p>Textdeck</p> } });
    expect(column("texte", "list")).toBeNull();
    expect(column("texte", "deck")!.hidden).toBe(false);
    expect(column("welt", "deck")).toBeNull();
  });
});
