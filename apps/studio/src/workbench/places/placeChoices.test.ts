import { describe, expect, it } from "vitest";
import type { HeaderSummary, PlaceSlots } from "../../api";
import type { Area, Room, World } from "../worldModel";
import { nameWarning, placeItems, presetChoice, reuseWarning } from "./placeChoices";

const header = (id: number, location_index: number, location_name: string): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  location_index,
  kanto: false,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const room = (h: HeaderSummary, name: string): Room => ({ header: h, name, suggested: name, placeKey: h.location_name, placeName: h.location_name, manual: false, shared: 0 });

// Zweiblattdorf (place 5) has two sections; its rooms are two houses of the village and the lab of Sandgemme (place 6).
const north = header(10, 5, "Zweiblattdorf");
const south = header(11, 5, "Zweiblattdorf");
const house = header(20, 5, "Zweiblattdorf");
const attic = header(21, 5, "Zweiblattdorf");
const lab = header(22, 6, "Sandgemme");
const lonely = header(30, 7, "Kraterberg");
const area: Area = { key: "w0:5", name: "Zweiblattdorf", placeKey: "Zweiblattdorf", headers: [north, south], rooms: [room(house, "Haus 1"), room(attic, "Haus 1 · Dachboden"), room(lab, "Labor")], matrix: 0 };
const world: World = {
  worldMaps: [{ key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area] }],
  moreAreas: [],
  unconnected: [room(lonely, "Höhle")],
  other: [],
  areaOf: new Map([
    [10, "w0:5"],
    [11, "w0:5"],
    [20, "w0:5"],
    [21, "w0:5"],
    [22, "w0:5"],
  ]),
  roomCount: 4,
  assignedRooms: 3,
};

describe("placeItems", () => {
  it("counts_the_sections_of_an_area_once_and_every_room_and_unknown_header_each", () => {
    const stray = header(99, 5, "Zweiblattdorf");
    const items = placeItems([north, south, house, attic, stray], ["Mysteriöser Ort", "", "", "", "", "Zweiblattdorf"], world);
    // One area (two sections), two rooms and a header the world does not know.
    expect(items[5]!.count).toBe("4 Gebiete und Räume");
    // Without a world every header counts.
    expect(placeItems([north, south, house], ["", "", "", "", "", "Zweiblattdorf"])[5]!.count).toBe("3 Gebiete und Räume");
  });

  it("lists_every_name_of_the_bank_with_its_users_and_no_place_first", () => {
    const headers = [header(0, 0, ""), north, south, house, lab];
    const names = ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme", "Route 201"];
    const items = placeItems(
      headers.map((h) => (h.location_index === 5 ? { ...h, location_index: 1 } : h.location_index === 6 ? { ...h, location_index: 2 } : h)),
      names,
    );
    expect(items.map((i) => [i.id, i.label, i.count])).toEqual([
      [0, "Kein Ort (Mysteriöser Ort)", "1 Gebiet oder Raum"],
      [1, "Zweiblattdorf", "3 Gebiete und Räume"],
      [2, "Sandgemme", "1 Gebiet oder Raum"],
      [3, "Route 201", "unbenutzt"],
    ]);
  });
});

describe("presetChoice", () => {
  it("presets_the_area_and_its_rooms_of_the_same_place", () => {
    expect(presetChoice(world, south)).toEqual([
      { id: 10, label: "Zweiblattdorf", checked: true, headers: [10, 11] },
      { id: 20, label: "Haus 1", checked: true, headers: [20] },
      { id: 21, label: "Haus 1 · Dachboden", checked: true, headers: [21] },
      { id: 22, label: "Labor", checked: false, headers: [22] },
    ]);
  });

  it("presets_only_the_room_when_opened_from_a_room", () => {
    expect(presetChoice(world, attic).map((c) => [c.label, c.checked])).toEqual([
      ["Zweiblattdorf", false],
      ["Haus 1", false],
      ["Haus 1 · Dachboden", true],
      ["Labor", false],
    ]);
    // A room without an area is the only choice.
    expect(presetChoice(world, lonely)).toEqual([{ id: 30, label: "Höhle", checked: true, headers: [30] }]);
  });
});

describe("nameWarning", () => {
  it("warns_from_17_characters", () => {
    expect(nameWarning("Sechzehn Zeichen")).toBeNull();
    expect(nameWarning("Siebzehn Zeichen!")).toBe("Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.");
  });
});

describe("reuseWarning", () => {
  it("warns_before_reusing_a_sinnoh_name", () => {
    const names = ["Mysteriöser Ort", "Zweiblattdorf", "Sandgemme"];
    const full: PlaceSlots = { bank: 279, count: 256, free: 0, reusable: [2, 1] };
    expect(reuseWarning(full, names)).toBe(
      "Alle freien Plätze sind belegt. Der neue Ort übernimmt den Platz von „Sandgemme“, einem Sinnoh-Namen, den kein Gebiet nutzt. Pokémon, die über den Pal-Park kommen und dort gefangen wurden, zeigen dann den neuen Namen als Fundort.",
    );
    expect(reuseWarning({ ...full, free: 3 }, names)).toBeNull();
    expect(reuseWarning({ ...full, reusable: [] }, names)).toBeNull();
  });
});
