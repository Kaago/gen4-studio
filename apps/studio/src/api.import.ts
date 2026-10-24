import type { AssetId } from "./api";
import { invoke } from "./platform/transport";

/**
 * Map part import and export, mirroring `apps/studio/src-tauri/src/commands/map_import.rs`
 * and `g4_kernel::map_import`. `api.ts` re-exports everything.
 *
 * The file extension decides the part: `.nsbmd`/`.bmd` model, `.per`/`.mp` permissions,
 * `.bld` buildings, `.bdhc`/`.bdhcam` terrain, `.bgs` background sound (HGSS only), `.bin` the
 * whole map file (also of the other family). Every check runs before anything is written; a
 * refused import changes nothing. An import is one undo step. Messages are the kernel's
 * (English, with numbers); Diamond/Pearl is refused.
 */

export type MapPart = "model" | "permissions" | "buildings" | "bdhc" | "bgs" | "whole";

/** Files (absolute paths; in the browser bridge they must lie under its roots) or another map of the project, unsaved edits included. */
export type ImportSource = { files: string[] } | { map: number };

export type PartState = { kind: "ok" } | { kind: "warning"; message: string } | { kind: "error"; message: string };

export interface PartReport {
  part: MapPart;
  /** The file it came from, null for a map of the project. */
  file: string | null;
  /** Bytes as they would be written (a model without its texture block). */
  size: number;
  state: PartState;
  /** Hints that never block (removed texture block, texture names missing in the area pack, terrain unchanged). */
  notes: string[];
}

/** Checks the parts of `source` for map `map`; changes nothing. */
export const inspectMapImport = (map: number, source: ImportSource) => invoke<PartReport[]>("inspect_map_import", { map, source });

/** Imports `parts` of `source` into map `map` as one undo step; returns the changed assets (empty when nothing changed). `keepBuildings` takes over the buildings of a whole map file of the other family. */
export const importMapParts = (map: number, source: ImportSource, parts: MapPart[], keepBuildings = false) =>
  invoke<AssetId[]>("import_map_parts", { map, source, parts, keepBuildings });

/** Writes `part` of map `map` (unsaved edits included) to `path`. */
export const exportMapPart = (map: number, part: MapPart, path: string) => invoke<void>("export_map_part", { map, part, path });
