import type { IDockviewPanel } from "dockview-react";
import { isSchauPanel } from "../../workbench/layout";

/**
 * An item of the tab menu: a line with a label (dockview builds it), or a separator. A locked line may say why (`hint`): the deck shows it
 * as the line's tooltip.
 */
export type TabMenuItem = "separator" | { label: string; action: () => void; disabled?: boolean; hint?: string };

/**
 * What the tab menu asks the deck to do: float a panel, dock it back, build the arrangement again; on the world's deck also set the
 * project's preview picture from a view (`setPreview`), locked while `previewLocked` gives a reason.
 */
export interface TabMenuActions {
  float: (id: string) => void;
  dock: (id: string) => void;
  reset: () => void;
  setPreview?: (panelId: string) => void;
  previewLocked?: (panelId: string) => string | null;
}

/** The views whose picture can become the preview (phase 4.0 §3.5): the 3D view and the world map (the chosen map's picture). */
const PREVIEW_VIEWS = new Set(["world", "worldmap"]);

/**
 * The context menu of a dock tab (right click, Shift+F10): „Maximieren“ or „Wiederherstellen“ for its group, „Schweben lassen“ (locked
 * while it floats), „Zurück ins Dock“ (only while it floats) and „Anordnung zurücksetzen“. A view panel stays in the view group: its tab
 * only maximizes, and on „3D“ and „Weltkarte“ offers „Als Vorschaubild“ below a separator (when the deck can set it). Only a group in the
 * grid can be maximized.
 */
export function tabMenuItems({ panel }: { panel: IDockviewPanel }, actions: TabMenuActions): TabMenuItem[] {
  const maximized = panel.api.isMaximized();
  const floats = panel.api.location.type === "floating";
  const maximize: TabMenuItem = maximized
    ? { label: "Wiederherstellen", action: () => panel.api.exitMaximized() }
    : { label: "Maximieren", action: () => panel.api.maximize(), disabled: panel.api.location.type !== "grid" };
  if (isSchauPanel(panel.id)) {
    const { setPreview, previewLocked } = actions;
    if (setPreview === undefined || !PREVIEW_VIEWS.has(panel.id)) return [maximize];
    const locked = previewLocked?.(panel.id) ?? null;
    return [maximize, "separator", { label: "Als Vorschaubild", action: () => setPreview(panel.id), disabled: locked !== null, ...(locked === null ? {} : { hint: locked }) }];
  }
  return [
    maximize,
    { label: "Schweben lassen", action: () => actions.float(panel.id), disabled: floats },
    ...(floats ? [{ label: "Zurück ins Dock", action: () => actions.dock(panel.id) }] : []),
    "separator",
    { label: "Anordnung zurücksetzen", action: () => actions.reset() },
  ];
}
