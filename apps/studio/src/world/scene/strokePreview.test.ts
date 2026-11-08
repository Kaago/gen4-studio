import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../../api";
import { szene } from "../../ui/tokens/tokens";
import { createGround } from "../ground";
import { HEIGHT_LAYER_Y, buildHeightLayer } from "./heightLayer";
import type { SceneHost } from "./SceneHost";
import { PREVIEW_LIFT, clearStrokePreview, holdStrokePreview, plateMeshUvs, plateUvs, setStrokePreview, setStrokeRect, uvOf } from "./strokePreview";

/** Matrix 0, 2 × 1: map 5 in cell (0, 0), raised by 4 altitude steps (2 scene units), map 7 beside it. */
const matrix: MatrixDto = { index: 0, name: "m", width: 2, height: 1, maps: [5, 7], headers: null, altitudes: [4, 0] };

const host = () => ({ matrix, ground: createGround(matrix, new Map()), paintPreview: new THREE.Group() }) as unknown as SceneHost;

const plates = (h: SceneHost) => h.paintPreview.children.filter((c): c is THREE.InstancedMesh => c instanceof THREE.InstancedMesh);

describe("strokePreview", () => {
  it("preview_plates_sit_above_the_height_layer", () => {
    const h = host();
    setStrokePreview(h, 5, [[1, 2], [40, 2]], { color: (cell) => (cell[0] === 1 ? szene.hoehe["3"] : szene.hoehe["0"]) });
    const [mesh] = plates(h);
    // The cell outside the map is left out.
    expect(mesh.count).toBe(1);
    const m4 = new THREE.Matrix4();
    mesh.getMatrixAt(0, m4);
    const at = new THREE.Vector3().setFromMatrixPosition(m4);
    expect(at.x).toBe(1.5);
    expect(at.z).toBe(2.5);
    expect(at.y).toBeCloseTo(HEIGHT_LAYER_Y + PREVIEW_LIFT + 2);
    expect(at.y).toBeGreaterThan(HEIGHT_LAYER_Y + 2);
    const heights = buildHeightLayer(matrix, new Map([[5, { map: {} } as never]]), h.ground);
    const heightTiles = heights.getObjectByName("tiles") as THREE.Object3D;
    const isolines = heights.getObjectByName("isolines") as THREE.Object3D;
    expect(mesh.renderOrder).toBeGreaterThan(Math.max(heightTiles.renderOrder, isolines.renderOrder));
    const material = mesh.material as THREE.MeshBasicMaterial;
    expect(material.depthTest).toBe(false);
    const color = new THREE.Color();
    mesh.getColorAt(0, color);
    expect(color.getHex()).toBe(szene.hoehe["3"]);
  });

  it("erase_is_hatched_and_rect_draws_a_dashed_frame", () => {
    const h = host();
    setStrokePreview(h, 5, [[0, 0]], { erase: true });
    expect((plates(h)[0].material as THREE.MeshBasicMaterial).map).toBeInstanceOf(THREE.DataTexture);
    setStrokeRect(h, 5, [1, 1], [3, 2]);
    const frame = h.paintPreview.children.find((c) => c instanceof THREE.LineSegments) as THREE.LineSegments;
    expect(frame.material).toBeInstanceOf(THREE.LineDashedMaterial);
    const xs = Array.from(frame.geometry.getAttribute("position").array).filter((_, i) => i % 3 === 0);
    expect(Math.min(...xs)).toBe(1);
    expect(Math.max(...xs)).toBe(4);
  });

  it("a_tile_texture_is_drawn_texel_for_texel_with_its_first_row_north", () => {
    const h = host();
    const picture = { width: 32, height: 32 } as unknown as CanvasImageSource;
    setStrokePreview(h, 5, [[0, 0]], { texture: picture });
    const map = (plates(h)[0].material as THREE.MeshBasicMaterial).map!;
    expect(map.image).toBe(picture);
    expect(map.magFilter).toBe(THREE.NearestFilter);
    expect(map.minFilter).toBe(THREE.NearestFilter);
    // flipY is ignored for bitmaps (the first row lands at v = 0): the plate's UVs put it at the north edge, no texture transform.
    expect(map.flipY).toBe(false);
    expect([map.repeat.y, map.offset.y]).toEqual([1, 0]);
    expect(Array.from(plates(h)[0].geometry.getAttribute("uv").array)).toEqual(plateUvs(0, false));
  });

  it("uv_of_matches_the_table_of_world_map_md", () => {
    // docs/formats/world_map.md, „Koordinaten und UV“: r0 (x, z), r1 (z, −x), r2 (−x, −z), r3 (−z, x), f: u = −u after the turn.
    expect(uvOf(3, 5, 0, false)).toEqual([3, 5]);
    expect(uvOf(3, 5, 1, false)).toEqual([5, -3]);
    expect(uvOf(3, 5, 2, false)).toEqual([-3, -5]);
    expect(uvOf(3, 5, 3, false)).toEqual([-5, 3]);
    expect(uvOf(3, 5, 0, true)).toEqual([-3, 5]);
    expect(uvOf(3, 5, 1, true)).toEqual([-5, -3]);
    expect(uvOf(3, 5, 3, true)).toEqual([5, 3]);
  });

  it("plate_uvs_turn_and_flip_like_the_mesh", () => {
    // Corners NW (0, 0), NE (1, 0), SW (0, 1), SE (1, 1); mesh uv = (u, −v) as meshGeometries divides it.
    expect(plateMeshUvs(0, false)).toEqual([0, 0, 1, 0, 0, -1, 1, -1]);
    expect(plateMeshUvs(1, false)).toEqual([0, 0, 0, 1, 1, 0, 1, 1]);
    expect(plateMeshUvs(2, false)).toEqual([0, 0, -1, 0, 0, 1, -1, 1]);
    expect(plateMeshUvs(3, false)).toEqual([0, 0, 0, -1, -1, 0, -1, -1]);
    expect(plateMeshUvs(0, true)).toEqual([0, 0, -1, 0, 0, -1, -1, -1]);
    // The bitmap's (column, row) at each corner, row 0 its first row: r0 unchanged (first row north, as before), r1 turned clockwise
    // seen from above (first row along the east edge), r2 upside down, r3 counter-clockwise; f mirrors the columns.
    expect(plateUvs(0, false)).toEqual([0, 0, 1, 0, 0, 1, 1, 1]);
    expect(plateUvs(1, false)).toEqual([0, 1, 0, 0, 1, 1, 1, 0]);
    expect(plateUvs(2, false)).toEqual([1, 1, 0, 1, 1, 0, 0, 0]);
    expect(plateUvs(3, false)).toEqual([1, 0, 1, 1, 0, 0, 0, 1]);
    expect(plateUvs(0, true)).toEqual([1, 0, 0, 0, 1, 1, 0, 1]);
    expect(plateUvs(1, true)).toEqual([1, 1, 1, 0, 0, 1, 0, 0]);
    // Each matches the mesh up to whole texture repeats, with v mirrored for the bitmap.
    for (const [r, f] of [[0, false], [1, false], [2, false], [3, false], [0, true], [1, true], [2, true], [3, true]] as const) {
      const mesh = plateMeshUvs(r, f);
      plateUvs(r, f).forEach((value, i) => {
        const want = i % 2 === 0 ? mesh[i] : 1 - mesh[i];
        expect(Number.isInteger(value - want)).toBe(true);
      });
    }
  });

  it("a_turned_tile_gets_its_turned_plate_uvs", () => {
    const h = host();
    setStrokePreview(h, 5, [[0, 0]], { texture: { width: 16, height: 16 } as unknown as CanvasImageSource, rotate: 1, flip: true });
    expect(Array.from(plates(h)[0].geometry.getAttribute("uv").array)).toEqual(plateUvs(1, true));
  });

  it("clear_frees_the_geometry", () => {
    const h = host();
    setStrokePreview(h, 5, [[0, 0], [1, 0]], { color: szene.hoehe["1"] });
    setStrokeRect(h, 5, [0, 0], [1, 0]);
    const [mesh] = plates(h);
    const geometry = vi.spyOn(mesh.geometry, "dispose");
    const material = vi.spyOn(mesh.material as THREE.Material, "dispose");
    clearStrokePreview(h);
    expect(geometry).toHaveBeenCalled();
    expect(material).toHaveBeenCalled();
    expect(h.paintPreview.children).toHaveLength(0);
  });

  it("a_held_preview_stays_until_its_own_clear", () => {
    const h = host();
    setStrokePreview(h, 5, [[0, 0]], { color: szene.hoehe["1"] });
    const first = holdStrokePreview(h);
    setStrokePreview(h, 5, [[2, 0]], { color: szene.hoehe["2"] });
    const second = holdStrokePreview(h);
    // A new stroke draws beside the held ones; clearing the live one leaves them.
    clearStrokePreview(h);
    expect(plates(h)).toHaveLength(2);
    clearStrokePreview(h, first);
    expect(plates(h)).toHaveLength(1);
    expect(plates(h)[0].userData.previewId).toBe(second);
    clearStrokePreview(h, second);
    expect(h.paintPreview.children).toHaveLength(0);
  });
});
