import { isCmdError } from "../api";
import type { BgDto, CoordDto, EventKind, EventsDto, HeaderSummary, MatrixDto, ObjectDto, WarpDto } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { termFor, termLabel } from "../workbench/terms";
import { eventColor } from "./colors";
import { MAP_SIZE } from "./layout";

export type { EventKind } from "../api";

/** Inspector names of the event classes. */
export const KIND_LABELS: Record<EventKind, string> = {
  warp: "Warp",
  object: "Objekt",
  coord: "Trigger",
  bg: "Hintergrund-Event",
};

/** The classes in the order they are offered. */
export const EVENT_KINDS: EventKind[] = ["warp", "object", "coord", "bg"];

export type EventData = WarpDto | ObjectDto | CoordDto | BgDto;

/** Marker colours from the scene tokens: a warp is the door, an object the figure, a trigger the zone, a background event the sign. */
export const MARKER_COLORS: Record<EventKind, string> = {
  warp: eventColor("warp"),
  object: eventColor("figur"),
  coord: eventColor("ausloeser"),
  bg: eventColor("schild"),
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

/**
 * Headline of an event in the inspector. A door: Tüfteln "Warp 0 → Header 0414 (T01R0201)" with `destName` the internal name;
 * Bauen "Warp 0 → Haus 2" with `destName` the name the world tree gives the destination, without one "Warp 0 → Ort 0414".
 */
export function eventTitle(kind: EventKind, data: EventData, depth: Depth, destName?: string): string {
  switch (kind) {
    case "warp": {
      const w = data as WarpDto;
      const dest = `${termFor("header", depth)} ${id4(w.dest_header_id)}`;
      if (!depthAtLeast(depth, "tuefteln")) return `Warp ${w.index} → ${destName || dest}`;
      return `Warp ${w.index} → ${destName ? `${dest} (${destName})` : dest}`;
    }
    case "object": {
      const o = data as ObjectDto;
      return `Objekt ${o.index} · ID ${o.local_id}`;
    }
    case "coord": {
      const c = data as CoordDto;
      return `Trigger ${c.index} · ${termLabel("script")} ${c.script}`;
    }
    case "bg": {
      const b = data as BgDto;
      return `Hintergrund-Event ${b.index} · ${termLabel("script")} ${b.script}`;
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
      return `Position x ${o.x}, z ${o.z}, y ${o.y} · Grafik ${o.graphics_id} · Bewegung ${o.movement_type} · ${termLabel("script")} ${o.script}`;
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

/** The event `index` of class `kind` in `events`, if there is one. */
export function eventAt(events: EventsDto, kind: EventKind, index: number): EventData | undefined {
  const list: EventData[] = kind === "warp" ? events.warps : kind === "object" ? events.objects : kind === "coord" ? events.coords : events.bg;
  return list.find((e) => e.index === index);
}

/** The tile (absolute over the matrix) nearest to (x, z) that lies inside the matrix. */
export function clampToMatrix(tile: { x: number; z: number }, matrix: MatrixDto): { x: number; z: number } {
  const clamp = (v: number, size: number) => Math.min(size * MAP_SIZE - 1, Math.max(0, v));
  return { x: clamp(tile.x, matrix.width), z: clamp(tile.z, matrix.height) };
}

/**
 * Where a warp leads: "0414 Zweiblattdorf (T01R0201)" for a header in the list (`known`), else
 * "unbekannt (1234)". A header without a location name shows its internal name once.
 */
export function warpDestinationLabel(dest: number, headers: HeaderSummary[]): { text: string; known: boolean } {
  const h = headers.find((x) => x.id === dest);
  if (!h) return { text: `unbekannt (${dest})`, known: false };
  const text = h.location_name ? `${id4(h.id)} ${h.location_name} (${h.internal_name})` : `${id4(h.id)} ${h.internal_name}`;
  return { text, known: true };
}

/** The shown form of a locked event file's reason, e.g. "schreibgeschützt: 17 bytes read, 16 would be written". */
export function lockedText(reason: string): string {
  return `schreibgeschützt: ${reason}`;
}

/** The reason carried by a "read_only" error about an event file (the message minus its "... is read-only: " lead), else null. */
export function eventReadOnlyReason(e: unknown): string | null {
  if (!isCmdError(e) || e.code !== "read_only" || e.asset?.kind !== "Event") return null;
  const marker = " is read-only: ";
  const at = e.message.indexOf(marker);
  return at < 0 ? e.message : e.message.slice(at + marker.length);
}

/** Added to the confirmation for a warp: what deleting it does to later doors and the warps to them, and what refuses it (3d.4). */
export const WARP_RENUMBER_NOTE =
  "Spätere Türen rücken nach; Warps, die auf sie zeigen, werden nachgezogen. Eine Tür, auf die ein Warp zeigt oder die ein Script per Index umsetzt, lässt sich nicht löschen.";

/** Text of the confirmation before an event is deleted: its class, index and position. */
export function eventDeletePrompt(kind: EventKind, index: number, data: Pick<EventData, "x" | "z"> | undefined): string {
  const what = `${KIND_LABELS[kind]} ${index}`;
  const ask = data ? `${what} (x ${data.x}, z ${data.z}) löschen?` : `${what} löschen?`;
  // The game addresses a warp by its index in the file: later warps move up by one, and the kernel
  // renumbers the warps that point at them (`delete::warps`), or refuses a door a warp or script targets.
  return kind === "warp" ? `${ask}\n\n${WARP_RENUMBER_NOTE}` : ask;
}

/** An event by event file, class and index: the identity the scene and the inspector bind to. */
export interface EventRef {
  file: number;
  kind: EventKind;
  index: number;
}

/** The shown event file and why it is locked (read-only). */
export interface EventLock {
  file: number;
  reason: string;
}

/**
 * Whether events of `file` may be added, deleted or moved, given the lock state of the shown event
 * file (null: it is not locked, or none is shown). A refusal carries the text to show.
 */
export function eventEditGate(lock: EventLock | null, file: number): { allowed: true } | { allowed: false; text: string } {
  return lock !== null && lock.file === file ? { allowed: false, text: lockedText(lock.reason) } : { allowed: true };
}
