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

/** A binary part of a text project file: hex bytes, a vanilla member by its SHA-1, or a side file next to the JSON. */
export type PartJson = { hex: string } | { base_sha1: string } | { file: string };

/**
 * The world document of a map (`data/maps/NNNN.json`, `world`, schema 2). `freeform[].nsbmd` is the verbatim model of Form 1 as a part,
 * `terrain.bdhc` the verbatim BDHC (left out once the ground's heights or ramps change).
 */
export interface WorldJson {
  version: number;
  tileset: number;
  layers: WorldLayerJson[];
  pieces?: { tile: number; at: [number, number]; rotate: number }[];
  freeform?: { glb?: string; nsbmd?: PartJson; at: [number, number, number] }[];
  terrain?: { bdhc: PartJson };
  permissions: { derive: boolean; override?: string[] };
}

/** One message of a bake: its code with the args the UI words it from, and the message. */
export interface WorldBakeMessage {
  code: string;
  level: "error" | "warning";
  args: string[];
  message: string;
}

/** The bake of a world map as the store holds it (6.3b); `ok` false: an error refused the model, the map kept its last one (`model_kept`). */
export interface WorldBakeReport {
  map: number;
  ok: boolean;
  style: "outdoor" | "indoor" | null;
  faces: number;
  model_bytes: number;
  model_share: number;
  model_kept: boolean;
  textures: string[];
  vram_bytes: number;
  bdhc_bytes: number;
  bdhc_payload: number;
  plates: number;
  blocked: number;
  verbatim_model: boolean;
  verbatim_terrain: boolean;
  messages: WorldBakeMessage[];
}

/**
 * What `world_get` answers: the world of the map (null: none), the faces and textures of its mesh, the estimated model size and its share of
 * the budget, and the bake of the map (null without world).
 */
export interface WorldGetDto {
  map: number;
  world: WorldJson | null;
  faces: number;
  textures: number;
  estimate: number;
  share: number;
  bake: WorldBakeReport | null;
}

/** The mesh of map `map` (no faces without a world). */
export const getWorldMesh = (map: number) => invoke<WorldMeshDto>("get_world_mesh", { map });
/** Texture `texture` of map texture package `pkg` in RGBA with its tile's palette; null for the synthetic tileset and a texture that does not decode. */
export const getTilesetTexture = (pkg: number, texture: string) => invoke<SpriteDto | null>("get_tileset_texture", { package: pkg, texture });
/** The world of map `map` with its mesh figures. */
export const worldGet = (map: number) => invoke<WorldGetDto>("world_get", { map });

/** A cell whose sampled height is no whole step: its centre's height in world units (null: no plate under it) and the step the world took. */
export interface HeightDeviation {
  x: number;
  z: number;
  height: number | null;
  step: number;
}

/** What „Karte als Quelle übernehmen“ did: the package of the world (guessed: no header shows the map), the deviating cells, the undo step and the bake. */
export interface WorldAdoptReport {
  map: number;
  tileset: number;
  tileset_guessed: boolean;
  deviations: HeightDeviation[];
  change: number | null;
  bake: WorldBakeReport;
}

/** What a world edit did (`world_edit`, `world_to_binary`): the map, its package, the cells, the undo step and the bake (null without world). */
export interface WorldReport {
  map: number;
  tileset: number;
  cells: number;
  change: number | null;
  bake: WorldBakeReport | null;
}

/** „Karte als Quelle übernehmen“ (Form 1, verbatim): the binary map `map` becomes a world map that bakes byte-equal; one undo step. */
export const worldAdopt = (map: number) => invoke<WorldAdoptReport>("world_adopt", { req: { map } });
/** „Als binäre Karte übernehmen“: the world of `map` goes, its bake stays as the map; one undo step. */
export const worldToBinary = (map: number) => invoke<WorldReport>("world_to_binary", { map });
/** The bake of world map `map` as the store holds it (null without world or outside a text project); rejects a world whose rows do not read. */
export const worldBake = (map: number) => invoke<WorldBakeReport | null>("world_bake", { map });
