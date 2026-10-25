import { dunkel, hell, szene } from "../ui/tokens/tokens";

const BLOCKED = 0x80;

export type TerrainKey = keyof typeof szene.gelaende;
export type EventColorKey = keyof typeof szene.ereignis;

/** `#RRGGBB` of a token colour (the tokens carry colours as numbers like 0xD65A00). The only place in `world/` that converts one. */
export function hexOf(rgb: number): string {
  return `#${rgb.toString(16).padStart(6, "0").toUpperCase()}`;
}

/** Terrain class of the behaviour class names (spec §8.1); a name that is not here (or none) is `unbekannt`. */
const CLASS_KEYS: Record<string, TerrainKey> = {
  block: "blockiert",
  water: "wasser",
  grass: "hohes-gras",
  ledge: "vorsprung",
  ice: "eis",
  sand: "spezial",
  slide: "spezial",
  warp: "spezial",
  door: "spezial",
  bridge: "spezial",
  furniture: "spezial",
};

/** Colour of a terrain class, as `#RRGGBB`. */
export function terrainColor(key: TerrainKey): string {
  return hexOf(szene.gelaende[key]);
}

/**
 * Overlay color for one permission cell: collision wins, then the behaviour class of the
 * terrain kind (null when the kind has no name). The raw `kind` byte no longer matters: the
 * class carries its meaning.
 */
export function permissionColor(_kind: number, collision: number, cls: string | null): string {
  if ((collision & BLOCKED) !== 0) return terrainColor("blockiert");
  return terrainColor((cls !== null && CLASS_KEYS[cls]) || "unbekannt");
}

/** Colour of an event kind, as `#RRGGBB`. */
export function eventColor(key: EventColorKey): string {
  return hexOf(szene.ereignis[key]);
}

/**
 * Outline of the selected marker or building: the halo on the light world surface, the core (white) on the dark one, where the
 * halo has only 2.6:1 and the spec (section 9) asks for 3:1.
 */
export function selectionColor(theme: "hell" | "dunkel"): string {
  return hexOf(theme === "dunkel" ? szene.auswahl.kern : szene.auswahl.halo);
}

/** A marker an error finding of the checks hits (`szene/status/fehler`, the same in both themes). */
export const FAULT_COLOR = hexOf(szene.status.fehler);

/** Wireframe box of a building whose model is missing (amber, like the class `vorsprung`). */
export const FALLBACK_BOX = terrainColor("vorsprung");

/** Background of the scene in a theme (`flaeche/welt`). */
export function sceneBackground(theme: "hell" | "dunkel"): string {
  return (theme === "dunkel" ? dunkel : hell)["flaeche/welt"];
}
