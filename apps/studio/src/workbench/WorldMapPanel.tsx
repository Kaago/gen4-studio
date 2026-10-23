import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, ReactNode, RefObject } from "react";
import type { HeaderSummary, MatrixSummary, WarpLink } from "../api";
import type { Message } from "../app/message";
import { thumbnails } from "../world/thumbnailStore";
import type { ThumbnailCache } from "../world/thumbnails";
import { AreaGrid, cellKey } from "./AreaGrid";
import type { NavTarget } from "./navigation";
import { buildPlaces, mapsText, placeColor } from "./places";
import type { Place, PlaceHeader } from "./places";
import { TERMS, id4 } from "./terms";
import { useMatrix } from "./useMatrix";
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
}

const CELL_SIZE = 32;
const MIN_SCALE = 0.5;
const MAX_SCALE = 3;
/** The status line "Kartenbilder n/m" shows only for a queue longer than this. */
const PROGRESS_THRESHOLD = 50;
const PROGRESS_INTERVAL_MS = 200;
const GREY = "#6b7280";
const LINE_COLOR = "#2563eb";

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

const card: CSSProperties = {
  all: "unset",
  boxSizing: "border-box",
  display: "flex",
  flexDirection: "column",
  width: "100%",
  padding: "4px 8px",
  cursor: "pointer",
  border: "1px solid #d1d5db",
  borderRadius: 4,
  background: "#fff",
  fontSize: 12,
};

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
      style={{ ...card, fontWeight: selected ? 700 : 400, color: unplaced ? GREY : "#111827", background: unplaced ? "#f3f4f6" : "#fff", boxShadow: selected ? "inset 3px 0 0 #2563eb" : "none" }}
    >
      <span>{h.internal_name}</span>
      <span style={{ color: GREY, fontWeight: 400 }}>{grey}</span>
      {unplaced && <span style={{ color: GREY, fontWeight: 400 }}>ohne Eingang</span>}
      {unplaced && <span style={{ color: GREY, fontWeight: 400, fontSize: 11 }}>{link.reason}</span>}
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
        style={{ position: "relative", flex: 1, minWidth: 0, overflow: "hidden", touchAction: "none", cursor: "grab", background: "#f9fafb" }}
      >
        {hasRegion ? (
          <div
            ref={gridBox}
            data-world-grid=""
            style={{ position: "absolute", left: 0, top: 0, transformOrigin: "0 0", transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }}
          >
            {grid}
            {labels.map((place) => (
              <span
                key={place.key}
                data-place-label={place.key}
                style={{
                  position: "absolute",
                  left: place.center.x * pitch + CELL_SIZE / 2,
                  top: place.center.y * pitch + CELL_SIZE / 2,
                  transform: "translate(-50%, -50%)",
                  pointerEvents: "none",
                  whiteSpace: "nowrap",
                  // Counter-scaled: the text keeps its size on screen while the map zooms.
                  fontSize: 12 / view.scale,
                  fontWeight: 700,
                  padding: `0 ${4 / view.scale}px`,
                  borderRadius: 3 / view.scale,
                  background: "rgba(255,255,255,0.8)",
                }}
              >
                {place.name}
              </span>
            ))}
          </div>
        ) : (
          <p style={{ margin: 0, padding: 16 }}>{message}</p>
        )}
      </div>
      <svg aria-hidden="true" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }}>
        {lines.map((l) => (
          <line
            key={l.header}
            data-room-id={l.header}
            x1={l.x1}
            y1={l.y1}
            x2={l.x2}
            y2={l.y2}
            stroke={LINE_COLOR}
            strokeWidth={1.5}
            strokeDasharray="4 3"
            visibility={l.hidden ? "hidden" : "visible"}
          />
        ))}
      </svg>
    </>
  );
});

interface PlaceListProps {
  places: Place[];
  links: ReadonlyMap<string, RoomLink[]>;
  selected: number | null;
  openKeys: ReadonlySet<string>;
  onToggle: (key: string, open: boolean) => void;
  onOpen: (id: number) => void;
  list: RefObject<HTMLDivElement | null>;
}

/** The places as one `details` each, a card per room. Memoised: pan and zoom of the map do not reach it. */
const PlaceList = memo(function PlaceList({ places, links, selected, openKeys, onToggle, onOpen, list }: PlaceListProps) {
  return (
    <div ref={list} style={{ width: 300, flex: "none", overflowY: "auto", boxSizing: "border-box", padding: "8px 12px", borderLeft: "1px solid #e5e7eb" }}>
      {places.map((place) => (
        <details key={place.key} data-place-key={place.key} open={openKeys.has(place.key)} onToggle={(e) => onToggle(place.key, e.currentTarget.open)}>
          <summary style={{ cursor: "pointer", lineHeight: 1.6 }}>
            <span style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, marginRight: 6, background: placeColor(place.key) }} />
            {place.name}
          </summary>
          <div style={{ display: "flex", flexDirection: "column", gap: 4, padding: "2px 0 8px 16px" }}>
            {place.headers.map((entry) =>
              entry.kind === "inside" ? (
                <RoomCard key={entry.header.id} entry={entry} link={links.get(place.key)?.find((l) => l.header === entry.header.id)} selected={entry.header.id === selected} onOpen={onOpen} />
              ) : (
                <button
                  key={entry.header.id}
                  type="button"
                  data-header-id={entry.header.id}
                  aria-current={entry.header.id === selected ? "true" : undefined}
                  title={TERMS.outside.short}
                  onClick={() => onOpen(entry.header.id)}
                  style={{ all: "unset", cursor: "pointer", fontSize: 12, fontWeight: entry.header.id === selected ? 700 : 400 }}
                >
                  {TERMS.outside.label} · {id4(entry.header.id)}
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
  const { headers, matrices, warpLinks, matricesLoaded, matricesFailed, selected, areaDataId, revision, goTo, onError, onLoadStatus, cache = thumbnails } = context;
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
    () => region && <AreaGrid matrix={region} headers={headers} areaFallback={areaDataId} cellSize={CELL_SIZE} selected={marked} colorOf={colorOf} onCellClick={onCellClick} cache={cache} />,
    [region, headers, areaDataId, marked, colorOf, onCellClick, cache],
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
  const noRegion = matricesLoaded ? "Keine Region gefunden (kein Raster mit Header-Abschnitt)." : matricesFailed ? "Übersicht nicht verfügbar" : "Lade Übersicht …";

  return (
    <div ref={rootBox} style={{ position: "relative", display: "flex", height: "100%", fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
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
      <PlaceList places={places} links={links} selected={selected} openKeys={openKeys} onToggle={onToggle} onOpen={openRoom} list={list} />
    </div>
  );
});
