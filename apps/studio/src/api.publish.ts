import { Channel } from "@tauri-apps/api/core";
import { inTauri, invoke } from "./platform/transport";

/**
 * Veröffentlichen (4.5), mirroring `apps/studio/src-tauri/src/commands/publish.rs` and `g4_kernel::patch::{release, publish}`. `api.ts`
 * re-exports everything. Field names are the Rust ones: snake_case, except the entries of `.g4/releases.json`, which are camelCase
 * (`baseSha1`). Refusals of `publishPatch` before anything is built: `unsaved`, `base_missing`, `bad_version`, `version_taken`, `bad_file`,
 * `bad_dir`, `exists` (without `overwrite`), `releases_broken`; after the build `no_changes`.
 */

/** One published patch as `.g4/releases.json` keeps it. `date` is RFC 3339 UTC, `file` relative to the project root (with `/`) when it lies in it, else absolute; `size` in bytes. */
export interface ReleaseEntry {
  version: string;
  date: string;
  file: string;
  notes: string;
  baseSha1: string;
  size: number;
}

/** The releases of the open project, oldest first; `stem` is the project part of a release's file name, `next_version` the proposal, `dir` the default place (`<root>/build`, absolute). */
export interface ReleasesDto {
  stem: string;
  next_version: string;
  dir: string;
  releases: ReleaseEntry[];
}

/** What to publish and where: `dir` absolute or relative to the project root (default `build/`), `file` the patch's name (`.xdelta` is added; default `<stem>-v<version>.xdelta`). */
export interface PublishOptionsDto {
  version: string;
  notes?: string;
  dir?: string;
  file?: string;
  readme?: boolean;
  overwrite?: boolean;
}

/** The phases of a release, in their order; each is reported at its start. */
export type PublishPhase = "build" | "encode" | "verify" | "write" | "record";

/** A file a release wrote (absolute path, size in bytes). */
export interface PublishedFile {
  kind: "patch" | "notes" | "readme";
  path: string;
  size: number;
}

/** What the patch of a release is (`g4_kernel::patch::PatchReport`): absolute paths, the size in bytes, both SHA-1 and the milliseconds of encoding and self-check. */
export interface PatchReportDto {
  patch: string;
  notes: string;
  rom: string;
  size: number;
  base_sha1: string;
  target_sha1: string;
  encode_ms: number;
  verify_ms: number;
}

/** What a release did: the entry it appended, the patch, the files (patch, notes, then `README.txt` when written), the list after it and the milliseconds per phase and in all. */
export interface PublishReportDto {
  release: ReleaseEntry;
  patch: PatchReportDto;
  files: PublishedFile[];
  releases: { version: number; releases: ReleaseEntry[] };
  phase_ms: Partial<Record<PublishPhase, number>>;
  ms: number;
}

/** The releases of the open project with the proposal for the next version; a broken or newer list is refused with `releases_broken`. Writes nothing. */
export const releases = () => invoke<ReleasesDto>("releases");

/**
 * Publishes a release of the saved project: builds it (`build/<Name>.nds`), writes the self-checked patch, its notes and optionally
 * `README.txt` at the chosen place and appends the entry to `.g4/releases.json`. `onProgress` follows the phases. The command takes the
 * options flat. Through the dev bridge (no channel) there are no progress events.
 */
export async function publishPatch(opts: PublishOptionsDto, onProgress: (p: { phase: PublishPhase }) => void): Promise<PublishReportDto> {
  const args: Record<string, unknown> = { ...opts };
  if (!inTauri()) return invoke<PublishReportDto>("publish_patch", args);
  const progress = new Channel<{ phase: PublishPhase }>();
  progress.onmessage = onProgress;
  return invoke<PublishReportDto>("publish_patch", { ...args, progress });
}

/** Shows a release file (or its folder) in the file manager; any other path is refused. */
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });
