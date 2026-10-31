import type { EventKind, EventPatch, EventsDto } from "../../api";
import { KIND_LABELS, eventAt } from "../../world/events";
import { TERMS } from "../terms";
import type { Depth } from "../../ui/frame/shellState";
import { depthAtLeast } from "../../ui/frame/shellState";
import { shownFields } from "./fields";
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
  /** Named choices (a facing): the field is a select, and a stored value without a name is still shown. */
  options?: { value: number; label: string }[];
  /** The lowest depth that shows the field; absent: Bauen. */
  from?: Depth;
}

/** The way an object faces; the game stores it as 0 to 3. */
export const FACINGS = [
  { value: 0, label: "Oben" },
  { value: 1, label: "Unten" },
  { value: 2, label: "Links" },
  { value: 3, label: "Rechts" },
];

/** Whether an object is a trainer who sees the player (`trainer_type`); other stored values stay visible. */
export const SIGHTLINES = [
  { value: 0, label: "Aus" },
  { value: 1, label: "Trainer sieht dich" },
];

/** Largest sight range in tiles the field offers. */
export const SIGHT_MAX = 15;

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
        { key: "dest_warp_id", label: "Ziel-Warp", ...U16, from: "erzaehlen" },
        ...(family === "Hgss" ? [{ key: "height", label: "Höhe", min: 0, max: 0xffff_ffff } satisfies EventFieldSpec] : []),
      ];
    case "object":
      return [
        ...position,
        { key: "local_id", label: "Lokale ID", ...U16 },
        { key: "graphics_id", label: "Grafik", ...U16, from: "erzaehlen" },
        { key: "movement_type", label: "Bewegung", ...U16, from: "erzaehlen" },
        { key: "dir", label: "Blickrichtung", ...I16, options: FACINGS, from: "erzaehlen" },
        { key: "script", label: TERMS.script.label, ...U16, from: "erzaehlen" },
        { key: "trainer_type", label: "Sichtlinie", ...U16, options: SIGHTLINES },
        { key: "sight", label: "Sichtweite", min: 0, max: SIGHT_MAX },
      ];
    case "coord":
      return [...position, { key: "width", label: "Breite", ...U16 }, { key: "length", label: "Länge", ...U16 }, { key: "script", label: TERMS.script.label, ...U16 }];
    case "bg":
      return [...position, { key: "event_type", label: "Typ", ...U16 }, { key: "script", label: TERMS.script.label, ...U16 }];
  }
}

/**
 * What a figure does, in words (pret movement types, numbered as DSPRE's `movementTypesArray`); a stored value without a word shows as
 * „Bewegung N“. 1 has no word: DSPRE calls it „None“ like 0, and a second word for the same thing would only puzzle. Cross-checked against
 * the figures of Platinum DE and HeartGold DE (31.10.2026): every value at least five figures use has a word. 47 (DSPRE: „Crashes game
 * (?)“) is used by 118 Platinum figures, all with graphics id 100 and none in HeartGold: the berry plants of Sinnoh's soft soil.
 */
export const MOVEMENT_WORDS: { value: number; label: string }[] = [
  { value: 0, label: "Steht still" },
  { value: 2, label: "Schaut sich um" },
  { value: 3, label: "Geht umher" },
  { value: 4, label: "Geht auf und ab" },
  { value: 5, label: "Geht hin und her" },
  { value: 6, label: "Schaut nach oben und links" },
  { value: 7, label: "Schaut nach oben und rechts" },
  { value: 8, label: "Schaut nach unten und links" },
  { value: 9, label: "Schaut nach unten und rechts" },
  { value: 10, label: "Schaut nach oben, unten und links" },
  { value: 11, label: "Schaut nach oben, rechts und unten" },
  { value: 12, label: "Schaut nach rechts, links und oben" },
  { value: 13, label: "Schaut nach rechts, links und unten" },
  { value: 14, label: "Blickt nach oben" },
  { value: 15, label: "Blickt nach unten" },
  { value: 16, label: "Blickt nach links" },
  { value: 17, label: "Blickt nach rechts" },
  { value: 18, label: "Dreht sich gegen den Uhrzeigersinn" },
  { value: 19, label: "Dreht sich im Uhrzeigersinn" },
  { value: 20, label: "Läuft auf und ab" },
  { value: 44, label: "Läuft eine Runde im Uhrzeigersinn" },
  { value: 45, label: "Schaut nach oben und unten" },
  { value: 46, label: "Schaut nach rechts und links" },
  { value: 47, label: "Beerenpflanze" },
  { value: 48, label: "Folgt dem Spieler" },
  { value: 51, label: "Versteckt im Schnee" },
  { value: 53, label: "Versteckt unter der Erde" },
  { value: 54, label: "Versteckt im Gras" },
];

/** The choices of the field „Bewegung“ in Erzählen: the words, and a stored value without one as „Bewegung N“. */
export function movementOptions(value: number): { value: number; label: string }[] {
  return MOVEMENT_WORDS.some((w) => w.value === value) ? MOVEMENT_WORDS : [...MOVEMENT_WORDS, { value, label: `Bewegung ${value}` }];
}

/** What Erzählen calls the fields it shows (the movement becomes a choice of words). */
const NARRATING: Partial<Record<EventFieldKey, Partial<EventFieldSpec>>> = {
  graphics_id: { label: "Aussehen" },
  movement_type: { options: MOVEMENT_WORDS },
  dest_warp_id: { label: "Tür" },
};

/** The fields of an event `depth` shows, in its words: Erzählen only those from Erzählen, Bauen and Tüfteln `eventFieldSpecs` as it is. */
export function eventFieldsAt(kind: EventKind, family: string | null, depth: Depth): EventFieldSpec[] {
  const specs = shownFields(eventFieldSpecs(kind, family), depth);
  return depthAtLeast(depth, "bauen") ? specs : specs.map((s) => ({ ...s, ...NARRATING[s.key] }));
}

const PATCH_KEYS: EventFieldKey[] = ["x", "z", "script", "event_type", "graphics_id", "movement_type", "dir", "local_id", "dest_header_id", "dest_warp_id", "height", "width", "length", "trainer_type", "sight"];

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
