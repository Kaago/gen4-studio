import type { HeaderSummary, MatrixSummary, WarpLink } from "../api";
import { id4, termLabel, termPlural } from "./terms";

export type PlaceKind = "outside" | "inside";

export interface PlaceHeader {
  header: HeaderSummary;
  /** `outside` when the overview knows the header's matrix and it has a header section, else `inside`. */
  kind: PlaceKind;
  /** Filled cells of the header's matrix; 0 when the overview does not know the matrix. */
  mapCount: number;
  /** "b × h" for a matrix of more than one cell, else null. */
  size: string | null;
  /** Warps that lead to this header. */
  warpsIn: number;
}

export interface Place {
  key: string;
  name: string;
  headers: PlaceHeader[];
}

export const UNNAMED_PLACE = "Ohne Ortsnamen";
export const UNNAMED_KEY = "__unnamed";

/**
 * Groups headers by location name (the places keep the order of their first header id, the
 * nameless group comes last). Inside a place the outside headers come first, then the rooms, each by id.
 */
export function buildPlaces(headers: HeaderSummary[], matrices: MatrixSummary[], links: WarpLink[] = []): Place[] {
  const matrixByIndex = new Map(matrices.map((m) => [m.index, m]));
  const incoming = new Map<number, number>();
  for (const link of links) incoming.set(link.dest_header_id, (incoming.get(link.dest_header_id) ?? 0) + 1);

  const places = new Map<string, Place>();
  for (const header of [...headers].sort((a, b) => a.id - b.id)) {
    const name = header.location_name.trim();
    const key = name === "" ? UNNAMED_KEY : name;
    let place = places.get(key);
    if (!place) {
      place = { key, name: name === "" ? UNNAMED_PLACE : name, headers: [] };
      places.set(key, place);
    }
    const matrix = matrixByIndex.get(header.matrix_id);
    place.headers.push({
      header,
      kind: matrix?.has_headers ? "outside" : "inside",
      mapCount: matrix?.filled ?? 0,
      size: matrix && matrix.width * matrix.height > 1 ? `${matrix.width} × ${matrix.height}` : null,
      warpsIn: incoming.get(header.id) ?? 0,
    });
  }

  const ordered = [...places.values()];
  for (const place of ordered) {
    // Stable sort: ids are already ascending, so only the kind moves headers.
    place.headers.sort((a, b) => Number(a.kind === "inside") - Number(b.kind === "inside"));
  }
  const unnamed = ordered.filter((p) => p.key === UNNAMED_KEY);
  return [...ordered.filter((p) => p.key !== UNNAMED_KEY), ...unnamed];
}

/**
 * Keeps the places whose name contains `query` whole, and of the others the headers whose code
 * (internal name) or four-digit id contains it; places left empty drop out. An empty query returns `places`.
 */
export function filterPlaces(places: Place[], query: string): Place[] {
  const q = query.trim().toLowerCase();
  if (q === "") return places;
  const result: Place[] = [];
  for (const place of places) {
    if (place.name.toLowerCase().includes(q)) {
      result.push(place);
      continue;
    }
    const headers = place.headers.filter(({ header }) => header.internal_name.toLowerCase().includes(q) || id4(header.id).includes(q));
    if (headers.length > 0) result.push({ ...place, headers });
  }
  return result;
}

export function placeOf(places: Place[], headerId: number): Place | undefined {
  return places.find((p) => p.headers.some((h) => h.header.id === headerId));
}

/** A pastel colour that depends only on the key, for the place's chips and map cells. */
export function placeColor(key: string): string {
  let hash = 0;
  for (let i = 0; i < key.length; i++) hash = (hash * 31 + key.charCodeAt(i)) >>> 0;
  return `hsl(${hash % 360} 60% 80%)`;
}

/** "1 Karte", "4 Karten (2 × 2)"; null when the overview does not know the room's matrix. */
export function mapsText({ mapCount, size }: PlaceHeader): string | null {
  if (mapCount === 0) return null;
  const maps = `${mapCount} ${mapCount === 1 ? termLabel("map") : termPlural("map")}`;
  return size ? `${maps} (${size})` : maps;
}

/** A script file a header uses, as the world explorer lists it below the header. */
export interface ScriptChild {
  kind: "script" | "levelScript";
  file: number;
  /** "Scriptdatei 0042" / "Level-Script 0042". */
  label: string;
}

/**
 * The children of a header in the tree: its script file (file 0 is a file like any other) and, when it has one, its level
 * script (0 means none).
 */
export function scriptChildren(header: Pick<HeaderSummary, "script_file_id" | "level_script_id">): ScriptChild[] {
  const children: ScriptChild[] = [{ kind: "script", file: header.script_file_id, label: `${termLabel("scriptFile")} ${id4(header.script_file_id)}` }];
  if (header.level_script_id !== 0) children.push({ kind: "levelScript", file: header.level_script_id, label: `${termLabel("levelScript")} ${id4(header.level_script_id)}` });
  return children;
}
