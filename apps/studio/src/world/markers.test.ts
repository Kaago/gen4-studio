import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { EventsDto } from "../api";
import { MARKER_Y, buildMarkers, moveMarker, pickMarker, selectMarker } from "./markers";

const matrix = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2] };

const events = (): EventsDto => ({
  event_file_id: 1,
  locked: null,
  bg: [{ index: 0, script: 1, event_type: 1, x: 20, z: 21, y: 0 }],
  objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, x: 12, z: 13, y: 0 }],
  warps: [
    { index: 0, x: 8, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 },
    { index: 1, x: 70, z: 4, dest_header_id: 1, dest_warp_id: 0, height: 0 },
  ],
  coords: [{ index: 0, script: 9000, x: 3, z: 5, width: 4, length: 2 }],
});

/** The ray looking straight down on tile (x, z). */
function rayOver(x: number, z: number) {
  const r = new THREE.Raycaster();
  r.set(new THREE.Vector3(x + 0.5, 10, z + 0.5), new THREE.Vector3(0, -1, 0));
  return r;
}

function instancePosition(mesh: THREE.InstancedMesh, i: number) {
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
    return mesh as THREE.InstancedMesh;
  };

  it("lifts a marker by the height of the cell of its tile", () => {
    const set = buildMarkers(raised, { ...events(), objects: [{ index: 0, local_id: 0, graphics_id: 0, movement_type: 0, script: 0, x: 40, z: 3, y: 0 }] });
    expect(instancePosition(at(set, "object"), 0)).toEqual([40.5, MARKER_Y + 1, 3.5]);
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
    expect(instancePosition(at(set, "object"), 0)).toEqual([40.5, MARKER_Y + 1, 3.5]);
    expect(set.outline?.position.y).toBe(MARKER_Y + 1);
    moveMarker(set, { kind: "object", index: 0 }, { x: 12, z: 13 });
    expect(instancePosition(at(set, "object"), 0)).toEqual([12.5, MARKER_Y, 13.5]);
    expect(set.outline?.position.y).toBe(MARKER_Y);
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
    expect(instancePosition(object[0], 0)).toEqual([12.5, 0.3, 13.5]);
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
    expect(set.outline?.position.toArray()).toEqual([12.5, 0.3, 13.5]);
  });
});
