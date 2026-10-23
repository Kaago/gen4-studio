// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  applyDensity,
  applyTheme,
  effectiveTheme,
  onThemeChange,
  readDensity,
  readTheme,
} from "./theme";

/** A controllable `matchMedia` for "(prefers-color-scheme: dark)"; `fire` simulates an OS switch. */
function stubMatchMedia(initialDark: boolean) {
  let dark = initialDark;
  const listeners = new Set<(e: { matches: boolean }) => void>();
  vi.stubGlobal("matchMedia", (query: string) => ({
    get matches() {
      return query.includes("dark") ? dark : false;
    },
    media: query,
    addEventListener: (_: string, cb: (e: { matches: boolean }) => void) => listeners.add(cb),
    removeEventListener: (_: string, cb: (e: { matches: boolean }) => void) => listeners.delete(cb),
  }));
  return {
    fire(nextDark: boolean) {
      dark = nextDark;
      for (const cb of [...listeners]) cb({ matches: dark });
    },
    listenerCount: () => listeners.size,
  };
}

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
  delete document.documentElement.dataset.dichte;
});
afterEach(() => vi.unstubAllGlobals());

describe("theme", () => {
  it("defaults_to_system_and_writes_the_attribute_only_for_a_fixed_theme", () => {
    stubMatchMedia(false);
    expect(readTheme()).toBe("system");
    applyTheme("dunkel");
    expect(document.documentElement.dataset.theme).toBe("dunkel");
    expect(localStorage.getItem("g4.theme")).toBe("dunkel");
    expect(readTheme()).toBe("dunkel");
    applyTheme("system");
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(readTheme()).toBe("system");
  });

  it("ignores_a_stored_value_that_is_not_a_theme", () => {
    stubMatchMedia(false);
    localStorage.setItem("g4.theme", "violett");
    expect(readTheme()).toBe("system");
  });

  it("effective_theme_follows_the_system_only_without_a_fixed_theme", () => {
    stubMatchMedia(true); // prefers dark
    applyTheme("system");
    expect(effectiveTheme()).toBe("dunkel");
    applyTheme("hell");
    expect(effectiveTheme()).toBe("hell");
  });

  it("on_theme_change_fires_once_per_switch_and_per_system_change", () => {
    const mm = stubMatchMedia(false);
    applyTheme("system");
    const cb = vi.fn();
    const off = onThemeChange(cb);

    applyTheme("dunkel"); // hell -> dunkel
    expect(cb).toHaveBeenCalledTimes(1);
    expect(cb).toHaveBeenLastCalledWith("dunkel");

    applyTheme("dunkel"); // no effective change
    expect(cb).toHaveBeenCalledTimes(1);

    mm.fire(true); // system change while a fixed theme is set: silent
    expect(cb).toHaveBeenCalledTimes(1);

    applyTheme("system"); // dunkel -> system, which is dark now: no effective change
    expect(cb).toHaveBeenCalledTimes(1);

    mm.fire(false); // system change while on "system"
    expect(cb).toHaveBeenCalledTimes(2);
    expect(cb).toHaveBeenLastCalledWith("hell");

    applyTheme("dunkel"); // hell -> dunkel
    expect(cb).toHaveBeenCalledTimes(3);

    off();
    applyTheme("hell");
    applyTheme("system");
    mm.fire(true);
    expect(cb).toHaveBeenCalledTimes(3);
    expect(mm.listenerCount()).toBe(0);
  });

  it("delivers_each_switch_once_to_every_subscriber", () => {
    const mm = stubMatchMedia(false);
    applyTheme("system");
    const a = vi.fn();
    const b = vi.fn();
    const offA = onThemeChange(a);
    const offB = onThemeChange(b);
    mm.fire(true);
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).toHaveBeenCalledTimes(1);
    offA();
    mm.fire(false);
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).toHaveBeenCalledTimes(2);
    offB();
  });
});

describe("density", () => {
  it("density_defaults_to_komfort_and_sets_data_dichte", () => {
    expect(readDensity()).toBe("komfort");
    applyDensity("kompakt");
    expect(document.documentElement.dataset.dichte).toBe("kompakt");
    expect(localStorage.getItem("g4.density")).toBe("kompakt");
    expect(readDensity()).toBe("kompakt");
    applyDensity("komfort");
    expect(document.documentElement.dataset.dichte).toBe("komfort");
    expect(readDensity()).toBe("komfort");
  });
});
