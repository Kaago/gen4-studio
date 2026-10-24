// What the frame remembers between runs (localStorage, key `g4.shell.v1`): the active area, which lists are open, the height
// of each area's hand, the depth, the recently opened projects, the part the Pokémon area shows and the selection of each data
// part. Plain module (no React); `ShellContext` holds the live copy. A state written before a field existed reads it as its default.

export type AreaId = "start" | "welt" | "geschichten" | "pokemon" | "gegenstaende" | "texte" | "technik" | "einstellungen";
export type Depth = "bauen" | "tuefteln";
export type Tool = "select" | "paint" | "figure" | "door" | "trigger" | "sign" | "building";

export const AREAS: readonly AreaId[] = ["start", "welt", "geschichten", "pokemon", "gegenstaende", "texte", "technik", "einstellungen"];

/** The parts of the Pokémon area, in the order of their keys ⌘1 to ⌘4. */
export type PokemonSub = "pokedex" | "trainer" | "encounters" | "moves";
export const POKEMON_SUBS: readonly PokemonSub[] = ["pokedex", "trainer", "encounters", "moves"];

/** The selected member of each data part that has a list to choose from (a species, a move, an item). */
export interface DataSelection {
  pokedex?: number;
  moves?: number;
  items?: number;
}
export type DataSelectKey = keyof DataSelection;
const SELECT_KEYS: readonly DataSelectKey[] = ["pokedex", "moves", "items"];

/** The hand (the lower dock) is never lower or higher than this, whatever the stored value says. */
export const HAND_MIN = 200;
export const HAND_MAX = 560;
export const HAND_DEFAULT = 286;

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
}

/** A hand height inside 200..560; anything that is not a number gets the default. */
export function clampHand(height: unknown): number {
  if (typeof height !== "number" || !Number.isFinite(height)) return HAND_DEFAULT;
  return Math.min(HAND_MAX, Math.max(HAND_MIN, Math.round(height)));
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
