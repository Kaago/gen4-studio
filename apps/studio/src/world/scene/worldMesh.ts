import * as THREE from "three";
import { getMapModelGlb, getTilesetTexture, getWorldMesh } from "../../api";
import type { MeshTextureDto, WorldMeshDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { disposeTree, parseGlb } from "../sceneUtil";
import { toTexture } from "../sprites";
import type { SceneHost } from "./SceneHost";

/** The layer of the pieces, footprints and model geometry (`g4_kernel::world::mesh::PIECE_LAYER`). */
const PIECE_LAYER = 255;
/** The layer of the free-form models (`g4_kernel::world::FREEFORM_LAYER`): solid geometry like the ground. */
const FREEFORM_LAYER = 254;
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
 * come counter-clockwise seen from the front, so both triangles face it). The faces of a geometry are ordered by layer (per layer the
 * footprints and grid faces before the model geometry), each run of one layer and kind one group; `userData.layers` holds the layer of
 * each group (its material index), `userData.models` whether it is model geometry (`dto.model`).
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
    // Stable: the faces of one layer and kind keep the mesher's order.
    const faces = [...unsorted].sort((a, b) => dto.layer[a] - dto.layer[b] || Number(dto.model[a]) - Number(dto.model[b]));
    const { width, height } = dto.textures[t] ?? { width: 1, height: 1 };
    const positions = new Float32Array(faces.length * 12);
    const uv = new Float32Array(faces.length * 8);
    const index: number[] = [];
    const geometry = new THREE.BufferGeometry();
    const layers: number[] = [];
    const models: boolean[] = [];
    faces.forEach((f, i) => {
      positions.set(dto.corners.slice(f * 12, f * 12 + 12), i * 12);
      for (let c = 0; c < 4; c++) {
        uv[i * 8 + c * 2] = dto.uv[f * 8 + c * 2] / width;
        uv[i * 8 + c * 2 + 1] = -dto.uv[f * 8 + c * 2 + 1] / height;
      }
      const v = i * 4;
      index.push(v, v + 1, v + 2, v, v + 2, v + 3);
      const layer = dto.layer[f];
      const model = dto.model[f] === true;
      if (layers[layers.length - 1] !== layer || models[models.length - 1] !== model) {
        layers.push(layer);
        models.push(model);
        geometry.addGroup(i * 6, 0, layers.length - 1);
      }
      geometry.groups[geometry.groups.length - 1].count += 6;
    });
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
    geometry.setIndex(index);
    geometry.computeVertexNormals();
    geometry.userData.layers = layers;
    geometry.userData.models = models;
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

/** Whether faces of `layer` are solid geometry drawn like the ground: model geometry (`model`) and every free-form face. */
const isSolid = (layer: number, model: boolean) => model || layer === FREEFORM_LAYER;

/**
 * The material of one layer: the texture where there is one (else the placeholder colour of `role`), overlays and footprints pulled to
 * the front, the footprints half transparent. Solid geometry (`isSolid`) gets neither: an offset would draw it through the buildings.
 */
function layerMaterial(texture: THREE.Texture | null, role: MeshTextureDto["role"], layer: number, model: boolean, footprintOffset: number): THREE.MeshLambertMaterial {
  const material = new THREE.MeshLambertMaterial({ side: THREE.FrontSide, alphaTest: 0.5 });
  if (texture) material.map = texture;
  else material.color.setHex(placeholderColor(role));
  if (layer > 0 && !isSolid(layer, model)) {
    const offset = layer === PIECE_LAYER ? footprintOffset : -layer;
    material.polygonOffset = true;
    material.polygonOffsetFactor = offset;
    material.polygonOffsetUnits = offset;
  }
  if (layer === PIECE_LAYER && !model) {
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
  // The footprints lie over the highest overlay (solid geometry and the footprints themselves are none).
  const overlays = dto.layer.filter((l, f) => l !== PIECE_LAYER && !isSolid(l, dto.model[f] === true));
  const footprintOffset = -(Math.max(0, ...overlays) + 1);
  const model = new THREE.Group();
  model.name = `world-mesh:${map}`;
  for (const [t, geometry] of meshGeometries(dto)) {
    const role = dto.textures[t]?.role ?? "ground";
    const models = geometry.userData.models as boolean[];
    const materials = (geometry.userData.layers as number[]).map((layer, g) => layerMaterial(loaded[t] ?? null, role, layer, models[g] === true, footprintOffset));
    const mesh = new THREE.Mesh(geometry, materials);
    mesh.name = dto.textures[t]?.name ?? `texture:${t}`;
    model.add(mesh);
  }
  model.userData.faces = dto.faces;
  return { model, faces: dto.faces };
}

/** The layers hidden per scene and map (the eye of „Ebenen“): a mesh loaded later (a stroke, undo) keeps them hidden. */
const hiddenLayers = new WeakMap<SceneHost, Map<number, Set<number>>>();

/** Shows or hides the materials of `layer` in `model` (a `world-mesh:<map>` group); other layers stay as they are. */
function showLayer(model: THREE.Object3D, layer: number, on: boolean) {
  model.traverse((o) => {
    const materials = (o as THREE.Mesh).material;
    if (!materials) return;
    for (const m of Array.isArray(materials) ? materials : [materials]) if (m.userData.layer === layer) m.visible = on;
  });
}

/**
 * Shows or hides layer `layer` of the world mesh of `map` in the scene of `h` (in every matrix cell showing it): the materials of that
 * layer (`userData.layer`, one per group of `meshGeometries`). Only the scene changes, nothing is saved; a mesh loaded later for `map`
 * (`mapModel`) keeps the layer hidden. Asks for a render.
 */
export function setLayerVisible(h: SceneHost, map: number, layer: number, on: boolean): void {
  let byMap = hiddenLayers.get(h);
  if (!byMap) hiddenLayers.set(h, (byMap = new Map()));
  const layers = byMap.get(map) ?? new Set<number>();
  if (on) layers.delete(layer);
  else layers.add(layer);
  if (layers.size > 0) byMap.set(map, layers);
  else byMap.delete(map);
  const name = `world-mesh:${map}`;
  h.scene.traverse((o) => {
    if (o.name === name) showLayer(o, layer, on);
  });
  h.queueRender?.();
}

/** Hides the layers of `map` hidden on `h` in a freshly loaded mesh `model`. */
function hideLayers(h: SceneHost, map: number, model: THREE.Object3D) {
  for (const layer of hiddenLayers.get(h)?.get(map) ?? []) showLayer(model, layer, false);
}

/**
 * The terrain model of `map` for the scene: the world mesh of a world map, else the GLB as before. The initial load trusts the flag
 * (`h.loaded.get(map)?.map.world`, fresh from its own fetch). A refresh (`probe`) does not: undo, redo and agent edits refetch the map
 * and its model at the same time, so it asks for the mesh first and takes it when it has faces, else the GLB. An empty mesh (no world,
 * one that does not read into cells, or one without tiles like a world adopted with Form 1) and a mesh that fails always give way to
 * the GLB, the baked model.
 *
 * Notes the result on the host (`noteWorldMesh`) only while the map's model generation (`mapModelGen`, bumped by both callers before
 * they call) is the one it started under: a superseded call's model is dropped by its caller, so its mark must not land either.
 * Forgets the marks of maps the scene no longer holds.
 */
export async function mapModel(h: SceneHost, map: number, area: number, { probe = false }: { probe?: boolean } = {}): Promise<THREE.Object3D> {
  for (const shown of [...h.worldMeshes.keys()]) if (!h.loaded.has(shown)) h.noteWorldMesh(shown, null);
  const gen = h.mapModelGen.get(map);
  const note = (faces: number | null) => {
    if (h.mapModelGen.get(map) === gen) h.noteWorldMesh(map, faces);
  };
  if (probe || h.loaded.get(map)?.map.world === true) {
    // A mesh that does not read gives way to the GLB (the baked model) like an empty one.
    const mesh = await worldMeshModel(map).catch(() => null);
    if (mesh !== null && mesh.faces > 0) {
      note(mesh.faces);
      hideLayers(h, map, mesh.model);
      return mesh.model;
    }
    if (mesh !== null) disposeTree(mesh.model);
  }
  const model = await getMapModelGlb(map, area).then(parseGlb);
  note(null);
  return model;
}
