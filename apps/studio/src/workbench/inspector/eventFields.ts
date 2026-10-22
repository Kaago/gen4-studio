import type { EventKind, EventPatch, EventsDto } from "../../api";
import { KIND_LABELS, eventAt } from "../../world/events";
import type { EventData } from "../../world/events";
import type { RecordBackend, RecordFields } from "./headerCommit";

/** The fields of an event the backend accepts in a patch (`EventPatch`). */
export type EventFieldKey = keyof EventPatch;

/** An editable number field of an event: its label and the inclusive range its stored type holds. */
export interface EventFieldSpec {
  key: EventFieldKey;
  label: string;
  min: number;
  max: number;
}

const U16 = { min: 0, max: 0xffff };
const I16 = { min: -0x8000, max: 0x7fff };

/** x and z as the kind stores them (u16 for warps and objects, i16 for triggers, i32 for background events). */
function positionRange(kind: EventKind): { min: number; max: number } {
  switch (kind) {
    case "warp":
    case "object":
      return U16;
    case "coord":
      return { min: -0x8000, max: 0x7fff };
    case "bg":
      return { min: -0x8000_0000, max: 0x7fff_ffff };
  }
}

/**
 * The number fields of an event in display order. A warp's destination header is not among them
 * (it is picked from the header list); its height only exists in HeartGold and SoulSilver
 * (`family` "Hgss"), the other games cannot store one.
 */
export function eventFieldSpecs(kind: EventKind, family: string | null): EventFieldSpec[] {
  const pos = positionRange(kind);
  const position: EventFieldSpec[] = [
    { key: "x", label: "x (Kachel)", ...pos },
    { key: "z", label: "z (Kachel)", ...pos },
  ];
  switch (kind) {
    case "warp":
      return [
        ...position,
        { key: "dest_warp_id", label: "Ziel-Warp", ...U16 },
        ...(family === "Hgss" ? [{ key: "height", label: "Höhe", min: 0, max: 0xffff_ffff } satisfies EventFieldSpec] : []),
      ];
    case "object":
      return [
        ...position,
        { key: "local_id", label: "Lokale ID", ...U16 },
        { key: "graphics_id", label: "Grafik", ...U16 },
        { key: "movement_type", label: "Bewegung", ...U16 },
        { key: "dir", label: "Richtung (0 oben, 1 unten, 2 links, 3 rechts)", ...I16 },
        { key: "script", label: "Skript", ...U16 },
      ];
    case "coord":
      return [...position, { key: "width", label: "Breite", ...U16 }, { key: "length", label: "Länge", ...U16 }, { key: "script", label: "Skript", ...U16 }];
    case "bg":
      return [...position, { key: "event_type", label: "Typ", ...U16 }, { key: "script", label: "Skript", ...U16 }];
  }
}

/** `n` rounded to a whole number inside the range of `spec`. */
export function clampEventValue(spec: Pick<EventFieldSpec, "min" | "max">, n: number): number {
  return Math.min(spec.max, Math.max(spec.min, Math.round(n)));
}

const PATCH_KEYS: EventFieldKey[] = ["x", "z", "script", "event_type", "graphics_id", "movement_type", "dir", "local_id", "dest_header_id", "dest_warp_id", "height", "width", "length"];

function value(e: EventData, key: string): number {
  return Number((e as unknown as Record<string, unknown>)[key]);
}

export const EVENT_FIELDS: RecordFields<EventData> = {
  value,
  with: (e, key, v) => ({ ...e, [key]: v }),
};

/** The fields in which `after` differs from `before`; nothing but the patchable fields counts. */
export function eventPatch(before: EventData, after: EventData): EventPatch {
  const patch: EventPatch = {};
  for (const key of PATCH_KEYS) {
    if (!(key in before) || !(key in after)) continue;
    const a = value(after, key);
    if (value(before, key) !== a) patch[key] = a;
  }
  return patch;
}

/**
 * Backend of an `EditSession` over the events of ONE class in ONE event file: the session id is
 * the event's index. A write sends only the fields that differ from what the session last read or
 * wrote. `onEvents` sees every file read (its lock state says whether editing is allowed at all).
 */
export function eventBackend(
  file: number,
  kind: EventKind,
  deps: {
    getEvents: (file: number) => Promise<EventsDto>;
    update: (file: number, kind: EventKind, index: number, patch: EventPatch) => Promise<void>;
    onEvents?: (events: EventsDto) => void;
  },
): RecordBackend<EventData> {
  let last: EventData | null = null;
  return {
    get: async (index) => {
      const events = await deps.getEvents(file);
      deps.onEvents?.(events);
      const event = eventAt(events, kind, index);
      if (!event) throw new Error(`${KIND_LABELS[kind]} ${index} existiert in Event-Datei ${file} nicht (mehr)`);
      return (last = event);
    },
    set: async (index, record) => {
      const patch = last ? eventPatch(last, record) : {};
      await deps.update(file, kind, index, patch);
      last = record;
    },
  };
}
