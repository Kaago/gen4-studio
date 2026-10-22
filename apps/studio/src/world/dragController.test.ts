import { describe, expect, it } from "vitest";
import { DragController } from "./dragController";
import type { DragHooks, DragSurface, PointerLike } from "./dragController";

type Listener = (e: Event) => void;

/** An event target that remembers listeners per type and phase. */
class FakeTarget {
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
    // Capture listeners first, as at a target in current browsers; stopImmediatePropagation ends the walk.
    const mine = this.listeners.filter((l) => l.type === type);
    for (const l of [...mine.filter((x) => x.capture), ...mine.filter((x) => !x.capture)]) {
      if ((e as { immediateStopped: boolean }).immediateStopped) return;
      l.fn(e as Event);
    }
  }
}

class FakePointer implements PointerLike {
  stopped = false;
  immediateStopped = false;
  prevented = false;
  constructor(
    public clientX: number,
    public clientY: number,
    public button = 0,
    public pointerId = 1,
  ) {}
  stopPropagation() {
    this.stopped = true;
  }
  stopImmediatePropagation() {
    this.immediateStopped = true;
  }
  preventDefault() {
    this.prevented = true;
  }
}

class FakeKey {
  stopped = false;
  immediateStopped = false;
  prevented = false;
  constructor(public key: string) {}
  stopPropagation() {
    this.stopped = true;
  }
  stopImmediatePropagation() {
    this.immediateStopped = true;
  }
  preventDefault() {
    this.prevented = true;
  }
}

/** One pixel is one tile on the plane; the subject starts at (10, 10) and can be grabbed anywhere. */
function setup(grab = true, clamp?: DragHooks<string, "ground">["clamp"]) {
  const host = new FakeTarget();
  const canvas = new FakeTarget();
  const keys = new FakeTarget();
  const calls: string[] = [];
  const hooks: DragHooks<string, "ground"> = {
    begin: () => (grab ? { subject: "thing", plane: "ground", origin: { x: 10, z: 10 } } : null),
    pointOn: (x, y) => ({ x, z: y }),
    move: (_s, at) => calls.push(`move ${at.x},${at.z}`),
    commit: (_s, at) => calls.push(`commit ${at.x},${at.z}`),
    cancel: () => calls.push("cancel"),
    clamp,
  };
  const surface: DragSurface = { host, canvas, keys };
  const controller = new DragController(surface, 1, hooks);
  const detach = controller.attach();
  const press = (x: number, y: number, button = 0) => {
    const e = new FakePointer(x, y, button);
    host.emit("pointerdown", e);
    return e;
  };
  const moveTo = (x: number, y: number) => canvas.emit("pointermove", new FakePointer(x, y));
  const release = (x: number, y: number) => {
    const e = new FakePointer(x, y);
    canvas.emit("pointerup", e);
    return e;
  };
  const escape = () => {
    const e = new FakeKey("Escape");
    keys.emit("keydown", e);
    return e;
  };
  return { host, canvas, keys, escape, calls, controller, detach, press, moveTo, release };
}

describe("DragController", () => {
  it("takes the press away from the camera and captures the pointer", () => {
    const t = setup();
    const e = t.press(100, 100);
    expect(e.stopped && e.prevented).toBe(true);
    expect(t.canvas.hasPointerCapture(1)).toBe(true);
    expect(t.controller.dragging).toBe(true);
  });

  it("leaves a press alone that nothing wants, or that is not the primary button", () => {
    const t = setup(false);
    expect(t.press(100, 100).stopped).toBe(false);
    expect(t.controller.dragging).toBe(false);
    const t2 = setup();
    expect(t2.press(100, 100, 2).stopped).toBe(false);
    expect(t2.controller.dragging).toBe(false);
  });

  it("is still a click until the pointer travelled, then follows it in snapped steps", () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(102, 101);
    expect(t.calls).toEqual([]);
    t.moveTo(104.4, 98.6);
    expect(t.calls).toEqual(["move 14,9"]);
  });

  it("commits once on release, with the last position, and releases the pointer", () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(105, 100);
    t.moveTo(106.2, 100);
    const up = t.release(106.2, 100);
    expect(t.calls).toEqual(["move 15,10", "move 16,10", "commit 16,10"]);
    expect(t.canvas.hasPointerCapture(1)).toBe(false);
    expect(t.controller.dragging).toBe(false);
    // The release belongs to the drag: later listeners do not see it as a click.
    expect(up.immediateStopped).toBe(true);
  });

  it("commits nothing for a press without movement or a drag that ends where it began", () => {
    const t = setup();
    t.press(100, 100);
    t.release(101, 100);
    expect(t.calls).toEqual(["cancel"]);
    const t2 = setup();
    t2.press(100, 100);
    t2.moveTo(110, 100);
    t2.moveTo(100.3, 100);
    t2.release(100.3, 100);
    expect(t2.calls).toEqual(["move 20,10", "move 10,10", "cancel"]);
  });

  it("does not touch a release it did not start", () => {
    const t = setup();
    const up = t.release(1, 1);
    expect(up.immediateStopped).toBe(false);
    expect(t.calls).toEqual([]);
  });

  it("cancels when the browser takes the pointer away, and writes nothing afterwards", () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(110, 100);
    t.canvas.captured.clear();
    t.canvas.emit("lostpointercapture", {});
    expect(t.calls).toEqual(["move 20,10", "cancel"]);
    t.release(110, 100);
    expect(t.calls).toEqual(["move 20,10", "cancel"]);
  });

  it("cancels on Escape without a write, keeps that key from everyone else, and the release that follows is nobody's click", () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(110, 100);
    const key = t.escape();
    expect(t.calls).toEqual(["move 20,10", "cancel"]);
    expect(key.stopped && key.prevented).toBe(true);
    expect(t.canvas.hasPointerCapture(1)).toBe(false);
    expect(t.controller.dragging).toBe(false);
    // The pointer is still down: moving on and releasing writes nothing.
    t.moveTo(120, 100);
    t.release(120, 100);
    expect(t.calls).toEqual(["move 20,10", "cancel"]);
  });

  it("cancels on Escape before the pointer moved too", () => {
    const t = setup();
    t.press(100, 100);
    t.escape();
    expect(t.calls).toEqual(["cancel"]);
  });

  it("leaves Escape alone when nothing is being dragged, and other keys during a drag", () => {
    const t = setup();
    expect(t.escape().stopped).toBe(false);
    t.press(100, 100);
    const other = new FakeKey("a");
    t.keys.emit("keydown", other);
    expect(other.stopped).toBe(false);
    expect(t.controller.dragging).toBe(true);
  });

  it("keeps the subject where the owner lets it be", () => {
    // Tiles 0 to 12 only: dragging beyond stops at the edge, and the commit is for the edge.
    const t = setup(true, (at) => ({ x: Math.min(at.x, 12), z: at.z }));
    t.press(100, 100);
    t.moveTo(120, 100);
    t.release(120, 100);
    expect(t.calls).toEqual(["move 12,10", "commit 12,10"]);
  });

  it("stops listening when detached", () => {
    const t = setup();
    t.detach();
    expect(t.host.listeners).toEqual([]);
    expect(t.canvas.listeners).toEqual([]);
    expect(t.keys.listeners).toEqual([]);
  });
});
