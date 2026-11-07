import * as THREE from "three";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MeshTextureDto, SpriteDto, WorldMeshDto } from "../../api";

const api = vi.hoisted(() => ({
  getWorldMesh: vi.fn(),
  getTilesetTexture: vi.fn(),
  getMapModelGlb: vi.fn(),
}));
vi.mock("../../api", () => api);
vi.mock("../sceneUtil", () => ({ disposeTree: () => undefined, parseGlb: async () => Object.assign(new THREE.Group(), { name: "glb" }) }));

import { szene } from "../../ui/tokens/tokens";
import type { SceneHost } from "./SceneHost";
import { mapModel, meshGeometries, placeholderColor, resetWorldTextures, worldMeshModel } from "./worldMesh";

/** A floor quad of one tile at cell (cx, cz) and height `y`, corners NW, SW, SE, NE (counter-clockwise from above), UV = (x, z). */
function floor(cx: number, cz: number, y = 0) {
  const x0 = 16 * cx - 256;
  const z0 = 16 * cz - 256;
  const corners = [x0, y, z0, x0, y, z0 + 16, x0 + 16, y, z0 + 16, x0 + 16, y, z0];
  const uv = [x0, z0, x0, z0 + 16, x0 + 16, z0 + 16, x0 + 16, z0];
  return { corners, uv };
}

function dto(faces: { cx: number; cz: number; texture: number; layer: number }[], textures: MeshTextureDto[]): WorldMeshDto {
  const quads = faces.map((f) => floor(f.cx, f.cz));
  return {
    map: 12,
    tileset: 3,
    faces: faces.length,
    textures,
    corners: quads.flatMap((q) => q.corners),
    uv: quads.flatMap((q) => q.uv),
    texture: faces.map((f) => f.texture),
    layer: faces.map((f) => f.layer),
  };
}

const small: MeshTextureDto = { name: "gras_a", width: 16, height: 16, role: "ground" };
const large: MeshTextureDto = { name: "hohes_gras", width: 32, height: 32, role: "grass" };

/** A host as far as `mapModel` reads it: the loaded maps and the world mesh marks. */
function fakeHost(world: Record<number, boolean>) {
  const notes: [number, number | null][] = [];
  const loaded = new Map(Object.entries(world).map(([id, w]) => [Number(id), { map: { world: w } }]));
  const worldMeshes = new Map<number, number>();
  const h = {
    loaded,
    worldMeshes,
    mapModelGen: new Map<number, number>(),
    noteWorldMesh(map: number, faces: number | null) {
      notes.push([map, faces]);
      if (faces === null) worldMeshes.delete(map);
      else worldMeshes.set(map, faces);
    },
  } as unknown as SceneHost;
  return { h, notes };
}

/** The materials of every mesh under `model`, in order. */
function materials(model: THREE.Object3D): THREE.MeshLambertMaterial[] {
  const out: THREE.MeshLambertMaterial[] = [];
  model.traverse((obj) => {
    const mesh = obj as THREE.Mesh;
    if (!mesh.isMesh) return;
    out.push(...((Array.isArray(mesh.material) ? mesh.material : [mesh.material]) as THREE.MeshLambertMaterial[]));
  });
  return out;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetWorldTextures();
});

describe("worldMesh", () => {
  it("one_geometry_per_texture_with_normalized_uv", () => {
    const geometries = meshGeometries(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }, { cx: 1, cz: 0, texture: 1, layer: 0 }], [small, large]));
    expect([...geometries.keys()]).toEqual([0, 1]);
    const first = geometries.get(0)!;
    expect(Array.from(first.getAttribute("position").array)).toEqual(floor(0, 0).corners);
    // u / width, −v / height.
    expect(Array.from(first.getAttribute("uv").array)).toEqual(floor(0, 0).uv.map((t, i) => (i % 2 === 0 ? t / 16 : -t / 16)));
    const second = geometries.get(1)!;
    expect(Array.from(second.getAttribute("position").array)).toEqual(floor(1, 0).corners);
    expect(Array.from(second.getAttribute("uv").array)).toEqual(floor(1, 0).uv.map((t, i) => (i % 2 === 0 ? t / 32 : -t / 32)));
  });

  it("indices_make_two_counter_clockwise_triangles", () => {
    const geometry = meshGeometries(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }, { cx: 1, cz: 0, texture: 0, layer: 0 }], [small])).get(0)!;
    expect(Array.from(geometry.getIndex()!.array)).toEqual([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    // Both triangles of a floor face upwards (front side seen from above).
    const p = geometry.getAttribute("position");
    const v = (i: number) => new THREE.Vector3(p.getX(i), p.getY(i), p.getZ(i));
    for (const [a, b, c] of [[0, 1, 2], [0, 2, 3]]) {
      const normal = new THREE.Vector3().crossVectors(v(b).sub(v(a)), v(c).sub(v(a)));
      expect(normal.y).toBeGreaterThan(0);
    }
  });

  it("placeholder_by_role_when_the_texture_is_missing", async () => {
    api.getWorldMesh.mockResolvedValue(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }], [large]));
    api.getTilesetTexture.mockResolvedValue(null);
    const { model, faces } = await worldMeshModel(12);
    expect(faces).toBe(1);
    expect(model.name).toBe("world-mesh:12");
    expect(api.getTilesetTexture).toHaveBeenCalledWith(3, "hohes_gras");
    const [material] = materials(model);
    expect(material.map).toBeNull();
    expect(material.color.getHex()).toBe(new THREE.Color(szene.gelaende["hohes-gras"]).getHex());
    expect(placeholderColor("grass")).toBe(szene.gelaende["hohes-gras"]);
    expect(placeholderColor("water")).toBe(szene.gelaende.wasser);
    expect(placeholderColor("wall")).toBe(szene.gelaende.blockiert);
    expect(placeholderColor("piece")).toBe(szene.gelaende.spezial);
    expect(placeholderColor("ground")).toBe(szene.gelaende.unbekannt);
    expect(placeholderColor("ramp")).toBe(szene.gelaende.unbekannt);
  });

  it("textures_the_faces_where_the_package_has_the_texture", async () => {
    api.getWorldMesh.mockResolvedValue(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }], [small]));
    const sprite: SpriteDto = { width: 16, height: 16, rgba: new Array(16 * 16 * 4).fill(255) };
    api.getTilesetTexture.mockResolvedValue(sprite);
    const [material] = materials((await worldMeshModel(12)).model);
    expect(material.map).not.toBeNull();
    expect(material.map!.wrapS).toBe(THREE.RepeatWrapping);
    expect(material.map!.wrapT).toBe(THREE.RepeatWrapping);
    expect(material.alphaTest).toBe(0.5);
    expect(material.side).toBe(THREE.FrontSide);
  });

  it("overlays_get_a_polygon_offset", async () => {
    api.getWorldMesh.mockResolvedValue(
      dto(
        [
          { cx: 0, cz: 0, texture: 0, layer: 0 },
          { cx: 0, cz: 0, texture: 0, layer: 1 },
          { cx: 0, cz: 0, texture: 1, layer: 2 },
          { cx: 1, cz: 1, texture: 1, layer: 255 },
        ],
        [small, large],
      ),
    );
    api.getTilesetTexture.mockResolvedValue(null);
    const all = materials((await worldMeshModel(12)).model);
    const byLayer = new Map(all.map((m) => [m.userData.layer as number, m]));
    expect(byLayer.get(0)!.polygonOffset).toBe(false);
    expect(byLayer.get(1)!.polygonOffset).toBe(true);
    expect([byLayer.get(1)!.polygonOffsetFactor, byLayer.get(1)!.polygonOffsetUnits]).toEqual([-1, -1]);
    expect([byLayer.get(2)!.polygonOffsetFactor, byLayer.get(2)!.polygonOffsetUnits]).toEqual([-2, -2]);
    // The footprints lie over every layer, half transparent.
    const footprint = byLayer.get(255)!;
    expect(footprint.polygonOffset).toBe(true);
    expect(footprint.polygonOffsetFactor).toBeLessThan(-2);
    expect(footprint.transparent).toBe(true);
    expect(footprint.opacity).toBe(0.6);
  });

  it("map_model_takes_the_glb_for_binary_maps", async () => {
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    const { h, notes } = fakeHost({ 7: false });
    const model = await mapModel(h, 7, 2);
    expect(model.name).toBe("glb");
    expect(api.getMapModelGlb).toHaveBeenCalledWith(7, 2);
    expect(api.getWorldMesh).not.toHaveBeenCalled();
    expect(notes).toEqual([[7, null]]);
  });

  it("map_model_takes_the_mesh_for_world_maps_and_marks_it", async () => {
    api.getWorldMesh.mockResolvedValue(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }], [small]));
    api.getTilesetTexture.mockResolvedValue(null);
    const { h, notes } = fakeHost({ 12: true });
    const model = await mapModel(h, 12, 2);
    expect(model.name).toBe("world-mesh:12");
    expect(api.getMapModelGlb).not.toHaveBeenCalled();
    expect(notes).toEqual([[12, 1]]);
  });

  it("map_model_forgets_marks_of_maps_no_longer_loaded", async () => {
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    const { h } = fakeHost({ 7: false });
    h.worldMeshes.set(40, 9);
    await mapModel(h, 7, 2);
    expect([...h.worldMeshes.keys()]).toEqual([]);
  });

  it("refresh_asks_for_the_mesh_before_the_flag_follows", async () => {
    // Redo of „Welt anlegen“: the map refetch has not flipped `world` yet when the model refresh starts.
    const { h, notes } = fakeHost({ 12: false });
    api.getWorldMesh.mockResolvedValue(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }], [small]));
    api.getTilesetTexture.mockResolvedValue(null);
    expect((await mapModel(h, 12, 2, { probe: true })).name).toBe("world-mesh:12");
    expect(api.getMapModelGlb).not.toHaveBeenCalled();
    expect(notes).toEqual([[12, 1]]);
  });

  it("refresh_takes_the_glb_when_the_mesh_is_empty", async () => {
    // Undo of „Welt anlegen“: the flag still says world, the backend has none any more.
    const { h, notes } = fakeHost({ 12: true });
    api.getWorldMesh.mockResolvedValue({ map: 12, tileset: 0, faces: 0, textures: [], corners: [], uv: [], texture: [], layer: [] });
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    expect((await mapModel(h, 12, 2, { probe: true })).name).toBe("glb");
    expect(notes).toEqual([[12, null]]);
  });

  it("only_the_latest_call_for_a_map_sets_its_mark", async () => {
    // Fast ⌘Z, ⇧⌘Z: two refreshes overlap and the older one resolves last.
    const { h, notes } = fakeHost({ 12: true });
    api.getTilesetTexture.mockResolvedValue(null);
    let resolveOld!: (d: WorldMeshDto) => void;
    api.getWorldMesh.mockImplementationOnce(() => new Promise<WorldMeshDto>((r) => (resolveOld = r)));
    api.getWorldMesh.mockResolvedValueOnce(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }], [small]));
    h.mapModelGen.set(12, 1);
    const older = mapModel(h, 12, 2, { probe: true });
    h.mapModelGen.set(12, 2);
    await mapModel(h, 12, 2, { probe: true });
    resolveOld(dto([{ cx: 0, cz: 0, texture: 0, layer: 0 }, { cx: 1, cz: 0, texture: 0, layer: 0 }, { cx: 2, cz: 0, texture: 0, layer: 0 }], [small]));
    await older;
    expect(notes).toEqual([[12, 1]]);
    expect(h.worldMeshes.get(12)).toBe(1);
  });

  it("map_model_takes_the_glb_when_the_world_went_meanwhile", async () => {
    const { h, notes } = fakeHost({ 12: true });
    // A map without a world answers an empty mesh.
    api.getWorldMesh.mockResolvedValue({ map: 12, tileset: 0, faces: 0, textures: [], corners: [], uv: [], texture: [], layer: [] });
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    expect((await mapModel(h, 12, 2)).name).toBe("glb");
    expect(notes).toEqual([[12, null]]);
  });
  it("map_model_falls_back_to_the_glb_without_faces", async () => {
    // A world adopted with Form 1 has no grid: the scene shows its baked model through the GLB.
    const { h, notes } = fakeHost({ 0: true });
    api.getWorldMesh.mockResolvedValue({ map: 0, tileset: 6, faces: 0, textures: [], corners: [], uv: [], texture: [], layer: [] });
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    expect((await mapModel(h, 0, 2)).name).toBe("glb");
    expect(api.getWorldMesh).toHaveBeenCalledWith(0);
    expect(api.getMapModelGlb).toHaveBeenCalledWith(0, 2);
    expect(notes).toEqual([[0, null]]);
  });

  it("map_model_falls_back_to_the_glb_on_error", async () => {
    // A world whose mesh does not read (rows that do not parse, a package that fails): the GLB, not a hole in the scene.
    const { h, notes } = fakeHost({ 0: true });
    api.getWorldMesh.mockRejectedValue({ code: "error", message: "world.schema: kaputt", asset: null });
    api.getMapModelGlb.mockResolvedValue(new ArrayBuffer(4));
    expect((await mapModel(h, 0, 2)).name).toBe("glb");
    expect((await mapModel(h, 0, 2, { probe: true })).name).toBe("glb");
    expect(notes).toEqual([
      [0, null],
      [0, null],
    ]);
  });
});
