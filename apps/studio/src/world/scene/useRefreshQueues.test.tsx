// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import * as THREE from "three";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { errorText, getMapModelGlb, mapModelTextureStatus } from "../../api";
import type { MatrixDto, TextureIssue } from "../../api";
import { enqueue } from "../../reload";
import type { RefreshQueue } from "../../reload";
import { textureNote } from "../load";
import { parseGlb } from "../sceneUtil";
import type { SceneHost } from "./SceneHost";
import type { SceneRefs } from "./types";
import { useMapModelRefresh } from "./useRefreshQueues";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
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
      [5, {}],
      [2, {}],
    ]),
    content: new THREE.Group(),
    models: [],
    queueRender: vi.fn(),
    disposed: false,
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
    const { refs } = fakeScene();
    act(() => root.render(<Probe refs={refs} queue={enqueue(null, 9)} />));
    await flush();
    expect(getMapModelGlb).not.toHaveBeenCalled();
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
});
