import { Channel } from "@tauri-apps/api/core";
import { inTauri, invoke } from "../platform/transport";

/**
 * Starting a project from a ROM, mirroring `apps/studio/src-tauri/src/commands/start.rs` and `g4_kernel::start`. `api.ts`
 * re-exports everything. Field names are the Rust ones (snake_case). None of the commands opens the project: `createProject`
 * answers with its `root`, and `openProject(root)` follows when the user asks for it.
 *
 * In the dev bridge the ROM must lie under an allowed root and the destination must not end in `_DSPRE_contents` (rejected with
 * code "error"); its parent must exist.
 */

/** How far a ROM is released for the editor. `locked` and `unknown` carry the reason, in German, ready to show. */
export type Support = { kind: "verified" } | { kind: "untested" } | { kind: "locked"; reason: string } | { kind: "unknown"; reason: string };

export interface RomInfo {
  /** Header title, without trailing NULs. */
  title: string;
  /** Header game code, for example "CPUD". */
  game_code: string;
  family: "dp" | "pt" | "hgss" | null;
  language: "english" | "french" | "german" | "italian" | "spanish" | "japanese" | "korean" | null;
  revision: number;
  /** File size in bytes. */
  size: number;
  /** SHA-1 of the whole file, 40 lowercase hex digits. */
  sha1: string;
  /** Label of the entry in `base_roms.yaml` (a known test base) when SHA-1 and game code match, else null. */
  known_base: string | null;
  support: Support;
}

/** What a new project starts as. */
export type Template = "original" | "data_only" | "empty_region";

export type CreatePhase = "read" | "extract" | "template" | "check";

/** Reported at the start of each phase. */
export interface CreateProgress {
  phase: CreatePhase;
  done: number;
  total: number;
}

export type DestState = "missing" | "empty" | "not_empty" | "not_a_directory" | "unreadable";

/** Whether a project can be created at a path; `ok` also asks for 2.5 times the ROM size of free space. */
export interface DestCheck {
  state: DestState;
  /** Free bytes on the volume; null where it cannot be asked. */
  free_bytes: number | null;
  needed_bytes: number;
  ok: boolean;
}

export interface ProjectParent {
  /** Folder where new projects go by default. */
  parent: string;
  /** Whether the project folder name gets the `_DSPRE_contents` ending (the app proposes it, the bridge refuses it). */
  dspre_suffix: boolean;
}

export interface Created {
  root: string;
  game_code: string;
  template: Template;
  sha1: string;
  phases: { phase: CreatePhase; ms: number }[];
}

/** Reads the header and hashes the file (a second or so). Rejects for a file that is not a DS ROM. */
export const inspectRom = (path: string) => invoke<RomInfo>("inspect_rom", { path });

export const defaultProjectParent = () => invoke<ProjectParent>("default_project_parent");

export const checkProjectDest = (dest: string, romSize: number) => invoke<DestCheck>("check_project_dest", { dest, romSize });

/**
 * Unpacks the ROM into a new project at `dest` (missing or an empty folder); `onProgress` is called as each phase starts.
 * Takes seconds. Through the dev bridge (no channel) there are no live progress events: `onProgress` gets the first phase
 * running before the call (the work starts with reading the ROM), then each phase once, done, from the answer.
 */
export async function createProject(rom: string, dest: string, template: Template, onProgress: (progress: CreateProgress) => void): Promise<Created> {
  if (!inTauri()) {
    onProgress({ phase: "read", done: 0, total: 0 });
    const created = await invoke<Created>("create_project", { rom, dest, template });
    for (const { phase } of created.phases) onProgress({ phase, done: 1, total: 1 });
    return created;
  }
  const progress = new Channel<CreateProgress>();
  progress.onmessage = onProgress;
  return invoke<Created>("create_project", { rom, dest, template, progress });
}
