import type { ChangedEntry, ChangedReport } from "../api";
import type { ChangesApi } from "./useChanges";

/** Test data for Technik › Änderungen: a report of the saved project against the base ROM. */

export const entry = (over: Partial<ChangedEntry> = {}): ChangedEntry => ({
  group: "Karten",
  label: "Karte 0003",
  state: "changed",
  target: { kind: "map", index: 3 },
  assets: [{ kind: "Map", index: 3 }],
  folder: "maps",
  ...over,
});

export const report = (over: Partial<ChangedReport> = {}): ChangedReport => ({ entries: [], files: 0, hashed: 0, recorded: 0, unbuilt: [], ms: 12, ...over });

/** A compare that is through (an empty report unless given); `compare` does nothing unless given. */
export const changesWith = (over: Partial<ChangesApi> = {}): ChangesApi => ({ report: report(), progress: null, error: null, busy: false, compare: async () => {}, ...over });
