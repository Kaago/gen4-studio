import type { Tool } from "../ui/frame/shellState";
import type { IconName } from "../ui/primitives";
import { t } from "../i18n";
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
  /** Key and effect pairs for the status line („Klick“ + „wählt“). */
  statusHints: { keys: string; does: string }[];
}

/** A key or gesture of a status line, in the language shown („Klick“, „Ziehen“, „⇧Klick“ …); the paint tools name theirs too. */
export type KeyWord = "click" | "shiftClick" | "drag" | "shiftDrag" | "rightClick" | "esc";
export const keyWord = (word: KeyWord): string => t(`workbench:keyWords.${word}`);

/** What a tool is apart from its words; label, hint and the two status pairs are `tools.<id>.*`, read when asked. */
interface ToolDef {
  id: Tool;
  key: string;
  icon: IconName;
  group: ToolSpec["group"];
  /** The keys of the two status pairs; what they do is `tools.<id>.does1` and `does2`. */
  keys: readonly [KeyWord, KeyWord];
}

const DEFS: readonly ToolDef[] = [
  { id: "select", key: "V", icon: "select", group: 0, keys: ["click", "drag"] },
  { id: "paint", key: "B", icon: "paint", group: 1, keys: ["click", "shiftClick"] },
  { id: "figure", key: "F", icon: "figure", group: 2, keys: ["click", "esc"] },
  { id: "door", key: "T", icon: "door", group: 2, keys: ["click", "esc"] },
  { id: "trigger", key: "A", icon: "trigger", group: 2, keys: ["click", "esc"] },
  { id: "sign", key: "S", icon: "sign", group: 2, keys: ["click", "esc"] },
  { id: "building", key: "G", icon: "building", group: 3, keys: ["click", "esc"] },
];

/** A tool whose words are looked up each time they are read: the module loads before i18next has a language. */
function toolSpec(def: ToolDef, hint: () => string = () => t(`workbench:tools.${def.id}.hint`)): ToolSpec {
  return {
    id: def.id,
    key: def.key,
    icon: def.icon,
    group: def.group,
    get label() {
      return t(`workbench:tools.${def.id}.label`);
    },
    get hint() {
      return hint();
    },
    get statusHints() {
      return [
        { keys: keyWord(def.keys[0]), does: t(`workbench:tools.${def.id}.does1`) },
        { keys: keyWord(def.keys[1]), does: t(`workbench:tools.${def.id}.does2`) },
      ];
    },
  };
}

export const TOOLS: ToolSpec[] = DEFS.map((def) => toolSpec(def));

/** The tooltip hint of „Gelände malen“ on a world map of the text project (6.4a); a binary map keeps the collision hint of `TOOLS`. */
export const worldPaintHint = (): string => t("workbench:tools.worldPaintHint");

const WORLD_TOOLS: ToolSpec[] = DEFS.map((def, i) => (def.id === "paint" ? toolSpec(def, worldPaintHint) : TOOLS[i]!));

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
  lock ? lockedText(lock.reason) : eventFile === null ? t("workbench:tools.noPlace") : null;
