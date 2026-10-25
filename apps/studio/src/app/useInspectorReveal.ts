import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import type { DockviewApi } from "dockview-react";
import type { InspectorModel } from "../workbench/Inspector";
import { useStableCallback } from "./useStable";

/** The pins that show an editor in the inspector (a hover, a tile or no pin does not). */
const EDITED: ReadonlySet<InspectorModel["kind"]> = new Set(["header", "event", "building", "area", "matrixCell"]);

/** Whether a text field has the keyboard: bringing a tab forward must not take it away. */
function typingInField(): boolean {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement || (el instanceof HTMLElement && el.isContentEditable);
}

/**
 * `setPinned` that also brings the Inspector of the world's dock to the front when the new pin is a header, event, building, area or matrix
 * cell: the inspector may share its tab group with Script and Probleme (Doppeldeck) or float, so a selection would otherwise change something
 * that is hidden. A text field that has the focus keeps it (the tab stays as it is then). A maximized group stays maximized: selecting in a
 * maximized view is what it is maximized for, and focusing the inspector behind it would restore it. An update function (history reloads
 * settling the pin again) and the pins that show nothing editable leave the tabs alone, and so does any pin while `paused` says a wizard works
 * (its step under the dock keeps the keyboard).
 */
export function useInspectorReveal(setPinned: Dispatch<SetStateAction<InspectorModel>>, dock: MutableRefObject<DockviewApi | null>, paused: () => boolean = () => false) {
  return useStableCallback((next: SetStateAction<InspectorModel>) => {
    setPinned(next);
    if (typeof next === "function" || !EDITED.has(next.kind) || typingInField() || paused()) return;
    try {
      const api = dock.current;
      const panel = api?.getPanel("inspector");
      if (!api || !panel) return;
      if (api.hasMaximizedGroup() && !panel.api.isMaximized()) return;
      panel.focus();
    } catch {
      /* the deck is going away */
    }
  });
}
