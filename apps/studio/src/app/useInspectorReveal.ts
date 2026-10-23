import type { Dispatch, SetStateAction } from "react";
import type { InspectorModel } from "../workbench/Inspector";
import type { DockApis } from "./useSceneSync";
import { useStableCallback } from "./useStable";

/** The pins that show an editor in the inspector (a hover, a tile or no pin does not). */
const EDITED: ReadonlySet<InspectorModel["kind"]> = new Set(["header", "event", "building", "area", "matrixCell"]);

/** Whether a text field has the keyboard: bringing a tab forward must not take it away. */
function typingInField(): boolean {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement || (el instanceof HTMLElement && el.isContentEditable);
}

/**
 * `setPinned` that also brings the hand's Inspector tab to the front when the new pin is a header, event, building, area or matrix cell:
 * the inspector shares its tab group with Text, Script and Probleme, so a selection would otherwise change something that is hidden.
 * A collapsed hand opens first; a text field that has the focus keeps it (the tab stays as it is then). An update function (history
 * reloads settling the pin again) and the pins that show nothing editable leave the tabs alone.
 */
export function useInspectorReveal(setPinned: Dispatch<SetStateAction<InspectorModel>>, dockApi: DockApis, revealHand: () => void) {
  return useStableCallback((next: SetStateAction<InspectorModel>) => {
    setPinned(next);
    if (typeof next === "function" || !EDITED.has(next.kind) || typingInField()) return;
    try {
      const panel = dockApi.hand.current?.getPanel("inspector");
      if (!panel) return;
      revealHand();
      panel.focus();
    } catch {
      /* the deck is going away */
    }
  });
}
