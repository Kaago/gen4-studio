import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { EventsDto } from "../api";
import { MARKER_Y, applySprite, buildMarkers, disposeMarkers, moveMarker, pickMarker, selectMarker } from "./markers";

const matrix = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2] };

const events = (): EventsDto => ({
  event_file_id: 1,
  locked: null,
  bg: [{ index: 0, script: 1, event_type: 1, x: 20, z: 21, y: 0 }],
  objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, dir: 1, x: 12, z: 13, y: 0 }],
  warps: [
    { index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 },
    { index: 1, x: 70, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 },
  ],
  coords: [{ index: 0, script: 9000, x: 3, z: 5, width: 4, length: 2 }],
});

/**
 * The ray of a camera south of and above tile (x, z) looking at the middle of its markers. (A sprite is a billboard,
 * so the raycaster needs a camera, and a camera looking straight down would see it edge-on.)
 */
function rayOver(x: number, z: number) {
  const camera = new THREE.PerspectiveCamera(30);
  camera.position.set(x + 0.5, 6, z + 4.5);
  camera.lookAt(x + 0.5, 0.3, z + 0.5);
  camera.updateMatrixWorld(true);
  const r = new THREE.Raycaster();
  r.setFromCamera(new THREE.Vector2(0, 0), camera);
  return r;
}

/** Where an instance or a sprite stands. */
function instancePosition(mesh: THREE.Object3D, i: number) {
  if (!(mesh instanceof THREE.InstancedMesh)) return mesh.position.toArray().map((v) => Math.round(v * 1000) / 1000);
  const m = new THREE.Matrix4();
  mesh.getMatrixAt(i, m);
  // Instance matrices are single precision.
  return new THREE.Vector3().setFromMatrixPosition(m).toArray().map((v) => Math.round(v * 1000) / 1000);
}

describe("buildMarkers", () => {
  it("draws a flat plane over the width x length area of a coordinate trigger", () => {
    const set = buildMarkers(matrix, events());
    expect(set.area).not.toBeNull();
    const m = new THREE.Matrix4();
    set.area?.getMatrixAt(0, m);
    const position = new THREE.Vector3();
    const scale = new THREE.Vector3();
    m.decompose(position, new THREE.Quaternion(), scale);
    // Tiles x 3..6 and z 5..6: centred at (5, 6), 4 x 2 tiles.
    expect(position.toArray().map((v) => Math.round(v * 1000) / 1000)).toEqual([5, 0.08, 6]);
    expect([scale.x, scale.z]).toEqual([4, 2]);
  });

  it("makes no area plane without coordinate triggers", () => {
    const set = buildMarkers(matrix, { ...events(), coords: [] });
    expect(set.area).toBeNull();
  });

  it("leaves events outside the matrix without a marker", () => {
    const set = buildMarkers(matrix, events());
    const warps = [...set.entries.values()].find((e) => e.kind === "warp");
    expect(warps?.data.map((d) => d.index)).toEqual([0]);
  });
});

describe("altitude", () => {
  // Cell (1, 0) has altitude 2 (one tile up), cell (0, 0) none.
  const raised = { ...matrix, altitudes: [0, 2] };
  const at = (set: ReturnType<typeof buildMarkers>, kind: string) => {
    const [mesh] = [...set.entries.entries()].find(([, e]) => e.kind === kind) ?? [];
    return mesh as THREE.Object3D;
  };

  it("lifts a marker by the height of the cell of its tile", () => {
    const set = buildMarkers(raised, { ...events(), objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, dir: 1, x: 40, z: 3, y: 0 }] });
    expect(instancePosition(at(set, "object"), 0)).toEqual([40.5, 1, 3.5]);
    // An event in the flat cell stays down.
    expect(instancePosition(at(set, "bg"), 0)[1]).toBe(MARKER_Y);
  });

  it("lifts the beacon, the area plane and the outline too", () => {
    const e = { ...events(), warps: [{ index: 0, x: 40, z: 3, dest_header_id: 1, dest_warp_id: 0, height: 0 }], coords: [{ index: 0, script: 9000, x: 40, z: 5, width: 4, length: 2 }] };
    const set = buildMarkers(raised, e);
    const warpMeshes = [...set.entries.entries()].filter(([, en]) => en.kind === "warp").map(([m]) => m);
    expect(warpMeshes.map((m) => instancePosition(m, 0)[1])).toEqual([MARKER_Y + 1, 0.6 + 1]);
    expect(instancePosition(set.area as THREE.InstancedMesh, 0)[1]).toBeCloseTo(0.08 + 1, 5);
    selectMarker(set, { kind: "warp", index: 0 });
    expect(set.outline?.position.y).toBe(0.6 + 1);
  });

  it("moves the marker to the height of the cell it is dragged into, and back", () => {
    const set = buildMarkers(raised, events());
    selectMarker(set, { kind: "object", index: 0 });
    moveMarker(set, { kind: "object", index: 0 }, { x: 40, z: 3 });
    // The sprite stands on the ground of the cell; the outline is as tall as the sprite.
    expect(instancePosition(at(set, "object"), 0)).toEqual([40.5, 1, 3.5]);
    expect(set.outline?.position.y).toBeCloseTo(1 + 0.2, 5);
    moveMarker(set, { kind: "object", index: 0 }, { x: 12, z: 13 });
    expect(instancePosition(at(set, "object"), 0)).toEqual([12.5, 0, 13.5]);
    expect(set.outline?.position.y).toBeCloseTo(0.2, 5);
  });

  it("keeps the beacon and the area plane at their own height above the cell when moved", () => {
    const set = buildMarkers(raised, events());
    moveMarker(set, { kind: "warp", index: 0 }, { x: 40, z: 4 });
    const warpMeshes = [...set.entries.entries()].filter(([, en]) => en.kind === "warp").map(([m]) => m);
    expect(warpMeshes.map((m) => instancePosition(m, 0)[1])).toEqual([MARKER_Y + 1, 0.6 + 1]);
    moveMarker(set, { kind: "coord", index: 0 }, { x: 40, z: 10 });
    expect(instancePosition(set.area as THREE.InstancedMesh, 0)[1]).toBeCloseTo(0.08 + 1, 5);
    moveMarker(set, { kind: "coord", index: 0 }, { x: 3, z: 5 });
    expect(instancePosition(set.area as THREE.InstancedMesh, 0)[1]).toBeCloseTo(0.08, 5);
  });
});

describe("pickMarker", () => {
  it("returns the marker under the ray, and nothing over bare ground or while hidden", () => {
    const set = buildMarkers(matrix, events());
    expect(pickMarker(set, rayOver(8, 4))).toMatchObject({ kind: "warp", data: { index: 0 } });
    expect(pickMarker(set, rayOver(12, 13))).toMatchObject({ kind: "object" });
    expect(pickMarker(set, rayOver(30, 30))).toBeNull();
    // The area plane is not pickable, only the marker standing in it.
    expect(pickMarker(set, rayOver(5, 6))).toBeNull();
    set.group.visible = false;
    expect(pickMarker(set, rayOver(8, 4))).toBeNull();
    expect(pickMarker(null, rayOver(8, 4))).toBeNull();
  });
});

describe("moveMarker", () => {
  it("moves every instance of the event, and nothing else", () => {
    const set = buildMarkers(matrix, events());
    moveMarker(set, { kind: "warp", index: 0 }, { x: 9, z: 6 });
    const meshes = [...set.entries.entries()].filter(([, e]) => e.kind === "warp").map(([mesh]) => mesh);
    // Flat marker and tall beacon, each at the new tile centre and its own height.
    expect(meshes.map((m) => instancePosition(m, 0))).toEqual([
      [9.5, 0.3, 6.5],
      [9.5, 0.6, 6.5],
    ]);
    expect(pickMarker(set, rayOver(9, 6))).toMatchObject({ kind: "warp", data: { index: 0 } });
    expect(pickMarker(set, rayOver(8, 4))).toBeNull();
    const [object] = [...set.entries.entries()].filter(([, e]) => e.kind === "object");
    expect(instancePosition(object[0], 0)).toEqual([12.5, 0, 13.5]);
  });

  it("moves the area plane of a coordinate trigger with it", () => {
    const set = buildMarkers(matrix, events());
    moveMarker(set, { kind: "coord", index: 0 }, { x: 10, z: 10 });
    expect(instancePosition(set.area as THREE.InstancedMesh, 0)).toEqual([12, 0.08, 11]);
  });

  it("puts an event back where its data says", () => {
    const set = buildMarkers(matrix, events());
    moveMarker(set, { kind: "bg", index: 0 }, { x: 25, z: 25 });
    moveMarker(set, { kind: "bg", index: 0 }, { x: 20, z: 21 });
    expect(pickMarker(set, rayOver(20, 21))).toMatchObject({ kind: "bg" });
  });
});

describe("selectMarker", () => {
  it("outlines the selected marker and the outline follows a move", () => {
    const set = buildMarkers(matrix, events());
    selectMarker(set, { kind: "warp", index: 0 });
    expect(set.outline?.position.toArray()).toEqual([8.5, 0.6, 4.5]);
    moveMarker(set, { kind: "warp", index: 0 }, { x: 9, z: 4 });
    expect(set.outline?.position.toArray()).toEqual([9.5, 0.6, 4.5]);
    // Moving another event leaves the outline alone.
    moveMarker(set, { kind: "object", index: 0 }, { x: 1, z: 1 });
    expect(set.outline?.position.toArray()).toEqual([9.5, 0.6, 4.5]);
  });

  it("outlines nothing for no selection or for an event without a marker", () => {
    const set = buildMarkers(matrix, events());
    selectMarker(set, { kind: "warp", index: 0 });
    selectMarker(set, null);
    expect(set.outline).toBeNull();
    expect(set.group.children.some((c) => c instanceof THREE.LineSegments)).toBe(false);
    // Warp 1 lies outside the matrix.
    selectMarker(set, { kind: "warp", index: 1 });
    expect(set.outline).toBeNull();
    expect(set.selected).toBeNull();
  });

  it("replaces the outline instead of stacking them", () => {
    const set = buildMarkers(matrix, events());
    selectMarker(set, { kind: "warp", index: 0 });
    selectMarker(set, { kind: "object", index: 0 });
    expect(set.group.children.filter((c) => c instanceof THREE.LineSegments)).toHaveLength(1);
    expect(set.outline?.position.toArray()).toEqual([12.5, 0.2, 13.5]);
  });
});

describe("object sprites", () => {
  const spriteOf = (set: ReturnType<typeof buildMarkers>) => [...set.entries.keys()].find((o) => o instanceof THREE.Sprite) as THREE.Sprite;
  const picture = (w: number, h: number) => ({ texture: new THREE.DataTexture(new Uint8Array(w * h * 4), w, h), width: w, height: h });

  it("shows an object as a bottom-anchored billboard on the middle of its tile, a coloured square (the figure colour) until its picture is there", () => {
    const set = buildMarkers(matrix, events());
    const sprite = spriteOf(set);
    expect(sprite).toBeInstanceOf(THREE.Sprite);
    expect(sprite.position.toArray()).toEqual([12.5, 0, 13.5]);
    expect(sprite.center.toArray()).toEqual([0.5, 0]);
    expect(sprite.scale.toArray()).toEqual([0.8, 0.4, 1]);
    expect(sprite.renderOrder).toBe(11);
    expect(sprite.material).toMatchObject({ depthTest: false, transparent: true });
    expect(sprite.material.map).toBe(set.placeholder);
    // The figure colour of the scene tokens (#4273AD), opaque.
    expect(Array.from(set.placeholder.image.data as Uint8Array)).toEqual([66, 115, 173, 255]);
    expect(set.group.children).toContain(sprite);
    // One sprite for the object, none for the other classes.
    expect([...set.entries.keys()].filter((o) => o instanceof THREE.Sprite)).toHaveLength(1);
  });

  it("gives every object its own sprite and leaves those outside the matrix out", () => {
    const object = (index: number, x: number) => ({ index, local_id: index, graphics_id: 0, movement_type: 0, script: 0, dir: 1, x, z: 3, y: 0 });
    const set = buildMarkers(matrix, { ...events(), objects: [object(0, 4), object(1, 5), object(2, 900)] });
    const sprites = [...set.entries.entries()].filter(([o]) => o instanceof THREE.Sprite);
    expect(sprites.map(([, e]) => e.data.map((d) => d.index))).toEqual([[0], [1]]);
    expect(new Set(sprites.map(([o]) => (o as THREE.Sprite).material)).size).toBe(2);
  });

  it("finds the sprite under the ray, and moves it", () => {
    const set = buildMarkers(matrix, events());
    expect(pickMarker(set, rayOver(12, 13))).toMatchObject({ kind: "object", data: { index: 0 } });
    expect(pickMarker(set, rayOver(14, 13))).toBeNull();
    moveMarker(set, { kind: "object", index: 0 }, { x: 14, z: 13 });
    expect(spriteOf(set).position.toArray()).toEqual([14.5, 0, 13.5]);
    expect(pickMarker(set, rayOver(14, 13))).toMatchObject({ kind: "object" });
    expect(pickMarker(set, rayOver(12, 13))).toBeNull();
  });

  it("applySprite shows the picture at width / 16 x height / 16 tiles", () => {
    const set = buildMarkers(matrix, events());
    const big = picture(32, 32);
    expect(applySprite(set, 0, big)).toBe(true);
    const sprite = spriteOf(set);
    expect(sprite.scale.toArray()).toEqual([2, 2, 1]);
    expect(sprite.material.map).toBe(big.texture);
    // The anchor stays at the bottom, the sprite still stands on its tile.
    expect(sprite.position.toArray()).toEqual([12.5, 0, 13.5]);
    expect(applySprite(set, 0, picture(16, 24))).toBe(true);
    expect(sprite.scale.toArray()).toEqual([1, 1.5, 1]);
    // The picked area follows the picture.
    expect(pickMarker(set, rayOver(12, 13))).toMatchObject({ kind: "object" });
  });

  it("applySprite does nothing for an object without a marker", () => {
    const set = buildMarkers(matrix, events());
    expect(applySprite(set, 7, picture(16, 16))).toBe(false);
    expect(spriteOf(set).material.map).toBe(set.placeholder);
  });

  it("outlines a selected object as tall as its sprite, also when the picture arrives later", () => {
    const set = buildMarkers(matrix, events());
    selectMarker(set, { kind: "object", index: 0 });
    const box = () => {
      const b = new THREE.Box3().setFromBufferAttribute(set.outline?.geometry.getAttribute("position") as THREE.BufferAttribute);
      return b.getSize(new THREE.Vector3()).toArray().map((v) => Math.round(v * 1000) / 1000);
    };
    expect(box()).toEqual([0.95, 0.4, 0.95]);
    applySprite(set, 0, picture(16, 32));
    expect(set.selected).toMatchObject({ kind: "object", index: 0 });
    // A sprite wider than the usual box is outlined at its own width.
    expect(box()).toEqual([1, 2, 1]);
    expect(set.outline?.position.toArray()).toEqual([12.5, 1, 13.5]);
    expect(set.group.children.filter((c) => c instanceof THREE.LineSegments)).toHaveLength(1);
  });

  it("disposes its own materials and placeholder with the set, and leaves the pictures to the sprite cache", () => {
    const set = buildMarkers(matrix, events());
    const shared = picture(16, 16);
    applySprite(set, 0, shared);
    const sprite = spriteOf(set);
    const material = vi.spyOn(sprite.material, "dispose");
    const placeholder = vi.spyOn(set.placeholder, "dispose");
    const cached = vi.spyOn(shared.texture, "dispose");
    const boxes = vi.spyOn(THREE.BufferGeometry.prototype, "dispose");
    disposeMarkers(set);
    expect(material).toHaveBeenCalled();
    expect(placeholder).toHaveBeenCalled();
    expect(cached).not.toHaveBeenCalled();
    // The geometry every sprite shares is not freed either: the other marker sets use it.
    expect(boxes.mock.contexts.includes(sprite.geometry)).toBe(false);
    boxes.mockRestore();
  });
});
