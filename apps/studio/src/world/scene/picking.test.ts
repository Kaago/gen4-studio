import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { flatGround } from "../ground";
import { MAP_SIZE } from "../layout";
import { OVERLAY_Y } from "./content";
import { slantedView, viewThrough } from "./dragTestKit.testkit";

/** A 2 x 1 matrix: cell 0 on the ground, cell 1 at altitude 2 (one tile up). */
const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2], headers: null, altitudes: [0, 2] };
const host = (m: MatrixDto | null = matrix) => slantedView(m);

describe("pickTile with altitudes", () => {
  it("hits the raised surface of a cell above the ground plane", () => {
    const { picker, through } = host();
    const at = through(37.5, OVERLAY_Y + 1, 5.5);
    const hit = picker.pickTile(at.x, at.y);
    expect(hit).toMatchObject({ mapId: 2, col: 1, row: 0, x: 5, y: 5 });
  });

  it("falls back to the ground plane where the raised plane is hit outside its cell", () => {
    const { picker, through } = host();
    // This ray crosses the plane of altitude 2 in cell 0 (which is on the ground), so only the ground plane counts.
    const at = through(30.5, OVERLAY_Y, 5.5);
    const hit = picker.pickTile(at.x, at.y);
    expect(hit).toMatchObject({ mapId: 1, col: 0, row: 0, x: 30, y: 5 });
  });

  it("still hits the ground plane of a matrix without altitudes", () => {
    const { picker, through } = host({ ...matrix, altitudes: null });
    const at = through(37.5, OVERLAY_Y, 5.5);
    expect(picker.pickTile(at.x, at.y)).toMatchObject({ mapId: 2, col: 1, x: 5, y: 5 });
  });

  it("returns null over nothing, and without a matrix", () => {
    const { picker, through } = host();
    const off = through(200, OVERLAY_Y, 5.5);
    expect(picker.pickTile(off.x, off.y)).toBeNull();
    expect(host(null).picker.pickTile(10, 10)).toBeNull();
  });
});

describe("surfacePoint", () => {
  const tall: MatrixDto = { ...matrix, altitudes: [0, 10] };

  it("is the point of the surface under the pointer, whatever the height of the cell", () => {
    const { picker, through } = host(tall);
    for (const [x, z, y] of [
      [36.5, 5.5, OVERLAY_Y + 5],
      [26.5, 5.5, OVERLAY_Y],
    ]) {
      const at = through(x, y, z);
      const p = picker.surfacePoint(at.x, at.y);
      expect(p?.x).toBeCloseTo(x, 3);
      expect(p?.z).toBeCloseTo(z, 3);
    }
  });

  it("includes cells without a map, and is null over no cell", () => {
    const empty = { ...tall, maps: [1, null] };
    const { picker, through } = host(empty);
    const over = through(36.5, OVERLAY_Y + 5, 5.5);
    expect(picker.surfacePoint(over.x, over.y)?.x).toBeCloseTo(36.5, 3);
    expect(picker.pickTile(over.x, over.y)).toBeNull();
    const off = through(200, OVERLAY_Y, 5.5);
    expect(picker.surfacePoint(off.x, off.y)).toBeNull();
    expect(host(null).picker.surfacePoint(10, 10)).toBeNull();
  });
});

describe("pickTile on the BDHC height", () => {
  /** One cell, one map: tiles get `at(x, y)` world units. */
  const one: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1], headers: null, altitudes: null };
  const heights = (at: (x: number, y: number) => number) => ({ 1: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => at(i % MAP_SIZE, Math.floor(i / MAP_SIZE))) });
  /** A camera 45° above the centre of tile (20, 10) at height 2, from the south. */
  const slanted = () => {
    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
    camera.position.set(20.5, 2 + 20, 10.5 + 20);
    camera.lookAt(20.5, 2, 10.5);
    return camera;
  };
  const topDown = () => {
    const camera = new THREE.OrthographicCamera(-16, 16, 16, -16, 0.5, 4000);
    camera.up.set(0, 0, -1);
    camera.position.set(16, 2000, 16);
    camera.lookAt(16, 0, 16);
    return camera;
  };

  it("picks_the_tile_under_the_pointer_on_a_slope", () => {
    // Columns from 16 on lie 32 E (two tiles) up.
    const slope = heights((x) => (x >= 16 ? 32 : 0));
    const { h, picker, through } = viewThrough(slanted(), one, slope);
    const at = through(20.5, 2, 10.5);
    expect(picker.pickTile(at.x, at.y)).toMatchObject({ mapId: 1, x: 20, y: 10 });
    // The matrix height alone puts the plate two tiles lower: the same pointer finds another tile.
    h.ground = flatGround(one);
    const flat = picker.pickTile(at.x, at.y);
    expect(flat).not.toBeNull();
    expect([flat?.x, flat?.y]).not.toEqual([20, 10]);
  });

  it("bridges_show_the_upper_deck", () => {
    // A bridge deck along row 10 (the kernel gives the tile centre its highest candidate, 32 E); the road below it lies at 0.
    const bridge = heights((_, y) => (y === 10 ? 32 : 0));
    const { h, picker, through } = viewThrough(slanted(), one, bridge);
    expect(h.ground.y(20, 10)).toBe(2);
    const at = through(20.5, OVERLAY_Y + 2, 10.5);
    // The ray also crosses the ground plane over a walkable tile of height 0: the deck, highest first, wins.
    h.ground = flatGround(one);
    expect(picker.pickTile(at.x, at.y)).toMatchObject({ x: 20, y: 8 });
    const deck = viewThrough(slanted(), one, bridge);
    expect(deck.picker.pickTile(at.x, at.y)).toMatchObject({ mapId: 1, x: 20, y: 10 });
  });

  it("top_down_picks_as_before", () => {
    const slope = heights((x) => (x >= 16 ? 32 : 0));
    for (const [x, z] of [
      [20, 10],
      [3, 27],
      [16, 16],
    ]) {
      const view = viewThrough(topDown(), one, slope);
      const at = view.through(x + 0.5, 0, z + 0.5);
      expect(view.picker.pickTile(at.x, at.y)).toMatchObject({ x, y: z });
      view.h.ground = flatGround(one);
      expect(view.picker.pickTile(at.x, at.y)).toMatchObject({ x, y: z });
    }
  });
});
