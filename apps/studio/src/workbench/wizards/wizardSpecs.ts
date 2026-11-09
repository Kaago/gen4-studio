import type { AreaCell, AreaReport, AreaSpec, AreaTarget, AssetKind, AssignHeader, ConnectDoorsInput, ConnectDoorsReport, DoorReport, EventFileChoice, EventsDto, HeaderSummary, MatrixDto, NameTablesDto, NewFigureReport, NewTrainerHereInput, NewTrainerHereReport, NextStep, PlaceChoice, RecipeOutcome, RoomReport, RoomSpec, WarpDto, WarpLink } from "../../api";
import { isCmdError } from "../../api";
import { className, speciesName, trainerTitle } from "../../data/trainer/trainerModel";
import { NOT_A_TEXT_PROJECT } from "../../flags/flagWords";
import { t } from "../../i18n";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { headerLabel } from "../headers";
import { UNNAMED_PLACE } from "../places";
import { nextStepsFor } from "../recipes";
import { id4, numberText, termLabel, termPlural } from "../terms";
import type { WarpTile } from "../warpTile";
import type { World } from "../worldModel";
import { lookItems, lookText } from "./looks";

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

/** What the „Header wählen“ step edits (shared by the area and the room wizard). */
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
  if (h.id === 0) return t("workbench:wizardSpecs.special.default");
  if (h.map_type === UNDERGROUND || h.internal_name === "UG") return t("workbench:wizardSpecs.special.underground");
  if (h.internal_name === "UNION") return t("workbench:wizardSpecs.special.union");
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
  /** Bauen: the place of the new area; null follows the default (`areaPlace`). */
  place: PlaceChoice | null;
  /** Bauen: the look (area data) of the new area; null keeps the template's. */
  lookAreaData: number | null;
  /** Bauen: the one-time consent to apply the Dynamic Headers patch while creating. */
  patchConsent: boolean;
  /** „Wo liegt das Gebiet?“: the matrix of the world map the area goes onto (seamless); null: a map of its own (connected by doors). */
  worldMap: number | null;
  /** The cell of the area's upper left map on a world map; kept while „Eigene Karte“ is chosen, sent only for `worldMap`. */
  target: AreaTarget | null;
}

/** The empty form; its `name` is read from the catalog when the form is copied (`{ ...emptyAreaForm }`), never at module scope. */
export const emptyAreaForm: AreaForm = {
  get name() {
    return t("workbench:wizardSpecs.newArea");
  },
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
  place: null,
  lookAreaData: null,
  patchConsent: false,
  worldMap: null,
  target: null,
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

/** The place a new area gets unless another is chosen: a new place named like the area. */
export const areaPlace = (form: Pick<AreaForm, "place" | "name">): PlaceChoice => form.place ?? { New: form.name.trim() };

/**
 * The place a new room gets unless another is chosen: the place of the header outside its entrance, else the place of its
 * template (what the copied header carries anyway).
 */
export function roomPlace(place: PlaceChoice | null, outside: HeaderSummary | null, template: HeaderSummary | null): PlaceChoice | null {
  if (place !== null) return place;
  const from = outside ?? template;
  return from ? { Existing: from.location_index } : null;
}

/** A chosen place as the form keeps it: the default itself is kept as null, so it follows later changes (the area's name, the entrance). */
export const placeForm = (choice: PlaceChoice, fallback: PlaceChoice | null): PlaceChoice | null => (JSON.stringify(choice) === JSON.stringify(fallback) ? null : choice);

/** What is wrong with a place, or null: a new place needs a name. */
export const placeError = (place: PlaceChoice | null): string | null => {
  if (place === null) return t("workbench:wizardSpecs.choosePlace");
  return "New" in place && place.New.trim() === "" ? t("workbench:wizardSpecs.placeNameEmpty") : null;
};

/**
 * The header Bauen gives a new area or room: always a new one, a copy of `template` with an automatic internal name, an
 * empty event file of its own, the chosen place and look, and the patch applied first when the user consented.
 */
function buildingHeader(form: { place: PlaceChoice | null; lookAreaData: number | null; patchConsent: boolean }, headerCount: number, template: number): AssignHeader | { error: string } {
  const error = placeError(form.place);
  if (error !== null || form.place === null) return { error: error ?? t("workbench:wizardSpecs.choosePlace") };
  const place = "New" in form.place ? { New: form.place.New.trim() } : form.place;
  return {
    id: headerCount,
    area_data_id: form.lookAreaData,
    event_file: "New",
    new_header: { template, internal_name: null },
    place,
    allow_patch: form.patchConsent,
  };
}

/**
 * The spec a wizard plans its draft with: while the game lacks the Dynamic Headers patch, a new header is planned as if the user had
 * consented to the patch, so the draft shows before the consent box of the last step (the plan writes nothing; without
 * `allow_patch` the kernel refuses a new header there). „Anlegen“ sends the spec itself, with the user's own consent.
 */
export function planSpec<T extends { header: AssignHeader | null }>(spec: T, patchMissing: boolean): T {
  if (!patchMissing || !spec.header?.new_header) return spec;
  return { ...spec, header: { ...spec.header, allow_patch: true } };
}

/** What `areaSpec` says while a world map is chosen and no cell on it yet: the step's grid hint asks for it, so the step shows no error. */
export const pickTarget = (): string => t("workbench:wizardSpecs.pickTarget");

/**
 * The backend request for the form, or what is wrong with it. `headerCount` is the number of headers there are now:
 * a new header gets that number. Tüfteln assigns the header the form chose; Bauen always makes a new one (`buildingHeader`),
 * a copy of `form.newHeader.template`.
 */
export function areaSpec(form: AreaForm, headerCount: number, depth: Depth): { spec: AreaSpec } | { error: string } {
  const { width, height, cells } = areaLayout(form);
  if (cells.length === 0) return { error: t("workbench:wizardSpecs.chooseMap") };
  if (width > MAX_SIDE || height > MAX_SIDE) return { error: t("workbench:wizardSpecs.tooLarge", { max: MAX_SIDE }) };
  const name = form.name.trim();
  if (name === "") return { error: t("workbench:wizardSpecs.nameEmpty") };
  const target = form.worldMap === null ? null : form.target;
  if (form.worldMap !== null && target?.matrix !== form.worldMap) return { error: pickTarget() };
  let header: AssignHeader | null;
  if (!depthAtLeast(depth, "tuefteln")) {
    if (form.newHeader === null) return { error: t("workbench:wizardSpecs.chooseTemplate") };
    const built = buildingHeader({ ...form, place: areaPlace(form) }, headerCount, form.newHeader.template);
    if ("error" in built) return built;
    header = built;
  } else header = assignHeader(form, headerCount);
  return {
    spec: {
      name,
      width,
      height,
      cells,
      // Bauen gives every new area its cells (the header section): the place shows only there.
      with_headers: !depthAtLeast(depth, "tuefteln") || form.withHeaders,
      header,
      // Only an area on a world map names a target: older callers send none.
      ...(target ? { target } : {}),
    },
  };
}

const eventFileText = (choice: EventFileChoice): string => {
  if (choice === "Keep") return t("workbench:wizardSpecs.eventFile.keep");
  if (choice === "New") return t("workbench:wizardSpecs.eventFile.new");
  return t("workbench:wizardSpecs.eventFile.copy", { id: id4(choice.Copy) });
};

/** ` „NAME“` for a typed internal name; nothing when the backend derives it (null). */
const quoted = (name: string | null): string => (name === null || name.trim() === "" ? "" : ` ${t("workbench:wizardSpecs.quoted", { name })}`);

/**
 * The sentence for the header a wizard points at the new matrix (`target`: the world map's raster instead), area data and event file
 * included.
 */
function assignLine(assign: AssignHeader, headers: readonly HeaderSummary[], target: AreaTarget | null = null): string {
  const raster = target ? t("workbench:wizardSpecs.raster", { id: id4(target.matrix) }) : t("workbench:wizardSpecs.newRaster");
  const area = assign.area_data_id === null ? "" : `, ${termLabel("areaData")} ${id4(assign.area_data_id)}`;
  const created = assign.new_header;
  if (created) {
    // „Keep“ on a new header keeps what the copy has: the template's event file, which both headers then share.
    const file = assign.event_file === "Keep" ? t("workbench:wizardSpecs.eventFile.keepShared", { id: id4(created.template) }) : eventFileText(assign.event_file);
    return t("workbench:wizardSpecs.assignNew", { id: id4(assign.id), name: quoted(created.internal_name), template: id4(created.template), raster, area, file });
  }
  const assigned = headers.find((h) => h.id === assign.id);
  const name = assigned ? ` ${t("workbench:wizardSpecs.quoted", { name: headerLabel(assigned) })}` : "";
  return t("workbench:wizardSpecs.assignExisting", { id: id4(assign.id), name, raster, area, file: eventFileText(assign.event_file) });
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
  // The technical word, as in the header choice's „Warps hierher“.
  return t("workbench:wizardSpecs.inUse", { count: incoming, id: id4(assign.id), matrix: id4(assigned.matrix_id) });
}

/** That creating cannot be undone, in the language that is shown. */
const notUndoable = () => t("workbench:wizardSpecs.notUndoable");

/** A warning when the header to be reassigned is a special one (see `specialHeader`); null otherwise. */
function specialLine(assign: AssignHeader, headers: readonly HeaderSummary[]): string | null {
  if (assign.new_header) return null;
  const assigned = headers.find((h) => h.id === assign.id);
  const why = assigned ? specialHeader(assigned) : null;
  return why === null ? null : t("workbench:wizardSpecs.specialLine", { id: id4(assign.id), why });
}

/** The place a new area or room gets, in words: „Neuer Ort „Neuheim“.“, „Ort: Zweiblattdorf.“ (a place no header carries yet by its number). */
function placeLine(place: PlaceChoice, headers: readonly HeaderSummary[]): string {
  if ("New" in place) return t("workbench:wizardSpecs.newPlace", { name: place.New });
  const named = headers.find((h) => h.location_index === place.Existing)?.location_name.trim();
  return t("workbench:wizardSpecs.place", { name: named || numberText(place.Existing) });
}

/** Bauen's lines for the new header: the place, the look (the template's unless chosen) and the empty list of figures and doors. */
function buildingLines(assign: AssignHeader, headers: readonly HeaderSummary[]): string[] {
  const template = headers.find((h) => h.id === assign.new_header?.template);
  const look = assign.area_data_id ?? template?.area_data_id;
  const lines = assign.place ? [placeLine(assign.place, headers)] : [];
  lines.push(`${look === undefined ? t("workbench:looks.template") : lookText(lookItems(headers, template ?? null), look, "bauen")}.`);
  // The empty event file Bauen always gives a new area or room, in its words.
  lines.push(t("workbench:wizardSpecs.newEvents"));
  return lines;
}

/** „Karte 0521“ or „Karten 0521, 0522“. */
const mapList = (maps: readonly number[]): string => `${maps.length === 1 ? termLabel("map") : termPlural("map")} ${maps.map(id4).join(", ")}`;

/** Where an area on a world map lies: the world map's name and the landscape cells its maps replace (see `targetBlock`). */
export interface TargetPlace {
  name: string;
  replaced: number;
}

/** The line for an area on a world map: Bauen names the world map and the landscape maps, Tüfteln the raster and the cell. */
function targetLine(target: AreaTarget, place: TargetPlace, depth: Depth): string {
  const n = place.replaced;
  if (!depthAtLeast(depth, "tuefteln")) {
    const replaced = n === 0 ? "" : t("workbench:wizardSpecs.replacesLandscape", { count: n });
    return t("workbench:wizardSpecs.targetBuild", { name: place.name, x: target.x, y: target.y, replaced });
  }
  return t("workbench:wizardSpecs.targetTinker", { count: n, matrix: id4(target.matrix), x: target.x, y: target.y });
}

/**
 * What „Anlegen“ will do, one sentence per line, for the last step. Bauen names the area, its place and its look, never the raster or
 * the header (its header is always a new one, see `buildingHeader`). `place` says where an area with a target lies.
 */
export function areaSummary(spec: AreaSpec, headers: readonly HeaderSummary[], links: readonly WarpLink[] = [], depth: Depth = "tuefteln", place: TargetPlace = { name: "", replaced: 0 }): string[] {
  const buildings = spec.cells.every((c) => c.clear_buildings) ? t("workbench:wizardSpecs.withoutBuildings") : t("workbench:wizardSpecs.withBuildings");
  const target = spec.target ?? null;
  const size = { name: spec.name, width: spec.width, height: spec.height };
  if (!depthAtLeast(depth, "tuefteln")) {
    const lines = [t("workbench:wizardSpecs.areaSummaryBuild", size)];
    if (target) lines.push(targetLine(target, place, depth));
    lines.push(t("workbench:wizardSpecs.copies", { maps: mapList(spec.cells.map((c) => c.source_map)), buildings }));
    if (spec.header !== null) lines.push(...buildingLines(spec.header, headers));
    lines.push(notUndoable());
    return lines;
  }
  const lines = target
    ? [t("workbench:wizardSpecs.areaSummaryTarget", size), targetLine(target, place, depth)]
    : [spec.with_headers ? t("workbench:wizardSpecs.rasterSummaryWithHeaders", size) : t("workbench:wizardSpecs.rasterSummary", size)];
  const copies = spec.cells.map((c) => t("workbench:wizardSpecs.copyToCell", { map: id4(c.source_map), x: c.x, y: c.y })).join(", ");
  lines.push(t("workbench:wizardSpecs.copies", { maps: copies, buildings }));
  lines.push(spec.header === null ? t("workbench:wizardSpecs.noHeader") : assignLine(spec.header, headers, target));
  const warnings = spec.header === null ? [] : [specialLine(spec.header, headers), inUseLine(spec.header, headers, links)];
  for (const warning of warnings) if (warning) lines.push(warning);
  lines.push(notUndoable());
  return lines;
}

/**
 * Where the patch a create applied put the old header folder (`report.snapshot`), as a result line; null without one. Tüfteln names the
 * path, Bauen only the backup folder (the path names the header folder).
 */
function snapshotLine(report: Pick<AreaReport, "patched" | "snapshot">, depth: Depth): string | null {
  if (!report.patched || !report.snapshot) return null;
  return !depthAtLeast(depth, "tuefteln") ? t("workbench:wizardSpecs.backupBuild") : t("workbench:wizardSpecs.backupTinker", { path: report.snapshot });
}

/** A wizard's result: what it made, where to go on in words, and the recipes that make sense next. */
export interface WizardOutcome {
  lines: string[];
  next: string[];
  steps: NextStep[];
}

/**
 * What `create_area` made, in words, and where to go on from there; Bauen names the area and its maps only (and that the patch kept the old
 * files). The steps are the kernel's for the recipe (the wizard still calls the old command, which answers none).
 */
export function areaResult(report: AreaReport, spec: AreaSpec, depth: Depth = "tuefteln"): WizardOutcome {
  const snapshot = snapshotLine(report, depth);
  const steps = nextStepsFor("create_area");
  if (!depthAtLeast(depth, "tuefteln")) {
    const count = report.maps.length;
    return {
      steps,
      lines: [t("workbench:wizardSpecs.areaCreated", { name: spec.name, count }), ...(snapshot ? [snapshot] : [])],
      // An area on a world map lies beside its neighbours: no door needed.
      next: [spec.target ? seamless() : t("workbench:wizardSpecs.connectWorld", { setDoor: setDoor() }), t("workbench:wizardSpecs.lookAtArea")],
    };
  }
  // An area on a world map made no raster: it went into the world map's.
  const size = { name: spec.name, width: spec.width, height: spec.height, matrix: id4(report.matrix) };
  const lines = [spec.target ? t("workbench:wizardSpecs.areaCreatedIn", size) : t("workbench:wizardSpecs.rasterCreated", size)];
  const maps = report.maps.map(([, , map]) => id4(map)).join(", ");
  lines.push(t("workbench:wizardSpecs.copied", { maps: `${report.maps.length === 1 ? termLabel("map") : termPlural("map")} ${maps}` }));
  if (report.header !== null) {
    const file = report.event_file === null ? "" : `, ${termLabel("eventFile")} ${id4(report.event_file)}`;
    lines.push(t("workbench:wizardSpecs.headerNow", { header: headerResultName(report.header, spec.header), matrix: id4(report.matrix), file }));
  }
  if (snapshot) lines.push(snapshot);
  return {
    lines,
    steps,
    next: [
      spec.target ? t("workbench:wizardSpecs.seamlessIn", { matrix: id4(report.matrix) }) : t("workbench:wizardSpecs.connectWorldTinker", { setDoor: setDoor() }),
      t("workbench:wizardSpecs.checkMaps"),
    ],
  };
}

/** The header a result names: a header the wizard made is called new, with its internal name. */
function headerResultName(id: number, assign: AssignHeader | null): string {
  const created = assign?.new_header;
  return created
    ? t("workbench:wizardSpecs.newHeaderName", { id: id4(id), name: quoted(created.internal_name), template: id4(created.template) })
    : `${termLabel("header")} ${id4(id)}`;
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

/**
 * The sources Bauen offers a new area, by name: the world maps (one entry per raster, so Johto and Kanto share one) and the areas
 * with a map of their own („Weitere Gebiete“). Rasters without an area are left to Tüfteln, except `current` (the raster the wizard
 * was opened on), named by the room or place that uses it.
 */
export function buildingSources(world: World | null, headers: readonly HeaderSummary[], current: number | null): { value: number; label: string }[] {
  const names = new Map<number, string[]>();
  for (const m of world?.worldMaps ?? []) names.set(m.matrix, [...(names.get(m.matrix) ?? []), m.name]);
  for (const area of world?.moreAreas ?? []) if (!names.has(area.matrix)) names.set(area.matrix, [area.name]);
  const sources = [...names].map(([value, labels]) => ({ value, label: labels.join(" / ") }));
  if (current === null || names.has(current)) return sources;
  const areas = world ? [...world.worldMaps.flatMap((m) => m.areas), ...world.moreAreas] : [];
  const room = [...areas.flatMap((a) => a.rooms), ...(world?.unconnected ?? [])].find((r) => r.header.matrix_id === current);
  const named = headers.find((h) => h.matrix_id === current && h.location_name.trim() !== "")?.location_name.trim();
  return [...sources, { value: current, label: room?.name ?? named ?? UNNAMED_PLACE }];
}

/** The world maps an area can go onto, one per raster: Johto and Kanto share one and are named together („Johto und Kanto“). */
export function worldMapChoices(world: World | null): { matrix: number; name: string }[] {
  const names = new Map<number, string[]>();
  for (const m of world?.worldMaps ?? []) names.set(m.matrix, [...(names.get(m.matrix) ?? []), m.name]);
  return [...names].map(([matrix, list]) => ({ matrix, name: list.join(` ${t("workbench:wizardSpecs.and")} `) }));
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
export const overviewNote = (loaded: boolean, failed: boolean): string | null =>
  loaded ? null : failed ? t("workbench:wizardSpecs.overviewFailed") : t("workbench:wizardSpecs.overviewLoading");

/**
 * Whether a wizard's error says that members stayed behind (the backend's code "partial"; the
 * wizard cannot repair them, the result step says so).
 */
export const isLateFailure = (e: unknown): boolean => isCmdError(e) && e.code === "partial";

/** The words of the members a failed run names, in the language that is shown (resolved when asked, not at module scope). */
const memberWords = (): Record<string, string> => ({
  header: termLabel("header"),
  headers: termPlural("header"),
  matrix: termLabel("matrix"),
  matrices: termPlural("matrix"),
  map: termLabel("map"),
  maps: termPlural("map"),
  "event file": termLabel("eventFile"),
  "event files": termPlural("eventFile"),
});

/** The words of the steps of a failed run. */
const stepWords = (): Record<string, string> => ({
  "header creation": t("workbench:wizardSpecs.step.headerCreation"),
  "map copy": t("workbench:wizardSpecs.step.mapCopy"),
  "matrix creation": t("workbench:wizardSpecs.step.matrixCreation"),
  "event file": t("workbench:wizardSpecs.step.eventFile"),
  "matrix edit": t("workbench:wizardSpecs.step.matrixEdit"),
  "header edit": t("workbench:wizardSpecs.step.headerEdit"),
  "warp edit": t("workbench:wizardSpecs.step.warpEdit"),
  "door tile": t("workbench:wizardSpecs.step.doorTile"),
});

/**
 * Bauen's words for the members and steps: the header is the new area or room itself („das Gebiet“, „der Raum“), its matrix the
 * arrangement of its maps (the wizard's step „Anordnung“). Both are named without their numbers.
 */
function buildingWords(kind: "area" | "room"): { members: Record<string, string>; steps: Record<string, string> } {
  return {
    members: {
      header: t(`workbench:wizardSpecs.building.${kind}.member`),
      headers: t(`workbench:wizardSpecs.building.${kind}.members`),
      matrix: t("workbench:wizardSpecs.building.arrangement"),
      matrices: t("workbench:wizardSpecs.building.arrangements"),
    },
    steps: {
      "header creation": t(`workbench:wizardSpecs.building.${kind}.create`),
      "matrix creation": t("workbench:wizardSpecs.building.createArrangement"),
      "matrix edit": t("workbench:wizardSpecs.building.changeArrangement"),
      "header edit": t(`workbench:wizardSpecs.building.${kind}.setUp`),
    },
  };
}

/** How the backend's late failure begins when the run applied the Dynamic Headers patch (see `created_text` in wizards.rs). */
const PATCH_CLAUSE = "the Dynamic Headers patch was applied and stays";

/**
 * The backend's text of a late failure ("header 0593 and map 0670 were created; map copy failed: …") in German: the patch when the
 * run applied it ("the Dynamic Headers patch was applied and stays, header 0002 was created; …", or with nothing else created
 * "the Dynamic Headers patch was applied and stays; header creation failed: …"), what was created and which step failed; the
 * system's reason stays as it came. Another text comes back unchanged. Bauen calls the patch an adaptation of the game code, the
 * header the new area or room (`kind`) and the matrix its arrangement, both without their numbers.
 */
export function partialText(message: string, depth: Depth = "tuefteln", kind: "area" | "room" = "area"): string {
  const patched = message.startsWith(PATCH_CLAUSE);
  const parts = patched
    ? /^(?:, (.*) (?:was|were) created)?; (.*?) failed: (.*)$/s.exec(message.slice(PATCH_CLAUSE.length))
    : /^(.*) (?:was|were) created; (.*?) failed: (.*)$/s.exec(message);
  if (!parts) return message;
  const [, members, step, reason] = parts;
  const building = !depthAtLeast(depth, "tuefteln") ? buildingWords(kind) : null;
  const said: string[] = [];
  if (patched) said.push(building ? t("workbench:wizardSpecs.patchedBuild") : t("workbench:wizardSpecs.patchedTinker"));
  if (members !== undefined) {
    // "header 0593, matrix 0131 and maps 0003, 0004": each group is a word and its ids.
    const groups = [...members.matchAll(/(event files?|headers?|matrix|matrices|maps?) ([\d, ]+?)(?=,? and |, [a-z]|$)/g)];
    if (groups.length === 0) return message;
    const member = (word: string, ids: string) => building?.members[word] ?? `${memberWords()[word] ?? word} ${ids.trim()}`;
    said.push(t("workbench:wizardSpecs.created", { list: groups.map(([, word, ids]) => member(word, ids)).join(", ") }));
  }
  said.push(t("workbench:wizardSpecs.thenFailed", { step: building?.steps[step] ?? stepWords()[step] ?? step, reason }));
  return said.join(" ");
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
  /** Bauen: the place of the new room; null follows the default (`roomPlace`). */
  place: PlaceChoice | null;
  /** Bauen: the look (area data) of the new room; null keeps the template's. */
  lookAreaData: number | null;
  /** Bauen: the one-time consent to apply the Dynamic Headers patch while creating. */
  patchConsent: boolean;
}

/** The empty form; its `name` is read from the catalog when the form is copied (`{ ...emptyRoomForm }`), never at module scope. */
export const emptyRoomForm: RoomForm = {
  get name() {
    return t("workbench:wizardSpecs.newRoom");
  },
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
  place: null,
  lookAreaData: null,
  patchConsent: false,
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

/** Bauen's name for the header outside a room's entrance, in the language that is shown. */
export const outsideArea = (): string => t("workbench:wizardSpecs.outsideArea");

/**
 * The backend request for the form, or what is wrong with it. `headerCount` is the number of headers there are now (a
 * new header gets that number). Tüfteln assigns the header the form chose; Bauen always makes a new one, a copy of the
 * template room's header (`buildingHeader`; the form's `place` must be resolved, see `roomPlace`). `checks` holds what is
 * known about the outside matrix and the event files (see `RoomChecks`); for the matrix alone its size may be passed.
 */
export function roomSpec(form: RoomForm, headerCount: number, depth: Depth, checks?: RoomChecks | { width: number; height: number } | null): { spec: RoomSpec } | { error: string } {
  const known: RoomChecks = checks && "width" in checks ? { outsideMatrix: checks } : (checks ?? {});
  const outsideMatrix = known.outsideMatrix;
  if (form.template === null || form.templateMap === null) return { error: t("workbench:wizardSpecs.chooseTemplate") };
  const name = form.name.trim();
  if (name === "") return { error: t("workbench:wizardSpecs.nameEmpty") };
  const building = !depthAtLeast(depth, "tuefteln") ? buildingHeader(form, headerCount, form.template) : null;
  if (building !== null && "error" in building) return building;
  const header = building ?? assignHeader(form, headerCount);
  if (header === null) return { error: t("workbench:wizardSpecs.chooseHeader") };
  // Bauen speaks of the area at the entrance, never of the header outside or its raster.
  const words = building !== null;
  if (form.connect) {
    const inside = [form.insideX, form.insideZ];
    if (inside.some((n) => !Number.isInteger(n) || n < 0 || n > MAP_LAST_TILE)) return { error: t("workbench:wizardSpecs.insideOffMap", { max: MAP_LAST_TILE }) };
    if (form.outsideHeader === null) return { error: words ? t("workbench:wizardSpecs.chooseOutsideArea") : t("workbench:wizardSpecs.chooseOutsideHeader") };
    if (form.outsideHeader === header.id) {
      return { error: words ? t("workbench:wizardSpecs.outsideIsRoom") : t("workbench:wizardSpecs.outsideIsRoomHeader") };
    }
    if (form.x === null || form.z === null) return { error: t("workbench:wizardSpecs.enterOutsideTile") };
    if (outsideMatrix) {
      const size = matrixTiles(outsideMatrix);
      if (form.x < 0 || form.z < 0 || form.x >= size.x || form.z >= size.z) {
        const where = { x: form.x, z: form.z, maxX: size.x - 1, maxZ: size.z - 1 };
        return { error: words ? t("workbench:wizardSpecs.offOutsideArea", where) : t("workbench:wizardSpecs.offRaster", where) };
      }
      const [cx, cy] = [Math.floor(form.x / 32), Math.floor(form.z / 32)];
      const cell = cy * outsideMatrix.width + cx;
      if (outsideMatrix.maps && (outsideMatrix.maps[cell] ?? null) === null) {
        return { error: t("workbench:wizardSpecs.cellWithoutMap", { x: form.x, z: form.z, cx, cy }) };
      }
      const owner = outsideMatrix.headers?.[cell];
      if (owner !== undefined && owner !== form.outsideHeader) {
        if (words) return { error: t("workbench:wizardSpecs.cellOfOtherArea", { x: form.x, z: form.z, cx, cy }) };
        return { error: t("workbench:wizardSpecs.cellOfOtherHeader", { x: form.x, z: form.z, cx, cy, owner: id4(owner), outside: id4(form.outsideHeader) }) };
      }
    }
    const taken = warpOnTile(known.outsideWarps, form.x, form.z);
    if (taken) {
      const tile = { x: form.x, z: form.z, index: taken.index };
      return { error: words ? t("workbench:wizardSpecs.takenOutside", tile) : t("workbench:wizardSpecs.takenOutsideTarget", { ...tile, target: id4(taken.dest_header_id) }) };
    }
    // In Bauen the room has a new, empty event file: nothing is kept.
    const kept = header.event_file === "Keep" ? known.keptWarps : null;
    const takenInside = kept ? warpOnTile(kept.warps, form.insideX, form.insideZ) : null;
    if (kept && takenInside) {
      return { error: t("workbench:wizardSpecs.takenInside", { x: form.insideX, z: form.insideZ, index: takenInside.index, file: id4(kept.file) }) };
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

/**
 * What „Anlegen“ will do, one sentence per line, for the last step. Bauen names the room, its place and its look and the area at
 * its entrance, never the raster or the header.
 */
export function roomSummary(spec: RoomSpec, headers: readonly HeaderSummary[], notConnected?: string, links: readonly WarpLink[] = [], doors: DoorNotes = {}, depth: Depth = "tuefteln"): string[] {
  const building = !depthAtLeast(depth, "tuefteln");
  const buildings = spec.clear_buildings ? t("workbench:wizardSpecs.withoutBuildings") : t("workbench:wizardSpecs.withBuildings");
  const copy = { name: spec.name, map: id4(spec.source_map), buildings };
  const lines = building
    ? [t("workbench:wizardSpecs.roomSummaryBuild", copy), ...buildingLines(spec.header, headers)]
    : [t("workbench:wizardSpecs.roomSummaryTinker", copy), assignLine(spec.header, headers)];
  if (!building) for (const warning of [specialLine(spec.header, headers), inUseLine(spec.header, headers, links)]) if (warning) lines.push(warning);
  const entrance = spec.entrance;
  if (entrance === null) {
    const why = notConnected ? ` (${notConnected})` : "";
    lines.push(
      building
        ? t("workbench:wizardSpecs.noEntranceBuild", { why, setDoor: setDoor() })
        : t("workbench:wizardSpecs.noEntranceTinker", { why, warp: termLabel("warp"), warps: termPlural("warp") }),
    );
  } else {
    const outside = headers.find((h) => h.id === entrance.outside_header);
    const from = building ? (outside ? headerLabel(outside) : t("workbench:wizardSpecs.theOutsideArea")) : `${id4(entrance.outside_header)}${outside ? ` ${headerLabel(outside)}` : ""}`;
    lines.push(t("workbench:wizardSpecs.connects", { from, x: entrance.x, z: entrance.z }));
    lines.push(t("workbench:wizardSpecs.wayBack", { x: entrance.inside_x, z: entrance.inside_z }));
    if (doors.reused) {
      lines.push(
        building
          ? t("workbench:wizardSpecs.reused", { index: doors.reused.index })
          : t("workbench:wizardSpecs.reusedTarget", { index: doors.reused.index, target: id4(doors.reused.dest_header_id) }),
      );
    }
    const outsideDead = doors.outsideTile ? !doors.outsideTile.fires : false;
    const insideDead = doors.insideTile ? !doors.insideTile.fires : false;
    if (outsideDead || insideDead) {
      const sides = outsideDead && insideDead ? t("workbench:wizardSpecs.sides.both") : outsideDead ? t("workbench:wizardSpecs.sides.outside") : t("workbench:wizardSpecs.sides.inside");
      lines.push(entrance.door_tiles ? t("workbench:wizardSpecs.makesDoor", { sides }) : t("workbench:wizardSpecs.doesNotFire", { sides }));
    }
  }
  lines.push(notUndoable());
  return lines;
}

/** Where a room is furnished: the tools and the „+“ of „In dieser Szene“. */
const furnish = () => t("workbench:wizardSpecs.furnish");

/** Where a door is set now (after a colon of the step it follows): the tool, or the „+“ of „In dieser Szene“. */
const setDoor = () => t("workbench:wizardSpecs.setDoor");
/** The next step after an area on a world map: it is walked into from its neighbours. */
const seamless = () => t("workbench:wizardSpecs.seamless");

/**
 * What `create_room` made, in words, and where to go on from there; Bauen names the room, its map and the doors, never its raster or
 * header. Both say where the patch the call applied put the old header folder. The steps are the kernel's for the recipe.
 */
export function roomResult(report: RoomReport, spec: RoomSpec, depth: Depth = "tuefteln"): WizardOutcome {
  const building = !depthAtLeast(depth, "tuefteln");
  const made = { name: spec.name, map: id4(report.map), matrix: id4(report.matrix) };
  const lines = building ? [t("workbench:wizardSpecs.roomCreatedBuild", made)] : [t("workbench:wizardSpecs.roomCreatedTinker", made)];
  if (!building) {
    const file = report.event_file === null ? "" : `, ${termLabel("eventFile")} ${id4(report.event_file)}`;
    lines.push(t("workbench:wizardSpecs.headerNow", { header: headerResultName(spec.header.id, spec.header), matrix: id4(report.matrix), file }));
  }
  if (spec.entrance !== null && report.outside_warp !== null && report.inside_warp !== null) {
    const where = building ? t("workbench:wizardSpecs.whereOutside") : t("workbench:wizardSpecs.whereHeader", { id: id4(spec.entrance.outside_header) });
    const warps = { outside: report.outside_warp, where, inside: report.inside_warp };
    lines.push(report.inside_warp_reused ? t("workbench:wizardSpecs.entranceReused", warps) : t("workbench:wizardSpecs.entrance", warps));
  }
  for (const tile of report.door_tiles ?? []) lines.push(t("workbench:wizardSpecs.doorTileSet", { map: id4(tile.map), x: tile.x, z: tile.y }));
  const snapshot = snapshotLine(report, depth);
  if (snapshot) lines.push(snapshot);
  return {
    lines,
    steps: nextStepsFor("create_room"),
    next:
      spec.entrance === null
        ? [t("workbench:wizardSpecs.connectEntrance", { setDoor: setDoor() }), building ? furnish() : t("workbench:wizardSpecs.furnishTinker")]
        : [t("workbench:wizardSpecs.tryRoom"), furnish()],
  };
}

/** Where the first warp of the template room is: the door the player leaves by, and so a good place for the way back in the new room. */
export function templateWarp(events: EventsDto | null): { x: number; z: number } | null {
  const first = events?.warps[0];
  return first ? { x: first.x, z: first.z } : null;
}

// ---------------------------------------------------------------------------------------------------------- recipes of Erzählen (3d.5)

/** What a recipe's `created` names, in words (the members it made first; they stay after undo). */
const createdWords = (): Partial<Record<AssetKind, string>> => ({
  Trainer: "Trainer",
  HeaderTable: termLabel("header"),
  Matrix: termLabel("matrix"),
  Map: termLabel("map"),
  Event: termLabel("eventFile"),
  TextArchive: termLabel("textArchive"),
  Script: termLabel("scriptFile"),
});

/** „Trainer 0042 angelegt (bleibt nach Rückgängig)“: a member a recipe created first. */
export const createdLine = (created: { kind: AssetKind; index: number }): string =>
  t("workbench:wizardSpecs.createdMember", { what: createdWords()[created.kind] ?? created.kind, id: id4(created.index) });

/** What „Neue Figur“ did, in the kernel's numbers; the next steps are the kernel's (`outcome.next`). */
export function figureResult(outcome: RecipeOutcome<NewFigureReport>, textProject = true): WizardOutcome {
  const r = outcome.report;
  return {
    lines: [
      t("workbench:wizardSpecs.objectIn", { object: r.object, file: id4(r.event_file) }),
      t("workbench:wizardSpecs.scriptMessage", { slot: r.slot, message: r.message, archive: id4(r.archive) }),
      ...outcome.created.map(createdLine),
    ],
    // „Zu einer Geschichte“ opens a story recipe, which needs a text project: outside one it is locked with the kernel's sentence.
    steps: textProject ? outcome.next : outcome.next.map((s) => (s.id === "story_join" ? { ...s, available: false, reason: NOT_A_TEXT_PROJECT } : s)),
    next: [t("workbench:wizardSpecs.tryFigure")],
  };
}

/** One end of „Tür verbinden“ as it was left: „Tür A: Warp 2 in Eventdatei 0005“, a new one „(neu)“, and the door tile it got. */
function doorLines(name: "A" | "B", door: DoorReport): string[] {
  const end = { name, warp: door.warp, file: id4(door.event_file) };
  const lines = [door.created ? t("workbench:wizardSpecs.doorEndNew", end) : t("workbench:wizardSpecs.doorEnd", end)];
  if (door.door_tile) lines.push(t("workbench:wizardSpecs.doorEndTile", { name, map: id4(door.door_tile.map), x: door.door_tile.x, z: door.door_tile.y }));
  return lines;
}

/** What „Tür verbinden“ did: both ends, and whether B leads back. */
export function doorsResult(outcome: RecipeOutcome<ConnectDoorsReport>): WizardOutcome {
  const r = outcome.report;
  return {
    lines: [...doorLines("A", r.a), ...doorLines("B", r.b), r.back ? t("workbench:wizardSpecs.doorBack") : t("workbench:wizardSpecs.doorStays")],
    steps: outcome.next,
    next: [t("workbench:wizardSpecs.tryDoor")],
  };
}

/**
 * What „Neuer Trainer hier“ did: the trainer it created (it stays after undo), where it stands, the battle of the preview („Kampfvorschau:
 * Wanderer Jens gegen Glumanda Lv. 12“, from what the wizard sent) and what the kernel says about its texts.
 */
export function trainerResult(outcome: RecipeOutcome<NewTrainerHereReport>, input: NewTrainerHereInput, names: Pick<NameTablesDto, "classes" | "species"> | null): WizardOutcome {
  const r = outcome.report;
  const created = outcome.created.length > 0 ? outcome.created.map(createdLine) : [createdLine({ kind: "Trainer", index: r.trainer })];
  const first = input.party[0];
  // The battle as the preview showed it: the trainer against its first Pokémon (a team the kernel took is never empty; no line without one).
  const battle = first
    ? [
        t("workbench:wizardSpecs.battlePreview", {
          trainer: trainerTitle(className(names?.classes ?? [], input.class), input.name, r.trainer),
          species: speciesName(names?.species ?? [], first.species),
          level: first.level,
        }),
      ]
    : [];
  return {
    lines: [
      ...created,
      t("workbench:wizardSpecs.objectInScript", { object: r.object, file: id4(r.event_file), script: r.script }),
      ...battle,
      ...r.text_problems.map((p) => t("workbench:wizardSpecs.textProblem", { message: p.message })),
    ],
    steps: outcome.next,
    next: [t("workbench:wizardSpecs.tryTrainer")],
  };
}

/** The end of „Tür verbinden“ the wizard sends: a door that is there, or a tile for a new one (exactly one of the two). */
export function doorEnd(end: { header: number | null; mode: "door" | "tile"; warp: number | null; x: number | null; z: number | null }): ConnectDoorsInput["a"] | null {
  if (end.header === null) return null;
  if (end.mode === "door") return end.warp === null ? null : { header: end.header, warp: end.warp };
  return end.x === null || end.z === null ? null : { header: end.header, tile: { x: end.x, z: end.z } };
}

/**
 * The look a recipe gives when the wizard sends none (`crates/g4-kernel/metadata/recipes.yaml` `default_look`, Diamond/Pearl take
 * Platinum's): the wizard shows it until another is picked, and sends `graphics_id: null` so the kernel's value counts.
 */
export function defaultLook(family: string | null | undefined, kind: "figure" | "trainer"): number {
  const hgss = (family ?? "").toLowerCase().startsWith("hg");
  if (kind === "figure") return hgss ? 317 : 6;
  return hgss ? 333 : 20;
}
