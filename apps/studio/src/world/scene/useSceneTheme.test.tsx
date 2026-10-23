// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyTheme } from "../../ui/tokens/theme";
import { dunkel, hell } from "../../ui/tokens/tokens";
import type { SceneHost } from "./SceneHost";
import { useSceneTheme } from "./useSceneTheme";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function Probe({ refs }: { refs: { handle: { current: SceneHost | null } } }) {
  useSceneTheme(refs);
  return null;
}

/** The parts of the host the theme switch may touch, and the parts it must leave alone. */
function fakeHost() {
  const models: unknown[] = [];
  const content = { children: [] as unknown[] };
  const host = {
    disposed: false,
    models,
    content,
    setBackground: vi.fn(),
    render: vi.fn(),
  };
  return { host, models, content };
}

describe("useSceneTheme", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    applyTheme("hell");
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    delete document.documentElement.dataset.theme;
  });

  it("recolours_the_background_once_per_switch_and_renders_once_without_touching_models_or_content", () => {
    const { host, models, content } = fakeHost();
    const refs = { handle: { current: host as unknown as SceneHost } };
    act(() => root.render(<Probe refs={refs} />));
    act(() => applyTheme("dunkel"));
    expect(host.setBackground).toHaveBeenCalledTimes(1);
    expect(host.setBackground).toHaveBeenCalledWith(dunkel["flaeche/welt"]);
    expect(host.render).toHaveBeenCalledTimes(1);
    expect(host.models).toBe(models);
    expect(host.content).toBe(content);
    expect(models).toEqual([]);
    expect(content.children).toEqual([]);
    act(() => applyTheme("hell"));
    expect(host.setBackground).toHaveBeenLastCalledWith(hell["flaeche/welt"]);
    expect(host.setBackground).toHaveBeenCalledTimes(2);
  });

  it("ignores_a_choice_that_changes_nothing_and_a_disposed_or_missing_host", () => {
    const { host } = fakeHost();
    const refs = { handle: { current: host as unknown as SceneHost | null } };
    act(() => root.render(<Probe refs={refs} />));
    act(() => applyTheme("hell"));
    expect(host.setBackground).not.toHaveBeenCalled();
    host.disposed = true;
    act(() => applyTheme("dunkel"));
    expect(host.setBackground).not.toHaveBeenCalled();
    refs.handle.current = null;
    expect(() => act(() => applyTheme("hell"))).not.toThrow();
  });

  it("follows_a_system_change_while_the_theme_is_system", () => {
    const listeners: (() => void)[] = [];
    let dark = false;
    vi.stubGlobal("matchMedia", () => ({
      get matches() {
        return dark;
      },
      addEventListener: (_: string, cb: () => void) => listeners.push(cb),
      removeEventListener: () => {},
    }));
    try {
      applyTheme("system");
      const { host } = fakeHost();
      const refs = { handle: { current: host as unknown as SceneHost } };
      act(() => root.render(<Probe refs={refs} />));
      dark = true;
      act(() => listeners.forEach((cb) => cb()));
      expect(host.setBackground).toHaveBeenCalledTimes(1);
      expect(host.setBackground).toHaveBeenCalledWith(dunkel["flaeche/welt"]);
      expect(host.render).toHaveBeenCalledTimes(1);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
