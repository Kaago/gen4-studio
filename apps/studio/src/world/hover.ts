import type { TileBehavior } from "../api";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { heightWords } from "./ground";

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

/** The plain word of a behaviour class that says what the tile is to walk on; the others (a warp, a door, ...) are just walkable. */
function classWord(cls: string | null): string {
  switch (cls) {
    case "grass":
      return t("world:hover.classes.grass");
    case "water":
      return t("world:hover.classes.water");
    case "ledge":
      return t("world:hover.classes.ledge");
    case "ice":
      return t("world:hover.classes.ice");
    default:
      return t("world:hover.walkable");
  }
}

/**
 * One-line status text, the BDHC height of the tile centre (`units`, see `heightWords`) last. Tüfteln (in German): „Karte 0187 · Kachel x 8,
 * y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert · Höhe 32 E (4 Stufen)“; Bauen leaves out the type and the ROM constant and puts the tile
 * first: „Kachel x 8, y 4 · Karte 0187 · blockiert · Höhe 4 Stufen“ (or „begehbar“, „hohes Gras“ ...).
 */
export function tileSummary(map: number, x: number, y: number, info: TileInfo, depth: Depth, units: number | null): string {
  const where = t("world:hover.map", { map: String(map).padStart(4, "0") });
  const tile = t("world:hover.tile", { x, y });
  const height = heightWords(units, depth);
  if (!depthAtLeast(depth, "tuefteln")) return [tile, where, info.blocked ? t("world:hover.blocked") : classWord(info.class), height].join(" · ");
  const hex = `0x${info.kind.toString(16).toUpperCase().padStart(2, "0")}`;
  const type = info.name === null ? hex : `${hex} ${info.name}`;
  return [where, tile, t("world:hover.type", { type }), info.blocked ? t("world:hover.blocked") : t("world:hover.walkable"), height].join(" · ");
}
