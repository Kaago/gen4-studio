import { useCallback, useEffect, useRef, useState } from "react";
import { tilesetGet, worldEdit, worldGet } from "../../api.world";
import type { TilesetDto, WorldEditRequest, WorldGetDto } from "../../api.world";
import { usePaint } from "../../workbench/paint/PaintContext";
import type { PaintState } from "../../workbench/paint/PaintContext";
import { heightColor } from "../scene/heightLayer";
import type { SceneHost } from "../scene/SceneHost";
import { clearStrokePreview, holdStrokePreview, setStrokePreview } from "../scene/strokePreview";
import type { PreviewLook } from "../scene/strokePreview";
import type { Stroke } from "../scene/strokes";
import { placeholderColor } from "../scene/worldMesh";
import { HEIGHT_MAX, HEIGHT_MIN, cellsOf, fillCells, heightSkipped, pickedTile, strokeRequest } from "./ops";
import type { Cell, PaintSub, WorldCells } from "./ops";
import { createStrokeQueue } from "./session";
import type { StrokeQueue } from "./session";

/** World units per height step (the steps of the world document, half a tile each). */
const UNITS_PER_STEP = 8;

/** What else the session needs from its scene: the host its previews live on, and where a note (not an error) goes. */
export interface PaintSessionOptions {
  scene?: () => SceneHost | null;
  onNotice?: (text: string) => void;
}

/** `cells` with the changes of `req` applied (a copy): what the world will be once a stroke on its way is answered. */
function applyRequest(cells: WorldCells, req: WorldEditRequest): WorldCells {
  const next: WorldCells = { tiles: cells.tiles.map((layer) => layer.map((r) => [...r])), heights: cells.heights.map((r) => [...r]), ramps: cells.ramps.map((r) => [...r]) };
  for (const change of req.changes) {
    for (const [x, z] of change.cells) {
      if (change.tile !== undefined) {
        const layer = next.tiles[change.layer ?? 0];
        if (layer?.[z]) layer[z][x] = change.tile;
      }
      if (change.height !== undefined && next.heights[z]) next.heights[z][x] = change.height;
      if (change.ramp !== undefined && next.ramps[z]) next.ramps[z][x] = change.ramp;
    }
  }
  return next;
}

/** The text of a height stroke whose cells are already at the limit: „2 Zellen schon an der Grenze (−16 bis 62).“ */
function limitNotice(n: number): string {
  return `${n === 1 ? "1 Zelle" : `${n} Zellen`} schon an der Grenze (${String(HEIGHT_MIN).replace("-", "−")} bis ${HEIGHT_MAX}).`;
}

/** The colour of height step `step` (clamped to the range) as the layer „Höhen“ shows it. */
const stepColor = (step: number) => heightColor(Math.min(HEIGHT_MAX, Math.max(HEIGHT_MIN, step)) * UNITS_PER_STEP);

/**
 * The look of a stroke's plates: brush, rectangle and fill the placeholder colour of the tile's role (the palette's pictures come with
 * its store), the eraser and removing a ramp hatched, height and ramp the colour of the step each cell goes to.
 */
function strokeLook(sub: PaintSub, lower: boolean, state: PaintState, cells: WorldCells | null, tileset: TilesetDto | null): PreviewLook {
  if (sub === "erase" || (sub === "ramp" && lower)) return { erase: true };
  const height = ([x, z]: Cell) => cells?.heights[z]?.[x] ?? 0;
  if (sub === "height") return { color: (cell) => stepColor(height(cell) + (lower ? -1 : 1)) };
  if (sub === "ramp") return { color: (cell) => stepColor(height(cell) + state.ramp.steps) };
  const index = Number.parseInt(state.tile.slice(0, 4), 16);
  return { color: placeholderColor(tileset?.tileset.tiles[index]?.role ?? "ground") };
}

/** The open paint session of `target`: its world (world_get, read again on every revision), tileset (tileset_get), queue;
 * `onStroke` builds the request, pushes it, then refreshes mesh, heights and world and clears the preview.
 *
 * The request of a stroke is built when it is drawn, from the world read last with the strokes still on their way applied, so a fast
 * second stroke (a second + on the same cells) builds on the first. A refused stroke loses its preview, reports through `onError` and the
 * world is read again. A height stroke whose cells are all at the limit sends nothing; the cells at the limit are named (`onNotice`). */
export function usePaintSession(
  target: number | null,
  revision: number,
  refresh: (map: number) => void,
  onError: (e: unknown) => void,
  options: PaintSessionOptions = {},
): { world: WorldGetDto | null; tileset: TilesetDto | null; onStroke(stroke: Stroke): void; onPick(map: number, at: Cell): void; pending: number; look(sub: PaintSub, lower: boolean): PreviewLook } {
  const paint = usePaint();
  const [world, setWorld] = useState<WorldGetDto | null>(null);
  const [tileset, setTileset] = useState<TilesetDto | null>(null);
  const [pending, setPending] = useState(0);
  const [queue] = useState<StrokeQueue>(() => createStrokeQueue(worldEdit, setPending));
  // What the callbacks read when they run: the latest props, the world read last and the requests on their way.
  const latest = useRef({ target, refresh, onError, options, state: paint.state, set: paint.set, tileset });
  latest.current = { target, refresh, onError, options, state: paint.state, set: paint.set, tileset };
  const read = useRef<WorldGetDto | null>(null);
  const inFlight = useRef<WorldEditRequest[]>([]);

  // Reads are numbered: an answer older than the last one asked for is dropped (a slow read must not undo a newer one).
  const reads = useRef(0);
  const readWorld = useCallback(async (map: number) => {
    const seq = ++reads.current;
    const dto = await worldGet(map);
    if (latest.current.target === map && seq === reads.current) {
      read.current = dto;
      setWorld(dto);
    }
    return dto;
  }, []);

  useEffect(() => {
    read.current = null;
    setWorld(null);
  }, [target]);
  // Opening the session and every later revision (a stroke answered, ⌘Z, a world_edit over MCP) read the world again.
  useEffect(() => {
    if (target !== null) readWorld(target).catch((e) => latest.current.onError(e));
  }, [target, revision, readWorld]);

  const pkg = world?.world?.tileset ?? null;
  useEffect(() => {
    if (pkg === null) {
      setTileset(null);
      return;
    }
    let live = true;
    tilesetGet(pkg).then(
      (dto) => live && setTileset(dto),
      () => live && setTileset(null),
    );
    return () => {
      live = false;
    };
  }, [pkg]);

  /** The cells of `map` as they will be once the strokes on their way are answered; null without its world. */
  const cellsFor = (map: number): WorldCells | null => {
    const dto = read.current;
    if (!dto?.world || dto.map !== map) return null;
    return inFlight.current.filter((req) => req.map === map).reduce(applyRequest, cellsOf(dto.world));
  };

  const onStroke = useCallback((stroke: Stroke) => {
    const { state, options: opts } = latest.current;
    const host = opts.scene?.() ?? null;
    const previews = stroke.preview === undefined ? [] : [stroke.preview];
    const clear = () => {
      if (host) for (const id of previews) clearStrokePreview(host, id);
      host?.render?.();
    };
    const cells = cellsFor(stroke.map);
    if (!cells || stroke.sub === "pick") {
      clear();
      return;
    }
    let strokeCells = stroke.cells;
    if (stroke.sub === "fill") {
      strokeCells = stroke.cells[0] ? fillCells(cells, state.layer, stroke.cells[0]) : [];
      if (host) {
        setStrokePreview(host, stroke.map, strokeCells, strokeLook("fill", false, state, cells, latest.current.tileset));
        previews.push(holdStrokePreview(host));
        host.render?.();
      }
    }
    const built = strokeRequest(stroke.map, stroke.sub, strokeCells, cells, { layer: state.layer, tile: state.tile, ramp: state.ramp, lower: stroke.lower });
    const skipped = stroke.sub === "height" ? heightSkipped(strokeCells, cells, stroke.lower) : 0;
    if (skipped > 0) opts.onNotice?.(limitNotice(skipped));
    if (!built) {
      clear();
      return;
    }
    const req = built.req;
    inFlight.current.push(req);
    const settled = () => {
      inFlight.current = inFlight.current.filter((r) => r !== req);
    };
    queue.push(req).then(
      async () => {
        settled();
        latest.current.refresh(req.map);
        await readWorld(req.map).catch((e) => latest.current.onError(e));
        clear();
      },
      async (e) => {
        settled();
        clear();
        latest.current.onError(e);
        await readWorld(req.map).catch(() => undefined);
      },
    );
  }, []);

  const onPick = useCallback((map: number, at: Cell) => {
    const cells = cellsFor(map);
    if (!cells) return;
    const tile = pickedTile(cells, latest.current.state.layer, at);
    if (tile === null) return;
    // The pipette goes back to the brush; a right click with another sub-tool keeps it.
    latest.current.set((s) => (s.sub === "pick" ? { tile, sub: "brush" } : { tile }));
  }, []);

  const look = useCallback((sub: PaintSub, lower: boolean) => {
    const dto = read.current;
    return strokeLook(sub, lower, latest.current.state, dto?.world ? cellsFor(dto.map) : null, latest.current.tileset);
  }, []);

  return { world, tileset, onStroke, onPick, pending, look };
}
