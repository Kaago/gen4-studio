import type { Tool } from "../ui/frame/shellState";
import type { IconName } from "../ui/primitives";
import { lockedText } from "../world/events";
import type { EventKind, EventLock } from "../world/events";

/** One tool of the toolbar in the 3D view: what it is called, its letter, and what the status line says while it is active. */
export interface ToolSpec {
  id: Tool;
  label: string;
  /** The letter that switches to the tool, shown in the corner of its button. */
  key: string;
  icon: IconName;
  /** The group of the toolbar the tool sits in: selecting, painting the ground, the events, the buildings. */
  group: 0 | 1 | 2 | 3;
  /** The second line of the tooltip: what a click in the scene does. */
  hint: string;
  /** Key and effect pairs for the status line ("Klick" + "wählt"). */
  statusHints: { keys: string; does: string }[];
}

export const TOOLS: ToolSpec[] = [
  {
    id: "select",
    label: "Auswählen",
    key: "V",
    icon: "select",
    group: 0,
    hint: "Wählt Marker, Gebäude und Karten in der Szene. Ziehen verschiebt das Gewählte. Malt nie.",
    statusHints: [
      { keys: "Klick", does: "wählt" },
      { keys: "Ziehen", does: "verschiebt" },
    ],
  },
  {
    id: "paint",
    label: "Gelände malen",
    key: "B",
    icon: "paint",
    group: 1,
    hint: "Klick setzt die Kollision der Kachel, Umschalt-Klick löscht sie.",
    statusHints: [
      { keys: "Klick", does: "setzt Kollision" },
      { keys: "⇧Klick", does: "löscht" },
    ],
  },
  {
    id: "figure",
    label: "Figur",
    key: "F",
    icon: "figure",
    group: 2,
    hint: "Klick auf eine Kachel stellt dort eine Figur hin und wählt wieder Auswählen.",
    statusHints: [
      { keys: "Klick", does: "stellt eine Figur hin" },
      { keys: "Esc", does: "bricht ab" },
    ],
  },
  {
    id: "door",
    label: "Tür",
    key: "T",
    icon: "door",
    group: 2,
    hint: "Klick auf eine Kachel legt dort eine Tür an (einen Warp) und wählt wieder Auswählen.",
    statusHints: [
      { keys: "Klick", does: "legt eine Tür an" },
      { keys: "Esc", does: "bricht ab" },
    ],
  },
  {
    id: "trigger",
    label: "Auslöser",
    key: "A",
    icon: "trigger",
    group: 2,
    hint: "Klick auf eine Kachel legt dort einen Auslöser an (einen Trigger) und wählt wieder Auswählen.",
    statusHints: [
      { keys: "Klick", does: "legt einen Auslöser an" },
      { keys: "Esc", does: "bricht ab" },
    ],
  },
  {
    id: "sign",
    label: "Schild",
    key: "S",
    icon: "sign",
    group: 2,
    hint: "Klick auf eine Kachel stellt dort ein Schild auf (ein Hintergrund-Event) und wählt wieder Auswählen.",
    statusHints: [
      { keys: "Klick", does: "stellt ein Schild auf" },
      { keys: "Esc", does: "bricht ab" },
    ],
  },
  {
    id: "building",
    label: "Gebäude",
    key: "G",
    icon: "building",
    group: 3,
    hint: "Klick auf eine Kachel setzt dort ein Gebäude und wählt wieder Auswählen.",
    statusHints: [
      { keys: "Klick", does: "setzt ein Gebäude" },
      { keys: "Esc", does: "bricht ab" },
    ],
  },
];

/** The tooltip hint of „Gelände malen“ on a world map of the text project (6.4a); a binary map keeps the collision hint of `TOOLS`. */
export const WORLD_PAINT_HINT = "Malt Kacheln, Höhen und Rampen auf die gewählte Welt-Karte.";

const WORLD_TOOLS: ToolSpec[] = TOOLS.map((t) => (t.id === "paint" ? { ...t, hint: WORLD_PAINT_HINT } : t));

/** The tools as the toolbar shows them for the selected map: on a world map „Gelände malen“ says that it paints tiles, heights and ramps. */
export function toolsFor(worldMap: boolean): ToolSpec[] {
  return worldMap ? WORLD_TOOLS : TOOLS;
}

/** The tool a typed letter switches to (upper or lower case), or null for any other key. */
export function toolForKey(key: string): Tool | null {
  const upper = key.toUpperCase();
  return TOOLS.find((t) => t.key === upper)?.id ?? null;
}

/** The event class a tool adds (figure, door, trigger, sign); the other tools have none. */
export const EVENT_TOOL_KIND: Partial<Record<Tool, EventKind>> = { figure: "object", door: "warp", trigger: "coord", sign: "bg" };

/** Whether the tool adds an event: those are locked with the event file. */
export const isEventTool = (tool: Tool): boolean => EVENT_TOOL_KIND[tool] !== undefined;

/** Why the event tools wait (the toolbar and the „+“ of the scene say it): a locked event file, or no shown place; null when they may add. */
export const eventToolsBlocked = (lock: EventLock | null, eventFile: number | null): string | null =>
  lock ? lockedText(lock.reason) : eventFile === null ? "Erst einen Ort wählen." : null;
