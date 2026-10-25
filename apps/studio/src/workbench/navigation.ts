import type { EventKind, HeaderSummary, SpeciesTab } from "../api";
import type { InspectorModel } from "./Inspector";
import type { DeckArea, PanelId } from "./layout";
import type { SceneGroup } from "./sceneModel";

/**
 * Where a link leads. `header` on a matrix cell, building or event is a header known to show it
 * (the cell's own header, say): navigation selects it first when the target is not shown yet.
 */
export type NavTarget =
  | { kind: "header"; id: number }
  | { kind: "matrixCell"; matrix: number; x: number; y: number; header?: number | null }
  | { kind: "area"; id: number }
  | { kind: "building"; map: number; index: number; header?: number | null }
  | { kind: "event"; eventFile: number; eventKind: EventKind; index: number; header?: number | null }
  /** A text archive, with one of its messages (omitted: the first). */
  | { kind: "textArchive"; index: number; message?: number }
  /** A group of „In dieser Szene“ (the doors, say): the list „Welt“ opens and focuses it; with `header`, that header's scene shows first. */
  | { kind: "sceneGroup"; group: SceneGroup; header?: number }
  /** A script file (or level script), at a 1-based line (omitted: where the cursor was). */
  | { kind: "script"; file: number; line?: number }
  /** A species (personal member) in the Pokédex, with a tab of its editor (omitted: the view). */
  | { kind: "species"; index: number; tab?: SpeciesTab }
  /** A move in the moves of the Pokémon area. */
  | { kind: "move"; index: number }
  /** An item by item id (a data member without the item table). */
  | { kind: "item"; id: number }
  /** A trainer in the trainers of the Pokémon area. */
  | { kind: "trainer"; index: number }
  /** An encounter file in the Begegnungen of the Pokémon area. */
  | { kind: "encounter"; index: number };

/**
 * An event target that names no header (a finding of the checks, whose subject is the event file) with the first header that reads its
 * file, so a link to it shows that place first; any other target, one with a header and one no header reads, as it is.
 */
export function withHeaderHint(target: NavTarget, headers: readonly Pick<HeaderSummary, "id" | "event_file_id">[]): NavTarget {
  if (target.kind !== "event" || typeof target.header === "number") return target;
  const header = headers.find((h) => h.event_file_id === target.eventFile);
  return header ? { ...target, header: header.id } : target;
}

/** The part of a data area a link opens, and the member it selects there. */
export interface DataNav {
  sub: "pokedex" | "trainer" | "encounters" | "moves" | "items";
  index: number;
  /** The tab of the Pokédex editor to show. */
  tab?: SpeciesTab;
}

/** What `nextSelection` needs to know about the current selection and the scene. */
export interface NavState {
  selectedHeader: number | null;
  /** The matrix the scene shows (null: none). */
  matrix: number | null;
  /** The maps the shown matrix places; omitted means none. */
  maps?: readonly number[];
  /** The event file the scene shows (the selected header's); omitted or null means none. */
  eventFile?: number | null;
  /** Every known header. When omitted, a header id is taken to exist. */
  headers?: readonly HeaderSummary[];
}

export interface NavResult {
  /** The area that shows the target: a text archive is in Texte, a species or move in Pokémon, an item in Gegenstände, everything else in Welt. */
  area: DeckArea;
  selectedHeader: number | null;
  /** What to pin in the inspector; null when the target does not exist or leaves the pin alone (a text archive). */
  pinned: InspectorModel | null;
  /** The panel that shows the target, in a dock of `area`; null when nothing changes. */
  panel: PanelId | null;
  /** Set for a text archive: what the text area opens (`message` null: the first one). */
  text?: { index: number; message: number | null };
  /** Set for a script: what the script panel opens (`line` null: keep the cursor). */
  script?: { file: number; line: number | null };
  /** Set for a species, move or item: what its data area selects. */
  data?: DataNav;
  /** Set for a group of the scene: what the list „Welt“ opens and focuses. */
  sceneGroup?: SceneGroup;
}

/**
 * The selection a link leads to, as a pure function (App applies it: selecting a header loads its
 * matrix, then the inspector is pinned and the panel focused). A header is only selected when the
 * target is a header, a matrix cell with a header, or a building or event that the shown scene
 * does not hold and a known header shows. A building or event the scene holds is just pinned.
 * Without a known header it is pinned alone: the editors read the backend, not the scene.
 * A text archive leads to the area Texte, a species or a move to Pokémon, an item to Gegenstände (none of them touches the selected
 * header or the pin), every other target to Welt.
 */
export function nextSelection(state: NavState, target: NavTarget): NavResult {
  // Outside the world neither the selection nor the pin changes: the texts and the data are not about a header.
  const keep = { selectedHeader: state.selectedHeader, pinned: null };
  switch (target.kind) {
    case "textArchive":
      return { area: "texte", ...keep, panel: "textEdit", text: { index: target.index, message: target.message ?? null } };
    case "species":
      // A tab is one of the editor's: the hand shows it; otherwise the view shows the species.
      return target.tab === undefined
        ? { area: "pokemon", ...keep, panel: "pokemonView", data: { sub: "pokedex", index: target.index } }
        : { area: "pokemon", ...keep, panel: "pokemonEdit", data: { sub: "pokedex", index: target.index, tab: target.tab } };
    case "trainer":
      return { area: "pokemon", ...keep, panel: "pokemonView", data: { sub: "trainer", index: target.index } };
    case "encounter":
      return { area: "pokemon", ...keep, panel: "pokemonView", data: { sub: "encounters", index: target.index } };
    case "move":
      return { area: "pokemon", ...keep, panel: "pokemonView", data: { sub: "moves", index: target.index } };
    case "item":
      return { area: "gegenstaende", ...keep, panel: "itemView", data: { sub: "items", index: target.id } };
    default:
      return { area: "welt", ...inWorld(state, target) };
  }
}

/** Where a target of the world leads (see `nextSelection`). */
function inWorld(state: NavState, target: Exclude<NavTarget, { kind: "textArchive" | "species" | "trainer" | "encounter" | "move" | "item" }>): Omit<NavResult, "area"> {
  const known = (id: number | null | undefined): id is number => typeof id === "number" && (state.headers === undefined || state.headers.some((h) => h.id === id));
  const toHeader = (id: number): Omit<NavResult, "area"> => ({ selectedHeader: id, pinned: { kind: "header", id }, panel: "headers" });
  // A header that the target needs selected first, unless it already is (null: keep the selection).
  const switchTo = (id: number | null | undefined): number | null => (known(id) && id !== state.selectedHeader ? id : null);

  switch (target.kind) {
    case "script":
      return { selectedHeader: state.selectedHeader, pinned: null, panel: "script", script: { file: target.file, line: target.line ?? null } };
    case "sceneGroup": {
      // The list „Welt“ holds the scene: another header's scene shows first (selected and pinned as a link to it does).
      const header = switchTo(target.header);
      return header === null
        ? { selectedHeader: state.selectedHeader, pinned: null, panel: "headers", sceneGroup: target.group }
        : { ...toHeader(header), sceneGroup: target.group };
    }
    case "header":
      return known(target.id) ? toHeader(target.id) : { selectedHeader: state.selectedHeader, pinned: null, panel: null };
    case "matrixCell":
      if (known(target.header)) return toHeader(target.header);
      return { selectedHeader: state.selectedHeader, pinned: { kind: "matrixCell", matrix: target.matrix, x: target.x, y: target.y }, panel: "matrix" };
    case "area":
      return { selectedHeader: state.selectedHeader, pinned: { kind: "area", id: target.id }, panel: "inspector" };
    case "building": {
      const shown = (state.maps ?? []).includes(target.map);
      const header = shown ? null : switchTo(target.header);
      return {
        selectedHeader: header ?? state.selectedHeader,
        pinned: { kind: "building", map: target.map, index: target.index },
        panel: "world",
      };
    }
    case "event": {
      const shown = (state.eventFile ?? null) === target.eventFile;
      // The hint only helps when its header really reads that event file.
      const shows = (id: number) => state.headers === undefined || state.headers.find((h) => h.id === id)?.event_file_id === target.eventFile;
      const hint = target.header;
      const header = !shown && typeof hint === "number" && shows(hint) ? switchTo(hint) : null;
      return {
        selectedHeader: header ?? state.selectedHeader,
        pinned: { kind: "event", eventFile: target.eventFile, eventKind: target.eventKind, index: target.index },
        panel: "world",
      };
    }
  }
}
