import type { AreaId } from "../ui/frame/shellState";

/** The panels of the docks. `headers` (the explorer) is not among them: it is the shell's list column and only a navigation target. */
export type PanelId = "headers" | "world" | "matrix" | "worldmap" | "inspector" | "text" | "script" | "problems";

/** The two docks of a deck: the view (Schau, above the hinge) and the hand (below it). */
export type DeckId = "schau" | "hand";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
}

/**
 * The panels of each dock. Order matters: the first panel is the anchor, the others are tabs of its group (`within`). The explorer is
 * no dock panel any more. The layout key is versioned (`v6`): a layout stored by an older panel set is not read, not migrated.
 */
export const DECK_PANELS: Record<"welt", Record<DeckId, PanelSpec[]>> = {
  welt: {
    schau: [
      { id: "world", title: "3D", position: { direction: "within" } },
      { id: "matrix", title: "Raster", position: { direction: "within", referencePanel: "world" } },
      { id: "worldmap", title: "Weltkarte", position: { direction: "within", referencePanel: "world" } },
    ],
    hand: [
      { id: "inspector", title: "Inspector", position: { direction: "within" } },
      { id: "text", title: "Text", position: { direction: "within", referencePanel: "inspector" } },
      { id: "script", title: "Script", position: { direction: "within", referencePanel: "inspector" } },
      { id: "problems", title: "Probleme", position: { direction: "within", referencePanel: "inspector" } },
    ],
  },
};

/** The panels of a dock of `area`; an area without a deck has none. */
export function deckPanels(area: AreaId, deck: DeckId): PanelSpec[] {
  return area === "welt" ? DECK_PANELS.welt[deck] : [];
}

/** The localStorage key of one dock's layout: per area and per deck, so one broken layout resets only its own dock. */
export const layoutKey = (area: AreaId, deck: DeckId) => `g4.layout.v6.${area}.${deck}`;

export function saveLayout(key: string, json: object): void {
  try {
    localStorage.setItem(key, JSON.stringify(json));
  } catch {
    /* the layout is simply not remembered */
  }
}

export function loadLayout(key: string): object | null {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * True when a serialized dockview layout can be restored into a dock with these `panels`: every one of them exists and no other
 * panel does (a layout stored by another panel set would bring back a panel that has no component).
 */
export function isSaneLayout(layout: object, panels: readonly PanelSpec[]): boolean {
  const stored = (layout as { panels?: unknown }).panels;
  if (typeof stored !== "object" || stored === null) return false;
  const known = new Set<string>(panels.map((p) => p.id));
  if (!panels.every((p) => Object.prototype.hasOwnProperty.call(stored, p.id))) return false;
  return Object.keys(stored).every((id) => known.has(id));
}
