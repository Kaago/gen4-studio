// Test helpers for the drag factories: a fake pointer surface and a stand-in picker. Only tests import this.
import { vi } from "vitest";
import type { DragSurface, PointerLike } from "../dragController";
import type { Picker } from "./picking";
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
    planePoint: (x, y) => ({ x, z: y }),
  };
}

/** The refs the factories read, with every callback a spy. */
export function fakeRefs(overrides: Partial<SceneCallbacks> = {}) {
  const callbacks: SceneCallbacks = {
    paintEnabled: true,
    onEdited: vi.fn(),
    onError: vi.fn(),
    onInspect: vi.fn(),
    onStatus: vi.fn(),
    onSelectBuilding: vi.fn(),
    onBuildingMoved: vi.fn(),
    onSelectEvent: vi.fn(),
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
    headers: { current: [] },
    area: { current: 0 },
  } as SceneRefs;
  return { refs, callbacks };
}

/** Lets promise callbacks run. */
export const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
