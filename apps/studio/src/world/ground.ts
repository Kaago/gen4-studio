import type { MatrixDto } from "../api";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { MAP_SIZE, MODEL_UNITS_PER_TILE, cellY, cellYAtTile } from "./layout";
import type { Loaded } from "./scene/types";

/**
 * The walking surface of the shown matrix, as overlay, markers, picking, event drag and draft see it: each tile is a level plate at the
 * height of its centre (the cell's altitude plus the BDHC height there). Buildings and terrain models stay on the matrix height.
 */
export interface Ground {
  /** Scene height of the walking surface at absolute tile (x, z): the cell's altitude plus the BDHC height of the tile centre / 16. */
  y(x: number, z: number): number;
  /** BDHC height of the tile centre in world units; null without map, plate or outside. */
  units(x: number, z: number): number | null;
  /** Distinct scene heights of every tile of the matrix, highest first. */
  levels(): readonly number[];
}

/** The distinct values of `heights`, highest first. */
const distinctDescending = (heights: Iterable<number>) => [...new Set(heights)].sort((a, b) => b - a);

/**
 * The ground of `matrix` from the heights the loaded maps carry now (`MapDto.heights`, 1024 tile centres in world units): a map's
 * `heights` are read when the ground is made, so a map refetched with new heights needs a new ground. A tile without a height (no
 * plate, an unreadable BDHC, a map that is not loaded, an empty cell) lies on its cell's altitude.
 */
export function createGround(matrix: MatrixDto, loaded: ReadonlyMap<number, Loaded>): Ground {
  const cells = matrix.width * matrix.height;
  const perCell: (readonly (number | null)[] | undefined)[] = Array.from({ length: cells }, (_, i) => {
    const id = matrix.maps[i];
    return id === null || id === undefined ? undefined : loaded.get(id)?.map.heights;
  });
  const units = (x: number, z: number): number | null => {
    const col = Math.floor(x / MAP_SIZE);
    const row = Math.floor(z / MAP_SIZE);
    if (col < 0 || row < 0 || col >= matrix.width || row >= matrix.height) return null;
    const value = perCell[row * matrix.width + col]?.[(Math.floor(z) - row * MAP_SIZE) * MAP_SIZE + (Math.floor(x) - col * MAP_SIZE)];
    return typeof value === "number" ? value : null;
  };
  let levels: number[] | null = null;
  return {
    units,
    y: (x, z) => cellYAtTile(matrix, x, z) + (units(x, z) ?? 0) / MODEL_UNITS_PER_TILE,
    levels: () => {
      if (levels) return levels;
      const seen = new Set<number>();
      for (let i = 0; i < cells; i++) {
        const base = cellY(matrix, i % matrix.width, Math.floor(i / matrix.width));
        const heights = perCell[i];
        seen.add(base);
        if (!heights) continue;
        for (let t = 0; t < MAP_SIZE * MAP_SIZE; t++) {
          const value = heights[t];
          if (typeof value === "number") seen.add(base + value / MODEL_UNITS_PER_TILE);
        }
      }
      levels = distinctDescending(seen.size ? seen : [0]);
      return levels;
    },
  };
}

/** The ground of the matrix height alone (no BDHC): every tile on its cell's altitude; without a matrix everything lies on 0. */
export function flatGround(matrix: MatrixDto | null): Ground {
  if (!matrix) return { y: () => 0, units: () => null, levels: () => [0] };
  // Without loaded maps no tile has a height: the matrix height alone.
  return createGround(matrix, new Map());
}

/** World units per height step (half a tile), the step the Bauen words count in. */
const UNITS_PER_STEP = 8;

/** `n` with at most `digits` decimals, the decimal point of the language shown and no trailing zeros of a whole number („0,5“, „4“; „0.5“ in English). */
const decimal = (n: number, digits: number) => {
  const rounded = Number(n.toFixed(digits));
  return (Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(digits)).replace(".", t("world:ground.decimalPoint"));
};

/**
 * The height of a tile in words: Bauen (and Erzählen) in steps of 8 E with one decimal („Höhe 4 Stufen“, „Höhe 1 Stufe“, „Höhe 0,5 Stufen“),
 * Tüfteln the world units first („Höhe 32 E (4 Stufen)“, „Höhe 3,99 E (0,5 Stufen)“); without a height „keine Höhe“.
 */
export function heightWords(units: number | null, depth: Depth): string {
  if (units === null) return t("world:ground.noHeight");
  // One step (or minus one) takes the singular; the count is the rounded value the words show.
  const count = Math.abs(Number((units / UNITS_PER_STEP).toFixed(1)));
  const steps = decimal(units / UNITS_PER_STEP, 1);
  if (!depthAtLeast(depth, "tuefteln")) return t("world:ground.heightSteps", { count, steps });
  return t("world:ground.heightUnitsSteps", { count, steps, units: decimal(units, 2) });
}
