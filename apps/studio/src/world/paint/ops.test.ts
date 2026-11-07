import { describe, expect, it } from "vitest";
import type { WorldJson } from "../../api.world";
import {
  type Cell,
  type PaintOptions,
  type WorldCells,
  cellsOf,
  fillCells,
  flipTile,
  lineCells,
  pickedTile,
  rectCells,
  strokeRequest,
  turnRamp,
  turnTile,
} from "./ops";

const row = (token: string) => Array(32).fill(token).join(" ");
const rows = (token: string) => Array.from({ length: 32 }, () => row(token));

/** A world of `0000` at height 0 with one overlay of `.`; `edit` changes the cells before the test reads them. */
function world(edit?: (c: WorldCells) => void): WorldCells {
  const w: WorldJson = {
    version: 1,
    tileset: 6,
    layers: [
      { name: "Boden", role: "ground", tiles: rows("0000"), heights: rows("0"), ramps: rows(".") },
      { name: "Büsche", role: "overlay", tiles: rows(".") },
    ],
    permissions: { derive: true },
  };
  const c = cellsOf(w);
  edit?.(c);
  return c;
}

const options = (o: Partial<PaintOptions> = {}): PaintOptions => ({
  layer: 0,
  tile: "0041",
  ramp: { dir: "e", steps: 1 },
  lower: false,
  ...o,
});

const key = ([x, z]: Cell) => `${x},${z}`;

describe("paint ops", () => {
  it("line_cells_cover_a_diagonal_without_gaps", () => {
    const cells = lineCells([0, 0], [5, 2]);
    expect(cells).toHaveLength(6);
    expect(cells[0]).toEqual([0, 0]);
    expect(cells[5]).toEqual([5, 2]);
    expect(new Set(cells.map(key)).size).toBe(6);
    for (let i = 1; i < cells.length; i++) {
      const [ax, az] = cells[i - 1];
      const [bx, bz] = cells[i];
      expect(Math.max(Math.abs(ax - bx), Math.abs(az - bz))).toBe(1);
    }
    // Backwards and steep lines too.
    expect(lineCells([5, 2], [0, 0])).toHaveLength(6);
    expect(lineCells([3, 3], [3, 3])).toEqual([[3, 3]]);
    expect(lineCells([1, 0], [2, 6])).toHaveLength(7);
  });

  it("rect_cells_in_either_corner_order", () => {
    const expected: Cell[] = [
      [1, 2],
      [2, 2],
      [3, 2],
      [1, 3],
      [2, 3],
      [3, 3],
    ];
    expect(rectCells([1, 2], [3, 3])).toEqual(expected);
    expect(rectCells([3, 3], [1, 2])).toEqual(expected);
    expect(rectCells([3, 2], [1, 3])).toEqual(expected);
  });

  it("fill_stops_at_another_tile_and_at_the_edge", () => {
    // An island of `0001` (2×2 at 4..5, 4..5 plus a tail at 6,5) in `0000`.
    const c = world((c) => {
      for (const [x, z] of [
        [4, 4],
        [5, 4],
        [4, 5],
        [5, 5],
        [6, 5],
      ])
        c.tiles[0][z][x] = "0001";
      c.tiles[0][6][7] = "0001"; // (7, 6) touches (6, 5) diagonally only: not part of the island
    });
    const island = fillCells(c, 0, [5, 5]);
    expect(island.map(key).sort()).toEqual(["4,4", "4,5", "5,4", "5,5", "6,5"]);
    // The sea runs to the edge and around the island, nothing else.
    const sea = fillCells(c, 0, [0, 0]);
    expect(sea).toHaveLength(1024 - 6);
    expect(sea.every(([x, z]) => x >= 0 && x < 32 && z >= 0 && z < 32)).toBe(true);
  });

  it("fill_counts_turned_tiles_as_other_tiles", () => {
    const c = world((c) => {
      c.tiles[0][0][1] = "0000r1";
      c.tiles[0][1][0] = "0000f";
    });
    // The corner cell is cut off by a turned and a mirrored copy of the same tile.
    expect(fillCells(c, 0, [0, 0])).toEqual([[0, 0]]);
  });

  it("picked_tile_falls_back_to_the_top_layer", () => {
    const c = world((c) => {
      c.tiles[1][2][3] = "0050";
    });
    expect(pickedTile(c, 1, [3, 2])).toBe("0050");
    expect(pickedTile(c, 0, [3, 2])).toBe("0000");
    // Empty on the active overlay: the topmost non-empty layer.
    expect(pickedTile(c, 1, [0, 0])).toBe("0000");
    const empty = world((c) => {
      c.tiles[0][0][0] = ".";
    });
    expect(pickedTile(empty, 1, [0, 0])).toBeNull();
  });

  it("turn_and_flip_tokens", () => {
    expect(turnTile("0041")).toBe("0041r1");
    expect(turnTile("0041r1")).toBe("0041r2");
    expect(turnTile("0041r2")).toBe("0041r3");
    expect(turnTile("0041r3")).toBe("0041");
    expect(turnTile("0041r3f")).toBe("0041f");
    expect(turnTile("0041f")).toBe("0041r1f");
    expect(turnTile("00AFR1")).toBe("00afr2");
    expect(turnTile(".")).toBe(".");
    expect(flipTile("0041")).toBe("0041f");
    expect(flipTile("0041r2f")).toBe("0041r2");
    expect(flipTile("0041r2")).toBe("0041r2f");
    expect(flipTile(".")).toBe(".");
    expect(turnRamp("n")).toBe("e");
    expect(turnRamp("e")).toBe("s");
    expect(turnRamp("s")).toBe("w");
    expect(turnRamp("w")).toBe("n");
  });

  it("brush_request_is_one_change_with_its_label", () => {
    const out = strokeRequest(
      0x7b,
      "brush",
      [
        [1, 1],
        [2, 1],
        [2, 1],
        [3, 2],
      ],
      world(),
      options(),
    );
    expect(out).toEqual({
      req: {
        map: 0x7b,
        label: "Gemalt: Karte 0123 (3 Zellen)",
        changes: [
          {
            layer: 0,
            cells: [
              [1, 1],
              [2, 1],
              [3, 2],
            ],
            tile: "0041",
          },
        ],
      },
      skipped: 0,
    });
  });

  it("unchanged_cells_send_nothing", () => {
    expect(strokeRequest(123, "brush", [[1, 1]], world(), options({ tile: "0000" }))).toBeNull();
    // Only the changed cell goes, in any case of the stored token.
    const c = world((c) => {
      c.tiles[0][0][0] = "0041";
    });
    const out = strokeRequest(123, "rect", rectCells([0, 0], [1, 0]), c, options());
    expect(out?.req.changes).toEqual([{ layer: 0, cells: [[1, 0]], tile: "0041" }]);
    expect(out?.req.label).toBe("Gemalt: Karte 0123 (1 Zelle)");
  });

  it("height_groups_cells_by_target", () => {
    const c = world((c) => {
      c.heights[0][1] = 2;
      c.heights[0][3] = 2;
    });
    const out = strokeRequest(
      123,
      "height",
      [
        [0, 0],
        [1, 0],
        [2, 0],
        [3, 0],
      ],
      c,
      options(),
    );
    expect(out?.req.changes).toEqual([
      {
        cells: [
          [0, 0],
          [2, 0],
        ],
        height: 1,
      },
      {
        cells: [
          [1, 0],
          [3, 0],
        ],
        height: 3,
      },
    ]);
    expect(out?.req.label).toBe("Höhe geändert: Karte 0123 (4 Zellen)");
    const lower = strokeRequest(123, "height", [[1, 0]], c, options({ lower: true }));
    expect(lower?.req.changes).toEqual([{ cells: [[1, 0]], height: 1 }]);
  });

  it("height_skips_cells_at_the_range", () => {
    const c = world((c) => {
      c.heights[0][0] = 62;
      c.heights[0][1] = 10;
      c.heights[0][2] = -16;
    });
    const up = strokeRequest(
      123,
      "height",
      [
        [0, 0],
        [1, 0],
      ],
      c,
      options(),
    );
    expect(up).toEqual({
      req: { map: 123, label: "Höhe geändert: Karte 0123 (1 Zelle)", changes: [{ cells: [[1, 0]], height: 11 }] },
      skipped: 1,
    });
    const down = strokeRequest(
      123,
      "height",
      [
        [2, 0],
        [1, 0],
      ],
      c,
      options({ lower: true }),
    );
    expect(down?.skipped).toBe(1);
    expect(down?.req.changes).toEqual([{ cells: [[1, 0]], height: 9 }]);
    // Nothing left to send: nothing to send.
    expect(strokeRequest(123, "height", [[0, 0]], c, options())).toBeNull();
  });

  it("height_and_ramp_write_the_ground", () => {
    const h = strokeRequest(123, "height", [[0, 0]], world(), options({ layer: 1 }));
    expect(h?.req.changes).toEqual([{ cells: [[0, 0]], height: 1 }]);
    expect(h?.req.changes[0]).not.toHaveProperty("layer");
    const r = strokeRequest(123, "ramp", [[0, 0]], world(), options({ layer: 1 }));
    expect(r?.req.changes).toEqual([{ cells: [[0, 0]], ramp: "e1" }]);
    expect(r?.req.changes[0]).not.toHaveProperty("layer");
  });

  it("ramp_token_and_removal", () => {
    const set = strokeRequest(
      123,
      "ramp",
      [
        [4, 4],
        [5, 4],
      ],
      world(),
      options({ ramp: { dir: "n", steps: 2 } }),
    );
    expect(set?.req).toEqual({
      map: 123,
      label: "Rampe geändert: Karte 0123 (2 Zellen)",
      changes: [
        {
          cells: [
            [4, 4],
            [5, 4],
          ],
          ramp: "n2",
        },
      ],
    });
    const c = world((c) => {
      c.ramps[4][4] = "e1";
    });
    const removed = strokeRequest(
      123,
      "ramp",
      [
        [4, 4],
        [5, 4],
      ],
      c,
      options({ lower: true }),
    );
    expect(removed?.req.changes).toEqual([{ cells: [[4, 4]], ramp: "." }]);
    expect(strokeRequest(123, "ramp", [[4, 4]], c, options())).toBeNull();
  });

  it("erase_sets_empty_on_the_active_layer", () => {
    const c = world((c) => {
      c.tiles[1][0][0] = "0050";
    });
    const out = strokeRequest(
      123,
      "erase",
      [
        [0, 0],
        [1, 0],
      ],
      c,
      options({ layer: 1 }),
    );
    expect(out?.req).toEqual({
      map: 123,
      label: "Radiert: Karte 0123 (1 Zelle)",
      changes: [{ layer: 1, cells: [[0, 0]], tile: "." }],
    });
  });

  it("fill_request_paints_the_filled_cells", () => {
    const c = world();
    const cells = fillCells(c, 1, [0, 0]);
    const out = strokeRequest(123, "fill", cells, c, options({ layer: 1 }));
    expect(out?.req.changes).toHaveLength(1);
    expect(out?.req.changes[0].cells).toHaveLength(1024);
    expect(out?.req.label).toBe("Gemalt: Karte 0123 (1024 Zellen)");
  });

  it("cells_of_reads_missing_rows_as_ground_defaults", () => {
    const c = cellsOf({
      version: 1,
      tileset: 0,
      layers: [{ name: "Boden", role: "ground", tiles: rows("0A0B"), heights: rows("+5") }],
      permissions: { derive: true },
    });
    expect(c.tiles[0][31][31]).toBe("0a0b");
    expect(c.heights[3][4]).toBe(5);
    expect(c.ramps[0][0]).toBe(".");
  });
});
