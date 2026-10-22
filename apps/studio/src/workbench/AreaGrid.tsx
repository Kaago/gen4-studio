import type { CSSProperties } from "react";
import type { HeaderSummary, MatrixDto } from "../api";
import { thumbnails } from "../world/thumbnailStore";
import { thumbnailArea } from "../world/thumbnails";
import type { ThumbnailCache } from "../world/thumbnails";
import { useThumbnail } from "../world/useThumbnail";
import { id4 } from "./terms";

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

const PLACEHOLDER = "#e5e7eb";

const base = (size: number): CSSProperties => ({
  all: "unset",
  boxSizing: "border-box",
  width: size,
  height: size,
  position: "relative",
  overflow: "hidden",
  cursor: "pointer",
  fontFamily: "ui-monospace, monospace",
  fontSize: size === 64 ? 11 : 9,
});

const tag: CSSProperties = { position: "absolute", padding: "0 3px", borderRadius: 2, background: "rgba(255,255,255,0.8)", color: "#111827", lineHeight: "14px" };

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

function Cell({ x, y, map, header, altitude, area, color, size, selected, hasHeaders, cache, onClick, onDoubleClick }: CellProps) {
  const picture = useThumbnail(cache, map, area);
  const parts = [`x ${x}, y ${y}`];
  if (map !== null) parts.push(`Karte ${id4(map)}`);
  if (header !== null) parts.push(`Header ${id4(header)}`);
  if (altitude !== null) parts.push(`Höhe ${altitude}`);
  const failure = map !== null && picture === null ? cache.failure(map, area) : null;
  if (failure) parts.push(`Bild fehlgeschlagen: ${failure}`);

  const style: CSSProperties = { ...base(size) };
  if (map === null) {
    Object.assign(style, { border: "1px dashed #9ca3af", color: "#9ca3af", display: "flex", alignItems: "center", justifyContent: "center", background: "transparent" });
  } else if (typeof picture === "string") {
    Object.assign(style, { backgroundImage: `url("${picture}")`, backgroundSize: "cover", backgroundPosition: "center", backgroundColor: PLACEHOLDER });
  } else {
    Object.assign(style, { backgroundColor: color ?? PLACEHOLDER, color: "#374151", display: "flex", alignItems: "center", justifyContent: "center" });
  }
  if (map !== null && color) style.boxShadow = `inset 0 -4px 0 ${color}`;
  style.outline = selected ? "3px solid #2563eb" : "1px solid #d1d5db";
  style.outlineOffset = selected ? -3 : -1;

  return (
    <button
      type="button"
      data-cell={cellKey(x, y)}
      aria-pressed={selected}
      title={parts.join(" · ")}
      style={style}
      onClick={(e) => onClick?.(x, y, { shift: e.shiftKey })}
      onDoubleClick={() => onDoubleClick?.(x, y)}
    >
      {map === null ? (
        "—"
      ) : (
        <>
          {typeof picture !== "string" && id4(map)}
          {size === 64 && typeof picture === "string" && <span style={{ ...tag, bottom: 2, left: 2 }}>{id4(map)}</span>}
          {size === 64 && hasHeaders && header !== null && <span style={{ ...tag, top: 2, right: 2 }}>{id4(header)}</span>}
        </>
      )}
    </button>
  );
}

/** The cells of a matrix as a grid of map pictures; shared by the matrix panel, the wizards and the world map. */
export function AreaGrid({ matrix, headers, areaFallback, cellSize, selected, onCellClick, onCellDoubleClick, onExtend, cache = thumbnails, colorOf }: AreaGridProps) {
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
            style={{ ...base(cellSize), border: "1px dashed #9ca3af", color: "#6b7280", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 16 }}
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
          area={thumbnailArea(header, headers, areaFallback)}
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
      style={{ display: "grid", gridTemplateColumns: `repeat(${columns}, ${cellSize}px)`, gap: 2, width: "max-content" }}
    >
      {cells}
    </div>
  );
}
