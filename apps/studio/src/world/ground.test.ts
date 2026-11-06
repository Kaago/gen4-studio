import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../api";
import { createGround, flatGround, heightWords } from "./ground";
import { MAP_SIZE } from "./layout";
import type { Loaded } from "./scene/types";

/** A 2 x 1 matrix: cell 0 on the ground, cell 1 at altitude 2 (one tile up). */
const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2], headers: null, altitudes: [0, 2] };

/** A map whose tile heights (in world units) are `at(x, y)`. */
const mapWith = (index: number, at: (x: number, y: number) => number | null): Loaded => ({
  map: {
    index,
    permissions: [],
    buildings: [],
    heights: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => at(i % MAP_SIZE, Math.floor(i / MAP_SIZE))),
  },
});

/** The left map is flat at 0 with 32 E at tile (3, 4); the right map has no heights at all. */
const loaded = () =>
  new Map<number, Loaded>([
    [1, mapWith(1, (x, y) => (x === 3 && y === 4 ? 32 : 0))],
    [2, { map: { index: 2, permissions: [], buildings: [], heights: [] } }],
  ]);

describe("createGround", () => {
  it("ground_adds_the_bdhc_height_to_the_altitude", () => {
    const ground = createGround(matrix, loaded());
    expect(ground.y(3, 4)).toBe(2);
    expect(ground.units(3, 4)).toBe(32);
    expect(ground.y(4, 4)).toBe(0);
    // The right cell is one tile up and its map has no heights.
    expect(ground.y(35, 4)).toBe(1);
    expect(ground.units(35, 4)).toBeNull();
  });

  it("null_heights_fall_back_to_the_altitude", () => {
    // A BDHC without a plate under some tiles (or an unreadable one: all null) gives null there.
    const holes = new Map<number, Loaded>([
      [1, mapWith(1, (x) => (x < 16 ? null : 16))],
      [2, mapWith(2, () => null)],
    ]);
    const ground = createGround(matrix, holes);
    expect(ground.units(3, 4)).toBeNull();
    expect(ground.y(3, 4)).toBe(0);
    expect(ground.y(20, 4)).toBe(1);
    expect(ground.units(40, 4)).toBeNull();
    expect(ground.y(40, 4)).toBe(1);
    // A map that is not loaded and tiles beside the matrix have no height either.
    const missing = createGround(matrix, new Map());
    expect(missing.units(40, 4)).toBeNull();
    expect(missing.y(40, 4)).toBe(1);
    expect(missing.units(-1, 4)).toBeNull();
    expect(missing.units(64, 4)).toBeNull();
    expect(missing.y(64, 4)).toBe(0);
  });

  it("levels_are_distinct_and_highest_first", () => {
    expect(createGround(matrix, loaded()).levels()).toEqual([2, 1, 0]);
    // An empty cell lies on its own altitude.
    expect(createGround({ ...matrix, maps: [1, null] }, loaded()).levels()).toEqual([2, 1, 0]);
    expect(createGround({ ...matrix, altitudes: null }, new Map()).levels()).toEqual([0]);
  });
});

describe("flatGround", () => {
  it("is the altitude of the matrix alone", () => {
    const ground = flatGround(matrix);
    expect(ground.y(3, 4)).toBe(0);
    expect(ground.y(35, 4)).toBe(1);
    expect(ground.units(3, 4)).toBeNull();
    expect(ground.levels()).toEqual([1, 0]);
    expect(flatGround(null).y(3, 4)).toBe(0);
    expect(flatGround(null).levels()).toEqual([0]);
  });
});

describe("heightWords", () => {
  it("height_words_in_bauen_and_tuefteln", () => {
    expect(heightWords(32, "bauen")).toBe("Höhe 4 Stufen");
    expect(heightWords(8, "bauen")).toBe("Höhe 1 Stufe");
    expect(heightWords(3.99, "bauen")).toBe("Höhe 0,5 Stufen");
    expect(heightWords(32, "erzaehlen")).toBe("Höhe 4 Stufen");
    expect(heightWords(32, "tuefteln")).toBe("Höhe 32 E (4 Stufen)");
    expect(heightWords(3.99, "tuefteln")).toBe("Höhe 3,99 E (0,5 Stufen)");
    expect(heightWords(8, "tuefteln")).toBe("Höhe 8 E (1 Stufe)");
    expect(heightWords(null, "bauen")).toBe("keine Höhe");
    expect(heightWords(null, "tuefteln")).toBe("keine Höhe");
  });
});
