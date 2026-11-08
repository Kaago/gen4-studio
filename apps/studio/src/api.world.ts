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

/** What a world edit did (`world_create`, `world_edit`, `world_layers`, `world_to_binary`): the map, its package, the cells, the undo step and the bake (null without world). */
export interface WorldReport {
  map: number;
  tileset: number;
  cells: number;
  change: number | null;
  bake: WorldBakeReport | null;
  /** Notes of a smart stroke (6.4b): the seam to a neighbour map, a cell's own permission kept; left out when there are none. */
  notes?: string[];
}

/** „Karte als Quelle übernehmen“ (Form 1, verbatim): the binary map `map` becomes a world map that bakes byte-equal; one undo step. */
export const worldAdopt = (map: number) => invoke<WorldAdoptReport>("world_adopt", { req: { map } });
/** „Als binäre Karte übernehmen“: the world of `map` goes, its bake stays as the map; one undo step. */
export const worldToBinary = (map: number) => invoke<WorldReport>("world_to_binary", { map });
/** The bake of world map `map` as the store holds it (null without world or outside a text project); rejects a world whose rows do not read. */
export const worldBake = (map: number) => invoke<WorldBakeReport | null>("world_bake", { map });

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
  /** The terrain brush (6.4b): edge set `set`, the cells' corners inside (`inside`, default true) or outside, `seed` the variants' (default 0). */
  terrain?: { set: string; inside?: boolean; seed?: number };
  /** The cliff brush (6.4b): edge set `set`, the cells' absolute height `height` (−16 to 62, ground only), `seed` the variants' (default 0). */
  cliff?: { set: string; height: number; seed?: number };
}

/** `world_edit`: the map, the history text (left out: „Welt geändert: Karte 0123 (37 Zellen)“) and the changes, one undo step. */
export interface WorldEditRequest {
  map: number;
  label?: string;
  changes: WorldChange[];
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
  tileset: { version: number; package: number; tiles: TileJson[]; edge_sets?: EdgeSetJson[] };
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

/** What `tileset_edit` did: the package, the tiles it changed and the undo step. */
export interface TilesetEditReport {
  package: number;
  tiles: number;
  change: number;
}

/** Changes tiles of the tileset of package `pkg` and (6.4b) its edge sets as one undo step (`changes` may be empty when `sets` is not). */
export const tilesetEdit = (pkg: number, changes: TileChange[], sets: EdgeSetOp[] = []) =>
  invoke<TilesetEditReport>("tileset_edit", { req: { package: pkg, changes, sets } });

/* ---- Smart drawing (6.4b): edge sets of a tileset, changed with `tileset_edit`. ---- */

/** A place of an edge set: the corner mask its tile shows (see `docs/formats/tileset.md`, „Kantensätze“). */
export type EdgePlace =
  | "center"
  | "n"
  | "e"
  | "s"
  | "w"
  | "outer_nw"
  | "outer_ne"
  | "outer_sw"
  | "outer_se"
  | "inner_nw"
  | "inner_ne"
  | "inner_sw"
  | "inner_se"
  | "diag_nwse"
  | "diag_nesw";

/** The 15 places in the order of the file. */
export const EDGE_PLACES: EdgePlace[] = [
  "center",
  "n",
  "e",
  "s",
  "w",
  "outer_nw",
  "outer_ne",
  "outer_sw",
  "outer_se",
  "inner_nw",
  "inner_ne",
  "inner_sw",
  "inner_se",
  "diag_nwse",
  "diag_nesw",
];

/**
 * One edge set of a tileset (`Tileset.edge_sets`): its name, the tile token of the plain outside, which cell a cliff with it blocks
 * (`upper`, the default, left out) and per place the tile tokens (variants) that fill it; empty places are left out.
 */
export interface EdgeSetJson {
  name: string;
  outside: string;
  block?: "upper" | "lower";
  places: Partial<Record<EdgePlace, string[]>>;
}

/** An edge set put (replacing the one of the same name in its place, else appended) or removed by name. */
export type EdgeSetOp = { put: EdgeSetJson } | { remove: { name: string } };
