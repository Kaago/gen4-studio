// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Atlas } from "./Atlas";
import type { AtlasProps } from "./Atlas";
import type { AreaId } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const LOCKED: AreaId[] = ["geschichten", "pokemon", "gegenstaende", "texte", "technik", "einstellungen"];
const HINTS: Partial<Record<AreaId, string>> = {
  geschichten: "Kommt mit Phase 3b.3",
  pokemon: "Kommt mit Phase 3b.3",
  gegenstaende: "Kommt mit Phase 3b.3",
  texte: "Kommt mit 3c.2",
  technik: "Kommt mit 3c.2",
  einstellungen: "Kommt mit 3c.2",
};

describe("Atlas", () => {
  let container: HTMLElement;
  let root: Root;
  const props = (over: Partial<AtlasProps> = {}): AtlasProps => ({ area: "welt", onArea: vi.fn(), onHelp: vi.fn(), locked: LOCKED, lockedHint: HINTS, density: "komfort", ...over });
  const show = (p: AtlasProps) => act(() => root.render(<Atlas {...p} />));
  const entry = (name: string) => container.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  it("is_a_navigation_and_marks_the_active_area_as_the_current_page", () => {
    show(props());
    expect(container.querySelector("nav")).not.toBeNull();
    expect(entry("Welt").getAttribute("aria-current")).toBe("page");
    expect(container.querySelectorAll("[aria-current=page]")).toHaveLength(1);
    show(props({ area: "start" }));
    expect(entry("Start").getAttribute("aria-current")).toBe("page");
    expect(entry("Welt").hasAttribute("aria-current")).toBe(false);
  });

  it("lists_the_areas_with_their_labels", () => {
    show(props());
    const labels = [...container.querySelectorAll("button")].map((b) => b.textContent);
    expect(labels).toEqual(["Start", "Welt", "Geschichten", "Pokémon", "Gegenstände", "Texte", "Technik", "Hilfe", "Einstellungen"]);
  });

  it("switches_to_a_free_area_on_click", () => {
    const p = props({ locked: LOCKED });
    show(p);
    act(() => entry("Start").click());
    expect(p.onArea).toHaveBeenCalledWith("start");
  });

  it("does_not_call_onArea_for_a_locked_area_and_marks_it_aria_disabled", () => {
    const p = props();
    show(p);
    for (const name of ["Geschichten", "Pokémon", "Gegenstände", "Texte", "Technik", "Einstellungen"]) {
      expect(entry(name).getAttribute("aria-disabled")).toBe("true");
      act(() => entry(name).click());
    }
    expect(p.onArea).not.toHaveBeenCalled();
    expect(entry("Welt").hasAttribute("aria-disabled")).toBe(false);
  });

  it("shows_the_hint_of_a_locked_area_as_a_tooltip", async () => {
    vi.useFakeTimers();
    show(props());
    await act(async () => {
      const b = entry("Texte");
      b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      b.dispatchEvent(new MouseEvent("mouseenter"));
      b.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Kommt mit 3c.2");
  });

  it("help_stays_clickable_and_calls_onHelp", () => {
    const p = props();
    show(p);
    expect(entry("Hilfe").hasAttribute("aria-disabled")).toBe(false);
    act(() => entry("Hilfe").click());
    expect(p.onHelp).toHaveBeenCalledOnce();
    expect(p.onArea).not.toHaveBeenCalled();
  });

  it("shows_symbols_only_in_the_compact_density_but_keeps_every_name_for_assistive_technology", () => {
    show(props({ density: "kompakt" }));
    const buttons = [...container.querySelectorAll("button")];
    expect(buttons).toHaveLength(9);
    for (const b of buttons) {
      expect(b.textContent).toBe("");
      expect(b.querySelector("svg")).not.toBeNull();
    }
    expect(buttons.map((b) => b.getAttribute("aria-label"))).toEqual(["Start", "Welt", "Geschichten", "Pokémon", "Gegenstände", "Texte", "Technik", "Hilfe", "Einstellungen"]);
    // The behaviour does not change with the density.
    expect(entry("Welt").getAttribute("aria-current")).toBe("page");
    expect(entry("Technik").getAttribute("aria-disabled")).toBe("true");
  });

  it("shows_the_labels_again_in_the_comfortable_density", () => {
    show(props({ density: "kompakt" }));
    show(props({ density: "komfort" }));
    expect(entry("Welt").textContent).toBe("Welt");
    expect(entry("Hilfe").textContent).toBe("Hilfe");
  });

  it("keeps_the_hint_of_a_locked_area_as_its_tooltip_in_the_compact_density", async () => {
    vi.useFakeTimers();
    show(props({ density: "kompakt" }));
    await act(async () => {
      const b = entry("Texte");
      b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      b.dispatchEvent(new MouseEvent("mouseenter"));
      b.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Kommt mit 3c.2");
  });
});
