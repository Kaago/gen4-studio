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
