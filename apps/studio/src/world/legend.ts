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

/** Names of the terrain classes, in the order the legend lists them. */
const TERRAIN: readonly { key: TerrainKey; label: string; pattern: Pattern }[] = [
  { key: "blockiert", label: "Blockiert", pattern: "hatch" },
  { key: "wasser", label: "Wasser", pattern: "waves" },
  { key: "hohes-gras", label: "Hohes Gras", pattern: "dots" },
  { key: "vorsprung", label: "Vorsprung", pattern: "arrow" },
  { key: "eis", label: "Eis", pattern: "crosshatch" },
  { key: "spezial", label: "Spezial", pattern: "none" },
  { key: "unbekannt", label: "Unbekannt", pattern: "none" },
];

/** Names of the event kinds; a warp is what the editor's vocabulary calls a door. */
const EVENTS: readonly { kind: EventColorKey; label: string; icon: IconName }[] = [
  { kind: "figur", label: "Figur", icon: "figure" },
  { kind: "warp", label: "Tür", icon: "door" },
  { kind: "ausloeser", label: "Auslöser", icon: "trigger" },
  { kind: "schild", label: "Schild", icon: "sign" },
  { kind: "gegenstand", label: "Gegenstand", icon: "items" },
];

/** What the legend of the scene lists: every terrain class (with its pattern) and event kind with its name and colour. */
export function legendEntries(): { terrain: TerrainEntry[]; events: EventEntry[] } {
  return {
    terrain: TERRAIN.map((t) => {
      const color = terrainColor(t.key);
      return { ...t, color, patternColor: t.pattern === "none" ? null : patternShade(color) };
    }),
    events: EVENTS.map((e) => ({ ...e, color: eventColor(e.kind) })),
  };
}
