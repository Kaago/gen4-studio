import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../../api";
import { selectionColor } from "../colors";
import { cellFrame, showCellFrame } from "./cellFrame";
import type { SceneHost } from "./SceneHost";
import { OVERLAY_Y } from "./content";

const matrix: MatrixDto = { index: 3, name: "m", width: 3, height: 3, maps: [1, 2, 3, 4, 5, 6, 7, 8, 9], altitudes: [0, 0, 0, 0, 0, 0, 0, 4, 0] };

describe("cellFrame", () => {
  it("draws_a_square_around_the_cell_at_its_altitude", () => {
    const frame = cellFrame(matrix, 1, 2, "#123456");
    expect(frame).toBeInstanceOf(THREE.LineLoop);
    const position = frame.geometry.getAttribute("position");
    const corners = Array.from({ length: position.count }, (_, i) => [position.getX(i), position.getY(i), position.getZ(i)]);
    // Altitude 4 is 2 scene units; the frame lies over the overlay plane of the cell (positions are 32-bit floats).
    const y = Math.fround(2 + OVERLAY_Y * 2);
    expect(corners).toEqual([
      [32, y, 64],
      [64, y, 64],
      [64, y, 96],
      [32, y, 96],
    ]);
    expect((frame.material as THREE.LineBasicMaterial).color.getHexString()).toBe("123456");
  });

  it("frames_the_selected_cell_of_the_shown_matrix_and_nothing_without_one_or_outside_it", () => {
    const shown = (selectedCell: { x: number; y: number } | null, m: MatrixDto | null) => {
      const setCellFrame = vi.fn();
      showCellFrame({ selectedCell, matrix: m, setCellFrame } as unknown as SceneHost, "hell");
      return setCellFrame.mock.calls[0][0] as THREE.LineLoop | null;
    };
    const frame = shown({ x: 2, y: 0 }, matrix);
    expect(frame?.geometry.getAttribute("position").getX(0)).toBe(64);
    expect(`#${(frame?.material as THREE.LineBasicMaterial).color.getHexString()}`.toUpperCase()).toBe(selectionColor("hell"));
    expect(shown(null, matrix)).toBeNull();
    expect(shown({ x: 0, y: 0 }, null)).toBeNull();
    expect(shown({ x: 3, y: 0 }, matrix)).toBeNull();
  });
});
