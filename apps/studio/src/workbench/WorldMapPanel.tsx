import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode, RefObject } from "react";
import type { HeaderSummary, MatrixDto, MatrixSummary, MemberCounts, WarpLink } from "../api";
import type { Message } from "../app/message";
import type { Depth } from "../ui/frame/shellState";
import { PanelHead } from "../ui/primitives";
import type { PanelHeadSwitcher } from "../ui/primitives";
import { thumbnails } from "../world/thumbnailStore";
import type { ThumbnailCache } from "../world/thumbnails";
import { AreaGrid, cellKey } from "./AreaGrid";
import { ExtendAreaDialog } from "./ExtendAreaDialog";
import type { HeaderIdentity } from "./headerIdentity";
import { MoreAreas } from "./MoreAreas";
import type { NavTarget } from "./navigation";
import type { CellRect } from "./sceneScope";
import { buildPlaces, mapsText, placeColor } from "./places";
import type { Place, PlaceHeader } from "./places";
import { TERMS, id4, termFor } from "./terms";
import { useMatrix } from "./useMatrix";
import styles from "./WorldMapPanel.module.css";
import { GRID_GAP, lineBetween, placeCells, regionMatrix, roomLinkIndex, roomLinks, wheelPixels } from "./worldMap";
import type { PlaceCells, RoomLink } from "./worldMap";
import { areaAtCell, cellKeysOfHeaders, firstNeighbour, halfWindow, planExtendTargets, planNeighbour, viewFor, worldExtendTargets, worldViews } from "./worldMapView";
import type { WorldViewKey } from "./worldMapView";
import type { Area, World } from "./worldModel";

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
  /** The world model; null until it is read (the panel then draws the region alone, without views). */
  world: World | null;
  /** The clear name of a header (the room plan's subtitle). */
  identify: (header: HeaderSummary) => HeaderIdentity;
  /** The pinned matrix cell, if any: marked on the world map and in the room plan instead of the selected area. */
  pinnedCell: { matrix: number; x: number; y: number } | null;
  /** The member counts: the extend dialog's range of maps. */
  counts: MemberCounts;
  /** A matrix was edited ("+"): refresh dirty count and, if it is the scene's matrix, the scene. */
  onEdited: (index: number) => void | Promise<void>;
  /** A map was appended ("+"): refresh the member counts. */
  onCountsChanged: () => void | Promise<void>;
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

/** Whether a point in cell units lies in the shown cells (a place's centre may lie between cells); everything is inside without a window. */
const inside = (rect: CellRect | null, p: { x: number; y: number }) =>
  rect === null || (p.x >= rect.x - 0.5 && p.y >= rect.y - 0.5 && p.x < rect.x + rect.width - 0.5 && p.y < rect.y + rect.height - 0.5);

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
  /** The cells the grid shows (a half of a split world map); null: the whole matrix. Labels and lines are placed relative to it. */
  window: CellRect | null;
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
const MapView = memo(function MapView({ grid, window: shownCells, hasRegion, message, labels, lineLinks, cardsShown, listLayout, rootBox, list }: MapViewProps) {
  const originX = shownCells?.x ?? 0;
  const originY = shownCells?.y ?? 0;
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
      if (link.cell === null || !inside(shownCells, link.cell)) continue;
      const cardEl = rootEl.querySelector(`[data-room-id="${link.header}"]`);
      if (!cardEl) continue;
      const rect = cardEl.getBoundingClientRect();
      const l = lineBetween(rect, { x: link.cell.x - originX, y: link.cell.y - originY }, CELL_SIZE, { x: gridRect.left, y: gridRect.top }, view.scale);
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
  }, [lineLinks, cardsShown, view.scale, rootBox, list, shownCells, originX, originY]);
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
            {labels.filter((place) => inside(shownCells, place.center)).map((place) => (
              <span
                key={place.key}
                data-place-label={place.key}
                className={styles.label}
                // Counter-scaled: the text keeps its size on screen while the map zooms.
                style={{ left: (place.center.x - originX) * pitch + CELL_SIZE / 2, top: (place.center.y - originY) * pitch + CELL_SIZE / 2, fontSize: 12 / view.scale, padding: `0 ${4 / view.scale}px`, borderRadius: 3 / view.scale }} /* dyn */
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
const NO_CELLS: ReadonlySet<string> = new Set();
/** Header 0 (`EVERYWHERE`) fills the landscape cells: it counts as no header here, a click on it never selects it. */
const LANDSCAPE = 0;

/**
 * Where "+" was pressed: the cell, the cell beside it whose map and header the dialog proposes (the area's on a world map, the nearest
 * filled one with a header in a room plan), and the name of the area or room the new map then belongs to.
 */
interface ExtendTarget {
  matrix: number;
  x: number;
  y: number;
  prefer: { x: number; y: number } | null;
  areaName: string | null;
}

/**
 * The world map panel. Its head switches between the views (`worldViews`): a world map, both halves of a split one, „Weitere Gebiete“
 * and, while a room or an area with its own map is selected, the room plan; the view follows the selection, a choice in the menu holds
 * until the next selection. A world map draws its matrix as a raster of map pictures (cells tinted by place, place names on top, wheel
 * to zoom, drag to pan, the other half dimmed) next to the list of places with a card per room; dashed lines run from the cards of the
 * selected header's place to the cell of each room's entrance. A click selects the area of a cell, ⌥click, a double click or a click on
 * the landscape the map; "+" beside the selected area creates a map there. The room plan draws the selected header's matrix at 64 px.
 * Until the world is read, it draws the region alone. Memoised: it renders when `context` changes (the owner keeps it, see
 * `WorldMapTab`), never for a hover of the scene; pan and zoom live in `MapView` and render neither the grid nor the list.
 */
export const WorldMapPanel = memo(function WorldMapPanel({ context }: { context: WorldMapContext }) {
  const {
    headers,
    matrices,
    warpLinks,
    matricesLoaded,
    matricesFailed,
    selected,
    areaDataId,
    revision,
    goTo,
    onError,
    onLoadStatus,
    cache = thumbnails,
    depth,
    world,
    identify,
    pinnedCell,
    counts,
    onEdited,
    onCountsChanged,
  } = context;
  useCacheProgress(cache, onLoadStatus);

  // The view: a choice in the menu while the selection stays, else the one that shows the selection, else the one shown last.
  const selectedHeader = useMemo(() => (selected === null ? null : (headers.find((h) => h.id === selected) ?? null)), [headers, selected]);
  const followed = useMemo(() => viewFor(world, selectedHeader), [world, selectedHeader]);
  const views = useMemo(() => (world ? worldViews(world, followed === "plan") : []), [world, followed]);
  const [choice, setChoice] = useState<{ key: WorldViewKey; selected: number | null } | null>(null);
  // The next selection ends a choice (the check below already ignores it for the render in between).
  useEffect(() => setChoice(null), [selected]);
  const lastView = useRef<WorldViewKey | null>(null);
  const offered = (key: WorldViewKey | null | undefined): key is WorldViewKey => typeof key === "string" && views.some((v) => v.key === key);
  const candidates = [choice?.selected === selected ? choice.key : null, followed, lastView.current, views[0]?.key];
  const view = candidates.find(offered) ?? null;
  useEffect(() => {
    lastView.current = view;
  });
  const viewItem = views.find((v) => v.key === view) ?? null;
  const worldMap = world?.worldMaps.find((m) => m.key === view) ?? null;

  // Without the overview every header would count as a room.
  const places = useMemo(() => buildPlaces(headers, matricesLoaded ? matrices : [], matricesLoaded ? warpLinks : []), [headers, matrices, warpLinks, matricesLoaded]);
  const regionIndex = useMemo(() => (matricesLoaded ? (regionMatrix(matrices)?.index ?? null) : null), [matrices, matricesLoaded]);
  const planIndex = view === "plan" ? (selectedHeader?.matrix_id ?? null) : null;
  const shownIndex = world === null ? regionIndex : (viewItem?.matrix ?? planIndex);
  const shown = useMatrix(shownIndex, revision, onError);
  // The matrix of a world map (or of the region while the world is not read), and the one of the room plan.
  const region = view === "plan" || view === "more" ? null : shown;
  const plan = view === "plan" ? shown : null;

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

  // A half of a split world map: its window and the other half's cells.
  const half = useMemo(() => (world && worldMap && region ? halfWindow(world, worldMap, region) : null), [world, worldMap, region]);
  // The area of the selection on this matrix (a room's: the area of its entrance); not one of the other half.
  const selectedArea = useMemo((): Area | null => {
    if (!world || selected === null || !region) return null;
    const key = world.areaOf.get(selected);
    const areas = worldMap ? worldMap.areas : world.worldMaps.filter((m) => m.matrix === region.index).flatMap((m) => m.areas);
    return areas.find((a) => a.key === key) ?? null;
  }, [world, worldMap, selected, region]);
  const areaIds = useMemo(() => new Set(selectedArea ? selectedArea.headers.map((h) => h.id) : selected === null ? [] : [selected]), [selectedArea, selected]);
  const areaCells = useMemo(() => (region ? cellKeysOfHeaders(region, areaIds) : NO_CELLS), [region, areaIds]);
  const marked = useMemo(() => {
    if (!region) return NO_CELLS;
    if (pinnedCell?.matrix === region.index) return new Set([cellKey(pinnedCell.x, pinnedCell.y)]);
    return areaCells;
  }, [region, pinnedCell, areaCells]);
  const extendable = useMemo(() => (region && selectedArea ? worldExtendTargets(region, [...areaIds]) : NO_CELLS), [region, selectedArea, areaIds]);

  /**
   * Pins the map of a cell; its header is selected first when it is not the shown one. The landscape (header 0), an empty cell and a
   * grid without header section use `fallback`: none on a world map (only the pin moves), the shown header in a room plan.
   */
  const selectMap = useCallback(
    (m: MatrixDto, x: number, y: number, fallback: number | null) => {
      const i = y * m.width + x;
      const own = m.maps[i] !== null ? m.headers?.[i] : undefined;
      const header = own === undefined || own === LANDSCAPE ? fallback : own;
      goTo({ kind: "matrixCell", matrix: m.index, x, y, header, show: "map" });
    },
    [goTo],
  );
  const onCellClick = useCallback(
    (x: number, y: number, { alt }: { shift: boolean; alt: boolean }) => {
      if (!region) return;
      if (!world) {
        // The region alone: a cell opens its header.
        const header = region.maps[y * region.width + x] !== null ? region.headers?.[y * region.width + x] : undefined;
        if (header !== undefined) goTo({ kind: "header", id: header });
        return;
      }
      const foreign = half?.foreign.has(cellKey(x, y)) ?? false;
      const area = alt || foreign ? null : areaAtCell(world, region, x, y);
      if (area) goTo({ kind: "header", id: area.headers[0].id });
      else if (alt || !foreign) selectMap(region, x, y, null);
    },
    [region, world, half, goTo, selectMap],
  );
  const onCellDoubleClick = useCallback((x: number, y: number) => region && world && selectMap(region, x, y, null), [region, world, selectMap]);
  const onPlanClick = useCallback((x: number, y: number) => plan && selectMap(plan, x, y, selected), [plan, selected, selectMap]);

  const [extendAt, setExtendAt] = useState<ExtendTarget | null>(null);
  // The dialog belongs to the matrix it was opened for: another view or matrix ends it.
  useEffect(() => setExtendAt(null), [shownIndex]);
  const onWorldExtend = useCallback(
    (x: number, y: number) => {
      if (!region) return;
      const prefer = firstNeighbour(region, x, y, (i) => areaCells.has(cellKey(i % region.width, Math.floor(i / region.width))));
      setExtendAt({ matrix: region.index, x, y, prefer, areaName: selectedArea?.name ?? null });
    },
    [region, areaCells, selectedArea],
  );
  const onPlanExtend = useCallback(
    (x: number, y: number) => {
      if (!plan) return;
      const prefer = planNeighbour(plan, x, y);
      // The new map belongs to the room or area of the copied cell (the shown header without header section).
      const own = prefer && plan.headers ? plan.headers[prefer.y * plan.width + prefer.x] : undefined;
      const header = own === undefined || own === LANDSCAPE ? selectedHeader : headers.find((h) => h.id === own);
      setExtendAt({ matrix: plan.index, x, y, prefer, areaName: header ? identify(header).name : null });
    },
    [plan, selectedHeader, headers, identify],
  );
  /** The extend dialog is done: a created copy shows in the counts and the matrix; a failed attempt may have left a copy behind, so it refreshes the same. */
  const afterExtend = useCallback(
    async (index: number) => {
      try {
        await onCountsChanged();
      } finally {
        await onEdited(index);
      }
    },
    [onCountsChanged, onEdited],
  );

  const grid = useMemo(
    () =>
      region && (
        <AreaGrid
          matrix={region}
          headers={headers}
          areaFallback={areaDataId}
          cellSize={CELL_SIZE}
          selected={marked}
          colorOf={colorOf}
          onCellClick={onCellClick}
          onCellDoubleClick={world ? onCellDoubleClick : undefined}
          onExtend={world ? onWorldExtend : undefined}
          extendable={world ? extendable : undefined}
          window={half?.rect}
          foreign={half?.foreign}
          cache={cache}
          depth={depth}
        />
      ),
    [region, headers, areaDataId, marked, colorOf, onCellClick, onCellDoubleClick, onWorldExtend, extendable, half, world, cache, depth],
  );
  const planSelected = useMemo(() => (plan && pinnedCell?.matrix === plan.index ? new Set([cellKey(pinnedCell.x, pinnedCell.y)]) : NO_CELLS), [plan, pinnedCell]);
  const planExtendable = useMemo(() => (plan ? planExtendTargets(plan) : NO_CELLS), [plan]);

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
  const openArea = useCallback(
    (area: Area) => {
      // The room plan of the area shows, also when its first header is selected already.
      setChoice(null);
      goTo({ kind: "header", id: area.headers[0].id });
    },
    [goTo],
  );

  const switcher = useMemo(
    (): PanelHeadSwitcher | null =>
      view === null ? null : { items: views.map((v) => ({ id: v.key, label: v.label })), value: view, onChange: (key) => setChoice({ key, selected }) },
    [views, view, selected],
  );

  // Everything that can move a card of the list; the map measures its lines again when it changes.
  const listLayout = useMemo(() => ({}), [places, links, selected, openKeys]);
  const rootBox = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const none = depth === "tuefteln" ? "Keine Region gefunden (kein Raster mit Header-Abschnitt)." : "Keine Region gefunden (keine Weltkarte mit Gebieten).";
  const noRegion = matricesLoaded ? none : matricesFailed ? "Übersicht nicht verfügbar" : "Lade Übersicht …";
  const extendMatrix = extendAt && shown?.index === extendAt.matrix ? shown : null;

  return (
    <div className={styles.panel} data-world-view={view ?? undefined}>
      {switcher && <PanelHead switcher={switcher} subtitle={view === "plan" && selectedHeader ? identify(selectedHeader).name : undefined} />}
      {view === "more" && world ? (
        <MoreAreas
          areas={world.moreAreas}
          matrices={matrices}
          headers={headers}
          areaFallback={areaDataId}
          revision={revision}
          cache={cache}
          colorOf={colorOf}
          depth={depth}
          onError={onError}
          onSelect={openArea}
        />
      ) : view === "plan" ? (
        <div className={styles.plan}>
          {plan ? (
            <AreaGrid
              matrix={plan}
              headers={headers}
              areaFallback={areaDataId}
              cellSize={64}
              selected={planSelected}
              colorOf={colorOf}
              onCellClick={onPlanClick}
              onExtend={onPlanExtend}
              extendable={planExtendable}
              label={`Raumplan, ${plan.height} Zeilen, ${plan.width} Spalten`}
              cache={cache}
              depth={depth}
            />
          ) : (
            <p className={styles.message}>Lade Raumplan …</p>
          )}
        </div>
      ) : (
        <div ref={rootBox} className={styles.body}>
          <MapView
            // A new view starts at the origin: pan and zoom of another world map do not carry over.
            key={view ?? "region"}
            grid={grid}
            window={half?.rect ?? null}
            hasRegion={region !== null}
            message={shownIndex === null ? noRegion : "Lade Region …"}
            labels={cellsOfPlaces}
            lineLinks={(selectedPlace !== undefined && links.get(selectedPlace)) || NO_LINKS}
            cardsShown={selectedPlace !== undefined && openKeys.has(selectedPlace)}
            listLayout={listLayout}
            rootBox={rootBox}
            list={list}
          />
          <PlaceList places={places} depth={depth} links={links} selected={selected} openKeys={openKeys} onToggle={onToggle} onOpen={openRoom} list={list} />
        </div>
      )}
      {extendAt && extendMatrix && (
        <ExtendAreaDialog
          matrix={extendMatrix}
          x={extendAt.x}
          y={extendAt.y}
          prefer={extendAt.prefer}
          areaName={extendAt.areaName}
          depth={depth}
          headers={headers}
          counts={counts}
          onDone={(report) => {
            setExtendAt(null);
            if (report) void afterExtend(extendMatrix.index);
          }}
          onError={(e) => {
            onError(e);
            void afterExtend(extendMatrix.index);
          }}
        />
      )}
    </div>
  );
});
