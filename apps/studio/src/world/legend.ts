import type { IconName } from "../ui/primitives";
import { eventColor, terrainColor } from "./colors";
import type { EventColorKey, TerrainKey } from "./colors";

export interface TerrainEntry {
  key: TerrainKey;
  label: string;
  color: string;
}

export interface EventEntry {
  kind: EventColorKey;
  label: string;
  color: string;
  icon: IconName;
}

/** Names of the terrain classes, in the order the legend lists them. */
const TERRAIN: readonly { key: TerrainKey; label: string }[] = [
  { key: "blockiert", label: "Blockiert" },
  { key: "wasser", label: "Wasser" },
  { key: "hohes-gras", label: "Hohes Gras" },
  { key: "vorsprung", label: "Vorsprung" },
  { key: "eis", label: "Eis" },
  { key: "spezial", label: "Spezial" },
  { key: "unbekannt", label: "Unbekannt" },
];

/** Names of the event kinds; a warp is what the editor's vocabulary calls a door. */
const EVENTS: readonly { kind: EventColorKey; label: string; icon: IconName }[] = [
  { kind: "figur", label: "Figur", icon: "figure" },
  { kind: "warp", label: "Tür", icon: "door" },
  { kind: "ausloeser", label: "Auslöser", icon: "trigger" },
  { kind: "schild", label: "Schild", icon: "sign" },
  { kind: "gegenstand", label: "Gegenstand", icon: "items" },
];

/** What the legend of the scene lists: every terrain class and event kind with its name and colour. */
export function legendEntries(): { terrain: TerrainEntry[]; events: EventEntry[] } {
  return {
    terrain: TERRAIN.map((t) => ({ ...t, color: terrainColor(t.key) })),
    events: EVENTS.map((e) => ({ ...e, color: eventColor(e.kind) })),
  };
}
