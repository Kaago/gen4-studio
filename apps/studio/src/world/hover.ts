import type { TileBehavior } from "../api";

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

/** One-line inspector text: "Karte 0187 · Kachel x 8, y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert". */
export function tileSummary(map: number, x: number, y: number, info: TileInfo): string {
  const hex = `0x${info.kind.toString(16).toUpperCase().padStart(2, "0")}`;
  const type = info.name === null ? hex : `${hex} ${info.name}`;
  return [`Karte ${String(map).padStart(4, "0")}`, `Kachel x ${x}, y ${y}`, `Typ ${type}`, info.blocked ? "blockiert" : "begehbar"].join(" · ");
}
