import { useId, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { gameLabel } from "../app/gameLabel";
import { ListPane } from "../ui/frame/ListPane";
import { useShell } from "../ui/frame/ShellContext";
import { WORLD_SPLIT_DEFAULT } from "../ui/frame/shellState";
import { Splitter } from "../ui/primitives";
import { SceneHierarchy } from "./SceneHierarchy";
import type { SceneHierarchyProps } from "./SceneHierarchy";
import { WorldExplorer } from "./WorldExplorer";
import type { WorldExplorerProps } from "./WorldExplorer";
import styles from "./WorldList.module.css";

export type WorldListProps = Omit<WorldExplorerProps, "folded" | "onFold"> & {
  /** Whether the list column shows or is folded to its grip (the shell remembers it per area). */
  listOpen: boolean;
  onToggleList: () => void;
  /** What „In dieser Szene“ shows; its callbacks and arrays must keep their identity (its rows are memoised on them). */
  scene: SceneHierarchyProps;
};

/** Neither part of the list is ever lower than this (px). */
const SECTION_MIN = 96;
/** The height of the splitter's grab area (`--g4-raum-8`). */
const LINE = 8;

/** The places' height in px for a share of `height`, and the bounds: 0.2..0.8 of it, and at least `SECTION_MIN` left for each part. */
function bounds(height: number, share: number): { value: number; min: number; max: number } {
  let min = Math.max(Math.round(height * 0.2), SECTION_MIN);
  let max = Math.min(Math.round(height * 0.8), height - SECTION_MIN - LINE);
  // A list too low for both minimums: the splitter has nowhere to go.
  if (min > max) min = max = Math.max(0, Math.round(height / 2));
  return { value: Math.min(max, Math.max(min, Math.round(height * share))), min, max };
}

/**
 * The list „Welt“: above, the places (the world tree, foldable to one row); below, „In dieser Szene“, what the shown area or room
 * holds; between them a splitter. The places' share of the height (0.2 to 0.8) and whether they are folded are the shell's, so a
 * new run finds them as they were; folded, the scene has the whole height. The share becomes px with the measured height of the
 * column (a drag, an arrow, Home and End size it, a double click halves it).
 */
export function WorldList({ listOpen, onToggleList, scene, ...places }: WorldListProps) {
  const { t } = useTranslation("workbench");
  const { state, setWorldListSplit, setPlacesFolded } = useShell();
  const share = state.worldListSplit;
  const folded = state.placesFolded;
  const placesId = useId();

  // The height of the column the two parts share; measured before the first paint and whenever it changes.
  const column = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);
  useLayoutEffect(() => {
    const el = column.current;
    if (!el) return;
    const measure = () => setHeight(el.getBoundingClientRect().height);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  const { value, min, max } = bounds(height, share);
  const onValue = (px: number) => {
    if (height > 0) setWorldListSplit(px / height);
  };

  return (
    <ListPane title={t("worldList.title")} subtitle={places.info ? gameLabel(places.info) : undefined} open={listOpen} onToggle={onToggleList}>
      <div ref={column} className={styles.column} data-world-list="">
        <div
          id={placesId}
          className={folded ? styles.foldedPlaces : styles.places}
          // The share as the basis of the places; the scene takes the rest.
          style={folded ? undefined : { flexBasis: `${share * 100}%` }} /* dyn */
        >
          <WorldExplorer {...places} folded={folded} onFold={setPlacesFolded} />
        </div>
        {!folded && (
          <Splitter
            orientation="horizontal"
            grows="down"
            label={t("worldList.splitter.label")}
            controls={placesId}
            value={value}
            min={min}
            max={max}
            onValue={onValue}
            onDoubleClick={() => setWorldListSplit(WORLD_SPLIT_DEFAULT)}
            // The places grow as the pointer goes down; the splitter only takes differences.
            measure={(e) => e.clientY}
            title={t("worldList.splitter.title")}
            dataAttrs={{ "data-splitter": "world-list" }}
          />
        )}
        <section className={styles.scene} aria-label={t("sceneHierarchy.title")}>
          <SceneHierarchy {...scene} />
        </section>
      </div>
    </ListPane>
  );
}
