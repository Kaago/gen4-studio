import { useMemo, useState } from "react";
import type { HeaderSummary, MatrixDto } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { AreaGrid } from "../AreaGrid";
import { TERMS, id4 } from "../terms";
import styles from "./PickGrid.module.css";

/** The place and colour of a header, as the matrix tab tints its cells. */
export type HeaderPlaces = ReadonlyMap<number, { name: string; color: string }>;

interface Props {
  matrix: MatrixDto;
  headers: HeaderSummary[];
  areaFallback: number;
  selected: ReadonlySet<string>;
  places: HeaderPlaces;
  /** The cell clicked last: the caption names it while the pointer is over no cell. */
  last: { x: number; y: number } | null;
  onCellClick: (x: number, y: number, modifiers: { shift: boolean }) => void;
  /** Bauen names a cell by its place and map, without the header number. Default: Tüfteln. */
  depth?: Depth;
  /** The grid's accessible name; default: the raster's (see `AreaGrid`). */
  label?: string;
}

/** "Route 201 · Header 0342 · Karte 0521" (Bauen: "Route 201 · Karte 0521"): which place a cell belongs to (a 32 px cell has room for no number). */
export function cellCaption(matrix: MatrixDto, places: HeaderPlaces, x: number, y: number, depth: Depth = "tuefteln"): string | null {
  if (x < 0 || y < 0 || x >= matrix.width || y >= matrix.height) return null;
  const i = y * matrix.width + x;
  const map = matrix.maps[i] ?? null;
  if (map === null) return `Keine ${TERMS.map.label} bei x ${x}, y ${y}`;
  const header = matrix.headers?.[i];
  const parts: string[] = [];
  if (header !== undefined) {
    const place = places.get(header);
    if (place) parts.push(place.name);
    if (depthAtLeast(depth, "tuefteln")) parts.push(`${TERMS.header.label} ${id4(header)}`);
  }
  parts.push(`${TERMS.map.label} ${id4(map)}`);
  return parts.join(" · ");
}

/**
 * The source grid of the area wizard: place colours on the cells and, below, a caption for the cell
 * under the pointer (else the one clicked last). The hover is state of this component only, and the
 * grid itself is not built again for it.
 */
export function PickGrid({ matrix, headers, areaFallback, selected, places, last, onCellClick, depth = "tuefteln", label }: Props) {
  const [hover, setHover] = useState<{ x: number; y: number } | null>(null);
  const colorOf = useMemo(() => (header: number | null) => (header === null ? null : (places.get(header)?.color ?? null)), [places]);
  const grid = useMemo(
    () => <AreaGrid matrix={matrix} headers={headers} areaFallback={areaFallback} cellSize={32} selected={selected} colorOf={colorOf} onCellClick={onCellClick} label={label} />,
    [matrix, headers, areaFallback, selected, colorOf, onCellClick, label],
  );
  const shown = hover ?? last;
  return (
    <div>
      <div
        className={styles.scroll}
        onMouseOver={(e) => {
          const key = e.target instanceof Element ? e.target.closest("[data-cell]")?.getAttribute("data-cell") : null;
          const [x, y] = key ? key.split(",").map(Number) : [];
          setHover(x === undefined || y === undefined ? null : { x, y });
        }}
        onMouseLeave={() => setHover(null)}
      >
        {grid}
      </div>
      <div className={styles.caption} data-caption="">
        {(shown && cellCaption(matrix, places, shown.x, shown.y, depth)) ?? ""}
      </div>
    </div>
  );
}
