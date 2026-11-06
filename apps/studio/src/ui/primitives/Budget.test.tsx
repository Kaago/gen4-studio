// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Budget } from "./Budget";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const css = readFileSync("src/ui/primitives/Budget.module.css", "utf8");

describe("Budget", () => {
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

  const meter = () => container.querySelector<HTMLElement>("[role=meter]")!;
  const nameOf = (el: HTMLElement) => document.getElementById(el.getAttribute("aria-labelledby") ?? "")?.textContent;
  const state = () => container.querySelector("[data-budget-state]")?.getAttribute("data-budget-state");

  it("budget_meter_names_used_and_max_and_its_state", () => {
    act(() => root.render(<Budget label="Modell" used={21452} max={61440} state="calm" unit="B" />));
    expect(nameOf(meter())).toBe("Modell");
    expect(meter().getAttribute("aria-valuenow")).toBe("21452");
    expect(meter().getAttribute("aria-valuemin")).toBe("0");
    expect(meter().getAttribute("aria-valuemax")).toBe("61440");
    expect(meter().getAttribute("aria-valuetext")).toBe("21.452 von 61.440 B");
    expect(container.textContent).toContain("21.452 von 61.440 B");
    expect(state()).toBe("calm");
    // Calm carries no badge.
    expect(container.textContent).not.toContain("Knapp");
    expect(container.textContent).not.toContain("Voll");

    act(() => root.render(<Budget label="Gebäude" used={31} max={32} state="tight" />));
    expect(state()).toBe("tight");
    expect(container.textContent).toContain("31 von 32");
    expect(container.textContent).toContain("Knapp");
    expect(meter().getAttribute("aria-valuetext")).toBe("31 von 32, Knapp");

    act(() => root.render(<Budget label="Terrain (BDHC)" used={40000} max={36864} state="full" unit="B" note="Vanilla höchstens 35.592 B" />));
    expect(state()).toBe("full");
    expect(container.textContent).toContain("Voll");
    expect(container.textContent).toContain("Vanilla höchstens 35.592 B");
    // Over the limit the bar stays full; the value still says how much.
    expect(meter().getAttribute("aria-valuenow")).toBe("40000");
    expect(container.querySelector<HTMLElement>("[data-budget-fill]")?.style.width).toBe("100%");
  });

  it("takes_its_own_words_and_a_technical_limit", () => {
    act(() => root.render(<Budget label="Texturen des Gebiets" used={37744} max={262144} state="calm" text="VRAM 37.744 von 262.144 B · 38 Texturen" tech="0x40000" />));
    expect(container.textContent).toContain("VRAM 37.744 von 262.144 B · 38 Texturen");
    expect(container.querySelector("[data-tech]")?.textContent).toBe("0x40000");
    expect(meter().getAttribute("aria-valuetext")).toBe("VRAM 37.744 von 262.144 B · 38 Texturen");
    expect(container.querySelector<HTMLElement>("[data-budget-fill]")?.style.width).toBe(`${(37744 / 262144) * 100}%`);
  });

  it("colours_the_fill_by_its_state_with_tokens", () => {
    expect(css).toMatch(/\.track\s*\{[^}]*background:\s*var\(--g4-flaeche-deck\)/);
    expect(css).toMatch(/\.fill\s*\{[^}]*background:\s*var\(--g4-akzent-grund\)/);
    expect(css).toMatch(/\.tight \.fill\s*\{[^}]*background:\s*var\(--g4-status-warnung\)/);
    expect(css).toMatch(/\.full \.fill\s*\{[^}]*background:\s*var\(--g4-status-fehler\)/);
  });
});
