// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import * as THREE from "three";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { errorText, getMap, getMapModelGlb, mapModelTextureStatus } from "../../api";
import type { MapDto, MatrixDto, TextureIssue } from "../../api";
import { enqueue } from "../../reload";
import type { RefreshQueue } from "../../reload";
import { createGround } from "../ground";
import { MAP_SIZE } from "../layout";
import { textureNote } from "../load";
import { parseGlb } from "../sceneUtil";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";
import { OVERLAY_Y, buildContent } from "./content";
import { useMapModelRefresh, useMapRefresh } from "./useRefreshQueues";
import { loadModels } from "./useSceneLoader";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getMap: vi.fn(),
  getMapModelGlb: vi.fn(),
  mapModelTextureStatus: vi.fn(),
}));
vi.mock("../sceneUtil", async (importOriginal) => ({ ...(await importOriginal<typeof import("../sceneUtil")>()), parseGlb: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function Probe({ refs, queue }: { refs: SceneRefs; queue: RefreshQueue<number> | null }) {
  useMapModelRefresh(refs, queue);
  return null;
}

/** Map 5 is shown in two cells, map 2 in one. */
const matrix: MatrixDto = { index: 0, name: "m", width: 3, height: 1, maps: [5, 2, 5], headers: null, altitudes: null };

const mesh = () => new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial());

/** An instance as the full load leaves it in the scene: tagged with its map, in `content` and `models`. */
function instance(h: SceneHost, mapId: number) {
  const m = mesh();
  m.userData.mapId = mapId;
  h.content.add(m);
  h.models.push(m);
  return m;
}

/** The parts of a scene handle the model swap touches. */
function fakeScene() {
  const h = {
    matrix,
    loaded: new Map([
      [5, { map: { world: false } }],
      [2, { map: { world: false } }],
    ]),
    content: new THREE.Group(),
    models: [],
    // Binary maps: `mapModel` takes their GLB and notes that they show no world mesh.
    worldMeshes: new Map(),
    noteWorldMesh: vi.fn(),
    queueRender: vi.fn(),
    disposed: false,
    mapModelGen: new Map(),
    // What `loadModels` reads besides the above: no buildings, nothing failed before.
    buildings: null,
    loadProblems: { names: null, maps: [] },
  } as unknown as SceneHost;
  const onError = vi.fn();
  const onStatus = vi.fn();
  const refs = { handle: { current: h }, area: { current: 3 }, callbacks: { current: { onError, onStatus } } } as unknown as SceneRefs;
  return { h, refs, onError, onStatus };
}

/** A promise resolved from outside. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

/** Lets the pending fetches, parses and status calls settle. */
const flush = () => act(async () => new Promise<void>((r) => setTimeout(r, 0)));

/** The scene instances cloned from `model` (clones share its geometry). */
const clonesOf = (h: SceneHost, model: THREE.Mesh) => h.content.children.filter((c) => (c as THREE.Mesh).geometry === model.geometry);

describe("useMapModelRefresh", () => {
  let container: HTMLElement;
  let root: Root;
  /** The model `parseGlb` gives for each buffer `getMapModelGlb` resolves with. */
  let models: Map<ArrayBuffer, THREE.Mesh>;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    models = new Map();
    vi.mocked(parseGlb).mockImplementation(async (buffer) => models.get(buffer) ?? mesh());
    vi.mocked(mapModelTextureStatus).mockResolvedValue(null);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.mocked(getMapModelGlb).mockReset();
    vi.mocked(mapModelTextureStatus).mockReset();
    vi.mocked(parseGlb).mockReset();
  });

  /** A buffer for which `parseGlb` gives `model`. */
  const glbOf = (model: THREE.Mesh) => {
    const buffer = new ArrayBuffer(0);
    models.set(buffer, model);
    return buffer;
  };

  it("lädt das Modell der Karte für das Gebiet der Szene und ersetzt ihre Instanzen", async () => {
    const { h, refs, onStatus } = fakeScene();
    const old = instance(h, 5);
    const other = instance(h, 2);
    const fresh = mesh();
    vi.mocked(getMapModelGlb).mockResolvedValue(glbOf(fresh));
    act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
    await flush();
    expect(getMapModelGlb).toHaveBeenCalledExactlyOnceWith(5, 3);
    expect(h.content.children).not.toContain(old);
    expect(h.models).not.toContain(old);
    expect(h.content.children).toContain(other);
    expect(clonesOf(h, fresh)).toHaveLength(2);
    expect(h.models.filter((m) => m.userData.mapId === 5)).toHaveLength(2);
    expect(h.queueRender).toHaveBeenCalled();
    // No progress toast: only a texture note would be reported.
    expect(onStatus).not.toHaveBeenCalled();
  });

  it("ignoriert eine Karte, die die Szene nicht zeigt", async () => {
    const { refs, onError } = fakeScene();
    act(() => root.render(<Probe refs={refs} queue={enqueue(null, 9)} />));
    await flush();
    expect(getMapModelGlb).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("nur das jüngste Ergebnis landet in der Szene", async () => {
    const { h, refs } = fakeScene();
    const first = deferred<ArrayBuffer>();
    const second = deferred<ArrayBuffer>();
    vi.mocked(getMapModelGlb).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const older = mesh();
    const newer = mesh();
    const olderDisposed = vi.spyOn(older.geometry, "dispose");
    // The second request arrives while the first still fetches: the queue grew by one entry.
    const q1 = enqueue(null, 5);
    act(() => root.render(<Probe refs={refs} queue={q1} />));
    act(() => root.render(<Probe refs={refs} queue={enqueue(q1, 5)} />));
    expect(getMapModelGlb).toHaveBeenCalledTimes(2);
    second.resolve(glbOf(newer));
    await flush();
    first.resolve(glbOf(older));
    await flush();
    expect(clonesOf(h, newer)).toHaveLength(2);
    expect(clonesOf(h, older)).toHaveLength(0);
    expect(h.content.children).toHaveLength(2);
    expect(olderDisposed).toHaveBeenCalled();
  });

  it("verwirft nach Matrixwechsel", async () => {
    const { h, refs } = fakeScene();
    const old = instance(h, 5);
    const pending = deferred<ArrayBuffer>();
    vi.mocked(getMapModelGlb).mockReturnValue(pending.promise);
    const model = mesh();
    const disposed = vi.spyOn(model.geometry, "dispose");
    act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
    h.matrix = { ...matrix };
    pending.resolve(glbOf(model));
    await flush();
    expect(h.content.children).toEqual([old]);
    expect(h.models).toEqual([old]);
    expect(disposed).toHaveBeenCalled();
    expect(h.queueRender).not.toHaveBeenCalled();
  });

  it("verwirft nach Wechsel der Area-Data", async () => {
    const { h, refs } = fakeScene();
    const old = instance(h, 5);
    const pending = deferred<ArrayBuffer>();
    vi.mocked(getMapModelGlb).mockReturnValue(pending.promise);
    const model = mesh();
    const disposed = vi.spyOn(model.geometry, "dispose");
    act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
    refs.area.current = 4;
    pending.resolve(glbOf(model));
    await flush();
    expect(h.content.children).toEqual([old]);
    expect(h.models).toEqual([old]);
    expect(disposed).toHaveBeenCalled();
  });

  it("ein überholter Abruf, der fehlschlägt, meldet keinen Fehler", async () => {
    const { h, refs, onError } = fakeScene();
    let rejectFirst!: (e: unknown) => void;
    const failing = new Promise<ArrayBuffer>((_, reject) => (rejectFirst = reject));
    const fresh = mesh();
    vi.mocked(getMapModelGlb).mockReturnValueOnce(failing).mockResolvedValueOnce(glbOf(fresh));
    const q1 = enqueue(null, 5);
    act(() => root.render(<Probe refs={refs} queue={q1} />));
    act(() => root.render(<Probe refs={refs} queue={enqueue(q1, 5)} />));
    await flush();
    rejectFirst(new Error("apicula kaputt"));
    await flush();
    expect(onError).not.toHaveBeenCalled();
    expect(clonesOf(h, fresh)).toHaveLength(2);
  });

  it("eine Szene, die noch lädt, verbraucht die Anfrage ohne Abruf", async () => {
    const { h, refs, onError } = fakeScene();
    h.matrix = null;
    const q1 = enqueue(null, 5);
    act(() => root.render(<Probe refs={refs} queue={q1} />));
    await flush();
    expect(getMapModelGlb).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    // Consumed: once the scene is built, a later request fetches only its own map.
    h.matrix = matrix;
    act(() => root.render(<Probe refs={refs} queue={enqueue(q1, 2)} />));
    await flush();
    expect(getMapModelGlb).toHaveBeenCalledExactlyOnceWith(2, 3);
  });

  it("meldet einen Texturhinweis als Status und einen Fehler über onError", async () => {
    const { h, refs, onError, onStatus } = fakeScene();
    const issue: TextureIssue = { tileset: 12, reason: "unsupported" };
    vi.mocked(mapModelTextureStatus).mockResolvedValue(issue);
    vi.mocked(getMapModelGlb).mockResolvedValueOnce(glbOf(mesh()));
    const q1 = enqueue(null, 5);
    act(() => root.render(<Probe refs={refs} queue={q1} />));
    await flush();
    expect(mapModelTextureStatus).toHaveBeenCalledExactlyOnceWith(5, 3);
    expect(onStatus).toHaveBeenCalledExactlyOnceWith({ kind: "info", text: textureNote([issue]) });
    expect(onError).not.toHaveBeenCalled();

    const shown = [...h.content.children];
    const failure = new Error("apicula kaputt");
    vi.mocked(getMapModelGlb).mockRejectedValueOnce(failure);
    act(() => root.render(<Probe refs={refs} queue={enqueue(q1, 5)} />));
    await flush();
    expect(onError).toHaveBeenCalledExactlyOnceWith(errorText(failure));
    expect(h.content.children).toEqual(shown);
    expect(onStatus).toHaveBeenCalledTimes(1);
  });

  describe("mit einer vollen Modellladung im Flug", () => {
    /** Every `getMapModelGlb` call with its map, answered from outside. */
    let calls: { map: number; glb: ReturnType<typeof deferred<ArrayBuffer>> }[];
    beforeEach(() => {
      calls = [];
      vi.mocked(getMapModelGlb).mockImplementation((map) => {
        const glb = deferred<ArrayBuffer>();
        calls.push({ map, glb });
        return glb.promise;
      });
    });
    /** The pending fetches for `map`, oldest first. */
    const callsFor = (map: number) => calls.filter((c) => c.map === map).map((c) => c.glb);

    it("ein Ergebnis der vollen Ladung nach einem Einzelreload der Karte wird verworfen", async () => {
      const { h, refs } = fakeScene();
      const loading = loadModels(h, matrix, 3, () => false, vi.fn());
      const [fullFetch] = callsFor(5);
      // The import lands while the full load still fetches map 5 (the old model).
      act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
      const [, reloadFetch] = callsFor(5);
      const fresh = mesh();
      const outdated = mesh();
      const outdatedDisposed = vi.spyOn(outdated.geometry, "dispose");
      reloadFetch.resolve(glbOf(fresh));
      await flush();
      fullFetch.resolve(glbOf(outdated));
      callsFor(2)[0].resolve(glbOf(mesh()));
      await act(() => loading);
      expect(clonesOf(h, fresh)).toHaveLength(2);
      expect(clonesOf(h, outdated)).toHaveLength(0);
      expect(outdatedDisposed).toHaveBeenCalled();
      expect(h.models.map((m) => m.userData.mapId).sort()).toEqual([2, 5, 5]);
      expect(h.content.children).toHaveLength(3);
    });

    it("ein Einzelreload, der beim Start der vollen Ladung noch lädt, wird verworfen", async () => {
      const { h, refs } = fakeScene();
      act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
      // The full load starts while the reload still fetches map 5; its own fetch is current too.
      const loading = loadModels(h, matrix, 3, () => false, vi.fn());
      const [reloadFetch, fullFetch] = callsFor(5);
      const full = mesh();
      const late = mesh();
      const lateDisposed = vi.spyOn(late.geometry, "dispose");
      fullFetch.resolve(glbOf(full));
      callsFor(2)[0].resolve(glbOf(mesh()));
      await act(() => loading);
      reloadFetch.resolve(glbOf(late));
      await flush();
      expect(clonesOf(h, full)).toHaveLength(2);
      expect(clonesOf(h, late)).toHaveLength(0);
      expect(lateDisposed).toHaveBeenCalled();
      expect(h.models.map((m) => m.userData.mapId).sort()).toEqual([2, 5, 5]);
      expect(h.content.children).toHaveLength(3);
    });

    it("zwei frische Ergebnisse für eine Karte lassen einen Satz Instanzen", async () => {
      const { h, refs } = fakeScene();
      // The reload lands first; the full load starts after it, so its fetch is current too.
      act(() => root.render(<Probe refs={refs} queue={enqueue(null, 5)} />));
      const first = mesh();
      const second = mesh();
      callsFor(5)[0].resolve(glbOf(first));
      await flush();
      expect(clonesOf(h, first)).toHaveLength(2);
      const loading = loadModels(h, matrix, 3, () => false, vi.fn());
      const [, fullFetch] = callsFor(5);
      fullFetch.resolve(glbOf(second));
      callsFor(2)[0].resolve(glbOf(mesh()));
      await act(() => loading);
      expect(clonesOf(h, second)).toHaveLength(2);
      expect(clonesOf(h, first)).toHaveLength(0);
      expect(h.models.map((m) => m.userData.mapId).sort()).toEqual([2, 5, 5]);
      expect(h.content.children).toHaveLength(3);
    });
  });
});

function MapProbe({ refs, queue }: { refs: SceneRefs; queue: RefreshQueue<number> | null }) {
  useMapRefresh(refs, queue);
  return null;
}

describe("useMapRefresh", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.mocked(getMap).mockReset();
  });

  const flat = (index: number, units: number): MapDto => ({ index, permissions: Array.from({ length: MAP_SIZE * MAP_SIZE }, () => [0, 0] as [number, number]), buildings: [], heights: Array.from({ length: MAP_SIZE * MAP_SIZE }, () => units) });

  it("gives the scene a new ground when the refetched map has new heights", async () => {
    // Map 5 in cells 0 and 2, map 2 in cell 1, all flat; an undo gives map 5 a height of 32 E.
    const loaded = new Map([
      [5, { map: flat(5, 0) }],
      [2, { map: flat(2, 0) }],
    ]);
    const ground = createGround(matrix, loaded);
    const built = buildContent(matrix, loaded, new Map(), ground);
    const h = {
      matrix,
      loaded,
      ground,
      tiles: built.tiles,
      bases: built.bases,
      names: new Map(),
      markers: null,
      buildings: null,
      selection: null,
      canvas: document.createElement("canvas"),
      disposed: false,
      render: vi.fn(),
      refreshHover: vi.fn(),
    } as unknown as SceneHost;
    const refs = { handle: { current: h }, area: { current: 3 }, callbacks: { current: { onError: vi.fn(), onBuildings: vi.fn(), onSelectBuilding: vi.fn() } } } as unknown as SceneRefs;
    vi.mocked(getMap).mockResolvedValue(flat(5, 32));
    act(() => root.render(<MapProbe refs={refs} queue={enqueue(null, 5)} />));
    await flush();
    expect(h.ground).not.toBe(ground);
    expect(h.ground.y(0, 0)).toBe(2);
    const y = (i: number) => {
      const m = new THREE.Matrix4();
      built.tiles.getMatrixAt(i, m);
      return new THREE.Vector3().setFromMatrixPosition(m).y;
    };
    // Both cells of map 5 stand on the new height, map 2 stays.
    expect(y(0)).toBeCloseTo(OVERLAY_Y + 2, 5);
    expect(y(2 * MAP_SIZE * MAP_SIZE)).toBeCloseTo(OVERLAY_Y + 2, 5);
    expect(y(MAP_SIZE * MAP_SIZE)).toBeCloseTo(OVERLAY_Y, 5);
    // The same heights again keep the ground.
    const kept = h.ground;
    act(() => root.render(<MapProbe refs={refs} queue={enqueue(enqueue(null, 5), 5)} />));
    await flush();
    expect(h.ground).toBe(kept);
    built.buildings.dispose();
  });
});
