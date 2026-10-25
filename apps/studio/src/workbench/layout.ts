import type { AreaId } from "../ui/frame/shellState";

/** The panels of the docks. `headers` (the explorer) is not among them: it is the shell's list column and only a navigation target. */
export type PanelId =
  | "headers"
  | "world"
  | "matrix"
  | "worldmap"
  | "inspector"
  | "script"
  | "problems"
  | "textView"
  | "textEdit"
  | "pokemonView"
  | "pokemonEdit"
  | "itemView"
  | "itemEdit";

/** The areas that have a deck (a list and two docks); the others show a pane. */
export type DeckArea = "welt" | "texte" | "pokemon" | "gegenstaende";
export const DECK_AREAS: readonly DeckArea[] = ["welt", "texte", "pokemon", "gegenstaende"];
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
 * beside it. The explorer and the text archives are no dock panels: they are the areas' lists. The layout key is versioned (`v10`): a layout stored by
 * an older panel set is not read, not migrated. The problems are a panel of the hands of Welt and Texte (the data areas have nothing to compile);
 * each area's dock is its own dockview, so the ids do not clash. The data areas have one panel per dock: the view of the selected member and
 * its editor, whatever part of the area shows.
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
    ],
  },
  texte: {
    schau: [{ id: "textView", title: "Vorschau", position: { direction: "within" } }],
    hand: [
      { id: "textEdit", title: "Nachrichten", position: { direction: "within" } },
      { id: "problems", title: "Probleme", position: { direction: "within", referencePanel: "textEdit" } },
    ],
  },
  pokemon: {
    schau: [{ id: "pokemonView", title: "Übersicht", position: { direction: "within" } }],
    hand: [{ id: "pokemonEdit", title: "Bearbeiten", position: { direction: "within" } }],
  },
  gegenstaende: {
    schau: [{ id: "itemView", title: "Übersicht", position: { direction: "within" } }],
    hand: [{ id: "itemEdit", title: "Bearbeiten", position: { direction: "within" } }],
  },
};

/** The panels of a dock of `area`; an area without a deck has none. */
export function deckPanels(area: AreaId, deck: DeckId): PanelSpec[] {
  return isDeckArea(area) ? DECK_PANELS[area][deck] : [];
}

/** The version of the stored layouts; v10: the world's hand lost „An diesem Ort“ (the list „Welt“ shows the scene). */
export const LAYOUT_VERSION = 10;

/** The localStorage key of one dock's layout: per area and per deck, so one broken layout resets only its own dock. */
export const layoutKey = (area: AreaId, deck: DeckId) => `g4.layout.v${LAYOUT_VERSION}.${area}.${deck}`;

/** A stored layout key of any version, capturing the version. */
const LAYOUT_KEY = /^g4\.layout\.v(\d+)\./;

/** Removes the layouts stored under a version before `LAYOUT_VERSION` (called once at the start); other keys stay. A storage that cannot be read is left alone. */
export function dropOldLayouts(storage: Storage): void {
  try {
    const old: string[] = [];
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      const version = key === null ? null : LAYOUT_KEY.exec(key);
      if (key !== null && version !== null && Number(version[1]) < LAYOUT_VERSION) old.push(key);
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

/** Forgets a stored layout: the dock builds its default the next time it is built. */
export function dropLayout(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    /* the old layout stays */
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
