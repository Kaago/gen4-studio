import type { TileBehavior } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";

/** What the hover shows about one tile: raw bytes plus the decomp's behaviour name and class (null when unnamed). */
export interface TileInfo {
  kind: number;
  collision: number;
  blocked: boolean;
  name: string | null;
  class: string | null;
}

/** Collision bit that blocks movement. */
const BLOCKED = 0x80;

export function describeTile(cell: [number, number], names: Map<number, TileBehavior>): TileInfo {
  const [kind, collision] = cell;
  const behavior = names.get(kind);
  return {
    kind,
    collision,
    blocked: (collision & BLOCKED) !== 0,
    name: behavior?.name ?? null,
    class: behavior?.class ?? null,
  };
}

/** The plain word of a behaviour class that says what the tile is to walk on; the others (a warp, a door, ...) have none. */
const CLASS_WORDS: Record<string, string> = { grass: "hohes Gras", water: "Wasser", ledge: "Vorsprung", ice: "Eis" };

/**
 * One-line status text. Tüfteln: "Karte 0187 · Kachel x 8, y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert"; Bauen leaves out the type
 * and the ROM constant and puts the tile first: "Kachel x 8, y 4 · Karte 0187 · blockiert" (or "begehbar", "hohes Gras" ...).
 */
export function tileSummary(map: number, x: number, y: number, info: TileInfo, depth: Depth): string {
  const where = `Karte ${String(map).padStart(4, "0")}`;
  const tile = `Kachel x ${x}, y ${y}`;
  if (!depthAtLeast(depth, "tuefteln")) return [tile, where, info.blocked ? "blockiert" : (CLASS_WORDS[info.class ?? ""] ?? "begehbar")].join(" · ");
  const hex = `0x${info.kind.toString(16).toUpperCase().padStart(2, "0")}`;
  const type = info.name === null ? hex : `${hex} ${info.name}`;
  return [where, tile, `Typ ${type}`, info.blocked ? "blockiert" : "begehbar"].join(" · ");
}
