import { HAND_DEFAULT, depthAtLeast } from "../ui/frame/shellState";
import type { AreaId, Depth } from "../ui/frame/shellState";

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

/** The areas that have a deck (a list and a dock); the others show a pane. */
export type DeckArea = "welt" | "texte" | "pokemon" | "gegenstaende";
export const DECK_AREAS: readonly DeckArea[] = ["welt", "texte", "pokemon", "gegenstaende"];
export const isDeckArea = (area: AreaId): area is DeckArea => (DECK_AREAS as readonly AreaId[]).includes(area);

/**
 * A panel of an area's dock; `schau` marks the view panels (3D, Raster, Weltkarte, the previews): they stay docked in the view group.
 * `tinkerOnly` marks a panel the dock has only in Tüfteln (the Raster): Bauen works without it.
 */
export interface PanelSpec {
  id: PanelId;
  title: string;
  schau?: true;
  tinkerOnly?: true;
}

/** The two arrangements of a dock, as building plans: „Werkbank“ (the inspector beside the view) and „Doppeldeck“ (the edit group below it). */
export type Arrangement = "werkbank" | "doppeldeck";

/**
 * The panels of the dock of each area with a deck, in the order the arrangements place them: the view panels first (`schau`). One dock per area
 * (3e.4); the problems are a panel of Welt and Texte (the data areas have nothing to compile); each area's dock is its own dockview, so
 * the ids do not clash.
 */
export const AREA_PANELS: Record<DeckArea, PanelSpec[]> = {
  welt: [
    { id: "world", title: "3D", schau: true },
    { id: "matrix", title: "Raster", schau: true, tinkerOnly: true },
    { id: "worldmap", title: "Weltkarte", schau: true },
    { id: "inspector", title: "Inspector" },
    { id: "script", title: "Script" },
    { id: "problems", title: "Probleme" },
  ],
  texte: [
    { id: "textView", title: "Vorschau", schau: true },
    { id: "textEdit", title: "Nachrichten" },
    { id: "problems", title: "Probleme" },
  ],
  pokemon: [
    { id: "pokemonView", title: "Übersicht", schau: true },
    { id: "pokemonEdit", title: "Bearbeiten" },
  ],
  gegenstaende: [
    { id: "itemView", title: "Übersicht", schau: true },
    { id: "itemEdit", title: "Bearbeiten" },
  ],
};

/** The panels of the dock of `area` in `depth`: all of them in Tüfteln, in Bauen and Erzählen those not only for Tüfteln (the world without the Raster). */
export function panelsFor(area: DeckArea, depth: Depth): PanelSpec[] {
  return depthAtLeast(depth, "tuefteln") ? AREA_PANELS[area] : AREA_PANELS[area].filter((p) => p.tinkerOnly !== true);
}

/** The arrangement a dock starts with: the world works at the 3D view with the inspector beside it, the others with the view above the editor. */
export const DEFAULT_ARRANGEMENT: Record<DeckArea, Arrangement> = {
  welt: "werkbank",
  texte: "doppeldeck",
  pokemon: "doppeldeck",
  gegenstaende: "doppeldeck",
};

/** One step of building an arrangement: add the panel `id` at `position` (the first step has none) with the group's size and limits (px). */
export interface ArrangementStep {
  id: PanelId;
  position?: { referencePanel: PanelId; direction: "within" | "right" | "below" | "left" | "above" };
  width?: number;
  height?: number;
  minimumWidth?: number;
  maximumWidth?: number;
}

/** The width of the inspector beside the view (px) and the limits the group keeps. */
const INSPECTOR_WIDTH = 340;
const INSPECTOR_MIN = 280;
const INSPECTOR_MAX = 480;
/** The height of the script group below the view in Werkbank (px). */
const SCRIPT_HEIGHT = 240;

/**
 * The steps that build `arrangement` of the dock of `area` in `depth`, in the order to apply them (each refers to a panel an earlier step
 * added). Only the world has both; the other areas have one plan, the view above the edit group (`HAND_DEFAULT` high), whatever is asked.
 * In Bauen the panels only for Tüfteln are left out (the Raster; no step refers to it).
 */
export function arrangementSteps(area: DeckArea, arrangement: Arrangement, depth: Depth): ArrangementStep[] {
  const panels = new Set<PanelId>(panelsFor(area, depth).map((p) => p.id));
  return allSteps(area, arrangement).filter((step) => panels.has(step.id));
}

/** The steps of `arrangement` with every panel of the area (Tüfteln). */
function allSteps(area: DeckArea, arrangement: Arrangement): ArrangementStep[] {
  switch (area) {
    case "welt": {
      const schau: ArrangementStep[] = [
        { id: "world" },
        { id: "matrix", position: { referencePanel: "world", direction: "within" } },
        { id: "worldmap", position: { referencePanel: "world", direction: "within" } },
      ];
      if (arrangement === "werkbank") {
        return [
          ...schau,
          { id: "inspector", position: { referencePanel: "world", direction: "right" }, width: INSPECTOR_WIDTH, minimumWidth: INSPECTOR_MIN, maximumWidth: INSPECTOR_MAX },
          { id: "script", position: { referencePanel: "world", direction: "below" }, height: SCRIPT_HEIGHT },
          { id: "problems", position: { referencePanel: "script", direction: "within" } },
        ];
      }
      return [
        ...schau,
        { id: "inspector", position: { referencePanel: "world", direction: "below" }, height: HAND_DEFAULT },
        { id: "script", position: { referencePanel: "inspector", direction: "within" } },
        { id: "problems", position: { referencePanel: "inspector", direction: "within" } },
      ];
    }
    case "texte":
      return [
        { id: "textView" },
        { id: "textEdit", position: { referencePanel: "textView", direction: "below" }, height: HAND_DEFAULT },
        { id: "problems", position: { referencePanel: "textEdit", direction: "within" } },
      ];
    case "pokemon":
      return [{ id: "pokemonView" }, { id: "pokemonEdit", position: { referencePanel: "pokemonView", direction: "below" }, height: HAND_DEFAULT }];
    case "gegenstaende":
      return [{ id: "itemView" }, { id: "itemEdit", position: { referencePanel: "itemView", direction: "below" }, height: HAND_DEFAULT }];
  }
}

/** True for a view panel (3D, Raster, Weltkarte, the previews): it stays in the view group and does not float. */
export const isSchauPanel = (id: string): boolean => DECK_AREAS.some((area) => AREA_PANELS[area].some((p) => p.id === id && p.schau === true));

/** The version of the stored layouts; v11: one dock per area (v10 had two, the view and the hand). */
export const LAYOUT_VERSION = 11;

/**
 * The localStorage key of an area's dock layout: one per area, so one broken layout resets only its own dock. An area with sub-areas
 * (Pokémon: Pokédex, Trainer, Begegnungen, Attacken) keeps one per sub-area (`sub`), under `.sub.<sub>`: that suffix matches neither an old
 * version nor the two-dock keys `dropOldLayouts` removes, nor `.arrangement`.
 */
export const layoutKey = (area: AreaId, sub?: string) => `g4.layout.v${LAYOUT_VERSION}.${area}${sub ? `.sub.${sub}` : ""}`;

/**
 * The localStorage key of the arrangement last built in an area's dock (what „Zurück ins Dock“ and „Anordnung zurücksetzen“ follow); per
 * sub-area like the layout.
 */
export const arrangementKey = (area: AreaId, sub?: string) => `${layoutKey(area, sub)}.arrangement`;

/** A stored layout key of any version, capturing the version. */
const LAYOUT_KEY = /^g4\.layout\.v(\d+)\./;
/** The keys an interim build of v11 stored for the two docks of a deck (`.schau`, `.hand`); one dock per area has no such key. */
const TWO_DOCK_KEY = /^g4\.layout\.v11\.[a-z]+\.(schau|hand)$/;

/**
 * Removes the layouts stored under a version before `LAYOUT_VERSION` and the two-dock keys of the interim v11 (called once at the start); other
 * keys stay. A storage that cannot be read is left alone.
 */
export function dropOldLayouts(storage: Storage): void {
  try {
    const old: string[] = [];
    for (let i = 0; i < storage.length; i++) {
      const key = storage.key(i);
      const version = key === null ? null : LAYOUT_KEY.exec(key);
      if (key !== null && version !== null && (Number(version[1]) < LAYOUT_VERSION || TWO_DOCK_KEY.test(key))) old.push(key);
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

/**
 * The arrangement last built in the dock of `area` (in its sub-area `sub`); the area's default when none is stored (or what is stored is no
 * arrangement).
 */
export function loadArrangement(area: DeckArea, sub?: string): Arrangement {
  try {
    const stored = localStorage.getItem(arrangementKey(area, sub));
    return stored === "werkbank" || stored === "doppeldeck" ? stored : DEFAULT_ARRANGEMENT[area];
  } catch {
    return DEFAULT_ARRANGEMENT[area];
  }
}

/** Remembers the arrangement built in the dock of `area` (in its sub-area `sub`). */
export function saveArrangement(area: DeckArea, arrangement: Arrangement, sub?: string): void {
  try {
    localStorage.setItem(arrangementKey(area, sub), arrangement);
  } catch {
    /* the default is built next time */
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

/** True when a view panel sits in a floating group of a serialized dockview layout (the view stays docked; a floating inspector is fine). */
function viewPanelFloats(layout: object): boolean {
  const floating = (layout as { floatingGroups?: unknown }).floatingGroups;
  if (!Array.isArray(floating)) return false;
  return floating.some((group: { data?: { views?: unknown } } | null) => {
    const views = group?.data?.views;
    return Array.isArray(views) && views.some((id) => typeof id === "string" && isSchauPanel(id));
  });
}

/**
 * True when a serialized dockview layout can be restored into a dock with these `panels`: every one of them exists and no other
 * panel does (a layout stored by another panel set would bring back a panel that has no component), and no view panel floats. A panel
 * only for Tüfteln may be missing or there (a layout stored in either depth): the dock adds or removes it after restoring
 * (`syncDepthPanels`).
 */
export function isSaneLayout(layout: object, panels: readonly { id: string; tinkerOnly?: true }[]): boolean {
  const stored = (layout as { panels?: unknown }).panels;
  if (typeof stored !== "object" || stored === null) return false;
  const known = new Set<string>(panels.map((p) => p.id));
  if (!panels.every((p) => p.tinkerOnly === true || Object.prototype.hasOwnProperty.call(stored, p.id))) return false;
  if (!Object.keys(stored).every((id) => known.has(id))) return false;
  return !viewPanelFloats(layout);
}
