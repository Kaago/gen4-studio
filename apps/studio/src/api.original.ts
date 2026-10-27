import { Channel } from "@tauri-apps/api/core";
import type { AssetId } from "./api";
import { isCmdError } from "./api";
import { inTauri, invoke } from "./platform/transport";

/**
 * „Auf Original zurücksetzen“ and the list „geändert gegenüber Original“ (4.1b), mirroring
 * `apps/studio/src-tauri/src/commands/original.rs` and `g4_kernel::original`. `api.ts` re-exports everything. Field names are the Rust
 * ones (snake_case). Without a found base ROM `planReset`, `resetToOriginal` and `changedFromOriginal` reject with code `base_missing`
 * (the message says why); an entry the base does not have with `no_original` („Kein Original: …“, „Neue Nachricht, kein Original.“); a
 * locked asset with `read_only`.
 */

/** What can be reset: `kind` is the tag, `index` the entry (a message also names its archive, a header its id, a script its file). */
export type ResetTarget =
  | {
      kind:
        | "map"
        | "event_file"
        | "text_archive"
        | "trainer"
        | "species"
        | "encounter"
        | "map_texture"
        | "building_texture"
        | "building_model"
        | "interior_building_model";
      index: number;
    }
  | { kind: "text_message"; archive: number; message: number }
  | { kind: "header"; id: number }
  | { kind: "script"; file: number };

export type ResetKind = ResetTarget["kind"];

/** One asset that differs from the base: German lines of both sides (`after` is the base). */
export interface ResetChange {
  label: string;
  before: string;
  after: string;
}

/** One header field that differs from the base: its serde name, the current value and the base value, each as JSON text. */
export interface HeaderFieldDiff {
  field: string;
  before: string;
  after: string;
}

/** What a reset of `target` would change. `same`: everything already equals the base (a reset records nothing). */
export interface ResetPlan {
  target: ResetTarget;
  label: string;
  changes: ResetChange[];
  /** The header fields that differ (a header only). */
  fields: HeaderFieldDiff[];
  /** Scripts, textures and models are written at once (a file step ⌘Z writes back); store values stay unsaved until „Speichern“. */
  writes_files: boolean;
  same: boolean;
}

export interface ResetResult {
  /** The assets whose value the reset changed. */
  changed: AssetId[];
  /** Whether a step was recorded (`false`: everything already equalled the base). */
  recorded: boolean;
  ms: number;
}

/**
 * The base ROM as resetting needs it: `ready` with its name, SHA-1 and the entries of the base per target kind (the keys are the `kind`s;
 * a message counts with `text_archive`; a kind the base or the project has no archive for is missing); `unavailable` with the German reason.
 */
export type OriginalStatus = { state: "ready"; label: string; sha1: string; counts: Record<string, number> } | { state: "unavailable"; reason: string };

export type ChangeState = "changed" | "new" | "missing";

/** One entity of the saved project that differs from the base. `target` is null where there is nothing to reset to. */
export interface ChangedEntry {
  group: string;
  label: string;
  state: ChangeState;
  target: ResetTarget | null;
  /** The store assets of the entry, so the unsaved ones can be marked. */
  assets: AssetId[];
  /** The folder below `unpacked/`; null for a header. */
  folder: string | null;
}

export interface ChangedReport {
  entries: ChangedEntry[];
  /** Members compared on the project side. */
  files: number;
  /** Member files read and hashed in this run. */
  hashed: number;
  /** Hashes written into the build state. */
  recorded: number;
  /** Unpacked folders without a build entry: hashed, nothing recorded. */
  unbuilt: string[];
  ms: number;
}

export interface ChangedProgress {
  done: number;
  total: number;
}

/** The base ROM as resetting needs it; reads the base ROM (ask once per project and after the base changed). */
export const originalStatus = () => invoke<OriginalStatus>("original_status");
/** What a reset of `target` would change; changes nothing. */
export const planReset = (target: ResetTarget) => invoke<ResetPlan>("plan_reset", { target });
/** Resets `target` to the base ROM as one undo step; `fields` picks header fields by serde name (default: every differing one). */
export const resetToOriginal = (target: ResetTarget, fields?: string[]) =>
  invoke<ResetResult>("reset_to_original", fields === undefined ? { target } : { target, fields });

/**
 * What of the saved project differs from the base ROM; `onProgress` follows the hashing. Through the dev bridge (no channel) there are no
 * progress events: `onProgress` gets one final progress from the report.
 */
export async function changedFromOriginal(onProgress: (progress: ChangedProgress) => void): Promise<ChangedReport> {
  if (!inTauri()) {
    const report = await invoke<ChangedReport>("changed_from_original");
    onProgress({ done: report.hashed, total: report.hashed });
    return report;
  }
  const progress = new Channel<ChangedProgress>();
  progress.onmessage = onProgress;
  return invoke<ChangedReport>("changed_from_original", { progress });
}

/** The entry of `target` checked against the count of the base: the index, the archive of a message, the header id, the script file. */
export function targetIndex(target: ResetTarget): number {
  switch (target.kind) {
    case "text_message":
      return target.archive;
    case "header":
      return target.id;
    case "script":
      return target.file;
    default:
      return target.index;
  }
}

/** The key of `target`'s kind in `OriginalStatus.counts`: a message counts with its archive. */
export function countKey(target: ResetTarget): string {
  return target.kind === "text_message" ? "text_archive" : target.kind;
}

const WORDS: Record<Exclude<ResetKind, "text_message">, string> = {
  map: "Karte",
  event_file: "Eventdatei",
  text_archive: "Textarchiv",
  trainer: "Trainer",
  species: "Pokémon",
  encounter: "Begegnungsdatei",
  header: "Header",
  script: "Scriptdatei",
  map_texture: "Kartentextur",
  building_texture: "Gebäudetextur",
  building_model: "Gebäudemodell",
  interior_building_model: "Innenraum-Gebäudemodell",
};

const id4 = (n: number) => String(n).padStart(4, "0");

/** The German name of the entity as the kernel gives it (`ResetTarget::label`): „Karte 0003“, „Nachricht 2 in Textarchiv 0005“. */
export function targetLabel(target: ResetTarget): string {
  if (target.kind === "text_message") return `Nachricht ${target.message} in Textarchiv ${id4(target.archive)}`;
  return `${WORDS[target.kind]} ${id4(targetIndex(target))}`;
}

/** The sentence of a failed plan or reset: the kernel's German text; a locked asset only its German reason (after „… is read-only: “). */
export function resetErrorText(e: unknown): string {
  if (!isCmdError(e)) return String(e);
  if (e.code !== "read_only") return e.message;
  const marker = " is read-only: ";
  const at = e.message.indexOf(marker);
  return at < 0 ? e.message : e.message.slice(at + marker.length);
}
