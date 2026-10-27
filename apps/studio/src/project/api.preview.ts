import { invoke } from "../platform/transport";

/**
 * The project preview for the start list, mirroring `apps/studio/src-tauri/src/commands/preview.rs` and
 * `g4_kernel::preview` (phase 4.0 design, §3.3). `api.ts` re-exports everything.
 *
 * Without `path` both commands work on the open project (rejected with "no project open" without one); with `path` they
 * touch only `<path>/.g4/preview.png` and `<path>/.g4/cache/preview-standard.png`, without opening the project. A folder
 * that is gone or has no valid preview answers `source: null`. In the dev bridge `path` and `file` must lie under an
 * allowed root.
 */

/** Which file a preview came from: the own picture (`.g4/preview.png`) or the standard one (the start town's map). */
export type PreviewSource = "eigen" | "standard";

export type PreviewDto = {
  source: PreviewSource | null;
  /** Base64 of a PNG of 256 × 192. */
  png: string | null;
  /** The start town the app renders the standard picture of; only for the open project, null without a recipe. */
  standardMap: { header: number; map: number; areaDataId: number } | null;
};

export const projectPreviewGet = (path?: string) => invoke<PreviewDto>("project_preview_get", { path: path ?? null });

/**
 * Writes `png` (base64 of a PNG or JPEG of any size) or `file` (path of one) into `slot` (default "eigen"), fitted to
 * 256 × 192; with neither it removes that slot's picture. Answers the preview after the write. Both together are rejected
 * („png und file schließen sich aus; nur eins von beiden angeben“); a picture that cannot be used is rejected with a code in the message
 * (`preview.format`, `preview.too_large`, `preview.unreadable`). No undo.
 */
export const projectPreviewSet = (args: { path?: string; slot?: PreviewSource; png?: string; file?: string }) =>
  invoke<PreviewDto>("project_preview_set", { path: null, slot: null, png: null, file: null, ...args });
