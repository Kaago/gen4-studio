import type { HistoryDto } from "../api";

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

/** The labels the commands build with `format!`; the id stays in the text. */
const PATTERNS: readonly [RegExp, (match: RegExpMatchArray) => string][] = [
  [/^assign header (\d+) to matrix (\d+)$/, (m) => `Header ${m[1]} Matrix ${m[2]} zuweisen`],
  [/^connect warp to header (\d+)$/, (m) => `Warp mit Header ${m[1]} verbinden`],
];

/** The German name of one undo step; a label this table does not know (or an empty one) reads "Änderung". */
export function historyLabel(label: string): string {
  const known = KNOWN[label];
  if (known !== undefined) return known;
  for (const [pattern, text] of PATTERNS) {
    const match = pattern.exec(label);
    if (match !== null) return text(match);
  }
  return "Änderung";
}

export const germanHistory = (history: HistoryDto): HistoryDto => ({ undo: history.undo.map(historyLabel), redo: history.redo.map(historyLabel) });
