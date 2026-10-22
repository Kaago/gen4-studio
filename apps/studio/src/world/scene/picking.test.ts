import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { OVERLAY_Y } from "./content";
import { slantedView } from "./dragTestKit.testkit";

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
