import type { AssetId, HeaderSummary } from "./api";

/** The asset kinds that are files rather than store values: a reset to original writes them at once, as a file step undo and redo write back. */
const FILE_KINDS: ReadonlySet<AssetId["kind"]> = new Set<AssetId["kind"]>(["Script", "MapTexture", "BuildingTexture", "BuildingModel", "InteriorBuildingModel"]);

/** Whether any of `assets` is a file (see `FILE_KINDS`): restoring or resetting it wrote the saved project. */
export function writesFiles(assets: readonly AssetId[]): boolean {
  return assets.some((a) => FILE_KINDS.has(a.kind));
}

/**
 * Store kinds a file step of the history reports too: the header table (a story made, changed or deleted, a flag or variable named, an
 * entity deleted with its files, an id renamed) and a trainer (its id renamed). The steps that write the files of the other file steps report
 * a file kind (`FILE_KINDS`; a recipe always a script).
 */
const FILE_STEP_STORE_KINDS: ReadonlySet<AssetId["kind"]> = new Set<AssetId["kind"]>(["HeaderTable", "Trainer"]);

/**
 * Whether undo or redo of a step that restored `assets` may have written the saved project: a file kind, or a store kind a file step
 * reports (`FILE_STEP_STORE_KINDS`). The assets do not tell a file step of the header table or a trainer from a store step of them, so
 * those count as written (a needless compare at worst); a store step of any other kind wrote nothing.
 */
export function historyWritesFiles(assets: readonly AssetId[]): boolean {
  return writesFiles(assets) || assets.some((a) => FILE_STEP_STORE_KINDS.has(a.kind));
}

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
  /** The map's raster picture is drawn again (it may show the restored model). */
  | { kind: "thumbnail"; map: number }
  /** The scene reloads this map's terrain model: a restored map can carry another model (undo of a parts import). */
  | { kind: "mapModel"; map: number }
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
  /**
   * The scene compares the tilesets of its area data and reloads its models if they changed; with `files` it reloads them in any case (a
   * texture or model file was written under the same numbers: a reset to original).
   */
  | { kind: "models"; files?: true }
  /** The text panel re-reads this archive. */
  | { kind: "textPanel"; archive: number }
  /** The header list and editor re-read the table; the scene reloads if the selected header's matrix, area data or event file changed. */
  | { kind: "headers" }
  /** The data areas (Pokémon, items) re-read the restored member. */
  | { kind: "dataPanel"; asset: AssetId };

/**
 * What to reload after undo or redo restored `asset`, so that exactly what the asset affects is
 * refreshed and nothing else. `scene` is null while no scene is shown.
 */
export function reloadPlan(
  asset: AssetId,
  scene: SceneState | null,
): ReloadAction[] {
  switch (asset.kind) {
    case "Map": {
      const actions: ReloadAction[] = [
        { kind: "unpin" },
        { kind: "buildings" },
        { kind: "thumbnail", map: asset.index },
      ];
      if (scene?.maps.includes(asset.index))
        actions.push({ kind: "map", map: asset.index }, { kind: "mapModel", map: asset.index });
      return actions;
    }
    case "WorldMap": {
      // A world edit (or its ⌘Z) changes the map's permissions and heights and the mesh the scene shows instead of its model.
      const actions: ReloadAction[] = [];
      if (scene?.maps.includes(asset.index)) actions.push({ kind: "map", map: asset.index }, { kind: "mapModel", map: asset.index });
      actions.push({ kind: "thumbnail", map: asset.index });
      return actions;
    }
    case "Matrix": {
      const actions: ReloadAction[] = [
        { kind: "unpin" },
        { kind: "matrixPanel" },
      ];
      if (scene?.matrix === asset.index) actions.push({ kind: "matrix" });
      return actions;
    }
    case "Event": {
      const actions: ReloadAction[] = [
        { kind: "unpin" },
        { kind: "eventsPanel" },
      ];
      if (scene?.eventFile === asset.index)
        actions.push({ kind: "events", file: asset.index });
      return actions;
    }
    case "AreaData": {
      const actions: ReloadAction[] = [{ kind: "areaPanel" }];
      if (scene?.area === asset.index) actions.push({ kind: "models" });
      return actions;
    }
    case "TextArchive":
      // The location bank is a text archive: its names are part of the header list (the tree, the picker „Ort“), so they follow too.
      return [{ kind: "textPanel", archive: asset.index }, { kind: "headers" }];
    case "HeaderTable":
      return [{ kind: "unpin" }, { kind: "headers" }];
    case "Script":
      // Only the file step of a reset to original (its ⌘Z, ⌘⇧Z) restores a script file: the app reads an open document again
      // (`useHistoryReload`'s `onFilesRestored`), nothing of the world shows it.
      return [];
    case "MapTexture":
    case "BuildingTexture":
      // Only a reset to original (or its ⌘Z) writes a texture: the tileset numbers stay, the pictures change.
      return [{ kind: "models", files: true }];
    case "BuildingModel":
    case "InteriorBuildingModel":
      // The same for a building model; the building editor reads its building (and its model) again.
      return [{ kind: "buildings" }, { kind: "models", files: true }];
    case "Encounter":
    case "Personal":
    case "Learnset":
    case "Evolution":
    case "Move":
    case "Item":
    case "TrainerProperties":
    case "TrainerParty":
    case "TrainerTextTable":
    case "TrainerTextOffsets":
    case "Trainer":
    case "EggMoves":
      return [{ kind: "dataPanel", asset }];
    default:
      return [];
  }
}

/**
 * What to reload after one undo or redo step restored `assets` (a step can change several): the
 * `reloadPlan` of every asset, each action once, in the order they first appear.
 */
export function historyPlan(
  assets: readonly AssetId[],
  scene: SceneState | null,
): ReloadAction[] {
  const seen = new Set<string>();
  return assets
    .flatMap((asset) => reloadPlan(asset, scene))
    .filter((action) => {
      const key = JSON.stringify(action);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
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
export function headerSync(
  shownKey: string | null,
  header: HeaderSummary,
): "scene" | "area" {
  return sceneKey(header) === shownKey ? "area" : "scene";
}

/**
 * The scene bookkeeping after a matrix fetch failed: the scene keeps showing what it showed, so the
 * key is forgotten (the next sync rebuilds instead of pairing the old maps with the new header's
 * area data) and the area is the one still shown.
 */
export function afterFailedSceneLoad(shownArea: number): {
  key: null;
  area: number;
} {
  return { key: null, area: shownArea };
}

/**
 * Whether the scene's models must reload: the tilesets of `area` (`key`) differ from the ones seen
 * before. Another area data is left to the scene load that comes with it; unknown tilesets (an
 * earlier read failed) reload to be safe.
 */
export function tilesetReload(
  known: { area: number; key: string } | null,
  area: number,
  key: string,
): boolean {
  return !known || (known.area === area && known.key !== key);
}

/**
 * Runs `fetch` and runs it again as long as `revision` (bumped by every undo and redo) moved while
 * it was in flight, so a fetch that raced with an undo never delivers the state from before it.
 */
export async function fetchCurrent<T>(
  fetch: () => Promise<T>,
  revision: () => number,
): Promise<T> {
  for (;;) {
    const before = revision();
    const value = await fetch();
    if (revision() === before) return value;
  }
}

/**
 * Requests to the scene ("refetch this map") as a queue instead of one slot: several requests made
 * before React runs the scene's effect (two undos in a row, say) are all kept. Every entry has a
 * running `seq`; the scene remembers the last one it handled and takes what is newer.
 */
export interface RefreshQueue<T> {
  entries: { seq: number; item: T }[];
}

/** The most requests kept; older ones were handled long ago (a scene looks after every render). */
export const MAX_QUEUED = 32;

/** `queue` with `item` appended; the one stored in React state, so it is built in the updater. */
export function enqueue<T>(
  queue: RefreshQueue<T> | null,
  item: T,
): RefreshQueue<T> {
  const entries = queue?.entries ?? [];
  const seq = (entries[entries.length - 1]?.seq ?? 0) + 1;
  return { entries: [...entries, { seq, item }].slice(-MAX_QUEUED) };
}

/**
 * Like `pendingSince`, but while the scene is not `ready` (its first load still builds) nothing is
 * handed out and nothing is consumed: the requests stay queued until the effect runs again once the
 * scene is built.
 */
export function drainWhenReady<T>(
  queue: RefreshQueue<T> | null,
  after: number,
  ready: boolean,
): { items: T[]; last: number } {
  return ready ? pendingSince(queue, after) : { items: [], last: after };
}

/** The requests newer than `after` (the last `seq` the scene handled) and the new last `seq`. */
export function pendingSince<T>(
  queue: RefreshQueue<T> | null,
  after: number,
): { items: T[]; last: number } {
  const fresh = (queue?.entries ?? []).filter((e) => e.seq > after);
  return {
    items: fresh.map((e) => e.item),
    last: fresh.length ? fresh[fresh.length - 1].seq : after,
  };
}
