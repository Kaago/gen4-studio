import type { AreaId } from "../ui/frame/shellState";

/** The panels of the docks. `headers` (the explorer) is not among them: it is the shell's list column and only a navigation target. */
export type PanelId = "headers" | "world" | "matrix" | "worldmap" | "tools" | "inspector" | "script" | "problems" | "context" | "textView" | "textEdit";

/** The areas that have a deck (a list and two docks); the others show a pane. */
export type DeckArea = "welt" | "texte";
export const DECK_AREAS: readonly DeckArea[] = ["welt", "texte"];
export const isDeckArea = (area: AreaId): area is DeckArea => (DECK_AREAS as readonly AreaId[]).includes(area);

/** The two docks of a deck: the view (Schau, above the hinge) and the hand (below it). */
export type DeckId = "schau" | "hand";

export interface PanelSpec {
  id: PanelId;
  title: string;
  position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string };
  /** The width a panel that gets a group of its own starts with (px). */
  width?: number;
}

/**
 * The panels of each dock of each area with a deck. Order matters: the first panel is the anchor, the others are tabs of its group (`within`) or sit
 * beside it. The explorer and the text archives are no dock panels: they are the areas' lists. The layout key is versioned (`v8`): a layout stored by
 * an older panel set is not read, not migrated. The problems are a panel of both hands; each area's dock is its own dockview, so the ids do not clash.
 */
export const DECK_PANELS: Record<DeckArea, Record<DeckId, PanelSpec[]>> = {
  welt: {
    schau: [
      { id: "world", title: "3D", position: { direction: "within" } },
      { id: "matrix", title: "Raster", position: { direction: "within", referencePanel: "world" } },
      { id: "worldmap", title: "Weltkarte", position: { direction: "within", referencePanel: "world" } },
    ],
    hand: [
      { id: "inspector", title: "Inspector", position: { direction: "within" } },
      { id: "script", title: "Script", position: { direction: "within", referencePanel: "inspector" } },
      { id: "problems", title: "Probleme", position: { direction: "within", referencePanel: "inspector" } },
      // The tools to the left of the inspector, "An diesem Ort" to its right: groups of one panel, so without a tab bar.
      { id: "tools", title: "Werkzeuge", position: { direction: "left", referencePanel: "inspector" }, width: 180 },
      { id: "context", title: "An diesem Ort", position: { direction: "right", referencePanel: "inspector" }, width: 260 },
    ],
  },
  texte: {
    schau: [{ id: "textView", title: "Vorschau", position: { direction: "within" } }],
    hand: [
      { id: "textEdit", title: "Nachrichten", position: { direction: "within" } },
      { id: "problems", title: "Probleme", position: { direction: "within", referencePanel: "textEdit" } },
    ],
  },
};

/** The panels of a dock of `area`; an area without a deck has none. */
export function deckPanels(area: AreaId, deck: DeckId): PanelSpec[] {
  return isDeckArea(area) ? DECK_PANELS[area][deck] : [];
}

/** The localStorage key of one dock's layout: per area and per deck, so one broken layout resets only its own dock. */
export const layoutKey = (area: AreaId, deck: DeckId) => `g4.layout.v8.${area}.${deck}`;

/** The prefix of the layouts of the version before (the world's hand still had the text panel). */
const OLD_PREFIX = "g4.layout.v7.";

/** Removes the layouts stored under `v7` (called once at the start); other keys stay. A storage that cannot be read is left alone. */
export function dropOldLayouts(storage: Storage): void {
  try {
    const old: string[] = [];
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      if (key !== null && key.startsWith(OLD_PREFIX)) old.push(key);
    }
    for (const key of old) storage.removeItem(key);
  } catch {
    /* nothing is dropped */
  }
}

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
