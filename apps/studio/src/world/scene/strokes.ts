import { szene } from "../../ui/tokens/tokens";
import { MAP_SIZE } from "../layout";
import { lineCells, rectCells } from "../paint/ops";
import type { Cell, PaintSub } from "../paint/ops";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";
import { clearStrokePreview, holdStrokePreview, setStrokePreview, setStrokeRect } from "./strokePreview";
import type { PreviewLook } from "./strokePreview";
import type { SceneRefs } from "./types";

/**
 * One stroke on a world map, as the pointer drew it: map-local cells in stroke order, each once (rectangle: the cells between its two
 * corners; fill: the clicked cell), `lower` with ⇧ held at its start. `preview` is the id the stroke's plates are kept under until its
 * answer (`clearStrokePreview(h, preview)`).
 */
export interface Stroke {
  map: number;
  sub: PaintSub;
  cells: Cell[];
  lower: boolean;
  preview?: number;
}

/** What strokes report to the paint session; `sub` null while no paint sub-tool paints a world map. */
export interface PaintCallbacks {
  sub: PaintSub | null;
  target: number | null;
  onStroke(stroke: Stroke): void;
  onPick(map: number, at: Cell): void;
  onSelectCell(cell: { col: number; row: number }): void;
  /** How the plates of a stroke of `sub` look (the tile's texture or placeholder, the colour of the target step); left out: a neutral look. */
  look?(sub: PaintSub, lower: boolean): PreviewLook;
}

/** A stroke being drawn: where it started (its map's matrix cell) and the cells so far. */
interface Drawing {
  pointerId: number;
  map: number;
  col: number;
  row: number;
  sub: PaintSub;
  lower: boolean;
  start: Cell;
  /** The last cell the pointer was over (may lie outside the map: the next line starts there). */
  last: Cell;
  /** The rectangle's other corner, inside the map. */
  end: Cell;
  cells: Cell[];
  seen: Set<number>;
  look: PreviewLook;
}

const inside = ([x, z]: Cell) => x >= 0 && x < MAP_SIZE && z >= 0 && z < MAP_SIZE;
const clamp = (v: number) => Math.min(MAP_SIZE - 1, Math.max(0, v));
/** Neutral plates when the session gives no look: hatched for removing, else the colour of an unknown tile. */
const fallbackLook = (sub: PaintSub, lower: boolean): PreviewLook => (sub === "erase" || (sub === "ramp" && lower) ? { erase: true } : { color: szene.gelaende.unbekannt });

/**
 * Pointer strokes on the canvas while `refs.callbacks.current.paint.sub` is set: cells of the target map only, each once,
 * gaps filled by `lineCells`; `rect` reports its two corners' cells (`rectCells`), `fill` and `pick` the clicked cell;
 * right button picks; pointercancel (and a lost pointer capture) drops the stroke. Draws the preview while the pointer moves.
 *
 * The pointer goes down in the capture phase, before the camera controls see it: the left button is then taken from them
 * (`setPaintPointer`), with ⌥ held it turns or pans the camera instead of painting. A stroke keeps the pointer (pointer capture) and
 * stops the press from reaching the drags of buildings and events.
 */
export function attachStrokes(h: SceneHost, picker: Picker, refs: SceneRefs): () => void {
  const canvas = h.canvas;
  let drawing: Drawing | null = null;

  /** The cell under the pointer, local to the matrix cell the stroke began in (outside 0 to 31 over a neighbour), or null beside the matrix. */
  const cellAt = (d: Drawing, clientX: number, clientY: number): Cell | null => {
    const picked = picker.pickTile(clientX, clientY);
    if (picked && picked.col === d.col && picked.row === d.row) return [picked.x, picked.y];
    const point = picker.surfacePoint(clientX, clientY);
    if (!point) return null;
    return [Math.floor(point.x) - d.col * MAP_SIZE, Math.floor(point.z) - d.row * MAP_SIZE];
  };

  const add = (d: Drawing, cell: Cell) => {
    if (!inside(cell)) return;
    const id = cell[1] * MAP_SIZE + cell[0];
    if (d.seen.has(id)) return;
    d.seen.add(id);
    d.cells.push(cell);
  };

  const draw = (d: Drawing) => {
    if (d.sub === "rect") {
      setStrokePreview(h, d.map, rectCells(d.start, d.end), d.look);
      setStrokeRect(h, d.map, d.start, d.end);
    } else if (d.sub !== "fill") {
      setStrokePreview(h, d.map, d.cells, d.look);
    }
    h.render();
  };

  /** Follows the pointer to `cell`: the rectangle moves its corner, the other sub-tools add the line from the last cell. */
  const follow = (d: Drawing, cell: Cell) => {
    if (d.sub === "fill") return;
    if (d.sub === "rect") {
      d.end = [clamp(cell[0]), clamp(cell[1])];
      return;
    }
    for (const c of lineCells(d.last, cell)) add(d, c);
    d.last = cell;
  };

  const onDown = (e: PointerEvent) => {
    const cb = refs.callbacks.current;
    const paint = cb.paint;
    const active = paint.sub !== null && paint.target !== null && cb.paintEnabled;
    h.setPaintPointer(active && !e.altKey);
    if (!active || e.altKey || drawing || (e.button !== 0 && e.button !== 2)) return;
    const picked = picker.pickTile(e.clientX, e.clientY);
    if (!picked) return;
    e.stopPropagation();
    if (picked.mapId !== paint.target) {
      if (e.button === 0) paint.onSelectCell({ col: picked.col, row: picked.row });
      return;
    }
    const at: Cell = [picked.x, picked.y];
    const sub = paint.sub as PaintSub;
    if (e.button === 2 || sub === "pick") {
      paint.onPick(picked.mapId, at);
      return;
    }
    canvas.setPointerCapture?.(e.pointerId);
    const lower = e.shiftKey === true;
    const d: Drawing = {
      pointerId: e.pointerId,
      map: picked.mapId,
      col: picked.col,
      row: picked.row,
      sub,
      lower,
      start: at,
      last: at,
      end: at,
      cells: [],
      seen: new Set(),
      look: paint.look?.(sub, lower) ?? fallbackLook(sub, lower),
    };
    add(d, at);
    drawing = d;
    draw(d);
  };

  const onMove = (e: PointerEvent) => {
    const d = drawing;
    if (!d || e.pointerId !== d.pointerId) return;
    const cell = cellAt(d, e.clientX, e.clientY);
    if (!cell) return;
    follow(d, cell);
    draw(d);
  };

  const onUp = (e: PointerEvent) => {
    const d = drawing;
    if (!d || e.pointerId !== d.pointerId) return;
    const cell = cellAt(d, e.clientX, e.clientY);
    if (cell) follow(d, cell);
    drawing = null;
    if (canvas.hasPointerCapture?.(e.pointerId)) canvas.releasePointerCapture(e.pointerId);
    const cells = d.sub === "rect" ? rectCells(d.start, d.end) : d.sub === "fill" ? [d.start] : d.cells;
    if (d.sub === "rect") setStrokePreview(h, d.map, cells, d.look);
    const preview = holdStrokePreview(h);
    h.render();
    refs.callbacks.current.paint.onStroke({ map: d.map, sub: d.sub, cells, lower: d.lower, preview });
  };

  const onCancel = (e: PointerEvent) => {
    const d = drawing;
    if (!d || e.pointerId !== d.pointerId) return;
    drawing = null;
    clearStrokePreview(h);
    h.render();
  };

  canvas.addEventListener("pointerdown", onDown, true);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  canvas.addEventListener("pointercancel", onCancel);
  // Losing the pointer otherwise (another element took it, the window lost it) ends the stroke like a cancel; after a release the
  // stroke is already gone.
  canvas.addEventListener("lostpointercapture", onCancel);
  return () => {
    canvas.removeEventListener("pointerdown", onDown, true);
    canvas.removeEventListener("pointermove", onMove);
    canvas.removeEventListener("pointerup", onUp);
    canvas.removeEventListener("pointercancel", onCancel);
    canvas.removeEventListener("lostpointercapture", onCancel);
    if (drawing) clearStrokePreview(h);
    drawing = null;
  };
}
