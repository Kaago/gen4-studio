import type { DockviewMessages } from "dockview-react";
import { t } from "../../i18n";

/**
 * What the dock says to a screen reader (its live region) and the names of its close buttons, in the language that is shown. Each text is
 * read from the `ui` catalog when dockview asks for it, not when this module loads. The texts of the keyboard docking (Ctrl+M, not
 * switched on) keep dockview's defaults.
 */
export const messages: Partial<DockviewMessages> = {
  panelOpened: (title) => t("ui:dock.panelOpened", { title }),
  panelClosed: (title) => t("ui:dock.panelClosed", { title }),
  groupMaximized: (title) => t("ui:dock.groupMaximized", { title }),
  groupRestored: (title) => t("ui:dock.groupRestored", { title }),
  groupFloated: (title) => t("ui:dock.groupFloated", { title }),
  groupDocked: (title) => t("ui:dock.groupDocked", { title }),
  groupPoppedOut: (title) => t("ui:dock.groupPoppedOut", { title }),
  closeTab: (title) => t("ui:dock.closeTab", { title }),
  closeTabPlain: () => t("ui:dock.closeTabPlain"),
};
