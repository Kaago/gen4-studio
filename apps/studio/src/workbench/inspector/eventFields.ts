import type { EventKind, EventPatch, EventsDto } from "../../api";
import { t } from "../../i18n";
import { KIND_LABELS, eventAt } from "../../world/events";
import { termLabel } from "../terms";
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
  /** A flag or a variable: the field shows its name and picks it (`FlagField`); 0 is none and stays a valid value. */
  pick?: "flag" | "var";
}

/**
 * A named choice whose label is read from the catalog each time it is shown, in the language that is shown then: the tables below
 * live at module scope, where i18next is not ready yet. Copying a choice (`{ ...c }`) reads its label at that moment.
 */
function choice(value: number, label: () => string): { value: number; label: string } {
  return {
    value,
    get label() {
      return label();
    },
  };
}

/** The way an object faces; the game stores it as 0 to 3. */
export const FACINGS = [
  choice(0, () => t("workbench:eventFields.facing.up")),
  choice(1, () => t("workbench:eventFields.facing.down")),
  choice(2, () => t("workbench:eventFields.facing.left")),
  choice(3, () => t("workbench:eventFields.facing.right")),
];

/** Whether an object is a trainer who sees the player (`trainer_type`); other stored values stay visible. */
export const SIGHTLINES = [choice(0, () => t("workbench:eventFields.sightline.off")), choice(1, () => t("workbench:eventFields.sightline.seesYou"))];

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
function baseFieldSpecs(kind: EventKind, family: string | null): EventFieldSpec[] {
  const pos = positionRange(kind);
  const position: EventFieldSpec[] = [
    { key: "x", label: t("workbench:eventFields.x"), ...pos },
    { key: "z", label: t("workbench:eventFields.z"), ...pos },
  ];
  switch (kind) {
    case "warp":
      return [
        ...position,
        { key: "dest_warp_id", label: t("workbench:eventFields.destWarp"), ...U16, from: "erzaehlen" },
        ...(family === "Hgss" ? [{ key: "height", label: t("workbench:eventFields.height"), min: 0, max: 0xffff_ffff } satisfies EventFieldSpec] : []),
      ];
    case "object":
      return [
        ...position,
        { key: "local_id", label: t("workbench:eventFields.localId"), ...U16 },
        { key: "graphics_id", label: t("workbench:eventFields.graphics"), ...U16, from: "erzaehlen" },
        { key: "movement_type", label: t("workbench:eventFields.movementType"), ...U16, from: "erzaehlen" },
        { key: "dir", label: t("workbench:eventFields.dir"), ...I16, options: FACINGS, from: "erzaehlen" },
        { key: "script", label: termLabel("script"), ...U16, from: "erzaehlen" },
        { key: "trainer_type", label: t("workbench:eventFields.trainerType"), ...U16, options: SIGHTLINES },
        { key: "sight", label: t("workbench:eventFields.sight"), min: 0, max: SIGHT_MAX },
        { key: "hidden_flag", label: t("workbench:eventFields.hiddenFlag"), ...U16, pick: "flag" },
      ];
    case "coord":
      return [
        ...position,
        { key: "width", label: t("workbench:eventFields.width"), ...U16 },
        { key: "length", label: t("workbench:eventFields.length"), ...U16 },
        { key: "script", label: termLabel("script"), ...U16 },
      ];
    case "bg":
      return [...position, { key: "event_type", label: t("workbench:eventFields.eventType"), ...U16 }, { key: "script", label: termLabel("script"), ...U16 }];
  }
}

/** What a coordinate trigger watches (3d.1): its variable, picked like a flag, and the value at which it fires; after „Script“. */
const triggerVarFields = (): EventFieldSpec[] => [
  { key: "var", label: t("workbench:eventFields.var"), ...U16, pick: "var" },
  { key: "value", label: t("workbench:eventFields.value"), ...U16 },
];

/** The number fields of an event in display order (see `baseFieldSpecs`), a trigger's variable and value last. */
export function eventFieldSpecs(kind: EventKind, family: string | null): EventFieldSpec[] {
  const specs = baseFieldSpecs(kind, family);
  return kind === "coord" ? [...specs, ...triggerVarFields()] : specs;
}

/**
 * What a figure does, in words (pret movement types, numbered as DSPRE's `movementTypesArray`); a stored value without a word shows as
 * „Bewegung N“. 1 has no word: DSPRE calls it „None“ like 0, and a second word for the same thing would only puzzle. Cross-checked against
 * the figures of Platinum DE and HeartGold DE (31.10.2026): every value at least five figures use has a word. 47 (DSPRE: „Crashes game
 * (?)“) is used by 118 Platinum figures, all with graphics id 100 and none in HeartGold: the berry plants of Sinnoh's soft soil.
 */
const MOVEMENTS = [
  [0, "standsStill"],
  [2, "looksAround"],
  [3, "walksAround"],
  [4, "walksUpAndDown"],
  [5, "walksLeftAndRight"],
  [6, "looksUpAndLeft"],
  [7, "looksUpAndRight"],
  [8, "looksDownAndLeft"],
  [9, "looksDownAndRight"],
  [10, "looksUpDownAndLeft"],
  [11, "looksUpRightAndDown"],
  [12, "looksRightLeftAndUp"],
  [13, "looksRightLeftAndDown"],
  [14, "facesUp"],
  [15, "facesDown"],
  [16, "facesLeft"],
  [17, "facesRight"],
  [18, "spinsCounterclockwise"],
  [19, "spinsClockwise"],
  [20, "pacesUpAndDown"],
  [44, "walksClockwiseLoop"],
  [45, "looksUpAndDown"],
  [46, "looksRightAndLeft"],
  [47, "berryPlant"],
  [48, "followsThePlayer"],
  [51, "hiddenInSnow"],
  [53, "hiddenUnderground"],
  [54, "hiddenInGrass"],
] as const;

export const MOVEMENT_WORDS: { value: number; label: string }[] = MOVEMENTS.map(([value, word]) => choice(value, () => t(`workbench:eventFields.movement.${word}`)));

/** The choices of the field „Bewegung“ in Erzählen: the words, and a stored value without one as „Bewegung N“. */
export function movementOptions(value: number): { value: number; label: string }[] {
  return MOVEMENT_WORDS.some((w) => w.value === value) ? MOVEMENT_WORDS : [...MOVEMENT_WORDS, { value, label: t("workbench:eventFields.movementN", { value }) }];
}

/** What Erzählen calls the fields it shows (the movement becomes a choice of words). */
const narrating = (key: EventFieldKey): Partial<EventFieldSpec> | undefined => {
  switch (key) {
    case "graphics_id":
      return { label: t("workbench:eventFields.look") };
    case "movement_type":
      return { options: MOVEMENT_WORDS };
    case "dest_warp_id":
      return { label: t("workbench:eventFields.door") };
    default:
      return undefined;
  }
};

/** The fields of an event `depth` shows, in its words: Erzählen only those from Erzählen, Bauen and Tüfteln `eventFieldSpecs` as it is. */
export function eventFieldsAt(kind: EventKind, family: string | null, depth: Depth): EventFieldSpec[] {
  const specs = shownFields(eventFieldSpecs(kind, family), depth);
  return depthAtLeast(depth, "bauen") ? specs : specs.map((s) => ({ ...s, ...narrating(s.key) }));
}

const PATCH_KEYS: EventFieldKey[] = ["x", "z", "script", "event_type", "graphics_id", "movement_type", "dir", "local_id", "dest_header_id", "dest_warp_id", "height", "width", "length", "trainer_type", "sight"];
// Flags and variables (3d.1).
PATCH_KEYS.push("hidden_flag", "var", "value");

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
      if (!event) throw new Error(t("workbench:eventFields.gone", { kind: KIND_LABELS[kind], index, file }));
      return (last = event);
    },
    set: async (index, record) => {
      const patch = last ? eventPatch(last, record) : {};
      await deps.update(file, kind, index, patch);
      last = record;
    },
  };
}
