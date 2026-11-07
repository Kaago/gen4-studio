import type { AssetId } from "./api";
import { invoke } from "./platform/transport";

/**
 * Map part import and export, mirroring `apps/studio/src-tauri/src/commands/map_import.rs`
 * and `g4_kernel::map_import`. `api.ts` re-exports everything.
 *
 * The file extension decides the part: `.nsbmd`/`.bmd` model, `.obj`/`.gltf`/`.glb` a model
 * converted by the baker, `.png` textures (all PNG files of a call are one part, appended to the
 * area's map texture pack), `.per`/`.mp` permissions, `.bld` buildings, `.bdhc`/`.bdhcam`
 * terrain, `.bgs` background sound (HGSS only), `.bin` the whole map file (also of the other
 * family). Every check runs before anything is written; a
 * refused import changes nothing. An import is one undo step. What the checks say carries a
 * code the dialog words in German (`workbench/inspector/importParts.ts`) and the kernel's
 * English sentence with the numbers; Diamond/Pearl is refused.
 */

export type MapPart = "model" | "permissions" | "buildings" | "bdhc" | "bgs" | "whole" | "textures";

/** Files (absolute paths; in the browser bridge they must lie under its roots) or another map of the project, unsaved edits included. */
export type ImportSource = { files: string[] } | { map: number };

/** What a check of a part says: its `code` with the `args` the dialog words it from, and the kernel's English sentence (the fallback). */
export interface PartMessage {
  code: string;
  args: string[];
  message: string;
}

export type PartState = { kind: "ok" } | ({ kind: "warning" | "error" } & PartMessage);

export interface PartReport {
  part: MapPart;
  /** The file it came from, null for a map of the project. */
  file: string | null;
  /** Bytes as they would be written (a model without its texture block). */
  size: number;
  state: PartState;
  /** Hints that never block (removed texture block, texture names missing in the area pack, terrain unchanged). */
  notes: PartMessage[];
  /** The figures of a converted model or of textures (`.obj`/`.gltf`/`.glb`, `.png`), only when their checks got that far. */
  preview?: ImportPreview;
}

/**
 * How OBJ, glTF and PNG files are converted (`g4_kernel::model_import::ImportOptions`): `unit` world units per model unit (16 = one tile,
 * 0.0625 to 256), `colors` per texture. A missing key takes the default (16 and 16); a value out of range refuses the call.
 */
export interface ImportOptions {
  unit?: number;
  colors?: 4 | 16;
}

/** A texture the import uses: new (converted from `file`, with its `colors` and mean colour error) or one of the pack (`colors` 0, `mean_error` null). */
export interface TexturePreview {
  name: string;
  file: string | null;
  width: number;
  height: number;
  colors: number;
  new: boolean;
  mean_error: number | null;
}

/** The figures of a converted model: polygons read, quads and triangles baked, faces dropped, quad strips and single quads, its size against the game's 0xF000 bytes. */
export interface ModelPreview {
  format: "obj" | "gltf";
  stats: { polygons: number; quads: number; triangles: number; dropped: number };
  strips: number;
  single_quads: number;
  budget: { used: number; max: number; state: "calm" | "tight" | "full" };
}

/** What the dialog shows of a converted part before the import (snake_case as the kernel sends it): the area's map texture pack, the area and its style, its VRAM with the new textures and the largest vanilla area's, the textures, the model and the files. */
export interface ImportPreview {
  pack: number;
  area: number;
  style: "outdoor" | "indoor";
  vram_bytes: number;
  vram_limit: number;
  textures: TexturePreview[];
  model: ModelPreview | null;
  files: string[];
}

/** Checks the parts of `source` for map `map`; changes nothing. `options` convert models and images (left out: the defaults). */
export const inspectMapImport = (map: number, source: ImportSource, options?: ImportOptions) =>
  invoke<PartReport[]>("inspect_map_import", options === undefined ? { map, source } : { map, source, options });

/** Imports `parts` of `source` into map `map` as one undo step; returns the changed assets (empty when nothing changed). `keepBuildings` takes over the buildings of a whole map file of the other family; `options` convert models and images as for the check. */
export const importMapParts = (map: number, source: ImportSource, parts: MapPart[], keepBuildings = false, options?: ImportOptions) =>
  invoke<AssetId[]>("import_map_parts", options === undefined ? { map, source, parts, keepBuildings } : { map, source, parts, keepBuildings, options });

/** Writes `part` of map `map` (unsaved edits included) to `path`. */
export const exportMapPart = (map: number, part: MapPart, path: string) => invoke<void>("export_map_part", { map, part, path });
