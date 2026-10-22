import type { AssetId, HeaderSummary } from "./api";

/** What the shown scene is built from; `reloadPlan` decides against it what a restored asset touches. */
export interface SceneState {
  /** Index of the matrix the scene shows (null: none). */
  matrix: number | null;
  /** The map in every cell of that matrix (null: an empty cell). */
  maps: readonly (number | null)[];
  /** The area data the scene should show. */
  area: number;
  /** The event file of the selected header (null: none selected). */
  eventFile: number | null;
}

/** One thing the UI reloads after undo or redo restored an asset. */
export type ReloadAction =
  /** A pinned building or event is bound by index, which the history can shift. */
  | { kind: "unpin" }
  /** The building editor re-reads its map. */
  | { kind: "buildings" }
  /** The scene refetches this map: overlay colours and buildings of all its cells. */
  | { kind: "map"; map: number }
  /** The matrix panel and the cell editor re-read their matrix. */
  | { kind: "matrixPanel" }
  /** The scene's matrix is read again; a changed layout rebuilds the scene. */
  | { kind: "matrix" }
  /** The event editor re-reads its file. */
  | { kind: "eventsPanel" }
  /** The scene reads this event file again and rebuilds its markers. */
  | { kind: "events"; file: number }
  /** The area data editor re-reads its area data. */
  | { kind: "areaPanel" }
  /** The scene compares the tilesets of its area data and reloads its models if they changed. */
  | { kind: "models" }
  /** The header list and editor re-read the table; the scene reloads if the selected header's matrix, area data or event file changed. */
  | { kind: "headers" };

/**
 * What to reload after undo or redo restored `asset`, so that exactly what the asset affects is
 * refreshed and nothing else. `scene` is null while no scene is shown.
 */
export function reloadPlan(asset: AssetId, scene: SceneState | null): ReloadAction[] {
  switch (asset.kind) {
    case "Map": {
      const actions: ReloadAction[] = [{ kind: "unpin" }, { kind: "buildings" }];
      if (scene?.maps.includes(asset.index)) actions.push({ kind: "map", map: asset.index });
      return actions;
    }
    case "Matrix": {
      const actions: ReloadAction[] = [{ kind: "unpin" }, { kind: "matrixPanel" }];
      if (scene?.matrix === asset.index) actions.push({ kind: "matrix" });
      return actions;
    }
    case "Event": {
      const actions: ReloadAction[] = [{ kind: "unpin" }, { kind: "eventsPanel" }];
      if (scene?.eventFile === asset.index) actions.push({ kind: "events", file: asset.index });
      return actions;
    }
    case "AreaData": {
      const actions: ReloadAction[] = [{ kind: "areaPanel" }];
      if (scene?.area === asset.index) actions.push({ kind: "models" });
      return actions;
    }
    case "HeaderTable":
      return [{ kind: "unpin" }, { kind: "headers" }];
    default:
      return [];
  }
}

/** What a header's scene is rebuilt from: the matrix and, read when a matrix loads, the event file. The area data only textures the models, which reload on their own. */
export function sceneKey(h: HeaderSummary): string {
  return `${h.matrix_id}:${h.event_file_id}`;
}

/**
 * What the scene needs when the selected header is read again: a rebuild when the matrix or event
 * file differs from `shownKey` (the key of the scene's content; null when unknown), else at most
 * the area data, which only reloads the models.
 */
export function headerSync(shownKey: string | null, header: HeaderSummary): "scene" | "area" {
  return sceneKey(header) === shownKey ? "area" : "scene";
}

/**
 * The scene bookkeeping after a matrix fetch failed: the scene keeps showing what it showed, so the
 * key is forgotten (the next sync rebuilds instead of pairing the old maps with the new header's
 * area data) and the area is the one still shown.
 */
export function afterFailedSceneLoad(shownArea: number): { key: null; area: number } {
  return { key: null, area: shownArea };
}

/**
 * Whether the scene's models must reload: the tilesets of `area` (`key`) differ from the ones seen
 * before. Another area data is left to the scene load that comes with it; unknown tilesets (an
 * earlier read failed) reload to be safe.
 */
export function tilesetReload(known: { area: number; key: string } | null, area: number, key: string): boolean {
  return !known || (known.area === area && known.key !== key);
}

/**
 * Runs `fetch` and runs it again as long as `revision` (bumped by every undo and redo) moved while
 * it was in flight, so a fetch that raced with an undo never delivers the state from before it.
 */
export async function fetchCurrent<T>(fetch: () => Promise<T>, revision: () => number): Promise<T> {
  for (;;) {
    const before = revision();
    const value = await fetch();
    if (revision() === before) return value;
  }
}

/** A request for the scene to refetch one map (overlay and buildings); a new `token` makes a new request. */
export interface MapRefresh {
  map: number;
  token: number;
}
