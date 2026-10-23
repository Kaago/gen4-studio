import type { AreaCell, AreaReport, AreaSpec, AssignHeader, EventFileChoice, EventsDto, HeaderSummary, MatrixDto, RoomReport, RoomSpec, WarpDto, WarpLink } from "../../api";
import { isCmdError } from "../../api";
import { headerLabel } from "../headers";
import { TERMS, id4 } from "../terms";
import type { WarpTile } from "../warpTile";

/** The internal name of a header the wizard creates: A-Z, 0-9 and `_` only, at most 16 bytes (the kernel's limit). */
export const INTERNAL_NAME_MAX = 16;

/**
 * An internal name derived from a typed name: words joined by `_`, upper case, ASCII letters and digits only (every
 * other character is dropped, not transliterated), at most 16 characters; "NEWMAP" when nothing is left.
 */
export function internalName(name: string): string {
  const cleaned = name
    .trim()
    .replace(/\s+/g, "_")
    .replace(/[^A-Za-z0-9_]/g, "")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "")
    .toUpperCase()
    .slice(0, INTERNAL_NAME_MAX);
  return cleaned === "" ? "NEWMAP" : cleaned;
}

/**
 * A header the wizard creates first. `template` is the header it is copied from; `internalName` is the typed name,
 * null while it follows the wizard's name.
 */
export interface NewHeaderForm {
  template: number;
  internalName: string | null;
}

/** What the "Header wählen" step edits (shared by the area and the room wizard). */
export interface HeaderChoiceValue {
  header: number | null;
  /** A header to be created instead (then `header` is null); `internalName` is what the field shows. */
  newHeader: { template: number; internalName: string } | null;
  /** Area data the header gets; null keeps the one it has. */
  areaDataId: number | null;
  eventFile: EventFileChoice;
}

/** The map type of the Underground (Platinum `location_specifier`, HGSS `location_type`). */
const UNDERGROUND = 6;

/**
 * Why a header that no warp leads to is still not free, or null: header 0 is the game's default for every cell without
 * one, the Underground header starts the Underground (a room on it stays black), the Union Room is reached by script.
 */
export function specialHeader(h: Pick<HeaderSummary, "id" | "internal_name" | "map_type">): string | null {
  if (h.id === 0) return "Vorgabe-Header aller Zellen ohne eigenen";
  if (h.map_type === UNDERGROUND || h.internal_name === "UG") return "Untergrund: das Spiel startet dort den Untergrund, ein Raum bleibt schwarz";
  if (h.internal_name === "UNION") return "Union-Raum, nur per Skript erreichbar";
  return null;
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
  /** Create a header (as the next number) and assign that one; `header` is then not used. */
  newHeader: NewHeaderForm | null;
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
  newHeader: null,
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

/**
 * The header assignment of a form: the existing header, or the header to be created, which gets the number
 * `headerCount` (the count of headers there are now). Null: none.
 */
export function assignHeader(
  form: { header: number | null; newHeader: NewHeaderForm | null; areaDataId: number | null; eventFile: EventFileChoice; name: string },
  headerCount: number,
): AssignHeader | null {
  const { newHeader } = form;
  if (newHeader !== null) {
    return {
      id: headerCount,
      area_data_id: form.areaDataId,
      event_file: form.eventFile,
      new_header: { template: newHeader.template, internal_name: internalName(newHeader.internalName ?? form.name) },
    };
  }
  return form.header === null ? null : { id: form.header, area_data_id: form.areaDataId, event_file: form.eventFile };
}

/** The form's new header as the step shows it (the name resolved), for `template`; null when none is asked for. */
export function newHeaderValue(newHeader: NewHeaderForm | null, name: string, template: number): HeaderChoiceValue["newHeader"] {
  return newHeader === null ? null : { template, internalName: newHeader.internalName ?? internalName(name) };
}

/** The new header as the form keeps it: a name equal to the one derived from `name` follows later changes of `name`. */
export function newHeaderForm(value: HeaderChoiceValue["newHeader"], name: string): NewHeaderForm | null {
  return value === null ? null : { template: value.template, internalName: value.internalName === internalName(name) ? null : value.internalName };
}

/**
 * The backend request for the form, or what is wrong with it. `headerCount` is the number of headers there are now:
 * a new header gets that number.
 */
export function areaSpec(form: AreaForm, headerCount: number): { spec: AreaSpec } | { error: string } {
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
      header: assignHeader(form, headerCount),
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
  const area = assign.area_data_id === null ? "" : `, ${TERMS.areaData.label} ${id4(assign.area_data_id)}`;
  const created = assign.new_header;
  if (created) {
    // "Keep" on a new header keeps what the copy has: the template's event file, which both headers then share.
    const file = assign.event_file === "Keep" ? `behalten (geteilt mit ${TERMS.header.label} ${id4(created.template)})` : eventFileText(assign.event_file);
    return `Der Assistent legt ${TERMS.header.label} ${id4(assign.id)} „${created.internal_name}“ neu an (Kopie von ${id4(created.template)}) und stellt ihn auf das neue ${TERMS.matrix.label} um${area}, ${TERMS.eventFile.label}: ${file}.`;
  }
  const assigned = headers.find((h) => h.id === assign.id);
  const name = assigned ? ` „${headerLabel(assigned)}“` : "";
  return `Stellt ${TERMS.header.label} ${id4(assign.id)}${name} auf das neue ${TERMS.matrix.label} um${area}, ${TERMS.eventFile.label}: ${eventFileText(assign.event_file)}.`;
}

/**
 * A warning when the header to be reassigned looks in use (warps lead to it): it loses the raster it shows today.
 * Null when no warp leads there, or its raster stays the same.
 */
function inUseLine(assign: AssignHeader, headers: readonly HeaderSummary[], links: readonly WarpLink[]): string | null {
  // A header that is yet to be made has no warps and no raster to lose.
  if (assign.new_header) return null;
  const incoming = links.filter((l) => l.dest_header_id === assign.id).length;
  const assigned = headers.find((h) => h.id === assign.id);
  if (incoming === 0 || !assigned) return null;
  // The technical word, as in the header choice's "Warps hierher".
  const warps = incoming === 1 ? `1 ${TERMS.warp.tech} führt` : `${incoming} ${TERMS.warp.tech}s führen`;
  return `Achtung: ${TERMS.header.label} ${id4(assign.id)} wird benutzt (${warps} hierher); er verliert sein bisheriges ${TERMS.matrix.label} ${id4(assigned.matrix_id)}.`;
}

const NOT_UNDOABLE = "Anlegen lässt sich nicht rückgängig machen.";

/** A warning when the header to be reassigned is a special one (see `specialHeader`); null otherwise. */
function specialLine(assign: AssignHeader, headers: readonly HeaderSummary[]): string | null {
  if (assign.new_header) return null;
  const assigned = headers.find((h) => h.id === assign.id);
  const why = assigned ? specialHeader(assigned) : null;
  return why === null ? null : `Achtung: ${TERMS.header.label} ${id4(assign.id)} ist ein Sonder-${TERMS.header.label} (${why}).`;
}

/** What "Anlegen" will do, one sentence per line, for the last step. */
export function areaSummary(spec: AreaSpec, headers: readonly HeaderSummary[], links: readonly WarpLink[] = []): string[] {
  const lines = [`Legt ${TERMS.matrix.label} „${spec.name}“ (${spec.width} × ${spec.height})${spec.with_headers ? ` mit ${TERMS.header.label}-Abschnitt` : ""} an.`];
  const copies = spec.cells.map((c) => `${TERMS.map.label} ${id4(c.source_map)} → Zelle (${c.x}, ${c.y})`).join(", ");
  lines.push(`Kopiert ${copies}, ${spec.cells.every((c) => c.clear_buildings) ? "ohne Gebäude" : "mit Gebäuden"}.`);
  lines.push(spec.header === null ? `Stellt keinen ${TERMS.header.label} um.` : assignLine(spec.header, headers));
  const warnings = spec.header === null ? [] : [specialLine(spec.header, headers), inUseLine(spec.header, headers, links)];
  for (const warning of warnings) if (warning) lines.push(warning);
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
    lines.push(`${headerResultName(report.header, spec.header)} zeigt jetzt auf ${TERMS.matrix.label} ${id4(report.matrix)}${file}.`);
  }
  return {
    lines,
    next: [`Mit der Welt verbinden: ${TERMS.header.label}-Editor › ${TERMS.warp.plural} › Warp hinzufügen (Ziel im Event-Editor wählen)`, `Karten prüfen: ${TERMS.matrix.label}-Tab`],
  };
}

/** The header a result names: a header the wizard made is called new, with its internal name. */
function headerResultName(id: number, assign: AssignHeader | null): string {
  const created = assign?.new_header;
  return created ? `Neuer ${TERMS.header.label} ${id4(id)} „${created.internal_name}“ (Kopie von ${id4(created.template)})` : `${TERMS.header.label} ${id4(id)}`;
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

/**
 * The header the wizard takes area data and event file from, and copies a new header from: `preferred` (the header the
 * wizard was opened for) when one of the selected cells has it, else the header of the first selected cell (in row
 * order) that is no special header (see `specialHeader`; the filler cells around a town belong to header 0), else the
 * first known one.
 */
export function sourceHeader(matrix: MatrixDto, headers: readonly HeaderSummary[], selected: readonly PickedCell[], preferred?: number): HeaderSummary | null {
  if (!matrix.headers) return null;
  const found: HeaderSummary[] = [];
  for (const c of [...selected].sort(rowMajor)) {
    const id = matrix.headers[c.y * matrix.width + c.x];
    const header = id === undefined ? undefined : headers.find((h) => h.id === id);
    if (header) found.push(header);
  }
  return found.find((h) => h.id === preferred) ?? found.find((h) => specialHeader(h) === null) ?? found[0] ?? null;
}

/**
 * The tileset (area data) the new area will be shown with, for its preview: the one the form sets, else the chosen
 * header's own (it keeps it), else the source header's (no header is assigned yet), else 0.
 */
export function previewArea(form: Pick<AreaForm, "areaDataId" | "header">, headers: readonly HeaderSummary[], source: HeaderSummary | null): number {
  if (form.areaDataId !== null) return form.areaDataId;
  const chosen = form.header === null ? undefined : headers.find((h) => h.id === form.header);
  return chosen?.area_data_id ?? source?.area_data_id ?? 0;
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

/** What a wizard shows in place of lists that need the overview: null once it is loaded. */
export const overviewNote = (loaded: boolean, failed: boolean): string | null => (loaded ? null : failed ? "Übersicht nicht verfügbar" : "Lade Übersicht …");

/**
 * Whether a wizard's error says that members stayed behind (the backend's code "partial"; the
 * wizard cannot repair them, the result step says so).
 */
export const isLateFailure = (e: unknown): boolean => isCmdError(e) && e.code === "partial";

const MEMBER_WORDS: Record<string, string> = {
  header: TERMS.header.label,
  headers: TERMS.header.plural,
  matrix: TERMS.matrix.label,
  matrices: TERMS.matrix.plural,
  map: TERMS.map.label,
  maps: TERMS.map.plural,
  "event file": TERMS.eventFile.label,
  "event files": TERMS.eventFile.plural,
};

const STEP_WORDS: Record<string, string> = {
  "header creation": `${TERMS.header.label} anlegen`,
  "map copy": `${TERMS.map.label} kopieren`,
  "matrix creation": `${TERMS.matrix.label} anlegen`,
  "event file": `${TERMS.eventFile.label} anlegen`,
  "matrix edit": `${TERMS.matrix.label} ändern`,
  "header edit": `${TERMS.header.label} umstellen`,
  "warp edit": `${TERMS.warp.tech} setzen`,
  "door tile": "Tür-Kachel setzen",
};

/**
 * The backend's text of a late failure ("header 0593 and map 0670 were created; map copy failed: …") in German: what
 * was created and which step failed; the system's reason stays as it came. Another text comes back unchanged.
 */
export function partialText(message: string): string {
  const parts = /^(.*) (?:was|were) created; (.*?) failed: (.*)$/s.exec(message);
  if (!parts) return message;
  const [, members, step, reason] = parts;
  // "header 0593, matrix 0131 and maps 0003, 0004": each group is a word and its ids.
  const groups = [...members.matchAll(/(event files?|headers?|matrix|matrices|maps?) ([\d, ]+?)(?=,? and |, [a-z]|$)/g)];
  if (groups.length === 0) return message;
  const created = groups.map(([, word, ids]) => `${MEMBER_WORDS[word] ?? word} ${ids.trim()}`).join(", ");
  return `Angelegt: ${created}. Danach fehlgeschlagen: ${STEP_WORDS[step] ?? step} (${reason}).`;
}

/** What the "Raum anlegen" wizard edits. `template` is the room's header, `templateMap` the map it is copied from. */
export interface RoomForm {
  name: string;
  template: number | null;
  templateMap: number | null;
  clearBuildings: boolean;
  header: number | null;
  /** Create a header (as the next number) and assign that one; `header` is then not used. */
  newHeader: NewHeaderForm | null;
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
  /** Tiles a warp would not fire on become doors (behaviour 0x69, collision 0x80). */
  doorTiles: boolean;
}

export const emptyRoomForm: RoomForm = {
  name: "Neuer Raum",
  template: null,
  templateMap: null,
  clearBuildings: false,
  header: null,
  newHeader: null,
  areaDataId: null,
  eventFile: "New",
  connect: true,
  outsideHeader: null,
  x: null,
  z: null,
  insideX: 16,
  insideZ: 30,
  doorTiles: true,
};

/** The last tile of a map (a map is 32 × 32 tiles). */
export const MAP_LAST_TILE = 31;

/** The tiles of a matrix per side (a map is 32 tiles wide). */
export const matrixTiles = (matrix: { width: number; height: number }) => ({ x: matrix.width * 32, z: matrix.height * 32 });

/** What `roomSpec` checks the doors against, as far as it is known (left out: not checked). */
export interface RoomChecks {
  /**
   * The outside header's matrix: the door outside must lie in it, with the cells (once read) in a cell with a map that,
   * with a header section, belongs to the outside header (the game loads a header's warps only in its own cells).
   */
  outsideMatrix?: { width: number; height: number; maps?: (number | null)[]; headers?: number[] | null } | null;
  /** The warps of the outside header's event file: the door outside needs a tile without one. */
  outsideWarps?: readonly WarpDto[] | null;
  /** The warps of the event file the room keeps ("behalten"): the door inside needs a tile without one there. */
  keptWarps?: { file: number; warps: readonly WarpDto[] } | null;
}

/** The map under tile (x, z) of a matrix; null off the matrix, in an empty cell and while the matrix is not read. */
export function mapUnderTile(matrix: Pick<MatrixDto, "width" | "height" | "maps"> | null, x: number, z: number): number | null {
  if (!matrix || x < 0 || z < 0 || x >= matrix.width * 32 || z >= matrix.height * 32) return null;
  return matrix.maps[Math.floor(z / 32) * matrix.width + Math.floor(x / 32)] ?? null;
}

/** The first warp on tile (x, z): the one the game takes there. */
export const warpOnTile = (warps: readonly WarpDto[] | null | undefined, x: number, z: number): WarpDto | null => warps?.find((w) => w.x === x && w.z === z) ?? null;

/**
 * The backend request for the form, or what is wrong with it. `headerCount` is the number of headers there are now (a
 * new header gets that number). `checks` holds what is known about the outside matrix and the event files (see
 * `RoomChecks`); for the matrix alone its size may be passed.
 */
export function roomSpec(form: RoomForm, headerCount: number, checks?: RoomChecks | { width: number; height: number } | null): { spec: RoomSpec } | { error: string } {
  const known: RoomChecks = checks && "width" in checks ? { outsideMatrix: checks } : (checks ?? {});
  const outsideMatrix = known.outsideMatrix;
  if (form.template === null || form.templateMap === null) return { error: "Vorlage wählen." };
  const name = form.name.trim();
  if (name === "") return { error: "Name darf nicht leer sein." };
  const header = assignHeader(form, headerCount);
  if (header === null) return { error: `${TERMS.header.label} wählen.` };
  if (form.connect) {
    const inside = [form.insideX, form.insideZ];
    if (inside.some((n) => !Number.isInteger(n) || n < 0 || n > MAP_LAST_TILE)) return { error: `Kachel liegt außerhalb der Karte (0 bis ${MAP_LAST_TILE}).` };
    if (form.outsideHeader === null) return { error: `Außen-${TERMS.header.label} wählen.` };
    if (form.outsideHeader === header.id) return { error: `Der Außen-${TERMS.header.label} darf nicht der ${TERMS.header.label} des neuen ${TERMS.room.label}s sein.` };
    if (form.x === null || form.z === null) return { error: `Kachel der Tür außen eintragen (x und z).` };
    if (outsideMatrix) {
      const size = matrixTiles(outsideMatrix);
      if (form.x < 0 || form.z < 0 || form.x >= size.x || form.z >= size.z) {
        return { error: `Kachel x ${form.x}, z ${form.z} liegt außerhalb des ${TERMS.matrix.label}s (x 0 bis ${size.x - 1}, z 0 bis ${size.z - 1}).` };
      }
      const [cx, cy] = [Math.floor(form.x / 32), Math.floor(form.z / 32)];
      const cell = cy * outsideMatrix.width + cx;
      if (outsideMatrix.maps && (outsideMatrix.maps[cell] ?? null) === null) {
        return { error: `Kachel x ${form.x}, z ${form.z} liegt in Zelle (${cx}, ${cy}) ohne ${TERMS.map.label}.` };
      }
      const owner = outsideMatrix.headers?.[cell];
      if (owner !== undefined && owner !== form.outsideHeader) {
        return {
          error: `Kachel x ${form.x}, z ${form.z} liegt in Zelle (${cx}, ${cy}) von ${TERMS.header.label} ${id4(owner)}, nicht von ${id4(form.outsideHeader)}: dort nimmt das Spiel die ${TERMS.warp.tech}s von ${id4(form.outsideHeader)} nicht.`,
        };
      }
    }
    const taken = warpOnTile(known.outsideWarps, form.x, form.z);
    if (taken) return { error: `Auf Kachel x ${form.x}, z ${form.z} außen liegt schon ${TERMS.warp.tech} ${taken.index} (Ziel ${TERMS.header.label} ${id4(taken.dest_header_id)}): eine andere Kachel wählen.` };
    const kept = form.eventFile === "Keep" ? known.keptWarps : null;
    const takenInside = kept ? warpOnTile(kept.warps, form.insideX, form.insideZ) : null;
    if (kept && takenInside) {
      return {
        error: `Auf Kachel x ${form.insideX}, z ${form.insideZ} innen liegt schon ${TERMS.warp.tech} ${takenInside.index} der ${TERMS.eventFile.label} ${id4(kept.file)}, die der ${TERMS.room.label} behält: eine andere Kachel oder „Kopie von …“ wählen.`,
      };
    }
  }
  return {
    spec: {
      name,
      source_map: form.templateMap,
      clear_buildings: form.clearBuildings,
      header,
      entrance:
        form.connect && form.outsideHeader !== null && form.x !== null && form.z !== null
          ? { outside_header: form.outsideHeader, x: form.x, z: form.z, inside_x: form.insideX, inside_z: form.insideZ, door_tiles: form.doorTiles }
          : null,
    },
  };
}

/** What the room summary knows about the doors beyond the request. */
export interface DoorNotes {
  /** The warp of the copied event file that stands on the inside tile: it becomes the way back. */
  reused?: WarpDto | null;
  /** The tiles a warp would not fire on now (see `warpTile`). */
  outsideTile?: WarpTile | null;
  insideTile?: WarpTile | null;
}

/** What "Anlegen" will do, one sentence per line, for the last step. */
export function roomSummary(spec: RoomSpec, headers: readonly HeaderSummary[], notConnected?: string, links: readonly WarpLink[] = [], doors: DoorNotes = {}): string[] {
  const lines = [`Legt ${TERMS.matrix.label} „${spec.name}“ mit einer Kopie von ${TERMS.map.label} ${id4(spec.source_map)} an, ${spec.clear_buildings ? "ohne Gebäude" : "mit Gebäuden"}.`];
  lines.push(assignLine(spec.header, headers));
  for (const warning of [specialLine(spec.header, headers), inUseLine(spec.header, headers, links)]) if (warning) lines.push(warning);
  const entrance = spec.entrance;
  if (entrance === null) {
    const why = notConnected ? ` (${notConnected})` : "";
    lines.push(`Legt keinen Eingang an${why}: ${TERMS.warp.label} später im ${TERMS.header.label}-Editor unter „${TERMS.warp.plural}“ setzen.`);
  } else {
    const outside = headers.find((h) => h.id === entrance.outside_header);
    const name = outside ? ` ${headerLabel(outside)}` : "";
    lines.push(`Verbindet ${id4(entrance.outside_header)}${name} (Kachel x ${entrance.x}, z ${entrance.z}) mit dem neuen ${TERMS.room.label}.`);
    lines.push(`Der Rückweg liegt im neuen ${TERMS.room.label} auf Kachel x ${entrance.inside_x}, z ${entrance.inside_z}.`);
    if (doors.reused) {
      lines.push(`Der kopierte ${TERMS.warp.tech} ${doors.reused.index} auf dieser Kachel (bisher Ziel ${TERMS.header.label} ${id4(doors.reused.dest_header_id)}) wird zum Rückweg; es kommt kein zweiter dazu.`);
    }
    const dead = [doors.outsideTile && !doors.outsideTile.fires ? "außen" : null, doors.insideTile && !doors.insideTile.fires ? "innen" : null].filter((w) => w !== null);
    if (dead.length > 0) {
      lines.push(
        entrance.door_tiles
          ? `Macht die Kachel ${dead.join(" und ")} zur Tür (Verhalten 0x69, Kollision 0x80), sonst löst der ${TERMS.warp.tech} im Spiel nicht aus.`
          : `Achtung: auf der Kachel ${dead.join(" und ")} löst ein ${TERMS.warp.tech} im Spiel nicht aus (keine Tür- oder Warp-Kachel).`,
      );
    }
  }
  lines.push(NOT_UNDOABLE);
  return lines;
}

/** What `create_room` made, in words, and where to go on from there. */
export function roomResult(report: RoomReport, spec: RoomSpec): { lines: string[]; next: string[] } {
  const lines = [`${TERMS.room.label} „${spec.name}“ angelegt: ${TERMS.matrix.label} ${id4(report.matrix)}, ${TERMS.map.label} ${id4(report.map)}.`];
  const file = report.event_file === null ? "" : `, ${TERMS.eventFile.label} ${id4(report.event_file)}`;
  lines.push(`${headerResultName(spec.header.id, spec.header)} zeigt jetzt auf ${TERMS.matrix.label} ${id4(report.matrix)}${file}.`);
  if (spec.entrance !== null && report.outside_warp !== null && report.inside_warp !== null) {
    const reused = report.inside_warp_reused ? " (der kopierte Warp auf der Kachel, umgestellt)" : "";
    lines.push(`Eingang: Warp ${report.outside_warp} in ${TERMS.header.label} ${id4(spec.entrance.outside_header)}, Rückweg: Warp ${report.inside_warp}${reused} im neuen ${TERMS.room.label}.`);
  }
  for (const t of report.door_tiles ?? []) lines.push(`Tür-Kachel gesetzt: ${TERMS.map.label} ${id4(t.map)}, Kachel x ${t.x}, z ${t.y} innerhalb der ${TERMS.map.label}.`);
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

