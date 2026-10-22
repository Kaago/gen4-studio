import type { AreaCell, AreaReport, AreaSpec, AssignHeader, EventFileChoice, EventsDto, HeaderSummary, MatrixDto, RoomReport, RoomSpec } from "../../api";
import { isCmdError } from "../../api";
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
  // A new area on an existing header would otherwise share that header's event file (and its warps).
  eventFile: "New",
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

/** The sentence for the header a wizard points at the new matrix: area data and event file included. */
function assignLine(assign: AssignHeader, headers: readonly HeaderSummary[]): string {
  const assigned = headers.find((h) => h.id === assign.id);
  const name = assigned ? ` „${headerLabel(assigned)}“` : "";
  const area = assign.area_data_id === null ? "" : `, ${TERMS.areaData.label} ${id4(assign.area_data_id)}`;
  return `Stellt ${TERMS.header.label} ${id4(assign.id)}${name} auf das neue ${TERMS.matrix.label} um${area}, ${TERMS.eventFile.label}: ${eventFileText(assign.event_file)}.`;
}

const NOT_UNDOABLE = "Anlegen lässt sich nicht rückgängig machen.";

/** What "Anlegen" will do, one sentence per line, for the last step. */
export function areaSummary(spec: AreaSpec, headers: readonly HeaderSummary[]): string[] {
  const lines = [`Legt ${TERMS.matrix.label} „${spec.name}“ (${spec.width} × ${spec.height})${spec.with_headers ? ` mit ${TERMS.header.label}-Abschnitt` : ""} an.`];
  const copies = spec.cells.map((c) => `${TERMS.map.label} ${id4(c.source_map)} → Zelle (${c.x}, ${c.y})`).join(", ");
  lines.push(`Kopiert ${copies}, ${spec.cells.every((c) => c.clear_buildings) ? "ohne Gebäude" : "mit Gebäuden"}.`);
  lines.push(spec.header === null ? `Stellt keinen ${TERMS.header.label} um.` : assignLine(spec.header, headers));
  lines.push(NOT_UNDOABLE);
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
    next: [`Mit der Welt verbinden: ${TERMS.header.label}-Editor › ${TERMS.warp.plural} › Warp hinzufügen (Ziel im Event-Editor wählen)`, `Karten prüfen: ${TERMS.matrix.label}-Tab`],
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

/**
 * Whether a wizard's error says that members stayed behind (the backend's code "partial"; the
 * wizard cannot repair them, the result step says so).
 */
export const isLateFailure = (e: unknown): boolean => isCmdError(e) && e.code === "partial";

/** What the "Raum anlegen" wizard edits. `template` is the room's header, `templateMap` the map it is copied from. */
export interface RoomForm {
  name: string;
  template: number | null;
  templateMap: number | null;
  clearBuildings: boolean;
  header: number | null;
  areaDataId: number | null;
  eventFile: EventFileChoice;
  /** The entrance is made now (off: the room is connected later). */
  connect: boolean;
  outsideHeader: number | null;
  /** The door outside, a tile of the outside header's matrix; null: not entered yet. */
  x: number | null;
  z: number | null;
  /** The door inside, a tile of the room's map. */
  insideX: number;
  insideZ: number;
}

export const emptyRoomForm: RoomForm = {
  name: "Neuer Raum",
  template: null,
  templateMap: null,
  clearBuildings: false,
  header: null,
  areaDataId: null,
  eventFile: "New",
  connect: true,
  outsideHeader: null,
  x: null,
  z: null,
  insideX: 16,
  insideZ: 30,
};

/** The last tile of a map (a map is 32 × 32 tiles). */
export const MAP_LAST_TILE = 31;

/** The tiles of a matrix per side (a map is 32 tiles wide). */
export const matrixTiles = (matrix: { width: number; height: number }) => ({ x: matrix.width * 32, z: matrix.height * 32 });

/**
 * The backend request for the form, or what is wrong with it. `outsideMatrix` is the size of the outside header's
 * matrix: the door outside must lie in it (null or omitted: unknown, not checked).
 */
export function roomSpec(form: RoomForm, outsideMatrix?: { width: number; height: number } | null): { spec: RoomSpec } | { error: string } {
  if (form.template === null || form.templateMap === null) return { error: "Vorlage wählen." };
  const name = form.name.trim();
  if (name === "") return { error: "Name darf nicht leer sein." };
  if (form.header === null) return { error: `${TERMS.header.label} wählen.` };
  if (form.connect) {
    const inside = [form.insideX, form.insideZ];
    if (inside.some((n) => !Number.isInteger(n) || n < 0 || n > MAP_LAST_TILE)) return { error: `Kachel liegt außerhalb der Karte (0 bis ${MAP_LAST_TILE}).` };
    if (form.outsideHeader === null) return { error: `Außen-${TERMS.header.label} wählen.` };
    if (form.outsideHeader === form.header) return { error: `Der Außen-${TERMS.header.label} darf nicht der ${TERMS.header.label} des neuen ${TERMS.room.label}s sein.` };
    if (form.x === null || form.z === null) return { error: `Kachel der Tür außen eintragen (x und z).` };
    if (outsideMatrix) {
      const size = matrixTiles(outsideMatrix);
      if (form.x < 0 || form.z < 0 || form.x >= size.x || form.z >= size.z) {
        return { error: `Kachel x ${form.x}, z ${form.z} liegt außerhalb des ${TERMS.matrix.label}s (x 0 bis ${size.x - 1}, z 0 bis ${size.z - 1}).` };
      }
    }
  }
  return {
    spec: {
      name,
      source_map: form.templateMap,
      clear_buildings: form.clearBuildings,
      header: { id: form.header, area_data_id: form.areaDataId, event_file: form.eventFile },
      entrance:
        form.connect && form.outsideHeader !== null && form.x !== null && form.z !== null
          ? { outside_header: form.outsideHeader, x: form.x, z: form.z, inside_x: form.insideX, inside_z: form.insideZ }
          : null,
    },
  };
}

/** What "Anlegen" will do, one sentence per line, for the last step. */
export function roomSummary(spec: RoomSpec, headers: readonly HeaderSummary[], notConnected?: string): string[] {
  const lines = [`Legt ${TERMS.matrix.label} „${spec.name}“ mit einer Kopie von ${TERMS.map.label} ${id4(spec.source_map)} an, ${spec.clear_buildings ? "ohne Gebäude" : "mit Gebäuden"}.`];
  lines.push(assignLine(spec.header, headers));
  const entrance = spec.entrance;
  if (entrance === null) {
    const why = notConnected ? ` (${notConnected})` : "";
    lines.push(`Legt keinen Eingang an${why}: ${TERMS.warp.label} später im ${TERMS.header.label}-Editor unter „Ausgänge“ setzen.`);
  } else {
    const outside = headers.find((h) => h.id === entrance.outside_header);
    const name = outside ? ` ${headerLabel(outside)}` : "";
    lines.push(`Verbindet ${id4(entrance.outside_header)}${name} (Kachel x ${entrance.x}, z ${entrance.z}) mit dem neuen ${TERMS.room.label}.`);
    lines.push(`Der Rückweg liegt im neuen ${TERMS.room.label} auf Kachel x ${entrance.inside_x}, z ${entrance.inside_z}.`);
  }
  lines.push(NOT_UNDOABLE);
  return lines;
}

/** What `create_room` made, in words, and where to go on from there. */
export function roomResult(report: RoomReport, spec: RoomSpec): { lines: string[]; next: string[] } {
  const lines = [`${TERMS.room.label} „${spec.name}“ angelegt: ${TERMS.matrix.label} ${id4(report.matrix)}, ${TERMS.map.label} ${id4(report.map)}.`];
  const file = report.event_file === null ? "" : `, ${TERMS.eventFile.label} ${id4(report.event_file)}`;
  lines.push(`${TERMS.header.label} ${id4(spec.header.id)} zeigt jetzt auf ${TERMS.matrix.label} ${id4(report.matrix)}${file}.`);
  if (spec.entrance !== null && report.outside_warp !== null && report.inside_warp !== null) {
    lines.push(`Eingang: Warp ${report.outside_warp} in ${TERMS.header.label} ${id4(spec.entrance.outside_header)}, Rückweg: Warp ${report.inside_warp} im neuen ${TERMS.room.label}.`);
  }
  return {
    lines,
    next:
      spec.entrance === null
        ? [`Eingang verbinden: ${TERMS.header.label}-Editor › Ausgänge › Warp hinzufügen`, `Innenraum einrichten: ${TERMS.map.label} im Raster-Tab bearbeiten`]
        : ["Ausprobieren: speichern, bauen und im Emulator hinein- und zurücklaufen", `Innenraum einrichten: Objekte und Trigger im ${TERMS.header.label}-Editor`],
  };
}

/** Where the first warp of the template room is: the door the player leaves by, and so a good place for the way back in the new room. */
export function templateWarp(events: EventsDto | null): { x: number; z: number } | null {
  const first = events?.warps[0];
  return first ? { x: first.x, z: first.z } : null;
}
