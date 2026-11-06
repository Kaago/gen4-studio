// Test helpers for the drag factories: a fake pointer surface and a stand-in picker. Only tests import this.
import * as THREE from "three";
import { vi } from "vitest";
import type { MatrixDto } from "../../api";
import type { DragSurface, PointerLike } from "../dragController";
import { createGround, flatGround } from "../ground";
import { createPicker } from "./picking";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import type { Loaded } from "./types";
import type { SceneCallbacks, SceneRefs } from "./types";

type Listener = (e: Event) => void;

/** An event target that remembers listeners per type and phase, with pointer capture. */
export class FakeTarget {
  listeners: { type: string; fn: Listener; capture: boolean }[] = [];
  captured = new Set<number>();
  addEventListener(type: string, fn: Listener, capture?: boolean) {
    this.listeners.push({ type, fn, capture: capture === true });
  }
  removeEventListener(type: string, fn: Listener, capture?: boolean) {
    this.listeners = this.listeners.filter((l) => !(l.type === type && l.fn === fn && l.capture === (capture === true)));
  }
  setPointerCapture(id: number) {
    this.captured.add(id);
  }
  hasPointerCapture(id: number) {
    return this.captured.has(id);
  }
  releasePointerCapture(id: number) {
    this.captured.delete(id);
  }
  emit(type: string, e: unknown) {
    const mine = this.listeners.filter((l) => l.type === type);
    for (const l of [...mine.filter((x) => x.capture), ...mine.filter((x) => !x.capture)]) {
      if ((e as { immediateStopped: boolean }).immediateStopped) return;
      l.fn(e as Event);
    }
  }
}

export class FakePointer implements PointerLike {
  immediateStopped = false;
  constructor(
    public clientX: number,
    public clientY: number,
    public button = 0,
    public pointerId = 1,
  ) {}
  stopPropagation() {}
  stopImmediatePropagation() {
    this.immediateStopped = true;
  }
  preventDefault() {}
}

export class FakeKey {
  constructor(public key: string) {}
  stopPropagation() {}
  stopImmediatePropagation() {}
  preventDefault() {}
}

/** The fake surface and the gestures on it; one pixel is one tile on the plane (see `picker`). */
export function surfaceKit() {
  const host = new FakeTarget();
  const canvas = new FakeTarget();
  const keys = new FakeTarget();
  const surface: DragSurface = { host, canvas, keys };
  return {
    surface,
    press: (x: number, y: number) => {
      const e = new FakePointer(x, y);
      host.emit("pointerdown", e);
      return e;
    },
    moveTo: (x: number, y: number) => canvas.emit("pointermove", new FakePointer(x, y)),
    release: (x: number, y: number) => canvas.emit("pointerup", new FakePointer(x, y)),
    escape: () => keys.emit("keydown", new FakeKey("Escape")),
    canvas,
  };
}

/** A picker that hits nothing by default; the ground plane maps pixels to tiles one to one. */
export function fakePicker(): Picker {
  return {
    raycaster: {} as Picker["raycaster"],
    aim: vi.fn(),
    markerAt: vi.fn(() => null),
    pickTile: vi.fn(() => null),
    surfacePoint: vi.fn(() => null),
    planePoint: (x, y) => ({ x, z: y }),
  };
}

/** The refs the factories read, with every callback a spy. */
export function fakeRefs(overrides: Partial<SceneCallbacks> = {}) {
  const callbacks: SceneCallbacks = {
    paintEnabled: true,
    tool: "select",
    onTileClick: vi.fn(),
    onBuildings: vi.fn(),
    onEdited: vi.fn(),
    onError: vi.fn(),
    onInspect: vi.fn(),
    onStatus: vi.fn(),
    onSelectBuilding: vi.fn(),
    onBuildingMoved: vi.fn(),
    onSelectEvent: vi.fn(),
    onSelectCell: vi.fn(),
    onEventMoved: vi.fn(),
    onEventsLocked: vi.fn(),
    ...overrides,
  };
  const refs = {
    handle: { current: null },
    callbacks: { current: callbacks },
    showPermissions: { current: true },
    showEvents: { current: true },
    eventFile: { current: 1 },
    destName: { current: () => undefined },
    area: { current: 0 },
  } as SceneRefs;
  return { refs, callbacks };
}

/** Lets promise callbacks run. */
export const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

const VIEW_SIZE = 800;

/** Tile heights in world units per map id, `MapDto.heights` of the maps a test view loads (missing: none). */
export type TestHeights = Record<number, (number | null)[]>;

/**
 * A scene handle with a real picker that looks through `camera` at `matrix` (every map in it loaded, with `heights`, and its ground
 * made from them); `through` gives the client position whose view ray passes through a scene point. The canvas is a real element.
 */
export function viewThrough(camera: THREE.Camera, matrix: MatrixDto | null, heights: TestHeights = {}) {
  camera.updateMatrixWorld(true);
  if (camera instanceof THREE.PerspectiveCamera || camera instanceof THREE.OrthographicCamera) camera.updateProjectionMatrix();
  const canvas = document.createElement("canvas");
  canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: VIEW_SIZE, height: VIEW_SIZE }) as DOMRect;
  const loaded = new Map<number, Loaded>((matrix?.maps ?? []).flatMap((id) => (id === null ? [] : [[id, { map: { index: id, permissions: [], buildings: [], heights: heights[id] ?? [] } } as Loaded] as const])));
  const ground = matrix ? createGround(matrix, loaded) : flatGround(null);
  const h = { canvas, camera, matrix, loaded, ground } as unknown as SceneHost;
  const through = (x: number, y: number, z: number) => {
    const p = new THREE.Vector3(x, y, z).project(camera);
    return { x: ((p.x + 1) / 2) * VIEW_SIZE, y: ((1 - p.y) / 2) * VIEW_SIZE };
  };
  return { h, picker: createPicker(h), through };
}

/**
 * A scene handle with a real picker and a slanted perspective camera over `matrix` (every map in it
 * loaded); `through` gives the client position whose view ray passes through a scene point.
 */
export function slantedView(matrix: MatrixDto | null, heights: TestHeights = {}) {
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
  camera.position.set(45, 30, 20);
  camera.lookAt(32, 0, 5);
  return viewThrough(camera, matrix, heights);
}
