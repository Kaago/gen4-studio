import { t } from "../i18n";
import type { IconName } from "../ui/primitives";
import { eventColor, terrainColor } from "./colors";
import type { EventColorKey, TerrainKey } from "./colors";
import { patternShade } from "./scene/patterns";
import type { Pattern } from "./scene/patterns";

export interface TerrainEntry {
  key: TerrainKey;
  label: string;
  color: string;
  /** The pattern the layer „Gelände“ draws on cells of this class (rule „Gelände-Muster“). */
  pattern: Pattern;
  /** `#RRGGBB` of the pattern lines (the colour × 0.55, like the shader); null without a pattern. */
  patternColor: string | null;
}

export interface EventEntry {
  kind: EventColorKey;
  label: string;
  color: string;
  icon: IconName;
}

/** The terrain classes, in the order the legend lists them; `name` is the key of the class's label in the catalog. */
const TERRAIN: readonly { key: TerrainKey; name: "blocked" | "water" | "tallGrass" | "ledge" | "ice" | "special" | "unknown"; pattern: Pattern }[] = [
  { key: "blockiert", name: "blocked", pattern: "hatch" },
  { key: "wasser", name: "water", pattern: "waves" },
  { key: "hohes-gras", name: "tallGrass", pattern: "dots" },
  { key: "vorsprung", name: "ledge", pattern: "arrow" },
  { key: "eis", name: "ice", pattern: "crosshatch" },
  { key: "spezial", name: "special", pattern: "none" },
  { key: "unbekannt", name: "unknown", pattern: "none" },
];

/** The event kinds; a warp is what the editor's vocabulary calls a door. `name` is the key of the kind's label in the catalog. */
const EVENTS: readonly { kind: EventColorKey; name: "figure" | "door" | "trigger" | "sign" | "item"; icon: IconName }[] = [
  { kind: "figur", name: "figure", icon: "figure" },
  { kind: "warp", name: "door", icon: "door" },
  { kind: "ausloeser", name: "trigger", icon: "trigger" },
  { kind: "schild", name: "sign", icon: "sign" },
  { kind: "gegenstand", name: "item", icon: "items" },
];

/** What the legend of the scene lists: every terrain class (with its pattern) and event kind with its name and colour. */
export function legendEntries(): { terrain: TerrainEntry[]; events: EventEntry[] } {
  return {
    terrain: TERRAIN.map(({ key, name, pattern }) => {
      const color = terrainColor(key);
      return { key, label: t(`world:legend.terrain.${name}`), pattern, color, patternColor: pattern === "none" ? null : patternShade(color) };
    }),
    events: EVENTS.map(({ kind, name, icon }) => ({ kind, label: t(`world:legend.events.${name}`), icon, color: eventColor(kind) })),
  };
}
