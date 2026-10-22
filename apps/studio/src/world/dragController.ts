import { planeDrag } from "./drag";
import { isClick } from "./paint";

export interface Point {
  x: number;
  z: number;
}

/** The parts of a pointer event the controller reads. */
export interface PointerLike {
  button: number;
  clientX: number;
  clientY: number;
  pointerId: number;
  stopPropagation(): void;
  stopImmediatePropagation(): void;
  preventDefault(): void;
}

/** What `begin` returns for a press on something draggable. */
export interface DragStart<S, P> {
  /** Whatever the owner needs to preview, commit and cancel the move (the dragged thing, its values at the press). */
  subject: S;
  /** The plane the pointer moves the subject on. */
  plane: P;
  /** The subject's position on that plane (scene units) at the press. */
  origin: Point;
}

export interface DragHooks<S, P> {
  /** A press with the primary button: the thing to drag, or null to leave the press to everybody else. */
  begin(e: PointerLike): DragStart<S, P> | null;
  /** The point of `plane` under a pointer position, or null when the view ray never reaches it. */
  pointOn(clientX: number, clientY: number, plane: P): Point | null;
  /** Optional: where the subject may be at all. `at` (snapped) is turned into the nearest allowed position before it is previewed or committed. */
  clamp?(at: Point): Point;
  /** The subject is dragged to `at` (snapped, scene units): preview it there. */
  move(subject: S, at: Point): void;
  /** The pointer was released after the subject was moved to `at`: write it. Not called for a drag that ends where it began. */
  commit(subject: S, at: Point): void;
  /** The drag ended without a change (no movement, back at the start, Escape, the pointer was taken away): show the subject as it was. */
  cancel(subject: S): void;
}

/** An element events can be listened on. */
interface Listenable {
  addEventListener(type: string, listener: (e: Event) => void, capture?: boolean): void;
  removeEventListener(type: string, listener: (e: Event) => void, capture?: boolean): void;
}

/** Where the controller listens: the element that sees the press first (capture phase), and the canvas. */
export interface DragSurface {
  host: Listenable;
  canvas: Listenable & {
    setPointerCapture(id: number): void;
    hasPointerCapture(id: number): boolean;
    releasePointerCapture(id: number): void;
  };
  /** Where the Escape key that cancels a drag is heard (the window). */
  keys: Listenable;
}

interface Active<S, P> {
  subject: S;
  plane: P;
  start: Point;
  origin: Point;
  downAt: { x: number; y: number };
  pointerId: number;
  moved: boolean;
  latest: Point;
}

/**
 * The pointer interaction shared by everything that is dragged on the ground plane: the press is
 * taken in the capture phase, before OrbitControls sees it (the camera stays still), the pointer
 * is captured, and nothing counts as a drag until the pointer has travelled a few pixels (a press
 * that does not is still a click). The subject follows the pointer in steps of `snap` tiles
 * (previewed through `move`); on release one `commit` reports the final position. A drag that
 * ends where it began, is cancelled with Escape, or loses its pointer capture is `cancel`led.
 */
export class DragController<S, P> {
  private active: Active<S, P> | null = null;

  constructor(
    private readonly surface: DragSurface,
    private readonly snap: number,
    private readonly hooks: DragHooks<S, P>,
  ) {}

  /** Whether a drag is in progress (the press has been taken, whether or not the pointer has moved). */
  get dragging(): boolean {
    return this.active !== null;
  }

  /** Starts listening; the returned function stops it (an unfinished drag is dropped without a word). */
  attach(): () => void {
    const { host, canvas, keys } = this.surface;
    const down = (e: Event) => this.onDown(e as PointerEvent);
    const move = (e: Event) => this.onMove(e as PointerEvent);
    const up = (e: Event) => this.onUp(e as PointerEvent);
    const lost = () => this.abort();
    const key = (e: Event) => this.onKey(e as KeyboardEvent);
    host.addEventListener("pointerdown", down, true);
    canvas.addEventListener("pointermove", move);
    // Capture phase: the release that ends a drag is never also a click for the listeners after this one.
    canvas.addEventListener("pointerup", up, true);
    canvas.addEventListener("lostpointercapture", lost);
    // Capture phase: Escape is heard here before the app's own handlers, which this key press must not reach.
    keys.addEventListener("keydown", key, true);
    return () => {
      host.removeEventListener("pointerdown", down, true);
      canvas.removeEventListener("pointermove", move);
      canvas.removeEventListener("pointerup", up, true);
      canvas.removeEventListener("lostpointercapture", lost);
      keys.removeEventListener("keydown", key, true);
      // The scene is going away: there is nothing left to show the subject in.
      this.active = null;
    };
  }

  /** Abandons the drag in progress: the subject is shown as it was and nothing is committed. */
  cancel() {
    const d = this.active;
    if (!d) return;
    this.active = null;
    if (this.surface.canvas.hasPointerCapture(d.pointerId)) this.surface.canvas.releasePointerCapture(d.pointerId);
    this.hooks.cancel(d.subject);
  }

  private onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const begun = this.hooks.begin(e);
    if (!begun) return;
    const start = this.hooks.pointOn(e.clientX, e.clientY, begun.plane);
    if (!start) return;
    e.stopPropagation();
    e.preventDefault();
    this.surface.canvas.setPointerCapture(e.pointerId);
    this.active = {
      subject: begun.subject,
      plane: begun.plane,
      start,
      origin: begun.origin,
      downAt: { x: e.clientX, y: e.clientY },
      pointerId: e.pointerId,
      moved: false,
      latest: begun.origin,
    };
  }

  private onMove(e: PointerEvent) {
    const d = this.active;
    // A press that has not travelled yet is still a click.
    if (!d || (!d.moved && isClick(d.downAt, { x: e.clientX, y: e.clientY }))) return;
    const at = this.hooks.pointOn(e.clientX, e.clientY, d.plane);
    if (!at) return;
    d.moved = true;
    const snapped = planeDrag(d.start, at, d.origin, this.snap);
    d.latest = this.hooks.clamp ? this.hooks.clamp(snapped) : snapped;
    this.hooks.move(d.subject, d.latest);
  }

  private onUp(e: PointerEvent) {
    const d = this.active;
    if (!d || e.pointerId !== d.pointerId) return;
    this.active = null;
    e.stopImmediatePropagation();
    if (this.surface.canvas.hasPointerCapture(d.pointerId)) this.surface.canvas.releasePointerCapture(d.pointerId);
    if (!d.moved || (d.latest.x === d.origin.x && d.latest.z === d.origin.z)) this.hooks.cancel(d.subject);
    else this.hooks.commit(d.subject, d.latest);
  }

  /** The browser took the pointer away (cancel, window switch): the move was never confirmed. */
  private abort() {
    const d = this.active;
    if (!d) return;
    this.active = null;
    this.hooks.cancel(d.subject);
  }

  private onKey(e: KeyboardEvent) {
    if (e.key !== "Escape" || !this.active) return;
    // The drag is what Escape ends; nothing else (a pin, a field) reacts to this press.
    e.stopPropagation();
    e.preventDefault();
    this.cancel();
  }
}
