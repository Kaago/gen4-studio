// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { StatusLine } from "./StatusLine";
import type { StatusLineProps } from "./StatusLine";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("StatusLine", () => {
  let container: HTMLElement;
  let root: Root;
  const props = (over: Partial<StatusLineProps> = {}): StatusLineProps => ({
    left: null,
    middle: null,
    right: null,
    bridge: "none",
    density: "komfort",
    onToggleDensity: vi.fn(),
    build: { builtAt: null, building: false },
    ...over,
  });
  const show = (p: StatusLineProps) => act(() => root.render(<StatusLine {...p} />));

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

  it("names_the_browser_mode_and_a_missing_bridge_and_stays_silent_in_the_app", () => {
    show(props({ bridge: "ok" }));
    expect(container.textContent).toContain("Browser-Modus (Brücke)");
    show(props({ bridge: "missing" }));
    expect(container.textContent).toContain("Brücke nicht erreichbar");
    expect(container.textContent).not.toContain("Browser-Modus");
    show(props({ bridge: "none" }));
    expect(container.textContent).not.toContain("Brücke");
  });

  it("says_whether_and_how_long_ago_the_rom_was_built", () => {
    show(props());
    expect(container.textContent).toContain("Noch nicht gebaut");
    show(props({ build: { builtAt: Date.now() - 4 * 60_000 - 5_000, building: false } }));
    expect(container.textContent).toContain("Gebaut vor 4 Minuten");
    expect(container.textContent).not.toContain("Noch nicht gebaut");
    show(props({ build: { builtAt: Date.now() - 2 * 3_600_000 - 5_000, building: false } }));
    expect(container.textContent).toContain("Gebaut vor 2 Stunden");
  });

  it("shows_the_build_while_it_runs_even_when_an_older_one_exists", () => {
    show(props({ build: { builtAt: Date.now() - 60_000, building: true } }));
    expect(container.textContent).toContain("Baut …");
    expect(container.textContent).not.toContain("Gebaut");
  });

  it("moves_the_age_on_while_the_window_stays_open", () => {
    vi.useFakeTimers();
    const builtAt = Date.now();
    show(props({ build: { builtAt, building: false } }));
    expect(container.textContent).toContain("Gebaut gerade eben");
    act(() => {
      vi.advanceTimersByTime(3 * 60_000);
    });
    expect(container.textContent).toContain("Gebaut vor 3 Minuten");
  });

  it("shows_the_slots_and_switches_the_density", () => {
    const p = props({ left: <span>links</span>, middle: <span>mitte</span>, right: <span>rechts</span> });
    show(p);
    expect(container.textContent).toContain("links");
    expect(container.textContent).toContain("mitte");
    expect(container.textContent).toContain("rechts");
    const density = [...container.querySelectorAll("button")].find((b) => b.textContent === "Komfort")!;
    act(() => density.click());
    expect(p.onToggleDensity).toHaveBeenCalledOnce();
    show(props({ density: "kompakt" }));
    expect([...container.querySelectorAll("button")].some((b) => b.textContent === "Kompakt")).toBe(true);
  });
});
