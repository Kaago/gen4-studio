import type { HistoryDto } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";

/** What the kernel calls a change in the undo history (it is not translated there) and what the hinge shows. */
const KNOWN: Readonly<Record<string, string>> = {
  paint: "Malen",
  "edit matrix cell": "Rasterfeld geändert",
  "edit matrix sections": "Raster-Abschnitte geändert",
  "resize matrix": "Raster-Größe geändert",
  "rename matrix": "Raster umbenannt",
  "build area": "Gebiet gebaut",
  "extend area": "Gebiet erweitert",
  "build room": "Raum gebaut",
  "door tile": "Türkachel gesetzt",
  "add event": "Event hinzugefügt",
  "delete event": "Event gelöscht",
  "move event": "Event verschoben",
  "edit event": "Event geändert",
  "add building": "Gebäude hinzugefügt",
  "delete building": "Gebäude gelöscht",
  "change building model": "Gebäudemodell gewechselt",
  "edit building": "Gebäude geändert",
  "edit header": "Header geändert",
  "edit area data": "Area-Data geändert",
  "edit text": "Text geändert",
  "add message": "Nachricht hinzugefügt",
  "remove message": "Nachricht gelöscht",
  "set permission": "Begehbarkeit geändert",
};

/** Bauen speaks of world maps and places: the kernel labels that would name a matrix or a header. */
const KNOWN_BUILDING: Readonly<Record<string, string>> = {
  "edit matrix cell": "Zelle geändert",
  "edit matrix sections": "Abschnitte der Weltkarte geändert",
  "resize matrix": "Größe der Weltkarte geändert",
  "rename matrix": "Weltkarte umbenannt",
  "edit header": "Ort geändert",
};

/** The parts of a map as `g4_kernel::map_import::Part::label` names them in an undo label, in German (Tüfteln, Bauen). */
const MAP_PARTS: Readonly<Record<string, readonly [string, string]>> = {
  model: ["Modell", "Modell"],
  permissions: ["Kollision", "Kollision"],
  buildings: ["Gebäude", "Gebäude"],
  terrain: ["BDHC", "Laufhöhen"],
  "background sound": ["Hintergrundklang", "Hintergrundklang"],
  "whole map": ["ganze Karte", "ganze Karte"],
};

/** "model and permissions and terrain" as "Modell, Kollision und BDHC" (an unknown part stays as it is). */
function mapParts(list: string, building: boolean): string {
  const parts = list.split(" and ").map((part) => MAP_PARTS[part]?.[building ? 1 : 0] ?? part);
  return parts.length < 2 ? parts.join("") : `${parts.slice(0, -1).join(", ")} und ${parts[parts.length - 1]}`;
}

/** The labels the commands build with `format!`; the id stays in the text. The second text is Bauen's. */
const PATTERNS: readonly [RegExp, (match: RegExpMatchArray) => string, (match: RegExpMatchArray) => string][] = [
  [/^assign header (\d+) to matrix (\d+)$/, (m) => `Header ${m[1]} Matrix ${m[2]} zuweisen`, (m) => `Ort ${m[1]} Weltkarte ${m[2]} zuweisen`],
  [/^connect warp to header (\d+)$/, (m) => `Warp mit Header ${m[1]} verbinden`, (m) => `Warp mit Ort ${m[1]} verbinden`],
  // `g4_kernel::map_import`: "map 0668: model and permissions and terrain imported".
  [/^map (\d+): (.+) imported$/, (m) => `Karte ${m[1]}: ${mapParts(m[2]!, false)} importiert`, (m) => `Karte ${m[1]}: ${mapParts(m[2]!, true)} importiert`],
];

/** What the kernel and the commands write: lowercase ASCII words. Anything else was named by the frontend (a German field label) already. */
const DEVELOPER_STRING = /^[a-z0-9 ]+$/;

/**
 * The German name of one undo step. Known kernel labels are translated (in Bauen without "Raster", "Matrix" and "Header"); a label
 * that is not a lowercase ASCII developer string (the editors commit with a German field label such as "Wetter") passes through; an
 * empty or unknown developer label reads "Änderung".
 */
export function historyLabel(label: string, depth: Depth): string {
  const known = (!depthAtLeast(depth, "tuefteln") ? KNOWN_BUILDING[label] : undefined) ?? KNOWN[label];
  if (known !== undefined) return known;
  for (const [pattern, text, building] of PATTERNS) {
    const match = pattern.exec(label);
    if (match !== null) return !depthAtLeast(depth, "tuefteln") ? building(match) : text(match);
  }
  return label !== "" && !DEVELOPER_STRING.test(label) ? label : "Änderung";
}

/** Translates both lists in the depth's words; the agents of the steps (`undo_agents`, `redo_agents`) stay aligned with them. */
export const germanHistory = (history: HistoryDto, depth: Depth): HistoryDto => ({
  ...history,
  undo: history.undo.map((label) => historyLabel(label, depth)),
  redo: history.redo.map((label) => historyLabel(label, depth)),
});
