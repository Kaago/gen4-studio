import { MATRIX_EMPTY } from "../api";
import type { DataRowDto, EventKind, EventsDto, HeaderSummary, MatrixDto, MatrixSummary } from "../api";
import type { Depth } from "../ui/frame/shellState";
import type { IconName } from "../ui/primitives";
import type { MapBuildings } from "../world/scene/types";
import { groupEvents } from "./context";
import { headerIdentity } from "./headerIdentity";
import type { HeaderIdentity } from "./headerIdentity";
import type { InspectorModel } from "./Inspector";
import { buildPlaces, scriptChildren } from "./places";
import type { Place } from "./places";
import { TERMS, id4, techLabel } from "./terms";
import { scriptTrainerLabel } from "./trainerScript";

/** The groups below the root of „In dieser Szene“, in this order: maps, figures, doors, triggers, signs, scripts. */
export type SceneGroup = "maps" | "object" | "warp" | "coord" | "bg" | "scripts";

/** What a row stands for; header, matrix cell, building and event are the inspector's selections, a script opens its tab. */
export type SceneRef =
  | { kind: "header"; id: number }
  | { kind: "matrixCell"; matrix: number; x: number; y: number }
  | { kind: "building"; map: number; index: number }
  | { kind: "event"; file: number; eventKind: EventKind; index: number }
  | { kind: "script"; file: number; level: boolean };

export interface SceneRow {
  /** Unique in the tree; a branch is open when its key is in the open set. */
  key: string;
  /** Zero-based: the root 0, the groups 1, their rows 2, the buildings of a map 3. */
  depth: number;
  kind: "header" | "group" | "map" | "building" | "event" | "script";
  /** The group of a group row. */
  group?: SceneGroup;
  label: string;
  /** Grey after the label: where an event stands, Tüfteln's header and matrix at the root. */
  tech?: string;
  /** The second line: where a door leads. */
  second?: string;
  icon: IconName;
  ref?: SceneRef;
  /** A door's target: the header id when the headers list it, null for an unknown target. */
  dest?: number | null;
  /** The rows of a group (the hits while searching). */
  count?: number;
  /** Present for a branch: whether it is open. */
  expanded?: boolean;
}

export interface SceneInput {
  header: HeaderSummary;
  headers: readonly HeaderSummary[];
  /** The matrix the scene shows; one of another header (while the new one loads) counts as none. */
  matrix: MatrixDto | null;
  events: EventsDto | null;
  buildings: readonly MapBuildings[];
  trainers?: readonly DataRowDto[] | null;
  depth: Depth;
  /**
   * What the world model says a header is and how it is called (`WorkbenchWorld.identify`): the root's name and symbol, a door's target.
   * Without it the places tell (`headerIdentity`), built once per call.
   */
  identify?: (h: HeaderSummary) => Pick<HeaderIdentity, "kind" | "name">;
}

export const groupKey = (group: SceneGroup) => `group:${group}`;
/** The key of the root (the area or room). */
export const ROOT_KEY = "header";
const GROUPS: readonly SceneGroup[] = ["maps", "object", "warp", "coord", "bg", "scripts"];

/** What starts open: the root and every group; a map's buildings start closed. */
export const SCENE_OPEN: ReadonlySet<string> = new Set([ROOT_KEY, ...GROUPS.map(groupKey)]);

const GROUP_LABEL: Record<SceneGroup, string> = { maps: TERMS.map.plural, object: "Figuren", warp: "Türen", coord: "Auslöser", bg: "Schilder", scripts: TERMS.script.plural };
export const GROUP_ICON: Record<SceneGroup, IconName> = { maps: "map", object: "figure", warp: "door", coord: "trigger", bg: "sign", scripts: "script" };

/** Whether two refs name the same thing. */
export function sameRef(a: SceneRef | null, b: SceneRef | null): boolean {
  if (a === null || b === null || a.kind !== b.kind) return false;
  return (Object.keys(a) as (keyof SceneRef)[]).every((k) => a[k] === b[k]);
}

/** The inspector's selection of a ref; a script has none (it opens in the script tab). */
export function inspectorOfScene(ref: SceneRef): InspectorModel | null {
  switch (ref.kind) {
    case "header":
      return { kind: "header", id: ref.id };
    case "matrixCell":
      return { kind: "matrixCell", matrix: ref.matrix, x: ref.x, y: ref.y };
    case "building":
      return { kind: "building", map: ref.map, index: ref.index };
    case "event":
      return { kind: "event", eventFile: ref.file, eventKind: ref.eventKind, index: ref.index };
    case "script":
      return null;
  }
}

/** The row of an inspector selection (the selected row of the tree); a selection no row stands for (an area data, a hover) has none. */
export function sceneRefOf(model: InspectorModel): SceneRef | null {
  switch (model.kind) {
    case "header":
      return { kind: "header", id: model.id };
    case "matrixCell":
      return { kind: "matrixCell", matrix: model.matrix, x: model.x, y: model.y };
    case "building":
      return { kind: "building", map: model.map, index: model.index };
    case "event":
      return { kind: "event", file: model.eventFile, eventKind: model.eventKind, index: model.index };
    default:
      return null;
  }
}

/** The places `headerIdentity` needs, with the shown matrix as the only one known (it tells the area from a room). */
function placesOf(headers: readonly HeaderSummary[], matrix: MatrixDto | null): Place[] {
  const known: MatrixSummary[] =
    matrix === null
      ? []
      : [{ index: matrix.index, name: matrix.name, width: matrix.width, height: matrix.height, has_headers: (matrix.headers ?? null) !== null, has_altitudes: false, filled: matrix.maps.filter(filled).length }];
  return buildPlaces([...headers], known);
}

const filled = (m: number | null | undefined): m is number => m !== null && m !== undefined && m !== MATRIX_EMPTY;

/** The cells of the matrix that are this header's: those its header section names, or every filled one of a matrix without one. */
function cellsOf(matrix: MatrixDto, header: number): { map: number; x: number; y: number }[] {
  const cells: { map: number; x: number; y: number }[] = [];
  for (const [i, map] of matrix.maps.entries()) {
    if (!filled(map)) continue;
    if (matrix.headers && matrix.headers[i] !== header) continue;
    cells.push({ map, x: i % matrix.width, y: Math.floor(i / matrix.width) });
  }
  return cells;
}

interface Child {
  row: Omit<SceneRow, "depth">;
  children?: Omit<SceneRow, "depth">[];
  /** A search found children: the branch shows open. */
  forced?: boolean;
}

const matches = (q: string, row: Omit<SceneRow, "depth">) => [row.label, row.tech, row.second].some((t) => t !== undefined && t.toLowerCase().includes(q));

/**
 * The rows of „In dieser Szene“ that show: the area or room as the root, below it the groups with rows (maps with their buildings,
 * figures, doors, triggers, signs, scripts); a group without rows is left out. A branch shows its children while its key is in
 * `open`. A search keeps the rows whose title or ident (the grey part, a door's target) contains it, and the groups and maps
 * that hold one, open; a group whose word matches keeps all its rows. Bauen speaks of the game (no "Raster", "Matrix", "Header").
 */
export function sceneRows(input: SceneInput, open: ReadonlySet<string>, query: string): SceneRow[] {
  const { header, headers, events, buildings, trainers = null, depth } = input;
  const tinker = depth === "tuefteln";
  const matrix = input.matrix !== null && input.matrix.index === header.matrix_id ? input.matrix : null;
  // The places are built only without the world model, and then once.
  let places: Place[] | null = null;
  const identity = input.identify ?? ((h: HeaderSummary) => headerIdentity((places ??= placesOf(headers, matrix)), h));
  const nameOf = (h: HeaderSummary) => identity(h).name;

  const groups = new Map<SceneGroup, Child[]>();

  // Maps: the cells of the header, each with the buildings of its map (once, under the first cell of a map).
  if (matrix !== null) {
    const seen = new Set<number>();
    groups.set(
      "maps",
      cellsOf(matrix, header.id).map(({ map, x, y }) => {
        const models = seen.has(map) ? [] : (buildings.find((b) => b.map === map)?.models ?? []);
        seen.add(map);
        const children = models.map((model, index) => ({
          key: `building:${map}:${index}`,
          kind: "building" as const,
          label: `Gebäude ${index + 1} · Modell ${model}`,
          icon: "building" as const,
          ref: { kind: "building" as const, map, index },
        }));
        return {
          row: { key: `map:${x},${y}`, kind: "map", label: `Karte ${id4(map)} · Zelle ${x}, ${y}`, icon: "map", ref: { kind: "matrixCell", matrix: matrix.index, x, y } },
          ...(children.length > 0 ? { children } : {}),
        };
      }),
    );
  }

  if (events !== null) {
    const byId = new Map(headers.map((h) => [h.id, h]));
    const eventGroups = groupEvents(events, depth, undefined, (script) => scriptTrainerLabel(trainers, script));
    for (const g of eventGroups) {
      groups.set(
        g.kind,
        g.rows.map((r) => {
          const ref: SceneRef = { kind: "event", file: events.event_file_id, eventKind: g.kind, index: r.index };
          const base = { key: `event:${g.kind}:${r.index}`, kind: "event" as const, tech: r.tech, icon: GROUP_ICON[g.kind], ref };
          if (r.dest === undefined) return { row: { ...base, label: r.title } };
          const target = byId.get(r.dest);
          const second =
            target === undefined ? `→ unbekanntes Ziel ${id4(r.dest)}` : tinker ? `→ ${techLabel("header", r.dest)} · ${nameOf(target)}` : `→ ${nameOf(target)}`;
          return { row: { ...base, label: `Warp ${r.index}`, second, dest: target === undefined ? null : r.dest } };
        }),
      );
    }
  }

  groups.set(
    "scripts",
    scriptChildren(header).map((child) => {
      const level = child.kind === "levelScript";
      return { row: { key: `script:${level ? "level" : "file"}:${child.file}`, kind: "script", label: child.label, icon: "script", ref: { kind: "script", file: child.file, level } } };
    }),
  );

  const q = query.trim().toLowerCase();
  const searching = q !== "";
  const root = identity(header);
  const rows: SceneRow[] = [
    {
      key: ROOT_KEY,
      depth: 0,
      kind: "header",
      label: root.name,
      ...(tinker ? { tech: `${techLabel("header", header.id)} · ${techLabel("matrix", header.matrix_id)}` } : {}),
      icon: root.kind === "area" ? "outside" : "room",
      ref: { kind: "header", id: header.id },
      expanded: searching || open.has(ROOT_KEY),
    },
  ];
  if (!rows[0]!.expanded) return rows;

  for (const group of GROUPS) {
    const all = groups.get(group) ?? [];
    const label = GROUP_LABEL[group];
    // While searching: the group's word keeps all its rows; else the rows that match, a map also for a matching building.
    const shown: Child[] =
      !searching || label.toLowerCase().includes(q)
        ? all
        : all.flatMap((c) => {
            if (matches(q, c.row)) return [c];
            const hits = (c.children ?? []).filter((b) => matches(q, b));
            return hits.length > 0 ? [{ ...c, children: hits, forced: true }] : [];
          });
    if (shown.length === 0) continue;
    const key = groupKey(group);
    const groupOpen = searching || open.has(key);
    rows.push({ key, depth: 1, kind: "group", group, label, icon: GROUP_ICON[group], count: shown.length, expanded: groupOpen });
    if (!groupOpen) continue;
    for (const c of shown) {
      const branch = c.children !== undefined;
      const childOpen = branch && (c.forced === true || open.has(c.row.key));
      rows.push({ ...c.row, depth: 2, ...(branch ? { expanded: childOpen } : {}) });
      if (childOpen) for (const b of c.children!) rows.push({ ...b, depth: 3 });
    }
  }
  return rows;
}
