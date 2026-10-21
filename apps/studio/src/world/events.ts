import type { BgDto, CoordDto, MatrixDto, ObjectDto, WarpDto } from "../api";
import { MAP_SIZE } from "./layout";

export type EventKind = "warp" | "object" | "coord" | "bg";

export type EventData = WarpDto | ObjectDto | CoordDto | BgDto;

export const MARKER_COLORS: Record<EventKind, string> = {
  warp: "#2563eb",
  object: "#eab308",
  coord: "#22c55e",
  bg: "#a855f7",
};

/**
 * Scene position (centre of the tile) of an event at absolute matrix tile (x, z), or null when
 * the tile lies outside the matrix. The scene uses tile units with cell (0, 0) at the origin.
 */
export function markerPosition(x: number, z: number, matrix: MatrixDto): { wx: number; wz: number } | null {
  if (x < 0 || z < 0 || x >= matrix.width * MAP_SIZE || z >= matrix.height * MAP_SIZE) return null;
  return { wx: x + 0.5, wz: z + 0.5 };
}

const id4 = (n: number) => String(n).padStart(4, "0");

/** Headline of an event in the inspector, e.g. "Warp 0 → Header 0414 (T01R0201)". */
export function eventTitle(kind: EventKind, data: EventData, destName?: string): string {
  switch (kind) {
    case "warp": {
      const w = data as WarpDto;
      const dest = `Header ${id4(w.dest_header_id)}`;
      return `Warp ${w.index} → ${destName ? `${dest} (${destName})` : dest}`;
    }
    case "object": {
      const o = data as ObjectDto;
      return `Objekt ${o.index} · ID ${o.local_id}`;
    }
    case "coord": {
      const c = data as CoordDto;
      return `Trigger ${c.index} · Skript ${c.script}`;
    }
    case "bg": {
      const b = data as BgDto;
      return `Hintergrund-Event ${b.index} · Skript ${b.script}`;
    }
  }
}

/** Second inspector line: position and the remaining fields of the event. */
export function eventDetails(kind: EventKind, data: EventData): string {
  switch (kind) {
    case "warp": {
      const w = data as WarpDto;
      return `Position x ${w.x}, z ${w.z} · Ziel-Warp ${w.dest_warp_id} · Höhe ${w.height}`;
    }
    case "object": {
      const o = data as ObjectDto;
      return `Position x ${o.x}, z ${o.z}, y ${o.y} · Grafik ${o.graphics_id} · Bewegung ${o.movement_type} · Skript ${o.script}`;
    }
    case "coord": {
      const c = data as CoordDto;
      return `Position x ${c.x}, z ${c.z} · Größe ${c.width} × ${c.length}`;
    }
    case "bg": {
      const b = data as BgDto;
      return `Position x ${b.x}, z ${b.z}, y ${b.y} · Typ ${b.event_type}`;
    }
  }
}
