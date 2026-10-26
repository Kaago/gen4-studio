import type { DockviewMessages } from "dockview-react";

/**
 * What the dock says to a screen reader (its live region) and the names of its close buttons, in German. The texts of the keyboard docking
 * (Ctrl+M, not switched on) keep dockview's defaults.
 */
export const messages: Partial<DockviewMessages> = {
  panelOpened: (title) => `${title} geöffnet`,
  panelClosed: (title) => `${title} geschlossen`,
  groupMaximized: (title) => `${title} maximiert`,
  groupRestored: (title) => `${title} wiederhergestellt`,
  groupFloated: (title) => `${title} schwebt`,
  groupDocked: (title) => `${title} angedockt`,
  groupPoppedOut: (title) => `${title} in eigenem Fenster`,
  closeTab: (title) => `${title} schließen`,
  closeTabPlain: () => "Schließen",
};
