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
  it("offers_als_vorschaubild_on_the_3d_and_weltkarte_tabs_after_maximize_and_a_separator", () => {
    const setPreview = vi.fn();
    const previewLocked = vi.fn((id: string) => (id === "worldmap" ? "Erst eine Karte wählen" : null));
    const run = { ...actions(), setPreview, previewLocked };
    const world = tabMenuItems({ panel: panel("world") }, run);
    expect(labels(world)).toEqual(["Maximieren", "—", "Als Vorschaubild"]);
    expect(item(world, "Als Vorschaubild").disabled).toBe(false);
    item(world, "Als Vorschaubild").action!();
    expect(setPreview).toHaveBeenCalledWith("world");
    // Without a chosen map the world map's item is locked and says why.
    const map = tabMenuItems({ panel: panel("worldmap", { maximized: true }) }, run);
    expect(labels(map)).toEqual(["Wiederherstellen", "—", "Als Vorschaubild"]);
    const locked = item(map, "Als Vorschaubild") as ContextMenuItemConfig & { hint?: string };
    expect(locked.disabled).toBe(true);
    expect(locked.hint).toBe("Erst eine Karte wählen");
    // With a chosen map it works on the world map.
    previewLocked.mockReturnValue(null);
    const chosen = tabMenuItems({ panel: panel("worldmap") }, run);
    expect(item(chosen, "Als Vorschaubild").disabled).toBe(false);
    item(chosen, "Als Vorschaubild").action!();
    expect(setPreview).toHaveBeenLastCalledWith("worldmap");
  });

  it("offers_no_preview_on_the_other_view_tabs_or_without_the_action", () => {
    const run = { ...actions(), setPreview: vi.fn(), previewLocked: () => null };
    for (const id of ["matrix", "textView", "pokemonView", "itemView"]) {
      expect(labels(tabMenuItems({ panel: panel(id) }, run))).toEqual(["Maximieren"]);
    }
    expect(labels(tabMenuItems({ panel: panel("inspector") }, run))).not.toContain("Als Vorschaubild");
    // A deck that does not set the preview (the texts', the data areas') has none on its view either.
    expect(labels(tabMenuItems({ panel: panel("world") }, actions()))).toEqual(["Maximieren"]);
  });
});
