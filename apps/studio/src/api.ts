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

export interface HeaderListing {
  headers: HeaderSummary[];
  /** Set when the text archive could not be read: every location_name is then empty. */
  names_error: string | null;
}

/** Map header fields of Diamond/Pearl (uxie `MapHeaderDP`, snake_case as serialized). */
export interface MapHeaderDP {
  area_data_id: number;
  unknown1: number;
  matrix_id: number;
  script_file_id: number;
  level_script_id: number;
  text_archive_id: number;
  music_day_id: number;
  music_night_id: number;
  wild_pokemon: number;
  event_file_id: number;
  location_name: number;
  weather_id: number;
  camera_angle_id: number;
  location_specifier: number;
  battle_background: number;
  flags: number;
}

/** Map header fields of Platinum (uxie `MapHeaderPt`). */
export interface MapHeaderPt {
  area_data_id: number;
  unknown1: number;
  matrix_id: number;
  script_file_id: number;
  level_script_id: number;
  text_archive_id: number;
  music_day_id: number;
  music_night_id: number;
  wild_pokemon: number;
  event_file_id: number;
  location_name: number;
  area_icon: number;
  weather_id: number;
  camera_angle_id: number;
  location_specifier: number;
  battle_background: number;
  flags: number;
}

/** Map header fields of HeartGold/SoulSilver (uxie `MapHeaderHGSS`). */
export interface MapHeaderHGSS {
  wild_pokemon: number;
  area_data_id: number;
  unknown0: number;
  worldmap_x: number;
  worldmap_y: number;
  matrix_id: number;
  script_file_id: number;
  level_script_id: number;
  text_archive_id: number;
  music_day_id: number;
  music_night_id: number;
  event_file_id: number;
  location_name: number;
  area_icon: number;
  unknown1: number;
  kanto_flag: boolean;
  weather_id: number;
  location_type: number;
  camera_angle_id: number;
  follow_mode: number;
  battle_background: number;
  flags: number;
}

/** One map header exactly as the backend serializes it: externally tagged by game family. */
export type MapHeader = { DP: MapHeaderDP } | { Pt: MapHeaderPt } | { HGSS: MapHeaderHGSS };

export type HeaderFamily = "DP" | "Pt" | "HGSS";

/** The game family a header belongs to (the tag of its serialized form). */
export function headerFamily(h: MapHeader): HeaderFamily {
  if ("DP" in h) return "DP";
  if ("Pt" in h) return "Pt";
  return "HGSS";
}

/** The fields of a header, whatever its family. */
export function headerFields(h: MapHeader): MapHeaderDP | MapHeaderPt | MapHeaderHGSS {
  if ("DP" in h) return h.DP;
  if ("Pt" in h) return h.Pt;
  return h.HGSS;
}

export interface MatrixDto {
  index: number;
  name: string;
  width: number;
  height: number;
  /** Row-major map ids; null marks an empty cell. */
  maps: (number | null)[];
  /** Row-major header ids; null (or absent) when the matrix has no header section. */
  headers?: number[] | null;
  /** Row-major altitudes; null (or absent) when the matrix has no altitude section. */
  altitudes?: number[] | null;
}

/** Map id the backend stores for, and accepts as, an empty matrix cell. */
export const MATRIX_EMPTY = 0xffff;

/** Positions in tiles from the map centre (y is vertical); scale 1.0 = unscaled. */
export interface BuildingDto {
  model_id: number;
  x: number;
  y: number;
  z: number;
  sx: number;
  sy: number;
  sz: number;
  /** Raw rotation words, 8 upper-case hex digits each. The game ignores them for map buildings; shown, never edited. */
  rotation: [string, string, string];
}

/** The fields of a building that can be changed; missing ones stay as they are. */
export type BuildingPatch = Partial<Pick<BuildingDto, "model_id" | "x" | "y" | "z" | "sx" | "sy" | "sz">>;

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

/** The four classes of events in an event file. */
export type EventKind = "warp" | "object" | "coord" | "bg";

export interface EventsDto {
  event_file_id: number;
  /** Why the file cannot be edited (the English reason from the backend); null when it is editable. */
  locked: string | null;
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

/** Wraps a call so that a call with the same arguments made while one is running gets that one's result. */
function shareInflight<A extends (number | string)[], R>(call: (...args: A) => Promise<R>): (...args: A) => Promise<R> {
  const running = new Map<string, Promise<R>>();
  return (...args) => {
    const key = args.join(":");
    let pending = running.get(key);
    if (!pending) {
      pending = call(...args).finally(() => running.delete(key));
      running.set(key, pending);
    }
    return pending;
  };
}

export const openProject = (path: string) => invoke<ProjectInfo>("open_project", { path });
export const listHeaders = () => invoke<HeaderListing>("list_headers");
/** Map header `id` including unsaved edits. */
export const getHeader = (id: number) => invoke<MapHeader>("get_header", { id });
/**
 * Replaces header `id` as a whole (same family, values within their bit fields). `label`
 * names the change in the undo history (the edited field).
 */
export const setHeader = (id: number, header: MapHeader, label?: string) => invoke<void>("set_header", { id, header, label });
/**
 * One area data record (tileset references and light), tagged by the layout of the project's
 * game family; field names follow the codec. `Dppt` covers Diamond, Pearl and Platinum.
 */
export type AreaDataDto =
  | { family: "Dppt"; buildings_tileset: number; map_tileset: number; unknown: number; light_type: number }
  | {
      family: "Hgss";
      buildings_tileset: number;
      map_tileset: number;
      dynamic_texture_type: number;
      area_type: number;
      light_type: number;
    };
/** Area data `id` including unsaved edits. */
export const getAreaData = (id: number) => invoke<AreaDataDto>("get_area_data", { id });
/**
 * Replaces area data `id` as a whole; the record must use the project's family layout (else the
 * call is refused and nothing changes). `label` names the change in the undo history.
 */
export const setAreaData = (id: number, area: AreaDataDto, label?: string) => invoke<void>("set_area_data", { id, area, label });
export const memberCounts = () => invoke<MemberCounts>("member_counts");
export const getMatrix = (index: number) => invoke<MatrixDto>("get_matrix", { index });
/**
 * Changes the given fields of one matrix cell; null leaves a field as it is. `map` MATRIX_EMPTY
 * empties the cell; `header` and `altitude` need the matrix to have that section.
 */
export const setMatrixCell = (index: number, x: number, y: number, map: number | null, header: number | null, altitude: number | null) =>
  invoke<void>("set_matrix_cell", { index, x, y, map, header, altitude });
/** Gives the matrix exactly the asked-for sections (added ones are filled with 0). */
export const setMatrixSections = (index: number, headers: boolean, altitudes: boolean) =>
  invoke<void>("set_matrix_sections", { index, headers, altitudes });
export const resizeMatrix = (index: number, width: number, height: number) => invoke<void>("resize_matrix", { index, width, height });
export const renameMatrix = (index: number, name: string) => invoke<void>("rename_matrix", { index, name });
/** Appends a 1 x 1 matrix and resolves to its index. Written at once; not undoable. */
export const createMatrix = (name: string) => invoke<number>("create_matrix", { name });
/** Appends a copy of a map and resolves to its index. Written at once; not undoable. */
export const duplicateMap = (source: number) => invoke<number>("duplicate_map", { source });
export const tileBehaviors = () => invoke<TileBehavior[]>("tile_behaviors");
export const getEvents = (eventFileId: number) => invoke<EventsDto>("get_events", { eventFileId });
export const getMap = (index: number) => invoke<MapDto>("get_map", { index });
/**
 * Raw GLB bytes of the map's terrain model, textured with the tileset of `areaDataId`
 * (converted by apicula, cached by content hash in the backend).
 */
export const getMapModelGlb = (index: number, areaDataId: number) =>
  invoke<ArrayBuffer>("get_map_model_glb", { index, areaDataId });

/**
 * Raw GLB bytes of a building model, textured with the building tileset of `areaDataId` (HGSS
 * interior areas use the interior model archive). Rejects with "building model N does not exist"
 * for an id outside its archive. Requests for the same model and area that are in flight at the
 * same time share one call, so the scene and the inspector never convert a model twice.
 */
export const getBuildingGlb = shareInflight((modelId: number, areaDataId: number) =>
  invoke<ArrayBuffer>("get_building_glb", { modelId, areaDataId }),
);
/**
 * The fields of an event that can be changed; missing ones stay as they are and fields the kind
 * does not have are ignored. `x` and `z` must fit the kind's field (0..65535 for warps and
 * objects, -32768..32767 for coordinate triggers), else the call is refused.
 */
export interface EventPatch {
  x?: number;
  z?: number;
  script?: number;
  event_type?: number;
  graphics_id?: number;
  movement_type?: number;
  local_id?: number;
  dest_header_id?: number;
  dest_warp_id?: number;
  height?: number;
  width?: number;
  length?: number;
}
/** Changes fields of event `index` of class `kind` in an event file (one undoable change). */
export const updateEvent = (eventFile: number, kind: EventKind, index: number, patch: EventPatch) =>
  invoke<void>("update_event", { eventFile, kind, index, patch });
/** Appends an event of class `kind` on absolute matrix tile (x, z) and resolves to its index within the class (one undoable change). */
export const addEvent = (eventFile: number, kind: EventKind, x: number, z: number) => invoke<number>("add_event", { eventFile, kind, x, z });
/** Removes an event; the ones of its class after it move up by one (one undoable change). */
export const deleteEvent = (eventFile: number, kind: EventKind, index: number) => invoke<void>("delete_event", { eventFile, kind, index });
/** Changes fields of building `index` of map `map` (one undoable change). */
export const updateBuilding = (map: number, index: number, patch: BuildingPatch) => invoke<void>("update_building", { map, index, patch });
/** Appends a building (model 0, map centre, scale 1.0) to a map and resolves to its index (one undoable change). */
export const addBuilding = (map: number) => invoke<number>("add_building", { map });
/** Removes a building; the ones after it move up by one (one undoable change). */
export const deleteBuilding = (map: number, index: number) => invoke<void>("delete_building", { map, index });

/** Why a map's model is untextured because of its tileset (see `mapModelTextureStatus`). */
export interface TextureIssue {
  /** Tileset index; null when the area data itself could not be read. */
  tileset: number | null;
  /** First line of the backend's (English) error. */
  reason: string;
}

/**
 * Whether the model of a map is shown untextured because its tileset or area data is unusable.
 * Ask after `getMapModelGlb`: it reads the backend's failure marker and never runs apicula.
 */
export const mapModelTextureStatus = (index: number, areaDataId: number) =>
  invoke<TextureIssue | null>("map_model_texture_status", { index, areaDataId });

export type AssetKind =
  | "Map"
  | "Matrix"
  | "AreaData"
  | "Event"
  | "TextArchive"
  | "MapTexture"
  | "BuildingTexture"
  | "BuildingModel"
  | "InteriorBuildingModel"
  | "HeaderTable";

/** Member count per asset kind; kinds the game has no archive for are missing (the header table counts 1). */
export type MemberCounts = Partial<Record<AssetKind, number>>;

export interface AssetId {
  kind: AssetKind;
  index: number;
}

/** Structured error of a backend command. `asset` is set when `code` is "stale" or "read_only". */
export interface CmdError {
  code: "stale" | "read_only" | "tool_missing" | "error";
  message: string;
  asset: AssetId | null;
}

export function isCmdError(e: unknown): e is CmdError {
  if (typeof e !== "object" || e === null) return false;
  const o = e as Record<string, unknown>;
  return (
    (o.code === "stale" || o.code === "read_only" || o.code === "tool_missing" || o.code === "error") &&
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
  /** Asset kinds that were packed (every kind the project has an archive for, by unpacked folder name) with their member counts. */
  packed: [string, number][];
  rom: string;
}

export const setPermission = (index: number, x: number, y: number, kind: number, collision: number) =>
  invoke<void>("set_permission", { index, x, y, kind, collision });
/** Resolves to the asset that was restored, or null when there was nothing to undo. */
export const undo = () => invoke<AssetId | null>("undo");
/** Resolves to the asset that was restored, or null when there was nothing to redo. */
export const redo = () => invoke<AssetId | null>("redo");
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
