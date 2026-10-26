import { Channel } from "@tauri-apps/api/core";
import { inTauri, invoke } from "./platform/transport";
import type { CheckFieldsDto } from "./api.checks";

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
  /** Index of that name in the location name bank (0: no place); two places can share a name, the index tells them apart. */
  location_index: number;
  /** HeartGold/SoulSilver: the header lies in Kanto (`kanto_flag`); always false in Diamond/Pearl and Platinum. */
  kanto: boolean;
  /** The kind of place the game treats the header as (Platinum `location_specifier`, HGSS `location_type`): 6 is the Underground. */
  map_type?: number;
  matrix_id: number;
  area_data_id: number;
  event_file_id: number;
  /** Text archive with the header's messages (the text panel lists which headers use an archive). */
  text_archive_id: number;
  /** Script file (member of the script archive) with the header's scripts. */
  script_file_id: number;
  /** Level script file of the header (0: none). */
  level_script_id: number;
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
  /** Facing: 0 up, 1 down, 2 left, 3 right. */
  dir: number;
  /** 1: a trainer who sees the player within `sight` tiles; 0: none. */
  trainer_type: number;
  /** The sight range in tiles (object `data[0]`). */
  sight: number;
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
/** Appends a copy of a map (without its buildings when `clearBuildings`) and resolves to its index. Written at once; not undoable. */
export const duplicateMap = (source: number, clearBuildings = false) => invoke<number>("duplicate_map", { source, clearBuildings });
/** The behaviour names of the open project's game family; rejects while no project is open. */
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
  dir?: number;
  local_id?: number;
  dest_header_id?: number;
  dest_warp_id?: number;
  height?: number;
  width?: number;
  length?: number;
  /** Objects only: 1 = the trainer sees the player, 0 = only talking starts the battle. */
  trainer_type?: number;
  /** Objects only: the sight range in tiles (object `data[0]`). */
  sight?: number;
}
/** A decoded overworld sprite: RGBA8, row-major, row 0 is the top. */
export interface SpriteDto {
  width: number;
  height: number;
  rgba: number[];
}
/**
 * The overworld sprite of an object with `graphicsId` facing `facing` (0 up, 1 down, 2 left, 3 right), or null
 * when it has none (no table entry, a 3D overworld, an unsupported format). Rejects when the sprite table or
 * archive cannot be read.
 */
export const getObjectSprite = shareInflight((graphicsId: number, facing: number) => invoke<SpriteDto | null>("get_object_sprite", { graphicsId, facing }));
/** Changes fields of event `index` of class `kind` in an event file (one undoable change). */
export const updateEvent = (eventFile: number, kind: EventKind, index: number, patch: EventPatch) =>
  invoke<void>("update_event", { eventFile, kind, index, patch });
/**
 * Appends an event of class `kind` on absolute matrix tile (x, z) and resolves to its index within the class (one undoable change). `patch`
 * is applied to the new event in the same change, named `label` (placing a trainer: script, sightline and sight in one undo step).
 */
export const addEvent = (eventFile: number, kind: EventKind, x: number, z: number, patch?: EventPatch, label?: string) =>
  invoke<number>("add_event", { eventFile, kind, x, z, ...(patch === undefined ? {} : { patch }), ...(label === undefined ? {} : { label }) });
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
  | "HeaderTable"
  | "DynamicHeaders"
  | "Script"
  | "Encounter"
  | "Personal"
  | "Learnset"
  | "Evolution"
  | "Move"
  | "Item"
  | "TrainerProperties"
  | "TrainerParty"
  | "TrainerTextTable"
  | "TrainerTextOffsets"
  | "ItemIcons"
  | "MonIcons"
  /** A trainer: its properties and party members as one asset (the store edits them together). */
  | "Trainer";

/** Member count per asset kind; kinds the game has no archive for are missing (the header table counts 1). */
export type MemberCounts = Partial<Record<AssetKind, number>>;

export interface AssetId {
  kind: AssetKind;
  index: number;
}

/**
 * Structured error of a backend command. `asset` is set when `code` is "stale" or "read_only",
 * and for an "error" of an undo or redo that could not restore that asset.
 * "partial": a wizard command failed after it had created members; they stay, the message names them.
 * "compile": scripts or texts did not compile; `problems` lists them (the build packed nothing).
 * "finding_stale": `planFix` / `applyFix` on a finding that is no longer reported as it was sent (its subject changed); nothing
 * was changed.
 */
export interface CmdError {
  code: "stale" | "read_only" | "tool_missing" | "partial" | "error" | "compile" | "finding_stale";
  message: string;
  asset: AssetId | null;
  /** The problems behind a "compile" error; the backend sends `[]` for every other code, but callers must handle absence. */
  problems?: ProblemDto[];
}

export function isCmdError(e: unknown): e is CmdError {
  if (typeof e !== "object" || e === null) return false;
  const o = e as Record<string, unknown>;
  return (
    (o.code === "stale" ||
      o.code === "read_only" ||
      o.code === "tool_missing" ||
      o.code === "partial" ||
      o.code === "error" ||
      o.code === "compile" ||
      o.code === "finding_stale") &&
    typeof o.message === "string" &&
    (o.asset === null || typeof o.asset === "object") &&
    (o.problems === undefined || Array.isArray(o.problems))
  );
}

/**
 * A problem of the last save or build, of one text (see `validateText`), of a data document, or a finding of the project checks. Lines and
 * columns are 1-based. A finding (`source` "check", see `CheckProblemDto`) also has the fields of `CheckFieldsDto` (the rows of the other
 * sources have none of them), and only a finding can be an `info`.
 */
export interface ProblemDto extends Partial<CheckFieldsDto> {
  severity: "error" | "warning" | "info";
  /** Relative to the project root with forward slashes; empty for a text that belongs to no file yet, and for a finding outside a source. */
  file: string;
  line: number;
  col: number;
  /** A finding: the kernel's English sentence; the UI words its own from `code` and `args` (`checks/messages.ts`). */
  message: string;
  /**
   * "encounter" and "trainer": the rows of a data document (file "encounters/0012", "trainers/0012"), see `ProblemsStore.replaceData`;
   * "check": a finding of the project checks, see `ProblemsStore.replaceChecks`.
   */
  source: "compile" | "text" | "encounter" | "trainer" | "check";
}

export interface SaveReport {
  written: AssetId[];
  /** The text warnings of the save, then the problems of the compile that follows a header table save in an adopted project. */
  problems: ProblemDto[];
  /** Sources that compile compiled without an error (project-relative): their earlier compile problems are gone. */
  compiled: string[];
  /** Text archives that compile appended to (a text literal resolved now); the backend holds them again. */
  touched_archives: number[];
}

export interface BuildReport {
  /** Dirty assets that were saved before building. */
  saved: number;
  /** Asset kinds that were packed (every kind the project has an archive for, by unpacked folder name) with their member counts. */
  packed: [string, number][];
  rom: string;
  /** Script files compiled before packing. */
  compiled: number[];
  /** Text archives encoded before packing. */
  encoded: number[];
  /** Warnings of the pre-build save, the compile and the text encoding; `source` tells them apart. */
  problems: ProblemDto[];
}

export const setPermission = (index: number, x: number, y: number, kind: number, collision: number) =>
  invoke<void>("set_permission", { index, x, y, kind, collision });
/** Resolves to every asset the step restored (one step can change several), or [] when there was nothing to undo. */
export const undo = () => invoke<AssetId[]>("undo");
/** Resolves to every asset the step restored, or [] when there was nothing to redo. */
export const redo = () => invoke<AssetId[]>("redo");
/** The undo labels (oldest first) and redo labels (the one `redo` applies next first); a label may be empty. */
export interface HistoryDto {
  undo: string[];
  redo: string[];
}
export const history = () => invoke<HistoryDto>("history");
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

/** One matrix as the overview lists it (no cells). `filled` counts the cells that hold a map. */
export interface MatrixSummary {
  index: number;
  name: string;
  width: number;
  height: number;
  has_headers: boolean;
  has_altitudes: boolean;
  filled: number;
}

/** All matrices; `unreadable` lists [index, reason] of those that could not be read. */
export interface MatrixListing {
  matrices: MatrixSummary[];
  unreadable: [number, string][];
}

/** A warp that leads to a header: where it sits and where it goes. */
export interface WarpLink {
  event_file: number;
  index: number;
  x: number;
  z: number;
  dest_header_id: number;
  dest_warp_id: number;
}

/** All warps of all event files; `unreadable` lists [event file, reason] of those that could not be read. */
export interface WarpListing {
  links: WarpLink[];
  unreadable: [number, string][];
}

/** What a new header gets for its event file: keep the one it has, a new empty one, or a copy of this file. */
export type EventFileChoice = "Keep" | "New" | { Copy: number };

/** One filled cell of a new area: a copy of `source_map` at (x, y). */
export interface AreaCell {
  x: number;
  y: number;
  source_map: number;
  clear_buildings: boolean;
}

/**
 * A header the wizard creates first: a copy of `template` (as the editor shows it) under this internal name; `null`
 * (or blank) lets the backend derive a unique one from the area's or room's name.
 */
export interface NewHeader {
  template: number;
  internal_name: string | null;
}

/** The place (in-game location name) a wizard gives its header: an existing index, or a new name it appends. */
export type PlaceChoice = { Existing: number } | { New: string };

/**
 * A header that is pointed at the new matrix and gets area data and an event file. With `new_header` it does not exist
 * yet: `id` is then the current header count (the number it gets), and "Keep" shares the template's event file.
 */
export interface AssignHeader {
  id: number;
  area_data_id: number | null;
  event_file: EventFileChoice;
  new_header?: NewHeader | null;
  /** A new header may apply the Dynamic Headers patch first when the game lacks it (not undoable). */
  allow_patch?: boolean;
  /** The place the header gets, in the same undo step as the assignment; absent keeps its place. */
  place?: PlaceChoice | null;
}

export interface AreaSpec {
  name: string;
  width: number;
  height: number;
  cells: AreaCell[];
  with_headers: boolean;
  header: AssignHeader | null;
  /**
   * The cell of an existing world map (a matrix with a header section) where the area's upper left cell goes: no new
   * matrix, the cells get the copies and the header. Absent or `null`: a matrix of its own.
   */
  target?: AreaTarget | null;
}

/** Where on a world map an area goes: the cell (x, y) of matrix `matrix` for its upper left cell. */
export interface AreaTarget {
  matrix: number;
  x: number;
  y: number;
}

/**
 * What `create_area` made: the matrix (the target's with one), its maps as [x, y, map id] (x, y within the area), the
 * event file and the header it assigned.
 */
export interface AreaReport {
  matrix: number;
  maps: [number, number, number][];
  event_file: number | null;
  header: number | null;
  /** The call applied the Dynamic Headers patch. */
  patched: boolean;
  /** The copy of an older `unpacked/dynamicHeaders` the patch made under `.g4/backup/`. */
  snapshot: string | null;
  /** The place the header got. */
  place: number | null;
  /** The landscape cells (a map, header 0) of the world map the area replaced; 0 without a target. */
  replaced: number;
}

export interface ExtendSpec {
  matrix: number;
  x: number;
  y: number;
  source_map: number;
  clear_buildings: boolean;
  header_cell: number | null;
}

export interface ExtendReport {
  map: number;
  width: number;
  height: number;
}

/** The door pair of a new room: the outside warp at (x, z) of `outside_header`, the inside one at (inside_x, inside_z). */
export interface Entrance {
  outside_header: number;
  x: number;
  z: number;
  inside_x: number;
  inside_z: number;
  /** Turns each of the two tiles a warp would not fire on into a door (behaviour 0x69, collision 0x80). */
  door_tiles?: boolean;
}

export interface RoomSpec {
  name: string;
  source_map: number;
  clear_buildings: boolean;
  header: AssignHeader;
  entrance: Entrance | null;
}

export interface RoomReport {
  matrix: number;
  map: number;
  event_file: number | null;
  outside_warp: number | null;
  inside_warp: number | null;
  /** The room warp is the copied warp that already stood on its tile, now leading outside. */
  inside_warp_reused: boolean;
  /** The tiles made doors (local tile x, y of the map), the outside one first. */
  door_tiles: { map: number; x: number; y: number }[];
  /** The call applied the Dynamic Headers patch. */
  patched: boolean;
  /** The copy of an older `unpacked/dynamicHeaders` the patch made under `.g4/backup/`. */
  snapshot: string | null;
  /** The place the header got. */
  place: number | null;
}

export const listMatrices = () => invoke<MatrixListing>("list_matrices");
export const listWarps = () => invoke<WarpListing>("list_warps");
/** Appends an empty event file and resolves to its index. Written at once; not undoable. */
export const createEventFile = () => invoke<number>("create_event_file");
/** Appends a copy of an event file and resolves to its index. Written at once; not undoable. */
export const duplicateEventFile = (source: number) => invoke<number>("duplicate_event_file", { source });
/** Each wizard is one call; written at once and not undoable. */
export const createArea = (spec: AreaSpec) => invoke<AreaReport>("create_area", { spec });
export const extendArea = (spec: ExtendSpec) => invoke<ExtendReport>("extend_area", { spec });
export const createRoom = (spec: RoomSpec) => invoke<RoomReport>("create_room", { spec });

/** What `splitRoom` did: the room's new matrix and the map copies in it (each copied map once). */
export interface SplitReport {
  matrix: number;
  maps: number[];
}

/**
 * Gives room `header` copies of the maps it shares with other rooms, on a new matrix of its own.
 * The copies and the matrix are written at once and stay; moving the room onto them is one undo
 * step, labelled with `name` (else the header id).
 */
export const splitRoom = (header: number, name?: string | null) =>
  invoke<SplitReport>("split_room", { header, name: name ?? null });

/** The Dynamic Headers patch of the open game: whether it exists for it, and whether it is in the game already. */
export interface HeaderPatchDto {
  supported: boolean;
  applied: boolean;
  /** The patch is in, but its routine calls a wrong function (the game hangs after the intro): `repairHeaderPatch` fixes it. */
  repairable: boolean;
  /** The NARC the headers live in once patched (null: no patch for this game). */
  narc: string | null;
}

export const headerPatchStatus = () => invoke<HeaderPatchDto>("header_patch_status");
/** What applying the patch did: its new state and where the old header folder went. */
export interface PatchReport extends HeaderPatchDto {
  /** The copy of an older `unpacked/dynamicHeaders` under `.g4/backup/` (null: there was none, or it was empty). */
  snapshot: string | null;
}

/** Changes the game's code for good; unsaved header edits stay unsaved (the patch takes the table from disk). */
export const applyHeaderPatch = () => invoke<PatchReport>("apply_header_patch");
/** Rewrites the 4 bytes of a broken patch's wrong call in arm9.bin; the header table is not touched. */
export const repairHeaderPatch = () => invoke<HeaderPatchDto>("repair_header_patch");

/** One message of a text archive: its id in the JSON source and its text (control codes stay as text). */
export interface TextMessageDto {
  id: string;
  text: string;
}

export interface TextArchiveDto {
  index: number;
  key: number;
  lang: string;
  messages: TextMessageDto[];
  /** Ids of the map headers that use this archive. */
  users: number[];
}

/** One edit of an archive's message list (the backend's serde enum). */
export type TextEdit = { Set: { index: number; text: string } } | "Append" | "RemoveLast";

/** One search hit: message `index` of archive `archive`, with an excerpt around the match. */
export interface TextHit {
  archive: number;
  index: number;
  excerpt: string;
}

/** Archive `index` including unsaved edits; rejects for an archive that cannot be decoded. */
export const getTextArchive = (index: number) => invoke<TextArchiveDto>("get_text_archive", { index });
/**
 * Applies one edit to an archive (one undoable change). Resolves to null when it left the archive as it was; rejects
 * (code "error") for a message that does not exist or when the last message would be removed.
 */
export const editTextArchive = (index: number, edit: TextEdit) => invoke<number | null>("edit_text_archive", { index, edit });
/** Appends an archive with one message and resolves to its index. Written at once; not undoable. */
export const createTextArchive = () => invoke<number>("create_text_archive");
/** Case-insensitive search over the messages of all archives; at most `limit` hits. */
export const searchText = (query: string, limit: number) => invoke<TextHit[]>("search_text", { query, limit });
/** Warnings for one message text (line 1 is its first line), for example "wider than the text box". */
export const validateText = (text: string) => invoke<ProblemDto[]>("validate_text", { text });

/**
 * Starts rotom-lsp for the open project; every message of the server (one JSON-RPC body, no framing) goes to
 * `onMessage`. Rejects with code "tool_missing" when the binary is not there. A new `Channel` per call: the
 * backend replaces a running server, and the old channel stops receiving.
 */
export async function lspStart(onMessage: (body: string) => void): Promise<void> {
  // The dev bridge has no channels: no language services in the browser, as when rotom-lsp is missing.
  if (!inTauri()) {
    return Promise.reject({ code: "tool_missing", message: "rotom-lsp fehlt — Completion und Diagnosen aus", asset: null });
  }
  const channel = new Channel<string>();
  channel.onmessage = onMessage;
  await invoke("lsp_start", { channel });
}

/**
 * Sends one JSON-RPC body to the server (the backend frames it). Rejects when the queue is full or the server is
 * dead and not restarted. A server that was restarted since the last send has lost the client's state: the
 * client has to initialize and open its documents again, and nothing tells it so.
 */
export const lspSend = (body: string) => invoke<void>("lsp_send", { body });

export const lspStop = () => invoke<void>("lsp_stop");

/** Whether rotom has taken the project over (`rotom.toml` and the sources exist) and how many script files it has. */
export interface ScriptsStatusDto {
  adopted: boolean;
  file_count: number;
}

export interface AdoptProgressDto {
  done: number;
  total: number;
}

export interface AdoptReportDto {
  decompiled: number;
  skipped_empty: number;
  /** Script files rotom could not decompile (file number, message); they stay binary. */
  failed: [number, string][];
}

/** A level script as rotom's JSON document; the panel shows it as text and hands the parsed text back. */
export type LevelScriptDoc = Record<string, unknown>;

/** A script file as the editor opens it (`kind` is the tag). A binary file cannot be edited. */
export type ScriptSourceDto = { kind: "script"; text: string } | { kind: "levelScript"; doc: LevelScriptDoc } | { kind: "binary"; size: number };

export interface SaveScriptDto {
  /** Every problem the compile reported, also those of other sources that were stale. */
  problems: ProblemDto[];
  /** Text archives a text literal in a script appended to; the store holds them again. */
  touched_archives: number[];
}

export type ScriptTargetDto = { kind: "file"; file: number; line: number } | { kind: "ambiguous"; candidates: number[] } | { kind: "unknown" };

export const scriptsStatus = () => invoke<ScriptsStatusDto>("scripts_status");

/**
 * Decompiles every script file into `expanded/scripts/`; `onProgress` is called as files are done. Takes seconds.
 * Through the dev bridge (no channel) there are no progress events: the call runs without them and `onProgress` gets
 * one final progress from the report.
 */
export async function adoptScripts(onProgress: (progress: AdoptProgressDto) => void): Promise<AdoptReportDto> {
  if (!inTauri()) {
    const report = await invoke<AdoptReportDto>("adopt_scripts");
    const total = report.decompiled + report.skipped_empty + report.failed.length;
    onProgress({ done: total, total });
    return report;
  }
  const progress = new Channel<AdoptProgressDto>();
  progress.onmessage = onProgress;
  return invoke<AdoptReportDto>("adopt_scripts", { progress });
}

export const readScript = (file: number) => invoke<ScriptSourceDto>("read_script", { file });
/**
 * Writes the source and compiles what is stale. A source that does not compile is saved anyway and comes back as
 * problems. Rejects (code "error") with "Textarchiv NNNN erst speichern" while a text archive has unsaved edits.
 */
export const saveScript = (file: number, text: string) => invoke<SaveScriptDto>("save_script", { file, text });
export const saveLevelScript = (file: number, doc: LevelScriptDoc) => invoke<SaveScriptDto>("save_level_script", { file, doc });
export const newScriptFile = (kind: "script" | "levelScript") => invoke<number>("new_script_file", { kind });
/** Appends an empty script to the script file and resolves to its number. Same refusal as `saveScript`. */
export const addScript = (file: number) => invoke<number>("add_script", { file });
export const resolveScript = (header: number, n: number) => invoke<ScriptTargetDto>("resolve_script", { header, n });

/** One script file in the project (`kind` "binary": rotom could not decompile it). */
export interface ScriptFileDto {
  file: number;
  kind: "script" | "levelScript" | "binary";
  size: number;
  /** Ids of the headers that use the file as their script file (level script for a level script; a binary file counts for both). */
  headers: number[];
}

/** A line of a script source that contains the search text (line is 1-based). */
export interface ScriptHitDto {
  file: number;
  line: number;
  excerpt: string;
}

export const listScriptFiles = () => invoke<ScriptFileDto[]>("list_script_files");
/** Case-insensitive full text search over the script sources (a file number finds that file); at most `limit` hits. */
export const searchScripts = (query: string, limit: number) => invoke<ScriptHitDto[]>("search_scripts", { query, limit });

/**
 * What the world tree knows that the ROM does not, kept in the project file `g4-world.json`: names that replace the
 * proposed names of world maps and rooms, matrices that are no world map, and rooms assigned to an area by hand
 * (`roomOwner`: room header id to area key, such as "w0:3" or "m240"; the id of one of the area's headers, as text, is
 * also accepted). Keys of the two records are header ids as text.
 */
export interface WorldMeta {
  version: 1;
  /** `filter` is "johto" or "kanto" for the two world maps one HeartGold/SoulSilver matrix gives. */
  worldMaps: { matrix: number; filter?: "johto" | "kanto"; name: string }[];
  notWorldMaps: number[];
  roomNames: Record<string, string>;
  roomOwner: Record<string, string>;
}

/** The world file as read: `broken` says why it could not be (then `meta` is empty and the file is left alone). */
export const worldMetaGet = () => invoke<{ meta: WorldMeta; broken: string | null }>("world_meta_get");
/** Writes the world file at once. Rejects while the file is broken ("g4-world.json ist beschädigt: … Repariere oder lösche die Datei."). */
export const worldMetaSet = (meta: WorldMeta) => invoke<void>("world_meta_set", { meta });

/** Room for new places in the location bank (`bank`, a text archive): `free` names can still be appended (256 at most);
 * `reusable` (HeartGold/SoulSilver only) are the Sinnoh names 1–125 no map uses, lowest first. */
export interface PlaceSlots {
  bank: number;
  count: number;
  free: number;
  reusable: number[];
}
/** What `createPlace` did: the new place's index and, when it took a reusable slot, the name it replaced. */
export interface PlaceCreated {
  index: number;
  reused: string | null;
}

export const placeSlots = () => invoke<PlaceSlots>("place_slots");
/** Creates the place `name` and gives it to the map headers `headers`, one undo step. Rejects an empty name, a
 * character the game does not know, a full bank and Diamond/Pearl (German sentences). */
export const createPlace = (name: string, headers: number[]) => invoke<PlaceCreated>("create_place", { name, headers });
/** Renames the place at `index` for every map header carrying it, one undo step. */
export const renamePlace = (index: number, name: string) => invoke<void>("rename_place", { index, name });
/** Gives the existing place at `index` to the map headers `headers`, one undo step. */
export const assignPlace = (headers: number[], index: number) => invoke<void>("assign_place", { headers, index });

export * from "./api.checks";
export * from "./api.data";
export * from "./api.import";
export * from "./project/api.start";
export * from "./project/api.templates";
