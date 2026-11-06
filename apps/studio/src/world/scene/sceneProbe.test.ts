import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { MatrixDto } from "../../api";
import { MAP_SIZE } from "../layout";
import { viewThrough } from "./dragTestKit.testkit";
import { installSceneProbe } from "./sceneProbe";

/** One cell, one map; columns from 16 on lie 32 E (two tiles) up. */
const one: MatrixDto = { index: 0, name: "m", width: 1, height: 1, maps: [1], headers: null, altitudes: null };
const slope = { 1: Array.from({ length: MAP_SIZE * MAP_SIZE }, (_, i) => (i % MAP_SIZE >= 16 ? 32 : 0)) };

function view(matrix: MatrixDto | null = one) {
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 500);
  camera.position.set(16, 30, 45);
  camera.lookAt(16, 0, 12);
  return viewThrough(camera, matrix, slope);
}

describe("installSceneProbe", () => {
  it("the_probe_points_at_the_tile_it_names", () => {
    const { h, picker } = view();
    const remove = installSceneProbe(h);
    // Tiles on both sides of the step, and the step itself.
    for (const [x, z] of [
      [20, 10],
      [15, 10],
      [16, 10],
      [3, 27],
      [30, 2],
    ]) {
      const at = window.__g4SceneProbe?.tileToClient(x, z);
      expect(at).not.toBeNull();
      expect(picker.pickTile(at!.clientX, at!.clientY)).toMatchObject({ x, y: z });
    }
    // Beside the matrix there is no tile.
    expect(window.__g4SceneProbe?.tileToClient(40, 10)).toBeNull();
    expect(window.__g4SceneProbe?.tileToClient(-1, 10)).toBeNull();
    remove();
  });

  it("the_probe_is_removed_on_cleanup", () => {
    const remove = installSceneProbe(view().h);
    expect(window.__g4SceneProbe).toBeDefined();
    remove();
    expect(window.__g4SceneProbe).toBeUndefined();
    // Without a matrix it names no point.
    const none = installSceneProbe(view(null).h);
    expect(window.__g4SceneProbe?.tileToClient(20, 10)).toBeNull();
    none();
  });
});
