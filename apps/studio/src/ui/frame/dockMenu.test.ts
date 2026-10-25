// @vitest-environment node
import type { ContextMenuItem, ContextMenuItemConfig, IDockviewPanel } from "dockview-react";
import { describe, expect, it, vi } from "vitest";
import { tabMenuItems } from "./dockMenu";

/** A panel as the menu sees it: its id, where its group is and whether that group is maximized. */
function panel(id: string, { floating = false, maximized = false } = {}) {
  const api = {
    location: { type: floating ? "floating" : "grid" },
    isMaximized: () => maximized,
    maximize: vi.fn(),
    exitMaximized: vi.fn(),
  };
  return { id, api } as unknown as IDockviewPanel & { api: typeof api };
}
const actions = () => ({ float: vi.fn(), dock: vi.fn(), reset: vi.fn() });
/** The labels of the items, a separator as "—". */
const labels = (items: ContextMenuItem[]) => items.map((i) => (i === "separator" ? "—" : (i as ContextMenuItemConfig).label));
const item = (items: ContextMenuItem[], label: string) => items.find((i) => typeof i === "object" && i.label === label) as ContextMenuItemConfig;

describe("tab menu", () => {
  it("offers_maximize_float_dock_and_reset_in_german", () => {
    const docked = panel("inspector");
    const run = actions();
    const items = tabMenuItems({ panel: docked }, run);
    expect(labels(items)).toEqual(["Maximieren", "Schweben lassen", "—", "Anordnung zurücksetzen"]);
    item(items, "Maximieren").action!();
    expect(docked.api.maximize).toHaveBeenCalledOnce();
    item(items, "Schweben lassen").action!();
    expect(run.float).toHaveBeenCalledWith("inspector");
    item(items, "Anordnung zurücksetzen").action!();
    expect(run.reset).toHaveBeenCalledOnce();
    // A maximized group offers to restore it.
    const maximized = panel("script", { maximized: true });
    const again = tabMenuItems({ panel: maximized }, run);
    expect(labels(again)[0]).toBe("Wiederherstellen");
    item(again, "Wiederherstellen").action!();
    expect(maximized.api.exitMaximized).toHaveBeenCalledOnce();
  });

  it("offers_only_maximize_on_a_view_tab", () => {
    for (const id of ["world", "matrix", "worldmap", "textView", "pokemonView", "itemView"]) {
      expect(labels(tabMenuItems({ panel: panel(id) }, actions()))).toEqual(["Maximieren"]);
    }
  });

  it("disables_float_for_a_floating_panel", () => {
    const run = actions();
    const items = tabMenuItems({ panel: panel("inspector", { floating: true }) }, run);
    expect(labels(items)).toEqual(["Maximieren", "Schweben lassen", "Zurück ins Dock", "—", "Anordnung zurücksetzen"]);
    expect(item(items, "Schweben lassen").disabled).toBe(true);
    // Only a group in the grid can be maximized.
    expect(item(items, "Maximieren").disabled).toBe(true);
    item(items, "Zurück ins Dock").action!();
    expect(run.dock).toHaveBeenCalledWith("inspector");
  });
});
