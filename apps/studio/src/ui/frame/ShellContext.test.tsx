// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyDensity, applyTheme } from "../tokens/theme";
import { ShellProvider, useShell } from "./ShellContext";
import type { ShellApi } from "./ShellContext";

vi.mock("../tokens/theme", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../tokens/theme")>();
  return { ...actual, applyTheme: vi.fn(actual.applyTheme), applyDensity: vi.fn(actual.applyDensity) };
});

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ShellProvider theme and density", () => {
  let container: HTMLElement;
  let root: Root;
  let shell: ShellApi;
  function Probe() {
    shell = useShell();
    return null;
  }
  beforeEach(() => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.dichte;
    vi.mocked(applyTheme).mockClear();
    vi.mocked(applyDensity).mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() =>
      root.render(
        <ShellProvider>
          <Probe />
        </ShellProvider>,
      ),
    );
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.dichte;
  });

  it("starts_on_the_stored_setting_system_by_default", () => {
    expect(shell.themeSetting).toBe("system");
    expect(shell.theme).toBe("hell");
    expect(shell.density).toBe("komfort");
  });

  it("set_theme_applies_the_theme_and_keeps_theme_and_setting_current", () => {
    act(() => shell.setTheme("dunkel"));
    expect(applyTheme).toHaveBeenCalledWith("dunkel");
    expect(document.documentElement.dataset.theme).toBe("dunkel");
    expect(shell.theme).toBe("dunkel");
    expect(shell.themeSetting).toBe("dunkel");
    act(() => shell.setTheme("system"));
    expect(applyTheme).toHaveBeenLastCalledWith("system");
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(shell.theme).toBe("hell");
    expect(shell.themeSetting).toBe("system");
  });

  it("toggle_theme_fixes_the_setting_to_the_theme_it_switched_to", () => {
    act(() => shell.toggleTheme());
    expect(shell.theme).toBe("dunkel");
    expect(shell.themeSetting).toBe("dunkel");
  });

  it("set_density_applies_the_density_and_keeps_it_current", () => {
    act(() => shell.setDensity("kompakt"));
    expect(applyDensity).toHaveBeenCalledWith("kompakt");
    expect(document.documentElement.dataset.dichte).toBe("kompakt");
    expect(shell.density).toBe("kompakt");
    act(() => shell.toggleDensity());
    expect(shell.density).toBe("komfort");
  });

  it("remembers_the_toolbar_anchor", () => {
    expect(shell.state.toolbar).toBe("left");
    act(() => shell.setToolbar("bottom"));
    expect(shell.state.toolbar).toBe("bottom");
    expect(JSON.parse(localStorage.getItem("g4.shell.v1")!).toolbar).toBe("bottom");
  });
});
