import { useLayoutEffect, useMemo, useRef } from "react";
import type { HeaderSummary, MatrixDto } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { Notice } from "../../ui/primitives";
import { AreaGrid } from "../AreaGrid";
import { TERMS } from "../terms";
import { cellBounds, centredPan } from "../worldMap";
import type { CellRect } from "../sceneScope";
import type { World } from "../worldModel";
import { cellsOfHeaders, isFilled, targetBlock } from "../worldMapView";
import parts from "./WizardStep.module.css";
import styles from "./PlaceOnWorldMap.module.css";

/** The size of a cell of the world map in the wizard. */
const CELL = 32;

interface Props {
  /** The world model: the area of `focus` the map opens on. */
  world: World | null;
  /** The world map's matrix (with its header section); null while it is read. */
  matrix: MatrixDto | null;
  headers: HeaderSummary[];
  /** The size of the new area in cells. */
  width: number;
  height: number;
  /** The cell of the area's upper left map; null: none chosen. */
  value: { x: number; y: number } | null;
  onChange: (value: { x: number; y: number }) => void;
  depth: Depth;
  /** The header of the source: the map opens on the cells of its area when it lies on this world map. */
  focus?: number | null;
}

/** The cells of the area `header` belongs to on `matrix` (its sections), as a rectangle; null when it has none there. */
function areaRect(world: World | null, matrix: MatrixDto, header: number | null | undefined): CellRect | null {
  if (!world || header === null || header === undefined) return null;
  const key = world.areaOf.get(header);
  const area = world.worldMaps.flatMap((m) => m.areas).find((a) => a.key === key && a.matrix === matrix.index);
  return area ? cellBounds(matrix.width, cellsOfHeaders(matrix, new Set(area.headers.map((h) => h.id)))) : null;
}

/**
 * „Wo liegt das Gebiet?“ on a world map: a click on an empty or landscape cell sets the area's upper left cell, the block of `width` ×
 * `height` cells is selected there, or marked red with the reason when it falls on an area or past the edge.
 */
export function PlaceOnWorldMap({ world, matrix, headers, width, height, value, onChange, depth, focus }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const block = useMemo(() => (matrix && value ? targetBlock(matrix, value, width, height) : null), [matrix, value, width, height]);

  // Once per world map: the view jumps to the chosen block, else to the source's area (centred, as the world map panel does).
  const jumped = useRef<number | null>(null);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el || !matrix || jumped.current === matrix.index) return;
    jumped.current = matrix.index;
    const rect = value ? { x: value.x, y: value.y, width, height } : areaRect(world, matrix, focus);
    if (!rect) return;
    const pan = centredPan(rect, { x: 0, y: 0 }, CELL, { width: el.clientWidth, height: el.clientHeight }, { scale: 1, x: -el.scrollLeft, y: -el.scrollTop });
    if (pan) {
      el.scrollLeft = Math.max(0, -pan.x);
      el.scrollTop = Math.max(0, -pan.y);
    }
  }, [matrix]);

  if (!matrix) return <p className={parts.note}>Lade …</p>;
  const headerCells = matrix.headers;
  /** Empty and landscape cells take the corner; a cell of an area does not (the same cells `targetBlock` counts free). */
  const pick = (x: number, y: number) => {
    const i = y * matrix.width + x;
    if (!isFilled(matrix.maps[i]) || (headerCells?.[i] ?? 0) === 0) onChange({ x, y });
  };
  const label = depth === "bauen" ? `${TERMS.worldMap.label}, ${matrix.height} Zeilen, ${matrix.width} Spalten` : undefined;
  return (
    <>
      <p className={parts.note}>Die Zelle anklicken, an der die linke obere {TERMS.map.label} des neuen {TERMS.area.label}s liegen soll.</p>
      <div ref={box} className={styles.map}>
        <AreaGrid
          matrix={matrix}
          headers={headers}
          areaFallback={0}
          cellSize={CELL}
          selected={block?.ok ? block.cells : undefined}
          foreign={block && !block.ok ? block.cells : undefined}
          onCellClick={pick}
          depth={depth}
          label={label}
        />
      </div>
      {block && !block.ok && <Notice kind="error">Der Block muss auf freie Zellen oder Landschaft fallen.</Notice>}
    </>
  );
}
