// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import { type Area, type Room, type World } from "./worldModel";
import { compareNames, readRecent, sortHeaders, sortWorld, touchRecent } from "./worldSort";

const header = (id: number, name: string, place = ""): HeaderSummary => ({
  id,
  internal_name: name,
  location_name: place,
  location_index: place ? id : 0,
  kanto: false,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id: 0,
  script_file_id: 0,
  level_script_id: 0,
});

const room = (id: number, name: string): Room => ({ header: header(id, `R${id}`), name, suggested: name, placeKey: "p", placeName: "p", manual: false, shared: 0 });

const area = (key: string, name: string, firstId: number, rooms: Room[] = []): Area => ({
  key,
  name,
  placeKey: name,
  headers: [header(firstId, `A${firstId}`, name)],
  rooms,
  matrix: 0,
});

const world = (patch: Partial<World>): World => ({
  worldMaps: [],
  moreAreas: [],
  unconnected: [],
  other: [],
  areaOf: new Map(),
  roomCount: 0,
  assignedRooms: 0,
  ...patch,
});

const names = (areas: Area[]) => areas.map((a) => a.name);

describe("sortWorld", () => {
  it("sorts_route_2_before_route_10", () => {
    const w = world({
      worldMaps: [{ key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area("a", "Route 10", 1), area("b", "Route 2", 2)] }],
      moreAreas: [area("c", "Route 10", 3), area("d", "Route 2", 4)],
    });
    const sorted = sortWorld(w, "name", []);
    expect(names(sorted.worldMaps[0].areas)).toEqual(["Route 2", "Route 10"]);
    expect(names(sorted.moreAreas)).toEqual(["Route 2", "Route 10"]);
  });

  it("sorts_umlauts_like_their_base_letters", () => {
    const w = world({ moreAreas: [area("a", "Bach", 1), area("b", "Äther", 2), area("c", "Achtung", 3)] });
    expect(names(sortWorld(w, "name", []).moreAreas)).toEqual(["Achtung", "Äther", "Bach"]);
    expect(compareNames("Äther", "Bach")).toBeLessThan(0);
    expect(compareNames("Route 2", "Route 10")).toBeLessThan(0);
  });

  it("keeps_the_game_order_by_first_header_id", () => {
    const w = world({
      worldMaps: [
        { key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area("a", "Zeta", 1), area("b", "Alpha", 2)] },
        { key: "w1", matrix: 1, filter: null, name: "Anders", areas: [area("c", "Beta", 3)] },
      ],
      moreAreas: [area("d", "Zulu", 4), area("e", "Alt", 5)],
    });
    for (const sort of ["game", "recent"] as const) {
      const sorted = sortWorld(w, sort, []);
      expect(names(sorted.worldMaps[0].areas)).toEqual(["Zeta", "Alpha"]);
      expect(names(sorted.moreAreas)).toEqual(["Zulu", "Alt"]);
    }
    // The world maps keep the order of the matrices in every sort.
    expect(sortWorld(w, "name", []).worldMaps.map((m) => m.name)).toEqual(["Sinnoh", "Anders"]);
  });

  it("puts_recent_areas_first_then_the_rest_in_game_order", () => {
    const w = world({
      worldMaps: [{ key: "w0", matrix: 0, filter: null, name: "Sinnoh", areas: [area("a", "A", 1), area("b", "B", 2), area("c", "C", 3), area("d", "D", 4)] }],
      moreAreas: [area("e", "E", 5), area("f", "F", 6)],
    });
    const sorted = sortWorld(w, "recent", ["d", "gone", "b", "f"]);
    expect(names(sorted.worldMaps[0].areas)).toEqual(["D", "B", "A", "C"]);
    expect(names(sorted.moreAreas)).toEqual(["F", "E"]);
  });

  it("sorts_rooms_by_name_only_in_the_name_sort", () => {
    const rooms = [room(1, "Haus 10"), room(2, "Arena"), room(3, "Haus 2")];
    const w = world({ moreAreas: [area("a", "A", 1, rooms)], unconnected: rooms });
    const ids = (a: Area) => a.rooms.map((r) => r.header.id);
    expect(ids(sortWorld(w, "game", []).moreAreas[0])).toEqual([1, 2, 3]);
    expect(ids(sortWorld(w, "recent", []).moreAreas[0])).toEqual([1, 2, 3]);
    expect(ids(sortWorld(w, "name", []).moreAreas[0])).toEqual([2, 3, 1]);
  });

  it("does_not_change_its_input", () => {
    const w = world({ moreAreas: [area("a", "B", 1, [room(1, "Z"), room(2, "A")]), area("b", "A", 2)] });
    const before = JSON.stringify(w);
    const sorted = sortWorld(w, "name", ["b"]);
    expect(JSON.stringify(w)).toBe(before);
    expect(sorted.moreAreas).not.toBe(w.moreAreas);
    expect(sorted.areaOf).toBe(w.areaOf);
  });
});

describe("sortHeaders", () => {
  const list = [header(3, "T02", "Jubelstadt"), header(1, "T10", "Eich"), header(2, "T3", ""), header(4, "R1", "Eich")];

  it("sorts_the_list_by_number_name_and_code", () => {
    const ids = (sort: "id" | "name" | "code") => sortHeaders([...list].reverse(), sort).map((h) => h.id);
    expect(ids("id")).toEqual([1, 2, 3, 4]);
    // The name is the one the list shows: the place, else the internal name; equal names fall back to the number.
    expect(ids("name")).toEqual([1, 4, 3, 2]);
    expect(ids("code")).toEqual([4, 3, 2, 1]);
  });

  it("does_not_change_its_input", () => {
    const input = [...list];
    sortHeaders(input, "name");
    expect(input).toEqual(list);
  });
});

describe("recent areas", () => {
  beforeEach(() => localStorage.clear());

  it("remembers_at_most_50_recent_areas_per_project", () => {
    expect(readRecent("/p/a")).toEqual([]);
    for (let i = 0; i < 60; i++) touchRecent("/p/a", `k${i}`);
    const recent = readRecent("/p/a");
    expect(recent).toHaveLength(50);
    expect(recent[0]).toBe("k59");
    expect(recent[49]).toBe("k10");
    expect(readRecent("/p/b")).toEqual([]);
    expect(localStorage.getItem("g4.world.recent./p/a")).not.toBeNull();
  });

  it("moves_a_touched_area_to_the_front_without_a_duplicate", () => {
    touchRecent("/p/a", "x");
    touchRecent("/p/a", "y");
    expect(touchRecent("/p/a", "x")).toEqual(["x", "y"]);
    expect(readRecent("/p/a")).toEqual(["x", "y"]);
  });

  it("reads_a_broken_value_as_empty_and_survives_a_failing_storage", () => {
    localStorage.setItem("g4.world.recent./p/a", "{nope");
    expect(readRecent("/p/a")).toEqual([]);
    localStorage.setItem("g4.world.recent./p/a", JSON.stringify(["a", 3, "b"]));
    expect(readRecent("/p/a")).toEqual(["a", "b"]);
    const get = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    const set = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(readRecent("/p/a")).toEqual([]);
    expect(touchRecent("/p/a", "z")).toEqual(["z"]);
    get.mockRestore();
    set.mockRestore();
  });
});
