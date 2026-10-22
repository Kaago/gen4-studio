import type { InspectorModel } from "./Inspector";

/**
 * What the inspector pins after the history moved (undo, redo) or a map or event file was
 * reloaded behind its back. A building is bound by its index in the map and an event by its index
 * in its class, and the history can shift indexes (undoing a delete re-inserts one in front), so
 * a pinned building or event is let go of: the selected header (or nothing) is shown instead.
 * Every other pin stays.
 */
export function pinAfterHistory(pinned: InspectorModel, selectedHeader: number | null): InspectorModel {
  if (pinned.kind !== "building" && pinned.kind !== "event") return pinned;
  return selectedHeader === null ? { kind: "none" } : { kind: "header", id: selectedHeader };
}

/**
 * What the inspector shows, given the pin and what the pointer is over in the scene. The scene's
 * hover wins over the pin, except while a building or event is pinned (it is edited and dragged
 * there): hovering tiles keeps it, and so does hovering the pinned event's own marker. Other
 * markers still take over.
 */
export function shownInspector(pinned: InspectorModel, hover: InspectorModel | null): InspectorModel {
  if (hover === null) return pinned;
  if (pinned.kind === "building" && hover.kind === "tile") return pinned;
  if (pinned.kind === "event") {
    if (hover.kind === "tile") return pinned;
    if (hover.kind === "eventHover" && hover.eventFile === pinned.eventFile && hover.eventKind === pinned.eventKind && hover.index === pinned.index) return pinned;
  }
  return hover;
}
