// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { visibleWords } from "../../ui/visibleText";
import { headActions, chooseAction } from "./headTesting";
import { InspectorHead } from "./InspectorHead";
import type { InspectorHeadProps, InspectorKind } from "./InspectorHead";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("InspectorHead", () => {
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
  const show = (props: Partial<InspectorHeadProps> = {}) =>
    act(() => root.render(<InspectorHead kind="figure" title="Objekt 2 · ID 1" depth="bauen" {...props} />));
  const text = () => container.textContent ?? "";
  const eyebrow = () => container.querySelector("header[data-inspector-kind] [data-eyebrow]");
  const menuButton = () => container.querySelector<HTMLButtonElement>("button[aria-label='Aktionen']");

  it("shows_the_title_as_a_heading_and_the_ident_grey", () => {
    show({ ident: "Objekt 2 · Eventdatei 0390" });
    expect(container.querySelector("header[data-inspector-kind]")?.getAttribute("data-inspector-kind")).toBe("figure");
    expect(container.querySelector("h2")?.textContent).toBe("Objekt 2 · ID 1");
    expect(container.querySelector("[data-ident]")?.textContent).toBe("Objekt 2 · Eventdatei 0390");
  });

  it("shows_the_kind_as_an_eyebrow_with_its_symbol", () => {
    const words: Record<InspectorKind, string> = {
      figure: "Figur",
      door: "Tür",
      trigger: "Auslöser",
      sign: "Schild",
      building: "Gebäude",
      map: "Karte",
      area: "Gebiet",
      room: "Raum",
      place: "Ort",
      look: "Aussehen",
    };
    const symbols = new Set<string>();
    for (const [kind, word] of Object.entries(words) as [InspectorKind, string][]) {
      show({ kind });
      // Sentence case, as the design system writes it: no capitals spaced out.
      expect(eyebrow()?.textContent, kind).toBe(word);
      const symbol = eyebrow()?.querySelector("svg");
      expect(symbol, kind).not.toBeNull();
      expect(symbol?.getAttribute("aria-hidden")).toBe("true");
      symbols.add(symbol!.innerHTML);
    }
    // Each kind has a symbol of its own.
    expect(symbols.size).toBe(Object.keys(words).length);
  });

  it("says_header_in_tinkering_and_area_or_room_in_building", () => {
    show({ kind: "area", title: "Zweiblattdorf", ident: "Zweiblattdorf · T01" });
    expect(eyebrow()?.textContent).toBe("Gebiet");
    show({ kind: "room", title: "Haus 1", ident: "Zweiblattdorf · T01R0101" });
    expect(eyebrow()?.textContent).toBe("Raum");
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
    show({ kind: "area", title: "Zweiblattdorf", ident: "Header 0411 · T01", depth: "tuefteln" });
    expect(eyebrow()?.textContent).toBe("Header");
    show({ kind: "room", title: "Haus 1", depth: "tuefteln" });
    expect(eyebrow()?.textContent).toBe("Header");
    show({ kind: "place", title: "Zweiblattdorf", depth: "tuefteln" });
    expect(eyebrow()?.textContent).toBe("Header");
    show({ kind: "look", title: "Area-Data 0007", depth: "tuefteln" });
    expect(eyebrow()?.textContent).toBe("Area-Data");
    // The other kinds are the same words in both depths.
    show({ kind: "door", depth: "tuefteln" });
    expect(eyebrow()?.textContent).toBe("Tür");
  });

  it("lists_the_actions_in_a_menu_only_when_there_are_some", () => {
    show();
    expect(menuButton()).toBeNull();
    show({ actions: [] });
    expect(menuButton()).toBeNull();
    const onDelete = vi.fn();
    show({ actions: [{ label: "Löschen", onSelect: onDelete, detail: "Entf" }] });
    expect(menuButton()).not.toBeNull();
    expect(menuButton()?.textContent).toBe("");
    expect(headActions(container)).toEqual(["LöschenEntf"]);
    chooseAction(container, "Löschen");
    expect(onDelete).toHaveBeenCalledOnce();
  });

  it("shows_the_badges", () => {
    show();
    expect(text()).not.toContain("Ungespeichert");
    expect(text()).not.toContain("Gesperrt");
    expect(text()).not.toContain("Nicht rückgängig");
    show({ unsaved: true });
    expect(text()).toContain("Ungespeichert");
    expect(text()).not.toContain("Gesperrt");
    show({ locked: true });
    expect(text()).toContain("Gesperrt");
    expect(text()).not.toContain("Ungespeichert");
    show({ irreversible: true });
    expect(text()).toContain("Nicht rückgängig");
    // The hover views are gone, and with them the badge that marked them.
    expect(text()).not.toContain("Nur Ansicht");
  });

  it("shows_the_entity_state_badge", () => {
    for (const depth of ["erzaehlen", "bauen", "tuefteln"] as const) {
      show({ state: "deleted", unsaved: true, depth });
      // The state stands before „Ungespeichert“, the same in every depth.
      const state = container.querySelector<HTMLElement>("header [data-entity-state]");
      expect(state?.getAttribute("data-entity-state")).toBe("deleted");
      expect(state?.textContent).toBe("gelöscht");
      expect(state?.nextElementSibling?.textContent).toBe("Ungespeichert");
    }
    show({ state: "changed" });
    expect(container.querySelector("[data-entity-state]")?.textContent).toBe("geändert");
    show({ state: "new" });
    expect(container.querySelector("[data-entity-state]")?.textContent).toBe("neu");
    show({});
    expect(container.querySelector("[data-entity-state]")).toBeNull();
  });
});
