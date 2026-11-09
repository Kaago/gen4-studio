import type { EdgePlace, EdgeSetJson, EdgeSetOp, TilesetDto } from "../../api.world";
import { tileParts } from "../../world/paint/ops";

/** The name of each place in the UI, as the kernel names it in its messages („Ecke innen NW“). */
export const PLACE_LABEL: Record<EdgePlace, string> = {
  center: "Mitte",
  n: "Kante N",
  e: "Kante O",
  s: "Kante S",
  w: "Kante W",
  outer_nw: "Ecke außen NW",
  outer_ne: "Ecke außen NO",
  outer_sw: "Ecke außen SW",
  outer_se: "Ecke außen SO",
  inner_nw: "Ecke innen NW",
  inner_ne: "Ecke innen NO",
  inner_sw: "Ecke innen SW",
  inner_se: "Ecke innen SO",
  diag_nwse: "Diagonale NW–SO",
  diag_nesw: "Diagonale NO–SW",
};

/**
 * The places in the editor's grid (rule „Kantensatz-Editor“): the 3 × 3 block of the area with its edges and outer corners, the 2 × 2
 * block of the inner corners and the row of the two diagonals; null fills a row up to three.
 */
export const PLACE_GRID: (EdgePlace | null)[][] = [
  ["outer_nw", "n", "outer_ne"],
  ["w", "center", "e"],
  ["outer_sw", "s", "outer_se"],
  ["inner_nw", "inner_ne", null],
  ["inner_sw", "inner_se", null],
  ["diag_nwse", "diag_nesw", null],
];

const DIAGONALS: readonly EdgePlace[] = ["diag_nwse", "diag_nesw"];

/** A token in the form the kernel compares (`parse_tile`): lowercase; a token outside the grammar as typed. */
const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** The set with `token` appended to the variants of `place`; a token the place holds already is not appended twice. */
export function withVariant(set: EdgeSetJson, place: EdgePlace, token: string): EdgeSetJson {
  const variants = set.places[place] ?? [];
  if (variants.some((v) => same(v, token))) return set;
  return { ...set, places: { ...set.places, [place]: [...variants, token] } };
}

/** The set without `token` in `place`; a place left without variants is left out (as the file writes empty places). */
export function withoutVariant(set: EdgeSetJson, place: EdgePlace, token: string): EdgeSetJson {
  const variants = (set.places[place] ?? []).filter((v) => !same(v, token));
  const places = { ...set.places };
  if (variants.length > 0) places[place] = variants;
  else delete places[place];
  return { ...set, places };
}

/** How many of the 13 places and of the 2 diagonals hold at least one tile. */
export function completeness(set: EdgeSetJson): { places: number; diagonals: number } {
  let places = 0;
  let diagonals = 0;
  for (const [place, variants] of Object.entries(set.places) as [EdgePlace, string[] | undefined][]) {
    if (!variants || variants.length === 0) continue;
    if (DIAGONALS.includes(place)) diagonals += 1;
    else places += 1;
  }
  return { places, diagonals };
}

/** A new, empty set with its outside tile (the block rule left out: the upper cell). */
export function newSet(name: string, outside: string): EdgeSetJson {
  return { name, outside, places: {} };
}

/** The distinct tiles (indices) of the set's places in ascending order, without the outside; tokens outside the grammar are skipped. */
export function placeTiles(set: EdgeSetJson): number[] {
  const tiles = new Set<number>();
  for (const variants of Object.values(set.places)) {
    for (const token of variants ?? []) {
      const parts = tileParts(token);
      if (parts) tiles.add(Number.parseInt(parts.digits, 16));
    }
  }
  return [...tiles].sort((a, b) => a - b);
}

/** Every tile (index) of the set (rule „Wand“, Ruling 13): the tiles of the places and the outside, distinct, ascending. */
export function setTiles(set: EdgeSetJson): number[] {
  const tiles = new Set(placeTiles(set));
  const outside = tileParts(set.outside);
  if (outside) tiles.add(Number.parseInt(outside.digits, 16));
  return [...tiles].sort((a, b) => a - b);
}

/** The tileset with edge set operations applied as the kernel applies them: `put` replaces the set of its name in place or appends, `remove` drops it. */
export function applyEdgeOps(tileset: TilesetDto, ops: readonly EdgeSetOp[]): TilesetDto {
  let sets = tileset.tileset.edge_sets ?? [];
  for (const op of ops) {
    if ("put" in op) {
      const at = sets.findIndex((s) => s.name === op.put.name);
      sets = at < 0 ? [...sets, op.put] : sets.map((s, i) => (i === at ? op.put : s));
    } else {
      sets = sets.filter((s) => s.name !== op.remove.name);
    }
  }
  return { ...tileset, tileset: { ...tileset.tileset, edge_sets: sets } };
}
