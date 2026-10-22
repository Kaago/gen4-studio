import * as THREE from "three";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getObjectSprite } from "../../api";
import type { EventsDto, MatrixDto, SpriteDto, TileBehavior } from "../../api";
import { MAP_SIZE, MODEL_SCALE } from "../layout";
import { resetSprites } from "../sprites";
import { OVERLAY_Y, addModel, buildContent, recolourMap, removeModels, setMarkers } from "./content";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), getObjectSprite: vi.fn() }));

const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, 1], headers: null, altitudes: null };

const loadedMap = (index: number): Loaded => ({ map: { index, permissions: Array.from({ length: MAP_SIZE * MAP_SIZE }, () => [0, 0] as [number, number]), buildings: [] } });

/** The parts of a scene handle the content functions touch. */
function handle(partial: Partial<SceneHost> = {}): SceneHost {
  return {
    scene: new THREE.Scene(),
    content: new THREE.Group(),
    models: [],
    matrix,
    loaded: new Map(),
    bases: new Map(),
    names: new Map(),
    tiles: null,
    markers: null,
    events: null,
    selectedEvent: null,
    queueRender: vi.fn(),
    ...partial,
  } as unknown as SceneHost;
}

const model = () => new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial());

describe("addModel", () => {
  it("hangs one scaled, centred instance into every cell showing the map", () => {
    const h = handle();
    addModel(h, matrix, 1, model());
    expect(h.models).toHaveLength(2);
    expect(h.content.children).toHaveLength(2);
    expect(h.models[0].scale.x).toBe(MODEL_SCALE);
    expect(h.models.map((m) => m.position.x)).toEqual([MAP_SIZE / 2, MAP_SIZE + MAP_SIZE / 2]);
    expect(h.models[0].position.z).toBe(MAP_SIZE / 2);
  });

  it("stands the model of a raised cell on the height of that cell", () => {
    const h = handle();
    addModel(h, { ...matrix, altitudes: [0, 4] }, 1, model());
    expect(h.models.map((m) => m.position.y)).toEqual([0, 2]);
  });

  it("disposes a model that no cell shows", () => {
    const h = handle();
    const m = model();
    const disposed = vi.spyOn(m.geometry, "dispose");
    addModel(h, matrix, 7, m);
    expect(h.models).toHaveLength(0);
    expect(disposed).toHaveBeenCalled();
  });
});

describe("removeModels", () => {
  it("takes every instance out of the content and clears the list", () => {
    const h = handle();
    addModel(h, matrix, 1, model());
    const instances = [...h.models];
    removeModels(h);
    expect(h.models).toEqual([]);
    expect(h.content.children).toHaveLength(0);
    expect(instances.every((i) => i.parent === null)).toBe(true);
  });
});

describe("buildContent / recolourMap", () => {
  const names = new Map<number, TileBehavior>();

  it("lays one overlay instance per tile of every loaded cell and remembers where each map starts", () => {
    const loaded = new Map([[1, loadedMap(1)]]);
    const built = buildContent(matrix, loaded, names);
    expect(built.tiles.count).toBe(2 * MAP_SIZE * MAP_SIZE);
    expect(built.bases.get(1)).toEqual([0, MAP_SIZE * MAP_SIZE]);
    expect(built.content.children).toContain(built.overlay);
    expect(built.content.children).toContain(built.buildings.root);
    built.buildings.dispose();
  });

  it("lifts the overlay tiles of a raised cell by the height of that cell", () => {
    const loaded = new Map([[1, loadedMap(1)]]);
    const built = buildContent({ ...matrix, altitudes: [0, 2] }, loaded, names);
    const y = (i: number) => {
      const m = new THREE.Matrix4();
      built.tiles.getMatrixAt(i, m);
      return new THREE.Vector3().setFromMatrixPosition(m).y;
    };
    expect(y(0)).toBeCloseTo(OVERLAY_Y, 5);
    expect(y(MAP_SIZE * MAP_SIZE)).toBeCloseTo(OVERLAY_Y + 1, 5);
    built.buildings.dispose();
  });

  it("recolours every instance of a map from its current permissions", () => {
    const loaded = new Map([[1, loadedMap(1)]]);
    const built = buildContent(matrix, loaded, names);
    const h = handle({ loaded, bases: built.bases, tiles: built.tiles, names });
    const before = new THREE.Color();
    built.tiles.getColorAt(0, before);
    const version = built.tiles.instanceColor?.version ?? 0;
    loaded.get(1)!.map.permissions[0] = [0, 0x80];
    recolourMap(h, 1);
    const after = new THREE.Color();
    built.tiles.getColorAt(0, after);
    const second = new THREE.Color();
    built.tiles.getColorAt(MAP_SIZE * MAP_SIZE, second);
    expect(after.getHex()).not.toBe(before.getHex());
    expect(second.getHex()).toBe(after.getHex());
    expect(built.tiles.instanceColor?.version).toBeGreaterThan(version);
    built.buildings.dispose();
  });

  it("ignores a map the scene does not show", () => {
    const h = handle();
    expect(() => recolourMap(h, 9)).not.toThrow();
  });
});

describe("setMarkers", () => {
  const events: EventsDto = {
    event_file_id: 1,
    locked: null,
    bg: [],
    objects: [],
    warps: [{ index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 }],
    coords: [],
  };

  it("shows markers in the scene with the given visibility", () => {
    const h = handle();
    setMarkers(h, events, false);
    expect(h.events).toBe(events);
    expect(h.markers?.group.visible).toBe(false);
    expect(h.scene.children).toContain(h.markers?.group);
  });

  it("replaces and disposes the markers before, and shows none for null events", () => {
    const h = handle();
    setMarkers(h, events, true);
    const old = h.markers!.group;
    setMarkers(h, null, true);
    expect(h.markers).toBeNull();
    expect(h.events).toBeNull();
    expect(h.scene.children).not.toContain(old);
  });

  it("keeps the selected event of the same file outlined", () => {
    const h = handle({ selectedEvent: { file: 1, kind: "warp", index: 0 } });
    setMarkers(h, events, true);
    expect(h.markers?.selected).toMatchObject({ kind: "warp", index: 0 });
  });

  it("builds no markers without a matrix", () => {
    const h = handle({ matrix: null });
    setMarkers(h, events, true);
    expect(h.markers).toBeNull();
    expect(h.events).toBe(events);
  });
});

describe("setMarkers sprites", () => {
  const object = (index: number, graphics_id: number, dir = 1) => ({ index, local_id: index, graphics_id, movement_type: 0, script: 0, dir, x: 4 + index, z: 5, y: 0 });
  const events = (...objects: ReturnType<typeof object>[]): EventsDto => ({ event_file_id: 1, locked: null, bg: [], objects, warps: [], coords: [] });
  const sprite = (w = 16, h = 32): SpriteDto => ({ width: w, height: h, rgba: new Array<number>(w * h * 4).fill(255) });
  const sprites = (h: SceneHost) => [...(h.markers?.entries.keys() ?? [])].filter((o): o is THREE.Sprite => o instanceof THREE.Sprite);
  const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

  beforeEach(() => {
    resetSprites();
    vi.mocked(getObjectSprite).mockReset().mockResolvedValue(null);
  });

  it("draws the sprite of an object when it arrives, and asks for a render", async () => {
    vi.mocked(getObjectSprite).mockImplementation(async (id) => (id === 7 ? sprite(16, 32) : null));
    const h = handle();
    setMarkers(h, events(object(0, 7, 2), object(1, 900)), true);
    const [known, unknown] = sprites(h);
    // The yellow square holds until then.
    expect(known.material.map).toBe(h.markers?.placeholder);
    await settle();
    expect(getObjectSprite).toHaveBeenCalledWith(7, 2);
    expect(known.material.map).not.toBe(h.markers?.placeholder);
    expect(known.scale.toArray()).toEqual([1, 2, 1]);
    // Unknown graphics: no picture, the marker stays.
    expect(unknown.material.map).toBe(h.markers?.placeholder);
    expect(unknown.scale.toArray()).toEqual([0.8, 0.4, 1]);
    expect(h.queueRender).toHaveBeenCalledTimes(1);
  });

  it("drops a sprite that arrives for markers that have been replaced meanwhile", async () => {
    let answer: (s: SpriteDto) => void = () => undefined;
    vi.mocked(getObjectSprite).mockReturnValue(new Promise((resolve) => (answer = resolve)));
    const h = handle();
    setMarkers(h, events(object(0, 7)), true);
    const [old] = sprites(h);
    setMarkers(h, events(object(0, 7)), true);
    answer(sprite());
    await settle();
    // One request served both sets, but only the current set is drawn into; the replaced one was disposed.
    expect(getObjectSprite).toHaveBeenCalledTimes(1);
    expect(sprites(h)[0]).not.toBe(old);
    expect(sprites(h)[0].scale.y).toBe(2);
    expect(old.scale.y).toBe(0.4);
    expect(h.queueRender).toHaveBeenCalledTimes(1);
  });

  it("shows a sprite that is already loaded as soon as the markers are rebuilt, without asking the backend again", async () => {
    vi.mocked(getObjectSprite).mockResolvedValue(sprite());
    const h = handle();
    setMarkers(h, events(object(0, 7)), true);
    await settle();
    setMarkers(h, events(object(0, 7)), true);
    expect(sprites(h)[0].scale.toArray()).toEqual([1, 2, 1]);
    expect(getObjectSprite).toHaveBeenCalledTimes(1);
  });

  it("keeps the marker when the sprite cannot be read", async () => {
    vi.mocked(getObjectSprite).mockRejectedValue({ code: "error", message: "no table", asset: null });
    const h = handle();
    setMarkers(h, events(object(0, 7)), true);
    await settle();
    expect(sprites(h)[0].material.map).toBe(h.markers?.placeholder);
    expect(h.queueRender).not.toHaveBeenCalled();
  });
});
