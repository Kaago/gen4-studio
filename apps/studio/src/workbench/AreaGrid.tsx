import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, RefObject } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import { thumbnails } from "../world/thumbnailStore";
import type { ThumbnailCache } from "../world/thumbnails";
import { useThumbnail } from "../world/useThumbnail";
import styles from "./AreaGrid.module.css";
import { id4 } from "./terms";
import { GRID_GAP } from "./worldMap";

export const cellKey = (x: number, y: number) => `${x},${y}`;

export interface AreaGridProps {
  matrix: MatrixDto;
  headers: HeaderSummary[];
  /** Area data for cells without a header of their own (a grid without header section): the one of the shown header. */
  areaFallback: number;
  cellSize: 32 | 64;
  selected?: ReadonlySet<string>;
  onCellClick?: (x: number, y: number, modifiers: { shift: boolean }) => void;
  onCellDoubleClick?: (x: number, y: number) => void;
  /** Shows a "+" column on the right and a "+" row below; a click reports the target cell (x = width or y = height). */
  onExtend?: (x: number, y: number) => void;
  /** Default: the app-wide cache. */
  cache?: ThumbnailCache;
  /** Colour per cell (place), optional. */
  colorOf?: (header: number | null) => string | null;
}

interface CellProps {
  x: number;
  y: number;
  map: number | null;
  header: number | null;
  altitude: number | null;
  area: number;
  color: string | null;
  size: 32 | 64;
  selected: boolean;
  /** The grid has a header section. */
  hasHeaders: boolean;
  cache: ThumbnailCache;
  onClick?: AreaGridProps["onCellClick"];
  onDoubleClick?: AreaGridProps["onCellDoubleClick"];
}

/** Whether the element is (nearly) in view; always true where there is no IntersectionObserver (tests). */
function useVisible(ref: RefObject<HTMLElement | null>, active: boolean): boolean {
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === "undefined");
  useEffect(() => {
    const el = ref.current;
    if (!active || !el || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => setVisible(entries[entries.length - 1]?.isIntersecting ?? false), { rootMargin: "128px" });
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, active]);
  return visible;
}

/** Memoised: a selection change or a new picture renders only the cells whose props changed, not the whole grid. */
const Cell = memo(function Cell({ x, y, map, header, altitude, area, color, size, selected, hasHeaders, cache, onClick, onDoubleClick }: CellProps) {
  const button = useRef<HTMLButtonElement>(null);
  const visible = useVisible(button, map !== null);
  // Only a cell in view asks for its picture, so a big grid does not queue hundreds of jobs the user never sees.
  const picture = useThumbnail(cache, map, area, visible);
  const parts = [`x ${x}, y ${y}`];
  if (map !== null) parts.push(`Karte ${id4(map)}`);
  if (header !== null) parts.push(`Header ${id4(header)}`);
  if (altitude !== null) parts.push(`Höhe ${altitude}`);
  const failure = map !== null && picture === null ? cache.failure(map, area) : null;
  if (failure) parts.push(`Bild fehlgeschlagen: ${failure}`);

  // What only the data knows: the picture, the place colour.
  const style: CSSProperties = {};
  if (map !== null && typeof picture === "string") style.backgroundImage = `url("${picture}")`; /* dyn */
  else if (map !== null && color) style.backgroundColor = color; /* dyn */
  if (map !== null && color) style.boxShadow = `inset 0 -4px 0 ${color}`; /* dyn */
  const classes = [styles.cell, size === 64 ? styles.big : styles.small, map === null ? styles.empty : "", selected ? styles.selected : ""].join(" ");

  return (
    <button
      ref={button}
      type="button"
      data-cell={cellKey(x, y)}
      aria-pressed={selected}
      title={parts.join(" · ")}
      className={classes}
      style={style} /* dyn */
      onClick={(e) => onClick?.(x, y, { shift: e.shiftKey })}
      onDoubleClick={() => onDoubleClick?.(x, y)}
    >
      {map === null ? (
        "—"
      ) : (
        <>
          {typeof picture !== "string" && <span className={styles.tag}>{id4(map)}</span>}
          {size === 64 && typeof picture === "string" && <span className={`${styles.tag} ${styles.tagMap}`}>{id4(map)}</span>}
          {size === 64 && hasHeaders && header !== null && <span className={`${styles.tag} ${styles.tagHeader}`}>{id4(header)}</span>}
        </>
      )}
    </button>
  );
});

/** The cells of a matrix as a grid of map pictures; shared by the matrix panel, the wizards and the world map. */
export function AreaGrid({ matrix, headers, areaFallback, cellSize, selected, onCellClick, onCellDoubleClick, onExtend, cache = thumbnails, colorOf }: AreaGridProps) {
  // The tileset of each header, looked up once per header list instead of a search per cell.
  const areaOf = useMemo(() => new Map(headers.map((h) => [h.id, h.area_data_id])), [headers]);
  const columns = matrix.width + (onExtend ? 1 : 0);
  const rows = matrix.height + (onExtend ? 1 : 0);
  const cells = [];
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < columns; x++) {
      if (x >= matrix.width || y >= matrix.height) {
        cells.push(
          <button
            type="button"
            key={cellKey(x, y)}
            data-extend={cellKey(x, y)}
            aria-label={`Karte bei x ${x}, y ${y} anlegen`}
            title={`Karte bei x ${x}, y ${y} anlegen`}
            className={`${styles.cell} ${cellSize === 64 ? styles.big : styles.small} ${styles.extend}`}
            onClick={() => onExtend?.(x, y)}
          >
            +
          </button>,
        );
        continue;
      }
      const i = y * matrix.width + x;
      const map = matrix.maps[i] ?? null;
      const header = matrix.headers && map !== null ? (matrix.headers[i] ?? null) : null;
      cells.push(
        <Cell
          key={cellKey(x, y)}
          x={x}
          y={y}
          map={map}
          header={header}
          altitude={matrix.altitudes ? (matrix.altitudes[i] ?? 0) : null}
          area={header === null ? areaFallback : (areaOf.get(header) ?? areaFallback)}
          color={colorOf ? colorOf(header) : null}
          size={cellSize}
          selected={selected?.has(cellKey(x, y)) ?? false}
          hasHeaders={matrix.headers != null}
          cache={cache}
          onClick={onCellClick}
          onDoubleClick={onCellDoubleClick}
        />,
      );
    }
  }
  return (
    <div
      role="group"
      aria-label={`Matrix ${matrix.index}, ${matrix.height} Zeilen, ${matrix.width} Spalten`}
      className={styles.grid}
      style={{ gridTemplateColumns: `repeat(${columns}, ${cellSize}px)`, gap: GRID_GAP }} /* dyn */
    >
      {cells}
    </div>
  );
}
