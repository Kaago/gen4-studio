import type { WorldChange, WorldEditRequest, WorldJson } from "../../api.world";

/**
 * The pure model of painting a world map (6.4a): the cells a sub-tool covers, the tokens it turns and flips, and the one
 * `world_edit` request of a stroke. Tokens follow the kernel's grammar (`g4_kernel::world::grid`): a tile is `.` or four hex
 * digits, optionally `r1` to `r3`, then optionally `f`, written lowercase; a ramp is `.` or `n`/`e`/`s`/`w` with 1 or 2 steps.
 * Grids are indexed `[z][x]`, cells are `[x, z]` like the kernel's.
 */

/** The sub-tools of „Gelände malen“. */
export type PaintSub = "brush" | "rect" | "fill" | "erase" | "pick" | "height" | "ramp";
/** A map-local cell. */
export type Cell = [x: number, z: number];
/** The uphill direction of a ramp. */
export type RampDir = "n" | "e" | "s" | "w";

/** What a stroke paints with: the active layer, the chosen tile token, the ramp and whether the stroke lowers (− / removes). */
export interface PaintOptions {
  layer: number;
  tile: string;
  ramp: { dir: RampDir; steps: 1 | 2 };
  lower: boolean;
}

/** The cells of a world as grids: `tiles[layer][z][x]` (lowercase tokens), `heights[z][x]`, `ramps[z][x]` of the ground. */
export interface WorldCells {
  tiles: string[][][];
  heights: number[][];
  ramps: string[][];
}

export const HEIGHT_MIN = -16;
export const HEIGHT_MAX = 62;

/** Cells per side of a map. */
const SIZE = 32;
const EMPTY = ".";

function grid<T>(rows: string[] | undefined, fallback: T, read: (token: string) => T): T[][] {
  return Array.from({ length: SIZE }, (_, z) => {
    const tokens = rows?.[z]?.split(" ") ?? [];
    return Array.from({ length: SIZE }, (_, x) => (tokens[x] === undefined ? fallback : read(tokens[x])));
  });
}

/** The grids of `world`; missing ground rows read as height 0 and no ramp. */
export function cellsOf(world: WorldJson): WorldCells {
  const ground = world.layers[0];
  return {
    tiles: world.layers.map((layer) => grid(layer.tiles, EMPTY, (t) => t.toLowerCase())),
    heights: grid(ground?.heights, 0, (t) => Number.parseInt(t, 10) || 0),
    ramps: grid(ground?.ramps, EMPTY, (t) => t.toLowerCase()),
  };
}

/** The cells of the line from `a` to `b` (Bresenham), both ends included, each once, 8-connected. */
export function lineCells(a: Cell, b: Cell): Cell[] {
  const [x1, z1] = b;
  let [x, z] = a;
  const dx = Math.abs(x1 - x);
  const dz = -Math.abs(z1 - z);
  const sx = x < x1 ? 1 : -1;
  const sz = z < z1 ? 1 : -1;
  let err = dx + dz;
  const out: Cell[] = [];
  for (;;) {
    out.push([x, z]);
    if (x === x1 && z === z1) return out;
    const e2 = 2 * err;
    if (e2 >= dz) {
      err += dz;
      x += sx;
    }
    if (e2 <= dx) {
      err += dx;
      z += sz;
    }
  }
}

/** The cells of the rectangle spanned by `a` and `b` (either corners), row by row. */
export function rectCells(a: Cell, b: Cell): Cell[] {
  const [x0, x1] = [Math.min(a[0], b[0]), Math.max(a[0], b[0])];
  const [z0, z1] = [Math.min(a[1], b[1]), Math.max(a[1], b[1])];
  const out: Cell[] = [];
  for (let z = z0; z <= z1; z++) for (let x = x0; x <= x1; x++) out.push([x, z]);
  return out;
}

const inside = ([x, z]: Cell) => x >= 0 && x < SIZE && z >= 0 && z < SIZE;

/** The 4-connected cells around `at` on `layer` with exactly the token of `at` (a turned or mirrored copy is another token). */
export function fillCells(cells: WorldCells, layer: number, at: Cell): Cell[] {
  const tiles = cells.tiles[layer];
  if (!tiles || !inside(at)) return [];
  const token = tiles[at[1]][at[0]];
  const seen = new Set<number>([at[1] * SIZE + at[0]]);
  const out: Cell[] = [];
  const stack: Cell[] = [at];
  while (stack.length > 0) {
    const cell = stack.pop() as Cell;
    out.push(cell);
    const [x, z] = cell;
    for (const next of [
      [x + 1, z],
      [x - 1, z],
      [x, z + 1],
      [x, z - 1],
    ] as Cell[]) {
      const id = next[1] * SIZE + next[0];
      if (inside(next) && !seen.has(id) && tiles[next[1]][next[0]] === token) {
        seen.add(id);
        stack.push(next);
      }
    }
  }
  return out;
}

/** The tile under `at`: on the active layer, else on the topmost layer that has one; null when every layer is empty there. */
export function pickedTile(cells: WorldCells, layer: number, at: Cell): string | null {
  const [x, z] = at;
  const own = cells.tiles[layer]?.[z]?.[x];
  if (own !== undefined && own !== EMPTY) return own;
  for (let l = cells.tiles.length - 1; l >= 0; l--) {
    const token = cells.tiles[l][z]?.[x];
    if (token !== undefined && token !== EMPTY) return token;
  }
  return null;
}

/** A tile token in parts; null for `.` or a token outside the grammar. */
function tileParts(token: string): { digits: string; rotate: number; flip: boolean } | null {
  const m = /^([0-9a-f]{4})(?:r([1-3]))?(f)?$/.exec(token.toLowerCase());
  return m ? { digits: m[1], rotate: m[2] ? Number(m[2]) : 0, flip: m[3] !== undefined } : null;
}

function tileToken(p: { digits: string; rotate: number; flip: boolean }): string {
  return `${p.digits}${p.rotate ? `r${p.rotate}` : ""}${p.flip ? "f" : ""}`;
}

/** The token one quarter turn further (`0041` → `0041r1` → … → `0041r3` → `0041`), the mirror kept; `.` stays. */
export function turnTile(token: string): string {
  const p = tileParts(token);
  return p ? tileToken({ ...p, rotate: (p.rotate + 1) % 4 }) : token;
}

/** The token mirrored or no longer mirrored; `.` stays. */
export function flipTile(token: string): string {
  const p = tileParts(token);
  return p ? tileToken({ ...p, flip: !p.flip }) : token;
}

const NEXT_DIR: Record<RampDir, RampDir> = { n: "e", e: "s", s: "w", w: "n" };

/** The ramp direction one quarter turn clockwise (n → e → s → w → n). */
export function turnRamp(dir: RampDir): RampDir {
  return NEXT_DIR[dir];
}

const VERBS: Record<PaintSub, string> = {
  brush: "Gemalt",
  rect: "Gemalt",
  fill: "Gemalt",
  erase: "Radiert",
  pick: "Aufgenommen",
  height: "Höhe geändert",
  ramp: "Rampe geändert",
};

/** The history text of a stroke: „Gemalt: Karte 0123 (3 Zellen)“. */
export function strokeLabel(sub: PaintSub, map: number, cells: number): string {
  const count = cells === 1 ? "1 Zelle" : `${cells} Zellen`;
  return `${VERBS[sub]}: Karte ${String(map).padStart(4, "0")} (${count})`;
}

/** `cells` without repeats, in first-seen order. */
function unique(cells: Cell[]): Cell[] {
  const seen = new Set<number>();
  return cells.filter(([x, z]) => {
    const id = z * SIZE + x;
    if (seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

/** The one request of a stroke (`cells` in stroke order, map-local), unchanged cells dropped; null when nothing changes. */
export function strokeRequest(
  map: number,
  sub: Exclude<PaintSub, "pick">,
  cells: Cell[],
  world: WorldCells,
  o: PaintOptions,
): { req: WorldEditRequest; skipped: number } | null {
  const own = unique(cells).filter(inside);
  const changes: WorldChange[] = [];
  let skipped = 0;
  if (sub === "height") {
    const groups = new Map<number, Cell[]>();
    for (const cell of own) {
      const target = world.heights[cell[1]][cell[0]] + (o.lower ? -1 : 1);
      if (target < HEIGHT_MIN || target > HEIGHT_MAX) {
        skipped++;
        continue;
      }
      const group = groups.get(target);
      if (group) group.push(cell);
      else groups.set(target, [cell]);
    }
    for (const [height, group] of groups) changes.push({ cells: group, height });
  } else if (sub === "ramp") {
    const ramp = o.lower ? EMPTY : `${o.ramp.dir}${o.ramp.steps}`;
    const changed = own.filter(([x, z]) => world.ramps[z][x] !== ramp);
    if (changed.length > 0) changes.push({ cells: changed, ramp });
  } else {
    const tile = sub === "erase" ? EMPTY : o.tile.toLowerCase();
    const tiles = world.tiles[o.layer];
    const changed = own.filter(([x, z]) => tiles?.[z][x] !== tile);
    if (changed.length > 0) changes.push({ layer: o.layer, cells: changed, tile });
  }
  const sent = changes.reduce((n, c) => n + c.cells.length, 0);
  if (sent === 0) return null;
  return { req: { map, label: strokeLabel(sub, map, sent), changes }, skipped };
}
