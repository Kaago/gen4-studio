import { describe, expect, it } from "vitest";
import type { CoordDto, EventPatch, EventsDto, WarpDto } from "../../api";
import type { EventData } from "../../world/events";
import { EVENT_FIELDS, MOVEMENT_WORDS, eventBackend, eventFieldSpecs, eventFieldsAt, eventPatch, movementOptions } from "./eventFields";
import { EditSession } from "./headerCommit";

const warp = (over: Partial<WarpDto> = {}): WarpDto => ({ index: 0, x: 8, z: 4, dest_header_id: 414, dest_warp_id: 1, height: 0, ...over });
const coord = (over: Partial<CoordDto> = {}): CoordDto => ({ index: 0, script: 9000, x: 3, z: 5, width: 2, length: 1, ...over });

describe("eventFieldSpecs", () => {
  const keys = (kind: Parameters<typeof eventFieldSpecs>[0], family: string | null = "Pt") => eventFieldSpecs(kind, family).map((s) => s.key);

  it("offers the fields each kind has, the destination header aside", () => {
    expect(keys("warp")).toEqual(["x", "z", "dest_warp_id"]);
    expect(keys("object")).toEqual(["x", "z", "local_id", "graphics_id", "movement_type", "dir", "script", "trainer_type", "sight"]);
    expect(keys("coord")).toEqual(["x", "z", "width", "length", "script"]);
    expect(keys("bg")).toEqual(["x", "z", "event_type", "script"]);
  });

  it("offers the warp height only where the game stores one", () => {
    expect(keys("warp", "Hgss")).toEqual(["x", "z", "dest_warp_id", "height"]);
    expect(keys("warp", "Dp")).not.toContain("height");
    expect(keys("warp", null)).not.toContain("height");
  });

  it("event_fields_offer_sight_only_for_objects", () => {
    const spec = (key: string) => eventFieldSpecs("object", "Pt").find((s) => s.key === key);
    // The sightline is a choice; a stored value without a name is still shown by the select.
    expect(spec("trainer_type")).toMatchObject({ label: "Sichtlinie", min: 0, max: 65535 });
    expect(spec("trainer_type")?.options).toEqual([
      { value: 0, label: "Aus" },
      { value: 1, label: "Trainer sieht dich" },
    ]);
    // The range in tiles is a number up to 15.
    expect(spec("sight")).toMatchObject({ label: "Sichtweite", min: 0, max: 15 });
    expect(spec("sight")?.options).toBeUndefined();
    for (const kind of ["warp", "coord", "bg"] as const) {
      for (const family of ["Pt", "Hgss"]) {
        expect(keys(kind, family)).not.toContain("trainer_type");
        expect(keys(kind, family)).not.toContain("sight");
      }
    }
  });

  it("labels the facing and limits it to the i16 the object stores", () => {
    const dir = eventFieldSpecs("object", null).find((s) => s.key === "dir");
    expect(dir).toMatchObject({ label: "Blickrichtung", min: -32768, max: 32767 });
    // The four facings are choices, in the order the game numbers them.
    expect(dir?.options).toEqual([
      { value: 0, label: "Oben" },
      { value: 1, label: "Unten" },
      { value: 2, label: "Links" },
      { value: 3, label: "Rechts" },
    ]);
  });

  it("limits x and z to the type the kind stores", () => {
    const range = (kind: Parameters<typeof eventFieldSpecs>[0]) => {
      const x = eventFieldSpecs(kind, null)[0];
      return [x.min, x.max];
    };
    expect(range("warp")).toEqual([0, 65535]);
    expect(range("object")).toEqual([0, 65535]);
    expect(range("coord")).toEqual([-32768, 32767]);
    expect(range("bg")).toEqual([-2147483648, 2147483647]);
  });
});

describe("event fields", () => {
  it("reads and sets numeric fields without touching the rest", () => {
    const w = warp();
    expect(EVENT_FIELDS.value(w, "dest_header_id")).toBe(414);
    expect(EVENT_FIELDS.with(w, "x", 9)).toEqual(warp({ x: 9 }));
    expect(w.x).toBe(8);
  });

  it("sends only the fields that changed", () => {
    expect(eventPatch(warp(), warp({ x: 9, dest_header_id: 1 }))).toEqual({ x: 9, dest_header_id: 1 });
    expect(eventPatch(warp(), warp())).toEqual({});
    // The index is no field; a field the kind lacks never shows up.
    expect(eventPatch(warp(), warp({ index: 4 }))).toEqual({});
    expect(eventPatch(coord(), coord({ width: 5 }))).toEqual({ width: 5 });
    const object: EventData = { index: 0, local_id: 1, graphics_id: 2, movement_type: 0, script: 0, dir: 1, trainer_type: 0, sight: 0, x: 3, z: 4, y: 0 };
    expect(eventPatch(object, { ...object, dir: 3 })).toEqual({ dir: 3 });
    expect(eventPatch(object, { ...object, trainer_type: 1, sight: 4 })).toEqual({ trainer_type: 1, sight: 4 });
  });
});

describe("EditSession over one event", () => {
  const dto = (warps: WarpDto[], locked: string | null = null): EventsDto => ({ event_file_id: 1, locked, bg: [], objects: [], warps, coords: [] });

  it("reads the event it is bound to and writes a patch with the changed field to its file and class", async () => {
    const updates: { file: number; kind: string; index: number; patch: EventPatch }[] = [];
    const seen: EventsDto[] = [];
    const backend = eventBackend(1, "warp", {
      getEvents: async () => structuredClone(dto([warp(), warp({ index: 1, x: 20 })], "17 bytes read")),
      update: async (file, kind, index, patch) => void updates.push({ file, kind, index, patch }),
      onEvents: (e) => seen.push(e),
    });
    const states: EventData[] = [];
    const session = new EditSession<EventData>(1, backend, EVENT_FIELDS, {
      onState: (e) => states.push(e),
      onRefused: () => {},
      onChanged: () => {},
      onError: (e) => {
        throw e;
      },
    });
    await session.load();
    expect(states[0]).toEqual(warp({ index: 1, x: 20 }));
    expect(seen[0].locked).toBe("17 bytes read");
    await session.commit("x", "x", 21);
    await session.commit("x", "x", 21);
    expect(updates).toEqual([{ file: 1, kind: "warp", index: 1, patch: { x: 21 } }]);
  });

  it("fails to load an event that is gone, naming it", async () => {
    const backend = eventBackend(3, "warp", { getEvents: async () => dto([]), update: async () => {} });
    await expect(backend.get(2)).rejects.toThrow("Warp 2 existiert in Event-Datei 3 nicht (mehr)");
  });
});

describe("the words of Erzählen", () => {
  it("movement_words_name_the_common_types", () => {
    for (const { value, label } of MOVEMENT_WORDS) {
      expect(label, `Bewegung ${value}`).toMatch(/^[^\d]+$/);
      expect(label.trim()).toBe(label);
    }
    const values = MOVEMENT_WORDS.map((w) => w.value);
    expect(new Set(values).size).toBe(values.length);
    expect(new Set(MOVEMENT_WORDS.map((w) => w.label)).size).toBe(values.length);
    // Standing still, looking around, wandering, the looking and facing variants and spinning (pret numbering).
    const word = (value: number) => MOVEMENT_WORDS.find((w) => w.value === value)?.label;
    expect(word(0)).toBe("Steht still");
    expect(word(2)).toBe("Schaut sich um");
    expect(word(3)).toBe("Geht umher");
    for (let value = 2; value <= 19; value++) expect(word(value), `Bewegung ${value}`).toBeDefined();
  });

  it("shows_a_stored_movement_without_a_word_as_bewegung_n", () => {
    expect(movementOptions(3)).toBe(MOVEMENT_WORDS);
    expect(movementOptions(999).slice(-1)[0]).toEqual({ value: 999, label: "Bewegung 999" });
  });

  it("names_the_fields_of_erzaehlen_and_offers_the_movement_words", () => {
    const labels = (kind: Parameters<typeof eventFieldsAt>[0], depth: Parameters<typeof eventFieldsAt>[2]) => eventFieldsAt(kind, "Pt", depth).map((s) => s.label);
    expect(labels("object", "erzaehlen")).toEqual(["Aussehen", "Bewegung", "Blickrichtung", "Script"]);
    expect(labels("warp", "erzaehlen")).toEqual(["Tür"]);
    expect(eventFieldsAt("object", "Pt", "erzaehlen").find((s) => s.key === "movement_type")?.options).toBe(MOVEMENT_WORDS);
    // Bauen and Tüfteln are the specs as they are.
    for (const depth of ["bauen", "tuefteln"] as const) {
      expect(eventFieldsAt("object", "Hgss", depth)).toEqual(eventFieldSpecs("object", "Hgss"));
      expect(eventFieldsAt("warp", "Hgss", depth)).toEqual(eventFieldSpecs("warp", "Hgss"));
    }
  });
});
