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
  matrix_id: number;
  area_data_id: number;
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

export const openProject = (path: string) => invoke<ProjectInfo>("open_project", { path });
export const listHeaders = () => invoke<HeaderSummary[]>("list_headers");
export const getMatrix = (index: number) => invoke<MatrixDto>("get_matrix", { index });
export const getMap = (index: number) => invoke<MapDto>("get_map", { index });
/** Raw GLB bytes of the map's terrain model (converted by apicula, cached by the backend). */
export const getMapModelGlb = (index: number) => invoke<ArrayBuffer>("get_map_model_glb", { index });

export interface AssetId {
  kind: "Map" | "Matrix" | "AreaData";
  index: number;
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
