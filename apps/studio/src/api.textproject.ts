import { Channel } from "@tauri-apps/api/core";
import type { AssetId } from "./api";
import { inTauri, invoke } from "./platform/transport";

/**
 * The text project as source (4.3b), mirroring `apps/studio/src-tauri/src/commands/textsource.rs` and `g4_kernel::textproject::sync`.
 * `api.ts` re-exports everything. Field names are the Rust ones (snake_case). Outside a text project `externalSync`, `conflictDiff` and
 * `resolveExternalConflict` reject with code `not_text_project`; a write while the project is being set up with `busy`.
 */

/** What the open folder is: a DSPRE folder (with whether it can be set up and why not), or a text project of a schema version. */
export type TextProjectStatus =
  | { state: "dspre"; adoptable: boolean; reason: string | null }
  /** `last_sync_at`: when the sync last recorded (Unix milliseconds), null before the first. */
  | { state: "current"; schema: number; last_sync_at: number | null }
  /** Read only until it is lifted. */
  | { state: "older"; schema: number }
  /** Not opened (kept for completeness). */
  | { state: "newer"; schema: number }
  /** A manifest that does not read; not opened. `reason` names file, line and column. */
  | { state: "broken"; reason: string };

/** A problem of a source file (the kernel's `Problem`: lines and columns 1-based, `file` relative to the project with `/`). */
export interface SyncProblem {
  severity: "error" | "warning" | "info";
  file: string;
  line: number;
  col: number;
  message: string;
}

/** One unit of the sync (an entity, a text archive, a script, a blob): its key, its German label and the store's assets it loads. */
export interface SyncEntry {
  key: string;
  label: string;
  assets: AssetId[];
}

/**
 * A unit changed on both sides since the last record, or one side of a unit the store holds unsaved (`unsaved`). `other`: what changed
 * outside, DSPRE's member („DSPRE-Fassung übernehmen“) or the source file („Geänderte Datei übernehmen“).
 */
export interface ConflictEntry extends SyncEntry {
  unsaved: boolean;
  other: "dspre" | "files";
}

/** A unit whose source does not read: nothing of it was written. */
export interface BrokenEntry {
  key: string;
  label: string;
  problems: SyncProblem[];
}

/**
 * What a sync did: `adopted` took DSPRE's side over („aus DSPRE“), `applied` a changed source („aus Datei“); `conflicts` and `broken`
 * wait for the user. `reloaded`: the store's assets loaded anew (the UI reads them again). `full`: the first sync, every file hashed.
 */
export interface SyncOutcome {
  adopted: SyncEntry[];
  applied: SyncEntry[];
  conflicts: ConflictEntry[];
  broken: BrokenEntry[];
  reloaded: AssetId[];
  full: boolean;
  ms: number;
}

export interface SyncProgress {
  done: number;
  total: number;
}

/** One differing field (a JSON pointer, a script line, a size and hash) with the Studio's value and the other side's, each as compact text. */
export interface DiffLine {
  path: string;
  studio: string;
  dspre: string;
}

/** At most 40 lines; `more` counts the rest. */
export interface ConflictDiff {
  lines: DiffLine[];
  more: number;
}

/** „Studio-Fassung behalten“ or the other side („DSPRE-Fassung übernehmen“, „Geänderte Datei übernehmen“). */
export type Keep = "studio" | "dspre";

/** The open folder's state as a text project. */
export const textProjectStatus = () => invoke<TextProjectStatus>("text_project_status");

/**
 * Takes over what changed outside on one side only and lists the rest; `onProgress` follows the scan. Through the dev bridge (no channel)
 * there are no progress events.
 */
export async function externalSync(onProgress: (progress: SyncProgress) => void): Promise<SyncOutcome> {
  if (!inTauri()) return invoke<SyncOutcome>("external_sync");
  const progress = new Channel<SyncProgress>();
  progress.onmessage = onProgress;
  return invoke<SyncOutcome>("external_sync", { progress });
}

/** The diff of conflict `key`: the Studio's side and the other one; writes nothing. */
export const conflictDiff = (key: string) => invoke<ConflictDiff>("conflict_diff", { key });

/**
 * The user's choice for conflict `key`; answers the assets loaded anew. `dspre` drops the unsaved values of the unit's assets; `studio`
 * against a changed source file (`other = files`) takes the file as the base of the unsaved value, which drops that asset's undo history.
 */
export const resolveExternalConflict = (key: string, keep: Keep) => invoke<AssetId[]>("resolve_external_conflict", { key, keep });
