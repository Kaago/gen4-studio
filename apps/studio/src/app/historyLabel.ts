import type { HistoryDto } from "../api";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";

/** What the kernel calls a change in the undo history (it is not translated there): the catalog key of what the hinge shows. */
const KNOWN_KEYS = {
  paint: "paint",
  "edit matrix cell": "editMatrixCell",
  "edit matrix sections": "editMatrixSections",
  "resize matrix": "resizeMatrix",
  "rename matrix": "renameMatrix",
  "build area": "buildArea",
  "extend area": "extendArea",
  "build room": "buildRoom",
  "door tile": "doorTile",
  "add event": "addEvent",
  "delete event": "deleteEvent",
  "move event": "moveEvent",
  "edit event": "editEvent",
  "add building": "addBuilding",
  "delete building": "deleteBuilding",
  "change building model": "changeBuildingModel",
  "edit building": "editBuilding",
  "edit header": "editHeader",
  "edit area data": "editAreaData",
  "edit text": "editText",
  "add message": "addMessage",
  "remove message": "removeMessage",
  "set permission": "setPermission",
  "delete warp": "deleteWarp",
} as const;
const KNOWN: Readonly<Record<string, (typeof KNOWN_KEYS)[keyof typeof KNOWN_KEYS]>> = KNOWN_KEYS;

/** Bauen speaks of world maps and places: the kernel labels that would name a matrix or a header have their own words (`history.building`). */
const KNOWN_BUILDING: Readonly<Record<string, "editMatrixCell" | "editMatrixSections" | "resizeMatrix" | "renameMatrix" | "editHeader">> = {
  "edit matrix cell": "editMatrixCell",
  "edit matrix sections": "editMatrixSections",
  "resize matrix": "resizeMatrix",
  "rename matrix": "renameMatrix",
  "edit header": "editHeader",
};

/** The parts of a map as `g4_kernel::map_import::Part::label` names them in an undo label: the catalog key of each (Tüfteln, Bauen). */
const MAP_PARTS: Readonly<Record<string, "model" | "permissions" | "buildings" | "terrain" | "backgroundSound" | "wholeMap">> = {
  model: "model",
  permissions: "permissions",
  buildings: "buildings",
  terrain: "terrain",
  "background sound": "backgroundSound",
  "whole map": "wholeMap",
};

/** "model and permissions and terrain" as "Modell, Kollision und BDHC" (an unknown part stays as it is). */
function mapParts(list: string, building: boolean): string {
  const parts = list.split(" and ").map((part) => {
    const key = MAP_PARTS[part];
    if (key === undefined) return part;
    return building ? t(`app:history.mapParts.building.${key}`) : t(`app:history.mapParts.tech.${key}`);
  });
  return parts.length < 2 ? parts.join("") : t("common:list.and", { rest: parts.slice(0, -1).join(", "), last: parts[parts.length - 1] });
}

/** The labels the commands build with `format!`; the id stays in the text. The second text is Bauen's. */
const PATTERNS: readonly [RegExp, (match: RegExpMatchArray) => string, (match: RegExpMatchArray) => string][] = [
  [/^assign header (\d+) to matrix (\d+)$/, (m) => t("app:history.assignHeader", { header: m[1], matrix: m[2] }), (m) => t("app:history.building.assignHeader", { header: m[1], matrix: m[2] })],
  [/^connect warp to header (\d+)$/, (m) => t("app:history.connectWarp", { header: m[1] }), (m) => t("app:history.building.connectWarp", { header: m[1] })],
  // `g4_kernel::map_import`: "map 0668: model and permissions and terrain imported".
  [/^map (\d+): (.+) imported$/, (m) => t("app:history.mapImported", { map: m[1], parts: mapParts(m[2]!, false) }), (m) => t("app:history.mapImported", { map: m[1], parts: mapParts(m[2]!, true) })],
];

/** What the kernel and the commands write: lowercase ASCII words. Anything else was named by the frontend (a German field label) already. */
const DEVELOPER_STRING = /^[a-z0-9 ]+$/;

/**
 * The name of one undo step in the language shown. Known kernel labels are translated (in Bauen without "Raster", "Matrix" and "Header"); a label
 * that is not a lowercase ASCII developer string (the editors commit with a German field label such as "Wetter") passes through; an
 * empty or unknown developer label reads „Änderung“.
 */
export function historyLabel(label: string, depth: Depth): string {
  const building = !depthAtLeast(depth, "tuefteln") ? KNOWN_BUILDING[label] : undefined;
  if (building !== undefined) return t(`app:history.building.${building}`);
  const known = KNOWN[label];
  if (known !== undefined) return t(`app:history.known.${known}`);
  for (const [pattern, text, building] of PATTERNS) {
    const match = pattern.exec(label);
    if (match !== null) return !depthAtLeast(depth, "tuefteln") ? building(match) : text(match);
  }
  return label !== "" && !DEVELOPER_STRING.test(label) ? label : t("app:history.unknown");
}

/** Translates both lists in the depth's words; the agents of the steps (`undo_agents`, `redo_agents`) stay aligned with them. */
export const germanHistory = (history: HistoryDto, depth: Depth): HistoryDto => ({
  ...history,
  undo: history.undo.map((label) => historyLabel(label, depth)),
  redo: history.redo.map((label) => historyLabel(label, depth)),
});
