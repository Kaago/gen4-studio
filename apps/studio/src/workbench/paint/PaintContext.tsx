import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useState } from "react";
import type { JSX, MutableRefObject, ReactNode } from "react";
import { getMap } from "../../api";
import { useTextProject } from "../../textproject/TextProjectContext";
import type { PaintOptions, PaintSub } from "../../world/paint/ops";
import { useMatrixSelection, useWorkbench } from "../WorkbenchContext";

/** What „Gelände malen“ paints with on a world map: the sub-tool and its options. */
export interface PaintState extends PaintOptions {
  sub: PaintSub;
}

export interface PaintModel {
  state: PaintState;
  set(patch: Partial<PaintState>): void;
  /** the selected cell's map when it is a world map of the text project */
  target: number | null;
}

/** The brush with the first tile of a package on the ground layer, a ramp of one step uphill to the east. The palette picks a real tile. */
export const INITIAL_PAINT: PaintState = { sub: "brush", layer: 0, tile: "0000", ramp: { dir: "e", steps: 1 }, lower: false };

const NONE: PaintModel = { state: INITIAL_PAINT, set: () => {}, target: null };

/** The paint model; outside a `PaintProvider` (tests, the gallery) the brush without a target. Exported for tests and the gallery. */
export const PaintContext = createContext<PaintModel>(NONE);

export function usePaint(): PaintModel {
  return useContext(PaintContext);
}

/**
 * The map of the selected cell (the inspector's matrix cell, on the shown matrix: the one the scene frames), or null without one or on
 * an empty cell.
 */
function useSelectedMap(): number | null {
  const cell = useMatrixSelection();
  const { matrix } = useWorkbench();
  if (cell === null || matrix === null || matrix === undefined || cell.matrix !== matrix.index) return null;
  if (cell.x >= matrix.width || cell.y >= matrix.height) return null;
  return matrix.maps[cell.y * matrix.width + cell.x] ?? null;
}

/**
 * Holds the paint state of „Gelände malen“ and finds its target: the selected cell's map when it carries a world document
 * (`MapDto.world`, read with `get_map`) in a text project (`useTextProject().active`; the DSPRE layout has no world maps). The flag is
 * read again when the scene reloads or refetches that map (a world created or adopted, undo, redo). `bridge` hands the model to the app's
 * keyboard shortcuts, which live above the workbench.
 */
export function PaintProvider({ children, bridge }: { children: ReactNode; bridge?: MutableRefObject<PaintModel | null> }): JSX.Element {
  const [state, setState] = useState<PaintState>(INITIAL_PAINT);
  const set = useCallback((patch: Partial<PaintState>) => setState((s) => ({ ...s, ...patch })), []);
  const textProject = useTextProject().active;
  const map = useSelectedMap();
  const { bus } = useWorkbench();
  const asked = textProject ? map : null;
  // The map last read and whether it is a world map; a read for another map is dropped.
  const [world, setWorld] = useState<{ map: number; world: boolean } | null>(null);
  useEffect(() => {
    if (asked === null) return;
    let live = true;
    const read = () =>
      void getMap(asked).then(
        (dto) => live && setWorld({ map: asked, world: dto.world === true }),
        () => live && setWorld({ map: asked, world: false }),
      );
    read();
    const off = bus?.subscribe(
      (e) => ((e.topic === "mapModelReload" || e.topic === "mapRefetch") && e.map === asked) || e.topic === "matrixData",
      read,
    );
    return () => {
      live = false;
      off?.();
    };
  }, [asked, bus]);
  const target = asked !== null && world?.map === asked && world.world ? asked : null;
  const model = useMemo<PaintModel>(() => ({ state, set, target }), [state, set, target]);
  useLayoutEffect(() => {
    if (!bridge) return;
    bridge.current = model;
    return () => {
      if (bridge.current === model) bridge.current = null;
    };
  }, [bridge, model]);
  return <PaintContext.Provider value={model}>{children}</PaintContext.Provider>;
}
