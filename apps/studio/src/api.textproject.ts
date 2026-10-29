import { Channel } from "@tauri-apps/api/core";
import type { AssetId } from "./api";
import type { InitReport } from "./api.git";
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

/** At most 40 lines; `more` counts the rest. `unsaved`: the Studio's side is the store's unsaved value of the unit (what the editor shows). */
export interface ConflictDiff {
  lines: DiffLine[];
  more: number;
  unsaved: boolean;
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

/** A phase of setting the project up as text project; `scripts` has `total` 0 when the scripts were adopted already. */
export type AdoptPhase = "scripts" | "base_layer" | "entities" | "sync";

/** How far the set-up is: `done` of `total` in `phase` (each phase reports `done` 0 first and `done` = `total` last). */
export interface AdoptProgress {
  phase: AdoptPhase;
  done: number;
  total: number;
}

/** One art folder of the export (`data/<art>/`). */
export interface ArtStats {
  art: string;
  entities: number;
  json_files: number;
  side_files: number;
  written: number;
  removed: number;
  bytes: number;
  ms: number;
}

/** What the export of the set-up wrote (4.3a): `files` counts every file of the text project, `written` the ones it wrote. */
export interface ExportReport {
  arts: ArtStats[];
  raw_blobs: number;
  raw_bytes: number;
  raw_ms: number;
  not_built: string[];
  texts: number;
  text_json_written: number;
  scripts: number;
  adopted: number | null;
  registries_written: number;
  base_layer_ms: number;
  base_layer_created: boolean;
  files: number;
  bytes: number;
  written: number;
  removed: number;
  problems: SyncProblem[];
  ms: number;
}

/**
 * The Git templates written anew after a set-up or a take-back, in a project whose `.gitignore` holds the block of Gen4 Studio (null
 * without the block); `templates_warning` the sentence when that failed (the set-up or take-back still went in).
 */
export interface TemplatesRewritten {
  templates: InitReport["templates"] | null;
  templates_warning: string | null;
}

/** What the set-up did: the export, whether the id files for rotom were written, how many files the sync recorded, the Git templates, the milliseconds. */
export interface AdoptReport extends TemplatesRewritten {
  export: ExportReport;
  include_written: boolean;
  sync_files: number;
  ms: number;
}

/** What taking the text project back did: the backup folder (absolute), the files written with numbers instead of ids, the Git templates, the milliseconds. */
export interface RevertReport extends TemplatesRewritten {
  backup: string;
  rewritten: string[];
  ms: number;
}

/** What lifting an older text project did: the schema versions, the files it changed, and the snapshot folder (absolute) or null. */
export interface MigrateReport {
  from: number;
  to: number;
  changed: string[];
  snapshot: string | null;
}

/** The kinds of symbolic ids (`ids::Resolver`). */
export type SymbolKind = "Species" | "Move" | "Item" | "Ability" | "Type" | "Map" | "Trainer" | "TrainerClass" | "Place";

/** The kinds whose id a head shows: Pokémon, header (`Map`, by header number) and trainer. */
export type EntityKind = "Species" | "Trainer" | "Map";

/** One file a rename writes, with the number of references in it. */
export interface RenameFile {
  path: string;
  count: number;
}

/**
 * What renaming `old_id` to `new_id` would write: the files and their references (`total`), whether the id files for rotom change
 * (`id_files`); `refusal` says why it cannot be done (a taken id, a fallback id of another number, wrong characters), nothing written then.
 * `kind` and `number` are null for an id the project does not know.
 */
export interface RenamePlan {
  kind: SymbolKind | null;
  number: number | null;
  old_id: string;
  new_id: string;
  files: RenameFile[];
  total: number;
  id_files: boolean;
  refusal: string | null;
}

/** A rename that went in: its plan, the undo step (`change`), the milliseconds and what did not go as it should (`warnings`). */
export interface RenameResult {
  plan: RenamePlan;
  change: number;
  ms: number;
  warnings: string[];
}

/**
 * Sets the DSPRE folder up as text project (10 to 120 s); `onProgress` follows its phases. The backend opens the session anew: the app
 * reads it again as after an agent's reopen. Through the dev bridge (no channel) there are no progress events. Refused with `unsaved`,
 * `base_missing`, `pending` or `error` (Diamond/Pearl, or the session changed meanwhile: then the files are set up and the project must be
 * opened anew).
 */
export async function adoptTextProject(onProgress: (progress: AdoptProgress) => void): Promise<AdoptReport> {
  if (!inTauri()) return invoke<AdoptReport>("adopt_text_project");
  const progress = new Channel<AdoptProgress>();
  progress.onmessage = onProgress;
  return invoke<AdoptReport>("adopt_text_project", { progress });
}

/** Takes the text project back (manifest and `data/` to `.g4/backup/`); the session is opened anew. Refused with `unsaved` or `pending`. */
export const revertTextProject = () => invoke<RevertReport>("revert_text_project");

/** Lifts a text project of an older schema to the current one; the session is opened anew when something changed. */
export const migrateTextProject = () => invoke<MigrateReport>("migrate_text_project");

/** The plan of renaming the register id `id` to `newId`; writes nothing. */
export const planRenameId = (id: string, newId: string) => invoke<RenamePlan>("plan_rename_id", { id, newId });

/** Renames the register id `id` to `newId` with every reference, as one undo step (⌘Z writes all files back). */
export const renameId = (id: string, newId: string) => invoke<RenameResult>("rename_id", { id, newId });

/** The ids of `kind` by number (`{25: "SPECIES_PIKACHU"}`); a register kind outside a text project has none. */
export const entityIds = (kind: EntityKind) => invoke<Record<number, string>>("entity_ids", { kind });
