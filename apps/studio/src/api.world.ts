import type { SpriteDto } from "./api";
import { invoke } from "./platform/transport";

/**
 * World maps (6.3a), mirroring `apps/studio/src-tauri/src/commands/world_map.rs` and `g4_kernel::world`. `api.ts` re-exports
 * everything. Field names are the Rust ones.
 */

/** The role of a tile in its tileset; the scene picks the placeholder colour of a missing texture by it. */
export type TileRole = "ground" | "wall" | "ramp" | "water" | "grass" | "piece";

/** One texture of a mesh: its name in the package, its size in texels, the role of its tile and the tile's palette. */
export interface MeshTextureDto {
  name: string;
  width: number;
  height: number;
  role: TileRole;
  palette?: string;
}

/**
 * The mesh of a world map as flat arrays: per face 12 corner coordinates (four corners `x, y, z` in world units around the map
 * centre, counter-clockwise seen from the front), 8 UV values (texels), the index of its texture in `textures` and its layer
 * (0 the ground, overlays from 1, 255 the footprints of the pieces). A map without a world has no faces.
 */
export interface WorldMeshDto {
  map: number;
  tileset: number;
  faces: number;
  textures: MeshTextureDto[];
  corners: number[];
  uv: number[];
  texture: number[];
  layer: number[];
}

/** One layer of a world: `tiles` 32 rows of 32 tile tokens; only the ground layer has `heights` (and `ramps` with a ramp). */
export interface WorldLayerJson {
  name: string;
  role: "ground" | "overlay";
  tiles: string[];
  heights?: string[];
  ramps?: string[];
}

/** The world document of a map (`data/maps/NNNN.json`, `world`). */
export interface WorldJson {
  version: number;
  tileset: number;
  layers: WorldLayerJson[];
  pieces?: { tile: number; at: [number, number]; rotate: number }[];
  freeform?: { glb?: string; nsbmd?: string; at: [number, number, number] }[];
  permissions: { derive: boolean; override?: string[] };
}

/** What `world_get` answers: the world of the map (null: none), the faces and textures of its mesh, the estimated model size and its share of the budget. */
export interface WorldGetDto {
  map: number;
  world: WorldJson | null;
  faces: number;
  textures: number;
  estimate: number;
  share: number;
}

/** The mesh of map `map` (no faces without a world). */
export const getWorldMesh = (map: number) => invoke<WorldMeshDto>("get_world_mesh", { map });
/** Texture `texture` of map texture package `pkg` in RGBA with its tile's palette; null for the synthetic tileset and a texture that does not decode. */
export const getTilesetTexture = (pkg: number, texture: string) => invoke<SpriteDto | null>("get_tileset_texture", { package: pkg, texture });
/** The world of map `map` with its mesh figures. */
export const worldGet = (map: number) => invoke<WorldGetDto>("world_get", { map });

/* ---- Painting (6.4a): `world_edit`, `world_layers`, the tileset commands. ---- */

/**
 * One change of `world_edit`: the cells (`[x, z]`) and exactly one of `tile` (on `layer`, the ground when left out), `height`,
 * `ramp` (the ground's) and `permission` (no `layer`), as tokens of the file.
 */
export interface WorldChange {
  layer?: number;
  cells: [number, number][];
  tile?: string;
  height?: number;
  ramp?: string;
  permission?: string;
}

/** `world_edit`: the map, the history text (left out: „Welt geändert: Karte 0123 (37 Zellen)“) and the changes, one undo step. */
export interface WorldEditRequest {
  map: number;
  label?: string;
  changes: WorldChange[];
}

/** What `world_edit` and `world_layers` did: the map, its package, the cells set and the undo step (null: nothing recorded). */
export interface WorldReport {
  map: number;
  tileset: number;
  cells: number;
  change: number | null;
}

/** Applies the changes of `req` as one undo step. */
export const worldEdit = (req: WorldEditRequest) => invoke<WorldReport>("world_edit", { req });

/** Adds an overlay named `name` or removes overlay `layer` (the ground stays). */
export type LayerOp = { add: { name: string } } | { remove: { layer: number } };

/** Adds or removes an overlay of map `map` as one undo step. */
export const worldLayers = (map: number, op: LayerOp) => invoke<WorldReport>("world_layers", { req: { map, op } });

/** One tile of a tileset (`data/tilesets/NNNN.json`), the kernel's `TileJson`; optional fields are left out when unset. */
export interface TileJson {
  texture: string;
  texture_index: number;
  palette?: string;
  size: [number, number];
  role: TileRole;
  kind: number;
  collision: number;
  height: "flat" | "ramp" | "wall";
  sound?: number;
  wall?: number;
  footprint?: [number, number];
  model?: string;
  origin: "vanilla" | "unclear" | "synthetic" | "user";
}

/** What `tileset_get` answers: the tileset of package `package`, `stored` when it is a file (else derived). */
export interface TilesetDto {
  package: number;
  stored: boolean;
  tileset: { version: number; package: number; tiles: TileJson[] };
}

/** The tileset of map texture package `pkg` (the file, else derived). */
export const tilesetGet = (pkg: number) => invoke<TilesetDto>("tileset_get", { package: pkg });

/** What `tileset_save` wrote: the package, the file, its tiles by origin and the undo step. */
export interface TilesetSaveReport {
  package: number;
  file: string;
  tiles: number;
  vanilla: number;
  unclear: number;
  change: number;
}

/** Writes the derived tileset of package `pkg` as a file (one undo step). */
export const tilesetSave = (pkg: number) => invoke<TilesetSaveReport>("tileset_save", { package: pkg });

/** One change of `tileset_edit`: tile `tile` and the values to set (`wall` null clears it). */
export interface TileChange {
  tile: number;
  role?: TileRole;
  kind?: number;
  collision?: number;
  sound?: number;
  wall?: number | null;
}

/** Changes tiles of the tileset of package `pkg` as one undo step (the command arrives with 6.3b). */
export const tilesetEdit = (pkg: number, changes: TileChange[]) =>
  invoke<{ package: number; tiles: number; change: number }>("tileset_edit", { req: { package: pkg, changes } });
