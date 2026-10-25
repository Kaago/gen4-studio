import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode, RefObject } from "react";
import type { HeaderSummary, MatrixSummary, WarpLink } from "../api";
import type { Message } from "../app/message";
import type { Depth } from "../ui/frame/shellState";
import { thumbnails } from "../world/thumbnailStore";
import type { ThumbnailCache } from "../world/thumbnails";
import { AreaGrid, cellKey } from "./AreaGrid";
import type { NavTarget } from "./navigation";
import { buildPlaces, mapsText, placeColor } from "./places";
import type { Place, PlaceHeader } from "./places";
import { TERMS, id4, termFor } from "./terms";
import { useMatrix } from "./useMatrix";
import styles from "./WorldMapPanel.module.css";
import { GRID_GAP, lineBetween, placeCells, regionMatrix, roomLinkIndex, roomLinks, wheelPixels } from "./worldMap";
import type { PlaceCells, RoomLink } from "./worldMap";

/** What the world map needs from App. */
export interface WorldMapContext {
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  warpLinks: WarpLink[];
  matricesLoaded: boolean;
  matricesFailed: boolean;
  /** The selected header: its place is open in the list and its cell is marked. */
  selected: number | null;
  /** Tileset of cells without a header of their own. */
  areaDataId: number;
  /** Changes whenever matrix data may have changed behind the panel (edits, undo, redo, reload). */
  revision: number;
  goTo: (target: NavTarget) => void;
  onError: (e: unknown) => void;
  onLoadStatus: (message: Message | null) => void;
  /** Default: the app-wide cache. */
  cache?: ThumbnailCache;
  /** Bauen calls an outside header "Gebiet", Tüfteln "Header". */
  depth: Depth;
}

const CELL_SIZE = 32;
const MIN_SCALE = 0.5;
const MAX_SCALE = 3;
/** The status line "Kartenbilder n/m" shows only for a queue longer than this. */
const PROGRESS_THRESHOLD = 50;
const PROGRESS_INTERVAL_MS = 200;

interface View {
  scale: number;
  x: number;
  y: number;
}

interface Line {
  header: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  /** The end points lie outside what is shown (scrolled or panned away). */
  hidden: boolean;
}

/**
 * Reports "Kartenbilder n/m" through `onLoadStatus` while more than 50 pictures of the shared cache are queued
 * (m = the longest queue since the line appeared), and clears the line when the queue is empty and on unmount.
 * Polls: the cache only announces finished pictures, and the first one may take a while.
 */
function useCacheProgress(cache: ThumbnailCache, onLoadStatus: (message: Message | null) => void) {
  const report = useRef(onLoadStatus);
  report.current = onLoadStatus;
  useEffect(() => {
    let peak = 0;
    let shown: string | null = null;
    const check = () => {
      const pending = cache.pending();
      if (shown === null && pending <= PROGRESS_THRESHOLD) return;
      peak = Math.max(peak, pending);
      if (pending === 0) {
        report.current(null);
        shown = null;
        peak = 0;
        return;
      }
      const text = `Kartenbilder ${peak - pending}/${peak}`;
      if (text === shown) return;
      shown = text;
      report.current({ kind: "info", text });
    };
    const timer = setInterval(check, PROGRESS_INTERVAL_MS);
    return () => {
      clearInterval(timer);
      if (shown !== null) report.current(null);
    };
  }, [cache]);
}

const clampScale = (scale: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));

const sameLines = (a: Line[], b: Line[]) =>
  a.length === b.length && a.every((l, i) => l.header === b[i].header && l.hidden === b[i].hidden && l.x1 === b[i].x1 && l.y1 === b[i].y1 && l.x2 === b[i].x2 && l.y2 === b[i].y2);

function RoomCard({ entry, link, selected, onOpen }: { entry: PlaceHeader; link: RoomLink | undefined; selected: boolean; onOpen: (id: number) => void }) {
  const h = entry.header;
  const grey = [id4(h.id), mapsText(entry)].filter((part) => part !== null).join(" · ");
  const unplaced = link !== undefined && link.cell === null;
  return (
    <button
      type="button"
      data-room-id={h.id}
      aria-current={selected ? "true" : undefined}
      title={unplaced ? `ohne Eingang: ${link.reason}` : undefined}
      onClick={() => onOpen(h.id)}
      className={`${styles.card} ${selected ? styles.current : ""} ${unplaced ? styles.unplaced : ""}`}
    >
      <span className={styles.cardName}>{h.internal_name}</span>
      <span className={styles.grey}>{grey}</span>
      {unplaced && <span className={styles.grey}>ohne Eingang</span>}
      {unplaced && <span className={`${styles.grey} ${styles.reason}`}>{link.reason}</span>}
    </button>
  );
}

interface MapViewProps {
  /** The region's grid (stable between renders of the panel while its data stays). */
  grid: ReactNode;
  /** False: no region to draw; `message` says why. */
  hasRegion: boolean;
  message: string;
  labels: PlaceCells[];
  /** The room links of the selected header's place: the only ones that get a line. */
  lineLinks: RoomLink[];
  /** The selected place's cards are on show (its details are open). */
  cardsShown: boolean;
  /** Changes whenever the list may have laid its cards out differently (places opened or closed, selection, new data): the lines are measured again. */
  listLayout: unknown;
  rootBox: RefObject<HTMLDivElement | null>;
  list: RefObject<HTMLDivElement | null>;
}

/**
 * The viewport with the region and the line overlay. It owns the pan and zoom state, so a wheel tick or a pointer move
 * renders only this component (a few labels and lines), not the grid's cells and not the place list.
 */
const MapView = memo(function MapView({ grid, hasRegion, message, labels, lineLinks, cardsShown, listLayout, rootBox, list }: MapViewProps) {
  const [view, setView] = useState<View>({ scale: 1, x: 12, y: 12 });
  const viewport = useRef<HTMLDivElement>(null);
  const gridBox = useRef<HTMLDivElement>(null);
  const dragged = useRef(false);
  const stopDrag = useRef<(() => void) | null>(null);
  useEffect(() => () => stopDrag.current?.(), []);

  // React registers wheel listeners as passive, which cannot keep the page from scrolling: a native one does.
  useEffect(() => {
    const el = viewport.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = el.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      const factor = Math.exp(-wheelPixels(e) * 0.0015);
      // Zoom around the pointer: the point under it stays where it is.
      setView((v) => {
        const scale = clampScale(v.scale * factor);
        const k = scale / v.scale;
        return { scale, x: px - (px - v.x) * k, y: py - (py - v.y) * k };
      });
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    dragged.current = false;
    const start = { px: e.clientX, py: e.clientY, x: view.x, y: view.y };
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - start.px;
      const dy = ev.clientY - start.py;
      if (!dragged.current && Math.hypot(dx, dy) < 4) return;
      dragged.current = true;
      setView((v) => ({ ...v, x: start.x + dx, y: start.y + dy }));
    };
    const end = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      stopDrag.current = null;
    };
    stopDrag.current?.();
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
    stopDrag.current = end;
  };
  // The click that ends a drag must not open the cell under the pointer.
  const onClickCapture = (e: React.MouseEvent) => {
    if (!dragged.current) return;
    dragged.current = false;
    e.stopPropagation();
    e.preventDefault();
  };

  // Lines from the room cards of the selected place to the cells of their entrances; positions are measured, not computed.
  const [lines, setLines] = useState<Line[]>([]);
  const measure = useCallback(() => {
    const rootEl = rootBox.current;
    const viewportEl = viewport.current;
    const gridEl = gridBox.current;
    const listEl = list.current;
    if (!rootEl || !viewportEl || !gridEl || !listEl) {
      setLines((prev) => (prev.length === 0 ? prev : []));
      return;
    }
    const origin = rootEl.getBoundingClientRect();
    const shown = viewportEl.getBoundingClientRect();
    const listRect = listEl.getBoundingClientRect();
    const gridRect = gridEl.getBoundingClientRect();
    const next: Line[] = [];
    for (const link of lineLinks) {
      if (link.cell === null) continue;
      const cardEl = rootEl.querySelector(`[data-room-id="${link.header}"]`);
      if (!cardEl) continue;
      const rect = cardEl.getBoundingClientRect();
      const l = lineBetween(rect, link.cell, CELL_SIZE, { x: gridRect.left, y: gridRect.top }, view.scale);
      const hidden =
        !cardsShown ||
        rect.height === 0 ||
        rect.top < listRect.top ||
        rect.bottom > listRect.bottom ||
        l.x2 < shown.left ||
        l.x2 > shown.right ||
        l.y2 < shown.top ||
        l.y2 > shown.bottom;
      next.push({ header: link.header, x1: l.x1 - origin.left, y1: l.y1 - origin.top, x2: l.x2 - origin.left, y2: l.y2 - origin.top, hidden });
    }
    setLines((prev) => (sameLines(prev, next) ? prev : next));
  }, [lineLinks, cardsShown, view.scale, rootBox, list]);
  // After every render that may have moved a card or the grid: pan, zoom, selection, opened places, new data. `listLayout`
  // is what brings a change of the list (its own state) here at all, as this component is memoised.
  useLayoutEffect(measure, [measure, listLayout, view]);
  const measureRef = useRef(measure);
  measureRef.current = measure;
  useEffect(() => {
    const rootEl = rootBox.current;
    const listEl = list.current;
    if (!rootEl) return;
    const again = () => measureRef.current();
    listEl?.addEventListener("scroll", again);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(again);
    observer?.observe(rootEl);
    return () => {
      listEl?.removeEventListener("scroll", again);
      observer?.disconnect();
    };
  }, [rootBox, list]);

  const pitch = CELL_SIZE + GRID_GAP;
  return (
    <>
      <div
        ref={viewport}
        data-world-viewport=""
        onPointerDown={onPointerDown}
        onClickCapture={onClickCapture}
        className={styles.viewport}
      >
        {hasRegion ? (
          <div
            ref={gridBox}
            data-world-grid=""
            className={styles.world}
            style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }} /* dyn */
          >
            {grid}
            {labels.map((place) => (
              <span
                key={place.key}
                data-place-label={place.key}
                className={styles.label}
                // Counter-scaled: the text keeps its size on screen while the map zooms.
                style={{ left: place.center.x * pitch + CELL_SIZE / 2, top: place.center.y * pitch + CELL_SIZE / 2, fontSize: 12 / view.scale, padding: `0 ${4 / view.scale}px`, borderRadius: 3 / view.scale }} /* dyn */
              >
                {place.name}
              </span>
            ))}
          </div>
        ) : (
          <p className={styles.message}>{message}</p>
        )}
      </div>
      <svg aria-hidden="true" className={styles.lines}>
        {lines.map((l) => (
          <line
            key={l.header}
            data-room-id={l.header}
            x1={l.x1}
            y1={l.y1}
            x2={l.x2}
            y2={l.y2}
            className={styles.line}
            visibility={l.hidden ? "hidden" : "visible"}
          />
        ))}
      </svg>
    </>
  );
});

interface PlaceListProps {
  places: Place[];
  depth: Depth;
  links: ReadonlyMap<string, RoomLink[]>;
  selected: number | null;
  openKeys: ReadonlySet<string>;
  onToggle: (key: string, open: boolean) => void;
  onOpen: (id: number) => void;
  list: RefObject<HTMLDivElement | null>;
}

/** The places as one `details` each, a card per room. Memoised: pan and zoom of the map do not reach it. */
const PlaceList = memo(function PlaceList({ places, depth, links, selected, openKeys, onToggle, onOpen, list }: PlaceListProps) {
  return (
    <div ref={list} className={styles.places}>
      {places.map((place) => (
        <details key={place.key} data-place-key={place.key} open={openKeys.has(place.key)} onToggle={(e) => onToggle(place.key, e.currentTarget.open)}>
          <summary className={styles.summary}>
            <span className={styles.chip} style={{ background: placeColor(place.key) }} /* dyn */ />
            {place.name}
          </summary>
          <div className={styles.rooms}>
            {place.headers.map((entry) =>
              entry.kind === "inside" ? (
                <RoomCard key={entry.header.id} entry={entry} link={links.get(place.key)?.find((l) => l.header === entry.header.id)} selected={entry.header.id === selected} onOpen={onOpen} />
              ) : (
                <button
                  key={entry.header.id}
                  type="button"
                  data-header-id={entry.header.id}
                  aria-current={entry.header.id === selected ? "true" : undefined}
                  title={depth === "tuefteln" ? TERMS.outside.short : TERMS.area.short}
                  onClick={() => onOpen(entry.header.id)}
                  className={`${styles.outside} ${entry.header.id === selected ? styles.current : ""}`}
                >
                  {termFor("header", depth, "area")} · {id4(entry.header.id)}
                </button>
              ),
            )}
          </div>
        </details>
      ))}
    </div>
  );
});

const NO_LINKS: RoomLink[] = [];

/**
 * The world map: the region as a raster of map pictures (cells tinted by place, place names on top, wheel to zoom, drag to
 * pan) next to the list of places with a card per room; dashed lines run from the cards of the selected header's place to
 * the cell of each room's entrance. Memoised: it renders when `context` changes (the owner keeps it, see `WorldMapTab`),
 * never for a hover of the scene; pan and zoom live in `MapView` and render neither the grid nor the list.
 */
export const WorldMapPanel = memo(function WorldMapPanel({ context }: { context: WorldMapContext }) {
  const { headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, revision, goTo, onError, onLoadStatus, cache = thumbnails, depth } = context;
  useCacheProgress(cache, onLoadStatus);

  // Without the overview every header would count as a room.
  const places = useMemo(() => buildPlaces(headers, matricesLoaded ? matrices : [], matricesLoaded ? warpLinks : []), [headers, matrices, warpLinks, matricesLoaded]);
  const regionIndex = useMemo(() => (matricesLoaded ? (regionMatrix(matrices)?.index ?? null) : null), [matrices, matricesLoaded]);
  const region = useMatrix(regionIndex, revision, onError);
  const cellsOfPlaces = useMemo(() => (region ? placeCells(region, places) : []), [region, places]);
  const links = useMemo(() => {
    const byPlace = new Map<string, RoomLink[]>();
    if (!region) return byPlace;
    const index = roomLinkIndex(headers, warpLinks);
    for (const place of places) byPlace.set(place.key, roomLinks(place, headers, warpLinks, region, index));
    return byPlace;
  }, [region, places, headers, warpLinks]);

  const colorByHeader = useMemo(() => new Map(places.flatMap((place) => place.headers.map(({ header }) => [header.id, placeColor(place.key)] as const))), [places]);
  const colorOf = useCallback((header: number | null) => (header === null ? null : (colorByHeader.get(header) ?? null)), [colorByHeader]);
  const marked = useMemo(() => {
    const keys = new Set<string>();
    if (region?.headers && selected !== null) {
      region.headers.forEach((h, i) => {
        if (h === selected && region.maps[i] !== null) keys.add(cellKey(i % region.width, Math.floor(i / region.width)));
      });
    }
    return keys;
  }, [region, selected]);
  const onCellClick = useCallback(
    (x: number, y: number) => {
      if (!region?.headers) return;
      const i = y * region.width + x;
      if (region.maps[i] !== null) goTo({ kind: "header", id: region.headers[i] });
    },
    [region, goTo],
  );
  const grid = useMemo(
    () => region && <AreaGrid matrix={region} headers={headers} areaFallback={areaDataId} cellSize={CELL_SIZE} selected={marked} colorOf={colorOf} onCellClick={onCellClick} cache={cache} depth={depth} />,
    [region, headers, areaDataId, marked, colorOf, onCellClick, cache, depth],
  );

  // The place of the selected header is open; the user opens and closes the others.
  const [openKeys, setOpenKeys] = useState<ReadonlySet<string>>(new Set());
  const selectedPlace = useMemo(() => (selected === null ? undefined : places.find((p) => p.headers.some((h) => h.header.id === selected))?.key), [places, selected]);
  useEffect(() => {
    if (selectedPlace !== undefined) setOpenKeys((prev) => (prev.has(selectedPlace) ? prev : new Set(prev).add(selectedPlace)));
  }, [selectedPlace]);
  const onToggle = useCallback(
    (key: string, open: boolean) =>
      setOpenKeys((prev) => {
        if (prev.has(key) === open) return prev;
        const next = new Set(prev);
        if (open) next.add(key);
        else next.delete(key);
        return next;
      }),
    [],
  );
  const openRoom = useCallback((id: number) => goTo({ kind: "header", id }), [goTo]);

  // Everything that can move a card of the list; the map measures its lines again when it changes.
  const listLayout = useMemo(() => ({}), [places, links, selected, openKeys]);
  const rootBox = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const none = depth === "tuefteln" ? "Keine Region gefunden (kein Raster mit Header-Abschnitt)." : "Keine Region gefunden (keine Weltkarte mit Gebieten).";
  const noRegion = matricesLoaded ? none : matricesFailed ? "Übersicht nicht verfügbar" : "Lade Übersicht …";

  return (
    <div ref={rootBox} className={styles.panel}>
      <MapView
        grid={grid}
        hasRegion={region !== null}
        message={regionIndex === null ? noRegion : "Lade Region …"}
        labels={cellsOfPlaces}
        lineLinks={(selectedPlace !== undefined && links.get(selectedPlace)) || NO_LINKS}
        cardsShown={selectedPlace !== undefined && openKeys.has(selectedPlace)}
        listLayout={listLayout}
        rootBox={rootBox}
        list={list}
      />
      <PlaceList places={places} depth={depth} links={links} selected={selected} openKeys={openKeys} onToggle={onToggle} onOpen={openRoom} list={list} />
    </div>
  );
});
