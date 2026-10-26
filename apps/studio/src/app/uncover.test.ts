// @vitest-environment jsdom
import type { DockviewApi } from "dockview-react";
import { describe, expect, it, vi } from "vitest";
import { uncover } from "./useSceneSync";

/** A dock whose group with `maximized` is maximized; the 3D view's panel says whether its own group is the one. */
function dock({ maximized, ownMaximized }: { maximized: boolean; ownMaximized: boolean }) {
  const panel = { api: { isMaximized: () => ownMaximized }, focus: vi.fn() };
  const api = { getPanel: (id: string) => (id === "world" ? panel : undefined), hasMaximizedGroup: () => maximized, exitMaximizedGroup: vi.fn() };
  return { api: api as unknown as DockviewApi & typeof api, panel };
}

describe("uncover", () => {
  it("restores_a_maximized_group_that_hides_the_3d_view_and_brings_the_view_forward", () => {
    // „Als Vorschaubild“ on the 3D view while another group fills the dock: the view must show to be captured.
    const hidden = dock({ maximized: true, ownMaximized: false });
    expect(uncover(hidden.api, "world")).toBe(true);
    expect(hidden.api.exitMaximizedGroup).toHaveBeenCalledOnce();
    expect(hidden.panel.focus).toHaveBeenCalledOnce();
    // The view's own group maximized, or nothing maximized: it only comes forward.
    for (const state of [{ maximized: true, ownMaximized: true }, { maximized: false, ownMaximized: false }]) {
      const d = dock(state);
      uncover(d.api, "world");
      expect(d.api.exitMaximizedGroup).not.toHaveBeenCalled();
      expect(d.panel.focus).toHaveBeenCalledOnce();
    }
    expect(uncover(hidden.api, "inspector")).toBe(false);
  });
});
