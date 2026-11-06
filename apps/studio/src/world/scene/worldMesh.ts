import * as THREE from "three";
import { getMapModelGlb, getTilesetTexture, getWorldMesh } from "../../api";
import type { MeshTextureDto, WorldMeshDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { disposeTree, parseGlb } from "../sceneUtil";
import { toTexture } from "../sprites";
import type { SceneHost } from "./SceneHost";

/** The layer of the footprints of the pieces (`g4_kernel::world::mesh::PIECE_LAYER`). */
const PIECE_LAYER = 255;
/** Opacity of the footprints: they say where a piece stands, the ground stays visible. */
const FOOTPRINT_OPACITY = 0.6;

/** The placeholder colour of a tile without a texture, by its role (`szene.gelaende`; no new tokens). */
export function placeholderColor(role: MeshTextureDto["role"]): number {
  switch (role) {
    case "grass":
      return szene.gelaende["hohes-gras"];
    case "water":
      return szene.gelaende.wasser;
    case "wall":
      return szene.gelaende.blockiert;
    case "piece":
      return szene.gelaende.spezial;
    default:
      return szene.gelaende.unbekannt;
  }
}

/**
 * One geometry per texture index: positions from `corners`, uv = (u / width, −v / height), indices 0 1 2, 0 2 3 per face (the corners
 * come counter-clockwise seen from the front, so both triangles face it). The faces of a geometry are ordered by layer, each run of a
 * layer one group; `userData.layers` holds the layer of each group (its material index).
 */
export function meshGeometries(dto: WorldMeshDto): Map<number, THREE.BufferGeometry> {
  const byTexture = new Map<number, number[]>();
  for (let f = 0; f < dto.faces; f++) {
    const t = dto.texture[f];
    let faces = byTexture.get(t);
    if (!faces) byTexture.set(t, (faces = []));
    faces.push(f);
  }
  const out = new Map<number, THREE.BufferGeometry>();
  for (const [t, unsorted] of [...byTexture].sort(([a], [b]) => a - b)) {
    // Stable: the faces of one layer keep the mesher's order.
    const faces = [...unsorted].sort((a, b) => dto.layer[a] - dto.layer[b]);
    const { width, height } = dto.textures[t] ?? { width: 1, height: 1 };
    const positions = new Float32Array(faces.length * 12);
    const uv = new Float32Array(faces.length * 8);
    const index: number[] = [];
    const geometry = new THREE.BufferGeometry();
    const layers: number[] = [];
    faces.forEach((f, i) => {
      positions.set(dto.corners.slice(f * 12, f * 12 + 12), i * 12);
      for (let c = 0; c < 4; c++) {
        uv[i * 8 + c * 2] = dto.uv[f * 8 + c * 2] / width;
        uv[i * 8 + c * 2 + 1] = -dto.uv[f * 8 + c * 2 + 1] / height;
      }
      const v = i * 4;
      index.push(v, v + 1, v + 2, v, v + 2, v + 3);
      const layer = dto.layer[f];
      if (layers[layers.length - 1] !== layer) {
        layers.push(layer);
        geometry.addGroup(i * 6, 0, layers.length - 1);
      }
      geometry.groups[geometry.groups.length - 1].count += 6;
    });
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
    geometry.setIndex(index);
    geometry.computeVertexNormals();
    geometry.userData.layers = layers;
    out.set(t, geometry);
  }
  return out;
}

/**
 * The textures of the world meshes, by package and texture name. The cache hands out shared textures (a disposed one is uploaded again
 * when drawn) and frees them in `resetWorldTextures`. Concurrent asks share one request; a texture the package lacks (`null`) is
 * remembered, a failed request is not.
 */
const textures = new Map<string, Promise<THREE.Texture | null>>();
/** Bumped by `resetWorldTextures`; a request started before it belongs to the previous project. */
let generation = 0;

function worldTexture(pkg: number, texture: string): Promise<THREE.Texture | null> {
  const key = `${pkg}:${texture}`;
  const known = textures.get(key);
  if (known) return known;
  const started = generation;
  const request = getTilesetTexture(pkg, texture).then(
    (dto) => {
      const made = dto ? toTexture(dto.width, dto.height, dto.rgba) : null;
      if (started !== generation) {
        made?.dispose();
        return null;
      }
      if (made) {
        made.wrapS = THREE.RepeatWrapping;
        made.wrapT = THREE.RepeatWrapping;
      }
      return made;
    },
    () => {
      if (textures.get(key) === request) textures.delete(key);
      return null;
    },
  );
  textures.set(key, request);
  return request;
}

/** Forgets every world texture and disposes them (another project, or the package's files were written). */
export function resetWorldTextures(): void {
  generation++;
  const all = [...textures.values()];
  textures.clear();
  for (const request of all) void request.then((t) => t?.dispose());
}

/** The material of one layer: the texture where there is one (else the placeholder colour of `role`), overlays and footprints pulled to the front. */
function layerMaterial(texture: THREE.Texture | null, role: MeshTextureDto["role"], layer: number, footprintOffset: number): THREE.MeshLambertMaterial {
  const material = new THREE.MeshLambertMaterial({ side: THREE.FrontSide, alphaTest: 0.5 });
  if (texture) material.map = texture;
  else material.color.setHex(placeholderColor(role));
  if (layer > 0) {
    const offset = layer === PIECE_LAYER ? footprintOffset : -layer;
    material.polygonOffset = true;
    material.polygonOffsetFactor = offset;
    material.polygonOffsetUnits = offset;
  }
  if (layer === PIECE_LAYER) {
    material.transparent = true;
    material.opacity = FOOTPRINT_OPACITY;
  }
  material.userData.layer = layer;
  return material;
}

/** The mesh group of map `map` (name `world-mesh:<map>`), textured where the package has the texture, else the placeholder of the role. */
export async function worldMeshModel(map: number): Promise<{ model: THREE.Group; faces: number }> {
  const dto = await getWorldMesh(map);
  const loaded = await Promise.all(dto.textures.map((t) => worldTexture(dto.tileset, t.name)));
  // The footprints lie over the highest overlay.
  const overlays = dto.layer.filter((l) => l !== PIECE_LAYER);
  const footprintOffset = -(Math.max(0, ...overlays) + 1);
  const model = new THREE.Group();
  model.name = `world-mesh:${map}`;
  for (const [t, geometry] of meshGeometries(dto)) {
    const role = dto.textures[t]?.role ?? "ground";
    const materials = (geometry.userData.layers as number[]).map((layer) => layerMaterial(loaded[t] ?? null, role, layer, footprintOffset));
    const mesh = new THREE.Mesh(geometry, materials);
    mesh.name = dto.textures[t]?.name ?? `texture:${t}`;
    model.add(mesh);
  }
  model.userData.faces = dto.faces;
  return { model, faces: dto.faces };
}

/**
 * The terrain model of `map` for the scene: the world mesh when `h.loaded.get(map)?.map.world`, else the GLB as before; notes the mesh
 * on the host (`noteWorldMesh`) and forgets the marks of maps the scene no longer holds. Undo and redo refetch the map and its model at
 * the same time, so the flag can change while the model loads: a GLB that arrives for a map that is a world map by then gives way to
 * the mesh, and an empty mesh (the map has no world, or it does not read into cells) to the GLB.
 */
export async function mapModel(h: SceneHost, map: number, area: number): Promise<THREE.Object3D> {
  for (const shown of [...h.worldMeshes.keys()]) if (!h.loaded.has(shown)) h.noteWorldMesh(shown, null);
  const isWorld = () => h.loaded.get(map)?.map.world === true;
  const mesh = async () => {
    const { model, faces } = await worldMeshModel(map);
    if (faces > 0) {
      h.noteWorldMesh(map, faces);
      return model;
    }
    disposeTree(model);
    return null;
  };
  const glb = () => getMapModelGlb(map, area).then(parseGlb);
  if (isWorld()) {
    const model = await mesh();
    if (model) return model;
    h.noteWorldMesh(map, null);
    return glb();
  }
  const model = await glb();
  if (isWorld()) {
    const world = await mesh();
    if (world) {
      disposeTree(model);
      return world;
    }
  }
  h.noteWorldMesh(map, null);
  return model;
}
