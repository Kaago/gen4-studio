import { memo, useMemo, useRef } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { thumbnails } from "../world/thumbnailStore";
import type { ThumbnailCache } from "../world/thumbnails";
import { useThumbnail } from "../world/useThumbnail";
import styles from "./AreaGrid.module.css";
import { rawCell } from "./matrix";
import type { CellRect } from "./sceneScope";
import { id4 } from "./terms";
import { useVisible } from "./useVisible";
import { GRID_GAP } from "./worldMap";

export const cellKey = (x: number, y: number) => `${x},${y}`;

export interface AreaGridProps {
  matrix: MatrixDto;
  headers: HeaderSummary[];
  /** Area data for cells without a header of their own (a grid without header section): the one of the shown header. */
  areaFallback: number;
  cellSize: 32 | 64;
  selected?: ReadonlySet<string>;
  onCellClick?: (x: number, y: number, modifiers: { shift: boolean; alt: boolean }) => void;
  onCellDoubleClick?: (x: number, y: number) => void;
  /**
   * Without `extendable`: shows a "+" column on the right and a "+" row below; a click reports the target cell (x = width or y = height).
   * With it: only the cells named there are "+" buttons, see `extendable`.
   */
  onExtend?: (x: number, y: number) => void;
  /**
   * The `cellKey`s that are "+" buttons (a cell inside the matrix, or one of the column x = width or the row y = height) and report
   * through `onExtend`; the extra column or row appears only when a key lies there. Their label says that the step cannot be undone.
   */
  extendable?: ReadonlySet<string>;
  /** Shows only these cells (cut to the matrix); `data-cell` keeps the matrix coordinates. Default: the whole matrix. */
  window?: CellRect;
  /** Cells drawn dimmed and marked `data-foreign` (the other half of a split world map). */
  foreign?: ReadonlySet<string>;
  /** The name of the group; default "Raster N, H Zeilen, W Spalten". */
  label?: string;
  /** Default: the app-wide cache. */
  cache?: ThumbnailCache;
  /** Colour per cell (place), optional. */
  colorOf?: (header: number | null) => string | null;
  /** The depth Tüfteln: every filled cell writes its map, header and altitude as three lines of raw values. */
  tinker?: boolean;
  /** The words of the tooltips (Bauen: "Nummer", Tüfteln: "Header"), for a grid that shows pictures in both depths; default: `tinker`. */
  depth?: Depth;
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
  foreign: boolean;
  /** The grid has a header section. */
  hasHeaders: boolean;
  cache: ThumbnailCache;
  tinker: boolean;
  /** The tooltip says "Header" (Tüfteln), else "Nummer". */
  techWords: boolean;
  onClick?: AreaGridProps["onCellClick"];
  onDoubleClick?: AreaGridProps["onCellDoubleClick"];
}

/** Memoised: a selection change or a new picture renders only the cells whose props changed, not the whole grid. */
const Cell = memo(function Cell({ x, y, map, header, altitude, area, color, size, selected, foreign, hasHeaders, cache, tinker, techWords, onClick, onDoubleClick }: CellProps) {
  const button = useRef<HTMLButtonElement>(null);
  const visible = useVisible(button, map !== null);
  // Only a cell in view asks for its picture, so a big grid does not queue hundreds of jobs the user never sees.
  const picture = useThumbnail(cache, map, area, visible);
  const parts = [`x ${x}, y ${y}`];
  if (map !== null) parts.push(`Karte ${id4(map)}`);
  if (header !== null) parts.push(`${techWords ? "Header" : "Nummer"} ${id4(header)}`);
  if (altitude !== null) parts.push(`Höhe ${altitude}`);
  const failure = map !== null && picture === null ? cache.failure(map, area) : null;
  if (failure) parts.push(`Bild fehlgeschlagen: ${failure}`);

  // What only the data knows: the picture, the place colour.
  const pictured = map !== null && typeof picture === "string";
  const tinted = map !== null && color !== null && color !== "";
  const style = {
    backgroundImage: pictured ? `url("${picture}")` : undefined, /* dyn */
    backgroundColor: !pictured && tinted ? color : undefined, /* dyn */
    boxShadow: tinted ? `inset 0 -4px 0 ${color}` : undefined, /* dyn */
  };
  const classes = [styles.cell, size === 64 ? styles.big : styles.small, map === null ? styles.empty : "", selected ? styles.selected : "", foreign ? styles.foreign : ""].join(" ");

  return (
    <button
      ref={button}
      type="button"
      data-cell={cellKey(x, y)}
      data-foreign={foreign ? "" : undefined}
      data-raw={tinker && map !== null ? rawCell(map, hasHeaders ? header : null, altitude) : undefined}
      aria-pressed={selected}
      title={parts.join(" · ")}
      className={classes}
      style={style} /* dyn */
      onClick={(e) => onClick?.(x, y, { shift: e.shiftKey, alt: e.altKey })}
      onDoubleClick={() => onDoubleClick?.(x, y)}
    >
      {map === null ? (
        "—"
      ) : tinker ? (
        <span className={styles.raw}>
          <span>{id4(map)}</span>
          {hasHeaders && header !== null && <span>{id4(header)}</span>}
          {altitude !== null && <span>{altitude}</span>}
        </span>
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

/** The words of a "+" button; the extendable cells of the world map say the step cannot be undone. */
const plusLabel = (x: number, y: number, notUndoable: boolean) => `Karte bei x ${x}, y ${y} anlegen${notUndoable ? " (nicht rückgängig)" : ""}`;

/** The cells of a matrix as a grid of map pictures; shared by the matrix panel, the wizards and the world map. */
export function AreaGrid({ matrix, headers, areaFallback, cellSize, selected, onCellClick, onCellDoubleClick, onExtend, extendable, window, foreign, label, cache = thumbnails, colorOf, tinker = false, depth }: AreaGridProps) {
  const techWords = depth === undefined ? tinker : depth === "tuefteln";
  // The tileset of each header, looked up once per header list instead of a search per cell.
  const areaOf = useMemo(() => new Map(headers.map((h) => [h.id, h.area_data_id])), [headers]);
  const x0 = Math.max(0, window?.x ?? 0);
  const y0 = Math.max(0, window?.y ?? 0);
  const x1 = Math.min(matrix.width, window ? window.x + window.width : matrix.width);
  const y1 = Math.min(matrix.height, window ? window.y + window.height : matrix.height);
  // The "+" column and row sit past the matrix edge: shown with a window that reaches the edge only.
  const plusKeys = extendable ? [...extendable].map((key) => key.split(",").map(Number)) : null;
  const extraColumn = x1 === matrix.width && (plusKeys ? plusKeys.some(([x]) => x >= matrix.width) : onExtend !== undefined);
  const extraRow = y1 === matrix.height && (plusKeys ? plusKeys.some(([, y]) => y >= matrix.height) : onExtend !== undefined);
  const columns = x1 - x0 + (extraColumn ? 1 : 0);
  const sizeClass = cellSize === 64 ? styles.big : styles.small;
  const cells = [];
  for (let y = y0; y < y1 + (extraRow ? 1 : 0); y++) {
    for (let x = x0; x < x1 + (extraColumn ? 1 : 0); x++) {
      const key = cellKey(x, y);
      const outside = x >= matrix.width || y >= matrix.height;
      if (extendable ? extendable.has(key) : outside) {
        const text = plusLabel(x, y, extendable !== undefined);
        cells.push(
          <button type="button" key={key} data-extend={key} aria-label={text} title={text} className={`${styles.cell} ${sizeClass} ${styles.extend}`} onClick={() => onExtend?.(x, y)}>
            +
          </button>,
        );
        continue;
      }
      if (outside) {
        // A position of the extra column or row that is no target: blank, so the grid keeps its columns.
        cells.push(<span key={key} aria-hidden="true" className={sizeClass} />);
        continue;
      }
      const i = y * matrix.width + x;
      const map = matrix.maps[i] ?? null;
      const header = matrix.headers && map !== null ? (matrix.headers[i] ?? null) : null;
      cells.push(
        <Cell
          key={key}
          x={x}
          y={y}
          map={map}
          header={header}
          altitude={matrix.altitudes ? (matrix.altitudes[i] ?? 0) : null}
          area={header === null ? areaFallback : (areaOf.get(header) ?? areaFallback)}
          color={colorOf ? colorOf(header) : null}
          size={cellSize}
          selected={selected?.has(key) ?? false}
          foreign={foreign?.has(key) ?? false}
          hasHeaders={matrix.headers != null}
          cache={cache}
          tinker={tinker}
          techWords={techWords}
          onClick={onCellClick}
          onDoubleClick={onCellDoubleClick}
        />,
      );
    }
  }
  return (
    <div
      role="group"
      aria-label={label ?? `Raster ${matrix.index}, ${matrix.height} Zeilen, ${matrix.width} Spalten`}
      className={styles.grid}
      style={{ gridTemplateColumns: `repeat(${columns}, ${cellSize}px)`, gap: GRID_GAP }} /* dyn */
    >
      {cells}
    </div>
  );
}
