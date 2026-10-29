import { Channel } from "@tauri-apps/api/core";
import type { ChangedEntry, ResetTarget } from "./api.original";
import { inTauri, invoke } from "./platform/transport";

/**
 * Git, minimal (4.4), mirroring `apps/studio/src-tauri/src/commands/git.rs` and `g4_kernel::git`. `api.ts` re-exports everything. Field
 * names are the Rust ones (snake_case). Git stays local: „Git einrichten“ runs `git init`, `git add -A` and the first commit, never a remote.
 * The status is read in the Studio itself (gix), also without an installed `git`.
 */

/** Which repository holds the project: none, the project's own, or a larger one around it (`workdir`, its work tree; read relative to the project). */
export type RepoKind = { kind: "none" } | { kind: "own" } | { kind: "enclosing"; workdir: string };

/** The commit `HEAD` points at: the full and the short id, the first line of its message, the committer's time (Unix seconds). */
export interface HeadInfo {
  id: string;
  short: string;
  summary: string;
  time: number;
}

/** An entity as the diff names it (`g4_kernel::diff::DiffSubject`): `index` is the header id, the script file, the item id; `member` a folder without a Studio kind; `file` any other project file. */
export type DiffSubject =
  | {
      kind:
        | "map"
        | "matrix"
        | "area_data"
        | "event_file"
        | "encounter"
        | "species"
        | "move"
        | "item"
        | "trainer"
        | "header"
        | "text_archive"
        | "script"
        | "map_texture"
        | "building_texture"
        | "building_model"
        | "interior_building_model";
      index: number;
    }
  | { kind: "member"; folder: string; index: number }
  | { kind: "file"; path: string };

/**
 * The Git state of the open project. `branch` is null with a detached `HEAD` (`detached`) and without a repository; with a repository
 * without a commit it names the branch to be (`head` is null then). `changes`: the changed paths below the project since `HEAD` as
 * `git status` counts them (untracked ones too); `entries`: those paths as entities, none without a commit. `cli`: the `git` the Studio
 * found, null when none is installed. `templates`: which of `.gitignore` and `.gitattributes` exist. `ms`: how long the read took.
 */
export interface GitStatus {
  repo: RepoKind;
  branch: string | null;
  detached: boolean;
  head: HeadInfo | null;
  changes: number;
  entries: ChangedEntry[];
  cli: { version: string } | null;
  layout: "dspre" | "text_project";
  templates: string[];
  ms: number;
}

/** A step of „Git einrichten“, reported at its start; `unpack` only in the DSPRE layout. */
export type InitPhase = "unpack" | "templates" | "init" | "add" | "commit";

/** What writing the block of Gen4 Studio did to one file. */
export type TemplateAction = "created" | "appended" | "updated" | "unchanged";

/**
 * What „Git einrichten“ did. `committed` is false with a `reason` (a sentence: no `git`, already in a repository, the commit failed with
 * Git's text) when no commit was made; `head` and `files` (entries of the index) only with the commit. `studio_identity`: Git knows no name
 * yet, so the commit is by „Gen4 Studio“. `phase_ms`: the milliseconds of each phase that ran (the backend always sends it).
 */
export interface InitReport {
  templates: { layout: "dspre" | "text_project"; files: { path: string; action: TemplateAction }[]; ignored_archives: number };
  committed: boolean;
  reason: string | null;
  head: HeadInfo | null;
  files: number;
  studio_identity: boolean;
  phase_ms?: Partial<Record<InitPhase, number>>;
  ms: number;
}

/** A step of completing a cloned text project: the base layer from the base ROM, the game files copied in, the text project imported, the sync recorded. */
export type RestorePhase = "extract" | "merge" | "import" | "sync";

export interface RestoreProgress {
  phase: RestorePhase;
  done: number;
  total: number;
}

/** What completing a clone did: the entries copied into the root (a folder counts once) and the milliseconds of the whole. */
export interface RestoreReport {
  moved: number;
  ms: number;
}

/** What an entity is compared against (the old side): the base ROM, the last commit, or the saved state (the new side then the unsaved value). */
export type Against = "base" | "head" | "saved";

/** How the new side relates to the old one; `same`: equal after normalization. */
export type DiffState = "changed" | "new" | "removed" | "same";

/** What a span of a word or line diff is. */
export type SpanOp = "same" | "removed" | "added";

/** A run of words of one side of a word diff. */
export interface WordSpan {
  op: SpanOp;
  text: string;
}

/** One differing value: its JSON path (`data.party[0].level`), both sides (strings without quotes, anything else compact JSON, null where the side has none). */
export interface FieldChange {
  path: string;
  old: string | null;
  new: string | null;
}

/** One side of a binary diff: its size and SHA-1 (40 hex digits), „wie in der Basis-ROM“ (a text project keeps no file then), or missing. */
export type BlobSide = { side: "bytes"; size: number; sha1: string } | { side: "base" } | { side: "absent" };

/**
 * The diff of one entity in the shape of its kind (`g4_kernel::diff::EntityDiff`, tagged by `shape`); `more` counts the rows past the cap.
 * `map`: per part (`model`, `permissions`, `buildings`, `bdhc`, `bgs`) whether it changed with the first 8 hex digits of each side's SHA-1,
 * the permission cells that differ (`kkcc`, empty where the side has no such cell) and the buildings as fields. `lines`: hunks with their
 * 1-based start on each side, each line with what it is.
 */
export type EntityDiff =
  | { shape: "fields"; changes: FieldChange[]; more: number }
  | {
      shape: "map";
      parts: { part: string; changed: boolean; old: string | null; new: string | null }[];
      cells: { x: number; y: number; old: string; new: string }[];
      buildings: FieldChange[];
      more: number;
    }
  | { shape: "messages"; messages: { index: number; state: DiffState; old: WordSpan[]; new: WordSpan[] }[]; more: number }
  | { shape: "lines"; hunks: { old_start: number; new_start: number; lines: [SpanOp, string][] }[]; more: number }
  | { shape: "binary"; old: BlobSide; new: BlobSide };

/**
 * The diff of one entity with what the user sees around it: its name, the names of both sides (`old_label` „Basis-ROM …“, „Commit abc1234
 * „…““, „Gespeichert“; `new_label` „Projekt (gespeichert)“ or „Ungespeichert“), the state, the reset target of the subject (null for a kind
 * without one) and how long it took.
 */
export interface EntityDiffReport {
  subject: DiffSubject;
  against: Against;
  label: string;
  old_label: string;
  new_label: string;
  state: DiffState;
  diff: EntityDiff;
  reset: ResetTarget | null;
  ms: number;
}

/** The Git state of the open project; writes nothing, needs no `git`. */
export const gitStatus = () => invoke<GitStatus>("git_status");

/**
 * „Git einrichten“ on the open project (about 10 s in the DSPRE layout); `onProgress` follows its phases. Refused with `unsaved` before
 * anything is written, `busy` while it already runs. A failed commit is no error: `committed: false` with Git's reason. Through the dev
 * bridge (no channel) there are no progress events.
 */
export async function gitInit(opts: { templatesOnly?: boolean }, onProgress: (p: { phase: InitPhase }) => void): Promise<InitReport> {
  const templatesOnly = opts.templatesOnly ?? false;
  if (!inTauri()) return invoke<InitReport>("git_init", { templatesOnly });
  const progress = new Channel<{ phase: InitPhase }>();
  progress.onmessage = onProgress;
  return invoke<InitReport>("git_init", { templatesOnly, progress });
}

/**
 * Completes the cloned text project at `root` from its base ROM and opens it in the backend (tens of seconds); `nds` is a base ROM the user
 * chose (checked against the project's base mark, then remembered), null to search for it. Refused with `base_missing` (the reason) when no
 * base ROM is found. Through the dev bridge (no channel) there are no progress events.
 */
export async function restoreTextProject(root: string, nds: string | null, onProgress: (p: RestoreProgress) => void): Promise<RestoreReport> {
  if (!inTauri()) return invoke<RestoreReport>("restore_text_project", { root, nds });
  const progress = new Channel<RestoreProgress>();
  progress.onmessage = onProgress;
  return invoke<RestoreReport>("restore_text_project", { root, nds, progress });
}

/**
 * `subject` against `against`, field by field; writes nothing. The command takes the subject flat (`kind`, `index`, `folder`, `path`, as the
 * MCP tool does). Refused with `base_missing` (against the base without a found base ROM), `no_repo`, `no_commit` (against the last commit)
 * and `not_diffable` (a project file against the base, a script against the saved state, a path or folder outside the project, no regular
 * file).
 */
export const diffEntity = (subject: DiffSubject, against: Against) => invoke<EntityDiffReport>("diff_entity", { ...subject, against });

/** The store's unsaved edits as entities (the header table per header), as „Gegen gespeichert“ lists them. */
export const unsavedChanges = () => invoke<ChangedEntry[]>("unsaved_changes");
