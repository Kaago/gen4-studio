import { useCallback, useEffect, useRef, useState } from "react";
import { tilesetGet, worldEdit, worldGet } from "../../api.world";
import type { TilesetDto, WorldEditRequest, WorldGetDto } from "../../api.world";
import { t } from "../../i18n";
import { usePaint } from "../../workbench/paint/PaintContext";
import type { PaintState } from "../../workbench/paint/PaintContext";
import { heightColor } from "../scene/heightLayer";
import type { SceneHost } from "../scene/SceneHost";
import { clearStrokePreview, holdStrokePreview, setStrokePreview } from "../scene/strokePreview";
import type { PreviewLook } from "../scene/strokePreview";
import type { Stroke } from "../scene/strokes";
import { placeholderColor } from "../scene/worldMesh";
import { HEIGHT_MAX, HEIGHT_MIN, cellsOf, cliffSkipped, fillCells, heightSkipped, pickedTile, strokeRequest, tileParts } from "./ops";
import type { Cell, PaintSub, WorldCells } from "./ops";
import { peekTileImage, tileImage } from "./paletteImages";
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
    // A terrain or cliff stroke's cells are known only from the kernel's answer: the next stroke builds on the world read after it.
    if (change.terrain !== undefined || change.cliff !== undefined) continue;
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
  return t("world:paint.limitNotice", { cells: t("world:paint.cells", { count: n }), min: String(HEIGHT_MIN).replace("-", "−"), max: HEIGHT_MAX });
}

/**
 * Whether `e` is the kernel refusing a stroke that changes nothing (its `NOTHING_TO_CHANGE`): from a terrain or cliff stroke it is no
 * error. The kernel says it in the language it was set to, and the catalogs keep its words for each language (`paint.nothingToChange`);
 * the German words are matched too, for a kernel that still speaks German whatever the app shows.
 */
const changesNothing = (e: unknown) => {
  if (typeof e !== "object" || e === null) return false;
  const message = (e as { message?: unknown }).message;
  return message === t("world:paint.nothingToChange") || message === t("world:paint.nothingToChange", { lng: "de" });
};

/** The note of a terrain or cliff stroke without an active edge set. */
const noEdgeSet = () => t("world:paint.noEdgeSet");

/** Whether `sub` paints with an edge set (the kernel resolves the cells). */
const isSmart = (sub: PaintSub) => sub === "terrain" || sub === "cliff";

/** The colour of height step `step` (clamped to the range) as the layer „Höhen“ shows it. */
const stepColor = (step: number) => heightColor(Math.min(HEIGHT_MAX, Math.max(HEIGHT_MIN, step)) * UNITS_PER_STEP);

/**
 * The look of a stroke's plates: brush, rectangle and fill the texture of the tile from the palette's store (`peekTileImage`), the
 * placeholder colour of its role while the picture is not there or when there is none; the eraser and removing a ramp hatched, height and
 * ramp the colour of the step each cell goes to. The terrain brush shows the placeholder colour of the role of the active edge set's first
 * `center` tile (with ⇧ of its `outside` tile), the cliff brush the colour of its target step: the first cell's height ± the steps.
 */
function strokeLook(sub: PaintSub, lower: boolean, state: PaintState, cells: WorldCells | null, tileset: TilesetDto | null): PreviewLook {
  if (sub === "erase" || (sub === "ramp" && lower)) return { erase: true };
  const height = ([x, z]: Cell) => cells?.heights[z]?.[x] ?? 0;
  if (sub === "height") return { color: (cell) => stepColor(height(cell) + (lower ? -1 : 1)) };
  if (sub === "ramp") return { color: (cell) => stepColor(height(cell) + state.ramp.steps) };
  if (sub === "cliff") {
    // The plates are coloured in stroke order: the first cell asked for fixes the target of the whole stroke.
    let target: number | null = null;
    return { color: (cell) => stepColor((target ??= height(cell) + (lower ? -state.cliffRise : state.cliffRise))) };
  }
  if (sub === "terrain") {
    const set = tileset?.tileset.edge_sets?.find((s) => s.name === state.edgeSet);
    const token = lower ? set?.outside : set?.places.center?.[0];
    const parts = token === undefined ? null : tileParts(token);
    const tile = parts ? tileset?.tileset.tiles[Number.parseInt(parts.digits, 16)] : undefined;
    return { color: placeholderColor(tile?.role ?? "ground") };
  }
  const parts = tileParts(state.tile);
  const tile = parts ? tileset?.tileset.tiles[Number.parseInt(parts.digits, 16)] : undefined;
  const picture = tileset && tile ? peekTileImage(tileset.package, tile.texture) : null;
  // Turned and mirrored as the token says, like the mesh will show it.
  return picture && parts ? { texture: picture, rotate: parts.rotate, flip: parts.flip } : { color: placeholderColor(tile?.role ?? "ground") };
}

/** The open paint session of `target`: its world (world_get, read again on every revision), tileset (tileset_get), queue;
 * `onStroke` builds the request, pushes it, then refreshes mesh, heights and world and clears the preview.
 *
 * The request of a stroke is built when it is drawn, from the world read last with the strokes still on their way applied, so a fast
 * second stroke (a second + on the same cells) builds on the first. A refused stroke loses its preview, reports through `onError` and the
 * world is read again. A height stroke whose cells are all at the limit sends nothing; the cells at the limit are named (`onNotice`).
 *
 * The terrain and cliff brushes (6.4b) send the painted cells with the active edge set and a seed drawn per stroke; without a set they send
 * nothing and ask for one, a cliff whose first cell is at the limit names its cells, the notes of the answer go to `onNotice` as one notice (joined with „ · “), and the
 * kernel's „Nichts zu ändern.“ for them is no error. */
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
  // `pending` counts a stroke until the session is done with it (world read again, refresh asked for, preview cleared), not only until
  // `world_edit` answers: the browser gate waits for 0.
  const [queue] = useState<StrokeQueue>(() => createStrokeQueue(worldEdit, () => {}));
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
  // The tileset is read again on every revision too (⌘Z of a tileset step, an edit over MCP): the palette and the tile section never show
  // stale defaults. The kernel caches it per package, so a read per stroke is cheap; the last one stays shown until the next answer.
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
  }, [pkg, revision]);

  // The picture of the chosen tile is asked for ahead, so the first stroke with it already shows its texture.
  const chosenTexture = tileset?.tileset.tiles[Number.parseInt(paint.state.tile.slice(0, 4), 16)]?.texture ?? null;
  useEffect(() => {
    if (tileset && chosenTexture !== null) void tileImage(tileset.package, chosenTexture);
  }, [tileset, chosenTexture]);

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
    const smart = isSmart(stroke.sub);
    if (smart && state.edgeSet === null) {
      opts.onNotice?.(noEdgeSet());
      clear();
      return;
    }
    const o = { layer: state.layer, tile: state.tile, ramp: state.ramp, lower: stroke.lower, edgeSet: state.edgeSet, cliffRise: state.cliffRise };
    // A seed per terrain or cliff stroke picks the variants of the places it resolves (the same seed gives the same tiles).
    const seed = smart ? (crypto.getRandomValues(new Uint32Array(1))[0] ?? 0) : undefined;
    const built = strokeRequest(stroke.map, stroke.sub, strokeCells, cells, o, seed);
    const skipped = stroke.sub === "height" ? heightSkipped(strokeCells, cells, stroke.lower) : stroke.sub === "cliff" ? cliffSkipped(strokeCells, cells, o) : 0;
    if (skipped > 0) opts.onNotice?.(limitNotice(skipped));
    if (!built) {
      clear();
      return;
    }
    const req = built.req;
    inFlight.current.push(req);
    setPending(inFlight.current.length);
    // A stroke leaves the requests on their way only once the world is read again: until then the world read last lacks it, and the
    // next stroke still builds on it (applying it again is safe, the changes are absolute values).
    const settled = () => {
      inFlight.current = inFlight.current.filter((r) => r !== req);
      setPending(inFlight.current.length);
    };
    // `settled` runs whatever happens on the way (a throwing refresh, clear or onError): a request must never stay on its way.
    queue.push(req).then(
      async (report) => {
        try {
          // All notes of one answer as ONE notice (Ruling 15): a later notice would hide an earlier one.
          const notes = report?.notes ?? [];
          if (notes.length > 0) latest.current.options.onNotice?.(notes.join(" · "));
          latest.current.refresh(req.map);
          await readWorld(req.map).catch((e) => latest.current.onError(e));
          clear();
        } finally {
          settled();
        }
      },
      async (e) => {
        try {
          clear();
          // A terrain or cliff stroke that changes nothing is silent (the client cannot know beforehand), like unchanged cells in 6.4a.
          if (!(smart && changesNothing(e))) latest.current.onError(e);
          await readWorld(req.map).catch(() => undefined);
        } finally {
          settled();
        }
      },
    )
      // A throw that reached here (onError itself throwing) has nowhere left to go; it must not become an unhandled rejection.
      .catch(() => undefined);
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
