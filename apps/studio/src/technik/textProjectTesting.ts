import type { ConflictEntry, SyncOutcome, TextProjectStatus } from "../api";
import type { ExternalSyncApi } from "../app/useExternalSync";
import type { TextProjectView } from "./TextProjectSurface";

/** Test data for Technik › Textprojekt: a sync's outcome, the sync as the app holds it, and the view the surface gets. */

export const outcome = (over: Partial<SyncOutcome> = {}): SyncOutcome => ({ adopted: [], applied: [], conflicts: [], broken: [], reloaded: [], full: false, ms: 3, ...over });

export const conflict = (over: Partial<ConflictEntry> = {}): ConflictEntry => ({
  key: "moves/0001",
  label: "Attacke 0001 Pfund",
  assets: [{ kind: "Move", index: 1 }],
  unsaved: false,
  other: "dspre",
  world: false,
  ...over,
});

/** A sync that is through with nothing found unless given; `run` answers its `last`. */
export const syncWith = (over: Partial<ExternalSyncApi> = {}): ExternalSyncApi => {
  const api: ExternalSyncApi = { last: outcome(), taken: null, running: false, progress: null, error: null, run: async () => api.last, ...over };
  return api;
};

export const CURRENT: TextProjectStatus = { state: "current", schema: 1, last_sync_at: null };

/** A current text project, synced, with nothing open, unless given. */
export const textWith = (over: Partial<TextProjectView> = {}): TextProjectView => ({
  status: CURRENT,
  sync: syncWith(),
  busy: false,
  onResolved: () => {},
  onSetUp: () => {},
  onRevert: () => {},
  ...over,
});
