import { describe, expect, it } from "vitest";
import type { HeaderSummary, MatrixDto, MatrixSummary, WarpLink, WorldMeta } from "../api";
import { UNNAMED_KEY, unnamedPlace } from "./places";
import { type Area, type World, type WorldInput, buildWorld, worldNames } from "./worldModel";

interface HeaderSpec {
  name: string;
  place?: string;
  index?: number;
  matrix: number;
  event?: number;
  type?: number;
  kanto?: boolean;
}

const header = (id: number, { name, place = "", index = 0, matrix, event = id, type = 4, kanto = false }: HeaderSpec): HeaderSummary => ({
  id,
  internal_name: name,
  location_name: place,
  location_index: index,
  kanto,
  map_type: type,
  matrix_id: matrix,
  area_data_id: 0,
  event_file_id: event,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const summary = (index: number, width: number, height: number, has_headers: boolean, filled = width * height): MatrixSummary => ({
  index,
  name: `m${index}`,
  width,
  height,
  has_headers,
  has_altitudes: false,
  filled,
});

const grid = (index: number, width: number, height: number, headers: number[], maps?: (number | null)[]): MatrixDto => ({
  index,
  name: `m${index}`,
  width,
  height,
  maps: maps ?? headers.map((_, i) => 100 * index + i),
  headers,
});

const warp = (event_file: number, dest_header_id: number, index = 0): WarpLink => ({ event_file, index, x: 0, z: 0, dest_header_id, dest_warp_id: 0 });

const emptyMeta = (): WorldMeta => ({ version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} });

// Matrix 0 (3 × 3, header section): Zweiblattdorf (1), Route 201 (2, 3), landscape (0), one empty cell.
// Matrix 5: the Kampfzone without a header section; matrix 7: Großmoor with a section and one place.
// Rooms: 4 house of Zweiblattdorf (warp from 1), 5 cave level 1 (warp to 3), 6 level 2 (only via 5), 7 house without
// warp in Zweiblattdorf, 8 its own place without warp, 11 and 12 share event file 0 with header 0 (its warp leads to 1).
// Other: 13 nameless on matrix 0 without a cell, 14 on an unknown matrix, 15 on a matrix without maps.
const ptHeaders = [
  header(0, { name: "EVERYWHERE", place: "Mysteriöser Ort", matrix: 0, event: 0, type: 2 }),
  header(1, { name: "T01", place: "Zweiblattdorf", index: 2, matrix: 0, type: 1 }),
  header(2, { name: "R201A", place: "Route 201", index: 3, matrix: 0, type: 2 }),
  header(3, { name: "R201B", place: "Route 201", index: 3, matrix: 0, type: 2 }),
  header(4, { name: "T01R0101", place: "Zweiblattdorf", index: 2, matrix: 20 }),
  header(5, { name: "D01R0101", place: "Bergwerk", index: 9, matrix: 21, type: 3 }),
  header(6, { name: "D01R0102", place: "Bergwerk", index: 9, matrix: 22, type: 3 }),
  header(7, { name: "T01R0201", place: "Zweiblattdorf", index: 2, matrix: 20 }),
  header(8, { name: "D99", place: "Villa", index: 40, matrix: 23 }),
  header(9, { name: "D32", place: "Kampfzone", index: 60, matrix: 5, type: 2 }),
  header(10, { name: "D06", place: "Großmoor", index: 70, matrix: 7, type: 2 }),
  header(11, { name: "DIRECT1", place: "Mysteriöser Ort", matrix: 24, event: 0 }),
  header(12, { name: "DIRECT2", place: "Mysteriöser Ort", matrix: 24, event: 0 }),
  header(13, { name: "R226A", place: "Mysteriöser Ort", matrix: 0, type: 2 }),
  header(14, { name: "LOST", place: "Zweiblattdorf", index: 2, matrix: 99 }),
  header(15, { name: "EMPTY", place: "Zweiblattdorf", index: 2, matrix: 25 }),
];

const ptMatrices = [
  summary(0, 3, 3, true, 8),
  summary(5, 2, 2, false),
  summary(7, 2, 2, true),
  ...[20, 21, 22, 23, 24].map((i) => summary(i, 1, 1, false)),
  summary(25, 1, 1, false, 0),
];

const ptCells = (): Map<number, MatrixDto> =>
  new Map([
    [0, grid(0, 3, 3, [1, 1, 0, 2, 3, 0, 0, 0, 0], [1, 2, 3, 4, 5, 6, 7, 8, null])],
    [7, grid(7, 2, 2, [10, 0, 0, 10])],
  ]);

const ptLinks = [warp(1, 4), warp(5, 3), warp(6, 5), warp(0, 1)];

const ptInput = (patch: Partial<WorldInput> = {}): WorldInput => ({
  family: "Pt",
  headers: ptHeaders,
  matrices: ptMatrices,
  links: ptLinks,
  cells: ptCells(),
  meta: emptyMeta(),
  ...patch,
});

// HeartGold/SoulSilver: one matrix with Johto (1, 2) and Kanto (3, 4); Route 29 has the same index in both halves.
// Room 5 is a Kanto room with an entrance at 4, room 6 a Kanto room of Route 29 without one.
const hgssHeaders = [
  header(0, { name: "EVERYWHERE", matrix: 0, event: 0 }),
  header(1, { name: "T20", place: "Neuborkia", index: 126, matrix: 0, type: 1 }),
  header(2, { name: "R29", place: "Route 29", index: 127, matrix: 0, type: 2 }),
  header(3, { name: "T01", place: "Alabastia", index: 200, matrix: 0, type: 1, kanto: true }),
  header(4, { name: "R29K", place: "Route 29", index: 127, matrix: 0, type: 2, kanto: true }),
  header(5, { name: "R29KR0101", place: "Route 29", index: 127, matrix: 10, kanto: true }),
  header(6, { name: "R29KR0201", place: "Route 29", index: 127, matrix: 10, kanto: true }),
];

const hgssInput = (patch: Partial<WorldInput> = {}): WorldInput => ({
  family: "Hgss",
  headers: hgssHeaders,
  matrices: [summary(0, 3, 2, true), summary(10, 1, 1, false)],
  links: [warp(5, 4)],
  cells: new Map([[0, grid(0, 3, 2, [1, 2, 0, 0, 3, 4])]]),
  meta: emptyMeta(),
  ...patch,
});

const allAreas = (world: World): Area[] => [...world.worldMaps.flatMap((m) => m.areas), ...world.moreAreas];
const areaKeyOf = (world: World, id: number) => world.areaOf.get(id);
const roomIds = (area: Area) => area.rooms.map((r) => r.header.id);
const area = (world: World, key: string): Area => {
  const found = allAreas(world).find((a) => a.key === key);
  if (!found) throw new Error(`no area ${key}`);
  return found;
};

describe("buildWorld", () => {
  it("finds_one_world_map_with_two_areas_and_the_landscape_left_out", () => {
    const world = buildWorld(ptInput());
    expect(world.worldMaps.map((m) => [m.key, m.matrix, m.filter, m.name])).toEqual([["w0", 0, null, "Sinnoh"]]);
    const areas = world.worldMaps[0].areas;
    expect(areas.map((a) => [a.key, a.name, a.placeKey])).toEqual([
      ["w0:2", "Zweiblattdorf", "Zweiblattdorf"],
      ["w0:3", "Route 201", "Route 201"],
    ]);
    expect(allAreas(world).some((a) => a.headers.some((h) => h.id === 0))).toBe(false);
  });

  it("groups_the_headers_of_one_place_into_one_area", () => {
    const route = area(buildWorld(ptInput()), "w0:3");
    expect(route.headers.map((h) => h.id)).toEqual([2, 3]);
    expect(route.matrix).toBe(0);
  });

  it("puts_standalone_matrices_under_more_areas", () => {
    const world = buildWorld(ptInput());
    expect(world.moreAreas.map((a) => [a.key, a.name, a.matrix, a.headers.map((h) => h.id)])).toEqual([
      ["m5", "Kampfzone", 5, [9]],
      ["m7", "Großmoor", 7, [10]],
    ]);
  });

  it("does_not_take_a_room_matrix_without_section_as_an_area", () => {
    // A 2 × 2 cave (type 3) and a single-cell route (type 2) stay rooms.
    const headers = [...ptHeaders, header(16, { name: "D02R0101", place: "Höhle", index: 11, matrix: 26, type: 3 }), header(17, { name: "R299", place: "Route 299", index: 12, matrix: 27, type: 2 })];
    const matrices = [...ptMatrices, summary(26, 2, 2, false), summary(27, 1, 1, false)];
    const world = buildWorld(ptInput({ headers, matrices }));
    expect(world.moreAreas.map((a) => a.key)).toEqual(["m5", "m7"]);
    expect(world.unconnected.map((r) => r.header.id)).toContain(16);
    expect(world.unconnected.map((r) => r.header.id)).toContain(17);
  });

  it("assigns_a_room_to_the_area_of_its_entrance", () => {
    const world = buildWorld(ptInput());
    expect(areaKeyOf(world, 4)).toBe("w0:2");
    expect(areaKeyOf(world, 5)).toBe("w0:3");
    const house = area(world, "w0:2").rooms.find((r) => r.header.id === 4);
    expect(house).toMatchObject({ name: "Haus 1", suggested: "Haus 1", placeKey: "Zweiblattdorf", placeName: "Zweiblattdorf", manual: false, shared: 1 });
  });

  it("follows_rooms_through_rooms_to_the_area", () => {
    const world = buildWorld(ptInput());
    expect(areaKeyOf(world, 6)).toBe("w0:3");
    const cave = area(world, "w0:3");
    expect(roomIds(cave)).toEqual([5, 6]);
    expect(cave.rooms.map((r) => [r.name, r.placeName])).toEqual([
      ["Ebene 1", "Bergwerk"],
      ["Ebene 2", "Bergwerk"],
    ]);
  });

  it("prefers_the_area_of_the_same_place_and_then_the_first_header_at_the_same_distance", () => {
    // Gate 16 lies between Zweiblattdorf and Route 201; gate 17 between Route 201 and the Kampfzone, of neither place.
    const headers = [...ptHeaders, header(16, { name: "R201R0101", place: "Route 201", index: 3, matrix: 26 }), header(17, { name: "R299R0101", place: "Tor", index: 13, matrix: 27 })];
    const matrices = [...ptMatrices, summary(26, 1, 1, false), summary(27, 1, 1, false)];
    const links = [...ptLinks, warp(16, 1), warp(16, 2, 1), warp(17, 9), warp(17, 3, 1)];
    const world = buildWorld(ptInput({ headers, matrices, links }));
    expect(areaKeyOf(world, 16)).toBe("w0:3");
    expect(areaKeyOf(world, 17)).toBe("w0:3");
  });

  it("falls_back_to_the_area_of_the_same_place", () => {
    const world = buildWorld(ptInput());
    expect(areaKeyOf(world, 7)).toBe("w0:2");
    expect(roomIds(area(world, "w0:2"))).toEqual([4, 7]);
  });

  it("falls_back_to_the_area_of_the_rooms_of_the_same_place", () => {
    // Level 3 of the Bergwerk has no warp (the game moves the player by script): it joins levels 1 and 2 at Route 201.
    const headers = [...ptHeaders, header(16, { name: "D01R0103", place: "Bergwerk", index: 9, matrix: 26, event: 0, type: 3 })];
    const matrices = [...ptMatrices, summary(26, 1, 1, false)];
    const world = buildWorld(ptInput({ headers, matrices }));
    expect(areaKeyOf(world, 16)).toBe("w0:3");
    expect(roomIds(area(world, "w0:3"))).toEqual([5, 6, 16]);
  });

  it("leaves_a_room_without_entrance_and_place_unconnected", () => {
    const world = buildWorld(ptInput());
    expect(areaKeyOf(world, 8)).toBeUndefined();
    const villa = world.unconnected.find((r) => r.header.id === 8);
    expect(villa).toMatchObject({ name: "Villa", placeKey: "Villa", placeName: "Villa", manual: false, shared: 0 });
  });

  it("ignores_the_exits_of_a_shared_event_file", () => {
    const world = buildWorld(ptInput());
    expect(world.unconnected.map((r) => r.header.id)).toEqual([8, 11, 12]);
    const direct = world.unconnected.filter((r) => r.header.id !== 8);
    expect(direct.map((r) => [r.placeKey, r.name, r.shared])).toEqual([
      [UNNAMED_KEY, "Raum 1", 1],
      [UNNAMED_KEY, "Raum 2", 1],
    ]);
  });

  it("keeps_orphans_and_the_landscape_header_in_other", () => {
    const world = buildWorld(ptInput());
    expect(world.other.map((h) => h.id)).toEqual([0, 13, 14, 15]);
  });

  it("splits_one_hgss_matrix_into_johto_and_kanto", () => {
    const world = buildWorld(hgssInput());
    expect(world.worldMaps.map((m) => [m.key, m.filter, m.name])).toEqual([
      ["w0", "johto", "Johto"],
      ["w0k", "kanto", "Kanto"],
    ]);
    expect(world.worldMaps.map((m) => m.areas.map((a) => [a.key, a.headers.map((h) => h.id)]))).toEqual([
      [
        ["w0:126", [1]],
        ["w0:127", [2]],
      ],
      [
        ["w0k:200", [3]],
        ["w0k:127", [4]],
      ],
    ]);
    expect(areaKeyOf(world, 5)).toBe("w0k:127");
  });

  it("names_world_maps_sinnoh_johto_kanto_and_takes_the_names_of_the_project_file", () => {
    expect(buildWorld(ptInput({ family: "Dp" })).worldMaps.map((m) => m.name)).toEqual(["Sinnoh"]);
    const meta = { ...emptyMeta(), worldMaps: [{ matrix: 0, name: "Hoenn" }] };
    expect(buildWorld(ptInput({ meta })).worldMaps.map((m) => m.name)).toEqual(["Hoenn"]);

    // A second matrix with two places is a world map named after its most frequent place.
    const headers = [...ptHeaders, header(16, { name: "D06R0101", place: "Moorhaus", index: 71, matrix: 7, type: 2 })];
    const cells = ptCells();
    cells.set(7, grid(7, 2, 2, [10, 10, 16, 0]));
    const world = buildWorld(ptInput({ headers, cells }));
    expect(world.worldMaps.map((m) => [m.key, m.name])).toEqual([
      ["w0", "Sinnoh"],
      ["w7", "Großmoor"],
    ]);
    expect(world.worldMaps[1].areas.map((a) => a.key)).toEqual(["w7:70", "w7:71"]);
  });

  it("drops_a_world_map_the_project_file_deselects", () => {
    const world = buildWorld(ptInput({ meta: { ...emptyMeta(), notWorldMaps: [0] } }));
    expect(world.worldMaps).toEqual([]);
    // Its places become one area with its own map, and the rooms follow their entrance there.
    const m0 = area(world, "m0");
    expect(m0.headers.map((h) => h.id)).toEqual([1, 2, 3]);
    expect(areaKeyOf(world, 4)).toBe("m0");
    expect(areaKeyOf(world, 6)).toBe("m0");
  });

  it("takes_the_owner_and_the_name_of_a_room_from_the_project_file", () => {
    const meta = { ...emptyMeta(), roomOwner: { "8": "w0:2", "5": "no-such-area" }, roomNames: { "4": "Omas Haus" } };
    const world = buildWorld(ptInput({ meta }));
    expect(areaKeyOf(world, 8)).toBe("w0:2");
    const town = area(world, "w0:2");
    expect(town.rooms.find((r) => r.header.id === 8)).toMatchObject({ manual: true, name: "Villa" });
    expect(town.rooms.find((r) => r.header.id === 4)).toMatchObject({ manual: false, name: "Omas Haus", suggested: "Haus 1" });
    // An owner that names no area is ignored.
    expect(areaKeyOf(world, 5)).toBe("w0:3");
    expect(area(world, "w0:3").rooms.find((r) => r.header.id === 5)?.manual).toBe(false);
  });

  it("prefers_the_area_of_the_rooms_half_for_a_room_without_entrance", () => {
    // Room 6 has no warp; Route 29 is an area in Johto (first header 2) and in Kanto (4): the Kanto room goes to Kanto.
    const world = buildWorld(hgssInput());
    expect(areaKeyOf(world, 6)).toBe("w0k:127");
  });

  it("prefers_the_half_of_the_room_in_the_vote_of_its_place", () => {
    // Bergwerk (index 9) has no area: a Johto level reached from Johto and a Kanto level reached from Kanto; the
    // Kanto level without warp (13) joins the Kanto one, though the Johto area has the lower first header id.
    const headers = [
      ...hgssHeaders,
      header(11, { name: "D01R0101", place: "Bergwerk", index: 9, matrix: 10 }),
      header(12, { name: "D01R0102", place: "Bergwerk", index: 9, matrix: 10, kanto: true }),
      header(13, { name: "D01R0103", place: "Bergwerk", index: 9, matrix: 10, kanto: true }),
    ];
    const world = buildWorld(hgssInput({ headers, links: [warp(5, 4), warp(11, 1), warp(12, 3)] }));
    expect(areaKeyOf(world, 11)).toBe("w0:126");
    expect(areaKeyOf(world, 12)).toBe("w0k:200");
    expect(areaKeyOf(world, 13)).toBe("w0k:200");
  });

  it("gives_one_world_map_for_an_hgss_matrix_with_only_johto_headers", () => {
    const headers = hgssHeaders.map((h) => ({ ...h, kanto: false }));
    const world = buildWorld(hgssInput({ headers }));
    expect(world.worldMaps.map((m) => [m.key, m.filter, m.name])).toEqual([["w0", null, "Johto"]]);
    expect(world.worldMaps[0].areas.map((a) => [a.key, a.headers.map((h) => h.id)])).toEqual([
      ["w0:126", [1]],
      ["w0:127", [2, 4]],
      ["w0:200", [3]],
    ]);
  });

  it("orders_areas_and_their_headers_by_id_whatever_the_cell_order", () => {
    const cells = ptCells();
    cells.set(0, grid(0, 3, 3, [3, 2, 0, 1, 1, 0, 0, 0, 0], [1, 2, 3, 4, 5, 6, 7, 8, null]));
    const world = buildWorld(ptInput({ cells }));
    expect(world.worldMaps[0].areas.map((a) => [a.key, a.headers.map((h) => h.id)])).toEqual([
      ["w0:2", [1]],
      ["w0:3", [2, 3]],
    ]);
  });

  it("takes_the_nearest_area_before_a_farther_one_of_the_same_place", () => {
    // Room 16 (Route 201) is one warp from Zweiblattdorf and two (over room 17) from Route 201.
    const headers = [...ptHeaders, header(16, { name: "R201X", place: "Route 201", index: 3, matrix: 26 }), header(17, { name: "R201Y", place: "Tor", index: 13, matrix: 27 })];
    const matrices = [...ptMatrices, summary(26, 1, 1, false), summary(27, 1, 1, false)];
    const links = [...ptLinks, warp(16, 1), warp(16, 17, 1), warp(17, 2)];
    const world = buildWorld(ptInput({ headers, matrices, links }));
    expect(areaKeyOf(world, 16)).toBe("w0:2");
  });

  it("puts_a_header_into_one_area_only", () => {
    // Header 1 also has a cell on matrix 7: it stays in the area of matrix 0, and matrix 7 keeps one place.
    const cells = ptCells();
    cells.set(7, grid(7, 2, 2, [10, 1, 0, 10]));
    const world = buildWorld(ptInput({ cells }));
    expect(world.worldMaps.map((m) => m.key)).toEqual(["w0"]);
    expect(allAreas(world).filter((a) => a.headers.some((h) => h.id === 1)).map((a) => a.key)).toEqual(["w0:2"]);
    expect(area(world, "m7").headers.map((h) => h.id)).toEqual([10]);
  });

  it("takes_an_owner_given_as_the_id_of_an_area_header", () => {
    const world = buildWorld(ptInput({ meta: { ...emptyMeta(), roomOwner: { "8": "3" } } }));
    expect(areaKeyOf(world, 8)).toBe("w0:3");
    expect(area(world, "w0:3").rooms.find((r) => r.header.id === 8)?.manual).toBe(true);
  });

  it("leaves_its_input_alone", () => {
    const input = ptInput({ meta: { ...emptyMeta(), roomOwner: { "8": "w0:2" }, roomNames: { "4": "Omas Haus" } } });
    const before = structuredClone({ ...input, cells: [...input.cells] });
    buildWorld(input);
    expect({ ...input, cells: [...input.cells] }).toEqual(before);
  });

  it("counts_the_rooms_and_the_assigned_ones", () => {
    const world = buildWorld(ptInput());
    expect(world.roomCount).toBe(7);
    expect(world.assignedRooms).toBe(4);
  });

  it("lists_every_header_exactly_once", () => {
    const world = buildWorld(ptInput());
    const ids = [
      ...allAreas(world).flatMap((a) => [...a.headers.map((h) => h.id), ...roomIds(a)]),
      ...world.unconnected.map((r) => r.header.id),
      ...world.other.map((h) => h.id),
    ].sort((a, b) => a - b);
    expect(ids).toEqual(ptHeaders.map((h) => h.id));
  });
});

describe("worldNames", () => {
  it("names_every_header_as_the_world_tree_does", () => {
    const names = worldNames(buildWorld(ptInput({ meta: { ...emptyMeta(), roomNames: { "4": "Omas Haus" } } })));
    expect(names.size).toBe(ptHeaders.length);
    // An area by its name, also one with its own map that has no header section (the places would call it a room).
    expect(names.get(1)).toEqual({ kind: "area", name: "Zweiblattdorf", placeName: "Zweiblattdorf" });
    expect(names.get(9)).toEqual({ kind: "area", name: "Kampfzone", placeName: "Kampfzone" });
    // A room by the name of the project file, else the one its code suggests.
    expect(names.get(4)).toEqual({ kind: "room", name: "Omas Haus", placeName: "Zweiblattdorf" });
    expect(names.get(7)).toEqual({ kind: "room", name: "Haus 2", placeName: "Zweiblattdorf" });
    // The others by their place, else their code.
    expect(names.get(14)).toEqual({ kind: "other", name: "Zweiblattdorf", placeName: "Zweiblattdorf" });
    expect(names.get(0)).toEqual({ kind: "other", name: "EVERYWHERE", placeName: unnamedPlace() });
  });
});
