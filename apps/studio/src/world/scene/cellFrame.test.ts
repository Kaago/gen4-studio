import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../../api";
import { selectionColor } from "../colors";
import { FRAME_WIDTH, cellFrame, showCellFrame } from "./cellFrame";
import type { SceneHost } from "./SceneHost";
import { OVERLAY_Y } from "./content";

const matrix: MatrixDto = { index: 3, name: "m", width: 3, height: 3, maps: [1, 2, 3, 4, 5, 6, 7, 8, 9], altitudes: [0, 0, 0, 0, 0, 0, 0, 4, 0] };

/** The corners of the frame's quads, four per side, as [x, y, z]. */
function corners(frame: THREE.Mesh) {
  const position = frame.geometry.getAttribute("position");
  return Array.from({ length: position.count }, (_, i) => [position.getX(i), position.getY(i), position.getZ(i)]);
}

const loaded = new Map(matrix.maps.map((id) => [id as number, {}]));

describe("cellFrame", () => {
  it("draws_a_flat_ring_half_a_tile_wide_around_the_cell_at_its_altitude", () => {
    const frame = cellFrame(matrix, 1, 2, "#123456");
    expect(frame).toBeInstanceOf(THREE.Mesh);
    // Altitude 4 is 2 scene units; the ring lies over the overlay plane of the cell (positions are 32-bit floats).
    const y = Math.fround(2 + OVERLAY_Y * 2);
    // The cell spans x 32..64, z 64..96; the ring's inner edge is its border, the outer edge half a tile beyond.
    expect(FRAME_WIDTH).toBe(0.5);
    expect(corners(frame)).toEqual([
      // north
      [31.5, y, 63.5],
      [64.5, y, 63.5],
      [64.5, y, 64],
      [31.5, y, 64],
      // south
      [31.5, y, 96],
      [64.5, y, 96],
      [64.5, y, 96.5],
      [31.5, y, 96.5],
      // west
      [31.5, y, 64],
      [32, y, 64],
      [32, y, 96],
      [31.5, y, 96],
      // east
      [64, y, 64],
      [64.5, y, 64],
      [64.5, y, 96],
      [64, y, 96],
    ]);
    // Four quads of two triangles, each facing up.
    const index = frame.geometry.getIndex()!;
    expect(index.count).toBe(24);
    const p = corners(frame).map(([x, py, z]) => new THREE.Vector3(x, py, z));
    for (let t = 0; t < 8; t++) {
      const [a, b, c] = [0, 1, 2].map((k) => p[index.getX(t * 3 + k)]!);
      expect(new THREE.Vector3().crossVectors(b.clone().sub(a), c.clone().sub(a)).y).toBeGreaterThan(0);
    }
    const material = frame.material as THREE.MeshBasicMaterial;
    expect(material.color.getHexString()).toBe("123456");
    // Buildings cover it; it is drawn after the tiles (renderOrder 10).
    expect(material.depthTest).toBe(true);
    expect(frame.renderOrder).toBeGreaterThan(10);
  });

  it("frames_the_selected_cell_of_the_shown_matrix_and_nothing_without_one_or_outside_it", () => {
    const shown = (selectedCell: { x: number; y: number } | null, m: MatrixDto | null) => {
      const setCellFrame = vi.fn();
      showCellFrame({ selectedCell, matrix: m, loaded, setCellFrame } as unknown as SceneHost, "hell");
      return setCellFrame.mock.calls[0][0] as THREE.Mesh | null;
    };
    const frame = shown({ x: 2, y: 0 }, matrix);
    expect(frame?.geometry.getAttribute("position").getX(0)).toBe(63.5);
    expect(`#${(frame?.material as THREE.MeshBasicMaterial).color.getHexString()}`.toUpperCase()).toBe(selectionColor("hell"));
    expect(shown(null, matrix)).toBeNull();
    expect(shown({ x: 0, y: 0 }, null)).toBeNull();
    expect(shown({ x: 3, y: 0 }, matrix)).toBeNull();
  });

  it("draws_no_frame_over_a_cell_whose_map_is_not_loaded", () => {
    // A landscape click far away on the world map only pins the cell: the masked scene matrix has no map there, or the map is not loaded.
    const masked: MatrixDto = { ...matrix, maps: [1, 2, null, 4, 5, 6, 7, 8, 9] };
    const shown = (cell: { x: number; y: number }, l: Map<number, unknown>) => {
      const setCellFrame = vi.fn();
      showCellFrame({ selectedCell: cell, matrix: masked, loaded: l, setCellFrame } as unknown as SceneHost, "hell");
      return setCellFrame.mock.calls[0][0] as THREE.Mesh | null;
    };
    expect(shown({ x: 2, y: 0 }, loaded)).toBeNull();
    expect(shown({ x: 1, y: 0 }, new Map([[1, {}]]))).toBeNull();
    expect(shown({ x: 1, y: 0 }, loaded)).toBeInstanceOf(THREE.Mesh);
  });
});
