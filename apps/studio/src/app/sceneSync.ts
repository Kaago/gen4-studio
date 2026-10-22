import type { HeaderSummary } from "../api";
import { afterFailedSceneLoad, headerSync, sceneKey, tilesetReload } from "../reload";

/**
 * The bookkeeping that keeps the shown scene consistent while headers are selected, the header
 * list is read again and the history moves. All of it is decided here; `useSceneSync` only holds
 * the state in a ref (the decisions must see it at once, not on the next render) and acts on the
 * outcomes.
 */
export interface SyncState {
  /** The latest header selection (or project switch) wins: a slower matrix fetch of an earlier one is dropped. */
  selectSeq: number;
  /** The latest header list read wins. */
  listSeq: number;
  /** The matrix and event file the shown scene was loaded for (see `sceneKey`); null when unknown. */
  key: string | null;
  /** The area data the scene should show (ahead of the shown one while a matrix is being fetched). */
  area: number;
  /** Scene matrices being fetched; the area is only changed alone while none is, so it never pairs with the old matrix. */
  loads: number;
  /** The tilesets of the area data the scene's models were requested with. */
  tileset: { area: number; key: string } | null;
  /** Bumped by every undo and redo: a fetch that was in flight meanwhile fetches again (see `fetchCurrent`). */
  history: number;
}

export function initialSync(): SyncState {
  return { selectSeq: 0, listSeq: 0, key: null, area: 0, loads: 0, tileset: null, history: 0 };
}

export type SyncAction =
  /** Another project was opened: every guard moves on and the scene's bookkeeping is forgotten. */
  | { type: "opened" }
  /** A header was selected; its matrix is about to be fetched. */
  | { type: "select"; header: HeaderSummary }
  /** The header list is about to be read. */
  | { type: "listStart" }
  /** The header list read under `seq` arrived; `header` is the selected header in it (undefined: none). */
  | { type: "listed"; seq: number; header: HeaderSummary | undefined }
  /** The matrix fetched under `seq` arrived. */
  | { type: "loaded"; seq: number }
  /** The matrix fetch under `seq` failed; `shownArea` is the area data the scene shows right now. */
  | { type: "loadFailed"; seq: number; shownArea: number }
  /** A matrix fetch ended, however it ended. */
  | { type: "loadEnded" }
  /** Undo or redo returned. */
  | { type: "historyMoved" }
  /** The tilesets of `area` were read; with `reload`, differing ones reload the models. */
  | { type: "tilesetRead"; area: number; key: string; reload: boolean }
  /** The area data `area` could not be read. */
  | { type: "tilesetFailed"; area: number };

export type SyncOutcome =
  | { kind: "none" }
  /** The result belongs to something that a newer selection, list read or project overtook: drop it. */
  | { kind: "stale" }
  /** The list read is current but the selected header is not in it. */
  | { kind: "noHeader" }
  /** Read the header list under the guard `seq` (answered by `listed`). */
  | { kind: "list"; seq: number }
  /** Fetch the matrix of the selection started under `seq` (and apply it with `loaded`). */
  | { kind: "load"; seq: number }
  /** The scene keeps its maps and reloads its models for `area`; `apply` is false while a matrix is in flight (it brings the area along). */
  | { kind: "area"; area: number; apply: boolean }
  /** The fetched matrix is current: show it together with `area`. */
  | { kind: "apply"; area: number }
  /** The current fetch failed: the scene bookkeeping was reset to what is shown. */
  | { kind: "reset" }
  /** Tilesets recorded, nothing to reload. */
  | { kind: "noted" }
  /** The tilesets changed: reload the scene's models. */
  | { kind: "models" }
  /** An unreadable area data of the shown scene: tell the user. */
  | { kind: "report" };

export interface SyncResult {
  state: SyncState;
  outcome: SyncOutcome;
}

const same = (state: SyncState): SyncResult => ({ state, outcome: { kind: "none" } });
const stale = (state: SyncState): SyncResult => ({ state, outcome: { kind: "stale" } });

/** Whether the selection started under `seq` is still the newest (nothing was selected or opened since). */
export function isCurrentSelection(state: SyncState, seq: number): boolean {
  return seq === state.selectSeq;
}

export function sceneSync(state: SyncState, action: SyncAction): SyncResult {
  switch (action.type) {
    case "opened":
      return same({ ...state, selectSeq: state.selectSeq + 1, listSeq: state.listSeq + 1, key: null, area: 0, tileset: null });
    case "select": {
      const next = { ...state, selectSeq: state.selectSeq + 1, key: sceneKey(action.header), area: action.header.area_data_id, loads: state.loads + 1 };
      return { state: next, outcome: { kind: "load", seq: next.selectSeq } };
    }
    case "listStart": {
      const next = { ...state, listSeq: state.listSeq + 1 };
      return { state: next, outcome: { kind: "list", seq: next.listSeq } };
    }
    case "listed": {
      if (action.seq !== state.listSeq) return stale(state);
      const header = action.header;
      if (!header) return { state, outcome: { kind: "noHeader" } };
      const withArea = { ...state, area: header.area_data_id };
      if (headerSync(state.key, header) === "area") {
        // Only the area data changed: the scene keeps its maps and reloads its models.
        return { state: withArea, outcome: { kind: "area", area: header.area_data_id, apply: state.loads === 0 } };
      }
      const next = { ...withArea, key: sceneKey(header), selectSeq: state.selectSeq + 1, loads: state.loads + 1 };
      return { state: next, outcome: { kind: "load", seq: next.selectSeq } };
    }
    case "loaded":
      return isCurrentSelection(state, action.seq) ? { state, outcome: { kind: "apply", area: state.area } } : stale(state);
    case "loadFailed": {
      if (!isCurrentSelection(state, action.seq)) return stale(state);
      const reset = afterFailedSceneLoad(action.shownArea);
      return { state: { ...state, key: reset.key, area: reset.area }, outcome: { kind: "reset" } };
    }
    case "loadEnded":
      return same({ ...state, loads: state.loads - 1 });
    case "historyMoved":
      return same({ ...state, history: state.history + 1 });
    case "tilesetRead": {
      // Another header was selected meanwhile; its own note follows.
      if (action.area !== state.area) return stale(state);
      const next = { ...state, tileset: { area: action.area, key: action.key } };
      const reload = action.reload && tilesetReload(state.tileset, action.area, action.key);
      return { state: next, outcome: { kind: reload ? "models" : "noted" } };
    }
    case "tilesetFailed": {
      // Nothing to compare: the next successful read reloads the models if it is asked to.
      const next = { ...state, tileset: null };
      return { state: next, outcome: action.area === state.area ? { kind: "report" } : { kind: "stale" } };
    }
  }
}
