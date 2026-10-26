import type { IDockviewPanel } from "dockview-react";
import { isSchauPanel } from "../../workbench/layout";

/** An item of the tab menu: a line with a label (dockview builds it), or a separator. */
export type TabMenuItem = "separator" | { label: string; action: () => void; disabled?: boolean };

/** What the tab menu asks the deck to do: float a panel, dock it back, build the arrangement again. */
export interface TabMenuActions {
  float: (id: string) => void;
  dock: (id: string) => void;
  reset: () => void;
}

/**
 * The context menu of a dock tab (right click, Shift+F10): „Maximieren“ or „Wiederherstellen“ for its group, „Schweben lassen“ (locked
 * while it floats), „Zurück ins Dock“ (only while it floats) and „Anordnung zurücksetzen“. A view panel stays in the view group: its tab
 * only maximizes. Only a group in the grid can be maximized.
 */
export function tabMenuItems({ panel }: { panel: IDockviewPanel }, actions: TabMenuActions): TabMenuItem[] {
  const maximized = panel.api.isMaximized();
  const floats = panel.api.location.type === "floating";
  const maximize: TabMenuItem = maximized
    ? { label: "Wiederherstellen", action: () => panel.api.exitMaximized() }
    : { label: "Maximieren", action: () => panel.api.maximize(), disabled: panel.api.location.type !== "grid" };
  if (isSchauPanel(panel.id)) return [maximize];
  return [
    maximize,
    { label: "Schweben lassen", action: () => actions.float(panel.id), disabled: floats },
    ...(floats ? [{ label: "Zurück ins Dock", action: () => actions.dock(panel.id) }] : []),
    "separator",
    { label: "Anordnung zurücksetzen", action: () => actions.reset() },
  ];
}
