import type { InspectorModel } from "./Inspector";

/**
 * What the inspector pins after the history moved (undo, redo) or a map was reloaded behind its
 * back. A building is bound by its index in the map, and the history can shift indexes (undoing a
 * delete re-inserts a building in front), so a pinned building is let go of: the selected header
 * (or nothing) is shown instead. Every other pin stays.
 */
export function pinAfterHistory(pinned: InspectorModel, selectedHeader: number | null): InspectorModel {
  if (pinned.kind !== "building") return pinned;
  return selectedHeader === null ? { kind: "none" } : { kind: "header", id: selectedHeader };
}
