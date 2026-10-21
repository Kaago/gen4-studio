import { invoke } from "@tauri-apps/api/core";

export interface ProjectInfo {
  root: string;
  game_code: string;
  family: string;
  language: string;
  header_count: number;
}

export interface HeaderSummary {
  id: number;
  internal_name: string;
  /** In-game location name; empty when the header has none. */
  location_name: string;
  matrix_id: number;
  area_data_id: number;
  event_file_id: number;
}

export interface MatrixDto {
  index: number;
  name: string;
  width: number;
  height: number;
  /** Row-major map ids; null marks an empty cell. */
  maps: (number | null)[];
}

/** Positions in tiles from the map centre (y is vertical); scale 1.0 = unscaled. */
export interface BuildingDto {
  model_id: number;
  x: number;
  y: number;
  z: number;
  sx: number;
  sy: number;
  sz: number;
}

export interface MapDto {
  index: number;
  /** [kind, collision] per cell, row-major (32 x 32). */
  permissions: [number, number][];
  buildings: BuildingDto[];
}

/**
 * Events of one event file. Coordinates are absolute tiles over the whole matrix
 * (matrix cell = x / 32, tile inside the map = x % 32).
 */
export interface BgDto {
  index: number;
  script: number;
  event_type: number;
  x: number;
  z: number;
  y: number;
}

export interface ObjectDto {
  index: number;
  local_id: number;
  graphics_id: number;
  movement_type: number;
  script: number;
  x: number;
  z: number;
  y: number;
}

export interface WarpDto {
  index: number;
  x: number;
  z: number;
  dest_header_id: number;
  dest_warp_id: number;
  height: number;
}

export interface CoordDto {
  index: number;
  script: number;
  x: number;
  z: number;
  width: number;
  length: number;
}

export interface EventsDto {
  event_file_id: number;
  bg: BgDto[];
  objects: ObjectDto[];
  warps: WarpDto[];
  coords: CoordDto[];
}

/** A named tile behaviour (decomp enum) for a permission kind byte; `class` drives the overlay colour. */
export interface TileBehavior {
  value: number;
  name: string;
  class: string;
}

/** Display text of any error thrown by a command (structured, string or Error). */
export function errorText(e: unknown): string {
  return isCmdError(e) ? e.message : String(e);
}

export const openProject = (path: string) => invoke<ProjectInfo>("open_project", { path });
export const listHeaders = () => invoke<HeaderSummary[]>("list_headers");
export const getMatrix = (index: number) => invoke<MatrixDto>("get_matrix", { index });
export const tileBehaviors = () => invoke<TileBehavior[]>("tile_behaviors");
export const getEvents = (eventFileId: number) => invoke<EventsDto>("get_events", { eventFileId });
export const getMap = (index: number) => invoke<MapDto>("get_map", { index });
/**
 * Raw GLB bytes of the map's terrain model, textured with the tileset of `areaDataId`
 * (converted by apicula, cached by content hash in the backend).
 */
export const getMapModelGlb = (index: number, areaDataId: number) =>
  invoke<ArrayBuffer>("get_map_model_glb", { index, areaDataId });

export type AssetKind =
  | "Map"
  | "Matrix"
  | "AreaData"
  | "Event"
  | "TextArchive"
  | "MapTexture"
  | "BuildingTexture"
  | "BuildingModel"
  | "InteriorBuildingModel";

export interface AssetId {
  kind: AssetKind;
  index: number;
}

/** Structured error of a backend command. `asset` is set when `code` is "stale". */
export interface CmdError {
  code: "stale" | "tool_missing" | "error";
  message: string;
  asset: AssetId | null;
}

export function isCmdError(e: unknown): e is CmdError {
  if (typeof e !== "object" || e === null) return false;
  const o = e as Record<string, unknown>;
  return (
    (o.code === "stale" || o.code === "tool_missing" || o.code === "error") &&
    typeof o.message === "string" &&
    (o.asset === null || typeof o.asset === "object")
  );
}

export interface SaveReport {
  written: AssetId[];
}

export interface BuildReport {
  /** Dirty assets that were saved before building. */
  saved: number;
  /** Asset kinds that were packed (maps, matrices, areaData) with their member counts. */
  packed: [string, number][];
  rom: string;
}

export const setPermission = (index: number, x: number, y: number, kind: number, collision: number) =>
  invoke<void>("set_permission", { index, x, y, kind, collision });
/** Resolves to false when there was nothing to undo. */
export const undo = () => invoke<boolean>("undo");
/** Resolves to false when there was nothing to redo. */
export const redo = () => invoke<boolean>("redo");
export const dirty = () => invoke<AssetId[]>("dirty");
export const save = () => invoke<SaveReport>("save");
/** Saves dirty assets, then builds the ROM at `out`. */
export const buildRom = (out: string) => invoke<BuildReport>("build_rom", { out });
/** Starts the emulator on the ROM and resolves to its PID without waiting. */
export const runRom = (emulator: string, rom: string) => invoke<number>("run_rom", { emulator, rom });
/** Drops the own changes of an asset and loads it from disk again. */
export const reloadAsset = (asset: AssetId) => invoke<void>("reload_asset", { kind: asset.kind, index: asset.index });
/** Keeps the own version of an asset and takes the file on disk as the new baseline. */
export const acceptDisk = (asset: AssetId) => invoke<void>("accept_disk", { kind: asset.kind, index: asset.index });
