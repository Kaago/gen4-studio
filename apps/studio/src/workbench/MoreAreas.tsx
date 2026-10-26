import { memo, useEffect, useRef, useState } from "react";
import type { HeaderSummary, MatrixSummary } from "../api";
import type { Depth } from "../ui/frame/shellState";
import { EmptyState } from "../ui/primitives";
import type { ThumbnailCache } from "../world/thumbnails";
import { AreaGrid } from "./AreaGrid";
import styles from "./MoreAreas.module.css";
import { useMatrix } from "./useMatrix";
import { useVisible } from "./useVisible";
import type { Area } from "./worldModel";

export interface MoreAreasProps {
  /** The areas with a map of their own (`World.moreAreas`). */
  areas: readonly Area[];
  /** The overview of the matrices: the number of maps of a tile, before its matrix is read. */
  matrices: readonly MatrixSummary[];
  headers: HeaderSummary[];
  /** Tileset of cells without a header of their own. */
  areaFallback: number;
  /** Changes whenever matrix data may have changed (edits, undo, redo, reload). */
  revision: number;
  cache: ThumbnailCache;
  colorOf: (header: number | null) => string | null;
  depth: Depth;
  onError: (e: unknown) => void;
  /** A tile was pressed: the area is to be selected. */
  onSelect: (area: Area) => void;
}

const mapsLabel = (n: number) => (n === 1 ? "1 Karte" : `${n} Karten`);

interface TileProps extends Omit<MoreAreasProps, "areas" | "matrices" | "onSelect"> {
  area: Area;
  filled: number | null;
  onSelect: (area: Area) => void;
}

/** One area as a button: its name, the number of its maps and the plan of its matrix, read once the tile came into view. */
function Tile({ area, filled, headers, areaFallback, revision, cache, colorOf, depth, onError, onSelect }: TileProps) {
  const box = useRef<HTMLButtonElement>(null);
  const visible = useVisible(box, true);
  // Once seen, the plan stays: scrolling back does not read the matrix again.
  const [seen, setSeen] = useState(visible);
  useEffect(() => {
    if (visible) setSeen(true);
  }, [visible]);
  const matrix = useMatrix(seen || visible ? area.matrix : null, revision, onError);
  return (
    <button ref={box} type="button" data-more-area={area.key} className={styles.tile} onClick={() => onSelect(area)}>
      <span className={styles.name}>{area.name}</span>
      {filled !== null && <span className={styles.count}>{mapsLabel(filled)}</span>}
      <span className={styles.plan} aria-hidden="true">
        {matrix && <AreaGrid matrix={matrix} headers={headers} areaFallback={areaFallback} cellSize={32} colorOf={colorOf} cache={cache} depth={depth} label={area.name} still />}
      </span>
    </button>
  );
}

/** „Weitere Gebiete“: the areas with a map of their own as tiles; a tile selects its area. Memoised with the world map panel. */
export const MoreAreas = memo(function MoreAreas({ areas, matrices, onSelect, ...rest }: MoreAreasProps) {
  if (areas.length === 0) return <EmptyState what="Keine weiteren Gebiete." why="Jedes Gebiet liegt auf einer Weltkarte." />;
  return (
    <div className={styles.tiles}>
      {areas.map((area) => (
        <Tile key={area.key} area={area} filled={matrices.find((m) => m.index === area.matrix)?.filled ?? null} onSelect={onSelect} {...rest} />
      ))}
    </div>
  );
});
