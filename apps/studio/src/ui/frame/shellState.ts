// What the frame remembers between runs (localStorage, key `g4.shell.v1`): the active area, which lists are open, the height
// of each area's hand, the depth, the recently opened projects, the part the Pokémon area shows, the selection of each data
// part, the anchor of the toolbar in the 3D view, whether the 3D view looks straight down, how the world tree and the header list are sorted, and how the list „Welt“ splits
// between the places and the scene. Plain module (no React); `ShellContext` holds the live copy. A state written
// before a field existed reads it as its default.

export type AreaId = "start" | "welt" | "geschichten" | "pokemon" | "gegenstaende" | "texte" | "technik" | "einstellungen";
export type Depth = "bauen" | "tuefteln";
export type Tool = "select" | "paint" | "figure" | "door" | "trigger" | "sign" | "building";

export const AREAS: readonly AreaId[] = ["start", "welt", "geschichten", "pokemon", "gegenstaende", "texte", "technik", "einstellungen"];

/** Where the toolbar floats in the 3D view: upright at the left edge, flat at the top (beside the layer bar), or flat at the bottom middle. */
export type ToolbarAnchor = "left" | "top" | "bottom";
const TOOLBAR_ANCHORS: readonly ToolbarAnchor[] = ["left", "top", "bottom"];

/** How the world tree orders its areas (and, by name, its rooms): game order (first header id), by name, or the areas chosen last first. */
export type TreeSort = "game" | "name" | "recent";
const TREE_SORTS: readonly TreeSort[] = ["game", "name", "recent"];
/** How the header list orders its rows: by number, by the name it shows, or by code. */
export type ListSort = "id" | "name" | "code";
const LIST_SORTS: readonly ListSort[] = ["id", "name", "code"];
export interface WorldSort {
  tree: TreeSort;
  list: ListSort;
}

/** The parts of the Pokémon area, in the order of their keys ⌘1 to ⌘4. */
export type PokemonSub = "pokedex" | "trainer" | "encounters" | "moves";
export const POKEMON_SUBS: readonly PokemonSub[] = ["pokedex", "trainer", "encounters", "moves"];

/** The selected member of each data part that has a list to choose from (a species, a trainer, an encounter file, a move, an item). */
export interface DataSelection {
  pokedex?: number;
  trainer?: number;
  encounters?: number;
  moves?: number;
  items?: number;
}
export type DataSelectKey = keyof DataSelection;
const SELECT_KEYS: readonly DataSelectKey[] = ["pokedex", "trainer", "encounters", "moves", "items"];

/** The hand (the lower dock) is never lower or higher than this, whatever the stored value says. */
export const HAND_MIN = 200;
export const HAND_MAX = 560;
export const HAND_DEFAULT = 286;

/** The share of the list „Welt“ the places take above the scene: never less than a fifth or more than four fifths. */
export const WORLD_SPLIT_MIN = 0.2;
export const WORLD_SPLIT_MAX = 0.8;
export const WORLD_SPLIT_DEFAULT = 0.5;

const MAX_RECENT = 5;
const KEY = "g4.shell.v1";

export interface RecentProject {
  path: string;
  name: string;
  /** The game as the title bar writes it ("Platin, DE"). */
  game: string;
  openedAt: number;
}

export interface ShellState {
  area: AreaId;
  listOpen: Record<AreaId, boolean>;
  handHeight: Record<AreaId, number>;
  handCollapsed: Record<AreaId, boolean>;
  depth: Depth;
  recent: RecentProject[];
  /** The part each area with parts shows. */
  sub: { pokemon: PokemonSub };
  /** The selection of each data part; a part without one has none. */
  selected: DataSelection;
  /** The anchor the toolbar was put at; a low view may show it elsewhere (`shownAnchor`), this stays. */
  toolbar: ToolbarAnchor;
  /** The 3D view looks straight down (orthographic) instead of slanted. */
  topDown: boolean;
  /** The sort of the world tree and of the header list. */
  worldSort: WorldSort;
  /** The share of the list „Welt“ the places take (0.2 to 0.8); the scene has the rest. */
  worldListSplit: number;
  /** The places are folded to one row (the scene then has the whole list). */
  placesFolded: boolean;
}

/** A hand height inside 200..560; anything that is not a number gets the default. */
export function clampHand(height: unknown): number {
  if (typeof height !== "number" || !Number.isFinite(height)) return HAND_DEFAULT;
  return Math.min(HAND_MAX, Math.max(HAND_MIN, Math.round(height)));
}

/** A share of the list „Welt“ inside 0.2..0.8; anything that is not a number gets the default. */
export function clampWorldSplit(share: unknown): number {
  if (typeof share !== "number" || !Number.isFinite(share)) return WORLD_SPLIT_DEFAULT;
  return Math.min(WORLD_SPLIT_MAX, Math.max(WORLD_SPLIT_MIN, share));
}

function perArea<T>(stored: unknown, fallback: T, valid: (v: unknown) => v is T, fix: (v: T) => T = (v) => v): Record<AreaId, T> {
  const record = typeof stored === "object" && stored !== null ? (stored as Record<string, unknown>) : {};
  return Object.fromEntries(AREAS.map((a) => [a, valid(record[a]) ? fix(record[a] as T) : fallback])) as Record<AreaId, T>;
}

const isBoolean = (v: unknown): v is boolean => typeof v === "boolean";
const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

function isRecent(v: unknown): v is RecentProject {
  if (typeof v !== "object" || v === null) return false;
  const r = v as Record<string, unknown>;
  return typeof r.path === "string" && typeof r.name === "string" && typeof r.game === "string" && isNumber(r.openedAt);
}

const isIndex = (v: unknown): v is number => typeof v === "number" && Number.isInteger(v) && v >= 0;

/** The selections that are member indices; anything else is dropped. */
function readSelection(stored: unknown): DataSelection {
  const record = typeof stored === "object" && stored !== null ? (stored as Record<string, unknown>) : {};
  const selection: DataSelection = {};
  for (const key of SELECT_KEYS) {
    const value = record[key];
    if (isIndex(value)) selection[key] = value;
  }
  return selection;
}

function readWorldSort(stored: unknown): WorldSort {
  const record = typeof stored === "object" && stored !== null ? (stored as Record<string, unknown>) : {};
  return { tree: TREE_SORTS.find((t) => t === record.tree) ?? "game", list: LIST_SORTS.find((l) => l === record.list) ?? "id" };
}

function stored(): unknown {
  try {
    const raw = localStorage.getItem(KEY);
    return raw === null ? null : JSON.parse(raw);
  } catch {
    return null;
  }
}

/** The stored state; every missing or invalid field has its default, a broken value as a whole gives the defaults. */
export function readShellState(): ShellState {
  const raw = stored();
  const s = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>) : {};
  return {
    area: AREAS.find((a) => a === s.area) ?? "start",
    listOpen: perArea(s.listOpen, true, isBoolean),
    handHeight: perArea(s.handHeight, HAND_DEFAULT, isNumber, clampHand),
    handCollapsed: perArea(s.handCollapsed, false, isBoolean),
    depth: s.depth === "tuefteln" ? "tuefteln" : "bauen",
    recent: Array.isArray(s.recent) ? s.recent.filter(isRecent).slice(0, MAX_RECENT) : [],
    sub: { pokemon: POKEMON_SUBS.find((p) => p === (s.sub as { pokemon?: unknown } | undefined)?.pokemon) ?? "pokedex" },
    selected: readSelection(s.selected),
    toolbar: TOOLBAR_ANCHORS.find((a) => a === s.toolbar) ?? "left",
    topDown: s.topDown === true,
    worldSort: readWorldSort(s.worldSort),
    worldListSplit: clampWorldSplit(s.worldListSplit),
    placesFolded: s.placesFolded === true,
  };
}

export function writeShellState(state: ShellState): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {
    /* the state is simply not remembered */
  }
}

/** The list with `project` first, an earlier entry of the same path removed, at most five kept. */
export function rememberRecent(list: readonly RecentProject[], project: RecentProject): RecentProject[] {
  return [project, ...list.filter((p) => p.path !== project.path)].slice(0, MAX_RECENT);
}
