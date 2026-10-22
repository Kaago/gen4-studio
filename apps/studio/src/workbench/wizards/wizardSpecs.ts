import type { AreaCell, AreaReport, AreaSpec, EventFileChoice, HeaderSummary, MatrixDto } from "../../api";
import { headerLabel } from "../headers";
import { TERMS, id4 } from "../terms";

/** What the "Header wählen" step edits (shared by the area and the room wizard). */
export interface HeaderChoiceValue {
  header: number | null;
  /** Area data the header gets; null keeps the one it has. */
  areaDataId: number | null;
  eventFile: EventFileChoice;
}

/** A cell of the source matrix the user picked, with the map it holds. */
export interface PickedCell {
  x: number;
  y: number;
  map: number;
}

export interface AreaForm {
  name: string;
  /** The matrix the cells are picked from. */
  sourceMatrix: number | null;
  selected: PickedCell[];
  /** Maps picked by number instead of by cell. */
  manualMaps: number[];
  clearBuildings: boolean;
  /** The new matrix gets a header section. */
  withHeaders: boolean;
  header: number | null;
  areaDataId: number | null;
  eventFile: EventFileChoice;
}

export const emptyAreaForm: AreaForm = {
  name: "Neues Gebiet",
  sourceMatrix: null,
  selected: [],
  manualMaps: [],
  clearBuildings: false,
  withHeaders: true,
  header: null,
  areaDataId: null,
  eventFile: "Keep",
};

/** The largest side of a matrix. */
const MAX_SIDE = 255;

const rowMajor = (a: { x: number; y: number }, b: { x: number; y: number }) => a.y - b.y || a.x - b.x;

/**
 * The new area: the hull of the selected cells (the cells keep their arrangement, relative to
 * the hull's corner), then the maps picked by number in one row behind it.
 */
export function areaLayout(form: AreaForm): { width: number; height: number; cells: AreaCell[] } {
  const picked = [...form.selected].sort(rowMajor);
  const minX = Math.min(...picked.map((c) => c.x));
  const minY = Math.min(...picked.map((c) => c.y));
  const hullWidth = picked.length === 0 ? 0 : Math.max(...picked.map((c) => c.x)) - minX + 1;
  const hullHeight = picked.length === 0 ? 0 : Math.max(...picked.map((c) => c.y)) - minY + 1;
  const cell = (x: number, y: number, source_map: number): AreaCell => ({ x, y, source_map, clear_buildings: form.clearBuildings });
  const cells = picked.map((c) => cell(c.x - minX, c.y - minY, c.map));
  form.manualMaps.forEach((map, i) => cells.push(cell(i, hullHeight, map)));
  return {
    width: Math.max(hullWidth, form.manualMaps.length),
    height: hullHeight + (form.manualMaps.length > 0 ? 1 : 0),
    cells,
  };
}

/** The backend request for the form, or what is wrong with it. */
export function areaSpec(form: AreaForm): { spec: AreaSpec } | { error: string } {
  const { width, height, cells } = areaLayout(form);
  if (cells.length === 0) return { error: `Mindestens eine ${TERMS.map.label} wählen.` };
  if (width > MAX_SIDE || height > MAX_SIDE) return { error: `Höchstens ${MAX_SIDE} Spalten/Zeilen.` };
  const name = form.name.trim();
  if (name === "") return { error: "Name darf nicht leer sein." };
  return {
    spec: {
      name,
      width,
      height,
      cells,
      with_headers: form.withHeaders,
      header: form.header === null ? null : { id: form.header, area_data_id: form.areaDataId, event_file: form.eventFile },
    },
  };
}

const eventFileText = (choice: EventFileChoice): string => {
  if (choice === "Keep") return "behalten";
  if (choice === "New") return "neu, leer";
  return `Kopie von ${id4(choice.Copy)}`;
};

/** What "Anlegen" will do, one sentence per line, for the last step. */
export function areaSummary(spec: AreaSpec, headers: readonly HeaderSummary[]): string[] {
  const lines = [`Legt ${TERMS.matrix.label} „${spec.name}“ (${spec.width} × ${spec.height})${spec.with_headers ? ` mit ${TERMS.header.label}-Abschnitt` : ""} an.`];
  const copies = spec.cells.map((c) => `${TERMS.map.label} ${id4(c.source_map)} → Zelle (${c.x}, ${c.y})`).join(", ");
  lines.push(`Kopiert ${copies}, ${spec.cells.every((c) => c.clear_buildings) ? "ohne Gebäude" : "mit Gebäuden"}.`);
  if (spec.header === null) {
    lines.push(`Stellt keinen ${TERMS.header.label} um.`);
  } else {
    const assigned = headers.find((h) => h.id === spec.header?.id);
    const name = assigned ? ` „${headerLabel(assigned)}“` : "";
    const area = spec.header.area_data_id === null ? "" : `, ${TERMS.areaData.label} ${id4(spec.header.area_data_id)}`;
    lines.push(`Stellt ${TERMS.header.label} ${id4(spec.header.id)}${name} auf das neue ${TERMS.matrix.label} um${area}, ${TERMS.eventFile.label}: ${eventFileText(spec.header.event_file)}.`);
  }
  lines.push("Anlegen lässt sich nicht rückgängig machen.");
  return lines;
}

/** What `create_area` made, in words, and where to go on from there. */
export function areaResult(report: AreaReport, spec: AreaSpec): { lines: string[]; next: string[] } {
  const lines = [`${TERMS.matrix.label} ${id4(report.matrix)} „${spec.name}“ (${spec.width} × ${spec.height}) angelegt.`];
  const maps = report.maps.map(([, , map]) => id4(map)).join(", ");
  lines.push(`${report.maps.length === 1 ? TERMS.map.label : TERMS.map.plural} ${maps} kopiert.`);
  if (report.header !== null) {
    const file = report.event_file === null ? "" : `, ${TERMS.eventFile.label} ${id4(report.event_file)}`;
    lines.push(`${TERMS.header.label} ${id4(report.header)} zeigt jetzt auf ${TERMS.matrix.label} ${id4(report.matrix)}${file}.`);
  }
  return {
    lines,
    next: [`Eingang verbinden: ${TERMS.room.label} anlegen… oder Inspector › Ausgänge`, `Karten prüfen: ${TERMS.matrix.label}-Tab`],
  };
}

const hasMap = (matrix: MatrixDto, x: number, y: number): number | null => {
  if (x < 0 || y < 0 || x >= matrix.width || y >= matrix.height) return null;
  return matrix.maps[y * matrix.width + x] ?? null;
};

/** The selection with the cell toggled; an empty cell cannot be picked (it has no map to copy). */
export function toggleSelection(selected: PickedCell[], matrix: MatrixDto, x: number, y: number): PickedCell[] {
  if (selected.some((c) => c.x === x && c.y === y)) return selected.filter((c) => c.x !== x || c.y !== y);
  const map = hasMap(matrix, x, y);
  return map === null ? selected : [...selected, { x, y, map }];
}

/** The selection plus every filled cell of the rectangle spanned by two cells. */
export function rectSelection(selected: PickedCell[], matrix: MatrixDto, from: { x: number; y: number }, to: { x: number; y: number }): PickedCell[] {
  const next = [...selected];
  for (let y = Math.min(from.y, to.y); y <= Math.max(from.y, to.y); y++) {
    for (let x = Math.min(from.x, to.x); x <= Math.max(from.x, to.x); x++) {
      const map = hasMap(matrix, x, y);
      if (map !== null && !next.some((c) => c.x === x && c.y === y)) next.push({ x, y, map });
    }
  }
  return next.sort(rowMajor);
}

/** The header of the first selected cell (in row order) that has a known one: where the wizard takes area data and event file from. */
export function sourceHeader(matrix: MatrixDto, headers: readonly HeaderSummary[], selected: readonly PickedCell[]): HeaderSummary | null {
  if (!matrix.headers) return null;
  for (const c of [...selected].sort(rowMajor)) {
    const id = matrix.headers[c.y * matrix.width + c.x];
    const found = id === undefined ? undefined : headers.find((h) => h.id === id);
    if (found) return found;
  }
  return null;
}

/** A list of map numbers typed as text; `invalid` holds the entries that are no map of the game (`mapMax` is the highest). */
export function parseMapList(text: string, mapMax: number): { maps: number[]; invalid: string[] } {
  const maps: number[] = [];
  const invalid: string[] = [];
  for (const entry of text.split(/[\s,;]+/).filter((e) => e !== "")) {
    const n = Number(entry);
    if (Number.isInteger(n) && n >= 0 && n <= mapMax) maps.push(n);
    else invalid.push(entry);
  }
  return { maps, invalid };
}
